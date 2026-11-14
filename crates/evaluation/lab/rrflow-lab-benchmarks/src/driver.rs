//! Runs one system at a time so systems never compete for a machine, then combines
//! the runs into one report. A failing system stays visible and cannot vanish.
use crate::manifest::VectorComparisonManifest;
use crate::manifest::{QualitySpec, WorkloadSpec};
use rrflow_evaluation_core::report::TestIdentity;
use rrflow_evaluation_core::report::{
    ArtifactReference, DimensionDefinition, LabEvidenceClass, LabReport, MetricDefinition,
    MetricDirection, Observation, ReportSystem, SystemStatus, Verdict, VerdictOutcome,
    ViewDefinition, REPORT_SCHEMA_VERSION,
};
use rrflow_evaluation_core::Availability;
use rrflow_evaluation_core::{canonical_json, require, sha256_hex, Result};
use rrflow_evaluation_core::{Digest, Id};
use rrflow_lab_datasets::corpus::{exact_answers, generate, GroundTruth, VectorSet};
use rrflow_lab_datasets::corpus::{CorpusSource, CorpusSpec};
use rrflow_lab_systems::adapter::{CollectionSpec, PointBatch, SearchRequest, SystemAdapter};
use rrflow_lab_systems::binding::{SearchSetting, SystemBinding, SystemKind};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

mod cases;
mod report;

pub use cases::*;
pub use report::*;
