//! Persistent rrflowKV implementation of the physical [`StorageEngine`] port.
//!
//! Semantic repositories encode logical keyspaces as stable byte prefixes and
//! publish through this store's snapshot transaction. The database's physical
//! MVCC sequence is deliberately independent of claim and runtime cursors
//! stored in those transactions.

use crate::access::{prepare_semantic_commit, SemanticCommitPlan};
use crate::engine::{PhysicalStoreEvidence, StorageEngine, RRFLOW_KV_COUNTER_EVIDENCE};
use crate::error::{Error, Result};
use crate::key_codec::prefix_end;
#[cfg(test)]
use crate::key_codec::KeyCodec;
use crate::keyspaces::{self, Durability};
use crate::transaction::{StorageTransaction, TransactionCommit, TransactionRollback};
use rrd_core::{
    AuditEnvelope, Millis, ObjectReference, ReadStamp, RuntimeCommit, RuntimeCommitOutcome,
    RuntimeLogAccumulator, RuntimeSchemaRegistry, ScopeId, SnapshotHandle, SnapshotId,
};
use rrd_lsm::filesystem::AnchoredPath;
use rrd_lsm::{
    CompactionOutcome, Database, DatabaseInspection, DatabaseOptions, GarbageCollectionReport,
    Manifest, Mutation, PageCacheStats, ReadOnlyDatabase, SegmentIoStats, SegmentOpenEvidence,
    Snapshot, SnapshotBundleFile, WriteLockWait, WritePathDiagnostics,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard};
use std::time::Instant;

mod inspection_cursor;
pub use inspection_cursor::{StorageInspectionCursor, StorageInspectionPage};

const RUNTIME_CHECKPOINT_PREFIX: &str = "runtime-";
pub const RRFLOW_KV_OPEN_EVIDENCE_VERSION: u16 = 2;

/// Work performed by startup checkpoint reconciliation after the physical
/// database has opened but before the store is published to callers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RrflowKvReconciliationEvidence {
    pub contract_version: u16,
    pub page_requests: u64,
    pub page_cache_hits: u64,
    pub page_cache_misses: u64,
    pub page_cache_loads: u64,
    pub page_cache_evictions: u64,
    pub page_bytes_read: u64,
    pub page_bytes_decoded: u64,
    pub page_bytes_borrowed: u64,
    pub page_bytes_allocated: u64,
    pub page_bytes_copied: u64,
    pub page_bytes_decompressed: u64,
    pub filter_checks: u64,
    pub filter_negatives: u64,
    pub io_fallbacks: u64,
    pub read_operations: u64,
    pub mmap_read_operations: u64,
    pub io_uring_read_operations: u64,
    pub bounded_read_operations: u64,
    pub bytes_read: u64,
}

impl Default for RrflowKvReconciliationEvidence {
    fn default() -> Self {
        Self {
            contract_version: RRFLOW_KV_OPEN_EVIDENCE_VERSION,
            page_requests: 0,
            page_cache_hits: 0,
            page_cache_misses: 0,
            page_cache_loads: 0,
            page_cache_evictions: 0,
            page_bytes_read: 0,
            page_bytes_decoded: 0,
            page_bytes_borrowed: 0,
            page_bytes_allocated: 0,
            page_bytes_copied: 0,
            page_bytes_decompressed: 0,
            filter_checks: 0,
            filter_negatives: 0,
            io_fallbacks: 0,
            read_operations: 0,
            mmap_read_operations: 0,
            io_uring_read_operations: 0,
            bounded_read_operations: 0,
            bytes_read: 0,
        }
    }
}

/// Immutable evidence for the two physically distinct read phases performed
/// by [`RrflowKvStore::open_with_options`]. Query evidence is owned by each
/// projected read and never inferred from these process-wide counters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RrflowKvOpenEvidence {
    pub contract_version: u16,
    pub created: bool,
    pub segment_validation: SegmentOpenEvidence,
    pub reconciliation: RrflowKvReconciliationEvidence,
}

pub struct RrflowKvStore {
    path: AnchoredPath,
    database: Mutex<Database>,
    open_evidence: RrflowKvOpenEvidence,
    native_write_path_diagnostics_enabled: AtomicBool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RrflowKvInspection {
    pub physical: DatabaseInspection,
    pub claim_sequence: u64,
    pub runtime_cursor: u64,
    pub control_journal_sequence: u64,
}

/// Read-only rrflowKV semantic inspection over an authenticated physical
/// snapshot. It intentionally exposes no storage transaction or repair path.
pub struct RrflowKvInspector {
    database: ReadOnlyDatabase,
    inspection: RrflowKvInspection,
}

struct RrflowKvTransaction<'a> {
    store: &'a RrflowKvStore,
    transaction: rrd_lsm::Transaction,
    begin_mutex_wait_ns: u64,
}

impl StorageTransaction for RrflowKvTransaction<'_> {
    fn snapshot_sequence(&self) -> u64 {
        self.transaction.snapshot().sequence
    }

    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        let database = self.store.lock()?;
        self.transaction.get(&database, key).map_err(Error::from)
    }

    fn scan(&self, start: &[u8], end: &[u8], limit: usize) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        let database = self.store.lock()?;
        self.transaction
            .scan(&database, start, end, limit)
            .map_err(Error::from)
    }

    fn put(&mut self, key: Vec<u8>, value: Vec<u8>) -> Result<()> {
        self.transaction.put(key, value).map_err(Error::from)
    }

    fn delete(&mut self, key: Vec<u8>) -> Result<()> {
        self.transaction.delete(key).map_err(Error::from)
    }

    fn commit(self: Box<Self>, durability: Durability) -> Result<TransactionCommit> {
        let Self {
            store,
            transaction,
            begin_mutex_wait_ns,
        } = *self;
        let diagnostics_enabled = store
            .native_write_path_diagnostics_enabled
            .load(Ordering::Acquire);
        let commit_lock_started = diagnostics_enabled.then(Instant::now);
        let mut database = store.lock()?;
        let commit_mutex_wait_ns = commit_lock_started.map_or(0, |started| {
            u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX)
        });
        let snapshot_prefix = keyspaces::space_prefix(keyspaces::RUNTIME_SNAPSHOTS);
        let removes_snapshot = transaction
            .pending_prefix(&snapshot_prefix)
            .any(|(_, value)| value.is_none());
        let snapshot_handles = transaction
            .pending_prefix(&snapshot_prefix)
            .filter_map(|(_, value)| value)
            .map(|bytes| {
                let handle: SnapshotHandle = serde_json::from_slice(bytes)?;
                handle.validate()?;
                Ok(handle)
            })
            .collect::<Result<Vec<_>>>()?;
        let mut created_checkpoints = Vec::new();
        for handle in &snapshot_handles {
            match ensure_runtime_checkpoint(&mut database, handle, handle.created_at) {
                Ok(true) => created_checkpoints.push(runtime_checkpoint_name(&handle.id)),
                Ok(false) => {}
                Err(error) => {
                    for name in created_checkpoints {
                        let _ = database.release_checkpoint(&name);
                    }
                    return Err(error);
                }
            }
        }
        let physical_durability = match durability {
            Durability::Authoritative => rrd_lsm::Durability::Authoritative,
            Durability::Buffered => rrd_lsm::Durability::Buffered,
        };
        // No untracked physical Snapshot escapes this store. Foreground and
        // idle maintenance use the same registered-reader retention contract.
        let commit = database.commit_transaction_with_protected_snapshots(
            transaction,
            physical_durability,
            &[],
            WriteLockWait {
                begin_mutex_wait_ns,
                commit_mutex_wait_ns,
            },
        );
        let outcome = match commit {
            Ok(outcome) => outcome,
            Err(error) => {
                for name in created_checkpoints {
                    let _ = database.release_checkpoint(&name);
                }
                return Err(error.into());
            }
        };
        if removes_snapshot {
            reconcile_runtime_checkpoints(&mut database, None, 0)?;
        }
        let (first_sequence, last_sequence) = outcome
            .receipt
            .as_ref()
            .map(|receipt| (Some(receipt.first_sequence), Some(receipt.last_sequence)))
            .unwrap_or((None, None));
        tracing::debug!(
            target: "rrd_store::transaction",
            storage_profile = "rrflow_kv",
            snapshot_sequence = outcome.snapshot_sequence,
            mutation_count = outcome.mutation_count,
            first_sequence,
            last_sequence,
            outcome = if outcome.receipt.is_some() { "committed" } else { "read_only" },
            "storage transaction completed"
        );
        Ok(TransactionCommit {
            snapshot_sequence: outcome.snapshot_sequence,
            mutation_count: outcome.mutation_count,
            first_sequence,
            last_sequence,
        })
    }

    fn rollback(self: Box<Self>) -> Result<TransactionRollback> {
        let outcome = self.transaction.rollback()?;
        Ok(TransactionRollback {
            snapshot_sequence: outcome.snapshot_sequence,
            discarded_mutations: outcome.discarded_mutations,
        })
    }
}

/// Read-only projection of the formats owned by the initial rrflowKV writer.
/// Installation can bind these without copying physical-format constants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RrflowKvInitializationFormats {
    pub application_format: u64,
    pub manifest_format: u16,
    pub wal_format: u16,
    pub batch_format: u16,
}

impl RrflowKvStore {
    pub const fn initialization_formats() -> RrflowKvInitializationFormats {
        RrflowKvInitializationFormats {
            application_format: keyspaces::RRFLOW_KV_FORMAT,
            manifest_format: rrd_lsm::MANIFEST_FORMAT_VERSION,
            wal_format: rrd_lsm::WAL_FORMAT_VERSION,
            batch_format: rrd_lsm::BATCH_FORMAT_VERSION,
        }
    }

