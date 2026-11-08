# 0001. Rust for the engine, API and CLI

Status: accepted

## Context
Fita runs matching over millions of records inside customer networks, often on modest machines. It must ship as one binary that is easy to install and hard to break.

## Decision
The engine, API, MCP server and CLI are written in Rust in one Cargo workspace. The UI is TypeScript. The Python SDK wraps the Rust engine with PyO3 and maturin.

## Consequences
Predictable memory and speed, a single static binary and access to Arrow and DataFusion. Contributors need Rust. Python users get the same engine through the SDK instead of a reimplementation.
