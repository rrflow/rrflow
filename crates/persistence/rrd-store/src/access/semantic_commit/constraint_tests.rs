use super::*;
use crate::{RrflowKvStore, RrflowMxStore, StorageEngine};
use rrd_core::{RuntimeLogicalModel, RuntimeRecordSchema, RuntimeRelationSchema, RuntimeType};
use std::cell::Cell;

struct Counted<'a> {
    transaction: &'a dyn StorageTransaction,
    rows: Cell<usize>,
}
impl AccessRead for Counted<'_> {
    fn read_key(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        self.transaction.get(key)
    }
    fn scan_range(
        &self,
        start: &[u8],
        end: &[u8],
        limit: usize,
    ) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        let rows = self.transaction.scan(start, end, limit)?;
        self.rows.set(self.rows.get() + rows.len());
        Ok(rows)
    }
}
fn node(id: &str) -> RuntimeRecord {
    RuntimeRecord {
        reference: RuntimeRef::new("node", id).unwrap(),
        valid_from: 10,
        valid_to: None,
        properties: BTreeMap::new(),
    }
}
fn edge(id: &str, from: &str, to: &str) -> RuntimeRelation {
    RuntimeRelation {
        reference: RuntimeRef::new("edge", id).unwrap(),
        from: node(from).reference,
        to: node(to).reference,
        valid_from: 10,
        valid_to: None,
        properties: BTreeMap::new(),
    }
}
fn schema() -> RuntimeSchemaRegistry {
    let mut schema = RuntimeSchemaRegistry::empty(1, "constraint fixture");
    schema
        .define_record_table(
            RuntimeType::new("node").unwrap(),
            RuntimeLogicalModel::Relational,
            RuntimeRecordSchema::default(),
        )
        .unwrap();
    schema
        .define_relation_table(
            RuntimeType::new("edge").unwrap(),
            RuntimeRelationSchema {
                from: BTreeSet::from([RuntimeType::new("node").unwrap()]),
                to: BTreeSet::from([RuntimeType::new("node").unwrap()]),
                unique_pair: true,
                max_outgoing: Some(1),
                max_incoming: Some(1),
                ..Default::default()
            },
        )
        .unwrap();
    schema
}
fn candidate(store: &dyn StorageEngine, mutations: Vec<RuntimeMutation>) -> RuntimeCommit {
    RuntimeCommit {
        scope: rrd_core::ScopeId::new("constraint-test").unwrap(),
        at: 20,
        actor: "constraint-test".into(),
        expected_cursor: store.runtime().cursor().unwrap(),
        mutations,
    }
}
fn apply(store: &dyn StorageEngine, mutations: Vec<RuntimeMutation>) {
    store
        .runtime()
        .commit(&candidate(store, mutations))
        .unwrap();
}
fn rows(store: &dyn StorageEngine, mutation: RuntimeMutation) -> usize {
    let commit = candidate(store, vec![mutation]);
    let transaction = store.begin_transaction().unwrap();
    let reader = Counted {
        transaction: &*transaction,
        rows: Cell::new(0),
    };
    validate_schema_and_objects(&reader, &commit).unwrap();
    reader.rows.get()
}
fn exercise_growth(store: &dyn StorageEngine) {
    apply(
        store,
        vec![
            RuntimeMutation::Schema { registry: schema() },
            RuntimeMutation::Record { record: node("a") },
            RuntimeMutation::Record { record: node("b") },
            RuntimeMutation::Relation {
                relation: edge("ab", "a", "b"),
            },
        ],
    );
    let update = RuntimeMutation::Relation {
        relation: edge("ab", "a", "b"),
    };
    let before = rows(store, update.clone());
    let growth = (0..96)
        .flat_map(|n| {
            let id = format!("unrelated-{n}");
            [
                RuntimeMutation::Record { record: node(&id) },
                RuntimeMutation::Relation {
                    relation: edge(&id, &id, &id),
                },
            ]
        })
        .collect();
    apply(store, growth);
    assert_eq!(
        rows(store, update),
        before,
        "unrelated relations must not enter constraint input"
    );
    assert_eq!(
        rows(store, RuntimeMutation::Record { record: node("a") }),
        0
    );
}
#[test]
fn constraint_reads_stay_scoped_after_unrelated_growth() {
    exercise_growth(&RrflowMxStore::new());
    let root = tempfile::tempdir().unwrap();
    exercise_growth(&RrflowKvStore::open(root.path()).unwrap());
    let reopened = RrflowKvStore::open(root.path()).unwrap();
    assert!(
        rows(
            &reopened,
            RuntimeMutation::Relation {
                relation: edge("ab", "a", "b")
            }
        ) <= 2
    );
}