    /// Creates exactly one new rrflowKV root. Parent-directory creation is an
    /// installer effect and therefore remains outside this constructor.
    pub fn create_new(path: &Path) -> Result<Self> {
        Self::create_new_with_options(path, DatabaseOptions::default())
    }

    pub fn create_new_with_options(path: &Path, options: DatabaseOptions) -> Result<Self> {
        Self::create_new_anchored(&AnchoredPath::from_path(path)?, options)
    }

    pub fn create_new_anchored(path: &AnchoredPath, options: DatabaseOptions) -> Result<Self> {
        let database = Database::create_anchored(path, options, Some(keyspaces::RRFLOW_KV_FORMAT))?;
        Self::from_explicit_database(database, true)
    }

    /// Opens an existing rrflowKV root without creating paths or reconciling
    /// checkpoints. Recovery validates bytes but never repairs a torn tail.
    pub fn open_existing(path: &Path) -> Result<Self> {
        Self::open_existing_with_options(path, DatabaseOptions::default())
    }

    pub fn open_existing_with_options(path: &Path, options: DatabaseOptions) -> Result<Self> {
        Self::open_existing_anchored(&AnchoredPath::open_root(path)?, options)
    }

    pub fn open_existing_anchored(path: &AnchoredPath, options: DatabaseOptions) -> Result<Self> {
        let database = Database::open_anchored(path, options).map_err(|error| {
            if matches!(
                error,
                rrd_lsm::Error::MutableLimit { .. }
                    | rrd_lsm::Error::SegmentHandleLimit { .. }
                    | rrd_lsm::Error::SourcePageLimit { .. }
            ) {
                return Error::from(error);
            }
            Error::Substrate(format!(
                "cannot open existing rrflowKV database {}: {error}",
                path.display()
            ))
        })?;
        Self::from_explicit_database(database, false)
    }

    fn from_explicit_database(database: Database, created: bool) -> Result<Self> {
        let _codec = keyspaces::RrflowKvKeyCodec::from_application_format(
            database.manifest().application_format,
        )
        .ok_or_else(|| {
            Error::Substrate(format!(
                "unsupported rrflowKV application format {:?}",
                database.manifest().application_format
            ))
        })?;
        let path = database.anchored_root().clone();
        Ok(Self {
            path,
            open_evidence: RrflowKvOpenEvidence {
                contract_version: RRFLOW_KV_OPEN_EVIDENCE_VERSION,
                created,
                segment_validation: database.segment_open_evidence().clone(),
                reconciliation: RrflowKvReconciliationEvidence::default(),
            },
            database: Mutex::new(database),
            native_write_path_diagnostics_enabled: AtomicBool::new(false),
        })
    }

    /// Opens an existing rrflowKV database or creates one when `path` is absent.
    /// An existing but invalid directory fails closed rather than being
    /// silently reinitialized.
    pub fn open(path: &(impl crate::filesystem::AnchorInput + ?Sized)) -> Result<Self> {
        Self::open_with_options(path, DatabaseOptions::default())
    }

    /// Opens with explicit rrflowKV cache and mutable-state bounds. Persistent
    /// format identity is unchanged; these are process-local operating limits.
    pub fn open_with_options(
        path: &(impl crate::filesystem::AnchorInput + ?Sized),
        options: DatabaseOptions,
    ) -> Result<Self> {
        let path = &path.anchor()?;
        let total_started = Instant::now();
        if !path.try_exists()? {
            if let Some(parent) = path.parent() {
                parent
                    .create_dir_all()
                    .map_err(|error| Error::Substrate(error.to_string()))?;
            }
        }
        let empty = path.try_exists()?
            && path.metadata()?.is_dir()
            && path
                .entries()
                .map_err(|error| Error::Substrate(error.to_string()))?
                .next()
                .is_none();
        let created = !path.try_exists()? || empty;
        let database_started = Instant::now();
        let mut database = if created {
            Database::create_anchored(path, options, Some(keyspaces::RRFLOW_KV_FORMAT))?
        } else {
            Database::open_anchored(path, options).map_err(|error| {
                if matches!(
                    error,
                    rrd_lsm::Error::MutableLimit { .. }
                        | rrd_lsm::Error::SegmentHandleLimit { .. }
                        | rrd_lsm::Error::SourcePageLimit { .. }
                ) {
                    return Error::from(error);
                }
                Error::Substrate(format!(
                    "cannot open rrflowKV database {}: {error}",
                    path.display()
                ))
            })?
        };
        let database_open_ms = database_started.elapsed().as_millis() as u64;
        let _codec = keyspaces::RrflowKvKeyCodec::from_application_format(
            database.manifest().application_format,
        )
        .ok_or_else(|| {
            Error::Substrate(format!(
                "unsupported rrflowKV application format {:?}",
                database.manifest().application_format
            ))
        })?;
        let segment_validation = database.segment_open_evidence().clone();
        let cache_before = database.page_cache_stats();
        let io_before = database.segment_io_stats();
        let checkpoints_started = Instant::now();
        reconcile_runtime_checkpoints(&mut database, None, 0).map_err(|error| {
            Error::Substrate(format!(
                "cannot reconcile runtime checkpoints while opening {}: {error}",
                path.display()
            ))
        })?;
        let checkpoint_reconcile_ms = checkpoints_started.elapsed().as_millis() as u64;
        let reconciliation = reconciliation_evidence(
            &cache_before,
            &database.page_cache_stats(),
            &io_before,
            &database.segment_io_stats(),
        )?;
        let open_evidence = RrflowKvOpenEvidence {
            contract_version: RRFLOW_KV_OPEN_EVIDENCE_VERSION,
            created,
            segment_validation,
            reconciliation,
        };
        tracing::info!(
            target: "rrflow_kv::open",
            path = %path.display(),
            created,
            database_open_ms,
            checkpoint_reconcile_ms,
            validated_segments = open_evidence.segment_validation.segment_count,
            validation_bytes = open_evidence.segment_validation.full_checksum_bytes,
            persisted_filter_count = open_evidence.segment_validation.persisted_filter_count,
            persisted_filter_bytes = open_evidence.segment_validation.persisted_filter_bytes,
            none_policy_segments = open_evidence.segment_validation.none_policy_segment_count,
            adaptive_lz4_policy_segments = open_evidence
                .segment_validation
                .adaptive_lz4_policy_segment_count,
            raw_pages = open_evidence.segment_validation.raw_page_count,
            compressed_pages = open_evidence.segment_validation.compressed_page_count,
            stored_page_bytes = open_evidence.segment_validation.stored_page_bytes,
            logical_page_bytes = open_evidence.segment_validation.logical_page_bytes,
            reconciliation_page_requests = open_evidence.reconciliation.page_requests,
            reconciliation_read_operations = open_evidence.reconciliation.read_operations,
            reconciliation_bytes_read = open_evidence.reconciliation.bytes_read,
            total_ms = total_started.elapsed().as_millis() as u64,
            "rrflowKV store open phases completed"
        );
        let path = database.anchored_root().clone();
        Ok(Self {
            path,
            database: Mutex::new(database),
            open_evidence,
            native_write_path_diagnostics_enabled: AtomicBool::new(false),
        })
    }

    pub fn path(&self) -> &Path {
        self.path.diagnostic_path()
    }

    pub fn anchored_path(&self) -> &AnchoredPath {
        &self.path
    }

    pub fn open_evidence(&self) -> &RrflowKvOpenEvidence {
        &self.open_evidence
    }

    /// Enables bounded native write-path diagnostics for subsequent commits.
    /// This process-local evidence is disabled by default and is never stored.
    pub fn enable_native_write_path_diagnostics(&self, capacity: usize) -> Result<()> {
        self.lock()?.enable_write_path_diagnostics(capacity)?;
        self.native_write_path_diagnostics_enabled
            .store(true, Ordering::Release);
        Ok(())
    }

    /// Disables and drains the current native write-path diagnostic buffer.
    pub fn take_native_write_path_diagnostics(&self) -> Result<Option<WritePathDiagnostics>> {
        self.native_write_path_diagnostics_enabled
            .store(false, Ordering::Release);
        Ok(self.lock()?.take_write_path_diagnostics())
    }

    /// Publishes the current rrflowKV memtable as an immutable segment.
    pub fn flush(&self, at: Millis) -> Result<Option<Manifest>> {
        self.lock()?.flush_memtable(at).map_err(Error::from)
    }

    /// Offer one physical maintenance quantum without queuing behind a reader
    /// or writer. Busy is a skipped offer, not a second worker or a retry loop.
    pub fn try_maintenance_step(&self) -> Result<bool> {
        match self.database.try_lock() {
            // The private database never exposes an untracked Snapshot.
            // Transactions own registered leases; inspection streams and
            // runtime checkpoints retain physical generations independently.
            // Logical versioned keys are data, not superseded MVCC versions.
            Ok(mut database) => database
                .maintenance_step_with_protected_snapshots(&[])
                .map_err(Error::from),
            Err(std::sync::TryLockError::WouldBlock) => Ok(false),
            Err(std::sync::TryLockError::Poisoned(_)) => {
                Err(Error::Substrate("rrflowKV database lock poisoned".into()))
            }
        }
    }

    pub fn manifest(&self) -> Result<Manifest> {
        Ok(self.lock()?.manifest().clone())
    }

    /// Compacts rrflowKV state after reconciling physical manifest pins with the
    /// authoritative logical snapshot catalog.
    pub fn compact(&self, now: Millis, at: Millis) -> Result<Option<CompactionOutcome>> {
        let mut database = self.lock()?;
        reconcile_runtime_checkpoints(&mut database, Some(now), at)?;
        database.compact(&[], at).map_err(Error::from)
    }

