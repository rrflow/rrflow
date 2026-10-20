use super::*;
use crate::engine::estate_layout::{install_intent_path, EstateLayout};
use crate::engine::token_key::{
    decode_api_key_document, encode_api_key_document, generate_api_key_document,
    generate_token_key, private_regular_file, write_private_new, ApiKeyDocument, TOKEN_KEY_BYTES,
};
use std::fs::File;
use std::io::Read;

const INSTALL_INTENT_FORMAT: u16 = 2;
const MAX_INSTALL_INTENT_BYTES: u64 = 32 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(in crate::engine) enum InstallApplyStage {
    IntentPublished,
    EstateDirectoriesReady,
    StorageReady,
    TokenReady,
    CredentialReady,
    EngineCommitted,
    LocatorPublished,
    IntentAcknowledged,
}

impl InstallApplyStage {
    pub(super) const fn as_str(self) -> &'static str {
        match self {
            Self::IntentPublished => "intent_published",
            Self::EstateDirectoriesReady => "estate_directories_ready",
            Self::StorageReady => "storage_ready",
            Self::TokenReady => "token_ready",
            Self::CredentialReady => "credential_ready",
            Self::EngineCommitted => "engine_committed",
            Self::LocatorPublished => "locator_published",
            Self::IntentAcknowledged => "intent_acknowledged",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::engine) enum InstallRecoveryState {
    NewIntent,
    OwnedIntent,
    EmptyStore,
    Committed,
    Published,
    Complete,
}

impl InstallRecoveryState {
    pub(super) const fn as_str(self) -> &'static str {
        match self {
            Self::NewIntent => "new_intent",
            Self::OwnedIntent => "owned_intent",
            Self::EmptyStore => "empty_store",
            Self::Committed => "committed",
            Self::Published => "published",
            Self::Complete => "complete",
        }
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct InstallIntent {
    format_version: u16,
    stage: InstallApplyStage,
    plan_sha256: String,
    target_kind: InstallationTargetKind,
    target: InstalledEstateIdentity,
    project_precondition_sha256: String,
    profile_id: CanonicalId,
    profile_sha256: String,
    executable_sha256: String,
    configuration_sha256: String,
    clock_anchor: ClockObservation,
    token_key_bytes: Vec<u8>,
    token_key_sha256: String,
    operator_credential_bytes: Vec<u8>,
    credential_sha256: String,
    intent_sha256: String,
}

impl std::fmt::Debug for InstallIntent {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("InstallIntent")
            .field("format_version", &self.format_version)
            .field("stage", &self.stage)
            .field("plan_sha256", &self.plan_sha256)
            .field("target_kind", &self.target_kind)
            .field("target", &self.target)
            .field(
                "project_precondition_sha256",
                &self.project_precondition_sha256,
            )
            .field("profile_id", &self.profile_id)
            .field("profile_sha256", &self.profile_sha256)
            .field("executable_sha256", &self.executable_sha256)
            .field("configuration_sha256", &self.configuration_sha256)
            .field("clock_anchor", &self.clock_anchor)
            .field("token_key_bytes", &"[REDACTED]")
            .field("token_key_sha256", &self.token_key_sha256)
            .field("operator_credential_bytes", &"[REDACTED]")
            .field("credential_sha256", &self.credential_sha256)
            .field("intent_sha256", &self.intent_sha256)
            .finish()
    }
}

impl InstallIntent {
    fn new(preview: &InstallationPreview, clock_anchor: &ClockObservation) -> Result<Self> {
        let token_key = generate_token_key().map_err(storage_error)?;
        let credential = generate_api_key_document(
            &preview.installation.plan_sha256,
            preview.installation.initial_principal_id.clone(),
            preview.installation.credential_bytes,
        )
        .map_err(storage_error)?;
        let operator_credential_bytes =
            encode_api_key_document(&credential).map_err(storage_error)?;
        let mut intent = Self {
            format_version: INSTALL_INTENT_FORMAT,
            stage: InstallApplyStage::IntentPublished,
            plan_sha256: preview.installation.plan_sha256.clone(),
            target_kind: preview.installation.target_kind,
            target: preview.installation.target.clone(),
            project_precondition_sha256: preview.installation.project_precondition_sha256.clone(),
            profile_id: preview.installation.profile_id.clone(),
            profile_sha256: preview.installation.profile_sha256.clone(),
            executable_sha256: preview.installation.executable_sha256.clone(),
            configuration_sha256: preview
                .installation
                .configuration
                .configuration_sha256
                .clone(),
            clock_anchor: clock_anchor.clone(),
            token_key_bytes: token_key.to_vec(),
            token_key_sha256: digest::sha256_hex(&token_key),
            operator_credential_bytes,
            credential_sha256: digest::sha256_hex(credential.credential().as_bytes()),
            intent_sha256: zero_digest(),
        };
        intent.intent_sha256 = intent.calculated_sha256()?;
        Ok(intent)
    }

    fn calculated_sha256(&self) -> Result<String> {
        json_sha256(&(
            self.format_version,
            self.stage,
            &self.plan_sha256,
            self.target_kind,
            &self.target,
            &self.project_precondition_sha256,
            &self.profile_id,
            &self.profile_sha256,
            &self.executable_sha256,
            &self.configuration_sha256,
            &self.clock_anchor,
            &self.token_key_bytes,
            &self.token_key_sha256,
            &self.operator_credential_bytes,
            &self.credential_sha256,
        ))
    }

    fn encoded(&self) -> Result<Vec<u8>> {
        serde_json::to_vec(self).map_err(contract_json)
    }

