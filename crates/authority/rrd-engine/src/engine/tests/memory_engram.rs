use super::*;
use rrd_contract::{
    DataReference, MemorySeatDefinition, PersistMemoryEstate, ProviderRepresentationDefinition,
    ResolveMemoryWarp, ResolveSeatIdentity, MEMORY_PROVIDER_IDENTITY_KIND, MEMORY_REPRESENTS_KIND,
};

fn memory_engram(
    provider_identity: Option<&str>,
    representation: Option<&str>,
    provider: Option<&str>,
    valid_from: u64,
) -> PersistMemoryEstate {
    let representations = match (provider_identity, representation, provider) {
        (Some(provider_identity), Some(representation), Some(provider)) => {
            vec![ProviderRepresentationDefinition {
                id: CanonicalId::new(representation).unwrap(),
                provider_identity: CanonicalId::new(provider_identity).unwrap(),
                provider: CanonicalId::new(provider).unwrap(),
                subject_sha256: digest::sha256_hex(
                    format!("{provider}:{provider_identity}").as_bytes(),
                ),
            }]
        }
        (None, None, None) => Vec::new(),
        _ => panic!("provider fixture must be complete"),
    };
    PersistMemoryEstate {
        scope: "instance:test-instance".into(),
        seat: MemorySeatDefinition {
            id: CanonicalId::new("clyffy").unwrap(),
            display_name: "Clyffy".into(),
            purpose: "Persistent RRFlow reasoning and recall identity".into(),
        },
        representations,
        valid_from,
    }
}

fn commit_engram_plan(
    engine: &RrdEngine,
    lease: &rrd_contract::SessionLease,
    request: &PersistMemoryEstate,
    at: u64,
    suffix: &str,
) {
    let plan = engine
        .plan_memory_engram(
            &lease.session_id,
            &lease.token,
            request,
            at,
            &format!("request-plan-{suffix}"),
            &format!("operation-plan-{suffix}"),
        )
        .unwrap();
    plan.validate().unwrap();
    let transaction = engine
        .begin_transaction(
            &lease.session_id,
            &lease.token,
            &BeginTransaction {
                scope: CanonicalId::new("data").unwrap(),
                timeout_ms: 1_000,
            },
            &mutation_context(
                &id(&format!("begin-{suffix}")),
                &format!("request-begin-{suffix}"),
                &format!("operation-begin-{suffix}"),
            ),
            at,
        )
        .unwrap();
    engine
        .commit_transaction(
            &lease.session_id,
            &lease.token,
            &transaction.transaction_id,
            &id(&format!("commit-{suffix}")),
            &CommitTransaction {
                operation_sha256: plan.operation_sha256,
                mutations: plan.mutations,
            },
            at,
            &format!("request-commit-{suffix}"),
            &format!("operation-commit-{suffix}"),
        )
        .unwrap();
}

