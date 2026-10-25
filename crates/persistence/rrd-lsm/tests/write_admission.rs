use rrd_lsm::{Database, Durability, Error, WriteBatchResource, WAL_MAX_PAYLOAD_BYTES};

const LARGE: usize = 8 * 1024 * 1024;
// Frozen batch-v2 envelope: 16-byte header, two 8-byte operation headers,
// and two one-byte keys. This expectation is independent of the accountant.
const SECOND: usize = WAL_MAX_PAYLOAD_BYTES - LARGE - 34;

fn assert_limit(error: Error) {
    assert!(matches!(error, Error::WriteBatchLimit {
        resource: WriteBatchResource::EncodedBytes, limit, observed,
    } if limit == WAL_MAX_PAYLOAD_BYTES as u64 && observed == limit + 1));
}

#[test]
fn staged_limit_rejection_preserves_values_and_capacity_through_reopen() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("admission");
    let mut database = Database::create(&root).unwrap();
    let before = database.snapshot();
    let mut transaction = database.begin_transaction().unwrap();
    transaction.put(b"a".to_vec(), vec![1; LARGE]).unwrap();
    assert_limit(
        transaction
            .put(b"b".to_vec(), vec![2; SECOND + 1])
            .unwrap_err(),
    );
    assert_eq!(transaction.write_count(), 1);
    assert_eq!(
        transaction.get(&database, b"a").unwrap(),
        Some(vec![1; LARGE])
    );
    assert_eq!(transaction.get(&database, b"b").unwrap(), None);
    assert_eq!(database.snapshot(), before);
    for _ in 0..3 {
        transaction.put(b"b".to_vec(), vec![2; SECOND]).unwrap();
        assert_limit(
            transaction
                .put(b"b".to_vec(), vec![3; SECOND + 1])
                .unwrap_err(),
        );
        assert_eq!(
            transaction.get(&database, b"b").unwrap(),
            Some(vec![2; SECOND])
        );
        transaction.delete(b"b".to_vec()).unwrap();
    }
    transaction.put(b"b".to_vec(), vec![4; SECOND]).unwrap();
    let receipt = database
        .commit_transaction(transaction, Durability::Authoritative)
        .unwrap();
    assert_eq!(receipt.mutation_count, 2);
    drop(database);
    let reopened = Database::open(&root).unwrap();
    assert_eq!(
        reopened.get(b"a", reopened.snapshot()).unwrap(),
        Some(vec![1; LARGE])
    );
    assert_eq!(
        reopened.get(b"b", reopened.snapshot()).unwrap(),
        Some(vec![4; SECOND])
    );
}

#[test]
fn rejected_staging_can_rollback_without_publishing_any_key() {
    let directory = tempfile::tempdir().unwrap();
    let database = Database::create(&directory.path().join("rollback")).unwrap();
    let mut transaction = database.begin_transaction().unwrap();
    transaction.put(b"a".to_vec(), vec![1; LARGE]).unwrap();
    assert_limit(
        transaction
            .put(b"b".to_vec(), vec![2; SECOND + 1])
            .unwrap_err(),
    );
    assert_eq!(transaction.rollback().unwrap().discarded_mutations, 1);
    assert_eq!(database.get(b"a", database.snapshot()).unwrap(), None);
    assert_eq!(database.get(b"b", database.snapshot()).unwrap(), None);
}
