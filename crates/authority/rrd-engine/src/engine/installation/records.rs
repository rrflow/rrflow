//! Installation preview, project locator, installed record, checkpoint and verification types.

use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InstallationPreview {
    pub preview_format_version: u16,
    pub installation: InstallationPlan,
    pub attunement: AttunementPlan,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attunement_proposal: Option<AttunementProposal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_read_resource_scope: Option<String>,
}

impl InstallationPreview {
    pub fn validate(&self) -> Result<()> {
        if self.preview_format_version != INSTALLATION_PREVIEW_FORMAT_VERSION {
            return Err(ServiceError::Contract(
                "unsupported installation preview version".into(),
            ));
        }
        if let Some(proposal) = &self.attunement_proposal {
            proposal.validate()?;
            if self.attunement.phases != planned_attunement_steps(&proposal.normalized_steps())?
                || self.attunement.estimated_min_duration_ms != proposal.estimated_min_duration_ms
                || self.attunement.estimated_max_duration_ms != proposal.estimated_max_duration_ms
            {
                return Err(ServiceError::ProjectBindingMismatch);
            }
        }
        let source_read_granted = self
            .installation
            .initial_grants
            .contains(&SecurityAction::ProjectSourceRead);
        if source_read_granted != self.source_read_resource_scope.is_some()
            || self
                .source_read_resource_scope
                .as_deref()
                .is_some_and(|scope| scope != "instance_prefix")
        {
            return Err(ServiceError::Contract(
                "project source-read authority and resource scope do not match".into(),
            ));
        }
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
    /// Pre-open projection, verified against the sealed installed record.
    pub storage: rrd_contract::StorageLimits,
}

impl ProjectLocator {
    pub(super) fn validate(&self) -> Result<()> {
        super::super::core::installed_storage_options(&self.storage)?;
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
    pub(in super::super) fn validate(&self) -> Result<()> {
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
                "installed engram record is inconsistent or corrupt".into(),
            ));
        }
        self.clock_anchor.validate()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct InstallationCheckpoint {
    pub(super) format_version: u16,
    pub(super) plan_sha256: String,
    pub(super) runtime_commit_sha256: String,
    pub(super) runtime_cursor: u64,
    pub(super) control_sequence: u64,
    pub(super) installed_record_sha256: String,
    pub(super) clock_anchor_unix_ms: u64,
    pub(super) checkpoint_sha256: String,
}

impl InstallationCheckpoint {
    pub(super) fn validate(&self) -> Result<()> {
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

// Private inspection facts deliberately contain no claimed current-source
// digest. Opening the engram consumes its verified binding; explicit full
// inspection additionally observes project inputs and constructs the report.
pub(super) struct InstalledBindingInspection {
    pub(super) profile: DefaultInstallProfile,
    pub(super) locator: ProjectLocator,
    pub(super) source_precondition_sha256: String,
    pub(super) runtime_manifest_sha256: String,
    pub(super) clock: ClockAssessment,
    pub(super) physical: rrd_store::RrflowKvInspection,
    pub(super) checks: Vec<InstallationVerificationCheck>,
}
