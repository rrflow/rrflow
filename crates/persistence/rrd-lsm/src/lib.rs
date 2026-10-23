//! Native storage substrate for RRD.
//!
//! This crate begins at the durable boundary. One accepted atomic batch is one
//! checksummed WAL frame. Recovery accepts only a contiguous valid prefix;
//! incomplete tail bytes are reported for explicit repair, while corruption in
//! a complete frame fails closed.

#[cfg(test)]
mod fault_io;

// The same owned rendezvous used by unit tests, explicitly compiled into a
// diagnostic primary artifact only. Never enabled by a default dependency.
#[cfg(all(feature = "crash-test", not(test)))]
#[doc(hidden)]
#[path = "fault_io/crash.rs"]
pub mod crash_test;

mod batch;
mod database;
mod error;
pub mod filesystem;
mod inspection;
mod io;
mod maintenance_limits;
mod manifest;
mod memtable;
mod mutable;
mod mutable_memory;
mod reclamation;
mod recovery_budget;
mod segment;
mod snapshot_bundle;
mod tail_repair;
mod transaction;
mod wal;
mod write_diagnostics;

use std::path::Path;

/// Requests directory durability using an admitted native directory handle.
/// Windows publication itself uses write-through rename semantics.
pub fn sync_directory(path: &Path) -> std::io::Result<()> {
    filesystem::AnchoredPath::open_root(path)?.sync_directory()
}

/// Standalone path entry point. Installed owners pass retained capabilities
/// directly; this function establishes authority for explicit utility inputs.
pub fn publish_rename(_directory: &Path, temporary: &Path, target: &Path) -> std::io::Result<()> {
    filesystem::AnchoredPath::from_path(temporary)?
        .rename_to(&filesystem::AnchoredPath::from_path(target)?)
}

