use crate::{MasterRecord, NormalizedRecord, SourceRecord, StoreError};

/// Persistence used by the engine. Domain crates never talk to storage
/// directly, they get data through this trait.
///
/// Every `replace_*` method replaces everything stored for the domain, which
/// is what a full run does.
pub trait Store {
    /// Replaces the raw source records of `domain`.
    fn replace_source_records(
        &mut self,
        domain: &str,
        records: &[SourceRecord],
    ) -> Result<(), StoreError>;

    /// Replaces the normalized records of `domain`.
    fn replace_normalized_records(
        &mut self,
        domain: &str,
        records: &[NormalizedRecord],
    ) -> Result<(), StoreError>;

    /// Replaces the golden records of `domain`.
    fn replace_master_records(
        &mut self,
        domain: &str,
        records: &[MasterRecord],
    ) -> Result<(), StoreError>;

    /// Returns the raw source records of `domain`, empty if there are none.
    fn source_records(&self, domain: &str) -> Result<Vec<SourceRecord>, StoreError>;

    /// Returns the normalized records of `domain`, empty if there are none.
    fn normalized_records(&self, domain: &str) -> Result<Vec<NormalizedRecord>, StoreError>;

    /// Returns the golden records of `domain`, empty if there are none.
    fn master_records(&self, domain: &str) -> Result<Vec<MasterRecord>, StoreError>;
}
