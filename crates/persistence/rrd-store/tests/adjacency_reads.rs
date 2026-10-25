//! Temporal adjacency ranges are exact candidate selectors: each returns
//! every authenticated relation version and retirement the commit path
//! indexed under one node, type, and direction, and nothing else.

use std::collections::{BTreeMap, BTreeSet};

use rrd_core::{
    ReadStamp, RuntimeChange, RuntimeCommit, RuntimeLogicalModel, RuntimeMutation,
    RuntimeProperties, RuntimeRecord, RuntimeRecordSchema, RuntimeRef, RuntimeRelation,
    RuntimeRelationSchema, RuntimeRetirement, RuntimeSchemaRegistry, RuntimeType, ScopeId,
};
use rrd_store::{
    Error, RrflowKvStore, RrflowMxStore, RuntimeAdjacencyDirection, RuntimeReadAccessPath,
    RuntimeReadBudget, RuntimeVersionedSource, StorageEngine,
};

const BUDGET: u64 = 10_000;

fn scope() -> ScopeId {
    ScopeId::new("project:adjacency-read").unwrap()
}

fn node(id: &str) -> RuntimeRef {
    RuntimeRef::new("entity", id).unwrap()
}

fn kind(name: &str) -> RuntimeType {
    RuntimeType::new(name).unwrap()
}

fn schema(revision: u64, entity_model: RuntimeLogicalModel) -> RuntimeSchemaRegistry {
    let mut registry = RuntimeSchemaRegistry::empty(revision, "adjacency read proof");
    registry
        .define_record_table(kind("entity"), entity_model, RuntimeRecordSchema::default())
        .unwrap();
    for relation in ["links", "cites"] {
        registry
            .define_relation_table(
                kind(relation),
                RuntimeRelationSchema {
                    from: BTreeSet::from([kind("entity")]),
                    to: BTreeSet::from([kind("entity")]),
                    ..RuntimeRelationSchema::default()
                },
            )
            .unwrap();
    }
    registry
}

fn entity(id: &str) -> RuntimeMutation {
    RuntimeMutation::Record {
        record: RuntimeRecord {
            reference: node(id),
            valid_from: 100,
            valid_to: None,
            properties: RuntimeProperties::new(),
        },
    }
}

fn relation(kind_name: &str, id: &str, from: &str, to: &str, valid_from: u64) -> RuntimeMutation {
    RuntimeMutation::Relation {
        relation: RuntimeRelation {
            reference: RuntimeRef::new(kind_name, id).unwrap(),
            from: node(from),
            to: node(to),
            valid_from,
            valid_to: None,
            properties: RuntimeProperties::new(),
        },
    }
}

fn retire(model: RuntimeLogicalModel, reference: RuntimeRef, at: u64) -> RuntimeMutation {
    RuntimeMutation::Retire {
        retirement: RuntimeRetirement {
            model,
            reference,
            effective_at: at,
        },
    }
}

fn commit(engine: &dyn StorageEngine, at: u64, mutations: Vec<RuntimeMutation>) {
    let expected_cursor = engine.runtime().cursor().unwrap();
    engine
        .runtime()
        .commit(&RuntimeCommit {
            scope: scope(),
            at,
            actor: "test:adjacency-read".into(),
            expected_cursor,
            mutations,
        })
        .unwrap();
}

