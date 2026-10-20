use super::*;
use crate::ServiceErrorKind;
use rrd_contract::{
    AssembleContext, ContextAccessPath, ContextEvidenceKind, ContextPlanStageKind,
    ContextPlanStageStatus, DataCatalogueIdentity, DataLogicalModel, DataReference, DataSchemaMode,
    DataSchemaRegistry, DataTableSchema, EmbeddingInput, EmbeddingNetworkPolicy,
    EnsureVectorCollection, GenerateEmbeddings, ListEmbeddingModels, NamedVectorDefinition,
    QueryValue, ReadAccessPath, VectorEmbeddingModel, VectorMemoryTier, VectorSearchMetric,
    VectorValueKind,
};
use std::collections::BTreeMap;

fn reference(kind: &str, id: &str) -> DataReference {
    DataReference {
        kind: CanonicalId::new(kind).unwrap(),
        id: CanonicalId::new(id).unwrap(),
    }
}

#[test]
fn context_flows_through_one_engine_stamp_and_survives_reopen() {
    let (root, engine) = isolated_engine();
    let lease = engine
        .create_session(
            &session_request(5_000, 2),
            &id("context-session"),
            1_000,
            "request-context-session",
            "operation-context-session",
        )
        .unwrap();
    let alpha = reference("note", "alpha");
    let beta = reference("note", "beta");
    let gamma = reference("note", "gamma");
    let relation_kind = CanonicalId::new("supports").unwrap();
    let mutations = vec![
        TransactionMutation::PutSchema {
            registry: DataSchemaRegistry {
                revision: 1,
                migration: "install context graph fixture".into(),
                catalogue: DataCatalogueIdentity::default(),
                tables: BTreeMap::from([
                    (
                        alpha.kind.clone(),
                        DataTableSchema {
                            model: DataLogicalModel::GraphNode,
                            mode: DataSchemaMode::Schemaless,
                            properties: BTreeMap::new(),
                            allow_additional_properties: false,
                        },
                    ),
                    (
                        relation_kind.clone(),
                        DataTableSchema {
                            model: DataLogicalModel::GraphRelation,
                            mode: DataSchemaMode::Schemaless,
                            properties: BTreeMap::new(),
                            allow_additional_properties: false,
                        },
                    ),
                ]),
                records: BTreeMap::new(),
                relations: BTreeMap::new(),
                events: BTreeMap::new(),
            },
        },
        TransactionMutation::PutRecord {
            reference: alpha.clone(),
            valid_from: 1_100,
            valid_to: None,
            properties: BTreeMap::from([
                (
                    "title".into(),
                    QueryValue::String("Authentication session recovery".into()),
                ),
                (
                    "body".into(),
                    QueryValue::String("Restore the durable session from its token.".into()),
                ),
            ]),
        },
        TransactionMutation::PutRecord {
            reference: beta.clone(),
            valid_from: 1_100,
            valid_to: None,
            properties: BTreeMap::from([(
                "body".into(),
                QueryValue::String("Rotate credentials only after recovery succeeds.".into()),
            )]),
        },
        TransactionMutation::PutRecord {
            reference: gamma,
            valid_from: 1_100,
            valid_to: None,
            properties: BTreeMap::from([(
                "body".into(),
                QueryValue::String("Unrelated rendering preferences.".into()),
            )]),
        },
        TransactionMutation::PutRelation {
            reference: reference("supports", "alpha-beta"),
            from: alpha,
            to: beta,
            valid_from: 1_100,
            valid_to: None,
            properties: BTreeMap::new(),
        },
    ];
    let transaction = engine
        .begin_transaction(
            &lease.session_id,
            &lease.token,
            &BeginTransaction {
                scope: CanonicalId::new("data").unwrap(),
                timeout_ms: 1_000,
            },
            &mutation_context(
                &id("context-begin"),
                "request-context-begin",
                "operation-context-begin",
            ),
            1_050,
        )
        .unwrap();
    let receipt = engine
        .commit_transaction(
            &lease.session_id,
            &lease.token,
            &transaction.transaction_id,
            &id("context-commit"),
            &CommitTransaction {
                operation_sha256: rrd_contract::transaction_operation_sha256(&mutations),
                mutations,
            },
            1_100,
            "request-context-commit",
            "operation-context-commit",
        )
        .unwrap();
    let request = AssembleContext {
        scope: "instance:test-instance".into(),
        query: "authentication session".into(),
        valid_at: 1_200,
        seeds: Vec::new(),
        max_graph_depth: 1,
        max_items: 8,
        max_output_bytes: 64 * 1024,
        max_storage_keys: 128,
    };
    let packet = engine
        .assemble_context(
            &lease.session_id,
            &lease.token,
            &request,
            1_200,
            "request-context",
            "operation-context",
        )
        .unwrap();
    packet.validate().unwrap();
    assert_eq!(packet.selected_versions, 5);
    assert!(packet
        .read_evidence
        .paths
        .iter()
        .any(|path| path.path == ReadAccessPath::RelationVersions));
    assert_eq!(packet.plan.security_policy_revision, 0);
    assert_eq!(
        packet.plan.authorization_sha256,
        digest::sha256_hex(b"rrd-security-disabled-loopback-development")
    );
    let mut tampered_plan = packet.plan.clone();
    tampered_plan.authorization_sha256 = "0".repeat(64);
    assert!(tampered_plan.validate().is_err());
    let stage = |kind| {
        packet
            .plan
            .stages
            .iter()
            .find(|stage| stage.kind == kind)
            .unwrap()
    };
    assert_eq!(
        stage(ContextPlanStageKind::Seed).status,
        ContextPlanStageStatus::Skipped
    );
    assert_eq!(
        stage(ContextPlanStageKind::Lexical).access_path,
        Some(ContextAccessPath::SnapshotBm25)
    );
    assert_eq!(
        stage(ContextPlanStageKind::Semantic).status,
        ContextPlanStageStatus::Skipped
    );
    assert_eq!(
        stage(ContextPlanStageKind::Graph).access_path,
        Some(ContextAccessPath::SnapshotGraphBidirectionalBfs)
    );
    assert_eq!(
        stage(ContextPlanStageKind::Fusion).access_path,
        Some(ContextAccessPath::ReciprocalRankFusion)
    );
    assert_eq!(
        packet.read.runtime_cursor,
        receipt.last_runtime_cursor.unwrap()
    );
    let alpha = packet
        .items
        .iter()
        .find(|item| item.identity == "record:note:alpha")
        .unwrap();
    assert!(alpha
        .evidence
        .iter()
        .any(|evidence| evidence.kind == ContextEvidenceKind::Text));
    let beta = packet
        .items
        .iter()
        .find(|item| item.identity == "record:note:beta")
        .unwrap();
    assert!(beta
        .evidence
        .iter()
        .any(|evidence| evidence.kind == ContextEvidenceKind::Graph));
    assert!(packet
        .items
        .iter()
        .all(|item| item.identity != "record:note:gamma"));

    // The same live root found by both a seed and lexical retrieval is
    // expanded once, and the plan reports the actual number of roots.
    let mut overlapping = request.clone();
    overlapping.seeds.push(reference("note", "alpha"));
    let overlapping = engine
        .assemble_context(
            &lease.session_id,
            &lease.token,
            &overlapping,
            1_200,
            "request-context-overlap",
            "operation-context-overlap",
        )
        .unwrap();
    let graph = overlapping
        .plan
        .stages
        .iter()
        .find(|stage| stage.kind == ContextPlanStageKind::Graph)
        .unwrap();
    assert!(graph.reason.ends_with("from 1 root(s)"));
    let neighbor = overlapping
        .items
        .iter()
        .find(|item| item.identity == "record:note:beta")
        .unwrap();
    assert_eq!(
        neighbor
            .evidence
            .iter()
            .filter(|evidence| evidence.kind == ContextEvidenceKind::Graph)
            .count(),
        1
    );

    let mut bounded_request = request.clone();
    bounded_request.max_items = 1;
    bounded_request.max_output_bytes = 4 * 1024;
    let bounded = engine
        .assemble_context(
            &lease.session_id,
            &lease.token,
            &bounded_request,
            1_200,
            "request-context-bounded",
            "operation-context-bounded",
        )
        .unwrap();
    assert_eq!(bounded.items.len(), 1);
    assert!(bounded.truncated);
    assert!(bounded.output_bytes <= bounded_request.max_output_bytes);

    let mut before_validity = request.clone();
    before_validity.valid_at = 1_050;
    let before_validity = engine
        .assemble_context(
            &lease.session_id,
            &lease.token,
            &before_validity,
            1_200,
            "request-context-before-validity",
            "operation-context-before-validity",
        )
        .unwrap();
    assert!(before_validity.items.is_empty());

    drop(engine);
    let reopened = RrdEngine::open(root.path(), instance(), TOKEN_KEY).unwrap();
    let reopened_packet = reopened
        .assemble_context(
            &lease.session_id,
            &lease.token,
            &request,
            1_201,
            "request-context-reopen",
            "operation-context-reopen",
        )
        .unwrap();
    assert_eq!(reopened_packet, packet);
}

