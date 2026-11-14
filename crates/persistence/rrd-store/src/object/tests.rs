use super::*;
use tempfile::tempdir;

#[test]
fn publication_keeps_a_reader_held_winner() {
    let directory = tempdir().unwrap();
    let store = LocalObjectStore::open(directory.path()).unwrap();
    let bytes = b"late immutable-object publisher";
    let winner = store.put(bytes).unwrap();
    let target = store.root.join(&winner.receipt.key);
    let stage = store.root.join("staging/late-publisher");
    let mut writer = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&stage)
        .unwrap();
    writer.write_all(bytes).unwrap();
    writer.sync_all().unwrap();
    drop(writer);
    // Model a publisher that observed absence before the winner appeared.
    // A reader may retain the winning file while this late caller publishes.
    #[cfg(windows)]
    let mut reader = {
        use std::os::windows::fs::OpenOptionsExt;
        std::fs::OpenOptions::new()
            .read(true)
            .share_mode(3) // read/write sharing; replacement is not authorized
            .open(target.diagnostic_path())
            .unwrap()
    };
    #[cfg(not(windows))]
    let mut reader = target.open_read().unwrap();
    #[cfg(unix)]
    let original_identity = {
        use std::os::unix::fs::MetadataExt;
        let metadata = reader.metadata().unwrap();
        (metadata.dev(), metadata.ino())
    };

    publish_object_path(&stage, &target, &winner.sha256)
        .expect("reuse the verified publication winner");
    assert!(!stage.try_exists().unwrap());
    assert_eq!(store.verify(&winner.sha256).unwrap(), winner);
    let mut retained = Vec::new();
    reader.read_to_end(&mut retained).unwrap();
    assert_eq!(retained, bytes);
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let metadata = target.metadata().unwrap();
        assert_eq!((metadata.dev(), metadata.ino()), original_identity);
    }
}

#[test]
fn publication_rejects_a_corrupt_collision_and_retains_the_stage() {
    let directory = tempdir().unwrap();
    let store = LocalObjectStore::open(directory.path()).unwrap();
    let bytes = b"verified stage must survive a corrupt collision";
    let winner = store.put(bytes).unwrap();
    let target = store.root.join(&winner.receipt.key);
    fs::write(target.diagnostic_path(), b"corrupt competing winner").unwrap();
    let stage = store.root.join("staging/collision-stage");
    let mut writer = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&stage)
        .unwrap();
    writer.write_all(bytes).unwrap();
    writer.sync_all().unwrap();
    drop(writer);

    assert!(matches!(
        publish_object_path(&stage, &target, &winner.sha256),
        Err(Error::ObjectCorrupt { .. })
    ));
    assert_eq!(fs::read(stage.diagnostic_path()).unwrap(), bytes);
    assert_eq!(
        fs::read(target.diagnostic_path()).unwrap(),
        b"corrupt competing winner"
    );
}

#[test]
fn publication_preserves_a_missing_source_error() {
    let directory = tempdir().unwrap();
    let store = LocalObjectStore::open(directory.path()).unwrap();
    let source = store.root.join("staging/missing");
    let target = store.root.join("staging/unpublished");
    assert!(publish_object_path(&source, &target, &digest::sha256_hex(b"missing")).is_err());
    assert!(!source.try_exists().unwrap());
    assert!(!target.try_exists().unwrap());
}

#[test]
fn concurrent_object_publishers_return_one_verified_address() {
    let directory = tempdir().unwrap();
    let store = LocalObjectStore::open(directory.path()).unwrap();
    let barrier = std::sync::Barrier::new(4);
    let bytes = b"concurrent immutable content";
    let results = std::thread::scope(|scope| {
        let workers: Vec<_> = (0..4)
            .map(|_| {
                scope.spawn(|| {
                    store.put_with_hook(bytes, |step| {
                        if step == ObjectStep::AfterStageSync {
                            barrier.wait();
                        }
                        Ok(())
                    })
                })
            })
            .collect();
        workers
            .into_iter()
            .map(|worker| worker.join().unwrap().unwrap())
            .collect::<Vec<_>>()
    });
    assert!(results.iter().all(|result| result == &results[0]));
    assert_eq!(store.verify(&results[0].sha256).unwrap(), results[0]);
    let inventory = store.inventory(&BTreeSet::new()).unwrap();
    assert_eq!(inventory.entries.len(), 1);
    assert!(inventory.staging_files.is_empty());
    assert!(inventory.quarantined_files.is_empty());
}