/// Puts, an endpoint move in each direction, a self-loop, a second relation
/// type, a retirement, and a re-put after retirement. Returns the stamp after
/// the first commit and the final stamp.
fn commit_corpus(engine: &dyn StorageEngine) -> (ReadStamp, ReadStamp) {
    commit(
        engine,
        100,
        vec![
            RuntimeMutation::Schema {
                registry: schema(1, RuntimeLogicalModel::Relational),
            },
            entity("a"),
            entity("b"),
            entity("c"),
            entity("d"),
            relation("links", "a-b", "a", "b", 100),
            relation("links", "a-c", "a", "c", 100),
            relation("links", "b-a", "b", "a", 100),
            relation("links", "a-a", "a", "a", 100),
            relation("cites", "a-b", "a", "b", 100),
        ],
    );
    let historical = engine.runtime().read_stamp(&scope()).unwrap();
    commit(
        engine,
        200,
        vec![
            // Moves the source endpoint away from `a`.
            relation("links", "a-c", "d", "c", 200),
            // Moves the target endpoint onto `a`.
            relation("links", "a-b", "a", "a", 200),
        ],
    );
    commit(
        engine,
        300,
        vec![retire(
            RuntimeLogicalModel::GraphRelation,
            RuntimeRef::new("links", "b-a").unwrap(),
            300,
        )],
    );
    commit(engine, 400, vec![relation("links", "b-a", "b", "a", 400)]);
    (historical, engine.runtime().read_stamp(&scope()).unwrap())
}

/// The exact commit-path oracle: a relation version is indexed under its own
/// endpoints; a retirement under the endpoints of the relation current when
/// it was committed.
fn expected(
    changes: &[RuntimeChange],
    relation_kind: &RuntimeType,
    at: &RuntimeRef,
    direction: RuntimeAdjacencyDirection,
) -> Vec<RuntimeChange> {
    let near = |relation: &RuntimeRelation| match direction {
        RuntimeAdjacencyDirection::Outgoing => relation.from.clone(),
        RuntimeAdjacencyDirection::Incoming => relation.to.clone(),
    };
    let mut current = BTreeMap::<RuntimeRef, RuntimeRelation>::new();
    let mut selected = Vec::new();
    for change in changes {
        match &change.mutation {
            RuntimeMutation::Relation { relation } => {
                if &relation.reference.kind == relation_kind && &near(relation) == at {
                    selected.push(change.clone());
                }
                current.insert(relation.reference.clone(), relation.clone());
            }
            RuntimeMutation::Retire { retirement }
                if retirement.model == RuntimeLogicalModel::GraphRelation =>
            {
                if let Some(relation) = current.remove(&retirement.reference) {
                    if &relation.reference.kind == relation_kind && &near(&relation) == at {
                        selected.push(change.clone());
                    }
                }
            }
            _ => {}
        }
    }
    selected
}

fn adjacency(
    relation_kind: &str,
    at: &str,
    direction: RuntimeAdjacencyDirection,
) -> RuntimeVersionedSource {
    RuntimeVersionedSource::Adjacency {
        relation: kind(relation_kind),
        node: node(at),
        direction,
    }
}

