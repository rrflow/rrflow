use rrd_core::{
    Claim, DataTransaction, Predicate, Producer, RuntimeCommit, RuntimeLogicalModel,
    RuntimeMutation, RuntimeProperties, RuntimeRecord, RuntimeRecordSchema, RuntimeRef,
    RuntimeSchemaRegistry, RuntimeTableSchema, RuntimeType, ScopeId, Subject,
};
use rrd_store::{
    control_restore_progress, create_application_backup, create_logical_backup,
    load_backup_catalogue, prune_backup_catalogue, restore_catalogued_backup,
    verify_backup_catalogue, verify_restored_backup_objects, BackupCoverage, BackupPrunePlan,
    ControlTransition, DataRuntime, LocalObjectStore, RrflowKvStore, StorageEngine,
    BACKUP_CATALOGUE_VERSION,
};
use std::fs::OpenOptions;
use std::io::{Seek, SeekFrom, Write};

/// The control families a test estate declares as its estate control state.
fn control_families() -> Vec<String> {
    vec![
        "server/state/install".into(),
        "server/state/test/attunement".into(),
        "server/state/test/audit".into(),
        "server/state/test/audit-head".into(),
        "server/state/test/security".into(),
    ]
}

fn transition(
    key: &str,
    expected: Option<&[u8]>,
    replacement: Option<&[u8]>,
    at: u64,
    action: &str,
) -> ControlTransition {
    ControlTransition {
        key: key.into(),
        expected: expected.map(<[u8]>::to_vec),
        replacement: replacement.map(<[u8]>::to_vec),
        at,
        actor: "backup-test".into(),
        action: action.into(),
        request_id: format!("request-{at}"),
        operation_id: format!("operation-{at}"),
    }
}

#[test]
fn application_backup_pages_fit_configured_bytes_through_restore_and_reopen() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source");
    let options = rrd_store::DatabaseOptions {
        projected_output_bytes: 8 * 1024,
        ..Default::default()
    };
    let engine = RrflowKvStore::open_with_options(&source, options).unwrap();
    let objects = LocalObjectStore::open(root.path().join("objects")).unwrap();
    let value = vec![b'x'; 128];
    for sequence in 1..=96 {
        engine
            .control()
            .commit(&transition(
                &format!("server/state/test/audit/{sequence}"),
                None,
                Some(&value),
                sequence,
                "audit.append",
            ))
            .unwrap();
    }
    assert!(matches!(
        engine.control().journal_since(0, 1024),
        Err(rrd_store::Error::ProjectedReadLimit { .. })
    ));
    for (cut, engine) in [Some(engine), None].into_iter().enumerate() {
        let engine = engine.unwrap_or_else(|| {
            RrflowKvStore::open_existing_with_options(&source, options).unwrap()
        });
        let catalogue = root.path().join(format!("catalogue-{cut}"));
        let entry = create_application_backup(
            &engine,
            &objects,
            &catalogue,
            "bounded",
            200,
            &control_families(),
        )
        .unwrap();
        assert!(entry.application_complete);
        assert_eq!(entry.control_archive.as_ref().unwrap().entry_count, 96);
        assert_eq!(
            verify_backup_catalogue(&catalogue).unwrap().backups,
            vec![entry.clone()]
        );
        let restored = root.path().join(format!("restored-{cut}"));
        rrd_store::restore_catalogued_backup_with_options(
            &catalogue,
            &entry.backup_id,
            &restored,
            300,
            options,
        )
        .unwrap();
        rrd_store::verify_restored_backup_objects_with_options(
            &catalogue,
            &entry.backup_id,
            &restored,
            options,
        )
        .unwrap();
        let restored = RrflowKvStore::open_existing_with_options(&restored, options).unwrap();
        for sequence in 1..=96 {
            assert_eq!(
                restored
                    .control()
                    .get(&format!("server/state/test/audit/{sequence}"))
                    .unwrap(),
                Some(value.clone())
            );
        }
        let progress = control_restore_progress(&restored).unwrap().unwrap();
        assert!(progress.complete);
        assert_eq!(progress.applied, 96);
        assert_eq!(
            engine
                .physical_store_evidence()
                .unwrap()
                .native_transactions,
            Some(0)
        );
    }
}

