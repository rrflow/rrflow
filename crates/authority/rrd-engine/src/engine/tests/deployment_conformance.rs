use super::*;
use crate::ServiceErrorKind;
use std::collections::BTreeMap;

use rrd_contract::{
    CommitTransaction, DataEngramOrganisation, DataLogicalModel, DataPropertySchema,
    DataRecordSchema, DataReference, DataSchemaMode, DataSchemaRegistry, DataTableSchema,
    DataValueType, DeploymentConformanceCorpus, ExecuteQuery, QueryBudget, QueryValue,
};

fn corpus() -> DeploymentConformanceCorpus {
    let corpus = serde_json::from_str(include_str!(
        "../../../../../../fixtures/rrd-deployment-conformance-v1.json"
    ))
    .unwrap();
    DeploymentConformanceCorpus::validate(&corpus).unwrap();
    corpus
}

fn run_corpus(engine: &RrdEngine, suffix: &str) -> Vec<CanonicalId> {
    let corpus = corpus();
    let lease = engine
        .create_session(
            &session_request(5_000, 2),
            &id(&format!("{suffix}-session-key")),
            1_000,
            &format!("request-{suffix}-session"),
            &format!("operation-{suffix}-session"),
        )
        .unwrap();
    let transaction = engine
        .begin_transaction(
            &lease.session_id,
            &lease.token,
            &BeginTransaction {
                scope: CanonicalId::new("data").unwrap(),
                timeout_ms: 1_000,
            },
            &mutation_context(
                &id(&format!("{suffix}-begin-key")),
                &format!("request-{suffix}-begin"),
                &format!("operation-{suffix}-begin"),
            ),
            1_100,
        )
        .unwrap();
    let table = CanonicalId::new("document").unwrap();
    let mut mutations = vec![TransactionMutation::PutSchema {
        registry: DataSchemaRegistry {
            revision: 1,
            migration: "install deployment conformance corpus".into(),
            organisation: DataEngramOrganisation::default(),
            tables: BTreeMap::from([(
                table.clone(),
                DataTableSchema {
                    model: DataLogicalModel::Document,
                    mode: DataSchemaMode::Strict,
                    properties: BTreeMap::new(),
                    allow_additional_properties: false,
                },
            )]),
            records: BTreeMap::from([(
                table.clone(),
                DataRecordSchema {
                    properties: BTreeMap::from([(
                        "body".into(),
                        DataPropertySchema {
                            value_type: DataValueType::String,
                            required: true,
                        },
                    )]),
                    allow_additional_properties: false,
                    unique_properties: Default::default(),
                },
            )]),
            relations: BTreeMap::new(),
            events: BTreeMap::new(),
        },
    }];
    mutations.extend(
        corpus
            .documents
            .iter()
            .map(|document| TransactionMutation::PutRecord {
                reference: DataReference {
                    kind: table.clone(),
                    id: document.id.clone(),
                },
                valid_from: corpus.query.valid_at,
                valid_to: None,
                properties: BTreeMap::from([(
                    "body".into(),
                    QueryValue::String(document.text.clone()),
                )]),
            }),
    );
    let commit = CommitTransaction {
        operation_sha256: rrd_contract::transaction_operation_sha256(&mutations),
        mutations,
    };
    let receipt = engine
        .commit_transaction(
            &lease.session_id,
            &lease.token,
            &transaction.transaction_id,
            &id(&format!("{suffix}-commit-key")),
            &commit,
            1_200,
            &format!("request-{suffix}-commit"),
            &format!("operation-{suffix}-commit"),
        )
        .unwrap();
    assert_eq!(
        receipt.last_runtime_cursor,
        Some(corpus.documents.len() as u64 + 1)
    );
    let result = engine
        .execute_query(
            &lease.session_id,
            &lease.token,
            &ExecuteQuery {
                scope: format!("instance:{}", instance()),
                query: corpus.query.rrflowql,
                parameters: BTreeMap::new(),
                budget: QueryBudget::default(),
            },
            1_300,
            &format!("request-{suffix}-query"),
            &format!("operation-{suffix}-query"),
        )
        .unwrap();
    assert!(result.plan.exact);
    let analysis = result
        .execution
        .analysis
        .as_ref()
        .expect("the shared deployment corpus must execute through DataFusion");
    assert_eq!(analysis.engine, "datafusion-55");
    assert_eq!(analysis.provider_scans, 1);
    assert!(analysis.input_rows > 0);
    assert!(analysis.input_batches > 0);
    assert!(analysis.physical_operators > 0);
    assert_eq!(
        result.rows[0].values["body"],
        QueryValue::String(corpus.documents[0].text.clone())
    );
    result
        .rows
        .iter()
        .map(|row| {
            CanonicalId::new(
                row.identity
                    .strip_prefix("record:document:")
                    .expect("corpus row has document identity"),
            )
            .unwrap()
        })
        .collect()
}

