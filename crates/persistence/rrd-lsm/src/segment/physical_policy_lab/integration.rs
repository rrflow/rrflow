//! Value placement, reopened databases, integrated cache policy and candidate decisions.

use super::*;

pub(super) fn observe_value_placement(table: &Memtable) -> Result<ValuePlacementObservation> {
    let mut non_null_versions = 0u64;
    let mut inline_value_bytes = 0u64;
    let mut latest_live_value_bytes = 0u64;
    for (_, versions) in table.all_versions() {
        for version in versions {
            if let Some(value) = &version.value {
                non_null_versions += 1;
                inline_value_bytes = inline_value_bytes
                    .checked_add(value.len() as u64)
                    .ok_or_else(|| Error::InvalidSegment("value byte total overflow".into()))?;
            }
        }
        if let Some(value) = versions.last().and_then(|version| version.value.as_ref()) {
            latest_live_value_bytes = latest_live_value_bytes
                .checked_add(value.len() as u64)
                .ok_or_else(|| Error::InvalidSegment("live value byte total overflow".into()))?;
        }
    }
    let separated_pointer_bytes = non_null_versions
        .checked_mul(VALUE_POINTER_BYTES as u64)
        .ok_or_else(|| Error::InvalidSegment("modeled pointer byte total overflow".into()))?;
    Ok(ValuePlacementObservation {
        evidence_kind: "analytical-model-only".into(),
        non_null_versions,
        inline_value_bytes,
        latest_live_value_bytes,
        obsolete_value_bytes: inline_value_bytes.saturating_sub(latest_live_value_bytes),
        separated_pointer_bytes,
        append_only_value_log_bytes: inline_value_bytes,
        modeled_compaction_bytes_avoided: inline_value_bytes
            .saturating_sub(separated_pointer_bytes),
        production_retainable: false,
        missing_proofs: vec![
            "authenticated pointer encoding".into(),
            "atomic WAL and manifest publication".into(),
            "snapshot and backup closure".into(),
            "value-log garbage collection".into(),
            "range-read amplification".into(),
            "corruption and crash recovery".into(),
        ],
    })
}

pub(super) fn observe_families(pages: &[PageSample]) -> Result<Vec<FamilyObservation>> {
    let mut grouped: BTreeMap<String, Vec<PageSample>> = BTreeMap::new();
    for page in pages {
        grouped
            .entry(page.family.clone())
            .or_default()
            .push(page.clone());
    }
    grouped
        .into_iter()
        .map(|(family, samples)| {
            let mut row_groups = BTreeMap::new();
            for sample in &samples {
                row_groups.insert(sample.row_group, sample.row_count);
            }
            let none = observe_codec("none", &samples)?;
            let lz4 = observe_codec("lz4", &samples)?;
            let zstd = observe_codec("zstd", &samples)?;
            Ok(FamilyObservation {
                family,
                rows: row_groups.values().copied().sum(),
                page_count: samples.len() as u64,
                logical_page_bytes: none.logical_bytes,
                lz4_adaptive_bytes: lz4.adaptive_stored_bytes,
                zstd_adaptive_bytes: zstd.adaptive_stored_bytes,
            })
        })
        .collect()
}

