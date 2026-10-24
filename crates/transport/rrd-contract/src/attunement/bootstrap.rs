//! Bundle-resident installation inputs and their governed initial effects.
//! These values describe effects and configuration, never private reasoning.
use super::*;
use crate::EndpointProfile;

pub const INSTALLATION_BOOTSTRAP_VERSION: u16 = 1;
pub const MAX_BOOTSTRAP_TEXT_BYTES: usize = 64 * 1024;
pub const MAX_BOOTSTRAP_RECORD_BYTES: u64 = 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InstallationSpecialization {
    pub format_version: u16,
    pub id: CanonicalId,
    pub revision: u64,
    pub seat: MemorySeatDefinition,
    pub instructions: String,
    /// A configured profile is a declaration only. Installation activates no
    /// provider binding, representation, credential resolver or endpoint.
    pub endpoint_profile: Option<EndpointProfile>,
}

impl InstallationSpecialization {
    pub fn validate(&self) -> Result<()> {
        if self.format_version != INSTALLATION_BOOTSTRAP_VERSION || self.revision == 0 {
            return invalid("unsupported specialization format or zero revision");
        }
        self.seat.validate()?;
        validate_text(&self.instructions)?;
        if let Some(profile) = &self.endpoint_profile {
            profile.validate()?;
        }
        Ok(())
    }

    pub fn digest(&self) -> Result<String> {
        self.validate()?;
        let bytes = serde_json::to_vec(&("rrflow-install-specialization-v1", self))
            .map_err(|error| crate::ContractError(error.to_string()))?;
        Ok(sha256_bytes(&bytes))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum InstallationRecordKind {
    SeatSchema,
    SeatRecord,
    SecurityAuthority,
    SecurityGenesisAudit,
    SecurityAuditHead,
    EffectiveConfiguration,
    InstalledBinding,
    InstallationCheckpoint,
    AttunementPlan,
    PendingAttunementJob,
}

impl InstallationRecordKind {
    pub const INITIAL: [Self; 10] = [
        Self::SeatSchema,
        Self::SeatRecord,
        Self::SecurityAuthority,
        Self::SecurityGenesisAudit,
        Self::SecurityAuditHead,
        Self::EffectiveConfiguration,
        Self::InstalledBinding,
        Self::InstallationCheckpoint,
        Self::AttunementPlan,
        Self::PendingAttunementJob,
    ];
}

/// The physical generator remains rrflowKV. Its format coordinates and the
/// admitted initial semantic payload bind the allowed generated subtree.
/// Actual physical inventory and manifest digests are verification evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InstallationStorageGeneration {
    pub application_format: u64,
    pub manifest_format: u16,
    pub wal_format: u16,
    pub batch_format: u16,
    pub max_initial_transaction_bytes: u64,
}

impl InstallationStorageGeneration {
    fn validate(&self) -> Result<()> {
        if self.application_format == 0
            || self.manifest_format == 0
            || self.wal_format == 0
            || self.batch_format == 0
            || self.max_initial_transaction_bytes == 0
            || self.max_initial_transaction_bytes > 16 * MAX_BOOTSTRAP_RECORD_BYTES
        {
            return invalid("installation storage generation has invalid formats or bounds");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum InstallationBootstrapPolicy {
    /// Retain project-owned files and permissions; no instruction inclusion.
    PreserveProjectOwnedState,
}

/// Exact static bytes and typed dynamic-generation inputs sealed by the plan.
/// Clock and entropy values are obtained only during authorized apply.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InstallationBootstrap {
    pub format_version: u16,
    pub template_id: CanonicalId,
    pub template_revision: u64,
    pub template_text: String,
    pub template_sha256: String,
    pub specialization: InstallationSpecialization,
    pub specialization_sha256: String,
    pub instructions_sha256: String,
    pub project_policy: InstallationBootstrapPolicy,
    pub initial_records: Vec<InstallationRecordKind>,
    pub max_record_bytes: u64,
    pub storage_generation: InstallationStorageGeneration,
    /// OS entropy, prepared once in the private installation intent.
    pub token_key_bytes: u16,
    pub credential_entropy_bytes: u16,
    /// Native owner/mode/DACL enforcement, before secret or storage effects.
    pub private_directories: Vec<String>,
    pub private_files: Vec<String>,
    /// Explicitly empty until a separately authorized external-capability plan.
    pub process_invocations: Vec<CanonicalId>,
    pub network_endpoints: Vec<String>,
    pub activated_provider_bindings: Vec<CanonicalId>,
}

impl InstallationBootstrap {
    pub fn rendered_instructions(&self) -> String {
        format!(
            "{}\n## Installed specialization\n\n{}\n\n{}\n\nSpecialization: {} revision {}\nManifest SHA-256: {}\n",
            self.template_text,
            self.specialization.seat.display_name,
            self.specialization.instructions,
            self.specialization.id,
            self.specialization.revision,
            self.specialization_sha256,
        )
    }

    pub fn validate(&self) -> Result<()> {
        if self.format_version != INSTALLATION_BOOTSTRAP_VERSION
            || self.template_revision == 0
            || self.max_record_bytes == 0
            || self.max_record_bytes > MAX_BOOTSTRAP_RECORD_BYTES
            || self.token_key_bytes != 32
            || !(32..=4096).contains(&self.credential_entropy_bytes)
        {
            return invalid("installation bootstrap version, entropy or record bound is invalid");
        }
        validate_text(&self.template_text)?;
        validate_text(&self.rendered_instructions())?;
        self.specialization.validate()?;
        self.storage_generation.validate()?;
        if self.template_sha256 != sha256_bytes(self.template_text.as_bytes())
            || self.specialization_sha256 != self.specialization.digest()?
            || self.instructions_sha256 != sha256_bytes(self.rendered_instructions().as_bytes())
            || self.initial_records != InstallationRecordKind::INITIAL
            || !self.process_invocations.is_empty()
            || !self.network_endpoints.is_empty()
            || !self.activated_provider_bindings.is_empty()
        {
            return invalid(
                "bootstrap digest, record closure or inactive external-effect policy differs",
            );
        }
        for paths in [&self.private_directories, &self.private_files] {
            if paths.is_empty()
                || paths.len() > 16
                || paths.windows(2).any(|pair| pair[0] >= pair[1])
                || paths.iter().any(|path| path.is_empty() || path.len() > 256)
            {
                return invalid("bootstrap private paths must be bounded, sorted and unique");
            }
        }
        Ok(())
    }
}

fn validate_text(text: &str) -> Result<()> {
    if text.trim().is_empty() || text.len() > MAX_BOOTSTRAP_TEXT_BYTES || text.contains('\0') {
        return invalid("bootstrap text must be nonempty, bounded UTF-8 without NUL");
    }
    Ok(())
}
