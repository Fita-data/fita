# Fita

Fita is an open source master data management engine. It reads customer and supplier records from the systems a company already has, finds the ones that describe the same entity, builds one golden record per entity with the source of every value, and serves that record to people, applications and AI agents.

It ships as a single binary written in Rust, stores its state in PostgreSQL and runs the same on a laptop as on a cluster. Configuration lives in YAML files kept in git. Nothing is merged without an explanation, and every automatic decision can be undone.

> Status: early development. The engine is being built in the open. Expect breaking changes until v1.0.

## What it does

- **Ingest.** Reads databases (PostgreSQL, Oracle, SQL Server, MySQL and more), files and applications through the Airbyte protocol. Read only. It never writes to a source without explicit approval.
- **Normalize.** Cleans names, tax IDs, addresses, phones and emails with country packs. The first pack is Spain.
- **Match.** Blocking, deterministic rules and a Fellegi-Sunter probabilistic model, then constrained clustering so that A~B and B~C never forces A=C when A and C are clearly different.
- **Survive.** Picks the best value for each attribute with per-attribute strategies and keeps full provenance and history.
- **Review.** Sends uncertain pairs to a review queue. Manual decisions always win over the engine.
- **Serve.** REST API, webhooks, exports, an MCP server for AI agents and a Python SDK.

## Repository layout

```
crates/        Rust workspace, one crate per responsibility
ui/            web interface (TypeScript, React, Vite), embedded in the binary
python/        Python SDK (PyO3 + maturin)
migrations/    PostgreSQL schema migrations (plain SQL, never edited once released)
examples/      sample projects, including the demo with synthetic data
deploy/        Dockerfile, docker-compose and Helm chart
docs/          architecture, development guide, roadmap and ADRs
```

Not every folder exists yet. See [docs/ROADMAP.md](docs/ROADMAP.md) for what is built and what is next.

## Quick start

```sh
cargo build --release
./target/release/fita init my-project --demo
./target/release/fita validate --project my-project
./target/release/fita run customers --project my-project
./target/release/fita export customers --project my-project --format csv
```

The demo uses synthetic data only.

## Documentation

- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) explains how the system is put together.
- [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md) explains how to work in this repository.
- [docs/ROADMAP.md](docs/ROADMAP.md) lists the work by version.
- [docs/adr](docs/adr) holds the architecture decisions.
- [CONTRIBUTING.md](CONTRIBUTING.md) covers pull requests and commits.

## Related repositories

- [fita-packs](https://github.com/Fita-data/fita-packs) country packs and issue detectors
- [fita-connectors](https://github.com/Fita-data/fita-connectors) connectors that Airbyte does not cover well
- [fita-dbt](https://github.com/Fita-data/fita-dbt) dbt package to join any table with the golden record
- [fita-docs](https://github.com/Fita-data/fita-docs) documentation site

## License

Apache License 2.0. See [LICENSE](LICENSE).
