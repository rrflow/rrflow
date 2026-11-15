//! POAM-045: an identity is found only in the collection its visible version
//! names. Identities move between collections, out and back, and later in
//! valid time; every search path answers what point retrieval answers at the
//! same stamp. POAM-048: a generation built or maintained after the moves
//! stops proposing the identities that left. POAM-047: a collection every
//! identity has left can be deleted.

use super::*;
use rrd_contract::{
    ExecuteRetrievalQuery, RetrievalOutput, RetrievalQuery, RetrievalRecommendStrategy,
    RetrievalResultShape, RetrievalVectorExample, ScrollVectorPoints, SessionLease,
    VectorSearchResult,
};
use std::collections::BTreeSet;

const DIMENSIONS: usize = 4;
const COLLECTIONS: [(&str, &str); 3] = [
    ("documents", "body-embedding"),
    ("archive", "archive-embedding"),
    // Another collection that shares `documents`' field.
    ("mirror", "body-embedding"),
];
const IDENTITIES: [&str; 8] = ["a", "b", "c", "d", "e", "f", "g", "h"];
/// Each move, committed in order: the identity, the collection it moves to,
/// its value seed and the valid time it moves from. Every identity but `e`
/// starts in `documents`; `e` starts in `archive`.
const MOVES: [(&str, &str, usize, u64); 6] = [
    // a: documents → archive from valid time 1.
    ("a", "archive", 0, 1),
    // b: documents → archive from valid time 50 only.
    ("b", "archive", 1, 50),
    // c: documents → mirror, which shares documents' field.
    ("c", "mirror", 2, 1),
    // d: documents → archive → documents.
    ("d", "archive", 3, 1),
    ("d", "documents", 3, 1),
    // e: archive → documents.
    ("e", "documents", 4, 1),
];

fn values(seed: usize) -> Vec<f32> {
    (0..DIMENSIONS)
        .map(|dimension| ((seed * 13 + dimension * 5) % 17) as f32 / 8.0 - 1.0)
        .collect()
}

fn scope() -> String {
    format!("instance:{}", instance())
}

fn field_of(collection: &str) -> &'static str {
    COLLECTIONS
        .iter()
        .find(|(name, _)| *name == collection)
        .map(|(_, field)| *field)
        .unwrap()
}

fn put(id: &str, collection: &str, seed: usize, valid_from: u64) -> TransactionMutation {
    TransactionMutation::PutVector {
        reference: reference("embedding", id),
        subject: reference("document", id),
        collection_id: CanonicalId::new(collection).unwrap(),
        vector_name: CanonicalId::new("body").unwrap(),
        field: CanonicalId::new(field_of(collection)).unwrap(),
        valid_from,
        valid_to: None,
        value: DataVectorValue::Dense {
            values: values(seed),
        },
        provenance: None,
        properties: DataProperties::new(),
    }
}

