//! Canonical installed-engram, deployment, and operator-configuration contracts.
//!
//! These values separate durable identity and resource policy from adapter
//! topology. They govern durable effects; they do not describe or constrain a
//! model's private reasoning process.

use crate::{
    invalid, sha256_bytes, AssembleContext, CanonicalId, QueryBudget, Result,
    MAX_CONTEXT_GRAPH_DEPTH, MAX_CONTEXT_ITEMS, MAX_CONTEXT_OUTPUT_BYTES, MAX_CONTEXT_STORAGE_KEYS,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;

mod cold_history;
mod configuration;
mod profile;
mod storage_limits;

pub use cold_history::*;
pub use configuration::*;
pub use profile::*;
pub use storage_limits::*;
