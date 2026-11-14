//! Portable authenticated append-only log primitives.
//!
//! The tree shape and domain separation follow RFC 9162 section 2.1: empty
//! roots hash the empty string, leaves hash `0x00 || input`, and parents hash
//! `0x01 || left || right`. RRD persists complete subtree nodes and a compact
//! frontier, so an append writes at most one node per tree level and a point
//! read can be authenticated with a logarithmic inclusion path.

use crate::{digest, Error, Result, RuntimeChange, RuntimeId, ScopeId};
use serde::{Deserialize, Serialize};

mod accumulator;
mod generation;

pub use accumulator::*;
pub use generation::*;

#[cfg(test)]
mod tests;
