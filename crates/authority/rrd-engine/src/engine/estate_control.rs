mod admin;
mod drivers;
mod local_recovery;
mod recovery;

pub(super) use drivers::*;
use local_recovery::*;

use super::estate_layout::{
    instance_operation_root, BACKUPS_DIRECTORY_NAME, RESTORES_DIRECTORY_NAME,
};

use super::installation::read_locator;

use super::restore_publication::require_distribution_binding;

use super::security::AuditEvent;

use super::*;

use rrd_estate::{EstateBackupDriver, EstateDriver};

use std::fs;

#[cfg(debug_assertions)]
use std::io::Write as _;

const ESTATE_RECOVERY_OPERATION_FORMAT: u16 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct EstateRecoveryPruneOperationState {
    format_version: u16,
    operation_sha256: String,
    instance_id: CanonicalId,
    operation_id: CanonicalId,
    evaluated_at: u64,
    expected_estate_revision: u64,
    expected_catalogue_sha256: String,
    retained_backup_ids: Vec<String>,
    prune_candidate_backup_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    result: Option<rrd_contract::EstateRecoveryPruneResult>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct EstateRecoveryRestoreOperationState {
    format_version: u16,
    operation_sha256: String,
    instance_id: CanonicalId,
    operation_id: CanonicalId,
    backup_sha256: String,
    restore_id: CanonicalId,
    started_at: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    result: Option<rrd_contract::EstateRecoveryRestoreResult>,
}

/// A bounded estate mutation accepted by the RRD composition root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case", tag = "action")]
pub enum EstateAdminAction {
    Create,
    SetDesired {
        instance: CanonicalId,
        idempotency_key: String,
        phase: rrd_contract::EstateDesiredPhase,
        deployment: CanonicalId,
        version: String,
        configuration_sha256: String,
    },
    ScheduleBackup {
        instance: CanonicalId,
        idempotency_key: String,
        label: String,
    },
    SetRecoveryPolicy {
        instance: CanonicalId,
        idempotency_key: String,
        max_rpo_ms: u64,
        max_rto_ms: u64,
        minimum_recovery_points: u16,
        retention_ms: u64,
    },
    PinRecoveryPoint {
        idempotency_key: String,
        backup_sha256: String,
        expires_at_unix_ms: Option<u64>,
    },
    ReleaseRecoveryPin {
        idempotency_key: String,
        pin_id: CanonicalId,
    },
}

/// Public result from an engine-owned estate administration operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum EstateAdminResult {
    Mutation(Box<rrd_contract::EstateMutationResult>),
    Backup(Box<rrd_contract::EstateBackupMutationResult>),
    Recovery(Box<rrd_contract::EstateRecoveryMutationResult>),
}

pub type EstateReconcileOutcome = rrd_estate::ReconcileOutcome;

pub type EstateBackupReconcileOutcome = rrd_estate::BackupReconcileOutcome;

fn estate_audit_resource(instance: &CanonicalId, estate: &CanonicalId) -> ResourcePath {
    ResourcePath {
        segments: vec![
            ResourceId::new(ResourceKind::Instance, instance.as_str())
                .expect("engine instance identity is canonical"),
            ResourceId::new(ResourceKind::Estate, estate.as_str())
                .expect("estate identity is canonical"),
        ],
    }
}

#[allow(clippy::too_many_arguments)]
fn audit_estate_operation<T, F>(
    engine: &RrdEngine,
    at: u64,
    principal_id: CanonicalId,
    resource: ResourcePath,
    request_id: String,
    operation_id: String,
    request_sha256: String,
    operation: F,
) -> Result<T>
where
    T: Serialize,
    F: FnOnce() -> Result<T>,
{
    engine.append_audit(AuditEvent {
        at_unix_ms: at,
        attempt: 1,
        principal_id: Some(principal_id.clone()),
        action: SecurityAction::EstateAdmin,
        resource: resource.clone(),
        request_id: request_id.clone(),
        operation_id: operation_id.clone(),
        phase: AuditPhase::Authorized,
        decision: AuditDecision::Allowed,
        status_code: 100,
        request_sha256: request_sha256.clone(),
        response_sha256: digest::sha256_hex(b"rrd-audit-completion-pending"),
    })?;
    let result = operation();
    let (decision, status_code, response_sha256) = match &result {
        Ok(response) => (
            AuditDecision::Allowed,
            200,
            digest::sha256_hex(&serde_json::to_vec(response).map_err(contract_json)?),
        ),
        Err(error) => {
            let (decision, status_code) = super::invocation::audit_failure(error);
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
        principal_id: Some(principal_id),
        action: SecurityAction::EstateAdmin,
        resource,
        request_id,
        operation_id,
        phase: AuditPhase::Completed,
        decision,
        status_code,
        request_sha256,
        response_sha256,
    })?;
    result
}
