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
use std::ops::Bound::{Excluded, Included};
use std::sync::{Arc, Mutex};

#[derive(Debug, Default)]
pub(crate) struct TransactionSnapshots {
    active: Mutex<BTreeMap<u64, usize>>,
}

impl TransactionSnapshots {
    fn acquire(&self, sequence: u64) -> Result<()> {
        let mut active = self
            .active
            .lock()
            .map_err(|_| Error::PoisonedTransactionState)?;
        let count = active.entry(sequence).or_default();
        *count = count
            .checked_add(1)
            .ok_or_else(|| Error::InvalidTransaction("snapshot pin count overflowed".into()))?;
        Ok(())
    }

    fn release(&self, sequence: u64) -> Result<()> {
        let mut active = self
            .active
            .lock()
            .map_err(|_| Error::PoisonedTransactionState)?;
        release_sequence(&mut active, sequence);
        Ok(())
    }

    fn release_after_panic(&self, sequence: u64) {
        let mut active = self
            .active
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        release_sequence(&mut active, sequence);
    }

    pub(crate) fn sequences(&self) -> Result<Vec<u64>> {
        Ok(self
            .active
            .lock()
            .map_err(|_| Error::PoisonedTransactionState)?
            .keys()
            .copied()
            .collect())
    }

    pub(crate) fn is_empty(&self) -> Result<bool> {
        Ok(self
            .active
            .lock()
            .map_err(|_| Error::PoisonedTransactionState)?
            .is_empty())
    }
}

fn release_sequence(active: &mut BTreeMap<u64, usize>, sequence: u64) {
    let Some(count) = active.get_mut(&sequence) else {
        return;
    };
    if *count == 1 {
        active.remove(&sequence);
    } else {
        *count -= 1;
    }
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
    pub fn scan(
        &self,
        database: &Database,
        start: &[u8],
        end: &[u8],
        limit: usize,
    ) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        if start >= end {
            return Err(Error::InvalidTransaction(
                "range start must sort before its exclusive end".into(),
            ));
        }
        if limit == 0 {
            return Err(Error::InvalidTransaction(
                "range result limit must be greater than zero".into(),
            ));
        }
        database.validate_transaction_owner(self)?;
        let mut visible = database
            .scan(start, Some(end), self.snapshot)?
            .into_iter()
            .collect::<BTreeMap<_, _>>();
        for (key, value) in self
            .writes
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

    /// Buffers a value. Repeated mutations of the same key collapse to the
    /// final transaction-local value before conflict validation and WAL write.
    pub fn put(&mut self, key: Vec<u8>, value: Vec<u8>) -> Result<()> {
        let size = self.write_size.with_replacement(
            &key,
            self.writes.get(&key).map(|value| value.as_deref()),
            Some(&value),
        )?;
        self.writes.insert(key, Some(value));
        self.write_size = size;
        Ok(())
    }

    /// Buffers a tombstone for one key.
    pub fn delete(&mut self, key: Vec<u8>) -> Result<()> {
        let size = self.write_size.with_replacement(
            &key,
            self.writes.get(&key).map(|value| value.as_deref()),
            None,
        )?;
        self.writes.insert(key, None);
        self.write_size = size;
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
            self.owner.release(self.snapshot.sequence)?;
            self.released = true;
        }
        Ok(())
    }
}

impl Drop for Transaction {
    fn drop(&mut self) {
        if !self.released {
            self.owner.release_after_panic(self.snapshot.sequence);
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
