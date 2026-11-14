use super::*;
use rrd_core::{RuntimeCommit, RuntimeSchemaRegistry, ScopeId};

#[test]
fn selection_event_range_authenticates_inclusion_but_not_omission() {
    use crate::{Durability, RrflowKvStore, StorageEngine};
    use rrd_core::{RuntimeEvent, RuntimeEventSchema, RuntimeProperties, RuntimeValue};

    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("selection-omission");
    let store = RrflowKvStore::create_new(&root).unwrap();
    let scope = ScopeId::new("instance:selection-omission").unwrap();
    let kind = RuntimeType::new("selection-omission").unwrap();
    let mut schema = RuntimeSchemaRegistry::empty(1, "selection omission fixture");
    schema
        .define_event_table(
            kind.clone(),
            RuntimeLogicalModel::Event,
            RuntimeEventSchema {
                allow_additional_properties: true,
                ..Default::default()
            },
        )
        .unwrap();
    let event = |label: &str| RuntimeMutation::Event {
        event: RuntimeEvent {
            kind: kind.clone(),
            subject: None,
            properties: RuntimeProperties::from([(
                "generation".into(),
                RuntimeValue::String(label.into()),
            )]),
        },
    };
    let first = store
        .runtime()
        .commit(&RuntimeCommit {
            scope: scope.clone(),
            at: 100,
            actor: "test:omission".into(),
            expected_cursor: 0,
            mutations: vec![RuntimeMutation::Schema { registry: schema }, event("first")],
        })
        .unwrap();
    let second = store
        .runtime()
        .commit(&RuntimeCommit {
            scope: scope.clone(),
            at: 200,
            actor: "test:omission".into(),
            expected_cursor: first.last_cursor,
            mutations: vec![event("second")],
        })
        .unwrap();
    let stamp = store.runtime().read_stamp(&scope).unwrap();
    let source = RuntimeVersionedSource::Events {
        kind: Some(kind.clone()),
    };
    let read = |store: &RrflowKvStore| {
        store.runtime().read_versioned(
            &stamp,
            std::slice::from_ref(&source),
            RuntimeReadBudget::new(4096).unwrap(),
        )
    };
    let before = read(&store).unwrap();
    assert_eq!(before.changes.len(), 2);
    assert_eq!(before.changes.last().unwrap().cursor, second.last_cursor);

    // Inject index loss only. The committed log, accumulator, read stamp,
    // first event, and control metadata are deliberately left intact.
    let reference = RuntimeRef::event(kind.clone(), second.last_cursor);
    let key = keyspaces::runtime_version_key(
        keyspaces::RUNTIME_EVENT_VERSIONS,
        &scope,
        &reference,
        200,
        second.last_cursor,
    );
    let mut transaction = store.begin_transaction().unwrap();
    assert!(transaction.get(&key).unwrap().is_some());
    transaction.delete(key).unwrap();
    transaction.commit(Durability::Authoritative).unwrap();

    // This is characterization of a known gap, not desired behavior:
    // the inclusion-authenticated first event is returned without proving
    // that the newer matching event was omitted from the index.
    let after = read(&store).unwrap();
    assert_eq!(after.changes.len(), 1);
    assert_eq!(after.changes[0].cursor, first.last_cursor);
    after.evidence.validate().unwrap();
    assert_eq!(store.runtime().read_stamp(&scope).unwrap(), stamp);
    drop(store);
    let reopened = RrflowKvStore::open_existing(&root).unwrap();
    let after_reopen = read(&reopened).unwrap();
    assert_eq!(after_reopen.changes, after.changes);
}

fn schema_change() -> (ReadStamp, RuntimeChange) {
    let scope = ScopeId::new("project:version-entry-validation").unwrap();
    let mut registry = RuntimeSchemaRegistry::empty(1, "version entry validation");
    registry
        .define_record_table(
            RuntimeType::new("document").unwrap(),
            RuntimeLogicalModel::Relational,
            Default::default(),
        )
        .unwrap();
    let commit = RuntimeCommit {
        scope: scope.clone(),
        at: 100,
        actor: "test:version-entry-validation".into(),
        expected_cursor: 0,
        mutations: vec![RuntimeMutation::Schema { registry }],
    };
    let change = RuntimeChange::committed(
        1,
        &commit,
        &commit.digest(),
        0,
        commit.mutations[0].clone(),
        None,
    );
    let read = ReadStamp::authenticated(
        scope,
        Some(1),
        0,
        1,
        Some(change.digest.clone()),
        "11".repeat(32),
    )
    .unwrap();
    (read, change)
}

