use super::flight::{launch_request, seed_notes, session};
use super::*;
use rrd_contract::{
    CloseTask, ExecuteQuery, ListTasks, OpenTask, PromptFlightContextMode, QueryBudget, QueryValue,
    ReadTask, TaskStatus, PROMPT_FLIGHT_KIND, PROMPT_FLIGHT_USE_KIND, TASK_INCLUDES_KIND,
    TASK_KIND,
};
use std::collections::{BTreeMap, BTreeSet};

const INSTANCE_SCOPE: &str = "instance:test-instance";

fn open_request(title: &str) -> OpenTask {
    OpenTask {
        scope: INSTANCE_SCOPE.into(),
        title: title.into(),
        goal: "Recover a durable session without rotating credentials first.".into(),
    }
}

fn close_request(task_id: &CanonicalId, status: TaskStatus, outcome: Option<&str>) -> CloseTask {
    CloseTask {
        scope: INSTANCE_SCOPE.into(),
        task_id: task_id.clone(),
        status,
        outcome: outcome.map(str::to_owned),
    }
}

fn read_request(task_id: &CanonicalId) -> ReadTask {
    ReadTask {
        scope: INSTANCE_SCOPE.into(),
        task_id: task_id.clone(),
        max_storage_keys: 4_096,
    }
}

/// Node identities one rrflowQL traversal reaches.
fn traverse(
    engine: &RrdEngine,
    lease: &rrd_contract::SessionLease,
    relation: &str,
    start: (&str, &str),
    direction: &str,
    name: &str,
) -> BTreeSet<String> {
    engine
        .execute_query(
            &lease.session_id,
            &lease.token,
            &ExecuteQuery {
                scope: INSTANCE_SCOPE.into(),
                query: format!(
                    "FROM traverse:{relation} START {}:{} DIRECTION {direction} DEPTH 1 AT VALID 2000 KNOWN HEAD PROJECT node_id",
                    start.0, start.1
                ),
                parameters: BTreeMap::new(),
                budget: QueryBudget::default(),
            },
            2_100,
            &format!("request-{name}"),
            &format!("operation-{name}"),
        )
        .unwrap()
        .rows
        .iter()
        .map(|row| match row.values.get("node_id") {
            Some(QueryValue::String(id)) => id.clone(),
            other => panic!("traversal row names its node: {other:?}"),
        })
        .collect()
}

