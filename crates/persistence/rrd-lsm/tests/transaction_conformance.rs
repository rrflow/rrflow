use rrd_lsm::{Database, Durability, Error};

fn create_database(parent: &tempfile::TempDir, name: &str) -> Database {
    Database::create(&parent.path().join(name)).unwrap()
}

fn seed(database: &mut Database, entries: &[(&[u8], &[u8])]) {
    let mut transaction = database.begin_transaction().unwrap();
    for (key, value) in entries {
        transaction.put(key.to_vec(), value.to_vec()).unwrap();
    }
    database
        .commit_transaction(transaction, Durability::Authoritative)
        .unwrap();
}

#[test]
fn aggregate_transaction_admission_preserves_buffers_and_releases_actual_owners() {
    let parent = tempfile::tempdir().unwrap();
    let root = parent.path().join("aggregate");
    let options = rrd_lsm::DatabaseOptions {
        transaction_limits: rrd_lsm::TransactionLimits {
            active: 2,
            buffered_bytes: 1024,
        },
        ..Default::default()
    };
    let mut database = Database::create_with_options(&root, options).unwrap();
    let mut first = database.begin_transaction().unwrap();
    let mut second = database.begin_transaction().unwrap();
    assert!(matches!(
        database.begin_transaction(),
        Err(Error::TransactionLimit {
            resource: rrd_lsm::TransactionResource::Active,
            limit: 2,
            observed: 3
        })
    ));
    first.put(b"a".to_vec(), b"prior".to_vec()).unwrap();
    let prior = database.transaction_ownership().unwrap();
    let mut oversized_capacity = Vec::with_capacity(2048);
    oversized_capacity.push(7);
    assert!(matches!(
        first.put(b"a".to_vec(), oversized_capacity),
        Err(Error::TransactionLimit {
            resource: rrd_lsm::TransactionResource::BufferedBytes,
            ..
        })
    ));
    assert_eq!(database.transaction_ownership().unwrap(), prior);
    assert_eq!(first.get(&database, b"a").unwrap(), Some(b"prior".to_vec()));
    first.put(b"a".to_vec(), vec![1; 512]).unwrap();
    assert!(second.put(b"b".to_vec(), vec![2; 512]).is_err());
    assert_eq!(second.write_count(), 0);
    first.delete(b"a".to_vec()).unwrap();
    second.put(b"b".to_vec(), vec![2; 512]).unwrap();
    assert!(database.transaction_ownership().unwrap().1 <= 1024);
    first.rollback().unwrap();
    assert_eq!(database.transaction_ownership().unwrap().0, 1);
    database
        .commit_transaction(second, Durability::Authoritative)
        .unwrap();
    assert_eq!(database.transaction_ownership().unwrap(), (0, 0));
    assert_eq!(
        database.get(b"b", database.snapshot()).unwrap(),
        Some(vec![2; 512])
    );
    drop(database);
    let reopened = Database::open_with_options(&root, options).unwrap();
    let first = reopened.begin_transaction().unwrap();
    let second = reopened.begin_transaction().unwrap();
    assert!(reopened.begin_transaction().is_err());
    drop(first);
    drop(second);
    assert_eq!(reopened.transaction_ownership().unwrap(), (0, 0));
}

#[test]
fn conflict_and_ordinary_drop_release_transaction_buffer_reservations() {
    let parent = tempfile::tempdir().unwrap();
    let mut database = create_database(&parent, "release");
    let mut first = database.begin_transaction().unwrap();
    let mut second = database.begin_transaction().unwrap();
    first.put(b"same".to_vec(), b"accepted".to_vec()).unwrap();
    second.put(b"same".to_vec(), b"conflict".to_vec()).unwrap();
    database
        .commit_transaction(first, Durability::Authoritative)
        .unwrap();
    assert_eq!(database.transaction_ownership().unwrap().0, 1);
    assert!(matches!(
        database.commit_transaction(second, Durability::Authoritative),
        Err(Error::TransactionConflict { .. })
    ));
    assert_eq!(database.transaction_ownership().unwrap(), (0, 0));
    let mut dropped = database.begin_transaction().unwrap();
    dropped.put(b"abandoned".to_vec(), vec![3; 256]).unwrap();
    drop(dropped);
    assert_eq!(database.transaction_ownership().unwrap(), (0, 0));
    assert!(database
        .get(b"abandoned", database.snapshot())
        .unwrap()
        .is_none());
    assert_eq!(
        database.get(b"same", database.snapshot()).unwrap(),
        Some(b"accepted".to_vec())
    );
}

