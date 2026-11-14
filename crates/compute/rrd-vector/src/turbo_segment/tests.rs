use super::*;
use crate::{FilterCondition, FilterExpression, FilterOperator, SearchMode};
use rrd_core::{ReadStamp, RuntimeValue, RuntimeVector, VectorValue};

fn config(scope: &ScopeId) -> TurboQuantSegmentConfig {
    TurboQuantSegmentConfig {
        id: ProjectionId::new("vector-turbo-body").unwrap(),
        scope: scope.clone(),
        field: "body".into(),
        dimensions: 64,
        metric: ScoreMetric::Cosine,
        bits: TurboQuantBits::Bits4,
        seed: 91,
        embedding_model: None,
        filter_properties: BTreeSet::from(["tenant".into()]),
        estimator: TurboQuantEstimator::Mse,
    }
}

fn candidate(scope: &ScopeId, cursor: u64, id: &str, tenant: &str) -> VectorCandidate {
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
            value: VectorValue::Dense {
                values: deterministic_vector(64, cursor + 10),
            },
            provenance: None,
            properties: RuntimeProperties::from([(
                "tenant".into(),
                RuntimeValue::String(tenant.into()),
            )]),
        },
    }
}

/// `id`'s version at `cursor`, ending at `valid_to` when it was superseded
/// or retired.
fn version(scope: &ScopeId, cursor: u64, id: &str, valid_to: Option<u64>) -> VectorCandidate {
    let mut candidate = candidate(scope, cursor, id, "a");
    candidate.vector.valid_to = valid_to;
    candidate
}

fn refs(ids: &[&str]) -> BTreeSet<RuntimeRef> {
    ids.iter()
        .map(|id| RuntimeRef::new("embedding", *id).unwrap())
        .collect()
}

#[test]
fn bounded_candidate_heap_matches_full_sort_for_ties_and_widths() {
    let scores = [0.0, -0.0, 1.0, -3.0, 1.0, 0.5, 10.0];
    let input = (0..211)
        .rev()
        .map(|i| SearchHit {
            reference: RuntimeRef::new("embedding", format!("p{:03}", i % 73)).unwrap(),
            subject: RuntimeRef::new("document", format!("p{i:03}")).unwrap(),
            source_cursor: i as u64 + 1,
            score: scores[i % scores.len()],
        })
        .collect::<Vec<_>>();
    let mut expected = input.clone();
    expected.sort_by(SearchHit::compare_best_first);
    for limit in [1, 2, 7, 19, 211, 250] {
        let mut selected = CandidateHits::new(limit);
        for hit in input.iter().cloned() {
            selected.consider(hit);
            assert!(selected.hits.len() <= limit);
        }
        assert_eq!(selected.finish(), expected[..limit.min(expected.len())]);
    }
}

