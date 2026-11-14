use rrd_contract::{
    attunement_checkpoint_sha256, attunement_plan_sha256, attunement_verification_sha256,
    installation_plan_sha256, installation_result_sha256, AttunementFailure, AttunementJob,
    AttunementJobState, AttunementLease, AttunementPhase, AttunementPhaseCheckpoint,
    AttunementPhasePlan, AttunementPlan, AttunementRuntimeCoordinates, AttunementStatus,
    AttunementVerification, AttunementVerificationCheck, AttunementVerificationStatus,
    CancelAttunement, CanonicalId, DeploymentForm, DeploymentProfile, EndpointPresentation,
    EstateConfiguration, InstallationActionDisposition, InstallationActionKind,
    InstallationActionResult, InstallationBootstrap, InstallationBootstrapPolicy,
    InstallationManagedPath, InstallationManagedPathKind, InstallationPlan, InstallationPlanAction,
    InstallationRecordKind, InstallationRemovalRule, InstallationResult,
    InstallationSpecialization, InstallationStorageGeneration, InstallationTargetKind,
    InstalledEstateIdentity, MemorySeatDefinition, ResumeAttunement, SecurityAction,
    StorageProfileKind, ATTUNEMENT_CONTRACT_VERSION, INSTALLATION_PLAN_CONTRACT_VERSION,
    INSTALL_ATTUNEMENT_CONTRACT_VERSION,
};
use schemars::{schema_for, JsonSchema};
use serde::{Deserialize, Serialize};

// Test fixture only: runtime plans own their positions and may differ.
const ATTUNEMENT_PHASES: [AttunementPhase; 11] = [
    AttunementPhase::Connect,
    AttunementPhase::Inventory,
    AttunementPhase::Parse,
    AttunementPhase::Normalize,
    AttunementPhase::EntityLink,
    AttunementPhase::LexicalIndex,
    AttunementPhase::Embed,
    AttunementPhase::VectorIndex,
    AttunementPhase::Graph,
    AttunementPhase::Ground,
    AttunementPhase::Verify,
];

const CREATED_AT: u64 = 1_800_000_000_000;

mod golden;
mod operations;
mod validation;

use golden::*;
use operations::*;
