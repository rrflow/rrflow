use super::*;

fn run<E: StorageEngine>(engine: &E, text: &str, keys: u64) -> rrd_query::QueryExecution {
    let catalog = fixture_catalog(engine);
    let physical =
        plan(&bind(&parse(text).unwrap(), &Parameters::new(), &catalog).unwrap()).unwrap();
    execute(
        engine,
        &physical,
        &ExecutionBudget {
            max_storage_keys: keys,
            ..ExecutionBudget::default()
        },
    )
    .unwrap()
}

fn compare<E: StorageEngine>(
    engine: &E,
    source: &str,
    id: &str,
    valid: u64,
    known: &str,
) -> rrd_query::QueryExecution {
    let base = format!("FROM {source} AT VALID {valid} KNOWN {known}");
    let all = run(engine, &format!("{base} PROJECT *"), 100_000);
    let selected = run(engine, &format!("{base} WHERE id = {id:?} PROJECT *"), 128);
    let reference = flattened_rows(&all)
        .into_iter()
        .filter(|row| row.values.get("id") == Some(&value(id)))
        .collect::<Vec<_>>();
    assert_eq!(flattened_rows(&selected), reference);
    selected
}

fn grow_and_change<E: StorageEngine>(engine: &E) -> (u64, u64) {
    let first = engine
        .runtime()
        .commit(&fixture_commit())
        .unwrap()
        .last_cursor;
    let before = compare(engine, "record:document", "a", 100, "HEAD");
    let mut mutations = Vec::new();
    for n in 0..96 {
        mutations.push(RuntimeMutation::Record {
            record: RuntimeRecord {
                reference: RuntimeRef::new("document", format!("unrelated-{n}")).unwrap(),
                valid_from: 10,
                valid_to: None,
                properties: properties(&[("status", "open"), ("title", "Unrelated")]),
            },
        });
        mutations.push(RuntimeMutation::Relation {
            relation: RuntimeRelation {
                reference: RuntimeRef::new("depends_on", format!("unrelated-edge-{n}")).unwrap(),
                from: RuntimeRef::new("document", "a").unwrap(),
                to: RuntimeRef::new("document", "b").unwrap(),
                valid_from: 20,
                valid_to: None,
                properties: properties(&[("strength", "hard")]),
            },
        });
    }
    let grown = engine
        .runtime()
        .commit(&RuntimeCommit {
            scope: ScopeId::new("instance:test").unwrap(),
            at: 101,
            actor: "identity-test".into(),
            expected_cursor: first,
            mutations,
        })
        .unwrap()
        .last_cursor;
    // This call exhausted the fixed budget with the previous family scan.
    let after = compare(engine, "record:document", "a", 100, "HEAD");
    assert_eq!(flattened_rows(&before), flattened_rows(&after));
    assert_eq!(after.selected_versions, 1);
    let records = |execution: &rrd_query::QueryExecution| {
        execution
            .read_evidence
            .paths
            .iter()
            .filter(|p| p.path == RuntimeReadAccessPath::RecordVersions)
            .map(|p| (p.keys_examined, p.values_decoded))
            .collect::<Vec<_>>()
    };
    assert_eq!(records(&after), records(&before));
    assert_eq!(
        compare(engine, "relation:depends_on", "a-b", 100, "HEAD").selected_versions,
        1
    );
    let updated = engine
        .runtime()
        .commit(&RuntimeCommit {
            scope: ScopeId::new("instance:test").unwrap(),
            at: 102,
            actor: "identity-test".into(),
            expected_cursor: grown,
            mutations: vec![
                RuntimeMutation::Record {
                    record: RuntimeRecord {
                        reference: RuntimeRef::new("document", "a").unwrap(),
                        valid_from: 30,
                        valid_to: None,
                        properties: properties(&[("status", "open"), ("title", "Updated")]),
                    },
                },
                RuntimeMutation::Relation {
                    relation: RuntimeRelation {
                        reference: RuntimeRef::new("depends_on", "a-b").unwrap(),
                        from: RuntimeRef::new("document", "b").unwrap(),
                        to: RuntimeRef::new("document", "a").unwrap(),
                        valid_from: 30,
                        valid_to: None,
                        properties: properties(&[("strength", "soft")]),
                    },
                },
            ],
        })
        .unwrap()
        .last_cursor;
    engine
        .runtime()
        .commit(&RuntimeCommit {
            scope: ScopeId::new("instance:test").unwrap(),
            at: 103,
            actor: "identity-test".into(),
            expected_cursor: updated,
            mutations: vec![
                RuntimeMutation::Retire {
                    retirement: RuntimeRetirement {
                        model: RuntimeLogicalModel::Relational,
                        reference: RuntimeRef::new("document", "a").unwrap(),
                        effective_at: 40,
                    },
                },
                RuntimeMutation::Retire {
                    retirement: RuntimeRetirement {
                        model: RuntimeLogicalModel::GraphRelation,
                        reference: RuntimeRef::new("depends_on", "a-b").unwrap(),
                        effective_at: 40,
                    },
                },
            ],
        })
        .unwrap();
    (first, updated)
}

