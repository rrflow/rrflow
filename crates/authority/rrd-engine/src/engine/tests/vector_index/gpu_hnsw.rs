//! GPU HNSW builds: durable evidence, fallback and require mode.

use super::*;

#[derive(Debug, Clone, Copy)]
pub(super) enum FakeHnswGpuMode {
    Correct,
    Fail,
    Corrupt,
}

pub(super) struct FakeHnswGpu {
    descriptor: rrd_vector::DenseBuildBackend,
    mode: FakeHnswGpuMode,
}

impl FakeHnswGpu {
    fn new(id: &str, mode: FakeHnswGpuMode) -> Self {
        Self {
            descriptor: rrd_vector::DenseBuildBackend {
                id: id.into(),
                target: rrd_vector::AcceleratorTarget::Gpu {
                    platform: "test-platform".into(),
                    device: "test-device-0".into(),
                },
                deterministic: true,
                supported_format_versions: std::collections::BTreeSet::from([
                    rrd_vector::HNSW_FORMAT_VERSION,
                ]),
            },
            mode,
        }
    }
}

impl rrd_vector::HnswArtifactBuilder for FakeHnswGpu {
    fn descriptor(&self) -> &rrd_vector::DenseBuildBackend {
        &self.descriptor
    }

    fn build(
        &mut self,
        config: &rrd_vector::HnswConfig,
        generation: u64,
        source_cursor: u64,
        candidates: &[rrd_vector::VectorCandidate],
    ) -> rrd_core::Result<Vec<u8>> {
        match self.mode {
            FakeHnswGpuMode::Correct => rrd_vector::HnswIndex::build(
                config.clone(),
                generation,
                source_cursor,
                candidates.to_vec(),
            )
            .map(|artifact| artifact.as_bytes().to_vec()),
            FakeHnswGpuMode::Fail => Err(rrd_core::Error::InvalidRuntime {
                reason: "test GPU device failed".into(),
            }),
            FakeHnswGpuMode::Corrupt => Ok(b"not-an-hnsw-artifact".to_vec()),
        }
    }
}

pub(super) fn prepare_gpu_index_fixture(engine: &RrdEngine) -> rrd_contract::SessionLease {
    let lease = engine
        .create_session(
            &session_request(9_000, 4),
            &id("gpu-index-session"),
            100,
            "request-gpu-index-session",
            "operation-gpu-index-session",
        )
        .unwrap();
    ensure_documents_memory_tier(
        engine,
        &lease,
        VectorMemoryTier::Cached,
        "gpu-index-collection",
        200,
    );
    commit_vectors(
        engine,
        &lease,
        300,
        vec![
            TransactionMutation::PutSchema {
                registry: vector_fixture_schema(
                    "install GPU index fixture schema",
                    DataRecordSchema {
                        allow_additional_properties: true,
                        ..DataRecordSchema::default()
                    },
                ),
            },
            document_mutation("gpu-a", "gpu alpha"),
            document_mutation("gpu-b", "gpu beta"),
            vector_mutation("gpu-a", vec![1.0, 0.0]),
            vector_mutation("gpu-b", vec![0.0, 1.0]),
        ],
    );
    lease
}

