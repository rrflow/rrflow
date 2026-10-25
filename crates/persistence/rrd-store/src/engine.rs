//! The canonical storage port used by the RRFlow engine.
//!
//! The port owns only physical snapshot-transaction creation and access to the
//! concrete semantic repositories shared by persistent rrflowKV and volatile
//! rrflowMX. Repository code therefore defines claim, control, projection,
//! invocation, and runtime behavior once. Only rrflowKV adds WAL-backed
//! durability and physical storage evidence.
//!
//! Cache and external database adapters compose above this boundary. They may
//! accelerate or supply data, but they cannot become a second state authority.

use crate::error::{Error, Result};
use crate::keyspaces::Durability;
use crate::transaction::{
    validate_range, validate_read_key, StorageTransaction, TransactionCommit, TransactionRollback,
    TransactionWriteSet,
};
use std::collections::BTreeMap;
use std::ops::Bound::{Excluded, Included};
use std::sync::Mutex;

/// `PhysicalStoreEvidence::evidence_level` of a backend without stable
/// physical counters.
pub const LOGICAL_ONLY_EVIDENCE: &str = "logical_only";
/// `PhysicalStoreEvidence::evidence_level` of rrflowKV's memtable, WAL,
/// maintenance, segment, page-cache, and segment I/O counters.
pub const RRFLOW_KV_COUNTER_EVIDENCE: &str = "rrflow_kv_counters";

/// A read-only physical counter snapshot used to attribute bounded storage
/// work to one logical operation. Counters are cumulative; callers difference
/// two snapshots taken immediately around the work. Backends without stable
/// physical counters return `logical_only` evidence instead of inventing it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhysicalStoreEvidence {
    pub backend: String,
    pub evidence_level: String,
    pub physical_sequence: Option<u64>,
    pub manifest_generation: Option<u64>,
    pub durable_sequence: Option<u64>,
    pub retained_mutable_bytes: Option<u64>,
    pub retained_projected_bytes: Option<u64>,
    pub retained_source_bytes: Option<u64>,
    pub peak_source_bytes: Option<u64>,
    pub denied_source_allocations: Option<u64>,

    pub checkpoint_entries: Option<u64>,
    pub checkpoint_limit: Option<u64>,
    pub native_transactions: Option<u64>,
    pub native_transaction_bytes: Option<u64>,
    pub active_read_views: Option<u64>,
    pub pinned_read_bytes: Option<u64>,
    pub memtable_versions: Option<u64>,
    pub memtable_bytes: Option<u64>,
    pub memtable_keys: Option<u64>,
    pub memtable_key_payload_bytes: Option<u64>,
    pub memtable_value_payload_bytes: Option<u64>,
    pub memtable_tombstones: Option<u64>,
    pub memtable_spilled_chains: Option<u64>,
    pub memtable_spilled_version_capacity: Option<u64>,
    pub memtable_version_record_bytes: Option<u64>,
    pub memtable_owned_bytes_lower_bound: Option<u64>,
    pub memtable_max_versions: Option<u64>,
    pub wal_payload_bytes: Option<u64>,
    pub wal_payload_max_bytes: Option<u64>,
    pub automatic_flushes: Option<u64>,
    pub maintenance_write_stalls: Option<u64>,
    pub failed_maintenance_flushes: Option<u64>,
    pub oversized_batches: Option<u64>,
    pub automatic_compactions: Option<u64>,
    pub failed_compactions: Option<u64>,
    pub compaction_input_bytes: Option<u64>,
    pub compaction_output_bytes: Option<u64>,
    pub peak_compaction_buffer_bytes: Option<u64>,
    pub l0_segment_count: Option<u64>,
    pub l0_compaction_trigger: Option<u64>,
    pub compaction_debt_segments: Option<u64>,
    pub compaction_target_segment_bytes: Option<u64>,
    pub segment_count: Option<u64>,
    pub segment_bytes: Option<u64>,
    pub cache_capacity_bytes: Option<u64>,
    pub cache_resident_bytes: Option<u64>,
    pub cache_entries: Option<u64>,
    pub cache_hits: Option<u64>,
    pub cache_misses: Option<u64>,
    pub cache_evictions: Option<u64>,
    pub page_loads: Option<u64>,
    pub page_bytes_read: Option<u64>,
    pub page_bytes_decoded: Option<u64>,
    pub page_bytes_borrowed: Option<u64>,
    pub page_bytes_allocated: Option<u64>,
    pub page_bytes_copied: Option<u64>,
    pub page_bytes_decompressed: Option<u64>,
    pub filter_checks: Option<u64>,
    pub filter_negatives: Option<u64>,
    pub segment_io_requested_mode: Option<String>,
    pub segment_io_mmap_segments: Option<u64>,
    pub segment_io_uring_segments: Option<u64>,
    pub segment_io_bounded_segments: Option<u64>,
    pub segment_io_fallbacks: Option<u64>,
    pub segment_io_last_fallback: Option<String>,
    pub segment_io_read_operations: Option<u64>,
    pub segment_io_mmap_reads: Option<u64>,
    pub segment_io_uring_reads: Option<u64>,
    pub segment_io_bounded_reads: Option<u64>,
    pub segment_io_bytes_read: Option<u64>,
    pub segment_io_max_request_bytes: Option<u64>,
    pub segment_io_peak_request_bytes: Option<u64>,
    pub segment_io_max_open_files: Option<u64>,
    pub segment_io_open_files: Option<u64>,
    pub segment_io_peak_open_files: Option<u64>,
    pub segment_io_handle_evictions: Option<u64>,
    pub segment_io_handle_reopens: Option<u64>,
    pub segment_io_handle_denials: Option<u64>,
}