pub(super) fn observe_reopened_database(
    root: &Path,
    config: PhysicalPolicyConfig,
    corpus: &Corpus,
) -> Result<ReopenedPointMissObservation> {
    let options = DatabaseOptions {
        page_cache_bytes: config.cache_bytes,
        segment_row_group_budget: integrated_row_group_budget(config)?,
        ..DatabaseOptions::default()
    };
    let mut database = Database::create_with_options(root, options)?;
    database.write_owned(
        WriteBatch::new(corpus.mutations.clone())?,
        Durability::Authoritative,
    )?;
    database.flush_memtable(1)?;
    let snapshot = database.snapshot();
    let segment_physical_bytes =
        database
            .manifest()
            .segments
            .iter()
            .try_fold(0u64, |total, segment| {
                total.checked_add(segment.bytes).ok_or_else(|| {
                    Error::InvalidSegment("manifest segment byte total overflow".into())
                })
            })?;
    drop(database);

    let database = Database::open_with_options(root, options)?;
    if database.snapshot() != snapshot {
        return Err(Error::InvalidSegment(
            "normal reopen changed the candidate corpus snapshot".into(),
        ));
    }
    let open = database.segment_open_evidence().clone();
    let before = database.page_cache_stats();
    let started = Instant::now();
    for probe in 0..config.point_misses {
        let family = probe % FAMILY_PREFIXES.len();
        let ordinal = (probe / FAMILY_PREFIXES.len()) % config.records_per_family;
        let key = corpus_key(FAMILY_PREFIXES[family].1, ordinal, true);
        if database.get(&key, snapshot)?.is_some() {
            return Err(Error::InvalidSegment(
                "normal reopen returned a value for an absent corpus key".into(),
            ));
        }
    }
    let elapsed_nanoseconds = started.elapsed().as_nanos().try_into().unwrap_or(u64::MAX);
    let after_misses = database.page_cache_stats();
    let mut hit_samples_verified = 0u64;
    for (key, expected) in corpus.expected_latest.iter().take(128) {
        if database.get(key, snapshot)? != *expected {
            return Err(Error::InvalidSegment(
                "normal reopen returned a wrong present/tombstoned value".into(),
            ));
        }
        hit_samples_verified += 1;
    }
    let after_reads = database.page_cache_stats();
    Ok(ReopenedPointMissObservation {
        integrated_rrflowkv: true,
        open_none_policy_segment_count: open.none_policy_segment_count,
        open_adaptive_lz4_policy_segment_count: open.adaptive_lz4_policy_segment_count,
        open_raw_page_count: open.raw_page_count,
        open_compressed_page_count: open.compressed_page_count,
        open_stored_page_bytes: open.stored_page_bytes,
        open_logical_page_bytes: open.logical_page_bytes,
        open_persisted_filter_count: open.persisted_filter_count,
        open_persisted_filter_bytes: open.persisted_filter_bytes,
        open_semantic_page_operations: open.semantic_page_operations,
        open_semantic_page_bytes: open.semantic_page_bytes,
        misses_verified: config.point_misses as u64,
        hit_samples_verified,
        elapsed_nanoseconds,
        segment_physical_bytes,
        filter_checks: after_misses
            .filter_checks
            .saturating_sub(before.filter_checks),
        filter_negatives: after_misses
            .filter_negatives
            .saturating_sub(before.filter_negatives),
        page_cache_hits: after_reads.hits.saturating_sub(before.hits),
        page_cache_misses: after_reads.misses.saturating_sub(before.misses),
        page_loads: after_reads.loads.saturating_sub(before.loads),
        bytes_read: after_reads.bytes_read.saturating_sub(before.bytes_read),
        bytes_decoded: after_reads
            .bytes_decoded
            .saturating_sub(before.bytes_decoded),
        bytes_decompressed: after_reads
            .bytes_decompressed
            .saturating_sub(before.bytes_decompressed),
        final_cache_resident_bytes: after_reads.resident_bytes as u64,
    })
}

pub(super) fn integrated_row_group_budget(
    config: PhysicalPolicyConfig,
) -> Result<SegmentRowGroupBudget> {
    let max_rows = config
        .versions_per_key
        .checked_mul(4)
        .ok_or_else(|| Error::InvalidConfiguration("cache-lab row bound overflow".into()))?;
    let target_bytes = config
        .value_bytes
        .checked_mul(max_rows)
        .and_then(|bytes| bytes.checked_add(1_024))
        .ok_or_else(|| Error::InvalidConfiguration("cache-lab byte target overflow".into()))?;
    Ok(SegmentRowGroupBudget {
        max_rows,
        target_bytes,
    })
}

pub(super) fn observe_integrated_cache_policies(
    root: &Path,
    config: PhysicalPolicyConfig,
    corpus: &Corpus,
) -> Result<Vec<IntegratedCachePolicyObservation>> {
    let policies = [
        PageCachePolicy::ExactLru,
        PageCachePolicy::ScanResistantLru {
            protected_capacity_basis_points:
                crate::DEFAULT_PAGE_CACHE_PROTECTED_CAPACITY_BASIS_POINTS,
        },
    ];
    policies
        .into_iter()
        .map(|policy| observe_integrated_cache_policy(root, config, corpus, policy))
        .collect()
}

