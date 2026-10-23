use super::*;
use crate::{DatabaseOptions, RrflowKvStore, RrflowMxStore};

fn seed(store: &dyn StorageEngine) -> Vec<ControlJournalEntry> {
    (1..=3)
        .map(|at| {
            store
                .control()
                .commit(&ControlTransition {
                    key: format!("server/state/test/{at}"),
                    expected: None,
                    replacement: Some(vec![at as u8]),
                    at,
                    actor: "test".into(),
                    action: "test".into(),
                    request_id: format!("request-{at}"),
                    operation_id: format!("operation-{at}"),
                })
                .unwrap()
        })
        .collect()
}

fn damage(store: &dyn StorageEngine, sequence: u64, replacement: Option<Vec<u8>>) {
    let mut transaction = store.begin_transaction().unwrap();
    let key = keyspaces::control_journal_key(sequence);
    match replacement {
        Some(value) => transaction.put(key, value).unwrap(),
        None => transaction.delete(key).unwrap(),
    }
    transaction.commit(Durability::Authoritative).unwrap();
}

#[test]
fn requested_journal_cut_rejects_missing_or_corrupt_entries_and_anchors() {
    for native in [false, true] {
        for sequence in 1..=3 {
            for corrupt in [false, true] {
                let root = tempfile::tempdir().unwrap();
                let store: Box<dyn StorageEngine> = if native {
                    Box::new(RrflowKvStore::open(root.path()).unwrap())
                } else {
                    Box::new(RrflowMxStore::new())
                };
                let entries = seed(&*store);
                let replacement = corrupt.then(|| {
                    let mut entry = entries[sequence as usize - 1].clone();
                    entry.action = "tampered".into();
                    serde_json::to_vec(&entry).unwrap()
                });
                damage(&*store, sequence, replacement);
                assert!(store.control().journal_since(sequence - 1, 1).is_err());
                assert!(store.control().journal_since(sequence, 1).is_err());
                // A requested earlier page need not validate future entries.
                if sequence > 1 {
                    assert_eq!(store.control().journal_since(0, 1).unwrap(), entries[..1]);
                }
            }
        }
    }
}

#[test]
fn journal_pages_do_not_read_later_system_families() {
    let root = tempfile::tempdir().unwrap();
    let store = RrflowKvStore::open_with_options(
        root.path(),
        DatabaseOptions {
            projected_output_bytes: 8192,
            ..Default::default()
        },
    )
    .unwrap();
    let entries = seed(&store);
    let mut transaction = store.begin_transaction().unwrap();
    for ordinal in 0..256 {
        let scope = ScopeId::new(format!("scope-{ordinal}")).unwrap();
        transaction
            .put(
                keyspaces::catalog_revision_key(&scope),
                1u64.to_be_bytes().to_vec(),
            )
            .unwrap();
    }
    transaction.commit(Durability::Authoritative).unwrap();
    assert_eq!(store.control().journal_since(0, 3).unwrap(), entries);
    assert_eq!(
        store.control().journal_since(2, usize::MAX).unwrap(),
        entries[2..]
    );

    // Correctly hashed data stored under another journal identity is corrupt.
    damage(&store, 2, Some(serde_json::to_vec(&entries[0]).unwrap()));
    assert!(store.control().journal_since(1, 1).is_err());
}
