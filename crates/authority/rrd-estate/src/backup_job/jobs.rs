//! Backup job requests, receipts, state and the repository operations.

use super::*;

pub const MAX_BACKUP_JOBS: usize = 4_096;
pub const MAX_BACKUP_IDEMPOTENCY_BINDINGS: usize = 4_096;
pub const MAX_BACKUP_RECEIPTS_PER_JOB: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackupJobState {
    Pending,
    Leased,
    Prepared,
    Succeeded,
    Failed,
}

impl BackupJobState {
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackupReceiptBoundary {
    Prepared,
    Completed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BackupJobReceipt {
    pub boundary: BackupReceiptBoundary,
    pub lease_epoch: u64,
    pub at: u64,
    pub evidence_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EstateBackupJob {
    pub id: CanonicalId,
    pub instance_id: CanonicalId,
    pub source_generation: u64,
    pub label: String,
    pub request_sha256: String,
    pub state: BackupJobState,
    pub attempts: u32,
    pub created_at: u64,
    pub updated_at: u64,
    pub recovery_policy: BackupRecoveryPolicySnapshot,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lease: Option<OperationLease>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub receipts: Vec<BackupJobReceipt>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backup_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archive_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub catalogue_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BackupRecoveryPolicySnapshot {
    pub revision: u64,
    pub max_rpo_ms: u64,
    pub max_rto_ms: u64,
    pub minimum_recovery_points: u16,
    pub retention_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BackupIdempotencyBinding {
    pub job_id: CanonicalId,
    pub request_sha256: String,
    pub bound_at: u64,
}

#[derive(Debug, Clone)]
pub struct ScheduleBackup {
    pub context: MutationContext,
    pub instance_id: CanonicalId,
    pub idempotency_key: String,
    pub label: String,
}

#[derive(Debug, Clone)]
pub struct BackupScheduleOutcome {
    pub document: EstateDocument,
    pub job: EstateBackupJob,
    pub idempotent_replay: bool,
}

#[derive(Debug, Clone)]
pub struct BackupLeaseRequest {
    pub context: MutationContext,
    pub worker: CanonicalId,
    pub lease_ms: u64,
}

#[derive(Debug, Clone)]
pub struct BackupPreparedRequest {
    pub context: MutationContext,
    pub worker: CanonicalId,
    pub lease_epoch: u64,
    pub evidence_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupResult {
    pub backup_id: String,
    pub archive_sha256: String,
    pub catalogue_sha256: String,
    pub evidence_sha256: String,
}

#[derive(Debug, Clone)]
pub struct BackupCompleteRequest {
    pub context: MutationContext,
    pub worker: CanonicalId,
    pub lease_epoch: u64,
    pub result: BackupResult,
}

#[derive(Debug, Clone)]
pub struct BackupFailRequest {
    pub context: MutationContext,
    pub worker: CanonicalId,
    pub lease_epoch: u64,
    pub evidence_sha256: String,
    pub error: String,
}

impl<'a, E: StorageEngine + ?Sized> EstateRepository<'a, E> {
    pub fn schedule_backup(&self, request: &ScheduleBackup) -> Result<BackupScheduleOutcome> {
        validate_context(&request.context)?;
        validate_ascii_key(&request.idempotency_key, "backup idempotency key")?;
        validate_backup_label(&request.label)?;
        let Some(current_bytes) = self.engine.control().get(&self.key)? else {
            return Err(Error::NotFound(self.estate_id.to_string()));
        };
        let mut document = super::super::decode(&current_bytes)?;

        if let Some(binding) = document.backup_idempotency.get(&request.idempotency_key) {
            let job = document
                .backup_jobs
                .get(binding.job_id.as_str())
                .cloned()
                .ok_or_else(|| Error::Invalid("backup idempotency job is missing".into()))?;
            let request_sha256 = backup_request_sha256(
                &request.instance_id,
                job.source_generation,
                &request.label,
                &job.recovery_policy,
            );
            if binding.request_sha256 != request_sha256
                || job.request_sha256 != request_sha256
                || job.id != request.context.operation_id
            {
                return Err(Error::IdempotencyConflict(request.idempotency_key.clone()));
            }
            return Ok(BackupScheduleOutcome {
                document,
                job,
                idempotent_replay: true,
            });
        }
        if document
            .backup_jobs
            .contains_key(request.context.operation_id.as_str())
        {
            return Err(Error::IdempotencyConflict(
                request.context.operation_id.to_string(),
            ));
        }
        if document.backup_jobs.len() == MAX_BACKUP_JOBS
            || document.backup_idempotency.len() == MAX_BACKUP_IDEMPOTENCY_BINDINGS
        {
            return Err(Error::Invalid(
                "estate backup job history is at its v1 bound".into(),
            ));
        }
        let source_generation = {
            let instance = document
                .instances
                .get(request.instance_id.as_str())
                .ok_or_else(|| Error::NotFound(request.instance_id.to_string()))?;
            if instance.desired.phase != super::super::DesiredPhase::Stopped
                || instance.observed.phase != ObservedPhase::Stopped
                || instance.observed.generation != instance.desired.generation
                || instance.observed.process_id.is_some()
            {
                return Err(Error::Invalid(
                    "local backup requires desired and observed stopped at the same generation"
                        .into(),
                ));
            }
            instance.desired.generation
        };
        let policy = super::super::recovery::ensure_recovery_policy(
            &mut document,
            &request.instance_id,
            request.context.at,
        )?;
        super::super::recovery::deny_active_recovery_prune(&document, &request.instance_id)?;
        let recovery_policy = BackupRecoveryPolicySnapshot {
            revision: policy.revision,
            max_rpo_ms: policy.max_rpo_ms,
            max_rto_ms: policy.max_rto_ms,
            minimum_recovery_points: policy.minimum_recovery_points,
            retention_ms: policy.retention_ms,
        };
        let request_sha256 = backup_request_sha256(
            &request.instance_id,
            source_generation,
            &request.label,
            &recovery_policy,
        );
        let job = EstateBackupJob {
            id: request.context.operation_id.clone(),
            instance_id: request.instance_id.clone(),
            source_generation,
            label: request.label.clone(),
            request_sha256: request_sha256.clone(),
            state: BackupJobState::Pending,
            attempts: 0,
            created_at: request.context.at,
            updated_at: request.context.at,
            recovery_policy,
            lease: None,
            receipts: Vec::new(),
            backup_id: None,
            archive_sha256: None,
            catalogue_sha256: None,
            error: None,
        };
        document.backup_jobs.insert(job.id.to_string(), job.clone());
        document.backup_idempotency.insert(
            request.idempotency_key.clone(),
            BackupIdempotencyBinding {
                job_id: job.id.clone(),
                request_sha256,
                bound_at: request.context.at,
            },
        );
        let document = self.commit_backup_schedule(
            current_bytes,
            document,
            &request.context,
            "estate.backup.schedule",
        )?;
        Ok(BackupScheduleOutcome {
            document,
            job,
            idempotent_replay: false,
        })
    }

    pub fn acquire_backup_lease(&self, request: &BackupLeaseRequest) -> Result<EstateDocument> {
        validate_context(&request.context)?;
        if !(1_000..=3_600_000).contains(&request.lease_ms) {
            return Err(Error::Invalid(
                "backup lease_ms must be in 1000..=3600000".into(),
            ));
        }
        self.update(&request.context, "estate.backup.lease", |document| {
            let job = backup_job_mut(document, &request.context.operation_id)?;
            if job.state.is_terminal() {
                return Err(Error::Invalid(
                    "terminal backup job cannot be leased".into(),
                ));
            }
            if job.lease.as_ref().is_some_and(|lease| {
                lease.expires_at > request.context.at && lease.owner != request.worker
            }) {
                return Err(Error::LeaseBusy(job.id.to_string()));
            }
            if job.lease.as_ref().is_some_and(|lease| {
                lease.expires_at > request.context.at && lease.owner == request.worker
            }) {
                return Ok(());
            }
            let epoch = job
                .lease
                .as_ref()
                .map(|lease| lease.epoch)
                .unwrap_or(0)
                .checked_add(1)
                .ok_or_else(|| Error::Invalid("backup lease epoch overflow".into()))?;
            let expires_at = request
                .context
                .at
                .checked_add(request.lease_ms)
                .ok_or_else(|| Error::Invalid("backup lease expiry overflow".into()))?;
            job.attempts = job
                .attempts
                .checked_add(1)
                .ok_or_else(|| Error::Invalid("backup attempts overflow".into()))?;
            if job.state == BackupJobState::Pending {
                job.state = BackupJobState::Leased;
            }
            job.updated_at = request.context.at;
            job.lease = Some(OperationLease {
                owner: request.worker.clone(),
                epoch,
                acquired_at: request.context.at,
                expires_at,
            });
            Ok(())
        })
    }

    pub fn record_backup_prepared(
        &self,
        request: &BackupPreparedRequest,
    ) -> Result<EstateDocument> {
        validate_context(&request.context)?;
        validate_sha256(&request.evidence_sha256, "backup prepared evidence")?;
        self.update(&request.context, "estate.backup.prepared", |document| {
            let job = backup_job_mut(document, &request.context.operation_id)?;
            if let Some(existing) = job.receipts.iter().find(|receipt| {
                receipt.boundary == BackupReceiptBoundary::Prepared
                    && receipt.lease_epoch == request.lease_epoch
            }) {
                if existing.evidence_sha256 == request.evidence_sha256 {
                    return Ok(());
                }
                return Err(Error::Invalid(
                    "backup prepared boundary was rebound".into(),
                ));
            }
            validate_backup_lease(
                job,
                &request.worker,
                request.lease_epoch,
                request.context.at,
            )?;
            if job.state != BackupJobState::Leased {
                return Err(Error::Invalid(
                    "backup prepare requires a leased job".into(),
                ));
            }
            push_backup_receipt(
                job,
                BackupReceiptBoundary::Prepared,
                request.lease_epoch,
                request.context.at,
                request.evidence_sha256.clone(),
            )?;
            job.state = BackupJobState::Prepared;
            job.updated_at = request.context.at;
            Ok(())
        })
    }

    pub fn record_backup_completed(
        &self,
        request: &BackupCompleteRequest,
    ) -> Result<EstateDocument> {
        validate_context(&request.context)?;
        validate_backup_result_fields(&request.result)?;
        self.update(&request.context, "estate.backup.completed", |document| {
            let instance_id = document
                .backup_jobs
                .get(request.context.operation_id.as_str())
                .map(|job| job.instance_id.clone())
                .ok_or_else(|| Error::NotFound(request.context.operation_id.to_string()))?;
            super::super::recovery::deny_active_recovery_prune(document, &instance_id)?;
            let job = backup_job_mut(document, &request.context.operation_id)?;
            if job.state == BackupJobState::Succeeded {
                if job.backup_id.as_ref() == Some(&request.result.backup_id)
                    && job.archive_sha256.as_ref() == Some(&request.result.archive_sha256)
                    && job.catalogue_sha256.as_ref() == Some(&request.result.catalogue_sha256)
                    && job.receipts.iter().any(|receipt| {
                        receipt.boundary == BackupReceiptBoundary::Completed
                            && receipt.lease_epoch == request.lease_epoch
                            && receipt.evidence_sha256 == request.result.evidence_sha256
                    })
                {
                    return Ok(());
                }
                return Err(Error::Invalid("backup completion was rebound".into()));
            }
            validate_backup_lease(
                job,
                &request.worker,
                request.lease_epoch,
                request.context.at,
            )?;
            if job.state != BackupJobState::Prepared {
                return Err(Error::Invalid(
                    "backup completion requires a prepared job".into(),
                ));
            }
            push_backup_receipt(
                job,
                BackupReceiptBoundary::Completed,
                request.lease_epoch,
                request.context.at,
                request.result.evidence_sha256.clone(),
            )?;
            job.backup_id = Some(request.result.backup_id.clone());
            job.archive_sha256 = Some(request.result.archive_sha256.clone());
            job.catalogue_sha256 = Some(request.result.catalogue_sha256.clone());
            job.state = BackupJobState::Succeeded;
            job.updated_at = request.context.at;
            job.error = None;
            let job_id = job.id.clone();
            super::super::recovery::record_completed_recovery_point(
                document,
                &job_id,
                request.context.at,
            )?;
            Ok(())
        })
    }

    pub fn record_backup_failed(&self, request: &BackupFailRequest) -> Result<EstateDocument> {
        validate_context(&request.context)?;
        validate_sha256(&request.evidence_sha256, "backup failure evidence")?;
        validate_error(Some(&request.error))?;
        self.update(&request.context, "estate.backup.failed", |document| {
            let job = backup_job_mut(document, &request.context.operation_id)?;
            validate_backup_lease(
                job,
                &request.worker,
                request.lease_epoch,
                request.context.at,
            )?;
            if !matches!(job.state, BackupJobState::Leased | BackupJobState::Prepared) {
                return Err(Error::Invalid(
                    "backup failure requires an active job".into(),
                ));
            }
            push_backup_receipt(
                job,
                BackupReceiptBoundary::Failed,
                request.lease_epoch,
                request.context.at,
                request.evidence_sha256.clone(),
            )?;
            job.state = BackupJobState::Failed;
            job.updated_at = request.context.at;
            job.error = Some(request.error.clone());
            Ok(())
        })
    }

    fn commit_backup_schedule(
        &self,
        expected: Vec<u8>,
        mut document: EstateDocument,
        context: &MutationContext,
        action: &str,
    ) -> Result<EstateDocument> {
        if context.at < document.updated_at {
            return Err(Error::Invalid(
                "mutation timestamp precedes the estate revision".into(),
            ));
        }
        document.revision = document
            .revision
            .checked_add(1)
            .ok_or_else(|| Error::Invalid("estate revision overflow".into()))?;
        document.updated_at = context.at;
        document.validate()?;
        self.engine.control().commit(&ControlTransition {
            key: self.key.clone(),
            expected: Some(expected),
            replacement: Some(encode(&document)?),
            at: context.at,
            actor: context.actor.clone(),
            action: action.into(),
            request_id: context.request_id.clone(),
            operation_id: context.operation_id.to_string(),
        })?;
        Ok(document)
    }
}
