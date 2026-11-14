//! Administering and reconciling an estate's store and backup store.

use super::*;

impl RrdEngine {
    /// Authorizes and applies one estate mutation through a single local RRD
    /// authority. Policy denial happens before the database can be created.
    #[allow(clippy::too_many_arguments)]
    pub fn administer_estate_store(
        database: &Path,
        authority_instance: CanonicalId,
        policy_path: &Path,
        key_path: &Path,
        estate_id: CanonicalId,
        at: u64,
        request_id: String,
        operation_id: CanonicalId,
        action: EstateAdminAction,
    ) -> Result<EstateAdminResult> {
        let permission = match &action {
            EstateAdminAction::Create => rrd_estate::LocalEstatePermission::Create,
            EstateAdminAction::SetDesired { .. } => rrd_estate::LocalEstatePermission::SetDesired,
            EstateAdminAction::ScheduleBackup { .. } => {
                rrd_estate::LocalEstatePermission::ScheduleBackup
            }
            EstateAdminAction::SetRecoveryPolicy { .. } => {
                rrd_estate::LocalEstatePermission::ManageRecoveryPolicy
            }
            EstateAdminAction::PinRecoveryPoint { .. }
            | EstateAdminAction::ReleaseRecoveryPin { .. } => {
                rrd_estate::LocalEstatePermission::ManageRecoveryHolds
            }
        };
        let policy = rrd_estate::LocalOperatorPolicy::load_json(policy_path)
            .map_err(ServiceError::Contract)?;
        let authorization = policy
            .authorize_key_file(key_path, estate_id, permission, at)
            .map_err(ServiceError::Contract)?;
        let audit_request_id = request_id.clone();
        let audit_operation_id = operation_id.to_string();
        let audit_request_sha256 =
            digest::sha256_hex(&serde_json::to_vec(&action).map_err(contract_json)?);
        let audit_resource = estate_audit_resource(&authority_instance, &authorization.estate_id);
        let audit_principal = authorization.operator_id.clone();
        let engine = Self::open_local_authority(database, authority_instance)?;
        let repository =
            rrd_estate::EstateRepository::new(&engine.storage, authorization.estate_id);
        engine.append_audit(AuditEvent {
            at_unix_ms: at,
            attempt: 1,
            principal_id: Some(audit_principal.clone()),
            action: SecurityAction::EstateAdmin,
            resource: audit_resource.clone(),
            request_id: audit_request_id.clone(),
            operation_id: audit_operation_id.clone(),
            phase: AuditPhase::Authorized,
            decision: AuditDecision::Allowed,
            status_code: 100,
            request_sha256: audit_request_sha256.clone(),
            response_sha256: digest::sha256_hex(b"rrd-audit-completion-pending"),
        })?;
        let context = rrd_estate::MutationContext {
            at,
            actor: authorization.operator_id.to_string(),
            request_id,
            operation_id,
        };
        let result = (|| match action {
            EstateAdminAction::Create => {
                let outcome = repository.create_idempotent(&context)?;
                Ok(EstateAdminResult::Mutation(Box::new(
                    rrd_contract::EstateMutationResult {
                        estate: rrd_estate::public_snapshot(&outcome.document),
                        idempotent_replay: outcome.idempotent_replay,
                    },
                )))
            }
            EstateAdminAction::SetDesired {
                instance,
                idempotency_key,
                phase,
                deployment,
                version,
                configuration_sha256,
            } => {
                let phase = match phase {
                    rrd_contract::EstateDesiredPhase::Running => rrd_estate::DesiredPhase::Running,
                    rrd_contract::EstateDesiredPhase::Stopped => rrd_estate::DesiredPhase::Stopped,
                    rrd_contract::EstateDesiredPhase::Absent => rrd_estate::DesiredPhase::Absent,
                };
                let outcome = repository.set_desired(&rrd_estate::SetDesired {
                    context,
                    instance_id: instance,
                    idempotency_key,
                    target: rrd_estate::DesiredTarget {
                        phase,
                        deployment_ref: deployment,
                        version,
                        configuration_sha256,
                    },
                })?;
                Ok(EstateAdminResult::Mutation(Box::new(
                    rrd_contract::EstateMutationResult {
                        estate: rrd_estate::public_snapshot(&outcome.document),
                        idempotent_replay: outcome.idempotent_replay,
                    },
                )))
            }
            EstateAdminAction::ScheduleBackup {
                instance,
                idempotency_key,
                label,
            } => {
                let outcome = repository.schedule_backup(&rrd_estate::ScheduleBackup {
                    context,
                    instance_id: instance,
                    idempotency_key,
                    label,
                })?;
                Ok(EstateAdminResult::Backup(Box::new(
                    rrd_contract::EstateBackupMutationResult {
                        estate: rrd_estate::public_snapshot(&outcome.document),
                        job: rrd_estate::public_backup_job(&outcome.job),
                        idempotent_replay: outcome.idempotent_replay,
                    },
                )))
            }
            EstateAdminAction::SetRecoveryPolicy {
                instance,
                idempotency_key,
                max_rpo_ms,
                max_rto_ms,
                minimum_recovery_points,
                retention_ms,
            } => {
                let outcome = repository.set_recovery_policy(&rrd_estate::SetRecoveryPolicy {
                    context,
                    instance_id: instance,
                    idempotency_key,
                    max_rpo_ms,
                    max_rto_ms,
                    minimum_recovery_points,
                    retention_ms,
                })?;
                Ok(EstateAdminResult::Recovery(Box::new(
                    rrd_contract::EstateRecoveryMutationResult {
                        recovery: rrd_estate::public_recovery_snapshot(&outcome.document),
                        idempotent_replay: outcome.idempotent_replay,
                    },
                )))
            }
            EstateAdminAction::PinRecoveryPoint {
                idempotency_key,
                backup_sha256,
                expires_at_unix_ms,
            } => {
                let outcome = repository.pin_recovery_point(&rrd_estate::PinRecoveryPoint {
                    context,
                    idempotency_key,
                    backup_id: backup_sha256,
                    expires_at: expires_at_unix_ms,
                })?;
                Ok(EstateAdminResult::Recovery(Box::new(
                    rrd_contract::EstateRecoveryMutationResult {
                        recovery: rrd_estate::public_recovery_snapshot(&outcome.document),
                        idempotent_replay: outcome.idempotent_replay,
                    },
                )))
            }
            EstateAdminAction::ReleaseRecoveryPin {
                idempotency_key,
                pin_id,
            } => {
                let outcome = repository.release_recovery_pin(&rrd_estate::ReleaseRecoveryPin {
                    context,
                    idempotency_key,
                    pin_id,
                })?;
                Ok(EstateAdminResult::Recovery(Box::new(
                    rrd_contract::EstateRecoveryMutationResult {
                        recovery: rrd_estate::public_recovery_snapshot(&outcome.document),
                        idempotent_replay: outcome.idempotent_replay,
                    },
                )))
            }
        })();
        let (decision, status_code, response_sha256) = match &result {
            Ok(response) => (
                AuditDecision::Allowed,
                200,
                digest::sha256_hex(&serde_json::to_vec(response).map_err(contract_json)?),
            ),
            Err(error) => {
                let (decision, status_code) = super::super::invocation::audit_failure(error);
                (
                    decision,
                    status_code,
                    digest::sha256_hex(error.to_string().as_bytes()),
                )
            }
        };
        engine.append_audit(AuditEvent {
            at_unix_ms: at,
            attempt: 1,
            principal_id: Some(audit_principal),
            action: SecurityAction::EstateAdmin,
            resource: audit_resource,
            request_id: audit_request_id,
            operation_id: audit_operation_id,
            phase: AuditPhase::Completed,
            decision,
            status_code,
            request_sha256: audit_request_sha256,
            response_sha256,
        })?;
        result
    }

