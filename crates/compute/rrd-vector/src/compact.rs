//! Compact, immutable dense-vector artifact with a zero-copy mmap read path.
//!
//! The legacy JSON segment remains a portable semantic fixture. This format
//! separates canonical JSON metadata from aligned row-major `f32` payloads so
//! production search does not deserialize or duplicate the vector corpus.

use crate::contract::invalid;
use crate::exact::validate_candidate_versions;
use crate::{ScoreMetric, SearchHit, SearchRequest, VectorCandidate, VectorSegmentConfig};
use memmap2::{Mmap, MmapOptions};
use rrd_core::{
    digest, EmbeddingProvenance, ProjectionStamp, ProjectionState, Result, RuntimeCommit,
    RuntimeMutation, RuntimeProperties, RuntimeRef, RuntimeVector, VectorCollectionAddress,
    VectorValue, DATA_RUNTIME_CONTRACT_VERSION,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

mod encoding;
mod segment;

use encoding::*;
pub use segment::*;

#[cfg(test)]
mod tests;