    /// Reclaims only objects unreachable from `CURRENT` or a live runtime
    /// snapshot's physical checkpoint.
    pub fn garbage_collect(&self, now: Millis, at: Millis) -> Result<GarbageCollectionReport> {
        let mut database = self.lock()?;
        reconcile_runtime_checkpoints(&mut database, Some(now), at)?;
        database.garbage_collect().map_err(Error::from)
    }

    /// Replays one already-authenticated logical-archive commit while
    /// preserving its original audit envelope. This is deliberately crate
    /// private: ordinary callers must use the live `StorageEngine` transaction
    /// path, which validates a read stamp against the current database state.
    pub(crate) fn restore_runtime_commit(
        &self,
        commit: &RuntimeCommit,
        audit: &AuditEnvelope,
    ) -> Result<RuntimeCommitOutcome> {
        self.runtime().restore_commit(commit, audit)
    }

    fn lock(&self) -> Result<MutexGuard<'_, Database>> {
        self.database
            .lock()
            .map_err(|_| Error::Substrate("rrflowKV database mutex poisoned".into()))
    }
}

impl RrflowKvInspector {
    pub fn open(path: &(impl crate::filesystem::AnchorInput + ?Sized)) -> Result<Self> {
        Self::open_with_options(path, DatabaseOptions::default())
    }

    /// Inspect existing bytes using the same native replay limits as writer
    /// open. This neither repairs storage nor changes persisted configuration.
    pub fn open_with_options(
        path: &(impl crate::filesystem::AnchorInput + ?Sized),
        options: DatabaseOptions,
    ) -> Result<Self> {
        Self::open_anchored_with_options(&path.anchor()?, options)
    }

    pub fn open_anchored(path: &AnchoredPath) -> Result<Self> {
        Self::open_anchored_with_options(path, DatabaseOptions::default())
    }

    pub fn open_anchored_with_options(
        path: &AnchoredPath,
        options: DatabaseOptions,
    ) -> Result<Self> {
        let database =
            ReadOnlyDatabase::open_anchored_with_options(path, options).map_err(Error::from)?;
        let inspection = inspect_database(&database)?;
        Ok(Self {
            database,
            inspection,
        })
    }

    pub fn inspection(&self) -> &RrflowKvInspection {
        &self.inspection
    }

    pub fn control_record(&self, key: &str) -> Result<Option<Vec<u8>>> {
        crate::control::validate_control_key(key)?;
        let logical = keyspaces::control_record_key(key);
        let physical = crate::access::runtime_state::checked_key(keyspaces::META, &logical)?;
        self.database.get(&physical).map_err(Error::from)
    }

    /// Reconstructs the canonical semantic read identity from the same
    /// authenticated, read-only physical snapshot used by this inspector.
    pub fn runtime_read_stamp(&self, scope: &ScopeId) -> Result<ReadStamp> {
        crate::access::runtime_state::read_stamp_with(&self.database, scope)
    }
}

impl crate::access::runtime_state::AccessRead for ReadOnlyDatabase {
    fn read_key(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        self.get(key).map_err(Error::from)
    }

    fn scan_range(
        &self,
        start: &[u8],
        end: &[u8],
        limit: usize,
    ) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        let mut rows = self.scan(start, Some(end)).map_err(Error::from)?;
        rows.truncate(limit);
        Ok(rows)
    }
}

fn inspect_database(database: &ReadOnlyDatabase) -> Result<RrflowKvInspection> {
    if keyspaces::RrflowKvKeyCodec::from_application_format(database.manifest().application_format)
        .is_none()
    {
        return Err(Error::Substrate(format!(
            "unsupported rrflowKV application format {:?}",
            database.manifest().application_format
        )));
    }
    let claim_sequence = inspect_sequence(database, &keyspaces::sequence_watermark_key())?;
    let runtime_cursor = inspect_sequence(database, &keyspaces::runtime_cursor_key())?;
    let control_journal_sequence =
        inspect_sequence(database, &keyspaces::control_journal_sequence_key())?;
    let inspection = RrflowKvInspection {
        physical: database.inspection().clone(),
        claim_sequence,
        runtime_cursor,
        control_journal_sequence,
    };
    Ok(inspection)
}

/// Explicit offline repair owner; ordinary inspectors retain no mutation API.
pub struct RrflowKvTailRepair {
    repair: rrd_lsm::TailRepair,
    inspection: RrflowKvInspection,
}
impl RrflowKvTailRepair {
    pub fn open_anchored(path: &AnchoredPath, options: DatabaseOptions) -> Result<Self> {
        let repair = rrd_lsm::TailRepair::open_anchored(path, options)?;
        let inspection = inspect_database(repair.reader())?;
        Ok(Self { repair, inspection })
    }
    pub fn inspection(&self) -> &RrflowKvInspection {
        &self.inspection
    }
    pub fn control_record(&self, key: &str) -> Result<Option<Vec<u8>>> {
        crate::control::validate_control_key(key)?;
        let logical = keyspaces::control_record_key(key);
        let physical = crate::access::runtime_state::checked_key(keyspaces::META, &logical)?;
        self.repair.reader().get(&physical).map_err(Error::from)
    }
    pub fn plan(&self) -> Result<rrd_lsm::TailRepairPlan> {
        self.repair.plan().map_err(Error::from)
    }
    pub fn apply(
        self,
        plan: &rrd_lsm::TailRepairPlan,
        expect: &str,
    ) -> Result<rrd_lsm::TailRepairReceipt> {
        self.repair.apply(plan, expect).map_err(Error::from)
    }
}

fn inspect_sequence(database: &ReadOnlyDatabase, key: &[u8]) -> Result<u64> {
    let physical = crate::access::runtime_state::checked_key(keyspaces::META, key)?;
    database
        .get(&physical)
        .map_err(Error::from)?
        .as_deref()
        .map(crate::access::runtime_state::decode_sequence)
        .transpose()
        .map(Option::unwrap_or_default)
}

fn reconciliation_evidence(
    cache_before: &PageCacheStats,
    cache_after: &PageCacheStats,
    io_before: &SegmentIoStats,
    io_after: &SegmentIoStats,
) -> Result<RrflowKvReconciliationEvidence> {
    if cache_before.capacity_bytes != cache_after.capacity_bytes
        || io_before.requested_mode != io_after.requested_mode
        || io_before.configured_max_request_bytes != io_after.configured_max_request_bytes
        || io_before.configured_max_open_files != io_after.configured_max_open_files
        || io_before.mmap_segments != io_after.mmap_segments
        || io_before.io_uring_segments != io_after.io_uring_segments
        || io_before.bounded_segments != io_after.bounded_segments
    {
        return Err(Error::Substrate(
            "rrflowKV open counters changed outside startup reconciliation".into(),
        ));
    }
    let delta = |before: u64, after: u64, name: &str| {
        after.checked_sub(before).ok_or_else(|| {
            Error::Substrate(format!(
                "rrflowKV startup reconciliation counter regressed: {name}"
            ))
        })
    };
    let page_cache_hits = delta(cache_before.hits, cache_after.hits, "cache hits")?;
    let page_cache_misses = delta(cache_before.misses, cache_after.misses, "cache misses")?;
    let page_requests = page_cache_hits
        .checked_add(page_cache_misses)
        .ok_or_else(|| {
            Error::Substrate("rrflowKV startup reconciliation page requests overflowed".into())
        })?;
    let read_operations = delta(
        io_before.read_operations,
        io_after.read_operations,
        "read operations",
    )?;
    let mmap_read_operations = delta(
        io_before.mmap_read_operations,
        io_after.mmap_read_operations,
        "mmap read operations",
    )?;
    let io_uring_read_operations = delta(
        io_before.io_uring_read_operations,
        io_after.io_uring_read_operations,
        "io_uring read operations",
    )?;
    let bounded_read_operations = delta(
        io_before.bounded_read_operations,
        io_after.bounded_read_operations,
        "bounded read operations",
    )?;
    let classified_reads = mmap_read_operations
        .checked_add(io_uring_read_operations)
        .and_then(|value| value.checked_add(bounded_read_operations))
        .ok_or_else(|| {
            Error::Substrate("rrflowKV startup reconciliation read classes overflowed".into())
        })?;
    if classified_reads != read_operations {
        return Err(Error::Substrate(
            "rrflowKV startup reconciliation read classes do not match total reads".into(),
        ));
    }
    let evidence = RrflowKvReconciliationEvidence {
        contract_version: RRFLOW_KV_OPEN_EVIDENCE_VERSION,
        page_requests,
        page_cache_hits,
        page_cache_misses,
        page_cache_loads: delta(cache_before.loads, cache_after.loads, "cache loads")?,
        page_cache_evictions: delta(
            cache_before.evictions,
            cache_after.evictions,
            "cache evictions",
        )?,
        page_bytes_read: delta(
            cache_before.bytes_read,
            cache_after.bytes_read,
            "page bytes read",
        )?,
        page_bytes_decoded: delta(
            cache_before.bytes_decoded,
            cache_after.bytes_decoded,
            "page bytes decoded",
        )?,
        page_bytes_borrowed: delta(
            cache_before.bytes_borrowed,
            cache_after.bytes_borrowed,
            "page bytes borrowed",
        )?,
        page_bytes_allocated: delta(
            cache_before.bytes_allocated,
            cache_after.bytes_allocated,
            "page bytes allocated",
        )?,
        page_bytes_copied: delta(
            cache_before.bytes_copied,
            cache_after.bytes_copied,
            "page bytes copied",
        )?,
        page_bytes_decompressed: delta(
            cache_before.bytes_decompressed,
            cache_after.bytes_decompressed,
            "page bytes decompressed",
        )?,
        filter_checks: delta(
            cache_before.filter_checks,
            cache_after.filter_checks,
            "filter checks",
        )?,
        filter_negatives: delta(
            cache_before.filter_negatives,
            cache_after.filter_negatives,
            "filter negatives",
        )?,
        io_fallbacks: delta(
            io_before.fallback_count,
            io_after.fallback_count,
            "I/O fallbacks",
        )?,
        read_operations,
        mmap_read_operations,
        io_uring_read_operations,
        bounded_read_operations,
        bytes_read: delta(io_before.bytes_read, io_after.bytes_read, "I/O bytes read")?,
    };
    if evidence.page_cache_loads != evidence.read_operations
        || evidence.page_bytes_read != evidence.bytes_read
    {
        return Err(Error::Substrate(
            "rrflowKV startup reconciliation page loads do not match physical reads".into(),
        ));
    }
    Ok(evidence)
}

