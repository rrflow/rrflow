mod apply;
mod open;
mod planning;
mod profile;
mod records;
mod storage;
mod validation;

pub use profile::*;
pub use records::*;
pub(super) use storage::*;
use validation::*;

#[cfg(test)]
use super::clock::FixedClock;

use super::clock::{assess, observe_required, require_accepted, ClockSource, HostSystemClock};

use super::*;

use crate::engine::engram_layout::{
    create_engram_directories, existing_engram_entries, reject_existing_engram, safe_join,
    validate_engram_directories, validate_managed_relative, validate_recovery_engram,
    EngramFilesystem, EngramLayout, LOCATOR_RELATIVE_PATH, STORAGE_DIRECTORY,
};

use crate::engine::memory_engram::build_memory_engram_plan;

use crate::engine::token_key::{
    read_api_key_document_anchored as read_api_key_document,
    read_token_key_anchored as read_token_key, ApiKeyDocument,
};

use rrd_contract::{
    attunement_plan_sha256, installation_plan_sha256, installation_result_sha256, AttunementJob,
    AttunementJobState, AttunementPhase, AttunementPhasePlan, AttunementPlan, DeploymentProfile,
    EngramConfiguration, EngramConfigurationInput, InstallationActionDisposition,
    InstallationActionKind, InstallationActionResult, InstallationManagedPathKind,
    InstallationPlan, InstallationPlanAction, InstallationResult, InstallationTargetKind,
    InstalledEngramIdentity, PersistMemoryEngram, StorageProfileKind, ATTUNEMENT_CONTRACT_VERSION,
    INSTALLATION_PLAN_CONTRACT_VERSION, INSTALL_ATTUNEMENT_CONTRACT_VERSION,
    MAX_ATTUNEMENT_DESCRIPTION_BYTES, MAX_ATTUNEMENT_PHASES,
};

use rrd_core::digest::Sha256 as IncrementalSha256;

use rrd_security::{Principal, PrincipalKind, ResourceGrant, SecurityState, SECURITY_FORMAT};

use rrd_store::filesystem::{AnchoredPath, OpenOptions};

use rrd_store::{ControlTransition, RrflowKvInspector};

use serde::de::DeserializeOwned;

use std::collections::{BTreeMap, BTreeSet};

use std::io::{Read, Write};

const INSTALL_PROFILE_BYTES: &[u8] = include_bytes!("../../assets/install/default-profile-v3.json");

const DEFAULT_CONFIGURATION_BYTES: &[u8] =
    include_bytes!("../../assets/install/default-engram-configuration-v3.toml");

const CLYFFY_PROFILE_BYTES: &[u8] = include_bytes!("../../assets/install/clyffy-profile-v3.json");

const INSTALL_PROFILE_FORMAT: u16 = 3;

const LOCATOR_FORMAT: u16 = 2;

const INSTALLED_RECORD_FORMAT: u16 = 2;

const INSTALL_CHECKPOINT_FORMAT: u16 = 2;

pub(super) const INSTALLED_RECORD_KEY: &str = "server/state/install/record";

const INSTALL_CHECKPOINT_KEY: &str = "server/state/install/checkpoint";

pub(super) const MAX_LOCATOR_BYTES: u64 = 64 * 1024;

const MAX_CONFIGURATION_BYTES: u64 = 64 * 1024;

const MAX_ATTUNEMENT_PROPOSAL_BYTES: u64 = 64 * 1024;

pub const INSTALLATION_PREVIEW_FORMAT_VERSION: u16 = 1;

pub const ATTUNEMENT_PROPOSAL_FORMAT_VERSION: u16 = 1;

/// Explicit, preview-bound choices that affect installation authority.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct InstallationPlanningOptions {
    pub attunement_proposal: Option<AttunementProposal>,
    pub allow_project_source_read: bool,
    /// Seal a policy under which the served engine keeps the project attuned;
    /// `None` keeps whatever the configuration file says.
    pub attunement_follow: Option<rrd_contract::AttunementFollowPolicy>,
}

mod admission;

mod bootstrap;

mod inventory;

mod lease;

mod policy;

pub(super) mod recovery;

use inventory::project_inventory_sha256_excluding;

pub(super) use inventory::{
    prepare_project_inventory, project_entries, project_inventory_sha256,
    read_admitted_inventory_file, validate_selected_inventory_path, ProjectEntry,
};

#[allow(unused_imports)] // Parse admission consumes this when its worker is wired.
pub(in crate::engine) use inventory::{project_change_fingerprint, read_selected_source};

use recovery::{
    acknowledge_intent, load_or_create_intent, materialize_secret, read_existing_intent,
    InstallApplyStage, InstallRecoveryState,
};
