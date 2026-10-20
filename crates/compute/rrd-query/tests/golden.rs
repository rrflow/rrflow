use rrd_core::{
    RuntimeCommit, RuntimeEvent, RuntimeEventSchema, RuntimeLogicalModel, RuntimeMutation,
    RuntimeProperties, RuntimeRecord, RuntimeRecordSchema, RuntimeRef, RuntimeRelation,
    RuntimeRelationSchema, RuntimeSchemaRegistry, RuntimeType, ScopeId,
};
use rrd_query::parse;
use rrd_query::{bind, plan, Catalog, ExecutionBudget, Parameters, Source, TraversalDirection};
use rrd_store::{RrflowMxStore, RuntimeReadBudget, StorageEngine};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Serialize)]
struct Vector {
    source: &'static str,
    plan: rrd_query::PhysicalPlan,
}

#[test]
fn physical_plan_matches_golden_vector() {
    let scope = ScopeId::new("instance:golden").unwrap();
    let mut schema = RuntimeSchemaRegistry::empty(1, "golden query contract");
    schema
        .define_record_table(
            RuntimeType::new("document").unwrap(),
            RuntimeLogicalModel::Relational,
            RuntimeRecordSchema {
                allow_additional_properties: false,
                properties: BTreeMap::new(),
                ..RuntimeRecordSchema::default()
            },
        )
        .unwrap();
    schema
        .define_event_table(
            RuntimeType::new("tool_result").unwrap(),
            RuntimeLogicalModel::Event,
            RuntimeEventSchema::default(),
        )
        .unwrap();
    schema
        .define_relation_table(
            RuntimeType::new("depends_on").unwrap(),
            RuntimeRelationSchema {
                from: BTreeSet::from([RuntimeType::new("document").unwrap()]),
                to: BTreeSet::from([RuntimeType::new("document").unwrap()]),
                ..RuntimeRelationSchema::default()
            },
        )
        .unwrap();
    let engine = RrflowMxStore::new();
    engine
        .runtime()
        .commit(&RuntimeCommit {
            scope: scope.clone(),
            at: 100,
            actor: "test:golden".into(),
            expected_cursor: 0,
            mutations: vec![
                RuntimeMutation::Schema { registry: schema },
                RuntimeMutation::Record {
                    record: RuntimeRecord {
                        reference: RuntimeRef::new("document", "a").unwrap(),
                        valid_from: 1,
                        valid_to: None,
                        properties: RuntimeProperties::new(),
                    },
                },
                RuntimeMutation::Record {
                    record: RuntimeRecord {
                        reference: RuntimeRef::new("document", "b").unwrap(),
                        valid_from: 1,
                        valid_to: None,
                        properties: RuntimeProperties::new(),
                    },
                },
                RuntimeMutation::Event {
                    event: RuntimeEvent {
                        kind: RuntimeType::new("tool_result").unwrap(),
                        subject: None,
                        properties: RuntimeProperties::new(),
                    },
                },
                RuntimeMutation::Relation {
                    relation: RuntimeRelation {
                        reference: RuntimeRef::new("depends_on", "a-b").unwrap(),
                        from: RuntimeRef::new("document", "a").unwrap(),
                        to: RuntimeRef::new("document", "b").unwrap(),
                        valid_from: 1,
                        valid_to: None,
                        properties: RuntimeProperties::new(),
                    },
                },
            ],
        })
        .unwrap();
    let catalog = Catalog::capture_for_sources(
        &engine,
        &scope,
        &[
            Source::Record {
                kind: RuntimeType::new("document").unwrap(),
            },
            Source::Event {
                kind: RuntimeType::new("tool_result").unwrap(),
            },
            Source::Traversal {
                relation: RuntimeType::new("depends_on").unwrap(),
                start: RuntimeRef::new("document", "a").unwrap(),
                direction: TraversalDirection::Outgoing,
                max_depth: 2,
            },
        ],
        RuntimeReadBudget::new(ExecutionBudget::default().max_storage_keys).unwrap(),
    )
    .unwrap();
    let vectors = [
        "FROM record:document AT VALID 100 KNOWN 4 PROJECT id LIMIT 5 EXPLAIN CONTRACT",
        "FROM event:tool_result AT VALID 100 KNOWN 4 WHERE cursor = 4 PROJECT cursor EXPLAIN CONTRACT",
        "FROM traverse:depends_on START document:a DIRECTION OUTGOING DEPTH 2 AT VALID 100 KNOWN 5 PROJECT node_id, depth EXPLAIN CONTRACT",
    ]
    .into_iter()
    .map(|source| {
        let query = parse(source).unwrap();
        Vector {
            source,
            plan: plan(&bind(&query, &Parameters::new(), &catalog).unwrap()).unwrap(),
        }
    })
    .collect::<Vec<_>>();
    let actual = format!("{}\n", serde_json::to_string_pretty(&vectors).unwrap());
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/plan-vectors.json");
    if std::env::var_os("RRFLOW_UPDATE_GOLDENS").is_some() {
        std::fs::create_dir_all(format!("{}/fixtures", env!("CARGO_MANIFEST_DIR"))).unwrap();
        std::fs::write(path, &actual).unwrap();
    }
    let expected = std::fs::read_to_string(path)
        .unwrap_or_else(|_| panic!("missing {path}; run with RRFLOW_UPDATE_GOLDENS=1"));
    assert_eq!(actual, expected);
}
