mod open;
mod publish;
mod read;
mod records;

use publish::*;
pub use records::*;

use crate::filesystem::{AnchoredPath, OpenOptions};

use crate::{Error, Result};

use rrd_core::digest;

use serde::{Deserialize, Serialize};

use std::collections::{BTreeMap, BTreeSet};

use std::fs::File;

use std::path::Path;

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use std::sync::Mutex;

pub const DEFAULT_CHECKPOINT_ENTRIES: usize = 1024;

pub const MANIFEST_FORMAT_VERSION: u16 = 3;

const MANIFEST_CONTROL_FORMAT_VERSION: u16 = 1;

pub(crate) const CURRENT_FILE: &str = "CURRENT";

pub(crate) const MANIFEST_DIRECTORY: &str = "manifests";

pub(crate) const CHECKPOINT_DIRECTORY: &str = "checkpoints";

pub(crate) const MANIFEST_LOCK_FILE: &str = "MANIFEST.LOCK";

static POINTER_TEMPORARY_ID: AtomicU64 = AtomicU64::new(1);

/// Holds the process lock for the full publication session. Manifest bytes are
/// synced before `CURRENT`, then the parent directory is synced after rename.
pub struct ManifestStore {
    root: AnchoredPath,
    _lock: File,
    temporary_nonce: String,
    publication_uncertain: AtomicBool,
    checkpoint_entries: usize,
    checkpoint_budget: crate::ReclamationLimits,
    checkpoint_gate: Mutex<()>,
}

impl Drop for ManifestStore {
    fn drop(&mut self) {
        // Release explicitly before Windows closes the handle. Relying only on
        // handle teardown can leave an immediate same-process reopen racing the
        // OS lock release.
        let _ = File::unlock(&self._lock);
    }
}
