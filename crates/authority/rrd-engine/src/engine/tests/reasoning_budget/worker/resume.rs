//! Engine control probes using retained task inputs; no generated model output,
//! provider call, training record or routing-quality claim.
use super::*;
use crate::engine::reasoning_budget::TaskAttemptAdmission;
use rrd_contract::{ResumeTaskBudget, TaskBudgetView};

fn stopped(
    steps: u64,
    cancelled: bool,
    uncertain: bool,
) -> (
    tempfile::TempDir,
    RrdEngine,
    SessionLease,
    EndpointSelectionRequest,
) {
    // No dispatch is performed. The configured URL cannot be reached by these
    // direct budget admission/completion probes.
    let (root, mut engine, lease, request) = admitted_fixture("http://127.0.0.1:1/v1");
    configure(&mut engine, steps);
    engine.engram_configuration.reasoning.max_run_elapsed_ms = 2_000;
    engine.engram_configuration.reasoning.max_step_elapsed_ms = 1_000;
    engine.engram_configuration.configuration_sha256 =
        rrd_contract::engram_configuration_sha256(&engine.engram_configuration).unwrap();
    let cancellation = EmbeddingCancellation::default();
    let context = context(&lease);
    let reservation = engine
        .reserve_admitted_task_attempt(
            &input(
                &request,
                &cancellation,
                &operation_digest(&request).unwrap(),
            ),
            || panic!("no repetition policy"),
            &FixedClock::at(1_000),
            &context,
        )
        .unwrap();
    if cancelled {
        cancellation.cancel();
    }
    assert!(engine
        .finish_admitted_task_attempt(
            reservation,
            Err(ServiceError::EndpointCallFailed {
                reason: "controlled completion fault",
                execution_may_continue: uncertain,
            }),
            |_| Ok(Vec::new()),
            &FixedClock::at(1_100),
            &context,
        )
        .is_err());
    (root, engine, lease, request)
}

fn context(lease: &SessionLease) -> EngineControlContext<'_> {
    EngineControlContext {
        now: 1_100,
        session_id: &lease.session_id,
        action: "resume-control-probe",
        request_id: "probe",
        operation_id: "probe",
    }
}

fn input<'a>(
    request: &'a EndpointSelectionRequest,
    cancellation: &'a EmbeddingCancellation,
    digest: &'a str,
) -> TaskAttemptAdmission<'a> {
    TaskAttemptAdmission {
        started: Instant::now(),
        scope: &request.scope,
        task_id: &request.task_id,
        attempt_id: &request.attempt_id,
        request_sha256: digest,
        cancellation,
    }
}

fn resume_request(
    engine: &RrdEngine,
    lease: &SessionLease,
    task: &CanonicalId,
) -> ResumeTaskBudget {
    ResumeTaskBudget {
        scope: SCOPE.into(),
        task_id: task.clone(),
        expected_budget_sha256: operation_digest(&budget(engine, lease, task)).unwrap(),
    }
}

fn resume(
    engine: &RrdEngine,
    lease: &SessionLease,
    request: &ResumeTaskBudget,
    at: u64,
) -> crate::Result<TaskBudgetView> {
    engine.resume_task_budget_with_clock(
        &lease.session_id,
        &lease.token,
        request,
        "resume",
        "resume",
        &FixedClock::at(at),
    )
}

