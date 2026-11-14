//! Collection overlays, retirement, point and payload index administration.

use super::*;

#[test]
pub(super) fn collection_filtered_hnsw_overlays_retirement_and_protects_payload_index() {
    let root = tempfile::tempdir().unwrap();
    let engine = RrdEngine::open(root.path(), instance(), TOKEN_KEY).unwrap();
    let lease = engine
        .create_session(
            &session_request(9_000, 8),
            &id("online-filter-session"),
            100,
            "request-session",
            "operation-session",
        )
        .unwrap();
    let scope = format!("instance:{}", instance());
    engine
        .ensure_vector_collection(
            &lease.session_id,
            &lease.token,
            &EnsureVectorCollection {
                scope: scope.clone(),
                collection_id: CanonicalId::new("filtered-documents").unwrap(),
                vectors: vec![NamedVectorDefinition {
                    name: CanonicalId::new("body").unwrap(),
                    field: CanonicalId::new("filtered-body").unwrap(),
                    kind: VectorValueKind::Dense,
                    dimensions: 2,
                    metric: VectorSearchMetric::Dot,
                    embedding_model: None,
                    memory_tier: VectorMemoryTier::Cached,
                }],
            },
            &mutation_context(
                &id("online-filter-collection"),
                "request-collection",
                "operation-collection",
            ),
            200,
        )
        .unwrap();
    engine
        .ensure_vector_payload_index(
            &lease.session_id,
            &lease.token,
            &EnsureVectorPayloadIndex {
                scope: scope.clone(),
                collection_id: CanonicalId::new("filtered-documents").unwrap(),
                field: CanonicalId::new("bucket").unwrap(),
                kind: VectorPayloadIndexKind::Unsigned,
            },
            &mutation_context(
                &id("online-filter-payload-index"),
                "request-payload-index",
                "operation-payload-index",
            ),
            210,
        )
        .unwrap();
    let vector = |id: &str, bucket: u64, values: Vec<f32>| TransactionMutation::PutVector {
        reference: reference("embedding", id),
        subject: reference("document", id),
        collection_id: CanonicalId::new("filtered-documents").unwrap(),
        vector_name: CanonicalId::new("body").unwrap(),
        field: CanonicalId::new("filtered-body").unwrap(),
        valid_from: 1,
        valid_to: None,
        value: DataVectorValue::Dense { values },
        provenance: None,
        properties: DataProperties::from([("bucket".into(), QueryValue::Unsigned(bucket))]),
    };
    commit_vectors(
        &engine,
        &lease,
        300,
        vec![
            TransactionMutation::PutSchema {
                registry: DataSchemaRegistry {
                    revision: 1,
                    migration: "install filtered HNSW schema".into(),
                    organisation: DataEstateOrganisation::default(),
                    tables: BTreeMap::from([
                        (
                            CanonicalId::new("embedding").unwrap(),
                            DataTableSchema {
                                model: DataLogicalModel::Vector,
                                mode: DataSchemaMode::Schemaless,
                                properties: BTreeMap::new(),
                                allow_additional_properties: false,
                            },
                        ),
                        (
                            CanonicalId::new("document").unwrap(),
                            DataTableSchema {
                                model: DataLogicalModel::Document,
                                mode: DataSchemaMode::Schemaless,
                                properties: BTreeMap::new(),
                                allow_additional_properties: false,
                            },
                        ),
                        (
                            CanonicalId::new("object").unwrap(),
                            DataTableSchema {
                                model: DataLogicalModel::Object,
                                mode: DataSchemaMode::Schemaless,
                                properties: BTreeMap::new(),
                                allow_additional_properties: false,
                            },
                        ),
                    ]),
                    records: BTreeMap::new(),
                    relations: BTreeMap::new(),
                    events: BTreeMap::new(),
                },
            },
            TransactionMutation::PutRecord {
                reference: reference("document", "alpha"),
                valid_from: 1,
                valid_to: None,
                properties: DataProperties::new(),
            },
            TransactionMutation::PutRecord {
                reference: reference("document", "beta"),
                valid_from: 1,
                valid_to: None,
                properties: DataProperties::new(),
            },
            vector("alpha", 1, vec![1.0, 0.0]),
            vector("beta", 2, vec![0.9, 0.1]),
        ],
    );
    let index = EnsureVectorIndex {
        scope: scope.clone(),
        collection_id: CanonicalId::new("filtered-documents").unwrap(),
        vector_name: CanonicalId::new("body").unwrap(),
        configuration: VectorIndexConfiguration::Hnsw {
            m: 4,
            ef_construction: 8,
            max_level: 4,
            seed: 47,
            filter_properties: vec![CanonicalId::new("bucket").unwrap()],
        },
        build_policy: VectorIndexBuildPolicy::Cpu,
        max_storage_keys: 10_000,
    };
    engine
        .ensure_vector_index(
            &lease.session_id,
            &lease.token,
            &index,
            &mutation_context(
                &id("operation-filter-index"),
                "request-filter-index",
                "operation-filter-index",
            ),
            400,
        )
        .unwrap();
    let search = SearchVectors {
        scope: scope.clone(),
        valid_at: 2,
        collection_id: Some(CanonicalId::new("filtered-documents").unwrap()),
        vector_name: Some(CanonicalId::new("body").unwrap()),
        field: None,
        query: VectorSearchQuery::Dense {
            values: vec![1.0, 0.0],
        },
        filter: Some(VectorPayloadFilter::Condition {
            condition: VectorPayloadCondition {
                property: CanonicalId::new("bucket").unwrap(),
                operator: VectorPayloadOperator::Equals {
                    value: QueryValue::Unsigned(2),
                },
            },
        }),
        metric: None,
        top_k: 1,
        mode: VectorSearchMode::RequireApproximate {
            exact_rerank: 2,
            ef_search: 2,
        },
        max_storage_keys: 10_000,
    };
    let filtered = engine
        .search_vectors(
            &lease.session_id,
            &lease.token,
            &search,
            500,
            "request-filter-search",
            "operation-filter-search",
        )
        .unwrap();
    assert_eq!(filtered.access_path.as_str(), "hnsw");
    assert_eq!(filtered.hits[0].reference.id.as_str(), "beta");

    commit_vectors(
        &engine,
        &lease,
        600,
        vec![TransactionMutation::RetireData {
            model: DataLogicalModel::Vector,
            target: DataTarget::Reference {
                reference: reference("embedding", "beta"),
            },
            effective_at: 2,
        }],
    );
    let immediate = engine
        .search_vectors(
            &lease.session_id,
            &lease.token,
            &search,
            700,
            "request-retired-overlay",
            "operation-retired-overlay",
        )
        .unwrap();
    assert_eq!(immediate.access_path.as_str(), "hnsw");
    assert!(immediate.hits.is_empty());
    assert!(engine
        .delete_vector_payload_index(
            &lease.session_id,
            &lease.token,
            &DeleteVectorPayloadIndex {
                scope: scope.clone(),
                collection_id: CanonicalId::new("filtered-documents").unwrap(),
                field: CanonicalId::new("bucket").unwrap(),
            },
            &mutation_context(
                &id("delete-active-filter-index"),
                "request-delete-active-filter-index",
                "operation-delete-active-filter-index",
            ),
            701,
        )
        .is_err());
    let maintained = engine
        .ensure_vector_index(
            &lease.session_id,
            &lease.token,
            &index,
            &mutation_context(
                &id("operation-filter-index-incremental"),
                "request-filter-index-incremental",
                "operation-filter-index-incremental",
            ),
            800,
        )
        .unwrap();
    assert_eq!(
        maintained.index.maintenance.mode,
        VectorIndexMaintenanceMode::Incremental
    );
    assert_eq!(maintained.index.maintenance.indexed_delta_vectors, 1);
    drop(engine);

    let reopened = RrdEngine::open(root.path(), instance(), TOKEN_KEY).unwrap();
    let after_reopen = reopened
        .search_vectors(
            &lease.session_id,
            &lease.token,
            &search,
            900,
            "request-filter-search-reopen",
            "operation-filter-search-reopen",
        )
        .unwrap();
    assert_eq!(after_reopen.access_path.as_str(), "hnsw");
    assert!(after_reopen.hits.is_empty());
}