    /// Advances one estate reconciliation boundary. The outward executable
    /// supplies coordinates only; catalog, driver, repository, and store
    /// construction remain inside `RrdEngine`.
    #[allow(clippy::too_many_arguments)]
    pub fn reconcile_estate_store(
        database: &Path,
        authority_instance: CanonicalId,
        state_root: &Path,
        catalog_path: &Path,
        estate_id: CanonicalId,
        worker: CanonicalId,
        lease_ms: u64,
        at: u64,
        hold_after_effect: Option<&Path>,
    ) -> Result<EstateReconcileOutcome> {
        let audit_resource = estate_audit_resource(&authority_instance, &estate_id);
        let audit_request_id = format!("estate-reconcile-request-{estate_id}-{at}");
        let audit_operation_id = format!("estate-reconcile-{estate_id}-{worker}-{at}");
        let audit_request_sha256 = operation_digest(&(
            "estate-reconcile-v1",
            &estate_id,
            &worker,
            lease_ms,
            at,
            state_root.to_string_lossy(),
            catalog_path.to_string_lossy(),
        ))?;
        let engine = Self::open_local_authority(database, authority_instance)?;
        audit_estate_operation(
            &engine,
            at,
            worker.clone(),
            audit_resource,
            audit_request_id,
            audit_operation_id,
            audit_request_sha256,
            || {
                let catalog = rrd_estate::LocalDeploymentCatalog::load_json(catalog_path)
                    .map_err(ServiceError::Contract)?;
                let driver = rrd_estate::LocalProcessDriver::new(state_root, catalog)
                    .map_err(ServiceError::Contract)?;
                let mut reconciler = rrd_estate::Reconciler::new(
                    &engine.storage,
                    estate_id,
                    worker.clone(),
                    lease_ms,
                    EffectHoldDriver {
                        inner: driver,
                        marker: hold_after_effect.map(Path::to_path_buf),
                    },
                )?;
                reconciler.step(at).map_err(Into::into)
            },
        )
    }

