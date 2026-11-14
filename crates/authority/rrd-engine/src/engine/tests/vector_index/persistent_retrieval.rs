//! Persistent retrieval indexes and hybrid fusion across reopen and staleness.

use super::*;

#[test]
pub(super) fn persistent_retrieval_indexes_and_hybrid_fusion_survive_reopen_and_staleness() {
    let root = tempfile::tempdir().unwrap();
    let engine = RrdEngine::open(root.path(), instance(), TOKEN_KEY).unwrap();
    let lease = engine
        .create_session(
            &session_request(9_000, 4),
            &id("vector-session"),
            100,
            "request-session",
            "operation-session",
        )
        .unwrap();
    engine
        .ensure_vector_collection(
            &lease.session_id,
            &lease.token,
            &EnsureVectorCollection {
                scope: format!("instance:{}", instance()),
                collection_id: CanonicalId::new("documents").unwrap(),
                vectors: vec![NamedVectorDefinition {
                    name: CanonicalId::new("body").unwrap(),
                    field: CanonicalId::new("body-embedding").unwrap(),
                    kind: VectorValueKind::Dense,
                    dimensions: 2,
                    metric: VectorSearchMetric::Dot,
                    embedding_model: None,
                    memory_tier: VectorMemoryTier::Cached,
                }],
            },
            &mutation_context(
                &id("vector-collection"),
                "request-collection",
                "operation-collection",
            ),
            200,
        )
        .unwrap();
    commit_vectors(
        &engine,
        &lease,
        300,
        vec![
            TransactionMutation::PutSchema {
                registry: vector_fixture_schema(
                    "install vector fixture schema",
                    DataRecordSchema {
                        properties: BTreeMap::from([(
                            "body".into(),
                            DataPropertySchema {
                                value_type: DataValueType::String,
                                required: true,
                            },
                        )]),
                        allow_additional_properties: true,
                        ..DataRecordSchema::default()
                    },
                ),
            },
            document_mutation("alpha", "rrflow durable reasoning"),
            document_mutation("beta", "unrelated storage"),
            document_mutation("gamma", "rrflow graph context"),
            vector_mutation("alpha", vec![1.0, 0.0]),
            vector_mutation("beta", vec![0.0, 1.0]),
            vector_mutation("gamma", vec![0.7, 0.3]),
        ],
    );

    let first = engine
        .ensure_vector_index(
            &lease.session_id,
            &lease.token,
            &index_request(),
            &mutation_context(
                &id("operation-index-1"),
                "request-index-1",
                "operation-index-1",
            ),
            400,
        )
        .unwrap();
    assert_eq!(first.index.kind.as_str(), "hnsw");
    assert_eq!(first.index.generation, 1);
    assert_eq!(first.index.indexed_vectors, 3);
    assert_eq!(
        first.index.maintenance.mode,
        VectorIndexMaintenanceMode::FullBuild
    );
    assert_eq!(first.index.maintenance.indexed_delta_vectors, 3);
    assert!(!first.idempotent_replay);
    assert_direct_read_evidence(
        first.selected_versions,
        &first.read_evidence,
        &[ReadAccessPath::VectorVersions],
    );

    let replay = engine
        .ensure_vector_index(
            &lease.session_id,
            &lease.token,
            &index_request(),
            &mutation_context(
                &id("operation-index-replay"),
                "request-index-replay",
                "operation-index-replay",
            ),
            401,
        )
        .unwrap();
    assert_eq!(replay.index, first.index);
    assert!(replay.idempotent_replay);

    let approximate = search_request(VectorSearchMode::RequireApproximate {
        exact_rerank: 3,
        ef_search: 3,
    });
    let before_restart = engine
        .search_vectors(
            &lease.session_id,
            &lease.token,
            &approximate,
            500,
            "request-search-1",
            "operation-search-1",
        )
        .unwrap();
    assert_eq!(before_restart.access_path.as_str(), "hnsw");
    assert!(!before_restart.exact);
    assert_eq!(before_restart.hits[0].reference.id.as_str(), "alpha");
    assert_direct_read_evidence(
        before_restart.selected_versions,
        &before_restart.read_evidence,
        &[ReadAccessPath::VectorVersions],
    );
    drop(engine);

    let reopened = RrdEngine::open(root.path(), instance(), TOKEN_KEY).unwrap();
    let after_restart = reopened
        .search_vectors(
            &lease.session_id,
            &lease.token,
            &approximate,
            501,
            "request-search-2",
            "operation-search-2",
        )
        .unwrap();
    assert_eq!(after_restart.access_path.as_str(), "hnsw");
    assert_eq!(after_restart.plan_sha256, before_restart.plan_sha256);

    commit_vectors(
        &reopened,
        &lease,
        600,
        vec![
            document_mutation("delta", "durable update"),
            vector_mutation("delta", vec![1.1, 0.0]),
        ],
    );
    let immediate = reopened
        .search_vectors(
            &lease.session_id,
            &lease.token,
            &approximate,
            700,
            "request-search-online-delta",
            "operation-search-online-delta",
        )
        .unwrap();
    assert_eq!(immediate.access_path.as_str(), "hnsw");
    assert_eq!(immediate.hits[0].reference.id.as_str(), "delta");

    let fallback = reopened
        .search_vectors(
            &lease.session_id,
            &lease.token,
            &search_request(VectorSearchMode::AllowApproximate {
                exact_rerank: 4,
                ef_search: 4,
            }),
            701,
            "request-search-fallback",
            "operation-search-fallback",
        )
        .unwrap();
    assert_eq!(fallback.access_path.as_str(), "exact_scan");
    assert!(fallback.exact);

    let rebuilt = reopened
        .ensure_vector_index(
            &lease.session_id,
            &lease.token,
            &index_request(),
            &mutation_context(
                &id("operation-index-2"),
                "request-index-2",
                "operation-index-2",
            ),
            800,
        )
        .unwrap();
    assert_eq!(rebuilt.index.generation, 2);
    assert_eq!(rebuilt.index.indexed_vectors, 4);
    assert_eq!(
        rebuilt.index.maintenance.mode,
        VectorIndexMaintenanceMode::Incremental
    );
    assert_eq!(rebuilt.index.maintenance.previous_generation, Some(1));
    assert_eq!(rebuilt.index.maintenance.indexed_delta_vectors, 1);
    assert!(rebuilt.index.source_cursor > first.index.source_cursor);
    let after_rebuild = reopened
        .search_vectors(
            &lease.session_id,
            &lease.token,
            &search_request(VectorSearchMode::RequireApproximate {
                exact_rerank: 4,
                ef_search: 4,
            }),
            801,
            "request-search-rebuilt",
            "operation-search-rebuilt",
        )
        .unwrap();
    assert_eq!(after_rebuild.access_path.as_str(), "hnsw");

    let turboquant_build = reopened
        .build_vector_quantization_artifact(
            &lease.session_id,
            &lease.token,
            &quantization_build(VectorQuantizationMethod::TurboQuant {
                bits: VectorQuantizationBits::Bits2,
                seed: 23,
                estimator: rrd_contract::VectorQuantizationEstimator::Mse,
            }),
            &mutation_context(
                &id("request-turboquant-build"),
                "request-turboquant-build",
                "operation-turboquant-build",
            ),
            850,
        )
        .unwrap();
    assert_eq!(
        turboquant_build.artifact.method,
        VectorQuantizationMethod::TurboQuant {
            bits: VectorQuantizationBits::Bits2,
            seed: 23,
            estimator: rrd_contract::VectorQuantizationEstimator::Mse,
        }
    );
    assert_eq!(turboquant_build.artifact.generation, 1);
    assert_eq!(turboquant_build.artifact.indexed_vectors, 4);
    assert!(
        turboquant_build.artifact.packed_vector_bytes
            < turboquant_build.artifact.full_precision_vector_bytes
    );
    let turboquant = reopened
        .activate_vector_quantization_artifact(
            &lease.session_id,
            &lease.token,
            &ActivateVectorQuantizationArtifact {
                scope: format!("instance:{}", instance()),
                artifact_id: turboquant_build.artifact.artifact_id.clone(),
                generation: turboquant_build.artifact.generation,
            },
            &mutation_context(
                &id("request-turboquant-activate"),
                "request-turboquant-activate",
                "operation-turboquant-activate",
            ),
            851,
        )
        .unwrap();
    assert_eq!(
        turboquant.artifact.state,
        VectorQuantizationArtifactState::Active
    );
    let turboquant_lifecycle = reopened
        .list_vector_quantization_artifacts(
            &lease.session_id,
            &lease.token,
            &ListVectorQuantizationArtifacts {
                scope: format!("instance:{}", instance()),
                collection_id: Some(CanonicalId::new("documents").unwrap()),
                vector_name: Some(CanonicalId::new("body").unwrap()),
                max_artifacts: 10,
            },
            852,
            "request-turboquant-lifecycle",
            "operation-turboquant-lifecycle",
        )
        .unwrap();
    assert_eq!(turboquant_lifecycle.artifacts.len(), 1);
    assert_eq!(
        turboquant_lifecycle.artifacts[0].state,
        VectorQuantizationArtifactState::Active
    );
    assert_eq!(
        turboquant_lifecycle.artifacts[0].object_sha256,
        turboquant.artifact.object_sha256
    );
    let scope = reopened
        .query_scope(&format!("instance:{}", instance()))
        .unwrap();
    let generic_entries =
        crate::vector_artifact_catalog_entries(&reopened.storage, &scope).unwrap();
    assert!(generic_entries
        .iter()
        .all(|entry| entry.kind != rrd_vector::VectorArtifactKind::TurboQuant));

    let turbo_search = reopened
        .search_vectors(
            &lease.session_id,
            &lease.token,
            &search_request(VectorSearchMode::RequireApproximate {
                exact_rerank: 4,
                ef_search: 4,
            }),
            860,
            "request-turbo-search-1",
            "operation-turbo-search-1",
        )
        .unwrap();
    assert_eq!(turbo_search.access_path.as_str(), "turboquant");
    assert_eq!(turbo_search.hits[0].reference.id.as_str(), "delta");
    assert!((turbo_search.hits[0].score - 1.1).abs() < 1e-6);
    drop(reopened);

    let reopened = RrdEngine::open(root.path(), instance(), TOKEN_KEY).unwrap();
    let reopened_turbo = reopened
        .search_vectors(
            &lease.session_id,
            &lease.token,
            &search_request(VectorSearchMode::RequireApproximate {
                exact_rerank: 4,
                ef_search: 4,
            }),
            861,
            "request-turbo-search-2",
            "operation-turbo-search-2",
        )
        .unwrap();
    assert_eq!(reopened_turbo.access_path.as_str(), "turboquant");
    assert_eq!(reopened_turbo.plan_sha256, turbo_search.plan_sha256);

    let bm25 = reopened
        .ensure_query_index(
            &lease.session_id,
            &lease.token,
            &id("hybrid-bm25-index"),
            &EnsureQueryIndex {
                scope: format!("instance:{}", instance()),
                index_id: CanonicalId::new("document-body-bm25").unwrap(),
                definition_query: "FROM record:document AT VALID 1 KNOWN HEAD PROJECT body".into(),
                unique: false,
                kind: QueryIndexKind::Bm25,
                full_text: None,
                budget: QueryBudget::default(),
            },
            900,
            "request-hybrid-bm25",
            "operation-hybrid-bm25",
        )
        .unwrap();
    assert_eq!(bm25.index.kind, QueryIndexKind::Bm25);

    let hybrid_request = SearchHybrid {
        scope: format!("instance:{}", instance()),
        valid_at: 1,
        document_kind: CanonicalId::new("document").unwrap(),
        text_field: CanonicalId::new("body").unwrap(),
        text_query: "rrflow".into(),
        collection_id: CanonicalId::new("documents").unwrap(),
        vector_name: CanonicalId::new("body").unwrap(),
        vector_query: VectorSearchQuery::Dense {
            values: vec![1.0, 0.0],
        },
        vector_filter: None,
        vector_mode: VectorSearchMode::RequireApproximate {
            exact_rerank: 4,
            ef_search: 4,
        },
        fusion: HybridFusion::ReciprocalRank {
            rank_constant: 60,
            text_weight_millionths: 1_000_000,
            vector_weight_millionths: 1_000_000,
        },
        top_k: 3,
        candidate_k: 4,
        max_storage_keys: 10_000,
    };
    let hybrid = reopened
        .search_hybrid(
            &lease.session_id,
            &lease.token,
            &hybrid_request,
            901,
            "request-hybrid-1",
            "operation-hybrid-1",
        )
        .unwrap();
    assert_eq!(hybrid.text_access_path.as_str(), "bm25_index");
    assert_eq!(hybrid.vector_access_path.as_str(), "turboquant");
    assert!(!hybrid.vector_exact);
    assert_eq!(hybrid.hits[0].subject.id.as_str(), "alpha");
    assert_eq!(hybrid.hits[0].text_rank, Some(1));
    assert_eq!(hybrid.hits[0].vector_rank, Some(2));
    assert_eq!(hybrid.read_manifest_sha256.len(), 64);
    assert_eq!(hybrid.fusion_plan_sha256.len(), 64);
    assert_direct_read_evidence(
        hybrid.selected_versions,
        &hybrid.read_evidence,
        &[ReadAccessPath::ReadStamp, ReadAccessPath::VectorVersions],
    );
    drop(reopened);

    let reopened = RrdEngine::open(root.path(), instance(), TOKEN_KEY).unwrap();
    let replayed_hybrid = reopened
        .search_hybrid(
            &lease.session_id,
            &lease.token,
            &hybrid_request,
            902,
            "request-hybrid-2",
            "operation-hybrid-2",
        )
        .unwrap();
    assert_eq!(replayed_hybrid, hybrid);
}
