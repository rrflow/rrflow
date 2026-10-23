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
use std::time::Duration;

const SCOPE: &str = "instance:test-instance";

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
    engine.estate_configuration.reasoning = rrd_contract::ReasoningLimits {
        max_run_elapsed_ms: 10_000,
        max_steps: steps,
        max_step_elapsed_ms: 5_000,
    };
    engine.estate_configuration.configuration_sha256 =
        rrd_contract::estate_configuration_sha256(&engine.estate_configuration).unwrap();
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
    engine.estate_configuration.reasoning.max_run_elapsed_ms = 5_000;
    engine.estate_configuration.reasoning.max_step_elapsed_ms = 5_000;
    engine.estate_configuration.configuration_sha256 =
        rrd_contract::estate_configuration_sha256(&engine.estate_configuration).unwrap();
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
