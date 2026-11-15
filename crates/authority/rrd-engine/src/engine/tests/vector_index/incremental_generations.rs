//! E-04c: a TurboQuant build extends the newest published generation of its
//! configuration with the vectors changed since it. The generation it
//! publishes is byte for byte the one a full build makes from every version,
//! its source-version reads and newly encoded versions follow the change, and
//! a build without a sound base encodes every version. The inner-product
//! estimator (E-04d) goes through the same lifecycle as its own configuration.

use super::*;
use crate::engine::vector::{read_vector_versions, source_history};
use rrd_contract::{BuildVectorQuantizationArtifactResult, SessionLease, VectorSearchResult};
use rrd_store::{DataRuntimeAccess, ImmutableObjectStore};
use std::collections::BTreeSet;

const DIMENSIONS: usize = 16;
const DOCUMENTS: usize = 12;
const SEED: u64 = 29;
/// The identities `change` touches in `documents`.
const CHANGED: [&str; 4] = ["added", "d3", "d5", "d7"];

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
        properties: DataProperties::new(),
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

/// `documents` holds `documents` vectors and `archive` holds `unrelated`.
fn fixture(documents: usize, unrelated: usize) -> (tempfile::TempDir, RrdEngine, SessionLease) {
    let root = tempfile::tempdir().unwrap();
    let engine = RrdEngine::open(root.path(), instance(), TOKEN_KEY).unwrap();
    let lease = engine
        .create_session(
            &session_request(9_000, 8),
            &id("incremental-generation-session"),
            100,
            "request-incremental-generation-session",
            "operation-incremental-generation-session",
        )
        .unwrap();
    ensure_collection(&engine, &lease, "documents", "body-embedding");
    ensure_collection(&engine, &lease, "archive", "archive-embedding");
    let mut mutations = vec![TransactionMutation::PutSchema {
        registry: vector_fixture_schema(
            "install incremental generation fixture",
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
    for row in 0..documents {
        let id = format!("d{row}");
        mutations.push(document_mutation(&id, "incremental fixture"));
        mutations.push(put(&id, "documents", "body-embedding", row, 1));
    }
    for row in 0..unrelated {
        let id = format!("u{row}");
        mutations.push(document_mutation(&id, "unrelated"));
        mutations.push(put(&id, "archive", "archive-embedding", 500 + row, 1));
    }
    commit_vectors(&engine, &lease, 300, mutations);
    (root, engine, lease)
}

/// A new document, an update visible from valid time 50, a move to
/// `archive`, and a retirement from valid time 2.
fn change(engine: &RrdEngine, lease: &SessionLease, at: u64) {
    commit_vectors(
        engine,
        lease,
        at,
        vec![
            document_mutation("added", "added after the first generation"),
            put("added", "documents", "body-embedding", 12, 1),
        ],
    );
    commit_vectors(
        engine,
        lease,
        at + 10,
        vec![put("d3", "documents", "body-embedding", 40, 50)],
    );
    commit_vectors(
        engine,
        lease,
        at + 20,
        vec![put("d5", "archive", "archive-embedding", 5, 1)],
    );
    commit_vectors(
        engine,
        lease,
        at + 30,
        vec![TransactionMutation::RetireData {
            model: DataLogicalModel::Vector,
            target: DataTarget::Reference {
                reference: reference("embedding", "d7"),
            },
            effective_at: 2,
        }],
    );
}

fn build_request(
    seed: u64,
    bits: VectorQuantizationBits,
    max_storage_keys: u64,
) -> BuildVectorQuantizationArtifact {
    BuildVectorQuantizationArtifact {
        scope: scope(),
        collection_id: CanonicalId::new("documents").unwrap(),
        vector_name: CanonicalId::new("body").unwrap(),
        method: VectorQuantizationMethod::TurboQuant {
            bits,
            seed,
            estimator: rrd_contract::VectorQuantizationEstimator::Mse,
        },
        filter_properties: Vec::new(),
        max_storage_keys,
    }
}

fn try_build(
    engine: &RrdEngine,
    lease: &SessionLease,
    request: &BuildVectorQuantizationArtifact,
    key: &str,
    now: u64,
) -> crate::Result<BuildVectorQuantizationArtifactResult> {
    engine.build_vector_quantization_artifact(
        &lease.session_id,
        &lease.token,
        request,
        &mutation_context(
            &id(&format!("build-{key}")),
            &format!("request-build-{key}"),
            &format!("operation-build-{key}"),
        ),
        now,
    )
}

fn build(
    engine: &RrdEngine,
    lease: &SessionLease,
    seed: u64,
    bits: VectorQuantizationBits,
    key: &str,
    now: u64,
) -> BuildVectorQuantizationArtifactResult {
    try_build(
        engine,
        lease,
        &build_request(seed, bits, 1_000_000),
        key,
        now,
    )
    .unwrap_or_else(|error| panic!("build {key} failed: {error:?}"))
}

fn activate(
    engine: &RrdEngine,
    lease: &SessionLease,
    built: &BuildVectorQuantizationArtifactResult,
    now: u64,
) {
    engine
        .activate_vector_quantization_artifact(
            &lease.session_id,
            &lease.token,
            &ActivateVectorQuantizationArtifact {
                scope: scope(),
                artifact_id: built.artifact.artifact_id.clone(),
                generation: built.artifact.generation,
            },
            &mutation_context(
                &id(&format!("activate-{}", built.artifact.generation)),
                &format!("request-activate-{}", built.artifact.generation),
                &format!("operation-activate-{}", built.artifact.generation),
            ),
            now,
        )
        .unwrap();
}

fn lifecycle(engine: &RrdEngine) -> rrd_vector::QuantizationArtifactCatalogue {
    let scope = engine.query_scope(&scope()).unwrap();
    crate::quantization_artifact_catalogue(&engine.storage, &scope).unwrap()
}

/// The published TurboQuant generation, decoded from its object.
fn published(engine: &RrdEngine, generation: u64) -> rrd_vector::TurboQuantSegment {
    let catalogue = lifecycle(engine);
    let entry = &catalogue
        .artifacts
        .iter()
        .find(|((_, built), _)| *built == generation)
        .expect("the generation is published")
        .1
        .entry;
    let bytes = engine.objects.get(&entry.object).unwrap();
    rrd_vector::TurboQuantSegment::from_bytes(&bytes).unwrap()
}

/// What a full build makes now, under `generation`'s configuration and
/// generation number, from every version of `documents.body` in the engram.
/// Returns it with the identity and cursor of each version it encodes.
fn full_build(
    engine: &RrdEngine,
    generation: &rrd_vector::TurboQuantSegment,
) -> (rrd_vector::TurboQuantSegment, Vec<(String, u64)>) {
    let scope = engine.query_scope(&scope()).unwrap();
    let read = engine.storage.runtime().read_stamp(&scope).unwrap();
    let direct = read_vector_versions(engine, &read, 1_000_000).unwrap();
    let rrd_vector::SourceHistory {
        versions: candidates,
        source_cursor,
    } = source_history(
        rrd_vector::candidates_from_changes(&direct.changes, &scope),
        &CanonicalId::new("documents").unwrap(),
        &CanonicalId::new("body").unwrap(),
    );
    let versions = candidates
        .iter()
        .map(|candidate| {
            (
                candidate.vector.reference.id.to_string(),
                candidate.source_cursor,
            )
        })
        .collect();
    let descriptor = generation.descriptor();
    let config = rrd_vector::TurboQuantSegmentConfig {
        id: descriptor.stamp.id.clone(),
        scope: scope.clone(),
        field: descriptor.field.clone(),
        dimensions: descriptor.dimensions,
        metric: descriptor.metric,
        bits: descriptor.bits,
        seed: descriptor.seed,
        embedding_model: descriptor.embedding_model.clone(),
        filter_properties: descriptor.filter_properties.clone(),
        estimator: descriptor.estimator,
    };
    let full = rrd_vector::TurboQuantSegment::build(
        config,
        descriptor.stamp.generation,
        source_cursor,
        candidates,
    )
    .unwrap();
    (full, versions)
}

/// The published generation equals the full build of the same versions.
fn assert_equals_full_build(engine: &RrdEngine, generation: u64) -> Vec<(String, u64)> {
    let published = published(engine, generation);
    let (full, versions) = full_build(engine, &published);
    assert_eq!(
        published.descriptor(),
        full.descriptor(),
        "generation {generation}"
    );
    assert!(
        published.as_bytes() == full.as_bytes(),
        "generation {generation} differs from a full build"
    );
    versions
}

fn changed_versions(versions: &[(String, u64)]) -> u64 {
    versions
        .iter()
        .filter(|(id, _)| CHANGED.contains(&id.as_str()))
        .count() as u64
}

fn search(seed: usize, valid_at: u64, mode: VectorSearchMode) -> SearchVectors {
    SearchVectors {
        scope: scope(),
        valid_at,
        collection_id: Some(CanonicalId::new("documents").unwrap()),
        vector_name: Some(CanonicalId::new("body").unwrap()),
        field: None,
        query: VectorSearchQuery::Dense {
            values: values(seed),
        },
        filter: None,
        metric: None,
        top_k: 5,
        mode,
        max_storage_keys: 1_000_000,
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
            5_000 + ordinal,
            &format!("request-incremental-search-{ordinal}"),
            &format!("operation-incremental-search-{ordinal}"),
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

/// TurboQuant serves the active generation with no overlay and agrees with
/// exact search, at the current valid time and before the update.
fn assert_serves_exactly(engine: &RrdEngine, lease: &SessionLease, from: u64) {
    for (offset, (seed, valid_at)) in [(12, 1), (40, 60), (3, 10), (1, 1)].into_iter().enumerate() {
        let ordinal = from + offset as u64 * 2;
        let exact = run(
            engine,
            lease,
            &search(seed, valid_at, VectorSearchMode::Exact),
            ordinal,
        );
        let approximate = run(
            engine,
            lease,
            &search(
                seed,
                valid_at,
                VectorSearchMode::RequireApproximate {
                    exact_rerank: 32,
                    ef_search: 32,
                },
            ),
            ordinal + 1,
        );
        assert_eq!(approximate.access_path.as_str(), "turboquant");
        assert_eq!(approximate.resources.overlay_candidates, 0);
        assert_eq!(
            hit_ids(&approximate),
            hit_ids(&exact),
            "seed {seed} at {valid_at}"
        );
    }
}

#[test]
fn an_extended_generation_equals_a_full_build_and_serves() {
    let (root, engine, lease) = fixture(DOCUMENTS, 4);
    let first = build(
        &engine,
        &lease,
        SEED,
        VectorQuantizationBits::Bits4,
        "first",
        400,
    );
    assert_eq!(first.base_generation, None);
    assert_eq!(first.encoded_versions, DOCUMENTS as u64);
    assert_equals_full_build(&engine, 1);

    change(&engine, &lease, 500);
    let second = build(
        &engine,
        &lease,
        SEED,
        VectorQuantizationBits::Bits4,
        "second",
        600,
    );
    assert_eq!(second.artifact.generation, 2);
    assert_eq!(second.base_generation, Some(1));
    let versions = assert_equals_full_build(&engine, 2);
    // Only the changed identities' histories in `documents` were encoded.
    assert_eq!(second.encoded_versions, changed_versions(&versions));
    assert!(second.encoded_versions < versions.len() as u64);
    activate(&engine, &lease, &second, 610);
    assert_serves_exactly(&engine, &lease, 0);

    // After reopen the published generation is the base, and extending it
    // stays exact.
    drop(engine);
    let engine = RrdEngine::open(root.path(), instance(), TOKEN_KEY).unwrap();
    commit_vectors(
        &engine,
        &lease,
        700,
        vec![put("d1", "documents", "body-embedding", 77, 1)],
    );
    let third = build(
        &engine,
        &lease,
        SEED,
        VectorQuantizationBits::Bits4,
        "third",
        710,
    );
    assert_eq!(third.base_generation, Some(2));
    let versions = assert_equals_full_build(&engine, 3);
    assert_eq!(
        third.encoded_versions,
        versions.iter().filter(|(id, _)| id == "d1").count() as u64
    );
    activate(&engine, &lease, &third, 720);
    assert_serves_exactly(&engine, &lease, 100);
}

/// POAM-048: `d3` leaves `documents` from valid time 50, which the base sees,
/// then changes in `archive` from valid time 10. That second version follows
/// one outside `documents`, so the store records no change to `documents`
/// and the build does not re-encode `d3`; a full build must hold the same.
#[test]
fn a_change_outside_the_source_after_a_move_leaves_the_extension_exact() {
    let (_root, engine, lease) = fixture(DOCUMENTS, 0);
    commit_vectors(
        &engine,
        &lease,
        400,
        vec![put("d3", "archive", "archive-embedding", 40, 50)],
    );
    let first = build(
        &engine,
        &lease,
        SEED,
        VectorQuantizationBits::Bits4,
        "first",
        410,
    );
    assert_eq!(first.base_generation, None);
    assert_equals_full_build(&engine, 1);

    commit_vectors(
        &engine,
        &lease,
        500,
        vec![put("d3", "archive", "archive-embedding", 41, 10)],
    );
    commit_vectors(
        &engine,
        &lease,
        510,
        vec![put("d5", "documents", "body-embedding", 55, 1)],
    );
    let second = build(
        &engine,
        &lease,
        SEED,
        VectorQuantizationBits::Bits4,
        "second",
        600,
    );
    assert_eq!(second.base_generation, Some(1));
    assert_equals_full_build(&engine, 2);
}

#[test]
fn an_extended_build_reads_and_encodes_only_the_change() {
    let measure = |documents: usize, unrelated: usize| {
        let (_root, engine, lease) = fixture(documents, unrelated);
        build(
            &engine,
            &lease,
            SEED,
            VectorQuantizationBits::Bits4,
            "first",
            400,
        );
        change(&engine, &lease, 500);
        let extended = build(
            &engine,
            &lease,
            SEED,
            VectorQuantizationBits::Bits4,
            "second",
            600,
        );
        assert_eq!(extended.base_generation, Some(1));
        assert_equals_full_build(&engine, 2);
        // Another seed has no base, so it encodes every version.
        let full = build(
            &engine,
            &lease,
            SEED + 1,
            VectorQuantizationBits::Bits4,
            "full",
            610,
        );
        assert_eq!(full.base_generation, None);
        // Every storage path's work, apart from the accumulator proof, whose
        // inclusion paths grow with the logarithm of the engram's leaves.
        let paths = extended
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
                )
            })
            .collect::<Vec<_>>();
        (
            (extended.encoded_versions, extended.selected_versions, paths),
            (full.encoded_versions, full.read_evidence.keys_examined),
        )
    };
    let (small, small_full) = measure(DOCUMENTS, 4);
    let (large, large_full) = measure(DOCUMENTS * 10, 80);
    assert_eq!(small, large, "source reads and encoding follow the change");
    assert!(large_full.0 > small_full.0, "a full build encodes more");
    assert!(large_full.1 > small_full.1, "a full build reads more");
    assert!(small.0 < small_full.0);
}

#[test]
fn only_a_generation_of_the_same_configuration_is_a_base() {
    let (_root, engine, lease) = fixture(DOCUMENTS, 4);
    build(
        &engine,
        &lease,
        SEED,
        VectorQuantizationBits::Bits4,
        "seed",
        400,
    );
    let reseeded = build(
        &engine,
        &lease,
        SEED + 1,
        VectorQuantizationBits::Bits4,
        "reseeded",
        410,
    );
    assert_eq!(reseeded.base_generation, None);
    assert_eq!(reseeded.encoded_versions, DOCUMENTS as u64);
    let narrower = build(
        &engine,
        &lease,
        SEED,
        VectorQuantizationBits::Bits2,
        "narrower",
        420,
    );
    assert_eq!(narrower.base_generation, None);
    assert_equals_full_build(&engine, 3);
    // The newest generation with the requested configuration is the base,
    // though newer generations of other configurations exist.
    change(&engine, &lease, 500);
    let again = build(
        &engine,
        &lease,
        SEED,
        VectorQuantizationBits::Bits4,
        "again",
        600,
    );
    assert_eq!(again.artifact.generation, 4);
    assert_eq!(again.base_generation, Some(1));
    assert_equals_full_build(&engine, 4);
}

#[test]
fn a_base_whose_object_does_not_verify_is_not_extended() {
    let (root, engine, lease) = fixture(DOCUMENTS, 4);
    build(
        &engine,
        &lease,
        SEED,
        VectorQuantizationBits::Bits4,
        "first",
        400,
    );
    let catalogue = lifecycle(&engine);
    let sha256 = catalogue
        .artifacts
        .values()
        .next()
        .unwrap()
        .entry
        .object
        .sha256
        .clone();
    drop(engine);
    // Damage the published generation's object on disk.
    let mut damaged = 0;
    let mut pending = vec![root.path().to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(&directory).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                pending.push(path);
            } else if path
                .to_string_lossy()
                .replace(['/', '\\'], "")
                .contains(&sha256)
            {
                let mut bytes = std::fs::read(&path).unwrap();
                let last = bytes.len() - 1;
                bytes[last] ^= 0xff;
                std::fs::write(&path, bytes).unwrap();
                damaged += 1;
            }
        }
    }
    assert_eq!(damaged, 1, "the generation's object is one file");
    let engine = RrdEngine::open(root.path(), instance(), TOKEN_KEY).unwrap();
    change(&engine, &lease, 500);
    let rebuilt = build(
        &engine,
        &lease,
        SEED,
        VectorQuantizationBits::Bits4,
        "rebuilt",
        600,
    );
    assert_eq!(
        rebuilt.base_generation, None,
        "a damaged base is not extended"
    );
    let versions = assert_equals_full_build(&engine, 2);
    assert_eq!(rebuilt.encoded_versions, versions.len() as u64);
}