#[test]
fn indivisible_backup_entry_is_denied_without_publication_and_can_resume() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source");
    let options = rrd_store::DatabaseOptions {
        projected_output_bytes: 8 * 1024,
        ..Default::default()
    };
    let engine = RrflowKvStore::open_with_options(&source, options).unwrap();
    let objects = LocalObjectStore::open(root.path().join("objects")).unwrap();
    let value = vec![b'x'; 8 * 1024];
    engine
        .control()
        .commit(&transition(
            "server/state/install/record",
            None,
            Some(&value),
            1,
            "install.record",
        ))
        .unwrap();
    let catalogue = root.path().join("catalogue");
    let error = create_application_backup(
        &engine,
        &objects,
        &catalogue,
        "oversized",
        200,
        &control_families(),
    )
    .unwrap_err();
    assert!(
        matches!(error, rrd_store::Error::ProjectedReadLimit {
            resource: rrd_store::ProjectedReadResource::OutputBufferBytes
                | rrd_store::ProjectedReadResource::BatchAllocatedBytes,
        limit: 8192, observed,
    } if observed > 8192),
        "{error}"
    );
    assert!(load_backup_catalogue(&catalogue)
        .unwrap()
        .backups
        .is_empty());
    assert_eq!(engine.control().sequence().unwrap(), 1);
    assert_eq!(
        engine
            .physical_store_evidence()
            .unwrap()
            .native_transactions,
        Some(0)
    );
    drop(engine);
    // The operator explicitly changes configuration; backup never raises it.
    let engine = RrflowKvStore::open_existing(&source).unwrap();
    let entry = create_application_backup(
        &engine,
        &objects,
        &catalogue,
        "oversized",
        200,
        &control_families(),
    )
    .unwrap();
    assert_eq!(entry.control_archive.as_ref().unwrap().entry_count, 1);
    let target = root.path().join("restored");
    assert!(rrd_store::restore_catalogued_backup_with_options(
        &catalogue,
        &entry.backup_id,
        &target,
        300,
        options
    )
    .is_err());
    assert!(
        !target.exists(),
        "verification denial must not publish the target"
    );
    restore_catalogued_backup(&catalogue, &entry.backup_id, &target, 301).unwrap();
    let restored = RrflowKvStore::open_existing(&target).unwrap();
    assert_eq!(
        restored
            .control()
            .get("server/state/install/record")
            .unwrap(),
        Some(value)
    );
    let progress = control_restore_progress(&restored).unwrap().unwrap();
    assert!(progress.complete);
    assert_eq!(progress.applied, 1);
}

fn claim(name: &str, at: u64) -> Claim {
    Claim::new(
        Subject::new(format!("backup:{name}")).unwrap(),
        Predicate::new("status").unwrap(),
        "retained",
        at,
        at,
        Producer {
            actor: "agent:backup-test".into(),
            on_behalf_of: None,
            session: None,
        },
    )
}

#[test]
fn catalogues_multiple_cuts_and_restores_the_selected_backup() {
    let root = tempfile::tempdir().unwrap();
    let engine = RrflowKvStore::open(&root.path().join("source")).unwrap();
    let catalogue_root = root.path().join("catalogue");
    engine.claims().append_batch(&[claim("one", 10)]).unwrap();
    let first = create_logical_backup(&engine, &catalogue_root, "first", 100).unwrap();
    assert_eq!(first.claims, BackupCoverage::Included);
    assert_eq!(first.object_payloads, BackupCoverage::ReferencedOnly);
    assert!(!first.application_complete);

    engine.claims().append_batch(&[claim("two", 20)]).unwrap();
    let second = create_logical_backup(&engine, &catalogue_root, "second", 200).unwrap();
    let catalogue = verify_backup_catalogue(&catalogue_root).unwrap();
    assert_eq!(catalogue.revision, 2);
    assert_eq!(catalogue.backups, vec![first.clone(), second]);

    let target = root.path().join("restored-first");
    restore_catalogued_backup(&catalogue_root, &first.backup_id, &target, 300).unwrap();
    let restored = RrflowKvStore::open(&target).unwrap();
    assert_eq!(restored.claims().sequence().unwrap(), 1);
    assert_eq!(
        restored.claims().claims_in_range(0, 1).unwrap(),
        vec![claim("one", 10)]
    );
}

#[test]
fn configured_logical_backup_rejects_before_publication_and_resumes() {
    let root = tempfile::tempdir().unwrap();
    let engine = RrflowKvStore::open(&root.path().join("source")).unwrap();
    engine.claims().append_batch(&[claim("one", 10)]).unwrap();
    let catalogue_root = root.path().join("catalogue");
    let entry = create_logical_backup(&engine, &catalogue_root, "logical", 100).unwrap();
    let before = directory_bytes(&catalogue_root);
    let target = root.path().join("restored");
    let low = rrd_store::DatabaseOptions {
        mutable_limits: rrd_store::MutableLimits {
            accounted_bytes: 1,
            versions: 1,
        },
        ..Default::default()
    };
    assert!(matches!(
        rrd_store::restore_catalogued_backup_with_options(
            &catalogue_root,
            &entry.backup_id,
            &target,
            200,
            low
        )
        .unwrap_err(),
        rrd_store::Error::MutableLimit { .. }
    ));
    assert!(!target.exists());
    let admitted = rrd_store::DatabaseOptions {
        mutable_limits: rrd_store::MutableLimits {
            accounted_bytes: 128 * 1024,
            versions: 1024,
        },
        ..Default::default()
    };
    let restored = rrd_store::restore_catalogued_backup_with_options(
        &catalogue_root,
        &entry.backup_id,
        &target,
        201,
        admitted,
    )
    .unwrap();
    assert!(restored.resumed && restored.reopened);
    let engine = RrflowKvStore::open_existing_with_options(&target, admitted).unwrap();
    assert_eq!(
        engine.claims().claims_in_range(0, 2).unwrap(),
        vec![claim("one", 10)]
    );
    assert_eq!(directory_bytes(&catalogue_root), before);
}