fn ensure_collection(engine: &RrdEngine, lease: &SessionLease, collection: &str) {
    engine
        .ensure_vector_collection(
            &lease.session_id,
            &lease.token,
            &EnsureVectorCollection {
                scope: scope(),
                collection_id: CanonicalId::new(collection).unwrap(),
                vectors: vec![NamedVectorDefinition {
                    name: CanonicalId::new("body").unwrap(),
                    field: CanonicalId::new(field_of(collection)).unwrap(),
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

/// Where each accelerator is built: before the moves, so it serves them from
/// its overlay, or after them, so it was built from the moved engram.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Accelerator {
    None,
    TurboQuantBefore,
    TurboQuantAfter,
    ScalarAfter,
    HnswBefore,
    HnswAfter,
}

fn quantize(engine: &RrdEngine, lease: &SessionLease, method: VectorQuantizationMethod, key: &str) {
    let built = engine
        .build_vector_quantization_artifact(
            &lease.session_id,
            &lease.token,
            &BuildVectorQuantizationArtifact {
                scope: scope(),
                collection_id: CanonicalId::new("documents").unwrap(),
                vector_name: CanonicalId::new("body").unwrap(),
                method,
                filter_properties: Vec::new(),
                max_storage_keys: 100_000,
            },
            &mutation_context(
                &id(&format!("build-{key}")),
                &format!("request-build-{key}"),
                &format!("operation-build-{key}"),
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
                &id(&format!("activate-{key}")),
                &format!("request-activate-{key}"),
                &format!("operation-activate-{key}"),
            ),
            410,
        )
        .unwrap();
}

fn hnsw(engine: &RrdEngine, lease: &SessionLease, key: &str) {
    engine
        .ensure_vector_index(
            &lease.session_id,
            &lease.token,
            &EnsureVectorIndex {
                scope: scope(),
                collection_id: CanonicalId::new("documents").unwrap(),
                vector_name: CanonicalId::new("body").unwrap(),
                configuration: VectorIndexConfiguration::Hnsw {
                    m: 4,
                    ef_construction: 16,
                    max_level: 4,
                    seed: 17,
                    filter_properties: Vec::new(),
                },
                build_policy: VectorIndexBuildPolicy::Cpu,
                max_storage_keys: 100_000,
            },
            &mutation_context(
                &id(&format!("hnsw-{key}")),
                &format!("request-hnsw-{key}"),
                &format!("operation-hnsw-{key}"),
            ),
            420,
        )
        .unwrap();
}

fn build(engine: &RrdEngine, lease: &SessionLease, accelerator: Accelerator, before: bool) {
    match (accelerator, before) {
        (Accelerator::TurboQuantBefore, true) | (Accelerator::TurboQuantAfter, false) => quantize(
            engine,
            lease,
            VectorQuantizationMethod::TurboQuant {
                bits: VectorQuantizationBits::Bits4,
                seed: 29,
                estimator: rrd_contract::VectorQuantizationEstimator::Mse,
            },
            "turboquant",
        ),
        (Accelerator::ScalarAfter, false) => {
            quantize(engine, lease, VectorQuantizationMethod::Scalar, "scalar")
        }
        (Accelerator::HnswBefore, true) | (Accelerator::HnswAfter, false) => {
            hnsw(engine, lease, "hnsw")
        }
        _ => {}
    }
}

/// The three collections, seven identities in `documents` and `e` in
/// `archive`, before any move.
fn engram() -> (tempfile::TempDir, RrdEngine, SessionLease) {
    let root = tempfile::tempdir().unwrap();
    let engine = RrdEngine::open(root.path(), instance(), TOKEN_KEY).unwrap();
    let lease = engine
        .create_session(
            &session_request(9_000, 8),
            &id("collection-moves-session"),
            100,
            "request-collection-moves-session",
            "operation-collection-moves-session",
        )
        .unwrap();
    for (collection, _) in COLLECTIONS {
        ensure_collection(&engine, &lease, collection);
    }
    let mut mutations = vec![TransactionMutation::PutSchema {
        registry: vector_fixture_schema(
            "install collection moves fixture",
            DataRecordSchema {
                properties: BTreeMap::new(),
                allow_additional_properties: true,
                ..DataRecordSchema::default()
            },
        ),
    }];
    for (row, id) in IDENTITIES.iter().enumerate() {
        mutations.push(document_mutation(id, "collection moves"));
        if *id != "e" {
            mutations.push(put(id, "documents", row, 1));
        }
    }
    mutations.push(put("e", "archive", 4, 1));
    commit_vectors(&engine, &lease, 300, mutations);
    (root, engine, lease)
}

fn commit_move(engine: &RrdEngine, lease: &SessionLease, index: usize) {
    let (id, collection, seed, valid_from) = MOVES[index];
    commit_vectors(
        engine,
        lease,
        500 + 10 * index as u64,
        vec![put(id, collection, seed, valid_from)],
    );
}

/// The engram, the accelerator, every move, and the accelerator again when
/// it is built after them.
fn fixture(accelerator: Accelerator) -> (tempfile::TempDir, RrdEngine, SessionLease) {
    let (root, engine, lease) = engram();
    build(&engine, &lease, accelerator, true);
    for index in 0..MOVES.len() {
        commit_move(&engine, &lease, index);
    }
    build(&engine, &lease, accelerator, false);
    (root, engine, lease)
}

/// The identities point retrieval finds in `collection` at `valid_at`.
fn retrieved(
    engine: &RrdEngine,
    lease: &SessionLease,
    collection: &str,
    valid_at: u64,
) -> BTreeSet<String> {
    engine
        .retrieve_vector_points(
            &lease.session_id,
            &lease.token,
            &RetrieveVectorPoints {
                scope: scope(),
                collection_id: CanonicalId::new(collection).unwrap(),
                vector_name: CanonicalId::new("body").unwrap(),
                valid_at,
                references: IDENTITIES
                    .iter()
                    .map(|id| reference("embedding", id))
                    .collect(),
                max_storage_keys: 100_000,
            },
            700,
            &format!("request-retrieve-{collection}-{valid_at}"),
            &format!("operation-retrieve-{collection}-{valid_at}"),
        )
        .unwrap()
        .points
        .into_iter()
        .map(|point| point.reference.id.as_str().to_owned())
        .collect()
}

/// The identities a scroll of `collection` pages through at `valid_at`.
fn scrolled(
    engine: &RrdEngine,
    lease: &SessionLease,
    collection: &str,
    valid_at: u64,
) -> BTreeSet<String> {
    let page = engine
        .scroll_vector_points(
            &lease.session_id,
            &lease.token,
            &ScrollVectorPoints {
                scope: scope(),
                collection_id: CanonicalId::new(collection).unwrap(),
                vector_name: CanonicalId::new("body").unwrap(),
                valid_at,
                after_reference: None,
                limit: 20,
                max_storage_keys: 100_000,
                filter: None,
            },
            710,
            &format!("request-scroll-{collection}-{valid_at}"),
            &format!("operation-scroll-{collection}-{valid_at}"),
        )
        .unwrap();
    assert!(!page.truncated);
    page.points
        .into_iter()
        .map(|point| point.reference.id.as_str().to_owned())
        .collect()
}

/// The vectors a retrieval query's recommend stage scores in `collection`
/// at `valid_at`: every one the collection holds there.
fn recommended(
    engine: &RrdEngine,
    lease: &SessionLease,
    collection: &str,
    valid_at: u64,
) -> BTreeSet<String> {
    let result = engine
        .execute_retrieval_query(
            &lease.session_id,
            &lease.token,
            &ExecuteRetrievalQuery {
                scope: scope(),
                valid_at,
                collection_id: CanonicalId::new(collection).unwrap(),
                query: RetrievalQuery::Recommend {
                    using: CanonicalId::new("body").unwrap(),
                    positive: vec![RetrievalVectorExample::Vector {
                        value: VectorSearchQuery::Dense { values: values(3) },
                    }],
                    negative: Vec::new(),
                    strategy: RetrievalRecommendStrategy::BestScore,
                    filter: None,
                },
                result_shape: RetrievalResultShape::Points,
                limit: 20,
                candidate_limit: 20,
                max_storage_keys: 100_000,
            },
            720,
            &format!("request-recommend-{collection}-{valid_at}"),
            &format!("operation-recommend-{collection}-{valid_at}"),
        )
        .unwrap_or_else(|error| panic!("recommend in {collection} at {valid_at}: {error:?}"));
    let RetrievalOutput::Points { hits } = result.output else {
        panic!("a points-shaped retrieval returns points");
    };
    hits.into_iter()
        .flat_map(|hit| hit.vector_references)
        .map(|reference| reference.id.as_str().to_owned())
        .collect()
}

fn search(
    engine: &RrdEngine,
    lease: &SessionLease,
    collection: &str,
    valid_at: u64,
    mode: VectorSearchMode,
) -> VectorSearchResult {
    search_near(engine, lease, collection, valid_at, 3, 20, mode)
}

/// A search of `collection` near the value of seed `near`.
fn search_near(
    engine: &RrdEngine,
    lease: &SessionLease,
    collection: &str,
    valid_at: u64,
    near: usize,
    top_k: u64,
    mode: VectorSearchMode,
) -> VectorSearchResult {
    engine
        .search_vectors(
            &lease.session_id,
            &lease.token,
            &SearchVectors {
                scope: scope(),
                valid_at,
                collection_id: Some(CanonicalId::new(collection).unwrap()),
                vector_name: Some(CanonicalId::new("body").unwrap()),
                field: None,
                query: VectorSearchQuery::Dense {
                    values: values(near),
                },
                filter: None,
                metric: None,
                top_k,
                mode,
                max_storage_keys: 100_000,
            },
            800,
            &format!("request-search-{collection}-{valid_at}"),
            &format!("operation-search-{collection}-{valid_at}"),
        )
        .unwrap_or_else(|error| panic!("search {collection} at {valid_at}: {error:?}"))
}

fn hit_ids(result: &VectorSearchResult) -> BTreeSet<String> {
    result
        .hits
        .iter()
        .map(|hit| hit.reference.id.as_str().to_owned())
        .collect()
}

fn assert_agrees_with_retrieval(accelerator: Accelerator) {
    let (_root, engine, lease) = fixture(accelerator);
    let approximate = VectorSearchMode::RequireApproximate {
        exact_rerank: 20,
        ef_search: 32,
    };
    for valid_at in [10, 60] {
        for (collection, _) in COLLECTIONS {
            let expected = retrieved(&engine, &lease, collection, valid_at);
            let exact = search(
                &engine,
                &lease,
                collection,
                valid_at,
                VectorSearchMode::Exact,
            );
            assert_eq!(
                hit_ids(&exact),
                expected,
                "{accelerator:?}: exact search of {collection} at {valid_at}"
            );
            assert_eq!(
                scrolled(&engine, &lease, collection, valid_at),
                expected,
                "{accelerator:?}: scroll of {collection} at {valid_at}"
            );
            assert_eq!(
                recommended(&engine, &lease, collection, valid_at),
                expected,
                "{accelerator:?}: recommend in {collection} at {valid_at}"
            );
            if accelerator != Accelerator::None && collection == "documents" {
                let served = search(&engine, &lease, collection, valid_at, approximate);
                let (path, before) = match accelerator {
                    Accelerator::TurboQuantBefore => ("turboquant", true),
                    Accelerator::TurboQuantAfter => ("turboquant", false),
                    Accelerator::ScalarAfter => ("scalar_quantized", false),
                    Accelerator::HnswBefore => ("hnsw", true),
                    Accelerator::HnswAfter => ("hnsw", false),
                    Accelerator::None => unreachable!(),
                };
                assert_eq!(served.access_path.as_str(), path);
                assert!(!served.exact, "{accelerator:?} serves approximately");
                // A generation built before the moves serves the identities
                // moved in from its overlay; one built after them has none.
                assert_eq!(
                    served.resources.overlay_candidates > 0,
                    before,
                    "{accelerator:?} overlay at {valid_at}"
                );
                assert_eq!(
                    hit_ids(&served),
                    expected,
                    "{accelerator:?}: {} search of {collection} at {valid_at}",
                    served.access_path.as_str()
                );
            }
        }
    }
    // The fixture's own expectations, so the oracle is not vacuous.
    let documents_early = retrieved(&engine, &lease, "documents", 10);
    let documents_late = retrieved(&engine, &lease, "documents", 60);
    assert!(!documents_early.contains("a") && documents_early.contains("b"));
    assert!(!documents_late.contains("b") && !documents_late.contains("c"));
    assert!(documents_late.contains("d") && documents_late.contains("e"));
    assert!(retrieved(&engine, &lease, "mirror", 60).contains("c"));
}

#[test]
fn exact_search_finds_a_moved_identity_only_where_it_is() {
    assert_agrees_with_retrieval(Accelerator::None);
}

#[test]
fn turboquant_built_before_the_moves_serves_them_from_its_overlay() {
    assert_agrees_with_retrieval(Accelerator::TurboQuantBefore);
}

#[test]
fn turboquant_built_after_the_moves_never_returns_an_old_version() {
    assert_agrees_with_retrieval(Accelerator::TurboQuantAfter);
}

#[test]
fn scalar_quantization_built_after_the_moves_never_returns_an_old_version() {
    assert_agrees_with_retrieval(Accelerator::ScalarAfter);
}

#[test]
fn hnsw_built_before_the_moves_serves_them_from_its_overlay() {
    assert_agrees_with_retrieval(Accelerator::HnswBefore);
}

#[test]
fn hnsw_built_after_the_moves_never_returns_an_old_version() {
    assert_agrees_with_retrieval(Accelerator::HnswAfter);
}

/// What point retrieval finds in every collection at both valid times.
fn membership(
    engine: &RrdEngine,
    lease: &SessionLease,
) -> BTreeMap<(&'static str, u64), BTreeSet<String>> {
    let mut membership = BTreeMap::new();
    for (collection, _) in COLLECTIONS {
        for valid_at in [10, 60] {
            membership.insert(
                (collection, valid_at),
                retrieved(engine, lease, collection, valid_at),
            );
        }
    }
    membership
}

/// A move is a removal from the source it leaves and an addition to the one
/// it joins, and nothing else; every identity whose candidacy in a source
/// changed is one of that source's deltas, so a generation's delta overlay
/// sees every move.
#[test]
fn every_move_is_a_delta_of_the_sources_it_leaves_and_joins() {
    let (_root, engine, lease) = engram();
    let scope_id = rrd_core::ScopeId::new(scope()).unwrap();
    let mut location = IDENTITIES
        .iter()
        .map(|id| (*id, if *id == "e" { "archive" } else { "documents" }))
        .collect::<BTreeMap<_, _>>();
    for (index, (id, to, _, _)) in MOVES.iter().enumerate() {
        let from = location[id];
        let after = engine
            .storage
            .runtime()
            .read_stamp(&scope_id)
            .unwrap()
            .commit_cursor;
        let earlier = membership(&engine, &lease);
        commit_move(&engine, &lease, index);
        let read = engine.storage.runtime().read_stamp(&scope_id).unwrap();
        let later = membership(&engine, &lease);
        for (collection, field) in COLLECTIONS {
            let deltas = engine
                .storage
                .runtime()
                .read_vector_deltas(
                    &read,
                    &rrd_store::VectorSourceAddress {
                        collection: rrd_core::VectorCollectionAddress {
                            collection_id: collection.into(),
                            vector_name: "body".into(),
                        },
                        field: field.into(),
                    },
                    after,
                    rrd_store::RuntimeReadBudget::new(100_000).unwrap(),
                )
                .unwrap();
            let named = deltas
                .deltas
                .iter()
                .map(|delta| delta.reference.id.as_str().to_owned())
                .collect::<BTreeSet<_>>();
            for valid_at in [10, 60] {
                let changed = earlier[&(collection, valid_at)]
                    .symmetric_difference(&later[&(collection, valid_at)])
                    .cloned()
                    .collect::<BTreeSet<_>>();
                assert!(
                    changed.is_subset(&named),
                    "move {index}: {collection} changed {changed:?} at {valid_at}, deltas {named:?}"
                );
            }
            let pointers = deltas
                .deltas
                .iter()
                .map(|delta| (delta.before.is_some(), delta.after.is_some()))
                .collect::<Vec<_>>();
            if collection == from {
                assert_eq!(
                    named,
                    BTreeSet::from([id.to_string()]),
                    "move {index} leaves {from}"
                );
                assert_eq!(
                    pointers,
                    [(true, false)],
                    "move {index} removes from {from}"
                );
            } else if collection == *to {
                assert_eq!(
                    named,
                    BTreeSet::from([id.to_string()]),
                    "move {index} joins {to}"
                );
                assert_eq!(pointers, [(false, true)], "move {index} adds to {to}");
            } else {
                assert!(
                    named.is_empty(),
                    "move {index} touches only its two sources"
                );
            }
        }
        location.insert(id, to);
    }
}

/// POAM-048: near `a`'s value, whose identity left `documents`, a search with
/// three rerank slots fills them with identities `documents` holds. A
/// generation that still proposed `a`, `b` or `c` would spend slots on them
/// and return fewer hits.
fn assert_fills_its_rerank_slots(engine: &RrdEngine, lease: &SessionLease, path: &str) {
    for valid_at in [10, 60] {
        let held = retrieved(engine, lease, "documents", valid_at);
        let served = search_near(
            engine,
            lease,
            "documents",
            valid_at,
            0,
            3,
            VectorSearchMode::RequireApproximate {
                exact_rerank: 3,
                ef_search: 32,
            },
        );
        assert_eq!(served.access_path.as_str(), path);
        assert_eq!(
            served.resources.overlay_candidates, 0,
            "{path} at {valid_at} serves a generation built after the moves"
        );
        assert_eq!(
            served.hits.len(),
            3.min(held.len()),
            "{path} at {valid_at} returned {:?}",
            hit_ids(&served)
        );
        assert!(hit_ids(&served).is_subset(&held), "{path} at {valid_at}");
    }
}

#[test]
fn turboquant_built_after_the_moves_proposes_only_identities_it_holds() {
    let (_root, engine, lease) = fixture(Accelerator::TurboQuantAfter);
    assert_fills_its_rerank_slots(&engine, &lease, "turboquant");
}

#[test]
fn turboquant_extended_after_the_moves_proposes_only_identities_it_holds() {
    let (_root, engine, lease) = fixture(Accelerator::TurboQuantBefore);
    let extended = engine
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
                filter_properties: Vec::new(),
                max_storage_keys: 100_000,
            },
            &mutation_context(
                &id("build-extended"),
                "request-build-extended",
                "operation-build-extended",
            ),
            600,
        )
        .unwrap();
    assert!(extended.base_generation.is_some(), "the build extends");
    engine
        .activate_vector_quantization_artifact(
            &lease.session_id,
            &lease.token,
            &ActivateVectorQuantizationArtifact {
                scope: scope(),
                artifact_id: extended.artifact.artifact_id,
                generation: extended.artifact.generation,
            },
            &mutation_context(
                &id("activate-extended"),
                "request-activate-extended",
                "operation-activate-extended",
            ),
            610,
        )
        .unwrap();
    assert_fills_its_rerank_slots(&engine, &lease, "turboquant");
}

#[test]
fn scalar_quantization_built_after_the_moves_proposes_only_identities_it_holds() {
    let (_root, engine, lease) = fixture(Accelerator::ScalarAfter);
    assert_fills_its_rerank_slots(&engine, &lease, "scalar_quantized");
}

#[test]
fn hnsw_built_after_the_moves_proposes_only_identities_it_holds() {
    let (_root, engine, lease) = fixture(Accelerator::HnswAfter);
    assert_fills_its_rerank_slots(&engine, &lease, "hnsw");
}

#[test]
fn hnsw_maintained_after_the_moves_proposes_only_identities_it_holds() {
    let (_root, engine, lease) = fixture(Accelerator::HnswBefore);
    hnsw(&engine, &lease, "hnsw-maintained");
    assert_fills_its_rerank_slots(&engine, &lease, "hnsw");
}

/// Deletes `collection` at `valid_at`, under its own request identity.
fn delete(
    engine: &RrdEngine,
    lease: &SessionLease,
    collection: &str,
    valid_at: u64,
) -> std::result::Result<rrd_contract::DeleteVectorCollectionResult, ServiceError> {
    engine.delete_vector_collection(
        &lease.session_id,
        &lease.token,
        &rrd_contract::DeleteVectorCollection {
            scope: scope(),
            collection_id: CanonicalId::new(collection).unwrap(),
            valid_at,
            max_storage_keys: 100_000,
        },
        &mutation_context(
            &id(&format!("delete-{collection}-{valid_at}")),
            &format!("request-delete-{collection}-{valid_at}"),
            &format!("operation-delete-{collection}-{valid_at}"),
        ),
        900 + valid_at,
    )
}

/// Moves every identity `documents` holds to `archive`, each from valid time
/// 1 except those `later` names, which move from the time given.
fn empty_documents(engine: &RrdEngine, lease: &SessionLease, later: &[(&str, u64)]) {
    let moved = IDENTITIES
        .iter()
        .enumerate()
        .filter(|(_, id)| **id != "e")
        .map(|(seed, id)| {
            let from = later
                .iter()
                .find(|(late, _)| late == id)
                .map_or(1, |(_, from)| *from);
            put(id, "archive", seed, from)
        })
        .collect::<Vec<_>>();
    commit_vectors(engine, lease, 500, moved);
}

/// POAM-047: once every identity has left `documents`, it holds no point at
/// any valid time and can be deleted.
#[test]
fn a_collection_every_identity_left_can_be_deleted() {
    let (_root, engine, lease) = engram();
    empty_documents(&engine, &lease, &[]);
    assert!(retrieved(&engine, &lease, "documents", 10).is_empty());
    assert!(retrieved(&engine, &lease, "documents", 60).is_empty());
    let deleted = delete(&engine, &lease, "documents", 60);
    assert!(deleted.is_ok(), "{deleted:?}");
}

#[test]
fn empty_collection_with_bound_hnsw_cannot_be_deleted_after_reopen() {
    let (root, engine, lease) = engram();
    empty_documents(&engine, &lease, &[]);
    hnsw(&engine, &lease, "empty-delete-guard");
    drop(engine);
    let reopened = RrdEngine::open(root.path(), instance(), TOKEN_KEY).unwrap();
    let error = delete(&reopened, &lease, "documents", 60).unwrap_err();
    assert!(
        error.to_string().contains("still owns an active HNSW"),
        "{error}"
    );
}

/// POAM-047: a collection still holds an identity until its move out takes
/// effect, one that moved back, and one that moves back only after the
/// deletion time.
#[test]
fn a_collection_an_identity_still_holds_refuses_deletion() {
    let (_root, engine, lease) = engram();
    empty_documents(&engine, &lease, &[("b", 100)]);
    assert!(retrieved(&engine, &lease, "documents", 60).contains("b"));
    assert!(delete(&engine, &lease, "documents", 60).is_err());
    assert!(retrieved(&engine, &lease, "documents", 100).is_empty());
    let deleted = delete(&engine, &lease, "documents", 100);
    assert!(deleted.is_ok(), "{deleted:?}");

    let (_root, engine, lease) = engram();
    empty_documents(&engine, &lease, &[]);
    commit_vectors(&engine, &lease, 510, vec![put("d", "documents", 3, 1)]);
    assert!(delete(&engine, &lease, "documents", 60).is_err());

    let (_root, engine, lease) = engram();
    empty_documents(&engine, &lease, &[]);
    commit_vectors(&engine, &lease, 510, vec![put("d", "documents", 3, 100)]);
    assert!(retrieved(&engine, &lease, "documents", 60).is_empty());
    assert!(retrieved(&engine, &lease, "documents", 100).contains("d"));
    assert!(delete(&engine, &lease, "documents", 60).is_err());
}

/// POAM-048: `h` holds the newest version in `documents`, which every
/// generation's coverage reached. When it moves out, the next generation
/// drops it yet still covers the move, so its coverage never moves backward
/// and it publishes.
#[test]
fn a_generation_after_its_newest_version_moves_out_still_publishes() {
    let (_root, engine, lease) = engram();
    hnsw(&engine, &lease, "hnsw-first");
    let turboquant = VectorQuantizationMethod::TurboQuant {
        bits: VectorQuantizationBits::Bits4,
        seed: 29,
        estimator: rrd_contract::VectorQuantizationEstimator::Mse,
    };
    quantize(&engine, &lease, turboquant.clone(), "turboquant-first");
    quantize(
        &engine,
        &lease,
        VectorQuantizationMethod::Scalar,
        "scalar-first",
    );
    commit_vectors(&engine, &lease, 600, vec![put("h", "archive", 7, 1)]);
    hnsw(&engine, &lease, "hnsw-second");
    quantize(&engine, &lease, turboquant, "turboquant-second");
    quantize(
        &engine,
        &lease,
        VectorQuantizationMethod::Scalar,
        "scalar-second",
    );
    assert!(!retrieved(&engine, &lease, "documents", 60).contains("h"));
}

#[test]
fn distinct_named_vector_sources_have_distinct_hnsw_indexes() {
    let (_root, engine, lease) = engram();
    let mut indexes = Vec::new();
    for (ordinal, (collection, vector_name)) in [("a-b", "c"), ("a", "b-c")].into_iter().enumerate()
    {
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
                        m: 4,
                        ef_construction: 16,
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

#[test]
fn an_empty_foreign_hnsw_generation_cannot_answer_for_a_populated_source() {
    let (_root, engine, lease) = engram();
    commit_vectors(
        &engine,
        &lease,
        350,
        vec![
            document_mutation("mirror-point", "mirror"),
            put("mirror-point", "mirror", 9, 1),
        ],
    );
    empty_documents(&engine, &lease, &[]);
    hnsw(&engine, &lease, "empty-source");
    let exact = search_near(&engine, &lease, "mirror", 1, 9, 1, VectorSearchMode::Exact);
    assert!(hit_ids(&exact).contains("mirror-point"));
    let result = engine.search_vectors(
        &lease.session_id,
        &lease.token,
        &SearchVectors {
            scope: scope(),
            valid_at: 1,
            collection_id: Some(CanonicalId::new("mirror").unwrap()),
            vector_name: Some(CanonicalId::new("body").unwrap()),
            field: None,
            query: VectorSearchQuery::Dense { values: values(9) },
            filter: None,
            metric: None,
            top_k: 1,
            mode: VectorSearchMode::RequireApproximate {
                exact_rerank: 8,
                ef_search: 16,
            },
            max_storage_keys: 100_000,
        },
        801,
        "request-foreign-empty-source",
        "operation-foreign-empty-source",
    );
    match result {
        Ok(served) => assert_eq!(hit_ids(&served), hit_ids(&exact)),
        Err(ServiceError::Vector(reason)) => assert!(reason.contains("no vector access path")),
        Err(error) => panic!("unexpected denial: {error:?}"),
    }
}

#[test]
fn an_empty_bound_hnsw_serves_later_source_updates_after_reopen() {
    let (root, engine, lease) = engram();
    empty_documents(&engine, &lease, &[]);
    hnsw(&engine, &lease, "empty-bound");
    let scope_id = rrd_core::ScopeId::new(scope()).unwrap();
    let entries = crate::vector_artifact_catalog_entries(&engine.storage, &scope_id).unwrap();
    let bound = entries.last().unwrap();
    assert_eq!(bound.source().unwrap().collection_id, "documents");
    assert_eq!(bound.source().unwrap().vector_name, "body");
    let projection_id = bound.descriptor.stamp().id.clone();
    commit_vectors(
        &engine,
        &lease,
        600,
        vec![
            document_mutation("new-source-point", "after build"),
            put("new-source-point", "documents", 9, 1),
        ],
    );
    let mode = VectorSearchMode::RequireApproximate {
        exact_rerank: 8,
        ef_search: 16,
    };
    let before = search_near(&engine, &lease, "documents", 1, 9, 1, mode);
    assert_eq!(before.access_path.as_str(), "hnsw");
    assert_eq!(
        hit_ids(&before),
        BTreeSet::from(["new-source-point".to_owned()])
    );
    drop(engine);
    let reopened = RrdEngine::open(root.path(), instance(), TOKEN_KEY).unwrap();
    let entries = crate::vector_artifact_catalog_entries(&reopened.storage, &scope_id).unwrap();
    let bound = entries
        .iter()
        .find(|entry| entry.descriptor.stamp().id == projection_id)
        .unwrap();
    assert_eq!(bound.source().unwrap().collection_id, "documents");
    assert_eq!(bound.source().unwrap().vector_name, "body");
    let after = search_near(&reopened, &lease, "documents", 1, 9, 1, mode);
    assert_eq!(after.access_path.as_str(), "hnsw");
    assert_eq!(hit_ids(&after), hit_ids(&before));
}
