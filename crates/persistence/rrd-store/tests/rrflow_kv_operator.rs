use rrd_core::{Claim, ClaimReader, Predicate, Producer, Reader, Subject};
use rrd_lsm::{
    DatabaseOptions, MaintenancePolicy, DEFAULT_MEMTABLE_MAX_VERSIONS,
    DEFAULT_WAL_PAYLOAD_MAX_BYTES,
};
use rrd_store::{InvocationInput, Outcome, RrflowKvStore, StorageEngine, Trigger};

fn claim(subject: &str, object: &str) -> Claim {
    Claim::new(
        Subject::new(subject).unwrap(),
        Predicate::new("status").unwrap(),
        object,
        100,
        100,
        Producer {
            actor: "test".into(),
            on_behalf_of: None,
            session: None,
        },
    )
}

fn invocation<'a>(arguments: &'a [String]) -> InvocationInput<'a> {
    InvocationInput {
        at: 6_000,
        trigger: Trigger::Event,
        command: "context-assemble",
        arguments,
        outcome: Outcome::Ok,
        duration_ms: 7,
        detail: Some("two claims".into()),
    }
}

#[test]
fn rrflow_kv_operator_evidence_survives_reopen() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("rrflow-kv");
    let store = RrflowKvStore::open(&path).unwrap();
    let claims = [claim("wp3", "active"), claim("wp4", "planned")];
    store.claims().append_batch(&claims).unwrap();
    let physical = store.physical_store_evidence().unwrap();
    assert_eq!(
        physical.wal_payload_max_bytes,
        Some(DEFAULT_WAL_PAYLOAD_MAX_BYTES as u64)
    );
    assert_eq!(
        physical.memtable_max_versions,
        Some(DEFAULT_MEMTABLE_MAX_VERSIONS as u64)
    );
    assert_eq!(physical.automatic_flushes, Some(0));
    assert_eq!(physical.maintenance_write_stalls, Some(0));
    assert_eq!(physical.automatic_compactions, Some(0));
    assert_eq!(physical.failed_compactions, Some(0));
    assert_eq!(physical.compaction_input_bytes, Some(0));
    assert_eq!(physical.compaction_output_bytes, Some(0));
    assert_eq!(physical.peak_compaction_buffer_bytes, Some(0));
    assert_eq!(physical.l0_segment_count, Some(0));
    assert_eq!(physical.compaction_debt_segments, Some(0));
    assert_eq!(
        physical.l0_compaction_trigger,
        Some(DatabaseOptions::default().compaction.l0_compaction_trigger as u64)
    );
    assert_eq!(physical.filter_checks, Some(0));
    assert_eq!(physical.filter_negatives, Some(0));

    let reader = Reader::new("agent:clyffy").unwrap();
    let subject = Subject::new("wp3").unwrap();
    let predicate = Predicate::new("status").unwrap();
    store
        .claims()
        .observe(&reader, &subject, &predicate, 5_000)
        .unwrap();
    let removal_report = store.claims().removal_report(1_000, 9_000).unwrap();
    assert_eq!(store.claims().access_count().unwrap(), 1);

    let arguments = vec!["subject=wp3".into(), "subject=wp4".into()];
    store.invocations().record(invocation(&arguments)).unwrap();
    assert_eq!(store.invocations().count().unwrap(), 1);

    drop(store);
    let reopened = RrflowKvStore::open(&path).unwrap();
    assert_eq!(reopened.invocations().count().unwrap(), 1);
    assert_eq!(reopened.claims().access_count().unwrap(), 1);
    assert_eq!(
        reopened.invocations().since(0).unwrap()[0].command,
        "context-assemble"
    );
    assert_eq!(
        reopened.claims().removal_report(1_000, 9_000).unwrap(),
        removal_report
    );
}

#[test]
fn rrflow_kv_applies_explicit_project_maintenance_bounds() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("rrflow-kv");
    let engine = RrflowKvStore::open_with_options(
        &path,
        DatabaseOptions {
            maintenance: MaintenancePolicy {
                wal_payload_max_bytes: usize::MAX,
                memtable_max_versions: 1,
            },
            ..DatabaseOptions::default()
        },
    )
    .unwrap();
    engine
        .claims()
        .append_batch(&[claim("bounded", "active")])
        .unwrap();
    engine
        .claims()
        .observe(
            &Reader::new("agent:clyffy").unwrap(),
            &Subject::new("bounded").unwrap(),
            &Predicate::new("status").unwrap(),
            5_000,
        )
        .unwrap();

    let physical = engine.physical_store_evidence().unwrap();
    assert_eq!(physical.memtable_max_versions, Some(1));
    assert_eq!(physical.automatic_flushes, Some(1));
    assert_eq!(physical.maintenance_write_stalls, Some(1));
    assert_eq!(physical.failed_maintenance_flushes, Some(0));
    assert_eq!(physical.l0_segment_count, Some(1));
    assert_eq!(physical.compaction_debt_segments, Some(0));
}