impl StorageEngine for RrflowKvStore {
    fn begin_transaction(&self) -> Result<Box<dyn StorageTransaction + '_>> {
        let diagnostics_enabled = self
            .native_write_path_diagnostics_enabled
            .load(Ordering::Acquire);
        let begin_lock_started = diagnostics_enabled.then(Instant::now);
        let database = self.lock()?;
        let begin_mutex_wait_ns = begin_lock_started.map_or(0, |started| {
            u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX)
        });
        let transaction = database.begin_transaction()?;
        drop(database);
        tracing::debug!(
            target: "rrd_store::transaction",
            storage_profile = "rrflow_kv",
            snapshot_sequence = transaction.snapshot().sequence,
            outcome = "begun",
            "storage transaction began"
        );
        Ok(Box::new(RrflowKvTransaction {
            store: self,
            transaction,
            begin_mutex_wait_ns,
        }))
    }

    fn claims(&self) -> crate::ClaimRepository<'_> {
        crate::ClaimRepository::new(self)
    }

    fn control(&self) -> crate::ControlRepository<'_> {
        crate::ControlRepository::new(self)
    }

    fn function_catalogue(&self) -> crate::FunctionCatalogueRepository<'_> {
        crate::FunctionCatalogueRepository::new(self)
    }

    fn projections(&self) -> crate::ProjectionRepository<'_> {
        crate::ProjectionRepository::new(self)
    }

    fn runtime(&self) -> crate::RuntimeRepository<'_> {
        crate::RuntimeRepository::new(self)
    }

    fn invocations(&self) -> crate::InvocationRepository<'_> {
        crate::InvocationRepository::new(self)
    }

    fn physical_store_evidence(&self) -> Result<PhysicalStoreEvidence> {
        let database = self.lock()?;
        let manifest = database.manifest();
        let cache = database.page_cache_stats();
        let source_pages = database.source_page_memory().stats();
        let segment_io = database.segment_io_stats();
        let maintenance = database.maintenance_policy();
        let compaction = database.compaction_policy();
        let maintenance_stats = database.maintenance_stats();
        let memtable = database.memtable().profile();
        let (checkpoint_entries, checkpoint_limit) = database.checkpoint_ownership()?;
        let (native_transactions, native_transaction_bytes) = database.transaction_ownership()?;
        let (active_read_views, pinned_read_bytes) = database.read_view_ownership()?;
        Ok(PhysicalStoreEvidence {
            backend: "rrflow_kv".into(),
            evidence_level: RRFLOW_KV_COUNTER_EVIDENCE.into(),
            physical_sequence: Some(database.snapshot().sequence),
            manifest_generation: Some(manifest.generation),
            durable_sequence: Some(manifest.durable_sequence),
            checkpoint_entries: Some(checkpoint_entries),
            checkpoint_limit: Some(checkpoint_limit),
            native_transactions: Some(native_transactions),
            native_transaction_bytes: Some(native_transaction_bytes),
            active_read_views: Some(active_read_views),
            pinned_read_bytes: Some(pinned_read_bytes),
            retained_mutable_bytes: Some(database.mutable_memory().stats().accounted_bytes as u64),
            retained_source_bytes: Some(source_pages.reserved_bytes as u64),
            peak_source_bytes: Some(source_pages.peak_reserved_bytes as u64),
            denied_source_allocations: Some(source_pages.denied_allocations),
            retained_projected_bytes: Some(
                database.projected_output_memory().stats().reserved_bytes as u64,
            ),
            memtable_versions: Some(database.memtable().version_count() as u64),
            memtable_bytes: Some(database.memtable().approximate_bytes() as u64),
            memtable_keys: Some(memtable.key_count as u64),
            memtable_key_payload_bytes: Some(memtable.key_payload_bytes as u64),
            memtable_value_payload_bytes: Some(memtable.value_payload_bytes as u64),
            memtable_tombstones: Some(memtable.tombstones as u64),
            memtable_spilled_chains: Some(memtable.spilled_chains as u64),
            memtable_spilled_version_capacity: Some(memtable.spilled_version_capacity as u64),
            memtable_version_record_bytes: Some(memtable.version_record_bytes as u64),
            memtable_owned_bytes_lower_bound: Some(memtable.owned_bytes_lower_bound as u64),
            memtable_max_versions: Some(maintenance.memtable_max_versions as u64),
            wal_payload_bytes: Some(database.wal_payload_bytes() as u64),
            wal_payload_max_bytes: Some(maintenance.wal_payload_max_bytes as u64),
            automatic_flushes: Some(maintenance_stats.automatic_flushes),
            maintenance_write_stalls: Some(maintenance_stats.write_stalls),
            failed_maintenance_flushes: Some(maintenance_stats.failed_flushes),
            oversized_batches: Some(maintenance_stats.oversized_batches),
            automatic_compactions: Some(maintenance_stats.automatic_compactions),
            failed_compactions: Some(maintenance_stats.failed_compactions),
            compaction_input_bytes: Some(maintenance_stats.compaction_input_bytes),
            compaction_output_bytes: Some(maintenance_stats.compaction_output_bytes),
            peak_compaction_buffer_bytes: Some(
                maintenance_stats.peak_compaction_buffer_bytes as u64,
            ),
            l0_segment_count: Some(database.l0_segment_count() as u64),
            l0_compaction_trigger: Some(compaction.l0_compaction_trigger as u64),
            compaction_debt_segments: Some(database.compaction_debt_segments() as u64),
            compaction_target_segment_bytes: Some(compaction.target_segment_bytes as u64),
            segment_count: Some(manifest.segments.len() as u64),
            segment_bytes: Some(manifest.segments.iter().map(|segment| segment.bytes).sum()),
            cache_capacity_bytes: Some(cache.capacity_bytes as u64),
            cache_resident_bytes: Some(cache.resident_bytes as u64),
            cache_entries: Some(cache.entries as u64),
            cache_hits: Some(cache.hits),
            cache_misses: Some(cache.misses),
            cache_evictions: Some(cache.evictions),
            page_loads: Some(cache.loads),
            page_bytes_read: Some(cache.bytes_read),
            page_bytes_decoded: Some(cache.bytes_decoded),
            page_bytes_borrowed: Some(cache.bytes_borrowed),
            page_bytes_allocated: Some(cache.bytes_allocated),
            page_bytes_copied: Some(cache.bytes_copied),
            page_bytes_decompressed: Some(cache.bytes_decompressed),
            filter_checks: Some(cache.filter_checks),
            filter_negatives: Some(cache.filter_negatives),
            segment_io_requested_mode: Some(segment_io.requested_mode.as_str().into()),
            segment_io_mmap_segments: Some(segment_io.mmap_segments),
            segment_io_uring_segments: Some(segment_io.io_uring_segments),
            segment_io_bounded_segments: Some(segment_io.bounded_segments),
            segment_io_fallbacks: Some(segment_io.fallback_count),
            segment_io_last_fallback: segment_io.last_fallback_reason,
            segment_io_read_operations: Some(segment_io.read_operations),
            segment_io_mmap_reads: Some(segment_io.mmap_read_operations),
            segment_io_uring_reads: Some(segment_io.io_uring_read_operations),
            segment_io_bounded_reads: Some(segment_io.bounded_read_operations),
            segment_io_bytes_read: Some(segment_io.bytes_read),
            segment_io_max_request_bytes: Some(segment_io.configured_max_request_bytes as u64),
            segment_io_peak_request_bytes: Some(segment_io.peak_request_bytes as u64),
            segment_io_max_open_files: Some(segment_io.configured_max_open_files as u64),
            segment_io_open_files: Some(segment_io.open_files as u64),
            segment_io_peak_open_files: Some(segment_io.peak_open_files as u64),
            segment_io_handle_evictions: Some(segment_io.handle_evictions),
            segment_io_handle_reopens: Some(segment_io.handle_reopens),
            segment_io_handle_denials: Some(segment_io.handle_denials),
        })
    }
}

/// Converts the one profile-neutral encoded semantic plan into the complete
/// rrflowKV mutation vector used by the temporary Raft application bridge.
///
/// The bridge appends coordinator metadata to this complete vector. Its raw
/// access remains an inventoried distributed-authority gap; centralizing the
/// semantic plan here removes the second planner but does not close that gap.
impl SemanticCommitPlan {
    pub fn into_parts_for(
        self,
        database: &Database,
    ) -> Result<(RuntimeCommitOutcome, Vec<Mutation>)> {
        database_codec(database)?;
        let (outcome, writes) = self.into_encoded_writes();
        let operations = writes
            .into_iter()
            .map(|(key, value)| match value {
                Some(value) => Mutation::Put { key, value },
                None => Mutation::Delete { key },
            })
            .collect();
        Ok((outcome, operations))
    }
}

struct RrflowKvSnapshotRead<'a> {
    database: &'a Database,
    snapshot: Snapshot,
}

impl crate::access::runtime_state::AccessRead for RrflowKvSnapshotRead<'_> {
    fn read_key(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        self.database.get(key, self.snapshot).map_err(Error::from)
    }

    fn scan_range(
        &self,
        start: &[u8],
        end: &[u8],
        limit: usize,
    ) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        Ok(self
            .database
            .scan(start, Some(end), self.snapshot)
            .map_err(Error::from)?
            .into_iter()
            .take(limit)
            .collect())
    }
}