#[test]
fn memory_objects_keep_the_same_content_address_and_receipt_boundary() {
    let store = MemoryObjectStore::new();
    let verified = store.put(b"memory canonical bytes").unwrap();
    assert_eq!(verified.receipt.backend, "memory");
    let reference = ObjectReference::for_bytes(
        "memory-object",
        None,
        "application/octet-stream",
        b"memory canonical bytes",
        verified.receipt.clone(),
    )
    .unwrap();
    assert_eq!(store.get(&reference).unwrap(), b"memory canonical bytes");
    assert_eq!(store.verify(&verified.sha256).unwrap(), verified);

    let removed = store
        .reclaim_orphans(&BTreeSet::from([reference.sha256.clone()]))
        .unwrap();
    assert_eq!(removed, vec![reference.sha256.clone()]);
    assert!(matches!(
        store.verify(&reference.sha256),
        Err(Error::ObjectMissing(_))
    ));
}

#[test]
fn local_put_is_content_addressed_idempotent_and_verified() {
    let directory = tempdir().unwrap();
    let store = LocalObjectStore::open(directory.path()).unwrap();
    let first = store.put(b"canonical bytes").unwrap();
    let second = store.put(b"canonical bytes").unwrap();
    assert_eq!(first, second);
    assert_eq!(store.verify(&first.sha256).unwrap(), first);
    assert!(
        store.inventory(&BTreeSet::new()).unwrap().entries[0].state == ObjectInventoryState::Orphan
    );
}

#[test]
fn local_file_put_streams_to_the_same_content_address() {
    let directory = tempdir().unwrap();
    let source = directory.path().join("source.bin");
    let bytes = vec![0x5a; 256 * 1024 + 17];
    fs::write(&source, &bytes).unwrap();
    let store = LocalObjectStore::open(directory.path().join("objects")).unwrap();
    let streamed = store.put_file(&source).unwrap();
    let in_memory = store.put(&bytes).unwrap();
    assert_eq!(streamed, in_memory);

    let reference = ObjectReference::for_verified(
        "streamed",
        None,
        "application/octet-stream",
        streamed.sha256.clone(),
        streamed.length,
        streamed.receipt.clone(),
    )
    .unwrap();
    let mut verified = store.verified_file(&reference).unwrap();
    let mut actual = Vec::new();
    verified.read_to_end(&mut actual).unwrap();
    assert_eq!(actual, bytes);
}

#[test]
fn verified_handle_keeps_authenticated_bytes_after_leaf_replacement() {
    let directory = tempdir().unwrap();
    let store = LocalObjectStore::open(directory.path()).unwrap();
    let object = store.put(b"authenticated original").unwrap();
    let reference = ObjectReference::for_verified(
        "retained",
        None,
        "application/octet-stream",
        object.sha256,
        object.length,
        object.receipt,
    )
    .unwrap();
    let mut file = store.verified_file(&reference).unwrap();
    let path = store.root().join(&reference.receipt.key);
    fs::rename(&path, path.with_extension("retained")).unwrap();
    fs::write(&path, b"substituted contents").unwrap();
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).unwrap();
    assert_eq!(bytes, b"authenticated original");
    assert!(store.verified_file(&reference).is_err());
}

