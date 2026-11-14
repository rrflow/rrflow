use super::*;
use crate::{
    HnswConfig, ScoreMetric, SearchMode, TurboQuantBits, TurboQuantSegmentConfig, VectorQuery,
    VectorSegmentConfig,
};
use rrd_core::{
    ReadStamp, RuntimeProperties, RuntimeRef, RuntimeVector, ScopeId, VectorCollectionAddress,
    VectorValue,
};

fn candidate(scope: &ScopeId, cursor: u64, id: &str, values: Vec<f32>) -> VectorCandidate {
    VectorCandidate {
        scope: scope.clone(),
        source_cursor: cursor,
        vector: RuntimeVector {
            reference: RuntimeRef::new("embedding", id).unwrap(),
            subject: RuntimeRef::new("document", id).unwrap(),
            collection: VectorCollectionAddress {
                collection_id: "documents".into(),
                vector_name: "body".into(),
            },
            field: "body".into(),
            valid_from: 1,
            valid_to: None,
            value: VectorValue::Dense { values },
            provenance: None,
            properties: RuntimeProperties::new(),
        },
    }
}

fn request(scope: ScopeId, mode: SearchMode) -> SearchRequest {
    SearchRequest {
        read: ReadStamp::new(scope.clone(), None, 0, 3, Some("11".repeat(32))).unwrap(),
        scope,
        valid_at: 2,
        field: "body".into(),
        query: VectorQuery::Dense {
            values: vec![1.0, 0.0],
        },
        metric: ScoreMetric::Dot,
        embedding_model: None,
        top_k: 1,
        mode,
        filter: None,
    }
}

#[test]
fn planner_decision_is_rechecked_at_execution() {
    let scope = ScopeId::new("instance:vector-runtime").unwrap();
    let values = vec![
        candidate(&scope, 1, "a", vec![1.0, 0.0]),
        candidate(&scope, 2, "b", vec![0.0, 1.0]),
        candidate(&scope, 3, "c", vec![0.5, 0.5]),
    ];
    let mut runtime = VectorRuntime::new(values.clone()).unwrap();
    let hnsw = HnswIndex::build(
        HnswConfig {
            id: ProjectionId::new("vector-hnsw-body").unwrap(),
            scope: scope.clone(),
            field: "body".into(),
            dimensions: 2,
            metric: ScoreMetric::Dot,
            embedding_model: None,
            m: 4,
            ef_construction: 8,
            max_level: 4,
            seed: 9,
            filter_properties: BTreeSet::new(),
        },
        1,
        3,
        values.clone(),
    )
    .unwrap();
    runtime.publish(0, hnsw).unwrap();
    let approximate = runtime
        .search(
            &request(
                scope.clone(),
                SearchMode::RequireApproximate { exact_rerank: 2 },
            ),
            3,
        )
        .unwrap();
    assert_eq!(approximate.plan.selected.kind, AccessPathKind::Hnsw);
    assert_eq!(approximate.hits[0].reference.id.as_str(), "a");
    let prepared_request = request(
        scope.clone(),
        SearchMode::RequireApproximate { exact_rerank: 2 },
    );
    let prepared = runtime.prepare_search(&prepared_request, 3).unwrap();

    let exact = ImmutableVectorSegment::build(
        VectorSegmentConfig {
            // Equal cost breaks by name, and this sorts after the exact scan's.
            id: ProjectionId::new("vector-exact-segment-body").unwrap(),
            scope: scope.clone(),
            field: "body".into(),
            dimensions: 2,
            metric: ScoreMetric::Dot,
            embedding_model: None,
            filter_properties: BTreeSet::new(),
        },
        1,
        3,
        values,
    )
    .unwrap();
    runtime.publish(1, exact).unwrap();
    assert!(runtime
        .execute_search(&prepared_request, &prepared)
        .unwrap_err()
        .to_string()
        .contains("stale catalog revision"));
    let exact = runtime
        .search(&request(scope, SearchMode::Exact), 3)
        .unwrap();
    assert_eq!(exact.plan.selected.kind, AccessPathKind::ExactScan);
    assert_eq!(exact.hits[0].reference.id.as_str(), "a");
}

