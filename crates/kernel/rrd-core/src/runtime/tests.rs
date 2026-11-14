use super::*;
use crate::Producer;

fn record(kind: &str, id: &str, at: u64) -> RuntimeRecord {
    RuntimeRecord {
        reference: RuntimeRef::new(kind, id).unwrap(),
        valid_from: at,
        valid_to: None,
        properties: RuntimeProperties::new(),
    }
}

#[test]
fn data_snapshot_never_infers_a_model_for_an_absent_table() {
    let scope = ScopeId::new("instance:missing-table-model").unwrap();
    let mut schema = RuntimeSchemaRegistry::empty(1, "reject inferred snapshot models");
    schema.records.insert(
        RuntimeType::new("declared").unwrap(),
        crate::RuntimeRecordSchema::default(),
    );
    let commit = RuntimeCommit {
        scope: scope.clone(),
        at: 10,
        actor: "agent:test".into(),
        expected_cursor: 0,
        mutations: vec![RuntimeMutation::Record {
            record: record("undeclared", "one", 10),
        }],
    };
    let change = RuntimeChange::committed(
        1,
        &commit,
        &commit.digest(),
        0,
        commit.mutations[0].clone(),
        None,
    );

    assert!(RuntimeDataSnapshot::from_changes(&[change], &schema, scope, 10, 1).is_err());
}

#[test]
fn commit_identity_is_stable_and_sensitive_to_order() {
    let base = RuntimeCommit {
        scope: ScopeId::new("instance:test").unwrap(),
        at: 10,
        actor: "agent:test".into(),
        expected_cursor: 0,
        mutations: vec![RuntimeMutation::Record {
            record: record("prompt", "p1", 10),
        }],
    };
    assert_eq!(base.digest(), base.clone().digest());
    let mut changed = base.clone();
    changed.mutations.push(RuntimeMutation::Record {
        record: record("outcome", "o1", 10),
    });
    assert_ne!(base.digest(), changed.digest());
}

#[test]
fn vector_commit_identity_covers_collection_and_named_vector() {
    let vector = |collection_id: &str, vector_name: &str| RuntimeCommit {
        scope: ScopeId::new("instance:vector-digest").unwrap(),
        at: 10,
        actor: "agent:test".into(),
        expected_cursor: 0,
        mutations: vec![RuntimeMutation::Vector {
            vector: RuntimeVector {
                reference: RuntimeRef::new("embedding", "document-a-body").unwrap(),
                subject: RuntimeRef::new("document", "a").unwrap(),
                collection: crate::VectorCollectionAddress {
                    collection_id: collection_id.into(),
                    vector_name: vector_name.into(),
                },
                field: "body".into(),
                valid_from: 10,
                valid_to: None,
                value: VectorValue::Dense {
                    values: vec![1.0, 0.0],
                },
                provenance: None,
                properties: RuntimeProperties::new(),
            },
        }],
    };

    let body = vector("documents", "body");
    assert_ne!(body.digest(), vector("archive", "body").digest());
    assert_ne!(body.digest(), vector("documents", "summary").digest());
}

#[test]
fn snapshot_is_bitemporal_and_traversable() {
    let scope = ScopeId::new("instance:test").unwrap();
    let prompt = record("prompt", "p1", 10);
    let outcome = record("outcome", "o1", 10);
    let relation = RuntimeRelation {
        reference: RuntimeRef::new("caused", "p1-o1").unwrap(),
        from: prompt.reference.clone(),
        to: outcome.reference.clone(),
        valid_from: 10,
        valid_to: None,
        properties: RuntimeProperties::new(),
    };
    let commit = RuntimeCommit {
        scope: scope.clone(),
        at: 10,
        actor: "agent:test".into(),
        expected_cursor: 0,
        mutations: vec![
            RuntimeMutation::Record {
                record: prompt.clone(),
            },
            RuntimeMutation::Record {
                record: outcome.clone(),
            },
            RuntimeMutation::Relation { relation },
        ],
    };
    let id = commit.digest();
    let mut previous = None;
    let changes = commit
        .mutations
        .clone()
        .into_iter()
        .enumerate()
        .map(|(index, mutation)| {
            let change = RuntimeChange::committed(
                index as u64 + 1,
                &commit,
                &id,
                index as u64,
                mutation,
                previous.clone(),
            );
            previous = Some(change.digest.clone());
            change
        })
        .collect::<Vec<_>>();
    let graph = RuntimeGraphSnapshot::from_changes(&changes, scope, 10, 3);
    assert_eq!(graph.records.len(), 2);
    assert_eq!(graph.relations.len(), 1);
    assert_eq!(
        graph.traverse(&prompt.reference, 1, &BTreeSet::new()).len(),
        2
    );
}

