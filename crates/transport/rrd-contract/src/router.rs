//! Model- and provider-neutral reasoning-tree routing contracts.
//!
//! A router proposes one bounded decision. It does not authorize work,
//! evaluate deterministic edge conditions, select physical query paths, or
//! mutate the active cursor. `RrdEngine` validates and applies any proposal.

use crate::{
    invalid, sha256_bytes, validate_sha256, AssembleContext, CanonicalId, CorrelationId,
    DataReference, ReasoningActiveCursor, ReasoningEdge, ReasoningRecipe, Result,
    MAX_CONTEXT_GRAPH_DEPTH, MAX_CONTEXT_ITEMS, MAX_CONTEXT_OUTPUT_BYTES, MAX_CONTEXT_QUERY_BYTES,
    MAX_CONTEXT_SEEDS, MAX_CONTEXT_STORAGE_KEYS, MAX_REASONING_EDGE_CONDITIONS,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

mod decision;
mod request;

pub use decision::*;
pub use request::*;
