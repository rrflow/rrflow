#[cfg(test)]
use super::clock::FixedClock;
use super::clock::{assess, observe_required, require_accepted, ClockSource, HostSystemClock};
use super::*;
use crate::engine::estate_layout::{
    create_estate_directories, reject_existing_estate, safe_join, validate_estate_directories,
    validate_managed_relative, validate_recovery_estate, EstateFilesystem, EstateLayout,
    LOCATOR_RELATIVE_PATH, STORAGE_DIRECTORY,
};
use crate::engine::memory_estate::build_memory_estate_plan;
use crate::engine::token_key::{
    read_api_key_document_anchored as read_api_key_document,
    read_token_key_anchored as read_token_key, ApiKeyDocument,
};
use rrd_contract::{
    attunement_plan_sha256, installation_plan_sha256, installation_result_sha256, AttunementJob,
    AttunementJobState, AttunementPhase, AttunementPhasePlan, AttunementPlan, DeploymentProfile,
    EstateConfiguration, EstateConfigurationInput, InstallationActionDisposition,
    InstallationActionKind, InstallationActionResult, InstallationManagedPathKind,
    InstallationPlan, InstallationPlanAction, InstallationResult, InstallationTargetKind,
    InstalledEstateIdentity, MemorySeatDefinition, PersistMemoryEstate, StorageProfileKind,
    ATTUNEMENT_PHASES, INSTALLATION_PLAN_CONTRACT_VERSION, INSTALL_ATTUNEMENT_CONTRACT_VERSION,
};
use rrd_core::digest::Sha256 as IncrementalSha256;
use rrd_security::{Principal, PrincipalKind, ResourceGrant, SecurityState, SECURITY_FORMAT};
use rrd_store::filesystem::{AnchoredPath, OpenOptions};
use rrd_store::{ControlTransition, RrflowKvInspector};
use serde::de::DeserializeOwned;
use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read, Write};

const INSTALL_PROFILE_BYTES: &[u8] = include_bytes!("../../assets/install/default-profile-v1.json");
const DEFAULT_CONFIGURATION_BYTES: &[u8] =
    include_bytes!("../../assets/install/default-estate-configuration-v2.toml");
const INSTALL_PROFILE_FORMAT: u16 = 1;
const LOCATOR_FORMAT: u16 = 1;
const INSTALLED_RECORD_FORMAT: u16 = 2;
const INSTALL_CHECKPOINT_FORMAT: u16 = 2;
pub(super) const INSTALLED_RECORD_KEY: &str = "server/state/install/record";
const INSTALL_CHECKPOINT_KEY: &str = "server/state/install/checkpoint";
pub(super) const MAX_LOCATOR_BYTES: u64 = 64 * 1024;
const MAX_CONFIGURATION_BYTES: u64 = 64 * 1024;

mod admission;
mod inventory;
mod lease;
pub(super) mod recovery;

use inventory::{project_inventory_sha256, project_inventory_sha256_excluding};

