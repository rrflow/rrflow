use rrd_contract::{CanonicalId, InstalledEngramIdentity};
use rrd_core::{
    digest, EmbeddingProvenance, ProjectionStamp, ProjectionState, RuntimeCommit,
    RuntimeLogicalModel, RuntimeMutation, RuntimeProperties, RuntimeRecord, RuntimeRecordSchema,
    RuntimeRef, RuntimeSchemaRegistry, RuntimeTableSchema, RuntimeType, RuntimeValue,
    RuntimeVector, ScopeId, VectorCollectionAddress, VectorNormalization, VectorValue,
    DATA_RUNTIME_CONTRACT_VERSION,
};
use rrd_engine::execute_traced_operator_search;
use rrd_operator_knowledge::{
    OperatorAccessPath, OperatorAdapterDescriptor, OperatorKnowledgeBinding,
    OperatorSearchControls, OperatorSearchRequest, OperatorSourceRevision, OperatorSyncWork,
    ReferenceOperatorAdapter, ReferenceOperatorWriter, OPERATOR_KNOWLEDGE_CONTRACT_VERSION,
};
use rrd_store::{RrflowKvStore, RrflowMxStore, StorageEngine};
use rrd_vector::{
    EmbeddingModelBinding, ScoreMetric, SearchMode, SearchRequest, VectorCandidate, VectorQuery,
    VectorRuntime,
};
use std::collections::BTreeSet;

fn scope() -> ScopeId {
    ScopeId::new("instance:operator-trace").unwrap()
}

fn model() -> EmbeddingModelBinding {
    EmbeddingModelBinding {
        name: "operator-fixture-v1".into(),
        digest: "11".repeat(32),
    }
}

fn fixture<E: StorageEngine>(store: &E) -> Vec<VectorCandidate> {
    let mut registry = RuntimeSchemaRegistry::empty(1, "operator trace fixture");
    registry
        .define_record_table(
            RuntimeType::new("document").unwrap(),
            RuntimeLogicalModel::Relational,
            RuntimeRecordSchema {
                allow_additional_properties: true,
                ..RuntimeRecordSchema::default()
            },
        )
        .unwrap();
    registry.tables.insert(
        RuntimeType::new("embedding").unwrap(),
        RuntimeTableSchema::schemaless(RuntimeLogicalModel::Vector),
    );
    let records = (0..3)
        .map(|index| RuntimeRecord {
            reference: RuntimeRef::new("document", format!("doc-{index}")).unwrap(),
            valid_from: 1,
            valid_to: None,
            properties: RuntimeProperties::new(),
        })
        .collect::<Vec<_>>();
    let vectors = [
        vec![1.0, 0.0, 0.0],
        vec![0.0, 1.0, 0.0],
        vec![0.5, 0.5, 0.0],
    ]
    .into_iter()
    .enumerate()
    .map(|(index, values)| RuntimeVector {
        collection: VectorCollectionAddress {
            collection_id: "documents".into(),
            vector_name: "body".into(),
        },
        reference: RuntimeRef::new("embedding", format!("doc-{index}-body")).unwrap(),
        subject: records[index].reference.clone(),
        field: "body".into(),
        valid_from: 1,
        valid_to: None,
        value: VectorValue::Dense { values },
        provenance: Some(EmbeddingProvenance {
            source_digest: format!("{:02x}", index + 32).repeat(32),
            model: model().name,
            model_digest: model().digest,
            dimensions: 3,
            normalization: VectorNormalization::None,
            generation_parameters: RuntimeProperties::new(),
        }),
        properties: RuntimeProperties::new(),
    })
    .collect::<Vec<_>>();
    let mut mutations = vec![RuntimeMutation::Schema { registry }];
    mutations.extend(
        records
            .iter()
            .cloned()
            .map(|record| RuntimeMutation::Record { record }),
    );
    mutations.extend(
        vectors
            .iter()
            .cloned()
            .map(|vector| RuntimeMutation::Vector { vector }),
    );
    store
        .runtime()
        .commit(&RuntimeCommit {
            scope: scope(),
            at: 10,
            actor: "fixture".into(),
            expected_cursor: 0,
            mutations,
        })
        .unwrap();
    vectors
        .into_iter()
        .enumerate()
        .map(|(index, vector)| VectorCandidate {
            scope: scope(),
            source_cursor: 5 + index as u64,
            vector,
        })
        .collect()
}

fn canonical(value: &str) -> CanonicalId {
    CanonicalId::new(value).unwrap()
}