impl PhysicalStoreEvidence {
    pub fn logical_only(backend: impl Into<String>) -> Self {
        Self {
            backend: backend.into(),
            evidence_level: LOGICAL_ONLY_EVIDENCE.into(),
            physical_sequence: None,
            manifest_generation: None,
            durable_sequence: None,
            retained_mutable_bytes: None,
            retained_projected_bytes: None,
            retained_source_bytes: None,
            peak_source_bytes: None,
            denied_source_allocations: None,

            checkpoint_entries: None,
            checkpoint_limit: None,
            native_transactions: None,
            native_transaction_bytes: None,
            active_read_views: None,
            pinned_read_bytes: None,
            memtable_versions: None,
            memtable_bytes: None,
            memtable_keys: None,
            memtable_key_payload_bytes: None,
            memtable_value_payload_bytes: None,
            memtable_tombstones: None,
            memtable_spilled_chains: None,
            memtable_spilled_version_capacity: None,
            memtable_version_record_bytes: None,
            memtable_owned_bytes_lower_bound: None,
            memtable_max_versions: None,
            wal_payload_bytes: None,
            wal_payload_max_bytes: None,
            automatic_flushes: None,
            maintenance_write_stalls: None,
            failed_maintenance_flushes: None,
            oversized_batches: None,
            automatic_compactions: None,
            failed_compactions: None,
            compaction_input_bytes: None,
            compaction_output_bytes: None,
            peak_compaction_buffer_bytes: None,
            l0_segment_count: None,
            l0_compaction_trigger: None,
            compaction_debt_segments: None,
            compaction_target_segment_bytes: None,
            segment_count: None,
            segment_bytes: None,
            cache_capacity_bytes: None,
            cache_resident_bytes: None,
            cache_entries: None,
            cache_hits: None,
            cache_misses: None,
            cache_evictions: None,
            page_loads: None,
            page_bytes_read: None,
            page_bytes_decoded: None,
            page_bytes_borrowed: None,
            page_bytes_allocated: None,
            page_bytes_copied: None,
            page_bytes_decompressed: None,
            filter_checks: None,
            filter_negatives: None,
            segment_io_requested_mode: None,
            segment_io_mmap_segments: None,
            segment_io_uring_segments: None,
            segment_io_bounded_segments: None,
            segment_io_fallbacks: None,
            segment_io_last_fallback: None,
            segment_io_read_operations: None,
            segment_io_mmap_reads: None,
            segment_io_uring_reads: None,
            segment_io_bounded_reads: None,
            segment_io_bytes_read: None,
            segment_io_max_request_bytes: None,
            segment_io_peak_request_bytes: None,
            segment_io_max_open_files: None,
            segment_io_open_files: None,
            segment_io_peak_open_files: None,
            segment_io_handle_evictions: None,
            segment_io_handle_reopens: None,
            segment_io_handle_denials: None,
        }
    }
}

pub trait StorageEngine: Send + Sync {
    fn begin_transaction(&self) -> Result<Box<dyn StorageTransaction + '_>>;
    fn claims(&self) -> crate::ClaimRepository<'_>;
    fn control(&self) -> crate::ControlRepository<'_>;
    fn function_catalogue(&self) -> crate::FunctionCatalogueRepository<'_>;
    fn projections(&self) -> crate::ProjectionRepository<'_>;
    fn runtime(&self) -> crate::RuntimeRepository<'_>;
    fn invocations(&self) -> crate::InvocationRepository<'_>;

    fn physical_store_evidence(&self) -> Result<PhysicalStoreEvidence> {
        Ok(PhysicalStoreEvidence::logical_only("unspecified"))
    }
}

