use rrd_core::{
    Claim, GeoPoint, GeoValue, Predicate, Producer, RuntimeCommit, RuntimeEvent,
    RuntimeEventSchema, RuntimeGeo, RuntimeGraphSnapshot, RuntimeLogicalModel, RuntimeMutation,
    RuntimeProperties, RuntimePropertySchema, RuntimeRecord, RuntimeRecordSchema, RuntimeRef,
    RuntimeRelation, RuntimeRelationSchema, RuntimeRetirement, RuntimeSchemaRegistry,
    RuntimeSeriesSample, RuntimeTableSchema, RuntimeType, RuntimeValue, RuntimeValueType, ScopeId,
    SeriesValue, Subject,
};
use rrd_query::{
    bind, execute, plan, Catalog, Error, ExecutionBudget, Parameters, PhysicalOperator,
    StampedQueryPipeline,
};
use rrd_query::{
    parse, ComparisonOperator, CursorExpr, Projection, Query, Source, TemporalSelector, TimeExpr,
};
use rrd_store::{
    RrflowKvStore, RrflowMxStore, RuntimeReadAccessPath, RuntimeReadBudget, RuntimeVersionedSource,
    StorageEngine,
};
use std::collections::{BTreeMap, BTreeSet};

fn value(value: &str) -> RuntimeValue {
    RuntimeValue::String(value.into())
}

fn properties(values: &[(&str, &str)]) -> RuntimeProperties {
    values
        .iter()
        .map(|(key, value)| ((*key).into(), RuntimeValue::String((*value).into())))
        .collect()
}

fn schema() -> RuntimeMutation {
    let mut registry = RuntimeSchemaRegistry::empty(1, "query fixture");
    let document_kind = RuntimeType::new("document").unwrap();
    registry
        .define_record_table(
            document_kind.clone(),
            RuntimeLogicalModel::Relational,
            RuntimeRecordSchema {
                properties: BTreeMap::from([
                    (
                        "status".into(),
                        RuntimePropertySchema::required(RuntimeValueType::String),
                    ),
                    (
                        "title".into(),
                        RuntimePropertySchema::required(RuntimeValueType::String),
                    ),
                ]),
                ..RuntimeRecordSchema::default()
            },
        )
        .unwrap();
    registry
        .define_record_table(
            RuntimeType::new("metric").unwrap(),
            RuntimeLogicalModel::Relational,
            RuntimeRecordSchema::default(),
        )
        .unwrap();
    registry
        .define_relation_table(
            RuntimeType::new("depends_on").unwrap(),
            RuntimeRelationSchema {
                from: BTreeSet::from([document_kind.clone()]),
                to: BTreeSet::from([document_kind.clone()]),
                properties: BTreeMap::from([(
                    "strength".into(),
                    RuntimePropertySchema::required(RuntimeValueType::String),
                )]),
                ..RuntimeRelationSchema::default()
            },
        )
        .unwrap();
    registry
        .define_event_table(
            RuntimeType::new("tool_result").unwrap(),
            RuntimeLogicalModel::Event,
            RuntimeEventSchema {
                subject_required: true,
                subject_types: BTreeSet::from([document_kind]),
                properties: BTreeMap::from([(
                    "ok".into(),
                    RuntimePropertySchema::required(RuntimeValueType::Bool),
                )]),
                ..RuntimeEventSchema::default()
            },
        )
        .unwrap();
    for (kind, model) in [
        ("sample", RuntimeLogicalModel::TimeSeries),
        ("location", RuntimeLogicalModel::Geo),
    ] {
        registry.tables.insert(
            RuntimeType::new(kind).unwrap(),
            RuntimeTableSchema::schemaless(model),
        );
    }
    RuntimeMutation::Schema { registry }
}

fn fixture_commit() -> RuntimeCommit {
    let first = RuntimeRef::new("document", "a").unwrap();
    let second = RuntimeRef::new("document", "b").unwrap();
    RuntimeCommit {
        scope: ScopeId::new("instance:test").unwrap(),
        at: 100,
        actor: "agent:test".into(),
        expected_cursor: 0,
        mutations: vec![
            schema(),
            RuntimeMutation::Record {
                record: RuntimeRecord {
                    reference: first.clone(),
                    valid_from: 10,
                    valid_to: None,
                    properties: properties(&[("status", "open"), ("title", "Alpha")]),
                },
            },
            RuntimeMutation::Record {
                record: RuntimeRecord {
                    reference: second.clone(),
                    valid_from: 20,
                    valid_to: None,
                    properties: properties(&[("status", "closed"), ("title", "Beta")]),
                },
            },
            RuntimeMutation::Relation {
                relation: RuntimeRelation {
                    reference: RuntimeRef::new("depends_on", "a-b").unwrap(),
                    from: first.clone(),
                    to: second,
                    valid_from: 20,
                    valid_to: None,
                    properties: properties(&[("strength", "hard")]),
                },
            },
            RuntimeMutation::Event {
                event: RuntimeEvent {
                    kind: RuntimeType::new("tool_result").unwrap(),
                    subject: Some(first),
                    properties: BTreeMap::from([("ok".into(), RuntimeValue::Bool(true))]),
                },
            },
            RuntimeMutation::Claim {
                claim: Claim::new(
                    Subject::new("document:a").unwrap(),
                    Predicate::new("status").unwrap(),
                    "ready",
                    30,
                    100,
                    Producer {
                        actor: "agent:test".into(),
                        on_behalf_of: None,
                        session: Some("fixture".into()),
                    },
                ),
            },
            RuntimeMutation::Record {
                record: RuntimeRecord {
                    reference: RuntimeRef::new("metric", "latency").unwrap(),
                    valid_from: 10,
                    valid_to: None,
                    properties: RuntimeProperties::new(),
                },
            },
            RuntimeMutation::SeriesSample {
                sample: RuntimeSeriesSample {
                    reference: RuntimeRef::new("sample", "latency-100").unwrap(),
                    series: RuntimeRef::new("metric", "latency").unwrap(),
                    observed_at: 100,
                    value: SeriesValue::Decimal("12.5".into()),
                    properties: RuntimeProperties::new(),
                },
            },
            RuntimeMutation::Geo {
                geo: RuntimeGeo {
                    reference: RuntimeRef::new("location", "alpha").unwrap(),
                    subject: RuntimeRef::new("document", "a").unwrap(),
                    field: "position".into(),
                    valid_from: 10,
                    valid_to: None,
                    value: GeoValue::Point {
                        point: GeoPoint {
                            longitude: -122.4,
                            latitude: 37.8,
                        },
                    },
                    properties: RuntimeProperties::new(),
                },
            },
            RuntimeMutation::Relation {
                relation: RuntimeRelation {
                    reference: RuntimeRef::new("depends_on", "b-a-cycle").unwrap(),
                    from: RuntimeRef::new("document", "b").unwrap(),
                    to: RuntimeRef::new("document", "a").unwrap(),
                    valid_from: 20,
                    valid_to: None,
                    properties: properties(&[("strength", "cycle")]),
                },
            },
        ],
    }
}

