//! Persisted, replayable runtime tracing contract.
//!
//! `tracing` spans remain cheap process diagnostics. This contract is the
//! durable causal evidence that Connectome may freeze, replay, compare, and
//! correlate with committed runtime truth. Callers provide clocks and IDs so
//! tests and replays remain deterministic.

use crate::{
    AuditDecision, Error, Millis, ProjectionStamp, ReadStamp, ReasoningActiveCursor, Result,
    RuntimeCommit, RuntimeEvent, RuntimeEventSchema, RuntimeLogicalModel, RuntimeMutation,
    RuntimeProperties, RuntimePropertySchema, RuntimeSchemaRegistry, RuntimeType, RuntimeValue,
    RuntimeValueType, ScopeId, SnapshotId,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

mod convergence;
mod event;
mod ids;
mod validation;
mod vocabulary;

use convergence::*;
pub use event::*;
pub use ids::*;
use validation::*;
pub use vocabulary::*;

#[cfg(test)]
mod tests;
