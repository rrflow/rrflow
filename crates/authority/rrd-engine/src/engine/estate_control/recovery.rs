//! Pruning estate backups and restoring an estate from a backup.

use super::*;

impl RrdEngine {
    /// Applies one estate-authorized retention decision through the existing
    /// estate and authenticated backup catalogue authorities. The durable
    /// estate intent fences policy, hold, restore, and backup completion
    /// mutations until physical pruning is verified and recorded.
    #[allow(clippy::too_many_arguments)]
    pub fn prune_estate_backups_store(
        database: &Path,
        authority_instance: CanonicalId,
        state_root: &Path,
        distribution_executable: &Path,
        policy_path: &Path,
        key_path: &Path,
        estate_id: CanonicalId,
        instance_id: CanonicalId,
        evaluated_at: u64,
        at: u64,
        request_id: String,
        operation_id: CanonicalId,
        idempotency_key: String,
    ) -> Result<rrd_contract::EstateRecoveryPruneResult> {
        validate_local_recovery_request(at, &idempotency_key)?;
        if evaluated_at == 0 || evaluated_at > at {
            return Err(ServiceError::Contract(
                "recovery prune evaluation time is invalid".into(),
            ));
        }
        let authorization = authorize_local_recovery(
            policy_path,
            key_path,
            estate_id,
            rrd_estate::LocalEstatePermission::PruneRecovery,
            at,
        )?;
        let state_root = canonical_local_state_root(state_root)?;
        let managed = resolve_managed_instance(&state_root, &instance_id, distribution_executable)?;
        let catalogue_root = managed.catalogue_root;
        require_direct_directory(&catalogue_root, "backup catalogue")?;
        let audit_resource = estate_audit_resource(&authority_instance, &authorization.estate_id);
        let audit_request_sha256 = operation_digest(&(
            "estate-recovery-prune-audit-v1",
            &authorization.estate_id,
            &instance_id,
            evaluated_at,
            &operation_id,
            &idempotency_key,
        ))?;
        let engine = Self::open_local_authority(database, authority_instance)?;
        audit_estate_operation(
            &engine,
            at,
            authorization.operator_id.clone(),
            audit_resource,
            request_id.clone(),
            operation_id.to_string(),
            audit_request_sha256,
            || {
                let repository = rrd_estate::EstateRepository::new(
                    &engine.storage,
                    authorization.estate_id.clone(),
                );
                let operation_sha256 = operation_digest(&(
                    "estate-recovery-prune-v1",
                    &authorization.estate_id,
                    &instance_id,
                    evaluated_at,
                    &operation_id,
                ))?;
                let state_key = local_recovery_operation_key(
                    "prune",
                    &authorization.estate_id,
                    &instance_id,
                    &idempotency_key,
                );
                let existing = engine.storage.control().get(&state_key)?;
                let recovered_operation = existing.is_some();
                let (prepared_bytes, mut state) = if let Some(bytes) = existing {
                    let state: EstateRecoveryPruneOperationState =
                        serde_json::from_slice(&bytes).map_err(contract_json)?;
                    if state.format_version != ESTATE_RECOVERY_OPERATION_FORMAT
                        || state.operation_sha256 != operation_sha256
                        || state.instance_id != instance_id
                        || state.operation_id != operation_id
                    {
                        return Err(ServiceError::IdempotencyConflict);
                    }
                    if let Some(mut result) = state.result {
                        result.idempotent_replay = true;
                        return Ok(result);
                    }
                    (bytes, state)
                } else {
                    let document = repository
                        .load()?
                        .ok_or_else(|| ServiceError::Estate("estate is not initialized".into()))?;
                    let decision =
                        rrd_estate::retention_decision(&document, &instance_id, evaluated_at)?;
                    if decision.prune_candidate_backup_ids.is_empty() {
                        return Err(ServiceError::Estate(
                            "recovery retention decision has no prune candidates".into(),
                        ));
                    }
                    let catalogue = rrd_store::verify_backup_catalogue(&catalogue_root)?;
                    let mut estate_inventory = decision.retained_backup_ids.clone();
                    estate_inventory.extend(decision.prune_candidate_backup_ids.clone());
                    estate_inventory.sort();
                    let mut physical_inventory = catalogue
                        .backups
                        .iter()
                        .map(|entry| entry.backup_id.clone())
                        .collect::<Vec<_>>();
                    physical_inventory.sort();
                    if estate_inventory != physical_inventory {
                        return Err(ServiceError::Backup(
                    "estate recovery points do not exactly match the physical backup catalogue"
                        .into(),
                ));
                    }
                    let state = EstateRecoveryPruneOperationState {
                        format_version: ESTATE_RECOVERY_OPERATION_FORMAT,
                        operation_sha256: operation_sha256.clone(),
                        instance_id: instance_id.clone(),
                        operation_id: operation_id.clone(),
                        evaluated_at,
                        expected_estate_revision: decision.estate_revision,
                        expected_catalogue_sha256: catalogue.catalogue_sha256,
                        retained_backup_ids: decision.retained_backup_ids,
                        prune_candidate_backup_ids: decision.prune_candidate_backup_ids,
                        result: None,
                    };
                    let bytes = serde_json::to_vec(&state).map_err(contract_json)?;
                    commit_local_recovery_control(
                        &engine.storage,
                        state_key.clone(),
                        None,
                        &state,
                        at,
                        authorization.operator_id.as_str(),
                        "estate.recovery.prune.operation.prepared",
                        &request_id,
                        operation_id.as_str(),
                    )?;
                    (bytes, state)
                };

                let prepare_key = local_recovery_idempotency_key("prune-prepare", &idempotency_key);
                let prepared =
                    repository.prepare_recovery_prune(&rrd_estate::PrepareRecoveryPrune {
                        context: rrd_estate::MutationContext {
                            at,
                            actor: authorization.operator_id.to_string(),
                            request_id: request_id.clone(),
                            operation_id: operation_id.clone(),
                        },
                        idempotency_key: prepare_key,
                        instance_id: instance_id.clone(),
                        expected_estate_revision: state.expected_estate_revision,
                        evaluated_at: state.evaluated_at,
                        expected_catalogue_sha256: state.expected_catalogue_sha256.clone(),
                        retained_backup_ids: state.retained_backup_ids.clone(),
                        prune_candidate_backup_ids: state.prune_candidate_backup_ids.clone(),
                    })?;
                let physical = rrd_store::prune_backup_catalogue(
                    &catalogue_root,
                    &rrd_store::BackupPrunePlan {
                        expected_catalogue_sha256: state.expected_catalogue_sha256.clone(),
                        retained_backup_ids: state.retained_backup_ids.clone(),
                        prune_candidate_backup_ids: state.prune_candidate_backup_ids.clone(),
                    },
                )?;
                let complete_operation = derived_recovery_operation_id(
                    "prune-complete",
                    &authorization.estate_id,
                    &instance_id,
                    &idempotency_key,
                )?;
                let completed =
                    repository.complete_recovery_prune(&rrd_estate::CompleteRecoveryPrune {
                        context: rrd_estate::MutationContext {
                            at,
                            actor: authorization.operator_id.to_string(),
                            request_id: request_id.clone(),
                            operation_id: complete_operation,
                        },
                        idempotency_key: local_recovery_idempotency_key(
                            "prune-complete",
                            &idempotency_key,
                        ),
                        intent_id: operation_id.clone(),
                        resulting_catalogue_sha256: physical.catalogue.catalogue_sha256.clone(),
                    })?;
                let decision = rrd_contract::EstateRetentionDecisionSnapshot {
                    estate_revision: state.expected_estate_revision,
                    instance_id: instance_id.clone(),
                    evaluated_at_unix_ms: state.evaluated_at,
                    retained_backup_ids: state.retained_backup_ids.clone(),
                    prune_candidate_backup_ids: state.prune_candidate_backup_ids.clone(),
                };
                let result = rrd_contract::EstateRecoveryPruneResult {
                    recovery: rrd_estate::public_recovery_snapshot(&completed.document),
                    decision,
                    catalogue_revision: physical.catalogue.revision,
                    catalogue_sha256: physical.catalogue.catalogue_sha256,
                    pruned_backup_ids: physical.pruned_backup_ids,
                    idempotent_replay: recovered_operation
                        || prepared.idempotent_replay
                        || physical.idempotent_replay
                        || completed.idempotent_replay,
                };
                state.result = Some(result.clone());
                commit_local_recovery_control(
                    &engine.storage,
                    state_key,
                    Some(prepared_bytes),
                    &state,
                    at,
                    authorization.operator_id.as_str(),
                    "estate.recovery.prune.operation.completed",
                    &request_id,
                    operation_id.as_str(),
                )?;
                Ok(result)
            },
        )
    }