fn read_budget() -> RuntimeReadBudget {
    RuntimeReadBudget::new(ExecutionBudget::default().max_storage_keys).unwrap()
}

fn fixture_sources() -> Vec<Source> {
    vec![
        Source::Record {
            kind: RuntimeType::new("document").unwrap(),
        },
        Source::Record {
            kind: RuntimeType::new("metric").unwrap(),
        },
        Source::Relation {
            kind: RuntimeType::new("depends_on").unwrap(),
        },
        Source::Traversal {
            relation: RuntimeType::new("depends_on").unwrap(),
            start: RuntimeRef::new("document", "a").unwrap(),
            direction: rrd_query::TraversalDirection::Outgoing,
            max_depth: 3,
        },
        Source::Event {
            kind: RuntimeType::new("tool_result").unwrap(),
        },
        Source::Series {
            kind: RuntimeType::new("metric").unwrap(),
        },
        Source::Geo {
            kind: RuntimeType::new("location").unwrap(),
        },
        Source::Claim {
            predicate: Some(Predicate::new("status").unwrap()),
        },
    ]
}

fn fixture_catalog<E: StorageEngine>(engine: &E) -> Catalog {
    Catalog::capture_for_sources(
        engine,
        &ScopeId::new("instance:test").unwrap(),
        &fixture_sources(),
        read_budget(),
    )
    .unwrap()
}

fn execute_fixture<E: StorageEngine>(engine: &E, text: &str) -> rrd_query::QueryExecution {
    engine.runtime().commit(&fixture_commit()).unwrap();
    let catalog = fixture_catalog(engine);
    let query = parse(text).unwrap();
    execute(
        engine,
        &plan(&bind(&query, &Parameters::new(), &catalog).unwrap()).unwrap(),
        &ExecutionBudget::default(),
    )
    .unwrap()
}

fn execute_text<E: StorageEngine>(
    engine: &E,
    text: &str,
) -> (rrd_query::PhysicalPlan, rrd_query::QueryExecution) {
    let catalog = fixture_catalog(engine);
    let query = parse(text).unwrap();
    let physical = plan(&bind(&query, &Parameters::new(), &catalog).unwrap()).unwrap();
    let execution = execute(engine, &physical, &ExecutionBudget::default()).unwrap();
    (physical, execution)
}

fn flattened_rows(execution: &rrd_query::QueryExecution) -> Vec<rrd_query::QueryRow> {
    execution
        .batches
        .iter()
        .flat_map(|batch| batch.rows.iter().cloned())
        .collect()
}

fn seed_historical_corrections<E: StorageEngine>(engine: &E) {
    let first = fixture_commit();
    let head = engine.runtime().commit(&first).unwrap().last_cursor;
    engine
        .runtime()
        .commit(&RuntimeCommit {
            scope: ScopeId::new("instance:test").unwrap(),
            at: 101,
            actor: "agent:correction".into(),
            expected_cursor: head,
            mutations: vec![
                RuntimeMutation::Record {
                    record: RuntimeRecord {
                        reference: RuntimeRef::new("document", "a").unwrap(),
                        valid_from: 10,
                        valid_to: None,
                        properties: properties(&[("status", "open"), ("title", "Alpha corrected")]),
                    },
                },
                RuntimeMutation::Claim {
                    claim: Claim::new(
                        Subject::new("document:a").unwrap(),
                        Predicate::new("status").unwrap(),
                        "corrected",
                        30,
                        101,
                        Producer {
                            actor: "agent:correction".into(),
                            on_behalf_of: None,
                            session: Some("correction".into()),
                        },
                    ),
                },
            ],
        })
        .unwrap();
}

