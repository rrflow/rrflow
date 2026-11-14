//! Typed, append-only contract for the persisted reasoning runtime graph.
//!
//! A runtime commit is the unit of causality and durability. It may contain
//! claims, typed records, typed relations, and lifecycle events. Storage
//! adapters allocate a single global cursor for every mutation, hash-chain the
//! resulting changes, and reject a commit whose expected cursor is stale.

use crate::ident::{check_canonical, check_text, text_ident};
use crate::{
    digest, Claim, Error, GeoValue, Millis, ObjectReference, Predicate, Result, RuntimeGeo,
    RuntimeSchemaRegistry, RuntimeSeriesSample, RuntimeVector, SeriesValue, Subject,
    VectorNormalization, VectorValue,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

mod audit;
mod data_snapshot;
mod encoding;
mod graph;
mod ids;
mod model;
mod projection;
mod stamps;
mod transaction;
mod validate;

pub use audit::*;
pub use data_snapshot::*;
use encoding::*;
pub use graph::*;
pub use ids::*;
pub use model::*;
pub use projection::*;
pub use stamps::*;
pub use transaction::*;
use validate::*;

#[cfg(test)]
mod tests;
