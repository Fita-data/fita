//! Implementations of [`fita_core::Store`].
//!
//! [`MemoryStore`] keeps everything in memory and is meant for tests.
//! [`FileStore`] keeps JSON files under `<project>/.fita/` so a project can run
//! locally without a database. The PostgreSQL store will come later; its
//! schema is already in `migrations/`.

mod file;
mod memory;

pub use file::{FileStore, STATE_DIR};
pub use memory::MemoryStore;

#[cfg(test)]
pub(crate) mod fixtures {
    use std::collections::BTreeMap;

    use fita_core::{
        ChosenValue, MasterId, MasterRecord, NormalizedRecord, RecordKey, SourceRecord, Strategy,
    };

    pub fn source_record(id: &str) -> SourceRecord {
        SourceRecord {
            key: RecordKey::new("billing", id),
            values: BTreeMap::from([("legal_name".to_string(), "Inventada, S.L.".to_string())]),
            modified_at: None,
        }
    }

    pub fn normalized_record(id: &str) -> NormalizedRecord {
        NormalizedRecord {
            key: RecordKey::new("billing", id),
            values: BTreeMap::from([("legal_name".to_string(), "INVENTADA SL".to_string())]),
            modified_at: None,
        }
    }

    pub fn master_record(number: u64) -> MasterRecord {
        let key = RecordKey::new("billing", number.to_string());
        MasterRecord {
            id: MasterId::new("cus", number).unwrap(),
            domain: "customers".into(),
            members: vec![key.clone()],
            links: Vec::new(),
            values: BTreeMap::from([(
                "legal_name".to_string(),
                ChosenValue {
                    value: "INVENTADA SL".into(),
                    record: key,
                    strategy: Strategy::FirstNonEmpty,
                },
            )]),
        }
    }
}
