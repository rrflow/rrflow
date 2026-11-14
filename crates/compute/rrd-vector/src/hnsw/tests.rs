use super::*;
use crate::{FilterCondition, FilterExpression, FilterOperator};
use rrd_core::{
    ReadStamp, RuntimeProperties, RuntimeRef, RuntimeValue, RuntimeVector, VectorCollectionAddress,
};

const KERNEL_METRICS: [ScoreMetric; 4] = [
    ScoreMetric::Dot,
    ScoreMetric::Cosine,
    ScoreMetric::Euclidean,
    ScoreMetric::Manhattan,
];

fn kernel_vectors(length: usize, seed: u32) -> (Vec<f32>, Vec<f32>) {
    let mut state = seed;
    let mut next = move || {
        state = state
            .wrapping_mul(2_654_435_761)
            .wrapping_add(1_013_904_223);
        (state >> 8) as f32 / (1_u32 << 24) as f32 * 2.0 - 1.0
    };
    let left = (0..length).map(|_| next()).collect();
    let right = (0..length).map(|_| next()).collect();
    (left, right)
}

fn both_kernels(left: &[f32], right: &[f32], metric: ScoreMetric) -> (f64, f64) {
    (
        score_dense_values(left, right, metric, HnswKernel::Scalar).unwrap(),
        score_dense_values(left, right, metric, HnswKernel::Auto).unwrap(),
    )
}

/// A lost lane or tail element moves a score far more than this rounding
/// allowance at the short lengths, where each element is a large share.
#[test]
fn the_automatic_kernel_agrees_with_the_scalar_kernel_at_every_length() {
    for length in (0..=67).chain([128, 384, 512, 1024]) {
        let (left, right) = kernel_vectors(length, length as u32 + 1);
        let allowance = 1e-5 * length.max(1) as f64;
        for metric in KERNEL_METRICS {
            let (scalar, automatic) = both_kernels(&left, &right, metric);
            assert!(
                (scalar - automatic).abs() <= allowance,
                "{metric:?} at {length}: {scalar} and {automatic}"
            );
        }
    }
}

#[test]
fn a_zero_vector_scores_as_the_scalar_kernel_scores_it() {
    let zero = vec![0.0_f32; 33];
    let (other, _) = kernel_vectors(33, 7);
    for metric in KERNEL_METRICS {
        for (left, right) in [(&zero, &other), (&other, &zero), (&zero, &zero)] {
            let (scalar, automatic) = both_kernels(left, right, metric);
            assert!(
                (scalar.is_nan() && automatic.is_nan()) || (scalar - automatic).abs() <= 1e-3,
                "{metric:?}: {scalar} and {automatic}"
            );
        }
    }
}

/// Lanes are f32 and the scalar kernel is f64. A product can overflow the
/// first where the second is finite, and that must not change a score.
#[test]
fn a_lane_that_overflows_gets_the_scalar_kernels_score() {
    let same = vec![1e30_f32; 33];
    let opposite = same.iter().map(|value| -value).collect::<Vec<_>>();
    let alternating = same
        .iter()
        .enumerate()
        .map(|(index, value)| if index % 2 == 0 { *value } else { -value })
        .collect::<Vec<_>>();
    for metric in KERNEL_METRICS {
        for other in [&same, &opposite, &alternating] {
            let (scalar, automatic) = both_kernels(&same, other, metric);
            assert!(scalar.is_finite(), "{metric:?}: {scalar}");
            assert_eq!(automatic, scalar, "{metric:?}");
        }
    }
}

/// Lanes are f32. Squares of small-magnitude values underflow there where
/// the f64 scalar kernel keeps them, so distinct distances would become
/// ties, or keep only their tail element, and the nearest candidates
/// would be lost at traversal. Vectors of 33 leave one tail element.
#[test]
fn a_lane_that_underflows_gets_the_scalar_kernels_score() {
    let tiny = vec![1e-30_f32; 33];
    let zero = vec![0.0_f32; 33];
    let doubled = tiny.iter().map(|value| value * 2.0).collect::<Vec<_>>();
    for metric in KERNEL_METRICS {
        for (left, right) in [(&tiny, &zero), (&tiny, &doubled), (&doubled, &tiny)] {
            let (scalar, automatic) = both_kernels(left, right, metric);
            assert!(scalar.is_finite(), "{metric:?}: {scalar}");
            // A metric whose lanes stay normal (Manhattan here) keeps its
            // f32 rounding; one that underflowed must be the scalar score.
            assert!(
                (scalar - automatic).abs() <= scalar.abs() * 1e-5,
                "{metric:?}: {scalar} and {automatic}"
            );
        }
    }
}

