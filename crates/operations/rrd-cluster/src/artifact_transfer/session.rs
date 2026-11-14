//! Transfer session policy, inventory, telemetry and state, and preparing and running transfers.

use super::*;

pub(super) const REPLAY_PAGE: usize = 4_096;
pub(super) const TRANSFER_SESSION_DIRECTORY: &str = "transfer-sessions-v1";
pub(super) const TRANSFER_SESSION_STATE_FILE: &str = "session.json";
pub(super) const TRANSFER_SESSION_STATE_VERSION: u16 = 1;
pub const ARTIFACT_TRANSFER_TELEMETRY_VERSION: u16 = 1;
pub(super) static PUBLISH_ORDINAL: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactTransferSessionPolicy {
    pub max_active_sessions: usize,
    pub max_reserved_bytes: u64,
    pub stale_incomplete_after_millis: u64,
    pub completed_receipt_retention_millis: u64,
    pub max_retained_receipts: usize,
}

impl Default for ArtifactTransferSessionPolicy {
    fn default() -> Self {
        Self {
            max_active_sessions: 64,
            max_reserved_bytes: 64 * 1024 * 1024 * 1024,
            stale_incomplete_after_millis: 24 * 60 * 60 * 1_000,
            completed_receipt_retention_millis: 7 * 24 * 60 * 60 * 1_000,
            max_retained_receipts: 4_096,
        }
    }
}