#[test]
fn pending_prefix_exposes_final_checkpoint_intent_without_other_keyspaces() {
    let parent = tempfile::tempdir().unwrap();
    let database = create_database(&parent, "intent");
    let mut transaction = database.begin_transaction().unwrap();
    transaction
        .put(b"snapshot:z".to_vec(), b"old".to_vec())
        .unwrap();
    transaction
        .put(b"other:a".to_vec(), b"outside".to_vec())
        .unwrap();
    transaction
        .put(b"snapshot:a".to_vec(), b"removed".to_vec())
        .unwrap();
    transaction.delete(b"snapshot:a".to_vec()).unwrap();
    transaction
        .put(b"snapshot:z".to_vec(), b"final".to_vec())
        .unwrap();
    let before = database.transaction_ownership().unwrap();
    assert_eq!(
        transaction.pending_prefix(b"snapshot:").collect::<Vec<_>>(),
        vec![
            (b"snapshot:a".as_slice(), None),
            (b"snapshot:z".as_slice(), Some(b"final".as_slice()))
        ]
    );
    assert_eq!(database.transaction_ownership().unwrap(), before);
    assert_eq!(transaction.pending_prefix(b"absent:").count(), 0);
    transaction.rollback().unwrap();
    assert_eq!(database.transaction_ownership().unwrap(), (0, 0));
}

#[test]
fn point_and_bounded_range_reads_overlay_the_ordered_write_set() {
    let parent = tempfile::tempdir().unwrap();
    let mut database = create_database(&parent, "point-range");
    seed(
        &mut database,
        &[
            (b"key:a", b"old-a"),
            (b"key:b", b"old-b"),
            (b"key:d", b"old-d"),
        ],
    );

    let mut transaction = database.begin_transaction().unwrap();
    transaction
        .put(b"key:a".to_vec(), b"new-a".to_vec())
        .unwrap();
    transaction.delete(b"key:b".to_vec()).unwrap();
    transaction
        .put(b"key:c".to_vec(), b"new-c".to_vec())
        .unwrap();
    transaction
        .put(b"key:c".to_vec(), b"final-c".to_vec())
        .unwrap();

    assert_eq!(
        transaction.get(&database, b"key:a").unwrap(),
        Some(b"new-a".to_vec())
    );
    assert_eq!(transaction.get(&database, b"key:b").unwrap(), None);
    assert_eq!(
        transaction.scan(&database, b"key:a", b"key:z", 3).unwrap(),
        vec![
            (b"key:a".to_vec(), b"new-a".to_vec()),
            (b"key:c".to_vec(), b"final-c".to_vec()),
            (b"key:d".to_vec(), b"old-d".to_vec()),
        ]
    );
    assert_eq!(transaction.write_count(), 3);

    let outcome = database
        .commit_transaction(transaction, Durability::Authoritative)
        .unwrap();
    assert_eq!(outcome.mutation_count, 3);
    assert!(outcome.receipt.is_some_and(|receipt| receipt.durable));
    let snapshot = database.snapshot();
    assert_eq!(
        database.get(b"key:a", snapshot).unwrap(),
        Some(b"new-a".to_vec())
    );
    assert_eq!(database.get(b"key:b", snapshot).unwrap(), None);
    assert_eq!(
        database.get(b"key:c", snapshot).unwrap(),
        Some(b"final-c".to_vec())
    );
}