/// The linear latest-version pass serves exactly the versions the exact
/// oracle does: newer versions not yet committed or not yet valid fall
/// back to the newest visible one, and a retired latest hides its identity.
#[test]
fn turbo_version_resolution_matches_the_exact_oracle() {
    let scope = ScopeId::new("instance:turbo-versions").unwrap();
    let mut versions = Vec::new();
    let mut cursor = 0;
    for identity in 0..24_u64 {
        for revision in 0..=(identity % 4) {
            cursor += 1;
            let mut version = candidate(
                &scope,
                cursor,
                &format!("p{identity:02}"),
                if (identity + revision) % 2 == 0 {
                    "a"
                } else {
                    "b"
                },
            );
            version.vector.valid_from = 1 + (identity * 7 + revision * 3) % 9;
            if (identity + revision) % 5 == 0 {
                version.vector.valid_to = Some(version.vector.valid_from + 4);
            }
            versions.push(version);
        }
    }
    let segment = TurboQuantSegment::build(config(&scope), 1, cursor, versions.clone()).unwrap();
    let (mut served_total, mut fell_back, mut hidden) = (0, 0, 0);
    for commit_cursor in [cursor / 3, cursor / 2, cursor] {
        for valid_at in [1, 4, 6, 9, 20] {
            for filtered in [false, true] {
                let request = SearchRequest {
                    scope: scope.clone(),
                    read: ReadStamp::new(
                        scope.clone(),
                        None,
                        0,
                        commit_cursor,
                        Some("11".repeat(32)),
                    )
                    .unwrap(),
                    valid_at,
                    field: "body".into(),
                    query: VectorQuery::Dense {
                        values: deterministic_vector(64, 7),
                    },
                    metric: ScoreMetric::Cosine,
                    embedding_model: None,
                    top_k: 1,
                    mode: SearchMode::RequireApproximate { exact_rerank: 1 },
                    filter: filtered.then(|| FilterExpression::Condition {
                        condition: FilterCondition {
                            property: "tenant".into(),
                            operator: FilterOperator::Equals {
                                value: RuntimeValue::String("a".into()),
                            },
                        },
                    }),
                };
                let served = segment
                    .search_candidates_at_with_kernel(
                        &request,
                        versions.len(),
                        0,
                        QuantizedKernel::Scalar,
                    )
                    .unwrap()
                    .into_iter()
                    .map(|hit| (hit.reference, hit.source_cursor))
                    .collect::<BTreeSet<_>>();
                let expected = crate::exact::visible_versions(&request, &versions)
                    .unwrap()
                    .into_iter()
                    .map(|(reference, source_cursor)| (reference.clone(), source_cursor))
                    .collect::<BTreeSet<_>>();
                assert_eq!(
                    served, expected,
                    "commit {commit_cursor}, valid at {valid_at}, filtered {filtered}"
                );
                let newest_committed = |reference: &RuntimeRef| {
                    versions
                        .iter()
                        .filter(|version| {
                            &version.vector.reference == reference
                                && version.source_cursor <= commit_cursor
                        })
                        .map(|version| version.source_cursor)
                        .max()
                };
                fell_back += served
                    .iter()
                    .filter(|(reference, cursor)| newest_committed(reference) != Some(*cursor))
                    .count();
                hidden += (0..24_u64)
                    .map(|identity| RuntimeRef::new("embedding", format!("p{identity:02}")))
                    .map(Result::unwrap)
                    .filter(|reference| {
                        newest_committed(reference).is_some()
                            && served.iter().all(|(served, _)| served != reference)
                    })
                    .count();
                served_total += served.len();
            }
        }
    }
    // The cases above must exercise every rule, not pass vacuously.
    assert!(served_total > 0 && fell_back > 0 && hidden > 0);
}

/// A header whose versions are out of order, or repeat one, is refused
/// even when its digest is valid, because search relies on that order.
#[test]
fn decode_refuses_versions_out_of_reference_and_cursor_order() {
    let scope = ScopeId::new("instance:turbo-order").unwrap();
    let segment = TurboQuantSegment::build(
        config(&scope),
        1,
        2,
        vec![
            candidate(&scope, 1, "a", "a"),
            candidate(&scope, 2, "b", "a"),
        ],
    )
    .unwrap();
    let bytes = segment.as_bytes();
    let header_len = u32::from_le_bytes(bytes[10..14].try_into().unwrap()) as usize;
    let header: SegmentHeader =
        serde_json::from_slice(&bytes[HEADER_PREFIX_BYTES..HEADER_PREFIX_BYTES + header_len])
            .unwrap();
    let width = header.packed_bytes_per_vector;
    let payload = segment.payload().to_vec();
    let forge = |candidates: Vec<StoredCandidate>, payload: Vec<u8>| {
        let header = SegmentHeader {
            candidates,
            ..header.clone()
        };
        let header_bytes = encode_json(&header).unwrap();
        let digest = artifact_digest(&header_bytes, &payload);
        encode_artifact(&header_bytes, &payload, &digest).unwrap()
    };
    let swapped = forge(
        vec![header.candidates[1].clone(), header.candidates[0].clone()],
        [&payload[width..], &payload[..width]].concat(),
    );
    let repeated = forge(
        vec![header.candidates[0].clone(), header.candidates[0].clone()],
        [&payload[..width], &payload[..width]].concat(),
    );
    for forged in [swapped, repeated] {
        let error = TurboQuantSegment::from_bytes(&forged).unwrap_err();
        assert!(
            error.to_string().contains("reference and cursor order"),
            "{error}"
        );
    }
}

