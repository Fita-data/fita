//! Finds records that describe the same entity and groups them.
//!
//! For now only exact rules are supported: a rule matches two records when
//! every attribute it lists has the same non empty normalized value. Records
//! are grouped by those values, and union-find joins the groups of every rule
//! into clusters.

mod union_find;

use std::collections::HashMap;

use fita_config::MatchRule;
use fita_core::{MatchLink, NormalizedRecord};

pub use union_find::UnionFind;

/// A group of records that describe the same entity.
#[derive(Debug, Clone, PartialEq)]
pub struct Cluster {
    /// Positions of the records in the input slice, in ascending order.
    pub members: Vec<usize>,
    /// Links that joined the members, one less than the number of members.
    pub links: Vec<MatchLink>,
}

/// Groups `records` into clusters with exact match `rules`.
///
/// Rules are applied in order, so when two records match by several rules the
/// first one is recorded as the reason. Every record ends up in exactly one
/// cluster, alone if nothing matched it. Clusters come out ordered by their
/// first member.
pub fn cluster(records: &[NormalizedRecord], rules: &[MatchRule]) -> Vec<Cluster> {
    let mut sets = UnionFind::new(records.len());
    let mut links: Vec<(usize, MatchLink)> = Vec::new();

    for rule in rules {
        // First record seen for each combination of values.
        let mut anchors: HashMap<Vec<&str>, usize> = HashMap::new();
        for (index, record) in records.iter().enumerate() {
            let Some(values) = rule_values(record, rule) else {
                continue;
            };
            let Some(&anchor) = anchors.get(&values) else {
                anchors.insert(values, index);
                continue;
            };
            if sets.union(anchor, index) {
                let link = MatchLink {
                    left: records[anchor].key.clone(),
                    right: record.key.clone(),
                    rule: rule.name.clone(),
                    confidence: rule.confidence,
                };
                links.push((anchor, link));
            }
        }
    }

    let mut cluster_of_root: HashMap<usize, usize> = HashMap::new();
    let mut clusters: Vec<Cluster> = Vec::new();
    for index in 0..records.len() {
        let root = sets.find(index);
        let position = *cluster_of_root.entry(root).or_insert_with(|| {
            clusters.push(Cluster {
                members: Vec::new(),
                links: Vec::new(),
            });
            clusters.len() - 1
        });
        clusters[position].members.push(index);
    }
    for (anchor, link) in links {
        let position = cluster_of_root[&sets.find(anchor)];
        clusters[position].links.push(link);
    }
    clusters
}

/// Values of the rule's attributes in `record`, or `None` if any is empty.
fn rule_values<'a>(record: &'a NormalizedRecord, rule: &MatchRule) -> Option<Vec<&'a str>> {
    if rule.attributes.is_empty() {
        return None;
    }
    let mut values = Vec::with_capacity(rule.attributes.len());
    for attribute in &rule.attributes {
        values.push(record.value(attribute)?);
    }
    Some(values)
}

#[cfg(test)]
mod tests {
    use super::*;
    use fita_core::RecordKey;
    use std::collections::BTreeMap;

    fn record(id: &str, values: &[(&str, &str)]) -> NormalizedRecord {
        NormalizedRecord {
            key: RecordKey::new("test", id),
            values: values
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect::<BTreeMap<_, _>>(),
            modified_at: None,
        }
    }

    fn rule(name: &str, attributes: &[&str]) -> MatchRule {
        MatchRule {
            name: name.into(),
            attributes: attributes.iter().map(|a| a.to_string()).collect(),
            confidence: 0.9,
        }
    }

    fn members(clusters: &[Cluster]) -> Vec<Vec<usize>> {
        clusters.iter().map(|c| c.members.clone()).collect()
    }

    #[test]
    fn groups_records_with_equal_values() {
        let records = vec![
            record("1", &[("tax_id", "B00000001")]),
            record("2", &[("tax_id", "B00000002")]),
            record("3", &[("tax_id", "B00000001")]),
        ];
        let clusters = cluster(&records, &[rule("same_tax_id", &["tax_id"])]);
        assert_eq!(members(&clusters), vec![vec![0, 2], vec![1]]);
        assert_eq!(
            clusters[0].links,
            vec![MatchLink {
                left: RecordKey::new("test", "1"),
                right: RecordKey::new("test", "3"),
                rule: "same_tax_id".into(),
                confidence: 0.9,
            }]
        );
        assert!(clusters[1].links.is_empty());
    }

    #[test]
    fn empty_values_never_match() {
        let records = vec![record("1", &[]), record("2", &[])];
        let clusters = cluster(&records, &[rule("same_tax_id", &["tax_id"])]);
        assert_eq!(members(&clusters), vec![vec![0], vec![1]]);
    }

    #[test]
    fn every_attribute_of_a_rule_must_be_equal() {
        let records = vec![
            record("1", &[("tax_id", "A"), ("postcode", "28001")]),
            record("2", &[("tax_id", "A"), ("postcode", "08001")]),
            record("3", &[("tax_id", "A"), ("postcode", "28001")]),
            record("4", &[("tax_id", "A")]),
        ];
        let clusters = cluster(&records, &[rule("r", &["tax_id", "postcode"])]);
        assert_eq!(members(&clusters), vec![vec![0, 2], vec![1], vec![3]]);
    }

    #[test]
    fn rules_chain_transitively() {
        // 1~2 by tax id, 2~3 by email: all three end up together.
        let records = vec![
            record("1", &[("tax_id", "A")]),
            record("2", &[("tax_id", "A"), ("email", "x@example.com")]),
            record("3", &[("email", "x@example.com")]),
            record("4", &[("email", "y@example.com")]),
        ];
        let rules = [
            rule("same_tax_id", &["tax_id"]),
            rule("same_email", &["email"]),
        ];
        let clusters = cluster(&records, &rules);
        assert_eq!(members(&clusters), vec![vec![0, 1, 2], vec![3]]);
        let reasons: Vec<&str> = clusters[0].links.iter().map(|l| l.rule.as_str()).collect();
        assert_eq!(reasons, ["same_tax_id", "same_email"]);
    }

    #[test]
    fn first_rule_gets_the_credit() {
        let records = vec![
            record("1", &[("tax_id", "A"), ("email", "x@example.com")]),
            record("2", &[("tax_id", "A"), ("email", "x@example.com")]),
        ];
        let rules = [
            rule("same_email", &["email"]),
            rule("same_tax_id", &["tax_id"]),
        ];
        let clusters = cluster(&records, &rules);
        assert_eq!(clusters[0].links.len(), 1);
        assert_eq!(clusters[0].links[0].rule, "same_email");
    }

    #[test]
    fn no_rules_means_one_cluster_per_record() {
        let records = vec![
            record("1", &[("tax_id", "A")]),
            record("2", &[("tax_id", "A")]),
        ];
        let clusters = cluster(&records, &[]);
        assert_eq!(members(&clusters), vec![vec![0], vec![1]]);
        assert!(cluster(&[], &[rule("r", &["tax_id"])]).is_empty());
    }

    #[test]
    fn links_are_one_less_than_members() {
        let records: Vec<_> = (0..6)
            .map(|i| record(&i.to_string(), &[("code", "same")]))
            .collect();
        let clusters = cluster(&records, &[rule("r", &["code"])]);
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].links.len(), 5);
    }
}
