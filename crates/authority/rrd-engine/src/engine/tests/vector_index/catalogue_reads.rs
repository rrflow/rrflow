use super::*;

fn semantic_counts(evidence: &ReadEvidence) -> (u64, u64, u64) {
    evidence.paths.iter().fold((0, 0, 0), |sum, path| {
        if matches!(
            path.path,
            ReadAccessPath::RecordVersions
                | ReadAccessPath::ObjectVersions
                | ReadAccessPath::VectorVersions
        ) {
            (
                sum.0 + path.keys_examined,
                sum.1 + path.values_decoded,
                sum.2 + path.decoded_bytes,
            )
        } else {
            sum
        }
    })
}

#[test]
fn retained_search_uses_its_artifact_generation_after_publication_and_reopen() {
    let root = tempfile::tempdir().unwrap();
    let engine = RrdEngine::open(root.path(), instance(), TOKEN_KEY).unwrap();
    let lease = prepare_gpu_index_fixture(&engine);
    let first = engine
        .ensure_vector_index(
            &lease.session_id,
            &lease.token,
            &index_request(),
            400,
            "catalogue-first",
            "catalogue-first",
        )
        .unwrap();
    assert_eq!(first.index.generation, 1);
    let scope = rrd_core::ScopeId::new(format!("instance:{}", instance())).unwrap();
    let retained = engine.storage.runtime().read_stamp(&scope).unwrap();
    let request = search_request(VectorSearchMode::RequireApproximate {
        exact_rerank: 3,
        ef_search: 3,
    });
    let before = engine
        .search_vectors_at(&request, retained.clone())
        .unwrap();
    let mut rebuild = index_request();
    let VectorIndexConfiguration::Hnsw { seed, .. } = &mut rebuild.configuration;
    *seed += 1;
    let second = engine
        .ensure_vector_index(
            &lease.session_id,
            &lease.token,
            &rebuild,
            500,
            "catalogue-second",
            "catalogue-second",
        )
        .unwrap();
    assert_eq!(second.index.generation, 2);
    let after = engine
        .search_vectors_at(&request, retained.clone())
        .unwrap();
    assert_eq!(after.resources.selected_generation, Some(1));
    assert_eq!(after.plan_sha256, before.plan_sha256);
    assert_eq!(after.hits, before.hits);
    let current = engine.storage.runtime().read_stamp(&scope).unwrap();
    assert_eq!(
        engine
            .search_vectors_at(&request, current.clone())
            .unwrap()
            .resources
            .selected_generation,
        Some(2)
    );
    drop(engine);
    let reopened = RrdEngine::open(root.path(), instance(), TOKEN_KEY).unwrap();
    let historical = reopened.search_vectors_at(&request, retained).unwrap();
    assert_eq!(historical.resources.selected_generation, Some(1));
    assert_eq!(historical.plan_sha256, before.plan_sha256);
    assert_eq!(historical.hits, before.hits);
    assert_eq!(
        reopened
            .search_vectors_at(&request, current)
            .unwrap()
            .resources
            .selected_generation,
        Some(2)
    );
}

