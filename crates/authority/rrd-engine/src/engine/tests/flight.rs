use super::*;
use rrd_contract::{
    DataLogicalModel, DataReference, DataSchemaMode, DataSchemaRegistry, DataTableSchema,
    DataTarget, ExecuteQuery, LaunchPromptFlight, ListPromptFlights, PromptFlightContextMode,
    PromptFlightContextRequest, PromptFlightProvider, PromptFlightStage, PromptFlightStatus,
    QueryBudget, QueryValue, ReadAccessPath, ReadDataSnapshot, ReadPromptFlight,
    PROMPT_FLIGHT_EVENT_KIND, PROMPT_FLIGHT_KIND, PROMPT_FLIGHT_USE_KIND,
};
use std::collections::{BTreeMap, BTreeSet};

const INSTANCE_SCOPE: &str = "instance:test-instance";

pub(super) fn launch_request(
    prompt: &str,
    context_mode: PromptFlightContextMode,
) -> LaunchPromptFlight {
    LaunchPromptFlight {
        scope: INSTANCE_SCOPE.into(),
        prompt: prompt.into(),
        provider: PromptFlightProvider::Observe,
        context_mode,
        context: PromptFlightContextRequest {
            valid_at: 1_200,
            seeds: Vec::new(),
            max_graph_depth: 2,
            max_items: 8,
            max_output_bytes: 65_536,
            max_storage_keys: 4_096,
        },
        acceptance_marker: None,
        task_id: None,
    }
}

