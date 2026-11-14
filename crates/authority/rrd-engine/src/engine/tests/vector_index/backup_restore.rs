//! Backups restore TurboQuant payloads before instance activation.

use super::*;

#[test]
pub(super) fn application_backup_restores_turboquant_payload_before_instance_activation() {
    let root = tempfile::tempdir().unwrap();
    let source_root = root.path().join("source");
    let engine = RrdEngine::open(&source_root, instance(), TOKEN_KEY).unwrap();
    let lease = engine
        .create_session(
            &session_request(9_000, 4),
            &id("backup-vector-session"),
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
                &id("backup-vector-collection"),
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
                    "install backup vector fixture schema",
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
            },
            document_mutation("alpha", "rrflow backup alpha"),
            document_mutation("beta", "rrflow backup beta"),
            vector_mutation("alpha", vec![1.0, 0.0]),
            vector_mutation("beta", vec![0.0, 1.0]),
        ],
    );
    let built = engine
        .build_vector_quantization_artifact(
            &lease.session_id,
            &lease.token,
            &quantization_build(VectorQuantizationMethod::TurboQuant {
                bits: VectorQuantizationBits::Bits2,
                seed: 23,
                estimator: rrd_contract::VectorQuantizationEstimator::Mse,
            }),
            &mutation_context(
                &id("request-backup-turboquant-build"),
                "request-backup-turboquant-build",
                "operation-backup-turboquant-build",
            ),
            400,
        )
        .unwrap();
    let active = engine
        .activate_vector_quantization_artifact(
            &lease.session_id,
            &lease.token,
            &ActivateVectorQuantizationArtifact {
                scope: format!("instance:{}", instance()),
                artifact_id: built.artifact.artifact_id,
                generation: built.artifact.generation,
            },
            &mutation_context(
                &id("request-backup-turboquant-activate"),
                "request-backup-turboquant-activate",
                "operation-backup-turboquant-activate",
            ),
            401,
        )
        .unwrap();
    assert_eq!(
        active.artifact.state,
        VectorQuantizationArtifactState::Active
    );

    let backup = engine
        .create_instance_backup(
            &lease.session_id,
            &lease.token,
            &id("backup-vector-key"),
            &CreateInstanceBackup {
                label: "vector-complete".into(),
                created_at_unix_ms: 500,
            },
            500,
            "request-backup-vector",
            "operation-backup-vector",
        )
        .unwrap();
    assert_eq!(
        backup.backup.object_payloads,
        BackupCoverageSnapshot::Included
    );
    assert_eq!(
        backup.backup.installed_control,
        BackupCoverageSnapshot::Included
    );
    assert!(backup.backup.application_complete);

    let restore_id = CanonicalId::new("vector-restore").unwrap();
    let restored = engine
        .restore_instance_backup(
            &lease.session_id,
            &lease.token,
            &id("restore-vector-key"),
            &RestoreInstanceBackup {
                backup_sha256: backup.backup.backup_sha256.clone(),
                restore_id: restore_id.clone(),
                restored_at_unix_ms: 600,
            },
            600,
            "request-restore-vector",
            "operation-restore-vector",
        )
        .unwrap();
    assert!(restored.reopened);
    drop(engine);
    std::fs::remove_dir_all(&source_root).unwrap();

    let restored_root = root
        .path()
        .join("rrd-service")
        .join(instance().as_str())
        .join("restores")
        .join(restore_id.as_str());
    let engine = RrdEngine::open(&restored_root, instance(), TOKEN_KEY).unwrap();
    let restored_lease = engine
        .create_session(
            &session_request(9_000, 2),
            &id("restored-vector-session"),
            700,
            "request-restored-session",
            "operation-restored-session",
        )
        .unwrap();
    let result = engine
        .search_vectors(
            &restored_lease.session_id,
            &restored_lease.token,
            &search_request(VectorSearchMode::RequireApproximate {
                exact_rerank: 2,
                ef_search: 2,
            }),
            800,
            "request-restored-search",
            "operation-restored-search",
        )
        .unwrap();
    assert_eq!(result.access_path.as_str(), "turboquant");
    assert_eq!(result.hits[0].reference.id.as_str(), "alpha");
}