/// rrflowMX: the process-local, volatile implementation of the complete
/// semantic storage port.
///
/// rrflowMX is also the reference side of storage conformance differentials,
/// but it is not a persistent RRFlow database: nothing here survives the
/// process. Arrow working sets and DataFusion execution are layered above this
/// port in `rrd-query`; they are not alternate state owned by rrflowMX.
#[derive(Default)]
pub struct RrflowMxStore {
    inner: Mutex<RrflowMxStoreInner>,
}

#[derive(Default)]
struct RrflowMxStoreInner {
    transaction_sequence: u64,
    transaction_values: BTreeMap<Vec<u8>, Vec<RrflowMxTransactionVersion>>,
}

#[derive(Debug)]
struct RrflowMxTransactionVersion {
    sequence: u64,
    value: Option<Vec<u8>>,
}

impl RrflowMxStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Observe calls recorded, for tests that assert telemetry flowed.
    pub fn observe_count(&self) -> u64 {
        u64::try_from(
            self.claims()
                .access_count()
                .expect("rrflowMX access observations remain readable"),
        )
        .expect("rrflowMX access count fits u64")
    }
}

struct RrflowMxTransaction<'a> {
    store: &'a RrflowMxStore,
    snapshot_sequence: u64,
    writes: TransactionWriteSet,
}

impl StorageTransaction for RrflowMxTransaction<'_> {
    fn snapshot_sequence(&self) -> u64 {
        self.snapshot_sequence
    }

    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        validate_read_key(key)?;
        if let Some(value) = self.writes.get(key) {
            return Ok(value.map(<[u8]>::to_vec));
        }
        let inner = self.store.inner.lock().expect("engine mutex");
        Ok(inner
            .transaction_values
            .get(key)
            .and_then(|versions| {
                versions
                    .iter()
                    .rev()
                    .find(|version| version.sequence <= self.snapshot_sequence)
            })
            .and_then(|version| version.value.clone()))
    }

    fn scan(&self, start: &[u8], end: &[u8], limit: usize) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        validate_range(start, end, limit)?;
        let inner = self.store.inner.lock().expect("engine mutex");
        let mut visible = inner
            .transaction_values
            .range::<[u8], _>((Included(start), Excluded(end)))
            .filter_map(|(key, versions)| {
                versions
                    .iter()
                    .rev()
                    .find(|version| version.sequence <= self.snapshot_sequence)
                    .and_then(|version| {
                        version
                            .value
                            .as_ref()
                            .map(|value| (key.clone(), value.clone()))
                    })
            })
            .collect::<BTreeMap<_, _>>();
        drop(inner);
        for (key, value) in self
            .writes
            .mutations()
            .range::<[u8], _>((Included(start), Excluded(end)))
        {
            match value {
                Some(value) => {
                    visible.insert(key.clone(), value.clone());
                }
                None => {
                    visible.remove(key.as_slice());
                }
            }
        }
        Ok(visible.into_iter().take(limit).collect())
    }

    fn put(&mut self, key: Vec<u8>, value: Vec<u8>) -> Result<()> {
        self.writes.put(key, value)
    }

    fn delete(&mut self, key: Vec<u8>) -> Result<()> {
        self.writes.delete(key)
    }

    fn commit(self: Box<Self>, _durability: Durability) -> Result<TransactionCommit> {
        let Self {
            store,
            snapshot_sequence,
            writes,
        } = *self;
        let mutation_count = writes.len();
        if mutation_count == 0 {
            tracing::debug!(
                target: "rrd_store::transaction",
                storage_profile = "rrflow_mx",
                snapshot_sequence,
                mutation_count,
                outcome = "read_only",
                "storage transaction committed without a write"
            );
            return Ok(TransactionCommit {
                snapshot_sequence,
                mutation_count,
                first_sequence: None,
                last_sequence: None,
            });
        }
        let batch = writes.into_batch()?;
        let mut inner = store.inner.lock().expect("engine mutex");
        for operation in batch.operations() {
            let key = operation.key();
            if let Some(conflicting_sequence) = inner
                .transaction_values
                .get(key)
                .and_then(|versions| versions.last())
                .map(|version| version.sequence)
                .filter(|sequence| *sequence > snapshot_sequence)
            {
                tracing::warn!(
                    target: "rrd_store::transaction",
                    storage_profile = "rrflow_mx",
                    snapshot_sequence,
                    conflicting_sequence,
                    mutation_count,
                    outcome = "conflict",
                    "storage transaction commit denied"
                );
                return Err(Error::TransactionConflict {
                    snapshot_sequence,
                    conflicting_sequence,
                });
            }
        }
        let mutation_count_u64 =
            u64::try_from(mutation_count).map_err(|_| Error::SequenceOverflow)?;
        let first_sequence = inner
            .transaction_sequence
            .checked_add(1)
            .ok_or(Error::SequenceOverflow)?;
        let last_sequence = inner
            .transaction_sequence
            .checked_add(mutation_count_u64)
            .ok_or(Error::SequenceOverflow)?;
        for (offset, operation) in batch.into_operations().into_iter().enumerate() {
            let (key, value) = match operation {
                rrd_lsm::Mutation::Put { key, value } => (key, Some(value)),
                rrd_lsm::Mutation::Delete { key } => (key, None),
            };
            inner
                .transaction_values
                .entry(key)
                .or_default()
                .push(RrflowMxTransactionVersion {
                    sequence: first_sequence + offset as u64,
                    value,
                });
        }
        inner.transaction_sequence = last_sequence;
        tracing::debug!(
            target: "rrd_store::transaction",
            storage_profile = "rrflow_mx",
            snapshot_sequence,
            mutation_count,
            first_sequence,
            last_sequence,
            outcome = "committed",
            "storage transaction committed"
        );
        Ok(TransactionCommit {
            snapshot_sequence,
            mutation_count,
            first_sequence: Some(first_sequence),
            last_sequence: Some(last_sequence),
        })
    }

    fn rollback(self: Box<Self>) -> Result<TransactionRollback> {
        let outcome = TransactionRollback {
            snapshot_sequence: self.snapshot_sequence,
            discarded_mutations: self.writes.len(),
        };
        tracing::debug!(
            target: "rrd_store::transaction",
            storage_profile = "rrflow_mx",
            snapshot_sequence = outcome.snapshot_sequence,
            discarded_mutations = outcome.discarded_mutations,
            outcome = "rolled_back",
            "storage transaction rolled back"
        );
        Ok(outcome)
    }
}

