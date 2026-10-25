use std::collections::{BTreeMap, BTreeSet};

use rrd_core::{
    Claim, GeoPoint, GeoValue, ObjectReceipt, Predicate, Producer, RuntimeChange, RuntimeCommit,
    RuntimeDataSnapshot, RuntimeEvent, RuntimeEventSchema, RuntimeGeo, RuntimeLogicalModel,
    RuntimeMutation, RuntimeProperties, RuntimeRecord, RuntimeRecordSchema, RuntimeRef,
    RuntimeRelation, RuntimeRelationSchema, RuntimeRetirement, RuntimeSchemaRegistry,
    RuntimeSeriesSample, RuntimeTableSchema, RuntimeType, RuntimeVector, ScopeId, SeriesValue,
    Subject, VectorCollectionAddress, VectorValue,
};
use rrd_store::{
    Error, RrflowKvStore, RrflowMxStore, RuntimeReadAccessPath, RuntimeReadBudget,
    RuntimeReadEvidence, RuntimeVersionedSource, StorageEngine,
};

const READ_BUDGET: u64 = 10_000;

fn scope() -> ScopeId {
    ScopeId::new("project:direct-read-proof").unwrap()
}

fn reference(kind: &str, id: &str) -> RuntimeRef {
    RuntimeRef::new(kind, id).unwrap()
}

