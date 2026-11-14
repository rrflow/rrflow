//! Bounded sustained evidence built on the existing storage manifest/ledger.
//! These records describe qualification; they never configure the engine.
use super::progress::ProgressJournal;
use super::sustained_backup::BackupTracker;
use super::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

mod evidence;
mod model;
mod summary;
mod validation;
mod writer;

pub use evidence::*;
pub use model::*;
pub use summary::*;
use validation::*;
pub use writer::*;

#[cfg(test)]
#[path = "sustained_backup_tests.rs"]
mod backup_tests;

#[cfg(test)]
pub(super) mod tests;
