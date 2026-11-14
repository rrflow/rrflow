//! One persisted catalogue and schema contract for every logical runtime model.

use crate::{
    Error, Result, RuntimeEvent, RuntimeMutation, RuntimeProperties, RuntimeRecord,
    RuntimeRelation, RuntimeType, RuntimeValue,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

mod organisation;
mod registry;
mod types;
mod validation;

pub use organisation::*;
pub use registry::*;
pub use types::*;
use validation::*;

#[cfg(test)]
mod tests;