#[test]
fn bounded_turbo_candidates_match_full_results_across_policy_and_visibility() {
    let scope = ScopeId::new("instance:bounded-turbo").unwrap();
    let mut versions = (1..=32)
        .map(|cursor| {
            candidate(
                &scope,
                cursor,
                &format!("point-{cursor:02}"),
                if cursor % 2 == 0 { "a" } else { "b" },
            )
        })
        .collect::<Vec<_>>();
    // Equal vectors force ties resolved by record identity. A later valid
    // version and a retirement exercise temporal selection before top-k.
    versions[1].vector.value = versions[0].vector.value.clone();
    versions[3].vector.valid_to = Some(5);
    let mut later = candidate(&scope, 33, "point-01", "a");
    later.vector.valid_from = 50;
    versions.push(later);
    for (bits, estimator) in [
        (TurboQuantBits::Bits4, TurboQuantEstimator::Mse),
        (TurboQuantBits::Bits2, TurboQuantEstimator::Mse),
        (TurboQuantBits::Bits1_5, TurboQuantEstimator::Mse),
        (TurboQuantBits::Bits1, TurboQuantEstimator::Mse),
        (TurboQuantBits::Bits4, TurboQuantEstimator::InnerProduct),
        (TurboQuantBits::Bits2, TurboQuantEstimator::InnerProduct),
        (TurboQuantBits::Bits1, TurboQuantEstimator::InnerProduct),
    ] {
        let mut policy = config(&scope);
        policy.bits = bits;
        policy.estimator = estimator;
        let segment = TurboQuantSegment::build(policy, 1, 33, versions.clone()).unwrap();
        let reopened = TurboQuantSegment::from_bytes(segment.as_bytes()).unwrap();
        let directory = tempfile::tempdir().unwrap();
        let artifact = directory.path().join("segment");
        std::fs::write(&artifact, segment.as_bytes()).unwrap();
        let mapped = TurboQuantSegment::open_mmap(&artifact).unwrap();
        for valid_at in [2, 10, 60] {
            for filtered in [false, true] {
                let request = SearchRequest {
                    scope: scope.clone(),
                    read: ReadStamp::new(scope.clone(), None, 0, 33, Some("11".repeat(32)))
                        .unwrap(),
                    valid_at,
                    field: "body".into(),
                    query: VectorQuery::Dense {
                        values: deterministic_vector(64, 99),
                    },
                    metric: ScoreMetric::Cosine,
                    embedding_model: None,
                    top_k: 1,
                    mode: SearchMode::RequireApproximate { exact_rerank: 64 },
                    filter: filtered.then(|| FilterExpression::Condition {
                        condition: FilterCondition {
                            property: "tenant".into(),
                            operator: FilterOperator::Equals {
                                value: RuntimeValue::String("a".into()),
                            },
                        },
                    }),
                };
                let full = reopened
                    .search_candidates_at_with_kernel(&request, 64, 33, QuantizedKernel::Scalar)
                    .unwrap();
                for limit in [1, 3, 8, 16, 64] {
                    for artifact in [&reopened, &mapped] {
                        let narrow = artifact
                            .search_candidates_at_with_kernel(
                                &request,
                                limit,
                                33,
                                QuantizedKernel::Scalar,
                            )
                            .unwrap();
                        assert_eq!(
                            narrow,
                            full[..limit.min(full.len())],
                            "{bits:?} {estimator:?} {valid_at} {filtered} {limit}"
                        );
                    }
                }
            }
        }
    }
}

