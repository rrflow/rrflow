//! A study is the small set of choices a person or a screen makes over a task catalog. The
//! requirements say what any study must do, why, and where that comes from. A check reports
//! each one, and a plan is written only once the binding ones are met. The audit then proves
//! from the runs themselves that the data arrived, because a study can look complete without it.
use crate::tasks::{
    exact_mcnemar, shuffle, CaptureField, SpendLimits, StudyStage, TaskArm, TaskComparisonManifest,
    TaskModel, TaskRun, TaskRunOutcome,
};
use rrflow_evaluation_core::Availability;
use rrflow_evaluation_core::{canonical_json, require, sha256_hex, Result};
use rrflow_evaluation_core::{Digest, Id};
use rrflow_lab_systems::sources::SourceList;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

mod audit;
mod evaluation;

pub use audit::*;
pub use evaluation::*;