fn historical_outcome<E: StorageEngine>(engine: &E) -> (String, String, String, String, u64, u64) {
    let (historical_plan, historical_record) = execute_text(
        engine,
        "FROM record:document AT VALID 100 KNOWN 10 WHERE id = \"a\" PROJECT title EXPLAIN CONTRACT",
    );
    let (_, current_record) = execute_text(
        engine,
        "FROM record:document AT VALID 100 KNOWN HEAD WHERE id = \"a\" PROJECT title EXPLAIN CONTRACT",
    );
    let (_, historical_claim) = execute_text(
        engine,
        "FROM claim:status AT VALID 100 KNOWN 10 PROJECT object EXPLAIN CONTRACT",
    );
    let (_, current_claim) = execute_text(
        engine,
        "FROM claim:status AT VALID 100 KNOWN HEAD PROJECT object EXPLAIN CONTRACT",
    );
    let string_field = |execution: &rrd_query::QueryExecution, field: &str| {
        let RuntimeValue::String(value) = &execution.batches[0].rows[0].values[field] else {
            panic!("{field} was not a string")
        };
        value.clone()
    };
    (
        string_field(&historical_record, "title"),
        string_field(&current_record, "title"),
        string_field(&historical_claim, "object"),
        string_field(&current_claim, "object"),
        historical_record.known_at_cursor,
        historical_plan.explanation.contract.source_cursor,
    )
}

#[test]
fn rrflow_mx_and_rrflow_kv_return_identical_exact_rows() {
    let memory = RrflowMxStore::new();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("rrflow-kv");
    let text = "FROM record:document AT VALID 100 KNOWN HEAD WHERE status = \"open\" PROJECT id, title EXPLAIN CONTRACT";
    let left = execute_fixture(&memory, text);
    let right = {
        let rrflow_kv = RrflowKvStore::open(&path).unwrap();
        let execution = execute_fixture(&rrflow_kv, text);
        rrflow_kv.flush(200).unwrap();
        execution
    };
    assert_eq!(left, right);
    assert_eq!(left.returned_rows, 1);
    assert_eq!(left.batches[0].rows[0].values["id"], value("a"));
    assert_eq!(left.batches[0].rows[0].values["title"], value("Alpha"));

    let reopened = RrflowKvStore::open(&path).unwrap();
    let before = reopened.physical_store_evidence().unwrap();
    let (_, reopened_execution) = execute_text(&reopened, text);
    let after = reopened.physical_store_evidence().unwrap();
    assert_eq!(reopened_execution, left);
    let loaded_pages = after.page_loads.unwrap() - before.page_loads.unwrap();
    let cached_pages = after.cache_hits.unwrap() - before.cache_hits.unwrap();
    assert!(
        loaded_pages + cached_pages > 0,
        "the query must consume an immutable page through either storage I/O or the bounded cache"
    );
    if loaded_pages > 0 {
        assert!(after.page_bytes_read.unwrap() > before.page_bytes_read.unwrap());
    }
    assert!(after.filter_checks.unwrap() > before.filter_checks.unwrap());
    assert_eq!(
        reopened_execution
            .read_evidence
            .paths
            .iter()
            .find(|path| path.path == RuntimeReadAccessPath::RecordVersions)
            .unwrap()
            .range_scans,
        1
    );

    let read = memory
        .runtime()
        .read_stamp(&ScopeId::new("instance:test").unwrap())
        .unwrap();
    let page = memory
        .runtime()
        .read_changes(&read, 0, read.commit_cursor as usize)
        .unwrap();
    let direct =
        RuntimeGraphSnapshot::from_changes(&page.changes, read.scope, 100, read.commit_cursor);
    let direct_ids = direct
        .records
        .iter()
        .filter(|record| {
            record.reference.kind.as_str() == "document"
                && record.properties.get("status") == Some(&value("open"))
        })
        .map(|record| record.reference.id.to_string())
        .collect::<Vec<_>>();
    let query_ids = left.batches[0]
        .rows
        .iter()
        .map(|row| match &row.values["id"] {
            RuntimeValue::String(id) => id.clone(),
            value => panic!("unexpected query id {value:?}"),
        })
        .collect::<Vec<_>>();
    assert_eq!(
        query_ids, direct_ids,
        "query must match the direct graph API"
    );
}

#[test]
fn stamped_pipeline_matches_manual_execution_and_retains_its_read_coordinate() {
    let engine = RrflowMxStore::new();
    engine.runtime().commit(&fixture_commit()).unwrap();
    let scope = ScopeId::new("instance:test").unwrap();
    let read = engine.runtime().read_stamp(&scope).unwrap();
    let query = parse(
        "FROM record:document AT VALID 100 KNOWN HEAD WHERE status = \"open\" PROJECT id, title EXPLAIN CONTRACT",
    )
    .unwrap();
    let parameters = Parameters::new();
    let budget = ExecutionBudget::default();

    let catalog =
        Catalog::capture_for_query_at(&engine, read.clone(), &query, read_budget()).unwrap();
    let manual_bound = bind(&query, &parameters, &catalog).unwrap();
    let manual_plan = plan(&manual_bound).unwrap();
    let manual_execution = execute(&engine, &manual_plan, &budget).unwrap();

    let pipeline = StampedQueryPipeline::new(&engine, read.clone()).unwrap();
    let stamped = pipeline.run(&query, &parameters, &budget).unwrap();
    assert_eq!(pipeline.read(), &read);
    assert_eq!(stamped.bound, manual_bound);
    assert_eq!(stamped.plan, manual_plan);
    assert_eq!(stamped.execution, manual_execution);
    assert_eq!(stamped.bound.read, read);
    assert_eq!(stamped.plan.logical.read, read);
    assert_eq!(stamped.execution.read_manifest, read.manifest_id);
    assert!(matches!(
        stamped.plan.operators.as_slice(),
        [
            PhysicalOperator::VersionedRead { .. },
            PhysicalOperator::DataFusionEvaluate
        ]
    ));

    engine
        .runtime()
        .commit(&RuntimeCommit {
            scope: scope.clone(),
            at: 101,
            actor: "agent:correction".into(),
            expected_cursor: read.commit_cursor,
            mutations: vec![RuntimeMutation::Record {
                record: RuntimeRecord {
                    reference: RuntimeRef::new("document", "a").unwrap(),
                    valid_from: 10,
                    valid_to: None,
                    properties: properties(&[("status", "open"), ("title", "Alpha corrected")]),
                },
            }],
        })
        .unwrap();

    let replay = pipeline.run(&query, &parameters, &budget).unwrap();
    assert_eq!(replay.bound, stamped.bound);
    assert_eq!(replay.plan, stamped.plan);
    assert_eq!(replay.execution.plan_digest, stamped.execution.plan_digest);
    assert_eq!(
        replay.execution.read_manifest,
        stamped.execution.read_manifest
    );
    assert_eq!(replay.execution.valid_at, stamped.execution.valid_at);
    assert_eq!(
        replay.execution.known_at_cursor,
        stamped.execution.known_at_cursor
    );
    assert_eq!(replay.execution.batches, stamped.execution.batches);
    assert_eq!(
        replay.execution.read_evidence.stamp_validation.method,
        "rfc9162_direct_versions",
        "retained-stamp evidence must authenticate direct semantic versions after the head advances"
    );

    let current_catalog = fixture_catalog(&engine);
    let current_bound = bind(&query, &parameters, &current_catalog).unwrap();
    assert!(matches!(
        pipeline.plan(&current_bound),
        Err(Error::Integrity(reason)) if reason.contains("pipeline read stamp")
    ));
}

