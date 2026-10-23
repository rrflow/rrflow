use rrd_store::{ControlTransition, Error, RrflowKvStore, RrflowMxStore, StorageEngine};
use std::sync::{Arc, Barrier};

fn transition(expected: Option<&[u8]>, replacement: Option<&[u8]>, at: u64) -> ControlTransition {
    keyed_transition("server/state/session/session-1", expected, replacement, at)
}

fn keyed_transition(
    key: &str,
    expected: Option<&[u8]>,
    replacement: Option<&[u8]>,
    at: u64,
) -> ControlTransition {
    ControlTransition {
        key: key.into(),
        expected: expected.map(<[u8]>::to_vec),
        replacement: replacement.map(<[u8]>::to_vec),
        at,
        actor: "rrd-server".into(),
        action: "session.transition".into(),
        request_id: format!("request-{at}"),
        operation_id: format!("operation-{at}"),
    }
}

fn assert_atomic_batch(engine: &dyn StorageEngine) {
    let entries = engine
        .control()
        .commit_batch(&[
            keyed_transition("server/state/audit/record-1", None, Some(b"record"), 20),
            keyed_transition("server/state/audit/head", None, Some(b"head"), 20),
        ])
        .unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].sequence + 1, entries[1].sequence);
    assert_eq!(
        entries[1].previous_digest.as_deref(),
        Some(entries[0].digest.as_str())
    );
    assert_eq!(
        engine.control().get("server/state/audit/record-1").unwrap(),
        Some(b"record".to_vec())
    );

    let before = engine.control().sequence().unwrap();
    let denied = engine.control().commit_batch(&[
        keyed_transition("server/state/audit/record-2", None, Some(b"record-2"), 21),
        keyed_transition("server/state/audit/head", None, Some(b"wrong-head"), 21),
    ]);
    assert!(matches!(denied, Err(Error::ControlConflict(_))));
    assert_eq!(engine.control().sequence().unwrap(), before);
    assert_eq!(
        engine.control().get("server/state/audit/record-2").unwrap(),
        None
    );
    assert_eq!(
        engine.control().get("server/state/audit/head").unwrap(),
        Some(b"head".to_vec())
    );
}

fn assert_journal(engine: &dyn StorageEngine) {
    assert_eq!(engine.control().sequence().unwrap(), 0);
    let created = engine
        .control()
        .commit(&transition(None, Some(b"open"), 10))
        .unwrap();
    assert_eq!(created.sequence, 1);
    assert!(created.verify());
    assert_eq!(
        engine.control().get(&created.key).unwrap(),
        Some(b"open".to_vec())
    );

    assert!(matches!(
        engine
            .control()
            .commit(&transition(None, Some(b"collision"), 11)),
        Err(Error::ControlConflict(_))
    ));
    let renewed = engine
        .control()
        .commit(&transition(Some(b"open"), Some(b"renewed"), 12))
        .unwrap();
    assert_eq!(renewed.sequence, 2);
    assert_eq!(
        renewed.previous_digest.as_deref(),
        Some(created.digest.as_str())
    );
    let deleted = engine
        .control()
        .commit(&transition(Some(b"renewed"), None, 13))
        .unwrap();
    assert_eq!(deleted.sequence, 3);
    assert_eq!(engine.control().get(&deleted.key).unwrap(), None);
    let journal = engine.control().journal_since(0, 10).unwrap();
    assert_eq!(journal, vec![created, renewed, deleted]);
    assert!(journal.iter().all(|entry| entry.verify()));
    assert_eq!(engine.control().sequence().unwrap(), 3);
}

#[test]
fn every_engine_materializes_and_journals_the_same_cas_transitions() {
    let memory = RrflowMxStore::new();
    assert_journal(&memory);
    assert_atomic_batch(&memory);
    let rrflow_kv = tempfile::tempdir().unwrap();
    let rrflow_kv = RrflowKvStore::open(rrflow_kv.path()).unwrap();
    assert_journal(&rrflow_kv);
    assert_atomic_batch(&rrflow_kv);
}

