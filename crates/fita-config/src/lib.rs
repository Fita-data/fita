//! Loads a Fita project from disk and validates it.
//!
//! A project is a folder with `fita.yaml`, `domains/`, `sources/` and
//! `rules/`. [`load`] reads every file, checks that names and references are
//! consistent and returns a [`Project`] or every problem it found, each one
//! with the file it comes from.
//!
//! Only part of the rule language is supported for now. Rules or strategies
//! that use something not supported yet produce a warning and are skipped.

mod error;
mod load;
mod model;
mod raw;
mod rules;

pub use error::{Issue, LoadError};
pub use load::load;
pub use model::{
    Attribute, Connector, Domain, MatchRule, Project, RuleSet, Source, DEFAULT_CSV_SEPARATOR,
};
