use super::*;

fn grow<E: StorageEngine>(engine: &E) {
    let first = engine
        .runtime()
        .commit(&fixture_commit())
        .unwrap()
        .last_cursor;
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
                reference: RuntimeRef::new("depends_on", format!("far-{n}")).unwrap(),
                from: reference.clone(),
                to: reference,
                valid_from: 10,
                valid_to: None,
                properties: properties(&[("strength", "hard")]),
            },
        });
    }
    engine
        .runtime()
        .commit(&RuntimeCommit {
            scope: ScopeId::new("instance:test").unwrap(),
            at: 101,
            actor: "pipeline-admission".into(),
            expected_cursor: first,
            mutations,
        })
        .unwrap();
}

fn pipeline<E: StorageEngine>(
    engine: &E,
    text: &str,
    keys: u64,
) -> Result<rrd_query::StampedQueryExecution, Error> {
    let read = engine
        .runtime()
        .read_stamp(&ScopeId::new("instance:test").unwrap())
        .unwrap();
    StampedQueryPipeline::new(engine, read).unwrap().run(
        &parse(text).unwrap(),
        &Parameters::new(),
        &ExecutionBudget {
            max_storage_keys: keys,
            ..ExecutionBudget::default()
        },
    )
}

fn check<E: StorageEngine>(engine: &E) {
    for (source, filter, rows) in [
        ("record:document", "id = \"a\"", 1),
        ("record:document", "id >= \"a\" AND id < \"c\"", 2),
        ("relation:depends_on", "from_id = \"a\"", 1),
    ] {
        let text = format!("FROM {source} AT VALID 100 KNOWN HEAD WHERE {filter} PROJECT *");
        let result = pipeline(engine, &text, 128).unwrap();
        assert_eq!(result.execution.returned_rows, rows);
        let evidence = &result.execution.read_evidence;
        evidence.validate().unwrap();
        assert_eq!(evidence.key_budget, 128);
        assert!(evidence
            .paths
            .iter()
            .any(|path| path.path == RuntimeReadAccessPath::SchemaVersions
                && path.values_decoded > 0));
        let actual = evidence.keys_examined;
        assert!(pipeline(engine, &text, actual).is_ok());
        assert!(matches!(
            pipeline(engine, &text, actual - 1),
            Err(Error::Budget(_))
        ));
    }
}

#[test]
fn stamped_addressed_queries_bound_capture_and_execution_together() {
    let memory = RrflowMxStore::new();
    grow(&memory);
    check(&memory);
    let root = tempfile::tempdir().unwrap();
    let native = RrflowKvStore::open(root.path()).unwrap();
    grow(&native);
    check(&native);
    drop(native);
    check(&RrflowKvStore::open(root.path()).unwrap());
}

#[test]
fn stamped_query_shared_budget_also_covers_family_capture_and_execution() {
    let engine = RrflowMxStore::new();
    engine.runtime().commit(&fixture_commit()).unwrap();
    for filter in ["status = \"open\"", "title MATCH \"Alpha\""] {
        let text = format!("FROM record:document AT VALID 100 KNOWN HEAD WHERE {filter} PROJECT *");
        let full = pipeline(&engine, &text, 1000).unwrap();
        let used = full.execution.read_evidence.keys_examined;
        assert!(pipeline(&engine, &text, used).is_ok());
        assert!(matches!(
            pipeline(&engine, &text, used - 1),
            Err(Error::Budget(_))
        ));
    }
}

#[test]
fn stamped_query_addressed_history_remains_exact_after_head_and_schema_changes() {
    let engine = RrflowMxStore::new();
    grow(&engine);
    let scope = ScopeId::new("instance:test").unwrap();
    let read = engine.runtime().read_stamp(&scope).unwrap();
    let query =
        parse("FROM record:document AT VALID 100 KNOWN HEAD WHERE id = $id PROJECT *").unwrap();
    let parameters = BTreeMap::from([("id".into(), value("a"))]);
    let pipeline = StampedQueryPipeline::new(&engine, read.clone()).unwrap();
    let budget = ExecutionBudget {
        max_storage_keys: 128,
        ..ExecutionBudget::default()
    };
    let original = pipeline.run(&query, &parameters, &budget).unwrap();
    let RuntimeMutation::Schema { mut registry } = schema() else {
        unreachable!()
    };
    registry.revision = 2;
    let update = engine
        .runtime()
        .commit(&RuntimeCommit {
            scope: scope.clone(),
            at: 102,
            actor: "pipeline-history".into(),
            expected_cursor: read.commit_cursor,
            mutations: vec![
                RuntimeMutation::Schema { registry },
                RuntimeMutation::Record {
                    record: RuntimeRecord {
                        reference: RuntimeRef::new("document", "a").unwrap(),
                        valid_from: 30,
                        valid_to: None,
                        properties: properties(&[("status", "closed"), ("title", "Changed")]),
                    },
                },
            ],
        })
        .unwrap();
    let replay = pipeline.run(&query, &parameters, &budget).unwrap();
    assert_eq!(replay.plan, original.plan);
    assert_eq!(replay.execution.batches, original.execution.batches);
    assert_eq!(
        replay.execution.read_evidence.stamp_validation.method,
        "rfc9162_direct_versions"
    );
    engine
        .runtime()
        .commit(&RuntimeCommit {
            scope: scope.clone(),
            at: 103,
            actor: "pipeline-history".into(),
            expected_cursor: update.last_cursor,
            mutations: vec![RuntimeMutation::Retire {
                retirement: RuntimeRetirement {
                    model: RuntimeLogicalModel::Relational,
                    reference: RuntimeRef::new("document", "a").unwrap(),
                    effective_at: 50,
                },
            }],
        })
        .unwrap();
    let current =
        StampedQueryPipeline::new(&engine, engine.runtime().read_stamp(&scope).unwrap()).unwrap();
    assert_eq!(
        current
            .run(&query, &parameters, &budget)
            .unwrap()
            .execution
            .returned_rows,
        0
    );
    let historic = parse(&format!(
        "FROM record:document AT VALID 100 KNOWN {} WHERE id = $id PROJECT *",
        read.commit_cursor
    ))
    .unwrap();
    assert_eq!(
        current
            .run(&historic, &parameters, &budget)
            .unwrap()
            .execution
            .batches,
        original.execution.batches
    );
}