#[test]
fn an_interrupted_build_leaves_nothing_to_extend() {
    let (root, engine, lease) = fixture(DOCUMENTS, 4);
    build(
        &engine,
        &lease,
        SEED,
        VectorQuantizationBits::Bits4,
        "first",
        400,
    );
    change(&engine, &lease, 500);
    // A build that staged its object and stopped before publishing it.
    let catalogue = lifecycle(&engine);
    let entry = &catalogue.artifacts.values().next().unwrap().entry;
    let staged = rrd_store::DataRuntimeRef::new(&engine.storage, &engine.objects)
        .stage_object(
            format!("{}.2.quantized-bytes", entry.descriptor.stamp().id),
            None,
            entry.object.media_type.clone(),
            b"an interrupted build's partial bytes",
        )
        .unwrap();
    assert_ne!(staged.sha256, entry.object.sha256);
    drop(engine);
    let engine = RrdEngine::open(root.path(), instance(), TOKEN_KEY).unwrap();
    let next = build(
        &engine,
        &lease,
        SEED,
        VectorQuantizationBits::Bits4,
        "next",
        600,
    );
    assert_eq!(next.artifact.generation, 2);
    assert_eq!(next.base_generation, Some(1));
    assert_equals_full_build(&engine, 2);
}

#[test]
fn racing_builds_publish_only_generations_equal_to_a_full_build() {
    let (_root, engine, lease) = fixture(DOCUMENTS, 4);
    build(
        &engine,
        &lease,
        SEED,
        VectorQuantizationBits::Bits4,
        "first",
        400,
    );
    change(&engine, &lease, 500);
    let results = std::thread::scope(|threads| {
        let racers = ["race-a", "race-b"].map(|key| {
            let engine = &engine;
            let lease = &lease;
            threads.spawn(move || {
                try_build(
                    engine,
                    lease,
                    &build_request(SEED, VectorQuantizationBits::Bits4, 1_000_000),
                    key,
                    600,
                )
            })
        });
        racers.map(|racer| racer.join().unwrap())
    });
    let published = results
        .iter()
        .filter_map(|result| result.as_ref().ok())
        .map(|built| built.artifact.generation)
        .collect::<BTreeSet<_>>();
    assert!(
        !published.is_empty(),
        "a racing build publishes: {results:?}"
    );
    assert_eq!(
        published.len(),
        results.iter().filter(|result| result.is_ok()).count(),
        "no generation is published twice"
    );
    for generation in &published {
        assert_equals_full_build(&engine, *generation);
    }
    for refused in results.iter().filter_map(|result| result.as_ref().err()) {
        assert!(
            refused.to_string().contains("already built"),
            "a racing build that loses is refused: {refused:?}"
        );
    }
}

