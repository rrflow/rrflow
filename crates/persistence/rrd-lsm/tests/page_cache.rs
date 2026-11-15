use rrd_lsm::{
    Database, DatabaseOptions, Durability, Error, Mutation, PageCachePolicy, PageCacheStats,
    ProjectedReadBudget, ProjectedReadProjection, ProjectedReadRange, ProjectedReadRequest,
    SegmentRowGroupBudget, Snapshot, WriteBatch,
};
use std::collections::BTreeMap;

const CACHE_BYTES: usize = 64 * 1024;
const FAMILY_PREFIXES: [&str; 8] = [
    "audit/",
    "edge/in/",
    "edge/out/",
    "record/",
    "runtime/",
    "scalar/",
    "term/",
    "vector/",
];
const RECORDS_PER_FAMILY: usize = 96;
const VALUE_BYTES: usize = 512;

#[derive(Debug)]
struct PolicyObservation {
    manifest: Vec<u8>,
    snapshot: Snapshot,
    rows: Vec<(Vec<u8>, Option<Vec<u8>>)>,
    hot_values: Vec<Vec<u8>>,
    stats: PageCacheStats,
    post_scan_hot_loads: u64,
}

#[test]
fn cache_policy_validation_fails_before_path_creation() {
    let root = tempfile::tempdir().unwrap();
    for (name, protected_capacity_basis_points) in [("zero", 0), ("full", 10_000)] {
        let path = root.path().join(name);
        let error = match Database::create_with_options(
            &path,
            options(PageCachePolicy::ScanResistantLru {
                protected_capacity_basis_points,
            }),
        ) {
            Ok(_) => panic!("invalid cache policy was accepted"),
            Err(error) => error,
        };
        assert!(matches!(error, Error::InvalidConfiguration(_)));
        assert!(
            !path.exists(),
            "invalid cache policy created {}",
            path.display()
        );
    }

    let exact = root.path().join("exact");
    let database =
        Database::create_with_options(&exact, options(PageCachePolicy::ExactLru)).unwrap();
    assert_eq!(
        database.page_cache_stats().policy,
        PageCachePolicy::ExactLru
    );
    drop(database);

    let no_residency = root.path().join("no-residency");
    let mut no_residency_options = options(PageCachePolicy::default());
    no_residency_options.page_cache_bytes = 0;
    let database = Database::create_with_options(&no_residency, no_residency_options).unwrap();
    assert_eq!(database.page_cache_stats().resident_bytes, 0);
    drop(database);

    let non_default = root.path().join("non-default");
    let policy = PageCachePolicy::ScanResistantLru {
        protected_capacity_basis_points: 6_250,
    };
    let database = Database::create_with_options(&non_default, options(policy)).unwrap();
    assert_eq!(database.page_cache_stats().policy, policy);
}

#[test]
fn cache_policy_is_process_local_and_durable_state_is_identical() {
    let root = tempfile::tempdir().unwrap();
    let database_root = root.path().join("engram");
    let expected = create_mixed_family_database(&database_root);
    let exact = observe(&database_root, PageCachePolicy::ExactLru, &expected);
    let scan_resistant = observe(
        &database_root,
        PageCachePolicy::ScanResistantLru {
            protected_capacity_basis_points: 8_000,
        },
        &expected,
    );

    assert_eq!(exact.manifest, scan_resistant.manifest);
    assert_eq!(exact.snapshot, scan_resistant.snapshot);
    assert_eq!(exact.rows, scan_resistant.rows);
    assert_eq!(exact.hot_values, scan_resistant.hot_values);
}

#[test]
fn scan_resistant_cache_preserves_reused_pages_across_a_mixed_family_scan() {
    let root = tempfile::tempdir().unwrap();
    let database_root = root.path().join("engram");
    let expected = create_mixed_family_database(&database_root);

    let exact = observe(&database_root, PageCachePolicy::ExactLru, &expected);
    let scan_resistant = observe(
        &database_root,
        PageCachePolicy::ScanResistantLru {
            protected_capacity_basis_points: 8_000,
        },
        &expected,
    );

    assert!(
        exact.post_scan_hot_loads > 0,
        "the corpus did not force exact-LRU post-scan reloads"
    );
    assert!(
        scan_resistant.post_scan_hot_loads < exact.post_scan_hot_loads,
        "scan resistance did not reduce post-scan loads: exact={}, scan-resistant={}",
        exact.post_scan_hot_loads,
        scan_resistant.post_scan_hot_loads
    );
    assert!(scan_resistant.stats.promotions > 0);
    assert!(
        scan_resistant.stats.same_scope_hits > 0,
        "the projected scan did not exercise same-scope promotion suppression"
    );
    assert!(scan_resistant.stats.protected_entries > 0);
    assert!(scan_resistant.stats.resident_bytes <= CACHE_BYTES);
    assert_eq!(
        scan_resistant.stats.resident_bytes,
        scan_resistant.stats.probationary_resident_bytes
            + scan_resistant.stats.protected_resident_bytes
    );
    assert_eq!(
        scan_resistant.stats.entries,
        scan_resistant.stats.probationary_entries + scan_resistant.stats.protected_entries
    );
    assert_eq!(exact.stats.probationary_resident_bytes, 0);
    assert_eq!(exact.stats.protected_resident_bytes, 0);
    assert_eq!(exact.stats.probationary_entries, 0);
    assert_eq!(exact.stats.protected_entries, 0);
}

