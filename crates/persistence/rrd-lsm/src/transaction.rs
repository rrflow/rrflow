//! Snapshot-isolated byte transactions for the rrflowKV physical substrate.
//!
//! A transaction pins the database sequence captured at `begin`, buffers one
//! final mutation per key, and overlays that write set on point and bounded
//! range reads. Commit performs optimistic conflict detection for every key in
//! the write set while the database's sole writer is held, then publishes one
//! WAL batch. Read-only keys and predicates are deliberately not conflict
//! ranges: the contract is snapshot isolation with write conflict detection,
//! not serializability.

use crate::batch::{validate_key, WriteBatchSize};
use crate::{AppendReceipt, Database, Error, Mutation, Result, Snapshot};
use std::collections::BTreeMap;
use std::ops::Bound::Included;

mod scan;
use std::sync::{Arc, Mutex};

/// Native transaction ownership; separate from active memtables and caller input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TransactionLimits {
    pub active: usize,
    pub buffered_bytes: usize,
}
impl Default for TransactionLimits {
    fn default() -> Self {
        Self {
            active: 64,
            buffered_bytes: 64 * 1024 * 1024,
        }
    }
}
impl TransactionLimits {
    pub(crate) fn validate(self) -> Result<()> {
        if self.active == 0 || self.buffered_bytes == 0 {
            return Err(Error::InvalidConfiguration(
                "native transaction limits must be positive".into(),
            ));
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransactionResource {
    Active,
    BufferedBytes,
}
impl TransactionResource {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active owners",
            Self::BufferedBytes => "owned buffer bytes",
        }
    }
}
#[derive(Debug, Default)]
struct TransactionOwnership {
    pins: BTreeMap<u64, usize>,
    active: usize,
    bytes: usize,
}
#[derive(Debug)]
pub(crate) struct TransactionSnapshots {
    state: Mutex<TransactionOwnership>,
    limits: TransactionLimits,
}
impl TransactionSnapshots {
    pub(crate) fn new(limits: TransactionLimits) -> Self {
        Self {
            state: Mutex::new(TransactionOwnership::default()),
            limits,
        }
    }
    fn acquire(&self, sequence: u64) -> Result<()> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| Error::PoisonedTransactionState)?;
        let next = state
            .active
            .checked_add(1)
            .ok_or(Error::PoisonedTransactionState)?;
        if next > self.limits.active {
            return Err(Error::TransactionLimit {
                resource: TransactionResource::Active,
                limit: self.limits.active,
                observed: next,
            });
        }
        let count = state.pins.entry(sequence).or_default();
        *count = count
            .checked_add(1)
            .ok_or(Error::PoisonedTransactionState)?;
        state.active = next;
        Ok(())
    }
    fn resize(&self, previous: usize, next: usize) -> Result<()> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| Error::PoisonedTransactionState)?;
        let total = state
            .bytes
            .checked_sub(previous)
            .and_then(|v| v.checked_add(next))
            .ok_or_else(|| Error::InvalidTransaction("transaction reservation overflow".into()))?;
        if total > self.limits.buffered_bytes {
            return Err(Error::TransactionLimit {
                resource: TransactionResource::BufferedBytes,
                limit: self.limits.buffered_bytes,
                observed: total,
            });
        }
        state.bytes = total;
        Ok(())
    }
    fn release(&self, sequence: u64, bytes: usize) -> Result<()> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| Error::PoisonedTransactionState)?;
        release_sequence(&mut state, sequence, bytes)
    }
    fn release_after_panic(&self, sequence: u64, bytes: usize) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _ = release_sequence(&mut state, sequence, bytes);
    }
    pub(crate) fn ownership(&self) -> Result<(u64, u64)> {
        let state = self
            .state
            .lock()
            .map_err(|_| Error::PoisonedTransactionState)?;
        Ok((state.active as u64, state.bytes as u64))
    }
    pub(crate) fn sequences(&self) -> Result<Vec<u64>> {
        Ok(self
            .state
            .lock()
            .map_err(|_| Error::PoisonedTransactionState)?
            .pins
            .keys()
            .copied()
            .collect())
    }
    pub(crate) fn is_empty(&self) -> Result<bool> {
        Ok(self
            .state
            .lock()
            .map_err(|_| Error::PoisonedTransactionState)?
            .pins
            .is_empty())
    }
}
fn release_sequence(state: &mut TransactionOwnership, sequence: u64, bytes: usize) -> Result<()> {
    let count = *state
        .pins
        .get(&sequence)
        .ok_or(Error::PoisonedTransactionState)?;
    let active = state
        .active
        .checked_sub(1)
        .ok_or(Error::PoisonedTransactionState)?;
    let remaining = state
        .bytes
        .checked_sub(bytes)
        .ok_or(Error::PoisonedTransactionState)?;
    if count == 1 {
        state.pins.remove(&sequence);
    } else {
        *state
            .pins
            .get_mut(&sequence)
            .ok_or(Error::PoisonedTransactionState)? = count - 1;
    }
    state.active = active;
    state.bytes = remaining;
    Ok(())
}
/// Payload capacities plus both map-entry and handoff mutation representations.
/// This excludes allocator/tree bookkeeping and caller-owned input allocations.
fn buffer_charge(key_capacity: usize, value: Option<&Vec<u8>>) -> Result<usize> {
    key_capacity
        .checked_add(value.map_or(0, Vec::capacity))
        .and_then(|v| v.checked_add(std::mem::size_of::<(Vec<u8>, Option<Vec<u8>>)>()))
        .and_then(|v| v.checked_add(std::mem::size_of::<Mutation>()))
        .ok_or_else(|| Error::InvalidTransaction("transaction buffer charge overflow".into()))
}

