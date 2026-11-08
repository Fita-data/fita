//! Picks the value of each attribute of a golden record and keeps where it
//! comes from.
//!
//! Strategies are tried in order until one gives a value. If none does, the
//! first non empty value among the members is taken. Supported strategies are
//! `source(a, b, ...)` and `most_recent`.

use std::collections::BTreeMap;

use fita_core::{ChosenValue, NormalizedRecord, Strategy};

/// Chooses a value for every attribute in `attributes` from the `members` of
/// one cluster.
///
/// `members` must be in a stable order, because ties and the fallback pick the
/// first one. `strategies` gives the configured strategies per attribute.
/// Attributes that are empty in every member are left out of the result.
pub fn survive(
    members: &[&NormalizedRecord],
    attributes: &[String],
    strategies: &BTreeMap<String, Vec<Strategy>>,
) -> BTreeMap<String, ChosenValue> {
    let mut chosen = BTreeMap::new();
    for attribute in attributes {
        let configured = strategies.get(attribute).map(Vec::as_slice).unwrap_or(&[]);
        if let Some(value) = choose(members, attribute, configured) {
            chosen.insert(attribute.clone(), value);
        }
    }
    chosen
}

/// Chooses the value of one attribute.
pub fn choose(
    members: &[&NormalizedRecord],
    attribute: &str,
    strategies: &[Strategy],
) -> Option<ChosenValue> {
    for strategy in strategies {
        let picked = match strategy {
            Strategy::Source(sources) => by_source(members, attribute, sources),
            Strategy::MostRecent => most_recent(members, attribute),
            Strategy::FirstNonEmpty => first_non_empty(members, attribute),
        };
        if let Some(record) = picked {
            return Some(chosen_value(record, attribute, strategy.clone()));
        }
    }
    first_non_empty(members, attribute)
        .map(|record| chosen_value(record, attribute, Strategy::FirstNonEmpty))
}

fn by_source<'a>(
    members: &[&'a NormalizedRecord],
    attribute: &str,
    sources: &[String],
) -> Option<&'a NormalizedRecord> {
    for source in sources {
        let found = members
            .iter()
            .find(|record| &record.key.source == source && record.value(attribute).is_some());
        if let Some(record) = found {
            return Some(record);
        }
    }
    None
}

/// The member with the latest modification time that has a value. Members
/// without a modification time are not considered. Ties go to the first.
fn most_recent<'a>(
    members: &[&'a NormalizedRecord],
    attribute: &str,
) -> Option<&'a NormalizedRecord> {
    let mut best: Option<&NormalizedRecord> = None;
    for record in members {
        if record.value(attribute).is_none() {
            continue;
        }
        let Some(modified_at) = record.modified_at else {
            continue;
        };
        let newer = match best.and_then(|best| best.modified_at) {
            Some(best_time) => modified_at > best_time,
            None => true,
        };
        if newer {
            best = Some(record);
        }
    }
    best
}

fn first_non_empty<'a>(
    members: &[&'a NormalizedRecord],
    attribute: &str,
) -> Option<&'a NormalizedRecord> {
    members
        .iter()
        .find(|record| record.value(attribute).is_some())
        .copied()
}

