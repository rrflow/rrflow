//! Backup catalogue, entry, manifest, prune and restore-progress types and limits.

use super::*;

/// Version 2 adds the control-journal archive: an application-complete backup
/// carries the estate's control state (installed record, configuration,
/// security policy and audit chain, attunement) as replayable journal entries.
/// A version 1 catalogue is refused rather than read as complete.
pub const BACKUP_CATALOGUE_VERSION: u16 = 2;
pub const CONTROL_RESTORE_PROGRESS_FORMAT: u16 = 1;
pub(super) const CATALOGUE_FILE: &str = "catalogue.json";
pub(super) const ARCHIVE_DIRECTORY: &str = "archives";
pub(super) const OBJECT_MANIFEST_DIRECTORY: &str = "object-manifests";
pub(super) const CATALOGUE_MANIFEST_DIRECTORY: &str = "catalogue-manifests";
pub(super) const CONTROL_ARCHIVE_DIRECTORY: &str = "control-archives";
pub(super) const OBJECT_PAYLOAD_DIRECTORY: &str = "payloads";
pub(super) const OBJECT_MANIFEST_VERSION: u16 = 1;
pub(super) const PAGE_SIZE: usize = 1_024;
pub(super) const CATALOGUE_MANIFEST_VERSION: u16 = 1;
pub(super) const MAX_ARCHIVE_OBJECTS: usize = 1_000_000;
pub(super) const MAX_ARCHIVE_CATALOGUE_RECORDS: usize = 1_000_000;
pub(super) const MAX_ARCHIVE_MANIFEST_BYTES: u64 = 256 * 1024 * 1024;
pub const MAX_BACKUP_PRUNE_RECEIPTS: usize = 4_096;
pub(super) static BACKUP_TEMP_ID: AtomicU64 = AtomicU64::new(1);

/// Page size is an acquisition hint, not permission to exceed native byte
/// admission. Learn a smaller size for this reader without changing its cut.
/// Each failed read is discarded before retry; no durable effect is retried.
pub(super) struct ControlJournalPages {
    limit: usize,
}

impl ControlJournalPages {
    pub(super) fn new() -> Self {
        Self { limit: PAGE_SIZE }
    }

    pub(super) fn read(
        &mut self,
        engine: &impl StorageEngine,
        after: u64,
    ) -> Result<Vec<crate::ControlJournalEntry>> {
        loop {
            match engine.control().journal_since(after, self.limit) {
                Err(Error::ProjectedReadLimit {
                    resource:
                        crate::ProjectedReadResource::OutputBufferBytes
                        | crate::ProjectedReadResource::BatchAllocatedBytes,
                    ..
                }) if self.limit > 1 => self.limit /= 2,
                result => return result,
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackupCoverage {
    Included,
    ReferencedOnly,
    RebuildRequired,
    Excluded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BackupEntry {
    pub backup_id: String,
    pub label: String,
    pub created_at: u64,
    pub archive_file: String,
    pub archive: LogicalArchiveInventory,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub object_manifest_file: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub object_manifest: Option<ObjectPayloadManifestInventory>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub catalogue_manifest_file: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub catalogue_manifest: Option<CatalogueManifestInventory>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub control_archive_file: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub control_archive: Option<ControlArchiveInventory>,
    pub claims: BackupCoverage,
    pub typed_runtime: BackupCoverage,
    pub catalogues: BackupCoverage,
    /// The estate's control state outside the catalogues: installed record and
    /// checkpoint, active configuration, security policy and audit chain, and
    /// attunement plan and job state, carried as replayable journal entries.
    pub installed_control: BackupCoverage,
    pub object_payloads: BackupCoverage,
    pub projections: BackupCoverage,
    pub invocation_telemetry: BackupCoverage,
    pub snapshot_leases: BackupCoverage,
    pub application_complete: bool,
}

/// Durable progress of one control-journal archive replay into a restored
/// root. It is written in the same physical transaction as each replayed
/// transition and stays in the root as restore provenance once complete.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControlRestoreProgress {
    pub format_version: u16,
    pub archive_sha256: String,
    pub entry_count: u64,
    pub applied: u64,
    pub complete: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_restored_sequence: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_restored_sequence: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObjectPayloadManifestInventory {
    pub format_version: u16,
    pub manifest_sha256: String,
    pub object_count: u64,
    pub payload_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogueManifestInventory {
    pub format_version: u16,
    pub manifest_sha256: String,
    pub source_control_sequence: u64,
    pub scope_count: u64,
    pub record_count: u64,
    pub payload_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CatalogueScopeSnapshot {
    pub(super) scope: ScopeId,
    pub(super) revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CatalogueRecordSnapshot {
    pub(super) scope: ScopeId,
    pub(super) key: String,
    pub(super) value: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CatalogueManifest {
    pub(super) format_version: u16,
    pub(super) source_control_sequence: u64,
    pub(super) scopes: Vec<CatalogueScopeSnapshot>,
    pub(super) records: Vec<CatalogueRecordSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct StoredCatalogueManifest {
    pub(super) payload: CatalogueManifest,
    pub(super) payload_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ObjectPayloadEntry {
    pub(super) sha256: String,
    pub(super) length: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ObjectPayloadManifest {
    pub(super) format_version: u16,
    pub(super) objects: Vec<ObjectPayloadEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct StoredObjectPayloadManifest {
    pub(super) payload: ObjectPayloadManifest,
    pub(super) payload_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BackupCatalogue {
    pub format_version: u16,
    pub revision: u64,
    pub catalogue_sha256: String,
    pub backups: Vec<BackupEntry>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub prune_receipts: Vec<BackupPruneReceipt>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BackupPrunePlan {
    pub expected_catalogue_sha256: String,
    pub retained_backup_ids: Vec<String>,
    pub prune_candidate_backup_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BackupPruneReceipt {
    pub request_sha256: String,
    pub expected_catalogue_sha256: String,
    pub retained_backup_ids: Vec<String>,
    pub removed_backups: Vec<BackupEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupPruneOutcome {
    pub catalogue: BackupCatalogue,
    pub pruned_backup_ids: Vec<String>,
    pub idempotent_replay: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CataloguePayload {
    pub(super) format_version: u16,
    pub(super) revision: u64,
    pub(super) backups: Vec<BackupEntry>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(super) prune_receipts: Vec<BackupPruneReceipt>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct StoredCatalogue {
    pub(super) payload: CataloguePayload,
    pub(super) payload_sha256: String,
}
