//! Does a capability help an agent do real work, at what cost? Each task is a real change
//! with a known answer. The design is frozen before any run, runs are paired across arms,
//! and the plan the agent wrote is graded mechanically, because a tool that returned the right
//! thing proves nothing if the agent did not use it. Results use the lab's ordinary report.
use crate::provider::{AgentProvider, AgentReply, AgentRequest};
use rrflow_evaluation_core::report::{
    ArtifactReference, DimensionDefinition, LabEvidenceClass, LabReport, MetricDefinition,
    MetricDirection, Observation, ReportSystem, SystemStatus, TestIdentity, Verdict,
    VerdictOutcome, ViewDefinition, REPORT_SCHEMA_VERSION,
};
use rrflow_evaluation_core::Availability;
use rrflow_evaluation_core::{canonical_json, require, sha256_hex, Result};
use rrflow_evaluation_core::{Digest, Id};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

mod execution;
mod manifest;
mod verdicts;

pub use execution::*;
pub use manifest::*;
pub use verdicts::*;