#[test]
fn read_stamps_and_snapshot_handles_fail_closed_on_tampering() {
    let read = ReadStamp::new(
        ScopeId::new("instance:test").unwrap(),
        Some(1),
        0,
        2,
        Some("11".repeat(32)),
    )
    .unwrap();
    let snapshot = SnapshotHandle::new(read.clone(), "agent:test", 100, 50).unwrap();
    let mut pin = RetentionPin::from_snapshot(&snapshot).unwrap();
    pin.minimum_cursor += 1;
    assert!(pin.validate().is_err());
    assert!(!snapshot.is_expired(149));
    assert!(snapshot.is_expired(150));

    let mut changed_read = read;
    changed_read.commit_cursor = 3;
    assert!(changed_read.validate().is_err());

    let mut authenticated = ReadStamp::authenticated(
        ScopeId::new("instance:test").unwrap(),
        Some(1),
        0,
        2,
        Some("11".repeat(32)),
        "22".repeat(32),
    )
    .unwrap();
    authenticated.accumulator_root = Some("33".repeat(32));
    assert!(authenticated.validate().is_err());

    let mut future_snapshot = snapshot.clone();
    future_snapshot.contract_version = DATA_RUNTIME_CONTRACT_VERSION + 1;
    assert!(future_snapshot.validate().is_err());

    let mut changed_snapshot = snapshot;
    changed_snapshot.owner = "agent:other".into();
    assert!(changed_snapshot.validate().is_err());
}

#[test]
fn transaction_and_audit_envelopes_bind_every_causal_field() {
    let scope = ScopeId::new("instance:test").unwrap();
    let read = ReadStamp::new(scope.clone(), None, 0, 0, None).unwrap();
    let commit = RuntimeCommit {
        scope: scope.clone(),
        at: 10,
        actor: "agent:test".into(),
        expected_cursor: 0,
        mutations: vec![RuntimeMutation::Event {
            event: RuntimeEvent {
                kind: RuntimeType::new("pulse").unwrap(),
                subject: None,
                properties: RuntimeProperties::new(),
            },
        }],
    };
    let transaction = DataTransaction::new(read.clone(), commit).unwrap();
    assert_eq!(transaction.digest(), transaction.clone().digest());

    let audit = AuditEnvelope {
        contract_version: DATA_RUNTIME_CONTRACT_VERSION,
        request_id: "request:test".into(),
        parent_request_id: None,
        at: 10,
        actor: "agent:test".into(),
        scope,
        operation: "runtime.commit".into(),
        resource: transaction.digest(),
        read: Some(read),
        decision: AuditDecision::Allow,
        outcome_cursor: Some(1),
        duration_ms: 3,
        previous_digest: None,
        digest: String::new(),
    }
    .seal()
    .unwrap();
    audit.validate().unwrap();
    let mut tampered = audit;
    tampered.duration_ms = 4;
    assert!(tampered.validate().is_err());
}

#[test]
fn transaction_preview_reads_its_writes_without_mutating_the_base() {
    let scope = ScopeId::new("instance:preview").unwrap();
    let reference = RuntimeRef::new("item", "one").unwrap();
    let mut before = record("item", "one", 1);
    before
        .properties
        .insert("state".into(), RuntimeValue::String("before".into()));
    let base = RuntimeGraphSnapshot {
        scope: scope.clone(),
        valid_at: 20,
        known_at_cursor: 3,
        records: vec![before.clone()],
        relations: Vec::new(),
    };
    let read = ReadStamp::new(scope.clone(), Some(1), 0, 3, Some("11".repeat(32))).unwrap();
    let mut after = record("item", "one", 10);
    after
        .properties
        .insert("state".into(), RuntimeValue::String("after".into()));
    let transaction = DataTransaction::new(
        read,
        RuntimeCommit {
            scope,
            at: 10,
            actor: "agent:preview".into(),
            expected_cursor: 3,
            mutations: vec![RuntimeMutation::Record {
                record: after.clone(),
            }],
        },
    )
    .unwrap();

    let view = transaction.preview(&base).unwrap();
    assert_eq!(view.prospective_cursor, 4);
    assert_eq!(view.pending_mutations, 1);
    assert_eq!(view.record(&reference), Some(&after));
    assert_eq!(
        base.records,
        vec![before],
        "the committed base is immutable"
    );

    let mut wrong_base = base;
    wrong_base.known_at_cursor = 2;
    assert!(transaction.preview(&wrong_base).is_err());
}

#[test]
fn claim_only_snapshot_needs_no_parallel_schema_or_recall_path() {
    let scope = ScopeId::new("instance:claims").unwrap();
    let claim = Claim::new(
        Subject::new("project-alpha").unwrap(),
        Predicate::new("status").unwrap(),
        "durable authentication recovery",
        10,
        10,
        Producer {
            actor: "agent:test".into(),
            on_behalf_of: None,
            session: None,
        },
    );
    let commit = RuntimeCommit {
        scope: scope.clone(),
        at: 10,
        actor: "agent:test".into(),
        expected_cursor: 0,
        mutations: vec![RuntimeMutation::Claim {
            claim: claim.clone(),
        }],
    };
    let change = RuntimeChange::committed(
        1,
        &commit,
        &commit.digest(),
        0,
        commit.mutations[0].clone(),
        None,
    );
    let snapshot = RuntimeDataSnapshot::from_claim_changes(&[change], scope, 20, 1).unwrap();
    assert_eq!(snapshot.schema_revision, 0);
    assert_eq!(snapshot.claims, vec![claim]);
    assert!(snapshot.records.is_empty());
}
