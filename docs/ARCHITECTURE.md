# Architecture

This document describes how Fita is built. Read it before changing anything that crosses a crate boundary.

## Principles

1. **One binary.** The same `fita` binary is the server, the worker and the CLI. A small install is one binary plus PostgreSQL.
2. **Nothing irreversible.** Every automatic decision is stored with its reason and can be undone. History is never deleted.
3. **Configuration is code.** Projects are YAML files in git. The UI is just another editor for those files.
4. **Read only by default.** Fita reads sources and never writes to them without an explicit, audited approval.
5. **Measured accuracy.** Matching quality is tested like code. A change that lowers precision on the benchmark does not merge.
6. **Domain code is pure.** Domain crates do not know about HTTP or PostgreSQL. Persistence is injected through traits.

## Crates

```
crates/
  fita-core        domain types, ids, errors and the storage traits
  fita-config      loads and validates projects and rules (YAML + JSON Schema)
  fita-expr        the rule expression language, parser and evaluator
  fita-connect     runs connectors that speak the Airbyte protocol, plus native DB and file connectors
  fita-normalize   attribute types, normalizers, validators and pack loading
  fita-match       blocking, comparison, deterministic rules, Fellegi-Sunter, clustering
  fita-survive     survivorship, provenance and golden record assembly
  fita-store       PostgreSQL access, migrations, job queue, in-memory store for tests
  fita-engine      orchestrates full and incremental runs
  fita-issues      issue detectors and impact
  fita-fiscal      tax id checks against public services (AEAT, VIES)
  fita-migrate     migration projects, blockers, crossref tables, load trials
  fita-api         REST API, webhooks, OpenAPI
  fita-mcp         MCP server for AI agents
  fita-auth        users, sessions, API keys, roles, SSO
  fita-cli         the fita binary and its subcommands
```

Later versions add `fita-relations` (hierarchies), `fita-refdata` (reference data), `fita-writeback` (approved writes to sources), `fita-notify` (email, Teams, Slack) and `fita-report` (PDF reports).

### Dependency rules

- `fita-core` depends on nothing else in the workspace.
- Domain crates (`fita-config`, `fita-expr`, `fita-normalize`, `fita-match`, `fita-survive`, `fita-issues`) depend only on `fita-core` and each other. They never import `sqlx`, `axum` or `tokio-postgres`.
- `fita-store` implements the traits from `fita-core`.
- `fita-engine` wires domain crates and storage together.
- `fita-api`, `fita-mcp` and `fita-cli` sit on top. The API and MCP call the same services and share the same permission layer.

If a change needs a domain crate to import a database or HTTP type, the design is wrong. Add a trait to `fita-core` instead.

These rules are tested by `crates/fita-architecture`, which reads the workspace with `cargo metadata` and fails `cargo test` when a crate breaks them. A new crate must be added to a layer in that test and in this list.

## Process roles

`fita serve` starts every role in one process. Large installs start the same binary several times with `--role api`, `--role worker` or `--role scheduler`. All coordination goes through PostgreSQL. There is no external queue.

- **api** serves the REST API, the embedded UI and MCP over HTTP. It never runs heavy jobs. It keeps an in-memory index of blocking keys for the "does this already exist" endpoint and refreshes it with PostgreSQL notifications.
- **worker** takes jobs with `SELECT ... FOR UPDATE SKIP LOCKED`, sends a heartbeat every 15 seconds and saves progress per batch. Jobs are idempotent, so a retry continues where the last attempt stopped.
- **scheduler** creates scheduled jobs. Only one is active at a time, through a PostgreSQL advisory lock.

## Where data lives

PostgreSQL is the system of record. It holds published configuration, source records, normalized records, golden records with history, decisions, issues, the job queue and the audit log.

Heavy matching does not run in PostgreSQL. A run exports the normalized records of a domain to Parquet in a working store (a local folder or an S3 compatible bucket) and processes them with Arrow and DataFusion in memory. Working files are disposable and can always be rebuilt from PostgreSQL.

## A run, step by step