#[test]
fn resume_preserves_effort_and_attempts_and_allows_only_a_new_admitted_step() {
    for cancelled in [false, true] {
        let (root, engine, lease, request) = stopped(8, cancelled, false);
        let identity = engine.installed_engram.clone();
        let configuration = engine.engram_configuration.clone();
        let mut expected = budget(&engine, &lease, &request.task_id);
        assert_eq!(
            expected.stopped,
            Some(if cancelled {
                ReasoningBudgetStop::Cancelled
            } else {
                ReasoningBudgetStop::AttemptFailed
            })
        );
        let attempt_read = ReadTaskAttempt {
            scope: request.scope.clone(),
            task_id: request.task_id.clone(),
            attempt_id: request.attempt_id.clone(),
        };
        let retained = engine
            .read_task_attempt(
                &lease.session_id,
                &lease.token,
                &attempt_read,
                1_100,
                "read",
                "read",
            )
            .unwrap();
        let command = resume_request(&engine, &lease, &request.task_id);
        let result = resume(&engine, &lease, &command, 1_200).unwrap();
        command.validate_response(&result).unwrap();
        expected.stopped = None;
        expected.stop_expected_sha256 = None;
        expected.last_observed_at_unix_ms = 1_200;
        assert_eq!(result.budget, Some(expected.clone()));
        assert!(matches!(
            resume(&engine, &lease, &command, 1_200),
            Err(ServiceError::IdempotencyConflict)
        ));
        drop(engine);
        let mut engine = RrdEngine::open_existing(root.path(), instance(), TOKEN_KEY).unwrap();
        engine.installed_engram = identity;
        engine.engram_configuration = configuration;
        assert_eq!(budget(&engine, &lease, &request.task_id), expected);
        assert_eq!(
            engine
                .read_task_attempt(
                    &lease.session_id,
                    &lease.token,
                    &attempt_read,
                    1_300,
                    "read",
                    "read"
                )
                .unwrap(),
            retained
        );
        let cancellation = EmbeddingCancellation::default();
        assert!(matches!(
            engine.reserve_admitted_task_attempt(
                &input(
                    &request,
                    &cancellation,
                    &operation_digest(&request).unwrap()
                ),
                || panic!("old attempt cannot resolve an adapter"),
                &FixedClock::at(1_300),
                &context(&lease)
            ),
            Err(ServiceError::ReasoningAttemptRecorded { .. })
        ));
        let mut next = request.clone();
        next.attempt_id = id("explicit-new-attempt");
        engine
            .reserve_admitted_task_attempt(
                &input(&next, &cancellation, &operation_digest(&next).unwrap()),
                || panic!("no repetition policy"),
                &FixedClock::at(1_300),
                &context(&lease),
            )
            .unwrap();
        let after = budget(&engine, &lease, &request.task_id);
        assert_eq!(after.reserved_steps, expected.reserved_steps + 1);
        assert_eq!(after.deadline_unix_ms, expected.deadline_unix_ms);
        assert_eq!(after.charged_elapsed_ms, expected.charged_elapsed_ms);
        assert_eq!(after.active_attempt, Some(next.attempt_id));
        assert!(matches!(
            resume(
                &engine,
                &lease,
                &resume_request(&engine, &lease, &request.task_id),
                1_400
            ),
            Err(ServiceError::ReasoningBudgetStopped(
                ReasoningBudgetStop::UnresolvedAttempt
            ))
        ));
        assert_eq!(budget(&engine, &lease, &request.task_id), after);
    }
}

#[test]
fn resume_denials_do_not_mutate_stopped_budgets() {
    for case in [
        "unknown",
        "steps",
        "deadline",
        "rollback",
        "configuration",
        "token",
        "closed",
        "stale",
    ] {
        let (_root, mut engine, lease, request) = stopped(
            if case == "steps" { 1 } else { 8 },
            false,
            case == "unknown",
        );
        let mut command = resume_request(&engine, &lease, &request.task_id);
        let original = budget(&engine, &lease, &request.task_id);
        let mut caller = lease.clone();
        let at = match case {
            "deadline" => original.deadline_unix_ms,
            "rollback" => original.last_observed_at_unix_ms - 1,
            _ => 1_200,
        };
        if case == "configuration" {
            configure(&mut engine, 7);
        }
        if case == "token" {
            caller.token = id("invalid-token");
        }
        if case == "stale" {
            command.expected_budget_sha256 = "0".repeat(64);
        }
        if case == "closed" {
            engine
                .close_task(
                    &lease.session_id,
                    &lease.token,
                    &rrd_contract::CloseTask {
                        scope: SCOPE.into(),
                        task_id: request.task_id.clone(),
                        status: rrd_contract::TaskStatus::Abandoned,
                        outcome: None,
                    },
                    1_150,
                    "close",
                    "close",
                )
                .unwrap();
        }
        let error = resume(&engine, &caller, &command, at).unwrap_err();
        match case {
            "unknown" => assert!(matches!(
                error,
                ServiceError::ReasoningBudgetStopped(ReasoningBudgetStop::UnresolvedAttempt)
            )),
            "steps" => assert!(matches!(
                error,
                ServiceError::ReasoningBudgetStopped(ReasoningBudgetStop::StepLimit)
            )),
            "deadline" => assert!(matches!(
                error,
                ServiceError::ReasoningBudgetStopped(ReasoningBudgetStop::Deadline)
            )),
            "rollback" => assert!(matches!(
                error,
                ServiceError::ReasoningBudgetStopped(ReasoningBudgetStop::ClockRollback)
            )),
            "configuration" => assert!(matches!(
                error,
                ServiceError::ReasoningBudgetStopped(ReasoningBudgetStop::ConfigurationChanged)
            )),
            "closed" => assert!(matches!(error, ServiceError::TaskClosed)),
            "stale" => assert!(matches!(error, ServiceError::IdempotencyConflict)),
            "token" => {}
            _ => unreachable!(),
        }
        assert_eq!(
            budget(&engine, &lease, &request.task_id),
            original,
            "{case}"
        );
    }
}