/// One transaction-local view over a single [`Database`] instance.
///
/// The transaction does not borrow the database, allowing the owning store to
/// release its mutex between operations. Its private owner token prevents a
/// snapshot or write set from being committed through another database.
#[derive(Debug)]
pub struct Transaction {
    snapshot: Snapshot,
    writes: BTreeMap<Vec<u8>, Option<Vec<u8>>>,
    write_size: WriteBatchSize,
    buffered_bytes: usize,
    owner: Arc<TransactionSnapshots>,
    released: bool,
}

impl Transaction {
    pub(crate) fn begin(snapshot: Snapshot, owner: Arc<TransactionSnapshots>) -> Result<Self> {
        owner.acquire(snapshot.sequence)?;
        Ok(Self {
            snapshot,
            writes: BTreeMap::new(),
            write_size: WriteBatchSize::default(),
            buffered_bytes: 0,
            owner,
            released: false,
        })
    }

    /// Physical sequence captured when the transaction began.
    pub fn snapshot(&self) -> Snapshot {
        self.snapshot
    }

    /// Number of distinct keys currently present in the local write set.
    pub fn write_count(&self) -> usize {
        self.writes.len()
    }

    /// Borrow final pending mutations in key order without creating a second owner.
    pub fn pending_prefix<'a>(
        &'a self,
        prefix: &'a [u8],
    ) -> impl Iterator<Item = (&'a [u8], Option<&'a [u8]>)> + 'a {
        self.writes
            .range::<[u8], _>((Included(prefix), std::ops::Bound::Unbounded))
            .take_while(move |(key, _)| key.starts_with(prefix))
            .map(|(key, value)| (key.as_slice(), value.as_deref()))
    }

    /// Reads one key from the local write set or the transaction's immutable
    /// database snapshot.
    pub fn get(&self, database: &Database, key: &[u8]) -> Result<Option<Vec<u8>>> {
        validate_key(key)?;
        database.validate_transaction_owner(self)?;
        match self.writes.get(key) {
            Some(value) => Ok(value.clone()),
            None => database.get(key, self.snapshot),
        }
    }

    /// Reads at most `limit` visible rows from `[start, end)` in byte order,
    /// with pending puts and deletes overlaid on the captured snapshot.
    /// Physical work uses projected-read budgets; returned payload and vector
    /// representation fit the configured projected-output ceiling. Returned
    /// vectors are caller-owned, separate from live source-buffer reservations.
    pub fn scan(
        &self,
        database: &Database,
        start: &[u8],
        end: &[u8],
        limit: usize,
    ) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        scan::bounded(self, database, start, end, limit)
    }

    /// Buffers a value. Repeated mutations of the same key collapse to the
    /// final transaction-local value before conflict validation and WAL write.
    pub fn put(&mut self, key: Vec<u8>, value: Vec<u8>) -> Result<()> {
        let size = self.write_size.with_replacement(
            &key,
            self.writes.get(&key).map(|value| value.as_deref()),
            Some(&value),
        )?;
        let reserved = self.reserve_replacement(&key, Some(&value))?;
        self.writes.insert(key, Some(value));
        self.write_size = size;
        self.finish_replacement(reserved)
    }

    /// Buffers a tombstone for one key.
    pub fn delete(&mut self, key: Vec<u8>) -> Result<()> {
        let size = self.write_size.with_replacement(
            &key,
            self.writes.get(&key).map(|value| value.as_deref()),
            None,
        )?;
        let reserved = self.reserve_replacement(&key, None)?;
        self.writes.insert(key, None);
        self.write_size = size;
        self.finish_replacement(reserved)
    }

    fn reserve_replacement(&mut self, key: &Vec<u8>, value: Option<&Vec<u8>>) -> Result<usize> {
        let (old, capacity) = match self.writes.get_key_value(key) {
            Some((stored, prior)) => (
                buffer_charge(stored.capacity(), prior.as_ref())?,
                stored.capacity(),
            ),
            None => (0, key.capacity()),
        };
        let replacement = buffer_charge(capacity, value)?;
        let next = self
            .buffered_bytes
            .checked_sub(old)
            .and_then(|v| v.checked_add(replacement))
            .ok_or_else(|| {
                Error::InvalidTransaction("transaction replacement charge overflow".into())
            })?;
        if next > self.buffered_bytes {
            self.owner.resize(self.buffered_bytes, next)?;
            self.buffered_bytes = next;
        }
        Ok(next)
    }

    fn finish_replacement(&mut self, next: usize) -> Result<()> {
        // Old buffers have actually been dropped by insertion before shrinking.
        if next < self.buffered_bytes {
            self.owner.resize(self.buffered_bytes, next)?;
            self.buffered_bytes = next;
        }
        Ok(())
    }

    /// Discards every pending mutation and releases the pinned snapshot.
    pub fn rollback(mut self) -> Result<TransactionRollback> {
        let outcome = TransactionRollback {
            snapshot_sequence: self.snapshot.sequence,
            discarded_mutations: self.writes.len(),
        };
        self.release()?;
        tracing::debug!(
            target: "rrd_lsm::transaction",
            snapshot_sequence = outcome.snapshot_sequence,
            discarded_mutations = outcome.discarded_mutations,
            outcome = "rolled_back",
            "physical transaction rolled back"
        );
        Ok(outcome)
    }

    pub(crate) fn belongs_to(&self, owner: &Arc<TransactionSnapshots>) -> bool {
        Arc::ptr_eq(&self.owner, owner)
    }

    pub(crate) fn pending(&self) -> &BTreeMap<Vec<u8>, Option<Vec<u8>>> {
        &self.writes
    }

    pub(crate) fn take_mutations(&mut self) -> Vec<Mutation> {
        self.write_size = WriteBatchSize::default();
        std::mem::take(&mut self.writes)
            .into_iter()
            .map(|(key, value)| match value {
                Some(value) => Mutation::Put { key, value },
                None => Mutation::Delete { key },
            })
            .collect()
    }

    fn release(&mut self) -> Result<()> {
        if !self.released {
            self.writes.clear();
            self.owner
                .release(self.snapshot.sequence, self.buffered_bytes)?;
            self.released = true;
        }
        Ok(())
    }
}