#[test]
fn explain_analyze_reports_governed_streaming_plane_and_preserves_reference_rows() {
    let engine = RrflowMxStore::new();
    engine.runtime().commit(&fixture_commit()).unwrap();
    let catalog = fixture_catalog(&engine);
    let query = parse(
        "FROM record:document AT VALID 100 KNOWN HEAD WHERE status != \"closed\" PROJECT title LIMIT 1 EXPLAIN ANALYZE",
    )
    .unwrap();
    let physical = plan(&bind(&query, &Parameters::new(), &catalog).unwrap()).unwrap();
    let budget = ExecutionBudget {
        max_batch_rows: 1,
        ..ExecutionBudget::default()
    };
    let result = execute(&engine, &physical, &budget).unwrap();
    assert_eq!(
        flattened_rows(&result),
        vec![rrd_query::QueryRow {
            identity: "record:document:a".into(),
            values: BTreeMap::from([("title".into(), value("Alpha"))]),
        }]
    );

    let analysis = result.analysis.as_ref().expect("analysis was requested");
    assert_eq!(analysis.engine, "datafusion-55");
    assert_eq!(analysis.provider_scans, 1);
    assert_eq!(analysis.input_rows, 2);
    assert_eq!(analysis.input_batches, 2);
    assert!(analysis.input_memory_bytes > 0);
    assert_eq!(analysis.output_batches, 1);
    assert_eq!(analysis.projection_pushdown, "exact");
    assert_eq!(analysis.filter_pushdown, "unsupported_exact_post_scan");
    assert_eq!(analysis.limit_pushdown, "retained_above_scan");
    assert!(analysis.physical_operators > 0);
    assert!(analysis.peak_memory_bytes >= analysis.input_memory_bytes);
    assert!(analysis.peak_memory_bytes <= budget.max_memory_bytes);
    assert!(analysis.spilled_bytes <= budget.max_spill_bytes);

    let too_small = ExecutionBudget {
        max_memory_bytes: analysis.input_memory_bytes,
        ..budget
    };
    assert!(matches!(
        execute(&engine, &physical, &too_small),
        Err(Error::Budget(reason)) if reason.contains("Arrow snapshot requires")
    ));

    let without_analysis = parse(
        "FROM record:document AT VALID 100 KNOWN HEAD WHERE status != \"closed\" PROJECT title LIMIT 1",
    )
    .unwrap();
    let without_analysis =
        plan(&bind(&without_analysis, &Parameters::new(), &catalog).unwrap()).unwrap();
    assert!(
        execute(&engine, &without_analysis, &ExecutionBudget::default())
            .unwrap()
            .analysis
            .is_none()
    );
}