#[test]
fn repeated_identical_backup_is_idempotent() {
    let root = tempfile::tempdir().unwrap();
    let engine = RrflowKvStore::open(&root.path().join("source")).unwrap();
    let catalogue_root = root.path().join("catalogue");
    engine.claims().append_batch(&[claim("one", 10)]).unwrap();
    let first = create_logical_backup(&engine, &catalogue_root, "daily", 100).unwrap();
    let retry = create_logical_backup(&engine, &catalogue_root, "daily", 100).unwrap();
    assert_eq!(retry, first);
    let catalogue = load_backup_catalogue(&catalogue_root).unwrap();
    assert_eq!(catalogue.revision, 1);
    assert_eq!(catalogue.backups.len(), 1);
}

#[test]
fn authenticated_prune_is_partition_bound_replayable_and_preserves_shared_artifacts() {
    let root = tempfile::tempdir().unwrap();
    let engine = RrflowKvStore::open(&root.path().join("source")).unwrap();
    let objects = LocalObjectStore::open(root.path().join("source-objects")).unwrap();
    let catalogue_root = root.path().join("catalogue");
    let first = create_application_backup(
        &engine,
        &objects,
        &catalogue_root,
        "first",
        100,
        &control_families(),
    )
    .unwrap();
    let second = create_application_backup(
        &engine,
        &objects,
        &catalogue_root,
        "second",
        200,
        &control_families(),
    )
    .unwrap();
    assert_eq!(first.archive_file, second.archive_file);
    assert_eq!(first.object_manifest_file, second.object_manifest_file);
    assert_eq!(
        first.catalogue_manifest_file,
        second.catalogue_manifest_file
    );
    let before = verify_backup_catalogue(&catalogue_root).unwrap();
    let plan = BackupPrunePlan {
        expected_catalogue_sha256: before.catalogue_sha256.clone(),
        retained_backup_ids: vec![second.backup_id.clone()],
        prune_candidate_backup_ids: vec![first.backup_id.clone()],
    };

    let pruned = prune_backup_catalogue(&catalogue_root, &plan).unwrap();
    assert!(!pruned.idempotent_replay);
    assert_eq!(pruned.pruned_backup_ids, vec![first.backup_id.clone()]);
    assert_eq!(pruned.catalogue.revision, 3);
    assert_eq!(pruned.catalogue.backups, vec![second.clone()]);
    assert!(catalogue_root.join(&second.archive_file).is_file());
    assert!(catalogue_root
        .join(second.object_manifest_file.as_ref().unwrap())
        .is_file());
    assert!(catalogue_root
        .join(second.catalogue_manifest_file.as_ref().unwrap())
        .is_file());

    let replay = prune_backup_catalogue(&catalogue_root, &plan).unwrap();
    assert!(replay.idempotent_replay);
    assert_eq!(
        replay.catalogue.catalogue_sha256,
        pruned.catalogue.catalogue_sha256
    );
    let stale = BackupPrunePlan {
        expected_catalogue_sha256: before.catalogue_sha256,
        retained_backup_ids: vec![second.backup_id],
        prune_candidate_backup_ids: vec!["f".repeat(64)],
    };
    assert!(prune_backup_catalogue(&catalogue_root, &stale)
        .unwrap_err()
        .to_string()
        .contains("stale"));
    assert!(create_application_backup(
        &engine,
        &objects,
        &catalogue_root,
        "first",
        100,
        &control_families(),
    )
    .unwrap_err()
    .to_string()
    .contains("cannot be resurrected"));
}

#[test]
fn prune_rejects_incomplete_or_corrupt_inventory_before_publication() {
    let root = tempfile::tempdir().unwrap();
    let engine = RrflowKvStore::open(&root.path().join("source")).unwrap();
    let catalogue_root = root.path().join("catalogue");
    engine.claims().append_batch(&[claim("one", 10)]).unwrap();
    let first = create_logical_backup(&engine, &catalogue_root, "first", 100).unwrap();
    engine.claims().append_batch(&[claim("two", 20)]).unwrap();
    let second = create_logical_backup(&engine, &catalogue_root, "second", 200).unwrap();
    let before = verify_backup_catalogue(&catalogue_root).unwrap();
    let incomplete = BackupPrunePlan {
        expected_catalogue_sha256: before.catalogue_sha256.clone(),
        retained_backup_ids: Vec::new(),
        prune_candidate_backup_ids: vec![first.backup_id.clone()],
    };
    assert!(prune_backup_catalogue(&catalogue_root, &incomplete)
        .unwrap_err()
        .to_string()
        .contains("complete disjoint"));

    let catalogue_bytes = std::fs::read(catalogue_root.join("catalogue.json")).unwrap();
    let archive = catalogue_root.join(&first.archive_file);
    let mut file = OpenOptions::new().write(true).open(&archive).unwrap();
    file.seek(SeekFrom::End(-1)).unwrap();
    file.write_all(&[0x7f]).unwrap();
    file.sync_all().unwrap();
    let valid_partition = BackupPrunePlan {
        expected_catalogue_sha256: before.catalogue_sha256,
        retained_backup_ids: vec![second.backup_id],
        prune_candidate_backup_ids: vec![first.backup_id],
    };
    assert!(prune_backup_catalogue(&catalogue_root, &valid_partition).is_err());
    assert_eq!(
        std::fs::read(catalogue_root.join("catalogue.json")).unwrap(),
        catalogue_bytes
    );
}

