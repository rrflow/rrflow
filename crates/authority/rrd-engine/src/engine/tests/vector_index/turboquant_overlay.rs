//! E-04b: an active TurboQuant generation keeps serving as vectors change. It
//! serves from its own generation plus the stamped source delta, reranks
//! exactly, and agrees with exact search on the same stamp.

use super::*;
use rrd_contract::{SessionLease, VectorSearchResult};

const DIMENSIONS: usize = 16;
const DOCUMENTS: usize = 12;

fn values(seed: usize) -> Vec<f32> {
    (0..DIMENSIONS)
        .map(|dimension| ((seed * 31 + dimension * 7) % 97) as f32 / 48.0 - 1.0)
        .collect()
}

fn scope() -> String {
    format!("instance:{}", instance())
}

fn put(
    id: &str,
    collection: &str,
    field: &str,
    bucket: u64,
    seed: usize,
    valid_from: u64,
) -> TransactionMutation {
    TransactionMutation::PutVector {
        reference: reference("embedding", id),
        subject: reference("document", id),
        collection_id: CanonicalId::new(collection).unwrap(),
        vector_name: CanonicalId::new("body").unwrap(),
        field: CanonicalId::new(field).unwrap(),
        valid_from,
        valid_to: None,
        value: DataVectorValue::Dense {
            values: values(seed),
        },
        provenance: None,
        properties: DataProperties::from([("bucket".into(), QueryValue::Unsigned(bucket))]),
    }
}

fn ensure_collection(engine: &RrdEngine, lease: &SessionLease, collection: &str, field: &str) {
    engine
        .ensure_vector_collection(
            &lease.session_id,
            &lease.token,
            &EnsureVectorCollection {
                scope: scope(),
                collection_id: CanonicalId::new(collection).unwrap(),
                vectors: vec![NamedVectorDefinition {
                    name: CanonicalId::new("body").unwrap(),
                    field: CanonicalId::new(field).unwrap(),
                    kind: VectorValueKind::Dense,
                    dimensions: DIMENSIONS as u32,
                    metric: VectorSearchMetric::Cosine,
                    embedding_model: None,
                    memory_tier: VectorMemoryTier::Cached,
                }],
            },
            &mutation_context(
                &id(&format!("collection-{collection}")),
                &format!("request-collection-{collection}"),
                &format!("operation-collection-{collection}"),
            ),
            200,
        )
        .unwrap();
}