/// Each accelerator was built while `moved` was in this collection; the
/// canonical candidates no longer hold it here. `kept` has three versions,
/// so a scan costs more than the exact segments and they are planned.
#[test]
fn an_accelerator_never_answers_with_an_identity_that_moved_away() {
    let scope = ScopeId::new("instance:vector-moves").unwrap();
    let canonical = vec![
        candidate(&scope, 2, "kept", vec![1.0, 0.0]),
        candidate(&scope, 3, "kept", vec![0.8, 0.6]),
        candidate(&scope, 4, "kept", vec![0.0, 1.0]),
        candidate(&scope, 5, "other", vec![0.6, 0.8]),
    ];
    let built = vec![
        candidate(&scope, 1, "moved", vec![1.0, 0.0]),
        canonical[2].clone(),
        canonical[3].clone(),
    ];
    let segment = VectorSegmentConfig {
        id: ProjectionId::new("vector-exact-moves").unwrap(),
        scope: scope.clone(),
        field: "body".into(),
        dimensions: 2,
        metric: ScoreMetric::Dot,
        embedding_model: None,
        filter_properties: BTreeSet::new(),
    };
    let artifacts: [(VectorArtifact, SearchMode, AccessPathKind); 3] = [
        (
            ImmutableVectorSegment::build(segment.clone(), 1, 5, built.clone())
                .unwrap()
                .into(),
            SearchMode::Exact,
            AccessPathKind::ExactSegment,
        ),
        (
            CompactDenseSegment::build(segment, 1, 5, built.clone())
                .unwrap()
                .into(),
            SearchMode::Exact,
            AccessPathKind::ExactSegment,
        ),
        (
            HnswIndex::build(
                HnswConfig {
                    id: ProjectionId::new("vector-hnsw-moves").unwrap(),
                    scope: scope.clone(),
                    field: "body".into(),
                    dimensions: 2,
                    metric: ScoreMetric::Dot,
                    embedding_model: None,
                    m: 4,
                    ef_construction: 8,
                    max_level: 4,
                    seed: 9,
                    filter_properties: BTreeSet::new(),
                },
                1,
                5,
                built,
            )
            .unwrap()
            .into(),
            SearchMode::RequireApproximate { exact_rerank: 3 },
            AccessPathKind::Hnsw,
        ),
    ];
    for (artifact, mode, kind) in artifacts {
        let mut runtime = VectorRuntime::new(canonical.clone()).unwrap();
        runtime.publish(0, artifact).unwrap();
        let mut search = request(scope.clone(), mode);
        search.read = ReadStamp::new(scope.clone(), None, 0, 5, Some("11".repeat(32))).unwrap();
        search.top_k = 2;
        let execution = runtime.search(&search, 8).unwrap();
        assert_eq!(execution.plan.selected.kind, kind);
        assert_eq!(
            execution
                .hits
                .iter()
                .map(|hit| (hit.reference.id.as_str(), hit.source_cursor))
                .collect::<Vec<_>>(),
            [("other", 5), ("kept", 4)],
            "{kind:?}"
        );
    }
}

#[test]
fn every_artifact_codec_round_trips_with_exact_descriptor_identity() {
    let scope = ScopeId::new("instance:vector-codecs").unwrap();
    let values = vec![
        candidate(&scope, 1, "a", vec![1.0, 0.0]),
        candidate(&scope, 2, "b", vec![0.0, 1.0]),
    ];
    let segment_config = VectorSegmentConfig {
        id: ProjectionId::new("vector-exact-codec").unwrap(),
        scope: scope.clone(),
        field: "body".into(),
        dimensions: 2,
        metric: ScoreMetric::Dot,
        embedding_model: None,
        filter_properties: BTreeSet::new(),
    };
    let exact = VectorArtifact::from(
        ImmutableVectorSegment::build(segment_config.clone(), 1, 2, values.clone()).unwrap(),
    );
    let compact = VectorArtifact::from(
        CompactDenseSegment::build(segment_config, 1, 2, values.clone()).unwrap(),
    );
    let hnsw = VectorArtifact::from(
        HnswIndex::build(
            HnswConfig {
                id: ProjectionId::new("vector-hnsw-codec").unwrap(),
                scope: scope.clone(),
                field: "body".into(),
                dimensions: 2,
                metric: ScoreMetric::Dot,
                embedding_model: None,
                m: 2,
                ef_construction: 4,
                max_level: 3,
                seed: 11,
                filter_properties: BTreeSet::new(),
            },
            1,
            2,
            values.clone(),
        )
        .unwrap(),
    );
    let turboquant = VectorArtifact::from(
        TurboQuantSegment::build(
            TurboQuantSegmentConfig {
                id: ProjectionId::new("vector-turbo-codec").unwrap(),
                scope,
                field: "body".into(),
                dimensions: 2,
                metric: ScoreMetric::Dot,
                bits: TurboQuantBits::Bits4,
                seed: 11,
                embedding_model: None,
                filter_properties: BTreeSet::new(),
                estimator: crate::TurboQuantEstimator::Mse,
            },
            1,
            2,
            values,
        )
        .unwrap(),
    );

    for artifact in [exact, compact, hnsw, turboquant] {
        let reopened = VectorArtifact::from_bytes(artifact.kind(), artifact.as_bytes()).unwrap();
        assert_eq!(reopened.kind(), artifact.kind());
        assert_eq!(reopened.descriptor(), artifact.descriptor());
        assert_eq!(reopened.as_bytes(), artifact.as_bytes());
    }
}

