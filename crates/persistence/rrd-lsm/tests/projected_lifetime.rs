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

#[test]
fn database_ceilings_cannot_be_raised_by_a_later_read_request() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("owner");
    let options = rrd_lsm::DatabaseOptions {
        active_read_views: 1,
        ..Default::default()
    };
    let database = Database::create_with_options(&root, options).unwrap();
    let held = view(&database);
    let mut raised = request(&database);
    raised.budget.max_active_views = usize::MAX;
    assert!(matches!(
        database.begin_projected_read(raised),
        Err(Error::ProjectedReadLimit {
            resource: ProjectedReadResource::ActiveViews,
            limit: 1,
            observed: 2
        })
    ));
    assert_eq!(database.read_view_ownership().unwrap(), (1, 0));
    drop(held);
    assert_eq!(database.read_view_ownership().unwrap(), (0, 0));
    drop(view(&database));
    drop(database);
    let reopened = Database::open_with_options(&root, options).unwrap();
    let held = view(&reopened);
    assert!(reopened.begin_projected_read(request(&reopened)).is_err());
    drop(held);
}

#[test]
fn read_view_and_independent_batch_charges_release_with_their_actual_owners() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("pins");
    let mut database = Database::create(&root).unwrap();
    database
        .write_owned(
            WriteBatch::new(vec![Mutation::Put {
                key: b"record".to_vec(),
                value: vec![7; 4096],
            }])
            .unwrap(),
            Durability::Authoritative,
        )
        .unwrap();
    let mut stream = view(&database);
    let pinned = stream.evidence().pinned_bytes;
    assert!(pinned > 1);
    let batch = stream.next_batch().unwrap().unwrap();
    assert_eq!(database.read_view_ownership().unwrap(), (1, pinned));
    drop(stream);
    assert_eq!(database.read_view_ownership().unwrap(), (0, 0));
    assert!(database.projected_output_memory().stats().reserved_bytes > 0);
    assert_eq!(batch.value(0).unwrap(), vec![7; 4096]);
    drop(batch);
    assert_eq!(database.read_view_ownership().unwrap(), (0, 0));
    assert_eq!(database.projected_output_memory().stats().reserved_bytes, 0);
    drop(database);
    let database = Database::open_with_options(
        &root,
        rrd_lsm::DatabaseOptions {
            pinned_read_bytes: 1,
            ..Default::default()
        },
    )
    .unwrap();
    let mut raised = request(&database);
    raised.budget.max_pinned_bytes = u64::MAX;
    assert!(matches!(
        database.begin_projected_read(raised),
        Err(Error::ProjectedReadLimit {
            resource: ProjectedReadResource::PinnedBytes,
            limit: 1,
            ..
        })
    ));
    assert_eq!(database.read_view_ownership().unwrap(), (0, 0));
}
