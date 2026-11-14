//! Public backup jobs and backup state and result validation.

use super::*;

pub fn public_backup_jobs(document: &EstateDocument) -> rrd_contract::EstateBackupJobsSnapshot {
    rrd_contract::EstateBackupJobsSnapshot {
        estate_id: document.id.clone(),
        estate_revision: document.revision,
        jobs: document
            .backup_jobs
            .values()
            .map(public_backup_job)
            .collect(),
    }
}

pub fn public_backup_job(job: &EstateBackupJob) -> rrd_contract::EstateBackupJobSnapshot {
    rrd_contract::EstateBackupJobSnapshot {
        id: job.id.clone(),
        instance_id: job.instance_id.clone(),
        source_generation: job.source_generation,
        label: job.label.clone(),
        request_sha256: job.request_sha256.clone(),
        state: match job.state {
            BackupJobState::Pending => rrd_contract::EstateBackupJobState::Pending,
            BackupJobState::Leased => rrd_contract::EstateBackupJobState::Leased,
            BackupJobState::Prepared => rrd_contract::EstateBackupJobState::Prepared,
            BackupJobState::Succeeded => rrd_contract::EstateBackupJobState::Succeeded,
            BackupJobState::Failed => rrd_contract::EstateBackupJobState::Failed,
        },
        attempts: job.attempts,
        created_at_unix_ms: job.created_at,
        updated_at_unix_ms: job.updated_at,
        lease: job
            .lease
            .as_ref()
            .map(|lease| rrd_contract::EstateLeaseSnapshot {
                owner: lease.owner.clone(),
                epoch: lease.epoch,
                acquired_at_unix_ms: lease.acquired_at,
                expires_at_unix_ms: lease.expires_at,
            }),
        receipts: job
            .receipts
            .iter()
            .map(|receipt| rrd_contract::EstateBackupReceiptSnapshot {
                boundary: match receipt.boundary {
                    BackupReceiptBoundary::Prepared => {
                        rrd_contract::EstateBackupReceiptBoundary::Prepared
                    }
                    BackupReceiptBoundary::Completed => {
                        rrd_contract::EstateBackupReceiptBoundary::Completed
                    }
                    BackupReceiptBoundary::Failed => {
                        rrd_contract::EstateBackupReceiptBoundary::Failed
                    }
                },
                lease_epoch: receipt.lease_epoch,
                at_unix_ms: receipt.at,
                evidence_sha256: receipt.evidence_sha256.clone(),
            })
            .collect(),
        backup_id: job.backup_id.clone(),
        archive_sha256: job.archive_sha256.clone(),
        catalogue_sha256: job.catalogue_sha256.clone(),
        error: job.error.clone(),
        recovery_policy: rrd_contract::EstateBackupRecoveryPolicySnapshot {
            revision: job.recovery_policy.revision,
            max_rpo_ms: job.recovery_policy.max_rpo_ms,
            max_rto_ms: job.recovery_policy.max_rto_ms,
            minimum_recovery_points: job.recovery_policy.minimum_recovery_points,
            retention_ms: job.recovery_policy.retention_ms,
        },
    }
}

pub(crate) fn validate_backup_state(document: &EstateDocument) -> Result<()> {
    if document.backup_jobs.len() > MAX_BACKUP_JOBS
        || document.backup_idempotency.len() > MAX_BACKUP_IDEMPOTENCY_BINDINGS
    {
        return Err(Error::Invalid(
            "estate backup job cardinality exceeded".into(),
        ));
    }
    for (key, job) in &document.backup_jobs {
        validate_id_key(key, &job.id)?;
        validate_backup_label(&job.label)?;
        validate_sha256(&job.request_sha256, "backup request")?;
        if job.source_generation == 0
            || job.created_at == 0
            || job.updated_at < job.created_at
            || !document.instances.contains_key(job.instance_id.as_str())
        {
            return Err(Error::Invalid(format!("backup job {} is invalid", job.id)));
        }
        let policy = &job.recovery_policy;
        if policy.revision == 0
            || policy.max_rpo_ms == 0
            || policy.max_rto_ms == 0
            || policy.minimum_recovery_points == 0
            || policy.retention_ms < policy.max_rpo_ms
        {
            return Err(Error::Invalid(format!(
                "backup job {} recovery policy is invalid",
                job.id
            )));
        }
        if job.receipts.len() > MAX_BACKUP_RECEIPTS_PER_JOB {
            return Err(Error::Invalid("backup receipt limit exceeded".into()));
        }
        if let Some(lease) = &job.lease {
            if lease.epoch == 0
                || lease.acquired_at == 0
                || lease.expires_at <= lease.acquired_at
                || lease.acquired_at < job.created_at
            {
                return Err(Error::Invalid("backup lease is invalid".into()));
            }
        }
        for receipt in &job.receipts {
            if receipt.lease_epoch == 0 || receipt.at == 0 {
                return Err(Error::Invalid("backup receipt is invalid".into()));
            }
            validate_sha256(&receipt.evidence_sha256, "backup receipt")?;
        }
        validate_error(job.error.as_deref())?;
        validate_backup_result(job)?;
    }
    for (key, binding) in &document.backup_idempotency {
        validate_ascii_key(key, "backup idempotency key")?;
        validate_sha256(&binding.request_sha256, "backup idempotency request")?;
        if binding.bound_at == 0 {
            return Err(Error::Invalid(
                "backup idempotency binding time is invalid".into(),
            ));
        }
        let job = document
            .backup_jobs
            .get(binding.job_id.as_str())
            .ok_or_else(|| Error::Invalid(format!("backup idempotency key {key} is orphaned")))?;
        if job.request_sha256 != binding.request_sha256 {
            return Err(Error::Invalid(format!(
                "backup idempotency key {key} request digest diverged"
            )));
        }
    }
    Ok(())
}

