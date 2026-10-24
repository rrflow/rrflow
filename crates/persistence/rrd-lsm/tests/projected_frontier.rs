use rrd_lsm::{
    CompactionPolicy, Database, DatabaseOptions, Durability, Mutation, ProjectedReadBudget,
    ProjectedReadProjection, ProjectedReadRange, ProjectedReadRequest, Snapshot, WriteBatch,
};

fn put(key: &str, value: &str) -> Mutation {
    Mutation::Put {
        key: key.as_bytes().to_vec(),
        value: value.as_bytes().to_vec(),
    }
}

fn write(database: &mut Database, mutations: Vec<Mutation>) {
    database
        .write(
            &WriteBatch::new(mutations).unwrap(),
            Durability::Authoritative,
        )
        .unwrap();
}

fn request(snapshot: Snapshot, ranges: Vec<ProjectedReadRange>) -> ProjectedReadRequest {
    ProjectedReadRequest {
        ranges,
        projection: ProjectedReadProjection::KeyValue,
        snapshot,
        budget: ProjectedReadBudget {
            max_batch_rows: 1,
            ..ProjectedReadBudget::default()
        },
    }
}

#[test]
fn one_row_page_does_not_read_later_disjoint_segments() {
    let root = tempfile::tempdir().unwrap();
    let mut database = Database::create_with_options(
        root.path(),
        DatabaseOptions {
            compaction: CompactionPolicy {
                l0_compaction_trigger: 16,
                ..CompactionPolicy::default()
            },
            ..DatabaseOptions::default()
        },
    )
    .unwrap();
    for index in 0..12 {
        write(
            &mut database,
            vec![put(
                &format!("key:{index:02}"),
                &format!("value:{index:02}"),
            )],
        );
        database.flush_memtable(index + 1).unwrap();
    }
    let snapshot = database.snapshot();
    let mut stream = database
        .begin_projected_read(request(snapshot, vec![ProjectedReadRange::all()]))
        .unwrap();
    assert_eq!(stream.evidence().selected_runs, 12);
    let first = stream.next_batch().unwrap().unwrap();
    assert_eq!(first.key(0), Some(b"key:00".as_slice()));
    assert_eq!(first.value(0), Some(b"value:00".as_slice()));
    assert_eq!(
        stream.evidence().selected_row_groups,
        1,
        "later disjoint runs cannot affect this page"
    );
    stream.cancel().unwrap();
    drop(database);
    assert_eq!(first.value(0), Some(b"value:00".as_slice()));
    let database = Database::open(root.path()).unwrap();
    let mut all = database
        .begin_projected_read(request(snapshot, vec![ProjectedReadRange::all()]))
        .unwrap();
    for index in 0..12 {
        let batch = all.next_batch().unwrap().unwrap();
        assert_eq!(batch.key(0), Some(format!("key:{index:02}").as_bytes()));
        assert_eq!(batch.value(0), Some(format!("value:{index:02}").as_bytes()));
        assert_eq!(all.evidence().selected_row_groups, index + 1);
    }
    assert!(all.next_batch().unwrap().is_none());
}

#[test]
fn deferred_runs_preserve_overlap_tombstones_sparse_ranges_and_old_snapshots() {
    let root = tempfile::tempdir().unwrap();
    let mut database = Database::create(root.path()).unwrap();
    write(
        &mut database,
        vec![put("a", "old"), put("b", "before"), put("z", "tail")],
    );
    let old = database.snapshot();
    database.flush_memtable(1).unwrap();
    write(
        &mut database,
        vec![
            put("a", "new"),
            Mutation::Delete { key: b"b".to_vec() },
            put("c", "middle"),
        ],
    );
    let middle = database.snapshot();
    database.flush_memtable(2).unwrap();
    write(
        &mut database,
        vec![put("a", "current"), put("d", "mutable")],
    );
    let latest = database.snapshot();
    for (snapshot, expected) in [
        (old, vec![("a", "old"), ("b", "before"), ("z", "tail")]),
        (middle, vec![("a", "new"), ("c", "middle"), ("z", "tail")]),
        (
            latest,
            vec![
                ("a", "current"),
                ("c", "middle"),
                ("d", "mutable"),
                ("z", "tail"),
            ],
        ),
    ] {
        let ranges = vec![
            ProjectedReadRange::new(b"a".to_vec(), Some(b"e".to_vec())).unwrap(),
            ProjectedReadRange::new(b"z".to_vec(), Some(b"zz".to_vec())).unwrap(),
        ];
        let mut stream = database
            .begin_projected_read(request(snapshot, ranges))
            .unwrap();
        for (key, value) in expected {
            let batch = stream.next_batch().unwrap().unwrap();
            assert_eq!(batch.key(0), Some(key.as_bytes()));
            assert_eq!(batch.value(0), Some(value.as_bytes()));
        }
        assert!(stream.next_batch().unwrap().is_none());
    }
}