    /// Advances one estate backup boundary with an engine-owned local backup
    /// driver. The driver opens a managed instance only as another installed
    /// `RrdEngine`, bound to the estate's distribution executable; no physical
    /// store handle escapes the composition root.
    #[allow(clippy::too_many_arguments)]
    pub fn reconcile_estate_backup_store(
        database: &Path,
        authority_instance: CanonicalId,
        state_root: &Path,
        distribution_executable: &Path,
        estate_id: CanonicalId,
        worker: CanonicalId,
        lease_ms: u64,
        at: u64,
        hold_after_effect: Option<&Path>,
    ) -> Result<EstateBackupReconcileOutcome> {
        let audit_resource = estate_audit_resource(&authority_instance, &estate_id);
        let audit_request_id = format!("estate-backup-reconcile-request-{estate_id}-{at}");
        let audit_operation_id = format!("estate-backup-reconcile-{estate_id}-{worker}-{at}");
        let audit_request_sha256 = operation_digest(&(
            "estate-backup-reconcile-v1",
            &estate_id,
            &worker,
            lease_ms,
            at,
            state_root.to_string_lossy(),
            distribution_executable.to_string_lossy(),
        ))?;
        let engine = Self::open_local_authority(database, authority_instance)?;
        audit_estate_operation(
            &engine,
            at,
            worker.clone(),
            audit_resource,
            audit_request_id,
            audit_operation_id,
            audit_request_sha256,
            || {
                let driver = EngineLocalBackupDriver::new(state_root, distribution_executable)
                    .map_err(ServiceError::Contract)?;
                let mut reconciler = rrd_estate::BackupReconciler::new(
                    &engine.storage,
                    estate_id,
                    worker.clone(),
                    lease_ms,
                    BackupEffectHoldDriver {
                        inner: driver,
                        marker: hold_after_effect.map(Path::to_path_buf),
                    },
                )?;
                reconciler.step(at).map_err(Into::into)
            },
        )
    }
}