/// Creates an empty database in the rrflowKV application format for an owner
/// that applies rrflowKV commits through the `&Database` functions below,
/// such as the Raft application bridge's canonical state.
pub fn create_rrflow_kv_database(
    root: &(impl crate::filesystem::AnchorInput + ?Sized),
) -> Result<Database> {
    Ok(Database::create_anchored(
        &root.anchor()?,
        DatabaseOptions::default(),
        Some(keyspaces::RRFLOW_KV_FORMAT),
    )?)
}

/// Whether `database` carries the rrflowKV application format that the
/// `&Database` functions below require.
pub fn is_rrflow_kv_database(database: &Database) -> bool {
    database_codec(database).is_ok()
}

/// Reads the exact rrflowKV cursor/schema pair used by the temporary Raft
/// application bridge before it submits the complete semantic plan.
pub fn rrflow_kv_commit_context(
    database: &Database,
    scope: &ScopeId,
) -> Result<(ReadStamp, Option<RuntimeSchemaRegistry>)> {
    let reader = RrflowKvSnapshotRead {
        database,
        snapshot: database.snapshot(),
    };
    let read = crate::access::runtime_state::read_stamp_with(&reader, scope)?;
    let schema = crate::access::runtime_state::get_json(
        &reader,
        keyspaces::RUNTIME_SCHEMAS,
        &keyspaces::runtime_schema_key(scope),
    )?;
    if schema
        .as_ref()
        .map(|value: &RuntimeSchemaRegistry| value.revision)
        != read.schema_revision
    {
        return Err(Error::Substrate(
            "rrflowKV runtime schema differs from its read stamp".into(),
        ));
    }
    Ok((read, schema))
}

/// Validates and lowers one canonical [`RuntimeCommit`] into the same encoded
/// semantic plan used by rrflowMX and ordinary rrflowKV repositories. This
/// remains public only for the already-inventoried Raft application path.
pub fn prepare_rrflow_kv_commit(
    database: &Database,
    commit: &RuntimeCommit,
) -> Result<SemanticCommitPlan> {
    prepare_rrflow_kv_commit_at_read(database, commit, None, None, None)
}

fn prepare_rrflow_kv_commit_at_read(
    database: &Database,
    commit: &RuntimeCommit,
    read: Option<&ReadStamp>,
    archived_audit: Option<&AuditEnvelope>,
    function_receipts: Option<(&str, &[crate::FunctionInvocationReceiptRecord])>,
) -> Result<SemanticCommitPlan> {
    let reader = RrflowKvSnapshotRead {
        database,
        snapshot: database.snapshot(),
    };
    prepare_semantic_commit(&reader, commit, read, archived_audit, function_receipts)
}

/// Reads a previously accepted rrflowKV runtime outcome from a caller-held
/// database snapshot. Coordinators use this before planning so content-addressed
/// retries remain idempotent even when the transport request id changes.
pub fn rrflow_kv_commit_outcome(
    database: &Database,
    commit_id: &str,
) -> Result<Option<RuntimeCommitOutcome>> {
    let outcome: Option<RuntimeCommitOutcome> = get_json(
        database,
        database.snapshot(),
        keyspaces::RUNTIME_COMMITS,
        &keyspaces::runtime_commit_key(commit_id),
    )?;
    if outcome
        .as_ref()
        .is_some_and(|outcome| outcome.commit_id != commit_id)
    {
        return Err(Error::Substrate(
            "runtime commit outcome key does not match its content identity".into(),
        ));
    }
    Ok(outcome)
}

fn reconcile_runtime_checkpoints(
    database: &mut Database,
    now: Option<Millis>,
    at: Millis,
) -> Result<()> {
    let snapshot = database.snapshot();
    let handles = scan_space(database, snapshot, keyspaces::RUNTIME_SNAPSHOTS, &[])?
        .into_iter()
        .map(|(_, bytes)| serde_json::from_slice::<SnapshotHandle>(&bytes).map_err(Error::from))
        .collect::<Result<Vec<_>>>()?;
    for handle in &handles {
        handle.validate()?;
    }
    let desired = handles
        .iter()
        .filter(|handle| now.is_none_or(|now| !handle.is_expired(now)))
        .map(|handle| runtime_checkpoint_name(&handle.id))
        .collect::<BTreeSet<_>>();
    let checkpoints = database.checkpoints()?;
    let existing = checkpoints
        .iter()
        .filter(|checkpoint| checkpoint.name.starts_with(RUNTIME_CHECKPOINT_PREFIX))
        .map(|checkpoint| checkpoint.name.clone())
        .collect::<BTreeSet<_>>();
    // Canonical live handles decide reachability. Release only obsolete runtime
    // pins before admitting missing pins so the quota cannot block reconciliation.
    for name in existing.difference(&desired) {
        database.release_checkpoint(name)?;
    }
    let missing = desired.difference(&existing).cloned().collect::<Vec<_>>();
    if !missing.is_empty() {
        let created_at = at.max(database.manifest().created_at);
        database.flush_memtable(created_at)?;
        for name in missing {
            database.checkpoint(&name, created_at)?;
        }
    }
    Ok(())
}

/// Materializes the physical manifest pin before its logical snapshot handle
/// can commit. A failed semantic transaction removes checkpoints it created;
/// an interrupted pre-publication pin is reclaimed by open-time reconciliation.
fn ensure_runtime_checkpoint(
    database: &mut Database,
    handle: &SnapshotHandle,
    at: Millis,
) -> Result<bool> {
    let name = runtime_checkpoint_name(&handle.id);
    if database
        .checkpoints()?
        .iter()
        .any(|checkpoint| checkpoint.name == name)
    {
        return Ok(false);
    }
    let created_at = at.max(database.manifest().created_at);
    database.flush_memtable(created_at)?;
    database.checkpoint(&name, created_at)?;
    Ok(true)
}

fn runtime_checkpoint_name(id: &SnapshotId) -> String {
    format!("{RUNTIME_CHECKPOINT_PREFIX}{}", id.as_str())
}

fn rrflow_kv_read_stamp(
    database: &Database,
    snapshot: Snapshot,
    scope: &ScopeId,
) -> Result<ReadStamp> {
    crate::access::runtime_state::read_stamp_with(
        &RrflowKvSnapshotRead { database, snapshot },
        scope,
    )
}

/// Reads the exact immutable-object closure for one scope from an authenticated
/// physical snapshot before that snapshot is installed on a replica.
pub fn rrflow_kv_snapshot_object_references(
    bundle: &SnapshotBundleFile,
    scope: &ScopeId,
) -> Result<Vec<ObjectReference>> {
    rrflow_kv_snapshot_artifact_view(bundle, scope).map(|(_, objects)| objects)
}

/// Reads the exact project read stamp and immutable-object closure directly
/// from an authenticated physical snapshot.
pub fn rrflow_kv_snapshot_artifact_view(
    bundle: &SnapshotBundleFile,
    scope: &ScopeId,
) -> Result<(ReadStamp, Vec<ObjectReference>)> {
    snapshot_codec(bundle)?;
    let cursor_key = keyspaces::runtime_cursor_key();
    let digest_key = keyspaces::runtime_last_digest_key();
    let accumulator_key = keyspaces::runtime_accumulator_state_key();
    let schema_key = keyspaces::runtime_schema_key(scope);
    let values = bundle
        .get_many(&[&cursor_key, &digest_key, &accumulator_key, &schema_key])
        .map_err(Error::from)?;
    let commit_cursor = values[0]
        .as_deref()
        .map(decode_sequence)
        .transpose()?
        .unwrap_or_default();
    let head_digest = values[1]
        .clone()
        .map(String::from_utf8)
        .transpose()
        .map_err(|error| Error::CorruptWatermark(error.to_string()))?
        .filter(|digest| !digest.is_empty());
    let accumulator = values[2]
        .as_deref()
        .map(serde_json::from_slice::<RuntimeLogAccumulator>)
        .transpose()?;
    if let Some(accumulator) = &accumulator {
        accumulator.validate()?;
        if accumulator.tree_size != commit_cursor {
            return Err(Error::Substrate(format!(
                "snapshot runtime accumulator size {} differs from cursor {commit_cursor}",
                accumulator.tree_size
            )));
        }
    }
    let schema_revision = values[3]
        .as_deref()
        .map(serde_json::from_slice::<RuntimeSchemaRegistry>)
        .transpose()?
        .map(|schema| schema.revision);
    let read = match accumulator {
        Some(accumulator) => ReadStamp::authenticated(
            scope.clone(),
            schema_revision,
            0,
            commit_cursor,
            head_digest,
            accumulator.root,
        )?,
        None if commit_cursor == 0 => ReadStamp::authenticated(
            scope.clone(),
            schema_revision,
            0,
            commit_cursor,
            head_digest,
            RuntimeLogAccumulator::new().root,
        )?,
        None => ReadStamp::new(
            scope.clone(),
            schema_revision,
            0,
            commit_cursor,
            head_digest,
        )?,
    };
    let objects = rrflow_kv_snapshot_objects(bundle, Some(scope))?;
    Ok((read, objects))
}

/// Reads the project artifact view from a live rrflowKV database snapshot. This
/// is used by the cluster adapter without reopening a second database handle.
pub fn rrflow_kv_database_artifact_view(
    database: &Database,
    scope: &ScopeId,
) -> Result<(ReadStamp, Vec<ObjectReference>)> {
    let snapshot = database.snapshot();
    let read = rrflow_kv_read_stamp(database, snapshot, scope)?;
    let rows = scan_space(database, snapshot, keyspaces::RUNTIME_OBJECTS, &[])?;
    let objects = decode_snapshot_objects(database_codec(database)?, rows, Some(scope))?;
    Ok((read, objects))
}