/// MSE artifacts keep the bytes they had before the inner-product
/// estimator existed: these digests and scores come from the codec at
/// E-04c (`69c36845`), on the same versions.
#[test]
fn mse_artifacts_keep_their_bytes_and_scores() {
    let scope = ScopeId::new("instance:probe").unwrap();
    let candidates = (1..=40u64)
        .map(|cursor| VectorCandidate {
            scope: scope.clone(),
            source_cursor: cursor,
            vector: RuntimeVector {
                reference: RuntimeRef::new("embedding", format!("p{cursor}")).unwrap(),
                subject: RuntimeRef::new("document", format!("p{cursor}")).unwrap(),
                collection: VectorCollectionAddress {
                    collection_id: "documents".into(),
                    vector_name: "body".into(),
                },
                field: "body".into(),
                valid_from: 1,
                valid_to: None,
                value: VectorValue::Dense {
                    values: (0..48)
                        .map(|dimension| {
                            (((cursor * 37 + dimension * 11) % 101) as f32) / 50.0 - 1.0
                        })
                        .collect(),
                },
                provenance: None,
                properties: RuntimeProperties::new(),
            },
        })
        .collect::<Vec<_>>();
    for (bits, digest, top) in [
        (
            TurboQuantBits::Bits4,
            "e29b417205f42a2ee95e932201784ceb9cccfb17c003b11d8b537c7cba20cc52",
            ("p35", 1.205_116_994_953_018_3),
        ),
        (
            TurboQuantBits::Bits2,
            "b1d05c52291448dc675e394ec7cc618b0546fcf3c3586f8033cde196733f8286",
            ("p5", 1.626_582_098_861_267_1),
        ),
        (
            TurboQuantBits::Bits1_5,
            "e1c691b939bd8e872d20f1096a3526861d0cc013352e0a83e5cdf592ba978e1a",
            ("p5", 1.633_507_625_007_59),
        ),
        (
            TurboQuantBits::Bits1,
            "32e3b64ef9cec42a5c4c287bfc6f939f44a615b9a545b3a846ced11cdfea9157",
            ("p5", 1.519_326_381_657_862_8),
        ),
    ] {
        let segment = TurboQuantSegment::build(
            TurboQuantSegmentConfig {
                id: ProjectionId::new("probe").unwrap(),
                scope: scope.clone(),
                field: "body".into(),
                dimensions: 48,
                metric: ScoreMetric::Dot,
                bits,
                seed: 5,
                embedding_model: None,
                filter_properties: BTreeSet::new(),
                estimator: TurboQuantEstimator::Mse,
            },
            1,
            40,
            candidates.clone(),
        )
        .unwrap();
        assert_eq!(
            segment.descriptor().stamp.artifact_digest,
            digest,
            "{bits:?}"
        );
        let hits = segment
            .search_candidates_at(
                &SearchRequest {
                    scope: scope.clone(),
                    read: ReadStamp::new(scope.clone(), None, 0, 40, Some("11".repeat(32)))
                        .unwrap(),
                    valid_at: 2,
                    field: "body".into(),
                    metric: ScoreMetric::Dot,
                    embedding_model: None,
                    query: VectorQuery::Dense {
                        values: (0..48)
                            .map(|dimension| dimension as f32 / 48.0 - 0.5)
                            .collect(),
                    },
                    filter: None,
                    top_k: 5,
                    mode: SearchMode::RequireApproximate { exact_rerank: 40 },
                },
                40,
                40,
            )
            .unwrap();
        assert_eq!(hits[0].reference.id.as_str(), top.0, "{bits:?}");
        assert!(
            (hits[0].score - top.1).abs() < 1e-9,
            "{bits:?}: {}",
            hits[0].score
        );
    }
}

#[test]
fn an_inner_product_configuration_refuses_what_its_estimator_cannot_serve() {
    let scope = ScopeId::new("instance:turbo-inner-product-configuration").unwrap();
    let inner_product = |change: &dyn Fn(&mut TurboQuantSegmentConfig)| {
        let mut config = config(&scope);
        config.estimator = TurboQuantEstimator::InnerProduct;
        change(&mut config);
        config.validate()
    };
    assert!(inner_product(&|_| {}).is_ok());
    assert!(inner_product(&|config| config.bits = TurboQuantBits::Bits1_5).is_err());
    assert!(inner_product(&|config| config.metric = ScoreMetric::Manhattan).is_err());
    assert!(inner_product(&|config| config.dimensions = MAX_QJL_DIMENSIONS).is_ok());
    assert!(inner_product(&|config| config.dimensions = MAX_QJL_DIMENSIONS + 1).is_err());
    // The MSE estimator keeps every width and metric.
    let mut mse = config(&scope);
    mse.bits = TurboQuantBits::Bits1_5;
    mse.metric = ScoreMetric::Manhattan;
    assert!(mse.validate().is_ok());
}

