//! Connectors that read records from sources.
//!
//! Only CSV files are supported for now.

mod csv_source;

pub use csv_source::{read_csv, CsvOptions};

use std::path::PathBuf;

/// Error returned by a connector.
#[derive(Debug, thiserror::Error)]
pub enum ConnectError {
    /// The file could not be opened or read.
    #[error("cannot read {path}: {source}")]
    Io {
        /// File involved.
        path: PathBuf,
        /// Underlying error.
        source: std::io::Error,
    },
    /// The file is not valid CSV or not valid UTF-8.
    #[error("{path}, line {line}: {message}")]
    Format {
        /// File involved.
        path: PathBuf,
        /// Line where the problem was found, starting at 1.
        line: u64,
        /// What is wrong.
        message: String,
    },
    /// A configured column is not in the header row.
    #[error("{path}: column {column:?} not found in the header")]
    MissingColumn {
        /// File involved.
        path: PathBuf,
        /// Column name that was expected.
        column: String,
    },
}
