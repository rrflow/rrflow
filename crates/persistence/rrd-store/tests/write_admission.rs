use rrd_store::{Durability, Error, RrflowKvStore, RrflowMxStore, StorageEngine};

const LARGE: usize = 8 * 1024 * 1024;
const SECOND: usize = 16 * 1024 * 1024 - LARGE - 34;

fn assert_limit(error: Error) {
    assert!(matches!(error, Error::WriteBatchLimit {
        resource: rrd_lsm::WriteBatchResource::EncodedBytes, limit, observed,
    } if limit == 16 * 1024 * 1024 && observed == limit + 1));
}

fn exercise(storage: &dyn StorageEngine) {
    let mut transaction = storage.begin_transaction().unwrap();
    transaction.put(b"a".to_vec(), vec![1; LARGE]).unwrap();
    assert_limit(
        transaction
            .put(b"b".to_vec(), vec![2; SECOND + 1])
            .unwrap_err(),
    );
    assert_eq!(transaction.get(b"a").unwrap(), Some(vec![1; LARGE]));
    assert_eq!(transaction.get(b"b").unwrap(), None);
    assert_eq!(transaction.rollback().unwrap().discarded_mutations, 1);
    let observer = storage.begin_transaction().unwrap();
    assert_eq!(observer.get(b"a").unwrap(), None);
    assert_eq!(observer.get(b"b").unwrap(), None);
    observer.rollback().unwrap();
    let mut transaction = storage.begin_transaction().unwrap();
    transaction.put(b"a".to_vec(), vec![1; LARGE]).unwrap();
    for _ in 0..3 {
        transaction.put(b"b".to_vec(), vec![2; SECOND]).unwrap();
        assert_limit(
            transaction
                .put(b"b".to_vec(), vec![3; SECOND + 1])
                .unwrap_err(),
        );
        assert_eq!(transaction.get(b"b").unwrap(), Some(vec![2; SECOND]));
        transaction.delete(b"b".to_vec()).unwrap();
    }
    transaction.put(b"b".to_vec(), vec![4; SECOND]).unwrap();
    assert_eq!(
        transaction
            .commit(Durability::Authoritative)
            .unwrap()
            .mutation_count,
        2
    );
    verify(storage);
}

fn verify(storage: &dyn StorageEngine) {
    let transaction = storage.begin_transaction().unwrap();
    assert_eq!(transaction.get(b"a").unwrap(), Some(vec![1; LARGE]));
    assert_eq!(transaction.get(b"b").unwrap(), Some(vec![4; SECOND]));
}

#[test]
fn mx_staging_rejects_without_partial_overlay_or_lost_capacity() {
    exercise(&RrflowMxStore::new());
}

#[test]
fn kv_staging_rejects_without_partial_overlay_and_reopens_exactly() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("kv");
    {
        let storage = RrflowKvStore::open(&root).unwrap();
        exercise(&storage);
    }
    verify(&RrflowKvStore::open(&root).unwrap());
}