#[test]
fn archive_or_catalogue_corruption_fails_closed() {
    let root = tempfile::tempdir().unwrap();
    let engine = RrflowKvStore::open(&root.path().join("source")).unwrap();
    let catalogue_root = root.path().join("catalogue");
    engine.claims().append_batch(&[claim("one", 10)]).unwrap();
    let entry = create_logical_backup(&engine, &catalogue_root, "daily", 100).unwrap();
    let archive = catalogue_root.join(&entry.archive_file);
    let mut file = OpenOptions::new().write(true).open(&archive).unwrap();
    file.seek(SeekFrom::End(-1)).unwrap();
    file.write_all(&[0x7f]).unwrap();
    file.sync_all().unwrap();
    assert!(verify_backup_catalogue(&catalogue_root).is_err());
    let target = root.path().join("must-not-exist");
    assert!(restore_catalogued_backup(&catalogue_root, &entry.backup_id, &target, 200).is_err());
    assert!(!target.exists());

    let catalogue = catalogue_root.join("catalogue.json");
    let mut bytes = std::fs::read(&catalogue).unwrap();
    let position = bytes.iter().position(|byte| *byte == b'f').unwrap();
    bytes[position] = b'e';
    std::fs::write(&catalogue, bytes).unwrap();
    assert!(load_backup_catalogue(&catalogue_root).is_err());
}

#[test]
fn application_backup_payload_corruption_fails_before_restore_publication() {
    let root = tempfile::tempdir().unwrap();
    let runtime = DataRuntime::new(
        RrflowKvStore::open(&root.path().join("source")).unwrap(),
        LocalObjectStore::open(root.path().join("source-objects")).unwrap(),
    );
    let scope = ScopeId::new("instance:complete-backup").unwrap();
    let mut schema = RuntimeSchemaRegistry::empty(1, "backup object schema");
    schema
        .define_record_table(
            RuntimeType::new("document").unwrap(),
            RuntimeLogicalModel::Relational,
            RuntimeRecordSchema::default(),
        )
        .unwrap();
    schema.tables.insert(
        RuntimeType::new("object").unwrap(),
        RuntimeTableSchema::schemaless(RuntimeLogicalModel::Object),
    );
    runtime
        .engine()
        .runtime()
        .commit(&RuntimeCommit {
            scope: scope.clone(),
            at: 10,
            actor: "backup-test".into(),
            expected_cursor: 0,
            mutations: vec![
                RuntimeMutation::Schema { registry: schema },
                RuntimeMutation::Record {
                    record: RuntimeRecord {
                        reference: RuntimeRef::new("document", "one").unwrap(),
                        valid_from: 10,
                        valid_to: None,
                        properties: RuntimeProperties::new(),
                    },
                },
            ],
        })
        .unwrap();
    let object = runtime
        .stage_object(
            "document-one-payload",
            Some(RuntimeRef::new("document", "one").unwrap()),
            "application/octet-stream",
            b"authenticated backup payload",
        )
        .unwrap();
    let read = runtime.engine().runtime().read_stamp(&scope).unwrap();
    runtime
        .commit(
            &DataTransaction::new(
                read.clone(),
                RuntimeCommit {
                    scope,
                    at: 20,
                    actor: "backup-test".into(),
                    expected_cursor: read.commit_cursor,
                    mutations: vec![RuntimeMutation::Object {
                        object: object.clone(),
                    }],
                },
            )
            .unwrap(),
        )
        .unwrap();
    let catalogue_root = root.path().join("catalogue");
    let entry = create_application_backup(
        runtime.engine(),
        runtime.objects(),
        &catalogue_root,
        "complete",
        100,
        &control_families(),
    )
    .unwrap();
    assert_eq!(entry.object_payloads, BackupCoverage::Included);
    assert_eq!(entry.catalogues, BackupCoverage::Included);
    assert!(entry.application_complete);
    verify_backup_catalogue(&catalogue_root).unwrap();

    let retained = catalogue_root.join("payloads").join(&object.receipt.key);
    let mut file = OpenOptions::new().write(true).open(retained).unwrap();
    file.seek(SeekFrom::End(-1)).unwrap();
    file.write_all(&[0x7f]).unwrap();
    file.sync_all().unwrap();
    assert!(verify_backup_catalogue(&catalogue_root).is_err());
    let target = root.path().join("must-not-publish");
    assert!(restore_catalogued_backup(&catalogue_root, &entry.backup_id, &target, 200).is_err());
    assert!(!target.exists());
}

#[test]
fn application_backup_restores_object_catalogue_and_audit_closure() {
    application_closure(false);
}

#[test]
fn configured_application_restore_rejects_catalogue_then_resumes_closure() {
    application_closure(true);
}

