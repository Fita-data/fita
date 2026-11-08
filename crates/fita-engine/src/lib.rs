//! Runs a domain of a project end to end: read, normalize, match, survive and
//! store.
//!
//! Every run is a full run. It replaces whatever the store held for the
//! domain. Golden record ids are numbered from 1 on each run, so they are not
//! stable between runs yet.

use std::collections::BTreeMap;
use std::path::Path;

use fita_config::{Connector, Domain, LoadError, Project, Source};
use fita_connect::{read_csv, ConnectError, CsvOptions};
use fita_core::{
    IdError, MasterId, MasterRecord, NormalizedRecord, SourceRecord, Store, StoreError,
};
use fita_store::FileStore;
use serde::Serialize;

/// Error returned by a run.
#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    /// The project could not be loaded.
    #[error(transparent)]
    Config(#[from] LoadError),
    /// The project has no domain with that name.
    #[error("the project has no domain called {0:?}")]
    UnknownDomain(String),
    /// A source could not be read.
    #[error("source {source_name:?}: {error}")]
    Connect {
        /// Name of the source.
        source_name: String,
        /// What went wrong.
        error: ConnectError,
    },
    /// A golden record id could not be built.
    #[error(transparent)]
    Id(#[from] IdError),
    /// The store failed.
    #[error(transparent)]
    Store(#[from] StoreError),
}

/// Counters of a finished run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RunSummary {
    /// Domain that was run.
    pub domain: String,
    /// Number of sources read.
    pub sources: usize,
    /// Number of records read from every source.
    pub source_records: usize,
    /// Number of match rules applied.
    pub rules: usize,
    /// Number of links between records found by the rules.
    pub links: usize,
    /// Number of golden records written.
    pub master_records: usize,
    /// Golden records built from more than one source record.
    pub merged_master_records: usize,
    /// Project warnings, such as rules skipped because they are not supported yet.
    pub warnings: Vec<String>,
}

/// Loads the project in `project_dir`, runs `domain` and saves the result as
/// JSON under `<project_dir>/.fita/`.
pub fn run(project_dir: &Path, domain: &str) -> Result<RunSummary, EngineError> {
    let project = fita_config::load(project_dir)?;
    let mut store = FileStore::new(project_dir);
    run_with_store(&project, domain, &mut store)
}

/// Runs `domain` of an already loaded project and saves the result in `store`.
pub fn run_with_store(
    project: &Project,
    domain_name: &str,
    store: &mut dyn Store,
) -> Result<RunSummary, EngineError> {
    let domain = project
        .domain(domain_name)
        .ok_or_else(|| EngineError::UnknownDomain(domain_name.to_string()))?;
    let sources = project.sources_of(domain_name);
    let rule_set = project.rules_of(domain_name);
    let rules = rule_set.map(|set| set.rules.as_slice()).unwrap_or(&[]);
    let no_strategies = BTreeMap::new();
    let strategies = rule_set.map_or(&no_strategies, |set| &set.survivorship);

    let source_records = read_sources(&sources)?;

    let types = domain.attribute_types();
    let normalized: Vec<NormalizedRecord> = source_records
        .iter()
        .map(|record| fita_normalize::normalize_record(record, &types))
        .collect();

    let clusters = fita_match::cluster(&normalized, rules);

    let attributes: Vec<String> = domain.attributes.iter().map(|a| a.name.clone()).collect();
    let mut masters = Vec::with_capacity(clusters.len());
    let mut links = 0;
    let mut merged = 0;
    for (position, cluster) in clusters.into_iter().enumerate() {
        let members: Vec<&NormalizedRecord> = cluster
            .members
            .iter()
            .map(|&index| &normalized[index])
            .collect();
        if members.len() > 1 {
            merged += 1;
        }
        links += cluster.links.len();
        masters.push(MasterRecord {
            id: master_id(domain, position)?,
            domain: domain.name.clone(),
            members: members.iter().map(|record| record.key.clone()).collect(),
            links: cluster.links,
            values: fita_survive::survive(&members, &attributes, strategies),
        });
    }

    store.replace_source_records(domain_name, &source_records)?;
    store.replace_normalized_records(domain_name, &normalized)?;
    store.replace_master_records(domain_name, &masters)?;

    Ok(RunSummary {
        domain: domain.name.clone(),
        sources: sources.len(),
        source_records: source_records.len(),
        rules: rules.len(),
        links,
        master_records: masters.len(),
        merged_master_records: merged,
        warnings: project.warnings.iter().map(ToString::to_string).collect(),
    })
}

/// Reads every source in the order given, which is sorted by name.
fn read_sources(sources: &[&Source]) -> Result<Vec<SourceRecord>, EngineError> {
    let mut records = Vec::new();
    for source in sources {
        let Connector::Csv { path, separator } = &source.connector;
        let options = CsvOptions {
            path: path.clone(),
            separator: *separator,
            key_column: source.key.clone(),
            modified_at_column: source.modified_at.clone(),
            mapping: source.mapping.clone(),
        };
        let read = read_csv(&source.name, &options).map_err(|error| EngineError::Connect {
            source_name: source.name.clone(),
            error,
        })?;
        records.extend(read);
    }
    Ok(records)
}

/// Id of the golden record at `position` (starting at 0) in this run.
fn master_id(domain: &Domain, position: usize) -> Result<MasterId, IdError> {
    let number = u64::try_from(position).map_or(u64::MAX, |n| n.saturating_add(1));
    MasterId::new(&domain.prefix, number)
}

#[cfg(test)]
mod tests;