fn installed_identity() -> InstalledEngramIdentity {
    InstalledEngramIdentity {
        project_id: canonical("operator-trace-project"),
        engram_id: canonical("operator-trace-engram"),
        instance_id: canonical("operator-trace-instance"),
    }
}

fn knowledge(installed: &InstalledEngramIdentity) -> OperatorKnowledgeBinding {
    OperatorKnowledgeBinding {
        contract_version: OPERATOR_KNOWLEDGE_CONTRACT_VERSION,
        adapter: "pgvector".into(),
        project_id: installed.project_id.to_string(),
        member: ".".into(),
        scope: scope(),
        config_digest: "22".repeat(32),
        source_identity_digest: "33".repeat(32),
        relation_digest: "44".repeat(32),
        tenant_digest: "55".repeat(32),
        model: model(),
        dimensions: 3,
        projection: ProjectionStamp {
            contract_version: DATA_RUNTIME_CONTRACT_VERSION,
            id: rrd_core::ProjectionId::new("operator-pgvector-knowledge").unwrap(),
            generation: 1,
            source_cursor: 7,
            config_digest: "22".repeat(32),
            artifact_digest: "66".repeat(32),
            state: ProjectionState::Ready,
        },
    }
}

fn descriptor() -> OperatorAdapterDescriptor {
    OperatorAdapterDescriptor {
        contract_version: OPERATOR_KNOWLEDGE_CONTRACT_VERSION,
        adapter: "pgvector".into(),
        implementation_digest: "77".repeat(32),
        max_dimensions: 2_000,
        vector_kinds: BTreeSet::from([rrd_operator_knowledge::OperatorVectorKind::Dense]),
        search_capabilities: std::collections::BTreeMap::from([(
            OperatorAccessPath::Exact,
            BTreeSet::from([ScoreMetric::Dot]),
        )]),
        supports_tenant_filter: true,
        supports_payload_filter: false,
        supports_stable_revision: true,
    }
}

fn revision(project_id: &str, stable: &str) -> OperatorSourceRevision {
    OperatorSourceRevision {
        contract_version: OPERATOR_KNOWLEDGE_CONTRACT_VERSION,
        adapter: "pgvector".into(),
        project_id: project_id.into(),
        source_identity_digest: "33".repeat(32),
        snapshot_digest: "88".repeat(32),
        catalog_digest: "99".repeat(32),
        stable_revision: Some(stable.into()),
        wal_lsn_digest: Some("aa".repeat(32)),
    }
}

fn request<E: StorageEngine>(
    store: &E,
    knowledge: &OperatorKnowledgeBinding,
) -> OperatorSearchRequest {
    OperatorSearchRequest {
        contract_version: OPERATOR_KNOWLEDGE_CONTRACT_VERSION,
        binding_digest: knowledge.digest().unwrap(),
        required_source_cursor: knowledge.projection.source_cursor,
        search: SearchRequest {
            scope: scope(),
            read: store.runtime().read_stamp(&scope()).unwrap(),
            valid_at: 10,
            field: "body".into(),
            query: VectorQuery::Dense {
                values: vec![0.314_159_27, 0.271_828_18, 0.0],
            },
            metric: ScoreMetric::Dot,
            embedding_model: Some(model()),
            top_k: 2,
            mode: SearchMode::Exact,
            filter: None,
        },
        controls: OperatorSearchControls::exact(),
        expected_stable_revision: Some("project-revision-7".into()),
    }
}

fn adapter(
    knowledge: &OperatorKnowledgeBinding,
    candidates: Vec<VectorCandidate>,
    stable: &str,
) -> ReferenceOperatorAdapter {
    ReferenceOperatorAdapter::new(
        descriptor(),
        knowledge,
        revision(&knowledge.project_id, stable),
        VectorRuntime::new(candidates).unwrap(),
    )
    .unwrap()
}

#[derive(Debug, PartialEq, Eq)]
struct TraceView {
    name: String,
    phase: String,
    outcome: String,
    parent: Option<String>,
    attributes: RuntimeProperties,
    encoded: String,
}