#[test]
fn equi_join_uses_one_stamp_matches_the_reference_oracle_and_is_strictly_bounded() {
    let text = "FROM record:document JOIN relation:depends_on ON id = from_id AT VALID 100 KNOWN HEAD WHERE right.strength != \"cycle\" PROJECT left.id, right.to_id EXPLAIN CONTRACT";
    let expected = vec![BTreeMap::from([
        ("left.id".into(), RuntimeValue::String("a".into())),
        ("right.to_id".into(), RuntimeValue::String("b".into())),
    ])];

    let memory = RrflowMxStore::new();
    let memory_execution = execute_fixture(&memory, text);
    assert_eq!(
        flattened_rows(&memory_execution)
            .into_iter()
            .map(|row| row.values)
            .collect::<Vec<_>>(),
        expected
    );
    assert!(memory_execution
        .batches
        .iter()
        .all(|batch| batch.rows.len() <= ExecutionBudget::default().max_batch_rows));

    let rrflow_kv = tempfile::tempdir().unwrap();
    let rrflow_kv_path = rrflow_kv.path().join("rrflow-kv");
    let persisted_rows = {
        let engine = RrflowKvStore::open(&rrflow_kv_path).unwrap();
        flattened_rows(&execute_fixture(&engine, text))
    };
    let reopened = RrflowKvStore::open(&rrflow_kv_path).unwrap();
    let (_, reopened_execution) = execute_text(&reopened, text);
    assert_eq!(flattened_rows(&reopened_execution), persisted_rows);
    assert_eq!(persisted_rows, flattened_rows(&memory_execution));

    let catalog = fixture_catalog(&memory);
    let unfiltered = parse(
        "FROM record:document JOIN relation:depends_on ON id = from_id AT VALID 100 KNOWN HEAD PROJECT left.id, right.to_id",
    )
    .unwrap();
    let physical = plan(&bind(&unfiltered, &Parameters::new(), &catalog).unwrap()).unwrap();
    let incompatible = parse(
        "FROM record:document JOIN relation:depends_on ON valid_from = from_id AT VALID 100 KNOWN HEAD PROJECT left.id",
    )
    .unwrap();
    assert!(matches!(
        bind(&incompatible, &Parameters::new(), &catalog),
        Err(Error::Binding(reason)) if reason.contains("incompatible types")
    ));
    let streamed = execute(
        &memory,
        &physical,
        &ExecutionBudget {
            max_batch_rows: 1,
            ..ExecutionBudget::default()
        },
    )
    .unwrap();
    assert_eq!(streamed.returned_rows, 2);
    assert!(!streamed.truncated);
    assert_eq!(streamed.batches.len(), 2);
    assert_eq!(streamed.batches[0].ordinal, 0);
    assert!(!streamed.batches[0].done);
    assert_eq!(streamed.batches[0].rows.len(), 1);
    assert_eq!(streamed.batches[1].ordinal, 1);
    assert!(streamed.batches[1].done);
    assert_eq!(streamed.batches[1].rows.len(), 1);
    assert_eq!(flattened_rows(&streamed).len(), streamed.returned_rows);

    let error = execute(
        &memory,
        &physical,
        &ExecutionBudget {
            max_rows: 1,
            ..ExecutionBudget::default()
        },
    )
    .unwrap_err();
    assert!(matches!(error, Error::Budget(reason) if reason.contains("join produces more")));
}

#[test]
fn valid_time_and_known_at_are_stable_across_profiles_and_rrflow_kv_reopen() {
    let memory = RrflowMxStore::new();
    seed_historical_corrections(&memory);
    let expected = historical_outcome(&memory);

    let rrflow_kv_root = tempfile::tempdir().unwrap();
    let rrflow_kv_path = rrflow_kv_root.path().join("rrflow-kv");
    {
        let rrflow_kv = RrflowKvStore::open(&rrflow_kv_path).unwrap();
        seed_historical_corrections(&rrflow_kv);
        assert_eq!(historical_outcome(&rrflow_kv), expected);
    }
    let reopened = RrflowKvStore::open(&rrflow_kv_path).unwrap();
    assert_eq!(historical_outcome(&reopened), expected);
    assert_eq!(
        expected,
        (
            "Alpha".into(),
            "Alpha corrected".into(),
            "ready".into(),
            "corrected".into(),
            10,
            3,
        )
    );
}

#[test]
fn series_and_geo_queries_match_every_persistent_engine() {
    for (text, field, expected) in [
        (
            "FROM series:metric AT VALID 100 KNOWN HEAD WHERE series_id = \"latency\" PROJECT observed_at, value EXPLAIN CONTRACT",
            "value",
            RuntimeValue::Decimal("12.5".into()),
        ),
        (
            "FROM geo:location AT VALID 100 KNOWN HEAD WHERE subject_id = \"a\" PROJECT geometry_kind, longitude, latitude EXPLAIN CONTRACT",
            "longitude",
            RuntimeValue::Decimal("-122.4".into()),
        ),
    ] {
        let memory = RrflowMxStore::new();
        let rrflow_kv_root = tempfile::tempdir().unwrap();
        let rrflow_kv = RrflowKvStore::open(rrflow_kv_root.path()).unwrap();
        let left = execute_fixture(&memory, text);
        assert_eq!(left, execute_fixture(&rrflow_kv, text), "{text}");
        assert_eq!(left.returned_rows, 1, "{text}");
        assert_eq!(left.batches[0].rows[0].values[field], expected, "{text}");
    }

    for text in [
        "FROM series:metric AT VALID 99 KNOWN HEAD PROJECT *",
        "FROM geo:location AT VALID 9 KNOWN HEAD PROJECT *",
    ] {
        let engine = RrflowMxStore::new();
        assert_eq!(execute_fixture(&engine, text).returned_rows, 0, "{text}");
    }
}

#[test]
fn typed_comparisons_match_every_persistent_engine_and_reject_unsupported_ordering() {
    let text = "FROM series:metric AT VALID 100 KNOWN HEAD WHERE observed_at >= 100 AND series_id != \"other\" PROJECT series_id, observed_at EXPLAIN CONTRACT";
    let memory = RrflowMxStore::new();
    let rrflow_kv_root = tempfile::tempdir().unwrap();
    let rrflow_kv = RrflowKvStore::open(rrflow_kv_root.path()).unwrap();
    let expected = execute_fixture(&memory, text);
    assert_eq!(expected, execute_fixture(&rrflow_kv, text));
    assert_eq!(expected.returned_rows, 1);
    assert_eq!(
        expected.batches[0].rows[0].values["observed_at"],
        RuntimeValue::Unsigned(100)
    );
    for predicate in [
        "observed_at = 100",
        "observed_at != 101",
        "observed_at < 101",
        "observed_at <= 100",
        "observed_at > 99",
        "observed_at >= 100",
    ] {
        let engine = RrflowMxStore::new();
        let query = format!(
            "FROM series:metric AT VALID 100 KNOWN HEAD WHERE {predicate} PROJECT observed_at"
        );
        assert_eq!(execute_fixture(&engine, &query).returned_rows, 1, "{query}");
    }

    let parsed = parse(text).unwrap();
    assert_eq!(parsed.filters[1].comparison, ComparisonOperator::NotEqual);

    let catalog = fixture_catalog(&memory);
    let decimal_ordering =
        parse("FROM series:metric AT VALID 100 KNOWN HEAD WHERE value > $minimum PROJECT value")
            .unwrap();
    let parameters = Parameters::from([("minimum".into(), RuntimeValue::Decimal("10.0".into()))]);
    assert!(matches!(
        bind(&decimal_ordering, &parameters, &catalog),
        Err(Error::Binding(_))
    ));
}

