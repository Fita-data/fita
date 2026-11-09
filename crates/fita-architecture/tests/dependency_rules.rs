//! Dependency rules from docs/ARCHITECTURE.md, checked on every `cargo test`.
//!
//! Only normal dependencies count. Dev dependencies are free, so tests can use
//! whatever they need.

use std::collections::{BTreeMap, BTreeSet};

use cargo_metadata::{DependencyKind, MetadataCommand};

/// Crates that hold domain logic. They stay pure: no I/O frameworks.
const DOMAIN: &[&str] = &[
    "fita-config",
    "fita-expr",
    "fita-normalize",
    "fita-match",
    "fita-survive",
    "fita-issues",
];

/// External crates a domain crate must never use.
const IO_CRATES: &[&str] = &[
    "sqlx",
    "axum",
    "tokio",
    "tokio-postgres",
    "reqwest",
    "hyper",
    "tiberius",
    "mysql_async",
    "oracle",
    "odbc-api",
];

/// Crates that sit on top and nothing else may depend on.
const TOP: &[&str] = &["fita-cli", "fita-api", "fita-mcp"];

/// Workspace crate name -> its normal dependencies (workspace and external).
fn graph() -> BTreeMap<String, BTreeSet<String>> {
    let manifest = concat!(env!("CARGO_MANIFEST_DIR"), "/../../Cargo.toml");
    let metadata = MetadataCommand::new()
        .manifest_path(manifest)
        .no_deps()
        .exec()
        .expect("cargo metadata runs");
    metadata
        .workspace_packages()
        .into_iter()
        .map(|p| {
            let deps = p
                .dependencies
                .iter()
                .filter(|d| d.kind == DependencyKind::Normal)
                .map(|d| d.name.clone())
                .collect();
            (p.name.clone(), deps)
        })
        .collect()
}

fn workspace_deps<'a>(
    graph: &'a BTreeMap<String, BTreeSet<String>>,
    name: &str,
) -> impl Iterator<Item = &'a String> {
    graph
        .get(name)
        .into_iter()
        .flatten()
        .filter(|d| graph.contains_key(d.as_str()))
}

#[test]
fn core_depends_on_nothing_in_the_workspace() {
    let g = graph();
    let deps: Vec<_> = workspace_deps(&g, "fita-core").collect();
    assert!(deps.is_empty(), "fita-core depends on {deps:?}");
}

#[test]
fn domain_crates_only_use_core_and_each_other() {
    let g = graph();
    let mut problems = Vec::new();
    for name in DOMAIN.iter().filter(|n| g.contains_key(**n)) {
        for dep in workspace_deps(&g, name) {
            if dep != "fita-core" && !DOMAIN.contains(&dep.as_str()) {
                problems.push(format!("{name} -> {dep}"));
            }
        }
    }
    assert!(
        problems.is_empty(),
        "domain crates reach outside the domain: {problems:?}"
    );
}

#[test]
fn domain_crates_do_no_io() {
    let g = graph();
    let mut problems = Vec::new();
    for name in DOMAIN.iter().filter(|n| g.contains_key(**n)) {
        for dep in &g[*name] {
            if IO_CRATES.contains(&dep.as_str()) {
                problems.push(format!("{name} -> {dep}"));
            }
        }
    }
    assert!(
        problems.is_empty(),
        "domain crates use I/O crates: {problems:?}"
    );
}

#[test]
fn store_only_uses_core() {
    let g = graph();
    let deps: Vec<_> = workspace_deps(&g, "fita-store")
        .filter(|d| *d != "fita-core")
        .collect();
    assert!(deps.is_empty(), "fita-store depends on {deps:?}");
}

#[test]
fn nothing_depends_on_the_top_layer() {
    let g = graph();
    let mut problems = Vec::new();
    for (name, deps) in &g {
        for dep in deps {
            if TOP.contains(&dep.as_str()) {
                problems.push(format!("{name} -> {dep}"));
            }
        }
    }
    assert!(
        problems.is_empty(),
        "crates depend on the top layer: {problems:?}"
    );
}

#[test]
fn only_the_top_layer_uses_the_engine() {
    let g = graph();
    let mut problems = Vec::new();
    for (name, deps) in &g {
        if deps.contains("fita-engine") && !TOP.contains(&name.as_str()) {
            problems.push(name.clone());
        }
    }
    assert!(
        problems.is_empty(),
        "only cli, api and mcp may use fita-engine: {problems:?}"
    );
}

#[test]
fn every_crate_is_known() {
    // A new crate must be placed in a layer here and in docs/ARCHITECTURE.md.
    let known: BTreeSet<&str> = DOMAIN
        .iter()
        .chain(TOP)
        .chain(&[
            "fita-core",
            "fita-store",
            "fita-connect",
            "fita-engine",
            "fita-architecture",
        ])
        .copied()
        .collect();
    let g = graph();
    let unknown: Vec<_> = g.keys().filter(|n| !known.contains(n.as_str())).collect();
    assert!(
        unknown.is_empty(),
        "add these crates to a layer: {unknown:?}"
    );
}
