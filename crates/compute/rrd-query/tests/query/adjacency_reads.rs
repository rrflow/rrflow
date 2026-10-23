use super::*;

fn bound<E: StorageEngine>(
    engine: &E,
    filter: &str,
    valid: u64,
    known: &str,
) -> rrd_query::PhysicalPlan {
    let text =
        format!("FROM relation:depends_on AT VALID {valid} KNOWN {known} WHERE {filter} PROJECT *");
    plan(
        &bind(
            &parse(&text).unwrap(),
            &Parameters::new(),
            &fixture_catalog(engine),
        )
        .unwrap(),
    )
    .unwrap()
}

fn compare<E: StorageEngine>(
    engine: &E,
    endpoint: &str,
    id: &str,
    valid: u64,
    known: &str,
) -> rrd_query::QueryExecution {
    let all = parse(&format!(
        "FROM relation:depends_on AT VALID {valid} KNOWN {known} PROJECT *"
    ))
    .unwrap();
    let physical =
        plan(&bind(&all, &Parameters::new(), &fixture_catalog(engine)).unwrap()).unwrap();
    let reference = execute(engine, &physical, &ExecutionBudget::default()).unwrap();
    let selected = execute(
        engine,
        &bound(engine, &format!("{endpoint}_id = {id:?}"), valid, known),
        &ExecutionBudget {
            max_storage_keys: 128,
            ..ExecutionBudget::default()
        },
    )
    .unwrap();
    assert_eq!(
        flattened_rows(&selected),
        flattened_rows(&reference)
            .into_iter()
            .filter(|r| r.values.get(&format!("{endpoint}_id")) == Some(&value(id)))
            .collect::<Vec<_>>()
    );
    selected
}

fn commit<E: StorageEngine>(engine: &E, at: u64, mutations: Vec<RuntimeMutation>) -> u64 {
    let scope = ScopeId::new("instance:test").unwrap();
    let head = engine.runtime().read_stamp(&scope).unwrap().commit_cursor;
    engine
        .runtime()
        .commit(&RuntimeCommit {
            scope,
            at,
            actor: "adjacency-filter-test".into(),
            expected_cursor: head,
            mutations,
        })
        .unwrap()
        .last_cursor
}

fn grow<E: StorageEngine>(engine: &E) -> u64 {
    let original = engine
        .runtime()
        .commit(&fixture_commit())
        .unwrap()
        .last_cursor;
    let before = compare(engine, "from", "a", 100, "HEAD");
    let mut mutations = Vec::new();
    for n in 0..96 {
        let reference = RuntimeRef::new("document", format!("far-{n}")).unwrap();
        mutations.push(RuntimeMutation::Record {
            record: RuntimeRecord {
                reference: reference.clone(),
                valid_from: 10,
                valid_to: None,
                properties: properties(&[("status", "open"), ("title", "Far")]),
            },
        });
        mutations.push(RuntimeMutation::Relation {
            relation: RuntimeRelation {
                reference: RuntimeRef::new("depends_on", format!("far-edge-{n}")).unwrap(),
                from: reference.clone(),
                to: reference,
                valid_from: 20,
                valid_to: None,
                properties: properties(&[("strength", "hard")]),
            },
        });
    }
    commit(engine, 101, mutations);
    let after = compare(engine, "from", "a", 100, "HEAD");
    assert_eq!(flattened_rows(&before), flattened_rows(&after));
    let keys = |r: &rrd_query::QueryExecution| {
        r.read_evidence
            .paths
            .iter()
            .filter(|p| {
                matches!(
                    p.path,
                    RuntimeReadAccessPath::AdjacencyVersions
                        | RuntimeReadAccessPath::RelationVersions
                )
            })
            .map(|p| (p.path, p.keys_examined, p.values_decoded))
            .collect::<Vec<_>>()
    };
    assert_eq!(keys(&before), keys(&after));
    assert!(after
        .read_evidence
        .paths
        .iter()
        .any(|p| p.path == RuntimeReadAccessPath::AdjacencyVersions && p.values_decoded == 1));
    assert_eq!(compare(engine, "to", "b", 100, "HEAD").returned_rows, 1);
    original
}