1. Read each source and store raw records in `source_record`.
2. Normalize them into `normalized_record`, including blocking keys.
3. Export the domain to Parquet.
4. Generate candidate pairs with blocking.
5. Score pairs and cluster them.
6. Compute survivorship and write the golden records that changed.
7. Detect issues, refresh the index and notify consumers.

Each step is a job that enqueues the next. An incremental run does the same for changed records and their block neighbours, and only re-clusters the affected components.

## Data model

Everything lives in the `fita` schema. Internal ids are UUID v7. Golden records also have a readable public id made of the domain prefix and a number, such as `cus_00412`.

Membership and values are temporal tables. A change closes the current row and opens a new one, so any golden record can be seen as it was at any date. Two exclusion constraints guarantee in the database that a source record belongs to one golden record at a time and that an attribute has one current value.

```sql
create table fita.master_member (
  master_id        uuid not null references fita.master_record,
  source_record_id uuid not null references fita.source_record,
  rule             text,
  confidence       real,
  decided_by       text not null,          -- engine or a decision id
  valid            tstzrange not null,
  exclude using gist (source_record_id with =, valid with &&)
);
```

This needs the `btree_gist` extension. The full schema lives in `migrations/`. Migrations are plain SQL, applied with sqlx at startup and never edited after a release. The binary refuses to start if the database schema is ahead of it.

Main tables: `domain`, `source`, `source_record`, `normalized_record`, `master_record`, `master_member`, `master_value`, `master_redirect`, `link_constraint`, `review_item`, `decision`, `run`, `source_sync`, `issue`, `fiscal_check`, `crossref`, `job`, `config_revision`, `audit_event`, plus the access tables (`app_user`, `role_binding`, `session`, `consumer`, `api_key`, `webhook_endpoint`, `webhook_delivery`, `consumer_call`).

## Project format

```
my-project/
  fita.yaml                 general settings, packs, working store
  domains/customers.yaml    attributes of a domain
  sources/billing.yaml      connector, mapping and schedule of a source
  rules/customers.yaml      blocking, matching and survivorship
  issues/customers.yaml     active detectors and thresholds
  consumers/*.yaml          API and MCP consumers
```

Config keys, commands and the API are in English. Domain, attribute and rule names are chosen by each user and can be in any language.

```yaml
# rules/customers.yaml
domain: customers
version: 8
blocking:
  - [tax_id]
  - [postcode, prefix(legal_name, 4)]
rules:
  - name: same_tax_id_and_postcode
    when: [equal(tax_id), equal(postcode)]
    confidence: 1.0
  - name: name_and_street
    when:
      - similarity(legal_name) >= 0.85
      - equal(street)
    confidence: 0.81
    action: review
  - name: different_tax_id
    when: [different(tax_id), valid(tax_id)]
    action: never_merge
model:
  enabled: true
  fields: [tax_id, legal_name, address, phone, email]
  merge_threshold: 0.92
  review_threshold: 0.75
survivorship:
  tax_id: [source(billing), valid]
  email: [source(crm, billing), most_recent]
```

Rules run in order and the first match decides. A `never_merge` rule always wins, even if it comes later, and so do manual decisions. If no rule decides and the model is on, the model decides with its two thresholds.

Secrets never appear in project files. Only references such as `${ENV_VAR}` or `${secret:name}`.

### Expression language

`fita-expr` parses rule conditions and compiles them to a function over two normalized records. It is deliberately small. No loops, no variables, no access outside the two records, so a rule can never hang the engine or leak data.

Functions: `equal`, `different`, `empty`, `valid`, `similarity`, `jaro_winkler`, `levenshtein`, `token_set`, `phonetic`, `distance_km`, `prefix`, `suffix`, `contains`, `in_list`. A list means AND. `any: [...]` means OR. Errors are reported with line and column.

## Ingestion

Connectors speak the Airbyte protocol (`spec`, `check`, `discover`, `read`, with `RECORD` and `STATE` messages on stdout). Fita runs community connector images unchanged with Docker, Podman or a Kubernetes Job. Database and file connectors are compiled into the binary.