fn application_closure(configured: bool) {
    let root = tempfile::tempdir().unwrap();
    let runtime = DataRuntime::new(
        RrflowKvStore::open(&root.path().join("source")).unwrap(),
        LocalObjectStore::open(root.path().join("source-objects")).unwrap(),
    );
    let scope = ScopeId::new("instance:closure").unwrap();
    let mut schema = RuntimeSchemaRegistry::empty(1, "backup closure schema");
    schema
        .define_record_table(
            RuntimeType::new("document").unwrap(),
            RuntimeLogicalModel::Relational,
            RuntimeRecordSchema::default(),
        )
        .unwrap();
    schema.tables.insert(
        RuntimeType::new("object").unwrap(),
        RuntimeTableSchema::schemaless(RuntimeLogicalModel::Object),
    );
    let bootstrap = runtime
        .engine()
        .runtime()
        .commit(&RuntimeCommit {
            scope: scope.clone(),
            at: 10,
            actor: "backup-closure".into(),
            expected_cursor: 0,
            mutations: vec![
                RuntimeMutation::Schema { registry: schema },
                RuntimeMutation::Record {
                    record: RuntimeRecord {
                        reference: RuntimeRef::new("document", "one").unwrap(),
                        valid_from: 10,
                        valid_to: None,
                        properties: RuntimeProperties::new(),
                    },
                },
            ],
        })
        .unwrap();
    let object = runtime
        .stage_object(
            "document-one-payload",
            Some(RuntimeRef::new("document", "one").unwrap()),
            "text/plain",
            b"portable object closure",
        )
        .unwrap();
    let read = runtime.engine().runtime().read_stamp(&scope).unwrap();
    let object_commit = runtime
        .commit(
            &DataTransaction::new(
                read,
                RuntimeCommit {
                    scope: scope.clone(),
                    at: 20,
                    actor: "backup-closure".into(),
                    expected_cursor: bootstrap.last_cursor,
                    mutations: vec![RuntimeMutation::Object {
                        object: object.clone(),
                    }],
                },
            )
            .unwrap(),
        )
        .unwrap();
    let catalogue_key = format!(
        "server/state/vector-collection-catalogue/{}",
        scope.as_str()
    );
    let catalogue_value = if configured {
        serde_json::to_vec(&serde_json::json!({"collection": "documents", "vector": "body", "description": "x".repeat(256 * 1024)})).unwrap()
    } else {
        br#"{"collection":"documents","vector":"body"}"#.to_vec()
    };
    runtime
        .engine()
        .control()
        .commit_catalog(
            &scope,
            &ControlTransition {
                key: catalogue_key.clone(),
                expected: None,
                replacement: Some(catalogue_value.clone()),
                at: 30,
                actor: "backup-closure".into(),
                action: "vector_catalogue.created".into(),
                request_id: "backup-closure-request".into(),
                operation_id: "backup-closure-operation".into(),
            },
        )
        .unwrap();

    let catalogue_root = root.path().join("catalogue");
    let entry = create_application_backup(
        runtime.engine(),
        runtime.objects(),
        &catalogue_root,
        "closure",
        100,
        &control_families(),
    )
    .unwrap();
    assert_eq!(
        entry.archive.runtime_audit_sha256.as_deref(),
        runtime
            .engine()
            .runtime()
            .audit(&object_commit.commit_id)
            .unwrap()
            .as_ref()
            .map(|audit| audit.digest.as_str())
    );
    assert_eq!(entry.object_manifest.as_ref().unwrap().object_count, 1);
    assert_eq!(entry.catalogue_manifest.as_ref().unwrap().record_count, 1);

    let target = root.path().join("restored");
    if configured {
        let low = rrd_store::DatabaseOptions {
            mutable_limits: rrd_store::MutableLimits {
                accounted_bytes: 128 * 1024,
                versions: 8192,
            },
            ..Default::default()
        };
        let error = rrd_store::restore_catalogued_backup_with_options(
            &catalogue_root,
            &entry.backup_id,
            &target,
            200,
            low,
        )
        .unwrap_err();
        assert!(
            matches!(
                error,
                rrd_store::Error::MutableLimit {
                    resource: rrd_store::MutableResource::AccountedBytes,
                    ..
                }
            ),
            "{error}"
        );
        assert!(!target.exists());
        let staging = root.path().join(format!(
            ".restored.rrd-restore-{}.staging",
            &entry.archive.archive_sha256[..16]
        ));
        let partial = RrflowKvStore::open_existing_with_options(&staging, low).unwrap();
        assert_eq!(partial.runtime().cursor().unwrap(), 3);
        assert_eq!(
            partial
                .runtime()
                .read_stamp(&scope)
                .unwrap()
                .catalog_revision,
            0
        );
        assert_eq!(partial.control().get(&catalogue_key).unwrap(), None);
        drop(partial);
        let admitted = rrd_store::DatabaseOptions {
            mutable_limits: rrd_store::MutableLimits {
                accounted_bytes: 8 * 1024 * 1024,
                versions: 8192,
            },
            ..Default::default()
        };
        let report = rrd_store::restore_catalogued_backup_with_options(
            &catalogue_root,
            &entry.backup_id,
            &target,
            201,
            admitted,
        )
        .unwrap();
        assert!(report.resumed && report.reopened);
        assert!(!staging.exists());
        let before = directory_bytes(&target);
        let denied = rrd_store::verify_restored_backup_objects_with_options(
            &catalogue_root,
            &entry.backup_id,
            &target,
            low,
        )
        .unwrap_err();
        assert!(
            matches!(denied, rrd_store::Error::MutableLimit { .. }),
            "{denied}"
        );
        assert_eq!(directory_bytes(&target), before);
        rrd_store::verify_restored_backup_objects_with_options(
            &catalogue_root,
            &entry.backup_id,
            &target,
            admitted,
        )
        .unwrap();
    } else {
        restore_catalogued_backup(&catalogue_root, &entry.backup_id, &target, 200).unwrap();
    }
    let restored = RrflowKvStore::open(&target).unwrap();
    assert_eq!(
        restored.control().get(&catalogue_key).unwrap(),
        Some(catalogue_value)
    );
    assert_eq!(
        restored
            .runtime()
            .read_stamp(&scope)
            .unwrap()
            .catalog_revision,
        1
    );
    assert_eq!(
        restored.runtime().audit(&object_commit.commit_id).unwrap(),
        runtime
            .engine()
            .runtime()
            .audit(&object_commit.commit_id)
            .unwrap()
    );
    let restored_objects = LocalObjectStore::open(target.join("immutable")).unwrap();
    assert_eq!(
        restored_objects.get(&object).unwrap(),
        b"portable object closure"
    );
}

