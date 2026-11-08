//! Parsing of rule conditions and survivorship strategies.
//!
//! This is a stopgap until `fita-expr` exists. It only understands
//! `equal(attribute)`, `source(a, b)` and `most_recent`, and recognises the
//! rest of the documented names so it can say they are not supported yet.

use fita_core::Strategy;
use serde_yaml::Value;

/// Rule functions documented in ARCHITECTURE.md.
const KNOWN_FUNCTIONS: [&str; 14] = [
    "equal",
    "different",
    "empty",
    "valid",
    "similarity",
    "jaro_winkler",
    "levenshtein",
    "token_set",
    "phonetic",
    "distance_km",
    "prefix",
    "suffix",
    "contains",
    "in_list",
];

/// Survivorship strategies documented in ARCHITECTURE.md but not supported yet.
const UNSUPPORTED_STRATEGIES: [&str; 5] = ["most_frequent", "valid", "longest", "all", "pinned"];

/// Result of parsing one entry of a rule's `when` list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Condition {
    /// `equal(attribute)`.
    Equal(String),
    /// A documented construct that is not supported yet, by name.
    Unsupported(String),
    /// Not a valid condition. The text explains why.
    Invalid(String),
}

/// Result of parsing one survivorship strategy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ParsedStrategy {
    Supported(Strategy),
    Unsupported(String),
    Invalid(String),
}

/// Joins list entries that YAML split at a comma inside parentheses.
///
/// In a flow list such as `[source(crm, billing), most_recent]` YAML sees
/// three entries: `source(crm`, `billing)` and `most_recent`. This puts back
/// together every run of entries whose parentheses only balance when joined.
pub(crate) fn rejoin_split_calls(entries: Vec<Value>) -> Vec<Value> {
    let mut joined = Vec::with_capacity(entries.len());
    let mut pending: Option<String> = None;
    for entry in entries {
        match (pending.take(), entry) {
            (Some(mut open), Value::String(text)) => {
                open.push_str(", ");
                open.push_str(text.trim());
                if paren_balance(&open) > 0 {
                    pending = Some(open);
                } else {
                    joined.push(Value::String(open));
                }
            }
            (Some(open), other) => {
                joined.push(Value::String(open));
                joined.push(other);
            }
            (None, Value::String(text)) if paren_balance(&text) > 0 => {
                pending = Some(text.trim().to_string());
            }
            (None, other) => joined.push(other),
        }
    }
    joined.extend(pending.map(Value::String));
    joined
}

fn paren_balance(text: &str) -> i64 {
    let mut balance = 0;
    for c in text.chars() {
        match c {
            '(' => balance += 1,
            ')' => balance -= 1,
            _ => {}
        }
    }
    balance
}

/// Returns true if `name` can be used as a domain, source or attribute name.
///
/// Names may use letters of any language, digits, `_` and `-`.
pub(crate) fn is_valid_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
}

/// Parses one entry of a `when` list.
pub(crate) fn parse_condition(entry: &Value) -> Condition {
    match entry {
        Value::String(text) => parse_condition_text(text),
        Value::Mapping(map) if map.contains_key("any") => Condition::Unsupported("any".into()),
        _ => Condition::Invalid("a condition must be a text such as equal(tax_id)".into()),
    }
}

fn parse_condition_text(text: &str) -> Condition {
    let text = text.trim();
    let Some((function, rest)) = split_call(text) else {
        return Condition::Invalid(format!(
            "{text:?} is not a function call such as equal(tax_id)"
        ));
    };
    if !KNOWN_FUNCTIONS.contains(&function) {
        return Condition::Invalid(format!("unknown function {function:?} in {text:?}"));
    }
    if function != "equal" {
        return Condition::Unsupported(function.to_string());
    }
    match rest.strip_suffix(')').map(str::trim) {
        Some(attribute) if is_valid_name(attribute) => Condition::Equal(attribute.to_string()),
        _ => Condition::Invalid(format!(
            "{text:?} must be equal(attribute) with one attribute"
        )),
    }
}