#[test]
fn rrflow_mx_and_rrflow_kv_composition_roots_pass_one_datafusion_corpus() {
    let corpus = corpus();
    let rrflow_mx = RrdEngine::rrflow_mx(instance(), TOKEN_KEY);
    assert!(!rrflow_mx.has_persistent_root());
    assert_eq!(
        rrflow_mx.storage_profile_kind(),
        rrd_contract::StorageProfileKind::RrflowMx
    );
    assert_eq!(
        rrflow_mx.readiness(1).unwrap().backend.as_str(),
        "rrflow_mx"
    );
    assert_eq!(run_corpus(&rrflow_mx, "rrflow-mx"), corpus.expected_ids);

    let root = tempfile::tempdir().unwrap();
    let embedded = RrdEngine::open(root.path(), instance(), TOKEN_KEY).unwrap();
    assert!(embedded.has_persistent_root());
    assert_eq!(
        embedded.storage_profile_kind(),
        rrd_contract::StorageProfileKind::RrflowKv
    );
    assert_eq!(embedded.readiness(1).unwrap().backend.as_str(), "rrflow_kv");
    assert!(RrdEngine::open(root.path(), instance(), TOKEN_KEY).is_err());
    assert_eq!(run_corpus(&embedded, "embedded"), corpus.expected_ids);
    drop(embedded);

    let reopened = RrdEngine::open(root.path(), instance(), TOKEN_KEY).unwrap();
    assert_eq!(reopened.readiness(1).unwrap().runtime_cursor, 3);
}

#[test]
fn configured_query_ceiling_rejects_excess_work_as_resource_exhaustion() {
    let engine = RrdEngine::rrflow_mx(instance(), TOKEN_KEY);
    let lease = engine
        .create_session(
            &session_request(5_000, 1),
            &id("configuration-limit-session"),
            1_000,
            "request-configuration-limit-session",
            "operation-configuration-limit-session",
        )
        .unwrap();
    let budget = QueryBudget {
        max_rows: engine.engram_configuration().query.max_rows + 1,
        ..QueryBudget::default()
    };
    let error = engine
        .execute_query(
            &lease.session_id,
            &lease.token,
            &ExecuteQuery {
                scope: format!("instance:{}", instance()),
                query: "SELECT * FROM document".into(),
                parameters: BTreeMap::new(),
                budget,
            },
            1_100,
            "request-configuration-limit-query",
            "operation-configuration-limit-query",
        )
        .unwrap_err();
    assert_eq!(error.kind(), ServiceErrorKind::ResourceExhausted);
    assert!(matches!(error, ServiceError::ConfigurationLimit(_)));
}

#[test]
fn rrflow_mx_rejects_durability_only_operations_before_preparing_state() {
    let engine = RrdEngine::rrflow_mx(instance(), TOKEN_KEY);
    let lease = engine
        .create_session(
            &session_request(5_000, 1),
            &id("rrflow-mx-backup-session"),
            1_000,
            "request-rrflow-mx-backup-session",
            "operation-rrflow-mx-backup-session",
        )
        .unwrap();
    let before = engine.storage.control().sequence().unwrap();
    let result = engine.create_instance_backup(
        &lease.session_id,
        &lease.token,
        &id("rrflow-mx-backup-key"),
        &rrd_contract::CreateInstanceBackup {
            label: "rrflow-mx-backup".into(),
            created_at_unix_ms: 1_100,
        },
        1_100,
        "request-rrflow-mx-backup",
        "operation-rrflow-mx-backup",
    );
    assert!(matches!(result, Err(ServiceError::Backup(_))));
    assert_eq!(engine.storage.control().sequence().unwrap(), before);
}