use recovery::{
    acknowledge_intent, load_or_create_intent, materialize_secret, read_existing_intent,
    InstallApplyStage, InstallRecoveryState,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DefaultInstallProfile {
    pub format_version: u16,
    pub id: CanonicalId,
    pub configuration_asset: String,
    pub deployment: DeploymentProfile,
    pub initial_seat: MemorySeatDefinition,
    pub initial_principal_id: CanonicalId,
    pub credential_bytes: u16,
    pub initial_grant_policy: CanonicalId,
    pub inactive_capabilities: Vec<CanonicalId>,
    pub attunement_phases: Vec<DefaultAttunementPhase>,
    pub estimated_min_duration_ms: u64,
    pub estimated_max_duration_ms: u64,
    pub inventory_max_files: u64,
    pub inventory_max_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DefaultAttunementPhase {
    pub phase: AttunementPhase,
    pub configuration: String,
    pub estimated_items: u64,
    pub estimated_input_bytes: u64,
}

impl DefaultInstallProfile {
    pub(super) fn load(requested: &str) -> Result<(Self, String)> {
        if requested != "default" {
            return Err(ServiceError::Contract(format!(
                "unsupported installation profile {requested:?}"
            )));
        }
        let profile: Self = serde_json::from_slice(INSTALL_PROFILE_BYTES).map_err(contract_json)?;
        profile.validate()?;
        Ok((profile, digest::sha256_hex(INSTALL_PROFILE_BYTES)))
    }

    fn validate(&self) -> Result<()> {
        if self.format_version != INSTALL_PROFILE_FORMAT
            || self.id.as_str() != "default"
            || self.configuration_asset != "default-estate-configuration-v2.toml"
            || self.initial_grant_policy.as_str() != "local-owner-all-actions"
            || !(32..=4_096).contains(&self.credential_bytes)
            || self.inventory_max_files == 0
            || self.inventory_max_bytes == 0
            || self.estimated_min_duration_ms > self.estimated_max_duration_ms
        {
            return Err(ServiceError::Contract(
                "default installation profile has invalid bounds or identity".into(),
            ));
        }
        self.initial_seat
            .validate()
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        self.deployment
            .validate()
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        if self.deployment.storage_profile != StorageProfileKind::RrflowKv {
            return Err(ServiceError::Contract(
                "default installation profile must use durable rrflowKV".into(),
            ));
        }
        if self
            .inactive_capabilities
            .windows(2)
            .any(|capabilities| capabilities[0] >= capabilities[1])
            || self.attunement_phases.len() != ATTUNEMENT_PHASES.len()
        {
            return Err(ServiceError::Contract(
                "default installation profile catalogues are incomplete or non-canonical".into(),
            ));
        }
        for (phase, expected) in self.attunement_phases.iter().zip(ATTUNEMENT_PHASES) {
            if phase.phase != expected
                || phase.configuration.trim().is_empty()
                || phase.configuration.len() > 1_024
            {
                return Err(ServiceError::Contract(
                    "default installation profile attunement phases are invalid".into(),
                ));
            }
        }
        Ok(())
    }
}

/// Loads the bundled default or one explicitly selected, bounded operator
/// configuration. Planning is the only lifecycle stage that reads this file;
/// apply consumes the effective configuration sealed into the plan.
pub fn load_estate_configuration(path: Option<&Path>) -> Result<EstateConfiguration> {
    let bytes = match path {
        None => DEFAULT_CONFIGURATION_BYTES.to_vec(),
        Some(path) => read_regular_bounded(
            &AnchoredPath::from_path(path).map_err(storage_error)?,
            MAX_CONFIGURATION_BYTES,
        )?,
    };
    let input: EstateConfigurationInput = toml::from_str(
        std::str::from_utf8(&bytes).map_err(|error| ServiceError::Contract(error.to_string()))?,
    )
    .map_err(|error| ServiceError::Contract(error.to_string()))?;
    EstateConfiguration::from_input(1, input)
        .map_err(|error| ServiceError::Contract(error.to_string()))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InstallationPreview {
    pub installation: InstallationPlan,
    pub attunement: AttunementPlan,
}

impl InstallationPreview {
    pub fn validate(&self) -> Result<()> {
        self.installation
            .validate_attunement_plan(&self.attunement)
            .map_err(|error| ServiceError::Contract(error.to_string()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectLocator {
    pub format_version: u16,
    pub product_version: String,
    pub identity: InstalledEstateIdentity,
    pub storage_root: String,
    pub token_key: String,
    pub operator_credential: String,
    pub plan_sha256: String,
    pub profile_id: CanonicalId,
    pub profile_sha256: String,
    pub executable_sha256: String,
    pub installed_record_sha256: String,
    pub configuration_revision: u64,
    pub configuration_sha256: String,
}

impl ProjectLocator {
    fn validate(&self) -> Result<()> {
        if self.format_version != LOCATOR_FORMAT
            || self.product_version != env!("CARGO_PKG_VERSION")
            || !is_sha256(&self.plan_sha256)
            || !is_sha256(&self.profile_sha256)
            || !is_sha256(&self.executable_sha256)
            || !is_sha256(&self.installed_record_sha256)
            || self.configuration_revision == 0
            || !is_sha256(&self.configuration_sha256)
        {
            return Err(ServiceError::Contract(
                "project locator identity or digest is invalid".into(),
            ));
        }
        self.identity
            .validate()
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        for path in [
            self.storage_root.as_str(),
            self.token_key.as_str(),
            self.operator_credential.as_str(),
        ] {
            validate_managed_relative(path)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InstalledEstateRecord {
    pub format_version: u16,
    pub installation: InstallationPlan,
    pub attunement_plan_sha256: String,
    pub credential_sha256: String,
    pub token_key_sha256: String,
    pub runtime_commit_sha256: String,
    pub expected_runtime_cursor: u64,
    pub bootstrap_control_sequence: u64,
    pub clock_anchor: ClockObservation,
    pub record_sha256: String,
}

impl InstalledEstateRecord {
    pub(super) fn validate(&self) -> Result<()> {
        self.installation
            .validate()
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        if self.format_version != INSTALLED_RECORD_FORMAT
            || self.attunement_plan_sha256 != self.installation.attunement_plan_sha256
            || !is_sha256(&self.credential_sha256)
            || !is_sha256(&self.token_key_sha256)
            || !is_sha256(&self.runtime_commit_sha256)
            || self.expected_runtime_cursor == 0
            || self.bootstrap_control_sequence == 0
            || self.record_sha256 != installed_record_sha256(self)?
        {
            return Err(ServiceError::Contract(
                "installed estate record is inconsistent or corrupt".into(),
            ));
        }
        self.clock_anchor.validate()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct InstallationCheckpoint {
    format_version: u16,
    plan_sha256: String,
    runtime_commit_sha256: String,
    runtime_cursor: u64,
    control_sequence: u64,
    installed_record_sha256: String,
    clock_anchor_unix_ms: u64,
    checkpoint_sha256: String,
}

impl InstallationCheckpoint {
    fn validate(&self) -> Result<()> {
        if self.format_version != INSTALL_CHECKPOINT_FORMAT
            || !is_sha256(&self.plan_sha256)
            || !is_sha256(&self.runtime_commit_sha256)
            || !is_sha256(&self.installed_record_sha256)
            || self.runtime_cursor == 0
            || self.control_sequence == 0
            || self.clock_anchor_unix_ms == 0
            || self.checkpoint_sha256 != installation_checkpoint_sha256(self)?
        {
            return Err(ServiceError::Contract(
                "installation checkpoint is inconsistent or corrupt".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallationVerificationStatus {
    Passed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InstallationVerificationCheck {
    pub id: CanonicalId,
    pub passed: bool,
    pub evidence_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InstallationVerificationReport {
    pub status: InstallationVerificationStatus,
    pub product_version: String,
    pub instance_id: CanonicalId,
    pub plan_sha256: String,
    pub project_inventory_sha256: String,
    pub attunement_source_current: bool,
    pub runtime_manifest_sha256: String,
    pub runtime_cursor: u64,
    pub control_journal_sequence: u64,
    pub clock: ClockAssessment,
    pub physical: rrd_store::RrflowKvInspection,
    pub checks: Vec<InstallationVerificationCheck>,
}

impl RrdEngine {
    /// Allocate one explicit, non-secret identity input before deterministic
    /// planning. Callers retain it across previews and generate a new value for
    /// each independent estate; allocation creates no project or engine state.
    pub fn generate_installation_identity() -> Result<String> {
        let entropy = crate::engine::token_key::generate_token_key()
            .map_err(|error| ServiceError::Storage(error.to_string()))?;
        Ok(digest::sha256_hex(&entropy))
    }

    pub fn plan_installation(
        project: &Path,
        target_kind: InstallationTargetKind,
        installation_identity: &str,
        profile_id: &str,
        configuration_path: Option<&Path>,
        executable: &Path,
    ) -> Result<InstallationPreview> {
        if !is_sha256(installation_identity) {
            return Err(ServiceError::Contract(
                "installation identity must contain 64 lowercase hexadecimal characters".into(),
            ));
        }
        let project = canonical_project(project, target_kind)?;
        reject_existing_estate(&project)?;
        let (profile, profile_sha256) = DefaultInstallProfile::load(profile_id)?;
        let configuration = load_estate_configuration(configuration_path)?;
        let executable_sha256 = sha256_file(executable)?;
        let project_precondition_sha256 = project_inventory_sha256(project.project(), &profile)?;
        let identity_seed = installation_identity_seed(
            installation_identity,
            target_kind,
            &project_precondition_sha256,
            &profile_sha256,
            &executable_sha256,
            &configuration.configuration_sha256,
        )?;
        let install_id = canonical_prefixed("install", &identity_seed)?;
        let attunement_id = canonical_prefixed("attune", &identity_seed)?;
        let instance_id = canonical_prefixed("instance", &identity_seed)?;
        let estate_id = canonical_prefixed("estate", &identity_seed)?;
        let project_id = canonical_prefixed("project", &identity_seed)?;
        let target = InstalledEstateIdentity {
            project_id,
            estate_id,
            instance_id,
        };
        let phases = profile
            .attunement_phases
            .iter()
            .map(|phase| {
                Ok(AttunementPhasePlan {
                    phase: phase.phase,
                    sequence: phase.phase.sequence(),
                    configuration_sha256: json_sha256(phase)?,
                    estimated_items: phase.estimated_items,
                    estimated_input_bytes: phase.estimated_input_bytes,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let mut attunement = AttunementPlan {
            contract_version: INSTALL_ATTUNEMENT_CONTRACT_VERSION,
            id: attunement_id,
            target: target.clone(),
            source_sha256: project_precondition_sha256.clone(),
            phases,
            estimated_min_duration_ms: profile.estimated_min_duration_ms,
            estimated_max_duration_ms: profile.estimated_max_duration_ms,
            plan_sha256: zero_digest(),
        };
        attunement.plan_sha256 = attunement_plan_sha256(&attunement)
            .map_err(|error| ServiceError::Contract(error.to_string()))?;

        let layout = EstateLayout::new(&install_id, &profile.initial_principal_id)?;
        let absent = digest::sha256_hex(b"rrflow-install-managed-path-absent-v1");
        let managed_paths = layout.managed_paths(&absent);
        let actions = installation_actions(
            &configuration.configuration_sha256,
            &attunement.plan_sha256,
            profile.credential_bytes,
        )?;
        let mut installation = InstallationPlan {
            contract_version: INSTALLATION_PLAN_CONTRACT_VERSION,
            installation_identity: installation_identity.into(),
            project_root_sha256: initial_project_root_sha256(&project)?,
            id: install_id.clone(),
            target,
            target_kind,
            deployment: profile.deployment,
            product_version: env!("CARGO_PKG_VERSION").into(),
            executable_sha256,
            profile_id: profile.id,
            profile_sha256,
            project_precondition_sha256,
            storage_root_id: install_id,
            configuration,
            initial_seat: profile.initial_seat,
            initial_principal_id: profile.initial_principal_id,
            initial_grants: SecurityAction::ALL.to_vec(),
            credential_bytes: profile.credential_bytes,
            inactive_capabilities: profile.inactive_capabilities,
            managed_paths,
            attunement_plan_id: attunement.id.clone(),
            attunement_plan_sha256: attunement.plan_sha256.clone(),
            actions,
            plan_sha256: zero_digest(),
        };
        installation.plan_sha256 = installation_plan_sha256(&installation)
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        let preview = InstallationPreview {
            installation,
            attunement,
        };
        preview.validate()?;
        Ok(preview)
    }

    pub fn apply_installation(
        project: &Path,
        target_kind: InstallationTargetKind,
        preview: &InstallationPreview,
        expected_plan_sha256: &str,
        executable: &Path,
    ) -> Result<InstallationResult> {
        let clock = HostSystemClock;
        Self::apply_installation_with_observer(
            project,
            target_kind,
            preview,
            expected_plan_sha256,
            executable,
            &clock,
            &mut |_, _| Ok(()),
        )
    }

    #[cfg(test)]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn apply_installation_at(
        project: &Path,
        target_kind: InstallationTargetKind,
        preview: &InstallationPreview,
        expected_plan_sha256: &str,
        at_unix_ms: u64,
        executable: &Path,
    ) -> Result<InstallationResult> {
        let clock = FixedClock::at(at_unix_ms);
        Self::apply_installation_with_observer(
            project,
            target_kind,
            preview,
            expected_plan_sha256,
            executable,
            &clock,
            &mut |_, _| Ok(()),
        )
    }

    #[cfg(test)]
    pub(super) fn apply_installation_with_test_clock(
        project: &Path,
        target_kind: InstallationTargetKind,
        preview: &InstallationPreview,
        expected_plan_sha256: &str,
        executable: &Path,
        clock: &dyn ClockSource,
    ) -> Result<InstallationResult> {
        Self::apply_installation_with_observer(
            project,
            target_kind,
            preview,
            expected_plan_sha256,
            executable,
            clock,
            &mut |_, _| Ok(()),
        )
    }

    #[cfg(test)]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn apply_installation_with_failure(
        project: &Path,
        target_kind: InstallationTargetKind,
        preview: &InstallationPreview,
        expected_plan_sha256: &str,
        at_unix_ms: u64,
        executable: &Path,
        fail_after: InstallApplyStage,
    ) -> Result<InstallationResult> {
        let mut injected = false;
        Self::apply_installation_with_observer(
            project,
            target_kind,
            preview,
            expected_plan_sha256,
            executable,
            &FixedClock::at(at_unix_ms),
            &mut |stage, _| {
                if stage == fail_after && !injected {
                    injected = true;
                    return Err(ServiceError::Storage(format!(
                        "test installation failpoint after {}",
                        stage.as_str()
                    )));
                }
                Ok(())
            },
        )
    }

    #[cfg(test)]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn apply_installation_with_test_observer(
        project: &Path,
        preview: &InstallationPreview,
        executable: &Path,
        observer: &mut dyn FnMut(InstallApplyStage, InstallRecoveryState) -> Result<()>,
    ) -> Result<InstallationResult> {
        Self::apply_installation_with_observer(
            project,
            preview.installation.target_kind,
            preview,
            &preview.installation.plan_sha256,
            executable,
            &FixedClock::at(1_800_000_000_000),
            observer,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn apply_installation_with_observer(
        project: &Path,
        target_kind: InstallationTargetKind,
        preview: &InstallationPreview,
        expected_plan_sha256: &str,
        executable: &Path,
        clock: &dyn ClockSource,
        observer: &mut dyn FnMut(InstallApplyStage, InstallRecoveryState) -> Result<()>,
    ) -> Result<InstallationResult> {
        preview.validate()?;
        if preview.installation.target_kind != target_kind
            || preview.installation.plan_sha256 != expected_plan_sha256
        {
            return Err(ServiceError::Contract(
                "installation apply mode or expected digest does not match the preview".into(),
            ));
        }
        let mut project = canonical_project_root(project)?;
        validate_estate_directories(&project, true)?;
        let executable_sha256 = sha256_file(executable)?;
        if executable_sha256 != preview.installation.executable_sha256 {
            return Err(ServiceError::ProjectBindingMismatch);
        }
        let (profile, profile_sha256) =
            DefaultInstallProfile::load(preview.installation.profile_id.as_str())?;
        if profile_sha256 != preview.installation.profile_sha256 {
            return Err(ServiceError::ProjectBindingMismatch);
        }

        let started = Instant::now();
        let layout = EstateLayout::new(
            &preview.installation.storage_root_id,
            &preview.installation.initial_principal_id,
        )?;
        validate_sealed_preview(preview, &profile, &profile_sha256, &executable_sha256)?;

        // Refused preconditions must not create even an empty admission file.
        // Existing leases are locked before reading mutable recovery state.
        let _lease = match lease::InstallationLease::existing(project.project())? {
            Some(lease) => lease,
            None => {
                admission::inspect(&project, &layout, preview, &profile, executable, clock)?;
                lease::InstallationLease::create_or_acquire(project.project())?
            }
        };
        // Recheck every mutable precondition under the retained native owner.
        // Re-admit private descendants in case another owner completed between
        // the initial read-only preflight and this successful acquisition.
        project.admit_missing_private_directories(true)?;
        let admitted = admission::inspect(&project, &layout, preview, &profile, executable, clock)?;
        observe_install_stage(
            preview,
            started,
            InstallApplyStage::LeaseAcquired,
            InstallRecoveryState::Admitted,
            None,
            observer,
        )?;

        let intent = match admitted {
            admission::Admission::Published { intent, result } => {
                if let Some(intent) = intent {
                    let (_, _, locator_bytes) = read_locator_anchored(&project)?;
                    validate_recovery_estate(&project, &layout)?;
                    observe_install_stage(
                        preview,
                        started,
                        InstallApplyStage::LocatorPublished,
                        InstallRecoveryState::Published,
                        Some(result.idempotent_replay),
                        observer,
                    )?;
                    cleanup_locator_pending(&project, &layout, &locator_bytes)?;
                    acknowledge_intent(&project, &layout, &intent)?;
                    observe_install_stage(
                        preview,
                        started,
                        InstallApplyStage::IntentAcknowledged,
                        InstallRecoveryState::Complete,
                        Some(result.idempotent_replay),
                        observer,
                    )?;
                }
                return Ok(*result);
            }
            admission::Admission::Recover(intent) => {
                observe_install_stage(
                    preview,
                    started,
                    InstallApplyStage::IntentPublished,
                    InstallRecoveryState::OwnedIntent,
                    None,
                    observer,
                )?;
                *intent
            }
            admission::Admission::New(clock_anchor) => {
                let (intent, recovery_state) = load_or_create_intent(
                    &project,
                    &layout,
                    preview,
                    &clock_anchor,
                    &executable_sha256,
                )?;
                if project_inventory_sha256_excluding(
                    project.project(),
                    &profile,
                    Some(Path::new(&layout.install_intent)),
                )? != preview.installation.project_precondition_sha256
                {
                    return Err(ServiceError::ProjectBindingMismatch);
                }
                validate_recovery_estate(&project, &layout)?;
                observe_install_stage(
                    preview,
                    started,
                    InstallApplyStage::IntentPublished,
                    recovery_state,
                    None,
                    observer,
                )?;
                intent
            }
        };

        let storage_relative = preview
            .installation
            .managed_path(InstallationManagedPathKind::StorageRoot)
            .relative_path
            .clone();
        let token_relative = preview
            .installation
            .managed_path(InstallationManagedPathKind::TokenKey)
            .relative_path
            .clone();
        let credential_relative = preview
            .installation
            .managed_path(InstallationManagedPathKind::OperatorCredential)
            .relative_path
            .clone();
        create_estate_directories(&mut project)?;
        validate_recovery_estate(&project, &layout)?;
        let locator_path = safe_join(&project, LOCATOR_RELATIVE_PATH)?;
        observe_install_stage(
            preview,
            started,
            InstallApplyStage::EstateDirectoriesReady,
            InstallRecoveryState::OwnedIntent,
            None,
            observer,
        )?;
        let storage_root = safe_join(&project, &storage_relative)?;
        let token_path = safe_join(&project, &token_relative)?;
        let credential_path = safe_join(&project, &credential_relative)?;
        let instance_id = preview.installation.target.instance_id.clone();
        let token_key = intent.token_key()?;
        let engine = match storage_root.metadata() {
            Ok(metadata) if !metadata.file_type().is_symlink() && metadata.is_dir() => {
                RrdEngine::open_existing_anchored(&storage_root, instance_id.clone(), token_key)?
            }
            Ok(_) => return Err(ServiceError::ProjectBindingMismatch),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                RrdEngine::create_new_anchored(&storage_root, instance_id.clone(), token_key)?
            }
            Err(error) => return Err(ServiceError::Storage(error.to_string())),
        };
        let store_state = classify_installation_store(&engine)?;
        observe_install_stage(
            preview,
            started,
            InstallApplyStage::StorageReady,
            store_state,
            Some(store_state == InstallRecoveryState::Committed),
            observer,
        )?;
        materialize_secret(
            &token_path,
            &token_key,
            store_state == InstallRecoveryState::EmptyStore,
        )?;
        observe_install_stage(
            preview,
            started,
            InstallApplyStage::TokenReady,
            store_state,
            Some(store_state == InstallRecoveryState::Committed),
            observer,
        )?;
        materialize_secret(
            &credential_path,
            intent.credential_bytes(),
            store_state == InstallRecoveryState::EmptyStore,
        )?;
        observe_install_stage(
            preview,
            started,
            InstallApplyStage::CredentialReady,
            store_state,
            Some(store_state == InstallRecoveryState::Committed),
            observer,
        )?;
        let credential_sha256 = intent.credential_sha256().to_owned();
        let token_key_sha256 = intent.token_key_sha256().to_owned();
        let installed_at_unix_ms = intent.clock_anchor().observed_at_unix_ms;

        let memory_plan = build_memory_estate_plan(
            &PersistMemoryEstate {
                scope: format!("instance:{instance_id}"),
                seat: preview.installation.initial_seat.clone(),
                representations: Vec::new(),
                valid_from: installed_at_unix_ms,
            },
            None,
        )?;
        let bootstrap_session = CorrelationId::new("installation-bootstrap")
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        let runtime_mutations = memory_plan
            .mutations
            .iter()
            .map(|mutation| public_runtime_mutation(mutation, &bootstrap_session))
            .collect::<Result<Vec<_>>>()?;
        let runtime_commit = RuntimeCommit {
            scope: ScopeId::new(format!("instance:{instance_id}")).map_err(core_contract)?,
            at: installed_at_unix_ms,
            actor: "rrflow:installer".into(),
            expected_cursor: 0,
            mutations: runtime_mutations,
        };
        runtime_commit.validate().map_err(core_contract)?;
        let runtime_commit_sha256 = runtime_commit.digest();
        let expected_runtime_cursor = runtime_commit.mutations.len() as u64;

        let resource = ResourcePath {
            segments: vec![
                ResourceId::new(ResourceKind::Instance, instance_id.as_str())
                    .map_err(|error| ServiceError::Contract(error.to_string()))?,
            ],
        };
        let grants = preview
            .installation
            .initial_grants
            .iter()
            .copied()
            .map(|action| ResourceGrant {
                action,
                resource_prefix: resource.clone(),
                data_policy: None,
            })
            .collect();
        let principal = Principal {
            id: preview.installation.initial_principal_id.clone(),
            kind: PrincipalKind::User,
            credential_sha256: credential_sha256.clone(),
            credential_revision: 1,
            not_before_unix_ms: installed_at_unix_ms,
            expires_at_unix_ms: u64::MAX,
            disabled: false,
            role_ids: BTreeSet::new(),
            grants,
        };
        let security = SecurityState {
            format_version: SECURITY_FORMAT,
            revision: 1,
            principals: BTreeMap::from([(principal.id.clone(), principal)]),
            roles: BTreeMap::new(),
            identity_bindings: BTreeMap::new(),
            jwt_issuers: BTreeMap::new(),
        };
        let request_id = format!(
            "install-request-{}",
            &preview.installation.plan_sha256[..16]
        );
        let operation_id = format!(
            "install-operation-{}",
            &preview.installation.plan_sha256[..16]
        );
        let mut transitions = rrd_security::prepare_security_initialization(
            &instance_id,
            &security,
            installed_at_unix_ms,
            "rrflow:installer",
            &request_id,
            &operation_id,
        )?;
        let bootstrap_control_sequence = transitions.len() as u64 + 5;
        let mut installed = InstalledEstateRecord {
            format_version: INSTALLED_RECORD_FORMAT,
            installation: preview.installation.clone(),
            attunement_plan_sha256: preview.attunement.plan_sha256.clone(),
            credential_sha256: credential_sha256.clone(),
            token_key_sha256,
            runtime_commit_sha256: runtime_commit_sha256.clone(),
            expected_runtime_cursor,
            bootstrap_control_sequence,
            clock_anchor: intent.clock_anchor().clone(),
            record_sha256: zero_digest(),
        };
        installed.record_sha256 = installed_record_sha256(&installed)?;
        installed.validate()?;
        let mut checkpoint = InstallationCheckpoint {
            format_version: INSTALL_CHECKPOINT_FORMAT,
            plan_sha256: preview.installation.plan_sha256.clone(),
            runtime_commit_sha256: runtime_commit_sha256.clone(),
            runtime_cursor: expected_runtime_cursor,
            control_sequence: bootstrap_control_sequence,
            installed_record_sha256: installed.record_sha256.clone(),
            clock_anchor_unix_ms: installed_at_unix_ms,
            checkpoint_sha256: zero_digest(),
        };
        checkpoint.checkpoint_sha256 = installation_checkpoint_sha256(&checkpoint)?;
        checkpoint.validate()?;
        let job = AttunementJob {
            contract_version: INSTALL_ATTUNEMENT_CONTRACT_VERSION,
            id: canonical_prefixed("job", &preview.attunement.plan_sha256)?,
            plan_id: preview.attunement.id.clone(),
            plan_sha256: preview.attunement.plan_sha256.clone(),
            revision: 1,
            state: AttunementJobState::Pending,
            current_phase: None,
            checkpoints: Vec::new(),
            lease: None,
            failure: None,
            created_at_unix_ms: installed_at_unix_ms,
            updated_at_unix_ms: installed_at_unix_ms,
        };
        job.validate(&preview.attunement)
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        transitions.extend([
            control_create(
                &estate_configuration_key(&instance_id),
                &preview.installation.configuration,
                installed_at_unix_ms,
                &request_id,
                &operation_id,
                "configuration.installed",
            )?,
            control_create(
                INSTALLED_RECORD_KEY,
                &installed,
                installed_at_unix_ms,
                &request_id,
                &operation_id,
                "install.record",
            )?,
            control_create(
                INSTALL_CHECKPOINT_KEY,
                &checkpoint,
                installed_at_unix_ms,
                &request_id,
                &operation_id,
                "install.checkpoint",
            )?,
            control_create(
                &attunement_plan_key(&instance_id, &preview.attunement.id),
                &preview.attunement,
                installed_at_unix_ms,
                &request_id,
                &operation_id,
                "attunement.plan.created",
            )?,
            control_create(
                &attunement_job_key(&instance_id, &job.id),
                &job,
                installed_at_unix_ms,
                &request_id,
                &operation_id,
                "attunement.job.created",
            )?,
        ]);
        let committed_before = store_state == InstallRecoveryState::Committed;
        if !committed_before {
            require_empty_installation(&engine, &transitions)?;
            let (outcome, entries) = engine
                .storage
                .runtime()
                .commit_with_control_transitions(&runtime_commit, &transitions)?;
            if outcome.commit_id != runtime_commit_sha256
                || outcome.last_cursor != expected_runtime_cursor
                || entries.last().map(|entry| entry.sequence) != Some(bootstrap_control_sequence)
            {
                return Err(ServiceError::StorageConflict(
                    "installation commit outcome differs from the sealed bootstrap".into(),
                ));
            }
        }
        drop(engine);
        let read = require_committed_installation(
            &storage_root,
            &runtime_commit,
            &transitions,
            expected_runtime_cursor,
            bootstrap_control_sequence,
        )?;
        observe_install_stage(
            preview,
            started,
            InstallApplyStage::EngineCommitted,
            InstallRecoveryState::Committed,
            Some(committed_before),
            observer,
        )?;
        let locator = ProjectLocator {
            format_version: LOCATOR_FORMAT,
            product_version: env!("CARGO_PKG_VERSION").into(),
            identity: preview.installation.target.clone(),
            storage_root: storage_relative,
            token_key: token_relative,
            operator_credential: credential_relative,
            plan_sha256: preview.installation.plan_sha256.clone(),
            profile_id: preview.installation.profile_id.clone(),
            profile_sha256: preview.installation.profile_sha256.clone(),
            executable_sha256: preview.installation.executable_sha256.clone(),
            installed_record_sha256: installed.record_sha256.clone(),
            configuration_revision: preview.installation.configuration.revision,
            configuration_sha256: preview
                .installation
                .configuration
                .configuration_sha256
                .clone(),
        };
        locator.validate()?;
        let locator_bytes = toml::to_string(&locator)
            .map_err(|error| ServiceError::Contract(error.to_string()))?
            .into_bytes();
        let locator_sha256 = digest::sha256_hex(&locator_bytes);
        publish_new_file(
            &locator_path,
            &safe_join(&project, &layout.locator_pending)?,
            &locator_bytes,
        )?;
        observe_install_stage(
            preview,
            started,
            InstallApplyStage::LocatorPublished,
            InstallRecoveryState::Published,
            Some(committed_before),
            observer,
        )?;
        let result = build_installation_result(
            preview,
            &job.id,
            &installed,
            &read.manifest_id,
            read.commit_cursor,
            bootstrap_control_sequence,
            &credential_sha256,
            &locator_sha256,
            committed_before,
        )?;
        acknowledge_intent(&project, &layout, &intent)?;
        observe_install_stage(
            preview,
            started,
            InstallApplyStage::IntentAcknowledged,
            InstallRecoveryState::Complete,
            Some(committed_before),
            observer,
        )?;
        Ok(result)
    }

    pub fn open_installed(project: &Path, executable: &Path) -> Result<Self> {
        let clock = HostSystemClock;
        Self::open_installed_with_clock(project, executable, &clock)
    }

    pub(super) fn open_installed_with_clock(
        project: &Path,
        executable: &Path,
        clock: &dyn ClockSource,
    ) -> Result<Self> {
        let project = canonical_project_root(project)?;
        Self::open_installed_anchored(&project, executable, clock)
    }

    pub(super) fn open_installed_anchored(
        project: &EstateFilesystem,
        executable: &Path,
        clock: &dyn ClockSource,
    ) -> Result<Self> {
        let preflight =
            Self::inspect_installed_anchored(project, executable, clock, "install.open.preflight")?;
        require_accepted(&preflight.clock)?;
        let (project, locator, _) = read_locator_anchored(project)?;
        let (_, profile_sha256) = DefaultInstallProfile::load(locator.profile_id.as_str())?;
        if sha256_file(executable)? != locator.executable_sha256
            || profile_sha256 != locator.profile_sha256
        {
            return Err(ServiceError::ProjectBindingMismatch);
        }
        let token_path = safe_join(&project, &locator.token_key)?;
        let storage_root = safe_join(&project, &locator.storage_root)?;
        let token_key = read_token_key(&token_path)
            .map_err(|error| ServiceError::Storage(error.to_string()))?;
        let mut engine = RrdEngine::open_existing_anchored(
            &storage_root,
            locator.identity.instance_id.clone(),
            token_key,
        )?;
        let bytes = engine
            .storage
            .control()
            .get(INSTALLED_RECORD_KEY)?
            .ok_or_else(|| ServiceError::Storage("installed estate record is missing".into()))?;
        let installed: InstalledEstateRecord = decode_json(&bytes, "installed estate record")?;
        installed.validate()?;
        require_locator_record_match(&locator, &installed)?;
        if locator.identity.instance_id != preflight.instance_id
            || locator.plan_sha256 != preflight.plan_sha256
            || installed.clock_anchor != preflight.clock.anchor
        {
            return Err(ServiceError::ProjectBindingMismatch);
        }
        let configuration_bytes = engine
            .storage
            .control()
            .get(&estate_configuration_key(&locator.identity.instance_id))?
            .ok_or_else(|| ServiceError::Storage("estate configuration is missing".into()))?;
        let configuration: EstateConfiguration =
            decode_json(&configuration_bytes, "estate configuration")?;
        configuration
            .validate()
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        if configuration.revision != locator.configuration_revision
            || configuration.configuration_sha256 != locator.configuration_sha256
        {
            return Err(ServiceError::ProjectBindingMismatch);
        }
        engine.bind_installed_state(
            locator.identity.clone(),
            installed.installation.deployment.clone(),
            configuration,
            safe_join(&project, STORAGE_DIRECTORY)?,
        )?;
        if !engine.security_enforced()? {
            return Err(ServiceError::Storage(
                "installed security authority is missing".into(),
            ));
        }
        Ok(engine)
    }

    pub fn read_installed_api_key(project: &Path) -> Result<ApiKeyDocument> {
        let (project, locator, _) = read_locator(project)?;
        let document = read_api_key_document(&safe_join(&project, &locator.operator_credential)?)
            .map_err(|error| ServiceError::Storage(error.to_string()))?;
        if document.plan_sha256 != locator.plan_sha256 {
            return Err(ServiceError::ProjectBindingMismatch);
        }
        Ok(document)
    }

    /// Reads the public, project-local installed-engine locator. The locator
    /// contains identities and paths but never credential material.
    pub fn read_project_locator(project: &Path) -> Result<ProjectLocator> {
        read_locator(project).map(|(_, locator, _)| locator)
    }

    pub fn inspect_installed(
        project: &Path,
        executable: &Path,
    ) -> Result<InstallationVerificationReport> {
        let clock = HostSystemClock;
        Self::inspect_installed_with_clock(project, executable, &clock)
    }

    pub(super) fn inspect_installed_with_clock(
        project: &Path,
        executable: &Path,
        clock: &dyn ClockSource,
    ) -> Result<InstallationVerificationReport> {
        Self::inspect_installed_with_clock_at_boundary(
            project,
            executable,
            clock,
            "install.inspect",
        )
    }

    fn inspect_installed_with_clock_at_boundary(
        project: &Path,
        executable: &Path,
        clock: &dyn ClockSource,
        boundary: &'static str,
    ) -> Result<InstallationVerificationReport> {
        let project = canonical_project(project, InstallationTargetKind::ExistingProject)?;
        Self::inspect_installed_anchored(&project, executable, clock, boundary)
    }

    pub(super) fn inspect_installed_anchored(
        project: &EstateFilesystem,
        executable: &Path,
        clock: &dyn ClockSource,
        boundary: &'static str,
    ) -> Result<InstallationVerificationReport> {
        let (project, locator, locator_bytes) = read_locator_anchored(project)?;

        let executable_sha256 = sha256_file(executable)?;
        if executable_sha256 != locator.executable_sha256 {
            return Err(ServiceError::ProjectBindingMismatch);
        }
        let (profile, profile_sha256) = DefaultInstallProfile::load(locator.profile_id.as_str())?;
        if profile_sha256 != locator.profile_sha256 {
            return Err(ServiceError::ProjectBindingMismatch);
        }
        let token_key = read_token_key(&safe_join(&project, &locator.token_key)?)
            .map_err(|error| ServiceError::Storage(error.to_string()))?;
        let credential = read_api_key_document(&safe_join(&project, &locator.operator_credential)?)
            .map_err(|error| ServiceError::Storage(error.to_string()))?;
        let storage_root = safe_join(&project, &locator.storage_root)?;
        super::core::validate_existing_object_layout(&storage_root)?;
        let inspector = RrflowKvInspector::open_anchored(&storage_root)?;
        let installed: InstalledEstateRecord =
            decode_control(&inspector, INSTALLED_RECORD_KEY, "installed estate record")?;
        installed.validate()?;
        require_locator_record_match(&locator, &installed)?;
        let configuration: EstateConfiguration = decode_control(
            &inspector,
            &estate_configuration_key(&locator.identity.instance_id),
            "estate configuration",
        )?;
        configuration
            .validate()
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        if configuration.revision != locator.configuration_revision
            || configuration.configuration_sha256 != locator.configuration_sha256
        {
            return Err(ServiceError::ProjectBindingMismatch);
        }
        let clock_assessment = assess(
            clock.observe(),
            &installed.clock_anchor,
            &configuration.clock,
            boundary,
        )?;
        let project_inventory_sha256 = project_inventory_sha256(project.project(), &profile)?;
        let attunement_source_current =
            project_inventory_sha256 == installed.installation.project_precondition_sha256;
        if credential.plan_sha256 != installed.installation.plan_sha256
            || credential.principal_id != installed.installation.initial_principal_id
            || digest::sha256_hex(credential.credential().as_bytes()) != installed.credential_sha256
            || digest::sha256_hex(&token_key) != installed.token_key_sha256
            || inspector.inspection().runtime_cursor < installed.expected_runtime_cursor
            || inspector.inspection().control_journal_sequence
                < installed.bootstrap_control_sequence
        {
            return Err(ServiceError::ProjectBindingMismatch);
        }
        let security_key = format!(
            "server/state/{}/security/policy",
            locator.identity.instance_id
        );
        let security: SecurityState = decode_control(&inspector, &security_key, "security state")?;
        security.validate()?;
        let principal = security
            .principals
            .get(&installed.installation.initial_principal_id)
            .ok_or(ServiceError::Unauthenticated)?;
        if principal.credential_sha256 != installed.credential_sha256
            || principal
                .grants
                .iter()
                .map(|grant| grant.action)
                .collect::<Vec<_>>()
                != installed.installation.initial_grants
        {
            return Err(ServiceError::ProjectBindingMismatch);
        }
        let attunement: AttunementPlan = decode_control(
            &inspector,
            &attunement_plan_key(
                &locator.identity.instance_id,
                &installed.installation.attunement_plan_id,
            ),
            "attunement plan",
        )?;
        installed
            .installation
            .validate_attunement_plan(&attunement)
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        let job_id = canonical_prefixed("job", &attunement.plan_sha256)?;
        let job: AttunementJob = decode_control(
            &inspector,
            &attunement_job_key(&locator.identity.instance_id, &job_id),
            "attunement job",
        )?;
        job.validate(&attunement)
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        let checkpoint: InstallationCheckpoint = decode_control(
            &inspector,
            INSTALL_CHECKPOINT_KEY,
            "installation checkpoint",
        )?;
        checkpoint.validate()?;
        if checkpoint.installed_record_sha256 != installed.record_sha256
            || checkpoint.runtime_cursor != installed.expected_runtime_cursor
            || checkpoint.control_sequence != installed.bootstrap_control_sequence
            || checkpoint.clock_anchor_unix_ms != installed.clock_anchor.observed_at_unix_ms
        {
            return Err(ServiceError::ProjectBindingMismatch);
        }

        let runtime_manifest_sha256 = runtime_manifest_from_inspection(&inspector, &locator)?;
        let evidence = [
            ("locator", digest::sha256_hex(&locator_bytes)),
            ("profile", profile_sha256),
            ("executable", executable_sha256),
            ("project-inventory", project_inventory_sha256.clone()),
            ("physical", json_sha256(&inspector.inspection())?),
            ("installed-record", installed.record_sha256.clone()),
            ("configuration", configuration.configuration_sha256.clone()),
            ("credentials", installed.credential_sha256.clone()),
            ("security", json_sha256(&security)?),
            ("runtime", runtime_manifest_sha256.clone()),
            ("attunement", attunement.plan_sha256),
            ("checkpoint", checkpoint.checkpoint_sha256),
        ];
        let mut checks = evidence
            .into_iter()
            .map(|(id, evidence_sha256)| {
                Ok(InstallationVerificationCheck {
                    id: CanonicalId::new(id)
                        .map_err(|error| ServiceError::Contract(error.to_string()))?,
                    passed: true,
                    evidence_sha256,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        checks.push(InstallationVerificationCheck {
            id: CanonicalId::new("clock")
                .map_err(|error| ServiceError::Contract(error.to_string()))?,
            passed: clock_assessment.passed(),
            evidence_sha256: json_sha256(&clock_assessment)?,
        });
        Ok(InstallationVerificationReport {
            status: if clock_assessment.passed() {
                InstallationVerificationStatus::Passed
            } else {
                InstallationVerificationStatus::Failed
            },
            product_version: locator.product_version,
            instance_id: locator.identity.instance_id,
            plan_sha256: locator.plan_sha256,
            project_inventory_sha256,
            attunement_source_current,
            runtime_manifest_sha256,
            runtime_cursor: inspector.inspection().runtime_cursor,
            control_journal_sequence: inspector.inspection().control_journal_sequence,
            clock: clock_assessment,
            physical: inspector.inspection().clone(),
            checks,
        })
    }
}

fn installation_identity_seed(
    installation_identity: &str,
    target_kind: InstallationTargetKind,
    project_precondition_sha256: &str,
    profile_sha256: &str,
    executable_sha256: &str,
    configuration_sha256: &str,
) -> Result<String> {
    json_sha256(&(
        "rrflow-installed-estate-v3",
        installation_identity,
        target_kind,
        project_precondition_sha256,
        profile_sha256,
        executable_sha256,
        configuration_sha256,
    ))
}

fn initial_project_root_sha256(project: &EstateFilesystem) -> Result<String> {
    let canonical = project
        .project()
        .diagnostic_path()
        .to_str()
        .ok_or(ServiceError::ProjectBindingMismatch)?;
    json_sha256(&("rrflow-initial-project-root-v1", canonical))
}

fn installation_actions(
    configuration_sha256: &str,
    attunement_plan_sha256: &str,
    credential_bytes: u16,
) -> Result<Vec<InstallationPlanAction>> {
    let action_kinds = [
        InstallationActionKind::ValidateProject,
        InstallationActionKind::AcquireInstallationLease,
        InstallationActionKind::CreateEstateDirectories,
        InstallationActionKind::CreateStorageRoot,
        InstallationActionKind::PrepareCredentials,
        InstallationActionKind::InitializeInstance,
        InstallationActionKind::CreateAttunementJob,
        InstallationActionKind::PublishProjectLocator,
    ];
    let estimates = [
        0,
        0,
        0,
        4_096,
        u64::from(credential_bytes) * 2 + 512,
        16_384,
        4_096,
        1_024,
    ];
    action_kinds
        .into_iter()
        .zip(estimates)
        .map(|(kind, estimated_write_bytes)| {
            Ok(InstallationPlanAction {
                kind,
                disposition: if kind == InstallationActionKind::ValidateProject {
                    InstallationActionDisposition::Unchanged
                } else {
                    InstallationActionDisposition::Create
                },
                input_sha256: json_sha256(&(kind, configuration_sha256, attunement_plan_sha256))?,
                estimated_write_bytes,
            })
        })
        .collect()
}

fn validate_sealed_preview(
    preview: &InstallationPreview,
    profile: &DefaultInstallProfile,
    profile_sha256: &str,
    executable_sha256: &str,
) -> Result<()> {
    let installation = &preview.installation;
    let seed = installation_identity_seed(
        &installation.installation_identity,
        installation.target_kind,
        &installation.project_precondition_sha256,
        profile_sha256,
        executable_sha256,
        &installation.configuration.configuration_sha256,
    )?;
    let install_id = canonical_prefixed("install", &seed)?;
    let expected_target = InstalledEstateIdentity {
        project_id: canonical_prefixed("project", &seed)?,
        estate_id: canonical_prefixed("estate", &seed)?,
        instance_id: canonical_prefixed("instance", &seed)?,
    };
    let expected_attunement_id = canonical_prefixed("attune", &seed)?;
    let expected_phases = profile
        .attunement_phases
        .iter()
        .map(|phase| {
            Ok(AttunementPhasePlan {
                phase: phase.phase,
                sequence: phase.phase.sequence(),
                configuration_sha256: json_sha256(phase)?,
                estimated_items: phase.estimated_items,
                estimated_input_bytes: phase.estimated_input_bytes,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let absent = digest::sha256_hex(b"rrflow-install-managed-path-absent-v1");
    let expected_paths =
        EstateLayout::new(&install_id, &profile.initial_principal_id)?.managed_paths(&absent);
    let expected_actions = installation_actions(
        &installation.configuration.configuration_sha256,
        &preview.attunement.plan_sha256,
        profile.credential_bytes,
    )?;
    if installation.id != install_id
        || installation.target != expected_target
        || installation.deployment != profile.deployment
        || installation.product_version != env!("CARGO_PKG_VERSION")
        || installation.executable_sha256 != executable_sha256
        || installation.profile_id != profile.id
        || installation.profile_sha256 != profile_sha256
        || installation.storage_root_id != installation.id
        || installation.configuration.revision != 1
        || installation.initial_seat != profile.initial_seat
        || installation.initial_principal_id != profile.initial_principal_id
        || installation.initial_grants != SecurityAction::ALL
        || installation.credential_bytes != profile.credential_bytes
        || installation.inactive_capabilities != profile.inactive_capabilities
        || installation.managed_paths != expected_paths
        || installation.attunement_plan_id != expected_attunement_id
        || installation.actions != expected_actions
        || preview.attunement.id != expected_attunement_id
        || preview.attunement.target != expected_target
        || preview.attunement.source_sha256 != installation.project_precondition_sha256
        || preview.attunement.phases != expected_phases
        || preview.attunement.estimated_min_duration_ms != profile.estimated_min_duration_ms
        || preview.attunement.estimated_max_duration_ms != profile.estimated_max_duration_ms
    {
        return Err(ServiceError::ProjectBindingMismatch);
    }
    Ok(())
}

fn canonical_project(
    project: &Path,
    target_kind: InstallationTargetKind,
) -> Result<EstateFilesystem> {
    let project = canonical_project_root(project)?;
    validate_installation_target(&project, target_kind)?;
    Ok(project)
}

fn canonical_project_root(project: &Path) -> Result<EstateFilesystem> {
    let project = AnchoredPath::open_root(project).map_err(storage_error)?;
    EstateFilesystem::admit(project, true)
}

fn validate_installation_target(
    project: &EstateFilesystem,
    target_kind: InstallationTargetKind,
) -> Result<()> {
    if target_kind == InstallationTargetKind::FreshProject {
        let admitted_lease = lease::admit_existing(project.project())?;
        for entry in project.project().entries().map_err(storage_error)? {
            let entry = entry.map_err(storage_error)?;
            if admitted_lease.is_some() && entry.relative_path() == Path::new(lease::RELATIVE_PATH)
            {
                continue;
            }
            return Err(ServiceError::ProjectBindingMismatch);
        }
    }
    Ok(())
}

fn observe_install_stage(
    preview: &InstallationPreview,
    started: Instant,
    stage: InstallApplyStage,
    recovery_state: InstallRecoveryState,
    idempotent_replay: Option<bool>,
    observer: &mut dyn FnMut(InstallApplyStage, InstallRecoveryState) -> Result<()>,
) -> Result<()> {
    let elapsed_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    let planned_write_bytes = preview
        .installation
        .actions
        .iter()
        .map(|action| action.estimated_write_bytes)
        .sum::<u64>();
    tracing::debug!(
        target: "rrflow::installation",
        operation = "install.apply",
        plan_sha256 = %preview.installation.plan_sha256,
        instance_id = %preview.installation.target.instance_id,
        stage = stage.as_str(),
        recovery_state = recovery_state.as_str(),
        idempotent_replay_known = idempotent_replay.is_some(),
        idempotent_replay = idempotent_replay.unwrap_or(false),
        planned_managed_path_count = preview.installation.managed_paths.len(),
        planned_write_bytes,
        elapsed_ms,
        "installation durable stage reached"
    );
    observer(stage, recovery_state)
}

fn classify_installation_store(engine: &RrdEngine) -> Result<InstallRecoveryState> {
    let claim_sequence = engine.storage.claims().sequence()?;
    let runtime_cursor = engine.storage.runtime().cursor()?;
    let control_sequence = engine.storage.control().sequence()?;
    let installed = engine.storage.control().get(INSTALLED_RECORD_KEY)?;
    if claim_sequence == 0 && runtime_cursor == 0 && control_sequence == 0 && installed.is_none() {
        return Ok(InstallRecoveryState::EmptyStore);
    }
    if runtime_cursor > 0 && control_sequence > 0 && installed.is_some() {
        return Ok(InstallRecoveryState::Committed);
    }
    Err(ServiceError::StorageConflict(
        "installation recovery found an unclassifiable engine state".into(),
    ))
}

fn require_empty_installation(engine: &RrdEngine, transitions: &[ControlTransition]) -> Result<()> {
    if engine.storage.claims().sequence()? != 0
        || engine.storage.runtime().cursor()? != 0
        || engine.storage.control().sequence()? != 0
    {
        return Err(ServiceError::StorageConflict(
            "installation recovery expected an empty engine".into(),
        ));
    }
    for transition in transitions {
        if engine.storage.control().get(&transition.key)?.is_some() {
            return Err(ServiceError::StorageConflict(format!(
                "installation recovery found unexpected control state at {}",
                transition.key
            )));
        }
    }
    Ok(())
}

fn require_committed_installation(
    storage_root: &AnchoredPath,
    runtime_commit: &RuntimeCommit,
    transitions: &[ControlTransition],
    expected_runtime_cursor: u64,
    expected_control_sequence: u64,
) -> Result<ReadStamp> {
    let inspector = RrflowKvInspector::open_anchored(storage_root)?;
    let inspection = inspector.inspection();
    if inspection.claim_sequence != 0
        || inspection.runtime_cursor != expected_runtime_cursor
        || inspection.control_journal_sequence != expected_control_sequence
    {
        return Err(ServiceError::StorageConflict(
            "installation recovery found a non-canonical committed sequence".into(),
        ));
    }
    for transition in transitions {
        let expected = transition.replacement.as_ref().ok_or_else(|| {
            ServiceError::StorageConflict(
                "installation bootstrap contains a non-create transition".into(),
            )
        })?;
        if inspector.control_record(&transition.key)?.as_ref() != Some(expected) {
            return Err(ServiceError::StorageConflict(format!(
                "installation committed control state differs at {}",
                transition.key
            )));
        }
    }
    let read = inspector.runtime_read_stamp(&runtime_commit.scope)?;
    if read.scope != runtime_commit.scope || read.commit_cursor != expected_runtime_cursor {
        return Err(ServiceError::StorageConflict(
            "installation semantic read stamp differs from the sealed bootstrap".into(),
        ));
    }
    Ok(read)
}

pub(super) fn sha256_file(path: &Path) -> Result<String> {
    let path = AnchoredPath::from_path(path).map_err(storage_error)?;
    let mut file = path.open_read().map_err(storage_error)?;
    let mut hasher = IncrementalSha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| ServiceError::Storage(error.to_string()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher.finalize_hex())
}

pub(super) fn read_locator(project: &Path) -> Result<(EstateFilesystem, ProjectLocator, Vec<u8>)> {
    let project = canonical_project(project, InstallationTargetKind::ExistingProject)?;
    read_locator_anchored(&project)
}

fn read_locator_anchored(
    project: &EstateFilesystem,
) -> Result<(EstateFilesystem, ProjectLocator, Vec<u8>)> {
    validate_estate_directories(project, false)?;
    let path = safe_join(project, LOCATOR_RELATIVE_PATH)?;
    let bytes = read_regular_bounded(&path, MAX_LOCATOR_BYTES)?;
    let locator: ProjectLocator = toml::from_str(
        std::str::from_utf8(&bytes).map_err(|error| ServiceError::Contract(error.to_string()))?,
    )
    .map_err(|error| ServiceError::Contract(error.to_string()))?;
    locator.validate()?;
    Ok((project.clone(), locator, bytes))
}

fn publish_new_file(path: &AnchoredPath, temporary: &AnchoredPath, bytes: &[u8]) -> Result<()> {
    let parent = path.parent().ok_or(ServiceError::ProjectBindingMismatch)?;
    if bytes.len() as u64 > MAX_LOCATOR_BYTES {
        return Err(ServiceError::ProjectBindingMismatch);
    }

    match path.metadata() {
        Ok(_) => {
            if read_regular_bounded(path, MAX_LOCATOR_BYTES)? != bytes {
                return Err(ServiceError::ProjectBindingMismatch);
            }
            return cleanup_pending_file(temporary, bytes);
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(ServiceError::Storage(error.to_string())),
    }

    let mut create_temporary = true;
    match temporary.metadata() {
        Ok(_) => {
            if read_regular_bounded(temporary, MAX_LOCATOR_BYTES)? == bytes {
                create_temporary = false;
            } else {
                return Err(ServiceError::ProjectBindingMismatch);
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(ServiceError::Storage(error.to_string())),
    }
    if create_temporary {
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        let write_result = (|| -> std::io::Result<()> {
            let mut file = options.open(temporary)?;
            file.write_all(bytes)?;
            file.sync_all()
        })();
        if let Err(error) = write_result {
            let _ = temporary.remove_file();
            return Err(ServiceError::Storage(error.to_string()));
        }
    }

    match temporary.rename_new_to(path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            if read_regular_bounded(path, MAX_LOCATOR_BYTES)? != bytes {
                return Err(ServiceError::ProjectBindingMismatch);
            }
        }
        Err(error) => return Err(ServiceError::Storage(error.to_string())),
    }
    parent
        .sync_directory()
        .map_err(|error| ServiceError::Storage(error.to_string()))?;
    cleanup_pending_file(temporary, bytes)
}

fn cleanup_locator_pending(
    project: &EstateFilesystem,
    layout: &EstateLayout,
    locator_bytes: &[u8],
) -> Result<()> {
    cleanup_pending_file(&safe_join(project, &layout.locator_pending)?, locator_bytes)
}

fn cleanup_pending_file(path: &AnchoredPath, expected: &[u8]) -> Result<()> {
    match path.metadata() {
        Ok(_) => {
            if read_regular_bounded(path, MAX_LOCATOR_BYTES)? != expected {
                return Err(ServiceError::ProjectBindingMismatch);
            }
            path.remove_file()
                .map_err(|error| ServiceError::Storage(error.to_string()))?;
            path.parent()
                .ok_or(ServiceError::ProjectBindingMismatch)?
                .sync_directory()
                .map_err(|error| ServiceError::Storage(error.to_string()))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(ServiceError::Storage(error.to_string())),
    }
}

pub(super) fn read_regular_bounded(path: &AnchoredPath, maximum: u64) -> Result<Vec<u8>> {
    let file = path.open_read().map_err(storage_error)?;
    let metadata = file
        .metadata()
        .map_err(|error| ServiceError::Storage(error.to_string()))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() || metadata.len() > maximum {
        return Err(ServiceError::ProjectBindingMismatch);
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.take(maximum + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| ServiceError::Storage(error.to_string()))?;
    if bytes.len() as u64 > maximum {
        return Err(ServiceError::ProjectBindingMismatch);
    }
    Ok(bytes)
}

fn replay_installation_result(
    project: &EstateFilesystem,
    preview: &InstallationPreview,
    executable: &Path,
    clock: &dyn ClockSource,
) -> Result<InstallationResult> {
    let report =
        RrdEngine::inspect_installed_anchored(project, executable, clock, "install.apply.replay")?;
    require_accepted(&report.clock)?;
    if report.plan_sha256 != preview.installation.plan_sha256 {
        return Err(ServiceError::IdempotencyConflict);
    }
    let (_, locator, locator_bytes) = read_locator_anchored(project)?;
    let inspector = RrflowKvInspector::open_anchored(&safe_join(project, &locator.storage_root)?)?;
    let installed: InstalledEstateRecord =
        decode_control(&inspector, INSTALLED_RECORD_KEY, "installed estate record")?;
    let job_id = canonical_prefixed("job", &preview.attunement.plan_sha256)?;
    build_installation_result(
        preview,
        &job_id,
        &installed,
        &report.runtime_manifest_sha256,
        report.runtime_cursor,
        installed.bootstrap_control_sequence,
        &installed.credential_sha256,
        &digest::sha256_hex(&locator_bytes),
        true,
    )
}

#[allow(clippy::too_many_arguments)]
fn build_installation_result(
    preview: &InstallationPreview,
    job_id: &CanonicalId,
    installed: &InstalledEstateRecord,
    runtime_manifest_sha256: &str,
    runtime_cursor: u64,
    control_journal_sequence: u64,
    credential_sha256: &str,
    locator_sha256: &str,
    idempotent_replay: bool,
) -> Result<InstallationResult> {
    let outputs = [
        preview.installation.project_precondition_sha256.clone(),
        json_sha256(
            preview
                .installation
                .managed_path(InstallationManagedPathKind::InstallationLease),
        )?,
        json_sha256(&preview.installation.managed_paths[..4])?,
        json_sha256(&(
            &preview.installation.storage_root_id,
            preview
                .installation
                .managed_path(InstallationManagedPathKind::StorageRoot),
        ))?,
        credential_sha256.into(),
        installed.runtime_commit_sha256.clone(),
        preview.attunement.plan_sha256.clone(),
        locator_sha256.into(),
    ];
    let mut result = InstallationResult {
        contract_version: INSTALL_ATTUNEMENT_CONTRACT_VERSION,
        plan_id: preview.installation.id.clone(),
        plan_sha256: preview.installation.plan_sha256.clone(),
        attunement_job_id: job_id.clone(),
        action_results: preview
            .installation
            .actions
            .iter()
            .zip(outputs)
            .map(|(action, output_sha256)| InstallationActionResult {
                kind: action.kind,
                output_sha256,
            })
            .collect(),
        runtime_manifest_sha256: runtime_manifest_sha256.into(),
        runtime_cursor,
        control_journal_sequence,
        installed_record_sha256: installed.record_sha256.clone(),
        credential_sha256: credential_sha256.into(),
        locator_sha256: locator_sha256.into(),
        applied_at_unix_ms: installed.clock_anchor.observed_at_unix_ms,
        idempotent_replay,
        result_sha256: zero_digest(),
    };
    result.result_sha256 = installation_result_sha256(&result)
        .map_err(|error| ServiceError::Contract(error.to_string()))?;
    result
        .validate_for(&preview.installation)
        .map_err(|error| ServiceError::Contract(error.to_string()))?;
    Ok(result)
}

fn control_create<T: Serialize>(
    key: &str,
    value: &T,
    at: u64,
    request_id: &str,
    operation_id: &str,
    action: &str,
) -> Result<ControlTransition> {
    let transition = ControlTransition {
        key: key.into(),
        expected: None,
        replacement: Some(serde_json::to_vec(value).map_err(contract_json)?),
        at,
        actor: "rrflow:installer".into(),
        action: action.into(),
        request_id: request_id.into(),
        operation_id: operation_id.into(),
    };
    transition.validate()?;
    Ok(transition)
}

pub(super) fn decode_control<T: DeserializeOwned>(
    inspector: &RrflowKvInspector,
    key: &str,
    label: &str,
) -> Result<T> {
    let bytes = inspector
        .control_record(key)?
        .ok_or_else(|| ServiceError::Storage(format!("{label} is missing")))?;
    decode_json(&bytes, label)
}

fn decode_json<T: DeserializeOwned>(bytes: &[u8], label: &str) -> Result<T> {
    serde_json::from_slice(bytes)
        .map_err(|error| ServiceError::Storage(format!("{label} is invalid: {error}")))
}

pub(super) fn require_locator_record_match(
    locator: &ProjectLocator,
    installed: &InstalledEstateRecord,
) -> Result<()> {
    if locator.plan_sha256 != installed.installation.plan_sha256
        || locator.profile_id != installed.installation.profile_id
        || locator.profile_sha256 != installed.installation.profile_sha256
        || locator.executable_sha256 != installed.installation.executable_sha256
        || locator.installed_record_sha256 != installed.record_sha256
        || locator.identity != installed.installation.target
        || locator.configuration_revision != installed.installation.configuration.revision
        || locator.configuration_sha256 != installed.installation.configuration.configuration_sha256
        || locator.storage_root
            != installed
                .installation
                .managed_path(InstallationManagedPathKind::StorageRoot)
                .relative_path
        || locator.token_key
            != installed
                .installation
                .managed_path(InstallationManagedPathKind::TokenKey)
                .relative_path
        || locator.operator_credential
            != installed
                .installation
                .managed_path(InstallationManagedPathKind::OperatorCredential)
                .relative_path
    {
        return Err(ServiceError::ProjectBindingMismatch);
    }
    Ok(())
}

fn attunement_plan_key(instance: &CanonicalId, plan: &CanonicalId) -> String {
    format!("server/state/{instance}/attunement/plan/{plan}")
}

fn attunement_job_key(instance: &CanonicalId, job: &CanonicalId) -> String {
    format!("server/state/{instance}/attunement/job/{job}")
}

pub(super) fn estate_configuration_key(instance: &CanonicalId) -> String {
    format!("server/state/{instance}/configuration/active")
}

pub(super) fn runtime_manifest_from_inspection(
    inspector: &RrflowKvInspector,
    locator: &ProjectLocator,
) -> Result<String> {
    let scope = ScopeId::new(format!("instance:{}", locator.identity.instance_id))
        .map_err(core_contract)?;
    let read = inspector.runtime_read_stamp(&scope)?;
    if read.commit_cursor != inspector.inspection().runtime_cursor {
        return Err(ServiceError::StorageConflict(
            "semantic read stamp and inspected runtime cursor differ".into(),
        ));
    }
    Ok(read.manifest_id)
}

fn installed_record_sha256(record: &InstalledEstateRecord) -> Result<String> {
    json_sha256(&(
        record.format_version,
        &record.installation,
        &record.attunement_plan_sha256,
        &record.credential_sha256,
        &record.token_key_sha256,
        &record.runtime_commit_sha256,
        record.expected_runtime_cursor,
        record.bootstrap_control_sequence,
        &record.clock_anchor,
    ))
}

fn installation_checkpoint_sha256(checkpoint: &InstallationCheckpoint) -> Result<String> {
    json_sha256(&(
        checkpoint.format_version,
        &checkpoint.plan_sha256,
        &checkpoint.runtime_commit_sha256,
        checkpoint.runtime_cursor,
        checkpoint.control_sequence,
        &checkpoint.installed_record_sha256,
        checkpoint.clock_anchor_unix_ms,
    ))
}

pub(super) fn json_sha256<T: Serialize + ?Sized>(value: &T) -> Result<String> {
    serde_json::to_vec(value)
        .map(|bytes| digest::sha256_hex(&bytes))
        .map_err(contract_json)
}

pub(super) fn canonical_prefixed(prefix: &str, sha256: &str) -> Result<CanonicalId> {
    if !is_sha256(sha256) {
        return Err(ServiceError::Contract(
            "identity seed is not SHA-256".into(),
        ));
    }
    CanonicalId::new(format!("{prefix}-{}", &sha256[..24]))
        .map_err(|error| ServiceError::Contract(error.to_string()))
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn zero_digest() -> String {
    "0".repeat(64)
}

fn storage_error(error: std::io::Error) -> ServiceError {
    ServiceError::Storage(error.to_string())
}
