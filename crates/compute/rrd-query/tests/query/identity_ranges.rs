use super::*;

fn run<E: StorageEngine>(engine: &E, filter: &str, keys: u64) -> rrd_query::QueryExecution {
    let text = format!("FROM record:document AT VALID 100 KNOWN HEAD WHERE {filter} PROJECT *");
    let physical = plan(
        &bind(
            &parse(&text).unwrap(),
            &Parameters::new(),
            &fixture_catalog(engine),
        )
        .unwrap(),
    )
    .unwrap();
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

#[test]
fn identity_range_read_cost_excludes_unrelated_records() {
    let engine = RrflowMxStore::new();
    let first = engine.runtime().commit(&fixture_commit()).unwrap();
    let before = run(&engine, "id >= \"a\" AND id < \"c\"", 128);
    engine
        .runtime()
        .commit(&RuntimeCommit {
            scope: ScopeId::new("instance:test").unwrap(),
            at: 101,
            actor: "range-test".into(),
            expected_cursor: first.last_cursor,
            mutations: (0..96)
                .map(|n| RuntimeMutation::Record {
                    record: RuntimeRecord {
                        reference: RuntimeRef::new("document", format!("unrelated-{n}")).unwrap(),
                        valid_from: 10,
                        valid_to: None,
                        properties: properties(&[("status", "open"), ("title", "Unrelated")]),
                    },
                })
                .collect(),
        })
        .unwrap();
    let after = run(&engine, "id >= \"a\" AND id < \"c\"", 128);
    assert_eq!(flattened_rows(&before), flattened_rows(&after));
    let counts = |execution: &rrd_query::QueryExecution| {
        execution
            .read_evidence
            .paths
            .iter()
            .filter(|p| p.path == RuntimeReadAccessPath::RecordVersions)
            .map(|p| (p.keys_examined, p.values_decoded))
            .collect::<Vec<_>>()
    };
    assert_eq!(counts(&before), counts(&after));
}

fn history<E: StorageEngine>(engine: &E, known: u64) {
    for source in ["record:document", "relation:depends_on"] {
        for valid in [20, 40, 70] {
            for known in [known.to_string(), "HEAD".into()] {
                let base = format!("FROM {source} AT VALID {valid} KNOWN {known}");
                let catalog = fixture_catalog(engine);
                let reference_plan = plan(
                    &bind(
                        &parse(&format!("{base} PROJECT *")).unwrap(),
                        &Parameters::new(),
                        &catalog,
                    )
                    .unwrap(),
                )
                .unwrap();
                let all = execute(engine, &reference_plan, &ExecutionBudget::default()).unwrap();
                for (lower, upper, include_lower, include_upper) in [
                    ("a", "b", true, false),
                    ("a", "b", false, true),
                    ("a", "b", true, true),
                    ("a", "b", false, false),
                    ("a", "a", true, true),
                    ("a", "a", false, false),
                    ("abcdefgh", "abcdefghx", true, true),
                    ("é", "雪", true, true),
                ] {
                    let lo = if include_lower { ">=" } else { ">" };
                    let hi = if include_upper { "<=" } else { "<" };
                    let query = parse(&format!(
                        "{base} WHERE id {lo} {lower:?} AND id {hi} {upper:?} PROJECT *"
                    ))
                    .unwrap();
                    let physical =
                        plan(&bind(&query, &Parameters::new(), &catalog).unwrap()).unwrap();
                    assert!(matches!(
                        physical.logical.version_sources.as_slice(),
                        [RuntimeVersionedSource::IdentityRange { .. }]
                    ));
                    let selected = execute(engine, &physical, &ExecutionBudget::default()).unwrap();
                    let expected = flattened_rows(&all)
                        .into_iter()
                        .filter(|row| {
                            let RuntimeValue::String(id) = row.values.get("id").unwrap() else {
                                panic!("nonstring ID")
                            };
                            (id.as_str() > lower || (include_lower && id == lower))
                                && (id.as_str() < upper || (include_upper && id == upper))
                        })
                        .collect::<Vec<_>>();
                    assert_eq!(flattened_rows(&selected), expected);
                }
            }
        }
    }
}

fn populate<E: StorageEngine>(engine: &E) -> u64 {
    let scope = ScopeId::new("instance:test").unwrap();
    let first = engine
        .runtime()
        .commit(&fixture_commit())
        .unwrap()
        .last_cursor;
    let mut mutations = Vec::new();
    for id in [
        "aa",
        "abcdefgh",
        "abcdefghx",
        "q\"uote",
        "slash\\id",
        "é",
        "雪",
    ] {
        mutations.push(RuntimeMutation::Record {
            record: RuntimeRecord {
                reference: RuntimeRef::new("document", id).unwrap(),
                valid_from: 10,
                valid_to: None,
                properties: properties(&[("status", "open"), ("title", "Range")]),
            },
        });
        mutations.push(RuntimeMutation::Relation {
            relation: RuntimeRelation {
                reference: RuntimeRef::new("depends_on", id).unwrap(),
                from: RuntimeRef::new("document", "a").unwrap(),
                to: RuntimeRef::new("document", id).unwrap(),
                valid_from: 10,
                valid_to: None,
                properties: properties(&[("strength", "hard")]),
            },
        });
    }
    let original = engine
        .runtime()
        .commit(&RuntimeCommit {
            scope: scope.clone(),
            at: 101,
            actor: "range-test".into(),
            expected_cursor: first,
            mutations,
        })
        .unwrap()
        .last_cursor;
    let updated = engine
        .runtime()
        .commit(&RuntimeCommit {
            scope: scope.clone(),
            at: 102,
            actor: "range-test".into(),
            expected_cursor: original,
            mutations: vec![RuntimeMutation::Record {
                record: RuntimeRecord {
                    reference: RuntimeRef::new("document", "a").unwrap(),
                    valid_from: 30,
                    valid_to: None,
                    properties: properties(&[("status", "closed"), ("title", "Updated")]),
                },
            }],
        })
        .unwrap()
        .last_cursor;
    engine
        .runtime()
        .commit(&RuntimeCommit {
            scope,
            at: 103,
            actor: "range-test".into(),
            expected_cursor: updated,
            mutations: vec![
                RuntimeMutation::Retire {
                    retirement: RuntimeRetirement {
                        model: RuntimeLogicalModel::Relational,
                        reference: RuntimeRef::new("document", "a").unwrap(),
                        effective_at: 50,
                    },
                },
                RuntimeMutation::Retire {
                    retirement: RuntimeRetirement {
                        model: RuntimeLogicalModel::GraphRelation,
                        reference: RuntimeRef::new("depends_on", "aa").unwrap(),
                        effective_at: 50,
                    },
                },
            ],
        })
        .unwrap();
    original
}

#[test]
fn identity_ranges_preserve_complete_temporal_histories_and_native_reopen() {
    let memory = RrflowMxStore::new();
    let original = populate(&memory);
    history(&memory, original);
    let root = tempfile::tempdir().unwrap();
    let native = RrflowKvStore::open(root.path()).unwrap();
    let original = populate(&native);
    history(&native, original);
    drop(native);
    history(&RrflowKvStore::open(root.path()).unwrap(), original);
}

#[test]
fn identity_ranges_resolve_parameters_and_preserve_fallback_predicates() {
    let engine = RrflowMxStore::new();
    populate(&engine);
    let catalog = fixture_catalog(&engine);
    let parameters = BTreeMap::from([("lo".into(), value("a")), ("hi".into(), value("b"))]);
    let query = parse("FROM record:document AT VALID 40 KNOWN HEAD WHERE id > $lo AND id <= $hi AND id >= $lo PROJECT *").unwrap();
    let physical = plan(&bind(&query, &parameters, &catalog).unwrap()).unwrap();
    assert!(matches!(
        &physical.logical.version_sources[0],
        RuntimeVersionedSource::IdentityRange {
            lower: std::ops::Bound::Excluded(_),
            upper: std::ops::Bound::Included(_),
            ..
        }
    ));
    assert!(execute(
        &engine,
        &physical,
        &ExecutionBudget {
            max_storage_keys: 1,
            ..ExecutionBudget::default()
        }
    )
    .is_err());
    for filter in [
        "id >= \"a\" AND title MATCH \"Range\"",
        "id != \"a\"",
        "id < \"\"",
        "id > \"z\" AND id < \"a\"",
    ] {
        let query = parse(&format!(
            "FROM record:document AT VALID 40 KNOWN HEAD WHERE {filter} PROJECT *"
        ))
        .unwrap();
        let physical = plan(&bind(&query, &Parameters::new(), &catalog).unwrap()).unwrap();
        assert!(matches!(
            physical.logical.version_sources.as_slice(),
            [RuntimeVersionedSource::Records { .. }]
        ));
    }
    assert_eq!(
        run(&engine, "id > \"z\" AND id < \"a\"", 100_000).returned_rows,
        0
    );
    for filter in ["id > \"é\"", "id <= \"a\""] {
        let query = parse(&format!(
            "FROM record:document AT VALID 40 KNOWN HEAD WHERE {filter} PROJECT *"
        ))
        .unwrap();
        let physical = plan(&bind(&query, &Parameters::new(), &catalog).unwrap()).unwrap();
        assert!(matches!(
            physical.logical.version_sources.as_slice(),
            [RuntimeVersionedSource::IdentityRange { .. }]
        ));
        assert!(
            !execute(&engine, &physical, &ExecutionBudget::default())
                .unwrap()
                .truncated
        );
    }
}
