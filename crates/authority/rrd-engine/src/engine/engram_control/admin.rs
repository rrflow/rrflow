//! Administering and reconciling an engram's store and backup store.

use super::*;

impl RrdEngine {
    /// Authorizes and applies one engram mutation through a single local RRD
    /// authority. Policy denial happens before the database can be created.
    #[allow(clippy::too_many_arguments)]
    pub fn administer_engram_store(
        database: &Path,
        authority_instance: CanonicalId,
        policy_path: &Path,
        key_path: &Path,
        engram_id: CanonicalId,
        at: u64,
        request_id: String,
        operation_id: CanonicalId,
        action: EngramAdminAction,
    ) -> Result<EngramAdminResult> {
        let permission = match &action {
            EngramAdminAction::Create => rrd_engram::LocalEngramPermission::Create,
            EngramAdminAction::SetDesired { .. } => rrd_engram::LocalEngramPermission::SetDesired,
            EngramAdminAction::ScheduleBackup { .. } => {
                rrd_engram::LocalEngramPermission::ScheduleBackup
            }
            EngramAdminAction::SetRecoveryPolicy { .. } => {
                rrd_engram::LocalEngramPermission::ManageRecoveryPolicy
            }
            EngramAdminAction::PinRecoveryPoint { .. }
            | EngramAdminAction::ReleaseRecoveryPin { .. } => {
                rrd_engram::LocalEngramPermission::ManageRecoveryHolds
            }
        };
        let policy = rrd_engram::LocalOperatorPolicy::load_json(policy_path)
            .map_err(ServiceError::Contract)?;
        let authorization = policy
            .authorize_key_file(key_path, engram_id, permission, at)
            .map_err(ServiceError::Contract)?;
        let audit_request_id = request_id.clone();
        let audit_operation_id = operation_id.to_string();
        let audit_request_sha256 =
            digest::sha256_hex(&serde_json::to_vec(&action).map_err(contract_json)?);
        let audit_resource = engram_audit_resource(&authority_instance, &authorization.engram_id);
        let audit_principal = authorization.operator_id.clone();
        let engine = Self::open_local_authority(database, authority_instance)?;
        let repository =
            rrd_engram::EngramRepository::new(&engine.storage, authorization.engram_id);
        engine.append_audit(AuditEvent {
            at_unix_ms: at,
            attempt: 1,
            principal_id: Some(audit_principal.clone()),
            action: SecurityAction::EngramAdmin,
            resource: audit_resource.clone(),
            request_id: audit_request_id.clone(),
            operation_id: audit_operation_id.clone(),
            phase: AuditPhase::Authorized,
            decision: AuditDecision::Allowed,
            status_code: 100,
            request_sha256: audit_request_sha256.clone(),
            response_sha256: digest::sha256_hex(b"rrd-audit-completion-pending"),
        })?;
        let context = rrd_engram::MutationContext {
            at,
            actor: authorization.operator_id.to_string(),
            request_id,
            operation_id,
        };
        let result = (|| match action {
            EngramAdminAction::Create => {
                let outcome = repository.create_idempotent(&context)?;
                Ok(EngramAdminResult::Mutation(Box::new(
                    rrd_contract::EngramMutationResult {
                        engram: rrd_engram::public_snapshot(&outcome.document),
                        idempotent_replay: outcome.idempotent_replay,
                    },
                )))
            }
            EngramAdminAction::SetDesired {
                instance,
                idempotency_key,
                phase,
                deployment,
                version,
                configuration_sha256,
            } => {
                let phase = match phase {
                    rrd_contract::EngramDesiredPhase::Running => rrd_engram::DesiredPhase::Running,
                    rrd_contract::EngramDesiredPhase::Stopped => rrd_engram::DesiredPhase::Stopped,
                    rrd_contract::EngramDesiredPhase::Absent => rrd_engram::DesiredPhase::Absent,
                };
                let outcome = repository.set_desired(&rrd_engram::SetDesired {
                    context,
                    instance_id: instance,
                    idempotency_key,
                    target: rrd_engram::DesiredTarget {
                        phase,
                        deployment_ref: deployment,
                        version,
                        configuration_sha256,
                    },
                })?;
                Ok(EngramAdminResult::Mutation(Box::new(
                    rrd_contract::EngramMutationResult {
                        engram: rrd_engram::public_snapshot(&outcome.document),
                        idempotent_replay: outcome.idempotent_replay,
                    },
                )))
            }
            EngramAdminAction::ScheduleBackup {
                instance,
                idempotency_key,
                label,
            } => {
                let outcome = repository.schedule_backup(&rrd_engram::ScheduleBackup {
                    context,
                    instance_id: instance,
                    idempotency_key,
                    label,
                })?;
                Ok(EngramAdminResult::Backup(Box::new(
                    rrd_contract::EngramBackupMutationResult {
                        engram: rrd_engram::public_snapshot(&outcome.document),
                        job: rrd_engram::public_backup_job(&outcome.job),
                        idempotent_replay: outcome.idempotent_replay,
                    },
                )))
            }
            EngramAdminAction::SetRecoveryPolicy {
                instance,
                idempotency_key,
                max_rpo_ms,
                max_rto_ms,
                minimum_recovery_points,
                retention_ms,
            } => {
                let outcome = repository.set_recovery_policy(&rrd_engram::SetRecoveryPolicy {
                    context,
                    instance_id: instance,
                    idempotency_key,
                    max_rpo_ms,
                    max_rto_ms,
                    minimum_recovery_points,
                    retention_ms,
                })?;
                Ok(EngramAdminResult::Recovery(Box::new(
                    rrd_contract::EngramRecoveryMutationResult {
                        recovery: rrd_engram::public_recovery_snapshot(&outcome.document),
                        idempotent_replay: outcome.idempotent_replay,
                    },
                )))
            }
            EngramAdminAction::PinRecoveryPoint {
                idempotency_key,
                backup_sha256,
                expires_at_unix_ms,
            } => {
                let outcome = repository.pin_recovery_point(&rrd_engram::PinRecoveryPoint {
                    context,
                    idempotency_key,
                    backup_id: backup_sha256,
                    expires_at: expires_at_unix_ms,
                })?;
                Ok(EngramAdminResult::Recovery(Box::new(
                    rrd_contract::EngramRecoveryMutationResult {
                        recovery: rrd_engram::public_recovery_snapshot(&outcome.document),
                        idempotent_replay: outcome.idempotent_replay,
                    },
                )))
            }
            EngramAdminAction::ReleaseRecoveryPin {
                idempotency_key,
                pin_id,
            } => {
                let outcome = repository.release_recovery_pin(&rrd_engram::ReleaseRecoveryPin {
                    context,
                    idempotency_key,
                    pin_id,
                })?;
                Ok(EngramAdminResult::Recovery(Box::new(
                    rrd_contract::EngramRecoveryMutationResult {
                        recovery: rrd_engram::public_recovery_snapshot(&outcome.document),
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
            action: SecurityAction::EngramAdmin,
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

    /// Advances one engram reconciliation boundary. The outward executable
    /// supplies coordinates only; catalog, driver, repository, and store
    /// construction remain inside `RrdEngine`.
    #[allow(clippy::too_many_arguments)]
    pub fn reconcile_engram_store(
        database: &Path,
        authority_instance: CanonicalId,
        state_root: &Path,
        catalog_path: &Path,
        engram_id: CanonicalId,
        worker: CanonicalId,
        lease_ms: u64,
        at: u64,
        hold_after_effect: Option<&Path>,
    ) -> Result<EngramReconcileOutcome> {
        let audit_resource = engram_audit_resource(&authority_instance, &engram_id);
        let audit_request_id = format!("engram-reconcile-request-{engram_id}-{at}");
        let audit_operation_id = format!("engram-reconcile-{engram_id}-{worker}-{at}");
        let audit_request_sha256 = operation_digest(&(
            "engram-reconcile-v1",
            &engram_id,
            &worker,
            lease_ms,
            at,
            state_root.to_string_lossy(),
            catalog_path.to_string_lossy(),
        ))?;
        let engine = Self::open_local_authority(database, authority_instance)?;
        audit_engram_operation(
            &engine,
            at,
            worker.clone(),
            audit_resource,
            audit_request_id,
            audit_operation_id,
            audit_request_sha256,
            || {
                let catalog = rrd_engram::LocalDeploymentCatalog::load_json(catalog_path)
                    .map_err(ServiceError::Contract)?;
                let driver = rrd_engram::LocalProcessDriver::new(state_root, catalog)
                    .map_err(ServiceError::Contract)?;
                let mut reconciler = rrd_engram::Reconciler::new(
                    &engine.storage,
                    engram_id,
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

    /// Advances one engram backup boundary with an engine-owned local backup
    /// driver. The driver opens a managed instance only as another installed
    /// `RrdEngine`, bound to the engram's distribution executable; no physical
    /// store handle escapes the composition root.
    #[allow(clippy::too_many_arguments)]
    pub fn reconcile_engram_backup_store(
        database: &Path,
        authority_instance: CanonicalId,
        state_root: &Path,
        distribution_executable: &Path,
        engram_id: CanonicalId,
        worker: CanonicalId,
        lease_ms: u64,
        at: u64,
        hold_after_effect: Option<&Path>,
    ) -> Result<EngramBackupReconcileOutcome> {
        let audit_resource = engram_audit_resource(&authority_instance, &engram_id);
        let audit_request_id = format!("engram-backup-reconcile-request-{engram_id}-{at}");
        let audit_operation_id = format!("engram-backup-reconcile-{engram_id}-{worker}-{at}");
        let audit_request_sha256 = operation_digest(&(
            "engram-backup-reconcile-v1",
            &engram_id,
            &worker,
            lease_ms,
            at,
            state_root.to_string_lossy(),
            distribution_executable.to_string_lossy(),
        ))?;
        let engine = Self::open_local_authority(database, authority_instance)?;
        audit_engram_operation(
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
                let mut reconciler = rrd_engram::BackupReconciler::new(
                    &engine.storage,
                    engram_id,
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