fn assert_adjacency_matches_oracle(engine: &dyn StorageEngine) {
    let (historical, read) = commit_corpus(engine);
    let budget = RuntimeReadBudget::new(BUDGET).unwrap();
    let head_log = engine
        .runtime()
        .read_changes(&read, 0, usize::MAX)
        .unwrap()
        .changes;
    let mut non_empty = 0;
    for read in [&historical, &read] {
        let log = engine
            .runtime()
            .read_changes(read, 0, usize::MAX)
            .unwrap()
            .changes;
        for relation_kind in ["links", "cites"] {
            for at in ["a", "b", "c", "d", "missing"] {
                for direction in [
                    RuntimeAdjacencyDirection::Outgoing,
                    RuntimeAdjacencyDirection::Incoming,
                ] {
                    let actual = engine
                        .runtime()
                        .read_versioned(read, &[adjacency(relation_kind, at, direction)], budget)
                        .unwrap();
                    let oracle = expected(&log, &kind(relation_kind), &node(at), direction);
                    assert_eq!(actual.changes, oracle, "{relation_kind} {at} {direction:?}");
                    non_empty += usize::from(!oracle.is_empty());
                    let path = actual
                        .evidence
                        .paths
                        .iter()
                        .find(|path| path.path == RuntimeReadAccessPath::AdjacencyVersions)
                        .unwrap();
                    // The physical range holds every entry up to the head; a
                    // historical stamp examines and then excludes later ones.
                    let physical = expected(&head_log, &kind(relation_kind), &node(at), direction);
                    assert_eq!(path.range_scans, 1);
                    assert_eq!(path.keys_examined, u64::try_from(physical.len()).unwrap());
                    assert!(!actual
                        .evidence
                        .paths
                        .iter()
                        .any(|path| path.path == RuntimeReadAccessPath::RelationVersions));
                }
            }
        }
    }
    assert!(non_empty >= 12);

    // Outgoing from `a` keeps the moved-away version of a-c, both a-b
    // versions, and a-a; the b-a retirement is indexed under b's outgoing and
    // a's incoming ranges.
    let outgoing = engine
        .runtime()
        .read_versioned(
            &read,
            &[adjacency("links", "a", RuntimeAdjacencyDirection::Outgoing)],
            budget,
        )
        .unwrap();
    let identities = outgoing
        .changes
        .iter()
        .map(|change| match &change.mutation {
            RuntimeMutation::Relation { relation } => relation.reference.id.to_string(),
            other => panic!("unexpected {other:?}"),
        })
        .collect::<Vec<_>>();
    assert_eq!(identities, ["a-b", "a-c", "a-a", "a-b"]);
    let incoming_a = engine
        .runtime()
        .read_versioned(
            &read,
            &[adjacency("links", "a", RuntimeAdjacencyDirection::Incoming)],
            budget,
        )
        .unwrap();
    assert_eq!(
        incoming_a
            .changes
            .iter()
            .filter(|change| matches!(&change.mutation, RuntimeMutation::Retire { .. }))
            .count(),
        1
    );
    let outgoing_b = engine
        .runtime()
        .read_versioned(
            &read,
            &[adjacency("links", "b", RuntimeAdjacencyDirection::Outgoing)],
            budget,
        )
        .unwrap();
    assert_eq!(
        outgoing_b
            .changes
            .iter()
            .map(|change| matches!(&change.mutation, RuntimeMutation::Retire { .. }))
            .collect::<Vec<_>>(),
        [false, true, false]
    );
}

#[test]
fn adjacency_ranges_match_the_commit_oracle_on_every_engine_and_reopen() {
    let mx = RrflowMxStore::new();
    assert_adjacency_matches_oracle(&mx);

    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("rrflow-kv");
    {
        let kv = RrflowKvStore::open(&path).unwrap();
        assert_adjacency_matches_oracle(&kv);
        kv.flush(500).unwrap();
    }
    let reopened = RrflowKvStore::open(&path).unwrap();
    let read = reopened.runtime().read_stamp(&scope()).unwrap();
    let mx_read = mx.runtime().read_stamp(&scope()).unwrap();
    for direction in [
        RuntimeAdjacencyDirection::Outgoing,
        RuntimeAdjacencyDirection::Incoming,
    ] {
        let source = [adjacency("links", "a", direction)];
        let budget = RuntimeReadBudget::new(BUDGET).unwrap();
        let left = mx
            .runtime()
            .read_versioned(&mx_read, &source, budget)
            .unwrap();
        let right = reopened
            .runtime()
            .read_versioned(&read, &source, budget)
            .unwrap();
        assert_eq!(left.changes, right.changes);
        assert_eq!(left.evidence, right.evidence);
    }
}

