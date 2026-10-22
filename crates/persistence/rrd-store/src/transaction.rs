//! Profile-neutral transaction contract for rrflowMX and rrflowKV.
//!
//! This is the physical byte transaction used by semantic repositories below
//! `RrdEngine`. It is intentionally limited to a captured snapshot, point and
//! bounded range reads, a local put/delete write set, commit, rollback, and
//! write-conflict reporting. Authentication, authorization, schema semantics,
//! graph/index fan-out, and idempotency receipts remain engine/repository work;
//! clients and adapters must not construct physical keys or invoke this port.

use crate::{Durability, Error, Result};
use rrd_lsm::{Mutation, WriteBatch, WriteBatchSize};
use std::collections::BTreeMap;

pub(crate) fn validate_read_key(key: &[u8]) -> Result<()> {
    Mutation::Delete { key: key.to_vec() }
        .validate()
        .map_err(Error::from)
}

pub(crate) fn validate_range(start: &[u8], end: &[u8], limit: usize) -> Result<()> {
    if start >= end {
        return Err(Error::Substrate(
            "transaction range start must sort before its exclusive end".into(),
        ));
    }
    if limit == 0 {
        return Err(Error::Substrate(
            "transaction range result limit must be greater than zero".into(),
        ));
    }
    Ok(())
}

/// One open snapshot-isolated byte transaction.
pub trait StorageTransaction: Send {
    /// Sequence captured when this transaction began.
    fn snapshot_sequence(&self) -> u64;

    /// Reads one key through the transaction's immutable snapshot and local
    /// write overlay.
    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>>;

    /// Reads at most `limit` visible rows from `[start, end)` in byte order.
    fn scan(&self, start: &[u8], end: &[u8], limit: usize) -> Result<Vec<(Vec<u8>, Vec<u8>)>>;

    /// Buffers or replaces the transaction-local value for one key.
    fn put(&mut self, key: Vec<u8>, value: Vec<u8>) -> Result<()>;

    /// Buffers a transaction-local tombstone for one key.
    fn delete(&mut self, key: Vec<u8>) -> Result<()>;

    /// Conflict-checks and atomically publishes the final write set. Consuming
    /// `self` makes every transaction single-use on success or failure.
    fn commit(self: Box<Self>, durability: Durability) -> Result<TransactionCommit>;

    /// Discards the write set. Dropping an open transaction has the same state
    /// effect but does not return a receipt.
    fn rollback(self: Box<Self>) -> Result<TransactionRollback>;
}

/// Profile-neutral commit result. rrflowMX and rrflowKV expose the same
/// sequence interval; durability/reopen proof is asserted separately because
/// rrflowMX is intentionally volatile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransactionCommit {
    pub snapshot_sequence: u64,
    pub mutation_count: usize,
    pub first_sequence: Option<u64>,
    pub last_sequence: Option<u64>,
}

/// Explicit rollback result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransactionRollback {
    pub snapshot_sequence: u64,
    pub discarded_mutations: usize,
}

#[derive(Debug, Default)]
pub(crate) struct TransactionWriteSet {
    writes: BTreeMap<Vec<u8>, Option<Vec<u8>>>,
    write_size: WriteBatchSize,
}

impl TransactionWriteSet {
    pub(crate) fn len(&self) -> usize {
        self.writes.len()
    }

    pub(crate) fn get(&self, key: &[u8]) -> Option<Option<&[u8]>> {
        self.writes.get(key).map(|value| value.as_deref())
    }

    pub(crate) fn put(&mut self, key: Vec<u8>, value: Vec<u8>) -> Result<()> {
        let size = self.write_size.with_replacement(
            &key,
            self.writes.get(&key).map(|value| value.as_deref()),
            Some(&value),
        )?;
        self.writes.insert(key, Some(value));
        self.write_size = size;
        Ok(())
    }

    pub(crate) fn delete(&mut self, key: Vec<u8>) -> Result<()> {
        let size = self.write_size.with_replacement(
            &key,
            self.writes.get(&key).map(|value| value.as_deref()),
            None,
        )?;
        self.writes.insert(key, None);
        self.write_size = size;
        Ok(())
    }

    pub(crate) fn into_mutations(self) -> BTreeMap<Vec<u8>, Option<Vec<u8>>> {
        self.writes
    }

    pub(crate) fn mutations(&self) -> &BTreeMap<Vec<u8>, Option<Vec<u8>>> {
        &self.writes
    }

    pub(crate) fn into_batch(self) -> Result<WriteBatch> {
        let mutations = self
            .writes
            .into_iter()
            .map(|(key, value)| match value {
                Some(value) => Mutation::Put { key, value },
                None => Mutation::Delete { key },
            })
            .collect();
        WriteBatch::new(mutations).map_err(Error::from)
    }
}