pub use batch::{Mutation, WriteBatch, WriteBatchResource, WriteBatchSize, BATCH_FORMAT_VERSION};
pub use database::{
    CompactionBoundary, CompactionOutcome, CompactionPolicy, Database, DatabaseOptions,
    FailureMode, FlushBoundary, GarbageCollectionReport, MaintenancePolicy, MaintenanceStats,
    Snapshot, SnapshotInstallBoundary, WriteBoundary, DEFAULT_COMPACTION_MAX_INPUT_SEGMENTS,
    DEFAULT_COMPACTION_TARGET_SEGMENT_BYTES, DEFAULT_L0_COMPACTION_TRIGGER,
    DEFAULT_MAX_COMPACTION_LEVEL, DEFAULT_MEMTABLE_MAX_VERSIONS, DEFAULT_WAL_PAYLOAD_MAX_BYTES,
};
pub use error::{Error, Result};
pub use inspection::{DatabaseInspection, InspectionFile, ReadOnlyDatabase};
pub use io::{SegmentIoMode, SegmentIoPolicy, SegmentIoStats, DEFAULT_SEGMENT_IO_REQUEST_BYTES};
pub use maintenance_limits::{CompactionLimits, MaintenanceResource};
pub use manifest::{
    Checkpoint, CurrentPointer, Manifest, ManifestStore, SegmentDescriptor, MANIFEST_FORMAT_VERSION,
};
pub use memtable::{Memtable, MemtableProfile, VersionedValue};
pub use mutable::{MutableLimits, MutableResource};
pub use mutable_memory::{MutableMemory, MutableMemoryStats, MutableRetentionLimits};
pub use reclamation::{ReclamationLimits, ReclamationResource};
pub use recovery_budget::{RecoveryLimits, RecoveryResource};
#[cfg(feature = "physical-policy-lab")]
pub use segment::{
    run_physical_policy_trial, CachePolicyObservation, CandidateDecision, CodecObservation,
    FamilyObservation, FilterObservation, IntegratedCachePolicyObservation, PhysicalPolicyConfig,
    PhysicalPolicyTrial, ReopenedPointMissObservation, ValuePlacementObservation,
    PHYSICAL_POLICY_EVIDENCE_VERSION,
};
pub use segment::{
    PageCachePolicy, PageCacheStats, ProjectedOutputMemory, ProjectedOutputStats,
    ProjectedReadBatch, ProjectedReadBudget, ProjectedReadEvidence, ProjectedReadOutcome,
    ProjectedReadProjection, ProjectedReadRange, ProjectedReadRequest, ProjectedReadResource,
    ProjectedReadStream, Segment, SegmentCompressionPolicy, SegmentOpenEvidence,
    SegmentRowGroupBudget, COMPRESSION_BASIS_POINTS,
    DEFAULT_COMPRESSION_MAXIMUM_PAGE_LOGICAL_BYTES,
    DEFAULT_COMPRESSION_MINIMUM_SAVINGS_BASIS_POINTS, DEFAULT_MAX_ACTIVE_READ_VIEWS,
    DEFAULT_MAX_BATCH_ALLOCATED_BYTES, DEFAULT_MAX_BATCH_ROWS, DEFAULT_MAX_OUTPUT_BUFFER_BYTES,
    DEFAULT_MAX_OUTPUT_ROWS, DEFAULT_MAX_PAGE_LOGICAL_BYTES, DEFAULT_MAX_PAGE_REQUESTS,
    DEFAULT_MAX_PINNED_READ_BYTES, DEFAULT_MAX_PROJECTED_READ_RANGES,
    DEFAULT_MAX_PROJECTED_READ_RUNS, DEFAULT_MAX_VERSIONS_EXAMINED, DEFAULT_PAGE_CACHE_BYTES,
    DEFAULT_PAGE_CACHE_PROTECTED_CAPACITY_BASIS_POINTS, DEFAULT_PROJECTED_OUTPUT_BYTES,
    DEFAULT_ROW_GROUP_MAX_ROWS, DEFAULT_ROW_GROUP_TARGET_BYTES, DEFAULT_SEGMENT_COMPRESSION_POLICY,
    PROJECTED_READ_CONTRACT_VERSION, SEGMENT_FORMAT_VERSION, SEGMENT_KEY_CODEC_DIGEST,
    SEGMENT_OPEN_EVIDENCE_VERSION, SEGMENT_PAGE_FORMAT_DIGEST, SEGMENT_SCHEMA_DIGEST,
};
pub use snapshot_bundle::{
    SnapshotBundle, SnapshotBundleFile, SnapshotExportBoundary, SnapshotSegment,
    SNAPSHOT_BUNDLE_FORMAT_VERSION, SNAPSHOT_BUNDLE_MAX_BYTES,
};
pub use tail_repair::{TailRepair, TailRepairPlan, TailRepairReceipt};
pub use transaction::{
    Transaction, TransactionCommit, TransactionLimits, TransactionResource, TransactionRollback,
};
pub use wal::{
    recover, recover_from, repair_torn_tail, AppendReceipt, Durability, RecoveredBatch, Recovery,
    WalBatch, WalWriter, WAL_FORMAT_VERSION, WAL_MAX_PAYLOAD_BYTES,
};
pub use write_diagnostics::{
    WriteLockWait, WriteMaintenanceDelta, WritePathDiagnostics, WritePathPhaseTiming,
    WritePathPhases, WritePathSample, WriteThreadResourceDelta, MAX_WRITE_PATH_DIAGNOSTIC_SAMPLES,
    WRITE_PATH_DIAGNOSTICS_CONTRACT_VERSION,
};

#[cfg(test)]
mod durable_fs_tests {
    use super::*;

    #[test]
    fn directory_sync_uses_the_platform_durability_boundary() {
        let root = tempfile::tempdir().unwrap();
        sync_directory(root.path()).unwrap();
    }

    #[test]
    fn durable_rename_publishes_files_and_directories() {
        let root = tempfile::tempdir().unwrap();

        let temporary_file = root.path().join("file.pending");
        let target_file = root.path().join("file.ready");
        std::fs::write(&temporary_file, b"ready").unwrap();
        std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&temporary_file)
            .unwrap()
            .sync_all()
            .unwrap();
        publish_rename(root.path(), &temporary_file, &target_file).unwrap();
        assert_eq!(std::fs::read(&target_file).unwrap(), b"ready");
        assert!(!temporary_file.exists());

        let temporary_directory = root.path().join("directory.pending");
        let target_directory = root.path().join("directory.ready");
        std::fs::create_dir(&temporary_directory).unwrap();
        std::fs::write(temporary_directory.join("entry"), b"durable").unwrap();
        publish_rename(root.path(), &temporary_directory, &target_directory).unwrap();
        assert_eq!(
            std::fs::read(target_directory.join("entry")).unwrap(),
            b"durable"
        );
        assert!(!temporary_directory.exists());
    }
}
