//! Explicit engine-owned attunement control. Bounded precondition observations
//! do not activate background execution or external capabilities. Connect verifies
//! the installation; Inventory commits bounded source observations with its
//! checkpoint. Other phases fail explicitly until their implementation is present.

mod control;
mod read;
mod replan;

use super::installation::{
    attunement_job_key, attunement_plan_key, canonical_prefixed, engram_configuration_key,
    project_inventory_sha256, validate_parse_policies, DefaultInstallProfile,
    InstalledEngramRecord, INSTALLED_RECORD_KEY,
};

use super::*;

use rrd_contract::{
    attunement_checkpoint_sha256, attunement_plan_sha256, attunement_replan_accept_sha256,
    attunement_replan_preview_sha256, attunement_replan_seed, derive_attunement_plan,
    AttunementFailure, AttunementJob, AttunementJobState, AttunementLease, AttunementPhase,
    AttunementPhaseCheckpoint, AttunementPlan, AttunementReadResult, AttunementRuntimeCoordinates,
    AttunementSourceVerification, AttunementStatus, CancelAttunement, ReadAttunement,
    ResumeAttunement, ATTUNEMENT_CONTRACT_VERSION,
};

pub use rrd_contract::{
    AttunementReplanPreview, AttunementReplanReceiptView, AttunementReplanResult,
};

use std::time::Instant;

mod discovery;

mod evidence;

mod follow;

pub use follow::{FollowOutcome, FollowTick};

mod inventory;

#[cfg(target_os = "linux")]
mod linux_supervisor;

mod parse;

#[cfg(windows)]
mod windows_supervisor;

#[allow(unused_imports)] // The installed Parse executor will consume this binding.
pub(in crate::engine) use inventory::load_parse_inventory;

#[cfg(test)]
pub(crate) use inventory::LegacySourceFileProjectionGuard;

pub(super) use inventory::{load_accepted_source, source_head_key};

#[cfg(any(windows, target_os = "linux"))]
const WORKER_FLAG: &str = "--rrflow-internal-parse-worker-v1";

/// Records the engine's own executable before any parser worker can start.
pub(super) fn capture_primary_executable() {
    #[cfg(windows)]
    windows_supervisor::capture_primary_executable();
}

const MAX_CONTROL_BYTES: usize = 64 * 1024;

const MAX_JOB_TRANSITIONS: u64 = 1024;

const MAX_LEASE_MS: u64 = 60_000;

/// Engine admission. Callers supply exact revision/plan identity, never a
/// checkpoint or phase output; transports expose only catalogued commands.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum AttunementCommand {
    Start {
        job_id: CanonicalId,
        plan_sha256: String,
        expected_revision: u64,
        lease_ms: u64,
    },
    Resume {
        request: ResumeAttunement,
        lease_ms: u64,
    },
    Step {
        job_id: CanonicalId,
        expected_revision: u64,
        lease_generation: u64,
    },
    Pause {
        job_id: CanonicalId,
        expected_revision: u64,
        lease_generation: u64,
    },
    RecoverExpired {
        job_id: CanonicalId,
        expected_revision: u64,
    },
    Cancel {
        request: CancelAttunement,
    },
}