#[test]
fn snapshot_reads_are_repeatable_and_range_phantoms_are_not_conflicts() {
    let parent = tempfile::tempdir().unwrap();
    let mut database = create_database(&parent, "repeatable-range");
    seed(&mut database, &[(b"item:a", b"one"), (b"item:c", b"three")]);

    let mut observer = database.begin_transaction().unwrap();
    let before = observer.scan(&database, b"item:", b"item;", 16).unwrap();

    let mut concurrent = database.begin_transaction().unwrap();
    concurrent.put(b"item:b".to_vec(), b"two".to_vec()).unwrap();
    database
        .commit_transaction(concurrent, Durability::Authoritative)
        .unwrap();

    assert_eq!(
        observer.scan(&database, b"item:", b"item;", 16).unwrap(),
        before,
        "a transaction never observes a range phantom after its snapshot"
    );
    observer
        .put(b"decision:observer".to_vec(), b"accepted".to_vec())
        .unwrap();
    database
        .commit_transaction(observer, Durability::Authoritative)
        .unwrap();

    let current = database.begin_transaction().unwrap();
    assert_eq!(
        current.scan(&database, b"item:", b"item;", 16).unwrap(),
        vec![
            (b"item:a".to_vec(), b"one".to_vec()),
            (b"item:b".to_vec(), b"two".to_vec()),
            (b"item:c".to_vec(), b"three".to_vec()),
        ]
    );
}

#[test]
fn same_key_put_delete_and_delete_put_conflict_at_commit() {
    let parent = tempfile::tempdir().unwrap();
    let mut database = create_database(&parent, "write-conflicts");
    seed(&mut database, &[(b"shared", b"initial")]);

    let mut update = database.begin_transaction().unwrap();
    let mut delete = database.begin_transaction().unwrap();
    update.put(b"shared".to_vec(), b"updated".to_vec()).unwrap();
    delete.delete(b"shared".to_vec()).unwrap();
    database
        .commit_transaction(update, Durability::Authoritative)
        .unwrap();
    assert!(matches!(
        database.commit_transaction(delete, Durability::Authoritative),
        Err(Error::TransactionConflict { .. })
    ));

    let mut delete = database.begin_transaction().unwrap();
    let mut update = database.begin_transaction().unwrap();
    delete.delete(b"shared".to_vec()).unwrap();
    update
        .put(b"shared".to_vec(), b"resurrected".to_vec())
        .unwrap();
    database
        .commit_transaction(delete, Durability::Authoritative)
        .unwrap();
    assert!(matches!(
        database.commit_transaction(update, Durability::Authoritative),
        Err(Error::TransactionConflict { .. })
    ));
}

#[test]
fn a_conflicted_logical_operation_can_retry_from_a_fresh_snapshot_once() {
    let parent = tempfile::tempdir().unwrap();
    let mut database = create_database(&parent, "retry");
    seed(&mut database, &[(b"counter", b"0")]);

    let mut stale = database.begin_transaction().unwrap();
    let mut winner = database.begin_transaction().unwrap();
    stale.put(b"counter".to_vec(), b"1".to_vec()).unwrap();
    winner.put(b"counter".to_vec(), b"1".to_vec()).unwrap();
    database
        .commit_transaction(winner, Durability::Authoritative)
        .unwrap();
    assert!(matches!(
        database.commit_transaction(stale, Durability::Authoritative),
        Err(Error::TransactionConflict { .. })
    ));

    let mut retry = database.begin_transaction().unwrap();
    assert_eq!(
        retry.get(&database, b"counter").unwrap(),
        Some(b"1".to_vec())
    );
    retry.put(b"counter".to_vec(), b"2".to_vec()).unwrap();
    let outcome = database
        .commit_transaction(retry, Durability::Authoritative)
        .unwrap();
    assert_eq!(outcome.mutation_count, 1);
    assert_eq!(
        database.get(b"counter", database.snapshot()).unwrap(),
        Some(b"2".to_vec())
    );
}

#[test]
fn rollback_and_read_only_commit_publish_no_wal_mutation() {
    let parent = tempfile::tempdir().unwrap();
    let mut database = create_database(&parent, "rollback");
    seed(&mut database, &[(b"stable", b"value")]);
    let before = database.snapshot();

    let mut discarded = database.begin_transaction().unwrap();
    discarded
        .put(b"discarded".to_vec(), b"value".to_vec())
        .unwrap();
    let rollback = discarded.rollback().unwrap();
    assert_eq!(rollback.snapshot_sequence, before.sequence);
    assert_eq!(rollback.discarded_mutations, 1);
    assert_eq!(database.snapshot(), before);
    assert_eq!(database.get(b"discarded", before).unwrap(), None);

    let read_only = database.begin_transaction().unwrap();
    assert_eq!(
        read_only.get(&database, b"stable").unwrap(),
        Some(b"value".to_vec())
    );
    let committed = database
        .commit_transaction(read_only, Durability::Authoritative)
        .unwrap();
    assert_eq!(committed.snapshot_sequence, before.sequence);
    assert_eq!(committed.mutation_count, 0);
    assert!(committed.receipt.is_none());
    assert_eq!(database.snapshot(), before);
}

