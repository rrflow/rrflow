//! Unified retrieval algebra: multimodal, late interaction and analytics.

use super::*;

#[test]
pub(super) fn unified_retrieval_algebra_executes_multimodal_late_interaction_and_analytics() {
    let root = tempfile::tempdir().unwrap();
    let engine = RrdEngine::open(root.path(), instance(), TOKEN_KEY).unwrap();
    let lease = engine
        .create_session(
            &session_request(9_000, 8),
            &id("retrieval-algebra-session"),
            100,
            "request-retrieval-session",
            "operation-retrieval-session",
        )
        .unwrap();
    let scope = format!("instance:{}", instance());
    let collection_id = CanonicalId::new("multimodal-retrieval").unwrap();
    let model_digest = "a".repeat(64);
    engine
        .ensure_vector_collection(
            &lease.session_id,
            &lease.token,
            &EnsureVectorCollection {
                scope: scope.clone(),
                collection_id: collection_id.clone(),
                vectors: vec![
                    NamedVectorDefinition {
                        name: CanonicalId::new("dense").unwrap(),
                        field: CanonicalId::new("dense-embedding").unwrap(),
                        kind: VectorValueKind::Dense,
                        dimensions: 2,
                        metric: VectorSearchMetric::Dot,
                        embedding_model: None,
                        memory_tier: VectorMemoryTier::Cached,
                    },
                    NamedVectorDefinition {
                        name: CanonicalId::new("image").unwrap(),
                        field: CanonicalId::new("image-embedding").unwrap(),
                        kind: VectorValueKind::Dense,
                        dimensions: 2,
                        metric: VectorSearchMetric::Dot,
                        embedding_model: None,
                        memory_tier: VectorMemoryTier::Cached,
                    },
                    NamedVectorDefinition {
                        name: CanonicalId::new("late").unwrap(),
                        field: CanonicalId::new("late-embedding").unwrap(),
                        kind: VectorValueKind::MultiDense,
                        dimensions: 2,
                        metric: VectorSearchMetric::Dot,
                        embedding_model: Some(VectorEmbeddingModel {
                            name: "colbert-test-v1".into(),
                            digest: model_digest.clone(),
                        }),
                        memory_tier: VectorMemoryTier::Cold,
                    },
                    NamedVectorDefinition {
                        name: CanonicalId::new("sparse").unwrap(),
                        field: CanonicalId::new("sparse-embedding").unwrap(),
                        kind: VectorValueKind::Sparse,
                        dimensions: 4,
                        metric: VectorSearchMetric::Dot,
                        embedding_model: None,
                        memory_tier: VectorMemoryTier::Cold,
                    },
                ],
            },
            &mutation_context(
                &id("ensure-retrieval-collection"),
                "request-retrieval-collection",
                "operation-retrieval-collection",
            ),
            200,
        )
        .unwrap();
    for (ordinal, (field, kind)) in [
        ("category", VectorPayloadIndexKind::Keyword),
        ("popularity", VectorPayloadIndexKind::Unsigned),
    ]
    .into_iter()
    .enumerate()
    {
        engine
            .ensure_vector_payload_index(
                &lease.session_id,
                &lease.token,
                &EnsureVectorPayloadIndex {
                    scope: scope.clone(),
                    collection_id: collection_id.clone(),
                    field: CanonicalId::new(field).unwrap(),
                    kind,
                },
                &mutation_context(
                    &id(&format!("ensure-retrieval-payload-{ordinal}")),
                    &format!("request-retrieval-payload-{ordinal}"),
                    &format!("operation-retrieval-payload-{ordinal}"),
                ),
                210 + ordinal as u64,
            )
            .unwrap();
    }

    let provenance = DataEmbeddingProvenance {
        source_sha256: "b".repeat(64),
        model: "colbert-test-v1".into(),
        model_sha256: model_digest.clone(),
        dimensions: 2,
        normalization: DataVectorNormalization::None,
        generation_parameters: DataProperties::new(),
    };
    let point = |id: &str,
                 vector_name: &str,
                 field: &str,
                 value: DataVectorValue,
                 category: &str,
                 popularity: u64| {
        TransactionMutation::PutVector {
            reference: reference("embedding", &format!("{id}-{vector_name}")),
            subject: reference("document", id),
            collection_id: collection_id.clone(),
            vector_name: CanonicalId::new(vector_name).unwrap(),
            field: CanonicalId::new(field).unwrap(),
            valid_from: 1,
            valid_to: None,
            provenance: (vector_name == "late").then(|| provenance.clone()),
            value,
            properties: DataProperties::from([
                ("category".into(), QueryValue::String(category.into())),
                ("popularity".into(), QueryValue::Unsigned(popularity)),
            ]),
        }
    };
    let mut mutations = vec![
        TransactionMutation::PutSchema {
            registry: vector_fixture_schema(
                "install unified retrieval fixture",
                DataRecordSchema {
                    properties: BTreeMap::from([(
                        "body".into(),
                        DataPropertySchema {
                            value_type: DataValueType::String,
                            required: true,
                        },
                    )]),
                    allow_additional_properties: false,
                    ..DataRecordSchema::default()
                },
            ),
        },
        document_mutation("alpha", "rrflow durable multimodal reasoning"),
        document_mutation("beta", "unrelated archival storage"),
        document_mutation("gamma", "rrflow graph reasoning context"),
    ];
    for (id, category, popularity, dense, image, sparse, late) in [
        (
            "alpha",
            "premium",
            10,
            vec![1.0, 0.0],
            vec![0.9, 0.1],
            (vec![0], vec![1.0]),
            vec![vec![1.0, 0.0], vec![0.0, 1.0]],
        ),
        (
            "beta",
            "standard",
            3,
            vec![0.0, 1.0],
            vec![0.1, 0.9],
            (vec![1], vec![1.0]),
            vec![vec![0.0, 1.0], vec![0.0, 1.0]],
        ),
        (
            "gamma",
            "premium",
            7,
            vec![0.7, 0.3],
            vec![0.8, 0.2],
            (vec![0, 2], vec![0.7, 0.4]),
            vec![vec![1.0, 0.0], vec![1.0, 0.0]],
        ),
    ] {
        mutations.extend([
            point(
                id,
                "dense",
                "dense-embedding",
                DataVectorValue::Dense { values: dense },
                category,
                popularity,
            ),
            point(
                id,
                "image",
                "image-embedding",
                DataVectorValue::Dense { values: image },
                category,
                popularity,
            ),
            point(
                id,
                "late",
                "late-embedding",
                DataVectorValue::MultiDense {
                    dimensions: 2,
                    vectors: late,
                },
                category,
                popularity,
            ),
            point(
                id,
                "sparse",
                "sparse-embedding",
                DataVectorValue::Sparse {
                    dimensions: 4,
                    indices: sparse.0,
                    values: sparse.1,
                },
                category,
                popularity,
            ),
        ]);
    }
    commit_vectors(&engine, &lease, 300, mutations);

    let nearest = |using: &str, query: VectorSearchQuery| RetrievalQuery::Nearest {
        using: CanonicalId::new(using).unwrap(),
        query,
        filter: None,
        mode: VectorSearchMode::Exact,
    };
    let fusion = RetrievalQuery::Fusion {
        prefetch: vec![
            RetrievalPrefetch {
                query: Box::new(RetrievalQuery::Keyword {
                    document_kind: CanonicalId::new("document").unwrap(),
                    text_field: CanonicalId::new("body").unwrap(),
                    query: "rrflow reasoning".into(),
                }),
                limit: 3,
            },
            RetrievalPrefetch {
                query: Box::new(nearest(
                    "dense",
                    VectorSearchQuery::Dense {
                        values: vec![1.0, 0.0],
                    },
                )),
                limit: 3,
            },
            RetrievalPrefetch {
                query: Box::new(nearest(
                    "sparse",
                    VectorSearchQuery::Sparse {
                        dimensions: 4,
                        indices: vec![0],
                        values: vec![1.0],
                    },
                )),
                limit: 3,
            },
        ],
        fusion: RetrievalFusion::ReciprocalRank {
            rank_constant: 60,
            weights_millionths: vec![1_000_000, 1_000_000, 1_000_000],
        },
    };
    let premium = VectorPayloadFilter::Condition {
        condition: VectorPayloadCondition {
            property: CanonicalId::new("category").unwrap(),
            operator: VectorPayloadOperator::Equals {
                value: QueryValue::String("premium".into()),
            },
        },
    };
    let reranked = RetrievalQuery::Rerank {
        prefetch: Box::new(RetrievalPrefetch {
            query: Box::new(fusion.clone()),
            limit: 3,
        }),
        stages: vec![
            RetrievalRerankStage::Exact {
                using: CanonicalId::new("image").unwrap(),
                query: VectorSearchQuery::Dense {
                    values: vec![1.0, 0.0],
                },
            },
            RetrievalRerankStage::Model {
                using: CanonicalId::new("late").unwrap(),
                model: VectorEmbeddingModel {
                    name: "colbert-test-v1".into(),
                    digest: model_digest,
                },
                query: VectorSearchQuery::MultiDense {
                    dimensions: 2,
                    vectors: vec![vec![1.0, 0.0], vec![0.0, 1.0]],
                    comparator: rrd_contract::MultiVectorComparator::MaxSim,
                },
            },
            RetrievalRerankStage::ScoreBoost {
                filter: premium,
                add_millionths: 250_000,
                multiply_millionths: 1_000_000,
            },
            RetrievalRerankStage::Mmr {
                using: CanonicalId::new("dense").unwrap(),
                diversity_millionths: 100_000,
            },
        ],
    };
    let points_request = ExecuteRetrievalQuery {
        scope: scope.clone(),
        valid_at: 1,
        collection_id: collection_id.clone(),
        query: reranked,
        result_shape: RetrievalResultShape::Points,
        limit: 3,
        candidate_limit: 3,
        max_storage_keys: 10_000,
    };
    let points = engine
        .execute_retrieval_query(
            &lease.session_id,
            &lease.token,
            &points_request,
            400,
            "request-unified-points",
            "operation-unified-points",
        )
        .unwrap();
    let RetrievalOutput::Points { hits } = &points.output else {
        panic!("expected point retrieval output")
    };
    assert_eq!(hits[0].subject.id.as_str(), "alpha");
    assert_eq!(hits[0].vector_references.len(), 4);
    assert!(points
        .stages
        .iter()
        .any(|stage| stage.kind.as_str() == "keyword"));
    assert!(points
        .stages
        .iter()
        .any(|stage| stage.kind.as_str() == "fusion-rrf"));
    assert!(points
        .stages
        .iter()
        .any(|stage| stage.kind.as_str() == "model-rerank"));
    assert!(points
        .stages
        .iter()
        .any(|stage| stage.kind.as_str() == "mmr"));
    assert_direct_read_evidence(
        points.selected_versions,
        &points.read_evidence,
        &[ReadAccessPath::ReadStamp, ReadAccessPath::VectorVersions],
    );

    let example = |id: &str| RetrievalVectorExample::Reference {
        reference: reference("embedding", &format!("{id}-dense")),
    };
    for (ordinal, query) in [
        RetrievalQuery::Recommend {
            using: CanonicalId::new("dense").unwrap(),
            positive: vec![example("alpha")],
            negative: vec![example("beta")],
            strategy: RetrievalRecommendStrategy::AverageVector,
            filter: None,
        },
        RetrievalQuery::Recommend {
            using: CanonicalId::new("dense").unwrap(),
            positive: vec![example("alpha")],
            negative: vec![example("beta")],
            strategy: RetrievalRecommendStrategy::BestScore,
            filter: None,
        },
        RetrievalQuery::Discover {
            using: CanonicalId::new("dense").unwrap(),
            target: RetrievalVectorExample::Vector {
                value: VectorSearchQuery::Dense {
                    values: vec![1.0, 0.0],
                },
            },
            context: vec![RetrievalContextPair {
                positive: example("alpha"),
                negative: example("beta"),
            }],
            filter: None,
        },
        RetrievalQuery::Context {
            using: CanonicalId::new("dense").unwrap(),
            context: vec![RetrievalContextPair {
                positive: example("alpha"),
                negative: example("beta"),
            }],
            filter: None,
        },
    ]
    .into_iter()
    .enumerate()
    {
        let result = engine
            .execute_retrieval_query(
                &lease.session_id,
                &lease.token,
                &ExecuteRetrievalQuery {
                    scope: scope.clone(),
                    valid_at: 1,
                    collection_id: collection_id.clone(),
                    query,
                    result_shape: RetrievalResultShape::Points,
                    limit: 2,
                    candidate_limit: 3,
                    max_storage_keys: 10_000,
                },
                410 + ordinal as u64,
                &format!("request-explore-{ordinal}"),
                &format!("operation-explore-{ordinal}"),
            )
            .unwrap();
        let RetrievalOutput::Points { hits } = result.output else {
            panic!("expected exploration points")
        };
        assert_eq!(hits[0].subject.id.as_str(), "alpha");
    }

    for (ordinal, result_shape) in [
        RetrievalResultShape::Groups {
            property: CanonicalId::new("category").unwrap(),
            max_groups: 2,
            hits_per_group: 1,
        },
        RetrievalResultShape::Facets {
            property: CanonicalId::new("category").unwrap(),
            limit: 2,
        },
        RetrievalResultShape::Matrix {
            using: CanonicalId::new("dense").unwrap(),
            sample: 3,
        },
    ]
    .into_iter()
    .enumerate()
    {
        let result = engine
            .execute_retrieval_query(
                &lease.session_id,
                &lease.token,
                &ExecuteRetrievalQuery {
                    scope: scope.clone(),
                    valid_at: 1,
                    collection_id: collection_id.clone(),
                    query: fusion.clone(),
                    result_shape,
                    limit: 2,
                    candidate_limit: 3,
                    max_storage_keys: 10_000,
                },
                420 + ordinal as u64,
                &format!("request-shape-{ordinal}"),
                &format!("operation-shape-{ordinal}"),
            )
            .unwrap();
        match result.output {
            RetrievalOutput::Groups { groups } => {
                assert_eq!(groups.len(), 2);
                assert!(groups.iter().all(|group| group.hits.len() == 1));
            }
            RetrievalOutput::Facets { facets } => {
                assert_eq!(facets.len(), 2);
                assert_eq!(facets.iter().map(|facet| facet.count).sum::<u64>(), 3);
            }
            RetrievalOutput::Matrix { subjects, cells } => {
                assert_eq!(subjects.len(), 3);
                assert_eq!(cells.len(), 6);
            }
            RetrievalOutput::Points { .. } => panic!("expected analytical output"),
        }
    }

    drop(engine);
    let reopened = RrdEngine::open(root.path(), instance(), TOKEN_KEY).unwrap();
    let replayed = reopened
        .execute_retrieval_query(
            &lease.session_id,
            &lease.token,
            &points_request,
            500,
            "request-unified-reopen",
            "operation-unified-reopen",
        )
        .unwrap();
    assert_eq!(replayed, points);
}