impl Drop for Transaction {
    fn drop(&mut self) {
        if !self.released {
            self.writes.clear();
            self.owner
                .release_after_panic(self.snapshot.sequence, self.buffered_bytes);
            self.released = true;
        }
    }
}

/// Result of a successful physical transaction commit. A read-only commit has
/// no WAL receipt and does not allocate a sequence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransactionCommit {
    pub snapshot_sequence: u64,
    pub mutation_count: usize,
    pub receipt: Option<AppendReceipt>,
}

/// Result of explicit rollback. Dropping a transaction has identical state
/// effects but intentionally produces no receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransactionRollback {
    pub snapshot_sequence: u64,
    pub discarded_mutations: usize,
}

#[cfg(test)]
mod ownership_tests {
    use super::*;

    #[test]
    fn poisoned_registry_drop_releases_the_original_owner_once() {
        let owner = Arc::new(TransactionSnapshots::new(TransactionLimits::default()));
        let mut transaction =
            Transaction::begin(Snapshot { sequence: 0 }, Arc::clone(&owner)).unwrap();
        transaction.put(b"key".to_vec(), vec![1; 128]).unwrap();
        let panic = std::panic::catch_unwind(|| {
            let _guard = owner.state.lock().unwrap();
            panic!("owned registry unwind");
        });
        assert!(panic.is_err());
        drop(transaction);
        let state = owner.state.lock().unwrap_err().into_inner();
        assert_eq!((state.active, state.bytes), (0, 0));
        assert!(state.pins.is_empty());
    }
}