#[test]
fn verified_stream_is_bounded_content_addressed_and_fail_closed() {
    let directory = tempdir().unwrap();
    let store = LocalObjectStore::open(directory.path()).unwrap();
    let bytes = vec![0x3c; 192 * 1024 + 9];
    let sha256 = digest::sha256_hex(&bytes);
    let mut reader = std::io::Cursor::new(bytes.clone());
    let streamed = store
        .put_verified_stream(&sha256, bytes.len() as u64, &mut reader)
        .unwrap();
    assert_eq!(streamed, store.put(&bytes).unwrap());

    let wrong = "11".repeat(32);
    let mut substituted = std::io::Cursor::new(bytes.clone());
    assert!(matches!(
        store.put_verified_stream(&wrong, bytes.len() as u64, &mut substituted),
        Err(Error::ObjectCorrupt { .. })
    ));
    assert!(matches!(store.verify(&wrong), Err(Error::ObjectMissing(_))));

    let mut truncated = std::io::Cursor::new(&bytes[..bytes.len() - 1]);
    assert!(matches!(
        store.put_verified_stream(&sha256, bytes.len() as u64, &mut truncated),
        Err(Error::ObjectLengthMismatch { .. })
    ));
    assert!(store
        .inventory(&BTreeSet::new())
        .unwrap()
        .staging_files
        .is_empty());
}

#[test]
fn failed_after_sync_remains_staged_and_never_visible() {
    let directory = tempdir().unwrap();
    let store = LocalObjectStore::open(directory.path()).unwrap();
    let result = store.put_with_hook(b"not published", |step| {
        if step == ObjectStep::AfterStageSync {
            Err(Error::FaultInjected("after_stage_sync"))
        } else {
            Ok(())
        }
    });
    assert!(matches!(result, Err(Error::FaultInjected(_))));
    let inventory = store.inventory(&BTreeSet::new()).unwrap();
    assert_eq!(inventory.entries.len(), 0);
    assert_eq!(inventory.staging_files.len(), 1);
}

#[test]
fn corrupt_object_is_reported_and_quarantined() {
    let directory = tempdir().unwrap();
    let store = LocalObjectStore::open(directory.path()).unwrap();
    let object = store.put(b"valid").unwrap();
    fs::write(store.root().join(&object.receipt.key), b"corrupt").unwrap();
    assert!(matches!(
        store.verify(&object.sha256),
        Err(Error::ObjectCorrupt { .. })
    ));
    let target = store.quarantine(&object.sha256).unwrap();
    assert!(target.exists());
    assert!(matches!(
        store.verify(&object.sha256),
        Err(Error::ObjectMissing(_))
    ));
}

#[test]
fn every_publication_boundary_has_explicit_recovery_state() {
    for point in [
        ObjectStep::BeforeStageWrite,
        ObjectStep::AfterStageSync,
        ObjectStep::AfterPublish,
        ObjectStep::BeforeVerify,
        ObjectStep::AfterVerify,
    ] {
        let directory = tempdir().unwrap();
        let store = LocalObjectStore::open(directory.path()).unwrap();
        let result = store.put_with_hook(b"boundary", |step| {
            if step == point {
                Err(Error::FaultInjected("object_boundary"))
            } else {
                Ok(())
            }
        });
        assert!(matches!(result, Err(Error::FaultInjected(_))));
        let inventory = store.inventory(&BTreeSet::new()).unwrap();
        match point {
            ObjectStep::BeforeStageWrite => {
                assert!(inventory.entries.is_empty());
                assert!(inventory.staging_files.is_empty());
            }
            ObjectStep::AfterStageSync => {
                assert!(inventory.entries.is_empty());
                assert_eq!(inventory.staging_files.len(), 1);
            }
            ObjectStep::AfterPublish | ObjectStep::BeforeVerify | ObjectStep::AfterVerify => {
                assert_eq!(inventory.entries.len(), 1);
                assert_eq!(inventory.entries[0].state, ObjectInventoryState::Orphan);
            }
        }
    }
}

#[test]
fn explicit_orphan_reclamation_removes_only_proven_candidates() {
    let directory = tempdir().unwrap();
    let store = LocalObjectStore::open(directory.path()).unwrap();
    let retained = store.put(b"retained").unwrap();
    let orphan = store.put(b"orphan").unwrap();
    let removed = store
        .reclaim_orphans(&BTreeSet::from([orphan.sha256.clone()]))
        .unwrap();
    assert_eq!(removed, vec![orphan.sha256]);
    assert!(store.verify(&retained.sha256).is_ok());
}
