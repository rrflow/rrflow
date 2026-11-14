use super::*;

fn small_config() -> PhysicalPolicyConfig {
    PhysicalPolicyConfig {
        seed: 0xca7c_4f10_0000_0001,
        records_per_family: 32,
        versions_per_key: 2,
        value_bytes: 128,
        point_misses: 512,
        cache_bytes: 32 * 1024,
    }
}

#[test]
fn physical_policy_lab_proves_adaptive_v6_and_keeps_other_candidates_non_production() {
    let first_root = tempfile::tempdir().unwrap();
    let second_root = tempfile::tempdir().unwrap();
    let first = run_physical_policy_trial(first_root.path(), small_config()).unwrap();
    let second = run_physical_policy_trial(second_root.path(), small_config()).unwrap();

    assert_eq!(first.corpus_digest, second.corpus_digest);
    assert_eq!(first.operation_count, second.operation_count);
    assert_eq!(first.key_count, second.key_count);
    assert_eq!(first.page_count, second.page_count);
    assert_eq!(first.segment_physical_bytes, second.segment_physical_bytes);
    assert_eq!(
        first.integrated_cache_policies,
        second.integrated_cache_policies
    );
    assert!(first.codecs.iter().all(|codec| codec.round_trip_exact));
    assert_eq!(first.persisted_row_group_filter.member_false_negatives, 0);
    assert!(first.persisted_row_group_filter.passes_policy_threshold);
    assert!(first
        .cache_candidates
        .iter()
        .all(|cache| cache.semantic_identity_exact && cache.exact_capacity_respected));
    let exact_cache = first
        .integrated_cache_policies
        .iter()
        .find(|cache| cache.policy == PageCachePolicy::ExactLru.kind())
        .unwrap();
    let scan_resistant_cache = first
        .integrated_cache_policies
        .iter()
        .find(|cache| cache.policy == PageCachePolicy::default().kind())
        .unwrap();
    assert_eq!(
        exact_cache.semantic_digest,
        scan_resistant_cache.semantic_digest
    );
    assert!(exact_cache.post_scan_hot_loads > 0);
    assert!(scan_resistant_cache.post_scan_hot_loads < exact_cache.post_scan_hot_loads);
    assert!(scan_resistant_cache.same_scope_hits > 0);
    assert!(scan_resistant_cache.promotions > 0);
    assert!(scan_resistant_cache.protected_entries > 0);
    assert!(!first.value_placement_candidate.production_retainable);
    assert!(first.reopened_point_miss.open_persisted_filter_count > 0);
    assert!(first.reopened_point_miss.open_persisted_filter_bytes > 0);
    assert_eq!(first.reopened_point_miss.open_semantic_page_operations, 0);
    assert_eq!(first.reopened_point_miss.open_semantic_page_bytes, 0);
    assert_eq!(first.reopened_point_miss.open_none_policy_segment_count, 0);
    assert!(
        first
            .reopened_point_miss
            .open_adaptive_lz4_policy_segment_count
            > 0
    );
    assert!(first.reopened_point_miss.open_raw_page_count > 0);
    assert!(first.reopened_point_miss.open_compressed_page_count > 0);
    assert!(
        first.reopened_point_miss.open_stored_page_bytes
            < first.reopened_point_miss.open_logical_page_bytes
    );
    assert!(first.reopened_point_miss.bytes_decompressed > 0);
    assert!(first.reopened_point_miss.filter_negatives > 0);
    assert!(first.reopened_point_miss.filter_checks > 0);
    assert!(first.reopened_point_miss.page_loads < first.reopened_point_miss.filter_checks);
    assert_eq!(first.segment_format_version, 6);
    let lz4 = first
        .decisions
        .iter()
        .find(|decision| decision.candidate == "adaptive-lz4-page-codec")
        .unwrap();
    assert_eq!(lz4.decision, "integrated");
    assert!(lz4.reason.contains("normal rrflowKV"));
    let zstd = first
        .decisions
        .iter()
        .find(|decision| decision.candidate == "adaptive-zstd-page-codec")
        .unwrap();
    assert_eq!(zstd.decision, "advance");
    assert!(zstd.reason.contains("laboratory-only"));

    let mut invalid_reopen = first.reopened_point_miss.clone();
    invalid_reopen.open_semantic_page_operations = 1;
    assert!(validate_persisted_filter_integration(
        first.row_group_count as usize,
        &first.persisted_row_group_filter,
        &invalid_reopen,
    )
    .is_err());
}

#[test]
fn physical_policy_lab_denies_unbounded_or_zero_configuration() {
    let mut config = small_config();
    config.records_per_family = 0;
    assert!(matches!(
        config.validate(),
        Err(Error::InvalidConfiguration(_))
    ));
    config = small_config();
    config.value_bytes = MAX_VALUE_BYTES + 1;
    assert!(matches!(
        config.validate(),
        Err(Error::InvalidConfiguration(_))
    ));
    config = small_config();
    config.cache_bytes = MAX_CACHE_BYTES + 1;
    assert!(matches!(
        config.validate(),
        Err(Error::InvalidConfiguration(_))
    ));
}

#[test]
fn canonical_filter_has_no_member_false_negatives() {
    let keys = (0..128)
        .map(|index| format!("record/{index:08x}").into_bytes())
        .collect::<Vec<_>>();
    let mut filter = RowGroupFilter::new(keys.len() as u32).unwrap();
    for key in &keys {
        filter.insert(key);
    }
    assert!(keys.iter().all(|key| filter.may_contain(key)));
    assert_eq!(
        RowGroupFilter::from_words(keys.len() as u32, filter.words().to_vec()).unwrap(),
        filter
    );
    assert!(RowGroupFilter::from_words(keys.len() as u32, Vec::new()).is_err());
}

#[test]
fn cache_candidates_preserve_identity_and_exact_capacity() {
    let pages = (0..64)
        .map(|id| PageSample {
            id,
            row_group: id / super::super::format::PAGES_PER_ROW_GROUP,
            row_count: 1,
            family: "record".into(),
            kind: "value-data",
            bytes: vec![id as u8; 1024],
        })
        .collect::<Vec<_>>();
    let observations = observe_caches(&pages, 16 * 1024).unwrap();
    assert_eq!(observations.len(), 2);
    assert!(observations
        .iter()
        .all(|observation| observation.semantic_identity_exact
            && observation.exact_capacity_respected
            && observation.peak_resident_bytes <= observation.capacity_bytes));
}

#[test]
fn encoded_segment_footer_is_not_mistaken_for_page_data() {
    let corpus = build_corpus(small_config()).unwrap();
    let mut table = Memtable::default();
    let operations = corpus.mutations.len() as u64;
    table
        .apply_owned_write_batch(WriteBatch::new(corpus.mutations).unwrap(), 1, operations)
        .unwrap();
    let encoded = encode(
        &table,
        SegmentRowGroupBudget::default(),
        SegmentCompressionPolicy::None,
    )
    .unwrap();
    let mut cursor = Cursor::new(&encoded.bytes);
    let metadata =
        parse_metadata(&mut cursor, encoded.bytes.len() as u64, encoded.checksum).unwrap();
    let pages = extract_pages(&encoded.bytes, &metadata.row_groups).unwrap();
    let page_bytes: usize = pages.iter().map(|page| page.bytes.len()).sum();
    assert!(page_bytes + FOOTER_BYTES < encoded.bytes.len());
}