#[test]
fn a_version_value_cannot_move_to_another_typed_key_or_family() {
    let (read, change) = schema_change();
    let mut key = keyspaces::runtime_schema_version_key(&read.scope, change.cursor);
    validate_version_entry(&RuntimeVersionedSource::Schema, &read, &key, &change).unwrap();

    *key.last_mut().unwrap() ^= 1;
    assert!(matches!(
        validate_version_entry(&RuntimeVersionedSource::Schema, &read, &key, &change,),
        Err(Error::Codec(_))
    ));
    assert!(matches!(
        validate_version_entry(
            &RuntimeVersionedSource::Records { kind: None },
            &read,
            &keyspaces::runtime_schema_version_key(&read.scope, change.cursor),
            &change,
        ),
        Err(Error::Codec(_))
    ));
}

fn admit(sources: &[RuntimeVersionedSource]) -> Result<Vec<RuntimeVersionedSource>> {
    SourceRanges::default().admit(
        sources,
        &ScopeId::new("project:overlap-validation").unwrap(),
    )
}

fn adjacency(node: &str, relation: &str) -> RuntimeVersionedSource {
    RuntimeVersionedSource::Adjacency {
        relation: RuntimeType::new(relation).unwrap(),
        node: RuntimeRef::new("document", node).unwrap(),
        direction: RuntimeAdjacencyDirection::Outgoing,
    }
}

fn committed(mutation: RuntimeMutation, cursor: u64) -> RuntimeChange {
    let commit = RuntimeCommit {
        scope: ScopeId::new("project:adjacency-entry-validation").unwrap(),
        at: 300,
        actor: "test:adjacency-entry-validation".into(),
        expected_cursor: cursor - 1,
        mutations: vec![mutation],
    };
    RuntimeChange::committed(
        cursor,
        &commit,
        &commit.digest(),
        0,
        commit.mutations[0].clone(),
        None,
    )
}

#[test]
fn an_adjacency_value_must_name_its_node_type_identity_time_and_cursor() {
    let relation = rrd_core::RuntimeRelation {
        reference: RuntimeRef::new("depends_on", "a-b").unwrap(),
        from: RuntimeRef::new("document", "a").unwrap(),
        to: RuntimeRef::new("document", "b").unwrap(),
        valid_from: 100,
        valid_to: None,
        properties: Default::default(),
    };
    let depends_on = RuntimeType::new("depends_on").unwrap();
    let outgoing = RuntimeAdjacencyDirection::Outgoing;
    let space = adjacency_space(outgoing);
    let put = committed(
        RuntimeMutation::Relation {
            relation: relation.clone(),
        },
        4,
    );
    let key = |relation: &rrd_core::RuntimeRelation, at: u64, cursor: u64| {
        keyspaces::runtime_adjacency_version_key(space, &put.scope, relation, at, cursor)
    };
    let validate = |node: &RuntimeRef,
                    direction: RuntimeAdjacencyDirection,
                    stored: &[u8],
                    change: &RuntimeChange| {
        validate_adjacency_entry(&depends_on, node, direction, stored, change)
    };
    validate(&relation.from, outgoing, &key(&relation, 100, 4), &put).unwrap();
    let mut moved = relation.clone();
    moved.to = RuntimeRef::new("document", "c").unwrap();
    for (node, direction, stored) in [
        (&relation.from, outgoing, key(&moved, 100, 4)),
        (&relation.from, outgoing, key(&relation, 101, 4)),
        (&relation.from, outgoing, key(&relation, 100, 5)),
        (&relation.to, outgoing, key(&relation, 100, 4)),
        (
            &relation.from,
            RuntimeAdjacencyDirection::Incoming,
            key(&relation, 100, 4),
        ),
    ] {
        assert!(matches!(
            validate(node, direction, &stored, &put),
            Err(Error::Codec(_))
        ));
    }

    let retirement = committed(
        RuntimeMutation::Retire {
            retirement: rrd_core::RuntimeRetirement {
                model: RuntimeLogicalModel::GraphRelation,
                reference: relation.reference.clone(),
                effective_at: 300,
            },
        },
        5,
    );
    validate(
        &relation.from,
        outgoing,
        &key(&relation, 300, 5),
        &retirement,
    )
    .unwrap();
    // The far endpoint is not part of a retirement and is not checked.
    validate(&relation.from, outgoing, &key(&moved, 300, 5), &retirement).unwrap();
    let mut renamed = relation.clone();
    renamed.reference = RuntimeRef::new("depends_on", "a-c").unwrap();
    let mut retyped = relation.clone();
    retyped.reference = RuntimeRef::new("cites", "a-b").unwrap();
    for stored in [
        key(&relation, 299, 5),
        key(&relation, 300, 6),
        key(&renamed, 300, 5),
        key(&retyped, 300, 5),
    ] {
        assert!(matches!(
            validate(&relation.from, outgoing, &stored, &retirement),
            Err(Error::Codec(_))
        ));
    }
    assert!(validate(&relation.to, outgoing, &key(&relation, 300, 5), &retirement).is_err());
    let record_retirement = committed(
        RuntimeMutation::Retire {
            retirement: rrd_core::RuntimeRetirement {
                model: RuntimeLogicalModel::Document,
                reference: relation.reference.clone(),
                effective_at: 300,
            },
        },
        5,
    );
    assert!(matches!(
        validate(
            &relation.from,
            outgoing,
            &key(&relation, 300, 5),
            &record_retirement
        ),
        Err(Error::Codec(_))
    ));
}