pub(super) fn observe_integrated_cache_policy(
    root: &Path,
    config: PhysicalPolicyConfig,
    corpus: &Corpus,
    policy: PageCachePolicy,
) -> Result<IntegratedCachePolicyObservation> {
    let database = Database::open_with_options(
        root,
        DatabaseOptions {
            page_cache_bytes: config.cache_bytes,
            page_cache_policy: policy,
            segment_row_group_budget: integrated_row_group_budget(config)?,
            ..DatabaseOptions::default()
        },
    )?;
    let snapshot = database.snapshot();
    let manifest_digest =
        sha256_hex(&serde_json::to_vec(database.manifest()).map_err(|error| {
            Error::InvalidSegment(format!("cache-lab manifest encoding failed: {error}"))
        })?);
    let hot_keys = FAMILY_PREFIXES
        .iter()
        .map(|(_, prefix)| corpus_key(prefix, 1, false))
        .collect::<Vec<_>>();
    let expected_hot_values = hot_keys
        .iter()
        .map(|key| {
            corpus
                .expected_latest
                .get(key)
                .and_then(Clone::clone)
                .ok_or_else(|| {
                    Error::InvalidSegment(
                        "cache-lab hot key is not present in the expected corpus".into(),
                    )
                })
        })
        .collect::<Result<Vec<_>>>()?;
    let expected_rows = corpus
        .expected_latest
        .iter()
        .filter_map(|(key, value)| {
            value
                .as_ref()
                .map(|value| (key.clone(), Some(value.clone())))
        })
        .collect::<Vec<_>>();
    let expected_row_digest = digest_projected_rows(&expected_rows)?;

    let before = database.page_cache_stats();
    let first_hot_values = read_integrated_hot_values(&database, snapshot, &hot_keys)?;
    let second_hot_values = read_integrated_hot_values(&database, snapshot, &hot_keys)?;
    let after_warm = database.page_cache_stats();

    let mut stream = database.begin_projected_read(ProjectedReadRequest {
        ranges: vec![ProjectedReadRange::all()],
        projection: ProjectedReadProjection::KeyValue,
        snapshot,
        budget: ProjectedReadBudget {
            max_batch_rows: 257,
            ..ProjectedReadBudget::default()
        },
    })?;
    let mut rows = Vec::with_capacity(expected_rows.len());
    while let Some(batch) = stream.next_batch()? {
        for row in 0..batch.len() {
            rows.push((
                batch
                    .key(row)
                    .ok_or_else(|| {
                        Error::InvalidSegment("cache-lab projected key is absent".into())
                    })?
                    .to_vec(),
                batch.value(row).map(<[u8]>::to_vec),
            ));
        }
    }
    drop(stream);
    let after_scan = database.page_cache_stats();
    let post_scan_hot_values = read_integrated_hot_values(&database, snapshot, &hot_keys)?;
    let after_post_scan = database.page_cache_stats();
    let projected_row_digest = digest_projected_rows(&rows)?;
    let semantic_identity_exact = first_hot_values == expected_hot_values
        && second_hot_values == expected_hot_values
        && post_scan_hot_values == expected_hot_values
        && rows == expected_rows
        && projected_row_digest == expected_row_digest;
    let classified_bytes = after_post_scan
        .probationary_resident_bytes
        .checked_add(after_post_scan.protected_resident_bytes)
        .ok_or_else(|| Error::InvalidSegment("cache-lab region byte total overflow".into()))?;
    let classified_entries = after_post_scan
        .probationary_entries
        .checked_add(after_post_scan.protected_entries)
        .ok_or_else(|| Error::InvalidSegment("cache-lab region entry total overflow".into()))?;
    let exact_capacity_respected = after_post_scan.resident_bytes <= config.cache_bytes
        && match policy {
            PageCachePolicy::ExactLru => classified_bytes == 0 && classified_entries == 0,
            PageCachePolicy::ScanResistantLru { .. } => {
                classified_bytes == after_post_scan.resident_bytes
                    && classified_entries == after_post_scan.entries
            }
        };
    let semantic_digest = digest_cache_semantics(&rows, &post_scan_hot_values)?;

    Ok(IntegratedCachePolicyObservation {
        policy: policy.kind().into(),
        protected_capacity_basis_points: u64::from(policy.protected_capacity_basis_points()),
        snapshot_sequence: snapshot.sequence,
        manifest_digest,
        semantic_digest,
        projected_row_digest,
        hot_family_count: FAMILY_PREFIXES.len() as u64,
        hot_requests_per_pass: hot_keys.len() as u64,
        projected_rows: rows.len() as u64,
        capacity_bytes: after_post_scan.capacity_bytes as u64,
        final_resident_bytes: after_post_scan.resident_bytes as u64,
        probationary_resident_bytes: after_post_scan.probationary_resident_bytes as u64,
        protected_resident_bytes: after_post_scan.protected_resident_bytes as u64,
        final_entries: after_post_scan.entries as u64,
        probationary_entries: after_post_scan.probationary_entries as u64,
        protected_entries: after_post_scan.protected_entries as u64,
        warm_hits: counter_delta(after_warm.hits, before.hits, "warm hits")?,
        warm_misses: counter_delta(after_warm.misses, before.misses, "warm misses")?,
        warm_loads: counter_delta(after_warm.loads, before.loads, "warm loads")?,
        scan_hits: counter_delta(after_scan.hits, after_warm.hits, "scan hits")?,
        scan_misses: counter_delta(after_scan.misses, after_warm.misses, "scan misses")?,
        scan_loads: counter_delta(after_scan.loads, after_warm.loads, "scan loads")?,
        post_scan_hot_hits: counter_delta(
            after_post_scan.hits,
            after_scan.hits,
            "post-scan hot hits",
        )?,
        post_scan_hot_misses: counter_delta(
            after_post_scan.misses,
            after_scan.misses,
            "post-scan hot misses",
        )?,
        post_scan_hot_loads: counter_delta(
            after_post_scan.loads,
            after_scan.loads,
            "post-scan hot loads",
        )?,
        admissions: counter_delta(after_post_scan.admissions, before.admissions, "admissions")?,
        admission_rejections: counter_delta(
            after_post_scan.admission_rejections,
            before.admission_rejections,
            "admission rejections",
        )?,
        promotions: counter_delta(after_post_scan.promotions, before.promotions, "promotions")?,
        demotions: counter_delta(after_post_scan.demotions, before.demotions, "demotions")?,
        same_scope_hits: counter_delta(
            after_post_scan.same_scope_hits,
            before.same_scope_hits,
            "same-scope hits",
        )?,
        evictions: counter_delta(after_post_scan.evictions, before.evictions, "evictions")?,
        duplicate_loads: counter_delta(
            after_post_scan.duplicate_loads,
            before.duplicate_loads,
            "duplicate loads",
        )?,
        bytes_read: counter_delta(after_post_scan.bytes_read, before.bytes_read, "bytes read")?,
        bytes_decoded: counter_delta(
            after_post_scan.bytes_decoded,
            before.bytes_decoded,
            "bytes decoded",
        )?,
        bytes_decompressed: counter_delta(
            after_post_scan.bytes_decompressed,
            before.bytes_decompressed,
            "bytes decompressed",
        )?,
        semantic_identity_exact,
        exact_capacity_respected,
    })
}

