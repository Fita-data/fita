use std::fs;
use std::io::{BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

use fita_core::{MasterRecord, NormalizedRecord, SourceRecord, Store, StoreError};
use serde::de::DeserializeOwned;
use serde::Serialize;

/// Folder inside a project where [`FileStore`] keeps its data.
pub const STATE_DIR: &str = ".fita";

const SOURCE_RECORDS_FILE: &str = "source_records.json";
const NORMALIZED_RECORDS_FILE: &str = "normalized_records.json";
const MASTER_RECORDS_FILE: &str = "master_records.json";

/// A store that keeps one JSON file per domain and kind of record under
/// `<project>/.fita/<domain>/`. Use it to run a project locally without a
/// database.
///
/// Files are written to a temporary name and renamed, so a failed write never
/// leaves half a file behind. Domain names must be valid folder names, which
/// project validation already guarantees.
#[derive(Debug, Clone)]
pub struct FileStore {
    root: PathBuf,
}

impl FileStore {
    /// Opens the store of the project in `project_dir`. Nothing is created
    /// until the first write.
    pub fn new(project_dir: &Path) -> Self {
        Self {
            root: project_dir.join(STATE_DIR),
        }
    }

    /// Folder where the data is kept.
    pub fn root(&self) -> &Path {
        &self.root
    }

    fn file(&self, domain: &str, name: &str) -> PathBuf {
        self.root.join(domain).join(name)
    }

    fn write<T: Serialize>(
        &self,
        domain: &str,
        name: &str,
        records: &[T],
    ) -> Result<(), StoreError> {
        let path = self.file(domain, name);
        let dir = self.root.join(domain);
        fs::create_dir_all(&dir).map_err(|source| StoreError::Io {
            path: dir.clone(),
            source,
        })?;

        let temporary = path.with_extension("json.tmp");
        let io_error = |source| StoreError::Io {
            path: temporary.clone(),
            source,
        };
        let file = fs::File::create(&temporary).map_err(io_error)?;
        let mut writer = BufWriter::new(file);
        serde_json::to_writer_pretty(&mut writer, records).map_err(|error| {
            StoreError::Corrupt {
                path: temporary.clone(),
                message: error.to_string(),
            }
        })?;
        writer.flush().map_err(io_error)?;
        writer
            .into_inner()
            .map_err(|error| io_error(error.into_error()))?
            .sync_all()
            .map_err(io_error)?;

        fs::rename(&temporary, &path).map_err(|source| StoreError::Io { path, source })
    }

    fn read<T: DeserializeOwned>(&self, domain: &str, name: &str) -> Result<Vec<T>, StoreError> {
        let path = self.file(domain, name);
        let file = match fs::File::open(&path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(source) => return Err(StoreError::Io { path, source }),
        };
        serde_json::from_reader(BufReader::new(file)).map_err(|error| StoreError::Corrupt {
            path,
            message: error.to_string(),
        })
    }
}

impl Store for FileStore {
    fn replace_source_records(
        &mut self,
        domain: &str,
        records: &[SourceRecord],
    ) -> Result<(), StoreError> {
        self.write(domain, SOURCE_RECORDS_FILE, records)
    }

    fn replace_normalized_records(
        &mut self,
        domain: &str,
        records: &[NormalizedRecord],
    ) -> Result<(), StoreError> {
        self.write(domain, NORMALIZED_RECORDS_FILE, records)
    }

    fn replace_master_records(
        &mut self,
        domain: &str,
        records: &[MasterRecord],
    ) -> Result<(), StoreError> {
        self.write(domain, MASTER_RECORDS_FILE, records)
    }

    fn source_records(&self, domain: &str) -> Result<Vec<SourceRecord>, StoreError> {
        self.read(domain, SOURCE_RECORDS_FILE)
    }

    fn normalized_records(&self, domain: &str) -> Result<Vec<NormalizedRecord>, StoreError> {
        self.read(domain, NORMALIZED_RECORDS_FILE)
    }

    fn master_records(&self, domain: &str) -> Result<Vec<MasterRecord>, StoreError> {
        self.read(domain, MASTER_RECORDS_FILE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::{master_record, normalized_record, source_record};
    use tempfile::TempDir;

    #[test]
    fn missing_files_read_as_empty() {
        let dir = TempDir::new().unwrap();
        let store = FileStore::new(dir.path());
        assert!(store.source_records("customers").unwrap().is_empty());
        assert!(store.master_records("customers").unwrap().is_empty());
        assert!(!store.root().exists());
    }

    #[test]
    fn round_trips_through_json_files() {
        let dir = TempDir::new().unwrap();
        let mut store = FileStore::new(dir.path());
        store
            .replace_source_records("customers", &[source_record("1")])
            .unwrap();
        store
            .replace_normalized_records("customers", &[normalized_record("1")])
            .unwrap();
        store
            .replace_master_records("customers", &[master_record(1), master_record(2)])
            .unwrap();

        let base = dir.path().join(".fita/customers");
        assert!(base.join("source_records.json").is_file());
        assert!(base.join("normalized_records.json").is_file());
        assert!(base.join("master_records.json").is_file());
        assert!(!base.join("master_records.json.tmp").exists());

        let reopened = FileStore::new(dir.path());
        assert_eq!(
            reopened.source_records("customers").unwrap(),
            vec![source_record("1")]
        );
        assert_eq!(
            reopened.normalized_records("customers").unwrap(),
            vec![normalized_record("1")]
        );
        assert_eq!(
            reopened.master_records("customers").unwrap(),
            vec![master_record(1), master_record(2)]
        );
        let text = fs::read_to_string(base.join("master_records.json")).unwrap();
        assert!(text.contains("\"cus_00002\""), "{text}");
    }

    #[test]
    fn replace_overwrites_previous_data() {
        let dir = TempDir::new().unwrap();
        let mut store = FileStore::new(dir.path());
        store
            .replace_master_records("customers", &[master_record(1), master_record(2)])
            .unwrap();
        store
            .replace_master_records("customers", &[master_record(3)])
            .unwrap();
        assert_eq!(
            store.master_records("customers").unwrap(),
            vec![master_record(3)]
        );
    }

    #[test]
    fn reports_corrupt_files() {
        let dir = TempDir::new().unwrap();
        let base = dir.path().join(".fita/customers");
        fs::create_dir_all(&base).unwrap();
        fs::write(base.join("master_records.json"), "{not json").unwrap();
        let error = FileStore::new(dir.path())
            .master_records("customers")
            .unwrap_err();
        assert!(matches!(error, StoreError::Corrupt { .. }), "{error}");
    }
}