fn traces<E: StorageEngine>(store: &E) -> Vec<TraceView> {
    store
        .runtime()
        .changes_since(0, usize::MAX, Some(&scope()))
        .unwrap()
        .changes
        .into_iter()
        .filter_map(|change| match change.mutation {
            RuntimeMutation::Event { event } if event.kind.as_str() == "runtime_trace" => {
                let string = |name: &str| match &event.properties[name] {
                    RuntimeValue::String(value) => value.clone(),
                    value => panic!("{name} had unexpected value {value:?}"),
                };
                let parent = event.properties.get("parent_span_id").map(|value| {
                    let RuntimeValue::String(value) = value else {
                        panic!("parent span had unexpected value {value:?}")
                    };
                    value.clone()
                });
                let RuntimeValue::Map(attributes) = &event.properties["attributes"] else {
                    panic!("trace attributes had the wrong shape")
                };
                Some(TraceView {
                    name: string("name"),
                    phase: string("phase"),
                    outcome: string("outcome"),
                    parent,
                    attributes: attributes.clone(),
                    encoded: serde_json::to_string(&event).unwrap(),
                })
            }
            _ => None,
        })
        .collect()
}

fn exercise<E: StorageEngine>(
    store: &E,
    installed: &InstalledEngramIdentity,
) -> (rrd_engine::TracedOperatorSearch, Vec<TraceView>) {
    let candidates = fixture(store);
    let knowledge = knowledge(installed);
    let request = request(store, &knowledge);
    let mut adapter = adapter(&knowledge, candidates, "project-revision-7");
    let result = execute_traced_operator_search(
        store,
        installed,
        &knowledge,
        &mut adapter,
        &request,
        "operator:test",
        100,
    )
    .unwrap();
    (result, traces(store))
}

