//! # rrd-store
//!
//! The semantic storage boundary shared by persistent rrflowKV and volatile
//! rrflowMX.
//!
//! rrflowKV is the only durable implementation. rrflowMX is the non-durable
//! conformance and embedded-memory profile. Both expose the same stamped
//! transactions, temporal claims, snapshots, audit chain, and projection work;
//! callers select a profile only at the engine composition root.

mod access;
#[cfg(feature = "crash-test")]
#[doc(hidden)]
pub use rrd_lsm::crash_test;
mod archive;
mod backup;
mod control;
mod ds;
mod engine;
mod error;
mod footprint;
mod gc;
mod invocation;
mod key_codec;
mod keyspaces;
mod object;
mod outcome;
mod projection;
mod repository;
mod rrflow_kv;
pub use rrflow_kv::{StorageInspectionCursor, StorageInspectionPage};
mod s3;
mod snapshot;
mod transaction;
mod writer;

pub use access::{
    FunctionArtifactMediaTypeRecord, FunctionArtifactRecord, FunctionCatalogueHeadRecord,
    FunctionCatalogueMembershipRecord, FunctionCataloguePublication, FunctionCatalogueSnapshot,
    FunctionDefinitionRecord, FunctionInvocationReceiptRecord, IndexCommitBindingDefinition,
    IndexCommitBindingKind, IndexSourceDelta, IndexSourceRow, RuntimeAdjacencyDirection,
    RuntimeReadAccessPath, RuntimeReadBudget, RuntimeReadEvidence, RuntimeReadPathEvidence,
    RuntimeVersionedRead, RuntimeVersionedSource, SemanticCommitPlan,
    TransactionFunctionBindingRecord, VectorSourceAddress, VectorSourceDelta, VectorSourceVersion,
    FUNCTION_CATALOGUE_STATE_FORMAT_VERSION, FUNCTION_INVOCATION_RECEIPT_FORMAT_VERSION,
    INDEX_COMMIT_BINDING_CONTRACT_VERSION, INDEX_SOURCE_DELTA_CONTRACT_VERSION,
    RUNTIME_VERSIONED_READ_CONTRACT_VERSION, VECTOR_SOURCE_DELTA_CONTRACT_VERSION,
};
pub use archive::{
    control_family_covers, control_family_of, export_logical_archive,
    export_logical_archive_with_progress, inspect_control_archive, inspect_logical_archive,
    read_control_archive, restore_logical_archive_to_new_root,
    restore_logical_archive_to_new_root_with, restore_logical_archive_to_new_root_with_options,
    restore_logical_archive_to_new_root_with_progress, validate_control_families,
    ControlArchiveInventory, LogicalArchiveCheckpoint, LogicalArchiveInventory,
    LogicalArchiveOperation, LogicalArchiveProgress, LogicalRestoreReport, CONTROL_ARCHIVE_VERSION,
    LOGICAL_ARCHIVE_VERSION,
};
pub use backup::{
    control_restore_progress, create_application_backup, create_logical_backup,
    load_backup_catalogue, prune_backup_catalogue, restore_catalogued_backup,
    restore_catalogued_backup_with_options, verify_backup_catalogue,
    verify_restored_backup_objects, verify_restored_backup_objects_with_options, BackupCatalogue,
    BackupCoverage, BackupEntry, BackupPruneOutcome, BackupPrunePlan, BackupPruneReceipt,
    CatalogueManifestInventory, ControlRestoreProgress, ObjectPayloadManifestInventory,
    BACKUP_CATALOGUE_VERSION, CONTROL_RESTORE_PROGRESS_FORMAT, MAX_BACKUP_PRUNE_RECEIPTS,
};
pub use control::{ControlJournalEntry, ControlTransition};
pub use ds::{DataRuntime, DataRuntimeAccess, DataRuntimeRef, DataRuntimeStep};
pub use engine::{
    PhysicalStoreEvidence, RrflowMxStore, StorageEngine, StorageProfile, LOGICAL_ONLY_EVIDENCE,
    RRFLOW_KV_COUNTER_EVIDENCE,
};
pub use error::{Error, Result};
pub use footprint::{measure_storage_footprint, FootprintBytes, StorageFootprint};
pub use gc::{PairStatus, RemovalReport, Verdict};
pub use invocation::{Invocation, InvocationInput, Outcome, Trigger};
pub use keyspaces::Durability;
pub use object::{
    ImmutableObjectStore, LocalObjectStore, MemoryObjectStore, ObjectInventory,
    ObjectInventoryEntry, ObjectInventoryState, ObjectStep, ObjectStoreBox, VerifiedObject,
};
pub use outcome::{AppendOutcome, IdempotentAppendOutcome};
pub use projection::{
    CurrentProjection, GroundedStamp, GroundingReport, ProjectionStatus, RebuildOutcome,
    CURRENT_PROJECTION,
};
pub use repository::{
    ClaimRepository, ControlRepository, FunctionCatalogueRepository, InvocationRepository,
    ProjectionRepository, RuntimeDataSnapshotRead, RuntimeRepository,
};
pub use rrd_core::{
    DataTransaction, DataTransactionView, ReadStamp, RetentionPin, RetentionPinId,
    RuntimeChangePage, RuntimeCommitOutcome, SnapshotHandle, SnapshotId,
};
pub use rrd_lsm::filesystem;
pub use rrd_lsm::{
    publish_rename as publish_durable_rename, sync_directory as sync_directory_metadata,
    DatabaseOptions, MutableLimits, MutableResource, TailRepairPlan, TailRepairReceipt,
    TransactionLimits, TransactionResource,
};
pub use rrflow_kv::{
    create_rrflow_kv_database, is_rrflow_kv_database, prepare_rrflow_kv_commit,
    rrflow_kv_commit_context, rrflow_kv_commit_outcome, rrflow_kv_database_artifact_view,
    rrflow_kv_snapshot_all_object_references, rrflow_kv_snapshot_artifact_view,
    rrflow_kv_snapshot_object_references, RrflowKvInitializationFormats, RrflowKvInspection,
    RrflowKvInspector, RrflowKvOpenEvidence, RrflowKvReconciliationEvidence, RrflowKvStore,
    RrflowKvTailRepair, RRFLOW_KV_OPEN_EVIDENCE_VERSION,
};
pub use s3::{
    ConditionalPut, S3Authentication, S3CompatibleObjectStore, S3MultipartUpload, S3ObjectClient,
    S3ObjectMetadata, S3TransferPolicy, S3TransportCapabilities, S3UploadedPart,
    DEFAULT_S3_RANGE_BYTES, S3_MAX_PARTS, S3_MAX_PART_BYTES, S3_MIN_PART_BYTES,
};
pub use transaction::{StorageTransaction, TransactionCommit, TransactionRollback};
pub use writer::{ClaimBatchWriter, ClaimBatchWriterConfig, ClaimBatchWriterStats};
