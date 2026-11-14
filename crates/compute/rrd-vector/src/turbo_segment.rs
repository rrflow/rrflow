use crate::contract::invalid;
use crate::exact::validate_candidate_versions;
use crate::qjl::{QjlProjection, MAX_QJL_DIMENSIONS};
use crate::turboquant::{inner_product_packed_len, inner_product_padded_dimensions};
use crate::{
    AccessPathKind, CandidatePath, EmbeddingModelBinding, QuantizedKernel,
    QuantizedMemoryPlacement, ScoreMetric, SearchHit, SearchRequest, TurboQuantBits,
    TurboQuantEstimator, TurboQuantInnerProductCode, TurboQuantQuery, TurboQuantVector,
    VectorCandidate, VectorQuery,
};
use memmap2::{Mmap, MmapOptions};
use rrd_core::{
    digest, ProjectionId, ProjectionStamp, ProjectionState, Result, RuntimeProperties, RuntimeRef,
    ScopeId, VectorCollectionAddress, VectorValue, DATA_RUNTIME_CONTRACT_VERSION,
};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{BTreeSet, BinaryHeap};
use std::fs::File;
use std::path::Path;
use std::sync::{Arc, OnceLock};

mod config;
mod encoding;
mod segment;

pub use config::*;
use encoding::*;

#[cfg(test)]
mod tests;