#[test]
fn overlapping_family_ranges_are_rejected_before_storage_access() {
    assert!(admit(&[
        RuntimeVersionedSource::Records { kind: None },
        RuntimeVersionedSource::Records {
            kind: Some(RuntimeType::new("document").unwrap()),
        },
    ])
    .is_err());
    assert!(admit(&[
        RuntimeVersionedSource::Records {
            kind: Some(RuntimeType::new("document").unwrap()),
        },
        RuntimeVersionedSource::Identity {
            model: RuntimeLogicalModel::Document,
            reference: RuntimeRef::new("document", "readme").unwrap(),
        },
    ])
    .is_err());
    // One physical identity selected under two record-like models is one
    // range, not two reads.
    assert!(admit(&[
        RuntimeVersionedSource::Identity {
            model: RuntimeLogicalModel::Document,
            reference: RuntimeRef::new("document", "readme").unwrap(),
        },
        RuntimeVersionedSource::Identity {
            model: RuntimeLogicalModel::Relational,
            reference: RuntimeRef::new("document", "readme").unwrap(),
        },
    ])
    .is_err());
}

#[test]
fn admitted_ranges_deduplicate_and_find_overlap_in_any_order() {
    let a = adjacency("a", "depends_on");
    let ab = adjacency("ab", "depends_on");
    let other_kind = adjacency("a", "depends_onx");
    let incoming = RuntimeVersionedSource::Adjacency {
        relation: RuntimeType::new("depends_on").unwrap(),
        node: RuntimeRef::new("document", "a").unwrap(),
        direction: RuntimeAdjacencyDirection::Incoming,
    };
    assert_eq!(
        admit(&[
            a.clone(),
            ab.clone(),
            a.clone(),
            other_kind.clone(),
            incoming.clone()
        ])
        .unwrap(),
        vec![a.clone(), ab, other_kind, incoming]
    );

    let scope = ScopeId::new("project:overlap-validation").unwrap();
    let mut ranges = SourceRanges::default();
    let family = RuntimeVersionedSource::Records {
        kind: Some(RuntimeType::new("document").unwrap()),
    };
    let identity = RuntimeVersionedSource::Identity {
        model: RuntimeLogicalModel::Document,
        reference: RuntimeRef::new("document", "readme").unwrap(),
    };
    assert_eq!(
        ranges
            .admit(std::slice::from_ref(&identity), &scope)
            .unwrap(),
        vec![identity.clone()]
    );
    assert!(ranges
        .admit(std::slice::from_ref(&identity), &scope)
        .unwrap()
        .is_empty());
    assert!(ranges.admit(std::slice::from_ref(&family), &scope).is_err());
}