fn schema() -> RuntimeSchemaRegistry {
    let entity = RuntimeType::new("entity").unwrap();
    let mut registry = RuntimeSchemaRegistry::empty(1, "direct versioned-read proof");
    registry
        .define_record_table(
            entity.clone(),
            RuntimeLogicalModel::Relational,
            RuntimeRecordSchema::default(),
        )
        .unwrap();
    registry
        .define_relation_table(
            RuntimeType::new("links").unwrap(),
            RuntimeRelationSchema {
                from: BTreeSet::from([entity.clone()]),
                to: BTreeSet::from([entity.clone()]),
                ..RuntimeRelationSchema::default()
            },
        )
        .unwrap();
    registry
        .define_event_table(
            RuntimeType::new("observed").unwrap(),
            RuntimeLogicalModel::Event,
            RuntimeEventSchema {
                subject_required: true,
                subject_types: BTreeSet::from([entity.clone()]),
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
        registry.tables.insert(
            RuntimeType::new(kind).unwrap(),
            RuntimeTableSchema::schemaless(model),
        );
    }
    registry
}

fn record(id: &str) -> RuntimeRecord {
    RuntimeRecord {
        reference: reference("entity", id),
        valid_from: 100,
        valid_to: None,
        properties: RuntimeProperties::new(),
    }
}

fn initial_mutations() -> Vec<RuntimeMutation> {
    let entity = reference("entity", "a");
    let object_sha256 = rrd_core::digest::sha256_hex(b"direct-read fixture");
    let object = rrd_core::ObjectReference::for_bytes(
        "source-a",
        Some(entity.clone()),
        "text/plain",
        b"direct-read fixture",
        ObjectReceipt {
            backend: "fixture".into(),
            key: rrd_core::ObjectReference::canonical_key(&object_sha256).unwrap(),
            version: Some("v1".into()),
            etag: None,
        },
    )
    .unwrap();
    vec![
        RuntimeMutation::Schema { registry: schema() },
        RuntimeMutation::Record {
            record: record("a"),
        },
        RuntimeMutation::Record {
            record: record("b"),
        },
        RuntimeMutation::Claim {
            claim: Claim::new(
                Subject::new("entity:a").unwrap(),
                Predicate::new("status").unwrap(),
                "ready",
                100,
                100,
                Producer {
                    actor: "test:direct-read".into(),
                    on_behalf_of: None,
                    session: Some("direct-read".into()),
                },
            ),
        },
        RuntimeMutation::Relation {
            relation: RuntimeRelation {
                reference: reference("links", "a-b"),
                from: entity.clone(),
                to: reference("entity", "b"),
                valid_from: 100,
                valid_to: None,
                properties: RuntimeProperties::new(),
            },
        },
        RuntimeMutation::Event {
            event: RuntimeEvent {
                kind: RuntimeType::new("observed").unwrap(),
                subject: Some(entity.clone()),
                properties: RuntimeProperties::new(),
            },
        },
        RuntimeMutation::Vector {
            vector: RuntimeVector {
                reference: reference("embedding", "a-title"),
                subject: entity.clone(),
                collection: VectorCollectionAddress {
                    collection_id: "documents".into(),
                    vector_name: "title".into(),
                },
                field: "title".into(),
                valid_from: 100,
                valid_to: None,
                value: VectorValue::Dense {
                    values: vec![0.6, 0.8],
                },
                provenance: None,
                properties: RuntimeProperties::new(),
            },
        },
        RuntimeMutation::SeriesSample {
            sample: RuntimeSeriesSample {
                reference: reference("sample", "a-temperature"),
                series: entity.clone(),
                observed_at: 100,
                value: SeriesValue::Decimal("21.5".into()),
                properties: RuntimeProperties::new(),
            },
        },
        RuntimeMutation::Geo {
            geo: RuntimeGeo {
                reference: reference("location", "a-current"),
                subject: entity,
                field: "position".into(),
                valid_from: 100,
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

fn retirements(event_cursor: u64) -> Vec<RuntimeMutation> {
    [
        (RuntimeLogicalModel::Relational, reference("entity", "a")),
        (
            RuntimeLogicalModel::GraphRelation,
            reference("links", "a-b"),
        ),
        (
            RuntimeLogicalModel::Event,
            reference("observed", &format!("cursor:{event_cursor}")),
        ),
        (
            RuntimeLogicalModel::Vector,
            reference("embedding", "a-title"),
        ),
        (
            RuntimeLogicalModel::TimeSeries,
            reference("sample", "a-temperature"),
        ),
        (RuntimeLogicalModel::Geo, reference("location", "a-current")),
        (RuntimeLogicalModel::Object, reference("object", "source-a")),
    ]
    .into_iter()
    .map(|(model, reference)| RuntimeMutation::Retire {
        retirement: RuntimeRetirement {
            model,
            reference,
            effective_at: 200,
        },
    })
    .collect()
}

#[derive(Clone)]
struct Corpus {
    retained: rrd_core::ReadStamp,
    current: rrd_core::ReadStamp,
    retained_changes: Vec<RuntimeChange>,
    current_changes: Vec<RuntimeChange>,
    retained_snapshot: RuntimeDataSnapshot,
    current_snapshot: RuntimeDataSnapshot,
}

fn commit_corpus(engine: &dyn StorageEngine) -> Corpus {
    let scope = scope();
    let first = engine
        .runtime()
        .commit(&RuntimeCommit {
            scope: scope.clone(),
            at: 100,
            actor: "test:direct-read".into(),
            expected_cursor: 0,
            mutations: initial_mutations(),
        })
        .unwrap();
    assert_eq!(first.last_cursor, 10);
    let retained = engine.runtime().read_stamp(&scope).unwrap();

    let noise_scope = ScopeId::new("project:direct-read-noise").unwrap();
    let noise = engine
        .runtime()
        .commit(&RuntimeCommit {
            scope: noise_scope,
            at: 150,
            actor: "test:direct-read-noise".into(),
            expected_cursor: first.last_cursor,
            mutations: vec![
                RuntimeMutation::Schema { registry: schema() },
                RuntimeMutation::Record {
                    record: record("noise"),
                },
            ],
        })
        .unwrap();
    let mut revised_schema = schema();
    revised_schema.revision = 2;
    revised_schema.migration = "retain direct version-key history".into();
    let mut final_mutations = vec![RuntimeMutation::Schema {
        registry: revised_schema,
    }];
    final_mutations.extend(retirements(6));
    let final_outcome = engine
        .runtime()
        .commit(&RuntimeCommit {
            scope: scope.clone(),
            at: 200,
            actor: "test:direct-read".into(),
            expected_cursor: noise.last_cursor,
            mutations: final_mutations,
        })
        .unwrap();
    assert_eq!(final_outcome.last_cursor, 20);
    let current = engine.runtime().read_stamp(&scope).unwrap();

    let retained_changes = oracle_changes(engine, &retained);
    let current_changes = oracle_changes(engine, &current);
    assert_eq!(retained_changes.len(), 10);
    assert_eq!(current_changes.len(), 18);
    let retained_snapshot = snapshot(&retained, &retained_changes, 150);
    let current_snapshot = snapshot(&current, &current_changes, 250);
    Corpus {
        retained,
        current,
        retained_changes,
        current_changes,
        retained_snapshot,
        current_snapshot,
    }
}

fn oracle_changes(engine: &dyn StorageEngine, read: &rrd_core::ReadStamp) -> Vec<RuntimeChange> {
    let page = engine.runtime().read_changes(read, 0, usize::MAX).unwrap();
    assert_eq!(page.through_cursor, read.commit_cursor);
    assert!(!page.has_more());
    page.changes
}

fn snapshot(
    read: &rrd_core::ReadStamp,
    changes: &[RuntimeChange],
    valid_at: u64,
) -> RuntimeDataSnapshot {
    let schema = changes
        .iter()
        .filter_map(|change| match &change.mutation {
            RuntimeMutation::Schema { registry } => Some(registry),
            _ => None,
        })
        .next_back()
        .unwrap();
    RuntimeDataSnapshot::from_changes(
        changes,
        schema,
        read.scope.clone(),
        valid_at,
        read.commit_cursor,
    )
    .unwrap()
}

fn assert_direct_reads(engine: &dyn StorageEngine, corpus: &Corpus) -> Vec<RuntimeReadEvidence> {
    let retained_validation = engine
        .runtime()
        .validate_read_stamp(&corpus.retained)
        .unwrap();
    assert_eq!(retained_validation.method, "rfc9162_direct_versions");
    assert!(retained_validation.change_reads > 0);
    assert!(retained_validation.proof_nodes > 0);
    let current_validation = engine
        .runtime()
        .validate_read_stamp(&corpus.current)
        .unwrap();
    assert_eq!(current_validation.method, "authenticated_current_head");
    assert_eq!(current_validation.change_reads, 0);
    assert_eq!(current_validation.proof_nodes, 0);

    let sources = RuntimeVersionedSource::all();
    let retained = engine
        .runtime()
        .read_versioned(
            &corpus.retained,
            &sources,
            RuntimeReadBudget::new(READ_BUDGET).unwrap(),
        )
        .unwrap();
    let current = engine
        .runtime()
        .read_versioned(
            &corpus.current,
            &sources,
            RuntimeReadBudget::new(READ_BUDGET).unwrap(),
        )
        .unwrap();
    assert_eq!(retained.changes, corpus.retained_changes);
    assert_eq!(current.changes, corpus.current_changes);
    assert_eq!(
        snapshot(&corpus.retained, &retained.changes, 150),
        corpus.retained_snapshot
    );
    assert_eq!(
        snapshot(&corpus.current, &current.changes, 250),
        corpus.current_snapshot
    );
    let repository_read = engine
        .runtime()
        .data_snapshot(
            &corpus.current.scope,
            250,
            usize::try_from(READ_BUDGET).unwrap(),
        )
        .unwrap();
    assert_eq!(repository_read.read, corpus.current);
    assert_eq!(repository_read.snapshot, corpus.current_snapshot);
    assert_eq!(
        repository_read.selected_versions,
        u64::try_from(corpus.current_changes.len()).unwrap()
    );
    assert_eq!(repository_read.read_evidence, current.evidence);
    assert_eq!(
        (
            retained.evidence.point_reads,
            retained.evidence.range_scans,
            retained.evidence.keys_examined,
            retained.evidence.values_decoded,
            retained.evidence.decoded_bytes,
        ),
        (29, 10, 48, 48, 15_056),
    );
    assert_eq!(
        (
            current.evidence.point_reads,
            current.evidence.range_scans,
            current.evidence.keys_examined,
            current.evidence.values_decoded,
            current.evidence.decoded_bytes,
        ),
        (40, 9, 58, 58, 14_938),
    );
    for evidence in [&retained.evidence, &current.evidence] {
        evidence.validate().unwrap();
        assert!(evidence.keys_examined <= evidence.key_budget);
        for path in [
            RuntimeReadAccessPath::SchemaVersions,
            RuntimeReadAccessPath::ClaimVersions,
            RuntimeReadAccessPath::RecordVersions,
            RuntimeReadAccessPath::RelationVersions,
            RuntimeReadAccessPath::EventVersions,
            RuntimeReadAccessPath::VectorVersions,
            RuntimeReadAccessPath::SeriesVersions,
            RuntimeReadAccessPath::GeoVersions,
            RuntimeReadAccessPath::ObjectVersions,
        ] {
            assert_eq!(
                evidence
                    .paths
                    .iter()
                    .find(|entry| entry.path == path)
                    .unwrap()
                    .range_scans,
                1
            );
        }
        assert!(evidence
            .paths
            .iter()
            .any(|entry| entry.path == RuntimeReadAccessPath::AccumulatorProof));
    }

    let records = engine
        .runtime()
        .read_versioned(
            &corpus.current,
            &[RuntimeVersionedSource::Records {
                kind: Some(RuntimeType::new("entity").unwrap()),
            }],
            RuntimeReadBudget::new(READ_BUDGET).unwrap(),
        )
        .unwrap();
    assert_eq!(records.changes.len(), 3);
    assert!(records.changes.iter().all(|change| matches!(
        change.mutation,
        RuntimeMutation::Record { .. } | RuntimeMutation::Retire { .. }
    )));

    let identity = engine
        .runtime()
        .read_versioned(
            &corpus.current,
            &[RuntimeVersionedSource::Identity {
                model: RuntimeLogicalModel::Relational,
                reference: reference("entity", "a"),
            }],
            RuntimeReadBudget::new(READ_BUDGET).unwrap(),
        )
        .unwrap();
    assert_eq!(identity.changes.len(), 2);
    assert!(identity
        .changes
        .iter()
        .all(|change| match &change.mutation {
            RuntimeMutation::Record { record } => record.reference == reference("entity", "a"),
            RuntimeMutation::Retire { retirement } => {
                retirement.reference == reference("entity", "a")
            }
            _ => false,
        }));

    assert!(matches!(
        engine.runtime().read_versioned(
            &corpus.current,
            &sources,
            RuntimeReadBudget::new(1).unwrap(),
        ),
        Err(Error::RuntimeReadBudgetExceeded {
            limit: 1,
            observed: 2
        })
    ));
    assert!(matches!(
        engine.runtime().read_versioned(
            &corpus.current,
            &sources,
            RuntimeReadBudget::new(23).unwrap(),
        ),
        Err(Error::RuntimeReadBudgetExceeded {
            limit: 23,
            observed: 24
        })
    ));
    vec![retained.evidence, current.evidence]
}

#[test]
fn rrflow_mx_and_rrflow_kv_direct_versions_match_the_exact_oracle_and_reopen() {
    let mx = RrflowMxStore::new();
    let mx_corpus = commit_corpus(&mx);
    let mx_evidence = assert_direct_reads(&mx, &mx_corpus);

    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("rrflow-kv");
    let kv_corpus = {
        let kv = RrflowKvStore::open(&path).unwrap();
        let corpus = commit_corpus(&kv);
        kv.flush(300).unwrap();
        corpus
    };
    // Opening reconciles snapshot checkpoints and can legitimately warm every
    // page in this compact fixture. Disable retention so the direct-read
    // assertion measures the read itself rather than a prior cache fill.
    let reopened = RrflowKvStore::open_with_options(
        &path,
        rrd_lsm::DatabaseOptions {
            page_cache_bytes: 0,
            ..rrd_lsm::DatabaseOptions::default()
        },
    )
    .unwrap();
    let before = reopened.physical_store_evidence().unwrap();
    let kv_evidence = assert_direct_reads(&reopened, &kv_corpus);
    let after = reopened.physical_store_evidence().unwrap();

    assert_eq!(mx_evidence, kv_evidence);
    assert!(after.page_loads.unwrap() > before.page_loads.unwrap());
    assert!(after.page_bytes_read.unwrap() > before.page_bytes_read.unwrap());
    assert!(after.filter_checks.unwrap() > before.filter_checks.unwrap());
}

#[test]
fn adjacent_versions_reuse_proof_nodes_without_changing_the_authenticated_result() {
    fn check(engine: &dyn StorageEngine, read: &rrd_core::ReadStamp) {
        let sources = [RuntimeVersionedSource::Records {
            kind: Some(RuntimeType::new("entity").unwrap()),
        }];
        let selected = engine
            .runtime()
            .read_versioned(read, &sources, RuntimeReadBudget::new(READ_BUDGET).unwrap())
            .unwrap();
        let expected = oracle_changes(engine, read)
            .into_iter()
            .filter(|change| matches!(change.mutation, RuntimeMutation::Record { .. }))
            .collect::<Vec<_>>();
        assert_eq!(selected.changes, expected);
        assert_eq!(selected.changes.len(), 32);
        let proofs = selected
            .evidence
            .paths
            .iter()
            .find(|path| path.path == RuntimeReadAccessPath::AccumulatorProof)
            .unwrap();
        eprintln!("adjacent proof backing reads={}", proofs.point_reads);
        assert!(
            proofs.point_reads < 96,
            "repeated proof nodes were reread: {}",
            proofs.point_reads
        );
        // A new call owns a fresh cache and must account for the same work.
        let exact = selected.evidence.keys_examined;
        let repeated = engine
            .runtime()
            .read_versioned(read, &sources, RuntimeReadBudget::new(exact).unwrap())
            .unwrap();
        assert_eq!(selected.changes, repeated.changes);
        assert_eq!(
            selected.evidence.keys_examined,
            repeated.evidence.keys_examined
        );
        assert!(matches!(
            engine.runtime().read_versioned(
                read,
                &sources,
                RuntimeReadBudget::new(exact - 1).unwrap(),
            ),
            Err(Error::RuntimeReadBudgetExceeded { .. })
        ));
    }
    fn seed(engine: &dyn StorageEngine) -> rrd_core::ReadStamp {
        let mut mutations = vec![RuntimeMutation::Schema { registry: schema() }];
        mutations.extend((0..32).map(|n| RuntimeMutation::Record {
            record: record(&format!("proof-{n:04}")),
        }));
        engine
            .runtime()
            .commit(&RuntimeCommit {
                scope: scope(),
                at: 100,
                actor: "test:proof-reuse".into(),
                expected_cursor: 0,
                mutations,
            })
            .unwrap();
        engine.runtime().read_stamp(&scope()).unwrap()
    }
    let mx = RrflowMxStore::new();
    check(&mx, &seed(&mx));
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("proof-node-reuse");
    let read = {
        let kv = RrflowKvStore::open(&path).unwrap();
        let read = seed(&kv);
        check(&kv, &read);
        kv.flush(200).unwrap();
        read
    };
    let reopened = RrflowKvStore::open(&path).unwrap();
    check(&reopened, &read);
}
