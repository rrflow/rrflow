//! Immutable, content-addressed object tier.
//!
//! Bytes are durable and verified before their canonical reference is handed
//! to the data transaction. A failed data transaction can therefore leave an
//! unreachable object, but never a reachable partial object; inventory and
//! reclamation make that asymmetry explicit.

use crate::{Error, Result};
use rrd_core::{digest, ObjectReceipt, ObjectReference};
use rrd_lsm::filesystem::{AnchoredPath, OpenOptions};
use std::collections::{BTreeMap, BTreeSet};
#[cfg(test)]
use std::fs;
use std::fs::File;
use std::io::{Cursor, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

mod local;
mod store;

pub use local::*;
pub use store::*;

#[cfg(test)]
mod tests;