#[test]
fn context_discovers_matching_local_vector_retrieval_without_caller_wiring() {
    let (_root, engine) = isolated_engine();
    let lease = engine
        .create_session(
            &session_request(5_000, 2),
            &id("vector-context-session"),
            1_000,
            "request-vector-context-session",
            "operation-vector-context-session",
        )
        .unwrap();
    engine.install_feature_hash_embedding(16, 7).unwrap();
    let catalogue = engine
        .list_embedding_models(
            &lease.session_id,
            &lease.token,
            &ListEmbeddingModels {
                scope: "instance:test-instance".into(),
            },
            1_010,
            "request-vector-context-models",
            "operation-vector-context-models",
        )
        .unwrap();
    let backend = catalogue.backends.into_iter().next().unwrap();
    let collection_id = CanonicalId::new("context-memory").unwrap();
    let vector_name = CanonicalId::new("semantic").unwrap();
    engine
        .ensure_vector_collection(
            &lease.session_id,
            &lease.token,
            &EnsureVectorCollection {
                scope: "instance:test-instance".into(),
                collection_id: collection_id.clone(),
                vectors: vec![NamedVectorDefinition {
                    name: vector_name.clone(),
                    field: CanonicalId::new("embedding").unwrap(),
                    kind: VectorValueKind::Dense,
                    dimensions: backend.dimensions,
                    metric: VectorSearchMetric::Cosine,
                    embedding_model: Some(VectorEmbeddingModel {
                        name: format!(
                            "{}/{}@{}",
                            backend.provider, backend.model, backend.revision
                        ),
                        digest: backend.model_sha256.clone(),
                    }),
                    memory_tier: VectorMemoryTier::Cached,
                }],
            },
            &mutation_context(
                &id("vector-context-collection"),
                "request-vector-context-collection",
                "operation-vector-context-collection",
            ),
            1_020,
        )
        .unwrap();

    let note = reference("note", "semantic-alpha");
    let embedding = reference("embedding", "semantic-alpha");
    let initial = vec![
        TransactionMutation::PutSchema {
            registry: DataSchemaRegistry {
                revision: 1,
                migration: "install semantic context fixture".into(),
                catalogue: DataCatalogueIdentity::default(),
                tables: BTreeMap::from([
                    (
                        note.kind.clone(),
                        DataTableSchema {
                            model: DataLogicalModel::GraphNode,
                            mode: DataSchemaMode::Schemaless,
                            properties: BTreeMap::new(),
                            allow_additional_properties: false,
                        },
                    ),
                    (
                        embedding.kind.clone(),
                        DataTableSchema {
                            model: DataLogicalModel::Vector,
                            mode: DataSchemaMode::Schemaless,
                            properties: BTreeMap::new(),
                            allow_additional_properties: false,
                        },
                    ),
                ]),
                records: BTreeMap::new(),
                relations: BTreeMap::new(),
                events: BTreeMap::new(),
            },
        },
        TransactionMutation::PutRecord {
            reference: note.clone(),
            valid_from: 1_100,
            valid_to: None,
            properties: BTreeMap::from([(
                "body".into(),
                QueryValue::String("Recover durable authentication state".into()),
            )]),
        },
    ];
    let transaction = engine
        .begin_transaction(
            &lease.session_id,
            &lease.token,
            &BeginTransaction {
                scope: CanonicalId::new("data").unwrap(),
                timeout_ms: 1_000,
            },
            &mutation_context(
                &id("vector-context-begin-record"),
                "request-vector-context-begin-record",
                "operation-vector-context-begin-record",
            ),
            1_050,
        )
        .unwrap();
    engine
        .commit_transaction(
            &lease.session_id,
            &lease.token,
            &transaction.transaction_id,
            &id("vector-context-commit-record"),
            &CommitTransaction {
                operation_sha256: rrd_contract::transaction_operation_sha256(&initial),
                mutations: initial,
            },
            1_100,
            "request-vector-context-commit-record",
            "operation-vector-context-commit-record",
        )
        .unwrap();
    let generated = engine
        .generate_embeddings(
            &lease.session_id,
            &lease.token,
            &GenerateEmbeddings {
                scope: "instance:test-instance".into(),
                backend_id: backend.id,
                network_policy: EmbeddingNetworkPolicy::Deny,
                inputs: vec![EmbeddingInput {
                    id: CanonicalId::new("semantic-alpha").unwrap(),
                    media_type: "text/plain; charset=utf-8".into(),
                    bytes: b"Recover durable authentication state".to_vec(),
                }],
            },
            1_110,
            "request-vector-context-embedding",
            "operation-vector-context-embedding",
        )
        .unwrap()
        .embeddings
        .into_iter()
        .next()
        .unwrap();
    let vector_mutations = vec![TransactionMutation::PutVector {
        reference: embedding,
        subject: note,
        collection_id,
        vector_name,
        field: CanonicalId::new("embedding").unwrap(),
        valid_from: 1_120,
        valid_to: None,
        value: generated.value,
        provenance: Some(generated.provenance),
        properties: BTreeMap::new(),
    }];
    let transaction = engine
        .begin_transaction(
            &lease.session_id,
            &lease.token,
            &BeginTransaction {
                scope: CanonicalId::new("data").unwrap(),
                timeout_ms: 1_000,
            },
            &mutation_context(
                &id("vector-context-begin-vector"),
                "request-vector-context-begin-vector",
                "operation-vector-context-begin-vector",
            ),
            1_115,
        )
        .unwrap();
    engine
        .commit_transaction(
            &lease.session_id,
            &lease.token,
            &transaction.transaction_id,
            &id("vector-context-commit-vector"),
            &CommitTransaction {
                operation_sha256: rrd_contract::transaction_operation_sha256(&vector_mutations),
                mutations: vector_mutations,
            },
            1_120,
            "request-vector-context-commit-vector",
            "operation-vector-context-commit-vector",
        )
        .unwrap();

    let packet = engine
        .assemble_context(
            &lease.session_id,
            &lease.token,
            &AssembleContext {
                scope: "instance:test-instance".into(),
                query: "authentication recovery".into(),
                valid_at: 1_200,
                seeds: Vec::new(),
                max_graph_depth: 1,
                max_items: 8,
                max_output_bytes: 64 * 1024,
                max_storage_keys: 128,
            },
            1_200,
            "request-vector-context",
            "operation-vector-context",
        )
        .unwrap();
    let item = packet
        .items
        .iter()
        .find(|item| item.identity == "record:note:semantic-alpha")
        .unwrap();
    assert_eq!(
        packet
            .plan
            .stages
            .iter()
            .find(|stage| stage.kind == ContextPlanStageKind::Semantic)
            .unwrap()
            .access_path,
        Some(ContextAccessPath::SnapshotVectorExact)
    );
    assert!(item
        .evidence
        .iter()
        .any(|evidence| evidence.kind == ContextEvidenceKind::Vector));
}