#[test]
pub(super) fn gpu_hnsw_build_evidence_is_public_durable_and_search_accounted() {
    let root = tempfile::tempdir().unwrap();
    let engine = RrdEngine::open(root.path(), instance(), TOKEN_KEY).unwrap();
    let lease = prepare_gpu_index_fixture(&engine);
    let backend_id = CanonicalId::new("test-gpu-hnsw").unwrap();
    engine
        .install_hnsw_accelerator(FakeHnswGpu::new(
            backend_id.as_str(),
            FakeHnswGpuMode::Correct,
        ))
        .unwrap();
    let mut request = index_request();
    request.build_policy = VectorIndexBuildPolicy::RequireGpu {
        backend_id: backend_id.clone(),
    };

    let built = engine
        .ensure_vector_index(
            &lease.session_id,
            &lease.token,
            &request,
            &mutation_context(
                &id("operation-gpu-index-build"),
                "request-gpu-index-build",
                "operation-gpu-index-build",
            ),
            400,
        )
        .unwrap();
    let evidence = built.index.build_evidence.clone().unwrap();
    assert_eq!(evidence.requested_backend_id.as_ref(), Some(&backend_id));
    assert_eq!(evidence.selected_backend_id, backend_id.as_str());
    assert!(matches!(
        evidence.selected_target,
        VectorIndexBuildTarget::Gpu { .. }
    ));
    assert!(!evidence.used_fallback);
    assert_eq!(
        evidence.byte_differential,
        VectorIndexDifferentialStatus::Passed
    );
    assert_eq!(
        evidence.semantic_differential,
        VectorIndexDifferentialStatus::Passed
    );
    assert_eq!(evidence.resources.input_vectors, 2);
    assert_eq!(evidence.resources.dimensions, 2);
    assert_eq!(evidence.resources.input_values, 4);
    assert_eq!(evidence.resources.semantic_probe_queries, 2);
    assert_eq!(
        evidence.resources.accelerator_artifact_bytes,
        Some(evidence.resources.cpu_artifact_bytes)
    );

    let replay = engine
        .ensure_vector_index(
            &lease.session_id,
            &lease.token,
            &request,
            &mutation_context(
                &id("operation-gpu-index-replay"),
                "request-gpu-index-replay",
                "operation-gpu-index-replay",
            ),
            401,
        )
        .unwrap();
    assert!(replay.idempotent_replay);
    assert_eq!(replay.index.generation, built.index.generation);
    assert_eq!(replay.index.build_evidence.as_ref(), Some(&evidence));

    let approximate_request = search_request(VectorSearchMode::RequireApproximate {
        exact_rerank: 2,
        ef_search: 4,
    });
    let approximate = engine
        .search_vectors(
            &lease.session_id,
            &lease.token,
            &approximate_request,
            500,
            "request-gpu-index-search",
            "operation-gpu-index-search",
        )
        .unwrap();
    assert_eq!(approximate.access_path.as_str(), "hnsw");
    assert_eq!(approximate.resources.canonical_candidates, 2);
    assert_eq!(approximate.resources.selected_candidates, 2);
    assert_eq!(approximate.resources.ef_search, 4);
    assert_eq!(approximate.resources.exact_rerank, 2);
    assert_eq!(approximate.resources.overlay_candidates, 0);
    assert_eq!(
        approximate.resources.selected_generation,
        Some(built.index.generation)
    );
    assert_eq!(
        approximate.resources.loaded_artifact_bytes,
        evidence.resources.cpu_artifact_bytes
    );
    assert_eq!(approximate.resources.result_hits, 1);
    drop(engine);

    // The adapter is intentionally not reinstalled. Durable evidence is
    // sufficient to replay the already-qualified immutable generation.
    let reopened = RrdEngine::open(root.path(), instance(), TOKEN_KEY).unwrap();
    let recovered = reopened
        .ensure_vector_index(
            &lease.session_id,
            &lease.token,
            &request,
            &mutation_context(
                &id("operation-gpu-index-recovered"),
                "request-gpu-index-recovered",
                "operation-gpu-index-recovered",
            ),
            600,
        )
        .unwrap();
    assert!(recovered.idempotent_replay);
    assert_eq!(recovered.index.build_evidence.as_ref(), Some(&evidence));
    let recovered_search = reopened
        .search_vectors(
            &lease.session_id,
            &lease.token,
            &approximate_request,
            601,
            "request-gpu-index-recovered-search",
            "operation-gpu-index-recovered-search",
        )
        .unwrap();
    assert_eq!(recovered_search.hits, approximate.hits);
    assert_eq!(
        recovered_search.resources.loaded_artifact_bytes,
        approximate.resources.loaded_artifact_bytes
    );
}

