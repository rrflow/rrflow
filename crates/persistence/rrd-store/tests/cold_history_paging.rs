use rrd_core::{
    RuntimeCommit, RuntimeLogicalModel, RuntimeMutation, RuntimeRecord, RuntimeRef,
    RuntimeSchemaRegistry, RuntimeTableSchema, RuntimeType, RuntimeValue, ScopeId,
};
use rrd_store::{Error, RrflowKvStore, StorageEngine};
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

fn schema() -> RuntimeMutation {
    let mut registry = RuntimeSchemaRegistry::empty(1, "history paging fixture");
    registry.tables.insert(
        RuntimeType::new("note").unwrap(),
        RuntimeTableSchema::schemaless(RuntimeLogicalModel::Relational),
    );
    RuntimeMutation::Schema { registry }
}

fn record(id: &str, body: &str) -> RuntimeMutation {
    RuntimeMutation::Record {
        record: RuntimeRecord {
            reference: RuntimeRef::new("note", id).unwrap(),
            valid_from: 1,
            valid_to: None,
            properties: BTreeMap::from([("body".into(), RuntimeValue::String(body.into()))]),
        },
    }
}

#[test]
fn bounded_history_pages_advance_across_filtered_scopes_without_splitting_entries() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("engram");
    let store = RrflowKvStore::open(&path).unwrap();
    let first = ScopeId::new("instance:first").unwrap();
    let other = ScopeId::new("instance:other").unwrap();
    let commit = |scope: ScopeId, expected_cursor, mutations| {
        store
            .runtime()
            .commit(&RuntimeCommit {
                scope,
                expected_cursor,
                at: 10 + expected_cursor,
                actor: "page-test".into(),
                mutations,
            })
            .unwrap()
    };
    commit(first.clone(), 0, vec![schema(), record("one", "first")]);
    commit(other, 2, vec![schema(), record("private", "other scope")]);
    commit(first.clone(), 4, vec![record("one", "updated")]);
    let stamp = store.runtime().read_stamp(&first).unwrap();
    let all = store.runtime().changes_since(0, 16, None).unwrap().changes;
    assert_eq!(all.len(), 5);
    let sizes = all
        .iter()
        .map(|change| serde_json::to_vec(change).unwrap().len() as u64)
        .collect::<Vec<_>>();
    let first_budget = sizes[0] + sizes[1] + sizes[2];
    let page = store
        .runtime()
        .read_changes_bounded_bytes(
            &stamp,
            0,
            16,
            first_budget,
            Instant::now() + Duration::from_secs(10),
        )
        .unwrap();
    assert_eq!(page.through_cursor, 3);
    assert_eq!(
        page.changes
            .iter()
            .map(|change| change.cursor)
            .collect::<Vec<_>>(),
        vec![1, 2]
    );
    let second_budget = sizes[2] + sizes[3] + sizes[4];
    let second = store
        .runtime()
        .read_changes_bounded_bytes(
            &stamp,
            3,
            16,
            second_budget,
            Instant::now() + Duration::from_secs(10),
        )
        .unwrap();
    assert_eq!(second.through_cursor, 5);
    assert_eq!(
        second
            .changes
            .iter()
            .map(|change| change.cursor)
            .collect::<Vec<_>>(),
        vec![5]
    );
    assert!(matches!(
        store.runtime().read_changes_bounded_bytes(
            &stamp,
            0,
            16,
            sizes[0] - 1,
            Instant::now() + Duration::from_secs(10)
        ),
        Err(Error::ColdHistorySourceLimit { .. })
    ));
    // The predecessor is decoded and dropped before the next entry is read.
    // Both fit individually, but their sum is deliberately larger than the
    // page budget; the second scope must still be traversable.
    let single_entry_budget = sizes[2].max(sizes[3]);
    assert!(sizes[2] + sizes[3] > single_entry_budget);
    let single = store
        .runtime()
        .read_changes_bounded_bytes(
            &stamp,
            3,
            1,
            single_entry_budget,
            Instant::now() + Duration::from_secs(10),
        )
        .unwrap();
    assert_eq!(single.through_cursor, 4);
    assert!(single.changes.is_empty());
    assert!(matches!(
        store
            .runtime()
            .read_changes_bounded_bytes(&stamp, 0, 16, first_budget, Instant::now()),
        Err(Error::ColdHistorySourceDeadline)
    ));
    drop(store);
    let store = RrflowKvStore::open(&path).unwrap();
    assert_eq!(
        store
            .runtime()
            .read_changes_bounded_bytes(
                &stamp,
                3,
                16,
                second_budget,
                Instant::now() + Duration::from_secs(10)
            )
            .unwrap()
            .changes,
        second.changes
    );
}
