//! Locator and record files, control-record coding and installation digests.

use super::*;

pub(in super::super) fn sha256_file(path: &Path) -> Result<String> {
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

pub(in super::super) fn read_locator(
    project: &Path,
) -> Result<(EstateFilesystem, ProjectLocator, Vec<u8>)> {
    let project = canonical_project(project, InstallationTargetKind::ExistingProject)?;
    read_locator_anchored(&project)
}

pub(in super::super) fn read_locator_anchored(
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

pub(in super::super) fn publish_new_file(
    path: &AnchoredPath,
    temporary: &AnchoredPath,
    bytes: &[u8],
) -> Result<()> {
    publish_new_file_with_stage(path, temporary, bytes, &mut || Ok(()))
}

pub(super) fn publish_new_file_with_stage(
    path: &AnchoredPath,
    temporary: &AnchoredPath,
    bytes: &[u8],
    staged: &mut dyn FnMut() -> Result<()>,
) -> Result<()> {
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

    staged()?;
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

pub(super) fn cleanup_locator_pending(
    project: &EstateFilesystem,
    layout: &EstateLayout,
    locator_bytes: &[u8],
) -> Result<()> {
    cleanup_pending_file(&safe_join(project, &layout.locator_pending)?, locator_bytes)
}

pub(super) fn cleanup_pending_file(path: &AnchoredPath, expected: &[u8]) -> Result<()> {
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

pub(in super::super) fn read_regular_bounded(path: &AnchoredPath, maximum: u64) -> Result<Vec<u8>> {
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

pub(super) fn replay_installation_result(
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
    let inspector = RrflowKvInspector::open_anchored_with_options(
        &safe_join(project, &locator.storage_root)?,
        super::super::core::installed_storage_options(&locator.storage)?,
    )?;
    let installed: InstalledEstateRecord =
        decode_control(&inspector, INSTALLED_RECORD_KEY, "installed engram record")?;
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
pub(super) fn build_installation_result(
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
        credential_sha256.into(),
        json_sha256(&preview.installation.managed_paths[..4])?,
        preview.installation.bootstrap.instructions_sha256.clone(),
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

pub(super) fn control_create<T: Serialize>(
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

pub(in super::super) fn decode_control<T: DeserializeOwned>(
    inspector: &RrflowKvInspector,
    key: &str,
    label: &str,
) -> Result<T> {
    let bytes = inspector
        .control_record(key)?
        .ok_or_else(|| ServiceError::Storage(format!("{label} is missing")))?;
    decode_json(&bytes, label)
}

pub(super) fn decode_json<T: DeserializeOwned>(bytes: &[u8], label: &str) -> Result<T> {
    serde_json::from_slice(bytes)
        .map_err(|error| ServiceError::Storage(format!("{label} is invalid: {error}")))
}

pub(in super::super) fn require_locator_record_match(
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
        || locator.storage != installed.installation.configuration.storage
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

pub(in super::super) fn attunement_plan_key(instance: &CanonicalId, plan: &CanonicalId) -> String {
    format!("server/state/{instance}/attunement/plan/{plan}")
}

pub(in super::super) fn attunement_job_key(instance: &CanonicalId, job: &CanonicalId) -> String {
    format!("server/state/{instance}/attunement/job/{job}")
}

pub(in super::super) fn engram_configuration_key(instance: &CanonicalId) -> String {
    format!("server/state/{instance}/configuration/active")
}

pub(in super::super) fn runtime_manifest_from_inspection(
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

pub(super) fn installed_record_sha256(record: &InstalledEstateRecord) -> Result<String> {
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

pub(super) fn installation_checkpoint_sha256(
    checkpoint: &InstallationCheckpoint,
) -> Result<String> {
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

pub(in super::super) fn json_sha256<T: Serialize + ?Sized>(value: &T) -> Result<String> {
    serde_json::to_vec(value)
        .map(|bytes| digest::sha256_hex(&bytes))
        .map_err(contract_json)
}

pub(in super::super) fn canonical_prefixed(prefix: &str, sha256: &str) -> Result<CanonicalId> {
    if !is_sha256(sha256) {
        return Err(ServiceError::Contract(
            "identity seed is not SHA-256".into(),
        ));
    }
    CanonicalId::new(format!("{prefix}-{}", &sha256[..24]))
        .map_err(|error| ServiceError::Contract(error.to_string()))
}

pub(super) fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

pub(super) fn zero_digest() -> String {
    "0".repeat(64)
}

pub(super) fn storage_error(error: std::io::Error) -> ServiceError {
    ServiceError::Storage(error.to_string())
}
