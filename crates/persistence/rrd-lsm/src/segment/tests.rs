use super::*;
use crate::{Durability, Mutation, WalWriter, WriteBatch};
use std::collections::BTreeMap;

fn cache_page(bytes: usize, marker: u8) -> Arc<LoadedPage> {
    Arc::new(LoadedPage {
        buffer: Buffer::from(vec![marker; bytes]),
        borrowed: false,
        allocated_bytes: bytes,
        copied_bytes: bytes,
        decompressed_bytes: 0,
        actual_io: None,
    })
}

#[test]
fn scan_resistant_cache_demotes_large_promotions_without_changing_the_page() {
    let shared = new_page_cache_with_policy(
        128,
        PageCachePolicy::ScanResistantLru {
            protected_capacity_basis_points: 5_000,
        },
    )
    .unwrap();
    let mut cache = shared.lock().unwrap();
    let page = cache_page(80, 1);
    let resident_bytes = page.resident_bytes();
    let inserted = cache
        .complete_load(
            (1, 1),
            Arc::clone(&page),
            resident_bytes,
            PageCacheAccess::OrdinaryOperation,
        )
        .unwrap();
    assert!(Arc::ptr_eq(&page, &inserted));

    let promoted = cache
        .lookup((1, 1), PageCacheAccess::OrdinaryOperation)
        .unwrap()
        .unwrap();
    assert!(Arc::ptr_eq(&page, &promoted));
    assert_eq!(cache.promotions, 1);
    assert_eq!(cache.demotions, 1);
    assert_eq!(
        cache.values.get(&(1, 1)).unwrap().region,
        CacheRegion::Probationary
    );
    assert_eq!(cache.protected_resident_bytes, 0);
    assert_eq!(cache.probationary_resident_bytes, resident_bytes);
    cache.validate_residency().unwrap();

    let oversize = cache_page(256, 2);
    let returned = cache
        .complete_load(
            (1, 2),
            Arc::clone(&oversize),
            oversize.resident_bytes(),
            PageCacheAccess::OrdinaryOperation,
        )
        .unwrap();
    assert!(Arc::ptr_eq(&oversize, &returned));
    assert!(!cache.values.contains_key(&(1, 2)));
    assert_eq!(cache.admission_rejections, 1);
    cache.validate_residency().unwrap();
}

#[test]
fn exact_lru_ignores_stale_heap_entries_and_renumbers_at_clock_wrap() {
    let shared = new_page_cache_with_policy(128, PageCachePolicy::ExactLru).unwrap();
    let mut cache = shared.lock().unwrap();
    for (ordinal, marker) in [(1, 1), (2, 2)] {
        let page = cache_page(64, marker);
        cache
            .complete_load(
                (1, ordinal),
                Arc::clone(&page),
                page.resident_bytes(),
                PageCacheAccess::OrdinaryOperation,
            )
            .unwrap();
    }
    assert!(cache
        .lookup((1, 1), PageCacheAccess::OrdinaryOperation)
        .unwrap()
        .is_some());
    cache.clock = u64::MAX;
    assert!(cache
        .lookup((1, 1), PageCacheAccess::OrdinaryOperation)
        .unwrap()
        .is_some());
    assert!(cache.clock > 0);

    let page = cache_page(64, 3);
    cache
        .complete_load(
            (1, 3),
            Arc::clone(&page),
            page.resident_bytes(),
            PageCacheAccess::OrdinaryOperation,
        )
        .unwrap();
    assert!(cache.values.contains_key(&(1, 1)));
    assert!(!cache.values.contains_key(&(1, 2)));
    assert!(cache.values.contains_key(&(1, 3)));
    assert_eq!(cache.resident_bytes, 128);
    cache.validate_residency().unwrap();
}

