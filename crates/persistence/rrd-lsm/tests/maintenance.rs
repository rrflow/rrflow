use rrd_lsm::{
    Database, DatabaseOptions, Durability, Error, MaintenancePolicy, Mutation, WriteBatch,
};
use tempfile::tempdir;

fn put(key: &str, value: &str) -> Mutation {
    Mutation::Put {
        key: key.as_bytes().to_vec(),
        value: value.as_bytes().to_vec(),
    }
}

fn options(max_versions: usize) -> DatabaseOptions {
    DatabaseOptions {
        maintenance: MaintenancePolicy {
            wal_payload_max_bytes: usize::MAX,
            memtable_max_versions: max_versions,
        },
        ..DatabaseOptions::default()
    }
}

#[test]
fn the_next_writer_synchronously_flushes_before_crossing_the_mutable_bound() {
    let root = tempdir().unwrap().keep().join("native");
    let mut database = Database::create_with_options(&root, options(2)).unwrap();
    database
        .write_owned(
            WriteBatch::new(vec![put("a", "one"), put("b", "two")]).unwrap(),
            Durability::Authoritative,
        )
        .unwrap();
    assert_eq!(database.manifest().durable_sequence, 0);
    assert_eq!(database.memtable().version_count(), 2);

    database
        .write_owned(
            WriteBatch::new(vec![put("c", "three")]).unwrap(),
            Durability::Authoritative,
        )
        .unwrap();

    assert_eq!(database.manifest().durable_sequence, 2);
    assert_eq!(database.memtable().version_count(), 1);
    assert_eq!(database.maintenance_stats().automatic_flushes, 1);
    assert_eq!(database.maintenance_stats().write_stalls, 1);
    assert_eq!(database.maintenance_stats().failed_flushes, 0);
    let snapshot = database.snapshot();
    assert_eq!(
        database.get(b"a", snapshot).unwrap().as_deref(),
        Some(b"one".as_slice())
    );
    assert_eq!(
        database.get(b"c", snapshot).unwrap().as_deref(),
        Some(b"three".as_slice())
    );

    drop(database);
    let reopened = Database::open_with_options(&root, options(2)).unwrap();
    let snapshot = reopened.snapshot();
    assert_eq!(snapshot.sequence, 3);
    assert_eq!(
        reopened.get(b"b", snapshot).unwrap().as_deref(),
        Some(b"two".as_slice())
    );
    assert_eq!(
        reopened.get(b"c", snapshot).unwrap().as_deref(),
        Some(b"three".as_slice())
    );
}

#[test]
fn one_oversized_atomic_batch_is_retained_and_reported_without_splitting() {
    let root = tempdir().unwrap().keep().join("native");
    let mut database = Database::create_with_options(&root, options(1)).unwrap();
    database
        .write_owned(
            WriteBatch::new(vec![put("a", "one"), put("b", "two")]).unwrap(),
            Durability::Authoritative,
        )
        .unwrap();

    assert_eq!(database.snapshot().sequence, 2);
    assert_eq!(database.memtable().version_count(), 2);
    assert_eq!(database.maintenance_stats().oversized_batches, 1);
    assert_eq!(database.maintenance_stats().automatic_flushes, 0);
}

#[test]
fn zero_maintenance_limits_fail_before_creating_storage() {
    let root = tempdir().unwrap().keep().join("native");
    let error = Database::create_with_options(
        &root,
        DatabaseOptions {
            maintenance: MaintenancePolicy {
                wal_payload_max_bytes: 0,
                memtable_max_versions: 1,
            },
            ..DatabaseOptions::default()
        },
    )
    .err()
    .expect("invalid policy must fail");
    assert!(matches!(error, Error::InvalidConfiguration(_)));
    assert!(!root.exists());
}