fn differential(store: &dyn StorageEngine, mutations: Vec<RuntimeMutation>) -> bool {
    let commit = candidate(store, mutations);
    let transaction = store.begin_transaction().unwrap();
    let records =
        values_for_scope::<RuntimeRecord>(&*transaction, keyspaces::RUNTIME_RECORDS, &commit.scope)
            .unwrap();
    let relations = values_for_scope::<RuntimeRelation>(
        &*transaction,
        keyspaces::RUNTIME_RELATIONS,
        &commit.scope,
    )
    .unwrap();
    let stored: RuntimeSchemaRegistry = get_json(
        &*transaction,
        keyspaces::RUNTIME_SCHEMAS,
        &keyspaces::runtime_schema_key(&commit.scope),
    )
    .unwrap()
    .unwrap();
    let registry = commit
        .mutations
        .iter()
        .find_map(|m| match m {
            RuntimeMutation::Schema { registry } => Some(registry),
            _ => None,
        })
        .unwrap_or(&stored);
    let expected = registry
        .validate_objects(&commit.mutations, &records, &relations)
        .is_ok();
    let narrowed = validate_schema_and_objects(&*transaction, &commit);
    assert_eq!(
        narrowed.is_ok(),
        expected,
        "constraint inputs changed verdict: {commit:?}, {narrowed:?}"
    );
    drop(transaction);
    let result = store.runtime().commit(&commit);
    assert_eq!(
        result.is_ok(),
        expected,
        "public commit differed: {result:?}"
    );
    if !expected {
        assert_eq!(store.runtime().cursor().unwrap(), commit.expected_cursor);
        assert!(store
            .runtime()
            .commit_outcome(&commit.digest())
            .unwrap()
            .is_none());
    }
    expected
}
fn relation(id: &str, from: &str, to: &str, start: u64, end: Option<u64>) -> RuntimeMutation {
    let mut relation = edge(id, from, to);
    relation.valid_from = start;
    relation.valid_to = end;
    RuntimeMutation::Relation { relation }
}
fn retire(id: &str) -> RuntimeMutation {
    RuntimeMutation::Retire {
        retirement: rrd_core::RuntimeRetirement {
            model: RuntimeLogicalModel::GraphRelation,
            reference: edge(id, "a", "b").reference,
            effective_at: 15,
        },
    }
}
fn history(store: &dyn StorageEngine, mask: u8) {
    let mut registry = schema();
    let rule = registry
        .relations
        .get_mut(&RuntimeType::new("edge").unwrap())
        .unwrap();
    rule.unique_pair = mask & 1 != 0;
    rule.max_outgoing = (mask & 2 != 0).then_some(1);
    rule.max_incoming = (mask & 4 != 0).then_some(1);
    let mut initial = vec![RuntimeMutation::Schema { registry }];
    initial.extend(
        ["a", "b", "c", "d", "e"]
            .into_iter()
            .map(|id| RuntimeMutation::Record { record: node(id) }),
    );
    initial.push(relation("base", "a", "b", 10, Some(20)));
    apply(store, initial);
    // Adjacent validity windows are allowed; overlap and each endpoint rule
    // are independently compared with the complete current-state oracle.
    assert!(differential(
        store,
        vec![relation("later", "a", "b", 20, Some(30))]
    ));
    differential(store, vec![relation("overlap", "a", "b", 15, Some(25))]);
    differential(store, vec![relation("out", "a", "c", 12, Some(18))]);
    differential(store, vec![relation("in", "c", "b", 12, Some(18))]);
    assert!(differential(
        store,
        vec![relation("base", "d", "e", 10, Some(20))]
    ));
    differential(
        store,
        vec![
            retire("base"),
            relation("replacement", "d", "e", 10, Some(20)),
        ],
    );
    // Some unconstrained masks accepted and retired base in the last batch.
    let transaction = store.begin_transaction().unwrap();
    let alive = get(
        &*transaction,
        keyspaces::RUNTIME_RELATIONS,
        &keyspaces::runtime_identity_key(
            keyspaces::RUNTIME_RELATIONS,
            &candidate(store, vec![]).scope,
            &edge("base", "d", "e").reference,
        ),
    )
    .unwrap()
    .is_some();
    drop(transaction);
    if alive {
        assert!(differential(store, vec![retire("base")]));
    }
    differential(store, vec![relation("base", "d", "e", 30, Some(40))]);
    differential(
        store,
        vec![
            relation("batch-a", "c", "e", 50, Some(60)),
            relation("batch-b", "c", "d", 50, Some(60)),
        ],
    );
    let mut stricter = schema();
    stricter.revision = 2;
    differential(store, vec![RuntimeMutation::Schema { registry: stricter }]);
    let mut stale = candidate(
        store,
        vec![RuntimeMutation::Record {
            record: node("stale"),
        }],
    );
    stale.expected_cursor = 0;
    assert!(matches!(
        store.runtime().commit(&stale),
        Err(Error::RuntimeConflict { .. })
    ));
}