pub(super) fn read_integrated_hot_values(
    database: &Database,
    snapshot: crate::Snapshot,
    hot_keys: &[Vec<u8>],
) -> Result<Vec<Vec<u8>>> {
    hot_keys
        .iter()
        .map(|key| {
            database.get(key, snapshot)?.ok_or_else(|| {
                Error::InvalidSegment("cache-lab present hot key was not visible".into())
            })
        })
        .collect()
}

pub(super) fn digest_projected_rows(rows: &[(Vec<u8>, Option<Vec<u8>>)]) -> Result<String> {
    let mut encoded = Vec::new();
    for (key, value) in rows {
        encoded.extend_from_slice(
            &u64::try_from(key.len())
                .map_err(|_| Error::InvalidSegment("cache-lab key length exceeds u64".into()))?
                .to_le_bytes(),
        );
        encoded.extend_from_slice(key);
        match value {
            Some(value) => {
                encoded.push(1);
                encoded.extend_from_slice(
                    &u64::try_from(value.len())
                        .map_err(|_| {
                            Error::InvalidSegment("cache-lab value length exceeds u64".into())
                        })?
                        .to_le_bytes(),
                );
                encoded.extend_from_slice(value);
            }
            None => {
                encoded.push(0);
                encoded.extend_from_slice(&0u64.to_le_bytes());
            }
        }
    }
    Ok(sha256_hex(&encoded))
}

pub(super) fn digest_cache_semantics(
    rows: &[(Vec<u8>, Option<Vec<u8>>)],
    hot_values: &[Vec<u8>],
) -> Result<String> {
    let mut encoded = digest_projected_rows(rows)?.into_bytes();
    for value in hot_values {
        encoded.extend_from_slice(
            &u64::try_from(value.len())
                .map_err(|_| Error::InvalidSegment("cache-lab hot value exceeds u64".into()))?
                .to_le_bytes(),
        );
        encoded.extend_from_slice(value);
    }
    Ok(sha256_hex(&encoded))
}