/// Commits three note records so prompt-matched context has something to
/// recall. Returns the receipt so a test can reason about cursors.
pub(super) fn seed_notes(
    engine: &RrdEngine,
    lease: &rrd_contract::SessionLease,
) -> rrd_contract::CommitReceipt {
    let note = CanonicalId::new("note").unwrap();
    let mutations = vec![
        TransactionMutation::PutSchema {
            registry: DataSchemaRegistry {
                revision: 1,
                migration: "install flight fixture".into(),
                catalogue: DataCatalogueIdentity::default(),
                tables: BTreeMap::from([(
                    note.clone(),
                    DataTableSchema {
                        model: DataLogicalModel::Document,
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
        TransactionMutation::PutRecord {
            reference: DataReference {
                kind: note.clone(),
                id: CanonicalId::new("alpha").unwrap(),
            },
            valid_from: 1_100,
            valid_to: None,
            properties: BTreeMap::from([(
                "body".into(),
                QueryValue::String("Restore the durable session from its token.".into()),
            )]),
        },
        TransactionMutation::PutRecord {
            reference: DataReference {
                kind: note.clone(),
                id: CanonicalId::new("beta").unwrap(),
            },
            valid_from: 1_100,
            valid_to: None,
            properties: BTreeMap::from([(
                "body".into(),
                QueryValue::String("Rotate credentials only after session recovery.".into()),
            )]),
        },
        TransactionMutation::PutRecord {
            reference: DataReference {
                kind: note,
                id: CanonicalId::new("gamma").unwrap(),
            },
            valid_from: 1_100,
            valid_to: None,
            properties: BTreeMap::from([(
                "body".into(),
                QueryValue::String("Unrelated rendering preferences.".into()),
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
                &id("flight-seed-begin"),
                "request-seed-begin",
                "operation-seed-begin",
            ),
            1_050,
        )
        .unwrap();
    engine
        .commit_transaction(
            &lease.session_id,
            &lease.token,
            &transaction.transaction_id,
            &id("flight-seed-commit"),
            &CommitTransaction {
                operation_sha256: rrd_contract::transaction_operation_sha256(&mutations),
                mutations,
            },
            1_100,
            "request-seed-commit",
            "operation-seed-commit",
        )
        .unwrap()
}

pub(super) fn session(engine: &RrdEngine, name: &str) -> rrd_contract::SessionLease {
    engine
        .create_session(
            &session_request(5_000, 4),
            &id(name),
            1_000,
            &format!("request-{name}"),
            &format!("operation-{name}"),
        )
        .unwrap()
}

#[test]
fn fresh_and_pruned_flights_share_a_cohort_commit_at_one_stamp_and_replay_exactly() {
    let (_root, engine) = isolated_engine();
    let lease = session(&engine, "flight-session");
    let receipt = seed_notes(&engine, &lease);
    let seeded_cursor = receipt
        .last_runtime_cursor
        .expect("data commit advances the runtime cursor");

    let fresh = engine
        .launch_prompt_flight(
            &lease.session_id,
            &lease.token,
            &id("flight-fresh"),
            &launch_request("session recovery", PromptFlightContextMode::Fresh),
            1_300,
            "request-flight-fresh",
            "operation-flight-fresh",
        )
        .unwrap();
    assert!(!fresh.idempotent_replay);
    let fresh_record = &fresh.flight.record;
    assert_eq!(fresh_record.status, PromptFlightStatus::Prepared);
    assert_eq!(fresh_record.provider, PromptFlightProvider::Observe);
    assert!(fresh_record.context.is_none());
    assert_eq!(fresh_record.metrics.context_items, 0);
    assert_eq!(fresh_record.use_count, 0);
    assert!(fresh.flight.uses.is_empty());
    assert_eq!(fresh_record.read.runtime_cursor, seeded_cursor);
    // One schema mutation installs both flight tables ahead of the first row.
    assert_eq!(fresh_record.flight_cursor, seeded_cursor + 2);
    assert_eq!(fresh_record.known_at_cursor, fresh_record.flight_cursor);
    assert_eq!(fresh_record.event_count, 3);
    assert_eq!(
        fresh
            .flight
            .events
            .iter()
            .map(|event| (event.ordinal, event.stage, event.kind.as_str()))
            .collect::<Vec<_>>(),
        vec![
            (0, PromptFlightStage::Prompt, "prompt_received"),
            (1, PromptFlightStage::Context, "fresh_baseline"),
            (2, PromptFlightStage::Outcome, "handoff_ready"),
        ]
    );
    assert_eq!(
        engine.storage.runtime().cursor().unwrap(),
        fresh_record.flight_cursor + 3,
        "schema, record, and three events are one commit"
    );

    let pruned = engine
        .launch_prompt_flight(
            &lease.session_id,
            &lease.token,
            &id("flight-pruned"),
            &launch_request("  session recovery  ", PromptFlightContextMode::Pruned),
            1_400,
            "request-flight-pruned",
            "operation-flight-pruned",
        )
        .unwrap();
    let pruned_record = &pruned.flight.record;
    assert_eq!(pruned_record.cohort_sha256, fresh_record.cohort_sha256);
    assert_ne!(pruned_record.flight_id, fresh_record.flight_id);
    let context = pruned_record
        .context
        .as_ref()
        .expect("pruned flight records its context");
    assert!(context.items > 0, "the seeded notes match the prompt");
    assert_eq!(pruned_record.metrics.context_items, context.items);
    assert_eq!(
        pruned_record.metrics.context_output_bytes,
        context.output_bytes
    );
    // The first flight that uses records installs the edge table, so one
    // schema mutation precedes the row; the edges follow it in rank order.
    assert_eq!(
        pruned_record.flight_cursor,
        pruned_record.read.runtime_cursor + 2
    );
    assert!(pruned_record.use_count > 0);
    assert_eq!(pruned.flight.uses.len() as u64, pruned_record.use_count);
    assert!(pruned
        .flight
        .uses
        .iter()
        .enumerate()
        .all(|(position, used)| used.rank == position as u64 + 1
            && used.reference.kind.as_str() == "note"
            && !used.evidence.is_empty()));
    assert_eq!(
        engine.storage.runtime().cursor().unwrap(),
        pruned_record.flight_cursor + pruned_record.use_count + pruned_record.event_count,
        "schema, record, edges, and events are one commit"
    );
    let context_event = &pruned.flight.events[1];
    assert_eq!(context_event.kind.as_str(), "context_assembled");
    assert_eq!(
        context_event.data.get("plan_sha256"),
        Some(&QueryValue::Digest(context.plan_sha256.clone()))
    );
    let stages = match context_event.data.get("stages") {
        Some(QueryValue::List(stages)) => stages,
        other => panic!("context event carries the plan stages: {other:?}"),
    };
    assert_eq!(
        stages.len(),
        5,
        "every canonical context stage is disclosed"
    );

    // Context the flight injected is exactly what context-assemble returns at
    // the same stamp: the flight is an evidence coordinate, not a copy.
    let packet = engine
        .assemble_context(
            &lease.session_id,
            &lease.token,
            &launch_request("session recovery", PromptFlightContextMode::Pruned)
                .context
                .assemble_context(INSTANCE_SCOPE, "session recovery"),
            1_450,
            "request-flight-context",
            "operation-flight-context",
        )
        .unwrap();
    // The plan digest binds the read stamp, which moved when the flight itself
    // committed; the stage decisions and the fused result are stamp-stable.
    assert_ne!(packet.plan.plan_sha256, context.plan_sha256);
    assert_eq!(packet.items.len() as u64, context.items);
    assert_eq!(packet.output_bytes, context.output_bytes);
    let recorded_decisions = stages
        .iter()
        .map(|stage| match stage {
            QueryValue::Map(entry) => entry.get("decision_sha256").cloned(),
            other => panic!("stage entry is a map: {other:?}"),
        })
        .collect::<Vec<_>>();
    assert_eq!(
        recorded_decisions,
        packet
            .plan
            .stages
            .iter()
            .map(|stage| Some(QueryValue::Digest(stage.decision_sha256.clone())))
            .collect::<Vec<_>>()
    );

    // Exact replay returns the recorded flight without a new commit.
    let head = engine.storage.runtime().cursor().unwrap();
    let replay = engine
        .launch_prompt_flight(
            &lease.session_id,
            &lease.token,
            &id("flight-pruned"),
            &launch_request("  session recovery  ", PromptFlightContextMode::Pruned),
            1_500,
            "request-flight-replay",
            "operation-flight-replay",
        )
        .unwrap();
    assert!(replay.idempotent_replay);
    assert_eq!(replay.flight, pruned.flight);
    assert_eq!(engine.storage.runtime().cursor().unwrap(), head);

    // The same key with a different request is a conflict, not a new flight.
    let conflict = engine.launch_prompt_flight(
        &lease.session_id,
        &lease.token,
        &id("flight-pruned"),
        &launch_request("a different prompt", PromptFlightContextMode::Pruned),
        1_600,
        "request-flight-conflict",
        "operation-flight-conflict",
    );
    assert!(matches!(conflict, Err(ServiceError::IdempotencyConflict)));
    assert_eq!(engine.storage.runtime().cursor().unwrap(), head);

    let page = engine
        .list_prompt_flights(
            &lease.session_id,
            &lease.token,
            &ListPromptFlights {
                scope: INSTANCE_SCOPE.into(),
                after_cursor: 0,
                limit: 1,
                max_storage_keys: 4_096,
            },
            1_700,
            "request-flight-list",
            "operation-flight-list",
        )
        .unwrap();
    assert_eq!(page.flights.len(), 1);
    assert!(page.truncated);
    assert_eq!(page.flights[0], *fresh_record);
    let rest = engine
        .list_prompt_flights(
            &lease.session_id,
            &lease.token,
            &ListPromptFlights {
                scope: INSTANCE_SCOPE.into(),
                after_cursor: page.flights[0].flight_cursor,
                limit: 8,
                max_storage_keys: 4_096,
            },
            1_700,
            "request-flight-list-rest",
            "operation-flight-list-rest",
        )
        .unwrap();
    assert_eq!(rest.flights, vec![pruned_record.clone()]);
    assert!(!rest.truncated);
    assert_eq!(rest.read.runtime_cursor, head);

    let read = engine
        .read_prompt_flight(
            &lease.session_id,
            &lease.token,
            &ReadPromptFlight {
                scope: INSTANCE_SCOPE.into(),
                flight_id: pruned_record.flight_id.clone(),
                max_storage_keys: 4_096,
            },
            1_800,
            "request-flight-read",
            "operation-flight-read",
        )
        .unwrap();
    assert_eq!(read.flight, pruned.flight);
    assert_eq!(read.read.runtime_cursor, head);

    let missing = engine.read_prompt_flight(
        &lease.session_id,
        &lease.token,
        &ReadPromptFlight {
            scope: INSTANCE_SCOPE.into(),
            flight_id: CanonicalId::new("flight-unknown").unwrap(),
            max_storage_keys: 4_096,
        },
        1_800,
        "request-flight-missing",
        "operation-flight-missing",
    );
    assert!(matches!(missing, Err(ServiceError::FlightNotFound)));

    let wrong_scope = engine.launch_prompt_flight(
        &lease.session_id,
        &lease.token,
        &id("flight-wrong-scope"),
        &LaunchPromptFlight {
            scope: "instance:other".into(),
            ..launch_request("session recovery", PromptFlightContextMode::Fresh)
        },
        1_900,
        "request-flight-wrong-scope",
        "operation-flight-wrong-scope",
    );
    assert!(matches!(wrong_scope, Err(ServiceError::WrongScope)));
}

#[test]
fn prompt_flights_survive_reopen_and_are_visible_in_the_temporal_graph() {
    let root = tempfile::tempdir().unwrap();
    let engine = RrdEngine::open(root.path(), instance(), TOKEN_KEY).unwrap();
    let lease = session(&engine, "flight-reopen-session");
    seed_notes(&engine, &lease);
    let launched = engine
        .launch_prompt_flight(
            &lease.session_id,
            &lease.token,
            &id("flight-durable"),
            &launch_request("session recovery", PromptFlightContextMode::Pruned),
            1_300,
            "request-flight-durable",
            "operation-flight-durable",
        )
        .unwrap();
    let head = engine.storage.runtime().cursor().unwrap();

    let snapshot = engine
        .read_data_snapshot(
            &lease.session_id,
            &lease.token,
            &ReadDataSnapshot {
                valid_at: 1_300,
                max_storage_keys: 4_096,
            },
            1_350,
            "request-flight-graph",
            "operation-flight-graph",
        )
        .unwrap();
    let flight_rows = snapshot
        .entries
        .iter()
        .filter(|entry| match &entry.target {
            DataTarget::Reference { reference } => reference.kind.as_str() == PROMPT_FLIGHT_KIND,
            DataTarget::Event { .. } => false,
        })
        .count();
    let flight_events = snapshot
        .entries
        .iter()
        .filter(|entry| match &entry.target {
            DataTarget::Event { kind, .. } => kind.as_str() == PROMPT_FLIGHT_EVENT_KIND,
            DataTarget::Reference { .. } => false,
        })
        .count();
    assert_eq!(flight_rows, 1, "the flight row is ordinary temporal data");
    assert_eq!(
        flight_events, 3,
        "every stage event is an addressable engine event"
    );
    drop(engine);

    let reopened = RrdEngine::open(root.path(), instance(), TOKEN_KEY).unwrap();
    assert_eq!(reopened.storage.runtime().cursor().unwrap(), head);
    let lease = session(&reopened, "flight-reopened-session");
    let read = reopened
        .read_prompt_flight(
            &lease.session_id,
            &lease.token,
            &ReadPromptFlight {
                scope: INSTANCE_SCOPE.into(),
                flight_id: launched.flight.record.flight_id.clone(),
                max_storage_keys: 4_096,
            },
            2_000,
            "request-flight-reopened",
            "operation-flight-reopened",
        )
        .unwrap();
    assert_eq!(read.flight, launched.flight);
}

#[test]
fn prompt_flight_operations_require_their_exact_grants() {
    let (_root, engine) = isolated_engine();
    let principal_id = CanonicalId::new("flight-reader").unwrap();
    let resource = ResourcePath {
        segments: vec![ResourceId::new(ResourceKind::Instance, instance().as_str()).unwrap()],
    };
    SecurityRepository::new(&engine.storage, instance())
        .initialize(
            SecurityState {
                format_version: rrd_security::SECURITY_FORMAT,
                revision: 1,
                principals: [(
                    principal_id.clone(),
                    Principal {
                        id: principal_id.clone(),
                        kind: PrincipalKind::Service,
                        credential_sha256: digest::sha256_hex(b"flight-reader-key"),
                        credential_revision: 1,
                        not_before_unix_ms: 1,
                        expires_at_unix_ms: u64::MAX,
                        disabled: false,
                        role_ids: Default::default(),
                        grants: vec![
                            ResourceGrant {
                                action: SecurityAction::SessionCreate,
                                resource_prefix: resource.clone(),
                                data_policy: None,
                            },
                            ResourceGrant {
                                action: SecurityAction::FlightRead,
                                resource_prefix: resource,
                                data_policy: None,
                            },
                        ],
                    },
                )]
                .into_iter()
                .collect(),
                roles: Default::default(),
                identity_bindings: Default::default(),
                jwt_issuers: Default::default(),
            },
            1,
            "security-test",
            "request-security",
            "operation-security",
        )
        .unwrap();
    let lease = engine
        .create_authenticated_session(
            &principal_id,
            b"flight-reader-key",
            &session_request(5_000, 2),
            &id("flight-reader-create"),
            1_000,
            "request-flight-reader-create",
            "operation-flight-reader-create",
        )
        .unwrap();
    let head = engine.storage.runtime().cursor().unwrap();

    let denied = engine.launch_prompt_flight(
        &lease.session_id,
        &lease.token,
        &id("flight-denied"),
        &launch_request("session recovery", PromptFlightContextMode::Fresh),
        1_100,
        "request-flight-denied",
        "operation-flight-denied",
    );
    assert!(matches!(denied, Err(ServiceError::PermissionDenied)));
    assert_eq!(
        engine.storage.runtime().cursor().unwrap(),
        head,
        "a denied launch writes nothing to the runtime log"
    );
    let denied_audit = SecurityRepository::new(&engine.storage, instance())
        .audit_since(0, 64)
        .unwrap()
        .records
        .into_iter()
        .map(|(_, record)| record)
        .find(|record| record.request_id == "request-flight-denied")
        .expect("the denied launch is audited");
    assert_eq!(denied_audit.action, SecurityAction::FlightLaunch);
    assert_eq!(denied_audit.decision, AuditDecision::Denied);

    // The read grant alone lists an empty page at the current stamp.
    let page = engine
        .list_prompt_flights(
            &lease.session_id,
            &lease.token,
            &ListPromptFlights {
                scope: INSTANCE_SCOPE.into(),
                after_cursor: 0,
                limit: 8,
                max_storage_keys: 64,
            },
            1_200,
            "request-flight-reader-list",
            "operation-flight-reader-list",
        )
        .unwrap();
    assert!(page.flights.is_empty());
    assert!(!page.truncated);
    assert_eq!(page.read.runtime_cursor, head);
}

#[test]
fn the_first_flight_on_an_empty_estate_installs_the_flight_tables() {
    let (_root, engine) = isolated_engine();
    let lease = session(&engine, "flight-empty-session");
    let scope = rrd_core::ScopeId::new(INSTANCE_SCOPE).unwrap();
    assert_eq!(engine.storage.runtime().schema(&scope).unwrap(), None);

    let launched = engine
        .launch_prompt_flight(
            &lease.session_id,
            &lease.token,
            &id("flight-empty"),
            &launch_request("session recovery", PromptFlightContextMode::Pruned),
            1_300,
            "request-flight-empty",
            "operation-flight-empty",
        )
        .unwrap();
    let record = &launched.flight.record;
    assert_eq!(record.read.runtime_cursor, 0);
    assert_eq!(record.read.schema_revision, None);
    assert_eq!(record.flight_cursor, 2, "the schema precedes the first row");
    let context = record.context.as_ref().unwrap();
    assert_eq!(context.items, 0, "an empty estate injects nothing");
    assert_eq!(launched.flight.events[1].kind.as_str(), "context_empty");

    let schema = engine.storage.runtime().schema(&scope).unwrap().unwrap();
    assert_eq!(schema.revision, 1);
    assert!(schema
        .records
        .contains_key(&rrd_core::RuntimeType::new(PROMPT_FLIGHT_KIND).unwrap()));
    assert!(schema
        .events
        .contains_key(&rrd_core::RuntimeType::new(PROMPT_FLIGHT_EVENT_KIND).unwrap()));
}

#[test]
fn each_prompt_is_a_graph_that_rrflowql_traverses_and_reads_cost_only_the_flight() {
    let (_root, engine) = isolated_engine();
    let lease = session(&engine, "flight-graph-session");
    seed_notes(&engine, &lease);
    let launched = engine
        .launch_prompt_flight(
            &lease.session_id,
            &lease.token,
            &id("flight-graph"),
            &launch_request("session recovery", PromptFlightContextMode::Pruned),
            1_300,
            "request-flight-graph",
            "operation-flight-graph",
        )
        .unwrap();
    let flight = &launched.flight;
    assert!(
        flight.record.use_count >= 2,
        "two seeded notes match the prompt"
    );
    let used = flight
        .uses
        .iter()
        .map(|used| used.reference.id.as_str().to_owned())
        .collect::<BTreeSet<_>>();

    // rrflowQL walks prompt -> the records its context used.
    let traversal = engine
        .execute_query(
            &lease.session_id,
            &lease.token,
            &ExecuteQuery {
                scope: INSTANCE_SCOPE.into(),
                query: format!(
                    "FROM traverse:{PROMPT_FLIGHT_USE_KIND} START {PROMPT_FLIGHT_KIND}:{} DIRECTION OUTGOING DEPTH 1 AT VALID 1300 KNOWN HEAD PROJECT node_id, depth, path",
                    flight.record.flight_id
                ),
                parameters: BTreeMap::new(),
                budget: QueryBudget::default(),
            },
            1_350,
            "request-flight-traverse",
            "operation-flight-traverse",
        )
        .unwrap();
    let reached = traversal
        .rows
        .iter()
        .map(|row| match row.values.get("node_id") {
            Some(QueryValue::String(id)) => id.clone(),
            other => panic!("traversal row names its node: {other:?}"),
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(reached, used);
    // And back: from a record to every prompt that used it.
    let first = &flight.uses[0].reference;
    let reverse = engine
        .execute_query(
            &lease.session_id,
            &lease.token,
            &ExecuteQuery {
                scope: INSTANCE_SCOPE.into(),
                query: format!(
                    "FROM traverse:{PROMPT_FLIGHT_USE_KIND} START {}:{} DIRECTION INCOMING DEPTH 1 AT VALID 1300 KNOWN HEAD PROJECT node_id",
                    first.kind, first.id
                ),
                parameters: BTreeMap::new(),
                budget: QueryBudget::default(),
            },
            1_360,
            "request-flight-reverse",
            "operation-flight-reverse",
        )
        .unwrap();
    assert_eq!(reverse.rows.len(), 1);
    assert_eq!(
        reverse.rows[0].values.get("node_id"),
        Some(&QueryValue::String(flight.record.flight_id.as_str().into()))
    );

    // A flight read names its record, edges, and events by identity: the
    // keys it examines do not grow when the estate gains unrelated flights.
    let read = |name: &str| {
        engine
            .read_prompt_flight(
                &lease.session_id,
                &lease.token,
                &ReadPromptFlight {
                    scope: INSTANCE_SCOPE.into(),
                    flight_id: flight.record.flight_id.clone(),
                    max_storage_keys: 4_096,
                },
                1_400,
                &format!("request-{name}"),
                &format!("operation-{name}"),
            )
            .unwrap()
    };
    let before = read("flight-read-before");
    assert_eq!(before.flight, *flight);
    for index in 0..8 {
        engine
            .launch_prompt_flight(
                &lease.session_id,
                &lease.token,
                &id(&format!("flight-noise-{index}")),
                &launch_request("session recovery", PromptFlightContextMode::Pruned),
                1_500 + index,
                &format!("request-flight-noise-{index}"),
                &format!("operation-flight-noise-{index}"),
            )
            .unwrap();
    }
    let after = read("flight-read-after");
    assert_eq!(after.flight, *flight);
    assert!(after.read.runtime_cursor > before.read.runtime_cursor);
    let event_path = |result: &rrd_contract::PromptFlightReadResult| {
        result
            .read_evidence
            .paths
            .iter()
            .find(|path| path.path == ReadAccessPath::EventVersions)
            .map(|path| path.keys_examined)
    };
    assert_eq!(event_path(&before), event_path(&after));
    let relation_path = |result: &rrd_contract::PromptFlightReadResult| {
        result
            .read_evidence
            .paths
            .iter()
            .find(|path| path.path == ReadAccessPath::RelationVersions)
            .map(|path| path.keys_examined)
    };
    assert_eq!(relation_path(&before), relation_path(&after));
}