#[test]
fn retained_generations_reject_before_wal_and_release_on_actual_owner_drop() {
    use rrd_lsm::{
        MutableResource, ProjectedReadBudget, ProjectedReadProjection, ProjectedReadRange,
        ProjectedReadRequest,
    };
    let directory = tempdir().unwrap();
    let root = directory.path().join("native");
    let mut policy = options(100);
    policy.mutable_retention.generations = 2;
    let mut database = Database::create_with_options(&root, policy).unwrap();
    let memory = database.mutable_memory();
    let write = |value: &str| WriteBatch::new(vec![put("key", value)]).unwrap();
    let reader = |database: &Database| {
        database
            .begin_projected_read(ProjectedReadRequest {
                ranges: vec![ProjectedReadRange::all()],
                projection: ProjectedReadProjection::KeyValue,
                snapshot: database.snapshot(),
                budget: ProjectedReadBudget::default(),
            })
            .unwrap()
    };
    database
        .write_owned(write("one"), Durability::Authoritative)
        .unwrap();
    let mut first = reader(&database);
    database
        .write_owned(write("two"), Durability::Authoritative)
        .unwrap();
    let mut second = reader(&database);
    assert_eq!(memory.stats().generations, 2);
    assert_eq!(memory.stats().versions, 3);
    let wal = root.join("wal").join("00000000000000000001.wal");
    let before = std::fs::read(&wal).unwrap();
    assert!(matches!(
        database.write_owned(write("three"), Durability::Authoritative),
        Err(Error::MutableLimit {
            resource: MutableResource::Generations,
            ..
        })
    ));
    assert_eq!(std::fs::read(&wal).unwrap(), before);
    assert_eq!(database.snapshot().sequence, 2);
    assert_eq!(memory.stats().denied_writes, 1);
    second.cancel().unwrap();
    database
        .write_owned(write("three"), Durability::Authoritative)
        .unwrap();
    assert_eq!(memory.stats().versions, 4);
    drop(database);
    assert_eq!(memory.stats().generations, 1);
    let batch = first.next_batch().unwrap().unwrap();
    assert_eq!(batch.value(0), Some(b"one".as_slice()));
    first.cancel().unwrap();
    assert_eq!(memory.stats().accounted_bytes, 0);
    assert_eq!(batch.value(0), Some(b"one".as_slice()));
    let reopened = Database::open(&root).unwrap();
    assert_eq!(
        reopened.get(b"key", reopened.snapshot()).unwrap(),
        Some(b"three".to_vec())
    );
}

#[test]
fn installed_write_maintenance_reclaims_only_an_admitted_prefix() {
    let directory = tempdir().unwrap();
    let root = directory.path().join("native");
    let mut policy = options(1);
    policy.reclamation.delete_files = 1;
    let mut database = Database::create_with_options(&root, policy).unwrap();
    database
        .write_owned(
            WriteBatch::new(vec![put("key", "one")]).unwrap(),
            Durability::Authoritative,
        )
        .unwrap();
    database.flush_memtable(1).unwrap();
    let pinned = database.checkpoint("retain", 2).unwrap();
    database
        .write_owned(
            WriteBatch::new(vec![put("key", "two")]).unwrap(),
            Durability::Authoritative,
        )
        .unwrap();
    database.flush_memtable(3).unwrap();
    database.compact(&[], 4).unwrap();
    let report = database.garbage_collect().unwrap();
    assert!(
        report.removed_manifests.len() + report.removed_segments.len() + report.removed_wals.len()
            <= 1
    );
    assert!(report.retained_manifests.contains(&pinned.manifest));
    assert!(database.release_checkpoint("retain").unwrap());
    for index in 0..12 {
        database
            .write_owned(
                WriteBatch::new(vec![put("key", &index.to_string())]).unwrap(),
                Durability::Authoritative,
            )
            .unwrap();
    }
    assert!(!root
        .join("manifests")
        .join(format!("{}.json", pinned.manifest))
        .exists());
    assert_eq!(
        database.get(b"key", database.snapshot()).unwrap(),
        Some(b"11".to_vec())
    );
}

#[test]
fn reclamation_inventory_denial_preserves_incoming_wal_and_unreachable_files() {
    let directory = tempdir().unwrap();
    let root = directory.path().join("native");
    let database = Database::create(&root).unwrap();
    drop(database);
    let orphan = root.join("manifests").join("orphan.json");
    std::fs::write(&orphan, b"evidence").unwrap();
    let mut policy = options(100);
    policy.reclamation.inventory_entries = 1;
    let mut database = Database::open_with_options(&root, policy).unwrap();
    let wal = root.join("wal").join("00000000000000000001.wal");
    let before = std::fs::read(&wal).unwrap();
    assert!(matches!(
        database.write_owned(
            WriteBatch::new(vec![put("key", "denied")]).unwrap(),
            Durability::Authoritative
        ),
        Err(Error::ReclamationLimit { .. })
    ));
    assert_eq!(std::fs::read(&wal).unwrap(), before);
    assert_eq!(std::fs::read(&orphan).unwrap(), b"evidence");
    assert_eq!(database.snapshot().sequence, 0);
}