/// A norm that overflows or underflows f32 can still give a finite score,
/// and that score is wrong, so being finite is not enough.
#[test]
fn a_cosine_norm_outside_the_normal_f32_range_gets_the_scalar_kernels_score() {
    let huge = vec![1e30_f32; 33];
    let tiny = vec![1e-30_f32; 33];
    let (ordinary, _) = kernel_vectors(33, 11);
    for (left, right) in [
        (&huge, &tiny),
        (&tiny, &huge),
        (&tiny, &tiny),
        (&huge, &ordinary),
        (&ordinary, &tiny),
    ] {
        let (scalar, automatic) = both_kernels(left, right, ScoreMetric::Cosine);
        assert!(scalar.is_finite(), "{scalar}");
        assert_eq!(automatic, scalar);
    }
}

#[test]
fn a_host_with_simd_uses_it() {
    #[cfg(target_arch = "aarch64")]
    assert!(
        simd_score().is_some(),
        "NEON is part of the aarch64 baseline"
    );
    #[cfg(target_arch = "x86_64")]
    assert_eq!(
        simd_score().is_some(),
        std::arch::is_x86_feature_detected!("avx2")
    );
}

fn candidate(scope: &ScopeId, cursor: u64, id: usize, values: Vec<f32>) -> VectorCandidate {
    VectorCandidate {
        scope: scope.clone(),
        source_cursor: cursor,
        vector: RuntimeVector {
            reference: RuntimeRef::new("embedding", format!("v-{id:03}")).unwrap(),
            subject: RuntimeRef::new("document", format!("d-{id:03}")).unwrap(),
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

fn config(scope: &ScopeId) -> HnswConfig {
    HnswConfig {
        id: ProjectionId::new("vector-hnsw-body").unwrap(),
        scope: scope.clone(),
        field: "body".into(),
        dimensions: 2,
        metric: ScoreMetric::Cosine,
        embedding_model: None,
        m: 8,
        ef_construction: 32,
        max_level: 8,
        seed: 7,
        filter_properties: BTreeSet::new(),
    }
}

/// A fixed stream of normal values, so a corpus is the same on every run and every machine.
struct Stream(u64);

impl Stream {
    fn uniform(&mut self) -> f64 {
        // xorshift64*, whose whole output is fixed by the seed.
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        let value = self.0.wrapping_mul(0x9E37_79B9_7F4A_7C15);
        ((value >> 11) as f64 + 0.5) / (1_u64 << 53) as f64
    }

    fn normal(&mut self) -> f32 {
        let radius = (-2.0 * self.uniform().ln()).sqrt();
        (radius * (std::f64::consts::TAU * self.uniform()).cos()) as f32
    }
}

/// Points around cluster centres, drawn as the lab's mixture draws them: real embeddings
/// cluster, and uniform data cannot show what clustering does to a graph.
fn clustered(
    scope: &ScopeId,
    first_id: usize,
    points: usize,
    clusters: usize,
    dimensions: usize,
    spread: f32,
    seed: u64,
) -> Vec<VectorCandidate> {
    let mut stream = Stream(seed);
    let centres = (0..clusters)
        .map(|_| {
            (0..dimensions)
                .map(|_| stream.normal())
                .collect::<Vec<f32>>()
        })
        .collect::<Vec<_>>();
    (0..points)
        .map(|index| {
            let centre = &centres[index % clusters];
            let values = centre
                .iter()
                .map(|value| value + spread * stream.normal())
                .collect();
            let id = first_id + index;
            candidate(scope, id as u64 + 1, id, values)
        })
        .collect()
}

fn clustered_config(scope: &ScopeId, dimensions: usize, m: usize, ef: usize) -> HnswConfig {
    HnswConfig {
        dimensions,
        m,
        ef_construction: ef,
        max_level: 6,
        ..config(scope)
    }
}

fn reachable_nodes(index: &HnswIndex) -> usize {
    let Some(entrypoint) = index.entrypoint else {
        return 0;
    };
    let mut seen = vec![false; index.nodes.len()];
    seen[entrypoint] = true;
    let mut pending = vec![entrypoint];
    while let Some(node) = pending.pop() {
        for neighbor in &index.nodes[node].neighbors[0] {
            if !seen[*neighbor] {
                seen[*neighbor] = true;
                pending.push(*neighbor);
            }
        }
    }
    seen.iter().filter(|seen| **seen).count()
}

/// The share of the true top ten a search finds, over queries drawn from the corpus's clusters.
fn recall(index: &HnswIndex, corpus: &[VectorCandidate], ef_search: usize) -> f64 {
    let scope = &index.config.scope;
    let cursor = index.descriptor.stamp.source_cursor;
    let mut stream = Stream(0xC0FF_EE00_D15E_A5E5);
    let mut found = 0;
    let mut wanted = 0;
    for query in corpus.iter().step_by(corpus.len() / 50) {
        let values = dense_candidate(query)
            .unwrap()
            .iter()
            .map(|value| value + 0.05 * stream.normal())
            .collect::<Vec<f32>>();
        let mut exact = corpus
            .iter()
            .map(|candidate| {
                (
                    score_dense_query(&values, candidate, ScoreMetric::Cosine, HnswKernel::Scalar)
                        .unwrap(),
                    candidate.vector.reference.clone(),
                )
            })
            .collect::<Vec<_>>();
        exact.sort_by(|left, right| right.0.total_cmp(&left.0));
        let truth = exact
            .into_iter()
            .take(10)
            .map(|(_, reference)| reference)
            .collect::<BTreeSet<_>>();
        let request = SearchRequest {
            scope: scope.clone(),
            read: ReadStamp::new(scope.clone(), None, 0, cursor, Some("11".repeat(32))).unwrap(),
            valid_at: 2,
            field: "body".into(),
            query: VectorQuery::Dense { values },
            metric: ScoreMetric::Cosine,
            embedding_model: None,
            top_k: 10,
            mode: SearchMode::RequireApproximate {
                exact_rerank: ef_search,
            },
            filter: None,
        };
        let hits = index.search(&request, ef_search).unwrap();
        found += hits
            .iter()
            .filter(|hit| truth.contains(&hit.reference))
            .count();
        wanted += truth.len();
    }
    found as f64 / wanted as f64
}

#[test]
fn a_clustered_corpus_builds_with_every_node_reachable_and_answers() {
    let scope = ScopeId::new("instance:hnsw").unwrap();
    let corpus = clustered(&scope, 0, 3_000, 32, 24, 0.35, 0x5EED_0001);
    let index = HnswIndex::build(
        clustered_config(&scope, 24, 16, 100),
        1,
        corpus.len() as u64,
        corpus.clone(),
    )
    .unwrap();
    assert_eq!(reachable_nodes(&index), corpus.len());
    for node in &index.nodes {
        for (layer, neighbors) in node.neighbors.iter().enumerate() {
            assert!(neighbors.len() <= layer_limit(16, layer));
        }
    }
    let found = recall(&index, &corpus, 128);
    assert!(found >= 0.95, "recall was {found}");
}

#[test]
fn two_tight_clusters_far_apart_stay_connected() {
    let scope = ScopeId::new("instance:hnsw").unwrap();
    let corpus = (0..400)
        .map(|index| {
            let side = if index % 2 == 0 { 1.0 } else { -1.0 };
            let wobble = index as f32 * 1e-4;
            candidate(
                &scope,
                index as u64 + 1,
                index,
                vec![side, wobble, side * wobble],
            )
        })
        .collect::<Vec<_>>();
    let index = HnswIndex::build(
        clustered_config(&scope, 3, 4, 32),
        1,
        corpus.len() as u64,
        corpus.clone(),
    )
    .unwrap();
    assert_eq!(reachable_nodes(&index), corpus.len());
}

#[test]
fn many_small_clusters_stay_connected() {
    let scope = ScopeId::new("instance:hnsw").unwrap();
    let corpus = clustered(&scope, 0, 1_200, 120, 16, 0.02, 0x5EED_0002);
    let index = HnswIndex::build(
        clustered_config(&scope, 16, 4, 32),
        1,
        corpus.len() as u64,
        corpus.clone(),
    )
    .unwrap();
    assert_eq!(reachable_nodes(&index), corpus.len());
}

#[test]
fn identical_vectors_stay_connected() {
    let scope = ScopeId::new("instance:hnsw").unwrap();
    let corpus = (0..300)
        .map(|index| candidate(&scope, index as u64 + 1, index, vec![0.6, 0.8]))
        .collect::<Vec<_>>();
    let index = HnswIndex::build(
        clustered_config(&scope, 2, 4, 32),
        1,
        corpus.len() as u64,
        corpus.clone(),
    )
    .unwrap();
    assert_eq!(reachable_nodes(&index), corpus.len());
}

#[test]
fn an_advance_into_new_clusters_stays_connected() {
    let scope = ScopeId::new("instance:hnsw").unwrap();
    let first = clustered(&scope, 0, 1_000, 8, 16, 0.2, 0x5EED_0003);
    let index = HnswIndex::build(
        clustered_config(&scope, 16, 8, 64),
        1,
        first.len() as u64,
        first.clone(),
    )
    .unwrap();
    let second = clustered(&scope, 1_000, 500, 8, 16, 0.2, 0x5EED_0004);
    let advanced = index.advance(2, 1_500, second.clone()).unwrap();
    assert_eq!(reachable_nodes(&advanced), first.len() + second.len());
    let mut whole = first;
    whole.extend(second);
    let found = recall(&advanced, &whole, 128);
    assert!(found >= 0.95, "recall was {found}");
}

#[test]
fn a_clustered_build_is_deterministic() {
    let scope = ScopeId::new("instance:hnsw").unwrap();
    let corpus = clustered(&scope, 0, 600, 16, 16, 0.2, 0x5EED_0005);
    let build = || {
        HnswIndex::build(
            clustered_config(&scope, 16, 8, 64),
            1,
            corpus.len() as u64,
            corpus.clone(),
        )
        .unwrap()
    };
    assert_eq!(build().as_bytes(), build().as_bytes());
}

#[test]
fn repair_links_each_orphan_from_its_closest_reachable_node_with_room() {
    let scope = ScopeId::new("instance:hnsw").unwrap();
    let node = |id: usize, values: Vec<f32>, neighbors: Vec<usize>| HnswNode {
        candidate: candidate(&scope, id as u64 + 1, id, values),
        level: 0,
        neighbors: vec![neighbors],
    };
    // Node 1 is closest to the orphan, node 2, and has room. Node 0 is farther. Node 3 is
    // reached through the orphan, so it must come along without a link of its own.
    let mut nodes = vec![
        node(0, vec![1.0, 0.0], vec![1]),
        node(1, vec![0.7, 0.7], vec![0]),
        node(2, vec![0.5, 0.9], vec![3]),
        node(3, vec![0.0, 1.0], Vec::new()),
    ];
    let settings = clustered_config(&scope, 2, 2, 8);
    repair_reachability(&settings, &mut nodes, Some(0)).unwrap();
    assert_eq!(nodes[1].neighbors[0], vec![0, 2]);
    assert_eq!(nodes[0].neighbors[0], vec![1]);
    assert_eq!(nodes[2].neighbors[0], vec![3, 1]);
    assert_eq!(nodes[3].neighbors[0], Vec::<usize>::new());
}

#[test]
fn repair_does_not_repeat_a_link_the_orphan_already_has() {
    let scope = ScopeId::new("instance:hnsw").unwrap();
    let node = |id: usize, values: Vec<f32>, neighbors: Vec<usize>| HnswNode {
        candidate: candidate(&scope, id as u64 + 1, id, values),
        level: 0,
        neighbors: vec![neighbors],
    };
    // Pruning is one-way, so the orphan can still point at its closest reachable node after
    // that node dropped it. The validator refuses a repeated link.
    let mut nodes = vec![
        node(0, vec![1.0, 0.0], Vec::new()),
        node(1, vec![0.9, 0.1], vec![0]),
    ];
    let settings = clustered_config(&scope, 2, 2, 8);
    repair_reachability(&settings, &mut nodes, Some(0)).unwrap();
    assert_eq!(nodes[0].neighbors[0], vec![1]);
    assert_eq!(nodes[1].neighbors[0], vec![0]);
}

/// The lab's own shape of data at the lab's own size. It takes a minute in a release build, so
/// it runs by hand: `cargo test --release -p rrd-vector --lib hnsw -- --ignored --nocapture`.
#[test]
#[ignore = "a minute in a release build"]
fn the_labs_mixture_at_its_full_size_builds_and_answers() {
    let scope = ScopeId::new("instance:hnsw").unwrap();
    let corpus = clustered(&scope, 0, 20_000, 64, 384, 0.35, 20_261_103);
    let started = std::time::Instant::now();
    let index = HnswIndex::build(
        HnswConfig {
            filter_properties: BTreeSet::from(["category".into()]),
            ..clustered_config(&scope, 384, 16, 100)
        },
        1,
        corpus.len() as u64,
        corpus.clone(),
    )
    .unwrap();
    eprintln!(
        "built in {:?}, {} bytes",
        started.elapsed(),
        index.as_bytes().len()
    );
    assert_eq!(reachable_nodes(&index), corpus.len());
    for ef in [16, 32, 64, 128] {
        let found = recall(&index, &corpus, ef);
        eprintln!("ef {ef}: recall {found}");
    }
    let found = recall(&index, &corpus, 128);
    assert!(found >= 0.95, "recall was {found}");
}

#[test]
fn repair_stays_within_the_degree_limit_and_leaves_a_full_graph_to_the_validator() {
    let scope = ScopeId::new("instance:hnsw").unwrap();
    let node = |id: usize, values: Vec<f32>, neighbors: Vec<usize>| HnswNode {
        candidate: candidate(&scope, id as u64 + 1, id, values),
        level: 0,
        neighbors: vec![neighbors],
    };
    // A limit of two at layer zero, with both reachable nodes already full of each other and
    // of a third, leaves nowhere to attach the orphan.
    let mut nodes = vec![
        node(0, vec![1.0, 0.0], vec![1, 2]),
        node(1, vec![0.9, 0.1], vec![0, 2]),
        node(2, vec![0.8, 0.2], vec![0, 1]),
        node(3, vec![0.0, 1.0], Vec::new()),
    ];
    let settings = clustered_config(&scope, 2, 1, 8);
    repair_reachability(&settings, &mut nodes, Some(0)).unwrap();
    assert!(nodes.iter().all(|node| node.neighbors[0].len() <= 2));
    assert!(nodes[3].neighbors[0].is_empty());
}

#[test]
fn selection_keeps_a_link_across_clusters_that_closeness_alone_would_drop() {
    let scope = ScopeId::new("instance:hnsw").unwrap();
    let node = |id: usize, values: Vec<f32>| HnswNode {
        candidate: candidate(&scope, id as u64 + 1, id, values),
        level: 0,
        neighbors: vec![Vec::new()],
    };
    // Three neighbors crowd one direction and one lies in another, with the query between
    // them. Closeness alone keeps two of the crowd, and node 3 would lose its link.
    let nodes = vec![
        node(0, vec![1.0, 0.00]),
        node(1, vec![1.0, 0.01]),
        node(2, vec![1.0, 0.02]),
        node(3, vec![0.0, 1.00]),
    ];
    let query = [0.7_f32, 0.7];
    let scored = (0..4)
        .map(|id| ScoredNode {
            id,
            score: score_dense_query(
                &query,
                &nodes[id].candidate,
                ScoreMetric::Cosine,
                HnswKernel::Scalar,
            )
            .unwrap(),
        })
        .collect::<Vec<_>>();
    let selected = select_neighbors(&nodes, scored, 2, ScoreMetric::Cosine).unwrap();
    assert_eq!(
        selected.iter().map(|scored| scored.id).collect::<Vec<_>>(),
        vec![2, 3]
    );
}

#[test]
fn a_graph_that_leaves_a_node_unreachable_is_still_refused() {
    let scope = ScopeId::new("instance:hnsw").unwrap();
    let node = |id: usize, values: Vec<f32>| HnswNode {
        candidate: candidate(&scope, id as u64 + 1, id, values),
        level: 0,
        neighbors: vec![Vec::new()],
    };
    let Err(error) = HnswIndex::from_body(HnswBody {
        config: config(&scope),
        generation: 1,
        source_cursor: 2,
        entrypoint: Some(0),
        indexed_delta_vectors: 2,
        nodes: vec![node(0, vec![1.0, 0.0]), node(1, vec![0.0, 1.0])],
        maintenance: HnswMaintenanceKind::FullBuild,
        previous_generation: None,
    }) else {
        panic!("an unreachable node must be refused");
    };
    let error = error.to_string();
    assert!(error.contains("not reachable"), "{error}");
}

#[test]
fn the_sample_changes_no_bytes_and_is_derived_on_read() {
    let (index, _) = flagged_index(1_000, |id| id % 2 == 0);
    assert!(index.cost_sample.len() <= COST_SAMPLE_IDENTITIES);
    let decoded = HnswIndex::from_bytes(index.as_bytes()).unwrap();
    assert_eq!(index.cost_sample, decoded.cost_sample);
    assert_eq!(index.as_bytes(), decoded.as_bytes());
    let text = String::from_utf8_lossy(index.as_bytes());
    assert!(
        !text.contains("cost_sample"),
        "the sample must not be stored"
    );
}

/// A graph over `points` two-dimensional vectors, where `selected` says who carries the flag.
fn flagged_index(points: usize, selected: impl Fn(usize) -> bool) -> (HnswIndex, ScopeId) {
    let scope = ScopeId::new("instance:hnsw-cost").unwrap();
    let values = (0..points).map(|index| {
        let angle = index as f32 * std::f32::consts::TAU / points as f32;
        let mut value = candidate(
            &scope,
            index as u64 + 1,
            index,
            vec![angle.cos(), angle.sin()],
        );
        value
            .vector
            .properties
            .insert("selected".into(), RuntimeValue::Bool(selected(index)));
        value
    });
    let mut build = config(&scope);
    build.filter_properties.insert("selected".into());
    let index = HnswIndex::build(build, 1, points as u64, values).unwrap();
    (index, scope)
}

/// A read past the generation's own cursor, as every read is once it has been published.
fn cost_request(scope: &ScopeId, cursor: u64, filtered: bool) -> SearchRequest {
    SearchRequest {
        scope: scope.clone(),
        read: ReadStamp::new(scope.clone(), None, 0, cursor, Some("11".repeat(32))).unwrap(),
        valid_at: 2,
        field: "body".into(),
        query: VectorQuery::Dense {
            values: vec![1.0, 0.0],
        },
        metric: ScoreMetric::Cosine,
        embedding_model: None,
        top_k: 1,
        mode: SearchMode::RequireApproximate { exact_rerank: 10 },
        filter: filtered.then(|| FilterExpression::Condition {
            condition: FilterCondition {
                property: "selected".into(),
                operator: FilterOperator::Equals {
                    value: RuntimeValue::Bool(true),
                },
            },
        }),
    }
}

#[test]
fn an_estimate_does_not_visit_the_whole_corpus() {
    let (index, scope) = flagged_index(4_000, |id| id % 2 == 0);
    for filtered in [false, true] {
        let request = cost_request(&scope, 4_001, filtered);
        IDENTITIES_VISITED.with(|visited| visited.set(0));
        index.estimated_search_cost(&request, 32).unwrap();
        let visited = IDENTITIES_VISITED.with(std::cell::Cell::get);
        assert!(visited <= 512, "one estimate visited {visited} identities");
    }
}

#[test]
fn a_common_filter_estimates_close_to_the_exact_count() {
    // One in ten carry the flag, so a search needs about ten times the candidates.
    let (index, scope) = flagged_index(4_000, |id| (id * 7) % 10 == 0);
    let cost = index
        .estimated_search_cost(&cost_request(&scope, 4_001, true), 32)
        .unwrap();
    let exact = 32 * 10 * HNSW_NAVIGATION_COST_MULTIPLIER as u64;
    assert!(
        cost >= exact / 2 && cost <= exact * 2,
        "estimated {cost} against {exact}"
    );
}

#[test]
fn no_filter_costs_about_what_ef_asks_for() {
    let (index, scope) = flagged_index(4_000, |_| true);
    let cost = index
        .estimated_search_cost(&cost_request(&scope, 4_001, false), 32)
        .unwrap();
    assert_eq!(cost, 32 * HNSW_NAVIGATION_COST_MULTIPLIER as u64);
}

#[test]
fn a_rare_filter_the_sample_misses_is_still_seen() {
    // Exactly one identity carries the flag, at a position an even stride skips.
    let (index, scope) = flagged_index(4_000, |id| id == 1_233);
    let cost = index
        .estimated_search_cost(&cost_request(&scope, 4_001, true), 10)
        .unwrap();
    // The same answer an exact count gives: one match, so the whole graph is worth walking.
    assert_eq!(cost, 4_000 * HNSW_NAVIGATION_COST_MULTIPLIER as u64);
}

#[test]
fn a_filter_nothing_matches_gives_the_cost_of_the_whole_graph() {
    let (index, scope) = flagged_index(1_000, |_| false);
    let cost = index
        .estimated_search_cost(&cost_request(&scope, 1_001, true), 10)
        .unwrap();
    assert_eq!(cost, 1_000 * HNSW_NAVIGATION_COST_MULTIPLIER as u64);
}

#[test]
fn hnsw_round_trip_is_deterministic_and_exact_reranks_candidates() {
    let scope = ScopeId::new("instance:hnsw").unwrap();
    let values = (0..64)
        .map(|index| {
            let angle = index as f32 * std::f32::consts::TAU / 64.0;
            candidate(
                &scope,
                index + 1,
                index as usize,
                vec![angle.cos(), angle.sin()],
            )
        })
        .collect::<Vec<_>>();
    let index = HnswIndex::build(config(&scope), 1, 64, values).unwrap();
    let decoded = HnswIndex::from_bytes(index.as_bytes()).unwrap();
    assert_eq!(index.descriptor(), decoded.descriptor());
    assert_eq!(index.as_bytes(), decoded.as_bytes());
    let request = SearchRequest {
        scope: scope.clone(),
        read: ReadStamp::new(scope, None, 0, 64, Some("11".repeat(32))).unwrap(),
        valid_at: 2,
        field: "body".into(),
        query: VectorQuery::Dense {
            values: vec![1.0, 0.0],
        },
        metric: ScoreMetric::Cosine,
        embedding_model: None,
        top_k: 5,
        mode: SearchMode::RequireApproximate { exact_rerank: 20 },
        filter: None,
    };
    let hits = decoded.search(&request, 32).unwrap();
    assert_eq!(hits.len(), 5);
    assert_eq!(hits[0].reference.id.as_str(), "v-000");
    assert_eq!(hits[0].score, 1.0);
}

#[test]
fn stale_or_corrupt_hnsw_fails_closed() {
    let scope = ScopeId::new("instance:hnsw-corrupt").unwrap();
    let index = HnswIndex::build(
        config(&scope),
        1,
        1,
        [candidate(&scope, 1, 0, vec![1.0, 0.0])],
    )
    .unwrap();
    let mut bytes = index.as_bytes().to_vec();
    let position = bytes.len() / 2;
    bytes[position] ^= 1;
    assert!(HnswIndex::from_bytes(&bytes).is_err());
    let request = SearchRequest {
        scope: scope.clone(),
        read: ReadStamp::new(scope, None, 0, 2, Some("11".repeat(32))).unwrap(),
        valid_at: 2,
        field: "body".into(),
        query: VectorQuery::Dense {
            values: vec![1.0, 0.0],
        },
        metric: ScoreMetric::Cosine,
        embedding_model: None,
        top_k: 1,
        mode: SearchMode::RequireApproximate { exact_rerank: 1 },
        filter: None,
    };
    assert!(index.search(&request, 1).is_err());
}

#[test]
fn selective_filter_admits_candidates_during_traversal_and_signals_crossover() {
    let scope = ScopeId::new("instance:hnsw-filter").unwrap();
    let mut values = (0..100)
        .map(|index| {
            let angle = index as f32 * std::f32::consts::TAU / 100.0;
            let mut value = candidate(
                &scope,
                index + 1,
                index as usize,
                vec![angle.cos(), angle.sin()],
            );
            value
                .vector
                .properties
                .insert("selected".into(), RuntimeValue::Bool(index == 37));
            value
        })
        .collect::<Vec<_>>();
    let mut build = config(&scope);
    build.filter_properties.insert("selected".into());
    let index = HnswIndex::build(build, 1, 100, values.drain(..)).unwrap();
    let request = SearchRequest {
        scope: scope.clone(),
        read: ReadStamp::new(scope, None, 0, 100, Some("11".repeat(32))).unwrap(),
        valid_at: 2,
        field: "body".into(),
        query: VectorQuery::Dense {
            values: vec![1.0, 0.0],
        },
        metric: ScoreMetric::Cosine,
        embedding_model: None,
        top_k: 1,
        mode: SearchMode::RequireApproximate { exact_rerank: 10 },
        filter: Some(FilterExpression::Condition {
            condition: FilterCondition {
                property: "selected".into(),
                operator: FilterOperator::Equals {
                    value: RuntimeValue::Bool(true),
                },
            },
        }),
    };
    assert_eq!(index.estimated_search_cost(&request, 10).unwrap(), 400);
    let hits = index.search(&request, 10).unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].reference.id.as_str(), "v-037");
}