fn chosen_value(record: &NormalizedRecord, attribute: &str, strategy: Strategy) -> ChosenValue {
    ChosenValue {
        value: record.value(attribute).unwrap_or_default().to_string(),
        record: record.key.clone(),
        strategy,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, TimeZone, Utc};
    use fita_core::RecordKey;

    fn record(
        source: &str,
        id: &str,
        values: &[(&str, &str)],
        modified_day: Option<u32>,
    ) -> NormalizedRecord {
        NormalizedRecord {
            key: RecordKey::new(source, id),
            values: values
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            modified_at: modified_day.map(day),
        }
    }

    fn day(n: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 1, n, 0, 0, 0).unwrap()
    }

    fn source(names: &[&str]) -> Strategy {
        Strategy::Source(names.iter().map(|n| n.to_string()).collect())
    }

    #[test]
    fn source_priority_picks_the_first_listed_source_with_a_value() {
        let billing = record("billing", "1", &[("email", "a@example.com")], None);
        let crm_empty = record("crm", "7", &[], None);
        let erp = record("erp", "3", &[("email", "b@example.com")], None);
        let members = [&billing, &crm_empty, &erp];

        let value = choose(&members, "email", &[source(&["crm", "erp", "billing"])]).unwrap();
        assert_eq!(value.value, "b@example.com");
        assert_eq!(value.record, RecordKey::new("erp", "3"));
        assert_eq!(value.source(), "erp");
        assert_eq!(value.strategy, source(&["crm", "erp", "billing"]));
    }

    #[test]
    fn most_recent_picks_the_latest_dated_value() {
        let old = record("crm", "1", &[("phone", "600000001")], Some(1));
        let undated = record("crm", "2", &[("phone", "600000002")], None);
        let new = record("billing", "3", &[("phone", "600000003")], Some(5));
        let newest_empty = record("billing", "4", &[], Some(9));
        let members = [&old, &undated, &new, &newest_empty];

        let value = choose(&members, "phone", &[Strategy::MostRecent]).unwrap();
        assert_eq!(value.value, "600000003");
        assert_eq!(value.strategy, Strategy::MostRecent);
    }

    #[test]
    fn most_recent_ties_go_to_the_first_member() {
        let a = record("crm", "1", &[("phone", "1")], Some(2));
        let b = record("crm", "2", &[("phone", "2")], Some(2));
        let value = choose(&[&a, &b], "phone", &[Strategy::MostRecent]).unwrap();
        assert_eq!(value.record.id, "1");
    }

    #[test]
    fn strategies_are_tried_in_order() {
        let crm = record("crm", "1", &[("email", "old@example.com")], Some(1));
        let web = record("web", "2", &[("email", "new@example.com")], Some(3));
        let members = [&crm, &web];

        let value = choose(
            &members,
            "email",
            &[source(&["billing"]), Strategy::MostRecent],
        )
        .unwrap();
        assert_eq!(value.value, "new@example.com");
        assert_eq!(value.strategy, Strategy::MostRecent);
    }

    #[test]
    fn falls_back_to_the_first_non_empty_value() {
        let empty = record("crm", "1", &[], None);
        let filled = record("billing", "2", &[("tax_id", "B00000001")], None);
        let members = [&empty, &filled];

        let value = choose(&members, "tax_id", &[Strategy::MostRecent]).unwrap();
        assert_eq!(value.value, "B00000001");
        assert_eq!(value.record, RecordKey::new("billing", "2"));
        assert_eq!(value.strategy, Strategy::FirstNonEmpty);

        let value = choose(&members, "tax_id", &[]).unwrap();
        assert_eq!(value.strategy, Strategy::FirstNonEmpty);
    }

    #[test]
    fn empty_everywhere_gives_nothing() {
        let a = record("crm", "1", &[], Some(1));
        assert_eq!(choose(&[&a], "email", &[Strategy::MostRecent]), None);
        assert_eq!(choose(&[], "email", &[]), None);
    }

    #[test]
    fn survive_builds_every_attribute() {
        let crm = record("crm", "1", &[("email", "ana@example.com")], None);
        let billing = record(
            "billing",
            "2",
            &[("email", "facturas@example.com"), ("tax_id", "B00000001")],
            None,
        );
        let attributes = vec![
            "tax_id".to_string(),
            "email".to_string(),
            "phone".to_string(),
        ];
        let strategies = BTreeMap::from([("email".to_string(), vec![source(&["billing"])])]);

        let values = survive(&[&crm, &billing], &attributes, &strategies);
        assert_eq!(values.len(), 2);
        assert_eq!(values["email"].value, "facturas@example.com");
        assert_eq!(values["tax_id"].strategy, Strategy::FirstNonEmpty);
        assert!(!values.contains_key("phone"));
    }
}
