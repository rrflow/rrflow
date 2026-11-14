//! Attunement phases, installation plans, managed paths, actions and results.

use super::*;

pub const INSTALL_ATTUNEMENT_CONTRACT_VERSION: u16 = 1;
/// Attunement plans and job transitions bind ordered plan positions in v2.
/// Installation results and restore envelopes retain their separate version.
pub const ATTUNEMENT_CONTRACT_VERSION: u16 = 2;
/// Independent identity and initial-root binding are mandatory for new plans.
/// Attunement and installation-result envelopes retain their own version.
pub const INSTALLATION_PLAN_CONTRACT_VERSION: u16 = 4;
pub const INSTALLATION_LEASE_PATH: &str = ".rrflow-install.lock";
pub const INSTALLATION_LEASE_PRECONDITION: &str = "rrflow-install-lease-absent-or-private-empty-v1";
pub const MAX_INSTALLATION_ACTIONS: usize = 16;
pub const MAX_ATTUNEMENT_PHASES: usize = 32;
pub const MAX_ATTUNEMENT_FAILURE_BYTES: usize = 4_096;
pub const MAX_ATTUNEMENT_CANCEL_REASON_BYTES: usize = 4_096;
pub const MAX_ATTUNEMENT_VERIFICATION_CHECKS: usize = 256;

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
    AcquireInstallationLease,
    CreateEstateDirectories,
    MaterializeBootstrap,
    CreateStorageRoot,
    PrepareCredentials,
    DeliverCredentials,
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
    InstallationLease,
    InstallationIntent,
    LocatorPending,
    ProjectBootstrap,
    BootstrapPending,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum InstallationRemovalRule {
    RemoveIfOwnedDigestMatches,
    RemoveIfOwnedTreeDigestMatches,
    RemoveIfOwnedAndEmpty,
    /// Retain the empty native admission inode; unlinking it can split owners.
    RetainForExclusion,
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
        if self.kind == InstallationManagedPathKind::InstallationLease {
            if self.relative_path != INSTALLATION_LEASE_PATH {
                return invalid("installation lease must use its fixed project-relative path");
            }
        } else if self.kind == InstallationManagedPathKind::InstallationIntent {
            if !self.relative_path.starts_with(".rrflow-install-")
                || !self.relative_path.ends_with(".json")
                || self.relative_path.contains(['/', '\\'])
                || self.relative_path.len() > 256
            {
                return invalid("installation intent must use its plan-addressed root path");
            }
        } else {
            validate_project_relative_path(&self.relative_path)?;
        }
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
    pub bootstrap: InstallationBootstrap,
    pub initial_seat: MemorySeatDefinition,
    pub initial_principal_id: CanonicalId,
    pub initial_grants: Vec<SecurityAction>,
    pub credential_bytes: u16,
    pub inactive_capabilities: Vec<CanonicalId>,
    pub managed_paths: Vec<InstallationManagedPath>,
    pub attunement_plan_id: CanonicalId,
    pub attunement_plan_sha256: String,
    pub actions: Vec<InstallationPlanAction>,
    /// Present when the project already holds a `.rrflow` folder that is not
    /// an installed estate. Apply makes that folder private in place and moves
    /// exactly these top-level entries into `.rrflow/previous/<plan id>/`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adopted_estate_entries: Option<Vec<String>>,
    pub plan_sha256: String,
}

/// The most entries one adoption moves; a larger leftover tree is refused.
pub const MAX_ADOPTED_ESTATE_ENTRIES: usize = 256;
/// The `.rrflow` entry that retains adopted contents; never itself adopted.
pub const ADOPTED_ESTATE_DIRECTORY: &str = "previous";