#[test]
fn expanding_reads_validate_one_stamp_and_never_reread_a_range() {
    let engine = RrflowMxStore::new();
    let (_, read) = commit_corpus(&engine);
    let budget = RuntimeReadBudget::new(BUDGET).unwrap();
    let start = [adjacency("links", "a", RuntimeAdjacencyDirection::Outgoing)];
    let mut steps = Vec::new();
    let expanded = engine
        .runtime()
        .read_versioned_expanding(&read, &start, budget, &mut |changes| {
            steps.push(
                changes
                    .iter()
                    .map(|change| change.cursor)
                    .collect::<Vec<_>>(),
            );
            let mut next = BTreeSet::new();
            for change in changes {
                if let RuntimeMutation::Relation { relation } = &change.mutation {
                    next.insert(relation.reference.clone());
                }
            }
            // Re-request the first range every step; it is never read twice.
            let mut sources = start.to_vec();
            sources.extend(
                next.into_iter()
                    .map(|reference| RuntimeVersionedSource::Identity {
                        model: RuntimeLogicalModel::GraphRelation,
                        reference,
                    }),
            );
            sources
        })
        .unwrap();
    // Step two adds a-c's move to d, which is not indexed under a; step
    // three selects only ranges already read and ends the read.
    assert_eq!(steps.len(), 2);
    assert_eq!(steps[0].len(), 4);
    assert_eq!(steps[1].len(), 5);

    let separately = engine
        .runtime()
        .read_versioned(
            &read,
            &[
                adjacency("links", "a", RuntimeAdjacencyDirection::Outgoing),
                RuntimeVersionedSource::Identity {
                    model: RuntimeLogicalModel::GraphRelation,
                    reference: RuntimeRef::new("links", "a-a").unwrap(),
                },
                RuntimeVersionedSource::Identity {
                    model: RuntimeLogicalModel::GraphRelation,
                    reference: RuntimeRef::new("links", "a-b").unwrap(),
                },
                RuntimeVersionedSource::Identity {
                    model: RuntimeLogicalModel::GraphRelation,
                    reference: RuntimeRef::new("links", "a-c").unwrap(),
                },
            ],
            budget,
        )
        .unwrap();
    assert_eq!(expanded.changes, separately.changes);
    assert_eq!(expanded.evidence, separately.evidence);
    let stamp = expanded
        .evidence
        .paths
        .iter()
        .find(|path| path.path == RuntimeReadAccessPath::ReadStamp)
        .unwrap();
    assert_eq!(
        stamp,
        separately
            .evidence
            .paths
            .iter()
            .find(|path| path.path == RuntimeReadAccessPath::ReadStamp)
            .unwrap()
    );

    let overlapping = engine
        .runtime()
        .read_versioned_expanding(&read, &start, budget, &mut |_| {
            vec![
                RuntimeVersionedSource::Relations {
                    kind: Some(kind("links")),
                },
                RuntimeVersionedSource::Identity {
                    model: RuntimeLogicalModel::GraphRelation,
                    reference: RuntimeRef::new("links", "a-b").unwrap(),
                },
            ]
        });
    assert!(matches!(overlapping, Err(Error::Substrate(_))));
}

#[test]
fn identity_history_keeps_a_retirement_recorded_under_an_earlier_record_model() {
    let engine = RrflowMxStore::new();
    let record = entity("e");
    commit(
        &engine,
        100,
        vec![
            RuntimeMutation::Schema {
                registry: schema(1, RuntimeLogicalModel::Relational),
            },
            record.clone(),
        ],
    );
    commit(
        &engine,
        200,
        vec![retire(RuntimeLogicalModel::Relational, node("e"), 200)],
    );
    commit(
        &engine,
        300,
        vec![
            RuntimeMutation::Schema {
                registry: schema(2, RuntimeLogicalModel::Document),
            },
            record,
        ],
    );
    let read = engine.runtime().read_stamp(&scope()).unwrap();
    let budget = RuntimeReadBudget::new(BUDGET).unwrap();
    let identity = engine
        .runtime()
        .read_versioned(
            &read,
            &[RuntimeVersionedSource::Identity {
                model: RuntimeLogicalModel::Document,
                reference: node("e"),
            }],
            budget,
        )
        .unwrap();
    let family = engine
        .runtime()
        .read_versioned(
            &read,
            &[RuntimeVersionedSource::Records {
                kind: Some(kind("entity")),
            }],
            budget,
        )
        .unwrap();
    assert_eq!(identity.changes.len(), 3);
    assert_eq!(identity.changes, family.changes);
}