pub(super) fn counter_delta(after: u64, before: u64, name: &str) -> Result<u64> {
    after
        .checked_sub(before)
        .ok_or_else(|| Error::InvalidSegment(format!("cache-lab {name} counter moved backwards")))
}

pub(super) fn validate_integrated_cache_policies(
    observations: &[IntegratedCachePolicyObservation],
) -> Result<()> {
    let exact = observations
        .iter()
        .find(|observation| observation.policy == PageCachePolicy::ExactLru.kind())
        .ok_or_else(|| Error::InvalidSegment("integrated exact-LRU evidence is absent".into()))?;
    let selected_policy = PageCachePolicy::default();
    let scan_resistant = observations
        .iter()
        .find(|observation| observation.policy == selected_policy.kind())
        .ok_or_else(|| {
            Error::InvalidSegment("integrated scan-resistant evidence is absent".into())
        })?;
    if !exact.semantic_identity_exact
        || !scan_resistant.semantic_identity_exact
        || !exact.exact_capacity_respected
        || !scan_resistant.exact_capacity_respected
        || exact.snapshot_sequence != scan_resistant.snapshot_sequence
        || exact.manifest_digest != scan_resistant.manifest_digest
        || exact.semantic_digest != scan_resistant.semantic_digest
        || exact.projected_row_digest != scan_resistant.projected_row_digest
        || exact.post_scan_hot_loads == 0
        || scan_resistant.post_scan_hot_loads >= exact.post_scan_hot_loads
        || scan_resistant.promotions == 0
        || scan_resistant.same_scope_hits == 0
        || scan_resistant.protected_entries == 0
    {
        return Err(Error::InvalidSegment(format!(
            "integrated scan-resistant cache policy failed: exact_post_scan_loads={}, scan_resistant_post_scan_loads={}, scan_warm_hits={}, scan_warm_loads={}, scan_scan_hits={}, scan_scan_loads={}, promotions={}, same_scope_hits={}, protected_entries={}",
            exact.post_scan_hot_loads,
            scan_resistant.post_scan_hot_loads,
            scan_resistant.warm_hits,
            scan_resistant.warm_loads,
            scan_resistant.scan_hits,
            scan_resistant.scan_loads,
            scan_resistant.promotions,
            scan_resistant.same_scope_hits,
            scan_resistant.protected_entries
        )));
    }
    Ok(())
}

