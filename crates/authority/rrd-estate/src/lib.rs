//! Persistent estate authority for RRD.
//!
//! This crate owns transport-neutral desired/observed state and durable
//! reconciliation boundaries. Connectome is a projection of these records,
//! never their source of truth.

mod authority;
mod backup_job;
mod backup_reconcile;
mod local_authorization;
mod local_process;
mod reconcile;
mod recovery;

pub use authority::{
    ApplyAuthority, AuthorityDesiredState, AuthorityIdempotencyBinding, AuthorityMutationOutcome,
    AuthorityObservedState, AuthorityReceipt, AuthorityReceiptBoundary, AuthorityResource,
    AuthorityResourceKind, AuthorityStatus, EstateAuthorityState, ESTATE_AUTHORITY_FORMAT,
    MAX_AUTHORITY_IDEMPOTENCY_BINDINGS, MAX_AUTHORITY_RECEIPTS, MAX_AUTHORITY_RESOURCES,
};
pub use backup_job::{
    public_backup_job, public_backup_jobs, BackupCompleteRequest, BackupFailRequest,
    BackupIdempotencyBinding, BackupJobReceipt, BackupJobState, BackupLeaseRequest,
    BackupPreparedRequest, BackupReceiptBoundary, BackupRecoveryPolicySnapshot, BackupResult,
    BackupScheduleOutcome, EstateBackupJob, ScheduleBackup, MAX_BACKUP_IDEMPOTENCY_BINDINGS,
    MAX_BACKUP_JOBS, MAX_BACKUP_RECEIPTS_PER_JOB,
};
pub use backup_reconcile::{
    BackupDriverRequest, BackupReconcileBoundary, BackupReconcileOutcome, BackupReconciler,
    EstateBackupDriver,
};
pub use local_authorization::{
    LocalEstatePermission, LocalOperatorAuthorization, LocalOperatorPolicy,
    LOCAL_OPERATOR_POLICY_FORMAT,
};
pub use local_process::{
    LocalArgument, LocalDeployment, LocalDeploymentCatalog, LocalProcessDriver, LocalReadiness,
    LocalShutdown, LOCAL_DEPLOYMENT_FORMAT,
};
pub use reconcile::{
    DriverEffect, DriverError, DriverErrorKind, DriverObservation, DriverRequest, EstateDriver,
    ReconcileBoundary, ReconcileOutcome, Reconciler,
};
pub use recovery::{
    public_recovery_snapshot, public_retention_decision, retention_decision, CompleteRecoveryPrune,
    EstateRecoveryPoint, EstateRecoveryPolicy, EstateRecoveryPruneIntent, EstateRestoreEvidence,
    EstateRetentionDecision, EstateRetentionPin, EstateRetentionPinKind, PinRecoveryPoint,
    PrepareRecoveryPrune, RecordRestoreEvidence, RecoveryIdempotencyBinding,
    RecoveryMutationOutcome, ReleaseRecoveryPin, SetRecoveryPolicy,
    MAX_RECOVERY_IDEMPOTENCY_BINDINGS, MAX_RECOVERY_PINS, MAX_RECOVERY_POINTS,
    MAX_RECOVERY_POLICIES, MAX_RECOVERY_PRUNE_INTENTS, MAX_RESTORE_EVIDENCE,
};

use rrd_contract::CanonicalId;
use rrd_core::digest;
use rrd_store::{ControlTransition, StorageEngine};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

mod document;
mod model;
mod repository;
mod requests;
mod validation;

pub use document::*;
pub use model::*;
pub use requests::*;
use validation::*;