#[test]
fn context_reads_active_claims_from_the_same_runtime_snapshot() {
    let (_root, engine) = isolated_engine();
    let lease = engine
        .create_session(
            &session_request(5_000, 2),
            &id("claim-context-session"),
            1_000,
            "request-claim-context-session",
            "operation-claim-context-session",
        )
        .unwrap();
    let transaction = engine
        .begin_transaction(
            &lease.session_id,
            &lease.token,
            &BeginTransaction {
                scope: CanonicalId::new("data").unwrap(),
                timeout_ms: 1_000,
            },
            &mutation_context(
                &id("claim-context-begin"),
                "request-claim-context-begin",
                "operation-claim-context-begin",
            ),
            1_050,
        )
        .unwrap();
    let mutations = vec![
        TransactionMutation::PutSchema {
            registry: DataSchemaRegistry {
                revision: 1,
                migration: "install claim context fixture".into(),
                catalogue: DataCatalogueIdentity::default(),
                tables: BTreeMap::from([(
                    CanonicalId::new("claim").unwrap(),
                    DataTableSchema {
                        model: DataLogicalModel::ReasoningClaim,
                        mode: DataSchemaMode::Schemaless,
                        properties: BTreeMap::new(),
                        allow_additional_properties: false,
                    },
                )]),
                records: BTreeMap::new(),
                relations: BTreeMap::new(),
                events: BTreeMap::new(),
            },
        },
        TransactionMutation::AssertClaim {
            subject: CanonicalId::new("project-alpha").unwrap(),
            predicate: CanonicalId::new("status").unwrap(),
            object: "Durable authentication recovery is operational".into(),
            valid_from: 1_100,
            tx_time: 1_100,
            producer: CanonicalId::new("agent-test").unwrap(),
            confidence: Some(0.9),
        },
    ];
    let receipt = engine
        .commit_transaction(
            &lease.session_id,
            &lease.token,
            &transaction.transaction_id,
            &id("claim-context-commit"),
            &CommitTransaction {
                operation_sha256: rrd_contract::transaction_operation_sha256(&mutations),
                mutations,
            },
            1_100,
            "request-claim-context-commit",
            "operation-claim-context-commit",
        )
        .unwrap();
    let packet = engine
        .assemble_context(
            &lease.session_id,
            &lease.token,
            &AssembleContext {
                scope: "instance:test-instance".into(),
                query: "authentication recovery".into(),
                valid_at: 1_200,
                seeds: Vec::new(),
                max_graph_depth: 1,
                max_items: 8,
                max_output_bytes: 64 * 1024,
                max_storage_keys: 128,
            },
            1_200,
            "request-claim-context",
            "operation-claim-context",
        )
        .unwrap();
    assert_eq!(
        packet.read.runtime_cursor,
        receipt.last_runtime_cursor.unwrap()
    );
    let item = packet
        .items
        .iter()
        .find(|item| {
            item.values.get("subject") == Some(&QueryValue::String("project-alpha".into()))
        })
        .unwrap();
    assert!(item.identity.starts_with("claim:"));
    assert!(item
        .evidence
        .iter()
        .any(|evidence| evidence.kind == ContextEvidenceKind::Text));
}