#[test]
fn bounded_graph_traversal_is_deterministic_across_every_engine() {
    for (text, expected_node) in [
        (
            "FROM traverse:depends_on START document:a DIRECTION OUTGOING DEPTH 3 AT VALID 100 KNOWN HEAD PROJECT node_id, depth, path EXPLAIN CONTRACT",
            "b",
        ),
        (
            "FROM traverse:depends_on START document:b DIRECTION INCOMING DEPTH 3 AT VALID 100 KNOWN HEAD PROJECT node_id, depth, path EXPLAIN CONTRACT",
            "a",
        ),
    ] {
        let memory = RrflowMxStore::new();
        let rrflow_kv_root = tempfile::tempdir().unwrap();
        let rrflow_kv = RrflowKvStore::open(rrflow_kv_root.path()).unwrap();
        let left = execute_fixture(&memory, text);
        assert_eq!(left, execute_fixture(&rrflow_kv, text), "{text}");
        assert_eq!(left.returned_rows, 1, "{text}");
        assert_eq!(
            left.batches[0].rows[0].values["node_id"],
            value(expected_node)
        );
        assert_eq!(
            left.batches[0].rows[0].values["depth"],
            RuntimeValue::Unsigned(1)
        );
    }
}

#[test]
fn all_source_families_execute_at_explicit_time() {
    let engine = RrflowMxStore::new();
    engine.runtime().commit(&fixture_commit()).unwrap();
    let catalog = fixture_catalog(&engine);
    for (text, identity) in [
        (
            "FROM relation:depends_on AT VALID 100 KNOWN HEAD WHERE id = \"a-b\" PROJECT id",
            "relation:depends_on:a-b",
        ),
        (
            "FROM event:tool_result AT VALID 100 KNOWN HEAD WHERE ok = true PROJECT cursor",
            "event:tool_result:5",
        ),
        (
            "FROM claim:status AT VALID 100 KNOWN HEAD PROJECT subject, object",
            "claim:document:a:status",
        ),
        (
            "FROM series:metric AT VALID 100 KNOWN HEAD WHERE series_id = \"latency\" PROJECT observed_at, value",
            "series:metric:latency:100:latency-100",
        ),
        (
            "FROM geo:location AT VALID 100 KNOWN HEAD WHERE subject_id = \"a\" PROJECT geometry_kind, longitude, latitude",
            "geo:location:alpha",
        ),
    ] {
        let query = parse(text).unwrap();
        let result = execute(
            &engine,
            &plan(&bind(&query, &Parameters::new(), &catalog).unwrap()).unwrap(),
            &ExecutionBudget::default(),
        )
        .unwrap();
        assert_eq!(result.returned_rows, 1, "{text}");
        assert_eq!(result.batches[0].rows[0].identity, identity);
    }
}

#[test]
fn bound_event_cursor_uses_one_typed_identity_range_on_every_engine() {
    fn exercise<E: StorageEngine>(engine: &E) -> rrd_query::QueryExecution {
        engine.runtime().commit(&fixture_commit()).unwrap();
        let catalog = fixture_catalog(engine);
        let query = parse(
            "FROM event:tool_result AT VALID 100 KNOWN HEAD WHERE cursor = 5 AND ok = true PROJECT cursor",
        )
        .unwrap();
        let physical = plan(&bind(&query, &Parameters::new(), &catalog).unwrap()).unwrap();
        let [PhysicalOperator::VersionedRead {
            sources,
            exact: true,
            ..
        }, PhysicalOperator::DataFusionEvaluate] = physical.operators.as_slice()
        else {
            panic!("event cursor did not select a versioned identity read")
        };
        assert!(matches!(
            sources.as_slice(),
            [RuntimeVersionedSource::Identity { model, reference }]
                if model.is_event_like()
                    && reference == &RuntimeRef::new("tool_result", "cursor:5").unwrap()
        ));
        assert_eq!(
            physical
                .explanation
                .candidates
                .iter()
                .find(|candidate| candidate.selected)
                .unwrap()
                .name,
            "versioned_identity_read"
        );
        execute(
            engine,
            &physical,
            &ExecutionBudget {
                max_storage_keys: 64,
                ..ExecutionBudget::default()
            },
        )
        .unwrap()
    }

    let memory = RrflowMxStore::new();
    let rrflow_kv_root = tempfile::tempdir().unwrap();
    let rrflow_kv = RrflowKvStore::open(rrflow_kv_root.path()).unwrap();
    let expected = exercise(&memory);
    assert_eq!(expected, exercise(&rrflow_kv));
    assert_eq!(expected.selected_versions, 1);
    assert_eq!(
        expected.read_evidence.stamp_validation.method,
        "authenticated_current_head"
    );
    assert_eq!(
        expected
            .read_evidence
            .paths
            .iter()
            .find(|path| path.path == RuntimeReadAccessPath::EventVersions)
            .unwrap()
            .range_scans,
        1
    );
    assert_eq!(expected.returned_rows, 1);
    assert_eq!(expected.batches[0].rows[0].identity, "event:tool_result:5");
}

