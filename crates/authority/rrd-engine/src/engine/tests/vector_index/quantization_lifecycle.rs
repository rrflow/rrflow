//! Quantization build, list, activate, retire, update and recovery.

use super::*;

pub(super) fn quantization_build(
    method: VectorQuantizationMethod,
) -> BuildVectorQuantizationArtifact {
    BuildVectorQuantizationArtifact {
        scope: format!("instance:{}", instance()),
        collection_id: CanonicalId::new("documents").unwrap(),
        vector_name: CanonicalId::new("body").unwrap(),
        method,
        filter_properties: Vec::new(),
        max_storage_keys: 10_000,
    }
}

pub(super) fn quantization_search(values: Vec<f32>, mode: VectorSearchMode) -> SearchVectors {
    SearchVectors {
        scope: format!("instance:{}", instance()),
        valid_at: 1,
        collection_id: Some(CanonicalId::new("documents").unwrap()),
        vector_name: Some(CanonicalId::new("body").unwrap()),
        field: None,
        query: VectorSearchQuery::Dense { values },
        filter: None,
        metric: None,
        top_k: 1,
        mode,
        max_storage_keys: 10_000,
    }
}

#[test]
pub(super) fn quantization_build_list_activate_retire_update_and_recovery_share_exact_truth() {
    let root = tempfile::tempdir().unwrap();
    let engine = RrdEngine::open(root.path(), instance(), TOKEN_KEY).unwrap();
    let lease = engine
        .create_session(
            &session_request(9_000, 4),
            &id("quantization-session"),
            100,
            "request-quantization-session",
            "operation-quantization-session",
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
                    dimensions: 64,
                    metric: VectorSearchMetric::Cosine,
                    embedding_model: None,
                    memory_tier: VectorMemoryTier::Cached,
                }],
            },
            &mutation_context(
                &id("quantization-collection"),
                "request-quantization-collection",
                "operation-quantization-collection",
            ),
            200,
        )
        .unwrap();
    let vector = |row: usize| {
        (0..64)
            .map(|dimension| ((row * 17 + dimension * 13) % 101) as f32 / 50.0 - 1.0)
            .collect::<Vec<_>>()
    };
    commit_vectors(
        &engine,
        &lease,
        300,
        std::iter::once(TransactionMutation::PutSchema {
            registry: vector_fixture_schema(
                "install quantization fixture schema",
                DataRecordSchema {
                    properties: BTreeMap::from([(
                        "body".into(),
                        DataPropertySchema {
                            value_type: DataValueType::String,
                            required: false,
                        },
                    )]),
                    allow_additional_properties: true,
                    ..DataRecordSchema::default()
                },
            ),
        })
        .chain((0..12).flat_map(|row| {
            let id = format!("q{row}");
            [
                document_mutation(&id, "quantization fixture"),
                vector_mutation(&id, vector(row)),
            ]
        }))
        .collect(),
    );

    let methods = [
        VectorQuantizationMethod::Scalar,
        VectorQuantizationMethod::Product {
            compression: VectorProductCompression::X64,
        },
        VectorQuantizationMethod::Binary,
        VectorQuantizationMethod::TurboQuant {
            bits: VectorQuantizationBits::Bits1,
            seed: 91,
            estimator: rrd_contract::VectorQuantizationEstimator::Mse,
        },
    ];
    let mut built = Vec::new();
    for (ordinal, method) in methods.into_iter().enumerate() {
        let result = engine
            .build_vector_quantization_artifact(
                &lease.session_id,
                &lease.token,
                &quantization_build(method),
                &mutation_context(
                    &id(&format!("request-quantization-build-{ordinal}")),
                    &format!("request-quantization-build-{ordinal}"),
                    &format!("operation-quantization-build-{ordinal}"),
                ),
                400 + ordinal as u64,
            )
            .unwrap();
        assert_eq!(
            result.artifact.state,
            VectorQuantizationArtifactState::Ready
        );
        built.push(result.artifact);
    }
    assert_eq!(built[1].maximum_compression_ratio, 64);
    assert_eq!(
        built[1].full_precision_vector_bytes / built[1].packed_vector_bytes,
        64
    );

    let list_request = ListVectorQuantizationArtifacts {
        scope: format!("instance:{}", instance()),
        collection_id: Some(CanonicalId::new("documents").unwrap()),
        vector_name: Some(CanonicalId::new("body").unwrap()),
        max_artifacts: 100,
    };
    let ready = engine
        .list_vector_quantization_artifacts(
            &lease.session_id,
            &lease.token,
            &list_request,
            450,
            "request-quantization-list-ready",
            "operation-quantization-list-ready",
        )
        .unwrap();
    assert_eq!(ready.artifacts.len(), 4);
    assert!(ready
        .artifacts
        .iter()
        .all(|artifact| artifact.state == VectorQuantizationArtifactState::Ready));

    let exact_before = engine
        .search_vectors(
            &lease.session_id,
            &lease.token,
            &quantization_search(
                vector(3),
                VectorSearchMode::AllowApproximate {
                    exact_rerank: 8,
                    ef_search: 8,
                },
            ),
            451,
            "request-quantization-search-before",
            "operation-quantization-search-before",
        )
        .unwrap();
    assert_eq!(exact_before.access_path.as_str(), "exact_scan");

    // TurboQuant has the same explicit build/activate lifecycle as every other
    // quantization method. It never enters the generic HNSW index contract or
    // catalogue.
    let activated_turbo = engine
        .activate_vector_quantization_artifact(
            &lease.session_id,
            &lease.token,
            &ActivateVectorQuantizationArtifact {
                scope: format!("instance:{}", instance()),
                artifact_id: built[3].artifact_id.clone(),
                generation: built[3].generation,
            },
            &mutation_context(
                &id("request-quantization-activate-turbo"),
                "request-quantization-activate-turbo",
                "operation-quantization-activate-turbo",
            ),
            452,
        )
        .unwrap();
    assert_eq!(
        activated_turbo.artifact.state,
        VectorQuantizationArtifactState::Active
    );
    assert_eq!(activated_turbo.artifact.generation, built[3].generation);
    assert_eq!(
        activated_turbo.artifact.object_sha256,
        built[3].object_sha256
    );

    for (ordinal, artifact) in built.iter().take(3).enumerate() {
        engine
            .activate_vector_quantization_artifact(
                &lease.session_id,
                &lease.token,
                &ActivateVectorQuantizationArtifact {
                    scope: format!("instance:{}", instance()),
                    artifact_id: artifact.artifact_id.clone(),
                    generation: artifact.generation,
                },
                &mutation_context(
                    &id(&format!("request-quantization-activate-{ordinal}")),
                    &format!("request-quantization-activate-{ordinal}"),
                    &format!("operation-quantization-activate-{ordinal}"),
                ),
                500 + ordinal as u64,
            )
            .unwrap();
    }
    let approximate_request = quantization_search(
        vector(3),
        VectorSearchMode::RequireApproximate {
            exact_rerank: 8,
            ef_search: 8,
        },
    );
    let active_search = engine
        .search_vectors(
            &lease.session_id,
            &lease.token,
            &approximate_request,
            550,
            "request-quantization-search-active",
            "operation-quantization-search-active",
        )
        .unwrap();
    assert!(!active_search.exact);
    assert_eq!(active_search.hits[0].reference.id.as_str(), "q3");
    drop(engine);

    let reopened = RrdEngine::open(root.path(), instance(), TOKEN_KEY).unwrap();
    let recovered = reopened
        .search_vectors(
            &lease.session_id,
            &lease.token,
            &approximate_request,
            600,
            "request-quantization-search-reopen",
            "operation-quantization-search-reopen",
        )
        .unwrap();
    assert_eq!(recovered.access_path, active_search.access_path);
    assert_eq!(recovered.hits, active_search.hits);

    for (ordinal, artifact) in built.iter().enumerate() {
        reopened
            .retire_vector_quantization_artifact(
                &lease.session_id,
                &lease.token,
                &RetireVectorQuantizationArtifact {
                    scope: format!("instance:{}", instance()),
                    artifact_id: artifact.artifact_id.clone(),
                    generation: artifact.generation,
                },
                &mutation_context(
                    &id(&format!("request-quantization-retire-{ordinal}")),
                    &format!("request-quantization-retire-{ordinal}"),
                    &format!("operation-quantization-retire-{ordinal}"),
                ),
                650 + ordinal as u64,
            )
            .unwrap();
    }
    let retired = reopened
        .list_vector_quantization_artifacts(
            &lease.session_id,
            &lease.token,
            &list_request,
            700,
            "request-quantization-list-retired",
            "operation-quantization-list-retired",
        )
        .unwrap();
    assert!(retired
        .artifacts
        .iter()
        .all(|artifact| artifact.state == VectorQuantizationArtifactState::Retired));
    assert!(reopened
        .search_vectors(
            &lease.session_id,
            &lease.token,
            &approximate_request,
            701,
            "request-quantization-search-retired",
            "operation-quantization-search-retired",
        )
        .is_err());

    commit_vectors(
        &reopened,
        &lease,
        710,
        vec![
            document_mutation("q12", "quantization update"),
            vector_mutation("q12", vector(3)),
        ],
    );
    let rebuilt = reopened
        .build_vector_quantization_artifact(
            &lease.session_id,
            &lease.token,
            &quantization_build(VectorQuantizationMethod::Scalar),
            &mutation_context(
                &id("request-quantization-rebuild"),
                "request-quantization-rebuild",
                "operation-quantization-rebuild",
            ),
            720,
        )
        .unwrap();
    assert_eq!(rebuilt.artifact.generation, 2);
    reopened
        .activate_vector_quantization_artifact(
            &lease.session_id,
            &lease.token,
            &ActivateVectorQuantizationArtifact {
                scope: format!("instance:{}", instance()),
                artifact_id: rebuilt.artifact.artifact_id,
                generation: rebuilt.artifact.generation,
            },
            &mutation_context(
                &id("request-quantization-reactivate"),
                "request-quantization-reactivate",
                "operation-quantization-reactivate",
            ),
            721,
        )
        .unwrap();
    let updated = reopened
        .search_vectors(
            &lease.session_id,
            &lease.token,
            &quantization_search(
                vector(3),
                VectorSearchMode::RequireApproximate {
                    exact_rerank: 13,
                    ef_search: 13,
                },
            ),
            722,
            "request-quantization-search-update",
            "operation-quantization-search-update",
        )
        .unwrap();
    assert_eq!(updated.access_path.as_str(), "scalar_quantized");
    assert_eq!(updated.hits[0].reference.id.as_str(), "q12");
}