#[test]
fn seat_becomes_self_through_provider_neutral_edges_and_survives_reopen() {
    let (root, engine) = isolated_engine();
    let lease = engine
        .create_session(
            &session_request(10_000, 8),
            &id("memory-engram-session"),
            1_000,
            "request-memory-engram-session",
            "operation-memory-engram-session",
        )
        .unwrap();
    let resolve = |engine: &RrdEngine, valid_at, suffix: &str| {
        engine.resolve_seat_identity(
            &lease.session_id,
            &lease.token,
            &ResolveSeatIdentity {
                scope: "instance:test-instance".into(),
                seat_id: CanonicalId::new("clyffy").unwrap(),
                valid_at,
                max_storage_keys: 256,
            },
            valid_at,
            &format!("request-resolve-{suffix}"),
            &format!("operation-resolve-{suffix}"),
        )
    };

    commit_engram_plan(
        &engine,
        &lease,
        &memory_engram(None, None, None, 1_100),
        1_100,
        "seat",
    );
    assert!(matches!(
        resolve(&engine, 1_150, "unrepresented"),
        Err(ServiceError::SeatNotRepresented)
    ));

    commit_engram_plan(
        &engine,
        &lease,
        &memory_engram(
            Some("openai-account"),
            Some("openai-represents-clyffy"),
            Some("openai"),
            1_200,
        ),
        1_200,
        "openai",
    );
    let represented = resolve(&engine, 1_250, "openai").unwrap();
    represented.validate().unwrap();
    assert_eq!(represented.seat_id.as_str(), "clyffy");
    assert_eq!(represented.representations.len(), 1);
    assert_eq!(represented.representations[0].provider.as_str(), "openai");
    assert_eq!(
        represented.uri,
        "rrflow://test-instance/data/rrflow-seat/clyffy"
    );

    let warp = engine
        .resolve_memory_warp(
            &lease.session_id,
            &lease.token,
            &ResolveMemoryWarp {
                scope: "instance:test-instance".into(),
                uri: represented.uri.clone(),
                query: String::new(),
                valid_at: 1_250,
                max_graph_depth: 1,
                max_items: 8,
                max_output_bytes: 64 * 1024,
                max_storage_keys: 256,
            },
            1_250,
            "request-warp-clyffy",
            "operation-warp-clyffy",
        )
        .unwrap();
    warp.validate().unwrap();
    assert!(warp
        .context
        .items
        .iter()
        .any(|item| item.identity == "record:rrflow-seat:clyffy"));
    assert!(warp
        .context
        .items
        .iter()
        .any(|item| item.identity == "record:rrflow-provider-identity:openai-account"));

    let replacement_request = memory_engram(
        Some("anthropic-account"),
        Some("anthropic-represents-clyffy"),
        Some("anthropic"),
        1_300,
    );
    let mut replacement = engine
        .plan_memory_engram(
            &lease.session_id,
            &lease.token,
            &replacement_request,
            1_300,
            "request-plan-replacement",
            "operation-plan-replacement",
        )
        .unwrap();
    replacement.mutations.splice(
        0..0,
        [
            TransactionMutation::RetireData {
                model: rrd_contract::DataLogicalModel::GraphRelation,
                target: rrd_contract::DataTarget::Reference {
                    reference: DataReference {
                        kind: CanonicalId::new(MEMORY_REPRESENTS_KIND).unwrap(),
                        id: CanonicalId::new("openai-represents-clyffy").unwrap(),
                    },
                },
                effective_at: 1_300,
            },
            TransactionMutation::RetireData {
                model: rrd_contract::DataLogicalModel::GraphNode,
                target: rrd_contract::DataTarget::Reference {
                    reference: DataReference {
                        kind: CanonicalId::new(MEMORY_PROVIDER_IDENTITY_KIND).unwrap(),
                        id: CanonicalId::new("openai-account").unwrap(),
                    },
                },
                effective_at: 1_300,
            },
        ],
    );
    replacement.operation_sha256 =
        rrd_contract::transaction_operation_sha256(&replacement.mutations);
    let replacement_transaction = engine
        .begin_transaction(
            &lease.session_id,
            &lease.token,
            &BeginTransaction {
                scope: CanonicalId::new("data").unwrap(),
                timeout_ms: 1_000,
            },
            &mutation_context(
                &id("begin-replacement"),
                "request-begin-replacement",
                "operation-begin-replacement",
            ),
            1_300,
        )
        .unwrap();
    engine
        .commit_transaction(
            &lease.session_id,
            &lease.token,
            &replacement_transaction.transaction_id,
            &id("commit-replacement"),
            &CommitTransaction {
                operation_sha256: replacement.operation_sha256,
                mutations: replacement.mutations,
            },
            1_300,
            "request-commit-replacement",
            "operation-commit-replacement",
        )
        .unwrap();

    let replaced = resolve(&engine, 1_350, "anthropic").unwrap();
    assert_eq!(replaced.seat_id, represented.seat_id);
    assert_eq!(replaced.uri, represented.uri);
    assert_eq!(replaced.representations.len(), 1);
    assert_eq!(replaced.representations[0].provider.as_str(), "anthropic");

    drop(engine);
    let reopened = RrdEngine::open(root.path(), instance(), TOKEN_KEY).unwrap();
    let after_reopen = resolve(&reopened, 1_350, "reopen").unwrap();
    assert_eq!(after_reopen, replaced);
}