#[test]
fn event_cursor_outside_the_stamp_is_an_exact_empty_path() {
    let engine = RrflowMxStore::new();
    engine.runtime().commit(&fixture_commit()).unwrap();
    let catalog = fixture_catalog(&engine);
    let query =
        parse("FROM event:tool_result AT VALID 100 KNOWN HEAD WHERE cursor = 99 PROJECT cursor")
            .unwrap();
    let physical = plan(&bind(&query, &Parameters::new(), &catalog).unwrap()).unwrap();
    let result = execute(
        &engine,
        &physical,
        &ExecutionBudget {
            max_storage_keys: 64,
            ..ExecutionBudget::default()
        },
    )
    .unwrap();
    assert_eq!(result.selected_versions, 0);
    assert_eq!(result.returned_rows, 0);
}

#[test]
fn cursor_lookup_uses_authenticated_logarithmic_validation() {
    const EVENTS: u64 = 4_096;
    let engine = RrflowMxStore::new();
    let fixture_head = engine
        .runtime()
        .commit(&fixture_commit())
        .unwrap()
        .last_cursor;
    let subject = RuntimeRef::new("document", "a").unwrap();
    engine
        .runtime()
        .commit(&RuntimeCommit {
            scope: ScopeId::new("instance:test").unwrap(),
            at: 101,
            actor: "agent:bulk".into(),
            expected_cursor: fixture_head,
            mutations: (0..EVENTS)
                .map(|_| RuntimeMutation::Event {
                    event: RuntimeEvent {
                        kind: RuntimeType::new("tool_result").unwrap(),
                        subject: Some(subject.clone()),
                        properties: BTreeMap::from([("ok".into(), RuntimeValue::Bool(true))]),
                    },
                })
                .collect(),
        })
        .unwrap();
    let catalog = fixture_catalog(&engine);
    let head = fixture_head + EVENTS;

    let point = parse(&format!(
        "FROM event:tool_result AT VALID 101 KNOWN HEAD WHERE cursor = {head} PROJECT cursor"
    ))
    .unwrap();
    let point_plan = plan(&bind(&point, &Parameters::new(), &catalog).unwrap()).unwrap();
    let point_result = execute(
        &engine,
        &point_plan,
        &ExecutionBudget {
            max_storage_keys: 64,
            ..ExecutionBudget::default()
        },
    )
    .unwrap();
    assert_eq!(point_result.selected_versions, 1);
    let proof = point_result
        .read_evidence
        .paths
        .iter()
        .find(|path| path.path == RuntimeReadAccessPath::AccumulatorProof)
        .unwrap();
    assert!(proof.keys_examined > 0);
    assert!(proof.keys_examined <= 13);
    assert_eq!(point_result.returned_rows, 1);

    let unbound =
        parse("FROM event:tool_result AT VALID 101 KNOWN HEAD WHERE ok = true PROJECT cursor")
            .unwrap();
    let unbound_plan = plan(&bind(&unbound, &Parameters::new(), &catalog).unwrap()).unwrap();
    assert!(matches!(
        execute(
            &engine,
            &unbound_plan,
            &ExecutionBudget {
                max_storage_keys: 64,
                ..ExecutionBudget::default()
            }
        ),
        Err(Error::Budget(_))
    ));
}

#[test]
fn text_and_typed_sdk_produce_the_same_plan() {
    let engine = RrflowMxStore::new();
    engine.runtime().commit(&fixture_commit()).unwrap();
    let catalog = fixture_catalog(&engine);
    let parsed = parse("FROM record:document AT VALID 100 KNOWN HEAD PROJECT id").unwrap();
    let mut typed = Query::new(
        Source::Record {
            kind: RuntimeType::new("document").unwrap(),
        },
        TemporalSelector {
            valid_at: TimeExpr::Literal(100),
            known_at: CursorExpr::Head,
        },
    );
    typed.projection = Projection::Fields(vec!["id".into()]);
    let parsed_plan = plan(&bind(&parsed, &Parameters::new(), &catalog).unwrap()).unwrap();
    let typed_plan = plan(&bind(&typed, &Parameters::new(), &catalog).unwrap()).unwrap();
    assert_eq!(parsed_plan, typed_plan);
}

