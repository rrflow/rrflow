//! Feature-gated C-06j measurements over rrflowKV's real segment-v6 encoder
//! and immutable-page reader.
//!
//! Nothing in this module owns production storage policy. It verifies the
//! canonical persisted filter, integrated adaptive-LZ4 behavior, and selected
//! scan-resistant cache policy, then keeps unintegrated codec, cache, and
//! value-placement candidates reproducible before another candidate changes
//! durable bytes.

#[cfg(test)]
use super::format::RowGroupFilter;
#[cfg(test)]
use super::format::FOOTER_BYTES;
use super::format::{
    encode, parse_metadata, sha256_hex, PageKind, FILTER_BITS_PER_KEY, FILTER_HASH_FUNCTIONS,
};
use crate::{
    Database, DatabaseOptions, Durability, Error, Memtable, Mutation, PageCachePolicy,
    ProjectedReadBudget, ProjectedReadProjection, ProjectedReadRange, ProjectedReadRequest, Result,
    SegmentCompressionPolicy, SegmentRowGroupBudget, WriteBatch, SEGMENT_FORMAT_VERSION,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::hint::black_box;
use std::io::Cursor;
use std::path::Path;
use std::time::Instant;

mod cache;
mod integration;
mod observations;
mod trial;
mod types;

use integration::*;
use observations::*;
pub use trial::*;
pub use types::*;

#[cfg(test)]
mod tests;
