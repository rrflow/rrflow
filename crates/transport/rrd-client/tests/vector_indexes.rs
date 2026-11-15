//! POAM-053: a client of a real server builds a graph index and declares a
//! payload index, and a filtered search uses that generation.
use rrd_client::{ClientConfig, RequestOptions, RrdClient};
use rrd_contract::*;
use rrd_core::digest;
use rrd_engine::RrdEngine;
use rrd_security::{
    Action, Principal, PrincipalKind, ResourceGrant, SecurityRepository, SecurityState,
    SECURITY_FORMAT,
};
use rrd_server::RrdHttpServer;
use rrd_store::RrflowKvStore;
use std::collections::BTreeMap;

const DIMENSIONS: usize = 8;
const POINTS: usize = 64;

fn mutation(id: &str) -> RequestOptions {
    RequestOptions::mutation(id, id, id).unwrap()
}
fn read(id: &str) -> RequestOptions {
    RequestOptions::read(id, id).unwrap()
}
fn denied(error: rrd_client::Error) {
    assert!(
        matches!(&error, rrd_client::Error::Api { error, .. } if error.code == ErrorCode::PermissionDenied),
        "{error:?}"
    );
}
fn canonical(value: &str) -> CanonicalId {
    CanonicalId::new(value).unwrap()
}
fn data_reference(kind: &str, id: &str) -> DataReference {
    DataReference {
        kind: canonical(kind),
        id: canonical(id),
    }
}
fn values(seed: usize) -> Vec<f32> {
    (0..DIMENSIONS)
        .map(|dimension| ((seed * 31 + dimension * 7) % 97) as f32 / 48.0 - 1.0)
        .collect()
}

fn schema() -> DataSchemaRegistry {
    let table = |model| DataTableSchema {
        model,
        mode: DataSchemaMode::Schemaless,
        properties: BTreeMap::new(),
        allow_additional_properties: false,
    };
    DataSchemaRegistry {
        revision: 1,
        migration: "install vector index fixture".into(),
        organisation: DataEngramOrganisation::default(),
        tables: BTreeMap::from([
            (canonical("document"), table(DataLogicalModel::Document)),
            (canonical("embedding"), table(DataLogicalModel::Vector)),
        ]),
        records: BTreeMap::new(),
        relations: BTreeMap::new(),
        events: BTreeMap::new(),
    }
}

fn points() -> Vec<TransactionMutation> {
    let mut mutations = vec![TransactionMutation::PutSchema { registry: schema() }];
    for point in 0..POINTS {
        let id = format!("p{point}");
        mutations.push(TransactionMutation::PutRecord {
            reference: data_reference("document", &id),
            valid_from: 1,
            valid_to: None,
            properties: DataProperties::new(),
        });
        mutations.push(TransactionMutation::PutVector {
            reference: data_reference("embedding", &id),
            subject: data_reference("document", &id),
            collection_id: canonical("documents"),
            vector_name: canonical("body"),
            field: canonical("body-embedding"),
            valid_from: 1,
            valid_to: None,
            value: DataVectorValue::Dense {
                values: values(point),
            },
            provenance: None,
            properties: DataProperties::from([(
                "bucket".into(),
                QueryValue::Unsigned((point % 4) as u64),
            )]),
        });
    }
    mutations
}

fn principal(name: &str, actions: Vec<Action>, resource: &ResourcePath) -> Principal {
    Principal {
        id: canonical(name),
        kind: PrincipalKind::Service,
        credential_sha256: digest::sha256_hex(name.as_bytes()),
        credential_revision: 1,
        not_before_unix_ms: 1,
        expires_at_unix_ms: u64::MAX,
        disabled: false,
        role_ids: Default::default(),
        grants: actions
            .into_iter()
            .map(|action| ResourceGrant {
                action,
                resource_prefix: resource.clone(),
                data_policy: None,
            })
            .collect(),
    }
}