#[test]
fn projected_scope_requires_cross_operation_reuse_before_promotion() {
    let shared = new_page_cache_with_policy(
        128,
        PageCachePolicy::ScanResistantLru {
            protected_capacity_basis_points: 5_000,
        },
    )
    .unwrap();
    let mut cache = shared.lock().unwrap();
    let page = cache_page(64, 1);
    cache
        .complete_load(
            (1, 1),
            Arc::clone(&page),
            page.resident_bytes(),
            PageCacheAccess::ProjectedScope(7),
        )
        .unwrap();

    let same_scope = cache
        .lookup((1, 1), PageCacheAccess::ProjectedScope(7))
        .unwrap()
        .unwrap();
    assert!(Arc::ptr_eq(&page, &same_scope));
    assert_eq!(cache.promotions, 0);
    assert_eq!(cache.same_scope_hits, 1);
    assert_eq!(
        cache.values.get(&(1, 1)).unwrap().region,
        CacheRegion::Probationary
    );

    let later_scope = cache
        .lookup((1, 1), PageCacheAccess::ProjectedScope(8))
        .unwrap()
        .unwrap();
    assert!(Arc::ptr_eq(&page, &later_scope));
    assert_eq!(cache.promotions, 1);
    assert_eq!(cache.same_scope_hits, 1);
    assert_eq!(
        cache.values.get(&(1, 1)).unwrap().region,
        CacheRegion::Protected
    );
    cache.validate_residency().unwrap();
}

fn two_key_sample_memtable() -> Memtable {
    let directory = tempfile::tempdir().unwrap();
    let wal_path = directory.path().join("span.wal");
    let mut wal = WalWriter::create(&wal_path).unwrap();
    let operations = vec![
        Mutation::Put {
            key: b"key:100".to_vec(),
            value: b"alpha".to_vec(),
        },
        Mutation::Put {
            key: b"key:200".to_vec(),
            value: b"beta".to_vec(),
        },
    ];
    wal.append_write_batch(
        &WriteBatch::new(operations).unwrap(),
        Durability::Authoritative,
    )
    .unwrap();
    drop(wal);

    let recovery = crate::recover(&wal_path).unwrap();
    Memtable::recover(&recovery.batches).unwrap()
}

fn mixed_family_memtable() -> Memtable {
    let mut versions = BTreeMap::<Vec<u8>, Vec<VersionedValue>>::new();
    versions.insert(
        b"ctl:alpha".to_vec(),
        vec![
            VersionedValue {
                sequence: 1,
                value: Some(b"ctl-v1".as_ref().into()),
            },
            VersionedValue {
                sequence: 7,
                value: None,
            },
        ],
    );
    versions.insert(
        b"doc:alpha".to_vec(),
        vec![VersionedValue {
            sequence: 2,
            value: Some(b"doc-v1".as_ref().into()),
        }],
    );
    versions.insert(
        b"doc:beta".to_vec(),
        vec![
            VersionedValue {
                sequence: 3,
                value: None,
            },
            VersionedValue {
                sequence: 5,
                value: Some(b"doc-v2".as_ref().into()),
            },
        ],
    );
    versions.insert(
        b"edge:alpha".to_vec(),
        vec![VersionedValue {
            sequence: 4,
            value: Some(b"edge-v1".as_ref().into()),
        }],
    );
    versions.insert(
        b"vec:alpha".to_vec(),
        vec![VersionedValue {
            sequence: 6,
            value: Some(b"vec-v1".as_ref().into()),
        }],
    );
    Memtable::from_versions(versions, 7).unwrap()
}

fn visible_ranges_oracle(
    table: &Memtable,
    ranges: &[(Vec<u8>, Vec<u8>)],
    read_sequence: u64,
) -> Vec<(Vec<u8>, VersionedValue)> {
    table
        .visible_versions(read_sequence)
        .into_iter()
        .filter_map(|(key, version)| {
            ranges
                .iter()
                .find(|(start, end)| {
                    key.as_slice() >= start.as_slice() && key.as_slice() < end.as_slice()
                })
                .map(|_| (key, version))
        })
        .collect()
}

#[test]
fn visible_scan_with_no_matching_keys_loads_only_spine_pages() {
    let directory = tempfile::tempdir().unwrap();
    let segments = directory.path().join("segments");
    let cache = new_page_cache(DEFAULT_PAGE_CACHE_BYTES);
    let io = IoContext::new(SegmentIoPolicy::default()).unwrap();
    let table = two_key_sample_memtable();
    let (segment, _path) = Segment::write_from_memtable_with_cache(
        &AnchoredPath::from_path(&segments).unwrap(),
        &table,
        SegmentRowGroupBudget::default(),
        DEFAULT_SEGMENT_COMPRESSION_POLICY,
        Arc::clone(&cache),
        io,
    )
    .unwrap();

    assert_eq!(segment.row_group_count(), 1);

    let before = super::page_cache_stats(&cache);
    let visible = segment
        .visible_from(
            b"key:150",
            Some(b"key:160"),
            segment.descriptor.maximum_sequence,
        )
        .unwrap();
    let after = super::page_cache_stats(&cache);
    let row_groups = u64::try_from(segment.row_group_count()).unwrap_or_default();

    assert!(visible.is_empty());
    assert_eq!(before.loads + 3 * row_groups, after.loads);
}

