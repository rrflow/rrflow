//! Immutable scalar, product, and binary quantization artifacts.
//!
//! These artifacts are candidate generators only. Canonical full-precision
//! vectors remain in the RRD change history and every planner execution exact
//! reranks the returned references before exposing results.

use crate::contract::invalid;
use crate::exact::validate_candidate_versions;
use crate::{
    AccessPathKind, CandidatePath, EmbeddingModelBinding, ScoreMetric, SearchHit, SearchRequest,
    VectorCandidate, VectorQuery,
};
use memmap2::{Mmap, MmapOptions};
use rrd_core::{
    digest, ProjectionId, ProjectionStamp, ProjectionState, Result, RuntimeProperties, RuntimeRef,
    ScopeId, VectorCollectionAddress, VectorValue, DATA_RUNTIME_CONTRACT_VERSION,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::path::Path;
use std::sync::Arc;

mod config;
mod encoding;
mod segment;

pub use config::*;
use encoding::*;
use segment::*;

#[cfg(test)]
mod tests;
