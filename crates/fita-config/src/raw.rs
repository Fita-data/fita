//! File formats exactly as written by users, before validation.

use std::collections::BTreeMap;

use serde::Deserialize;
use serde_yaml::Value;

/// `fita.yaml`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawSettings {
    pub name: String,
    /// Documented but not supported yet.
    pub packs: Option<Value>,
    /// Documented but not supported yet.
    pub working_store: Option<Value>,
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
    pub schedule: Option<Value>,
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
    pub survivorship: BTreeMap<String, Vec<Value>>,
    /// Documented but not supported yet.
    pub blocking: Option<Value>,
    /// Documented but not supported yet.
    pub model: Option<Value>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawRule {
    pub name: String,
    pub when: Vec<Value>,
    pub confidence: Option<f64>,
    pub action: Option<String>,
}