#[test]
pub(super) fn gpu_hnsw_failure_falls_back_and_require_mode_cannot_publish() {
    let root = tempfile::tempdir().unwrap();
    let engine = RrdEngine::open(root.path(), instance(), TOKEN_KEY).unwrap();
    let lease = prepare_gpu_index_fixture(&engine);
    let exact_request = search_request(VectorSearchMode::Exact);
    let exact_before = engine
        .search_vectors(
            &lease.session_id,
            &lease.token,
            &exact_request,
            350,
            "request-gpu-exact-before",
            "operation-gpu-exact-before",
        )
        .unwrap();

    let failing_id = CanonicalId::new("test-gpu-fail").unwrap();
    engine
        .install_hnsw_accelerator(FakeHnswGpu::new(failing_id.as_str(), FakeHnswGpuMode::Fail))
        .unwrap();
    let mut fallback_request = index_request();
    fallback_request.build_policy = VectorIndexBuildPolicy::PreferGpu {
        backend_id: failing_id.clone(),
        allow_cpu_fallback: true,
    };
    let fallback = engine
        .ensure_vector_index(
            &lease.session_id,
            &lease.token,
            &fallback_request,
            &mutation_context(
                &id("operation-gpu-fallback"),
                "request-gpu-fallback",
                "operation-gpu-fallback",
            ),
            400,
        )
        .unwrap();
    let fallback_evidence = fallback.index.build_evidence.unwrap();
    assert!(fallback_evidence.used_fallback);
    assert!(fallback_evidence
        .fallback_reason
        .as_deref()
        .is_some_and(|reason| reason.contains("test GPU device failed")));
    assert!(matches!(
        fallback_evidence.selected_target,
        VectorIndexBuildTarget::Cpu
    ));
    assert_eq!(
        fallback_evidence.byte_differential,
        VectorIndexDifferentialStatus::NotRun
    );

    let scope = rrd_core::ScopeId::new(format!("instance:{}", instance())).unwrap();
    let entries_before_require =
        crate::vector_artifact_catalog_entries(&engine.storage, &scope).unwrap();
    let mut require_request = fallback_request.clone();
    require_request.build_policy = VectorIndexBuildPolicy::RequireGpu {
        backend_id: failing_id,
    };
    assert!(engine
        .ensure_vector_index(
            &lease.session_id,
            &lease.token,
            &require_request,
            &mutation_context(
                &id("operation-gpu-required-failure"),
                "request-gpu-required-failure",
                "operation-gpu-required-failure"
            ),
            410,
        )
        .is_err());
    let entries_after_require =
        crate::vector_artifact_catalog_entries(&engine.storage, &scope).unwrap();
    assert_eq!(entries_after_require, entries_before_require);

    let corrupt_id = CanonicalId::new("test-gpu-corrupt").unwrap();
    engine
        .install_hnsw_accelerator(FakeHnswGpu::new(
            corrupt_id.as_str(),
            FakeHnswGpuMode::Corrupt,
        ))
        .unwrap();
    let mut corrupt_request = index_request();
    corrupt_request.build_policy = VectorIndexBuildPolicy::PreferGpu {
        backend_id: corrupt_id,
        allow_cpu_fallback: true,
    };
    let corrupt_fallback = engine
        .ensure_vector_index(
            &lease.session_id,
            &lease.token,
            &corrupt_request,
            &mutation_context(
                &id("operation-gpu-corrupt-fallback"),
                "request-gpu-corrupt-fallback",
                "operation-gpu-corrupt-fallback",
            ),
            420,
        )
        .unwrap();
    let corrupt_evidence = corrupt_fallback.index.build_evidence.unwrap();
    assert!(corrupt_evidence.used_fallback);
    assert_eq!(
        corrupt_evidence.byte_differential,
        VectorIndexDifferentialStatus::Failed
    );
    assert_eq!(
        corrupt_evidence.semantic_differential,
        VectorIndexDifferentialStatus::NotRun
    );

    let exact_after = engine
        .search_vectors(
            &lease.session_id,
            &lease.token,
            &exact_request,
            500,
            "request-gpu-exact-after",
            "operation-gpu-exact-after",
        )
        .unwrap();
    assert_eq!(exact_after.hits, exact_before.hits);
    let approximate = engine
        .search_vectors(
            &lease.session_id,
            &lease.token,
            &search_request(VectorSearchMode::RequireApproximate {
                exact_rerank: 2,
                ef_search: 4,
            }),
            501,
            "request-gpu-corrupt-search",
            "operation-gpu-corrupt-search",
        )
        .unwrap();
    assert_eq!(approximate.hits, exact_before.hits);
}