pub(crate) fn validate_idempotency(key: &str, digest: &str) -> Result<()> {
    if key.is_empty()
        || key.len() > 128
        || !key
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
    {
        return Err(Error::Substrate("invalid idempotency key".into()));
    }
    if digest.len() != 64
        || !digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(Error::Substrate(
            "operation digest must be lowercase SHA-256".into(),
        ));
    }
    Ok(())
}

impl StorageEngine for RrflowMxStore {
    fn begin_transaction(&self) -> Result<Box<dyn StorageTransaction + '_>> {
        let snapshot_sequence = self
            .inner
            .lock()
            .expect("engine mutex")
            .transaction_sequence;
        tracing::debug!(
            target: "rrd_store::transaction",
            storage_profile = "rrflow_mx",
            snapshot_sequence,
            outcome = "begun",
            "storage transaction began"
        );
        Ok(Box::new(RrflowMxTransaction {
            store: self,
            snapshot_sequence,
            writes: TransactionWriteSet::default(),
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
        Ok(PhysicalStoreEvidence::logical_only("rrflow_mx"))
    }
}

/// One composition-root-owned RRFlow storage profile.
///
/// The upper engine composition root selects the profile once: rrflowKV is
/// persistent and rrflowMX is process-local. This enum owns no selection
/// heuristics and cannot open storage on its own.
pub enum StorageProfile {
    RrflowMx(RrflowMxStore),
    RrflowKv(Box<crate::RrflowKvStore>),
}

impl StorageProfile {
    pub fn rrflow_mx() -> Self {
        Self::RrflowMx(RrflowMxStore::new())
    }

    pub fn rrflow_kv(store: crate::RrflowKvStore) -> Self {
        Self::RrflowKv(Box::new(store))
    }

    pub fn as_rrflow_kv(&self) -> Option<&crate::RrflowKvStore> {
        match self {
            Self::RrflowMx(_) => None,
            Self::RrflowKv(store) => Some(store.as_ref()),
        }
    }

    pub fn backend_name(&self) -> &'static str {
        match self {
            Self::RrflowMx(_) => "rrflow_mx",
            Self::RrflowKv(_) => "rrflow_kv",
        }
    }

    fn engine(&self) -> &(dyn StorageEngine + Send + Sync) {
        match self {
            Self::RrflowMx(engine) => engine,
            Self::RrflowKv(engine) => engine.as_ref(),
        }
    }
}

impl StorageEngine for StorageProfile {
    fn begin_transaction(&self) -> Result<Box<dyn StorageTransaction + '_>> {
        self.engine().begin_transaction()
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
        self.engine().physical_store_evidence()
    }
}