fn history<E: StorageEngine>(engine: &E, first: u64, updated: u64) {
    for (source, id) in [("record:document", "a"), ("relation:depends_on", "a-b")] {
        assert_eq!(
            compare(engine, source, id, 100, &first.to_string()).returned_rows,
            1
        );
        assert_eq!(
            compare(engine, source, id, 35, &updated.to_string()).returned_rows,
            1
        );
        assert_eq!(compare(engine, source, id, 25, "HEAD").returned_rows, 1);
        assert_eq!(compare(engine, source, id, 35, "HEAD").returned_rows, 1);
        assert_eq!(compare(engine, source, id, 50, "HEAD").returned_rows, 0);
        assert_eq!(
            compare(engine, source, "absent", 50, "HEAD").returned_rows,
            0
        );
    }
    assert_eq!(run(engine, "FROM record:document AT VALID 25 KNOWN HEAD WHERE id = \"a\" AND status = \"closed\" PROJECT *", 128).returned_rows, 0);
    // Empty identity is a valid equality expression, not a bind-time reference.
    assert_eq!(
        run(
            engine,
            "FROM record:document AT VALID 25 KNOWN HEAD WHERE id = \"\" PROJECT *",
            100_000
        )
        .returned_rows,
        0
    );
    let query =
        parse("FROM record:document AT VALID 25 KNOWN HEAD WHERE id != \"a\" PROJECT id").unwrap();
    let physical =
        plan(&bind(&query, &Parameters::new(), &fixture_catalog(engine)).unwrap()).unwrap();
    assert!(matches!(
        physical.logical.version_sources.as_slice(),
        [RuntimeVersionedSource::Records { .. }]
    ));
    // BM25 scores depend on the full family corpus even with an ID predicate.
    let lexical = parse("FROM record:document AT VALID 25 KNOWN HEAD WHERE id = \"b\" AND title MATCH \"Beta\" PROJECT title").unwrap();
    let physical =
        plan(&bind(&lexical, &Parameters::new(), &fixture_catalog(engine)).unwrap()).unwrap();
    assert!(matches!(
        physical.logical.version_sources.as_slice(),
        [RuntimeVersionedSource::Records { .. }]
    ));
    let parameterized =
        parse("FROM record:document AT VALID 25 KNOWN HEAD WHERE id = $target PROJECT title")
            .unwrap();
    let physical = plan(
        &bind(
            &parameterized,
            &Parameters::from([("target".into(), value("a"))]),
            &fixture_catalog(engine),
        )
        .unwrap(),
    )
    .unwrap();
    assert!(matches!(
        physical.logical.version_sources.as_slice(),
        [RuntimeVersionedSource::Identity { .. }]
    ));
    assert_eq!(
        execute(
            engine,
            &physical,
            &ExecutionBudget {
                max_storage_keys: 128,
                ..ExecutionBudget::default()
            }
        )
        .unwrap()
        .returned_rows,
        1
    );
    assert!(matches!(
        execute(
            engine,
            &physical,
            &ExecutionBudget {
                max_storage_keys: 1,
                ..ExecutionBudget::default()
            }
        ),
        Err(Error::Budget(_))
    ));
}

#[test]
fn addressed_id_queries_stay_bounded_and_match_family_history_across_native_reopen() {
    let memory = RrflowMxStore::new();
    let (first, updated) = grow_and_change(&memory);
    history(&memory, first, updated);
    let root = tempfile::tempdir().unwrap();
    let native = RrflowKvStore::open(root.path()).unwrap();
    let (first, updated) = grow_and_change(&native);
    history(&native, first, updated);
    drop(native);
    let reopened = RrflowKvStore::open(root.path()).unwrap();
    history(&reopened, first, updated);
}