    fn validate(&self, preview: &InstallationPreview, executable_sha256: &str) -> Result<()> {
        let token_key = self.token_key()?;
        let credential = self.credential()?;
        if self.format_version != INSTALL_INTENT_FORMAT
            || self.stage != InstallApplyStage::IntentPublished
            || self.plan_sha256 != preview.installation.plan_sha256
            || self.target_kind != preview.installation.target_kind
            || self.target != preview.installation.target
            || self.project_precondition_sha256 != preview.installation.project_precondition_sha256
            || self.profile_id != preview.installation.profile_id
            || self.profile_sha256 != preview.installation.profile_sha256
            || self.executable_sha256 != executable_sha256
            || self.executable_sha256 != preview.installation.executable_sha256
            || self.configuration_sha256 != preview.installation.configuration.configuration_sha256
            || self.token_key_sha256 != digest::sha256_hex(&token_key)
            || credential.plan_sha256 != preview.installation.plan_sha256
            || credential.principal_id != preview.installation.initial_principal_id
            || credential.credential_bytes != preview.installation.credential_bytes
            || self.credential_sha256 != digest::sha256_hex(credential.credential().as_bytes())
            || self.intent_sha256 != self.calculated_sha256()?
        {
            return Err(ServiceError::ProjectBindingMismatch);
        }
        self.clock_anchor.validate()
    }

    pub(super) fn clock_anchor(&self) -> &ClockObservation {
        &self.clock_anchor
    }

    pub(super) fn token_key(&self) -> Result<[u8; TOKEN_KEY_BYTES]> {
        self.token_key_bytes
            .as_slice()
            .try_into()
            .map_err(|_| ServiceError::ProjectBindingMismatch)
    }

    pub(super) fn token_key_sha256(&self) -> &str {
        &self.token_key_sha256
    }

    pub(super) fn credential(&self) -> Result<ApiKeyDocument> {
        decode_api_key_document(&self.operator_credential_bytes).map_err(storage_error)
    }

    pub(super) fn credential_bytes(&self) -> &[u8] {
        &self.operator_credential_bytes
    }

    pub(super) fn credential_sha256(&self) -> &str {
        &self.credential_sha256
    }
}

pub(super) fn load_or_create_intent(
    project: &Path,
    layout: &EstateLayout,
    preview: &InstallationPreview,
    clock_anchor: &ClockObservation,
    executable_sha256: &str,
) -> Result<(InstallIntent, InstallRecoveryState)> {
    let path = install_intent_path(project, layout)?;
    match std::fs::symlink_metadata(&path) {
        Ok(_) => {
            let intent = read_intent(&path)?;
            intent.validate(preview, executable_sha256)?;
            Ok((intent, InstallRecoveryState::OwnedIntent))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let intent = InstallIntent::new(preview, clock_anchor)?;
            intent.validate(preview, executable_sha256)?;
            write_private_new(&path, &intent.encoded()?).map_err(storage_error)?;
            let readback = read_intent(&path)?;
            if readback != intent {
                return Err(ServiceError::ProjectBindingMismatch);
            }
            Ok((readback, InstallRecoveryState::NewIntent))
        }
        Err(error) => Err(storage_error(error)),
    }
}

pub(super) fn read_existing_intent(
    project: &Path,
    layout: &EstateLayout,
    preview: &InstallationPreview,
    executable_sha256: &str,
) -> Result<Option<InstallIntent>> {
    let path = install_intent_path(project, layout)?;
    match std::fs::symlink_metadata(&path) {
        Ok(_) => {
            let intent = read_intent(&path)?;
            intent.validate(preview, executable_sha256)?;
            Ok(Some(intent))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(storage_error(error)),
    }
}

pub(super) fn materialize_secret(path: &Path, expected: &[u8], allow_create: bool) -> Result<()> {
    match std::fs::symlink_metadata(path) {
        Ok(_) => {
            let observed = read_private_bounded(path, MAX_INSTALL_INTENT_BYTES)?;
            if observed != expected {
                return Err(ServiceError::ProjectBindingMismatch);
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound && allow_create => {
            write_private_new(path, expected).map_err(storage_error)?;
            if read_private_bounded(path, MAX_INSTALL_INTENT_BYTES)? != expected {
                return Err(ServiceError::ProjectBindingMismatch);
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(ServiceError::ProjectBindingMismatch);
        }
        Err(error) => return Err(storage_error(error)),
    }
    Ok(())
}

pub(super) fn acknowledge_intent(
    project: &Path,
    layout: &EstateLayout,
    expected: &InstallIntent,
) -> Result<()> {
    let path = install_intent_path(project, layout)?;
    if read_intent(&path)? != *expected {
        return Err(ServiceError::ProjectBindingMismatch);
    }
    std::fs::remove_file(&path).map_err(storage_error)?;
    rrd_store::sync_directory_metadata(project).map_err(storage_error)
}

fn read_intent(path: &Path) -> Result<InstallIntent> {
    let bytes = read_private_bounded(path, MAX_INSTALL_INTENT_BYTES)?;
    let intent: InstallIntent = serde_json::from_slice(&bytes).map_err(contract_json)?;
    if intent.encoded()? != bytes {
        return Err(ServiceError::ProjectBindingMismatch);
    }
    Ok(intent)
}

fn read_private_bounded(path: &Path, maximum: u64) -> Result<Vec<u8>> {
    let metadata = private_regular_file(path).map_err(storage_error)?;
    if metadata.len() > maximum {
        return Err(ServiceError::ProjectBindingMismatch);
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    File::open(path)
        .map_err(storage_error)?
        .take(maximum + 1)
        .read_to_end(&mut bytes)
        .map_err(storage_error)?;
    if bytes.len() as u64 > maximum {
        return Err(ServiceError::ProjectBindingMismatch);
    }
    Ok(bytes)
}

fn storage_error(error: std::io::Error) -> ServiceError {
    ServiceError::Storage(error.to_string())
}