impl ArtifactTransferSessionPolicy {
    pub fn validate(&self) -> Result<()> {
        if self.max_active_sessions == 0
            || self.max_active_sessions > 65_536
            || self.max_reserved_bytes == 0
            || self.stale_incomplete_after_millis == 0
            || self.completed_receipt_retention_millis == 0
            || self.max_retained_receipts == 0
            || self.max_retained_receipts > 1_000_000
        {
            return Err(ClusterError::Invalid(
                "artifact session policy is outside its bounded contract".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactTransferSessionInventory {
    pub active_sessions: usize,
    pub reserved_bytes: u64,
    pub partial_bytes: u64,
    pub retained_receipts: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactTransferGcReport {
    pub scanned_sessions: usize,
    pub removed_incomplete: usize,
    pub removed_completed: usize,
    pub reclaimed_partial_bytes: u64,
    pub remaining: ArtifactTransferSessionInventory,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactTransferTelemetrySnapshot {
    pub contract_version: u16,
    pub started_at: u64,
    pub observed_at: u64,
    pub policy: ArtifactTransferSessionPolicy,
    pub inventory: ArtifactTransferSessionInventory,
    pub begin_requests: u64,
    pub chunk_requests: u64,
    pub complete_requests: u64,
    pub begin_responses: u64,
    pub accepted_chunks: u64,
    pub completed_responses: u64,
    pub completed_receipt_replays: u64,
    pub denied: u64,
    pub failed: u64,
    pub quota_denials: u64,
    pub gc_runs: u64,
    pub gc_removed_incomplete: u64,
    pub gc_removed_completed: u64,
    pub gc_reclaimed_partial_bytes: u64,
    pub overflowed: bool,
}

impl ArtifactTransferTelemetrySnapshot {
    pub fn validate(&self) -> Result<()> {
        self.policy.validate()?;
        if self.contract_version != ARTIFACT_TRANSFER_TELEMETRY_VERSION
            || self.started_at > self.observed_at
            || self.inventory.active_sessions > self.policy.max_active_sessions
            || self.inventory.reserved_bytes > self.policy.max_reserved_bytes
            || self.inventory.partial_bytes > self.inventory.reserved_bytes
            || self.inventory.retained_receipts > self.policy.max_retained_receipts
        {
            return Err(ClusterError::Invalid(
                "artifact transfer telemetry is outside its bounded contract".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug)]
pub(super) struct ArtifactTransferTelemetryState {
    pub(super) started_at: u64,
    pub(super) begin_requests: u64,
    pub(super) chunk_requests: u64,
    pub(super) complete_requests: u64,
    pub(super) begin_responses: u64,
    pub(super) accepted_chunks: u64,
    pub(super) completed_responses: u64,
    pub(super) completed_receipt_replays: u64,
    pub(super) denied: u64,
    pub(super) failed: u64,
    pub(super) quota_denials: u64,
    pub(super) gc_runs: u64,
    pub(super) gc_removed_incomplete: u64,
    pub(super) gc_removed_completed: u64,
    pub(super) gc_reclaimed_partial_bytes: u64,
    pub(super) overflowed: bool,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum ArtifactTransferRequestKind {
    Begin,
    Chunk,
    Complete,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ArtifactTransferSessionState {
    pub(super) version: u16,
    pub(super) manifest_digest: String,
    pub(super) last_activity_at: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) completed_at: Option<u64>,
}

impl ArtifactTransferSessionState {
    pub(super) fn active(manifest_digest: String, now: u64) -> Self {
        Self {
            version: TRANSFER_SESSION_STATE_VERSION,
            manifest_digest,
            last_activity_at: now,
            completed_at: None,
        }
    }

    pub(super) fn validate(&self, expected_digest: &str) -> Result<()> {
        if self.version != TRANSFER_SESSION_STATE_VERSION || self.manifest_digest != expected_digest
        {
            return Err(ClusterError::Denied(
                "artifact session state differs from its directory identity".into(),
            ));
        }
        Ok(())
    }
}

pub(super) struct SessionInventoryEntry {
    pub(super) digest: String,
    pub(super) path: PathBuf,
    pub(super) state: ArtifactTransferSessionState,
    pub(super) complete: bool,
    pub(super) reserved_bytes: u64,
    pub(super) partial_bytes: u64,
}

/// Captures every immutable object reference visible at one exact project read
/// and binds its digest closure into the replica transfer plan.
pub fn prepare_artifact_transfer<E: StorageEngine>(
    mut plan: crate::ReplicaTransferPlan,
    engine: &E,
    scope: &ScopeId,
) -> Result<ArtifactTransferManifest> {
    plan.validate()?;
    let read = engine
        .runtime()
        .read_stamp(scope)
        .map_err(cluster_store_error)?;
    let mut cursor = 0;
    let mut objects = BTreeMap::new();
    loop {
        let page = engine
            .runtime()
            .read_changes(&read, cursor, REPLAY_PAGE)
            .map_err(cluster_store_error)?;
        for change in page.changes {
            if !change.verify_digest() {
                return Err(ClusterError::Invalid(
                    "artifact transfer replay encountered a corrupt change digest".into(),
                ));
            }
            if let RuntimeMutation::Object { object } = change.mutation {
                objects.insert(object.reference.clone(), object);
            }
        }
        if page.through_cursor == read.commit_cursor {
            break;
        }
        if page.through_cursor <= cursor {
            return Err(ClusterError::Unavailable(
                "artifact transfer replay did not reach its project read stamp".into(),
            ));
        }
        cursor = page.through_cursor;
    }
    let objects = objects.into_values().collect::<Vec<_>>();
    plan.artifact_digests = objects
        .iter()
        .map(|object| object.sha256.clone())
        .collect::<BTreeSet<_>>();
    ArtifactTransferManifest::new(plan, scope.clone(), read, objects)
}

/// Hydrates a target object tier from a verified source. Existing verified
/// content is reused; partial success remains harmless unreachable content and
/// no completion receipt exists until the entire manifest verifies.
pub fn transfer_artifacts<S, T>(
    source: &S,
    target: &T,
    manifest: &ArtifactTransferManifest,
    completed_at: u64,
) -> Result<ArtifactTransferReceipt>
where
    S: ImmutableObjectStore,
    T: ImmutableObjectStore,
{
    manifest.validate()?;
    let mut receipts_by_digest = BTreeMap::new();
    let mut transferred_by_digest = BTreeMap::new();
    for object in &manifest.objects {
        if receipts_by_digest.contains_key(&object.sha256) {
            continue;
        }
        let existing = target.verify(&object.sha256);
        let (verified, transferred) = match existing {
            Ok(verified) if verified.length == object.length => (verified, false),
            Ok(verified) => {
                return Err(ClusterError::Invalid(format!(
                    "target object {} has length {}, expected {}",
                    object.sha256, verified.length, object.length
                )))
            }
            Err(StoreError::ObjectMissing(_)) | Err(StoreError::ObjectCorrupt { .. }) => {
                let mut reader = source.open_verified(object).map_err(cluster_store_error)?;
                let verified = target
                    .put_verified_stream(&object.sha256, object.length, reader.as_mut())
                    .map_err(cluster_store_error)?;
                (verified, true)
            }
            Err(error) => return Err(cluster_store_error(error)),
        };
        if verified.sha256 != object.sha256 || verified.length != object.length {
            return Err(ClusterError::Invalid(
                "target object verification differs from the transfer manifest".into(),
            ));
        }
        transferred_by_digest.insert(object.sha256.clone(), transferred);
        receipts_by_digest.insert(object.sha256.clone(), verified.receipt);
    }
    let mut accounted = BTreeSet::new();
    let receipts = manifest
        .objects
        .iter()
        .map(|object| ArtifactReplicaObjectReceipt {
            reference: object.reference.clone(),
            sha256: object.sha256.clone(),
            length: object.length,
            target: receipts_by_digest[&object.sha256].clone(),
            transferred: transferred_by_digest[&object.sha256]
                && accounted.insert(object.sha256.clone()),
        })
        .collect();
    ArtifactTransferReceipt::new(manifest, receipts, completed_at)
}
