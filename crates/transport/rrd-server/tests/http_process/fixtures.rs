//! Seeded query fixtures and the deployment conformance corpus.

use super::*;

pub(super) fn seed_query_fixture(root: &Path) {
    let engine = RrflowKvStore::open(root).unwrap();
    let mut registry = RuntimeSchemaRegistry::empty(1, "RRD query fixture");
    registry
        .define_record_table(
            RuntimeType::new("document").unwrap(),
            RuntimeLogicalModel::Relational,
            RuntimeRecordSchema {
                properties: BTreeMap::from([(
                    "title".into(),
                    RuntimePropertySchema::required(RuntimeValueType::String),
                )]),
                ..RuntimeRecordSchema::default()
            },
        )
        .unwrap();
    registry
        .define_event_table(
            RuntimeType::new("observed").unwrap(),
            RuntimeLogicalModel::Event,
            RuntimeEventSchema {
                subject_required: true,
                subject_types: [RuntimeType::new("document").unwrap()]
                    .into_iter()
                    .collect(),
                allow_additional_properties: true,
                ..RuntimeEventSchema::default()
            },
        )
        .unwrap();
    engine
        .runtime()
        .commit(&RuntimeCommit {
            scope: ScopeId::new("instance:socket-test").unwrap(),
            at: 100,
            actor: "rrd-query-fixture".into(),
            expected_cursor: 0,
            mutations: vec![
                RuntimeMutation::Schema { registry },
                RuntimeMutation::Record {
                    record: RuntimeRecord {
                        reference: RuntimeRef::new("document", "alpha").unwrap(),
                        valid_from: 100,
                        valid_to: None,
                        properties: RuntimeProperties::from([(
                            "title".into(),
                            RuntimeValue::String("Alpha".into()),
                        )]),
                    },
                },
            ],
        })
        .unwrap();
}

pub(super) fn deployment_corpus() -> DeploymentConformanceCorpus {
    let corpus = serde_json::from_str(include_str!(
        "../../../../../fixtures/rrd-deployment-conformance-v1.json"
    ))
    .unwrap();
    DeploymentConformanceCorpus::validate(&corpus).unwrap();
    corpus
}

pub(super) fn deployment_mutations() -> Vec<TransactionMutation> {
    let corpus = deployment_corpus();
    let mut mutations = vec![json!({
        "mutation": "put_schema",
        "registry": {
            "revision": 1,
            "migration": "install deployment conformance corpus",
            "tables": {
                "document": {"model": "relational", "mode": "strict"}
            },
            "records": {
                "document": {
                    "properties": {
                        "body": {"value_type": "string", "required": true}
                    },
                    "allow_additional_properties": false,
                    "unique_properties": []
                }
            },
            "relations": {},
            "events": {}
        }
    })];
    mutations.extend(corpus.documents.iter().map(|document| {
        json!({
            "mutation": "put_record",
            "reference": {"kind": "document", "id": document.id},
            "valid_from": corpus.query.valid_at,
            "properties": {
                "body": {"type": "string", "value": document.text}
            }
        })
    }));
    serde_json::from_value(Value::Array(mutations)).unwrap()
}

pub(super) fn extend_installed_schema_for_deployment_corpus(root: &Path, scope: &str) {
    let storage = RrflowKvStore::open(root).unwrap();
    let scope = ScopeId::new(scope).unwrap();
    let read = storage.runtime().read_stamp(&scope).unwrap();
    let mut registry = storage.runtime().schema(&scope).unwrap().unwrap();
    registry.revision += 1;
    registry.migration = "extend installed schema for deployment corpus".into();
    registry
        .define_record_table(
            RuntimeType::new("document").unwrap(),
            RuntimeLogicalModel::Relational,
            RuntimeRecordSchema {
                properties: BTreeMap::from([(
                    "body".into(),
                    RuntimePropertySchema::required(RuntimeValueType::String),
                )]),
                ..RuntimeRecordSchema::default()
            },
        )
        .unwrap();
    registry
        .define_relation_table(
            RuntimeType::new("deployment_link").unwrap(),
            RuntimeRelationSchema {
                from: BTreeSet::from([RuntimeType::new("document").unwrap()]),
                to: BTreeSet::from([RuntimeType::new("document").unwrap()]),
                ..RuntimeRelationSchema::default()
            },
        )
        .unwrap();
    storage
        .runtime()
        .commit(&RuntimeCommit {
            scope,
            at: 2,
            actor: "rrd-server-installed-fixture".into(),
            expected_cursor: read.commit_cursor,
            mutations: vec![RuntimeMutation::Schema { registry }],
        })
        .unwrap();
}