#[test]
fn extending_a_generation_equals_a_full_build() {
    let scope = ScopeId::new("instance:turbo-extend").unwrap();
    let first = (1..=6)
        .map(|cursor| version(&scope, cursor, &format!("point-{cursor}"), None))
        .collect::<Vec<_>>();
    let base = TurboQuantSegment::build(config(&scope), 1, 6, first.clone()).unwrap();

    // Since the base: point-2 is updated, point-3 is retired, point-4
    // leaves this source (its history here is unchanged), and point-7 is
    // new. Each changed identity arrives with its complete history.
    let histories = vec![
        version(&scope, 2, "point-2", Some(7)),
        version(&scope, 7, "point-2", None),
        version(&scope, 3, "point-3", Some(8)),
        version(&scope, 4, "point-4", None),
        version(&scope, 9, "point-7", None),
    ];
    let replaced = refs(&["point-2", "point-3", "point-4", "point-7"]);
    let extended =
        TurboQuantSegment::extend(&base, config(&scope), 2, &replaced, histories.clone(), 9)
            .unwrap();

    let unchanged = first
        .into_iter()
        .filter(|candidate| !replaced.contains(&candidate.vector.reference));
    let full = TurboQuantSegment::build(config(&scope), 2, 9, unchanged.chain(histories)).unwrap();
    assert_eq!(extended.descriptor(), full.descriptor());
    assert_eq!(extended.as_bytes(), full.as_bytes());
    assert_eq!(extended.descriptor().candidate_versions, 8);
    assert_eq!(extended.descriptor().stamp.source_cursor, 9);

    // Extending again from the extended generation stays exact.
    let again = TurboQuantSegment::extend(
        &extended,
        config(&scope),
        3,
        &refs(&["point-1"]),
        [
            version(&scope, 10, "point-1", None),
            version(&scope, 1, "point-1", Some(10)),
        ],
        10,
    )
    .unwrap();
    let reopened = TurboQuantSegment::from_bytes(again.as_bytes()).unwrap();
    assert_eq!(reopened.descriptor(), again.descriptor());
    assert_eq!(again.descriptor().candidate_versions, 9);
}

#[test]
fn extending_refuses_what_a_full_build_would_not_make() {
    let scope = ScopeId::new("instance:turbo-extend-refusals").unwrap();
    let base = TurboQuantSegment::build(
        config(&scope),
        2,
        2,
        [version(&scope, 1, "a", None), version(&scope, 2, "b", None)],
    )
    .unwrap();
    let changed = refs(&["b"]);
    let fresh = || [version(&scope, 3, "b", None)];

    let mut seeded = config(&scope);
    seeded.seed += 1;
    assert!(TurboQuantSegment::extend(&base, seeded, 3, &changed, fresh(), 3).is_err());
    let mut narrower = config(&scope);
    narrower.bits = TurboQuantBits::Bits2;
    assert!(TurboQuantSegment::extend(&base, narrower, 3, &changed, fresh(), 3).is_err());
    assert!(
        TurboQuantSegment::extend(&base, config(&scope), 2, &changed, fresh(), 3).is_err(),
        "the next generation is later than its base"
    );
    assert!(
        TurboQuantSegment::extend(&base, config(&scope), 3, &changed, fresh(), 1).is_err(),
        "the next generation never covers less than its base"
    );
    assert!(
        TurboQuantSegment::extend(&base, config(&scope), 3, &changed, fresh(), 2).is_err(),
        "the next generation covers every version it holds"
    );
    assert!(
        TurboQuantSegment::extend(
            &base,
            config(&scope),
            3,
            &changed,
            [version(&scope, 3, "a", None)],
            3,
        )
        .is_err(),
        "an unchanged identity keeps its stored versions"
    );
    assert!(
        TurboQuantSegment::extend(&base, config(&scope), 3, &refs(&["a", "b"]), [], 3).is_err(),
        "a generation holds at least one version"
    );
    let other = ScopeId::new("instance:turbo-extend-other").unwrap();
    assert!(TurboQuantSegment::extend(
        &base,
        config(&scope),
        3,
        &changed,
        [version(&other, 3, "b", None)],
        3,
    )
    .is_err());
    assert!(TurboQuantSegment::extend(&base, config(&scope), 3, &changed, fresh(), 3).is_ok());
}

