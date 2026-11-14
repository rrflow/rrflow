//! Publication paths, backup and token-key checks, receipts, intents and filesystem helpers.

use super::*;

pub(super) fn publication_paths(
    project: &EstateFilesystem,
    locator: &ProjectLocator,
    restore_id: &CanonicalId,
) -> Result<PublicationPaths> {
    let instance = &locator.identity.instance_id;
    let restores = format!("{STORAGE_DIRECTORY}/{RESTORES_DIRECTORY_NAME}/{instance}");
    let storage_root_id = storage_root_id(locator)?;
    let candidate = format!("{restores}/{restore_id}");
    let quarantine = format!("{restores}/{restore_id}.quarantined-{storage_root_id}");
    let intent = format!("{restores}/.publish-{restore_id}.json");
    Ok(PublicationPaths {
        project: project.to_owned(),
        active: safe_join(project, &locator.storage_root)?,
        candidate: safe_join(project, &candidate)?,
        quarantine: safe_join(project, &quarantine)?,
        intent: safe_join(project, &intent)?,
        restores: instance_operation_root(project, RESTORES_DIRECTORY_NAME, instance)?,
        backups: instance_operation_root(project, BACKUPS_DIRECTORY_NAME, instance)?,
    })
}

pub(super) fn storage_root_id(locator: &ProjectLocator) -> Result<CanonicalId> {
    let name = Path::new(&locator.storage_root)
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or(ServiceError::ProjectBindingMismatch)?;
    CanonicalId::new(name).map_err(|_| ServiceError::ProjectBindingMismatch)
}

pub(super) fn relative(project: &EstateFilesystem, path: &AnchoredPath) -> Result<String> {
    path.diagnostic_path()
        .strip_prefix(project.project().diagnostic_path())
        .map_err(|_| ServiceError::ProjectBindingMismatch)?
        .components()
        .map(|component| {
            component
                .as_os_str()
                .to_str()
                .ok_or(ServiceError::ProjectBindingMismatch)
        })
        .collect::<Result<Vec<_>>>()
        .map(|segments| segments.join("/"))
}

pub(in super::super) fn require_distribution_binding(
    locator: &ProjectLocator,
    executable: &Path,
) -> Result<String> {
    let (_, profile_sha256) = DefaultInstallProfile::load(locator.profile_id.as_str())?;
    let executable_sha256 = sha256_file(executable)?;
    if executable_sha256 != locator.executable_sha256 || profile_sha256 != locator.profile_sha256 {
        return Err(ServiceError::ProjectBindingMismatch);
    }
    Ok(executable_sha256)
}

pub(super) fn require_directory(path: &AnchoredPath, label: &str) -> Result<()> {
    match path.metadata() {
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => Ok(()),
        Ok(_) => Err(ServiceError::Contract(format!(
            "{label} is not a directory"
        ))),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Err(ServiceError::Contract(format!("{label} is absent")))
        }
        Err(error) => Err(storage_error(error)),
    }
}

pub(super) fn catalogued_backup(
    backups: &AnchoredPath,
    backup_sha256: &str,
) -> Result<rrd_store::BackupEntry> {
    let catalogue = rrd_store::verify_backup_catalogue(backups)?;
    let entry = catalogue
        .backups
        .into_iter()
        .find(|entry| entry.backup_id == backup_sha256)
        .ok_or_else(|| ServiceError::Backup("backup is not catalogued".into()))?;
    if !entry.application_complete {
        return Err(ServiceError::Backup(
            "only an application-complete backup can be published".into(),
        ));
    }
    Ok(entry)
}

pub(super) fn active_token_key_sha256(
    active: &AnchoredPath,
    installed: &InstalledEstateRecord,
) -> Result<String> {
    let token_key = read_token_key(&active.join(TOKEN_KEY_FILE)).map_err(storage_error)?;
    let token_key_sha256 = digest::sha256_hex(&token_key);
    if token_key_sha256 != installed.token_key_sha256 {
        return Err(ServiceError::ProjectBindingMismatch);
    }
    Ok(token_key_sha256)
}