#[test]
fn turboquant_is_planner_selected_and_exactly_reranked() {
    let scope = ScopeId::new("instance:turbo-runtime").unwrap();
    let values = vec![
        candidate(&scope, 1, "a", vec![1.0, 0.0]),
        candidate(&scope, 2, "b", vec![0.0, 1.0]),
        candidate(&scope, 3, "c", vec![0.5, 0.5]),
    ];
    let mut runtime = VectorRuntime::new(values.clone()).unwrap();
    let artifact = TurboQuantSegment::build(
        TurboQuantSegmentConfig {
            id: ProjectionId::new("vector-turbo-runtime").unwrap(),
            scope: scope.clone(),
            field: "body".into(),
            dimensions: 2,
            metric: ScoreMetric::Dot,
            bits: TurboQuantBits::Bits4,
            seed: 77,
            embedding_model: None,
            filter_properties: BTreeSet::new(),
            estimator: crate::TurboQuantEstimator::Mse,
        },
        1,
        3,
        values,
    )
    .unwrap();
    runtime
        .restore_quantization_lifecycle_active(artifact)
        .unwrap();
    let execution = runtime
        .search(
            &request(scope, SearchMode::RequireApproximate { exact_rerank: 2 }),
            8,
        )
        .unwrap();
    assert_eq!(execution.plan.selected.kind, AccessPathKind::TurboQuant);
    assert_eq!(execution.hits[0].reference.id.as_str(), "a");
    assert_eq!(execution.hits[0].score, 1.0);
}

/// The path a request that allows approximation selects over `canonical`
/// 64-dimensional vectors, with a 4-bit TurboQuant generation that is
/// `overlay` writes behind, and that path's estimated cost.
fn allow_approximate_selection(
    canonical: u64,
    overlay: u64,
    exact_rerank: usize,
) -> (AccessPathKind, u64) {
    const DIMENSIONS: usize = 64;
    let scope = ScopeId::new("instance:turbo-cost").unwrap();
    let values = (1..=canonical)
        .map(|cursor| {
            let vector = (0..DIMENSIONS)
                .map(|dimension| ((cursor as usize * 31 + dimension * 7) % 97) as f32 / 48.0 - 1.0)
                .collect();
            candidate(&scope, cursor, &format!("d{cursor}"), vector)
        })
        .collect::<Vec<_>>();
    let generation_cursor = canonical - overlay;
    let mut runtime = VectorRuntime::new(values.clone()).unwrap();
    let artifact = TurboQuantSegment::build(
        TurboQuantSegmentConfig {
            id: ProjectionId::new("vector-turbo-cost").unwrap(),
            scope: scope.clone(),
            field: "body".into(),
            dimensions: DIMENSIONS,
            metric: ScoreMetric::Dot,
            bits: TurboQuantBits::Bits4,
            seed: 77,
            embedding_model: None,
            filter_properties: BTreeSet::new(),
            estimator: crate::TurboQuantEstimator::Mse,
        },
        1,
        generation_cursor,
        values
            .into_iter()
            .filter(|candidate| candidate.source_cursor <= generation_cursor),
    )
    .unwrap();
    runtime
        .restore_quantization_lifecycle_active(artifact)
        .unwrap();
    let mut search = request(scope.clone(), SearchMode::AllowApproximate { exact_rerank });
    search.read = ReadStamp::new(scope, None, 0, canonical, Some("11".repeat(32))).unwrap();
    search.query = VectorQuery::Dense {
        values: vec![0.25; DIMENSIONS],
    };
    search.top_k = 10;
    let plan = runtime.search(&search, 32).unwrap().plan;
    (plan.selected.kind, plan.selected.estimated_cost)
}

/// Scanning 2,044 codes at the measured 0.23 of an exact score, reranking
/// 32 and scoring a 4-vector overlay is far less work than 2,048 exact
/// scores, so the planner serves the request from TurboQuant. Priced at
/// its candidate count, as before, it tied exact and lost on path id.
#[test]
fn allow_approximate_prefers_turboquant_when_scanning_codes_is_cheaper() {
    assert_eq!(
        allow_approximate_selection(2_048, 4, 32),
        (AccessPathKind::TurboQuant, 471 + 32 + 4)
    );
}