#[test]
fn artifact_reader_rejects_candidate_without_collection_address() {
    let scope = ScopeId::new("instance:turbo-required-address").unwrap();
    let segment =
        TurboQuantSegment::build(config(&scope), 1, 1, [candidate(&scope, 1, "a", "a")]).unwrap();
    let header_len = u32::from_le_bytes(
        segment.as_bytes()[10..14]
            .try_into()
            .expect("fixed header length"),
    ) as usize;
    let header_end = HEADER_PREFIX_BYTES + header_len;
    let mut header: serde_json::Value =
        serde_json::from_slice(&segment.as_bytes()[HEADER_PREFIX_BYTES..header_end]).unwrap();
    header["candidates"][0]
        .as_object_mut()
        .unwrap()
        .remove("collection");
    let header = serde_json::to_vec(&header).unwrap();
    let payload = &segment.as_bytes()[header_end..];
    let checksum = artifact_digest(&header, payload);
    let bytes = encode_artifact(&header, payload, &checksum).unwrap();

    let error = TurboQuantSegment::from_bytes(&bytes).unwrap_err();
    assert!(error.to_string().contains("missing field `collection`"));
}

#[test]
fn binary_artifact_round_trips_filters_and_accounts_for_compression() {
    let scope = ScopeId::new("instance:turbo-segment").unwrap();
    let candidates = (1..=32)
        .map(|cursor| {
            candidate(
                &scope,
                cursor,
                &format!("point-{cursor:02}"),
                if cursor % 2 == 0 { "a" } else { "b" },
            )
        })
        .collect::<Vec<_>>();
    let segment = TurboQuantSegment::build(config(&scope), 1, 32, candidates).unwrap();
    assert_eq!(segment.descriptor.packed_vector_bytes, 32 * 32);
    assert_eq!(segment.descriptor.full_precision_vector_bytes, 32 * 64 * 4);
    let reopened = TurboQuantSegment::from_bytes(segment.as_bytes()).unwrap();
    assert_eq!(reopened.descriptor(), segment.descriptor());

    let request = SearchRequest {
        scope: scope.clone(),
        read: ReadStamp::new(scope, None, 0, 32, Some("11".repeat(32))).unwrap(),
        valid_at: 100,
        field: "body".into(),
        query: VectorQuery::Dense {
            values: deterministic_vector(64, 99),
        },
        metric: ScoreMetric::Cosine,
        embedding_model: None,
        top_k: 4,
        mode: SearchMode::RequireApproximate { exact_rerank: 8 },
        filter: Some(FilterExpression::Condition {
            condition: FilterCondition {
                property: "tenant".into(),
                operator: FilterOperator::Equals {
                    value: RuntimeValue::String("a".into()),
                },
            },
        }),
    };
    let hits = reopened.search_candidates_at(&request, 8, 32).unwrap();
    assert_eq!(hits.len(), 8);
    assert!(hits.iter().all(|hit| {
        hit.reference
            .id
            .as_str()
            .chars()
            .last()
            .and_then(|character| character.to_digit(10))
            .is_some_and(|digit| digit % 2 == 0)
    }));
}

#[test]
fn artifact_corruption_and_stale_reads_fail_closed() {
    let scope = ScopeId::new("instance:turbo-corrupt").unwrap();
    let segment =
        TurboQuantSegment::build(config(&scope), 1, 1, [candidate(&scope, 1, "point", "a")])
            .unwrap();
    let mut corrupt = segment.as_bytes().to_vec();
    *corrupt.last_mut().unwrap() ^= 0x80;
    assert!(TurboQuantSegment::from_bytes(&corrupt).is_err());

    let request = SearchRequest {
        scope: scope.clone(),
        read: ReadStamp::new(scope, None, 0, 2, Some("11".repeat(32))).unwrap(),
        valid_at: 100,
        field: "body".into(),
        query: VectorQuery::Dense {
            values: deterministic_vector(64, 99),
        },
        metric: ScoreMetric::Cosine,
        embedding_model: None,
        top_k: 1,
        mode: SearchMode::RequireApproximate { exact_rerank: 1 },
        filter: None,
    };
    assert!(segment.search_candidates_at(&request, 1, 2).is_err());
}

fn deterministic_vector(dimensions: usize, seed: u64) -> Vec<f32> {
    let mut state = seed;
    (0..dimensions)
        .map(|index| {
            state = mix(state, index as u64 + 1);
            let unit = (state >> 40) as f32 / ((1_u32 << 24) - 1) as f32;
            unit * 2.0 - 1.0
        })
        .collect()
}

fn mix(mut value: u64, salt: u64) -> u64 {
    value ^= salt.wrapping_mul(0x9e37_79b9_7f4a_7c15);
    value ^= value >> 30;
    value = value.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}