/// Parses one survivorship strategy, such as `source(crm, billing)`.
pub(crate) fn parse_strategy(text: &str) -> ParsedStrategy {
    let text = text.trim();
    if text == "most_recent" {
        return ParsedStrategy::Supported(Strategy::MostRecent);
    }
    if UNSUPPORTED_STRATEGIES.contains(&text) {
        return ParsedStrategy::Unsupported(text.to_string());
    }
    let Some(("source", rest)) = split_call(text) else {
        return ParsedStrategy::Invalid(format!("unknown survivorship strategy {text:?}"));
    };
    let Some(inner) = rest.strip_suffix(')') else {
        return ParsedStrategy::Invalid(format!("{text:?} is missing a closing parenthesis"));
    };
    let mut sources = Vec::new();
    for name in inner.split(',').map(str::trim) {
        if !is_valid_name(name) {
            return ParsedStrategy::Invalid(format!("{text:?} must list source names"));
        }
        sources.push(name.to_string());
    }
    ParsedStrategy::Supported(Strategy::Source(sources))
}

/// Splits `name(rest` into the function name and everything after `(`.
fn split_call(text: &str) -> Option<(&str, &str)> {
    let (function, rest) = text.split_once('(')?;
    let function = function.trim();
    if function.is_empty() || !function.chars().all(|c| c.is_ascii_lowercase() || c == '_') {
        return None;
    }
    Some((function, rest))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(value: &str) -> Value {
        Value::String(value.to_string())
    }

    #[test]
    fn parses_equal() {
        assert_eq!(
            parse_condition(&text("equal(tax_id)")),
            Condition::Equal("tax_id".into())
        );
        assert_eq!(
            parse_condition(&text(" equal( cif ) ")),
            Condition::Equal("cif".into())
        );
    }

    #[test]
    fn other_documented_functions_are_unsupported() {
        assert_eq!(
            parse_condition(&text("similarity(legal_name) >= 0.85")),
            Condition::Unsupported("similarity".into())
        );
        assert_eq!(
            parse_condition(&text("different(tax_id)")),
            Condition::Unsupported("different".into())
        );
        let any: Value = serde_yaml::from_str("any: [equal(a), equal(b)]").unwrap();
        assert_eq!(parse_condition(&any), Condition::Unsupported("any".into()));
    }

    #[test]
    fn rejects_malformed_conditions() {
        for bad in [
            "tax_id",
            "same(tax_id)",
            "equal(tax_id, phone)",
            "equal(tax_id",
            "equal()",
        ] {
            assert!(
                matches!(parse_condition(&text(bad)), Condition::Invalid(_)),
                "{bad}"
            );
        }
        assert!(matches!(
            parse_condition(&Value::Number(3.into())),
            Condition::Invalid(_)
        ));
    }

    #[test]
    fn parses_strategies() {
        assert_eq!(
            parse_strategy("source(crm, billing)"),
            ParsedStrategy::Supported(Strategy::Source(vec!["crm".into(), "billing".into()]))
        );
        assert_eq!(
            parse_strategy("most_recent"),
            ParsedStrategy::Supported(Strategy::MostRecent)
        );
        assert_eq!(
            parse_strategy("valid"),
            ParsedStrategy::Unsupported("valid".into())
        );
        for bad in [
            "newest",
            "source()",
            "source(crm",
            "source(crm,)",
            "first_non_empty",
        ] {
            assert!(
                matches!(parse_strategy(bad), ParsedStrategy::Invalid(_)),
                "{bad}"
            );
        }
    }

    #[test]
    fn rejoins_calls_split_by_yaml() {
        let list: Vec<Value> =
            serde_yaml::from_str("[source(crm, billing, erp), most_recent, in_list(code, a, b)]")
                .unwrap();
        assert_eq!(list.len(), 7);
        assert_eq!(
            rejoin_split_calls(list),
            vec![
                text("source(crm, billing, erp)"),
                text("most_recent"),
                text("in_list(code, a, b)"),
            ]
        );
    }

    #[test]
    fn rejoin_leaves_balanced_and_unclosed_entries_alone() {
        let list = vec![text("equal(tax_id)"), text("source(crm")];
        assert_eq!(rejoin_split_calls(list.clone()), list);
        let any: Value = serde_yaml::from_str("any: [equal(a)]").unwrap();
        let list = vec![text("source(crm"), any.clone()];
        assert_eq!(rejoin_split_calls(list), vec![text("source(crm"), any]);
    }

    #[test]
    fn validates_names() {
        assert!(is_valid_name("razón_social"));
        assert!(is_valid_name("tax-id2"));
        assert!(!is_valid_name(""));
        assert!(!is_valid_name("a b"));
        assert!(!is_valid_name("../x"));
    }
}
