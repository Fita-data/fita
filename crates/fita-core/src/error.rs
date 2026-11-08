use std::path::PathBuf;

/// Error returned when a [`crate::MasterId`] cannot be built or parsed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum IdError {
    /// The prefix is empty or has characters other than lowercase ASCII letters and digits.
    #[error("invalid master id prefix {0:?}, use lowercase letters and digits")]
    InvalidPrefix(String),
    /// Master id numbers start at 1.
    #[error("master id number must be 1 or greater")]
    ZeroNumber,
    /// The text does not look like `prefix_00001`.
    #[error("invalid master id {0:?}, expected something like cus_00001")]
    InvalidFormat(String),
}

/// Error returned by a [`crate::Store`] implementation.
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    /// Reading or writing a file failed.
    #[error("cannot access {path}: {source}")]
    Io {
        /// File or directory involved.
        path: PathBuf,
        /// Underlying error.
        source: std::io::Error,
    },
    /// Stored data could not be encoded or decoded.
    #[error("invalid stored data in {path}: {message}")]
    Corrupt {
        /// File involved.
        path: PathBuf,
        /// What went wrong.
        message: String,
    },
}
