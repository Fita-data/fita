//! Validated project types.

use std::collections::BTreeMap;
use std::path::PathBuf;

use fita_core::Strategy;
use fita_normalize::AttributeType;

use crate::Issue;

/// Separator used by the CSV connector when none is configured.
pub const DEFAULT_CSV_SEPARATOR: u8 = b',';

/// A loaded and validated project.
#[derive(Debug, Clone)]
pub struct Project {
    /// Project folder.
    pub root: PathBuf,
    /// Project name from `fita.yaml`.
    pub name: String,
    /// Domains, sorted by name.
    pub domains: Vec<Domain>,
    /// Sources, sorted by name.
    pub sources: Vec<Source>,
    /// Matching and survivorship rules, at most one set per domain.
    pub rules: Vec<RuleSet>,
    /// Things that are valid but ignored for now, such as rule functions that
    /// are not supported yet.
    pub warnings: Vec<Issue>,
}

impl Project {
    /// Returns the domain called `name`.
    pub fn domain(&self, name: &str) -> Option<&Domain> {
        self.domains.iter().find(|domain| domain.name == name)
    }

    /// Returns the sources of `domain`, sorted by name.
    pub fn sources_of(&self, domain: &str) -> Vec<&Source> {
        self.sources
            .iter()
            .filter(|source| source.domain == domain)
            .collect()
    }

    /// Returns the rule set of `domain`, if it has one.
    pub fn rules_of(&self, domain: &str) -> Option<&RuleSet> {
        self.rules.iter().find(|rules| rules.domain == domain)
    }
}

/// A domain, such as customers or suppliers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Domain {
    /// Domain name.
    pub name: String,
    /// Prefix of the public ids of its golden records, such as `cus`.
    pub prefix: String,
    /// Attributes in the order they are declared.
    pub attributes: Vec<Attribute>,
}

impl Domain {
    /// Returns the attribute called `name`.
    pub fn attribute(&self, name: &str) -> Option<&Attribute> {
        self.attributes
            .iter()
            .find(|attribute| attribute.name == name)
    }

    /// Returns the type of every attribute, keyed by attribute name.
    pub fn attribute_types(&self) -> BTreeMap<String, AttributeType> {
        self.attributes
            .iter()
            .map(|attribute| (attribute.name.clone(), attribute.attribute_type))
            .collect()
    }
}

/// An attribute of a domain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attribute {
    /// Attribute name, chosen by the user.
    pub name: String,
    /// Type, which decides how values are normalized.
    pub attribute_type: AttributeType,
    /// Personal data that must never appear in logs.
    pub personal: bool,
}

/// A source of records for one domain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Source {
    /// Source name.
    pub name: String,
    /// Domain its records belong to.
    pub domain: String,
    /// How to read it.
    pub connector: Connector,
    /// Column with the id of each record in the source.
    pub key: String,
    /// Optional column with the last modification time of each record.
    pub modified_at: Option<String>,
    /// Source column for each attribute, keyed by attribute name.
    pub mapping: BTreeMap<String, String>,
}

/// How to read a source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Connector {
    /// A UTF-8 CSV file with a header row.
    Csv {
        /// File path, already resolved against the project folder.
        path: PathBuf,
        /// Field separator byte.
        separator: u8,
    },
}

/// Matching and survivorship rules of a domain.
#[derive(Debug, Clone, PartialEq)]
pub struct RuleSet {
    /// Domain the rules apply to.
    pub domain: String,
    /// Version number written by the user, if any.
    pub version: Option<u64>,
    /// Supported match rules, in the order they are declared.
    pub rules: Vec<MatchRule>,
    /// Supported survivorship strategies per attribute, in order.
    pub survivorship: BTreeMap<String, Vec<Strategy>>,
}

/// A match rule whose conditions are all `equal(attribute)`: two records
/// match when every listed attribute has the same non empty normalized value.
#[derive(Debug, Clone, PartialEq)]
pub struct MatchRule {
    /// Rule name.
    pub name: String,
    /// Attributes that must be equal.
    pub attributes: Vec<String>,
    /// Confidence between 0 and 1.
    pub confidence: f64,
}