#[test]
fn write_skew_is_permitted_and_therefore_serializability_is_not_claimed() {
    let parent = tempfile::tempdir().unwrap();
    let mut database = create_database(&parent, "write-skew");
    seed(
        &mut database,
        &[(b"doctor:alice", b"on"), (b"doctor:bob", b"on")],
    );

    let mut alice = database.begin_transaction().unwrap();
    let mut bob = database.begin_transaction().unwrap();
    assert_eq!(
        alice.get(&database, b"doctor:bob").unwrap(),
        Some(b"on".to_vec())
    );
    assert_eq!(
        bob.get(&database, b"doctor:alice").unwrap(),
        Some(b"on".to_vec())
    );
    alice
        .put(b"doctor:alice".to_vec(), b"off".to_vec())
        .unwrap();
    bob.put(b"doctor:bob".to_vec(), b"off".to_vec()).unwrap();

    database
        .commit_transaction(alice, Durability::Authoritative)
        .unwrap();
    database
        .commit_transaction(bob, Durability::Authoritative)
        .unwrap();
    let snapshot = database.snapshot();
    assert_eq!(
        database.get(b"doctor:alice", snapshot).unwrap(),
        Some(b"off".to_vec())
    );
    assert_eq!(
        database.get(b"doctor:bob", snapshot).unwrap(),
        Some(b"off".to_vec())
    );
}

#[test]
fn active_transactions_pin_history_through_compaction() {
    let parent = tempfile::tempdir().unwrap();
    let mut database = create_database(&parent, "transaction-pin");
    seed(&mut database, &[(b"versioned", b"old")]);
    let historical = database.begin_transaction().unwrap();

    let mut update = database.begin_transaction().unwrap();
    update.put(b"versioned".to_vec(), b"new".to_vec()).unwrap();
    database
        .commit_transaction(update, Durability::Authoritative)
        .unwrap();
    let outcome = database.compact(&[], 100).unwrap().unwrap();
    assert!(outcome
        .protected_sequences
        .contains(&historical.snapshot().sequence));
    assert_eq!(
        historical.get(&database, b"versioned").unwrap(),
        Some(b"old".to_vec())
    );
    historical.rollback().unwrap();
}

#[test]
fn transactions_cannot_cross_database_instances_and_invalid_ranges_fail_closed() {
    let parent = tempfile::tempdir().unwrap();
    let first = create_database(&parent, "first");
    let second = create_database(&parent, "second");
    let transaction = first.begin_transaction().unwrap();
    assert!(matches!(
        transaction.get(&second, b"key"),
        Err(Error::TransactionDatabaseMismatch)
    ));
    assert!(matches!(
        transaction.scan(&first, b"z", b"a", 1),
        Err(Error::InvalidTransaction(_))
    ));
    assert!(matches!(
        transaction.scan(&first, b"a", b"z", 0),
        Err(Error::InvalidTransaction(_))
    ));
}

#[test]
fn authoritative_transaction_reopens_at_the_committed_value() {
    let parent = tempfile::tempdir().unwrap();
    let root = parent.path().join("reopen");
    let committed_sequence = {
        let mut database = Database::create(&root).unwrap();
        let mut transaction = database.begin_transaction().unwrap();
        transaction
            .put(b"durable".to_vec(), b"value".to_vec())
            .unwrap();
        database
            .commit_transaction(transaction, Durability::Authoritative)
            .unwrap()
            .receipt
            .unwrap()
            .last_sequence
    };
    let reopened = Database::open(&root).unwrap();
    assert_eq!(reopened.snapshot().sequence, committed_sequence);
    assert_eq!(
        reopened.get(b"durable", reopened.snapshot()).unwrap(),
        Some(b"value".to_vec())
    );
}
