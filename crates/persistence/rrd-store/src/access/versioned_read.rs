//! Read-stamp-bound access to canonical semantic version entries.
//!
//! This is the normal state-read substrate shared by rrflowMX and rrflowKV.
//! It selects scope-bound typed version ranges, validates every returned key
//! against its value, authenticates each value against the stamped RFC 9162
//! root, and reports the complete bounded access shape. The authoritative
//! change log is not a range source here; replay, changefeed, archive, and
//! recovery keep their explicit paged-log APIs.

use super::proof_nodes::ProofNodes;
use super::runtime_state::{read_accumulator_node, validate_read_stamp, AccessRead};
use super::vector::{VectorSourceAddress, VectorSourceDelta, VectorSourceVersion};
use crate::key_codec::{prefix_end, KeyCodec};
use crate::keyspaces::{self, Space};
use crate::{Error, Result};
use rrd_core::{
    Predicate, ReadStamp, RuntimeChange, RuntimeId, RuntimeLogAccumulator, RuntimeLogicalModel,
    RuntimeMutation, RuntimeReadValidation, RuntimeRef, RuntimeType,
};
use serde::{Deserialize, Serialize};
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::ops::Bound;

mod metering;
mod ranges;
mod reads;
mod types;
mod validation;

use metering::*;
use ranges::*;
pub(crate) use reads::*;
pub use types::*;
use validation::*;

#[cfg(test)]
#[path = "versioned_identity_ranges.rs"]
mod identity_range_tests;

#[cfg(test)]
#[path = "vector_delta_read_tests.rs"]
mod vector_delta_read_tests;

#[cfg(test)]
#[path = "versioned_read_batch_tests.rs"]
mod versioned_read_batch_tests;

#[cfg(test)]
mod tests;
