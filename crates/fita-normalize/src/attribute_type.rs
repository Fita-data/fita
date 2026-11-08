use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// Type of a domain attribute. It decides how values are normalized.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttributeType {
    /// Tax identification number, such as a Spanish NIF or CIF.
    TaxId,
    /// Name of a company.
    CompanyName,
    /// Name of a person.
    PersonName,
    /// Postal address.
    Address,
    /// Phone number.
    Phone,
    /// Email address.
    Email,
    /// Bank account number.
    Iban,
    /// Money amount.
    Amount,
    /// Calendar date.
    Date,
    /// Short code, such as a country or customer group.
    Code,
    /// Free text.
    Text,
}

impl AttributeType {
    /// Every known type, in the order used in documentation.
    pub const ALL: [AttributeType; 11] = [
        AttributeType::TaxId,
        AttributeType::CompanyName,
        AttributeType::PersonName,
        AttributeType::Address,
        AttributeType::Phone,
        AttributeType::Email,
        AttributeType::Iban,
        AttributeType::Amount,
        AttributeType::Date,
        AttributeType::Code,
        AttributeType::Text,
    ];

    /// Name used in configuration files, such as `tax_id`.
    pub fn name(self) -> &'static str {
        match self {
            AttributeType::TaxId => "tax_id",
            AttributeType::CompanyName => "company_name",
            AttributeType::PersonName => "person_name",
            AttributeType::Address => "address",
            AttributeType::Phone => "phone",
            AttributeType::Email => "email",
            AttributeType::Iban => "iban",
            AttributeType::Amount => "amount",
            AttributeType::Date => "date",
            AttributeType::Code => "code",
            AttributeType::Text => "text",
        }
    }
}

impl fmt::Display for AttributeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Error returned when parsing a type name that does not exist.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownAttributeType(pub String);

impl fmt::Display for UnknownAttributeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let known: Vec<&str> = AttributeType::ALL.iter().map(|t| t.name()).collect();
        write!(
            f,
            "unknown attribute type {:?}, expected one of: {}",
            self.0,
            known.join(", ")
        )
    }
}

impl std::error::Error for UnknownAttributeType {}

impl FromStr for AttributeType {
    type Err = UnknownAttributeType;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        AttributeType::ALL
            .into_iter()
            .find(|t| t.name() == name)
            .ok_or_else(|| UnknownAttributeType(name.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_every_known_name() {
        for attribute_type in AttributeType::ALL {
            assert_eq!(attribute_type.name().parse(), Ok(attribute_type));
        }
    }

    #[test]
    fn rejects_unknown_names() {
        let error = "nif".parse::<AttributeType>().unwrap_err();
        assert_eq!(error, UnknownAttributeType("nif".into()));
        assert!(error.to_string().contains("tax_id"));
    }
}