pub(super) fn decide_candidates(
    codecs: &[CodecObservation],
    filter: &FilterObservation,
    caches: &[CachePolicyObservation],
    integrated_caches: &[IntegratedCachePolicyObservation],
    values: &ValuePlacementObservation,
) -> Vec<CandidateDecision> {
    let mut decisions = Vec::new();
    for codec in codecs.iter().filter(|codec| codec.codec != "none") {
        let passes = codec.round_trip_exact
            && codec.savings_basis_points >= ADAPTIVE_CODEC_MINIMUM_SAVINGS_BPS
            && codec.adaptive_selected_pages > 0;
        let integrated = codec.codec == "lz4";
        decisions.push(CandidateDecision {
            candidate: format!("adaptive-{}-page-codec", codec.codec),
            decision: match (passes, integrated) {
                (true, true) => "integrated",
                (true, false) => "advance",
                (false, _) => "reject",
            }
            .into(),
            reason: if integrated {
                format!(
                    "segment_v6=true, exact_round_trip={}, selected_pages={}, savings_basis_points={}, required_savings_basis_points={ADAPTIVE_CODEC_MINIMUM_SAVINGS_BPS}; authenticated adaptive LZ4 is integrated in the normal rrflowKV flush, compaction, reopen, and read path",
                    codec.round_trip_exact, codec.adaptive_selected_pages, codec.savings_basis_points
                )
            } else {
                format!(
                    "exact_round_trip={}, selected_pages={}, savings_basis_points={}, required_savings_basis_points={ADAPTIVE_CODEC_MINIMUM_SAVINGS_BPS}; this candidate remains laboratory-only and requires a separate production-format plan",
                    codec.round_trip_exact, codec.adaptive_selected_pages, codec.savings_basis_points
                )
            },
        });
    }
    decisions.push(CandidateDecision {
        candidate: "persisted-row-group-bloom".into(),
        decision: if filter.passes_policy_threshold {
            "integrated"
        } else {
            "integrated-policy-failed"
        }
        .into(),
        reason: format!(
            "segment_v6=true, member_false_negatives={}, false_positive_ppm={}, limit_ppm={}, serialized_bytes={}",
            filter.member_false_negatives,
            filter.false_positive_parts_per_million,
            FILTER_FALSE_POSITIVE_LIMIT_PPM,
            filter.serialized_bytes
        ),
    });
    let baseline = caches.iter().find(|cache| cache.policy == "exact-byte-lru");
    let candidate = caches
        .iter()
        .find(|cache| cache.policy == "exact-byte-segmented-lru-candidate");
    let cache_advances = baseline
        .zip(candidate)
        .is_some_and(|(baseline, candidate)| {
            baseline.semantic_identity_exact
                && candidate.semantic_identity_exact
                && candidate.exact_capacity_respected
                && candidate.post_scan_hot_hits > baseline.post_scan_hot_hits
        });
    decisions.push(CandidateDecision {
        candidate: "exact-byte-segmented-lru".into(),
        decision: if cache_advances { "advance" } else { "reject" }.into(),
        reason: match (baseline, candidate) {
            (Some(baseline), Some(candidate)) => format!(
                "post_scan_hot_hits={} versus LRU {}; both must preserve identity and exact capacity",
                candidate.post_scan_hot_hits, baseline.post_scan_hot_hits
            ),
            _ => "required cache observations are absent".into(),
        },
    });
    let exact_integrated = integrated_caches
        .iter()
        .find(|cache| cache.policy == PageCachePolicy::ExactLru.kind());
    let selected_policy = PageCachePolicy::default();
    let scan_resistant_integrated = integrated_caches
        .iter()
        .find(|cache| cache.policy == selected_policy.kind());
    let cache_integrated =
        exact_integrated
            .zip(scan_resistant_integrated)
            .is_some_and(|(exact, scan_resistant)| {
                exact.semantic_identity_exact
                    && scan_resistant.semantic_identity_exact
                    && exact.exact_capacity_respected
                    && scan_resistant.exact_capacity_respected
                    && exact.semantic_digest == scan_resistant.semantic_digest
                    && exact.post_scan_hot_loads > 0
                    && scan_resistant.post_scan_hot_loads < exact.post_scan_hot_loads
                    && scan_resistant.promotions > 0
                    && scan_resistant.same_scope_hits > 0
            });
    decisions.push(CandidateDecision {
        candidate: "scan-resistant-page-cache".into(),
        decision: if cache_integrated {
            "integrated"
        } else {
            "integrated-policy-failed"
        }
        .into(),
        reason: match (exact_integrated, scan_resistant_integrated) {
            (Some(exact), Some(scan_resistant)) => format!(
                "real_rrflowkv_reader=true, semantic_identity_exact={}, exact_capacity_respected={}, post_scan_hot_loads={} versus exact-LRU {}, promotions={}, same_scope_hits={}, duplicate_loads={}",
                scan_resistant.semantic_identity_exact
                    && exact.semantic_identity_exact
                    && scan_resistant.semantic_digest == exact.semantic_digest,
                scan_resistant.exact_capacity_respected && exact.exact_capacity_respected,
                scan_resistant.post_scan_hot_loads,
                exact.post_scan_hot_loads,
                scan_resistant.promotions,
                scan_resistant.same_scope_hits,
                scan_resistant.duplicate_loads
            ),
            _ => "required integrated cache observations are absent".into(),
        },
    });
    decisions.push(CandidateDecision {
        candidate: "semantic-family-cache-partitions".into(),
        decision: if cache_integrated {
            "reject"
        } else {
            "unresolved"
        }
        .into(),
        reason: "the real mixed audit/edge/record/runtime/scalar/term/vector corpus is served by one family-neutral policy, so semantic quotas would duplicate key knowledge and can strand capacity".into(),
    });
    decisions.push(CandidateDecision {
        candidate: "value-separation".into(),
        decision: "reject".into(),
        reason: format!(
            "{} reports {} modeled bytes avoided, but production_retainable={} and authenticated pointer publication, recovery, GC, snapshot, corruption, and range-read proofs are absent",
            values.evidence_kind,
            values.modeled_compaction_bytes_avoided,
            values.production_retainable
        ),
    });
    decisions
}

pub(super) fn peak_rss_bytes() -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        let status = std::fs::read_to_string("/proc/self/status").ok()?;
        let kib = status
            .lines()
            .find_map(|line| line.strip_prefix("VmHWM:"))?
            .split_whitespace()
            .next()?
            .parse::<u64>()
            .ok()?;
        kib.checked_mul(1024)
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}