/// Twelve documents in `documents`, `unrelated` vectors in `archive`, and an
/// active TurboQuant generation over `documents` that covers `bucket`.
fn fixture(unrelated: usize) -> (tempfile::TempDir, RrdEngine, SessionLease) {
    let root = tempfile::tempdir().unwrap();
    let engine = RrdEngine::open(root.path(), instance(), TOKEN_KEY).unwrap();
    let lease = engine
        .create_session(
            &session_request(9_000, 8),
            &id("turboquant-overlay-session"),
            100,
            "request-turboquant-overlay-session",
            "operation-turboquant-overlay-session",
        )
        .unwrap();
    ensure_collection(&engine, &lease, "documents", "body-embedding");
    ensure_collection(&engine, &lease, "archive", "archive-embedding");
    engine
        .ensure_vector_payload_index(
            &lease.session_id,
            &lease.token,
            &EnsureVectorPayloadIndex {
                scope: scope(),
                collection_id: CanonicalId::new("documents").unwrap(),
                field: CanonicalId::new("bucket").unwrap(),
                kind: VectorPayloadIndexKind::Unsigned,
            },
            &mutation_context(
                &id("turboquant-overlay-payload-index"),
                "request-turboquant-overlay-payload-index",
                "operation-turboquant-overlay-payload-index",
            ),
            210,
        )
        .unwrap();
    let mut mutations = vec![TransactionMutation::PutSchema {
        registry: vector_fixture_schema(
            "install TurboQuant overlay fixture",
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
    }];
    for row in 0..DOCUMENTS {
        let id = format!("d{row}");
        mutations.push(document_mutation(&id, "overlay fixture"));
        mutations.push(put(
            &id,
            "documents",
            "body-embedding",
            (row % 3) as u64,
            row,
            1,
        ));
    }
    for row in 0..unrelated {
        let id = format!("u{row}");
        mutations.push(document_mutation(&id, "unrelated"));
        mutations.push(put(&id, "archive", "archive-embedding", 0, 500 + row, 1));
    }
    commit_vectors(&engine, &lease, 300, mutations);
    let built = engine
        .build_vector_quantization_artifact(
            &lease.session_id,
            &lease.token,
            &BuildVectorQuantizationArtifact {
                scope: scope(),
                collection_id: CanonicalId::new("documents").unwrap(),
                vector_name: CanonicalId::new("body").unwrap(),
                method: VectorQuantizationMethod::TurboQuant {
                    bits: VectorQuantizationBits::Bits4,
                    seed: 29,
                    estimator: rrd_contract::VectorQuantizationEstimator::Mse,
                },
                filter_properties: vec![CanonicalId::new("bucket").unwrap()],
                max_storage_keys: 100_000,
            },
            &mutation_context(
                &id("request-turboquant-overlay-build"),
                "request-turboquant-overlay-build",
                "operation-turboquant-overlay-build",
            ),
            400,
        )
        .unwrap();
    engine
        .activate_vector_quantization_artifact(
            &lease.session_id,
            &lease.token,
            &ActivateVectorQuantizationArtifact {
                scope: scope(),
                artifact_id: built.artifact.artifact_id,
                generation: built.artifact.generation,
            },
            &mutation_context(
                &id("request-turboquant-overlay-activate"),
                "request-turboquant-overlay-activate",
                "operation-turboquant-overlay-activate",
            ),
            410,
        )
        .unwrap();
    (root, engine, lease)
}

/// Writes after activation: a new document, an update visible only from
/// valid time 50, a move to `archive`, and a retirement from valid time 2.
fn change(engine: &RrdEngine, lease: &SessionLease) {
    commit_vectors(
        engine,
        lease,
        500,
        vec![
            document_mutation("d12", "added after activation"),
            put("d12", "documents", "body-embedding", 1, 12, 1),
        ],
    );
    commit_vectors(
        engine,
        lease,
        510,
        vec![put("d3", "documents", "body-embedding", 1, 40, 50)],
    );
    commit_vectors(
        engine,
        lease,
        520,
        vec![put("d5", "archive", "archive-embedding", 2, 5, 1)],
    );
    commit_vectors(
        engine,
        lease,
        530,
        vec![TransactionMutation::RetireData {
            model: DataLogicalModel::Vector,
            target: DataTarget::Reference {
                reference: reference("embedding", "d7"),
            },
            effective_at: 2,
        }],
    );
}

fn search(
    seed: usize,
    valid_at: u64,
    filter: Option<VectorPayloadFilter>,
    mode: VectorSearchMode,
    max_storage_keys: u64,
) -> SearchVectors {
    SearchVectors {
        scope: scope(),
        valid_at,
        collection_id: Some(CanonicalId::new("documents").unwrap()),
        vector_name: Some(CanonicalId::new("body").unwrap()),
        field: None,
        query: VectorSearchQuery::Dense {
            values: values(seed),
        },
        filter,
        metric: None,
        top_k: 5,
        mode,
        max_storage_keys,
    }
}

const APPROXIMATE: VectorSearchMode = VectorSearchMode::RequireApproximate {
    exact_rerank: 32,
    ef_search: 32,
};

fn bucket(value: u64) -> VectorPayloadFilter {
    VectorPayloadFilter::Condition {
        condition: VectorPayloadCondition {
            property: CanonicalId::new("bucket").unwrap(),
            operator: VectorPayloadOperator::Equals {
                value: QueryValue::Unsigned(value),
            },
        },
    }
}

fn run(
    engine: &RrdEngine,
    lease: &SessionLease,
    request: &SearchVectors,
    ordinal: u64,
) -> VectorSearchResult {
    engine
        .search_vectors(
            &lease.session_id,
            &lease.token,
            request,
            1_000 + ordinal,
            &format!("request-turboquant-overlay-search-{ordinal}"),
            &format!("operation-turboquant-overlay-search-{ordinal}"),
        )
        .unwrap_or_else(|error| panic!("search {ordinal} failed: {error:?}"))
}

fn hit_ids(result: &VectorSearchResult) -> Vec<(String, f64)> {
    result
        .hits
        .iter()
        .map(|hit| (hit.reference.id.as_str().to_owned(), hit.score))
        .collect()
}

/// Every approximate search on the changed engram serves TurboQuant and
/// returns exactly what exact search returns on the same data.
fn assert_agrees_with_exact(engine: &RrdEngine, lease: &SessionLease) {
    let cases = [
        (12, 1, None),
        (40, 60, None),
        (3, 10, None),
        (5, 1, None),
        (7, 1, None),
        (1, 1, Some(bucket(1))),
        (40, 60, Some(bucket(1))),
    ];
    for (ordinal, (seed, valid_at, filter)) in cases.into_iter().enumerate() {
        let ordinal = ordinal as u64 * 2;
        let exact = run(
            engine,
            lease,
            &search(
                seed,
                valid_at,
                filter.clone(),
                VectorSearchMode::Exact,
                100_000,
            ),
            ordinal,
        );
        let approximate = run(
            engine,
            lease,
            &search(seed, valid_at, filter.clone(), APPROXIMATE, 100_000),
            ordinal + 1,
        );
        // A request that only allows approximation takes the general planner,
        // which may now serve the generation with its overlay too.
        let allowed = run(
            engine,
            lease,
            &search(
                seed,
                valid_at,
                filter,
                VectorSearchMode::AllowApproximate {
                    exact_rerank: 32,
                    ef_search: 32,
                },
                100_000,
            ),
            ordinal + 100,
        );
        assert_eq!(hit_ids(&allowed), hit_ids(&exact), "allowed case {ordinal}");
        assert_eq!(
            approximate.access_path.as_str(),
            "turboquant",
            "case {ordinal}"
        );
        assert!(!approximate.exact);
        assert_eq!(
            hit_ids(&approximate),
            hit_ids(&exact),
            "seed {seed} at valid time {valid_at}"
        );
        assert!(approximate.resources.overlay_candidates > 0);
        // d7 is retired from valid time 2.
        if valid_at >= 2 {
            assert!(hit_ids(&approximate).iter().all(|(id, _)| id != "d7"));
        }
        // d5 moved to `archive`. Exact search still returns its old version
        // from `documents` (POAM-045), and TurboQuant agrees with it above;
        // the move's own correction belongs to that record's package.
    }
}

#[test]
fn turboquant_keeps_serving_after_writes_and_equals_exact() {
    let (_root, engine, lease) = fixture(4);
    let before = run(
        &engine,
        &lease,
        &search(3, 1, None, APPROXIMATE, 100_000),
        0,
    );
    assert_eq!(before.access_path.as_str(), "turboquant");
    assert_eq!(before.resources.overlay_candidates, 0);
    change(&engine, &lease);
    assert_agrees_with_exact(&engine, &lease);
    // The new document is found from the overlay with exact scoring.
    let new = run(
        &engine,
        &lease,
        &search(12, 1, None, APPROXIMATE, 100_000),
        100,
    );
    assert_eq!(new.hits[0].reference.id.as_str(), "d12");
    // Before its valid time, d3 is still its original version.
    let early = run(
        &engine,
        &lease,
        &search(3, 10, None, APPROXIMATE, 100_000),
        101,
    );
    assert_eq!(early.hits[0].reference.id.as_str(), "d3");
}

#[test]
fn turboquant_answers_survive_reopen() {
    let (root, engine, lease) = fixture(4);
    change(&engine, &lease);
    let request = search(40, 60, None, APPROXIMATE, 100_000);
    let before = run(&engine, &lease, &request, 0);
    drop(engine);
    let reopened = RrdEngine::open(root.path(), instance(), TOKEN_KEY).unwrap();
    let after = run(&reopened, &lease, &request, 1);
    assert_eq!(after.access_path, before.access_path);
    assert_eq!(hit_ids(&after), hit_ids(&before));
    assert_agrees_with_exact(&reopened, &lease);
}

#[test]
fn turboquant_work_stays_flat_as_unrelated_vectors_grow() {
    let measure = |unrelated: usize| {
        let (_root, engine, lease) = fixture(unrelated);
        change(&engine, &lease);
        // A rerank narrower than the generation bounds the hydrated set
        // inside the collection too.
        let narrow = VectorSearchMode::RequireApproximate {
            exact_rerank: 6,
            ef_search: 32,
        };
        let approximate = run(&engine, &lease, &search(3, 1, None, narrow, 1_000_000), 0);
        let exact = run(
            &engine,
            &lease,
            &search(3, 1, None, VectorSearchMode::Exact, 1_000_000),
            1,
        );
        assert_eq!(approximate.access_path.as_str(), "turboquant");
        assert!(
            approximate.resources.canonical_candidates < exact.resources.canonical_candidates,
            "TurboQuant reranks fewer versions than the collection holds"
        );
        // Every storage path's work, apart from the accumulator proof, whose
        // inclusion paths grow with the logarithm of the engram's leaves.
        let paths = approximate
            .read_evidence
            .paths
            .iter()
            .filter(|path| path.path != ReadAccessPath::AccumulatorProof)
            .map(|path| {
                (
                    path.path,
                    path.point_reads,
                    path.range_scans,
                    path.keys_examined,
                    path.values_decoded,
                )
            })
            .collect::<Vec<_>>();
        let proof = approximate
            .read_evidence
            .paths
            .iter()
            .find(|path| path.path == ReadAccessPath::AccumulatorProof)
            .map_or(0, |path| path.keys_examined);
        (
            (
                approximate.selected_versions,
                approximate.resources.canonical_candidates,
                paths,
            ),
            proof,
            (exact.selected_versions, exact.read_evidence.keys_examined),
        )
    };
    let (small, small_proof, small_exact) = measure(4);
    let (large, large_proof, large_exact) = measure(80);
    assert_eq!(small, large, "TurboQuant reads the same versions and keys");
    assert!(
        large_exact.0 > small_exact.0,
        "the exact scan hydrates more"
    );
    assert!(
        large_proof - small_proof < large_exact.1 - small_exact.1,
        "only the logarithmic proof grows, by less than the exact scan"
    );
}

#[test]
fn turboquant_search_is_bounded_by_its_budget() {
    let (_root, engine, lease) = fixture(4);
    change(&engine, &lease);
    // The catalogue, delta and rerank reads share one budget: exactly the
    // keys a served search examined suffice, and one fewer is refused.
    let served = run(
        &engine,
        &lease,
        &search(3, 1, None, APPROXIMATE, 1_000_000),
        0,
    );
    assert_eq!(served.access_path.as_str(), "turboquant");
    let keys = served.read_evidence.keys_examined;
    let exact_budget = run(&engine, &lease, &search(3, 1, None, APPROXIMATE, keys), 1);
    assert_eq!(hit_ids(&exact_budget), hit_ids(&served));
    assert_eq!(exact_budget.read_evidence.key_budget, keys);
    let error = engine
        .search_vectors(
            &lease.session_id,
            &lease.token,
            &search(3, 1, None, APPROXIMATE, keys - 1),
            2_000,
            "request-turboquant-overlay-budget",
            "operation-turboquant-overlay-budget",
        )
        .unwrap_err();
    assert_eq!(error.kind(), crate::ServiceErrorKind::ResourceExhausted);
}

#[test]
fn unsupported_filter_is_rejected_before_vector_versions_are_read() {
    for unrelated in [0, 64] {
        let (_root, engine, _lease) = fixture(unrelated);
        let scope_id = rrd_core::ScopeId::new(scope()).unwrap();
        let read = engine.storage.runtime().read_stamp(&scope_id).unwrap();
        let catalogue = crate::runtime::vector_catalog::read_vector_catalogue_versions(
            &engine.storage,
            &read,
            &[],
            rrd_store::RuntimeReadBudget::new(100_000).unwrap(),
        )
        .unwrap();
        let budget = catalogue.evidence.keys_examined;
        assert!(budget > 0);
        let unsupported = VectorPayloadFilter::Condition {
            condition: VectorPayloadCondition {
                property: CanonicalId::new("unindexed").unwrap(),
                operator: VectorPayloadOperator::Equals {
                    value: QueryValue::Unsigned(1),
                },
            },
        };
        let request = search(1, 2, Some(unsupported), APPROXIMATE, budget);
        let error = engine.search_vectors_at(&request, read).unwrap_err();
        assert!(
            matches!(error, ServiceError::Vector(ref reason)
                if reason.contains("no vector access path satisfies the request contract")),
            "expected no-path refusal with {unrelated} unrelated vectors, got {error:?}"
        );
    }
}
