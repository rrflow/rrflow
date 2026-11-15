use super::{require, Result};
use rrd_contract::{CapabilityStatus, DiagnosticStorageSnapshot, EngramConfiguration};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Largest integer exactly representable by every JSON client used for evidence.
pub const MAX_EXACT_INTEGER: u64 = 9_007_199_254_740_991;
pub const MAX_CASES: usize = 256;
pub const MAX_ATTEMPTS: usize = 10_000;
pub const MAX_DOCUMENT_BYTES: u64 = 8 * 1024 * 1024;

/// The evaluation vocabulary's bounded identifier and digest, shared with every harness.
pub use rrflow_evaluation_core::{Digest, Id};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(try_from = "u64", into = "u64")]
#[schemars(!try_from, !into)]
pub struct Limit(#[schemars(range(min = 1, max = 9007199254740991_u64))] u64);
impl TryFrom<u64> for Limit {
    type Error = String;
    fn try_from(value: u64) -> Result<Self> {
        require(
            (1..=MAX_EXACT_INTEGER).contains(&value),
            "limit must be positive, finite and JSON-exact",
        )?;
        Ok(Self(value))
    }
}
impl From<Limit> for u64 {
    fn from(value: Limit) -> Self {
        value.0
    }
}
impl Limit {
    pub fn get(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceClass {
    Synthetic,
    IoModel,
    NativeProcess,
    NativeCapacity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DurabilityClass {
    Authoritative,
    Buffered,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum NativePlatform {
    LinuxExt4,
    MacosApfs,
    WindowsNtfs,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Candidate {
    #[schemars(regex(pattern = "^[0-9a-f]{40}$"))]
    pub commit: String,
    #[schemars(regex(pattern = "^[0-9a-f]{40}$"))]
    pub tree: String,
    pub clean: bool,
    pub executable_sha256: Digest,
    pub public_catalogue_sha256: Digest,
    pub configuration_sha256: Digest,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Environment {
    pub platform: NativePlatform,
    #[schemars(length(min = 1, max = 256))]
    pub target: String,
    #[schemars(length(min = 1, max = 256))]
    pub toolchain: String,
    #[schemars(length(min = 1, max = 256))]
    pub cpu: String,
    #[schemars(length(min = 1, max = 256))]
    pub os_version: String,
    #[schemars(length(min = 1, max = 256))]
    pub filesystem_device: String,
    #[schemars(length(min = 1, max = 256))]
    pub mount_options: String,
    #[schemars(length(min = 1, max = 256))]
    pub flush_primitive: String,
    #[schemars(length(min = 1, max = 256))]
    pub measurement_tool: String,
}

/// Proposed, frozen laboratory ceilings. They are NOT installed configuration.
/// Byte reservations are disjoint; recovery replay bytes differ from memory.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Budgets {
    pub atomic_batch_bytes: Limit,
    pub atomic_mutations: Limit,
    pub mutable_bytes: Limit,
    pub mutable_versions: Limit,
    pub queue_bytes: Limit,
    pub queue_requests: Limit,
    pub queue_wait_ms: Limit,
    pub maintenance_bytes: Limit,
    pub maintenance_workers: Limit,
    pub cache_bytes: Limit,
    pub decoded_bytes: Limit,
    pub retained_bytes: Limit,
    pub recovery_memory_bytes: Limit,
    pub recovery_replay_bytes: Limit,
    pub recovery_frames: Limit,
    pub recovery_ms: Limit,
    pub process_bytes: Limit,
    /// Optional shared native pool. Individual ceilings are not reservations;
    /// the summed per-engram disjoint reservations must fit this aggregate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aggregate_process_bytes: Option<Limit>,
    pub host_bytes: Limit,
    pub engrams: Limit,
    pub disk_bytes: Limit,
    pub temporary_disk_bytes: Limit,
    pub recovery_reserve_bytes: Limit,
    pub run_ms: Limit,
    pub attempts: Limit,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OperationBinding {
    pub operation: Id,
    /// Reported driver support, not activation or an inferred catalogue promise.
    pub availability: CapabilityStatus,
    pub driver_sha256: Digest,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RequiredCase {
    pub id: Id,
    pub operation: Id,
    #[schemars(range(max = 9007199254740991_u64))]
    pub seed: u64,
    pub expected_ledger_sha256: Digest,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StorageManifest {
    #[schemars(range(min = 1, max = 1))]
    pub format_version: u16,
    pub profile: Id,
    pub evidence_class: EvidenceClass,
    pub durability: DurabilityClass,
    pub candidate: Candidate,
    pub environment: Environment,
    pub corpus_sha256: Digest,
    pub installed_configuration: EngramConfiguration,
    pub storage_observation: DiagnosticStorageSnapshot,
    pub budgets: Budgets,
    #[schemars(length(min = 1, max = 256))]
    pub bindings: Vec<OperationBinding>,
    #[schemars(length(min = 1, max = 256))]
    pub required_cases: Vec<RequiredCase>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CaseStatus {
    Executed,
    Failed,
    Skipped,
    Unimplemented,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FailurePhase {
    Admission,
    Append,
    Sync,
    Publication,
    Recovery,
    Readback,
    Measurement,
    Cleanup,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Failure {
    pub phase: FailurePhase,
    pub code: Id,
    pub occurrence: Limit,
    pub artifact_sha256: Digest,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CaseResult {
    pub id: Id,
    #[schemars(range(max = 9007199254740991_u64))]
    pub seed: u64,
    pub status: CaseStatus,
    /// Faults are retained even if recovery and the independent oracle pass.
    #[schemars(length(max = 256))]
    pub faults: Vec<Failure>,
    pub limitation: Option<Id>,
    pub ledger: super::AcknowledgementLedger,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StorageResult {
    #[schemars(range(min = 1, max = 1))]
    pub format_version: u16,
    pub manifest_sha256: Digest,
    #[schemars(length(min = 1, max = 256))]
    pub cases: Vec<CaseResult>,
}