#[test]
fn an_extended_build_is_bounded_by_its_budget() {
    let measured = {
        let (_root, engine, lease) = fixture(DOCUMENTS, 4);
        build(
            &engine,
            &lease,
            SEED,
            VectorQuantizationBits::Bits4,
            "first",
            400,
        );
        change(&engine, &lease, 500);
        build(
            &engine,
            &lease,
            SEED,
            VectorQuantizationBits::Bits4,
            "second",
            600,
        )
    };
    let keys = measured.read_evidence.keys_examined;
    let (_root, engine, lease) = fixture(DOCUMENTS, 4);
    build(
        &engine,
        &lease,
        SEED,
        VectorQuantizationBits::Bits4,
        "first",
        400,
    );
    change(&engine, &lease, 500);
    let error = try_build(
        &engine,
        &lease,
        &build_request(SEED, VectorQuantizationBits::Bits4, keys - 1),
        "short",
        600,
    )
    .unwrap_err();
    assert_eq!(error.kind(), crate::ServiceErrorKind::ResourceExhausted);
    let exact = try_build(
        &engine,
        &lease,
        &build_request(SEED, VectorQuantizationBits::Bits4, keys),
        "exact",
        610,
    )
    .unwrap();
    assert_eq!(exact.base_generation, Some(1));
    assert_eq!(exact.read_evidence.keys_examined, keys);
    assert_eq!(exact.read_evidence.key_budget, keys);
}

