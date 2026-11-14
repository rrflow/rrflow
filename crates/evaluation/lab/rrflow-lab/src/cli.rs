//! Run and combine comparison-lab tests. See eval/lab/README.md.
use rrflow_evaluation_core::report::{ArtifactReference, LabReport, SystemStatus};
use rrflow_evaluation_core::Availability;
use rrflow_evaluation_core::{canonical_json, sha256_hex, Result};
use rrflow_evaluation_core::{schema, Digest, Id};
use rrflow_lab::settings::{
    approve_all, build_plan, launch_digest, needs_approval, planned_files, runs_somewhere,
    waiting_for_approval, Definitions, LabPlan, LabSettings, PlannedTest, SystemState,
};
use rrflow_lab_benchmarks::catalog::{
    launch_arguments, launch_info, resolve_test, vectordbbench_arguments, Pod, SystemDefinition,
    TestDefinition,
};
use rrflow_lab_benchmarks::driver::{
    combine, empty_run, now_unix_ms, prepare_inputs, run_system_with_progress, SystemRun,
};
use rrflow_lab_benchmarks::import::{import_vectordbbench, MAX_IMPORT_BYTES};
use rrflow_lab_benchmarks::manifest::VectorComparisonManifest;
use rrflow_lab_benchmarks::references::{
    compare, ComparedSeek, Comparison, HardwareClass, ListedAnswer, MeasuredSpeed, ReferenceFile,
    TestConditions,
};
use rrflow_lab_datasets::knowledge::{read_repository_text, CHUNK_TABLE};
use rrflow_lab_definitions::content::{
    lab_files, pinned_keys, references_of, RrflowContent, RRFLOW_PRODUCT,
};
use rrflow_lab_definitions::home::{DefinitionHome, PublishKind, PublishOptions};
use rrflow_lab_definitions::pack::{
    build_pack, generate_key, read_pack, InstallOutcome, PackContent as _, PackRequest, PackStore,
    StoreStatus, TrustedKey, TrustedKeys, VerifiedPack, MAX_PACK_BYTES,
};
use rrflow_lab_evals::provider::ClaudeCode;
use rrflow_lab_evals::study::{
    audit_runs, check_study, plan_study, CaptureAudit, RequirementStatus, StudyCheck, StudyConfig,
    StudyRequirements,
};
use rrflow_lab_evals::tasks::{
    charged_usd, combine_task_runs, run_order, run_task, RunInputs, RunKey, TaskComparisonManifest,
    TaskRun,
};
use rrflow_lab_systems::adapters::custom::CustomProtocol;
use rrflow_lab_systems::adapters::rrflow::RrflowAdapter;
use rrflow_lab_systems::registry::connect;
use rrflow_lab_systems::sources::SourceList;
use serde::de::DeserializeOwned;
use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

mod args;
mod commands;
mod compare;
mod definitions;
mod runs;
mod settings;
mod store;
mod studies;

use args::*;
use commands::*;
use compare::*;
use definitions::*;
use runs::*;
use settings::*;
use store::*;
use studies::*;

pub(crate) fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    dispatch(&arguments).unwrap_or_else(|error| {
        eprintln!("rrflow-lab: {error}");
        ExitCode::FAILURE
    })
}
