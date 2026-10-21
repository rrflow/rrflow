//! Versioned installation and project-attunement wire contracts.
//!
//! These types describe plans, durable job snapshots, and mutation requests.
//! They do not execute phases or own lifecycle state: `RrdEngine` is the sole
//! authority that may persist transitions and checkpoints.

use crate::{
    invalid, sha256_bytes, validate_sha256, CanonicalId, DeploymentProfile, EstateConfiguration,
    InstalledEstateIdentity, MemorySeatDefinition, Result, SecurityAction,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const INSTALL_ATTUNEMENT_CONTRACT_VERSION: u16 = 1;
/// Independent identity and initial-root binding are mandatory for new plans.
/// Attunement and installation-result envelopes retain their own version.
pub const INSTALLATION_PLAN_CONTRACT_VERSION: u16 = 2;
pub const MAX_INSTALLATION_ACTIONS: usize = 16;
pub const MAX_ATTUNEMENT_PHASES: usize = 32;
pub const MAX_ATTUNEMENT_FAILURE_BYTES: usize = 4_096;
pub const MAX_ATTUNEMENT_CANCEL_REASON_BYTES: usize = 4_096;
pub const MAX_ATTUNEMENT_VERIFICATION_CHECKS: usize = 256;

/// The canonical order in which an estate becomes queryable and verified.
///
/// A phase may perform no content work when its inputs are unchanged or its
/// optional capability is not configured, but it still produces a checkpoint
/// that explains and binds that outcome. Callers cannot skip phase positions.
pub const ATTUNEMENT_PHASES: [AttunementPhase; 11] = [
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

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum AttunementPhase {
    Connect,
    Inventory,
    Parse,
    Normalize,
    EntityLink,
    LexicalIndex,
    Embed,
    VectorIndex,
    Graph,
    Ground,
    Verify,
}

impl AttunementPhase {
    pub const fn sequence(self) -> u16 {
        match self {
            Self::Connect => 1,
            Self::Inventory => 2,
            Self::Parse => 3,
            Self::Normalize => 4,
            Self::EntityLink => 5,
            Self::LexicalIndex => 6,
            Self::Embed => 7,
            Self::VectorIndex => 8,
            Self::Graph => 9,
            Self::Ground => 10,
            Self::Verify => 11,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum InstallationTargetKind {
    FreshProject,
    ExistingProject,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum InstallationActionKind {
    ValidateProject,
    CreateEstateDirectories,
    CreateStorageRoot,
    PrepareCredentials,
    InitializeInstance,
    CreateAttunementJob,
    PublishProjectLocator,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum InstallationActionDisposition {
    Create,
    Update,
    Unchanged,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InstallationPlanAction {
    pub kind: InstallationActionKind,
    pub disposition: InstallationActionDisposition,
    pub input_sha256: String,
    pub estimated_write_bytes: u64,
}

impl InstallationPlanAction {
    fn validate(&self) -> Result<()> {
        validate_sha256(&self.input_sha256, "installation action input_sha256")?;
        if self.disposition == InstallationActionDisposition::Unchanged
            && self.estimated_write_bytes != 0
        {
            return invalid("an unchanged installation action cannot estimate written bytes");
        }
        Ok(())
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum InstallationManagedPathKind {
    EstateDirectory,
    StorageDirectory,
    StorageRootsDirectory,
    CredentialDirectory,
    StorageRoot,
    TokenKey,
    OperatorCredential,
    ProjectLocator,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum InstallationRemovalRule {
    RemoveIfOwnedDigestMatches,
    RemoveIfOwnedTreeDigestMatches,
    RemoveIfOwnedAndEmpty,
}

/// One RRFlow-owned path whose state is bound by the preview. The path is
/// project-relative so the plan cannot silently redirect effects elsewhere.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InstallationManagedPath {
    pub kind: InstallationManagedPathKind,
    pub relative_path: String,
    pub precondition_sha256: String,
    pub removal_rule: InstallationRemovalRule,
}

impl InstallationManagedPath {
    fn validate(&self) -> Result<()> {
        validate_project_relative_path(&self.relative_path)?;
        validate_sha256(
            &self.precondition_sha256,
            "installation managed path precondition_sha256",
        )
    }
}

/// A deterministic preview of installation work. Applying this plan must use
/// the same digest; implementations cannot silently re-plan during mutation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InstallationPlan {
    pub contract_version: u16,
    /// Explicit 256-bit identity input, generated once per independent install.
    pub installation_identity: String,
    /// Initial canonical project root; not the permanent estate identity.
    pub project_root_sha256: String,
    pub id: CanonicalId,
    pub target: InstalledEstateIdentity,
    pub target_kind: InstallationTargetKind,
    pub deployment: DeploymentProfile,
    pub product_version: String,
    pub executable_sha256: String,
    pub profile_id: CanonicalId,
    pub profile_sha256: String,
    pub project_precondition_sha256: String,
    pub storage_root_id: CanonicalId,
    pub configuration: EstateConfiguration,
    pub initial_seat: MemorySeatDefinition,
    pub initial_principal_id: CanonicalId,
    pub initial_grants: Vec<SecurityAction>,
    pub credential_bytes: u16,
    pub inactive_capabilities: Vec<CanonicalId>,
    pub managed_paths: Vec<InstallationManagedPath>,
    pub attunement_plan_id: CanonicalId,
    pub attunement_plan_sha256: String,
    pub actions: Vec<InstallationPlanAction>,
    pub plan_sha256: String,
}

impl InstallationPlan {
    pub fn validate(&self) -> Result<()> {
        if self.contract_version != INSTALLATION_PLAN_CONTRACT_VERSION {
            return invalid("unsupported installation plan contract version; expected 2");
        }
        validate_sha256(&self.installation_identity, "installation identity")?;
        validate_sha256(
            &self.project_root_sha256,
            "installation project_root_sha256",
        )?;
        self.target.validate()?;
        self.deployment.validate()?;
        validate_bounded_text(&self.product_version, "installation product_version", 64)?;
        validate_sha256(&self.executable_sha256, "installation executable_sha256")?;
        validate_sha256(&self.profile_sha256, "installation profile_sha256")?;
        validate_sha256(
            &self.project_precondition_sha256,
            "installation project_precondition_sha256",
        )?;
        self.configuration.validate()?;
        self.initial_seat.validate()?;
        if self.initial_grants.is_empty() {
            return invalid("installation initial_grants cannot be empty");
        }
        validate_sorted_unique(
            &self.initial_grants,
            "installation initial_grants must be sorted and unique",
        )?;
        if !(32..=4_096).contains(&self.credential_bytes) {
            return invalid("installation credential_bytes is outside its bound");
        }
        validate_sorted_unique(
            &self.inactive_capabilities,
            "installation inactive_capabilities must be sorted and unique",
        )?;
        let expected_paths = [
            (
                InstallationManagedPathKind::EstateDirectory,
                InstallationRemovalRule::RemoveIfOwnedAndEmpty,
            ),
            (
                InstallationManagedPathKind::StorageDirectory,
                InstallationRemovalRule::RemoveIfOwnedAndEmpty,
            ),
            (
                InstallationManagedPathKind::StorageRootsDirectory,
                InstallationRemovalRule::RemoveIfOwnedAndEmpty,
            ),
            (
                InstallationManagedPathKind::CredentialDirectory,
                InstallationRemovalRule::RemoveIfOwnedAndEmpty,
            ),
            (
                InstallationManagedPathKind::StorageRoot,
                InstallationRemovalRule::RemoveIfOwnedTreeDigestMatches,
            ),
            (
                InstallationManagedPathKind::TokenKey,
                InstallationRemovalRule::RemoveIfOwnedDigestMatches,
            ),
            (
                InstallationManagedPathKind::OperatorCredential,
                InstallationRemovalRule::RemoveIfOwnedDigestMatches,
            ),
            (
                InstallationManagedPathKind::ProjectLocator,
                InstallationRemovalRule::RemoveIfOwnedDigestMatches,
            ),
        ];
        if self.managed_paths.len() != expected_paths.len() {
            return invalid("installation plan must bind every managed path exactly once");
        }
        let mut paths = BTreeSet::new();
        for (managed, (expected_kind, expected_rule)) in
            self.managed_paths.iter().zip(expected_paths)
        {
            managed.validate()?;
            if managed.kind != expected_kind
                || managed.removal_rule != expected_rule
                || !paths.insert(&managed.relative_path)
            {
                return invalid("installation managed paths are duplicated or out of order");
            }
        }
        validate_sha256(
            &self.attunement_plan_sha256,
            "installation attunement_plan_sha256",
        )?;
        if self.actions.is_empty() || self.actions.len() > MAX_INSTALLATION_ACTIONS {
            return invalid("installation action count is outside its bound");
        }
        let expected = [
            InstallationActionKind::ValidateProject,
            InstallationActionKind::CreateEstateDirectories,
            InstallationActionKind::CreateStorageRoot,
            InstallationActionKind::PrepareCredentials,
            InstallationActionKind::InitializeInstance,
            InstallationActionKind::CreateAttunementJob,
            InstallationActionKind::PublishProjectLocator,
        ];
        if self.actions.len() != expected.len() {
            return invalid("installation plan must contain every canonical action exactly once");
        }
        for (action, expected_kind) in self.actions.iter().zip(expected) {
            action.validate()?;
            if action.kind != expected_kind {
                return invalid("installation actions are missing, duplicated, or out of order");
            }
        }
        validate_sha256(&self.plan_sha256, "installation plan_sha256")?;
        if self.plan_sha256 != installation_plan_sha256(self)? {
            return invalid("installation plan digest does not match its content");
        }
        Ok(())
    }

    pub fn validate_attunement_plan(&self, plan: &AttunementPlan) -> Result<()> {
        self.validate()?;
        plan.validate()?;
        if self.target != plan.target
            || self.attunement_plan_id != plan.id
            || self.attunement_plan_sha256 != plan.plan_sha256
        {
            return invalid("installation plan does not bind the supplied attunement plan");
        }
        Ok(())
    }

    pub fn managed_path(&self, kind: InstallationManagedPathKind) -> &InstallationManagedPath {
        let index = match kind {
            InstallationManagedPathKind::EstateDirectory => 0,
            InstallationManagedPathKind::StorageDirectory => 1,
            InstallationManagedPathKind::StorageRootsDirectory => 2,
            InstallationManagedPathKind::CredentialDirectory => 3,
            InstallationManagedPathKind::StorageRoot => 4,
            InstallationManagedPathKind::TokenKey => 5,
            InstallationManagedPathKind::OperatorCredential => 6,
            InstallationManagedPathKind::ProjectLocator => 7,
        };
        &self.managed_paths[index]
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InstallationActionResult {
    pub kind: InstallationActionKind,
    pub output_sha256: String,
}

impl InstallationActionResult {
    fn validate(&self) -> Result<()> {
        validate_sha256(
            &self.output_sha256,
            "installation action result output_sha256",
        )
    }
}

/// The durable receipt returned after a previously previewed plan is applied.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InstallationResult {
    pub contract_version: u16,
    pub plan_id: CanonicalId,
    pub plan_sha256: String,
    pub attunement_job_id: CanonicalId,
    pub action_results: Vec<InstallationActionResult>,
    pub runtime_manifest_sha256: String,
    pub runtime_cursor: u64,
    pub control_journal_sequence: u64,
    pub installed_record_sha256: String,
    pub credential_sha256: String,
    pub locator_sha256: String,
    pub applied_at_unix_ms: u64,
    pub idempotent_replay: bool,
    pub result_sha256: String,
}

impl InstallationResult {
    pub fn validate_for(&self, plan: &InstallationPlan) -> Result<()> {
        plan.validate()?;
        validate_contract_version(self.contract_version)?;
        if self.plan_id != plan.id || self.plan_sha256 != plan.plan_sha256 {
            return invalid("installation result does not bind its plan identity and digest");
        }
        validate_sha256(&self.plan_sha256, "installation result plan_sha256")?;
        if self.action_results.len() != plan.actions.len() {
            return invalid("installation result does not cover every planned action");
        }
        for (result, action) in self.action_results.iter().zip(&plan.actions) {
            result.validate()?;
            if result.kind != action.kind {
                return invalid("installation result actions do not match plan order");
            }
        }
        validate_sha256(
            &self.runtime_manifest_sha256,
            "installation result runtime_manifest_sha256",
        )?;
        if self.control_journal_sequence == 0 {
            return invalid("installation result control_journal_sequence must be nonzero");
        }
        validate_sha256(
            &self.installed_record_sha256,
            "installation result installed_record_sha256",
        )?;
        validate_sha256(
            &self.credential_sha256,
            "installation result credential_sha256",
        )?;
        validate_sha256(&self.locator_sha256, "installation result locator_sha256")?;
        if self.applied_at_unix_ms == 0 {
            return invalid("installation result applied_at_unix_ms must be nonzero");
        }
        validate_sha256(&self.result_sha256, "installation result_sha256")?;
        if self.result_sha256 != installation_result_sha256(self)? {
            return invalid("installation result digest does not match its content");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AttunementPhasePlan {
    pub phase: AttunementPhase,
    pub sequence: u16,
    pub configuration_sha256: String,
    pub estimated_items: u64,
    pub estimated_input_bytes: u64,
}

impl AttunementPhasePlan {
    fn validate(&self) -> Result<()> {
        if self.sequence != self.phase.sequence() {
            return invalid("attunement phase sequence does not match its canonical phase");
        }
        validate_sha256(
            &self.configuration_sha256,
            "attunement phase configuration_sha256",
        )
    }
}

/// A bounded, content-addressed plan over one project source snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AttunementPlan {
    pub contract_version: u16,
    pub id: CanonicalId,
    pub target: InstalledEstateIdentity,
    pub source_sha256: String,
    pub phases: Vec<AttunementPhasePlan>,
    pub estimated_min_duration_ms: u64,
    pub estimated_max_duration_ms: u64,
    pub plan_sha256: String,
}

impl AttunementPlan {
    pub fn validate(&self) -> Result<()> {
        validate_contract_version(self.contract_version)?;
        self.target.validate()?;
        validate_sha256(&self.source_sha256, "attunement source_sha256")?;
        if self.phases.len() != ATTUNEMENT_PHASES.len() || self.phases.len() > MAX_ATTUNEMENT_PHASES
        {
            return invalid("attunement plan must contain every canonical phase exactly once");
        }
        for (phase, expected) in self.phases.iter().zip(ATTUNEMENT_PHASES) {
            phase.validate()?;
            if phase.phase != expected {
                return invalid("attunement phases are missing, duplicated, or out of order");
            }
        }
        if self.estimated_min_duration_ms > self.estimated_max_duration_ms {
            return invalid("attunement duration estimate is inverted");
        }
        validate_sha256(&self.plan_sha256, "attunement plan_sha256")?;
        if self.plan_sha256 != attunement_plan_sha256(self)? {
            return invalid("attunement plan digest does not match its content");
        }
        Ok(())
    }

    fn phase(&self, phase: AttunementPhase) -> &AttunementPhasePlan {
        &self.phases[usize::from(phase.sequence() - 1)]
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AttunementRuntimeCoordinates {
    pub runtime_manifest_sha256: String,
    pub runtime_cursor: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema_revision: Option<u64>,
    pub catalogue_revision: u64,
}

impl AttunementRuntimeCoordinates {
    fn validate(&self) -> Result<()> {
        validate_sha256(
            &self.runtime_manifest_sha256,
            "attunement runtime_manifest_sha256",
        )?;
        if self.schema_revision == Some(0) {
            return invalid("attunement schema revision zero is not installed");
        }
        Ok(())
    }
}

/// One durably committed phase output. Checkpoints form a digest chain: the
/// first input is the plan source and each later input is the prior output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AttunementPhaseCheckpoint {
    pub contract_version: u16,
    pub job_id: CanonicalId,
    pub plan_sha256: String,
    pub phase: AttunementPhase,
    pub sequence: u16,
    pub configuration_sha256: String,
    pub input_sha256: String,
    pub output_sha256: String,
    pub coordinates: AttunementRuntimeCoordinates,
    pub committed_at_unix_ms: u64,
    pub checkpoint_sha256: String,
}

impl AttunementPhaseCheckpoint {
    pub fn validate_for(
        &self,
        plan: &AttunementPlan,
        job_id: &CanonicalId,
        previous: Option<&Self>,
    ) -> Result<()> {
        plan.validate()?;
        validate_contract_version(self.contract_version)?;
        if &self.job_id != job_id || self.plan_sha256 != plan.plan_sha256 {
            return invalid("attunement checkpoint does not bind its job and plan");
        }
        if self.sequence != self.phase.sequence() {
            return invalid("attunement checkpoint sequence does not match its phase");
        }
        let planned = plan.phase(self.phase);
        if self.configuration_sha256 != planned.configuration_sha256 {
            return invalid("attunement checkpoint configuration digest differs from its plan");
        }
        validate_sha256(&self.input_sha256, "attunement checkpoint input_sha256")?;
        validate_sha256(&self.output_sha256, "attunement checkpoint output_sha256")?;
        match previous {
            None if self.sequence == 1 && self.input_sha256 == plan.source_sha256 => {}
            Some(previous)
                if self.sequence == previous.sequence + 1
                    && self.input_sha256 == previous.output_sha256 =>
            {
                if self.committed_at_unix_ms < previous.committed_at_unix_ms {
                    return invalid("attunement checkpoint commit time moved backwards");
                }
                if self.coordinates.runtime_cursor < previous.coordinates.runtime_cursor
                    || self.coordinates.schema_revision < previous.coordinates.schema_revision
                    || self.coordinates.catalogue_revision < previous.coordinates.catalogue_revision
                {
                    return invalid("attunement checkpoint coordinates moved backwards");
                }
            }
            _ => return invalid("attunement checkpoint skipped a phase or broke its digest chain"),
        }
        self.coordinates.validate()?;
        if self.committed_at_unix_ms == 0 {
            return invalid("attunement checkpoint commit time must be greater than zero");
        }
        validate_sha256(&self.checkpoint_sha256, "attunement checkpoint_sha256")?;
        if self.checkpoint_sha256 != attunement_checkpoint_sha256(self)? {
            return invalid("attunement checkpoint digest does not match its content");
        }
        Ok(())
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum AttunementJobState {
    Pending,
    Running,
    Paused,
    CancelRequested,
    Cancelled,
    Failed,
    Succeeded,
}

impl AttunementJobState {
    pub const fn permits(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::Pending, Self::Running | Self::CancelRequested)
                | (
                    Self::Running,
                    Self::Running
                        | Self::Paused
                        | Self::CancelRequested
                        | Self::Failed
                        | Self::Succeeded
                )
                | (Self::Paused, Self::Running | Self::CancelRequested)
                | (Self::Failed, Self::Running | Self::CancelRequested)
                | (Self::CancelRequested, Self::Cancelled)
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AttunementLease {
    pub id: CanonicalId,
    pub holder: CanonicalId,
    pub generation: u64,
    pub acquired_at_unix_ms: u64,
    pub expires_at_unix_ms: u64,
}

impl AttunementLease {
    fn validate(&self) -> Result<()> {
        if self.generation == 0
            || self.acquired_at_unix_ms == 0
            || self.expires_at_unix_ms <= self.acquired_at_unix_ms
        {
            return invalid("attunement lease generation or time interval is invalid");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AttunementFailure {
    pub phase: AttunementPhase,
    pub code: CanonicalId,
    pub message: String,
    pub retryable: bool,
    pub evidence_sha256: String,
    pub observed_at_unix_ms: u64,
}

impl AttunementFailure {
    fn validate(&self) -> Result<()> {
        validate_bounded_text(
            &self.message,
            "attunement failure message",
            MAX_ATTUNEMENT_FAILURE_BYTES,
        )?;
        validate_sha256(&self.evidence_sha256, "attunement failure evidence_sha256")?;
        if self.observed_at_unix_ms == 0 {
            return invalid("attunement failure observation time must be greater than zero");
        }
        Ok(())
    }
}

/// The authoritative durable job snapshot projected by `RrdEngine`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AttunementJob {
    pub contract_version: u16,
    pub id: CanonicalId,
    pub plan_id: CanonicalId,
    pub plan_sha256: String,
    pub revision: u64,
    pub state: AttunementJobState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_phase: Option<AttunementPhase>,
    #[serde(default)]
    pub checkpoints: Vec<AttunementPhaseCheckpoint>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lease: Option<AttunementLease>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure: Option<AttunementFailure>,
    pub created_at_unix_ms: u64,
    pub updated_at_unix_ms: u64,
}

impl AttunementJob {
    pub fn validate(&self, plan: &AttunementPlan) -> Result<()> {
        plan.validate()?;
        validate_contract_version(self.contract_version)?;
        if self.plan_id != plan.id || self.plan_sha256 != plan.plan_sha256 {
            return invalid("attunement job does not bind its plan identity and digest");
        }
        if self.revision == 0 {
            return invalid("attunement job revision must be greater than zero");
        }
        if self.created_at_unix_ms == 0 || self.updated_at_unix_ms < self.created_at_unix_ms {
            return invalid("attunement job timestamps are invalid");
        }
        if self.checkpoints.len() > ATTUNEMENT_PHASES.len() {
            return invalid("attunement job has more checkpoints than canonical phases");
        }
        let mut previous = None;
        for (checkpoint, expected_phase) in self.checkpoints.iter().zip(ATTUNEMENT_PHASES) {
            if checkpoint.phase != expected_phase {
                return invalid("attunement job checkpoints are not a canonical phase prefix");
            }
            checkpoint.validate_for(plan, &self.id, previous)?;
            if checkpoint.committed_at_unix_ms < self.created_at_unix_ms
                || checkpoint.committed_at_unix_ms > self.updated_at_unix_ms
            {
                return invalid("attunement checkpoint lies outside the job time interval");
            }
            previous = Some(checkpoint);
        }

        let next_phase = self.next_phase();
        match self.state {
            AttunementJobState::Pending => {
                if self.revision != 1
                    || !self.checkpoints.is_empty()
                    || self.current_phase.is_some()
                    || self.lease.is_some()
                    || self.failure.is_some()
                {
                    return invalid("a pending attunement job cannot contain execution state");
                }
            }
            AttunementJobState::Running => {
                if self.current_phase != next_phase || next_phase.is_none() || self.lease.is_none()
                {
                    return invalid("a running attunement job must lease its next canonical phase");
                }
                if self.failure.is_some() {
                    return invalid("a running attunement job cannot retain a failure");
                }
            }
            AttunementJobState::Paused => {
                if self.current_phase != next_phase
                    || next_phase.is_none()
                    || self.lease.is_some()
                    || self.failure.is_some()
                {
                    return invalid(
                        "a paused attunement job must identify its unleased next phase",
                    );
                }
            }
            AttunementJobState::CancelRequested => {
                if self.current_phase != next_phase
                    || next_phase.is_none()
                    || self.failure.is_some()
                {
                    return invalid("a cancellation request must identify an incomplete phase");
                }
            }
            AttunementJobState::Cancelled => {
                if self.current_phase.is_some() || self.lease.is_some() || self.failure.is_some() {
                    return invalid("a cancelled attunement job must be terminal and unleased");
                }
            }
            AttunementJobState::Failed => {
                let Some(failure) = &self.failure else {
                    return invalid("a failed attunement job must carry failure evidence");
                };
                failure.validate()?;
                if self.current_phase != next_phase
                    || next_phase != Some(failure.phase)
                    || self.lease.is_some()
                    || failure.observed_at_unix_ms > self.updated_at_unix_ms
                {
                    return invalid("attunement failure is not bound to the next phase");
                }
            }
            AttunementJobState::Succeeded => {
                if self.checkpoints.len() != ATTUNEMENT_PHASES.len()
                    || self.current_phase.is_some()
                    || self.lease.is_some()
                    || self.failure.is_some()
                {
                    return invalid("a succeeded attunement job must checkpoint every phase");
                }
            }
        }
        if let Some(lease) = &self.lease {
            lease.validate()?;
            if lease.acquired_at_unix_ms < self.created_at_unix_ms {
                return invalid("attunement lease predates its job");
            }
        }
        if self
            .failure
            .as_ref()
            .is_some_and(|failure| failure.observed_at_unix_ms < self.created_at_unix_ms)
        {
            return invalid("attunement failure predates its job");
        }
        Ok(())
    }

    pub fn validate_transition(&self, next: &Self, plan: &AttunementPlan) -> Result<()> {
        self.validate(plan)?;
        next.validate(plan)?;
        if self.id != next.id
            || self.plan_id != next.plan_id
            || self.plan_sha256 != next.plan_sha256
            || self.created_at_unix_ms != next.created_at_unix_ms
        {
            return invalid("attunement transition changed immutable job identity");
        }
        if next.revision
            != self
                .revision
                .checked_add(1)
                .ok_or_else(|| crate::ContractError("attunement job revision overflowed".into()))?
            || next.updated_at_unix_ms <= self.updated_at_unix_ms
        {
            return invalid("attunement transition must advance revision and update time once");
        }
        if !self.state.permits(next.state) {
            return invalid("attunement job state transition is not permitted");
        }
        if next.checkpoints.len() < self.checkpoints.len()
            || next.checkpoints.len() > self.checkpoints.len() + 1
            || !next.checkpoints.starts_with(&self.checkpoints)
        {
            return invalid("attunement transition removed, changed, or skipped checkpoints");
        }
        let appended_checkpoint = next.checkpoints.len() == self.checkpoints.len() + 1;
        if appended_checkpoint
            && (self.state != AttunementJobState::Running
                || !matches!(
                    next.state,
                    AttunementJobState::Running | AttunementJobState::Succeeded
                ))
        {
            return invalid("only a running worker may append one committed checkpoint");
        }
        if next.state == AttunementJobState::Succeeded && !appended_checkpoint {
            return invalid("attunement success must append the final verify checkpoint");
        }
        if self.state == AttunementJobState::Running
            && next.state == AttunementJobState::Running
            && !appended_checkpoint
        {
            let lease_advanced =
                self.lease
                    .as_ref()
                    .zip(next.lease.as_ref())
                    .is_some_and(|(before, after)| {
                        before.id == after.id
                            && before.holder == after.holder
                            && after.generation > before.generation
                            && after.expires_at_unix_ms > before.expires_at_unix_ms
                    });
            if !lease_advanced {
                return invalid(
                    "a running self-transition must checkpoint work or renew its lease",
                );
            }
        }
        if self.state == AttunementJobState::Failed
            && next.state == AttunementJobState::Running
            && !self
                .failure
                .as_ref()
                .is_some_and(|failure| failure.retryable)
        {
            return invalid("a non-retryable attunement failure cannot resume");
        }
        Ok(())
    }

    pub fn next_phase(&self) -> Option<AttunementPhase> {
        ATTUNEMENT_PHASES.get(self.checkpoints.len()).copied()
    }

    fn last_checkpoint_sha256(&self) -> Option<&str> {
        self.checkpoints
            .last()
            .map(|checkpoint| checkpoint.checkpoint_sha256.as_str())
    }
}

/// Read response for an engine-owned job. Events may point at this snapshot;
/// they never replace it as lifecycle authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AttunementStatus {
    pub contract_version: u16,
    pub observed_at_unix_ms: u64,
    pub job: AttunementJob,
}

impl AttunementStatus {
    pub fn validate(&self, plan: &AttunementPlan) -> Result<()> {
        validate_contract_version(self.contract_version)?;
        self.job.validate(plan)?;
        if self.observed_at_unix_ms < self.job.updated_at_unix_ms {
            return invalid("attunement status observation predates the job snapshot");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResumeAttunement {
    pub contract_version: u16,
    pub job_id: CanonicalId,
    pub plan_sha256: String,
    pub expected_job_revision: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_checkpoint_sha256: Option<String>,
    pub resume_from_phase: AttunementPhase,
    pub requested_at_unix_ms: u64,
}

impl ResumeAttunement {
    pub fn validate_for(&self, job: &AttunementJob, plan: &AttunementPlan) -> Result<()> {
        validate_contract_version(self.contract_version)?;
        job.validate(plan)?;
        if !matches!(
            job.state,
            AttunementJobState::Paused | AttunementJobState::Failed
        ) || (job.state == AttunementJobState::Failed
            && !job
                .failure
                .as_ref()
                .is_some_and(|failure| failure.retryable))
        {
            return invalid("only a paused or retryable failed attunement job may resume");
        }
        validate_job_command_binding(
            &self.job_id,
            &self.plan_sha256,
            self.expected_job_revision,
            self.last_checkpoint_sha256.as_deref(),
            self.requested_at_unix_ms,
            job,
        )?;
        if job.next_phase() != Some(self.resume_from_phase) {
            return invalid("attunement resume phase does not match the next canonical phase");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CancelAttunement {
    pub contract_version: u16,
    pub job_id: CanonicalId,
    pub plan_sha256: String,
    pub expected_job_revision: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_checkpoint_sha256: Option<String>,
    pub reason: String,
    pub requested_at_unix_ms: u64,
}

impl CancelAttunement {
    pub fn validate_for(&self, job: &AttunementJob, plan: &AttunementPlan) -> Result<()> {
        validate_contract_version(self.contract_version)?;
        job.validate(plan)?;
        if !matches!(
            job.state,
            AttunementJobState::Pending
                | AttunementJobState::Running
                | AttunementJobState::Paused
                | AttunementJobState::Failed
        ) {
            return invalid("attunement job is not in a cancellable state");
        }
        validate_bounded_text(
            &self.reason,
            "attunement cancellation reason",
            MAX_ATTUNEMENT_CANCEL_REASON_BYTES,
        )?;
        validate_job_command_binding(
            &self.job_id,
            &self.plan_sha256,
            self.expected_job_revision,
            self.last_checkpoint_sha256.as_deref(),
            self.requested_at_unix_ms,
            job,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AttunementVerificationStatus {
    Passed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AttunementVerificationCheck {
    pub id: CanonicalId,
    pub passed: bool,
    pub evidence_sha256: String,
}

impl AttunementVerificationCheck {
    fn validate(&self) -> Result<()> {
        validate_sha256(
            &self.evidence_sha256,
            "attunement verification evidence_sha256",
        )
    }
}

/// Evidence envelope bound to the authoritative job snapshot. A successful
/// envelope requires a succeeded job and the final `verify` checkpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AttunementVerification {
    pub contract_version: u16,
    pub job_id: CanonicalId,
    pub plan_sha256: String,
    pub job_revision: u64,
    pub last_checkpoint_sha256: String,
    pub runtime_manifest_sha256: String,
    pub status: AttunementVerificationStatus,
    pub checks: Vec<AttunementVerificationCheck>,
    pub verified_at_unix_ms: u64,
    pub verification_sha256: String,
}

impl AttunementVerification {
    pub fn validate_for(&self, job: &AttunementJob, plan: &AttunementPlan) -> Result<()> {
        validate_contract_version(self.contract_version)?;
        job.validate(plan)?;
        if self.job_id != job.id
            || self.plan_sha256 != job.plan_sha256
            || self.job_revision != job.revision
            || Some(self.last_checkpoint_sha256.as_str()) != job.last_checkpoint_sha256()
        {
            return invalid("attunement verification does not bind the job checkpoint");
        }
        let Some(last_checkpoint) = job.checkpoints.last() else {
            return invalid("attunement verification requires a committed checkpoint");
        };
        validate_sha256(
            &self.runtime_manifest_sha256,
            "attunement verification runtime_manifest_sha256",
        )?;
        if self.runtime_manifest_sha256 != last_checkpoint.coordinates.runtime_manifest_sha256 {
            return invalid("attunement verification runtime manifest differs from its checkpoint");
        }
        if self.checks.is_empty() || self.checks.len() > MAX_ATTUNEMENT_VERIFICATION_CHECKS {
            return invalid("attunement verification check count is outside its bound");
        }
        let mut ids = BTreeSet::new();
        for check in &self.checks {
            check.validate()?;
            if !ids.insert(&check.id) {
                return invalid("attunement verification check ids must be unique");
            }
        }
        match self.status {
            AttunementVerificationStatus::Passed
                if job.state == AttunementJobState::Succeeded
                    && self.checks.iter().all(|check| check.passed) => {}
            AttunementVerificationStatus::Failed
                if job.state == AttunementJobState::Failed
                    && job.current_phase == Some(AttunementPhase::Verify)
                    && self.checks.iter().any(|check| !check.passed) => {}
            _ => return invalid("attunement verification status differs from job and checks"),
        }
        if self.verified_at_unix_ms < job.updated_at_unix_ms {
            return invalid("attunement verification predates its job snapshot");
        }
        validate_sha256(&self.verification_sha256, "attunement verification_sha256")?;
        if self.verification_sha256 != attunement_verification_sha256(self)? {
            return invalid("attunement verification digest does not match its content");
        }
        Ok(())
    }
}

pub fn installation_plan_sha256(plan: &InstallationPlan) -> Result<String> {
    digest_json(
        b"rrflow-installation-plan-v2",
        &(
            (
                plan.contract_version,
                &plan.installation_identity,
                &plan.project_root_sha256,
                &plan.id,
                &plan.target,
                plan.target_kind,
                &plan.deployment,
                &plan.product_version,
                &plan.executable_sha256,
                &plan.profile_id,
                &plan.profile_sha256,
                &plan.project_precondition_sha256,
                &plan.storage_root_id,
            ),
            (
                &plan.configuration,
                &plan.initial_seat,
                &plan.initial_principal_id,
                &plan.initial_grants,
                plan.credential_bytes,
                &plan.inactive_capabilities,
                &plan.managed_paths,
                &plan.attunement_plan_id,
                &plan.attunement_plan_sha256,
                &plan.actions,
            ),
        ),
    )
}

pub fn installation_result_sha256(result: &InstallationResult) -> Result<String> {
    digest_json(
        b"rrflow-installation-result-v1",
        &(
            result.contract_version,
            &result.plan_id,
            &result.plan_sha256,
            &result.attunement_job_id,
            &result.action_results,
            &result.runtime_manifest_sha256,
            result.runtime_cursor,
            result.control_journal_sequence,
            &result.installed_record_sha256,
            &result.credential_sha256,
            &result.locator_sha256,
            result.applied_at_unix_ms,
            result.idempotent_replay,
        ),
    )
}

pub fn attunement_plan_sha256(plan: &AttunementPlan) -> Result<String> {
    digest_json(
        b"rrflow-attunement-plan-v1",
        &(
            plan.contract_version,
            &plan.id,
            &plan.target,
            &plan.source_sha256,
            &plan.phases,
            plan.estimated_min_duration_ms,
            plan.estimated_max_duration_ms,
        ),
    )
}

pub fn attunement_checkpoint_sha256(checkpoint: &AttunementPhaseCheckpoint) -> Result<String> {
    digest_json(
        b"rrflow-attunement-checkpoint-v1",
        &(
            checkpoint.contract_version,
            &checkpoint.job_id,
            &checkpoint.plan_sha256,
            checkpoint.phase,
            checkpoint.sequence,
            &checkpoint.configuration_sha256,
            &checkpoint.input_sha256,
            &checkpoint.output_sha256,
            &checkpoint.coordinates,
            checkpoint.committed_at_unix_ms,
        ),
    )
}

pub fn attunement_verification_sha256(verification: &AttunementVerification) -> Result<String> {
    digest_json(
        b"rrflow-attunement-verification-v1",
        &(
            verification.contract_version,
            &verification.job_id,
            &verification.plan_sha256,
            verification.job_revision,
            &verification.last_checkpoint_sha256,
            &verification.runtime_manifest_sha256,
            verification.status,
            &verification.checks,
            verification.verified_at_unix_ms,
        ),
    )
}

pub(crate) fn validate_contract_version(version: u16) -> Result<()> {
    if version != INSTALL_ATTUNEMENT_CONTRACT_VERSION {
        return invalid(format!(
            "unsupported install/attunement contract version {version}; expected {INSTALL_ATTUNEMENT_CONTRACT_VERSION}"
        ));
    }
    Ok(())
}

pub(crate) fn validate_project_relative_path(value: &str) -> Result<()> {
    validate_bounded_text(value, "installation managed relative_path", 4_096)?;
    if value.starts_with('/') || value.contains('\\') {
        return invalid("installation managed path must use portable project-relative syntax");
    }
    let segments = value.split('/').collect::<Vec<_>>();
    if segments.first().copied() != Some(".rrflow")
        || segments
            .iter()
            .any(|segment| segment.is_empty() || matches!(*segment, "." | ".."))
    {
        return invalid("installation managed path must remain below .rrflow");
    }
    Ok(())
}

fn validate_sorted_unique<T: Ord>(values: &[T], message: &str) -> Result<()> {
    if values.windows(2).any(|pair| pair[0] >= pair[1]) {
        return invalid(message);
    }
    Ok(())
}

fn validate_job_command_binding(
    job_id: &CanonicalId,
    plan_sha256: &str,
    expected_job_revision: u64,
    last_checkpoint_sha256: Option<&str>,
    requested_at_unix_ms: u64,
    job: &AttunementJob,
) -> Result<()> {
    validate_sha256(plan_sha256, "attunement command plan_sha256")?;
    if job_id != &job.id
        || plan_sha256 != job.plan_sha256
        || expected_job_revision != job.revision
        || last_checkpoint_sha256 != job.last_checkpoint_sha256()
    {
        return invalid("attunement command does not bind the current job revision and checkpoint");
    }
    if requested_at_unix_ms < job.updated_at_unix_ms {
        return invalid("attunement command predates the job snapshot");
    }
    Ok(())
}

pub(crate) fn validate_bounded_text(value: &str, field: &str, maximum: usize) -> Result<()> {
    if value.trim().is_empty() || value.len() > maximum || value.as_bytes().contains(&0) {
        return invalid(format!(
            "{field} must be non-empty, contain no NUL, and use at most {maximum} bytes"
        ));
    }
    Ok(())
}

pub(crate) fn digest_json<T: Serialize>(domain: &[u8], value: &T) -> Result<String> {
    let encoded = serde_json::to_vec(value).map_err(|error| {
        crate::ContractError(format!("contract digest encoding failed: {error}"))
    })?;
    let mut bytes = Vec::with_capacity(domain.len() + 1 + encoded.len());
    bytes.extend_from_slice(domain);
    bytes.push(0);
    bytes.extend_from_slice(&encoded);
    Ok(sha256_bytes(&bytes))
}