- The last `STATE` is committed only after every previous record is in PostgreSQL.
- Full loads compare a hash of each record and mark missing ones with `deleted_at`. Nothing is deleted.
- Incremental loads use a cursor column or the source change capture. A weekly full load reconciles deletes.
- Writes go in batches of 5,000 with `COPY` and `INSERT ... ON CONFLICT (source_id, external_id)`.
- A removed mapped column puts the source in `failing` instead of silently emptying an attribute.

| Database | Driver |
| --- | --- |
| PostgreSQL | `tokio-postgres` |
| Oracle | `oracle` (Instant Client) |
| SQL Server, Azure SQL | `tiberius` |
| MySQL, MariaDB | `mysql_async` |
| IBM Db2, IBM i, SAP HANA, others | `odbc-api` |

## Normalization

Normalization is a pure function. Same input, same output, plus warnings per value (for example a tax id with a wrong check digit). Types in v0.1 are `tax_id`, `company_name`, `person_name`, `address`, `phone`, `email`, `iban`, `amount`, `date`, `code` and `text`. Some types produce derived attributes, for example `address` produces `street`, `number`, `postcode`, `city` and `province`.

Country specific logic lives in packs (see the `fita-packs` repository). Core functions are Rust. External pack functions run as WebAssembly in wasmtime with no network, no disk and limits on memory and time.

## Matching

1. **Blocking.** Only records sharing a blocking key are compared. Pairs are deduplicated as two u64 in a hash set. Blocks over 500 records are not expanded, they are logged and split with the next blocking rule.
2. **Comparison.** Each pair gets a comparison vector with a level per field (exact, very close, close, different, empty). Cut points depend on the attribute type. Tax ids and phones only allow exact, different or empty.
3. **Decision.** Deterministic rules first. Then the Fellegi-Sunter model. `u` is estimated from random pairs, `m` with EM, and re-estimated from manual decisions once there are about 200. Parameters are stored with every run so any decision can be explained.
4. **Clustering.** Union-find over edges sorted by weight, skipping any union that would join two records with a cannot-link constraint (a `never_merge` rule, two valid and different tax ids, or a manual split). Manual merges go first as infinite weight edges.

**Stable ids.** Each new cluster inherits the id of the previous golden record it shares most members with. On a split the largest part keeps the id. On a merge the oldest id survives and the other goes to `master_redirect`. Retired ids are never reused.

**Accuracy.** Each run holds out 20% of manual decisions, compares the engine against them and stores precision and recall in `run`. If precision drops more than two points, the run waits for confirmation before publishing.

## Survivorship

Strategies per attribute, applied in order until one gives a value: `source(a, b, ...)` priority, `most_recent`, `most_frequent`, `valid`, `longest`, `all` for multi value attributes, and `pinned` for values fixed by a person. Every value stores the source record it came from and the strategy that chose it.

## API and MCP

All routes live under `/v1`. Bearer API keys for applications, secure session cookies for the UI. Cursor pagination, errors in RFC 9457 format, rate limits per key, OpenAPI generated with utoipa at `/v1/openapi.json`. The UI uses only the public API.

`POST /v1/{domain}/match` answers "does this already exist" in under 100 ms using the in-memory blocking index and the same rules and model as the engine.

Webhooks are signed with HMAC-SHA256 in a `Fita-Signature` header, delivered at least once with exponential retries for 24 hours.

The MCP server (`fita-mcp`, built on `rmcp`) exposes read tools: `search_records`, `get_record`, `get_source_ids`, `get_history`, `list_issues`, `check_tax_id`. It never performs actions with side effects. The only write is `propose_correction`, which always lands in the review queue.

## Security

- TLS required except in local development mode.
- Passwords with Argon2id. API keys are 256 random bits, shown once, stored as a hash.
- Four base roles per domain (admin, rule editor, reviewer, reader). Checks run in one shared layer used by API, MCP and UI.
- Attributes marked `personal: true` never appear in logs, metrics or traces.
- `audit_event` is append only and hash chained. `fita audit verify` detects tampering.
- `fita erase` removes or anonymizes a person across every table and cache.
- Dependencies checked with cargo-deny and cargo-audit on every pull request.
