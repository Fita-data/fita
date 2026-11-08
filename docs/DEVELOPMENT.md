# Development guide

How to work in this repository. If you are new, read [ARCHITECTURE.md](ARCHITECTURE.md) first, then this file, then pick an item from [ROADMAP.md](ROADMAP.md).

## Setup

- Rust stable (see `rust-toolchain.toml`)
- PostgreSQL 15 or newer with the `btree_gist` extension, or Docker to run `deploy/docker/docker-compose.yml`
- Node 20+ for `ui/`
- Python 3.10+ and maturin for `python/`

```sh
cargo build
cargo test --workspace
docker compose -f deploy/docker/docker-compose.yml up -d   # local PostgreSQL with dummy credentials
cargo test --workspace -- --ignored                         # integration tests that need PostgreSQL
```

## Before every commit

```sh
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo deny check
```

All four must pass. CI runs the same commands.

## Where things go

| You are adding | Put it in |
| --- | --- |
| A new domain type, id or error | `fita-core` |
| A new config key | `fita-config`, plus the JSON Schema and a test with an invalid file |
| A new rule function | `fita-expr`, plus docs in the rule reference |
| A new attribute type or normalizer | `fita-normalize` (generic) or a pack in `fita-packs` (country specific) |
| A new database connector | `fita-connect/src/db/` |
| A new connector for an application | usually `fita-connectors`, not here |
| A change to matching or clustering | `fita-match`, with a benchmark run |
| A schema change | a new file in `migrations/`, never an edit to an existing one |
| A new endpoint | `fita-api`, with utoipa annotations so it appears in OpenAPI |
| A new MCP tool | `fita-mcp`, reusing the same service as the API |
| A new CLI command | `fita-cli`, with `--help` examples and `--output json` support |
| A decision that future contributors should know | a new ADR in `docs/adr/` |

## Code conventions

**Rust**

- Edition 2021. `rustfmt` defaults. Clippy with `-D warnings`.
- Errors with `thiserror` in libraries and `anyhow` only in `fita-cli`. No `unwrap()` or `expect()` outside tests, except for invariants that are proven right above and commented.
- No `unsafe` unless an ADR explains why.
- Public items have a doc comment that says what they do and when to use them.
- Async only where there is I/O. Domain crates are synchronous and use rayon for parallel work.
- Keep functions small and names explicit. Prefer a clear loop to a clever iterator chain.
- New dependencies must be Apache-2.0, MIT, BSD or similar. No GPL, AGPL or LGPL. `cargo deny` enforces this.

**SQL**

- Lowercase keywords, snake_case names, everything in the `fita` schema.
- Every table that changes over time uses a `valid tstzrange` column and an exclusion constraint, not an `updated_at` overwrite.

**TypeScript (ui/)**

- React with function components, TanStack Query for data and TanStack Table for tables.
- The API client is generated from OpenAPI. Do not hand write fetch calls.
- ESLint and Prettier. Every screen works with the keyboard alone.

**Python (python/)**

- ruff for lint and format. Type hints on public functions.

## Tests

- Unit tests live next to the code (`#[cfg(test)] mod tests`).
- Normalizers and validators need golden tests (input and expected output in a CSV under `tests/golden/`) and property tests with proptest, for example that normalizing twice equals normalizing once.
- Integration tests that need PostgreSQL are marked `#[ignore]` and run in CI with a real database.
- `fita-match`, `fita-survive` and `fita-expr` must stay above 90% coverage.
- A pull request that touches matching must include the benchmark result before and after. Precision may not drop more than half a point.

## Data rules

- Test data is synthetic. Never commit real company names, tax ids, emails, phones or addresses from any client or system.
- Fixtures that look real must be generated. Use made up names and valid but fictitious tax ids.
- No credentials anywhere. Use `${ENV_VAR}` references and `.env.example` with dummy values.

## Things not to do

- Do not edit a released migration.
- Do not add a dependency from a domain crate to `sqlx`, `axum`, `tokio-postgres` or `reqwest`.
- Do not let the MCP server or a webhook trigger a write to a source.
- Do not log the value of an attribute marked `personal`.
- Do not merge two records with a cannot-link constraint, even if the score is 1.0.
- Do not reuse a retired public id.
- Do not change behaviour and refactor in the same pull request.

## Working on a roadmap item

1. Pick an unchecked item in [ROADMAP.md](ROADMAP.md) and open an issue if there is none.
2. Create a short branch from `main`, for example `nif-check-digit`.
3. Write the test first when the behaviour is clear (validators, parsers, clustering cases).
4. Keep the change focused on that item. If you find something else, open another issue.
5. Update the docs that describe the behaviour you changed.
6. Open a pull request using the template in [CONTRIBUTING.md](../CONTRIBUTING.md).
