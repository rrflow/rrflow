//! Digest-verified local backup catalogue over retained logical archives.
//!
//! Every digest here detects changed bytes. None authenticates a producer:
//! the catalogue shares the estate's filesystem trust domain, so a keyed
//! authenticator would be stored beside the key it protects. Operations that
//! read a catalogue say "verify", never "authenticate".

use crate::access::runtime_state::{checked_key, get};
use crate::archive::{ControlArchiveWriter, CATALOGUE_CONTROL_FAMILIES};
use crate::filesystem::{AnchorInput, AnchoredPath, OpenOptions};
use crate::keyspaces::{self, Durability};
use crate::repository::prepare_control_batch;
use crate::{
    control_family_covers, control_family_of, export_logical_archive, inspect_control_archive,
    inspect_logical_archive, read_control_archive,
    restore_logical_archive_to_new_root_with_options, validate_control_families,
    ControlArchiveInventory, ControlTransition, Error, ImmutableObjectStore, LocalObjectStore,
    LogicalArchiveInventory, LogicalRestoreReport, Result, RrflowKvStore, StorageEngine,
    CONTROL_ARCHIVE_VERSION,
};
use rrd_core::digest;
use rrd_core::{ObjectReference, RuntimeMutation, ScopeId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::io::{Read, Write};
use std::path::{Component, Path};
use std::sync::atomic::{AtomicU64, Ordering};

mod catalogue;
mod catalogues;
mod control_journal;
mod create;
mod object_payloads;
mod persistence;
mod restore;
mod retention;
mod types;
mod validation;

pub use catalogue::*;
use catalogues::*;
use control_journal::*;
pub use create::*;
use object_payloads::*;
use persistence::*;
pub use restore::*;
use retention::*;
pub use types::*;
use validation::*;

#[cfg(test)]
mod capture_tests;

#[cfg(test)]
mod tests;