#[test]
fn materialized_state_and_hash_chain_survive_restart() {
    let root = tempfile::tempdir().unwrap();
    let engine = RrflowKvStore::open(root.path()).unwrap();
    engine
        .control()
        .commit(&transition(None, Some(b"open"), 10))
        .unwrap();
    drop(engine);
    let reopened = RrflowKvStore::open(root.path()).unwrap();
    assert_eq!(
        reopened
            .control()
            .get("server/state/session/session-1")
            .unwrap(),
        Some(b"open".to_vec())
    );
    assert_eq!(reopened.control().journal_since(0, 10).unwrap().len(), 1);
    assert_eq!(reopened.control().sequence().unwrap(), 1);
}

fn assert_concurrent_disjoint_progress(engine: Arc<dyn StorageEngine>) {
    const WRITERS: usize = 8;
    let barrier = Arc::new(Barrier::new(WRITERS));
    let handles = (0..WRITERS)
        .map(|writer| {
            let engine = Arc::clone(&engine);
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                barrier.wait();
                engine.control().commit(&keyed_transition(
                    &format!("server/state/concurrent/{writer}"),
                    None,
                    Some(b"committed"),
                    100 + writer as u64,
                ))
            })
        })
        .collect::<Vec<_>>();
    for handle in handles {
        handle.join().unwrap().unwrap();
    }
    let journal = engine.control().journal_since(0, WRITERS).unwrap();
    assert_eq!(journal.len(), WRITERS);
    assert_eq!(engine.control().sequence().unwrap(), WRITERS as u64);
    assert!(journal.iter().all(|entry| entry.verify()));
}

#[test]
fn concurrent_disjoint_control_writes_serialize_through_the_shared_journal() {
    assert_concurrent_disjoint_progress(Arc::new(RrflowMxStore::new()));
    let root = tempfile::tempdir().unwrap();
    assert_concurrent_disjoint_progress(Arc::new(RrflowKvStore::open(root.path()).unwrap()));
}

#[test]
fn small_journal_pages_remain_bounded_after_growth_and_reopen() {
    let root = tempfile::tempdir().unwrap();
    let options = rrd_store::DatabaseOptions {
        projected_output_bytes: 8 * 1024,
        ..Default::default()
    };
    let engine = RrflowKvStore::open_with_options(root.path(), options).unwrap();
    let expected = (1..=96)
        .map(|sequence| {
            engine
                .control()
                .commit(&keyed_transition(
                    &format!("server/state/page/{sequence}"),
                    None,
                    Some(b"retained"),
                    sequence,
                ))
                .unwrap()
        })
        .collect::<Vec<_>>();
    for engine in [Some(engine), None] {
        let engine = engine.unwrap_or_else(|| {
            RrflowKvStore::open_existing_with_options(root.path(), options).unwrap()
        });
        let mut after = 0;
        let mut actual = Vec::new();
        loop {
            let page = engine.control().journal_since(after, 3).unwrap();
            if page.is_empty() {
                break;
            }
            assert!(page.len() <= 3);
            after = page.last().unwrap().sequence;
            actual.extend(page);
        }
        assert_eq!(actual, expected);
        assert!(engine
            .control()
            .journal_since(u64::MAX, 3)
            .unwrap()
            .is_empty());
        assert!(engine.control().journal_since(0, 0).is_err());
        // Pagination must not disable the existing byte admission ceiling.
        let denied = engine.control().journal_since(0, 96).unwrap_err();
        assert!(
            matches!(denied, Error::ProjectedReadLimit {
            resource: rrd_store::ProjectedReadResource::OutputBufferBytes,
            limit: 8192,
            observed,
        } if observed > 8192),
            "{denied}"
        );
        let resources = engine.physical_store_evidence().unwrap();
        assert_eq!(resources.native_transactions, Some(0));
    }
}