impl InstallationPlan {
    pub fn validate(&self) -> Result<()> {
        if self.contract_version != INSTALLATION_PLAN_CONTRACT_VERSION {
            return invalid("unsupported installation plan contract version; expected 4");
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
        self.bootstrap.validate()?;
        if self.bootstrap.specialization.seat != self.initial_seat
            || self.bootstrap.credential_entropy_bytes != self.credential_bytes
        {
            return invalid("bootstrap seat or credential generation differs from installation");
        }
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
            (
                InstallationManagedPathKind::InstallationLease,
                InstallationRemovalRule::RetainForExclusion,
            ),
            (
                InstallationManagedPathKind::InstallationIntent,
                InstallationRemovalRule::RemoveIfOwnedDigestMatches,
            ),
            (
                InstallationManagedPathKind::LocatorPending,
                InstallationRemovalRule::RemoveIfOwnedDigestMatches,
            ),
            (
                InstallationManagedPathKind::ProjectBootstrap,
                InstallationRemovalRule::RemoveIfOwnedDigestMatches,
            ),
            (
                InstallationManagedPathKind::BootstrapPending,
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
            if managed.kind == InstallationManagedPathKind::InstallationLease
                && (managed.relative_path != INSTALLATION_LEASE_PATH
                    || managed.precondition_sha256
                        != sha256_bytes(INSTALLATION_LEASE_PRECONDITION.as_bytes()))
            {
                return invalid(
                    "installation lease must bind the canonical empty-file admission rule",
                );
            }
            if managed.kind != expected_kind
                || managed.removal_rule != expected_rule
                || !paths.insert(&managed.relative_path)
            {
                return invalid("installation managed paths are duplicated or out of order");
            }
        }
        let expected_extra_paths = [
            (
                InstallationManagedPathKind::InstallationIntent,
                format!(".rrflow-install-{}.json", self.storage_root_id),
            ),
            (
                InstallationManagedPathKind::LocatorPending,
                format!(".rrflow/.config.{}.pending", self.storage_root_id),
            ),
            (
                InstallationManagedPathKind::ProjectBootstrap,
                ".rrflow/bootstrap.md".to_owned(),
            ),
            (
                InstallationManagedPathKind::BootstrapPending,
                format!(".rrflow/.bootstrap.{}.pending", self.storage_root_id),
            ),
        ];
        for (kind, expected) in expected_extra_paths {
            if self.managed_path(kind).relative_path != expected {
                return invalid(
                    "bootstrap or transient path differs from its installation identity",
                );
            }
        }
        let mut private_directories: Vec<_> = self.managed_paths[..4]
            .iter()
            .map(|path| path.relative_path.clone())
            .collect();
        private_directories.sort();
        let mut private_files: Vec<_> = [
            InstallationManagedPathKind::TokenKey,
            InstallationManagedPathKind::OperatorCredential,
            InstallationManagedPathKind::InstallationLease,
            InstallationManagedPathKind::InstallationIntent,
        ]
        .into_iter()
        .map(|kind| self.managed_path(kind).relative_path.clone())
        .collect();
        private_files.sort();
        if self.bootstrap.private_directories != private_directories
            || self.bootstrap.private_files != private_files
        {
            return invalid("bootstrap private-object policy differs from managed paths");
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
            InstallationActionKind::AcquireInstallationLease,
            InstallationActionKind::PrepareCredentials,
            InstallationActionKind::CreateEstateDirectories,
            InstallationActionKind::MaterializeBootstrap,
            InstallationActionKind::CreateStorageRoot,
            InstallationActionKind::DeliverCredentials,
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
        if let Some(entries) = &self.adopted_estate_entries {
            validate_adopted_estate_entries(entries)?;
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
            InstallationManagedPathKind::InstallationLease => 8,
            InstallationManagedPathKind::InstallationIntent => 9,
            InstallationManagedPathKind::LocatorPending => 10,
            InstallationManagedPathKind::ProjectBootstrap => 11,
            InstallationManagedPathKind::BootstrapPending => 12,
        };
        &self.managed_paths[index]
    }
}

/// Canonical initial grant choices for a newly installed estate. The optional
/// source-read action is explicit and never accepts caller-authored grant sets.
pub fn installation_initial_grants(project_source_read: bool) -> Vec<SecurityAction> {
    SecurityAction::ALL
        .into_iter()
        .filter(|action| *action != SecurityAction::ProjectSourceRead || project_source_read)
        .collect()
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
