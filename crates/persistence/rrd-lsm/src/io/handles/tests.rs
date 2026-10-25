use super::*;
use std::io::Read;

fn fixture() -> (tempfile::TempDir, AnchoredPath) {
    let directory = tempfile::tempdir().unwrap();
    let root = AnchoredPath::open_root(directory.path()).unwrap();
    for index in 0..16_u8 {
        std::fs::write(directory.path().join(index.to_string()), [index; 16]).unwrap();
    }
    (directory, root)
}

#[test]
fn evicted_sources_reopen_with_a_hard_bound_and_release_on_drop() {
    let (_directory, root) = fixture();
    let pool = HandlePool::new(2);
    let sources: Vec<_> = (0..16)
        .map(|index| pool.open(&root.join(index.to_string())).unwrap())
        .collect();
    for (index, source) in sources.iter().enumerate() {
        let lease = source.lease().unwrap();
        let mut bytes = Vec::new();
        (&*lease).read_to_end(&mut bytes).unwrap();
        assert_eq!(bytes, vec![index as u8; 16]);
        assert!(pool.stats().open <= 2);
    }
    assert_eq!(pool.stats().peak, 2);
    assert!(pool.stats().evictions >= 14);
    assert_eq!(pool.stats().reopens, 16);
    drop(sources);
    assert_eq!(pool.stats().open, 0);
}

#[test]
fn leased_files_cannot_be_evicted_and_released_capacity_is_reusable() {
    let (_directory, root) = fixture();
    let pool = HandlePool::new(1);
    let first = pool.open(&root.join("0")).unwrap();
    let lease = first.lease().unwrap();
    assert!(matches!(
        pool.open(&root.join("1")),
        Err(Error::SegmentHandleLimit {
            limit: 1,
            observed: 2
        })
    ));
    assert_eq!(pool.stats().denials, 1);
    drop(lease);
    let second = pool.open(&root.join("1")).unwrap();
    assert!(second.lease().is_ok());
    assert!(first.lease().is_ok());
    assert_eq!(pool.stats().peak, 1);
}

#[test]
fn substituted_file_is_rejected_after_eviction_without_poisoning_capacity() {
    let (directory, root) = fixture();
    let pool = HandlePool::new(1);
    let first = pool.open(&root.join("0")).unwrap();
    let second = pool.open(&root.join("1")).unwrap();
    std::fs::rename(
        directory.path().join("0"),
        directory.path().join("original"),
    )
    .unwrap();
    std::fs::write(directory.path().join("0"), [0; 16]).unwrap();
    assert!(matches!(first.lease(), Err(Error::InvalidSegment(_))));
    assert_eq!(pool.stats().open, 0);
    assert!(second.lease().is_ok());
    std::fs::remove_file(directory.path().join("0")).unwrap();
    std::fs::rename(
        directory.path().join("original"),
        directory.path().join("0"),
    )
    .unwrap();
    assert!(first.lease().is_ok());
    assert_eq!(pool.stats().peak, 1);
}

#[test]
fn concurrent_native_leases_enforce_admission_before_open() {
    let (_directory, root) = fixture();
    let pool = HandlePool::new(4);
    let ready = std::sync::Barrier::new(5);
    let release = std::sync::Barrier::new(5);
    std::thread::scope(|scope| {
        for index in 0..4 {
            let pool = &pool;
            let root = &root;
            let ready = &ready;
            let release = &release;
            scope.spawn(move || {
                let source = pool.open(&root.join(index.to_string())).unwrap();
                let lease = source.lease().unwrap();
                ready.wait();
                release.wait();
                assert_eq!(lease.metadata().unwrap().len(), 16);
            });
        }
        ready.wait();
        assert!(matches!(
            pool.open(&root.join("5")),
            Err(Error::SegmentHandleLimit { .. })
        ));
        assert_eq!(pool.stats().open, 4);
        release.wait();
    });
    assert_eq!(pool.stats().open, 0);
    assert_eq!(pool.stats().peak, 4);
    assert!(pool.open(&root.join("5")).is_ok());
}
