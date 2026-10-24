//! Read-only repository access through one admitted physical transaction.
//! The transaction retains its native snapshot pin until this view drops.
use crate::{
    Durability, Error, Result, StorageEngine, StorageTransaction, TransactionCommit,
    TransactionRollback,
};
use std::sync::Mutex;

pub(crate) struct SnapshotStore<'a> {
    transaction: Mutex<Box<dyn StorageTransaction + 'a>>,
    sequence: u64,
}

impl<'a> SnapshotStore<'a> {
    pub(crate) fn capture(source: &'a impl StorageEngine) -> Result<Self> {
        let transaction = source.begin_transaction()?;
        let sequence = transaction.snapshot_sequence();
        Ok(Self {
            transaction: Mutex::new(transaction),
            sequence,
        })
    }
}

struct SnapshotRead<'a, 'source> {
    view: &'a SnapshotStore<'source>,
}

fn read_only() -> Error {
    Error::Archive("captured backup snapshot does not permit mutation".into())
}

impl StorageTransaction for SnapshotRead<'_, '_> {
    fn snapshot_sequence(&self) -> u64 {
        self.view.sequence
    }
    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        self.view
            .transaction
            .lock()
            .map_err(|_| Error::Archive("backup snapshot lock poisoned".into()))?
            .get(key)
    }
    fn scan(&self, start: &[u8], end: &[u8], limit: usize) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        self.view
            .transaction
            .lock()
            .map_err(|_| Error::Archive("backup snapshot lock poisoned".into()))?
            .scan(start, end, limit)
    }
    fn put(&mut self, _key: Vec<u8>, _value: Vec<u8>) -> Result<()> {
        Err(read_only())
    }
    fn delete(&mut self, _key: Vec<u8>) -> Result<()> {
        Err(read_only())
    }
    fn commit(self: Box<Self>, _durability: Durability) -> Result<TransactionCommit> {
        Err(read_only())
    }
    fn rollback(self: Box<Self>) -> Result<TransactionRollback> {
        Ok(TransactionRollback {
            snapshot_sequence: self.view.sequence,
            discarded_mutations: 0,
        })
    }
}

impl StorageEngine for SnapshotStore<'_> {
    fn begin_transaction(&self) -> Result<Box<dyn StorageTransaction + '_>> {
        Ok(Box::new(SnapshotRead { view: self }))
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{RrflowKvStore, RrflowMxStore};

    fn exercise(source: &impl StorageEngine) {
        let mut initial = source.begin_transaction().unwrap();
        initial
            .put(b"snapshot/a".to_vec(), b"before".to_vec())
            .unwrap();
        initial.commit(Durability::Authoritative).unwrap();
        let view = SnapshotStore::capture(source).unwrap();
        let first = view.begin_transaction().unwrap();
        let mut writer = source.begin_transaction().unwrap();
        writer
            .put(b"snapshot/a".to_vec(), b"after".to_vec())
            .unwrap();
        writer.put(b"snapshot/b".to_vec(), b"new".to_vec()).unwrap();
        writer.commit(Durability::Authoritative).unwrap();
        let mut second = view.begin_transaction().unwrap();
        assert_eq!(first.snapshot_sequence(), second.snapshot_sequence());
        assert_eq!(second.get(b"snapshot/a").unwrap(), Some(b"before".to_vec()));
        assert_eq!(
            second.scan(b"snapshot/", b"snapshot0", 8).unwrap(),
            vec![(b"snapshot/a".to_vec(), b"before".to_vec())]
        );
        assert!(second
            .put(b"snapshot/c".to_vec(), b"denied".to_vec())
            .is_err());
        assert!(second.delete(b"snapshot/a".to_vec()).is_err());
        assert!(second.commit(Durability::Authoritative).is_err());
        assert_eq!(first.rollback().unwrap().discarded_mutations, 0);
        drop(view);
        assert_eq!(
            source
                .begin_transaction()
                .unwrap()
                .get(b"snapshot/a")
                .unwrap(),
            Some(b"after".to_vec())
        );
    }

    #[test]
    fn captured_repositories_keep_one_cut_and_deny_effects() {
        exercise(&RrflowMxStore::new());
        let root = tempfile::tempdir().unwrap();
        let source = RrflowKvStore::open(&root.path().join("store")).unwrap();
        exercise(&source);
        assert_eq!(
            source
                .physical_store_evidence()
                .unwrap()
                .native_transactions,
            Some(0)
        );
    }
}
