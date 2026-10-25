use super::*;

fn descriptor(stored: &[u8], logical_bytes: usize, compressed: bool) -> PageDescriptor {
    PageDescriptor {
        kind: PageKind::ValueData,
        compression: if compressed {
            format::PageCompression::Lz4Block
        } else {
            format::PageCompression::None
        },
        row_start: 0,
        row_count: 1,
        null_count: 0,
        offset: 0,
        physical_bytes: stored.len(),
        logical_bytes,
        statistic_min: 0,
        statistic_max: 0,
        digest: ring::digest::digest(&ring::digest::SHA256, stored)
            .as_ref()
            .try_into()
            .unwrap(),
    }
}

#[test]
fn source_pages_stay_charged_after_cache_eviction_until_last_slice_drops() {
    let cache = new_page_cache_with_limits(64, PageCachePolicy::ExactLru, 64).unwrap();
    let pool = source_page_memory(&cache);
    let bytes = Arc::new(vec![17; 64]);
    let desc = descriptor(&bytes, 64, false);
    let page = Arc::new(read_page(&PageSource::Bytes(bytes.clone()), &desc, &cache).unwrap());
    let slice = page.buffer.slice_with_length(3, 1);
    cache
        .lock()
        .unwrap()
        .complete_load((1, 0), page.clone(), 64, PageCacheAccess::OrdinaryOperation)
        .unwrap();
    drop(page);
    assert!(matches!(
        read_page(&PageSource::Bytes(bytes.clone()), &desc, &cache),
        Err(Error::SourcePageLimit {
            limit: 64,
            observed: 128
        })
    ));
    assert_eq!(page_cache_stats(&cache).resident_bytes, 0);
    assert_eq!(pool.stats().reserved_bytes, 64);
    assert_eq!(slice.as_slice(), &[17]);
    drop(cache); // A retained immutable slice owns its pool after the database/cache closes.
    assert_eq!(pool.stats().reserved_bytes, 64);
    assert!(slice.into_mutable().is_err());
    assert_eq!(pool.stats().reserved_bytes, 0);
}

#[test]
fn source_page_concurrent_misses_cannot_oversubscribe_and_failed_checks_release() {
    let cache = new_page_cache_with_limits(0, PageCachePolicy::ExactLru, 64).unwrap();
    let pool = source_page_memory(&cache);
    let bytes = Arc::new(vec![9; 64]);
    let desc = descriptor(&bytes, 64, false);
    let barrier = std::sync::Barrier::new(8);
    let admitted = std::sync::atomic::AtomicUsize::new(0);
    std::thread::scope(|scope| {
        for _ in 0..8 {
            let (cache, bytes, desc, barrier, admitted) =
                (&cache, &bytes, &desc, &barrier, &admitted);
            scope.spawn(move || {
                barrier.wait();
                let page = read_page(&PageSource::Bytes(bytes.clone()), desc, cache);
                if page.is_ok() {
                    admitted.fetch_add(1, Ordering::SeqCst);
                } else {
                    assert!(matches!(page, Err(Error::SourcePageLimit { .. })));
                }
                barrier.wait(); // All attempts finish while the successful owner is retained.
                drop(page);
            });
        }
    });
    assert_eq!(admitted.load(Ordering::SeqCst), 1);
    assert_eq!(pool.stats().peak_reserved_bytes, 64);
    assert_eq!(pool.stats().reserved_bytes, 0);
    let mut corrupt = desc.clone();
    corrupt.digest[0] ^= 1;
    assert!(read_page(&PageSource::Bytes(bytes.clone()), &corrupt, &cache).is_err());
    assert_eq!(pool.stats().reserved_bytes, 0);
    assert_eq!(
        read_page(&PageSource::Bytes(bytes), &desc, &cache)
            .unwrap()
            .buffer
            .len(),
        64
    );
}

#[test]
fn source_page_file_decode_reserves_overlapping_input_and_output() {
    let logical = vec![37; 4096];
    let stored = lz4_flex::block::compress(&logical);
    let fixture = tempfile::tempdir().unwrap();
    let path = fixture.path().join("page");
    std::fs::write(&path, &stored).unwrap();
    let io = IoContext::new(SegmentIoPolicy::default()).unwrap();
    let source = PageSource::File {
        file: Arc::new(
            io.open_segment(&AnchoredPath::from_path(&path).unwrap())
                .unwrap(),
        ),
        selected: SelectedIo::Bounded,
        io,
    };
    let desc = descriptor(&stored, logical.len(), true);
    let cache = new_page_cache_with_limits(0, PageCachePolicy::ExactLru, logical.len()).unwrap();
    let pool = source_page_memory(&cache);
    assert!(matches!(
        read_page(&source, &desc, &cache),
        Err(Error::SourcePageLimit { .. })
    ));
    assert_eq!(pool.stats().reserved_bytes, 0);
    let cache =
        new_page_cache_with_limits(0, PageCachePolicy::ExactLru, logical.len() + stored.len())
            .unwrap();
    let pool = source_page_memory(&cache);
    let page = read_page(&source, &desc, &cache).unwrap();
    assert_eq!(page.buffer.as_slice(), logical);
    assert_eq!(
        pool.stats().peak_reserved_bytes,
        logical.len() + stored.len()
    );
    assert_eq!(pool.stats().reserved_bytes, logical.len());
    drop(page);
    assert_eq!(pool.stats().reserved_bytes, 0);
}

#[test]
fn source_page_mapped_raw_bytes_borrow_without_private_allocation() {
    let mut file = tempfile::tempfile().unwrap();
    use std::io::Write;
    let bytes = vec![23; 4096];
    file.write_all(&bytes).unwrap();
    let mapping = unsafe { MmapOptions::new().map(&file).unwrap() };
    let source = PageSource::Mapped {
        bytes: Arc::new(mapping),
        io: IoContext::new(SegmentIoPolicy::default()).unwrap(),
    };
    let cache = new_page_cache_with_limits(0, PageCachePolicy::ExactLru, 1).unwrap();
    let page = read_page(&source, &descriptor(&bytes, bytes.len(), false), &cache).unwrap();
    assert_eq!(page.buffer.as_slice(), bytes);
    assert_eq!(source_page_memory(&cache).stats().reserved_bytes, 0);
    assert!(page.borrowed);
}

#[test]
fn source_page_pressure_evicts_cache_owners_and_failed_decode_releases_allocation() {
    let cache = new_page_cache_with_limits(64, PageCachePolicy::ExactLru, 64).unwrap();
    let pool = source_page_memory(&cache);
    let bytes = Arc::new(vec![3; 64]);
    let desc = descriptor(&bytes, 64, false);
    let page = Arc::new(read_page(&PageSource::Bytes(bytes.clone()), &desc, &cache).unwrap());
    cache
        .lock()
        .unwrap()
        .complete_load((1, 0), page, 64, PageCacheAccess::OrdinaryOperation)
        .unwrap();
    assert_eq!(pool.stats().reserved_bytes, 64);
    let next = read_page(&PageSource::Bytes(bytes), &desc, &cache).unwrap();
    assert_eq!(pool.stats().reserved_bytes, 64);
    assert_eq!(pool.stats().denied_allocations, 0);
    assert_eq!(page_cache_stats(&cache).evictions, 1);
    drop(next);
    let bad = Arc::new(vec![255]);
    assert!(read_page(
        &PageSource::Bytes(bad.clone()),
        &descriptor(&bad, 64, true),
        &cache
    )
    .is_err());
    assert_eq!(pool.stats().reserved_bytes, 0);
}