pub(super) fn candidate_token_key_state(
    candidate: &AnchoredPath,
    expected_sha256: &str,
) -> Result<TokenKeyState> {
    let path = candidate.join(TOKEN_KEY_FILE);
    match path.metadata() {
        Ok(_) => {
            let token_key = read_token_key(&path).map_err(storage_error)?;
            if digest::sha256_hex(&token_key) != expected_sha256 {
                return Err(ServiceError::ProjectBindingMismatch);
            }
            Ok(TokenKeyState::Present)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(TokenKeyState::Absent),
        Err(error) => Err(storage_error(error)),
    }
}

pub(super) fn publication_receipt_key(instance: &CanonicalId, plan_id: &CanonicalId) -> String {
    format!("server/state/{instance}/restore/publication/{plan_id}")
}

/// A completed publication whose intent was acknowledged leaves the candidate
/// absent, the quarantine present, and the receipt in the published root.
pub(super) fn published_receipt(
    paths: &PublicationPaths,
    locator: &ProjectLocator,
    plan: &RestorePublicationPlan,
) -> Result<Option<PublicationReceipt>> {
    if paths.candidate.metadata().is_ok() || paths.quarantine.metadata().is_err() {
        return Ok(None);
    }
    let inspector = RrflowKvInspector::open_anchored_with_options(
        &paths.active,
        super::super::core::installed_storage_options(&locator.storage)?,
    )?;
    let key = publication_receipt_key(&locator.identity.instance_id, &plan.id);
    let Some(bytes) = inspector.control_record(&key)? else {
        return Ok(None);
    };
    let receipt: PublicationReceipt = serde_json::from_slice(&bytes).map_err(contract_json)?;
    if receipt.format_version != PUBLICATION_RECEIPT_FORMAT
        || receipt.plan_sha256 != plan.plan_sha256
        || receipt.plan_id != plan.id
    {
        return Err(ServiceError::IdempotencyConflict);
    }
    Ok(Some(receipt))
}

/// A replay re-verifies the published estate the way the first application
/// did and reports its current coordinates under the recorded receipt.
pub(super) fn replay_result(
    paths: &PublicationPaths,
    plan: &RestorePublicationPlan,
    receipt: &PublicationReceipt,
    executable: &Path,
) -> Result<RestorePublicationResult> {
    let report = RrdEngine::inspect_installed_anchored(
        &paths.project,
        executable,
        &super::super::clock::HostSystemClock,
        "restore.publication.verify",
    )?;
    if report.status != InstallationVerificationStatus::Passed {
        return Err(ServiceError::Storage(
            "published estate failed quick verification".into(),
        ));
    }
    build_result(
        plan,
        receipt,
        &report.runtime_manifest_sha256,
        report.runtime_cursor,
        report.control_journal_sequence,
        true,
    )
}

pub(super) fn build_result(
    plan: &RestorePublicationPlan,
    receipt: &PublicationReceipt,
    runtime_manifest_sha256: &str,
    runtime_cursor: u64,
    control_journal_sequence: u64,
    idempotent_replay: bool,
) -> Result<RestorePublicationResult> {
    if receipt.action_outputs.len() != plan.actions.len() {
        return Err(ServiceError::Contract(
            "publication receipt does not cover every planned action".into(),
        ));
    }
    let mut result = RestorePublicationResult {
        contract_version: INSTALL_ATTUNEMENT_CONTRACT_VERSION,
        plan_id: plan.id.clone(),
        plan_sha256: plan.plan_sha256.clone(),
        action_results: plan
            .actions
            .iter()
            .zip(&receipt.action_outputs)
            .map(|(action, output_sha256)| RestorePublicationActionResult {
                kind: action.kind,
                output_sha256: output_sha256.clone(),
            })
            .collect(),
        published_root: receipt.published_root.clone(),
        quarantine_root: receipt.quarantine_root.clone(),
        locator_sha256: receipt.locator_sha256.clone(),
        runtime_manifest_sha256: runtime_manifest_sha256.into(),
        runtime_cursor,
        control_journal_sequence,
        applied_at_unix_ms: receipt.applied_at_unix_ms,
        idempotent_replay,
        result_sha256: String::new(),
    };
    result.result_sha256 = restore_publication_result_sha256(&result).map_err(contract_error)?;
    result.validate_for(plan).map_err(contract_error)?;
    Ok(result)
}

pub(super) fn advance(
    path: &AnchoredPath,
    intent: &mut PublicationIntent,
    stage: PublicationStage,
) -> Result<()> {
    intent.stage = stage;
    intent.intent_sha256 = intent.calculated_sha256()?;
    let bytes = encode(intent)?;
    let temporary = path.with_extension("json.pending");
    let _ = temporary.remove_file();
    write_private_new(&temporary, &bytes).map_err(storage_error)?;
    temporary.rename_to(path).map_err(storage_error)?;
    if read_intent(path)? != *intent {
        return Err(ServiceError::ProjectBindingMismatch);
    }
    Ok(())
}

pub(super) fn acknowledge_intent(path: &AnchoredPath, expected: &PublicationIntent) -> Result<()> {
    if read_intent(path)? != *expected {
        return Err(ServiceError::ProjectBindingMismatch);
    }
    path.remove_file().map_err(storage_error)?;
    sync_parent(path)
}

pub(super) fn read_intent(path: &AnchoredPath) -> Result<PublicationIntent> {
    let bytes = read_private_bounded(path, MAX_PUBLICATION_INTENT_BYTES).map_err(storage_error)?;
    let intent: PublicationIntent = serde_json::from_slice(&bytes).map_err(contract_json)?;
    if encode(&intent)? != bytes || intent.intent_sha256 != intent.calculated_sha256()? {
        return Err(ServiceError::ProjectBindingMismatch);
    }
    Ok(intent)
}

pub(super) fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    serde_json::to_vec(value).map_err(contract_json)
}

pub(super) fn sync_parent(path: &AnchoredPath) -> Result<()> {
    sync_directory(&path.parent().ok_or(ServiceError::ProjectBindingMismatch)?)
}

pub(super) fn sync_directory(path: &AnchoredPath) -> Result<()> {
    path.sync_directory().map_err(storage_error)
}

pub(super) fn is_lower_hex_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

pub(super) fn contract_error(error: rrd_contract::ContractError) -> ServiceError {
    ServiceError::Contract(error.to_string())
}

pub(super) fn storage_error(error: std::io::Error) -> ServiceError {
    ServiceError::Storage(error.to_string())
}
