use std::collections::BTreeMap;
use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::MasterId;

/// Identifies one record in one source: the source name plus the record id
/// the source system uses.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RecordKey {
    /// Name of the source, as configured in `sources/`.
    pub source: String,
    /// Id of the record inside the source.
    pub id: String,
}

impl RecordKey {
    /// Builds a key from a source name and a record id.
    pub fn new(source: impl Into<String>, id: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            id: id.into(),
        }
    }
}

impl fmt::Display for RecordKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.source, self.id)
    }
}

/// A record as read from a source, with raw values already mapped to domain
/// attributes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceRecord {
    /// Where the record comes from.
    pub key: RecordKey,
    /// Raw value per attribute, exactly as read.
    pub values: BTreeMap<String, String>,
    /// Last modification time reported by the source, if it has one.
    pub modified_at: Option<DateTime<Utc>>,
}

/// A source record after normalization. Empty values are left out.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedRecord {
    /// Where the record comes from.
    pub key: RecordKey,
    /// Normalized value per attribute. Attributes that normalize to an empty
    /// string are not present.
    pub values: BTreeMap<String, String>,
    /// Last modification time reported by the source, if it has one.
    pub modified_at: Option<DateTime<Utc>>,
}

impl NormalizedRecord {
    /// Returns the normalized value of `attribute`, or `None` if it is empty.
    pub fn value(&self, attribute: &str) -> Option<&str> {
        self.values.get(attribute).map(String::as_str)
    }
}

/// How a survivorship value was chosen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Strategy {
    /// Priority by source: the first listed source with a value wins.
    Source(Vec<String>),
    /// The value from the record with the latest modification time wins.
    MostRecent,
    /// No configured strategy gave a value, so the first non empty value of
    /// the cluster was taken. Not configurable, it is always the last resort.
    FirstNonEmpty,
}

/// The value chosen for one attribute of a golden record, with its provenance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChosenValue {
    /// The chosen normalized value.
    pub value: String,
    /// The record the value comes from. Its `source` is the source of the value.
    pub record: RecordKey,
    /// The strategy that chose it.
    pub strategy: Strategy,
}

impl ChosenValue {
    /// Name of the source the value comes from.
    pub fn source(&self) -> &str {
        &self.record.source
    }
}

/// Why two records ended up in the same golden record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MatchLink {
    /// One of the two records.
    pub left: RecordKey,
    /// The other record.
    pub right: RecordKey,
    /// Name of the rule that matched them.
    pub rule: String,
    /// Confidence configured for that rule, between 0 and 1.
    pub confidence: f64,
}

/// A golden record: one entity built from one or more source records.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MasterRecord {
    /// Public id, such as `cus_00001`.
    pub id: MasterId,
    /// Domain the record belongs to.
    pub domain: String,
    /// Source records that form this golden record.
    pub members: Vec<RecordKey>,
    /// Links that joined the members. Empty when there is a single member.
    pub links: Vec<MatchLink>,
    /// Chosen value per attribute. Attributes without any value are left out.
    pub values: BTreeMap<String, ChosenValue>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_key_displays_source_and_id() {
        assert_eq!(RecordKey::new("billing", "42").to_string(), "billing/42");
    }

    #[test]
    fn normalized_value_lookup() {
        let record = NormalizedRecord {
            key: RecordKey::new("crm", "1"),
            values: BTreeMap::from([("email".to_string(), "ana@example.com".to_string())]),
            modified_at: None,
        };
        assert_eq!(record.value("email"), Some("ana@example.com"));
        assert_eq!(record.value("phone"), None);
    }

    #[test]
    fn strategy_serializes_in_snake_case() {
        let json = serde_json::to_string(&Strategy::Source(vec!["crm".into()])).unwrap();
        assert_eq!(json, r#"{"source":["crm"]}"#);
        assert_eq!(
            serde_json::to_string(&Strategy::MostRecent).unwrap(),
            r#""most_recent""#
        );
    }

    #[test]
    fn chosen_value_exposes_source() {
        let value = ChosenValue {
            value: "B00000001".into(),
            record: RecordKey::new("billing", "7"),
            strategy: Strategy::FirstNonEmpty,
        };
        assert_eq!(value.source(), "billing");
    }
}