    /// Restores one estate-owned recovery point only to the absent
    /// `.rrflow/rrd/restores/<instance>/<restore-id>` staging root of the
    /// managed instance, where the offline `restore plan|apply` publication
    /// finds it; verifies the reopened closure; and records measured RPO/RTO
    /// evidence before releasing its in-flight retention hold.
    #[allow(clippy::too_many_arguments)]
    pub fn restore_estate_backup_store(
        database: &Path,
        authority_instance: CanonicalId,
        state_root: &Path,
        distribution_executable: &Path,
        policy_path: &Path,
        key_path: &Path,
        estate_id: CanonicalId,
        instance_id: CanonicalId,
        backup_sha256: String,
        restore_id: CanonicalId,
        started_at: u64,
        at: u64,
        request_id: String,
        operation_id: CanonicalId,
        idempotency_key: String,
    ) -> Result<rrd_contract::EstateRecoveryRestoreResult> {
        validate_local_recovery_request(at, &idempotency_key)?;
        if started_at == 0 || started_at > at {
            return Err(ServiceError::Contract(
                "recovery restore start time is invalid".into(),
            ));
        }
        validate_sha256_contract(&backup_sha256, "backup_sha256")?;
        let authorization = authorize_local_recovery(
            policy_path,
            key_path,
            estate_id,
            rrd_estate::LocalEstatePermission::RestoreRecovery,
            at,
        )?;
        let state_root = canonical_local_state_root(state_root)?;
        let managed = resolve_managed_instance(&state_root, &instance_id, distribution_executable)?;
        let catalogue_root = managed.catalogue_root;
        require_direct_directory(&catalogue_root, "backup catalogue")?;
        let restore_parent = managed.restore_parent;
        restore_parent
            .create_dir_all()
            .map_err(|e| ServiceError::Backup(e.to_string()))?;
        let restore_parent = restore_parent
            .pin_directory()
            .map_err(|e| ServiceError::Backup(e.to_string()))?;
        let target = restore_parent.join(restore_id.as_str());
        let audit_resource = estate_audit_resource(&authority_instance, &authorization.estate_id);
        let audit_request_sha256 = operation_digest(&(
            "estate-recovery-restore-audit-v1",
            &authorization.estate_id,
            &instance_id,
            &backup_sha256,
            &restore_id,
            started_at,
            &operation_id,
            &idempotency_key,
        ))?;
        let engine = Self::open_local_authority(database, authority_instance)?;
        audit_estate_operation(
            &engine,
            at,
            authorization.operator_id.clone(),
            audit_resource,
            request_id.clone(),
            operation_id.to_string(),
            audit_request_sha256,
            || {
                let repository = rrd_estate::EstateRepository::new(
                    &engine.storage,
                    authorization.estate_id.clone(),
                );
                let operation_sha256 = operation_digest(&(
                    "estate-recovery-restore-v1",
                    &authorization.estate_id,
                    &instance_id,
                    &backup_sha256,
                    &restore_id,
                    started_at,
                    &operation_id,
                ))?;
                let state_key = local_recovery_operation_key(
                    "restore",
                    &authorization.estate_id,
                    &instance_id,
                    &idempotency_key,
                );
                let existing = engine.storage.control().get(&state_key)?;
                let recovered_operation = existing.is_some();
                let (prepared_bytes, mut state) = if let Some(bytes) = existing {
                    let state: EstateRecoveryRestoreOperationState =
                        serde_json::from_slice(&bytes).map_err(contract_json)?;
                    if state.format_version != ESTATE_RECOVERY_OPERATION_FORMAT
                        || state.operation_sha256 != operation_sha256
                        || state.instance_id != instance_id
                        || state.operation_id != operation_id
                        || state.backup_sha256 != backup_sha256
                        || state.restore_id != restore_id
                    {
                        return Err(ServiceError::IdempotencyConflict);
                    }
                    if let Some(mut result) = state.result {
                        result.idempotent_replay = true;
                        return Ok(result);
                    }
                    (bytes, state)
                } else {
                    let document = repository
                        .load()?
                        .ok_or_else(|| ServiceError::Estate("estate is not initialized".into()))?;
                    let point = document
                        .recovery_points
                        .get(&backup_sha256)
                        .ok_or_else(|| ServiceError::Estate("recovery point is unknown".into()))?;
                    if point.instance_id != instance_id || point.pruned_at.is_some() {
                        return Err(ServiceError::Estate(
                            "recovery point is unavailable for this instance".into(),
                        ));
                    }
                    let catalogue = rrd_store::verify_backup_catalogue(&catalogue_root)?;
                    let entry = catalogue
                        .backups
                        .iter()
                        .find(|entry| entry.backup_id == backup_sha256)
                        .ok_or_else(|| {
                            ServiceError::Backup("recovery point is not catalogued".into())
                        })?;
                    if entry.archive.archive_sha256 != point.archive_sha256 {
                        return Err(ServiceError::Backup(
                            "recovery point archive identity diverges from the catalogue".into(),
                        ));
                    }
                    let state = EstateRecoveryRestoreOperationState {
                        format_version: ESTATE_RECOVERY_OPERATION_FORMAT,
                        operation_sha256: operation_sha256.clone(),
                        instance_id: instance_id.clone(),
                        operation_id: operation_id.clone(),
                        backup_sha256: backup_sha256.clone(),
                        restore_id: restore_id.clone(),
                        started_at,
                        result: None,
                    };
                    let bytes = serde_json::to_vec(&state).map_err(contract_json)?;
                    commit_local_recovery_control(
                        &engine.storage,
                        state_key.clone(),
                        None,
                        &state,
                        at,
                        authorization.operator_id.as_str(),
                        "estate.recovery.restore.operation.prepared",
                        &request_id,
                        operation_id.as_str(),
                    )?;
                    (bytes, state)
                };

                let hold_operation = derived_recovery_operation_id(
                    "restore-hold",
                    &authorization.estate_id,
                    &instance_id,
                    &idempotency_key,
                )?;
                let held = repository.pin_recovery_point(&rrd_estate::PinRecoveryPoint {
                    context: rrd_estate::MutationContext {
                        at,
                        actor: authorization.operator_id.to_string(),
                        request_id: request_id.clone(),
                        operation_id: hold_operation.clone(),
                    },
                    idempotency_key: local_recovery_idempotency_key(
                        "restore-hold",
                        &idempotency_key,
                    ),
                    backup_id: backup_sha256.clone(),
                    expires_at: None,
                })?;
                let catalogue = rrd_store::verify_backup_catalogue(&catalogue_root)?;
                let entry = catalogue
                    .backups
                    .iter()
                    .find(|entry| entry.backup_id == backup_sha256)
                    .ok_or_else(|| {
                        ServiceError::Backup("recovery point is not catalogued".into())
                    })?;
                let (inventory, reopened, physical_replay) = if target
                    .try_exists()
                    .map_err(|e| ServiceError::Backup(e.to_string()))?
                {
                    let metadata = target.metadata().map_err(|error| {
                        ServiceError::Backup(format!("cannot inspect restore target: {error}"))
                    })?;
                    if metadata.file_type().is_symlink() || !metadata.is_dir() {
                        return Err(ServiceError::Backup(
                            "existing restore target is not a direct canonical directory".into(),
                        ));
                    }
                    let restored =
                        RrflowKvStore::open_existing_anchored(&target, Default::default())?;
                    if restored.claims().sequence()? != entry.archive.claim_sequence
                        || restored.runtime().cursor()? != entry.archive.runtime_cursor
                    {
                        return Err(ServiceError::Backup(
                            "existing restore target watermarks differ from the recovery point"
                                .into(),
                        ));
                    }
                    drop(restored);
                    rrd_store::verify_restored_backup_objects(
                        &catalogue_root,
                        &backup_sha256,
                        &target,
                    )?;
                    (entry.archive.clone(), true, true)
                } else {
                    let report = rrd_store::restore_catalogued_backup(
                        &catalogue_root,
                        &backup_sha256,
                        &target,
                        at,
                    )?;
                    let restored =
                        RrflowKvStore::open_existing_anchored(&target, Default::default())?;
                    if restored.claims().sequence()? != report.inventory.claim_sequence
                        || restored.runtime().cursor()? != report.inventory.runtime_cursor
                    {
                        return Err(ServiceError::Backup(
                            "published restore watermarks failed verification".into(),
                        ));
                    }
                    drop(restored);
                    rrd_store::verify_restored_backup_objects(
                        &catalogue_root,
                        &backup_sha256,
                        &target,
                    )?;
                    (report.inventory, report.reopened, false)
                };
                let closure_sha256 = digest::sha256_hex(
                    &serde_json::to_vec(&("estate-recovery-closure-v1", entry, &inventory))
                        .map_err(contract_json)?,
                );
                let evidence =
                    repository.record_restore_evidence(&rrd_estate::RecordRestoreEvidence {
                        context: rrd_estate::MutationContext {
                            at,
                            actor: authorization.operator_id.to_string(),
                            request_id: request_id.clone(),
                            operation_id: operation_id.clone(),
                        },
                        idempotency_key: local_recovery_idempotency_key(
                            "restore-evidence",
                            &idempotency_key,
                        ),
                        restore_id: restore_id.clone(),
                        instance_id: instance_id.clone(),
                        backup_id: backup_sha256.clone(),
                        started_at: state.started_at,
                        completed_at: at,
                        restored_claim_sequence: inventory.claim_sequence,
                        restored_runtime_cursor: inventory.runtime_cursor,
                        closure_sha256,
                    })?;
                let release_operation = derived_recovery_operation_id(
                    "restore-release",
                    &authorization.estate_id,
                    &instance_id,
                    &idempotency_key,
                )?;
                let released =
                    repository.release_recovery_pin(&rrd_estate::ReleaseRecoveryPin {
                        context: rrd_estate::MutationContext {
                            at,
                            actor: authorization.operator_id.to_string(),
                            request_id: request_id.clone(),
                            operation_id: release_operation,
                        },
                        idempotency_key: local_recovery_idempotency_key(
                            "restore-release",
                            &idempotency_key,
                        ),
                        pin_id: hold_operation,
                    })?;
                let result = rrd_contract::EstateRecoveryRestoreResult {
                    recovery: rrd_estate::public_recovery_snapshot(&released.document),
                    restore_id: restore_id.clone(),
                    backup_sha256: backup_sha256.clone(),
                    inventory: super::super::backup::public_archive(&inventory),
                    reopened,
                    idempotent_replay: recovered_operation
                        || held.idempotent_replay
                        || physical_replay
                        || evidence.idempotent_replay
                        || released.idempotent_replay,
                };
                state.result = Some(result.clone());
                commit_local_recovery_control(
                    &engine.storage,
                    state_key,
                    Some(prepared_bytes),
                    &state,
                    at,
                    authorization.operator_id.as_str(),
                    "estate.recovery.restore.operation.completed",
                    &request_id,
                    operation_id.as_str(),
                )?;
                Ok(result)
            },
        )
    }
}
