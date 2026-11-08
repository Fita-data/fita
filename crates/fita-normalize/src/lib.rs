//! Attribute types and the normalizers that clean values before matching.
//!
//! Normalization is a pure function: the same input always gives the same
//! output. Check digit validation of tax ids is not done here yet.

mod attribute_type;
mod normalizers;

use std::collections::BTreeMap;

use fita_core::{NormalizedRecord, SourceRecord};

pub use attribute_type::{AttributeType, UnknownAttributeType};
pub use normalizers::{normalize_email, normalize_name, normalize_phone, normalize_tax_id};

/// Normalizes one raw value according to its attribute type.
///
/// Returns an empty string when nothing is left after cleaning.
pub fn normalize_value(attribute_type: AttributeType, raw: &str) -> String {
    match attribute_type {
        AttributeType::CompanyName | AttributeType::PersonName => normalize_name(raw),
        AttributeType::Email => normalize_email(raw),
        AttributeType::TaxId => normalize_tax_id(raw),
        AttributeType::Phone => normalize_phone(raw),
        AttributeType::Address
        | AttributeType::Iban
        | AttributeType::Amount
        | AttributeType::Date
        | AttributeType::Code
        | AttributeType::Text => normalizers::normalize_text(raw),
    }
}

/// Normalizes every value of a source record.
///
/// `types` gives the type of each attribute of the domain. Attributes missing
/// from `types` are normalized as [`AttributeType::Text`]. Values that end up
/// empty are left out of the result.
pub fn normalize_record(
    record: &SourceRecord,
    types: &BTreeMap<String, AttributeType>,
) -> NormalizedRecord {
    let mut values = BTreeMap::new();
    for (attribute, raw) in &record.values {
        let attribute_type = types.get(attribute).copied().unwrap_or(AttributeType::Text);
        let normalized = normalize_value(attribute_type, raw);
        if !normalized.is_empty() {
            values.insert(attribute.clone(), normalized);
        }
    }
    NormalizedRecord {
        key: record.key.clone(),
        values,
        modified_at: record.modified_at,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fita_core::RecordKey;

    #[test]
    fn dispatches_by_type() {
        assert_eq!(
            normalize_value(AttributeType::CompanyName, "Café Ñandú, S.L."),
            "CAFE NANDU SL"
        );
        assert_eq!(
            normalize_value(AttributeType::Email, " Ana@Example.COM "),
            "ana@example.com"
        );
        assert_eq!(
            normalize_value(AttributeType::TaxId, "ES-B00.000.001"),
            "B00000001"
        );
        assert_eq!(
            normalize_value(AttributeType::Phone, "+34 600 00 00 01"),
            "34600000001"
        );
        assert_eq!(normalize_value(AttributeType::Code, "  A   12 "), "A 12");
    }

    #[test]
    fn normalizes_a_record_and_drops_empty_values() {
        let record = SourceRecord {
            key: RecordKey::new("crm", "1"),
            values: BTreeMap::from([
                (
                    "legal_name".to_string(),
                    "Ferretería Inventada, S.A.".to_string(),
                ),
                ("phone".to_string(), "  -  ".to_string()),
                ("notes".to_string(), " some  text ".to_string()),
            ]),
            modified_at: None,
        };
        let types = BTreeMap::from([
            ("legal_name".to_string(), AttributeType::CompanyName),
            ("phone".to_string(), AttributeType::Phone),
        ]);
        let normalized = normalize_record(&record, &types);
        assert_eq!(normalized.key, record.key);
        assert_eq!(
            normalized.value("legal_name"),
            Some("FERRETERIA INVENTADA SA")
        );
        assert_eq!(normalized.value("phone"), None);
        assert_eq!(normalized.value("notes"), Some("some text"));
    }

    #[test]
    fn normalizing_twice_changes_nothing() {
        let samples = [
            "  José  Pérez-Gómez ",
            "ES 12.345.678-z",
            "Ana@Example.org",
            "(+34) 600-000-001",
        ];
        let types = [
            AttributeType::PersonName,
            AttributeType::TaxId,
            AttributeType::Email,
            AttributeType::Phone,
            AttributeType::Text,
        ];
        for attribute_type in types {
            for sample in samples {
                let once = normalize_value(attribute_type, sample);
                assert_eq!(
                    normalize_value(attribute_type, &once),
                    once,
                    "{attribute_type:?} {sample}"
                );
            }
        }
    }
}
