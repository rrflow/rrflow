//! Physical, bounded and restart-safe vector memory tiers.

use super::*;

#[test]
pub(super) fn engine_vector_memory_tiers_are_physical_bounded_and_restart_safe() {
    let root = tempfile::tempdir().unwrap();
    let limits = crate::VectorResidencyLimits {
        pinned_bytes: 1024 * 1024,
        cached_bytes: 1024 * 1024,
    };
    let engine =
        RrdEngine::open_with_vector_residency(root.path(), instance(), TOKEN_KEY, limits).unwrap();
    let lease = engine
        .create_session(
            &session_request(9_000, 4),
            &id("residency-session"),
            100,
            "request-residency-session",
            "operation-residency-session",
        )
        .unwrap();
    ensure_documents_memory_tier(
        &engine,
        &lease,
        VectorMemoryTier::Cached,
        "residency-cached",
        200,
    );
    commit_vectors(
        &engine,
        &lease,
        300,
        vec![
            TransactionMutation::PutSchema {
                registry: vector_fixture_schema(
                    "install residency fixture schema",
                    DataRecordSchema {
                        allow_additional_properties: true,
                        ..DataRecordSchema::default()
                    },
                ),
            },
            document_mutation("resident-a", "resident alpha"),
            document_mutation("resident-b", "resident beta"),
            vector_mutation("resident-a", vec![1.0, 0.0]),
            vector_mutation("resident-b", vec![0.0, 1.0]),
        ],
    );
    engine
        .ensure_vector_index(
            &lease.session_id,
            &lease.token,
            &index_request(),
            &mutation_context(
                &id("operation-residency-index"),
                "request-residency-index",
                "operation-residency-index",
            ),
            400,
        )
        .unwrap();
    let require_approximate = search_request(VectorSearchMode::RequireApproximate {
        exact_rerank: 2,
        ef_search: 4,
    });

    let cached = engine
        .search_vectors(
            &lease.session_id,
            &lease.token,
            &require_approximate,
            500,
            "request-residency-cached",
            "operation-residency-cached",
        )
        .unwrap();
    assert_eq!(cached.access_path.as_str(), "hnsw");
    let snapshot = engine.vector_residency_snapshot().unwrap();
    assert_eq!(snapshot.cached_entries, 1);
    assert_eq!(snapshot.pinned_entries, 0);
    assert_eq!(snapshot.misses, 1);
    engine
        .search_vectors(
            &lease.session_id,
            &lease.token,
            &require_approximate,
            501,
            "request-residency-cached-hit",
            "operation-residency-cached-hit",
        )
        .unwrap();
    assert_eq!(engine.vector_residency_snapshot().unwrap().cached_hits, 1);

    ensure_documents_memory_tier(
        &engine,
        &lease,
        VectorMemoryTier::Pinned,
        "residency-pinned",
        600,
    );
    engine
        .search_vectors(
            &lease.session_id,
            &lease.token,
            &require_approximate,
            601,
            "request-residency-pinned",
            "operation-residency-pinned",
        )
        .unwrap();
    let snapshot = engine.vector_residency_snapshot().unwrap();
    assert_eq!(snapshot.pinned_entries, 1);
    assert_eq!(snapshot.cached_entries, 0);
    assert_eq!(snapshot.tier_transitions, 1);

    ensure_documents_memory_tier(
        &engine,
        &lease,
        VectorMemoryTier::Cold,
        "residency-cold",
        700,
    );
    let cold = engine
        .search_vectors(
            &lease.session_id,
            &lease.token,
            &require_approximate,
            701,
            "request-residency-cold",
            "operation-residency-cold",
        )
        .unwrap();
    let snapshot = engine.vector_residency_snapshot().unwrap();
    assert_eq!(snapshot.pinned_entries, 0);
    assert_eq!(snapshot.cached_entries, 0);
    assert_eq!(snapshot.tier_transitions, 2);
    assert_eq!(snapshot.cold_owned_loads, 1);
    drop(engine);

    let reopened =
        RrdEngine::open_with_vector_residency(root.path(), instance(), TOKEN_KEY, limits).unwrap();
    let empty = reopened.vector_residency_snapshot().unwrap();
    assert_eq!(empty.pinned_entries, 0);
    assert_eq!(empty.cached_entries, 0);
    let recovered = reopened
        .search_vectors(
            &lease.session_id,
            &lease.token,
            &require_approximate,
            800,
            "request-residency-reopen",
            "operation-residency-reopen",
        )
        .unwrap();
    assert_eq!(recovered.access_path, cold.access_path);
    assert_eq!(recovered.hits, cold.hits);
    assert_eq!(
        reopened
            .vector_residency_snapshot()
            .unwrap()
            .cold_owned_loads,
        1
    );
    ensure_documents_memory_tier(
        &reopened,
        &lease,
        VectorMemoryTier::Pinned,
        "residency-pressure",
        900,
    );
    drop(reopened);

    let pressured = RrdEngine::open_with_vector_residency(
        root.path(),
        instance(),
        TOKEN_KEY,
        crate::VectorResidencyLimits {
            pinned_bytes: 0,
            cached_bytes: 1024 * 1024,
        },
    )
    .unwrap();
    let fallback = pressured
        .search_vectors(
            &lease.session_id,
            &lease.token,
            &search_request(VectorSearchMode::AllowApproximate {
                exact_rerank: 2,
                ef_search: 4,
            }),
            901,
            "request-residency-fallback",
            "operation-residency-fallback",
        )
        .unwrap();
    assert_eq!(fallback.access_path.as_str(), "exact_scan");
    assert!(fallback.exact);
    let error = pressured
        .search_vectors(
            &lease.session_id,
            &lease.token,
            &require_approximate,
            902,
            "request-residency-pressure",
            "operation-residency-pressure",
        )
        .unwrap_err();
    assert!(matches!(error, ServiceError::VectorPressure(_)));
    assert_eq!(error.kind(), crate::ServiceErrorKind::ResourceExhausted);
    assert_eq!(
        pressured
            .vector_residency_snapshot()
            .unwrap()
            .pinned_entries,
        0
    );
}
