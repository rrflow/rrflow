use rrd_core::{
    Claim, DataTransaction, EmbeddingProvenance, GeoPoint, GeoValue, Predicate, Producer,
    RuntimeCommit, RuntimeEvent, RuntimeEventSchema, RuntimeGeo, RuntimeLogicalModel,
    RuntimeMutation, RuntimeProperties, RuntimeRecord, RuntimeRecordSchema, RuntimeRef,
    RuntimeRelation, RuntimeRelationSchema, RuntimeSchemaRegistry, RuntimeSeriesSample,
    RuntimeTableSchema, RuntimeType, RuntimeVector, ScopeId, SeriesValue, Subject,
    VectorCollectionAddress, VectorNormalization, VectorValue,
};
use rrd_store::{
    export_logical_archive, export_logical_archive_with_progress, inspect_logical_archive,
    restore_logical_archive_to_new_root, restore_logical_archive_to_new_root_with_progress,
    DataRuntime, Error, LocalObjectStore, LogicalArchiveCheckpoint, RrflowKvStore, StorageEngine,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::OpenOptions;
use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

fn claim(name: impl std::fmt::Display, at: u64) -> Claim {
    Claim::new(
        Subject::new(format!("archive:{name}")).unwrap(),
        Predicate::new("status").unwrap(),
        "retained",
        at,
        at,
        Producer {
            actor: "agent:archive-resilience".into(),
            on_behalf_of: None,
            session: Some("archive-resilience".into()),
        },
    )
}

fn mixed_source(root: &Path) -> RrflowKvStore {
    let engine = RrflowKvStore::open(root).unwrap();
    engine
        .claims()
        .append_batch(&[claim("before", 10)])
        .unwrap();
    engine
        .runtime()
        .commit(&RuntimeCommit {
            scope: ScopeId::new("instance:resilience").unwrap(),
            at: 20,
            actor: "agent:archive-resilience".into(),
            expected_cursor: 0,
            mutations: vec![RuntimeMutation::Claim {
                claim: claim("runtime-one", 20),
            }],
        })
        .unwrap();
    engine
        .claims()
        .append_batch(&[claim("middle", 30)])
        .unwrap();
    engine
        .runtime()
        .commit(&RuntimeCommit {
            scope: ScopeId::new("instance:resilience").unwrap(),
            at: 40,
            actor: "agent:archive-resilience".into(),
            expected_cursor: 1,
            mutations: vec![RuntimeMutation::Claim {
                claim: claim("runtime-two", 40),
            }],
        })
        .unwrap();
    engine.claims().append_batch(&[claim("after", 50)]).unwrap();
    engine
}

fn assert_same_runtime(left: &RrflowKvStore, right: &RrflowKvStore) {
    assert_eq!(
        left.claims().sequence().unwrap(),
        right.claims().sequence().unwrap()
    );
    assert_eq!(
        left.runtime().cursor().unwrap(),
        right.runtime().cursor().unwrap()
    );
    let claims = left.claims().sequence().unwrap();
    assert_eq!(
        left.claims().claims_in_range(0, claims).unwrap(),
        right.claims().claims_in_range(0, claims).unwrap()
    );
    let cursor = left.runtime().cursor().unwrap();
    let left_changes = left
        .runtime()
        .changes_since(0, cursor as usize + 1, None)
        .unwrap()
        .changes;
    let right_changes = right
        .runtime()
        .changes_since(0, cursor as usize + 1, None)
        .unwrap()
        .changes;
    assert_eq!(left_changes, right_changes);
    for commit_id in left_changes
        .iter()
        .map(|change| &change.commit_id)
        .collect::<BTreeSet<_>>()
    {
        assert_eq!(
            left.runtime().audit(commit_id).unwrap(),
            right.runtime().audit(commit_id).unwrap()
        );
    }
}

#[test]
fn interrupted_export_after_database_boundary_resumes_to_identical_bytes() {
    let root = tempfile::tempdir().unwrap();
    let source = mixed_source(&root.path().join("source"));
    let uninterrupted = root.path().join("uninterrupted.rrd-archive");
    let resumed = root.path().join("resumed.rrd-archive");
    let expected = export_logical_archive(&source, &uninterrupted).unwrap();

    let error = export_logical_archive_with_progress(&source, &resumed, |progress| {
        if progress.checkpoint == LogicalArchiveCheckpoint::ActionDurable
            && progress.completed_actions == 2
        {
            return Err(Error::FaultInjected("export_action_durable"));
        }
        Ok(())
    })
    .unwrap_err();
    assert!(matches!(
        error,
        Error::FaultInjected("export_action_durable")
    ));
    assert!(!resumed.exists());

    let actual = export_logical_archive(&source, &resumed).unwrap();
    assert_eq!(actual, expected);
    assert_eq!(
        std::fs::read(resumed).unwrap(),
        std::fs::read(uninterrupted).unwrap()
    );
}

#[test]
fn interrupted_export_after_receipt_boundary_resumes_idempotently() {
    let root = tempfile::tempdir().unwrap();
    let source = mixed_source(&root.path().join("source"));
    let archive = root.path().join("receipt-resume.rrd-archive");
    let error = export_logical_archive_with_progress(&source, &archive, |progress| {
        if progress.checkpoint == LogicalArchiveCheckpoint::ReceiptDurable
            && progress.completed_actions == 3
        {
            return Err(Error::FaultInjected("export_receipt_durable"));
        }
        Ok(())
    })
    .unwrap_err();
    assert!(matches!(
        error,
        Error::FaultInjected("export_receipt_durable")
    ));
    let inventory = export_logical_archive(&source, &archive).unwrap();
    assert_eq!(inspect_logical_archive(&archive).unwrap(), inventory);
}

#[test]
fn interrupted_export_after_final_footer_resumes_by_publication() {
    let root = tempfile::tempdir().unwrap();
    let source = mixed_source(&root.path().join("source"));
    let uninterrupted = root.path().join("uninterrupted.rrd-archive");
    let resumed = root.path().join("finalized-resume.rrd-archive");
    let expected = export_logical_archive(&source, &uninterrupted).unwrap();

    let error = export_logical_archive_with_progress(&source, &resumed, |progress| {
        if progress.checkpoint == LogicalArchiveCheckpoint::ArchiveFinalized {
            return Err(Error::FaultInjected("export_archive_finalized"));
        }
        Ok(())
    })
    .unwrap_err();
    assert!(matches!(
        error,
        Error::FaultInjected("export_archive_finalized")
    ));
    assert!(!resumed.exists());

    let actual = export_logical_archive(&source, &resumed).unwrap();
    assert_eq!(actual, expected);
    assert_eq!(
        std::fs::read(resumed).unwrap(),
        std::fs::read(uninterrupted).unwrap()
    );
}

#[test]
fn tampered_export_receipt_is_denied() {
    let root = tempfile::tempdir().unwrap();
    let source = mixed_source(&root.path().join("source"));
    let archive = root.path().join("tamper.rrd-archive");
    export_logical_archive_with_progress(&source, &archive, |progress| {
        if progress.checkpoint == LogicalArchiveCheckpoint::ReceiptDurable
            && progress.completed_actions == 1
        {
            return Err(Error::FaultInjected("leave_receipt"));
        }
        Ok(())
    })
    .unwrap_err();
    let receipt = root
        .path()
        .join(".tamper.rrd-archive.rrd-export.receipt.json");
    flip_last_byte(&receipt);
    assert!(export_logical_archive(&source, &archive).is_err());
    assert!(!archive.exists());
}

#[test]
fn missing_export_receipt_is_reconstructed_from_the_durable_prefix() {
    let root = tempfile::tempdir().unwrap();
    let source = mixed_source(&root.path().join("source"));
    let archive = root.path().join("missing-receipt.rrd-archive");
    export_logical_archive_with_progress(&source, &archive, |progress| {
        if progress.checkpoint == LogicalArchiveCheckpoint::ActionDurable
            && progress.completed_actions == 2
        {
            return Err(Error::FaultInjected("remove_receipt"));
        }
        Ok(())
    })
    .unwrap_err();
    std::fs::remove_file(
        root.path()
            .join(".missing-receipt.rrd-archive.rrd-export.receipt.json"),
    )
    .unwrap();
    let inventory = export_logical_archive(&source, &archive).unwrap();
    assert_eq!(inspect_logical_archive(&archive).unwrap(), inventory);
}

#[test]
fn source_advancement_invalidates_an_unfinished_export_cut() {
    let root = tempfile::tempdir().unwrap();
    let source = mixed_source(&root.path().join("source"));
    let archive = root.path().join("advanced.rrd-archive");
    export_logical_archive_with_progress(&source, &archive, |progress| {
        if progress.checkpoint == LogicalArchiveCheckpoint::ReceiptDurable
            && progress.completed_actions == 1
        {
            return Err(Error::FaultInjected("source_will_advance"));
        }
        Ok(())
    })
    .unwrap_err();
    source
        .claims()
        .append_batch(&[claim("new-head", 60)])
        .unwrap();
    assert!(export_logical_archive(&source, &archive).is_err());
    assert!(!archive.exists());
    assert!(!root
        .path()
        .join(".advanced.rrd-archive.rrd-export.partial")
        .exists());
    assert!(!root
        .path()
        .join(".advanced.rrd-archive.rrd-export.receipt.json")
        .exists());
    export_logical_archive(&source, &archive).unwrap();
}

#[test]
fn interrupted_restore_reconciles_action_ahead_of_receipt() {
    let root = tempfile::tempdir().unwrap();
    let source = mixed_source(&root.path().join("source"));
    let archive = root.path().join("backup.rrd-archive");
    let target = root.path().join("restored");
    export_logical_archive(&source, &archive).unwrap();
    let error = restore_logical_archive_to_new_root_with_progress(
        &archive,
        &target,
        100,
        |_| Ok(()),
        |progress| {
            if progress.checkpoint == LogicalArchiveCheckpoint::ActionDurable
                && progress.completed_actions == 2
            {
                return Err(Error::FaultInjected("restore_action_durable"));
            }
            Ok(())
        },
    )
    .unwrap_err();
    assert!(matches!(
        error,
        Error::FaultInjected("restore_action_durable")
    ));
    assert!(!target.exists());

    let report = restore_logical_archive_to_new_root(&archive, &target, 101).unwrap();
    assert!(report.resumed);
    assert_same_runtime(&source, &RrflowKvStore::open(&target).unwrap());
}

#[test]
fn interrupted_restore_after_receipt_resumes_without_duplicate_mutations() {
    let root = tempfile::tempdir().unwrap();
    let source = mixed_source(&root.path().join("source"));
    let archive = root.path().join("backup.rrd-archive");
    let target = root.path().join("restored");
    export_logical_archive(&source, &archive).unwrap();
    restore_logical_archive_to_new_root_with_progress(
        &archive,
        &target,
        100,
        |_| Ok(()),
        |progress| {
            if progress.checkpoint == LogicalArchiveCheckpoint::ReceiptDurable
                && progress.completed_actions == 3
            {
                return Err(Error::FaultInjected("restore_receipt_durable"));
            }
            Ok(())
        },
    )
    .unwrap_err();
    restore_logical_archive_to_new_root(&archive, &target, 101).unwrap();
    assert_same_runtime(&source, &RrflowKvStore::open(&target).unwrap());
}

#[test]
fn tampered_restore_receipt_is_denied_before_target_publication() {
    let root = tempfile::tempdir().unwrap();
    let source = mixed_source(&root.path().join("source"));
    let archive = root.path().join("backup.rrd-archive");
    let target = root.path().join("restored");
    let inventory = export_logical_archive(&source, &archive).unwrap();
    restore_logical_archive_to_new_root_with_progress(
        &archive,
        &target,
        100,
        |_| Ok(()),
        |progress| {
            if progress.checkpoint == LogicalArchiveCheckpoint::ReceiptDurable
                && progress.completed_actions == 1
            {
                return Err(Error::FaultInjected("leave_restore_receipt"));
            }
            Ok(())
        },
    )
    .unwrap_err();
    let receipt = restore_receipt(root.path(), &target, &inventory.archive_sha256);
    flip_last_byte(&receipt);
    assert!(restore_logical_archive_to_new_root(&archive, &target, 101).is_err());
    assert!(!target.exists());
}

#[test]
fn claim_stream_crosses_multiple_pages_without_reordering() {
    let root = tempfile::tempdir().unwrap();
    let source = RrflowKvStore::open(&root.path().join("source")).unwrap();
    let claims = (1..=2_100)
        .map(|index| claim(index, index))
        .collect::<Vec<_>>();
    source.claims().append_batch(&claims).unwrap();
    let archive = root.path().join("claims.rrd-archive");
    let target = root.path().join("restored");
    let inventory = export_logical_archive(&source, &archive).unwrap();
    assert_eq!(inventory.standalone_claims, 2_100);
    restore_logical_archive_to_new_root(&archive, &target, 100).unwrap();
    assert_eq!(
        RrflowKvStore::open(&target)
            .unwrap()
            .claims()
            .claims_in_range(0, 2_100)
            .unwrap(),
        claims
    );
}

#[test]
fn one_commit_split_across_runtime_pages_remains_atomic() {
    let root = tempfile::tempdir().unwrap();
    let source = RrflowKvStore::open(&root.path().join("source")).unwrap();
    let mutations = (1..=1_100)
        .map(|index| RuntimeMutation::Claim {
            claim: claim(format!("runtime-{index}"), index),
        })
        .collect::<Vec<_>>();
    let outcome = source
        .runtime()
        .commit(&RuntimeCommit {
            scope: ScopeId::new("instance:paged-commit").unwrap(),
            at: 1_100,
            actor: "agent:paged-commit".into(),
            expected_cursor: 0,
            mutations,
        })
        .unwrap();
    let archive = root.path().join("runtime.rrd-archive");
    let target = root.path().join("restored");
    let inventory = export_logical_archive(&source, &archive).unwrap();
    assert_eq!(inventory.runtime_commits, 1);
    assert_eq!(inventory.runtime_mutations, 1_100);
    restore_logical_archive_to_new_root(&archive, &target, 100).unwrap();
    let restored = RrflowKvStore::open(&target).unwrap();
    assert_eq!(
        restored
            .runtime()
            .commit_outcome(&outcome.commit_id)
            .unwrap(),
        Some(outcome)
    );
    assert_same_runtime(&source, &restored);
}

#[test]
fn every_canonical_runtime_family_and_transaction_audit_round_trips() {
    all_family_round_trip(false);
}

#[test]
fn configured_restore_preserves_all_runtime_families_and_audit() {
    all_family_round_trip(true);
}

fn all_family_round_trip(configured: bool) {
    let root = tempfile::tempdir().unwrap();
    let source_path = root.path().join("source");
    let objects = root.path().join("objects");
    let runtime = DataRuntime::new(
        RrflowKvStore::open(&source_path).unwrap(),
        LocalObjectStore::open(&objects).unwrap(),
    );
    let scope = ScopeId::new("instance:all-families").unwrap();
    runtime
        .engine()
        .runtime()
        .commit(&RuntimeCommit {
            scope: scope.clone(),
            at: 10,
            actor: "agent:all-families".into(),
            expected_cursor: 0,
            mutations: bootstrap_mutations(),
        })
        .unwrap();
    let object = runtime
        .stage_object(
            "entity-a-source",
            Some(RuntimeRef::new("entity", "a").unwrap()),
            "text/plain",
            b"object bytes live in the application backup closure",
        )
        .unwrap();
    let read = runtime.engine().runtime().read_stamp(&scope).unwrap();
    let transaction = DataTransaction::new(
        read,
        RuntimeCommit {
            scope: scope.clone(),
            at: 20,
            actor: "agent:all-families".into(),
            expected_cursor: 3,
            mutations: all_family_mutations(object),
        },
    )
    .unwrap();
    let outcome = runtime.commit(&transaction).unwrap();
    let archive = root.path().join("all-families.rrd-archive");
    let target = root.path().join("restored");
    export_logical_archive(runtime.engine(), &archive).unwrap();
    if configured {
        rrd_store::restore_logical_archive_to_new_root_with_options(
            &archive,
            &target,
            100,
            rrd_store::DatabaseOptions {
                mutable_limits: rrd_store::MutableLimits {
                    accounted_bytes: 512 * 1024,
                    versions: 4096,
                },
                ..Default::default()
            },
            |_| Ok(()),
            |_| Ok(()),
        )
        .unwrap();
    } else {
        restore_logical_archive_to_new_root(&archive, &target, 100).unwrap();
    }
    let restored = RrflowKvStore::open(&target).unwrap();
    assert_same_runtime(runtime.engine(), &restored);
    let expected = runtime
        .engine()
        .runtime()
        .data_snapshot(&scope, 100, 4096)
        .unwrap();
    let actual = restored.runtime().data_snapshot(&scope, 100, 4096).unwrap();
    assert_eq!(actual.read, expected.read);
    assert_eq!(actual.selected_versions, expected.selected_versions);
    assert_eq!(actual.snapshot, expected.snapshot);
    let data = &actual.snapshot;
    assert_eq!(data.records.len(), 2);
    for count in [
        data.claims.len(),
        data.relations.len(),
        data.events.len(),
        data.vectors.len(),
        data.series.len(),
        data.geo.len(),
        data.objects.len(),
    ] {
        assert_eq!(count, 1);
    }
    let subject = RuntimeRef::new("entity", "a").unwrap();
    assert_eq!(data.relations[0].value.from, subject);
    assert_eq!(
        data.relations[0].value.to,
        RuntimeRef::new("entity", "b").unwrap()
    );
    assert_eq!(data.vectors[0].value.subject, subject);
    assert_eq!(
        data.vectors[0]
            .value
            .provenance
            .as_ref()
            .unwrap()
            .source_digest,
        "11".repeat(32)
    );
    assert_eq!(
        restored.runtime().outbox_since(0, 64).unwrap(),
        runtime.engine().runtime().outbox_since(0, 64).unwrap()
    );
    assert_eq!(
        restored.runtime().projection_deltas_since(0, 64).unwrap(),
        runtime
            .engine()
            .runtime()
            .projection_deltas_since(0, 64)
            .unwrap()
    );

    assert_eq!(
        restored.runtime().audit(&outcome.commit_id).unwrap(),
        runtime
            .engine()
            .runtime()
            .audit(&outcome.commit_id)
            .unwrap()
    );
}

fn bootstrap_mutations() -> Vec<RuntimeMutation> {
    let mut schema = RuntimeSchemaRegistry::empty(1, "archive all-family schema");
    let entity = RuntimeType::new("entity").unwrap();
    schema
        .define_record_table(
            entity.clone(),
            RuntimeLogicalModel::Relational,
            RuntimeRecordSchema::default(),
        )
        .unwrap();
    schema
        .define_relation_table(
            RuntimeType::new("links").unwrap(),
            RuntimeRelationSchema {
                from: BTreeSet::from([entity.clone()]),
                to: BTreeSet::from([entity.clone()]),
                ..RuntimeRelationSchema::default()
            },
        )
        .unwrap();
    schema
        .define_event_table(
            RuntimeType::new("observed").unwrap(),
            RuntimeLogicalModel::Event,
            RuntimeEventSchema {
                subject_required: true,
                subject_types: BTreeSet::from([entity]),
                properties: BTreeMap::new(),
                allow_additional_properties: false,
            },
        )
        .unwrap();
    for (kind, model) in [
        ("embedding", RuntimeLogicalModel::Vector),
        ("sample", RuntimeLogicalModel::TimeSeries),
        ("location", RuntimeLogicalModel::Geo),
        ("object", RuntimeLogicalModel::Object),
    ] {
        schema.tables.insert(
            RuntimeType::new(kind).unwrap(),
            RuntimeTableSchema::schemaless(model),
        );
    }
    vec![
        RuntimeMutation::Schema { registry: schema },
        RuntimeMutation::Record {
            record: record("a"),
        },
        RuntimeMutation::Record {
            record: record("b"),
        },
    ]
}

fn record(id: &str) -> RuntimeRecord {
    RuntimeRecord {
        reference: RuntimeRef::new("entity", id).unwrap(),
        valid_from: 10,
        valid_to: None,
        properties: RuntimeProperties::new(),
    }
}

fn all_family_mutations(object: rrd_core::ObjectReference) -> Vec<RuntimeMutation> {
    let subject = RuntimeRef::new("entity", "a").unwrap();
    vec![
        RuntimeMutation::Claim {
            claim: claim("all-families", 20),
        },
        RuntimeMutation::Relation {
            relation: RuntimeRelation {
                reference: RuntimeRef::new("links", "a-b").unwrap(),
                from: subject.clone(),
                to: RuntimeRef::new("entity", "b").unwrap(),
                valid_from: 20,
                valid_to: None,
                properties: RuntimeProperties::new(),
            },
        },
        RuntimeMutation::Event {
            event: RuntimeEvent {
                kind: RuntimeType::new("observed").unwrap(),
                subject: Some(subject.clone()),
                properties: RuntimeProperties::new(),
            },
        },
        RuntimeMutation::Vector {
            vector: RuntimeVector {
                reference: RuntimeRef::new("embedding", "a-title").unwrap(),
                subject: subject.clone(),
                collection: VectorCollectionAddress {
                    collection_id: "documents".into(),
                    vector_name: "title".into(),
                },
                field: "title".into(),
                valid_from: 20,
                valid_to: None,
                value: VectorValue::Dense {
                    values: vec![0.0, 0.6, 0.8],
                },
                provenance: Some(EmbeddingProvenance {
                    source_digest: "11".repeat(32),
                    model: "fixture-embedder".into(),
                    model_digest: "22".repeat(32),
                    dimensions: 3,
                    normalization: VectorNormalization::UnitL2,
                    generation_parameters: RuntimeProperties::new(),
                }),
                properties: RuntimeProperties::new(),
            },
        },
        RuntimeMutation::SeriesSample {
            sample: RuntimeSeriesSample {
                reference: RuntimeRef::new("sample", "a-temperature-20").unwrap(),
                series: subject.clone(),
                observed_at: 20,
                value: SeriesValue::Decimal("21.5".into()),
                properties: RuntimeProperties::new(),
            },
        },
        RuntimeMutation::Geo {
            geo: RuntimeGeo {
                reference: RuntimeRef::new("location", "a-current").unwrap(),
                subject,
                field: "position".into(),
                valid_from: 20,
                valid_to: None,
                value: GeoValue::Point {
                    point: GeoPoint {
                        longitude: -122.4194,
                        latitude: 37.7749,
                    },
                },
                properties: RuntimeProperties::new(),
            },
        },
        RuntimeMutation::Object { object },
    ]
}

fn restore_receipt(root: &Path, target: &Path, archive_sha256: &str) -> PathBuf {
    root.join(format!(
        ".{}.rrd-restore-{}.receipt.json",
        target.file_name().unwrap().to_string_lossy(),
        &archive_sha256[..16]
    ))
}

fn flip_last_byte(path: &Path) {
    let mut file = OpenOptions::new().write(true).open(path).unwrap();
    file.seek(SeekFrom::End(-1)).unwrap();
    file.write_all(b"x").unwrap();
    file.sync_all().unwrap();
}

#[test]
fn configured_restore_rejection_retains_acknowledged_prefix_and_resumes_exactly() {
    for (limits, resource) in [
        (
            rrd_store::MutableLimits {
                accounted_bytes: 128 * 1024,
                versions: 8192,
            },
            rrd_store::MutableResource::AccountedBytes,
        ),
        (
            rrd_store::MutableLimits {
                accounted_bytes: 8 * 1024 * 1024,
                versions: 64,
            },
            rrd_store::MutableResource::Versions,
        ),
    ] {
        let directory = tempfile::tempdir().unwrap();
        let source = RrflowKvStore::open(&directory.path().join("source")).unwrap();
        let acknowledged = claim("prior", 10);
        source
            .claims()
            .append_batch(std::slice::from_ref(&acknowledged))
            .unwrap();
        let request = RuntimeCommit {
            scope: ScopeId::new("instance:restore-admission").unwrap(),
            at: 20,
            actor: "restore-admission".into(),
            expected_cursor: 0,
            mutations: (0..32)
                .map(|index| {
                    let mut value = claim(format!("large-{index}"), 20);
                    value.object = "x".repeat(4096);
                    RuntimeMutation::Claim { claim: value }
                })
                .collect(),
        };
        let expected = source.runtime().commit(&request).unwrap();
        let archive = directory.path().join("source.archive");
        let inventory = export_logical_archive(&source, &archive).unwrap();
        let archive_bytes = std::fs::read(&archive).unwrap();
        let target = directory.path().join("target");
        let policy = rrd_store::DatabaseOptions {
            mutable_limits: limits,
            ..Default::default()
        };
        let error = rrd_store::restore_logical_archive_to_new_root_with_options(
            &archive,
            &target,
            100,
            policy,
            |_| Ok(()),
            |_| Ok(()),
        )
        .unwrap_err();
        assert!(
            matches!(error, Error::MutableLimit { resource: actual, limit, observed }
            if actual == resource && observed > limit),
            "{error}"
        );
        assert!(!target.exists());
        let staging = directory.path().join(format!(
            ".target.rrd-restore-{}.staging",
            &inventory.archive_sha256[..16]
        ));
        let staged = RrflowKvStore::open_existing_with_options(&staging, policy).unwrap();
        assert_eq!(
            staged.claims().claims_in_range(0, 64).unwrap(),
            vec![acknowledged]
        );
        assert_eq!(staged.runtime().cursor().unwrap(), 0);
        assert!(staged
            .runtime()
            .commit_outcome(&request.digest())
            .unwrap()
            .is_none());
        drop(staged);
        let admitted = rrd_store::DatabaseOptions {
            mutable_limits: rrd_store::MutableLimits {
                accounted_bytes: 8 * 1024 * 1024,
                versions: 8192,
            },
            ..Default::default()
        };
        let restored = rrd_store::restore_logical_archive_to_new_root_with_options(
            &archive,
            &target,
            101,
            admitted,
            |_| Ok(()),
            |_| Ok(()),
        )
        .unwrap();
        assert!(restored.resumed && restored.reopened);
        assert!(!staging.exists());
        assert!(!restore_receipt(directory.path(), &target, &inventory.archive_sha256).exists());
        let reopened = RrflowKvStore::open_existing_with_options(&target, admitted).unwrap();
        assert_same_runtime(&source, &reopened);
        assert_eq!(reopened.claims().sequence().unwrap(), 33);
        assert_eq!(reopened.runtime().cursor().unwrap(), 32);
        assert_eq!(
            reopened
                .runtime()
                .commit_outcome(&request.digest())
                .unwrap(),
            Some(expected)
        );
        assert_eq!(std::fs::read(&archive).unwrap(), archive_bytes);
    }
}

#[test]
fn configured_restore_checks_population_reopen_before_publication() {
    let directory = tempfile::tempdir().unwrap();
    let source = RrflowKvStore::open(&directory.path().join("source")).unwrap();
    source.claims().append_batch(&[claim("prior", 10)]).unwrap();
    let archive = directory.path().join("source.archive");
    let inventory = export_logical_archive(&source, &archive).unwrap();
    let target = directory.path().join("target");
    let key = "server/state/population/value";
    let value = vec![b'x'; 256 * 1024];
    let populate = |staging: &rrd_store::filesystem::AnchoredPath| {
        let store = RrflowKvStore::open(staging)?;
        if store.control().get(key)?.is_none() {
            store.control().commit(&rrd_store::ControlTransition {
                key: key.into(),
                expected: None,
                replacement: Some(value.clone()),
                at: 20,
                actor: "population".into(),
                action: "populated".into(),
                request_id: "population".into(),
                operation_id: "population".into(),
            })?;
        }
        Ok(())
    };
    let low = rrd_store::DatabaseOptions {
        mutable_limits: rrd_store::MutableLimits {
            accounted_bytes: 128 * 1024,
            versions: 8192,
        },
        ..Default::default()
    };
    let error = rrd_store::restore_logical_archive_to_new_root_with_options(
        &archive,
        &target,
        100,
        low,
        populate,
        |_| Ok(()),
    )
    .unwrap_err();
    assert!(matches!(error, Error::MutableLimit { .. }), "{error}");
    assert!(!target.exists());
    let staging = directory.path().join(format!(
        ".target.rrd-restore-{}.staging",
        &inventory.archive_sha256[..16]
    ));
    let files = restore_file_bytes(&staging);
    let error = rrd_store::restore_logical_archive_to_new_root_with_options(
        &archive,
        &target,
        101,
        low,
        |_| panic!("denied replay must not call population"),
        |_| Ok(()),
    )
    .unwrap_err();
    assert!(matches!(error, Error::MutableLimit { .. }), "{error}");
    assert!(!target.exists());
    assert_eq!(restore_file_bytes(&staging), files);
    let admitted = rrd_store::DatabaseOptions {
        mutable_limits: rrd_store::MutableLimits {
            accounted_bytes: 8 * 1024 * 1024,
            versions: 8192,
        },
        ..Default::default()
    };
    let report = rrd_store::restore_logical_archive_to_new_root_with_options(
        &archive,
        &target,
        102,
        admitted,
        populate,
        |_| Ok(()),
    )
    .unwrap();
    assert!(report.resumed && report.reopened);
    let restored = RrflowKvStore::open_existing_with_options(&target, admitted).unwrap();
    assert_same_runtime(&source, &restored);
    assert_eq!(restored.control().get(key).unwrap(), Some(value));
    assert_eq!(restored.control().sequence().unwrap(), 1);
}

fn restore_file_bytes(root: &Path) -> std::collections::BTreeMap<PathBuf, Vec<u8>> {
    fn visit(root: &Path, at: &Path, files: &mut std::collections::BTreeMap<PathBuf, Vec<u8>>) {
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