#[test]
fn visible_scan_with_matching_keys_loads_value_columns() {
    let directory = tempfile::tempdir().unwrap();
    let segments = directory.path().join("segments");
    let cache = new_page_cache(DEFAULT_PAGE_CACHE_BYTES);
    let io = IoContext::new(SegmentIoPolicy::default()).unwrap();
    let table = two_key_sample_memtable();
    let (segment, _path) = Segment::write_from_memtable_with_cache(
        &AnchoredPath::from_path(&segments).unwrap(),
        &table,
        SegmentRowGroupBudget::default(),
        DEFAULT_SEGMENT_COMPRESSION_POLICY,
        Arc::clone(&cache),
        io,
    )
    .unwrap();

    let before = super::page_cache_stats(&cache);
    let visible = segment
        .visible_from(
            b"key:150",
            Some(b"key:250"),
            segment.descriptor.maximum_sequence,
        )
        .unwrap();
    let after = super::page_cache_stats(&cache);
    let row_groups = u64::try_from(segment.row_group_count()).unwrap_or_default();

    assert_eq!(visible.len(), 1);
    assert_eq!(before.loads + 6 * row_groups, after.loads);
}

#[test]
fn visible_scan_matches_memtable_visibility_across_ranges_and_sequences() {
    let directory = tempfile::tempdir().unwrap();
    let segments = directory.path().join("segments");
    let cache = new_page_cache(DEFAULT_PAGE_CACHE_BYTES);
    let io = IoContext::new(SegmentIoPolicy::default()).unwrap();
    let table = mixed_family_memtable();
    let (segment, _path) = Segment::write_from_memtable_with_cache(
        &AnchoredPath::from_path(&segments).unwrap(),
        &table,
        SegmentRowGroupBudget::default(),
        DEFAULT_SEGMENT_COMPRESSION_POLICY,
        Arc::clone(&cache),
        io,
    )
    .unwrap();

    let queries = [
        (b"".as_ref(), None),
        (b"ctl:".as_ref(), Some(b"doc:".as_ref())),
        (b"doc:".as_ref(), Some(b"edge:".as_ref())),
        (b"edge:".as_ref(), Some(b"vec:".as_ref())),
        (b"vec:".as_ref(), Some(b"zzz".as_ref())),
    ];

    for read_sequence in 0..=table.maximum_sequence() {
        for (start, end) in queries {
            let expected = table.visible_from(start, end, read_sequence);
            let actual = segment.visible_from(start, end, read_sequence).unwrap();
            assert_eq!(actual, expected, "sequence {read_sequence}");
        }
    }
}

#[test]
fn visible_ranges_match_memtable_oracle_for_mixed_families() {
    let directory = tempfile::tempdir().unwrap();
    let segments = directory.path().join("segments");
    let cache = new_page_cache(DEFAULT_PAGE_CACHE_BYTES);
    let io = IoContext::new(SegmentIoPolicy::default()).unwrap();
    let table = mixed_family_memtable();
    let (segment, _path) = Segment::write_from_memtable_with_cache(
        &AnchoredPath::from_path(&segments).unwrap(),
        &table,
        SegmentRowGroupBudget::default(),
        DEFAULT_SEGMENT_COMPRESSION_POLICY,
        Arc::clone(&cache),
        io,
    )
    .unwrap();

    let ranges = vec![
        (b"ctl:".to_vec(), b"doc:".to_vec()),
        (b"edge:".to_vec(), b"vec:".to_vec()),
        (b"vec:".to_vec(), b"zzz".to_vec()),
    ];

    for read_sequence in 0..=table.maximum_sequence() {
        let expected = visible_ranges_oracle(&table, &ranges, read_sequence);
        let actual = segment.visible_ranges(&ranges, read_sequence).unwrap();
        assert_eq!(actual, expected, "sequence {read_sequence}");
    }
}
