use rrd_lsm::{Database, Durability, Error, Mutation, ReadOnlyDatabase, WriteBatch};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn put(key: &str, value: &str) -> Mutation {
    Mutation::Put {
        key: key.as_bytes().to_vec(),
        value: value.as_bytes().to_vec(),
    }
}

fn inventory(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn visit(root: &Path, current: &Path, output: &mut BTreeMap<PathBuf, Vec<u8>>) {
        let mut entries = std::fs::read_dir(current)
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        entries.sort_by_key(std::fs::DirEntry::file_name);
        for entry in entries {
            let path = entry.path();
            if path.is_dir() {
                visit(root, &path, output);
            } else {
                // Windows mandatory locking rejects even a read at EOF on
                // the empty manifest lease. Its zero length proves the empty
                // payload without trying to read through the writer's lock.
                let bytes = if entry.metadata().unwrap().len() == 0 {
                    Vec::new()
                } else {
                    std::fs::read(&path).unwrap()
                };
                output.insert(path.strip_prefix(root).unwrap().to_owned(), bytes);
            }
        }
    }

    let mut output = BTreeMap::new();
    if root.exists() {
        visit(root, root, &mut output);
    }
    output
}

#[test]
fn absent_inspection_and_existing_open_create_no_files() {
    let parent = tempfile::tempdir().unwrap();
    let root = parent.path().join("missing");
    assert!(ReadOnlyDatabase::open(&root).is_err());
    assert!(Database::open(&root).is_err());
    assert!(!root.exists());
}

#[test]
fn inspector_excludes_a_live_writer_and_never_changes_bytes() {
    let parent = tempfile::tempdir().unwrap();
    let root = parent.path().join("native");
    let database = Database::create(&root).unwrap();
    let before = inventory(&root);
    assert!(matches!(
        ReadOnlyDatabase::open(&root),
        Err(Error::DatabaseWriterLock { .. })
    ));
    assert_eq!(inventory(&root), before);
    drop(database);
}

#[test]
fn inspector_authenticates_manifest_lineage_segments_and_wal_reads() {
    let parent = tempfile::tempdir().unwrap();
    let root = parent.path().join("native");
    let mut database =
        Database::create_with_application_format(&root, rrd_lsm::DatabaseOptions::default(), 7)
            .unwrap();
    // Retain the ancestry this case authenticates; ordinary write maintenance
    // is allowed to reclaim an unpinned parent.
    let ancestor = database.checkpoint("inspection-lineage", 1).unwrap();
    database
        .write_owned(
            WriteBatch::new(vec![put("alpha", "one"), put("beta", "two")]).unwrap(),
            Durability::Authoritative,
        )
        .unwrap();
    database.flush_memtable(10).unwrap().unwrap();
    database
        .write_owned(
            WriteBatch::new(vec![put("alpha", "three"), put("gamma", "four")]).unwrap(),
            Durability::Authoritative,
        )
        .unwrap();
    let expected_sequence = database.snapshot().sequence;
    drop(database);

    let before = inventory(&root);
    let inspected = ReadOnlyDatabase::open(&root).unwrap();
    assert_eq!(inspected.inspection().application_format, Some(7));
    assert_eq!(inspected.inspection().visible_sequence, expected_sequence);
    assert_eq!(inspected.inspection().manifest_lineage.len(), 2);
    assert!(inspected
        .inspection()
        .manifest_lineage
        .contains(&ancestor.manifest));
    assert!(!inspected.inspection().files.is_empty());
    assert_eq!(
        inspected.get(b"alpha").unwrap().as_deref(),
        Some(b"three".as_slice())
    );
    assert_eq!(
        inspected.scan(b"a", Some(b"z")).unwrap(),
        vec![
            (b"alpha".to_vec(), b"three".to_vec()),
            (b"beta".to_vec(), b"two".to_vec()),
            (b"gamma".to_vec(), b"four".to_vec()),
        ]
    );
    drop(inspected);
    assert_eq!(inventory(&root), before);

    let reopened = Database::open(&root).unwrap();
    assert_eq!(
        reopened
            .get(b"alpha", reopened.snapshot())
            .unwrap()
            .as_deref(),
        Some(b"three".as_slice())
    );
}

#[test]
fn torn_wal_fails_without_repairing_the_tail() {
    use std::io::Write as _;

    let parent = tempfile::tempdir().unwrap();
    let root = parent.path().join("native");
    let mut database = Database::create(&root).unwrap();
    database
        .write_owned(
            WriteBatch::new(vec![put("alpha", "one")]).unwrap(),
            Durability::Authoritative,
        )
        .unwrap();
    let wal = root.join("wal").join(format!(
        "{:020}.wal",
        database.manifest().wal_start_sequence
    ));
    drop(database);
    std::fs::OpenOptions::new()
        .append(true)
        .open(&wal)
        .unwrap()
        .write_all(&[0xff])
        .unwrap();

    let before = inventory(&root);
    assert!(matches!(
        ReadOnlyDatabase::open(&root),
        Err(Error::TornTail { .. })
    ));
    assert_eq!(inventory(&root), before);
}

#[cfg(unix)]
#[test]
fn create_open_and_inspection_reject_symbolic_database_paths() {
    use std::os::unix::fs::symlink;

    let parent = tempfile::tempdir().unwrap();
    let actual = parent.path().join("actual");
    std::fs::create_dir(&actual).unwrap();
    let linked_root = parent.path().join("linked-root");
    symlink(&actual, &linked_root).unwrap();
    assert!(Database::create(&linked_root).is_err());

    let root = parent.path().join("database");
    drop(Database::create(&root).unwrap());
    let wal = root.join("wal");
    let redirected = root.join("redirected-wal");
    std::fs::rename(&wal, &redirected).unwrap();
    symlink(&redirected, &wal).unwrap();
    assert!(Database::open(&root).is_err());
    assert!(ReadOnlyDatabase::open(&root).is_err());

    let pointer_root = parent.path().join("pointer-database");
    drop(Database::create(&pointer_root).unwrap());
    let current = pointer_root.join("CURRENT");
    let redirected_current = pointer_root.join("REDIRECTED-CURRENT");
    std::fs::rename(&current, &redirected_current).unwrap();
    symlink(&redirected_current, &current).unwrap();
    assert!(Database::open(&pointer_root).is_err());
    assert!(ReadOnlyDatabase::open(&pointer_root).is_err());
}

#[test]
fn inspection_accepts_a_valid_garbage_collected_parent_boundary() {
    let parent = tempfile::tempdir().unwrap();
    let root = parent.path().join("database");
    let mut database = Database::create(&root).unwrap();
    database
        .write_owned(
            WriteBatch::new(vec![put("alpha", "one")]).unwrap(),
            Durability::Authoritative,
        )
        .unwrap();
    database.flush_memtable(10).unwrap().unwrap();
    let current_generation = database.manifest().generation;
    assert!(current_generation > 1);
    let collection = database.garbage_collect().unwrap();
    assert!(!collection.removed_manifests.is_empty());
    drop(database);

    let inspected = ReadOnlyDatabase::open(&root).unwrap();
    assert_eq!(
        inspected.inspection().current_generation,
        current_generation
    );
    assert_eq!(inspected.inspection().manifest_lineage.len(), 1);
    assert_eq!(
        inspected.get(b"alpha").unwrap().as_deref(),
        Some(b"one".as_slice())
    );
}
