//! File formats exactly as written by users, before validation.

use std::collections::BTreeMap;

use serde::de::IgnoredAny;
use serde::Deserialize;

/// `fita.yaml`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawSettings {
    pub name: String,
    /// Documented but not supported yet.
    pub packs: Option<IgnoredAny>,
    /// Documented but not supported yet.
    pub working_store: Option<IgnoredAny>,
}

/// `domains/*.yaml`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawDomain {
    pub name: String,
    pub prefix: String,
    pub attributes: Vec<RawAttribute>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawAttribute {
    pub name: String,
    #[serde(rename = "type")]
    pub attribute_type: String,
    #[serde(default)]
    pub personal: bool,
}

/// `sources/*.yaml`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawSource {
    pub name: String,
    pub domain: String,
    pub connector: RawConnector,
    pub key: String,
    pub modified_at: Option<String>,
    pub mapping: BTreeMap<String, String>,
    /// Documented but not supported yet.
    pub schedule: Option<IgnoredAny>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawConnector {
    #[serde(rename = "type")]
    pub kind: String,
    pub path: Option<String>,
    pub separator: Option<String>,
}

/// `rules/*.yaml`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawRules {
    pub domain: String,
    pub version: Option<u64>,
    #[serde(default)]
    pub rules: Vec<RawRule>,
    #[serde(default)]
    pub survivorship: BTreeMap<String, Vec<RawEntry>>,
    /// Documented but not supported yet.
    pub blocking: Option<IgnoredAny>,
    /// Documented but not supported yet.
    pub model: Option<IgnoredAny>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawRule {
    pub name: String,
    pub when: Vec<RawEntry>,
    pub confidence: Option<f64>,
    pub action: Option<String>,
}

/// One entry of a `when` list or of a survivorship list.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub(crate) enum RawEntry {
    /// A text such as `equal(tax_id)` or `most_recent`.
    Text(String),
    /// An `any: [...]` block. Not supported yet, so its content is not kept.
    Any {
        #[allow(dead_code)]
        any: IgnoredAny,
    },
    /// Anything else, which is always an error.
    Other(IgnoredAny),
}