#[tokio::test]
async fn a_client_builds_an_index_and_a_filtered_search_uses_it() {
    let root = tempfile::tempdir().unwrap();
    let instance = canonical("indexes");
    let scope = format!("instance:{instance}");
    let storage = RrflowKvStore::open(root.path()).unwrap();
    let resource = ResourcePath {
        segments: vec![ResourceId::new(ResourceKind::Instance, "indexes").unwrap()],
    };
    let builder = principal(
        "builder",
        vec![
            Action::SessionCreate,
            Action::TransactionBegin,
            Action::TransactionCommit,
            Action::VectorCollectionEnsure,
            Action::VectorPayloadIndexEnsure,
            Action::VectorPayloadIndexDelete,
            Action::VectorSearch,
            Action::ChangefeedRead,
        ],
        &resource,
    );
    let searcher = principal(
        "searcher",
        vec![Action::SessionCreate, Action::VectorSearch],
        &resource,
    );
    SecurityRepository::new(&storage, instance.clone())
        .initialize(
            SecurityState {
                format_version: SECURITY_FORMAT,
                revision: 1,
                principals: BTreeMap::from([
                    (builder.id.clone(), builder),
                    (searcher.id.clone(), searcher),
                ]),
                roles: BTreeMap::new(),
                identity_bindings: BTreeMap::new(),
                jwt_issuers: BTreeMap::new(),
            },
            1,
            "bootstrap",
            "bootstrap",
            "bootstrap",
        )
        .unwrap();
    drop(storage);
    let engine = RrdEngine::open(root.path(), instance.clone(), [0x6b; 32]).unwrap();
    let server = RrdHttpServer::bind(engine, "127.0.0.1:0".parse().unwrap()).unwrap();
    let client = RrdClient::connect_local(
        server.local_addr(),
        instance.clone(),
        ClientConfig::default(),
    )
    .unwrap();
    let (shutdown, receiver) = tokio::sync::oneshot::channel();
    let running = tokio::spawn(server.serve_until(async move {
        let _ = receiver.await;
    }));
    let limits = || CreateSession {
        limits: SessionLimits {
            idle_timeout_ms: 60_000,
            absolute_timeout_ms: 300_000,
            max_open_transactions: 1,
        },
    };
    let session = client
        .create_session(
            canonical("builder"),
            "builder",
            limits(),
            mutation("session-builder"),
        )
        .await
        .unwrap();
    let reader = client
        .create_session(
            canonical("searcher"),
            "searcher",
            limits(),
            mutation("session-searcher"),
        )
        .await
        .unwrap();

    client
        .ensure_vector_collection(
            &session,
            EnsureVectorCollection {
                scope: scope.clone(),
                collection_id: canonical("documents"),
                vectors: vec![NamedVectorDefinition {
                    name: canonical("body"),
                    field: canonical("body-embedding"),
                    kind: VectorValueKind::Dense,
                    dimensions: DIMENSIONS as u32,
                    metric: VectorSearchMetric::Cosine,
                    embedding_model: None,
                    memory_tier: VectorMemoryTier::Cached,
                }],
            },
            mutation("collection"),
        )
        .await
        .unwrap();

    let payload_index = |field: &str| EnsureVectorPayloadIndex {
        scope: scope.clone(),
        collection_id: canonical("documents"),
        field: canonical(field),
        kind: VectorPayloadIndexKind::Unsigned,
    };
    let declared = client
        .ensure_vector_payload_index(&session, payload_index("bucket"), mutation("payload-index"))
        .await
        .unwrap();
    assert_eq!(declared.index.field, canonical("bucket"));
    assert!(!declared.idempotent_replay);
    let replayed = client
        .ensure_vector_payload_index(&session, payload_index("bucket"), mutation("payload-index"))
        .await
        .unwrap();
    assert!(replayed.idempotent_replay);

    let transaction = client
        .begin_transaction(
            &session,
            BeginTransaction {
                scope: canonical("data"),
                timeout_ms: 30_000,
            },
            mutation("begin"),
        )
        .await
        .unwrap();
    let mutations = points();
    client
        .commit_transaction(
            &session,
            &transaction.transaction_id,
            CommitTransaction {
                operation_sha256: transaction_operation_sha256(&mutations),
                mutations,
            },
            RequestOptions::new(
                "commit",
                "commit",
                Some("commit"),
                Some(
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as u64
                        + 60_000,
                ),
            )
            .unwrap(),
        )
        .await
        .unwrap();

    let index_request = EnsureVectorIndex {
        scope: scope.clone(),
        collection_id: canonical("documents"),
        vector_name: canonical("body"),
        configuration: VectorIndexConfiguration::Hnsw {
            m: 8,
            ef_construction: 64,
            max_level: 6,
            seed: 17,
            filter_properties: vec![canonical("bucket")],
        },
        build_policy: VectorIndexBuildPolicy::Cpu,
        max_storage_keys: 100_000,
    };
    let built = client
        .ensure_vector_index(&session, index_request.clone(), mutation("index-build"))
        .await
        .unwrap();
    assert_eq!(built.index.kind, canonical("hnsw"));
    assert_eq!(built.index.generation, 1);
    assert_eq!(built.index.indexed_vectors, POINTS as u64);
    assert!(!built.idempotent_replay);
    let retried = client
        .ensure_vector_index(&session, index_request.clone(), mutation("index-build"))
        .await
        .unwrap();
    assert!(retried.idempotent_replay);
    assert_eq!(retried.index.generation, 1);

    let search = |mode| SearchVectors {
        scope: scope.clone(),
        valid_at: 2,
        collection_id: Some(canonical("documents")),
        vector_name: Some(canonical("body")),
        field: None,
        query: VectorSearchQuery::Dense { values: values(9) },
        filter: Some(VectorPayloadFilter::Condition {
            condition: VectorPayloadCondition {
                property: canonical("bucket"),
                operator: VectorPayloadOperator::Equals {
                    value: QueryValue::Unsigned(2),
                },
            },
        }),
        metric: None,
        top_k: 5,
        mode,
        max_storage_keys: 100_000,
    };
    let approximate = client
        .search_vectors(
            &reader,
            search(VectorSearchMode::RequireApproximate {
                exact_rerank: 16,
                ef_search: 32,
            }),
            read("search-approximate"),
        )
        .await
        .unwrap();
    assert_eq!(approximate.access_path, canonical("hnsw"));
    let exact = client
        .search_vectors(
            &reader,
            search(VectorSearchMode::Exact),
            read("search-exact"),
        )
        .await
        .unwrap();
    let neighbors = |result: &VectorSearchResult| {
        result
            .hits
            .iter()
            .map(|hit| hit.reference.id.to_string())
            .collect::<Vec<_>>()
    };
    assert_eq!(exact.hits.len(), 5);
    assert_eq!(neighbors(&approximate), neighbors(&exact));

    // The scope's changefeed still reads, and it shows the index it now holds.
    let page = client
        .read_changefeed(
            &session,
            ReadChangefeed {
                scope: scope.clone(),
                after_cursor: 0,
                limit: 1_000,
            },
            read("changefeed"),
        )
        .await
        .unwrap();
    assert!(page.changes.iter().any(|change| matches!(
        &change.mutation,
        ChangeMutationSnapshot::Data {
            mutation: TransactionMutation::PutRecord { reference, .. },
        } if reference.kind.as_str() == "vector_artifact"
            && reference.id.as_str() == format!("{}.1", built.index.index_id)
    )));

    // A generation that filters on the payload index protects it, and an
    // unrelated index deletes.
    let delete = |field: &str| DeleteVectorPayloadIndex {
        scope: scope.clone(),
        collection_id: canonical("documents"),
        field: canonical(field),
    };
    assert!(client
        .delete_vector_payload_index(&session, delete("bucket"), mutation("delete-bucket"))
        .await
        .is_err());
    client
        .ensure_vector_payload_index(&session, payload_index("spare"), mutation("spare-index"))
        .await
        .unwrap();
    let deleted = client
        .delete_vector_payload_index(&session, delete("spare"), mutation("delete-spare"))
        .await
        .unwrap();
    assert_eq!(deleted.deleted_index.field, canonical("spare"));

    // Building and declaring take their grants.
    denied(
        client
            .ensure_vector_index(&reader, index_request, mutation("denied-build"))
            .await
            .unwrap_err(),
    );
    denied(
        client
            .ensure_vector_payload_index(&reader, payload_index("other"), mutation("denied-ensure"))
            .await
            .unwrap_err(),
    );
    denied(
        client
            .delete_vector_payload_index(&reader, delete("bucket"), mutation("denied-delete"))
            .await
            .unwrap_err(),
    );

    shutdown.send(()).unwrap();
    running.await.unwrap().unwrap();
}