#[test]
fn application_backup_carries_and_replays_the_declared_control_journal() {
    control_journal_closure(false);
}

#[test]
fn configured_application_restore_retains_control_prefix_and_resumes_exactly() {
    control_journal_closure(true);
}

fn control_journal_closure(configured: bool) {
    let root = tempfile::tempdir().unwrap();
    let engine = RrflowKvStore::open(&root.path().join("source")).unwrap();
    let objects = LocalObjectStore::open(root.path().join("source-objects")).unwrap();
    engine.claims().append_batch(&[claim("one", 10)]).unwrap();
    let scope = ScopeId::new("instance:control").unwrap();
    let catalogue_key = format!("server/state/index-catalogue/{}", scope.as_str());

    // Installed control state: a record that is later replaced, a policy, an
    // audit chain whose head moves, and an attunement job that is deleted.
    let record_v1 = br#"{"installed":1}"#;
    let replacement = if configured {
        vec![b'x'; 256 * 1024]
    } else {
        br#"{"installed":2}"#.to_vec()
    };
    let record_v2 = replacement.as_slice();
    let audit_one = br#"{"audit":"one"}"#;
    let audit_two = br#"{"audit":"two"}"#;
    let head_one = br#"{"head":"one"}"#;
    let head_two = br#"{"head":"two"}"#;
    let control = engine.control();
    control
        .commit(&transition(
            "server/state/install/record",
            None,
            Some(record_v1),
            100,
            "install.record",
        ))
        .unwrap();
    control
        .commit(&transition(
            "server/state/test/security/policy",
            None,
            Some(br#"{"policy":true}"#),
            101,
            "security.initialized",
        ))
        .unwrap();
    control
        .commit_batch(&[
            transition(
                "server/state/test/audit/one",
                None,
                Some(audit_one),
                102,
                "security.audit",
            ),
            transition(
                "server/state/test/audit-head",
                None,
                Some(head_one),
                102,
                "security.audit.head",
            ),
        ])
        .unwrap();
    // Excluded families: a session lease and the catalogue, which travels in
    // the catalogue manifest instead.
    control
        .commit(&transition(
            "server/state/test/session/lease",
            None,
            Some(br#"{"session":true}"#),
            103,
            "session.created",
        ))
        .unwrap();
    control
        .commit_catalog(
            &scope,
            &transition(
                &catalogue_key,
                None,
                Some(br#"{"indexes":[]}"#),
                104,
                "index_catalogue.created",
            ),
        )
        .unwrap();
    control
        .commit(&transition(
            "server/state/test/attunement/job/j",
            None,
            Some(br#"{"job":"pending"}"#),
            105,
            "attunement.job.created",
        ))
        .unwrap();
    control
        .commit(&transition(
            "server/state/install/record",
            Some(record_v1),
            Some(record_v2),
            106,
            "install.record.replaced",
        ))
        .unwrap();
    control
        .commit_batch(&[
            transition(
                "server/state/test/audit/two",
                None,
                Some(audit_two),
                107,
                "security.audit",
            ),
            transition(
                "server/state/test/audit-head",
                Some(head_one),
                Some(head_two),
                107,
                "security.audit.head",
            ),
        ])
        .unwrap();
    control
        .commit(&transition(
            "server/state/test/attunement/job/j",
            Some(br#"{"job":"pending"}"#),
            None,
            108,
            "attunement.job.removed",
        ))
        .unwrap();
    assert_eq!(control.sequence().unwrap(), 11);

    let catalogue_root = root.path().join("catalogue");
    let entry = create_application_backup(
        &engine,
        &objects,
        &catalogue_root,
        "control",
        200,
        &control_families(),
    )
    .unwrap();
    assert_eq!(entry.installed_control, BackupCoverage::Included);
    assert!(entry.application_complete);
    let inventory = entry.control_archive.as_ref().unwrap();
    assert_eq!(inventory.families, control_families());
    assert_eq!(inventory.source_control_sequence, 11);
    assert_eq!(inventory.entry_count, 9, "two excluded entries stay out");
    assert_eq!(inventory.first_sequence, Some(1));
    assert_eq!(inventory.last_sequence, Some(11));
    assert!(catalogue_root
        .join(entry.control_archive_file.as_ref().unwrap())
        .is_file());
    let catalogue = verify_backup_catalogue(&catalogue_root).unwrap();
    assert_eq!(catalogue.format_version, BACKUP_CATALOGUE_VERSION);

    let target = root.path().join("restored");
    let original_archive = directory_bytes(&catalogue_root);
    if configured {
        let low = rrd_store::DatabaseOptions {
            mutable_limits: rrd_store::MutableLimits {
                accounted_bytes: 128 * 1024,
                versions: 8192,
            },
            ..Default::default()
        };
        let error = rrd_store::restore_catalogued_backup_with_options(
            &catalogue_root,
            &entry.backup_id,
            &target,
            300,
            low,
        )
        .unwrap_err();
        assert!(
            matches!(
                error,
                rrd_store::Error::MutableLimit {
                    resource: rrd_store::MutableResource::AccountedBytes,
                    ..
                }
            ),
            "{error}"
        );
        assert!(!target.exists());
        let staging = root.path().join(format!(
            ".restored.rrd-restore-{}.staging",
            &entry.archive.archive_sha256[..16]
        ));
        let partial = RrflowKvStore::open_existing_with_options(&staging, low).unwrap();
        let progress = control_restore_progress(&partial).unwrap().unwrap();
        assert_eq!(progress.applied, 5);
        assert!(!progress.complete);
        assert_eq!(partial.control().sequence().unwrap(), 6);
        assert_eq!(
            partial
                .control()
                .get("server/state/install/record")
                .unwrap(),
            Some(record_v1.to_vec())
        );
        assert_eq!(partial.claims().sequence().unwrap(), 1);
        drop(partial);
        let admitted = rrd_store::DatabaseOptions {
            mutable_limits: rrd_store::MutableLimits {
                accounted_bytes: 8 * 1024 * 1024,
                versions: 8192,
            },
            ..Default::default()
        };
        let report = rrd_store::restore_catalogued_backup_with_options(
            &catalogue_root,
            &entry.backup_id,
            &target,
            301,
            admitted,
        )
        .unwrap();
        assert!(report.resumed && report.reopened);
        assert!(!staging.exists());
        let before = directory_bytes(&target);
        assert!(matches!(
            rrd_store::verify_restored_backup_objects_with_options(
                &catalogue_root,
                &entry.backup_id,
                &target,
                low
            )
            .unwrap_err(),
            rrd_store::Error::MutableLimit { .. }
        ));
        assert_eq!(directory_bytes(&target), before);
        rrd_store::verify_restored_backup_objects_with_options(
            &catalogue_root,
            &entry.backup_id,
            &target,
            admitted,
        )
        .unwrap();
    } else {
        restore_catalogued_backup(&catalogue_root, &entry.backup_id, &target, 300).unwrap();
    }
    assert_eq!(directory_bytes(&catalogue_root), original_archive);
    let restored = RrflowKvStore::open_existing(&target).unwrap();
    for (key, expected) in [
        ("server/state/install/record", Some(record_v2.to_vec())),
        (
            "server/state/test/security/policy",
            Some(br#"{"policy":true}"#.to_vec()),
        ),
        ("server/state/test/audit/one", Some(audit_one.to_vec())),
        ("server/state/test/audit/two", Some(audit_two.to_vec())),
        ("server/state/test/audit-head", Some(head_two.to_vec())),
        ("server/state/test/attunement/job/j", None),
        ("server/state/test/session/lease", None),
    ] {
        assert_eq!(
            restored.control().get(key).unwrap(),
            expected,
            "restored control value for {key}"
        );
    }
    assert_eq!(
        restored.control().get(&catalogue_key).unwrap(),
        Some(br#"{"indexes":[]}"#.to_vec()),
        "the catalogue still arrives through the catalogue manifest"
    );

    // The audit chain is derived from the control journal's actions and
    // order, so the restored journal must carry the same transitions in the
    // same order after the catalogue restore's own entries.
    let journal = restored.control().journal_since(0, 64).unwrap();
    let replayed = journal
        .iter()
        .filter(|entry| !entry.key.starts_with("server/state/index-catalogue/"))
        .collect::<Vec<_>>();
    let source = engine
        .control()
        .journal_since(0, 64)
        .unwrap()
        .into_iter()
        .filter(|entry| {
            !entry.key.starts_with("server/state/index-catalogue/")
                && !entry.key.starts_with("server/state/test/session/")
        })
        .collect::<Vec<_>>();
    assert_eq!(replayed.len(), source.len());
    for (restored_entry, source_entry) in replayed.iter().zip(&source) {
        assert_eq!(restored_entry.key, source_entry.key);
        assert_eq!(restored_entry.before_sha256, source_entry.before_sha256);
        assert_eq!(restored_entry.replacement, source_entry.replacement);
        assert_eq!(restored_entry.at, source_entry.at);
        assert_eq!(restored_entry.actor, source_entry.actor);
        assert_eq!(restored_entry.action, source_entry.action);
        assert_eq!(restored_entry.request_id, source_entry.request_id);
        assert_eq!(restored_entry.operation_id, source_entry.operation_id);
    }
    assert_eq!(
        journal
            .iter()
            .filter(|entry| entry.action == "security.audit")
            .map(|entry| entry.replacement.clone().unwrap())
            .collect::<Vec<_>>(),
        vec![audit_one.to_vec(), audit_two.to_vec()]
    );
    let progress = control_restore_progress(&restored).unwrap().unwrap();
    assert!(progress.complete);
    assert_eq!(progress.applied, 9);
    assert_eq!(progress.archive_sha256, inventory.archive_sha256);
    drop(restored);
    verify_restored_backup_objects(&catalogue_root, &entry.backup_id, &target).unwrap();
}

#[test]
fn version_one_catalogues_are_refused_rather_than_read_as_complete() {
    let root = tempfile::tempdir().unwrap();
    let catalogue_root = root.path().join("catalogue");
    std::fs::create_dir_all(&catalogue_root).unwrap();
    std::fs::write(
        catalogue_root.join("catalogue.json"),
        br#"{"payload":{"format_version":1,"revision":0,"backups":[]},"payload_sha256":"0000000000000000000000000000000000000000000000000000000000000000"}"#,
    )
    .unwrap();
    assert!(load_backup_catalogue(&catalogue_root)
        .unwrap_err()
        .to_string()
        .contains("unsupported backup catalogue version 1"));
}

#[test]
fn a_control_scope_overlapping_the_catalogue_families_is_refused() {
    let root = tempfile::tempdir().unwrap();
    let engine = RrflowKvStore::open(&root.path().join("source")).unwrap();
    let objects = LocalObjectStore::open(root.path().join("source-objects")).unwrap();
    let catalogue_root = root.path().join("catalogue");
    let error = create_application_backup(
        &engine,
        &objects,
        &catalogue_root,
        "overlap",
        100,
        &["server/state/index-catalogue".to_owned()],
    )
    .unwrap_err();
    assert!(error.to_string().contains("catalogue manifest scope"));
    assert!(!catalogue_root.exists(), "a refused scope writes nothing");
}

#[test]
fn verification_reads_only_and_a_lost_catalogue_is_not_an_empty_one() {
    let root = tempfile::tempdir().unwrap();
    let engine = RrflowKvStore::open(&root.path().join("source")).unwrap();
    let objects = LocalObjectStore::open(root.path().join("source-objects")).unwrap();
    let catalogue_root = root.path().join("catalogue");
    let entry = create_application_backup(
        &engine,
        &objects,
        &catalogue_root,
        "complete",
        100,
        &control_families(),
    )
    .unwrap();
    verify_backup_catalogue(&catalogue_root).unwrap();

    // A missing payload store fails verification without being recreated.
    let payloads = catalogue_root.join("payloads");
    let kept = root.path().join("payloads-kept");
    std::fs::rename(&payloads, &kept).unwrap();
    assert!(verify_backup_catalogue(&catalogue_root).is_err());
    assert!(
        !payloads.exists(),
        "verification must not create the payload store it failed to find"
    );
    std::fs::rename(&kept, &payloads).unwrap();
    verify_backup_catalogue(&catalogue_root).unwrap();

    // Restore verification of a target that lost its object store fails
    // without creating one.
    let target = root.path().join("restored");
    restore_catalogued_backup(&catalogue_root, &entry.backup_id, &target, 200).unwrap();
    verify_restored_backup_objects(&catalogue_root, &entry.backup_id, &target).unwrap();
    let immutable = target.join("immutable");
    let kept = root.path().join("immutable-kept");
    std::fs::rename(&immutable, &kept).unwrap();
    assert!(verify_restored_backup_objects(&catalogue_root, &entry.backup_id, &target).is_err());
    assert!(!immutable.exists());
    std::fs::rename(&kept, &immutable).unwrap();

    // A catalogue file lost beside its retained artifacts is a loss, not an
    // empty catalogue; an untouched root still reads as empty.
    std::fs::remove_file(catalogue_root.join("catalogue.json")).unwrap();
    let lost = load_backup_catalogue(&catalogue_root).unwrap_err();
    assert!(lost.to_string().contains("absent while retained"));
    assert!(verify_backup_catalogue(&catalogue_root).is_err());
    let untouched = load_backup_catalogue(&root.path().join("never-used")).unwrap();
    assert!(untouched.backups.is_empty());
    assert_eq!(untouched.revision, 0);
}

fn directory_bytes(
    root: &std::path::Path,
) -> std::collections::BTreeMap<std::path::PathBuf, Vec<u8>> {
    fn visit(
        root: &std::path::Path,
        at: &std::path::Path,
        files: &mut std::collections::BTreeMap<std::path::PathBuf, Vec<u8>>,
    ) {
        for entry in std::fs::read_dir(at).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                visit(root, &path, files);
            } else {
                files.insert(
                    path.strip_prefix(root).unwrap().to_owned(),
                    std::fs::read(path).unwrap(),
                );
            }
        }
    }
    let mut files = std::collections::BTreeMap::new();
    visit(root, root, &mut files);
    files
}
