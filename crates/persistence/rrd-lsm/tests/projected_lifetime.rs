use rrd_lsm::{
    Database, Durability, Error, Mutation, ProjectedReadBudget, ProjectedReadProjection,
    ProjectedReadRange, ProjectedReadRequest, ProjectedReadResource, ProjectedReadStream,
    WriteBatch,
};

fn view(database: &Database) -> ProjectedReadStream {
    database.begin_projected_read(request(database)).unwrap()
}

fn request(database: &Database) -> ProjectedReadRequest {
    ProjectedReadRequest {
        ranges: vec![ProjectedReadRange::all()],
        projection: ProjectedReadProjection::KeyValue,
        snapshot: database.snapshot(),
        budget: ProjectedReadBudget {
            max_active_views: 2,
            ..ProjectedReadBudget::default()
        },
    }
}

fn empty_views(release: &str) {
    let directory = tempfile::tempdir().unwrap();
    let database = Database::create(&directory.path().join("empty")).unwrap();
    let mut held = view(&database);
    for _ in 0..32 {
        let mut transient = view(&database);
        assert!(matches!(
            database.begin_projected_read(request(&database)),
            Err(Error::ProjectedReadLimit {
                resource: ProjectedReadResource::ActiveViews,
                ..
            })
        ));
        match release {
            "complete" => assert!(transient.next_batch().unwrap().is_none()),
            "cancel" => transient.cancel().unwrap(),
            "drop" => {}
            _ => unreachable!(),
        }
        drop(transient);
        // Admission must be reusable while the other zero-byte view is live.
        let mut replacement = view(&database);
        replacement.cancel().unwrap();
    }
    assert!(held.next_batch().unwrap().is_none());
    drop(view(&database));
}

#[test]
fn overlapping_empty_views_complete_and_release_capacity() {
    empty_views("complete");
}

#[test]
fn overlapping_empty_views_cancel_and_release_capacity() {
    empty_views("cancel");
}

#[test]
fn overlapping_empty_views_drop_and_release_capacity() {
    empty_views("drop");
}

#[test]
fn nonempty_view_can_release_while_an_empty_generation_remains_live() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("mixed");
    let mut database = Database::create(&root).unwrap();
    let mut empty = view(&database);
    let manifest = database.manifest().digest.clone();
    database
        .write_owned(
            WriteBatch::new(vec![Mutation::Put {
                key: b"record".to_vec(),
                value: b"durable".to_vec(),
            }])
            .unwrap(),
            Durability::Authoritative,
        )
        .unwrap();
    assert_eq!(database.manifest().digest, manifest);
    let mut populated = view(&database);
    assert!(populated.evidence().pinned_bytes > 0);
    let batch = populated.next_batch().unwrap().unwrap();
    assert_eq!(batch.value(0), Some(b"durable".as_slice()));
    populated.cancel().unwrap();
    assert!(empty.next_batch().unwrap().is_none());
    drop(view(&database));
    drop(database);
    let reopened = Database::open(&root).unwrap();
    let mut restored = view(&reopened);
    let restored_batch = restored.next_batch().unwrap().unwrap();
    assert_eq!(restored_batch.key(0), Some(b"record".as_slice()));
    assert_eq!(restored_batch.value(0), Some(b"durable".as_slice()));
    assert!(restored.next_batch().unwrap().is_none());
    assert_eq!(batch.value(0), Some(b"durable".as_slice()));
}
