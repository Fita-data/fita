//! Simple, type specific normalizers.

use unicode_normalization::char::is_combining_mark;
use unicode_normalization::UnicodeNormalization;

/// Normalizes a company or person name for matching.
///
/// Uppercases, removes accents and other diacritics (so `Ñ` becomes `N`),
/// turns dashes, slashes and underscores into spaces, drops any other
/// punctuation and collapses whitespace. `Pérez-Gómez, S.L.` becomes
/// `PEREZ GOMEZ SL`.
pub fn normalize_name(raw: &str) -> String {
    let mut cleaned = String::with_capacity(raw.len());
    for c in raw.nfd() {
        if is_combining_mark(c) {
            continue;
        }
        if c.is_alphanumeric() {
            cleaned.extend(c.to_uppercase());
        } else if c.is_whitespace() || is_word_separator(c) {
            cleaned.push(' ');
        }
    }
    collapse_whitespace(&cleaned)
}

/// Normalizes an email address: trims it and lowercases it.
pub fn normalize_email(raw: &str) -> String {
    raw.trim().to_lowercase()
}

/// Normalizes a tax id: uppercases, removes every separator and drops a
/// leading `ES` country prefix. Check digits are not validated.
pub fn normalize_tax_id(raw: &str) -> String {
    let compact: String = raw
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_uppercase())
        .collect();
    match compact.strip_prefix("ES") {
        Some(rest) if !rest.is_empty() => rest.to_string(),
        _ => compact,
    }
}

/// Normalizes a phone number by keeping only its digits.
pub fn normalize_phone(raw: &str) -> String {
    raw.chars().filter(|c| c.is_ascii_digit()).collect()
}

/// Default normalizer for types without a specific one: trims and collapses
/// whitespace.
pub(crate) fn normalize_text(raw: &str) -> String {
    collapse_whitespace(raw)
}

fn is_word_separator(c: char) -> bool {
    matches!(c, '-' | '‐' | '–' | '—' | '/' | '\\' | '_')
}

fn collapse_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_uppercased_without_accents_or_signs() {
        assert_eq!(normalize_name("José Pérez-Gómez"), "JOSE PEREZ GOMEZ");
        assert_eq!(
            normalize_name("  Distribuciones   Ñandú, S.L. "),
            "DISTRIBUCIONES NANDU SL"
        );
        assert_eq!(normalize_name("O'Neill & Hijos"), "ONEILL HIJOS");
        assert_eq!(
            normalize_name("Comercial\tÀvila/Sur"),
            "COMERCIAL AVILA SUR"
        );
        assert_eq!(normalize_name("Çà et là"), "CA ET LA");
        assert_eq!(normalize_name(" .,; "), "");
    }

    #[test]
    fn emails_are_lowercased_and_trimmed() {
        assert_eq!(normalize_email("  Info@Example.ORG "), "info@example.org");
        assert_eq!(normalize_email("   "), "");
    }

    #[test]
    fn tax_ids_lose_separators_and_es_prefix() {
        assert_eq!(normalize_tax_id("b-00.000.001"), "B00000001");
        assert_eq!(normalize_tax_id("ES B00000001"), "B00000001");
        assert_eq!(normalize_tax_id("es12345678z"), "12345678Z");
        assert_eq!(normalize_tax_id("X 0000000-T"), "X0000000T");
        assert_eq!(normalize_tax_id("ES"), "ES");
        assert_eq!(normalize_tax_id(" - "), "");
    }

    #[test]
    fn tax_ids_are_not_checked() {
        // Wrong check digit on purpose: normalization keeps it as is.
        assert_eq!(normalize_tax_id("00000000-A"), "00000000A");
    }

    #[test]
    fn phones_keep_only_digits() {
        assert_eq!(normalize_phone("+34 (600) 00-00-01"), "34600000001");
        assert_eq!(normalize_phone("ext."), "");
    }

    #[test]
    fn text_collapses_whitespace() {
        assert_eq!(normalize_text("  a \n b\t c "), "a b c");
    }
}
