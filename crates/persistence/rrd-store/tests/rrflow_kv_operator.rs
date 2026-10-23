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