#[test]
fn context_rejects_recall_work_above_the_effective_estate_ceiling() {
    let (_root, engine) = isolated_engine();
    let request = AssembleContext {
        scope: "instance:test-instance".into(),
        query: "bounded recall".into(),
        valid_at: 1,
        seeds: Vec::new(),
        max_graph_depth: engine.estate_configuration().recall.max_graph_depth,
        max_items: engine.estate_configuration().recall.max_items + 1,
        max_output_bytes: engine.estate_configuration().recall.max_output_bytes,
        max_storage_keys: engine.estate_configuration().recall.max_storage_keys,
    };
    let error = engine.assemble_context_at(&request, None).unwrap_err();
    assert_eq!(error.kind(), ServiceErrorKind::ResourceExhausted);
    assert!(matches!(error, ServiceError::ConfigurationLimit(_)));
}

/// A visible relation must not resurrect an invisible requested anchor.
/// Use the public engine methods so selection, plan evidence and storage
/// temporal filtering are exercised together.
#[test]
fn context_graph_anchors_require_visible_records_in_memory_and_after_reopen() {
    fn commit(
        engine: &RrdEngine,
        lease: &rrd_contract::SessionLease,
        key: &str,
        mutations: Vec<TransactionMutation>,
    ) {
        let transaction = engine
            .begin_transaction(
                &lease.session_id,
                &lease.token,
                &BeginTransaction {
                    scope: CanonicalId::new("data").unwrap(),
                    timeout_ms: 1_000,
                },
                &mutation_context(&id(key), "anchor-begin", "anchor-begin"),
                1_050,
            )
            .unwrap();
        engine
            .commit_transaction(
                &lease.session_id,
                &lease.token,
                &transaction.transaction_id,
                &id(key),
                &CommitTransaction {
                    operation_sha256: rrd_contract::transaction_operation_sha256(&mutations),
                    mutations,
                },
                1_100,
                "anchor-commit",
                "anchor-commit",
            )
            .unwrap();
    }

    fn verify(
        engine: &RrdEngine,
        lease: &rrd_contract::SessionLease,
    ) -> Vec<rrd_contract::ContextPacket> {
        let mut packets = Vec::new();
        for (seed, valid_at, visible) in [
            ("expired", 1_300, false),
            ("future", 1_300, false),
            ("retired", 1_300, false),
            ("missing", 1_300, false),
            ("expired", 1_150, true),
            ("future", 1_450, true),
            ("retired", 1_150, true),
        ] {
            for mixed in [false, true] {
                let mut seeds = vec![reference("note", seed)];
                if mixed {
                    seeds.push(reference("note", "live"));
                }
                let request = AssembleContext {
                    scope: "instance:test-instance".into(),
                    query: String::new(),
                    valid_at,
                    seeds,
                    max_graph_depth: 1,
                    max_items: 8,
                    max_output_bytes: 64 * 1024,
                    max_storage_keys: 256,
                };
                let packet = engine
                    .assemble_context(
                        &lease.session_id,
                        &lease.token,
                        &request,
                        1_500,
                        "anchor-read",
                        "anchor-read",
                    )
                    .unwrap();
                packet.validate().unwrap();
                let identities = packet
                    .items
                    .iter()
                    .map(|item| item.identity.clone())
                    .collect::<std::collections::BTreeSet<_>>();
                let mut expected = std::collections::BTreeSet::new();
                if visible {
                    expected.insert(format!("record:note:{seed}"));
                    expected.insert(format!("record:note:{seed}-leaf"));
                }
                if mixed {
                    expected.insert("record:note:live".to_owned());
                    expected.insert("record:note:live-leaf".to_owned());
                }
                assert_eq!(
                    identities, expected,
                    "seed={seed}, at={valid_at}, mixed={mixed}"
                );
                let graph = packet
                    .plan
                    .stages
                    .iter()
                    .find(|stage| stage.kind == ContextPlanStageKind::Graph)
                    .unwrap();
                assert_eq!(
                    graph.status,
                    if visible || mixed {
                        ContextPlanStageStatus::Selected
                    } else {
                        ContextPlanStageStatus::Skipped
                    }
                );
                assert!(!packet.truncated);
                packets.push(packet);
            }
        }
        packets
    }

    for persistent in [true, false] {
        let root = tempfile::tempdir().unwrap();
        let engine = if persistent {
            RrdEngine::open(root.path(), instance(), TOKEN_KEY).unwrap()
        } else {
            RrdEngine::rrflow_mx(instance(), TOKEN_KEY)
        };
        let lease = engine
            .create_session(
                &session_request(5_000, 2),
                &id("anchor-session"),
                1_000,
                "anchor-session",
                "anchor-session",
            )
            .unwrap();
        let mut mutations = vec![TransactionMutation::PutSchema {
            registry: DataSchemaRegistry {
                revision: 1,
                migration: "temporal context anchors".into(),
                catalogue: DataCatalogueIdentity::default(),
                tables: [
                    ("note", DataLogicalModel::GraphNode),
                    ("supports", DataLogicalModel::GraphRelation),
                ]
                .into_iter()
                .map(|(name, model)| {
                    (
                        CanonicalId::new(name).unwrap(),
                        DataTableSchema {
                            model,
                            mode: DataSchemaMode::Schemaless,
                            properties: BTreeMap::new(),
                            allow_additional_properties: false,
                        },
                    )
                })
                .collect(),
                records: BTreeMap::new(),
                relations: BTreeMap::new(),
                events: BTreeMap::new(),
            },
        }];
        for (name, valid_from, valid_to) in [
            ("live", 1_100, None),
            ("expired", 1_100, Some(1_200)),
            ("future", 1_400, None),
            ("retired", 1_100, None),
        ] {
            mutations.push(TransactionMutation::PutRecord {
                reference: reference("note", name),
                valid_from,
                valid_to,
                properties: BTreeMap::new(),
            });
            mutations.push(TransactionMutation::PutRecord {
                reference: reference("note", &format!("{name}-leaf")),
                valid_from: 1_100,
                valid_to: None,
                properties: BTreeMap::new(),
            });
            mutations.push(TransactionMutation::PutRelation {
                reference: reference("supports", name),
                from: reference("note", name),
                to: reference("note", &format!("{name}-leaf")),
                valid_from: 1_100,
                valid_to: None,
                properties: BTreeMap::new(),
            });
        }
        commit(&engine, &lease, "anchor-seed", mutations);
        commit(
            &engine,
            &lease,
            "anchor-retire",
            vec![TransactionMutation::RetireData {
                model: DataLogicalModel::GraphNode,
                target: rrd_contract::DataTarget::Reference {
                    reference: reference("note", "retired"),
                },
                effective_at: 1_250,
            }],
        );
        let packets = verify(&engine, &lease);
        if persistent {
            drop(engine);
            let reopened = RrdEngine::open(root.path(), instance(), TOKEN_KEY).unwrap();
            assert_eq!(verify(&reopened, &lease), packets);
        }
    }
}