fn options(page_cache_policy: PageCachePolicy) -> DatabaseOptions {
    DatabaseOptions {
        page_cache_bytes: CACHE_BYTES,
        page_cache_policy,
        segment_row_group_budget: SegmentRowGroupBudget {
            max_rows: 4,
            target_bytes: 4 * 1024,
        },
        ..DatabaseOptions::default()
    }
}

fn create_mixed_family_database(root: &std::path::Path) -> BTreeMap<Vec<u8>, Vec<u8>> {
    let mut database =
        Database::create_with_options(root, options(PageCachePolicy::ExactLru)).unwrap();
    let mut expected = BTreeMap::new();
    let mut mutations = Vec::new();
    for (family_index, family) in FAMILY_PREFIXES.iter().enumerate() {
        for record in 0..RECORDS_PER_FAMILY {
            let key = format!("{family}{record:04}").into_bytes();
            let value = value(family_index, record);
            expected.insert(key.clone(), value.clone());
            mutations.push(Mutation::Put { key, value });
        }
    }
    database
        .write_owned(
            WriteBatch::new(mutations).unwrap(),
            Durability::Authoritative,
        )
        .unwrap();
    assert!(database.flush_memtable(1).unwrap().is_some());
    drop(database);
    expected
}

fn observe(
    root: &std::path::Path,
    policy: PageCachePolicy,
    expected: &BTreeMap<Vec<u8>, Vec<u8>>,
) -> PolicyObservation {
    let database = Database::open_with_options(root, options(policy)).unwrap();
    let manifest = serde_json::to_vec(database.manifest()).unwrap();
    let snapshot = database.snapshot();
    let hot_keys = FAMILY_PREFIXES
        .iter()
        .map(|family| format!("{family}{:04}", 0).into_bytes())
        .collect::<Vec<_>>();

    let first_hot_values = read_hot_values(&database, snapshot, &hot_keys);
    let second_hot_values = read_hot_values(&database, snapshot, &hot_keys);
    assert_eq!(first_hot_values, second_hot_values);

    let mut stream = database
        .begin_projected_read(ProjectedReadRequest {
            ranges: vec![ProjectedReadRange::all()],
            projection: ProjectedReadProjection::KeyValue,
            snapshot,
            budget: ProjectedReadBudget {
                max_batch_rows: 17,
                ..ProjectedReadBudget::default()
            },
        })
        .unwrap();
    let mut rows = Vec::new();
    while let Some(batch) = stream.next_batch().unwrap() {
        for row in 0..batch.len() {
            rows.push((
                batch.key(row).unwrap().to_vec(),
                batch.value(row).map(<[u8]>::to_vec),
            ));
        }
    }
    let expected_rows = expected
        .iter()
        .map(|(key, value)| (key.clone(), Some(value.clone())))
        .collect::<Vec<_>>();
    assert_eq!(rows, expected_rows);

    let before_post_scan = database.page_cache_stats();
    let hot_values = read_hot_values(&database, snapshot, &hot_keys);
    let stats = database.page_cache_stats();
    assert_eq!(hot_values, first_hot_values);
    assert!(stats.resident_bytes <= stats.capacity_bytes);

    PolicyObservation {
        manifest,
        snapshot,
        rows,
        hot_values,
        stats,
        post_scan_hot_loads: stats.loads - before_post_scan.loads,
    }
}

fn read_hot_values(database: &Database, snapshot: Snapshot, keys: &[Vec<u8>]) -> Vec<Vec<u8>> {
    keys.iter()
        .map(|key| database.get(key, snapshot).unwrap().unwrap())
        .collect()
}

fn value(family: usize, record: usize) -> Vec<u8> {
    let mut state = ((family as u64 + 1) << 48) ^ record as u64 ^ 0x9e37_79b9_7f4a_7c15;
    (0..VALUE_BYTES)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state as u8
        })
        .collect()
}
