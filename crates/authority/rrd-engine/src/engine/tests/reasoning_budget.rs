use super::*;
use crate::engine::clock::{ClockSource, FixedClock};
use crate::engine::{
    EngineControlContext, ReasoningBudgetStop, TaskEmbeddingAttempt, TaskReasoningBudget,
};
use rrd_contract::{
    EmbeddingInput, EmbeddingNetworkPolicy, GenerateEmbeddings, OpenTask, SessionLease,
};
use rrd_inference::{
    EmbeddingBackend, EmbeddingBackendDescriptor, EmbeddingCancellation, EmbeddingRequest,
    FeatureHashBackend,
};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    mpsc, Arc, Barrier,
};
use std::time::{Duration, Instant};

const SCOPE: &str = "instance:test-instance";

mod completion;
mod lfg_flow;
mod proposal;
mod worker;

// Retained real task/request inputs in a fresh test engram. These are admission
// and persistence probes: no provider is installed or called, no model output
// is manufactured, and no record is admitted as training or evaluation data.
fn actual_request_fixture() -> (
    tempfile::TempDir,
    RrdEngine,
    SessionLease,
    CanonicalId,
    Vec<u8>,
) {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../transport/rrd-contract/tests/fixtures/endpoint-selection-observed/available");
    let task_bytes = std::fs::read(path.join("task.json")).unwrap();
    let task: serde_json::Value = serde_json::from_slice(&task_bytes).unwrap();
    let request = std::fs::read(path.join("choice-request.json")).unwrap();
    let hashes: serde_json::Value =
        serde_json::from_slice(&std::fs::read(path.join("files.json")).unwrap()).unwrap();
    assert_eq!(digest::sha256_hex(&request), hashes["choice-request.json"]);
    assert_eq!(digest::sha256_hex(&task_bytes), hashes["task.json"]);
    let request = super::observed_configuration::read(&path, "choice-request.json");
    let (root, mut engine) = isolated_engine();
    configure(&mut engine, 3);
    let lease = engine
        .create_session(
            &session_request(9_000, 4),
            &id("actual-input-session"),
            10,
            "session",
            "session",
        )
        .unwrap();
    let task_id = engine
        .open_task(
            &lease.session_id,
            &lease.token,
            &id("actual-input-task"),
            &OpenTask {
                scope: SCOPE.into(),
                title: task["title"].as_str().unwrap().into(),
                goal: task["body"].as_str().unwrap().into(),
            },
            20,
            "open",
            "open",
        )
        .unwrap()
        .task
        .task_id;
    (root, engine, lease, task_id, request)
}

#[test]
fn common_admission_of_actual_request_shares_budget_and_survives_reopen() {
    use crate::engine::reasoning_budget::TaskAttemptAdmission;
    let (root, engine, lease, task, bytes) = actual_request_fixture();
    let request_sha256 = digest::sha256_hex(&bytes);
    let attempt_id = id("actual-wire-reservation");
    let cancellation = EmbeddingCancellation::default();
    let input = TaskAttemptAdmission {
        started: Instant::now(),
        scope: SCOPE,
        task_id: &task,
        attempt_id: &attempt_id,
        request_sha256: &request_sha256,
        cancellation: &cancellation,
    };
    let context = EngineControlContext {
        now: 50,
        session_id: &lease.session_id,
        action: "test-reservation",
        request_id: "reserve",
        operation_id: "reserve",
    };
    let reserved = engine
        .reserve_admitted_task_attempt(
            &input,
            || panic!("no repetition policy: do not resolve an adapter"),
            &FixedClock::at(1_000),
            &context,
        )
        .unwrap();
    drop(reserved);
    let original = state(&engine, &lease, &task);
    assert_eq!(original.reserved_steps, 1);
    assert_eq!(original.active_attempt, Some(attempt_id.clone()));
    drop(engine);
    let mut engine = RrdEngine::open_existing(root.path(), instance(), TOKEN_KEY).unwrap();
    configure(&mut engine, 3);
    assert_eq!(state(&engine, &lease, &task), original);
    assert!(matches!(
        engine.reserve_admitted_task_attempt(
            &input,
            || panic!("replay must not resolve an adapter"),
            &FixedClock::at(1_100),
            &context
        ),
        Err(ServiceError::ReasoningAttemptRecorded {
            outcome_sha256: None
        })
    ));
    let mut changed = bytes.clone();
    changed.push(b' '); // Explicit in-memory identity perturbation, not a captured request.
    let changed_sha256 = digest::sha256_hex(&changed);
    let changed_input = TaskAttemptAdmission {
        request_sha256: &changed_sha256,
        ..input
    };
    assert!(matches!(
        engine.reserve_admitted_task_attempt(
            &changed_input,
            || panic!("conflicting replay must not resolve an adapter"),
            &FixedClock::at(1_100),
            &context
        ),
        Err(ServiceError::IdempotencyConflict)
    ));
    let wire: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let embedding = GenerateEmbeddings {
        scope: SCOPE.into(),
        backend_id: wire["model"].as_str().unwrap().into(),
        network_policy: EmbeddingNetworkPolicy::Deny,
        inputs: vec![EmbeddingInput {
            id: CanonicalId::new("retained-request").unwrap(),
            media_type: "application/json".into(),
            bytes,
        }],
    };
    assert!(matches!(
        call(
            &engine,
            &lease,
            &task,
            &embedding,
            "embedding-after-reservation",
            &FixedClock::at(1_100)
        ),
        Err(ServiceError::ReasoningBudgetStopped(
            ReasoningBudgetStop::UnresolvedAttempt
        ))
    ));
    assert_eq!(state(&engine, &lease, &task), original);
}