fn change<E: StorageEngine>(engine: &E) {
    for (at, valid_from, from, to) in [(102, 30, "b", "a"), (103, 15, "a", "b")] {
        commit(
            engine,
            at,
            vec![RuntimeMutation::Relation {
                relation: RuntimeRelation {
                    reference: RuntimeRef::new("depends_on", "a-b").unwrap(),
                    from: RuntimeRef::new("document", from).unwrap(),
                    to: RuntimeRef::new("document", to).unwrap(),
                    valid_from,
                    valid_to: None,
                    properties: properties(&[("strength", "soft")]),
                },
            }],
        );
        for valid in [20, 35, 60] {
            for endpoint in ["from", "to"] {
                for id in ["a", "b"] {
                    compare(engine, endpoint, id, valid, "HEAD");
                }
            }
        }
    }
    commit(
        engine,
        104,
        vec![RuntimeMutation::Retire {
            retirement: RuntimeRetirement {
                model: RuntimeLogicalModel::GraphRelation,
                reference: RuntimeRef::new("depends_on", "a-b").unwrap(),
                effective_at: 50,
            },
        }],
    );
}

fn history<E: StorageEngine>(engine: &E, original: u64) {
    assert_eq!(
        compare(engine, "from", "a", 100, &original.to_string()).returned_rows,
        1
    );
    for valid in [20, 35, 60] {
        for endpoint in ["from", "to"] {
            for id in ["a", "b", "absent"] {
                compare(engine, endpoint, id, valid, "HEAD");
            }
        }
    }
    for filter in [
        "from_id != \"a\"",
        "from_id = \"a\" AND strength MATCH \"hard\"",
    ] {
        assert!(matches!(
            bound(engine, filter, 35, "HEAD")
                .logical
                .version_sources
                .as_slice(),
            [RuntimeVersionedSource::Relations { .. }]
        ));
    }
    assert!(matches!(
        bound(engine, "id = \"a-b\" AND from_id = \"a\"", 35, "HEAD")
            .logical
            .version_sources
            .as_slice(),
        [RuntimeVersionedSource::Identity { .. }]
    ));
}

#[test]
fn endpoint_filter_reads_are_bounded_temporally_complete_and_reopen_exactly() {
    let memory = RrflowMxStore::new();
    let original = grow(&memory);
    change(&memory);
    history(&memory, original);
    let root = tempfile::tempdir().unwrap();
    let native = RrflowKvStore::open(root.path()).unwrap();
    let original = grow(&native);
    change(&native);
    history(&native, original);
    drop(native);
    history(&RrflowKvStore::open(root.path()).unwrap(), original);
}

#[test]
fn endpoint_filter_requires_a_unique_kind_and_preserves_colliding_node_ids() {
    let engine = RrflowMxStore::new();
    engine.runtime().commit(&fixture_commit()).unwrap();
    let RuntimeMutation::Schema { mut registry } = schema() else {
        unreachable!()
    };
    registry.revision = 2;
    registry
        .relations
        .get_mut(&RuntimeType::new("depends_on").unwrap())
        .unwrap()
        .from
        .insert(RuntimeType::new("metric").unwrap());
    commit(
        &engine,
        101,
        vec![
            RuntimeMutation::Schema { registry },
            RuntimeMutation::Record {
                record: RuntimeRecord {
                    reference: RuntimeRef::new("metric", "a").unwrap(),
                    valid_from: 10,
                    valid_to: None,
                    properties: RuntimeProperties::new(),
                },
            },
            RuntimeMutation::Relation {
                relation: RuntimeRelation {
                    reference: RuntimeRef::new("depends_on", "metric-a-b").unwrap(),
                    from: RuntimeRef::new("metric", "a").unwrap(),
                    to: RuntimeRef::new("document", "b").unwrap(),
                    valid_from: 20,
                    valid_to: None,
                    properties: properties(&[("strength", "hard")]),
                },
            },
        ],
    );
    let ambiguous = bound(&engine, "from_id = \"a\"", 100, "HEAD");
    assert!(matches!(
        ambiguous.logical.version_sources.as_slice(),
        [RuntimeVersionedSource::Relations { .. }]
    ));
    assert_eq!(
        execute(&engine, &ambiguous, &ExecutionBudget::default())
            .unwrap()
            .returned_rows,
        2
    );
    for kind in ["metric", "document"] {
        let explicit = bound(
            &engine,
            &format!("from_id = \"a\" AND from_kind = {kind:?}"),
            100,
            "HEAD",
        );
        assert!(matches!(
            explicit.logical.version_sources.as_slice(),
            [RuntimeVersionedSource::Adjacency { .. }]
        ));
        let rows = flattened_rows(
            &execute(
                &engine,
                &explicit,
                &ExecutionBudget {
                    max_storage_keys: 128,
                    ..ExecutionBudget::default()
                },
            )
            .unwrap(),
        );
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].values.get("from_kind"), Some(&value(kind)));
    }
}