#[test]
fn catalogue_search_work_excludes_unrelated_history_and_shares_the_request_budget() {
    let root = tempfile::tempdir().unwrap();
    let engine = RrdEngine::open(root.path(), instance(), TOKEN_KEY).unwrap();
    let lease = prepare_gpu_index_fixture(&engine);
    engine
        .ensure_vector_index(
            &lease.session_id,
            &lease.token,
            &index_request(),
            400,
            "catalogue-index",
            "catalogue-index",
        )
        .unwrap();
    let scope = rrd_core::ScopeId::new(format!("instance:{}", instance())).unwrap();
    let retained = engine.storage.runtime().read_stamp(&scope).unwrap();
    let request = search_request(VectorSearchMode::RequireApproximate {
        exact_rerank: 3,
        ef_search: 3,
    });
    let before = engine
        .search_vectors_at(&request, retained.clone())
        .unwrap();
    assert_direct_read_evidence(
        before.selected_versions,
        &before.read_evidence,
        &[
            ReadAccessPath::VectorVersions,
            ReadAccessPath::RecordVersions,
            ReadAccessPath::ObjectVersions,
        ],
    );
    // A vectors-only allowance must not silently fund catalogue reads.
    let vectors = engine
        .storage
        .runtime()
        .read_versioned(
            &retained,
            &[rrd_store::RuntimeVersionedSource::Vectors { kind: None }],
            rrd_store::RuntimeReadBudget::new(request.max_storage_keys).unwrap(),
        )
        .unwrap();
    let mut restricted = request.clone();
    restricted.max_storage_keys = vectors.evidence.keys_examined;
    assert!(restricted.max_storage_keys < before.read_evidence.keys_examined);
    let error = engine
        .search_vectors_at(&restricted, retained.clone())
        .unwrap_err();
    assert!(error.to_string().contains("budget"), "{error}");
    assert_eq!(error.kind(), crate::ServiceErrorKind::ResourceExhausted);
    assert!(!error.retryable());
    let storage_fault = ServiceError::from(rrd_store::Error::Substrate("I/O failure".into()));
    assert_eq!(storage_fault.kind(), crate::ServiceErrorKind::Internal);
    // The exact measured allowance succeeds; one fewer key fails closed.
    restricted.max_storage_keys = before.read_evidence.keys_examined;
    assert_eq!(
        engine
            .search_vectors_at(&restricted, retained.clone())
            .unwrap()
            .hits,
        before.hits
    );
    restricted.max_storage_keys -= 1;
    assert!(engine
        .search_vectors_at(&restricted, retained.clone())
        .unwrap_err()
        .to_string()
        .contains("budget"));

    for batch in 0..16 {
        commit_vectors(
            &engine,
            &lease,
            500 + batch * 2,
            (0..64)
                .map(|item| {
                    document_mutation(&format!("unrelated-{batch}-{item}"), "source history")
                })
                .collect(),
        );
    }
    let current = engine.storage.runtime().read_stamp(&scope).unwrap();
    let grown = engine.search_vectors_at(&request, current.clone()).unwrap();
    let historical = engine
        .search_vectors_at(&request, retained.clone())
        .unwrap();
    for result in [&grown, &historical] {
        assert_eq!(result.hits, before.hits);
        assert_eq!(result.selected_versions, before.selected_versions);
        assert_eq!(
            semantic_counts(&result.read_evidence),
            semantic_counts(&before.read_evidence)
        );
    }
    println!(
        "catalogue_growth_evidence={}",
        serde_json::json!({
            "unrelated_records": 1024,
            "before_cursor": retained.commit_cursor,
            "after_cursor": current.commit_cursor,
            "before": before.read_evidence,
            "after": grown.read_evidence,
            "retained": historical.read_evidence,
        })
    );
    drop(engine);
    let reopened = RrdEngine::open(root.path(), instance(), TOKEN_KEY).unwrap();
    for read in [retained, current] {
        let result = reopened.search_vectors_at(&request, read).unwrap();
        assert_eq!(result.hits, before.hits);
        assert_eq!(
            semantic_counts(&result.read_evidence),
            semantic_counts(&before.read_evidence)
        );
    }
}

#[test]
fn retained_quantization_state_survives_activation_retirement_and_reopen() {
    let root = tempfile::tempdir().unwrap();
    let engine = RrdEngine::open(root.path(), instance(), TOKEN_KEY).unwrap();
    let lease = prepare_gpu_index_fixture(&engine);
    let built = engine
        .build_vector_quantization_artifact(
            &lease.session_id,
            &lease.token,
            &quantization_build(VectorQuantizationMethod::Scalar),
            400,
            "catalogue-build",
            "catalogue-build",
        )
        .unwrap();
    let scope = rrd_core::ScopeId::new(format!("instance:{}", instance())).unwrap();
    let ready = engine.storage.runtime().read_stamp(&scope).unwrap();
    engine
        .activate_vector_quantization_artifact(
            &lease.session_id,
            &lease.token,
            &ActivateVectorQuantizationArtifact {
                scope: scope.to_string(),
                artifact_id: built.artifact.artifact_id.clone(),
                generation: built.artifact.generation,
            },
            500,
            "catalogue-activate",
            "catalogue-activate",
        )
        .unwrap();
    let active = engine.storage.runtime().read_stamp(&scope).unwrap();
    let request = search_request(VectorSearchMode::AllowApproximate {
        exact_rerank: 2,
        ef_search: 2,
    });
    let before = engine.search_vectors_at(&request, active.clone()).unwrap();
    assert_eq!(before.access_path.as_str(), "scalar_quantized");
    engine
        .retire_vector_quantization_artifact(
            &lease.session_id,
            &lease.token,
            &RetireVectorQuantizationArtifact {
                scope: scope.to_string(),
                artifact_id: built.artifact.artifact_id,
                generation: built.artifact.generation,
            },
            600,
            "catalogue-retire",
            "catalogue-retire",
        )
        .unwrap();
    let retired = engine.storage.runtime().read_stamp(&scope).unwrap();
    for read in [ready.clone(), retired.clone()] {
        assert_eq!(
            engine
                .search_vectors_at(&request, read)
                .unwrap()
                .access_path
                .as_str(),
            "exact_scan"
        );
    }
    assert_eq!(
        engine
            .search_vectors_at(&request, active.clone())
            .unwrap()
            .plan_sha256,
        before.plan_sha256
    );
    drop(engine);
    let reopened = RrdEngine::open(root.path(), instance(), TOKEN_KEY).unwrap();
    let recovered = reopened.search_vectors_at(&request, active).unwrap();
    assert_eq!(recovered.plan_sha256, before.plan_sha256);
    assert_eq!(recovered.hits, before.hits);
    for read in [ready, retired] {
        assert_eq!(
            reopened
                .search_vectors_at(&request, read)
                .unwrap()
                .access_path
                .as_str(),
            "exact_scan"
        );
    }
}
