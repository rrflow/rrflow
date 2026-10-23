//! Real authenticated HTTP -> engine -> durable store -> reopen -> replay.
use rrd_client::{ClientConfig, RequestOptions, RrdClient};
use rrd_contract::*;
use rrd_core::digest;
use rrd_engine::RrdEngine;
use rrd_security::{
    Action, Principal, PrincipalKind, ResourceGrant, SecurityRepository, SecurityState,
    SECURITY_FORMAT,
};
use rrd_server::RrdHttpServer;
use rrd_store::RrflowKvStore;
use std::collections::BTreeMap;

fn mutation(id: &str) -> RequestOptions {
    RequestOptions::mutation(id, id, id).unwrap()
}
fn read(id: &str) -> RequestOptions {
    RequestOptions::read(id, id).unwrap()
}
fn denied(error: rrd_client::Error) {
    assert!(
        matches!(error, rrd_client::Error::Api { error, .. } if error.code == ErrorCode::PermissionDenied)
    );
}

#[tokio::test]
async fn public_task_effort_executes_stops_and_replays_after_reopen() {
    let root = tempfile::tempdir().unwrap();
    let instance = CanonicalId::new("effort").unwrap();
    let storage = RrflowKvStore::open(root.path()).unwrap();
    let resource = ResourcePath {
        segments: vec![ResourceId::new(ResourceKind::Instance, "effort").unwrap()],
    };
    let principals = [
        (
            "writer",
            vec![
                Action::SessionCreate,
                Action::TaskWrite,
                Action::ReasoningWrite,
                Action::ReasoningRead,
                Action::EmbeddingGenerate,
            ],
        ),
        ("reader", vec![Action::SessionCreate, Action::ReasoningRead]),
        (
            "no-inference",
            vec![
                Action::SessionCreate,
                Action::ReasoningWrite,
                Action::ReasoningRead,
            ],
        ),
    ]
    .into_iter()
    .map(|(name, actions)| {
        let id = CanonicalId::new(name).unwrap();
        (
            id.clone(),
            Principal {
                id,
                kind: PrincipalKind::Service,
                credential_sha256: digest::sha256_hex(name.as_bytes()),
                credential_revision: 1,
                not_before_unix_ms: 1,
                expires_at_unix_ms: u64::MAX,
                disabled: false,
                role_ids: Default::default(),
                grants: actions
                    .into_iter()
                    .map(|action| ResourceGrant {
                        action,
                        resource_prefix: resource.clone(),
                        data_policy: None,
                    })
                    .collect(),
            },
        )
    })
    .collect();
    SecurityRepository::new(&storage, instance.clone())
        .initialize(
            SecurityState {
                format_version: SECURITY_FORMAT,
                revision: 1,
                principals,
                roles: BTreeMap::new(),
                identity_bindings: BTreeMap::new(),
                jwt_issuers: BTreeMap::new(),
            },
            1,
            "bootstrap",
            "bootstrap",
            "bootstrap",
        )
        .unwrap();
    drop(storage);
    let engine = RrdEngine::open(root.path(), instance.clone(), [0x6b; 32]).unwrap();
    let server = RrdHttpServer::bind(engine, "127.0.0.1:0".parse().unwrap()).unwrap();
    let client = RrdClient::connect_local(
        server.local_addr(),
        instance.clone(),
        ClientConfig::default(),
    )
    .unwrap();
    let (shutdown, receiver) = tokio::sync::oneshot::channel();
    let running = tokio::spawn(server.serve_until(async move {
        let _ = receiver.await;
    }));
    let mut sessions = Vec::new();
    for name in ["writer", "reader", "no-inference"] {
        sessions.push(
            client
                .create_session(
                    CanonicalId::new(name).unwrap(),
                    name,
                    CreateSession {
                        limits: SessionLimits {
                            idle_timeout_ms: 60_000,
                            absolute_timeout_ms: 300_000,
                            max_open_transactions: 1,
                        },
                    },
                    mutation(&format!("session-{name}")),
                )
                .await
                .unwrap(),
        );
    }
    let session = &sessions[0];
    let scope = "instance:effort".to_string();
    let task = client
        .open_task(
            session,
            OpenTask {
                scope: scope.clone(),
                title: "Bounded public work".into(),
                goal: "Preserve effort and results across reconnect".into(),
            },
            mutation("open"),
        )
        .await
        .unwrap()
        .task
        .task_id;
    let config = ConfigureTaskBudget {
        repetition_policy: None,
        scope: scope.clone(),
        task_id: task.clone(),
        limits: ReasoningLimits {
            max_run_elapsed_ms: 60_000,
            max_steps: 2,
            max_step_elapsed_ms: 5_000,
        },
    };
    let configured = client
        .configure_task_budget(session, config.clone(), mutation("configure"))
        .await
        .unwrap();
    assert_eq!(
        client
            .configure_task_budget(session, config.clone(), mutation("same-config"))
            .await
            .unwrap(),
        configured
    );
    denied(
        client
            .configure_task_budget(&sessions[1], config.clone(), mutation("denied-config"))
            .await
            .unwrap_err(),
    );
    let activation = ActivateBuiltinEmbedding {
        scope: scope.clone(),
        dimensions: 32,
        seed: 42,
    };
    denied(
        client
            .activate_builtin_embedding(
                &sessions[1],
                activation.clone(),
                mutation("denied-activation"),
            )
            .await
            .unwrap_err(),
    );
    let models = client
        .activate_builtin_embedding(session, activation.clone(), mutation("activate"))
        .await
        .unwrap();
    assert_eq!(
        client
            .activate_builtin_embedding(session, activation, mutation("activate-again"))
            .await
            .unwrap(),
        models
    );
    let execute = ExecuteTaskEmbedding {
        task_id: task.clone(),
        attempt_id: CorrelationId::new("first").unwrap(),
        request: GenerateEmbeddings {
            scope: scope.clone(),
            backend_id: models.backends[0].id.clone(),
            network_policy: EmbeddingNetworkPolicy::Deny,
            inputs: vec![EmbeddingInput {
                id: CanonicalId::new("source").unwrap(),
                media_type: "text/plain".into(),
                bytes: b"Ground this task in an actual bounded execution".to_vec(),
            }],
        },
    };
    denied(
        client
            .execute_task_embedding(&sessions[2], execute.clone(), mutation("denied-execute"))
            .await
            .unwrap_err(),
    );
    let output = client
        .execute_task_embedding(session, execute.clone(), mutation("execute"))
        .await
        .unwrap();
    assert_eq!(output.embeddings.len(), 1);
    let read_budget = ReadTaskBudget {
        scope: scope.clone(),
        task_id: task.clone(),
    };
    let active = client
        .read_task_budget(&sessions[1], read_budget.clone(), read("read-budget"))
        .await
        .unwrap();
    assert_eq!(active.budget.as_ref().unwrap().reserved_steps, 1);
    assert_eq!(
        active.budget.as_ref().unwrap().deadline_unix_ms,
        configured.budget.as_ref().unwrap().deadline_unix_ms
    );
    let attempt = ReadTaskAttempt {
        scope: scope.clone(),
        task_id: task.clone(),
        attempt_id: execute.attempt_id.clone(),
    };
    let receipt = client
        .read_task_attempt(session, attempt.clone(), read("receipt"))
        .await
        .unwrap();
    assert_eq!(
        receipt.attempt.as_ref().unwrap().result.as_ref(),
        Some(&output)
    );
    assert_eq!(
        receipt.attempt.as_ref().unwrap().status,
        TaskAttemptStatus::Accepted
    );
    assert!(client
        .execute_task_embedding(session, execute.clone(), mutation("duplicate"))
        .await
        .is_err());
    let stale_stop = StopTaskBudget {
        scope: scope.clone(),
        task_id: task.clone(),
        expected_budget_sha256: configured.budget_sha256.unwrap(),
    };
    assert!(client
        .stop_task_budget(session, stale_stop.clone(), mutation("stale-stop"))
        .await
        .is_err());
    let stop = StopTaskBudget {
        expected_budget_sha256: active.budget_sha256.unwrap(),
        ..stale_stop
    };
    denied(
        client
            .stop_task_budget(&sessions[1], stop.clone(), mutation("denied-stop"))
            .await
            .unwrap_err(),
    );
    let stopped = client
        .stop_task_budget(session, stop.clone(), mutation("stop"))
        .await
        .unwrap();
    assert_eq!(
        stopped.budget.as_ref().unwrap().stopped,
        Some(ReasoningBudgetStop::Cancelled)
    );
    assert_eq!(
        client
            .stop_task_budget(session, stop.clone(), mutation("stop-retry"))
            .await
            .unwrap(),
        stopped
    );
    let mut next = execute.clone();
    next.attempt_id = CorrelationId::new("second").unwrap();
    assert!(client
        .execute_task_embedding(session, next, mutation("after-stop"))
        .await
        .is_err());
    let mut foreign = read_budget.clone();
    foreign.scope = "instance:foreign".into();
    assert!(client
        .read_task_budget(session, foreign, read("foreign"))
        .await
        .is_err());

    let repeat_task = client
        .open_task(
            session,
            OpenTask {
                scope: scope.clone(),
                title: "No repeated work".into(),
                goal: "Detect cycles across request IDs".into(),
            },
            mutation("repeat-open"),
        )
        .await
        .unwrap()
        .task
        .task_id;
    let repeat_config = ConfigureTaskBudget {
        scope: scope.clone(),
        task_id: repeat_task.clone(),
        limits: ReasoningLimits {
            max_run_elapsed_ms: 60_000,
            max_steps: 4,
            max_step_elapsed_ms: 5_000,
        },
        repetition_policy: Some(TaskRepetitionPolicy {
            max_equivalent_attempts: 1,
        }),
    };
    client
        .configure_task_budget(session, repeat_config.clone(), mutation("repeat-config"))
        .await
        .unwrap();
    let mut repeat_request = ExecuteTaskEmbedding {
        task_id: repeat_task.clone(),
        ..execute
    };
    client
        .execute_task_embedding(session, repeat_request.clone(), mutation("repeat-a"))
        .await
        .unwrap();
    let original_bytes = repeat_request.request.inputs[0].bytes.clone();
    repeat_request.attempt_id = CorrelationId::new("different-source").unwrap();
    repeat_request.request.inputs[0].bytes.push(b'!');
    client
        .execute_task_embedding(session, repeat_request.clone(), mutation("repeat-b"))
        .await
        .unwrap();
    repeat_request.attempt_id = CorrelationId::new("renamed-attempt").unwrap();
    repeat_request.request.inputs[0].id = CanonicalId::new("renamed-input").unwrap();
    repeat_request.request.inputs[0].bytes = original_bytes;
    let error = client
        .execute_task_embedding(session, repeat_request, mutation("repeat-a-again"))
        .await
        .unwrap_err();
    assert!(
        matches!(error, rrd_client::Error::Api { error, .. } if error.code == ErrorCode::FailedPrecondition)
    );
    let repeat_read = ReadTaskBudget {
        scope: scope.clone(),
        task_id: repeat_task,
    };
    let repeat_stopped = client
        .read_task_budget(session, repeat_read.clone(), read("repeat-stopped"))
        .await
        .unwrap();
    assert_eq!(
        repeat_stopped.budget.as_ref().unwrap().stopped,
        Some(ReasoningBudgetStop::RepeatedWork)
    );
    assert_eq!(repeat_stopped.budget.as_ref().unwrap().reserved_steps, 2);
    assert_eq!(
        repeat_stopped
            .budget
            .as_ref()
            .unwrap()
            .equivalent_attempts
            .len(),
        2
    );
    drop(client);
    shutdown.send(()).unwrap();
    running.await.unwrap().unwrap();

    let engine = RrdEngine::open_existing(root.path(), instance.clone(), [0x6b; 32]).unwrap();
    // No backend is activated after reopen: replay must be a durable read.
    let server = RrdHttpServer::bind(engine, "127.0.0.1:0".parse().unwrap()).unwrap();
    let client =
        RrdClient::connect_local(server.local_addr(), instance, ClientConfig::default()).unwrap();
    let (shutdown, receiver) = tokio::sync::oneshot::channel();
    let running = tokio::spawn(server.serve_until(async move {
        let _ = receiver.await;
    }));
    assert_eq!(
        client
            .read_task_attempt(session, attempt, read("reopened-receipt"))
            .await
            .unwrap(),
        receipt
    );
    assert_eq!(
        client
            .read_task_budget(session, read_budget, read("reopened-budget"))
            .await
            .unwrap(),
        stopped
    );
    assert_eq!(
        client
            .configure_task_budget(session, config, mutation("reopened-configure"))
            .await
            .unwrap(),
        stopped
    );
    assert_eq!(
        client
            .stop_task_budget(session, stop, mutation("reopened-stop"))
            .await
            .unwrap(),
        stopped
    );
    assert_eq!(
        client
            .read_task_budget(session, repeat_read, read("reopened-repetition"))
            .await
            .unwrap(),
        repeat_stopped
    );
    assert_eq!(
        client
            .configure_task_budget(session, repeat_config, mutation("reopened-repeat-config"))
            .await
            .unwrap(),
        repeat_stopped
    );
    shutdown.send(()).unwrap();
    running.await.unwrap().unwrap();
}
