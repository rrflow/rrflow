//! Authenticated, resumable S3-compatible immutable object adapter.
//!
//! The transport owns HTTP, TLS, credentials, and SigV4/mTLS mechanics. This
//! layer admits only a transport that declares the exact S3 capabilities RRD
//! requires, then owns deterministic keys, bounded retry, multipart resume,
//! checksums, conditional completion, range-streamed verification, and RRD
//! object receipts.

use crate::{
    Error, ImmutableObjectStore, ObjectInventory, ObjectInventoryEntry, ObjectInventoryState,
    Result, VerifiedObject,
};
use rrd_core::{digest, ObjectReceipt, ObjectReference};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{Cursor, Read};

mod client;
mod store;

pub use client::*;
pub use store::*;

#[cfg(test)]
mod tests;
