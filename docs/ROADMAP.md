# Roadmap

Work is grouped in three versions. Each item is a unit that fits in one pull request or a short series. Check an item only when it is merged with tests and docs.

## v0.1, a working engine for customers and suppliers in Spain

Goal: run end to end on real sized data for one domain and reach at least 92% precision on the benchmark.

### Foundations
- [x] Cargo workspace with `fita-core`, `fita-config`, `fita-connect`, `fita-normalize`, `fita-match`, `fita-survive`, `fita-store`, `fita-engine`, `fita-cli`
- [ ] CI with fmt, clippy, tests and cargo-deny
- [ ] ADRs 0001 to 0005
- [ ] PostgreSQL schema and sqlx migrations, temporal tables with exclusion constraints
- [x] In-memory store implementing the storage traits, for tests
- [ ] Job queue with `SKIP LOCKED`, heartbeats and idempotent retries
- [ ] Process roles `api`, `worker` and `scheduler`

### Configuration and rules
- [ ] Project loader with JSON Schema validation and errors with line and column
- [ ] `fita-expr` parser and evaluator for rule conditions
- [ ] Published versions stored in `config_revision` with the git commit

### Ingestion
- [ ] CSV connector with encoding and separator detection
- [ ] Excel connector with sheet and header row selection
- [ ] Airbyte protocol runner for Docker and Podman
- [ ] Native PostgreSQL, Oracle, SQL Server and MySQL connectors (table, view or SQL query)
- [ ] Read only permission script for each database engine
- [ ] Full and incremental loads, change detection by hash, soft deletes
- [ ] Column mapping suggestion and 500 row test run
- [ ] Schema drift detection that puts the source in `failing`

### Normalization
- [ ] Attribute types `tax_id`, `company_name`, `person_name`, `address`, `phone`, `email`, `iban`, `amount`, `date`, `code`, `text`
- [ ] Spanish NIF, NIE and CIF check digit validation
- [ ] Company name cleaning with legal form extraction
- [ ] Address parsing with street type dictionary and postcode validation
- [ ] Golden and property tests for every normalizer

### Matching and survivorship
- [x] Exact match and plain union-find (first end to end path)
- [ ] Blocking with size limit, warnings and split
- [ ] Similarity functions and comparison levels per type
- [ ] Ordered deterministic rules with `never_merge`
- [ ] Fellegi-Sunter model with `u` from random pairs and EM for `m`
- [ ] Constrained clustering with cannot-link
- [ ] Stable ids and `master_redirect`
- [ ] All survivorship strategies and pinned values
- [ ] History and undo on temporal tables
- [ ] Rule simulation against the published version

### Review, API and UI
- [ ] Review queue with priority, claims, actions and immediate recompute
- [ ] REST routes for records, members, history, match, lookup, review, sources, runs and simulate
- [ ] OpenAPI generation and API keys
- [ ] Migration projects, blockers and crossref export
- [ ] UI screens Records, Review, Sources, Rules, Migration and login
- [ ] CLI `init`, `validate`, `simulate`, `run`, `serve`, `sources test`, `users`, `keys`, `migrate db`, `backup`, `export`

### Platform
- [ ] Docker image and Linux binaries for x86_64 and aarch64
- [ ] Required TLS, four roles and hash chained audit log
- [ ] Synthetic benchmark of 50,000 records running on every pull request
- [ ] Integration tests with real PostgreSQL

## v1.0, ready for production in a mid sized company

### Sources and data
- [ ] MariaDB, IBM Db2 and IBM i, SAP HANA and generic ODBC connectors
- [ ] Change capture for PostgreSQL (logical replication) and SQL Server
- [ ] Read windows and connection limits per source
- [ ] Any connector from the Airbyte catalog
- [ ] WebAssembly packs
- [ ] Transactional sources for payment issue detectors
- [ ] Reference data catalogs and code crosswalks
- [ ] Optional geocoding

### Engine
- [ ] Incremental runs in minutes on 10 million records
- [ ] Accuracy measurement on every run and hold when precision drops
- [ ] Model re-estimation from manual decisions
- [ ] Rule suggestions from repeated decisions

### Issues and fiscal
- [ ] Issue detectors with visible impact calculation
- [ ] Tax id checks against AEAT and VIES with cache
- [ ] Readiness indicators
- [ ] `fita diagnose` with PDF and JSON report

### Output
- [ ] Signed webhooks with retries
- [ ] Export to views, Parquet and Airbyte destinations
- [ ] MCP server (stdio and HTTP) with read tools
- [ ] Notifications by email, Teams and Slack
- [ ] Excel export and import of corrections

### UI, SDK and platform
- [ ] Issues, Access and Settings screens
- [ ] UI in English and Spanish, live updates with SSE, WCAG 2.2 AA
- [ ] Python SDK wheels for Linux, macOS and Windows
- [ ] OpenID Connect and SAML login
- [ ] Helm chart, metrics, traces and dashboards
- [ ] Zero downtime upgrades and `fita erase`
- [ ] SBOM, signed images and vulnerability policy

## v2.0, the complete MDM

- [ ] Configurable relationships, cycle free hierarchies and temporal relationships
- [ ] Materials and products domain with units and equivalences
- [ ] Portugal, France and Germany packs
- [ ] Change capture for Oracle, MySQL, MariaDB and IBM i
- [ ] Field level permissions and masking for users and consumers
- [ ] Multi level approvals and segregation of duties
- [ ] Field level encryption with the customer key manager
- [ ] SCIM provisioning and audit export to SIEM
- [ ] Approved writeback to Salesforce, SAP and Dynamics with conflict control
- [ ] `propose_correction` MCP tool
- [ ] Load files for SAP Migration Cockpit
- [ ] Hierarchies and Quality screens
- [ ] 10 million record benchmark and sizing guide