#[test]
fn binding_and_budget_fail_closed() {
    let engine = RrflowMxStore::new();
    engine.runtime().commit(&fixture_commit()).unwrap();
    let catalog = fixture_catalog(&engine);
    let unknown = parse("FROM record:document AT VALID 100 KNOWN HEAD PROJECT missing").unwrap();
    assert!(matches!(
        bind(&unknown, &Parameters::new(), &catalog),
        Err(Error::Binding(_))
    ));
    let wrong_type =
        parse("FROM record:document AT VALID 100 KNOWN HEAD WHERE status = true PROJECT id")
            .unwrap();
    assert!(matches!(
        bind(&wrong_type, &Parameters::new(), &catalog),
        Err(Error::Binding(_))
    ));

    let query = parse("FROM record:document AT VALID 100 KNOWN HEAD PROJECT *").unwrap();
    let physical = plan(&bind(&query, &Parameters::new(), &catalog).unwrap()).unwrap();
    assert!(matches!(
        execute(
            &engine,
            &physical,
            &ExecutionBudget {
                max_storage_keys: 5,
                ..ExecutionBudget::default()
            }
        ),
        Err(Error::Budget(_))
    ));
    let mut forged = physical;
    forged.explanation.contract.exact = false;
    assert!(matches!(
        execute(&engine, &forged, &ExecutionBudget::default()),
        Err(Error::Integrity(_))
    ));

    let limited = parse("FROM record:document AT VALID 100 KNOWN HEAD PROJECT id LIMIT 1").unwrap();
    let result = execute(
        &engine,
        &plan(&bind(&limited, &Parameters::new(), &catalog).unwrap()).unwrap(),
        &ExecutionBudget::default(),
    )
    .unwrap();
    assert_eq!(result.returned_rows, 1);
    assert!(
        !result.truncated,
        "a semantic LIMIT is not budget truncation"
    );

    let record_only = Catalog::capture_for_query(
        &engine,
        &ScopeId::new("instance:test").unwrap(),
        &query,
        read_budget(),
    )
    .unwrap();
    let relation = parse("FROM relation:depends_on AT VALID 100 KNOWN HEAD PROJECT id").unwrap();
    assert!(matches!(
        bind(&relation, &Parameters::new(), &record_only),
        Err(Error::Catalog(reason)) if reason.contains("did not capture required semantic source")
    ));
    assert!(matches!(
        Catalog::capture_for_query(
            &engine,
            &ScopeId::new("instance:test").unwrap(),
            &query,
            RuntimeReadBudget::new(1).unwrap(),
        ),
        Err(Error::Budget(_))
    ));
}

#[test]
fn event_queries_honor_cursor_typed_retirement_at_valid_time() {
    let engine = RrflowMxStore::new();
    let initial = engine.runtime().commit(&fixture_commit()).unwrap();
    engine
        .runtime()
        .commit(&RuntimeCommit {
            scope: ScopeId::new("instance:test").unwrap(),
            at: 101,
            actor: "agent:event-retirement".into(),
            expected_cursor: initial.last_cursor,
            mutations: vec![RuntimeMutation::Retire {
                retirement: RuntimeRetirement {
                    model: RuntimeLogicalModel::Event,
                    reference: RuntimeRef::new("tool_result", "cursor:5").unwrap(),
                    effective_at: 101,
                },
            }],
        })
        .unwrap();
    let before = execute_text(
        &engine,
        "FROM event:tool_result AT VALID 100 KNOWN HEAD PROJECT cursor",
    )
    .1;
    let after = execute_text(
        &engine,
        "FROM event:tool_result AT VALID 101 KNOWN HEAD PROJECT cursor",
    )
    .1;
    assert_eq!(before.returned_rows, 1);
    assert_eq!(after.returned_rows, 0);
}

#[test]
fn a_schemaless_record_table_can_start_a_traversal() {
    // A strict relation may point into a schemaless record table; that table's
    // records are graph nodes and must be traversal starts like strict ones.
    let mut registry = RuntimeSchemaRegistry::empty(1, "schemaless traversal fixture");
    let note = RuntimeType::new("note").unwrap();
    let cites = RuntimeType::new("cites").unwrap();
    registry.tables.insert(
        note.clone(),
        RuntimeTableSchema::schemaless(RuntimeLogicalModel::Document),
    );
    registry
        .define_relation_table(
            cites.clone(),
            RuntimeRelationSchema {
                from: BTreeSet::from([note.clone()]),
                to: BTreeSet::from([note.clone()]),
                ..RuntimeRelationSchema::default()
            },
        )
        .unwrap();
    let first = RuntimeRef::new("note", "a").unwrap();
    let second = RuntimeRef::new("note", "b").unwrap();
    let record = |reference: &RuntimeRef| RuntimeMutation::Record {
        record: RuntimeRecord {
            reference: reference.clone(),
            valid_from: 10,
            valid_to: None,
            properties: properties(&[("title", "note")]),
        },
    };
    let scope = ScopeId::new("instance:test").unwrap();
    let engine = RrflowMxStore::new();
    engine
        .runtime()
        .commit(&RuntimeCommit {
            scope: scope.clone(),
            at: 100,
            actor: "agent:test".into(),
            expected_cursor: 0,
            mutations: vec![
                RuntimeMutation::Schema { registry },
                record(&first),
                record(&second),
                RuntimeMutation::Relation {
                    relation: RuntimeRelation {
                        reference: RuntimeRef::new("cites", "a-b").unwrap(),
                        from: first,
                        to: second.clone(),
                        valid_from: 10,
                        valid_to: None,
                        properties: RuntimeProperties::new(),
                    },
                },
            ],
        })
        .unwrap();
    let text = "FROM traverse:cites START note:b DIRECTION INCOMING DEPTH 1 AT VALID 100 KNOWN HEAD PROJECT node_id";
    let query = parse(text).unwrap();
    let catalog = Catalog::capture_for_sources(
        &engine,
        &scope,
        &[Source::Traversal {
            relation: cites,
            start: second,
            direction: rrd_query::TraversalDirection::Incoming,
            max_depth: 1,
        }],
        read_budget(),
    )
    .unwrap();
    let execution = execute(
        &engine,
        &plan(&bind(&query, &Parameters::new(), &catalog).unwrap()).unwrap(),
        &ExecutionBudget::default(),
    )
    .unwrap();
    assert_eq!(execution.returned_rows, 1);
    assert_eq!(execution.batches[0].rows[0].values["node_id"], value("a"));
}