#[test]
pub(super) fn collection_point_and_payload_index_administration_is_atomic_and_restart_safe() {
    let root = tempfile::tempdir().unwrap();
    let engine = RrdEngine::open(root.path(), instance(), TOKEN_KEY).unwrap();
    let lease = engine
        .create_session(
            &session_request(9_000, 8),
            &id("vector-admin-session"),
            100,
            "request-session",
            "operation-session",
        )
        .unwrap();
    let collection_request = EnsureVectorCollection {
        scope: format!("instance:{}", instance()),
        collection_id: CanonicalId::new("multimodal").unwrap(),
        vectors: vec![
            NamedVectorDefinition {
                name: CanonicalId::new("dense").unwrap(),
                field: CanonicalId::new("dense-embedding").unwrap(),
                kind: VectorValueKind::Dense,
                dimensions: 3,
                metric: VectorSearchMetric::Cosine,
                embedding_model: None,
                memory_tier: VectorMemoryTier::Cached,
            },
            NamedVectorDefinition {
                name: CanonicalId::new("sparse").unwrap(),
                field: CanonicalId::new("sparse-embedding").unwrap(),
                kind: VectorValueKind::Sparse,
                dimensions: 8,
                metric: VectorSearchMetric::Dot,
                embedding_model: None,
                memory_tier: VectorMemoryTier::Cold,
            },
            NamedVectorDefinition {
                name: CanonicalId::new("late").unwrap(),
                field: CanonicalId::new("late-embedding").unwrap(),
                kind: VectorValueKind::MultiDense,
                dimensions: 2,
                metric: VectorSearchMetric::Dot,
                embedding_model: None,
                memory_tier: VectorMemoryTier::Pinned,
            },
        ],
    };
    engine
        .ensure_vector_collection(
            &lease.session_id,
            &lease.token,
            &collection_request,
            &mutation_context(
                &id("ensure-multimodal"),
                "request-collection",
                "operation-collection",
            ),
            200,
        )
        .unwrap();
    for (ordinal, (field, kind)) in [
        ("tenant", VectorPayloadIndexKind::Keyword),
        ("priority", VectorPayloadIndexKind::Unsigned),
    ]
    .into_iter()
    .enumerate()
    {
        engine
            .ensure_vector_payload_index(
                &lease.session_id,
                &lease.token,
                &EnsureVectorPayloadIndex {
                    scope: collection_request.scope.clone(),
                    collection_id: collection_request.collection_id.clone(),
                    field: CanonicalId::new(field).unwrap(),
                    kind,
                },
                &mutation_context(
                    &id(&format!("payload-index-{field}")),
                    &format!("request-payload-index-{ordinal}"),
                    &format!("operation-payload-index-{ordinal}"),
                ),
                210 + ordinal as u64,
            )
            .unwrap();
    }
    let indexes = engine
        .list_vector_payload_indexes(
            &lease.session_id,
            &lease.token,
            &ListVectorPayloadIndexes {
                scope: collection_request.scope.clone(),
                collection_id: collection_request.collection_id.clone(),
            },
            220,
            "request-list-payload-indexes",
            "operation-list-payload-indexes",
        )
        .unwrap();
    assert_eq!(indexes.indexes.len(), 2);

    let payload = DataProperties::from([
        ("tenant".into(), QueryValue::String("alpha".into())),
        ("priority".into(), QueryValue::Unsigned(7)),
    ]);
    let subject = reference("document", "alpha");
    let dense_reference = reference("embedding", "alpha-dense");
    let sparse_reference = reference("embedding", "alpha-sparse");
    let late_reference = reference("embedding", "alpha-late");
    let point = |reference: DataReference,
                 vector_name: &str,
                 field: &str,
                 value: DataVectorValue| TransactionMutation::PutVector {
        reference,
        subject: subject.clone(),
        collection_id: collection_request.collection_id.clone(),
        vector_name: CanonicalId::new(vector_name).unwrap(),
        field: CanonicalId::new(field).unwrap(),
        valid_from: 1,
        valid_to: None,
        value,
        provenance: None,
        properties: payload.clone(),
    };
    commit_vectors(
        &engine,
        &lease,
        300,
        vec![
            TransactionMutation::PutSchema {
                registry: DataSchemaRegistry {
                    revision: 1,
                    migration: "install multimodal point schema".into(),
                    organisation: DataEstateOrganisation::default(),
                    tables: BTreeMap::from([
                        (
                            CanonicalId::new("embedding").unwrap(),
                            DataTableSchema {
                                model: DataLogicalModel::Vector,
                                mode: DataSchemaMode::Schemaless,
                                properties: BTreeMap::new(),
                                allow_additional_properties: false,
                            },
                        ),
                        (
                            CanonicalId::new("document").unwrap(),
                            DataTableSchema {
                                model: DataLogicalModel::Document,
                                mode: DataSchemaMode::Schemaless,
                                properties: BTreeMap::new(),
                                allow_additional_properties: false,
                            },
                        ),
                    ]),
                    records: BTreeMap::new(),
                    relations: BTreeMap::new(),
                    events: BTreeMap::new(),
                },
            },
            TransactionMutation::PutRecord {
                reference: subject.clone(),
                valid_from: 1,
                valid_to: None,
                properties: DataProperties::new(),
            },
            point(
                dense_reference.clone(),
                "dense",
                "dense-embedding",
                DataVectorValue::Dense {
                    values: vec![1.0, 0.0, 0.0],
                },
            ),
            point(
                sparse_reference.clone(),
                "sparse",
                "sparse-embedding",
                DataVectorValue::Sparse {
                    dimensions: 8,
                    indices: vec![1, 6],
                    values: vec![0.5, 0.75],
                },
            ),
            point(
                late_reference.clone(),
                "late",
                "late-embedding",
                DataVectorValue::MultiDense {
                    dimensions: 2,
                    vectors: vec![vec![1.0, 0.0], vec![0.0, 1.0]],
                },
            ),
        ],
    );

    let invalid_transaction = engine
        .begin_transaction(
            &lease.session_id,
            &lease.token,
            &BeginTransaction {
                scope: CanonicalId::new("data").unwrap(),
                timeout_ms: 1_000,
            },
            &mutation_context(
                &id("begin-invalid-payload"),
                "request-begin-invalid-payload",
                "operation-begin-invalid-payload",
            ),
            350,
        )
        .unwrap();
    let invalid_mutations = vec![TransactionMutation::PutVector {
        reference: reference("embedding", "invalid-payload"),
        subject: subject.clone(),
        collection_id: collection_request.collection_id.clone(),
        vector_name: CanonicalId::new("dense").unwrap(),
        field: CanonicalId::new("dense-embedding").unwrap(),
        valid_from: 1,
        valid_to: None,
        value: DataVectorValue::Dense {
            values: vec![1.0, 0.0, 0.0],
        },
        provenance: None,
        properties: DataProperties::from([
            ("tenant".into(), QueryValue::String("alpha".into())),
            ("priority".into(), QueryValue::String("not-unsigned".into())),
        ]),
    }];
    let invalid_request = CommitTransaction {
        operation_sha256: rrd_contract::transaction_operation_sha256(&invalid_mutations),
        mutations: invalid_mutations,
    };
    let invalid = engine.commit_transaction(
        &lease.session_id,
        &lease.token,
        &invalid_transaction.transaction_id,
        &id("commit-invalid-payload"),
        &invalid_request,
        351,
        "request-commit-invalid-payload",
        "operation-commit-invalid-payload",
    );
    assert!(matches!(invalid, Err(ServiceError::Vector(_))));

    let live_delete = engine.delete_vector_collection(
        &lease.session_id,
        &lease.token,
        &DeleteVectorCollection {
            scope: collection_request.scope.clone(),
            collection_id: collection_request.collection_id.clone(),
            valid_at: 1,
            max_storage_keys: 10_000,
        },
        &mutation_context(
            &id("delete-live-collection"),
            "request-delete-live",
            "operation-delete-live",
        ),
        400,
    );
    assert!(matches!(live_delete, Err(ServiceError::Vector(_))));

    drop(engine);
    let engine = RrdEngine::open(root.path(), instance(), TOKEN_KEY).unwrap();
    let reopened_indexes = engine
        .list_vector_payload_indexes(
            &lease.session_id,
            &lease.token,
            &ListVectorPayloadIndexes {
                scope: collection_request.scope.clone(),
                collection_id: collection_request.collection_id.clone(),
            },
            500,
            "request-list-reopened-indexes",
            "operation-list-reopened-indexes",
        )
        .unwrap();
    assert_eq!(reopened_indexes.indexes, indexes.indexes);
    for (vector_name, reference) in [
        ("dense", dense_reference.clone()),
        ("sparse", sparse_reference.clone()),
        ("late", late_reference.clone()),
    ] {
        let retrieved = engine
            .retrieve_vector_points(
                &lease.session_id,
                &lease.token,
                &RetrieveVectorPoints {
                    scope: collection_request.scope.clone(),
                    collection_id: collection_request.collection_id.clone(),
                    vector_name: CanonicalId::new(vector_name).unwrap(),
                    valid_at: 1,
                    references: vec![reference],
                    max_storage_keys: 10_000,
                },
                510,
                "request-retrieve-reopened",
                "operation-retrieve-reopened",
            )
            .unwrap();
        assert_eq!(retrieved.points.len(), 1);
        assert_eq!(retrieved.points[0].payload, payload);
        assert_direct_read_evidence(
            retrieved.selected_versions,
            &retrieved.read_evidence,
            &[ReadAccessPath::VectorVersions],
        );
    }

    commit_vectors(
        &engine,
        &lease,
        600,
        [dense_reference, sparse_reference, late_reference]
            .into_iter()
            .map(|reference| TransactionMutation::RetireData {
                model: DataLogicalModel::Vector,
                target: DataTarget::Reference { reference },
                effective_at: 2,
            })
            .collect(),
    );
    let deleted_index = engine
        .delete_vector_payload_index(
            &lease.session_id,
            &lease.token,
            &DeleteVectorPayloadIndex {
                scope: collection_request.scope.clone(),
                collection_id: collection_request.collection_id.clone(),
                field: CanonicalId::new("tenant").unwrap(),
            },
            &mutation_context(
                &id("delete-tenant-index"),
                "request-delete-tenant-index",
                "operation-delete-tenant-index",
            ),
            700,
        )
        .unwrap();
    assert_eq!(deleted_index.deleted_index.field.as_str(), "tenant");
    assert_eq!(deleted_index.collection.payload_indexes.len(), 1);

    let delete_request = DeleteVectorCollection {
        scope: collection_request.scope.clone(),
        collection_id: collection_request.collection_id.clone(),
        valid_at: 2,
        max_storage_keys: 10_000,
    };
    let delete_context = mutation_context(
        &id("delete-empty-collection"),
        "request-delete-empty",
        "operation-delete-empty",
    );
    let deleted = engine
        .delete_vector_collection(
            &lease.session_id,
            &lease.token,
            &delete_request,
            &delete_context,
            800,
        )
        .unwrap();
    assert!(!deleted.idempotent_replay);
    drop(engine);

    let engine = RrdEngine::open(root.path(), instance(), TOKEN_KEY).unwrap();
    let replay = engine
        .delete_vector_collection(
            &lease.session_id,
            &lease.token,
            &delete_request,
            &delete_context,
            900,
        )
        .unwrap();
    assert!(replay.idempotent_replay);
    let collections = engine
        .list_vector_collections(
            &lease.session_id,
            &lease.token,
            &ListVectorCollections {
                scope: collection_request.scope,
            },
            901,
            "request-list-deleted",
            "operation-list-deleted",
        )
        .unwrap();
    assert!(collections.collections.is_empty());
}