fn inner_product_request(seed: u64) -> BuildVectorQuantizationArtifact {
    let mut request = build_request(seed, VectorQuantizationBits::Bits4, 1_000_000);
    request.method = VectorQuantizationMethod::TurboQuant {
        bits: VectorQuantizationBits::Bits4,
        seed,
        estimator: rrd_contract::VectorQuantizationEstimator::InnerProduct,
    };
    request
}

/// E-04d: an inner-product generation is built, verified, served, extended
/// and reopened like an MSE one, and it is its own configuration.
#[test]
fn an_inner_product_generation_serves_extends_and_is_its_own_configuration() {
    let (root, engine, lease) = fixture(DOCUMENTS, 4);
    let first = try_build(
        &engine,
        &lease,
        &inner_product_request(SEED),
        "product-first",
        400,
    )
    .unwrap();
    assert_eq!(first.base_generation, None);
    assert_eq!(
        first.artifact.method,
        inner_product_request(SEED).method,
        "the listed method keeps its estimator"
    );
    assert_equals_full_build(&engine, 1);
    activate(&engine, &lease, &first, 410);
    assert_serves_exactly(&engine, &lease, 0);

    // An MSE build of the same seed and width is another configuration.
    let mse = build(
        &engine,
        &lease,
        SEED,
        VectorQuantizationBits::Bits4,
        "mse",
        420,
    );
    assert_eq!(mse.base_generation, None);
    assert_ne!(
        mse.artifact.configuration_sha256,
        first.artifact.configuration_sha256
    );

    change(&engine, &lease, 500);
    let second = try_build(
        &engine,
        &lease,
        &inner_product_request(SEED),
        "product-second",
        600,
    )
    .unwrap();
    assert_eq!(second.artifact.generation, 3);
    assert_eq!(second.base_generation, Some(1));
    let versions = assert_equals_full_build(&engine, 3);
    assert_eq!(second.encoded_versions, changed_versions(&versions));

    drop(engine);
    let engine = RrdEngine::open(root.path(), instance(), TOKEN_KEY).unwrap();
    activate(&engine, &lease, &second, 610);
    assert_serves_exactly(&engine, &lease, 100);

    // 1.5 bits has no inner-product estimator.
    let mut refused = inner_product_request(SEED);
    refused.method = VectorQuantizationMethod::TurboQuant {
        bits: VectorQuantizationBits::Bits1_5,
        seed: SEED,
        estimator: rrd_contract::VectorQuantizationEstimator::InnerProduct,
    };
    assert!(try_build(&engine, &lease, &refused, "product-refused", 700).is_err());
}
