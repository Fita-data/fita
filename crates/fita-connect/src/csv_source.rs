use std::collections::{BTreeMap, HashSet};
use std::fs::File;
use std::path::{Path, PathBuf};

use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use csv::{ReaderBuilder, StringRecord};
use fita_core::{RecordKey, SourceRecord};

use crate::ConnectError;

/// Date and time formats accepted in the modification column, besides RFC 3339.
/// Times without an offset are taken as UTC.
const NAIVE_DATE_TIME_FORMATS: [&str; 2] = ["%Y-%m-%d %H:%M:%S", "%Y-%m-%dT%H:%M:%S"];

/// How to read a CSV source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsvOptions {
    /// Path of the file. It must be UTF-8 and have a header row.
    pub path: PathBuf,
    /// Field separator byte, such as `b','` or `b';'`.
    pub separator: u8,
    /// Column with the record id. Ids must be unique and not empty.
    pub key_column: String,
    /// Optional column with the last modification time of each record.
    /// Accepts RFC 3339, `YYYY-MM-DD HH:MM:SS` (UTC) or `YYYY-MM-DD`.
    pub modified_at_column: Option<String>,
    /// Source column for each attribute, keyed by attribute name.
    pub mapping: BTreeMap<String, String>,
}

/// Reads every record of a CSV file and maps its columns to attributes.
///
/// `source` is the source name stored in each [`RecordKey`]. Columns that are
/// not mapped are ignored. Values are kept exactly as read.
pub fn read_csv(source: &str, options: &CsvOptions) -> Result<Vec<SourceRecord>, ConnectError> {
    let path = options.path.as_path();
    let file = File::open(path).map_err(|source| ConnectError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let mut reader = ReaderBuilder::new()
        .delimiter(options.separator)
        .has_headers(true)
        .from_reader(file);

    let headers = reader
        .headers()
        .map_err(|error| csv_error(path, error))?
        .clone();
    let key_index = column_index(path, &headers, &options.key_column)?;
    let modified_index = match &options.modified_at_column {
        Some(column) => Some(column_index(path, &headers, column)?),
        None => None,
    };
    let mut attribute_indexes = Vec::new();
    for (attribute, column) in &options.mapping {
        attribute_indexes.push((attribute.clone(), column_index(path, &headers, column)?));
    }

    let mut records = Vec::new();
    let mut seen_ids = HashSet::new();
    for row in reader.records() {
        let row = row.map_err(|error| csv_error(path, error))?;
        let line = row.position().map_or(0, |position| position.line());
        let format_error = |message: String| ConnectError::Format {
            path: path.to_path_buf(),
            line,
            message,
        };

        let id = field(&row, key_index).trim();
        if id.is_empty() {
            return Err(format_error(format!(
                "empty value in key column {:?}",
                options.key_column
            )));
        }
        if !seen_ids.insert(id.to_string()) {
            return Err(format_error(format!("repeated record id {id:?}")));
        }

        let modified_at = match modified_index {
            Some(index) => parse_modified_at(field(&row, index)).map_err(format_error)?,
            None => None,
        };

        let mut values = BTreeMap::new();
        for (attribute, index) in &attribute_indexes {
            values.insert(attribute.clone(), field(&row, *index).to_string());
        }
        records.push(SourceRecord {
            key: RecordKey::new(source, id),
            values,
            modified_at,
        });
    }
    Ok(records)
}

fn column_index(path: &Path, headers: &StringRecord, column: &str) -> Result<usize, ConnectError> {
    headers
        .iter()
        .position(|header| header.trim() == column)
        .ok_or_else(|| ConnectError::MissingColumn {
            path: path.to_path_buf(),
            column: column.to_string(),
        })
}

/// Returns a field of a row. The reader rejects rows with a different number
/// of fields than the header, so every index found in the header exists.
fn field(row: &StringRecord, index: usize) -> &str {
    row.get(index).unwrap_or_default()
}

fn csv_error(path: &Path, error: csv::Error) -> ConnectError {
    let line = error.position().map_or(0, |position| position.line());
    let message = match error.kind() {
        csv::ErrorKind::Utf8 { .. } => "the file is not valid UTF-8".to_string(),
        _ => error.to_string(),
    };
    match error.into_kind() {
        csv::ErrorKind::Io(source) => ConnectError::Io {
            path: path.to_path_buf(),
            source,
        },
        _ => ConnectError::Format {
            path: path.to_path_buf(),
            line,
            message,
        },
    }
}

/// Parses the modification column. An empty value means no date.
fn parse_modified_at(text: &str) -> Result<Option<DateTime<Utc>>, String> {
    let text = text.trim();
    if text.is_empty() {
        return Ok(None);
    }
    if let Ok(date_time) = DateTime::parse_from_rfc3339(text) {
        return Ok(Some(date_time.with_timezone(&Utc)));
    }
    for format in NAIVE_DATE_TIME_FORMATS {
        if let Ok(date_time) = NaiveDateTime::parse_from_str(text, format) {
            return Ok(Some(date_time.and_utc()));
        }
    }
    if let Ok(date) = NaiveDate::parse_from_str(text, "%Y-%m-%d") {
        return Ok(Some(date.and_time(chrono::NaiveTime::MIN).and_utc()));
    }
    Err(format!("cannot read modification date {text:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use std::fs;
    use tempfile::TempDir;

    fn write(dir: &TempDir, content: &[u8]) -> PathBuf {
        let path = dir.path().join("source.csv");
        fs::write(&path, content).unwrap();
        path
    }

    fn options(path: PathBuf, separator: u8) -> CsvOptions {
        CsvOptions {
            path,
            separator,
            key_column: "id".into(),
            modified_at_column: None,
            mapping: BTreeMap::from([
                ("legal_name".to_string(), "name".to_string()),
                ("tax_id".to_string(), "nif".to_string()),
            ]),
        }
    }

    #[test]
    fn reads_and_maps_columns() {
        let dir = TempDir::new().unwrap();
        let path = write(
            &dir,
            "id;name;nif;ignored\n1;Ferretería Inventada SL;B00000001;x\n2;\"Uno; Dos SA\";;y\n"
                .as_bytes(),
        );
        let records = read_csv("billing", &options(path, b';')).unwrap();

        assert_eq!(records.len(), 2);
        assert_eq!(records[0].key, RecordKey::new("billing", "1"));
        assert_eq!(records[0].values["legal_name"], "Ferretería Inventada SL");
        assert_eq!(records[0].values["tax_id"], "B00000001");
        assert!(!records[0].values.contains_key("ignored"));
        assert_eq!(records[1].values["legal_name"], "Uno; Dos SA");
        assert_eq!(records[1].values["tax_id"], "");
        assert_eq!(records[0].modified_at, None);
    }

    #[test]
    fn handles_a_utf8_bom() {
        let dir = TempDir::new().unwrap();
        let path = write(&dir, "\u{feff}id,name,nif\n1,Ana,X\n".as_bytes());
        let records = read_csv("crm", &options(path, b',')).unwrap();
        assert_eq!(records[0].key.id, "1");
    }

    #[test]
    fn reads_modification_dates() {
        let dir = TempDir::new().unwrap();
        let path = write(
            &dir,
            b"id,name,nif,updated\n1,a,b,2026-03-01T10:00:00+01:00\n2,a,b,2026-03-01 09:30:00\n3,a,b,2026-03-02\n4,a,b,\n",
        );
        let mut opts = options(path, b',');
        opts.modified_at_column = Some("updated".into());
        let records = read_csv("crm", &opts).unwrap();
        let dates: Vec<_> = records.iter().map(|record| record.modified_at).collect();
        assert_eq!(
            dates,
            vec![
                Some(Utc.with_ymd_and_hms(2026, 3, 1, 9, 0, 0).unwrap()),
                Some(Utc.with_ymd_and_hms(2026, 3, 1, 9, 30, 0).unwrap()),
                Some(Utc.with_ymd_and_hms(2026, 3, 2, 0, 0, 0).unwrap()),
                None,
            ]
        );
    }

    #[test]
    fn rejects_bad_dates_with_the_line() {
        let dir = TempDir::new().unwrap();
        let path = write(
            &dir,
            b"id,name,nif,updated\n1,a,b,2026-01-01\n2,a,b,yesterday\n",
        );
        let mut opts = options(path, b',');
        opts.modified_at_column = Some("updated".into());
        let error = read_csv("crm", &opts).unwrap_err();
        assert!(
            matches!(error, ConnectError::Format { line: 3, .. }),
            "{error}"
        );
    }

    #[test]
    fn reports_missing_columns() {
        let dir = TempDir::new().unwrap();
        let path = write(&dir, b"id,name\n1,a\n");
        let error = read_csv("crm", &options(path, b',')).unwrap_err();
        assert!(
            matches!(&error, ConnectError::MissingColumn { column, .. } if column == "nif"),
            "{error}"
        );
    }

    #[test]
    fn rejects_empty_and_repeated_ids() {
        let dir = TempDir::new().unwrap();
        let path = write(&dir, b"id,name,nif\n1,a,b\n 1 ,c,d\n");
        let error = read_csv("crm", &options(path, b',')).unwrap_err();
        assert!(error.to_string().contains("repeated record id"), "{error}");

        let path = write(&dir, b"id,name,nif\n ,a,b\n");
        let error = read_csv("crm", &options(path, b',')).unwrap_err();
        assert!(
            error.to_string().contains("empty value in key column"),
            "{error}"
        );
    }

    #[test]
    fn rejects_rows_with_wrong_field_count() {
        let dir = TempDir::new().unwrap();
        let path = write(&dir, b"id,name,nif\n1,a\n");
        let error = read_csv("crm", &options(path, b',')).unwrap_err();
        assert!(matches!(error, ConnectError::Format { .. }), "{error}");
    }

    #[test]
    fn rejects_invalid_utf8() {
        let dir = TempDir::new().unwrap();
        let path = write(&dir, b"id,name,nif\n1,Ferreter\xeda,b\n");
        let error = read_csv("crm", &options(path, b',')).unwrap_err();
        assert!(error.to_string().contains("not valid UTF-8"), "{error}");
    }

    #[test]
    fn reports_missing_files() {
        let dir = TempDir::new().unwrap();
        let error = read_csv("crm", &options(dir.path().join("nope.csv"), b',')).unwrap_err();
        assert!(matches!(error, ConnectError::Io { .. }));
    }
}
