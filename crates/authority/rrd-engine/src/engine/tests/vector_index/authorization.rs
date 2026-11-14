//! Vector administration uses distinct deny-by-default actions.

use super::*;

#[test]
pub(super) fn vector_administration_uses_distinct_deny_by_default_actions() {
    let (_root, engine) = isolated_engine();
    let admin_id = CanonicalId::new("vector-admin").unwrap();
    let limited_id = CanonicalId::new("vector-limited").unwrap();
    let query_only_id = CanonicalId::new("vector-query-only").unwrap();
    let resource = ResourcePath {
        segments: vec![ResourceId::new(ResourceKind::Instance, instance().as_str()).unwrap()],
    };
    let principal =
        |principal_id: CanonicalId, credential: &[u8], actions: &[SecurityAction]| Principal {
            id: principal_id,
            kind: PrincipalKind::Service,
            credential_sha256: digest::sha256_hex(credential),
            credential_revision: 1,
            not_before_unix_ms: 1,
            expires_at_unix_ms: u64::MAX,
            disabled: false,
            role_ids: Default::default(),
            grants: actions
                .iter()
                .copied()
                .map(|action| ResourceGrant {
                    action,
                    resource_prefix: resource.clone(),
                    data_policy: None,
                })
                .collect(),
        };
    SecurityRepository::new(&engine.storage, instance())
        .initialize(
            SecurityState {
                format_version: rrd_security::SECURITY_FORMAT,
                revision: 1,
                principals: BTreeMap::from([
                    (
                        admin_id.clone(),
                        principal(
                            admin_id.clone(),
                            b"vector-admin-key",
                            &[
                                SecurityAction::SessionCreate,
                                SecurityAction::VectorCollectionEnsure,
                                SecurityAction::VectorCollectionList,
                                SecurityAction::VectorCollectionDelete,
                                SecurityAction::VectorPayloadIndexEnsure,
                                SecurityAction::VectorPayloadIndexList,
                                SecurityAction::VectorPayloadIndexDelete,
                            ],
                        ),
                    ),
                    (
                        limited_id.clone(),
                        principal(
                            limited_id.clone(),
                            b"vector-limited-key",
                            &[SecurityAction::SessionCreate],
                        ),
                    ),
                    (
                        query_only_id.clone(),
                        principal(
                            query_only_id.clone(),
                            b"vector-query-only-key",
                            &[SecurityAction::SessionCreate, SecurityAction::QueryExecute],
                        ),
                    ),
                ]),
                roles: Default::default(),
                identity_bindings: Default::default(),
                jwt_issuers: Default::default(),
            },
            1,
            "vector-security-test",
            "request-vector-security",
            "operation-vector-security",
        )
        .unwrap();
    let admin = engine
        .create_authenticated_session(
            &admin_id,
            b"vector-admin-key",
            &session_request(5_000, 2),
            &id("vector-admin-session"),
            100,
            "request-admin-session",
            "operation-admin-session",
        )
        .unwrap();
    let limited = engine
        .create_authenticated_session(
            &limited_id,
            b"vector-limited-key",
            &session_request(5_000, 2),
            &id("vector-limited-session"),
            100,
            "request-limited-session",
            "operation-limited-session",
        )
        .unwrap();
    let query_only = engine
        .create_authenticated_session(
            &query_only_id,
            b"vector-query-only-key",
            &session_request(5_000, 2),
            &id("vector-query-only-session"),
            100,
            "request-query-only-session",
            "operation-query-only-session",
        )
        .unwrap();
    let scope = format!("instance:{}", instance());
    let collection_id = CanonicalId::new("secured").unwrap();
    engine
        .ensure_vector_collection(
            &admin.session_id,
            &admin.token,
            &EnsureVectorCollection {
                scope: scope.clone(),
                collection_id: collection_id.clone(),
                vectors: vec![NamedVectorDefinition {
                    name: CanonicalId::new("dense").unwrap(),
                    field: CanonicalId::new("embedding").unwrap(),
                    kind: VectorValueKind::Dense,
                    dimensions: 2,
                    metric: VectorSearchMetric::Dot,
                    embedding_model: None,
                    memory_tier: VectorMemoryTier::Cached,
                }],
            },
            &mutation_context(
                &id("ensure-secured"),
                "request-ensure-secured",
                "operation-ensure-secured",
            ),
            200,
        )
        .unwrap();
    let quantization_build = BuildVectorQuantizationArtifact {
        scope: scope.clone(),
        collection_id: collection_id.clone(),
        vector_name: CanonicalId::new("dense").unwrap(),
        method: VectorQuantizationMethod::Scalar,
        filter_properties: Vec::new(),
        max_storage_keys: 10,
    };
    assert!(matches!(
        engine.build_vector_quantization_artifact(
            &limited.session_id,
            &limited.token,
            &quantization_build,
            &mutation_context(
                &id("request-limited-quantization-build"),
                "request-limited-quantization-build",
                "operation-limited-quantization-build"
            ),
            201
        ),
        Err(ServiceError::PermissionDenied)
    ));
    let quantization_list = ListVectorQuantizationArtifacts {
        scope: scope.clone(),
        collection_id: Some(collection_id.clone()),
        vector_name: None,
        max_artifacts: 10,
    };
    assert!(matches!(
        engine.list_vector_quantization_artifacts(
            &limited.session_id,
            &limited.token,
            &quantization_list,
            202,
            "request-limited-quantization-list",
            "operation-limited-quantization-list",
        ),
        Err(ServiceError::PermissionDenied)
    ));
    assert!(engine
        .list_vector_quantization_artifacts(
            &admin.session_id,
            &admin.token,
            &quantization_list,
            203,
            "request-admin-quantization-list",
            "operation-admin-quantization-list",
        )
        .unwrap()
        .artifacts
        .is_empty());
    let artifact_id = CanonicalId::new("quant-scalar-secured-dense").unwrap();
    assert!(matches!(
        engine.activate_vector_quantization_artifact(
            &limited.session_id,
            &limited.token,
            &ActivateVectorQuantizationArtifact {
                scope: scope.clone(),
                artifact_id: artifact_id.clone(),
                generation: 1,
            },
            &mutation_context(
                &id("request-limited-quantization-activate"),
                "request-limited-quantization-activate",
                "operation-limited-quantization-activate"
            ),
            204
        ),
        Err(ServiceError::PermissionDenied)
    ));
    assert!(matches!(
        engine.retire_vector_quantization_artifact(
            &limited.session_id,
            &limited.token,
            &RetireVectorQuantizationArtifact {
                scope: scope.clone(),
                artifact_id,
                generation: 1,
            },
            &mutation_context(
                &id("request-limited-quantization-retire"),
                "request-limited-quantization-retire",
                "operation-limited-quantization-retire"
            ),
            205
        ),
        Err(ServiceError::PermissionDenied)
    ));
    let retrieval = ExecuteRetrievalQuery {
        scope: scope.clone(),
        valid_at: 1,
        collection_id: collection_id.clone(),
        query: RetrievalQuery::Nearest {
            using: CanonicalId::new("dense").unwrap(),
            query: VectorSearchQuery::Dense {
                values: vec![1.0, 0.0],
            },
            filter: None,
            mode: VectorSearchMode::Exact,
        },
        result_shape: RetrievalResultShape::Points,
        limit: 1,
        candidate_limit: 1,
        max_storage_keys: 10,
    };
    assert!(matches!(
        engine.execute_retrieval_query(
            &limited.session_id,
            &limited.token,
            &retrieval,
            205,
            "request-limited-retrieval",
            "operation-limited-retrieval",
        ),
        Err(ServiceError::PermissionDenied)
    ));
    assert!(matches!(
        engine.execute_retrieval_query(
            &query_only.session_id,
            &query_only.token,
            &retrieval,
            206,
            "request-query-only-retrieval",
            "operation-query-only-retrieval",
        ),
        Err(ServiceError::PermissionDenied)
    ));
    let ensure_index = EnsureVectorPayloadIndex {
        scope: scope.clone(),
        collection_id: collection_id.clone(),
        field: CanonicalId::new("tenant").unwrap(),
        kind: VectorPayloadIndexKind::Keyword,
    };
    assert!(matches!(
        engine.ensure_vector_payload_index(
            &limited.session_id,
            &limited.token,
            &ensure_index,
            &mutation_context(
                &id("limited-ensure-index"),
                "request-limited-ensure-index",
                "operation-limited-ensure-index",
            ),
            210,
        ),
        Err(ServiceError::PermissionDenied)
    ));
    assert!(matches!(
        engine.list_vector_payload_indexes(
            &limited.session_id,
            &limited.token,
            &ListVectorPayloadIndexes {
                scope: scope.clone(),
                collection_id: collection_id.clone(),
            },
            211,
            "request-limited-list-index",
            "operation-limited-list-index",
        ),
        Err(ServiceError::PermissionDenied)
    ));
    assert!(matches!(
        engine.delete_vector_payload_index(
            &limited.session_id,
            &limited.token,
            &DeleteVectorPayloadIndex {
                scope: scope.clone(),
                collection_id: collection_id.clone(),
                field: CanonicalId::new("tenant").unwrap(),
            },
            &mutation_context(
                &id("limited-delete-index"),
                "request-limited-delete-index",
                "operation-limited-delete-index",
            ),
            212,
        ),
        Err(ServiceError::PermissionDenied)
    ));
    assert!(matches!(
        engine.delete_vector_collection(
            &limited.session_id,
            &limited.token,
            &DeleteVectorCollection {
                scope,
                collection_id,
                valid_at: 1,
                max_storage_keys: 10,
            },
            &mutation_context(
                &id("limited-delete-collection"),
                "request-limited-delete-collection",
                "operation-limited-delete-collection",
            ),
            213,
        ),
        Err(ServiceError::PermissionDenied)
    ));
}