impl AttunementCommand {
    fn binding(&self) -> (&CanonicalId, u64) {
        match self {
            Self::Start {
                job_id,
                expected_revision,
                ..
            }
            | Self::Step {
                job_id,
                expected_revision,
                ..
            }
            | Self::Pause {
                job_id,
                expected_revision,
                ..
            }
            | Self::RecoverExpired {
                job_id,
                expected_revision,
            } => (job_id, *expected_revision),
            Self::Resume { request, .. } => (&request.job_id, request.expected_job_revision),
            Self::Cancel { request } => (&request.job_id, request.expected_job_revision),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttunementControlResult {
    /// On replay, the exact accepted snapshot, not the potentially newer job.
    pub status: AttunementStatus,
    pub idempotent_replay: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReplanReceipt {
    format_version: u16,
    principal_id: CanonicalId,
    operation_sha256: String,
    result: AttunementReplanResult,
}

/// An existing committed control receipt, read without replaying its command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttunementReceiptView {
    pub plan: AttunementPlan,
    pub principal_id: CanonicalId,
    pub operation_sha256: String,
    pub status: AttunementStatus,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Receipt {
    format_version: u16,
    principal_id: CanonicalId,
    operation_sha256: String,
    status: AttunementStatus,
}

fn advance_after_checkpoint(job: &mut AttunementJob, plan: &AttunementPlan) {
    job.current_phase = job.next_phase(plan);
    if job.current_phase.is_none() {
        job.state = AttunementJobState::Succeeded;
        job.lease = None;
    }
}

fn attunement_replan_receipt_key(
    instance: &CanonicalId,
    job_id: &CanonicalId,
    principal: &CanonicalId,
    key: &CorrelationId,
) -> Result<String> {
    Ok(format!(
        "{}/replan-receipt/{}",
        attunement_job_key(instance, job_id),
        operation_digest(&(principal, key))?
    ))
}

fn attunement_receipt_key(
    instance: &CanonicalId,
    job_id: &CanonicalId,
    principal: &CanonicalId,
    idempotency_key: &CorrelationId,
) -> Result<String> {
    Ok(format!(
        "{}/receipt/{}",
        attunement_job_key(instance, job_id),
        operation_digest(&(principal, idempotency_key))?
    ))
}

fn claim(
    job: &mut AttunementJob,
    plan: &AttunementPlan,
    holder: &CanonicalId,
    lease_ms: u64,
    at: u64,
) -> Result<()> {
    if lease_ms == 0 || lease_ms > MAX_LEASE_MS {
        return Err(ServiceError::ConfigurationLimit(
            "attunement lease must be 1..=60000 milliseconds".into(),
        ));
    }
    job.state = AttunementJobState::Running;
    job.current_phase = job.next_phase(plan);
    job.failure = None;
    job.lease = Some(AttunementLease {
        id: CanonicalId::new(format!(
            "lease-{}",
            operation_digest(&(&job.id, job.revision))?
        ))
        .map_err(contract)?,
        holder: holder.clone(),
        generation: job.revision,
        acquired_at_unix_ms: at,
        expires_at_unix_ms: at
            .checked_add(lease_ms)
            .ok_or_else(|| ServiceError::Contract("lease time overflow".into()))?,
    });
    Ok(())
}

fn require_worker(
    job: &AttunementJob,
    holder: &CanonicalId,
    generation: u64,
    at: u64,
) -> Result<()> {
    if job.state != AttunementJobState::Running
        || job.lease.as_ref().is_none_or(|lease| {
            &lease.holder != holder
                || lease.generation != generation
                || at < lease.acquired_at_unix_ms
                || at >= lease.expires_at_unix_ms
        })
    {
        return Err(ServiceError::StorageConflict(
            "attunement worker lease is stale, expired or foreign".into(),
        ));
    }
    Ok(())
}

fn worker(session: &CorrelationId) -> Result<CanonicalId> {
    CanonicalId::new(format!(
        "worker-{}",
        digest::sha256_hex(session.as_str().as_bytes())
    ))
    .map_err(contract)
}

fn status(job: AttunementJob, plan: &AttunementPlan, at: u64) -> Result<AttunementStatus> {
    let status = AttunementStatus {
        contract_version: ATTUNEMENT_CONTRACT_VERSION,
        observed_at_unix_ms: at,
        job,
    };
    status.validate(plan).map_err(contract)?;
    Ok(status)
}

// Bound retained output during serialization, not after allocating arbitrary
// caller-supplied strings. One encoded control envelope is at most 64 KiB.
fn encode(value: &impl Serialize) -> Result<Vec<u8>> {
    struct Bounded(Vec<u8>);
    impl std::io::Write for Bounded {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if bytes.len() > MAX_CONTROL_BYTES.saturating_sub(self.0.len()) {
                return Err(std::io::Error::other("attunement control record too large"));
            }
            self.0.extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut writer = Bounded(Vec::with_capacity(MAX_CONTROL_BYTES));
    serde_json::to_writer(&mut writer, value).map_err(|error| {
        if error.is_io() {
            ServiceError::ConfigurationLimit("attunement control record too large".into())
        } else {
            contract_json(error)
        }
    })?;
    Ok(writer.0)
}

fn decode<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    if bytes.len() > MAX_CONTROL_BYTES {
        return Err(ServiceError::ConfigurationLimit(
            "attunement control record too large".into(),
        ));
    }
    serde_json::from_slice(bytes)
        .map_err(|error| ServiceError::Storage(format!("invalid attunement control: {error}")))
}

fn contract(error: impl std::fmt::Display) -> ServiceError {
    ServiceError::Contract(error.to_string())
}

fn elapsed_time(now: u64, started: Instant) -> Result<u64> {
    let elapsed =
        u64::try_from(started.elapsed().as_millis()).map_err(|_| ServiceError::DeadlineExceeded)?;
    now.checked_add(elapsed)
        .ok_or(ServiceError::DeadlineExceeded)
}

fn check_deadline(context: &RequestContext, now: u64) -> Result<()> {
    if context
        .deadline_unix_ms
        .is_some_and(|deadline| now >= deadline)
    {
        return Err(ServiceError::DeadlineExceeded);
    }
    Ok(())
}
