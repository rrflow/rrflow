//! Estate limits, errors, phases, operation kinds and states, activity policy, instances, operations and the estate document.

use super::*;

pub const ESTATE_FORMAT: u16 = 1;
pub const MAX_INSTANCES: usize = 1_024;
pub const MAX_OPERATIONS: usize = 4_096;
pub const MAX_IDEMPOTENCY_BINDINGS: usize = 4_096;
pub const MAX_RECEIPTS_PER_OPERATION: usize = 64;
pub(super) const MAX_CREATE_REPLAY_JOURNAL_ENTRIES: usize = 65_536;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Store(rrd_store::Error),
    Invalid(String),
    NotFound(String),
    AlreadyExists(String),
    IdempotencyConflict(String),
    LeaseBusy(String),
    StaleLease(String),
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Store(error) => write!(formatter, "estate storage failed: {error}"),
            Self::Invalid(message) => write!(formatter, "invalid estate state: {message}"),
            Self::NotFound(id) => write!(formatter, "estate resource not found: {id}"),
            Self::AlreadyExists(id) => write!(formatter, "estate already exists: {id}"),
            Self::IdempotencyConflict(key) => {
                write!(formatter, "estate idempotency key was rebound: {key}")
            }
            Self::LeaseBusy(operation) => {
                write!(formatter, "estate operation lease is held: {operation}")
            }
            Self::StaleLease(operation) => {
                write!(formatter, "estate operation lease is stale: {operation}")
            }
        }
    }
}

impl std::error::Error for Error {}

impl From<rrd_store::Error> for Error {
    fn from(error: rrd_store::Error) -> Self {
        Self::Store(error)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DesiredPhase {
    Running,
    Stopped,
    Absent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservedPhase {
    Unknown,
    Provisioning,
    Starting,
    Running,
    Stopping,
    Stopped,
    Deleting,
    Absent,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationKind {
    Provision,
    Start,
    Stop,
    Restart,
    Upgrade,
    Delete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationState {
    Pending,
    Leased,
    Prepared,
    Applied,
    Succeeded,
    Failed,
    Superseded,
}

impl OperationState {
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed | Self::Superseded)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReceiptBoundary {
    Prepared,
    Applied,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivityClass {
    Unknown,
    Active,
    Idle,
    Stale,
    Neglected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActivityPolicy {
    pub idle_after_ms: u64,
    pub stale_after_ms: u64,
    pub neglected_after_ms: u64,
}

impl Default for ActivityPolicy {
    fn default() -> Self {
        Self {
            idle_after_ms: 5 * 60 * 1_000,
            stale_after_ms: 30 * 60 * 1_000,
            neglected_after_ms: 7 * 24 * 60 * 60 * 1_000,
        }
    }
}

impl ActivityPolicy {
    pub fn validate(&self) -> Result<()> {
        if self.idle_after_ms == 0
            || self.idle_after_ms >= self.stale_after_ms
            || self.stale_after_ms >= self.neglected_after_ms
        {
            return Err(Error::Invalid(
                "activity thresholds must be non-zero and strictly increasing".into(),
            ));
        }
        Ok(())
    }

    pub fn classify(&self, last_activity_at: Option<u64>, evaluated_at: u64) -> ActivityClass {
        let Some(last_activity_at) = last_activity_at else {
            return ActivityClass::Unknown;
        };
        let elapsed = evaluated_at.saturating_sub(last_activity_at);
        if elapsed <= self.idle_after_ms {
            ActivityClass::Active
        } else if elapsed <= self.stale_after_ms {
            ActivityClass::Idle
        } else if elapsed <= self.neglected_after_ms {
            ActivityClass::Stale
        } else {
            ActivityClass::Neglected
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DesiredInstance {
    pub generation: u64,
    pub phase: DesiredPhase,
    pub deployment_ref: CanonicalId,
    pub version: String,
    pub configuration_sha256: String,
    pub updated_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservedInstance {
    pub generation: u64,
    pub phase: ObservedPhase,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub process_id: Option<u32>,
    pub observed_at: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InstanceActivity {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_meaningful_runtime_at: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_heartbeat_at: Option<u64>,
    pub class: ActivityClass,
    pub evaluated_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManagedInstance {
    pub id: CanonicalId,
    pub desired: DesiredInstance,
    pub observed: ObservedInstance,
    pub activity: InstanceActivity,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationLease {
    pub owner: CanonicalId,
    pub epoch: u64,
    pub acquired_at: u64,
    pub expires_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationReceipt {
    pub boundary: ReceiptBoundary,
    pub lease_epoch: u64,
    pub at: u64,
    pub evidence_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EstateOperation {
    pub id: CanonicalId,
    pub instance_id: CanonicalId,
    pub kind: OperationKind,
    pub desired_generation: u64,
    pub request_sha256: String,
    pub state: OperationState,
    pub attempts: u32,
    pub created_at: u64,
    pub updated_at: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lease: Option<OperationLease>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub receipts: Vec<OperationReceipt>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IdempotencyBinding {
    pub operation_id: CanonicalId,
    pub request_sha256: String,
    pub bound_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EstateDocument {
    pub format: u16,
    pub id: CanonicalId,
    pub revision: u64,
    pub created_at: u64,
    pub updated_at: u64,
    pub activity_policy: ActivityPolicy,
    pub authority: EstateAuthorityState,
    pub instances: BTreeMap<String, ManagedInstance>,
    pub operations: BTreeMap<String, EstateOperation>,
    pub idempotency: BTreeMap<String, IdempotencyBinding>,
    pub backup_jobs: BTreeMap<String, EstateBackupJob>,
    pub backup_idempotency: BTreeMap<String, BackupIdempotencyBinding>,
    pub recovery_policies: BTreeMap<String, EstateRecoveryPolicy>,
    pub recovery_points: BTreeMap<String, EstateRecoveryPoint>,
    pub recovery_pins: BTreeMap<String, EstateRetentionPin>,
    pub restore_evidence: BTreeMap<String, EstateRestoreEvidence>,
    pub recovery_idempotency: BTreeMap<String, RecoveryIdempotencyBinding>,
    pub recovery_prune_intents: BTreeMap<String, EstateRecoveryPruneIntent>,
}