/// With 40 vectors, reranking 32 exactly already costs most of an exact
/// scan, so the planner keeps the exact path.
#[test]
fn allow_approximate_keeps_exact_when_reranking_costs_as_much() {
    assert_eq!(
        allow_approximate_selection(40, 4, 32),
        (AccessPathKind::ExactScan, 40)
    );
}

#[test]
fn generic_publication_rejects_turboquant() {
    let scope = ScopeId::new("instance:turbo-generic-denial").unwrap();
    let values = vec![
        candidate(&scope, 1, "a", vec![1.0, 0.0]),
        candidate(&scope, 2, "b", vec![0.0, 1.0]),
    ];
    let mut runtime = VectorRuntime::new(values.clone()).unwrap();
    let artifact = TurboQuantSegment::build(
        TurboQuantSegmentConfig {
            id: ProjectionId::new("quant-turbo-generic-denial").unwrap(),
            scope,
            field: "body".into(),
            dimensions: 2,
            metric: ScoreMetric::Dot,
            bits: TurboQuantBits::Bits2,
            seed: 17,
            embedding_model: None,
            filter_properties: BTreeSet::new(),
            estimator: crate::TurboQuantEstimator::Mse,
        },
        1,
        2,
        values,
    )
    .unwrap();

    let error = runtime.publish(0, artifact).unwrap_err().to_string();
    assert!(error.contains("quantization lifecycle"));
    assert_eq!(runtime.catalog().revision, 0);
    assert!(runtime.catalog().entries.is_empty());
}

#[test]
fn lifecycle_restore_is_revision_neutral_and_generic_turbo_is_rejected() {
    let scope = ScopeId::new("instance:quantization-authority").unwrap();
    let values = vec![
        candidate(&scope, 1, "a", vec![1.0, 0.0]),
        candidate(&scope, 2, "b", vec![0.0, 1.0]),
    ];
    let mut runtime = VectorRuntime::new(values.clone()).unwrap();
    let hnsw = HnswIndex::build(
        HnswConfig {
            id: ProjectionId::new("vector-hnsw-authority").unwrap(),
            scope: scope.clone(),
            field: "body".into(),
            dimensions: 2,
            metric: ScoreMetric::Dot,
            embedding_model: None,
            m: 2,
            ef_construction: 4,
            max_level: 3,
            seed: 7,
            filter_properties: BTreeSet::new(),
        },
        1,
        2,
        values.clone(),
    )
    .unwrap();
    assert_eq!(runtime.publish(0, hnsw).unwrap(), 1);
    let rejected_generic = TurboQuantSegment::build(
        TurboQuantSegmentConfig {
            id: ProjectionId::new("vector-generic-turbo-authority").unwrap(),
            scope: scope.clone(),
            field: "body".into(),
            dimensions: 2,
            metric: ScoreMetric::Dot,
            bits: TurboQuantBits::Bits2,
            seed: 11,
            embedding_model: None,
            filter_properties: BTreeSet::new(),
            estimator: crate::TurboQuantEstimator::Mse,
        },
        1,
        2,
        values.clone(),
    )
    .unwrap();
    assert!(runtime.publish(1, rejected_generic).is_err());
    assert_eq!(runtime.catalog().revision, 1);

    let lifecycle = TurboQuantSegment::build(
        TurboQuantSegmentConfig {
            id: ProjectionId::new("quant-turbo-authority").unwrap(),
            scope: scope.clone(),
            field: "body".into(),
            dimensions: 2,
            metric: ScoreMetric::Dot,
            bits: TurboQuantBits::Bits2,
            seed: 13,
            embedding_model: None,
            filter_properties: BTreeSet::new(),
            estimator: crate::TurboQuantEstimator::Mse,
        },
        1,
        2,
        values.clone(),
    )
    .unwrap();
    assert_eq!(
        runtime
            .restore_quantization_lifecycle_active(lifecycle)
            .unwrap(),
        1
    );
    assert_eq!(runtime.catalog().revision, 1);

    let exact = ImmutableVectorSegment::build(
        VectorSegmentConfig {
            id: ProjectionId::new("vector-exact-authority").unwrap(),
            scope,
            field: "body".into(),
            dimensions: 2,
            metric: ScoreMetric::Dot,
            embedding_model: None,
            filter_properties: BTreeSet::new(),
        },
        1,
        2,
        values,
    )
    .unwrap();
    assert_eq!(runtime.publish(1, exact).unwrap(), 2);
}