#[test]
fn common_admission_rejects_invalid_identities_without_budget_mutation() {
    use crate::engine::reasoning_budget::TaskAttemptAdmission;
    let (_root, engine, lease, task, bytes) = actual_request_fixture();
    let request_sha256 = digest::sha256_hex(&bytes);
    let attempt_id = id("identity-validation");
    let cancellation = EmbeddingCancellation::default();
    let context = EngineControlContext {
        now: 50,
        session_id: &lease.session_id,
        action: "test-reservation",
        request_id: "reserve",
        operation_id: "reserve",
    };
    let input = TaskAttemptAdmission {
        started: Instant::now(),
        scope: SCOPE,
        task_id: &task,
        attempt_id: &attempt_id,
        request_sha256: &request_sha256,
        cancellation: &cancellation,
    };
    let invalid = TaskAttemptAdmission {
        request_sha256: &request_sha256[..63],
        ..input
    };
    assert!(matches!(
        engine.reserve_admitted_task_attempt(
            &invalid,
            || panic!("malformed identity must not resolve an adapter"),
            &FixedClock::at(1_000),
            &context
        ),
        Err(ServiceError::Contract(_))
    ));
    assert!(engine
        .inspect_task_reasoning_budget(
            &lease.session_id,
            &lease.token,
            SCOPE,
            &task,
            60,
            "read",
            "read"
        )
        .unwrap()
        .is_none());
    let configured = repetition_budget(&engine, &lease, &task, 1);
    let clock = FixedClock::at(configured.started_at_unix_ms + 1);
    assert!(matches!(
        engine.reserve_admitted_task_attempt(
            &input,
            || Ok(request_sha256[..63].into()),
            &clock,
            &context
        ),
        Err(ServiceError::Contract(_))
    ));
    assert_eq!(state(&engine, &lease, &task), configured);
    assert!(matches!(
        engine.reserve_admitted_task_attempt(
            &input,
            || Err(ServiceError::Inference("capability unavailable".into())),
            &clock,
            &context
        ),
        Err(ServiceError::Inference(_))
    ));
    assert_eq!(state(&engine, &lease, &task), configured);
    engine
        .reserve_admitted_task_attempt(&input, || Ok(request_sha256.clone()), &clock, &context)
        .unwrap();
    let reserved = state(&engine, &lease, &task);
    assert_eq!(reserved.reserved_steps, 1);
    assert_eq!(reserved.equivalent_attempts.get(&request_sha256), Some(&1));
    assert_eq!(reserved.deadline_unix_ms, configured.deadline_unix_ms);
}

fn repetition_budget(
    engine: &RrdEngine,
    lease: &SessionLease,
    task: &CanonicalId,
    maximum: u64,
) -> TaskReasoningBudget {
    engine
        .configure_task_budget(
            &lease.session_id,
            &lease.token,
            &rrd_contract::ConfigureTaskBudget {
                scope: SCOPE.into(),
                task_id: task.clone(),
                limits: engine.engram_configuration.reasoning.clone(),
                repetition_policy: Some(rrd_contract::TaskRepetitionPolicy {
                    max_equivalent_attempts: maximum,
                }),
            },
            30,
            "repetition-config",
            "repetition-config",
        )
        .unwrap()
        .budget
        .unwrap()
}