#[test]
fn operator_search_is_project_bound_private_and_equal_across_engines() {
    let installed = installed_identity();
    let memory = RrflowMxStore::new();
    let rrflow_kv_root = tempfile::tempdir().unwrap();
    let rrflow_kv_path = rrflow_kv_root.path().join("rrflow-kv");
    let rrflow_kv = RrflowKvStore::open(&rrflow_kv_path).unwrap();

    let (memory_result, memory_traces) = exercise(&memory, &installed);
    let (rrflow_kv_result, rrflow_kv_traces) = exercise(&rrflow_kv, &installed);
    assert_eq!(memory_result, rrflow_kv_result);
    assert_eq!(memory_result.result.hits.len(), 2);
    assert_eq!(memory.runtime().cursor().unwrap(), 12);
    let normalize = |traces: &[TraceView]| {
        traces
            .iter()
            .map(|trace| {
                (
                    trace.name.clone(),
                    trace.phase.clone(),
                    trace.outcome.clone(),
                    trace.parent.clone(),
                    trace.attributes.clone(),
                )
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(normalize(&memory_traces), normalize(&rrflow_kv_traces));
    assert_eq!(
        memory_traces
            .iter()
            .map(|trace| (
                trace.name.as_str(),
                trace.phase.as_str(),
                trace.outcome.as_str()
            ))
            .collect::<Vec<_>>(),
        [
            ("operator.knowledge.search", "start", "running"),
            ("operator.knowledge.execute", "start", "running"),
            ("operator.knowledge.execute", "finish", "ok"),
            ("operator.knowledge.search", "finish", "ok"),
        ]
    );
    let root = memory_traces[0].parent.clone();
    assert!(root.is_none());
    assert!(memory_traces[1].parent.is_some());
    let encoded = memory_traces
        .iter()
        .map(|trace| trace.encoded.as_str())
        .collect::<String>();
    for secret in ["0.31415927", "0.27182818", "doc-0", "project-revision-7"] {
        assert!(!encoded.contains(secret), "trace leaked {secret}");
    }

    drop(rrflow_kv);
    let reopened = RrflowKvStore::open(&rrflow_kv_path).unwrap();
    assert_eq!(normalize(&traces(&reopened)), normalize(&rrflow_kv_traces));
}

#[test]
fn stale_projection_revision_and_foreign_project_are_durable_denials() {
    let installed = installed_identity();

    let stale_store = RrflowMxStore::new();
    let candidates = fixture(&stale_store);
    let stale_knowledge = knowledge(&installed);
    let stale_request = request(&stale_store, &stale_knowledge);
    let mut stale = adapter(&stale_knowledge, candidates, "project-revision-6");
    let error = execute_traced_operator_search(
        &stale_store,
        &installed,
        &stale_knowledge,
        &mut stale,
        &stale_request,
        "operator:test",
        100,
    )
    .unwrap_err();
    assert!(error.to_string().contains("stale"));
    assert_eq!(
        traces(&stale_store)
            .iter()
            .map(|trace| trace.outcome.as_str())
            .collect::<Vec<_>>(),
        ["running", "running", "denied", "denied"]
    );

    let projection_store = RrflowMxStore::new();
    let candidates = fixture(&projection_store);
    let mut projection_knowledge = knowledge(&installed);
    projection_knowledge.projection.source_cursor = 6;
    let mut projection_request = request(&projection_store, &projection_knowledge);
    projection_request.required_source_cursor = 7;
    let mut projection_adapter = adapter(&projection_knowledge, candidates, "project-revision-7");
    let error = execute_traced_operator_search(
        &projection_store,
        &installed,
        &projection_knowledge,
        &mut projection_adapter,
        &projection_request,
        "operator:test",
        100,
    )
    .unwrap_err();
    assert!(error.to_string().contains("projection is stale"));
    assert_eq!(
        traces(&projection_store)
            .iter()
            .map(|trace| trace.outcome.as_str())
            .collect::<Vec<_>>(),
        ["running", "running", "denied", "denied"]
    );

    let foreign_store = RrflowMxStore::new();
    let candidates = fixture(&foreign_store);
    let mut foreign = knowledge(&installed);
    foreign.project_id = "another-project".into();
    let request = request(&foreign_store, &foreign);
    let mut adapter = adapter(&foreign, candidates, "project-revision-7");
    let error = execute_traced_operator_search(
        &foreign_store,
        &installed,
        &foreign,
        &mut adapter,
        &request,
        "operator:test",
        100,
    )
    .unwrap_err();
    assert!(error.to_string().contains("not installed project"));
    assert_eq!(traces(&foreign_store).len(), 2);
    assert_eq!(traces(&foreign_store)[1].outcome, "denied");
}

#[test]
fn raw_query_identity_is_digest_only() {
    let query = serde_json::to_vec(&VectorQuery::Dense {
        values: vec![0.314_159_27, 0.271_828_18, 0.0],
    })
    .unwrap();
    assert_eq!(digest::sha256_hex(&query).len(), 64);
}

#[test]
fn traced_outbox_retry_applies_external_payload_once() {
    let installed = installed_identity();
    let store = RrflowMxStore::new();
    let candidates = fixture(&store);
    let knowledge = knowledge(&installed);
    let source = store
        .runtime()
        .outbox_since(0, 100)
        .unwrap()
        .into_iter()
        .find(|work| work.source_cursor == 7)
        .unwrap();
    let source_change = store
        .runtime()
        .changes_since(6, 1, Some(&scope()))
        .unwrap()
        .changes
        .pop()
        .unwrap();
    let payload = serde_json::to_vec(&candidates[2].vector).unwrap();
    let work = OperatorSyncWork::for_vector(
        &knowledge,
        &source,
        source_change.digest,
        digest::sha256_hex(&payload),
    )
    .unwrap();
    let mut writer = ReferenceOperatorWriter::new(
        descriptor(),
        &knowledge,
        revision(&knowledge.project_id, "project-revision-8"),
    )
    .unwrap();

    let first = rrd_engine::execute_traced_operator_sync(
        &store,
        &installed,
        &knowledge,
        &mut writer,
        &work,
        &payload,
        "operator:test",
        200,
    )
    .unwrap();
    let retry = rrd_engine::execute_traced_operator_sync(
        &store,
        &installed,
        &knowledge,
        &mut writer,
        &work,
        &payload,
        "operator:test",
        300,
    )
    .unwrap();
    assert!(first.receipt.applied_now);
    assert!(retry.receipt.idempotent_replay);
    assert_eq!(writer.apply_count(), 1);
    assert_eq!(store.runtime().cursor().unwrap(), 16);
    let traces = traces(&store);
    assert_eq!(
        traces
            .iter()
            .map(|trace| (trace.name.as_str(), trace.outcome.as_str()))
            .collect::<Vec<_>>(),
        [
            ("operator.knowledge.sync", "running"),
            ("operator.knowledge.apply", "running"),
            ("operator.knowledge.apply", "ok"),
            ("operator.knowledge.sync", "ok"),
            ("operator.knowledge.sync", "running"),
            ("operator.knowledge.apply", "running"),
            ("operator.knowledge.apply", "ok"),
            ("operator.knowledge.sync", "ok"),
        ]
    );
    let encoded = traces
        .iter()
        .map(|trace| trace.encoded.as_str())
        .collect::<String>();
    assert!(!encoded.contains("doc-2"));
    assert!(encoded.contains("upsert_vector"));
    assert!(traces[2].attributes.contains_key("applied_now"));
    assert_eq!(
        traces[6].attributes.get("idempotent_replay"),
        Some(&RuntimeValue::Bool(true))
    );
}