pub(super) fn validate_backup_result(job: &EstateBackupJob) -> Result<()> {
    let result_count = [
        job.backup_id.as_deref(),
        job.archive_sha256.as_deref(),
        job.catalogue_sha256.as_deref(),
    ]
    .iter()
    .filter(|value| value.is_some())
    .count();
    if result_count != 0 && result_count != 3 {
        return Err(Error::Invalid(
            "backup result identity is only partially populated".into(),
        ));
    }
    for (name, value) in [
        ("backup identity", job.backup_id.as_deref()),
        ("backup archive", job.archive_sha256.as_deref()),
        ("backup catalogue", job.catalogue_sha256.as_deref()),
    ] {
        if let Some(value) = value {
            validate_sha256(value, name)?;
        }
    }
    match job.state {
        BackupJobState::Pending => {
            if job.attempts != 0 || job.lease.is_some() || !job.receipts.is_empty() {
                return Err(Error::Invalid(
                    "pending backup job has execution state".into(),
                ));
            }
        }
        BackupJobState::Leased | BackupJobState::Prepared => {
            if job.attempts == 0 || job.lease.is_none() {
                return Err(Error::Invalid(
                    "active backup job lacks a lease attempt".into(),
                ));
            }
            if job.state == BackupJobState::Leased && !job.receipts.is_empty() {
                return Err(Error::Invalid("leased backup job has receipts".into()));
            }
            if job.state == BackupJobState::Prepared
                && !job
                    .receipts
                    .iter()
                    .any(|receipt| receipt.boundary == BackupReceiptBoundary::Prepared)
            {
                return Err(Error::Invalid(
                    "prepared backup job lacks its receipt".into(),
                ));
            }
        }
        BackupJobState::Succeeded => {
            if result_count != 3
                || job.error.is_some()
                || !job
                    .receipts
                    .iter()
                    .any(|receipt| receipt.boundary == BackupReceiptBoundary::Completed)
            {
                return Err(Error::Invalid(
                    "succeeded backup job lacks its result".into(),
                ));
            }
        }
        BackupJobState::Failed => {
            if job.error.is_none()
                || !job
                    .receipts
                    .iter()
                    .any(|receipt| receipt.boundary == BackupReceiptBoundary::Failed)
            {
                return Err(Error::Invalid("failed backup job lacks an error".into()));
            }
        }
    }
    Ok(())
}

pub(super) fn backup_job_mut<'a>(
    document: &'a mut EstateDocument,
    id: &CanonicalId,
) -> Result<&'a mut EstateBackupJob> {
    document
        .backup_jobs
        .get_mut(id.as_str())
        .ok_or_else(|| Error::NotFound(id.to_string()))
}

pub(super) fn validate_backup_lease(
    job: &EstateBackupJob,
    worker: &CanonicalId,
    epoch: u64,
    at: u64,
) -> Result<()> {
    let lease = job
        .lease
        .as_ref()
        .ok_or_else(|| Error::StaleLease(job.id.to_string()))?;
    if lease.owner != *worker || lease.epoch != epoch || lease.expires_at <= at {
        return Err(Error::StaleLease(job.id.to_string()));
    }
    Ok(())
}

pub(super) fn push_backup_receipt(
    job: &mut EstateBackupJob,
    boundary: BackupReceiptBoundary,
    lease_epoch: u64,
    at: u64,
    evidence_sha256: String,
) -> Result<()> {
    if job.receipts.len() == MAX_BACKUP_RECEIPTS_PER_JOB {
        return Err(Error::Invalid("backup receipt limit exceeded".into()));
    }
    job.receipts.push(BackupJobReceipt {
        boundary,
        lease_epoch,
        at,
        evidence_sha256,
    });
    Ok(())
}

pub(super) fn validate_backup_result_fields(result: &BackupResult) -> Result<()> {
    validate_sha256(&result.backup_id, "backup identity")?;
    validate_sha256(&result.archive_sha256, "backup archive")?;
    validate_sha256(&result.catalogue_sha256, "backup catalogue")?;
    validate_sha256(&result.evidence_sha256, "backup completion evidence")
}

pub(super) fn validate_backup_label(label: &str) -> Result<()> {
    if label.is_empty()
        || label.len() > 128
        || label.trim() != label
        || !label
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return Err(Error::Invalid(
            "backup label must be 1-128 ASCII alphanumeric, '.', '-', or '_' characters".into(),
        ));
    }
    Ok(())
}

pub(super) fn backup_request_sha256(
    instance_id: &CanonicalId,
    source_generation: u64,
    label: &str,
    recovery_policy: &BackupRecoveryPolicySnapshot,
) -> String {
    digest::sha256_hex(
        &serde_json::to_vec(&(
            "rrd-estate-backup-create-v1",
            instance_id,
            source_generation,
            label,
            recovery_policy,
        ))
        .expect("backup request fields serialize"),
    )
}

pub(crate) fn empty_backup_jobs() -> BTreeMap<String, EstateBackupJob> {
    BTreeMap::new()
}

pub(crate) fn empty_backup_idempotency() -> BTreeMap<String, BackupIdempotencyBinding> {
    BTreeMap::new()
}