#[test]
fn repetition_denies_cycles_and_renamed_reordered_inputs_after_reopen() {
    let (root, engine, lease, task, mut request, calls) = fixture(6);
    request.inputs.push(EmbeddingInput {
        id: CanonicalId::new("second").unwrap(),
        media_type: "text/plain".into(),
        bytes: b"another source".to_vec(),
    });
    let configured = repetition_budget(&engine, &lease, &task, 1);
    let clock = FixedClock::at(configured.started_at_unix_ms + 1);
    let first = call(&engine, &lease, &task, &request, "first", &clock).unwrap();
    let mut changed = request.clone();
    changed.inputs[0].bytes.push(b'!');
    call(&engine, &lease, &task, &changed, "changed-source", &clock).unwrap();
    let inner = FeatureHashBackend::new(32, 45).unwrap();
    let mut descriptor = inner.descriptor().clone();
    descriptor.id = "test:changed-model:cpu:v1".into();
    let mut changed_model = request.clone();
    changed_model.backend_id = descriptor.id.clone();
    engine
        .install_embedding_backend(CountedBackend {
            inner,
            descriptor: Some(descriptor),
            calls: calls.clone(),
            blocked: None,
        })
        .unwrap();
    call(
        &engine,
        &lease,
        &task,
        &changed_model,
        "changed-model",
        &clock,
    )
    .unwrap();
    assert_eq!(state(&engine, &lease, &task).equivalent_attempts.len(), 3);
    drop(engine);
    let mut engine = RrdEngine::open_existing(root.path(), instance(), TOKEN_KEY).unwrap();
    configure(&mut engine, 6);
    let inner = FeatureHashBackend::new(32, 44).unwrap();
    let mut descriptor = inner.descriptor().clone();
    descriptor.id = "test:renamed-backend:cpu:v1".into();
    engine
        .install_embedding_backend(CountedBackend {
            inner,
            descriptor: Some(descriptor),
            calls: calls.clone(),
            blocked: None,
        })
        .unwrap();
    let lease = engine
        .create_session(
            &session_request(9_000, 4),
            &id("reconnected"),
            40,
            "reconnected",
            "reconnected",
        )
        .unwrap();
    let receipt = engine
        .read_task_attempt(
            &lease.session_id,
            &lease.token,
            &rrd_contract::ReadTaskAttempt {
                scope: SCOPE.into(),
                task_id: task.clone(),
                attempt_id: id("first"),
            },
            45,
            "receipt",
            "receipt",
        )
        .unwrap();
    assert_eq!(receipt.attempt.unwrap().result, Some(first));
    assert!(matches!(
        call(&engine, &lease, &task, &request, "first", &clock),
        Err(ServiceError::ReasoningAttemptRecorded { .. })
    ));
    request.backend_id = "test:renamed-backend:cpu:v1".into();
    request.inputs.reverse();
    for (i, input) in request.inputs.iter_mut().enumerate() {
        input.id = CanonicalId::new(format!("renamed-{i}")).unwrap();
    }
    assert!(matches!(
        call(&engine, &lease, &task, &request, "renamed-attempt", &clock),
        Err(ServiceError::ReasoningBudgetStopped(
            ReasoningBudgetStop::RepeatedWork
        ))
    ));
    let stopped = state(&engine, &lease, &task);
    assert_eq!(stopped.reserved_steps, 3);
    assert_eq!(stopped.deadline_unix_ms, configured.deadline_unix_ms);
    assert_eq!(calls.load(Ordering::SeqCst), 6);
    let mut policy_change = rrd_contract::ConfigureTaskBudget {
        scope: SCOPE.into(),
        task_id: task.clone(),
        limits: configured.limits,
        repetition_policy: None,
    };
    assert!(matches!(
        engine.configure_task_budget(
            &lease.session_id,
            &lease.token,
            &policy_change,
            61,
            "disable",
            "disable"
        ),
        Err(ServiceError::IdempotencyConflict)
    ));
    policy_change.repetition_policy = Some(rrd_contract::TaskRepetitionPolicy {
        max_equivalent_attempts: 2,
    });
    assert!(matches!(
        engine.configure_task_budget(
            &lease.session_id,
            &lease.token,
            &policy_change,
            62,
            "raise",
            "raise"
        ),
        Err(ServiceError::IdempotencyConflict)
    ));
    drop(engine);
    let mut engine = RrdEngine::open_existing(root.path(), instance(), TOKEN_KEY).unwrap();
    configure(&mut engine, 6);
    assert_eq!(state(&engine, &lease, &task), stopped);
}

