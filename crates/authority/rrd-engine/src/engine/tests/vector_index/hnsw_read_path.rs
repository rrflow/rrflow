//! An active HNSW generation serves a search that requires an approximate path from the graph's
//! own candidates and the stamped source delta since it, and reranks exactly. It reads no other
//! vector, and it agrees with exact search on the same stamp.

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

fn ensure_collection(
    engine: &RrdEngine,
    lease: &SessionLease,
    collection: &str,
    field: &str,
    tier: VectorMemoryTier,
) {
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
                    memory_tier: tier,
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

/// Twelve documents in `documents` and `unrelated` vectors in `archive`, with a payload index on
/// `bucket`, and no generation yet.
fn engram(
    unrelated: usize,
    tier: VectorMemoryTier,
) -> (tempfile::TempDir, RrdEngine, SessionLease) {
    let root = tempfile::tempdir().unwrap();
    let engine = RrdEngine::open(root.path(), instance(), TOKEN_KEY).unwrap();
    let lease = engine
        .create_session(
            &session_request(9_000, 8),
            &id("hnsw-read-path-session"),
            100,
            "request-hnsw-read-path-session",
            "operation-hnsw-read-path-session",
        )
        .unwrap();
    ensure_collection(&engine, &lease, "documents", "body-embedding", tier);
    ensure_collection(&engine, &lease, "archive", "archive-embedding", tier);
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
                &id("hnsw-read-path-payload-index"),
                "request-hnsw-read-path-payload-index",
                "operation-hnsw-read-path-payload-index",
            ),
            210,
        )
        .unwrap();
    let mut mutations = vec![TransactionMutation::PutSchema {
        registry: vector_fixture_schema(
            "install HNSW read path fixture",
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
        mutations.push(document_mutation(&id, "read path fixture"));
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
    (root, engine, lease)
}

fn build(engine: &RrdEngine, lease: &SessionLease, key: &str, filter_properties: &[&str]) {
    engine
        .ensure_vector_index(
            &lease.session_id,
            &lease.token,
            &EnsureVectorIndex {
                scope: scope(),
                collection_id: CanonicalId::new("documents").unwrap(),
                vector_name: CanonicalId::new("body").unwrap(),
                configuration: VectorIndexConfiguration::Hnsw {
                    m: 8,
                    ef_construction: 64,
                    max_level: 4,
                    seed: 17,
                    filter_properties: filter_properties
                        .iter()
                        .map(|property| CanonicalId::new(*property).unwrap())
                        .collect(),
                },
                build_policy: VectorIndexBuildPolicy::Cpu,
                max_storage_keys: 100_000,
            },
            &mutation_context(
                &id(&format!("hnsw-read-path-{key}")),
                &format!("request-hnsw-read-path-{key}"),
                &format!("operation-hnsw-read-path-{key}"),
            ),
            400,
        )
        .unwrap();
}

fn fixture(unrelated: usize) -> (tempfile::TempDir, RrdEngine, SessionLease) {
    let (root, engine, lease) = engram(unrelated, VectorMemoryTier::Cached);
    build(&engine, &lease, "build", &["bucket"]);
    (root, engine, lease)
}

/// Writes after the build: a new document, an update visible only from valid time 50, a move to
/// `archive`, and a retirement from valid time 2.
fn change(engine: &RrdEngine, lease: &SessionLease) {
    commit_vectors(
        engine,
        lease,
        500,
        vec![
            document_mutation("d12", "added after the build"),
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
    try_run(engine, lease, request, ordinal)
        .unwrap_or_else(|error| panic!("search {ordinal} failed: {error:?}"))
}

fn try_run(
    engine: &RrdEngine,
    lease: &SessionLease,
    request: &SearchVectors,
    ordinal: u64,
) -> Result<VectorSearchResult, ServiceError> {
    engine.search_vectors(
        &lease.session_id,
        &lease.token,
        request,
        1_000 + ordinal,
        &format!("request-hnsw-read-path-search-{ordinal}"),
        &format!("operation-hnsw-read-path-search-{ordinal}"),
    )
}

fn hit_ids(result: &VectorSearchResult) -> Vec<(String, f64)> {
    result
        .hits
        .iter()
        .map(|hit| (hit.reference.id.as_str().to_owned(), hit.score))
        .collect()
}

/// Every approximate search on the changed engram serves HNSW, and returns exactly what exact
/// search and the general planner return on the same data.
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
        let ordinal = ordinal as u64 * 3;
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
            ordinal + 2,
        );
        assert_eq!(approximate.access_path.as_str(), "hnsw", "case {ordinal}");
        assert!(!approximate.exact);
        assert_eq!(
            hit_ids(&approximate),
            hit_ids(&exact),
            "seed {seed} at valid time {valid_at}"
        );
        assert_eq!(hit_ids(&allowed), hit_ids(&exact), "allowed case {ordinal}");
        assert!(approximate.resources.overlay_candidates > 0);
        // d7 is retired from valid time 2.
        if valid_at >= 2 {
            assert!(hit_ids(&approximate).iter().all(|(id, _)| id != "d7"));
        }
    }
}

#[test]
fn hnsw_keeps_serving_after_writes_and_equals_exact() {
    let (_root, engine, lease) = fixture(4);
    let before = run(
        &engine,
        &lease,
        &search(3, 1, None, APPROXIMATE, 100_000),
        0,
    );
    assert_eq!(before.access_path.as_str(), "hnsw");
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
fn hnsw_never_answers_from_a_collection_an_identity_left() {
    let (_root, engine, lease) = fixture(4);
    change(&engine, &lease);
    // d5 moved to `archive` at the same valid time it held in `documents`.
    let moved = run(
        &engine,
        &lease,
        &search(5, 1, None, APPROXIMATE, 100_000),
        0,
    );
    assert_eq!(moved.access_path.as_str(), "hnsw");
    assert!(
        hit_ids(&moved).iter().all(|(id, _)| id != "d5"),
        "{:?}",
        hit_ids(&moved)
    );
}

#[test]
fn hnsw_answers_survive_reopen() {
    let (root, engine, lease) = fixture(4);
    change(&engine, &lease);
    let request = search(40, 60, None, APPROXIMATE, 100_000);
    let before = run(&engine, &lease, &request, 0);
    drop(engine);
    let reopened = RrdEngine::open(root.path(), instance(), TOKEN_KEY).unwrap();
    let after = run(&reopened, &lease, &request, 1);
    assert_eq!(after.access_path, before.access_path);
    assert_eq!(after.plan_sha256, before.plan_sha256);
    assert_eq!(hit_ids(&after), hit_ids(&before));
    assert_agrees_with_exact(&reopened, &lease);
}

#[test]
fn hnsw_work_stays_flat_as_unrelated_vectors_grow() {
    let measure = |unrelated: usize| {
        let (_root, engine, lease) = fixture(unrelated);
        change(&engine, &lease);
        // A rerank narrower than the generation bounds the hydrated set inside the collection too.
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
        assert_eq!(approximate.access_path.as_str(), "hnsw");
        assert!(
            approximate.resources.canonical_candidates < exact.resources.canonical_candidates,
            "HNSW reranks fewer versions than the collection holds"
        );
        // Every storage path's work, apart from the accumulator proof, whose inclusion paths grow
        // with the logarithm of the engram's leaves.
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
    assert_eq!(small, large, "HNSW reads the same versions and keys");
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
fn hnsw_search_is_bounded_by_its_budget() {
    let (_root, engine, lease) = fixture(4);
    change(&engine, &lease);
    // The catalogue, delta and rerank reads share one budget: exactly the keys a served search
    // examined suffice, and one fewer is refused.
    let served = run(
        &engine,
        &lease,
        &search(3, 1, None, APPROXIMATE, 1_000_000),
        0,
    );
    assert_eq!(served.access_path.as_str(), "hnsw");
    let keys = served.read_evidence.keys_examined;
    let exact_budget = run(&engine, &lease, &search(3, 1, None, APPROXIMATE, keys), 1);
    assert_eq!(hit_ids(&exact_budget), hit_ids(&served));
    assert_eq!(exact_budget.read_evidence.key_budget, keys);
    let error = try_run(
        &engine,
        &lease,
        &search(3, 1, None, APPROXIMATE, keys - 1),
        2,
    )
    .unwrap_err();
    assert_eq!(error.kind(), crate::ServiceErrorKind::ResourceExhausted);
}

#[test]
fn hnsw_read_evidence_names_what_the_search_read() {
    let (_root, engine, lease) = fixture(4);
    change(&engine, &lease);
    let served = run(
        &engine,
        &lease,
        &search(3, 1, None, APPROXIMATE, 1_000_000),
        0,
    );
    let read = |path: ReadAccessPath| {
        served
            .read_evidence
            .paths
            .iter()
            .any(|entry| entry.path == path)
    };
    assert!(
        read(ReadAccessPath::VectorSourceDeltas),
        "the delta is read"
    );
    assert!(read(ReadAccessPath::VectorVersions), "the rerank is read");
    assert!(
        read(ReadAccessPath::AccumulatorProof),
        "every read is proved"
    );
    assert!(served.selected_versions > 0);
    assert!(served.resources.loaded_artifact_bytes > 0);
    assert_eq!(served.resources.selected_generation, Some(1));
}

/// A refusal costs the catalogue and no vector, so it does not depend on how many vectors there
/// are, and a budget far below reading them does not change what the caller is told.
fn assert_refused_without_reading(
    engine: &RrdEngine,
    lease: &SessionLease,
    request: &SearchVectors,
) {
    let error = try_run(engine, lease, request, 900).unwrap_err();
    assert!(
        matches!(&error, ServiceError::Vector(reason) if reason.contains("no vector access path satisfies the request contract")),
        "{error:?}"
    );
}

#[test]
fn a_filter_the_generation_does_not_cover_is_refused_without_reading_vectors() {
    // A generation with no filter coverage over more vectors than the budget could read.
    let (_root, engine, lease) = engram(400, VectorMemoryTier::Cached);
    build(&engine, &lease, "uncovered", &[]);
    assert_refused_without_reading(
        &engine,
        &lease,
        &search(1, 1, Some(bucket(1)), APPROXIMATE, 64),
    );
    // The same request allowed to fall back to an exact scan still answers.
    let allowed = run(
        &engine,
        &lease,
        &search(
            1,
            1,
            Some(bucket(1)),
            VectorSearchMode::AllowApproximate {
                exact_rerank: 32,
                ef_search: 32,
            },
            100_000,
        ),
        1,
    );
    assert_eq!(allowed.access_path.as_str(), "exact_scan");
    assert!(!allowed.hits.is_empty());
}

#[test]
fn no_generation_at_all_is_refused_without_reading_vectors() {
    let (_root, engine, lease) = engram(400, VectorMemoryTier::Cached);
    assert_refused_without_reading(&engine, &lease, &search(1, 1, None, APPROXIMATE, 64));
}

fn activate_turboquant(engine: &RrdEngine, lease: &SessionLease, filter_properties: &[&str]) {
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
                filter_properties: filter_properties
                    .iter()
                    .map(|property| CanonicalId::new(*property).unwrap())
                    .collect(),
                max_storage_keys: 100_000,
            },
            &mutation_context(
                &id("request-hnsw-read-path-turbo-build"),
                "request-hnsw-read-path-turbo-build",
                "operation-hnsw-read-path-turbo-build",
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
                &id("request-hnsw-read-path-turbo-activate"),
                "request-hnsw-read-path-turbo-activate",
                "operation-hnsw-read-path-turbo-activate",
            ),
            410,
        )
        .unwrap();
}

#[test]
fn an_uncovered_filter_on_turboquant_is_refused_without_reading_vectors() {
    // POAM-055's own case: an active TurboQuant generation that cannot cover the filter.
    let (_root, engine, lease) = engram(400, VectorMemoryTier::Cached);
    activate_turboquant(&engine, &lease, &[]);
    assert_refused_without_reading(
        &engine,
        &lease,
        &search(1, 1, Some(bucket(1)), APPROXIMATE, 64),
    );
}

/// `mirror` names the same field as `documents` and holds other vectors, all older than a
/// generation `publish` builds for `documents`, so no overlay can answer for them either.
fn assert_serves_only_its_own_collection(path: &str, publish: impl Fn(&RrdEngine, &SessionLease)) {
    let (_root, engine, lease) = engram(4, VectorMemoryTier::Cached);
    ensure_collection(
        &engine,
        &lease,
        "mirror",
        "body-embedding",
        VectorMemoryTier::Cached,
    );
    let mut mutations = Vec::new();
    for row in 0..6 {
        let id = format!("m{row}");
        mutations.push(document_mutation(&id, "mirror"));
        mutations.push(put(&id, "mirror", "body-embedding", 0, 200 + row, 1));
    }
    commit_vectors(&engine, &lease, 350, mutations);
    commit_vectors(
        &engine,
        &lease,
        360,
        vec![
            document_mutation("d20", "after mirror"),
            put("d20", "documents", "body-embedding", 0, 20, 1),
        ],
    );
    publish(&engine, &lease);

    let in_mirror = |mode| {
        let mut request = search(201, 1, None, mode, 100_000);
        request.collection_id = Some(CanonicalId::new("mirror").unwrap());
        request
    };
    // Nothing serves `mirror` approximately, and a request that requires it is told so.
    match try_run(&engine, &lease, &in_mirror(APPROXIMATE), 0) {
        Err(error) => assert!(
            matches!(&error, ServiceError::Vector(reason) if reason.contains("no vector access path")),
            "{path}: {error:?}"
        ),
        Ok(result) => panic!(
            "{} answered {:?} from a generation that holds none of this collection",
            result.access_path.as_str(),
            hit_ids(&result)
        ),
    }
    // A request that allows approximation still finds what is there.
    let exact = run(&engine, &lease, &in_mirror(VectorSearchMode::Exact), 1);
    assert_eq!(exact.hits[0].reference.id.as_str(), "m1");
    let allowed = run(
        &engine,
        &lease,
        &in_mirror(VectorSearchMode::AllowApproximate {
            exact_rerank: 32,
            ef_search: 32,
        }),
        2,
    );
    assert_eq!(
        hit_ids(&allowed),
        hit_ids(&exact),
        "{path}: {}",
        allowed.access_path
    );
    // The collection the generation was built for still uses it.
    let documents = run(
        &engine,
        &lease,
        &search(3, 1, None, APPROXIMATE, 100_000),
        3,
    );
    assert_eq!(documents.access_path.as_str(), path);
}

#[test]
fn an_hnsw_generation_of_another_collection_does_not_answer_for_this_one() {
    assert_serves_only_its_own_collection("hnsw", |engine, lease| {
        build(engine, lease, "documents-only", &["bucket"]);
    });
}

#[test]
fn a_turboquant_generation_of_another_collection_does_not_answer_for_this_one() {
    assert_serves_only_its_own_collection("turboquant", |engine, lease| {
        activate_turboquant(engine, lease, &[]);
    });
}

#[test]
fn a_generation_over_the_cache_bound_still_serves_without_reading_the_corpus() {
    // Residency decides how the bytes are held, and not what the search reads.
    let root = tempfile::tempdir().unwrap();
    let engine = RrdEngine::open_with_vector_residency(
        root.path(),
        instance(),
        TOKEN_KEY,
        crate::VectorResidencyLimits {
            pinned_bytes: 16 * 1024 * 1024,
            cached_bytes: 1,
        },
    )
    .unwrap();
    let lease = engine
        .create_session(
            &session_request(9_000, 8),
            &id("hnsw-read-path-bypass-session"),
            100,
            "request-hnsw-read-path-bypass-session",
            "operation-hnsw-read-path-bypass-session",
        )
        .unwrap();
    ensure_collection(
        &engine,
        &lease,
        "documents",
        "body-embedding",
        VectorMemoryTier::Cached,
    );
    let mut mutations = vec![TransactionMutation::PutSchema {
        registry: vector_fixture_schema(
            "install bypass fixture",
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
        mutations.push(document_mutation(&id, "bypass"));
        mutations.push(put(&id, "documents", "body-embedding", 0, row, 1));
    }
    commit_vectors(&engine, &lease, 300, mutations);
    build(&engine, &lease, "bypass", &[]);
    let served = run(
        &engine,
        &lease,
        &search(3, 1, None, APPROXIMATE, 100_000),
        0,
    );
    assert_eq!(served.access_path.as_str(), "hnsw");
    assert_eq!(served.hits[0].reference.id.as_str(), "d3");
}

/// Carried from agent/vector-generation-read-path-wip-20261109: two source
/// addresses that share a field must receive distinct HNSW indexes.
#[test]
fn distinct_source_addresses_receive_distinct_hnsw_indexes() {
    let (_root, engine, lease) = engram(0, VectorMemoryTier::Cached);
    let addresses = [("a-b", "c"), ("a", "b-c")];
    let mut indexes = Vec::new();
    for (ordinal, (collection, vector_name)) in addresses.into_iter().enumerate() {
        engine
            .ensure_vector_collection(
                &lease.session_id,
                &lease.token,
                &EnsureVectorCollection {
                    scope: scope(),
                    collection_id: CanonicalId::new(collection).unwrap(),
                    vectors: vec![NamedVectorDefinition {
                        name: CanonicalId::new(vector_name).unwrap(),
                        field: CanonicalId::new("body-embedding").unwrap(),
                        kind: VectorValueKind::Dense,
                        dimensions: DIMENSIONS as u32,
                        metric: VectorSearchMetric::Cosine,
                        embedding_model: None,
                        memory_tier: VectorMemoryTier::Cached,
                    }],
                },
                &mutation_context(
                    &id(&format!("source-collection-{ordinal}")),
                    &format!("request-source-collection-{ordinal}"),
                    &format!("operation-source-collection-{ordinal}"),
                ),
                350 + ordinal as u64,
            )
            .unwrap();
        let result = engine
            .ensure_vector_index(
                &lease.session_id,
                &lease.token,
                &EnsureVectorIndex {
                    scope: scope(),
                    collection_id: CanonicalId::new(collection).unwrap(),
                    vector_name: CanonicalId::new(vector_name).unwrap(),
                    configuration: VectorIndexConfiguration::Hnsw {
                        m: 8,
                        ef_construction: 64,
                        max_level: 4,
                        seed: 17,
                        filter_properties: Vec::new(),
                    },
                    build_policy: VectorIndexBuildPolicy::Cpu,
                    max_storage_keys: 100_000,
                },
                &mutation_context(
                    &id(&format!("source-index-{ordinal}")),
                    &format!("request-source-index-{ordinal}"),
                    &format!("operation-source-index-{ordinal}"),
                ),
                400 + ordinal as u64,
            )
            .unwrap();
        indexes.push(result.index);
    }
    assert_ne!(indexes[0].index_id, indexes[1].index_id);
}

/// Carried from agent/vector-generation-read-path-wip-20261109: the empty
/// foreign-generation oracle that #304 repairs through the source identity.
#[test]
fn an_emptied_hnsw_generation_does_not_answer_for_another_collection() {
    let (_root, engine, lease) = engram(4, VectorMemoryTier::Cached);
    ensure_collection(
        &engine,
        &lease,
        "mirror",
        "body-embedding",
        VectorMemoryTier::Cached,
    );
    commit_vectors(
        &engine,
        &lease,
        350,
        vec![
            document_mutation("mirror-before-exit", "mirror"),
            put("mirror-before-exit", "mirror", "body-embedding", 0, 200, 1),
        ],
    );
    // Moving every source identity at its original valid time leaves no graph nodes,
    // but the generation still covers the later move cursors.
    commit_vectors(
        &engine,
        &lease,
        360,
        (0..DOCUMENTS)
            .map(|row| {
                put(
                    &format!("d{row}"),
                    "archive",
                    "archive-embedding",
                    0,
                    row,
                    1,
                )
            })
            .collect(),
    );
    build(&engine, &lease, "empty-documents", &["bucket"]);

    let mut mirror = search(200, 1, None, VectorSearchMode::Exact, 100_000);
    mirror.collection_id = Some(CanonicalId::new("mirror").unwrap());
    let exact = run(&engine, &lease, &mirror, 0);
    assert_eq!(exact.hits[0].reference.id.as_str(), "mirror-before-exit");
    mirror.mode = APPROXIMATE;
    match try_run(&engine, &lease, &mirror, 1) {
        Err(ServiceError::Vector(reason)) => assert!(reason.contains("no vector access path")),
        Ok(result) => panic!("empty foreign HNSW answered {:?}", hit_ids(&result)),
        Err(error) => panic!("unexpected denial: {error:?}"),
    }
}