#[test]
fn a_task_opens_once_replays_exactly_and_closes_once() {
    let (_root, engine) = isolated_engine();
    let lease = session(&engine, "task-session");
    let head = engine.storage.runtime().cursor().unwrap();

    let opened = engine
        .open_task(
            &lease.session_id,
            &lease.token,
            &id("task-open"),
            &open_request("Session recovery"),
            1_200,
            "request-task-open",
            "operation-task-open",
        )
        .unwrap();
    assert!(!opened.idempotent_replay);
    let task = &opened.task;
    assert_eq!(task.status, TaskStatus::Open);
    assert_eq!(task.opened_at_unix_ms, 1_200);
    assert_eq!(task.closed_at_unix_ms, None);
    assert_eq!(
        task.task_cursor,
        head + 2,
        "the task table precedes the row"
    );
    assert_eq!(task.known_at_cursor, task.task_cursor);

    // The same session and key replay the row; a different request under that
    // key is a conflict and writes nothing.
    let replay = engine
        .open_task(
            &lease.session_id,
            &lease.token,
            &id("task-open"),
            &open_request("Session recovery"),
            1_250,
            "request-task-replay",
            "operation-task-replay",
        )
        .unwrap();
    assert!(replay.idempotent_replay);
    assert_eq!(replay.task, *task);
    let after_open = engine.storage.runtime().cursor().unwrap();
    assert!(matches!(
        engine.open_task(
            &lease.session_id,
            &lease.token,
            &id("task-open"),
            &open_request("A different title"),
            1_260,
            "request-task-conflict",
            "operation-task-conflict",
        ),
        Err(ServiceError::IdempotencyConflict)
    ));
    assert_eq!(engine.storage.runtime().cursor().unwrap(), after_open);

    let read = engine
        .read_task(
            &lease.session_id,
            &lease.token,
            &read_request(&task.task_id),
            1_300,
            "request-task-read",
            "operation-task-read",
        )
        .unwrap();
    assert_eq!(read.task, *task);
    assert_eq!(read.selected_versions, 1);

    let closed = engine
        .close_task(
            &lease.session_id,
            &lease.token,
            &close_request(&task.task_id, TaskStatus::Completed, Some("Recovered.")),
            1_400,
            "request-task-close",
            "operation-task-close",
        )
        .unwrap();
    assert!(!closed.idempotent_replay);
    assert_eq!(closed.task.status, TaskStatus::Completed);
    assert_eq!(closed.task.outcome.as_deref(), Some("Recovered."));
    assert_eq!(closed.task.closed_at_unix_ms, Some(1_400));
    assert_eq!(closed.task.task_cursor, task.task_cursor);
    assert_eq!(closed.task.known_at_cursor, after_open + 1);
    assert_eq!(closed.task.idempotency_key, task.idempotency_key);

    // Closing again the same way replays; any other close fails and writes
    // nothing.
    let again = engine
        .close_task(
            &lease.session_id,
            &lease.token,
            &close_request(&task.task_id, TaskStatus::Completed, Some("Recovered.")),
            1_500,
            "request-task-close-again",
            "operation-task-close-again",
        )
        .unwrap();
    assert!(again.idempotent_replay);
    assert_eq!(again.task, closed.task);
    let after_close = engine.storage.runtime().cursor().unwrap();
    assert!(matches!(
        engine.close_task(
            &lease.session_id,
            &lease.token,
            &close_request(&task.task_id, TaskStatus::Abandoned, None),
            1_510,
            "request-task-reclose",
            "operation-task-reclose",
        ),
        Err(ServiceError::TaskClosed)
    ));
    assert!(matches!(
        engine.close_task(
            &lease.session_id,
            &lease.token,
            &close_request(
                &CanonicalId::new("task-missing").unwrap(),
                TaskStatus::Completed,
                None
            ),
            1_520,
            "request-task-close-missing",
            "operation-task-close-missing",
        ),
        Err(ServiceError::TaskNotFound)
    ));
    assert_eq!(engine.storage.runtime().cursor().unwrap(), after_close);
    assert!(matches!(
        engine.read_task(
            &lease.session_id,
            &lease.token,
            &read_request(&CanonicalId::new("task-missing").unwrap()),
            1_530,
            "request-task-read-missing",
            "operation-task-read-missing",
        ),
        Err(ServiceError::TaskNotFound)
    ));

    let second = engine
        .open_task(
            &lease.session_id,
            &lease.token,
            &id("task-open-second"),
            &open_request("Credential rotation"),
            1_600,
            "request-task-open-second",
            "operation-task-open-second",
        )
        .unwrap()
        .task;
    assert_eq!(second.task_cursor, after_close + 1, "the table exists now");
    let list = |after_cursor: u64, limit: u16, name: &str| {
        engine
            .list_tasks(
                &lease.session_id,
                &lease.token,
                &ListTasks {
                    scope: INSTANCE_SCOPE.into(),
                    after_cursor,
                    limit,
                    max_storage_keys: 4_096,
                },
                1_700,
                &format!("request-{name}"),
                &format!("operation-{name}"),
            )
            .unwrap()
    };
    let page = list(0, 8, "task-list");
    assert_eq!(page.tasks, vec![closed.task.clone(), second.clone()]);
    assert!(!page.truncated);
    let first_only = list(0, 1, "task-list-first");
    assert_eq!(first_only.tasks, vec![closed.task]);
    assert!(first_only.truncated);
    let rest = list(first_only.tasks[0].task_cursor, 8, "task-list-rest");
    assert_eq!(rest.tasks, vec![second]);
}