/// Reads every immutable reference in a physical snapshot. Unlike the
/// project-specific transfer view, this permits multiple scopes and is the
/// final target-side activation gate.
pub fn rrflow_kv_snapshot_all_object_references(
    bundle: &SnapshotBundleFile,
) -> Result<Vec<ObjectReference>> {
    rrflow_kv_snapshot_objects(bundle, None)
}

fn rrflow_kv_snapshot_objects(
    bundle: &SnapshotBundleFile,
    required_scope: Option<&ScopeId>,
) -> Result<Vec<ObjectReference>> {
    let codec = snapshot_codec(bundle)?;
    let start = keyspaces::space_prefix(keyspaces::RUNTIME_OBJECTS);
    let end = prefix_end(&start);
    let values = bundle.scan(&start, end.as_deref()).map_err(Error::from)?;
    decode_snapshot_objects(codec, values, required_scope)
}

fn decode_snapshot_objects(
    codec: keyspaces::RrflowKvKeyCodec,
    values: Vec<(Vec<u8>, Vec<u8>)>,
    required_scope: Option<&ScopeId>,
) -> Result<Vec<ObjectReference>> {
    if values.len() > 1_000_000 {
        return Err(Error::Substrate(
            "rrflowKV snapshot object-reference limit exceeded".into(),
        ));
    }
    let mut objects = values
        .into_iter()
        .map(|(stored_key, value)| {
            let object: ObjectReference = serde_json::from_slice(&value)?;
            object.validate()?;
            let (encoded_scope, encoded_reference) = keyspaces::parse_runtime_identity_key(
                codec,
                keyspaces::RUNTIME_OBJECTS,
                &stored_key,
            )?;
            if required_scope.is_some_and(|scope| scope != &encoded_scope) {
                return Err(Error::Substrate(
                    "rrflowKV snapshot object project scope differs from the transfer".into(),
                ));
            }
            let expected = keyspaces::runtime_identity_key(
                keyspaces::RUNTIME_OBJECTS,
                &encoded_scope,
                &object.reference,
            );
            if stored_key != expected || encoded_reference != object.reference {
                return Err(Error::Substrate(
                    "rrflowKV snapshot object key/value identity differs from its canonical reference"
                        .into(),
                ));
            }
            Ok(object)
        })
        .collect::<Result<Vec<_>>>()?;
    objects.sort_by(|left, right| left.reference.cmp(&right.reference));
    if required_scope.is_some()
        && objects
            .windows(2)
            .any(|pair| pair[0].reference >= pair[1].reference)
    {
        return Err(Error::Substrate(
            "rrflowKV snapshot contains duplicate object references".into(),
        ));
    }
    Ok(objects)
}

#[cfg(test)]
fn read_sequence(database: &Database, snapshot: Snapshot, key: &[u8]) -> Result<u64> {
    get(database, snapshot, keyspaces::META, key)?
        .as_deref()
        .map(decode_sequence)
        .transpose()
        .map(Option::unwrap_or_default)
}

fn decode_sequence(value: &[u8]) -> Result<u64> {
    std::str::from_utf8(value)
        .map_err(|error| Error::CorruptWatermark(error.to_string()))?
        .parse::<u64>()
        .map_err(|error| Error::CorruptWatermark(error.to_string()))
}

fn get(
    database: &Database,
    snapshot: Snapshot,
    space: keyspaces::Space,
    key: &[u8],
) -> Result<Option<Vec<u8>>> {
    database
        .get(&encoded_storage_key(database, space, key)?, snapshot)
        .map_err(Error::from)
}

fn get_json<T: DeserializeOwned>(
    database: &Database,
    snapshot: Snapshot,
    space: keyspaces::Space,
    key: &[u8],
) -> Result<Option<T>> {
    get(database, snapshot, space, key)?
        .map(|bytes| serde_json::from_slice(&bytes).map_err(Error::from))
        .transpose()
}

fn scan_space(
    database: &Database,
    snapshot: Snapshot,
    space: keyspaces::Space,
    prefix: &[u8],
) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
    let start = if prefix.is_empty() {
        database_codec(database)?;
        keyspaces::space_prefix(space)
    } else {
        encoded_storage_key(database, space, prefix)?
    };
    let end = prefix_end(&start);
    database
        .scan(&start, end.as_deref(), snapshot)
        .map_err(Error::from)
}

#[cfg(test)]
fn storage_key(space: keyspaces::Space, key: &[u8]) -> Vec<u8> {
    keyspaces::validate_space(KeyCodec, space, key)
        .expect("rrflowKV mutations use a canonical typed key");
    key.to_vec()
}

fn database_codec(database: &Database) -> Result<keyspaces::RrflowKvKeyCodec> {
    keyspaces::RrflowKvKeyCodec::from_application_format(database.manifest().application_format)
        .ok_or_else(|| {
            Error::Substrate(format!(
                "unsupported rrflowKV application format {:?}",
                database.manifest().application_format
            ))
        })
}

fn snapshot_codec(bundle: &SnapshotBundleFile) -> Result<keyspaces::RrflowKvKeyCodec> {
    keyspaces::RrflowKvKeyCodec::from_application_format(bundle.source_manifest.application_format)
        .ok_or_else(|| {
            Error::Substrate(format!(
                "unsupported rrflowKV snapshot application format {:?}",
                bundle.source_manifest.application_format
            ))
        })
}

