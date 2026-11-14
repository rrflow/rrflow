//! Project-scoped external operator knowledge.
//!
//! RRD owns policy, evidence, and freshness decisions. An adapter owns its
//! external transaction and returns bounded identities plus plan evidence. No
//! type in this crate claims a transaction spanning RRD and the external
//! system.

use rrd_core::{
    digest, Error, ProjectionFamily, ProjectionStamp, ProjectionState, ProjectionWork, Result,
    ScopeId,
};
use rrd_vector::{EmbeddingModelBinding, ScoreMetric, SearchMode, SearchRequest, VectorRuntime};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[cfg(feature = "pgvector-postgres")]
mod pgvector_postgres;
#[cfg(feature = "pgvector-postgres")]
pub use pgvector_postgres::{
    PgvectorDeletePayload, PgvectorDeployment, PgvectorLiveAdapter, PgvectorSyncPayload,
    PGVECTOR_LIVE_IMPLEMENTATION_VERSION,
};

mod model;
mod pgvector_plan;
mod search;
mod sync;

pub use model::*;
pub use pgvector_plan::*;
pub use search::*;
pub use sync::*;

#[cfg(test)]
mod tests;
