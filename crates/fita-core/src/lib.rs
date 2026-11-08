//! Domain types, ids, errors and the storage trait shared by every Fita crate.
//!
//! This crate depends on nothing else in the workspace. Domain crates build on
//! these types and storage backends implement [`Store`].

mod error;
mod id;
mod record;
mod store;

pub use error::{IdError, StoreError};
pub use id::{is_valid_prefix, MasterId};
pub use record::{
    ChosenValue, MasterRecord, MatchLink, NormalizedRecord, RecordKey, SourceRecord, Strategy,
};
pub use store::Store;