fn encoded_storage_key(
    database: &Database,
    space: keyspaces::Space,
    key: &[u8],
) -> Result<Vec<u8>> {
    let codec = database_codec(database)?;
    keyspaces::validate_space(codec, space, key)?;
    Ok(key.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rrd_core::{
        digest, Claim, DataTransaction, ObjectReceipt, Predicate, Producer, ProjectionId,
        RuntimeEvent, RuntimeEventSchema, RuntimeLogicalModel, RuntimeMutation, RuntimeProperties,
        RuntimePropertySchema, RuntimeRecord, RuntimeRecordSchema, RuntimeRef, RuntimeRelation,
        RuntimeRelationSchema, RuntimeSchemaRegistry, RuntimeTableSchema, RuntimeType,
        RuntimeValue, RuntimeValueType, RuntimeVector, Subject, VectorCollectionAddress,
        VectorValue,
    };
    use rrd_lsm::{FailureMode, WriteBatch, WriteBoundary};
    use std::collections::BTreeMap;

    #[test]
    fn idle_maintenance_does_not_queue_behind_the_database_owner() {
        let directory = tempfile::tempdir().unwrap();
        let store = RrflowKvStore::open(&directory.path().join("idle")).unwrap();
        let owner = store.lock().unwrap();
        assert!(!store.try_maintenance_step().unwrap());
        drop(owner);
        // Opening a store can leave one physical reclamation quantum.
        store.try_maintenance_step().unwrap();
        assert!(!store.try_maintenance_step().unwrap());
    }

    #[test]
    fn reconciliation_evidence_fails_on_counter_regression() {
        let before_cache = PageCacheStats {
            hits: 1,
            ..PageCacheStats::default()
        };
        let after_cache = PageCacheStats::default();
        let error = reconciliation_evidence(
            &before_cache,
            &after_cache,
            &SegmentIoStats::default(),
            &SegmentIoStats::default(),
        )
        .unwrap_err();
        assert!(error.to_string().contains("counter regressed: cache hits"));
    }

    #[test]
    fn reconciliation_evidence_fails_on_derived_counter_overflow() {
        let after_cache = PageCacheStats {
            hits: u64::MAX,
            misses: 1,
            ..PageCacheStats::default()
        };
        let error = reconciliation_evidence(
            &PageCacheStats::default(),
            &after_cache,
            &SegmentIoStats::default(),
            &SegmentIoStats::default(),
        )
        .unwrap_err();
        assert!(error.to_string().contains("page requests overflowed"));
    }

    #[test]
    fn reconciliation_evidence_attributes_actual_io_backend() {
        let after_cache = PageCacheStats {
            misses: 1,
            loads: 1,
            bytes_read: 64,
            ..PageCacheStats::default()
        };
        let after_io = SegmentIoStats {
            fallback_count: 1,
            read_operations: 1,
            bounded_read_operations: 1,
            bytes_read: 64,
            ..SegmentIoStats::default()
        };
        let evidence = reconciliation_evidence(
            &PageCacheStats::default(),
            &after_cache,
            &SegmentIoStats::default(),
            &after_io,
        )
        .unwrap();
        assert_eq!(evidence.io_fallbacks, 1);
        assert_eq!(evidence.read_operations, 1);
        assert_eq!(evidence.io_uring_read_operations, 0);
        assert_eq!(evidence.bounded_read_operations, 1);
        assert_eq!(evidence.bytes_read, 64);
    }

    fn claim() -> Claim {
        Claim::new(
            rrd_core::Subject::new("test-subject").unwrap(),
            rrd_core::Predicate::new("test-predicate").unwrap(),
            "test-value",
            10,
            11,
            Producer {
                actor: "rrflow-kv-test".into(),
                on_behalf_of: None,
                session: None,
            },
        )
    }

    fn failure_scope() -> ScopeId {
        ScopeId::new("instance:rrflow-kv-failure").unwrap()
    }

    fn failure_schema() -> RuntimeSchemaRegistry {
        let item_kind = RuntimeType::new("item").unwrap();
        let mut registry = RuntimeSchemaRegistry::empty(1, "rrflowKV semantic failure matrix");
        registry
            .define_record_table(
                item_kind.clone(),
                RuntimeLogicalModel::Relational,
                RuntimeRecordSchema {
                    properties: BTreeMap::from([
                        (
                            "status".into(),
                            RuntimePropertySchema::required(RuntimeValueType::String),
                        ),
                        (
                            "title".into(),
                            RuntimePropertySchema::required(RuntimeValueType::String),
                        ),
                        (
                            "code".into(),
                            RuntimePropertySchema::required(RuntimeValueType::String),
                        ),
                    ]),
                    ..RuntimeRecordSchema::default()
                },
            )
            .unwrap();
        registry.tables.insert(
            RuntimeType::new("embedding").unwrap(),
            RuntimeTableSchema::schemaless(RuntimeLogicalModel::Vector),
        );
        registry
            .define_event_table(
                RuntimeType::new("pulse").unwrap(),
                RuntimeLogicalModel::Event,
                RuntimeEventSchema {
                    subject_required: true,
                    subject_types: BTreeSet::from([item_kind.clone()]),
                    properties: BTreeMap::new(),
                    allow_additional_properties: false,
                },
            )
            .unwrap();
        registry
            .define_relation_table(
                RuntimeType::new("links").unwrap(),
                RuntimeRelationSchema {
                    from: BTreeSet::from([item_kind.clone()]),
                    to: BTreeSet::from([item_kind]),
                    ..RuntimeRelationSchema::default()
                },
            )
            .unwrap();
        registry
    }

    fn failure_record(id: &str, status: &str, title: &str, valid_from: u64) -> RuntimeRecord {
        RuntimeRecord {
            reference: RuntimeRef::new("item", id).unwrap(),
            valid_from,
            valid_to: None,
            properties: BTreeMap::from([
                (
                    "code".into(),
                    RuntimeValue::String(format!("{id}-{valid_from}")),
                ),
                ("status".into(), RuntimeValue::String(status.into())),
                ("title".into(), RuntimeValue::String(title.into())),
            ]),
        }
    }

    fn bootstrap_failure_database(root: &Path) -> Database {
        let store = RrflowKvStore::open(root).unwrap();
        store
            .runtime()
            .commit(&RuntimeCommit {
                scope: failure_scope(),
                at: 10,
                actor: "agent:rrflow-kv-failure".into(),
                expected_cursor: 0,
                mutations: vec![
                    RuntimeMutation::Schema {
                        registry: failure_schema(),
                    },
                    RuntimeMutation::Record {
                        record: failure_record("one", "open", "First", 10),
                    },
                    RuntimeMutation::Record {
                        record: failure_record("two", "closed", "Second", 10),
                    },
                ],
            })
            .unwrap();
        let bindings = [
            crate::IndexCommitBindingDefinition {
                id: ProjectionId::new("item-status").unwrap(),
                record_kind: RuntimeType::new("item").unwrap(),
                fields: vec!["status".into()],
                kind: crate::IndexCommitBindingKind::Scalar { unique: false },
                configuration_sha256: digest::sha256_hex(b"item-status:scalar"),
            },
            crate::IndexCommitBindingDefinition {
                id: ProjectionId::new("item-code").unwrap(),
                record_kind: RuntimeType::new("item").unwrap(),
                fields: vec!["code".into()],
                kind: crate::IndexCommitBindingKind::Scalar { unique: true },
                configuration_sha256: digest::sha256_hex(b"item-code:unique"),
            },
            crate::IndexCommitBindingDefinition {
                id: ProjectionId::new("item-title-bm25").unwrap(),
                record_kind: RuntimeType::new("item").unwrap(),
                fields: vec!["title".into()],
                kind: crate::IndexCommitBindingKind::Bm25,
                configuration_sha256: digest::sha256_hex(b"item-title:bm25"),
            },
        ];
        let scope = failure_scope();
        store
            .control()
            .commit_catalog_with_index_bindings(
                &scope,
                1,
                &crate::ControlTransition {
                    key: format!("server/state/index-catalogue/{scope}"),
                    expected: None,
                    replacement: Some(br#"{"contract":"semantic-failure-matrix-v1"}"#.to_vec()),
                    at: 20,
                    actor: "agent:rrflow-kv-failure".into(),
                    action: "index_catalogue.installed".into(),
                    request_id: "request-semantic-failure-index".into(),
                    operation_id: "operation-semantic-failure-index".into(),
                },
                &bindings,
            )
            .unwrap();
        drop(store);
        Database::open(root).unwrap()
    }

    fn failure_transaction(database: &Database) -> DataTransaction {
        let scope = failure_scope();
        let read = rrflow_kv_read_stamp(database, database.snapshot(), &scope).unwrap();
        let item = RuntimeRef::new("item", "one").unwrap();
        DataTransaction::new(
            read,
            RuntimeCommit {
                scope,
                at: 100,
                actor: "agent:rrflow-kv-failure".into(),
                expected_cursor: 3,
                mutations: vec![
                    RuntimeMutation::Record {
                        record: failure_record("one", "pending", "First revised", 100),
                    },
                    RuntimeMutation::Relation {
                        relation: RuntimeRelation {
                            reference: RuntimeRef::new("links", "one-two").unwrap(),
                            from: RuntimeRef::new("item", "one").unwrap(),
                            to: RuntimeRef::new("item", "two").unwrap(),
                            valid_from: 100,
                            valid_to: None,
                            properties: RuntimeProperties::new(),
                        },
                    },
                    RuntimeMutation::Vector {
                        vector: RuntimeVector {
                            reference: RuntimeRef::new("embedding", "one-title").unwrap(),
                            subject: item.clone(),
                            collection: VectorCollectionAddress {
                                collection_id: "items".into(),
                                vector_name: "semantic".into(),
                            },
                            field: "title".into(),
                            valid_from: 100,
                            valid_to: None,
                            value: VectorValue::Dense {
                                values: vec![1.0, 0.0],
                            },
                            provenance: None,
                            properties: BTreeMap::new(),
                        },
                    },
                    RuntimeMutation::Event {
                        event: RuntimeEvent {
                            kind: RuntimeType::new("pulse").unwrap(),
                            subject: Some(item),
                            properties: RuntimeProperties::new(),
                        },
                    },
                    RuntimeMutation::Claim {
                        claim: Claim::new(
                            Subject::new("item:one").unwrap(),
                            Predicate::new("status").unwrap(),
                            "ready",
                            100,
                            100,
                            Producer {
                                actor: "agent:rrflow-kv-failure".into(),
                                on_behalf_of: None,
                                session: Some("rrflow-kv-failure".into()),
                            },
                        ),
                    },
                ],
            },
        )
        .unwrap()
    }

    fn failure_function_receipt(commit_id: &str) -> crate::FunctionInvocationReceiptRecord {
        let canonical_receipt_json = format!(
            "{{\"invocation_id\":\"failure-function\",\"runtime_commit_sha256\":\"{commit_id}\"}}"
        );
        crate::FunctionInvocationReceiptRecord {
            format_version: crate::FUNCTION_INVOCATION_RECEIPT_FORMAT_VERSION,
            invocation_id: "failure-function".into(),
            runtime_commit_sha256: Some(commit_id.into()),
            receipt_sha256: digest::sha256_hex(format!("receipt:{commit_id}").as_bytes()),
            canonical_receipt_sha256: digest::sha256_hex(canonical_receipt_json.as_bytes()),
            canonical_receipt_json,
        }
    }

    #[test]
    fn rrflow_kv_multi_family_transaction_recovers_all_or_none_at_every_wal_boundary() {
        for mode in [FailureMode::Crash, FailureMode::StorageFull] {
            for boundary in [
                WriteBoundary::Prepared,
                WriteBoundary::WalAppended,
                WriteBoundary::WalSynced,
                WriteBoundary::Visible,
            ] {
                let directory = tempfile::tempdir().unwrap();
                let root = directory.path().join("rrflow-kv-failure");
                let mut database = bootstrap_failure_database(&root);
                let transaction = failure_transaction(&database);
                let function_receipt = failure_function_receipt(&transaction.commit.digest());
                let plan = prepare_rrflow_kv_commit_at_read(
                    &database,
                    &transaction.commit,
                    Some(&transaction.read),
                    None,
                    Some(("rrflow-kv-failure", std::slice::from_ref(&function_receipt))),
                )
                .unwrap();
                let expected = plan.outcome().clone();
                let (_, operations) = plan.into_parts_for(&database).unwrap();
                for (space, name, minimum) in [
                    (keyspaces::RUNTIME_RECORDS, "current record", 1),
                    (keyspaces::RUNTIME_RECORD_VERSIONS, "record version", 1),
                    (keyspaces::RUNTIME_RELATIONS, "current relation", 1),
                    (keyspaces::RUNTIME_RELATION_VERSIONS, "relation version", 1),
                    (keyspaces::RUNTIME_OUTGOING_EDGES, "outgoing edge", 1),
                    (
                        keyspaces::RUNTIME_OUTGOING_EDGE_VERSIONS,
                        "outgoing edge version",
                        1,
                    ),
                    (keyspaces::RUNTIME_INCOMING_EDGES, "incoming edge", 1),
                    (
                        keyspaces::RUNTIME_INCOMING_EDGE_VERSIONS,
                        "incoming edge version",
                        1,
                    ),
                    (
                        keyspaces::RUNTIME_INDEX_SOURCE_DELTAS,
                        "scalar, unique, and BM25 source deltas",
                        3,
                    ),
                    (keyspaces::RUNTIME_SCALAR_ENTRIES, "scalar index entry", 1),
                    (keyspaces::RUNTIME_UNIQUE_ENTRIES, "unique index entry", 1),
                    (keyspaces::RUNTIME_VECTORS, "current vector", 1),
                    (keyspaces::RUNTIME_VECTOR_VERSIONS, "vector version", 1),
                    (
                        keyspaces::RUNTIME_VECTOR_SOURCE_DELTAS,
                        "vector source delta",
                        1,
                    ),
                    (keyspaces::RUNTIME_CHANGES, "runtime changes", 5),
                    (keyspaces::RUNTIME_PROJECTION_DELTAS, "projection deltas", 1),
                    (keyspaces::RUNTIME_OUTBOX, "outbox", 1),
                    (keyspaces::RUNTIME_AUDIT, "audit", 1),
                    (keyspaces::RUNTIME_COMMITS, "commit outcome", 1),
                    (keyspaces::INVOCATIONS, "function receipt", 1),
                ] {
                    let prefix = keyspaces::space_prefix(space);
                    assert!(
                        operations
                            .iter()
                            .filter(|operation| operation.key().starts_with(&prefix))
                            .count()
                            >= minimum,
                        "semantic plan lacks {name}"
                    );
                }
                let baseline_snapshot = database.snapshot();
                let baseline = operations
                    .iter()
                    .map(|operation| {
                        (
                            operation.key().to_vec(),
                            database.get(operation.key(), baseline_snapshot).unwrap(),
                        )
                    })
                    .collect::<BTreeMap<_, _>>();
                let mut physical = database.begin_transaction().unwrap();
                for operation in &operations {
                    match operation {
                        Mutation::Put { key, value } => {
                            physical.put(key.clone(), value.clone()).unwrap()
                        }
                        Mutation::Delete { key } => physical.delete(key.clone()).unwrap(),
                    }
                }
                let error = database
                    .commit_transaction_with_failure(
                        physical,
                        rrd_lsm::Durability::Authoritative,
                        boundary,
                        mode,
                    )
                    .unwrap_err();
                assert!(matches!(error, rrd_lsm::Error::InjectedFailure { .. }));

                let visible_before_reopen = boundary == WriteBoundary::Visible;
                let in_process_snapshot = database.snapshot();
                for operation in &operations {
                    let expected_value = if visible_before_reopen {
                        match operation {
                            Mutation::Put { value, .. } => Some(value.clone()),
                            Mutation::Delete { .. } => None,
                        }
                    } else {
                        baseline[operation.key()].clone()
                    };
                    assert_eq!(
                        database.get(operation.key(), in_process_snapshot).unwrap(),
                        expected_value,
                        "in-process family split for mode={mode:?} boundary={boundary:?}"
                    );
                }
                drop(database);

                let mut recovered = Database::open(&root).unwrap();
                let snapshot = recovered.snapshot();
                let published = boundary != WriteBoundary::Prepared;
                for operation in &operations {
                    let expected_value = if published {
                        match operation {
                            Mutation::Put { value, .. } => Some(value.clone()),
                            Mutation::Delete { .. } => None,
                        }
                    } else {
                        baseline[operation.key()].clone()
                    };
                    assert_eq!(
                        recovered.get(operation.key(), snapshot).unwrap(),
                        expected_value,
                        "reopened family split for mode={mode:?} boundary={boundary:?}"
                    );
                }
                let expected_runtime_cursor = if published { 8 } else { 3 };
                let expected_claim_sequence = if published { 1 } else { 0 };
                assert_eq!(
                    read_sequence(&recovered, snapshot, &keyspaces::runtime_cursor_key(),).unwrap(),
                    expected_runtime_cursor,
                    "mode={mode:?} boundary={boundary:?}"
                );
                assert_eq!(
                    read_sequence(&recovered, snapshot, &keyspaces::sequence_watermark_key(),)
                        .unwrap(),
                    expected_claim_sequence,
                    "mode={mode:?} boundary={boundary:?}"
                );
                let outcome: Option<RuntimeCommitOutcome> = get_json(
                    &recovered,
                    snapshot,
                    keyspaces::RUNTIME_COMMITS,
                    &keyspaces::runtime_commit_key(&expected.commit_id),
                )
                .unwrap();
                assert_eq!(outcome.as_ref(), published.then_some(&expected));
                let audit: Option<AuditEnvelope> = get_json(
                    &recovered,
                    snapshot,
                    keyspaces::RUNTIME_AUDIT,
                    &keyspaces::runtime_audit_key(&expected.commit_id),
                )
                .unwrap();
                assert_eq!(
                    audit.as_ref().and_then(|value| value.read.as_ref()),
                    published.then_some(&transaction.read)
                );
                assert_eq!(
                    audit.as_ref().and_then(|value| value.outcome_cursor),
                    published.then_some(8)
                );
                let stored_function_receipt: Option<crate::FunctionInvocationReceiptRecord> =
                    get_json(
                        &recovered,
                        snapshot,
                        keyspaces::INVOCATIONS,
                        &keyspaces::function_invocation_receipt_key(
                            "rrflow-kv-failure",
                            &function_receipt.invocation_id,
                        ),
                    )
                    .unwrap();
                assert_eq!(
                    stored_function_receipt.as_ref(),
                    published.then_some(&function_receipt)
                );
                let record: Option<RuntimeRecord> = get_json(
                    &recovered,
                    snapshot,
                    keyspaces::RUNTIME_RECORDS,
                    &keyspaces::runtime_identity_key(
                        keyspaces::RUNTIME_RECORDS,
                        &transaction.commit.scope,
                        &RuntimeRef::new("item", "one").unwrap(),
                    ),
                )
                .unwrap();
                let expected_record = if published {
                    failure_record("one", "pending", "First revised", 100)
                } else {
                    failure_record("one", "open", "First", 10)
                };
                assert_eq!(record.unwrap().properties, expected_record.properties);
                let relation: Option<RuntimeRelation> = get_json(
                    &recovered,
                    snapshot,
                    keyspaces::RUNTIME_RELATIONS,
                    &keyspaces::runtime_identity_key(
                        keyspaces::RUNTIME_RELATIONS,
                        &transaction.commit.scope,
                        &RuntimeRef::new("links", "one-two").unwrap(),
                    ),
                )
                .unwrap();
                assert_eq!(relation.is_some(), published);

                recovered
                    .write_owned(
                        WriteBatch::new(vec![Mutation::Put {
                            key: keyspaces::control_record_key("post-reopen"),
                            value: b"accepted".to_vec(),
                        }])
                        .unwrap(),
                        rrd_lsm::Durability::Authoritative,
                    )
                    .unwrap();
            }
        }
    }

    #[test]
    fn new_rrflow_kv_database_authenticates_and_reopens_typed_key_format() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().join("compact-rrflow-kv");
        let expected = claim();
        let engine = RrflowKvStore::open(&root).unwrap();
        assert_eq!(
            engine.manifest().unwrap().application_format,
            Some(keyspaces::RRFLOW_KV_FORMAT)
        );
        engine
            .claims()
            .append_batch(std::slice::from_ref(&expected))
            .unwrap();
        {
            let database = engine.lock().unwrap();
            let start = keyspaces::space_prefix(keyspaces::CLAIMS);
            let end = prefix_end(&start).unwrap();
            let rows = database
                .scan(&start, Some(&end), database.snapshot())
                .unwrap();
            assert_eq!(rows.len(), 1);
            keyspaces::parse_claim_key(KeyCodec, &rows[0].0).unwrap();
        }
        engine.flush(12).unwrap();
        drop(engine);

        let reopened = RrflowKvStore::open(&root).unwrap();
        assert_eq!(
            reopened.manifest().unwrap().application_format,
            Some(keyspaces::RRFLOW_KV_FORMAT)
        );
        assert_eq!(
            reopened.claims().claims_in_range(0, 1).unwrap(),
            vec![expected]
        );
    }

    #[test]
    fn rrflow_kv_open_denies_unknown_authenticated_application_format() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().join("unknown-rrflow-kv-format");
        drop(
            Database::create_with_application_format(&root, DatabaseOptions::default(), 7).unwrap(),
        );
        assert!(matches!(
            RrflowKvStore::open(&root),
            Err(Error::Substrate(reason)) if reason.contains("unsupported rrflowKV application format")
        ));
    }

    #[test]
    fn snapshot_object_closure_denies_foreign_project_references() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().join("multi-project-rrflow-kv");
        let mut database = Database::create_with_application_format(
            &root,
            DatabaseOptions::default(),
            keyspaces::RRFLOW_KV_FORMAT,
        )
        .unwrap();
        let first_scope = ScopeId::new("project:first").unwrap();
        let second_scope = ScopeId::new("project:second").unwrap();
        let object = |id: &str, bytes: &[u8]| {
            let sha256 = rrd_core::digest::sha256_hex(bytes);
            ObjectReference::for_bytes(
                id,
                None,
                "application/octet-stream",
                bytes,
                ObjectReceipt {
                    backend: "fixture".into(),
                    key: ObjectReference::canonical_key(&sha256).unwrap(),
                    version: None,
                    etag: None,
                },
            )
            .unwrap()
        };
        let first = object("first:bytes", b"first");
        let second = object("second:bytes", b"second");
        database
            .write_owned(
                WriteBatch::new(vec![
                    Mutation::Put {
                        key: storage_key(
                            keyspaces::RUNTIME_OBJECTS,
                            &keyspaces::runtime_identity_key(
                                keyspaces::RUNTIME_OBJECTS,
                                &first_scope,
                                &first.reference,
                            ),
                        ),
                        value: serde_json::to_vec(&first).unwrap(),
                    },
                    Mutation::Put {
                        key: storage_key(
                            keyspaces::RUNTIME_OBJECTS,
                            &keyspaces::runtime_identity_key(
                                keyspaces::RUNTIME_OBJECTS,
                                &second_scope,
                                &second.reference,
                            ),
                        ),
                        value: serde_json::to_vec(&second).unwrap(),
                    },
                ])
                .unwrap(),
                rrd_lsm::Durability::Authoritative,
            )
            .unwrap();
        let spool = directory.path().join("multi-project.snapshot");
        let bundle = database.export_snapshot_file(1, &spool).unwrap();

        let error = rrflow_kv_snapshot_object_references(&bundle, &first_scope).unwrap_err();
        assert!(error.to_string().contains("project scope"));
    }
}