#[test]
fn native_write_path_diagnostics_are_explicit_bounded_and_semantically_inert() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("rrflow-kv");
    let store = RrflowKvStore::open(&path).unwrap();
    assert!(store
        .take_native_write_path_diagnostics()
        .unwrap()
        .is_none());

    store.enable_native_write_path_diagnostics(1).unwrap();
    let expected = claim("diagnostic", "accepted");
    let started = std::time::Instant::now();
    store
        .claims()
        .append_batch(std::slice::from_ref(&expected))
        .unwrap();
    let operation_ns = u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX);
    let diagnostics = store.take_native_write_path_diagnostics().unwrap().unwrap();

    assert_eq!(diagnostics.capacity, 1);
    assert_eq!(diagnostics.observed_samples, 1);
    assert_eq!(diagnostics.dropped_samples, 0);
    assert_eq!(diagnostics.samples.len(), 1);
    let sample = &diagnostics.samples[0];
    assert_eq!(sample.mutation_count, 3);
    assert_eq!(sample.durability, rrd_lsm::Durability::Authoritative);
    // Uncontended acquisitions can fall within one clock tick. Preserve zero;
    // the two disjoint measured waits must fit inside the observed operation.
    assert!(sample.lock_wait.total_ns() <= operation_ns);
    assert!(sample.phases.physical_total.wall_ns <= operation_ns);

    drop(store);
    let reopened = RrflowKvStore::open(&path).unwrap();
    assert_eq!(
        reopened
            .claims()
            .history(&expected.subject, &expected.predicate)
            .unwrap(),
        vec![expected]
    );
}

#[test]
fn idle_maintenance_reclaims_superseded_versions_without_losing_owned_history() {
    use rrd_store::Durability;
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("tracked-history");
    let store = RrflowKvStore::open(&path).unwrap();
    let mut tx = store.begin_transaction().unwrap();
    tx.put(b"hot".to_vec(), b"initial".to_vec()).unwrap();
    tx.put(b"deleted".to_vec(), b"initial".to_vec()).unwrap();
    tx.commit(Durability::Authoritative).unwrap();
    let mut claims = vec![claim("history", "initial")];
    store.claims().append_batch(&claims).unwrap();
    store.flush(1).unwrap();
    let held = store.begin_transaction().unwrap();
    let mut reference = store.inspect_storage(1, 4096).unwrap();
    let mut expected_pages = Vec::new();
    loop {
        let page = reference.page().unwrap();
        expected_pages.push(page.sha256);
        if !page.source_open {
            break;
        }
        reference.advance(page.ordinal).unwrap();
    }
    drop(reference);
    let mut retained = store.inspect_storage(1, 4096).unwrap();
    for ordinal in 1..=32 {
        let value = ordinal.to_string().into_bytes();
        let mut tx = store.begin_transaction().unwrap();
        tx.put(b"hot".to_vec(), value.clone()).unwrap();
        tx.put(format!("history/{ordinal}").into_bytes(), value)
            .unwrap();
        tx.delete(b"deleted".to_vec()).unwrap();
        tx.commit(Durability::Authoritative).unwrap();
        let mut next = claim("history", &ordinal.to_string());
        next.tx_time = 100 + ordinal;
        next.valid_from = 100 + ordinal;
        store
            .claims()
            .append_batch(std::slice::from_ref(&next))
            .unwrap();
        claims.push(next);
        store.flush(ordinal + 1).unwrap();
    }
    claims.reverse();
    assert_eq!(
        store
            .claims()
            .history(&claims[0].subject, &claims[0].predicate)
            .unwrap(),
        claims
    );
    let before = store.manifest().unwrap();
    let entries = |manifest: &rrd_lsm::Manifest| {
        manifest
            .segments
            .iter()
            .map(|segment| segment.entries)
            .sum::<u64>()
    };
    let mut tail = store.begin_transaction().unwrap();
    tail.put(b"hot".to_vec(), b"unflushed".to_vec()).unwrap();
    tail.commit(Durability::Authoritative).unwrap();
    let acknowledged = store.physical_store_evidence().unwrap().physical_sequence;
    for _ in 0..256 {
        if !store.try_maintenance_step().unwrap() {
            break;
        }
        assert_eq!(held.get(b"hot").unwrap(), Some(b"initial".to_vec()));
        assert_eq!(held.get(b"deleted").unwrap(), Some(b"initial".to_vec()));
    }
    let after = store.manifest().unwrap();
    eprintln!(
        "owned_history_physical_entries={} -> {}",
        entries(&before),
        entries(&after)
    );
    assert!(
        entries(&after) + 32 <= entries(&before),
        "superseded physical history must shrink: {} -> {}",
        entries(&before),
        entries(&after)
    );
    assert_eq!(after.wal_start_sequence, before.wal_start_sequence);
    assert_eq!(after.durable_sequence, before.durable_sequence);
    assert_eq!(
        store
            .physical_store_evidence()
            .unwrap()
            .compaction_debt_segments,
        Some(0)
    );
    let mut actual_pages = Vec::new();
    loop {
        let page = retained.page().unwrap();
        actual_pages.push(page.sha256);
        if !page.source_open {
            break;
        }
        retained.advance(page.ordinal).unwrap();
    }
    assert_eq!(actual_pages, expected_pages);
    drop(retained);
    drop(held);
    drop(store);
    let reopened = RrflowKvStore::open(&path).unwrap();
    assert_eq!(
        reopened
            .physical_store_evidence()
            .unwrap()
            .physical_sequence,
        acknowledged
    );
    let tx = reopened.begin_transaction().unwrap();
    assert_eq!(tx.get(b"hot").unwrap(), Some(b"unflushed".to_vec()));
    assert_eq!(tx.get(b"deleted").unwrap(), None);
    for ordinal in 1..=32 {
        assert_eq!(
            tx.get(format!("history/{ordinal}").as_bytes()).unwrap(),
            Some(ordinal.to_string().into_bytes())
        );
    }
    assert_eq!(
        reopened
            .claims()
            .history(&claims[0].subject, &claims[0].predicate)
            .unwrap(),
        claims
    );
}