#[test]
fn constrained_histories_match_whole_scope_for_every_rule_combination() {
    for mask in 0..8 {
        history(&RrflowMxStore::new(), mask);
        let root = tempfile::tempdir().unwrap();
        history(&RrflowKvStore::open(root.path()).unwrap(), mask);
        let reopened = RrflowKvStore::open(root.path()).unwrap();
        differential(
            &reopened,
            vec![relation("reopened", "c", "d", 50, Some(60))],
        );
    }
}

#[test]
fn unique_records_keep_typed_scope_and_schema_change_oracle() {
    let root = tempfile::tempdir().unwrap();
    let store = RrflowKvStore::open(root.path()).unwrap();
    let mut registry = schema();
    let unique = RuntimeType::new("unique").unwrap();
    registry
        .define_record_table(
            unique.clone(),
            RuntimeLogicalModel::Relational,
            RuntimeRecordSchema {
                properties: BTreeMap::from([(
                    "code".into(),
                    rrd_core::RuntimePropertySchema::required(rrd_core::RuntimeValueType::String),
                )]),
                unique_properties: BTreeSet::from(["code".into()]),
                ..Default::default()
            },
        )
        .unwrap();
    apply(&store, vec![RuntimeMutation::Schema { registry }]);
    let record = |id: &str, code: &str| {
        let mut record = node(id);
        record.reference.kind = unique.clone();
        record
            .properties
            .insert("code".into(), rrd_core::RuntimeValue::String(code.into()));
        RuntimeMutation::Record { record }
    };
    assert!(differential(&store, vec![record("one", "x")]));
    apply(
        &store,
        (0..96)
            .map(|n| RuntimeMutation::Record {
                record: node(&format!("other-{n}")),
            })
            .collect(),
    );
    assert_eq!(rows(&store, record("one", "x")), 1);
    assert_eq!(
        rows(
            &store,
            RuntimeMutation::Record {
                record: node("other")
            }
        ),
        0
    );
    assert!(!differential(&store, vec![record("two", "x")]));
    assert!(differential(
        &store,
        vec![record("one", "y"), record("two", "x")]
    ));
    drop(store);
    let store = RrflowKvStore::open(root.path()).unwrap();
    assert!(!differential(&store, vec![record("two", "y")]));
}

#[test]
fn pair_uniqueness_does_not_scan_other_pairs_at_the_same_endpoint() {
    let root = tempfile::tempdir().unwrap();
    let store = RrflowKvStore::open(root.path()).unwrap();
    let mut registry = schema();
    let rule = registry
        .relations
        .get_mut(&RuntimeType::new("edge").unwrap())
        .unwrap();
    rule.max_incoming = None;
    rule.max_outgoing = None;
    apply(
        &store,
        vec![
            RuntimeMutation::Schema { registry },
            RuntimeMutation::Record { record: node("a") },
            RuntimeMutation::Record { record: node("b") },
            relation("base", "a", "b", 10, None),
        ],
    );
    let before = rows(&store, relation("base", "a", "b", 10, None));
    apply(
        &store,
        (0..96)
            .flat_map(|n| {
                let id = format!("other-{n}");
                [
                    RuntimeMutation::Record { record: node(&id) },
                    relation(&id, "a", &id, 10, None),
                ]
            })
            .collect(),
    );
    assert_eq!(before, 1);
    assert_eq!(rows(&store, relation("base", "a", "b", 10, None)), before);
    assert!(!differential(
        &store,
        vec![relation("collision", "a", "b", 10, None)]
    ));
    assert!(differential(
        &store,
        vec![relation("free", "a", "a", 10, None)]
    ));
}

#[test]
fn constraint_inputs_reject_misplaced_and_disagreeing_adjacency() {
    for misplaced in [false, true] {
        let store = RrflowMxStore::new();
        apply(
            &store,
            vec![
                RuntimeMutation::Schema { registry: schema() },
                RuntimeMutation::Record { record: node("a") },
                RuntimeMutation::Record { record: node("b") },
                relation("base", "a", "b", 10, None),
            ],
        );
        let mut changed = edge("base", "a", "b");
        let scope = candidate(&store, vec![]).scope;
        let key =
            keyspaces::runtime_adjacency_key(keyspaces::RUNTIME_INCOMING_EDGES, &scope, &changed);
        if misplaced {
            changed.from = node("wrong").reference;
        } else {
            changed.valid_to = Some(50);
        }
        let mut transaction = store.begin_transaction().unwrap();
        transaction
            .put(key, serde_json::to_vec(&changed).unwrap())
            .unwrap();
        transaction
            .commit(crate::Durability::Authoritative)
            .unwrap();
        let transaction = store.begin_transaction().unwrap();
        assert!(validate_schema_and_objects(
            &*transaction,
            &candidate(&store, vec![relation("base", "a", "b", 10, None)])
        )
        .is_err());
    }
}
