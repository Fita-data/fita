# 0002. PostgreSQL as system of record, Parquet as working store

Status: accepted

## Context
We need durable, queryable state with history and strong constraints, and also fast batch computation over large domains.

## Decision
PostgreSQL holds all state, history (temporal tables with exclusion constraints) and the job queue. Heavy matching exports a domain to Parquet and runs in memory with Arrow and DataFusion. Parquet files are disposable.

## Consequences
One external dependency to operate. No extra queue or cache service. Constraints in the database protect invariants even if the code has a bug. Requires the `btree_gist` extension.
