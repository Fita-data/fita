use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::IdError;

/// Minimum number of digits in the number part of a [`MasterId`].
const MIN_DIGITS: usize = 5;

/// Returns true if `prefix` can be used as a domain prefix in a [`MasterId`].
///
/// A prefix is one or more lowercase ASCII letters or digits, such as `cus`.
pub fn is_valid_prefix(prefix: &str) -> bool {
    !prefix.is_empty()
        && prefix
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
}

/// Readable public id of a golden record, made of the domain prefix and a
/// number, such as `cus_00412`.
///
/// The number is padded to five digits and grows past that when needed.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct MasterId {
    prefix: String,
    number: u64,
}

impl MasterId {
    /// Builds an id from a domain prefix and a number starting at 1.
    pub fn new(prefix: &str, number: u64) -> Result<Self, IdError> {
        if !is_valid_prefix(prefix) {
            return Err(IdError::InvalidPrefix(prefix.to_string()));
        }
        if number == 0 {
            return Err(IdError::ZeroNumber);
        }
        Ok(Self {
            prefix: prefix.to_string(),
            number,
        })
    }

    /// Domain prefix, such as `cus`.
    pub fn prefix(&self) -> &str {
        &self.prefix
    }

    /// Number part of the id.
    pub fn number(&self) -> u64 {
        self.number
    }
}

impl fmt::Display for MasterId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}_{:0width$}",
            self.prefix,
            self.number,
            width = MIN_DIGITS
        )
    }
}

impl FromStr for MasterId {
    type Err = IdError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let invalid = || IdError::InvalidFormat(text.to_string());
        let (prefix, digits) = text.rsplit_once('_').ok_or_else(invalid)?;
        if digits.len() < MIN_DIGITS || !digits.chars().all(|c| c.is_ascii_digit()) {
            return Err(invalid());
        }
        let number: u64 = digits.parse().map_err(|_| invalid())?;
        let id = Self::new(prefix, number)?;
        // Reject forms like cus_000001 that would not print back the same way.
        if id.to_string() != text {
            return Err(invalid());
        }
        Ok(id)
    }
}

impl TryFrom<String> for MasterId {
    type Error = IdError;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        text.parse()
    }
}

impl From<MasterId> for String {
    fn from(id: MasterId) -> Self {
        id.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_with_five_digits() {
        let id = MasterId::new("cus", 412).unwrap();
        assert_eq!(id.to_string(), "cus_00412");
    }

    #[test]
    fn grows_past_five_digits() {
        let id = MasterId::new("sup", 1_234_567).unwrap();
        assert_eq!(id.to_string(), "sup_1234567");
    }

    #[test]
    fn rejects_bad_prefix_and_zero() {
        assert_eq!(
            MasterId::new("Cus", 1),
            Err(IdError::InvalidPrefix("Cus".into()))
        );
        assert_eq!(MasterId::new("", 1), Err(IdError::InvalidPrefix("".into())));
        assert_eq!(MasterId::new("cus", 0), Err(IdError::ZeroNumber));
    }

    #[test]
    fn parses_round_trip() {
        let id: MasterId = "cus_00001".parse().unwrap();
        assert_eq!(id.prefix(), "cus");
        assert_eq!(id.number(), 1);
        assert_eq!(id.to_string(), "cus_00001");
    }

    #[test]
    fn rejects_malformed_text() {
        for text in [
            "cus",
            "cus_1",
            "cus_000001",
            "cus_0000a",
            "_00001",
            "cus_00000",
        ] {
            assert!(text.parse::<MasterId>().is_err(), "{text} should not parse");
        }
    }

    #[test]
    fn serializes_as_string() {
        let id = MasterId::new("cus", 7).unwrap();
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"cus_00007\"");
        let back: MasterId = serde_json::from_str(&json).unwrap();
        assert_eq!(back, id);
        assert!(serde_json::from_str::<MasterId>("\"bad\"").is_err());
    }

    #[test]
    fn validates_prefixes() {
        assert!(is_valid_prefix("cus"));
        assert!(is_valid_prefix("p2"));
        assert!(!is_valid_prefix("cus_x"));
        assert!(!is_valid_prefix("CUS"));
        assert!(!is_valid_prefix(""));
    }
}
