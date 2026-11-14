//! Versioned installation and project-attunement wire contracts.
//!
//! These types describe plans, durable job snapshots, and mutation requests.
//! They do not execute phases or own lifecycle state: `RrdEngine` is the sole
//! authority that may persist transitions and checkpoints.

use crate::{
    invalid, sha256_bytes, validate_sha256, CanonicalId, CorrelationId, DeploymentProfile,
    EstateConfiguration, InstalledEstateIdentity, MemorySeatDefinition, Result, SecurityAction,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

mod bootstrap;
mod derivation;
mod discovery;
pub use bootstrap::{
    InstallationBootstrap, InstallationBootstrapPolicy, InstallationRecordKind,
    InstallationSpecialization, InstallationStorageGeneration, INSTALLATION_BOOTSTRAP_VERSION,
    MAX_BOOTSTRAP_RECORD_BYTES, MAX_BOOTSTRAP_TEXT_BYTES,
};
pub use derivation::{
    attunement_capability, attunement_capability_configuration_sha256, default_source_parse_policy,
    derive_attunement_phases, derive_attunement_plan, AttunementCapability, AttunementEvidence,
    AttunementPrecondition, ATTUNEMENT_CAPABILITIES, ATTUNEMENT_EVIDENCE_FORMAT_VERSION,
    MAX_ATTUNEMENT_EVIDENCE_LANGUAGES,
};
pub use discovery::{
    AttunementJobPage, AttunementJobSummary, ListAttunementJobs, MAX_ATTUNEMENT_JOB_PAGE,
    MAX_ATTUNEMENT_JOB_PAGE_BYTES,
};

mod control;
mod digests;
mod installation;
mod job;
mod plan;
mod replan;
mod resume_cancel;
mod source_parse;
mod verification;

pub use control::*;
pub use digests::*;
pub use installation::*;
pub use job::*;
pub use plan::*;
pub use replan::*;
pub use resume_cancel::*;
pub use source_parse::*;
pub use verification::*;