#[test]
fn flights_join_an_open_task_and_rrflowql_walks_task_to_flights_to_records() {
    let (_root, engine) = isolated_engine();
    let lease = session(&engine, "task-graph-session");
    seed_notes(&engine, &lease);
    let task = engine
        .open_task(
            &lease.session_id,
            &lease.token,
            &id("task-graph-open"),
            &open_request("Session recovery"),
            1_200,
            "request-task-graph-open",
            "operation-task-graph-open",
        )
        .unwrap()
        .task;

    let launch = |key: &str, task_id: Option<&CanonicalId>, at: u64| {
        let mut request = launch_request("session recovery", PromptFlightContextMode::Pruned);
        request.task_id = task_id.cloned();
        engine.launch_prompt_flight(
            &lease.session_id,
            &lease.token,
            &id(key),
            &request,
            at,
            &format!("request-{key}"),
            &format!("operation-{key}"),
        )
    };
    let first = launch("task-flight-first", Some(&task.task_id), 1_300).unwrap();
    let record = &first.flight.record;
    assert_eq!(record.task_id.as_ref(), Some(&task.task_id));
    // Schema, record, edges, events, and the task edge are one commit; the
    // task edge comes last so event cursors stay computable from the record.
    assert_eq!(
        engine.storage.runtime().cursor().unwrap(),
        record.flight_cursor + record.use_count + record.event_count + 1
    );
    let second = launch("task-flight-second", Some(&task.task_id), 1_400).unwrap();
    let outside = launch("task-flight-outside", None, 1_500).unwrap();
    assert_eq!(outside.flight.record.task_id, None);

    // Task -> its flights, and only its flights.
    let flights = traverse(
        &engine,
        &lease,
        TASK_INCLUDES_KIND,
        (TASK_KIND, task.task_id.as_str()),
        "OUTGOING",
        "task-to-flights",
    );
    assert_eq!(
        flights,
        BTreeSet::from([
            record.flight_id.as_str().to_owned(),
            second.flight.record.flight_id.as_str().to_owned(),
        ])
    );
    // Each flight -> the records its context used.
    let mut records = BTreeSet::new();
    for flight_id in &flights {
        records.extend(traverse(
            &engine,
            &lease,
            PROMPT_FLIGHT_USE_KIND,
            (PROMPT_FLIGHT_KIND, flight_id),
            "OUTGOING",
            &format!("flight-to-records-{flight_id}"),
        ));
    }
    let used = first
        .flight
        .uses
        .iter()
        .chain(&second.flight.uses)
        .map(|used| used.reference.id.as_str().to_owned())
        .collect::<BTreeSet<_>>();
    assert!(!used.is_empty());
    assert_eq!(records, used);
    // And back: a flight names the task it joined.
    assert_eq!(
        traverse(
            &engine,
            &lease,
            TASK_INCLUDES_KIND,
            (PROMPT_FLIGHT_KIND, record.flight_id.as_str()),
            "INCOMING",
            "flight-to-task",
        ),
        BTreeSet::from([task.task_id.as_str().to_owned()])
    );

    // A missing or closed task admits no flight, and the refusal writes
    // nothing; a flight that joined before the close still replays.
    let missing = CanonicalId::new("task-missing").unwrap();
    assert!(matches!(
        launch("task-flight-missing", Some(&missing), 1_600),
        Err(ServiceError::TaskNotFound)
    ));
    engine
        .close_task(
            &lease.session_id,
            &lease.token,
            &close_request(&task.task_id, TaskStatus::Abandoned, None),
            1_700,
            "request-task-graph-close",
            "operation-task-graph-close",
        )
        .unwrap();
    let head = engine.storage.runtime().cursor().unwrap();
    assert!(matches!(
        launch("task-flight-late", Some(&task.task_id), 1_800),
        Err(ServiceError::TaskClosed)
    ));
    assert_eq!(engine.storage.runtime().cursor().unwrap(), head);
    let replay = launch("task-flight-first", Some(&task.task_id), 1_900).unwrap();
    assert!(replay.idempotent_replay);
    assert_eq!(replay.flight, first.flight);
}

#[test]
fn task_operations_require_their_exact_grants() {
    let (_root, engine) = isolated_engine();
    let principal_id = CanonicalId::new("task-reader").unwrap();
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
                        credential_sha256: digest::sha256_hex(b"task-reader-key"),
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
                                action: SecurityAction::TaskRead,
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
            b"task-reader-key",
            &session_request(5_000, 2),
            &id("task-reader-create"),
            1_000,
            "request-task-reader-create",
            "operation-task-reader-create",
        )
        .unwrap();
    let head = engine.storage.runtime().cursor().unwrap();

    let denied = engine.open_task(
        &lease.session_id,
        &lease.token,
        &id("task-denied"),
        &open_request("Session recovery"),
        1_100,
        "request-task-denied",
        "operation-task-denied",
    );
    assert!(matches!(denied, Err(ServiceError::PermissionDenied)));
    assert_eq!(engine.storage.runtime().cursor().unwrap(), head);
    let denied_audit = SecurityRepository::new(&engine.storage, instance())
        .audit_since(0, 64)
        .unwrap()
        .records
        .into_iter()
        .map(|(_, record)| record)
        .find(|record| record.request_id == "request-task-denied")
        .expect("the denied open is audited");
    assert_eq!(denied_audit.action, SecurityAction::TaskWrite);
    assert_eq!(denied_audit.decision, AuditDecision::Denied);

    let page = engine
        .list_tasks(
            &lease.session_id,
            &lease.token,
            &ListTasks {
                scope: INSTANCE_SCOPE.into(),
                after_cursor: 0,
                limit: 8,
                max_storage_keys: 64,
            },
            1_200,
            "request-task-reader-list",
            "operation-task-reader-list",
        )
        .unwrap();
    assert!(page.tasks.is_empty());
    assert_eq!(page.read.runtime_cursor, head);
}
