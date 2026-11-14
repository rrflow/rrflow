use crate::contract::invalid;
use crate::exact::{score_dense, validate_candidate_versions};
use crate::{
    search_exact, AccessPathKind, CandidatePath, EmbeddingModelBinding, ScoreMetric, SearchHit,
    SearchMode, SearchRequest, VectorCandidate, VectorQuery,
};
use rrd_core::{
    digest, ProjectionId, ProjectionStamp, ProjectionState, Result, ScopeId, VectorValue,
    DATA_RUNTIME_CONTRACT_VERSION,
};
use serde::{Deserialize, Serialize};
use std::cmp::{Ordering, Reverse};
use std::collections::{BTreeMap, BTreeSet, BinaryHeap};

mod config;
mod graph;
mod index;
mod score;
mod validate;

pub use config::*;
use graph::*;
pub use index::*;
use score::*;
use validate::*;

#[cfg(test)]
mod tests;
