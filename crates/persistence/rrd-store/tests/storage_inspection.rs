use rrd_store::{DatabaseOptions, Durability, RrflowKvStore, StorageEngine};
use sha2::{Digest, Sha256};

fn page_hash(sequence: u64, ordinal: u64, key: &[u8], value: &[u8]) -> String {
    let mut hash = Sha256::new();
    hash.update(b"rrflow-storage-inspection-page-v1");
    for number in [sequence, ordinal, 1] {
        hash.update(number.to_be_bytes());
    }
    for bytes in [key, value] {
        hash.update((bytes.len() as u64).to_be_bytes());
        hash.update(bytes);
    }
    hash.finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[test]
fn retained_page_and_snapshot_survive_maintenance_and_source_cancellation() {
    let root = tempfile::tempdir().unwrap();
    let store = RrflowKvStore::open(&root.path().join("kv")).unwrap();
    let mut seed = store.begin_transaction().unwrap();
    for index in 0..16 {
        seed.put(format!("a{index:02}").into_bytes(), b"original".to_vec())
            .unwrap();
    }
    seed.commit(Durability::Authoritative).unwrap();
    store.flush(1).unwrap();
    let mut cursor = store.inspect_storage(1, 1024 * 1024).unwrap();
    let original = cursor.page().unwrap();
    assert_eq!(
        original.sha256,
        page_hash(original.sequence, 0, b"a00", b"original")
    );
    assert!(original.source_open);
    assert_eq!(
        store.physical_store_evidence().unwrap().active_read_views,
        Some(1)
    );
    let mut update = store.begin_transaction().unwrap();
    update.put(b"a00".to_vec(), b"updated".to_vec()).unwrap();
    update.delete(b"a01".to_vec()).unwrap();
    update.commit(Durability::Authoritative).unwrap();
    store.flush(2).unwrap();
    store.compact(3, 3).unwrap();
    store.garbage_collect(4, 4).unwrap();
    assert_eq!(cursor.page().unwrap(), original);
    cursor.advance(0).unwrap();
    let second = cursor.page().unwrap();
    assert_eq!(
        second.sha256,
        page_hash(original.sequence, 1, b"a01", b"original")
    );
    cursor.advance(0).unwrap(); // Lost-response retry must not skip a page.
    assert_eq!(cursor.page().unwrap(), second);
    assert!(cursor.advance(99).is_err());
    assert_eq!(cursor.page().unwrap(), second);
    cursor.cancel_source().unwrap();
    let retained = cursor.page().unwrap();
    assert_eq!(retained.sha256, second.sha256);
    assert!(!retained.source_open);
    let counters = store.physical_store_evidence().unwrap();
    assert_eq!(counters.active_read_views, Some(0));
    assert_eq!(counters.pinned_read_bytes, Some(0));
    assert!(counters.retained_projected_bytes.unwrap() > 0);
    store.garbage_collect(5, 5).unwrap();
    drop(store);
    assert_eq!(cursor.page().unwrap(), retained);
    drop(cursor);
    let reopened = RrflowKvStore::open(&root.path().join("kv")).unwrap();
    let read = reopened.begin_transaction().unwrap();
    assert_eq!(read.get(b"a00").unwrap(), Some(b"updated".to_vec()));
    assert_eq!(read.get(b"a01").unwrap(), None);
}

#[test]
fn inspection_admission_and_drop_release_native_owners() {
    let root = tempfile::tempdir().unwrap();
    let store = RrflowKvStore::open_with_options(
        &root.path().join("kv"),
        DatabaseOptions {
            active_read_views: 1,
            ..Default::default()
        },
    )
    .unwrap();
    let mut seed = store.begin_transaction().unwrap();
    for index in 0..16 {
        seed.put(format!("a{index:02}").into_bytes(), vec![7; 128])
            .unwrap();
    }
    seed.commit(Durability::Authoritative).unwrap();
    let cursor = store.inspect_storage(1, 4096).unwrap();
    assert!(store.inspect_storage(1, 4096).is_err());
    assert_eq!(
        store.physical_store_evidence().unwrap().active_read_views,
        Some(1)
    );
    drop(cursor);
    let counters = store.physical_store_evidence().unwrap();
    assert_eq!(counters.active_read_views, Some(0));
    assert_eq!(counters.retained_projected_bytes, Some(0));
    assert!(store.inspect_storage(0, 4096).is_err());
    assert!(store.inspect_storage(1, 0).is_err());
    drop(store.inspect_storage(1, 4096).unwrap());
    assert_eq!(
        store.physical_store_evidence().unwrap().active_read_views,
        Some(0)
    );
}
