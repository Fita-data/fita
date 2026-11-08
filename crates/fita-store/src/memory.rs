use std::collections::HashMap;

use fita_core::{MasterRecord, NormalizedRecord, SourceRecord, Store, StoreError};

/// A store that keeps everything in memory. Use it in tests.
#[derive(Debug, Default, Clone)]
pub struct MemoryStore {
    source_records: HashMap<String, Vec<SourceRecord>>,
    normalized_records: HashMap<String, Vec<NormalizedRecord>>,
    master_records: HashMap<String, Vec<MasterRecord>>,
}

impl MemoryStore {
    /// Creates an empty store.
    pub fn new() -> Self {
        Self::default()
    }
}

impl Store for MemoryStore {
    fn replace_source_records(
        &mut self,
        domain: &str,
        records: &[SourceRecord],
    ) -> Result<(), StoreError> {
        self.source_records
            .insert(domain.to_string(), records.to_vec());
        Ok(())
    }

    fn replace_normalized_records(
        &mut self,
        domain: &str,
        records: &[NormalizedRecord],
    ) -> Result<(), StoreError> {
        self.normalized_records
            .insert(domain.to_string(), records.to_vec());
        Ok(())
    }

    fn replace_master_records(
        &mut self,
        domain: &str,
        records: &[MasterRecord],
    ) -> Result<(), StoreError> {
        self.master_records
            .insert(domain.to_string(), records.to_vec());
        Ok(())
    }

    fn source_records(&self, domain: &str) -> Result<Vec<SourceRecord>, StoreError> {
        Ok(self.source_records.get(domain).cloned().unwrap_or_default())
    }

    fn normalized_records(&self, domain: &str) -> Result<Vec<NormalizedRecord>, StoreError> {
        Ok(self
            .normalized_records
            .get(domain)
            .cloned()
            .unwrap_or_default())
    }

    fn master_records(&self, domain: &str) -> Result<Vec<MasterRecord>, StoreError> {
        Ok(self.master_records.get(domain).cloned().unwrap_or_default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::{master_record, normalized_record, source_record};

    #[test]
    fn starts_empty() {
        let store = MemoryStore::new();
        assert!(store.source_records("customers").unwrap().is_empty());
        assert!(store.normalized_records("customers").unwrap().is_empty());
        assert!(store.master_records("customers").unwrap().is_empty());
    }

    #[test]
    fn replaces_per_domain() {
        let mut store = MemoryStore::new();
        store
            .replace_source_records("customers", &[source_record("1"), source_record("2")])
            .unwrap();
        store
            .replace_source_records("customers", &[source_record("3")])
            .unwrap();
        store
            .replace_normalized_records("customers", &[normalized_record("3")])
            .unwrap();
        store
            .replace_master_records("customers", &[master_record(1)])
            .unwrap();

        assert_eq!(
            store.source_records("customers").unwrap(),
            vec![source_record("3")]
        );
        assert_eq!(
            store.normalized_records("customers").unwrap(),
            vec![normalized_record("3")]
        );
        assert_eq!(
            store.master_records("customers").unwrap(),
            vec![master_record(1)]
        );
        assert!(store.master_records("suppliers").unwrap().is_empty());
    }
}