#[test]
fn repetition_allowance_is_not_double_spent_by_competing_workers() {
    let (_root, engine, lease, task, request, calls) = fixture(4);
    let configured = repetition_budget(&engine, &lease, &task, 1);
    let clock = FixedClock::at(configured.started_at_unix_ms + 1);
    let barrier = Barrier::new(2);
    let results = std::thread::scope(|threads| {
        let run = |key| {
            barrier.wait();
            call(&engine, &lease, &task, &request, key, &clock)
        };
        let first = threads.spawn(move || run("one"));
        let second = threads.spawn(move || run("two"));
        [first.join().unwrap(), second.join().unwrap()]
    });
    assert_eq!(
        results.iter().filter(|r| r.is_ok()).count(),
        1,
        "{results:?}"
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(matches!(
        call(&engine, &lease, &task, &request, "third", &clock),
        Err(ServiceError::ReasoningBudgetStopped(
            ReasoningBudgetStop::RepeatedWork
        ))
    ));
    assert_eq!(state(&engine, &lease, &task).reserved_steps, 1);
}

#[test]
fn repetition_history_is_bounded_without_evicting_old_identities() {
    let (root, mut engine, lease, task, mut request, calls) = fixture(130);
    engine.engram_configuration.reasoning.max_run_elapsed_ms = 60_000;
    engine.engram_configuration.configuration_sha256 =
        rrd_contract::engram_configuration_sha256(&engine.engram_configuration).unwrap();
    let configured = repetition_budget(&engine, &lease, &task, 2);
    let clock = FixedClock::at(configured.started_at_unix_ms + 1);
    for index in 0..128 {
        // One token guarantees a nonzero FeatureHash vector. Multi-token
        // cancellation is a separate adapter deficiency, not this oracle.
        request.inputs[0].bytes = format!("source{index}").into_bytes();
        call(
            &engine,
            &lease,
            &task,
            &request,
            &format!("attempt-{index}"),
            &clock,
        )
        .unwrap();
    }
    request.inputs[0].bytes = b"source0".to_vec();
    call(
        &engine,
        &lease,
        &task,
        &request,
        "existing-remainder",
        &clock,
    )
    .unwrap();
    request.inputs[0].bytes = b"source128".to_vec();
    assert!(matches!(
        call(&engine, &lease, &task, &request, "over-capacity", &clock),
        Err(ServiceError::ReasoningBudgetStopped(
            ReasoningBudgetStop::RepetitionTrackingLimit
        ))
    ));
    let stopped = state(&engine, &lease, &task);
    assert_eq!(stopped.equivalent_attempts.len(), 128);
    assert_eq!(stopped.reserved_steps, 129);
    assert_eq!(calls.load(Ordering::SeqCst), 129);
    drop(engine);
    let mut engine = RrdEngine::open_existing(root.path(), instance(), TOKEN_KEY).unwrap();
    configure(&mut engine, 130);
    engine.engram_configuration.reasoning.max_run_elapsed_ms = 60_000;
    engine.engram_configuration.configuration_sha256 =
        rrd_contract::engram_configuration_sha256(&engine.engram_configuration).unwrap();
    assert_eq!(
        engine.engram_configuration.configuration_sha256,
        stopped.configuration_sha256
    );
    engine.installed_engram = Some(rrd_contract::InstalledEngramIdentity {
        project_id: CanonicalId::new("tracking-limit-project").unwrap(),
        engram_id: CanonicalId::new("tracking-limit-engram").unwrap(),
        instance_id: engine.instance.clone(),
    });

    let authorization_at = stopped.last_observed_at_unix_ms + 1;
    assert!(authorization_at < stopped.deadline_unix_ms);
    let principal_id = engine.instance.clone();
    let credential = id("tracking-limit-resume-test-credential");
    let principal = rrd_security::Principal {
        id: principal_id.clone(),
        kind: rrd_security::PrincipalKind::Service,
        credential_sha256: digest::sha256_hex(credential.as_str().as_bytes()),
        credential_revision: 1,
        not_before_unix_ms: 1,
        expires_at_unix_ms: u64::MAX,
        disabled: false,
        role_ids: Default::default(),
        grants: SecurityAction::ALL
            .into_iter()
            .map(|action| rrd_security::ResourceGrant {
                action,
                resource_prefix: engine.instance_resource(),
                data_policy: None,
            })
            .collect(),
    };
    rrd_security::SecurityRepository::new(&engine.storage, engine.instance.clone())
        .initialize(
            rrd_security::SecurityState {
                format_version: rrd_security::SECURITY_FORMAT,
                revision: 1,
                principals: [(principal_id.clone(), principal)].into_iter().collect(),
                roles: Default::default(),
                identity_bindings: Default::default(),
                jwt_issuers: Default::default(),
            },
            authorization_at,
            "tracking-limit-test-authority",
            "tracking-limit-test-authority",
            "tracking-limit-test-authority",
        )
        .unwrap();
    let authorized_lease = engine
        .create_authenticated_session(
            &principal_id,
            credential.as_str().as_bytes(),
            &session_request(9_000, 4),
            &id("tracking-limit-resume-session"),
            authorization_at,
            "tracking-limit-resume-session",
            "tracking-limit-resume-session",
        )
        .unwrap();
    let read_request = rrd_contract::ReadTaskBudget {
        scope: SCOPE.into(),
        task_id: task.clone(),
    };
    let public_before = engine
        .read_task_budget(
            &authorized_lease.session_id,
            &authorized_lease.token,
            &read_request,
            authorization_at + 1,
            "tracking-limit-resume-budget-before",
            "tracking-limit-resume-budget-before",
        )
        .unwrap();
    assert_eq!(public_before.budget, Some(stopped.clone()));
    let expected_budget_sha256 = public_before.budget_sha256.clone().unwrap();
    let resume = engine.resume_task_budget_with_clock(
        &authorized_lease.session_id,
        &authorized_lease.token,
        &rrd_contract::ResumeTaskBudget {
            scope: SCOPE.into(),
            task_id: task.clone(),
            expected_budget_sha256,
        },
        "tracking-limit-resume",
        "tracking-limit-resume",
        &FixedClock::at(authorization_at + 2),
    );
    assert!(
        matches!(
            resume,
            Err(ServiceError::ReasoningBudgetStopped(
                ReasoningBudgetStop::RepetitionTrackingLimit
            ))
        ),
        "tracking-limit resume returned {resume:?}"
    );
    let public_after = engine
        .read_task_budget(
            &authorized_lease.session_id,
            &authorized_lease.token,
            &read_request,
            authorization_at + 3,
            "tracking-limit-resume-budget-after",
            "tracking-limit-resume-budget-after",
        )
        .unwrap();
    assert_eq!(public_after, public_before);
    assert_eq!(calls.load(Ordering::SeqCst), 129);
}

#[test]
fn public_budget_is_immutable_narrowed_and_stop_survives_provider_completion() {
    use rrd_contract::{
        ConfigureTaskBudget, ExecuteTaskEmbedding, ReadTaskAttempt, ReadTaskBudget, StopTaskBudget,
        TaskAttemptStatus,
    };
    let (root, engine, lease, task, mut request, calls) = fixture(3);
    let config = ConfigureTaskBudget {
        repetition_policy: None,
        scope: SCOPE.into(),
        task_id: task.clone(),
        limits: rrd_contract::ReasoningLimits {
            max_run_elapsed_ms: 9_000,
            max_steps: 2,
            max_step_elapsed_ms: 4_000,
        },
    };
    let configured = engine
        .configure_task_budget(
            &lease.session_id,
            &lease.token,
            &config,
            30,
            "configure",
            "configure",
        )
        .unwrap();
    assert_eq!(
        engine
            .configure_task_budget(
                &lease.session_id,
                &lease.token,
                &config,
                31,
                "again",
                "again"
            )
            .unwrap(),
        configured
    );
    let mut changed = config.clone();
    changed.limits.max_steps = 3;
    assert!(matches!(
        engine.configure_task_budget(
            &lease.session_id,
            &lease.token,
            &changed,
            32,
            "change",
            "change"
        ),
        Err(ServiceError::IdempotencyConflict)
    ));
    changed.limits.max_steps = 4;
    assert!(matches!(
        engine.configure_task_budget(
            &lease.session_id,
            &lease.token,
            &changed,
            33,
            "exceed",
            "exceed"
        ),
        Err(ServiceError::ConfigurationLimit(_))
    ));
    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let inner = FeatureHashBackend::new(32, 45).unwrap();
    let mut descriptor = inner.descriptor().clone();
    descriptor.id = "test:public-stop:cpu:v1".into();
    request.backend_id = descriptor.id.clone();
    engine
        .install_embedding_backend(CountedBackend {
            inner,
            descriptor: Some(descriptor),
            calls: calls.clone(),
            blocked: Some((entered_tx, release_rx)),
        })
        .unwrap();
    let execute = ExecuteTaskEmbedding {
        task_id: task.clone(),
        attempt_id: id("public-blocked"),
        request,
    };
    let read = ReadTaskBudget {
        scope: SCOPE.into(),
        task_id: task.clone(),
    };
    let stop = std::thread::scope(|threads| {
        let (done_tx, done_rx) = mpsc::channel();
        let (engine, lease, execute) = (&engine, &lease, &execute);
        threads.spawn(move || {
            done_tx
                .send(engine.execute_task_embedding(
                    &lease.session_id,
                    &lease.token,
                    execute,
                    40,
                    "execute",
                    "execute",
                ))
                .unwrap();
        });
        entered_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        let active = engine
            .read_task_budget(
                &lease.session_id,
                &lease.token,
                &read,
                50,
                "active",
                "active",
            )
            .unwrap();
        let stale = StopTaskBudget {
            scope: SCOPE.into(),
            task_id: task.clone(),
            expected_budget_sha256: configured.budget_sha256.clone().unwrap(),
        };
        assert!(matches!(
            engine.stop_task_budget(
                &lease.session_id,
                &lease.token,
                &stale,
                51,
                "stale",
                "stale"
            ),
            Err(ServiceError::IdempotencyConflict)
        ));
        let stop = StopTaskBudget {
            expected_budget_sha256: active.budget_sha256.unwrap(),
            ..stale
        };
        let stopped = engine
            .stop_task_budget(&lease.session_id, &lease.token, &stop, 52, "stop", "stop")
            .unwrap();
        assert_eq!(
            stopped.budget.unwrap().stopped,
            Some(ReasoningBudgetStop::Cancelled)
        );
        // Stop returns and the caller unwinds while non-cooperative work is
        // still blocked. Never pretend cancellation terminated that work.
        let result = done_rx.recv_timeout(Duration::from_secs(2));
        release_tx.send(()).unwrap();
        assert!(matches!(
            result.unwrap(),
            Err(ServiceError::InferenceCancelled {
                execution_may_continue: true
            })
        ));
        stop
    });
    let final_state = engine
        .read_task_budget(&lease.session_id, &lease.token, &read, 60, "final", "final")
        .unwrap();
    assert_eq!(
        final_state.budget.as_ref().unwrap().stopped,
        Some(ReasoningBudgetStop::Cancelled)
    );
    assert!(final_state.budget.as_ref().unwrap().execution_may_continue);
    assert_eq!(final_state.budget.as_ref().unwrap().reserved_steps, 1);
    assert_eq!(
        final_state.budget.as_ref().unwrap().deadline_unix_ms,
        configured.budget.unwrap().deadline_unix_ms
    );
    assert_eq!(
        engine
            .stop_task_budget(
                &lease.session_id,
                &lease.token,
                &stop,
                61,
                "retry-stop",
                "retry-stop"
            )
            .unwrap(),
        final_state
    );
    drop(engine);
    let mut engine = RrdEngine::open_existing(root.path(), instance(), TOKEN_KEY).unwrap();
    configure(&mut engine, 3);
    assert_eq!(
        engine
            .read_task_budget(
                &lease.session_id,
                &lease.token,
                &read,
                62,
                "reopen",
                "reopen"
            )
            .unwrap(),
        final_state
    );
    let attempt = engine
        .read_task_attempt(
            &lease.session_id,
            &lease.token,
            &ReadTaskAttempt {
                scope: SCOPE.into(),
                task_id: task,
                attempt_id: execute.attempt_id,
            },
            63,
            "receipt",
            "receipt",
        )
        .unwrap()
        .attempt
        .unwrap();
    assert_eq!(attempt.status, TaskAttemptStatus::Stopped);
    assert!(attempt.result.is_none());
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

struct CountedBackend {
    inner: FeatureHashBackend,
    descriptor: Option<EmbeddingBackendDescriptor>,
    calls: Arc<AtomicUsize>,
    blocked: Option<(mpsc::Sender<()>, mpsc::Receiver<()>)>,
}
impl EmbeddingBackend for CountedBackend {
    fn descriptor(&self) -> &EmbeddingBackendDescriptor {
        self.descriptor
            .as_ref()
            .unwrap_or_else(|| self.inner.descriptor())
    }
    fn embed(&mut self, request: &EmbeddingRequest) -> rrd_core::Result<rrd_core::VectorValue> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if let Some((entered, release)) = &self.blocked {
            entered.send(()).unwrap();
            release.recv_timeout(Duration::from_secs(10)).unwrap();
        }
        self.inner.embed(request)
    }
}

fn configure(engine: &mut RrdEngine, steps: u64) {
    engine.engram_configuration.reasoning = rrd_contract::ReasoningLimits {
        max_run_elapsed_ms: 10_000,
        max_steps: steps,
        max_step_elapsed_ms: 5_000,
    };
    engine.engram_configuration.configuration_sha256 =
        rrd_contract::engram_configuration_sha256(&engine.engram_configuration).unwrap();
}

fn fixture(
    steps: u64,
) -> (
    tempfile::TempDir,
    RrdEngine,
    SessionLease,
    CanonicalId,
    GenerateEmbeddings,
    Arc<AtomicUsize>,
) {
    let (root, mut engine) = isolated_engine();
    configure(&mut engine, steps);
    let lease = engine
        .create_session(
            &session_request(9_000, 4),
            &id("budget-session"),
            10,
            "create",
            "create",
        )
        .unwrap();
    let task = engine
        .open_task(
            &lease.session_id,
            &lease.token,
            &id("budget-task"),
            &OpenTask {
                scope: SCOPE.into(),
                title: "Bounded work".into(),
                goal: "Do not restart effort on reconnect".into(),
            },
            20,
            "open",
            "open",
        )
        .unwrap()
        .task
        .task_id;
    let calls = Arc::new(AtomicUsize::new(0));
    let backend = CountedBackend {
        inner: FeatureHashBackend::new(32, 44).unwrap(),
        descriptor: None,
        calls: calls.clone(),
        blocked: None,
    };
    let request = GenerateEmbeddings {
        scope: SCOPE.into(),
        backend_id: backend.descriptor().id.clone(),
        network_policy: EmbeddingNetworkPolicy::Deny,
        inputs: vec![EmbeddingInput {
            id: CanonicalId::new("input").unwrap(),
            media_type: "text/plain".into(),
            bytes: b"real task budget vector".to_vec(),
        }],
    };
    engine.install_embedding_backend(backend).unwrap();
    (root, engine, lease, task, request, calls)
}

fn call(
    engine: &RrdEngine,
    lease: &SessionLease,
    task: &CanonicalId,
    request: &GenerateEmbeddings,
    key: &str,
    clock: &dyn ClockSource,
) -> crate::Result<rrd_contract::GenerateEmbeddingsResult> {
    engine.generate_task_embeddings_with_clock(
        &lease.session_id,
        &lease.token,
        &TaskEmbeddingAttempt {
            task_id: task,
            attempt_id: &id(key),
            request,
            cancellation: &EmbeddingCancellation::default(),
        },
        50,
        key,
        key,
        clock,
    )
}

fn state(engine: &RrdEngine, lease: &SessionLease, task: &CanonicalId) -> TaskReasoningBudget {
    engine
        .inspect_task_reasoning_budget(
            &lease.session_id,
            &lease.token,
            SCOPE,
            task,
            60,
            "inspect",
            "inspect",
        )
        .unwrap()
        .unwrap()
}

#[test]
fn task_budget_counts_real_flow_and_keeps_original_allowance_after_reopen() {
    let (root, engine, lease, task, request, calls) = fixture(2);
    let first = call(
        &engine,
        &lease,
        &task,
        &request,
        "one",
        &FixedClock::at(1_000),
    )
    .unwrap();
    assert_eq!(first.embeddings.len(), 1);
    let digest = digest::sha256_hex(&serde_json::to_vec(&first).unwrap());
    assert!(
        matches!(call(&engine, &lease, &task, &request, "one", &FixedClock::at(1_010)),
        Err(ServiceError::ReasoningAttemptRecorded { outcome_sha256: Some(value) }) if value == digest)
    );
    let mut changed = request.clone();
    changed.inputs[0].bytes.push(b'!');
    assert!(matches!(
        call(
            &engine,
            &lease,
            &task,
            &changed,
            "one",
            &FixedClock::at(1_010)
        ),
        Err(ServiceError::IdempotencyConflict)
    ));
    let original = state(&engine, &lease, &task);
    assert_eq!(original.reserved_steps, 1);
    assert!(original.charged_elapsed_ms > 0);
    drop(engine);
    let mut engine = RrdEngine::open_existing(root.path(), instance(), TOKEN_KEY).unwrap();
    configure(&mut engine, 2);
    engine
        .install_embedding_backend(CountedBackend {
            inner: FeatureHashBackend::new(32, 44).unwrap(),
            descriptor: None,
            calls: calls.clone(),
            blocked: None,
        })
        .unwrap();
    assert_eq!(state(&engine, &lease, &task), original);
    call(
        &engine,
        &lease,
        &task,
        &request,
        "two",
        &FixedClock::at(1_100),
    )
    .unwrap();
    assert!(matches!(
        call(
            &engine,
            &lease,
            &task,
            &request,
            "three",
            &FixedClock::at(1_200)
        ),
        Err(ServiceError::ReasoningBudgetStopped(
            ReasoningBudgetStop::StepLimit
        ))
    ));
    let final_state = state(&engine, &lease, &task);
    assert_eq!(final_state.reserved_steps, 2);
    assert_eq!(final_state.deadline_unix_ms, original.deadline_unix_ms);
    assert_eq!(final_state.started_at_unix_ms, original.started_at_unix_ms);
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[test]
fn reserved_but_not_dispatched_attempt_remains_unresolved_after_reopen() {
    let (root, engine, lease, task, request, calls) = fixture(2);
    let attempt_id = id("interrupted");
    let cancel = EmbeddingCancellation::default();
    let reserved = engine
        .reserve_task_attempt(
            &TaskEmbeddingAttempt {
                task_id: &task,
                attempt_id: &attempt_id,
                request: &request,
                cancellation: &cancel,
            },
            &FixedClock::at(1_000),
            &EngineControlContext {
                now: 50,
                session_id: &lease.session_id,
                action: "test-reserve",
                request_id: "reserve",
                operation_id: "reserve",
            },
        )
        .unwrap();
    drop(reserved);
    let original = state(&engine, &lease, &task);
    drop(engine);
    let mut engine = RrdEngine::open_existing(root.path(), instance(), TOKEN_KEY).unwrap();
    configure(&mut engine, 2);
    assert_eq!(state(&engine, &lease, &task), original);
    assert!(matches!(
        call(
            &engine,
            &lease,
            &task,
            &request,
            "interrupted",
            &FixedClock::at(1_100)
        ),
        Err(ServiceError::ReasoningAttemptRecorded {
            outcome_sha256: None
        })
    ));
    assert!(matches!(
        call(
            &engine,
            &lease,
            &task,
            &request,
            "replacement",
            &FixedClock::at(1_100)
        ),
        Err(ServiceError::ReasoningBudgetStopped(
            ReasoningBudgetStop::UnresolvedAttempt
        ))
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_eq!(state(&engine, &lease, &task).reserved_steps, 1);
}

#[test]
fn concurrent_workers_cannot_spend_the_final_step_twice() {
    let (_root, engine, lease, task, request, calls) = fixture(1);
    let barrier = Barrier::new(2);
    let results = std::thread::scope(|threads| {
        let run = |key| {
            barrier.wait();
            call(
                &engine,
                &lease,
                &task,
                &request,
                key,
                &FixedClock::at(1_000),
            )
        };
        let first = threads.spawn(move || run("worker-one"));
        let second = threads.spawn(move || run("worker-two"));
        [first.join().unwrap(), second.join().unwrap()]
    });
    assert_eq!(
        results.iter().filter(|r| r.is_ok()).count(),
        1,
        "{results:?}"
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(state(&engine, &lease, &task).reserved_steps, 1);
}

#[test]
fn rollback_expiry_and_changed_configuration_are_sticky() {
    for reason in [
        ReasoningBudgetStop::ClockRollback,
        ReasoningBudgetStop::Deadline,
        ReasoningBudgetStop::ConfigurationChanged,
    ] {
        let (_root, mut engine, lease, task, request, calls) = fixture(3);
        call(
            &engine,
            &lease,
            &task,
            &request,
            "initial",
            &FixedClock::at(1_000),
        )
        .unwrap();
        if reason == ReasoningBudgetStop::ConfigurationChanged {
            configure(&mut engine, 4);
        }
        let at = match reason {
            ReasoningBudgetStop::ClockRollback => 999,
            ReasoningBudgetStop::Deadline => 11_000,
            _ => 1_100,
        };
        assert!(
            matches!(call(&engine, &lease, &task, &request, "denied", &FixedClock::at(at)), Err(ServiceError::ReasoningBudgetStopped(value)) if value == reason)
        );
        // A later good clock/configuration cannot clear a recorded stop.
        configure(&mut engine, 3);
        assert!(
            matches!(call(&engine, &lease, &task, &request, "later", &FixedClock::at(2_000)), Err(ServiceError::ReasoningBudgetStopped(value)) if value == reason)
        );
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(state(&engine, &lease, &task).reserved_steps, 1);
    }
}

#[test]
fn completion_clock_failure_withholds_result_and_does_not_repeat_provider() {
    struct LostClock(AtomicUsize);
    impl ClockSource for LostClock {
        fn observe(
            &self,
        ) -> std::result::Result<crate::ClockObservation, crate::ClockObservationFailure> {
            if self.0.fetch_add(1, Ordering::SeqCst) == 0 {
                FixedClock::at(1_000).observe()
            } else {
                Err(crate::ClockObservationFailure::Unrepresentable)
            }
        }
    }
    let (_root, engine, lease, task, request, calls) = fixture(3);
    assert!(matches!(
        call(
            &engine,
            &lease,
            &task,
            &request,
            "lost-result",
            &LostClock(AtomicUsize::new(0))
        ),
        Err(ServiceError::ClockUnavailable(_))
    ));
    assert!(matches!(
        call(
            &engine,
            &lease,
            &task,
            &request,
            "lost-result",
            &FixedClock::at(1_100)
        ),
        Err(ServiceError::ReasoningAttemptRecorded {
            outcome_sha256: None
        })
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        state(&engine, &lease, &task).active_attempt,
        Some(id("lost-result"))
    );
}

#[test]
fn inherited_remaining_run_deadline_bounds_a_blocked_provider() {
    interrupted_budgeted_provider(false);
}

#[test]
fn cancellation_persists_stop_and_uncertainty_for_running_work() {
    interrupted_budgeted_provider(true);
}

fn interrupted_budgeted_provider(cancel: bool) {
    let (_root, mut engine, lease, task, mut request, calls) = fixture(3);
    engine.engram_configuration.reasoning.max_run_elapsed_ms = 5_000;
    engine.engram_configuration.reasoning.max_step_elapsed_ms = 5_000;
    engine.engram_configuration.configuration_sha256 =
        rrd_contract::engram_configuration_sha256(&engine.engram_configuration).unwrap();
    call(
        &engine,
        &lease,
        &task,
        &request,
        "first",
        &FixedClock::at(1_000),
    )
    .unwrap();
    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let inner = FeatureHashBackend::new(32, 45).unwrap();
    let mut descriptor = inner.descriptor().clone();
    descriptor.id = "test:budget-blocked:cpu:v1".into();
    let backend = CountedBackend {
        inner,
        descriptor: Some(descriptor),
        calls: calls.clone(),
        blocked: Some((entered_tx, release_rx)),
    };
    request.backend_id = backend.descriptor().id.clone();
    engine.install_embedding_backend(backend).unwrap();
    std::thread::scope(|threads| {
        let (done_tx, done_rx) = mpsc::channel();
        let cancellation = EmbeddingCancellation::default();
        let caller_cancellation = cancellation.clone();
        let (engine, lease, task, request) = (&engine, &lease, &task, &request);
        threads.spawn(move || {
            done_tx
                .send(engine.generate_task_embeddings_with_clock(
                    &lease.session_id,
                    &lease.token,
                    &TaskEmbeddingAttempt {
                        task_id: task,
                        attempt_id: &id("blocked"),
                        request,
                        cancellation: &caller_cancellation,
                    },
                    50,
                    "blocked",
                    "blocked",
                    &FixedClock::at(if cancel { 1_100 } else { 5_500 }),
                ))
                .unwrap();
        });
        entered_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        if cancel {
            cancellation.cancel();
        }
        let before_release = done_rx.recv_timeout(Duration::from_secs(2));
        let held = engine.generate_embeddings(
            &lease.session_id,
            &lease.token,
            request,
            70,
            "busy",
            "busy",
        );
        release_tx.send(()).unwrap();
        let error = before_release.unwrap().unwrap_err();
        if cancel {
            assert!(matches!(
                error,
                ServiceError::InferenceCancelled {
                    execution_may_continue: true
                }
            ));
        } else {
            assert!(matches!(
                error,
                ServiceError::InferenceDeadlineExceeded {
                    execution_may_continue: true
                }
            ));
        }
        assert!(matches!(held, Err(ServiceError::InferenceBusy)));
    });
    assert_eq!(
        state(&engine, &lease, &task).stopped,
        Some(if cancel {
            ReasoningBudgetStop::Cancelled
        } else {
            ReasoningBudgetStop::Deadline
        })
    );
    assert!(state(&engine, &lease, &task).execution_may_continue);
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert!(matches!(
        call(
            &engine,
            &lease,
            &task,
            &request,
            "after-stop",
            &FixedClock::at(5_600)
        ),
        Err(ServiceError::ReasoningBudgetStopped(_))
    ));
}

#[test]
fn closed_tasks_and_foreign_scopes_cannot_allocate_a_budget() {
    let (_root, engine, lease, task, request, calls) = fixture(2);
    let mut foreign = request.clone();
    foreign.scope = "instance:foreign".into();
    assert!(matches!(
        call(
            &engine,
            &lease,
            &task,
            &foreign,
            "foreign",
            &FixedClock::at(1_000)
        ),
        Err(ServiceError::WrongScope)
    ));
    engine
        .close_task(
            &lease.session_id,
            &lease.token,
            &rrd_contract::CloseTask {
                scope: SCOPE.into(),
                task_id: task.clone(),
                status: rrd_contract::TaskStatus::Abandoned,
                outcome: None,
            },
            40,
            "close",
            "close",
        )
        .unwrap();
    assert!(matches!(
        call(
            &engine,
            &lease,
            &task,
            &request,
            "closed",
            &FixedClock::at(1_000)
        ),
        Err(ServiceError::TaskClosed)
    ));
    assert!(engine
        .inspect_task_reasoning_budget(
            &lease.session_id,
            &lease.token,
            SCOPE,
            &task,
            60,
            "read",
            "read"
        )
        .unwrap()
        .is_none());
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[test]
fn failed_attempt_is_charged_and_cannot_be_retried_under_a_new_id() {
    let (_root, engine, lease, task, request, calls) = fixture(3);
    let mut unavailable = request.clone();
    unavailable.backend_id = "test:unavailable:cpu:v1".into();
    assert!(call(
        &engine,
        &lease,
        &task,
        &unavailable,
        "failed",
        &FixedClock::at(1_000)
    )
    .is_err());
    let stopped = state(&engine, &lease, &task);
    assert_eq!(stopped.reserved_steps, 1);
    assert_eq!(stopped.stopped, Some(ReasoningBudgetStop::AttemptFailed));
    assert!(matches!(
        call(
            &engine,
            &lease,
            &task,
            &request,
            "replacement",
            &FixedClock::at(1_100)
        ),
        Err(ServiceError::ReasoningBudgetStopped(
            ReasoningBudgetStop::AttemptFailed
        ))
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[test]
fn cancelled_or_unauthenticated_attempts_do_not_dispatch() {
    let (_root, engine, lease, task, request, calls) = fixture(3);
    let cancelled = EmbeddingCancellation::default();
    cancelled.cancel();
    let input = TaskEmbeddingAttempt {
        task_id: &task,
        attempt_id: &id("cancelled"),
        request: &request,
        cancellation: &cancelled,
    };
    assert!(engine
        .generate_task_embeddings(
            &lease.session_id,
            &id("invalid-token"),
            &input,
            50,
            "bad",
            "bad"
        )
        .is_err());
    assert!(engine
        .inspect_task_reasoning_budget(
            &lease.session_id,
            &lease.token,
            SCOPE,
            &task,
            60,
            "read",
            "read"
        )
        .unwrap()
        .is_none());
    assert!(matches!(
        engine.generate_task_embeddings_with_clock(
            &lease.session_id,
            &lease.token,
            &input,
            50,
            "cancel",
            "cancel",
            &FixedClock::at(1_000)
        ),
        Err(ServiceError::ReasoningBudgetStopped(
            ReasoningBudgetStop::Cancelled
        ))
    ));
    assert_eq!(state(&engine, &lease, &task).reserved_steps, 0);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}
