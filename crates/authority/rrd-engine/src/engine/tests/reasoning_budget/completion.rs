//! Bookkeeping/freshness tests in isolated engine engrams. The supplied output
//! tuple projects retained provider bytes onto a fresh task solely to test the
//! ledger; it is not evidence of a provider call for that packet, nor learning data.
use super::*;
use crate::engine::endpoint_proposal::capacity::CapacityLease;
use crate::engine::reasoning_budget::{
    AttemptAuthorityGuard, Reservation, TaskAttemptAdmission, TaskAttemptOutput,
    TaskCompletionAdmission,
};
use crate::engine::{operation_digest, Result};
use rrd_contract::{ReadTaskAttempt, TaskAttemptInvalidation, TaskAttemptStatus};

fn context(lease: &SessionLease) -> EngineControlContext<'_> {
    EngineControlContext {
        now: 50,
        session_id: &lease.session_id,
        action: "test-proposal-completion",
        request_id: "proposal-ledger",
        operation_id: "proposal-ledger",
    }
}

fn reserve(
    engine: &RrdEngine,
    lease: &SessionLease,
    request: &rrd_contract::EndpointSelectionRequest,
    cancellation: &EmbeddingCancellation,
    at: u64,
) -> Result<Reservation> {
    engine.reserve_admitted_task_attempt(
        &TaskAttemptAdmission {
            started: Instant::now(),
            scope: &request.scope,
            task_id: &request.task_id,
            attempt_id: &request.attempt_id,
            request_sha256: &operation_digest(request).unwrap(),
            cancellation,
        },
        || panic!("no repetition policy or adapter expected"),
        &FixedClock::at(at),
        &context(lease),
    )
}

fn read(
    engine: &RrdEngine,
    lease: &SessionLease,
    request: &rrd_contract::EndpointSelectionRequest,
) -> rrd_contract::TaskAttemptView {
    engine
        .read_task_attempt(
            &lease.session_id,
            &lease.token,
            &ReadTaskAttempt {
                scope: request.scope.clone(),
                task_id: request.task_id.clone(),
                attempt_id: request.attempt_id.clone(),
            },
            100,
            "read-proposal-ledger",
            "read-proposal-ledger",
        )
        .unwrap()
}

// Synthetic data tests the durable ledger only; it is not an LFG service result.
fn lfg_advisory(request: &rrd_contract::EndpointSelectionRequest) -> rrd_contract::LfgAdvisory {
    rrd_contract::LfgAdvisory {
        request_id: request.attempt_id.clone(),
        revision: 1,
        input_sha256: "a".repeat(64),
        assessment_sha256: "b".repeat(64),
        rules_sha256: "c".repeat(64),
        request_sha256: "d".repeat(64),
        response_sha256: "e".repeat(64),
        status: "available".into(),
        intent_status: "inconclusive".into(),
        intent_category: "unknown".into(),
        suggested_role: "general".into(),
        suggested_effort: "normal".into(),
        reason_ids: vec!["synthetic".into()],
        elapsed_ms: 1,
    }
}

#[test]
fn lfg_checkpoint_survives_interruption_and_refuses_replay() {
    let (root, engine, lease, _, request, _, _) = proposal::tuple_fixture();
    let cancellation = EmbeddingCancellation::default();
    let mut reservation = reserve(&engine, &lease, &request, &cancellation, 1_000).unwrap();
    let advisory = lfg_advisory(&request);
    engine
        .checkpoint_lfg_advisory(
            &mut reservation,
            advisory.clone(),
            |_| Ok(TaskCompletionAdmission::default()),
            &FixedClock::at(1_001),
            &context(&lease),
        )
        .unwrap();
    let before = read(&engine, &lease, &request);
    assert_eq!(
        before.attempt.as_ref().unwrap().status,
        TaskAttemptStatus::Reserved
    );
    assert_eq!(before.attempt.as_ref().unwrap().lfg, Some(advisory));
    drop(reservation);
    drop(engine);
    let mut reopened = RrdEngine::open_existing(root.path(), instance(), TOKEN_KEY).unwrap();
    configure(&mut reopened, 3);
    assert_eq!(read(&reopened, &lease, &request), before);
    assert!(reserve(&reopened, &lease, &request, &cancellation, 1_100).is_err());
}

#[test]
fn lfg_checkpoint_guard_conflict_cannot_publish_and_failure_preserves_prior_checkpoint() {
    let (_root, engine, lease, _, request, _, _) = proposal::tuple_fixture();
    let cancellation = EmbeddingCancellation::default();
    let mut reservation = reserve(&engine, &lease, &request, &cancellation, 1_000).unwrap();
    let advisory = lfg_advisory(&request);
    let key = crate::engine::session::session_key(&engine.instance, &lease.session_id);
    assert!(engine
        .checkpoint_lfg_advisory(
            &mut reservation,
            advisory.clone(),
            |_| Ok(TaskCompletionAdmission {
                guards: vec![AttemptAuthorityGuard {
                    key: key.clone(),
                    expected: b"stale".to_vec()
                }],
                ..Default::default()
            }),
            &FixedClock::at(1_001),
            &context(&lease)
        )
        .is_err());
    assert!(read(&engine, &lease, &request)
        .attempt
        .unwrap()
        .lfg
        .is_none());
    engine
        .checkpoint_lfg_advisory(
            &mut reservation,
            advisory.clone(),
            |_| Ok(TaskCompletionAdmission::default()),
            &FixedClock::at(1_002),
            &context(&lease),
        )
        .unwrap();
    assert!(engine
        .finish_controlled_task_attempt(
            reservation,
            Err(ServiceError::EndpointCallFailed {
                reason: "synthetic controller failure",
                execution_may_continue: false,
            }),
            |_| Ok(TaskCompletionAdmission::default()),
            &FixedClock::at(1_003),
            &context(&lease)
        )
        .is_err());
    let stopped = read(&engine, &lease, &request).attempt.unwrap();
    assert_eq!(stopped.status, TaskAttemptStatus::Stopped);
    assert_eq!(stopped.lfg, Some(advisory));
    assert!(stopped.endpoint_proposal.is_none());
}

#[test]
fn stale_lfg_observation_is_recorded_then_successor_can_be_admitted() {
    let (_root, engine, lease, _, request, _, _) = proposal::tuple_fixture();
    let cancellation = EmbeddingCancellation::default();
    let mut reservation = reserve(&engine, &lease, &request, &cancellation, 1_000).unwrap();
    let advisory = lfg_advisory(&request);
    let invalidation = engine
        .checkpoint_lfg_advisory(
            &mut reservation,
            advisory.clone(),
            |_| {
                Ok(TaskCompletionAdmission {
                    invalidation: Some(TaskAttemptInvalidation::InputsChanged),
                    ..Default::default()
                })
            },
            &FixedClock::at(1_001),
            &context(&lease),
        )
        .unwrap();
    assert_eq!(invalidation, Some(TaskAttemptInvalidation::InputsChanged));
    assert!(matches!(
        engine.finish_controlled_task_attempt(
            reservation,
            Err(ServiceError::TaskAttemptInvalidated(invalidation.unwrap())),
            |_| Ok(TaskCompletionAdmission::default()),
            &FixedClock::at(1_002),
            &context(&lease)
        ),
        Err(ServiceError::TaskAttemptInvalidated(
            TaskAttemptInvalidation::InputsChanged
        ))
    ));
    let prior = read(&engine, &lease, &request).attempt.unwrap();
    assert_eq!(prior.status, TaskAttemptStatus::Invalidated);
    assert_eq!(prior.lfg, Some(advisory));
    let mut successor = request.clone();
    successor.attempt_id = id("lfg-stale-successor");
    assert!(reserve(&engine, &lease, &successor, &cancellation, 1_003).is_ok());
}

#[test]
fn supplied_proposal_output_is_atomic_readable_after_reopen_and_never_reexecuted() {
    let (root, engine, lease, _, request, _, proposal) = proposal::tuple_fixture();
    let cancellation = EmbeddingCancellation::default();
    let reservation = reserve(&engine, &lease, &request, &cancellation, 1_000).unwrap();
    let key = crate::engine::session::session_key(&engine.instance, &lease.session_id);
    let expected = engine.storage.control().get(&key).unwrap().unwrap();
    engine
        .finish_admitted_task_attempt(
            reservation,
            Ok(proposal.clone().into()),
            |_| {
                Ok(vec![AttemptAuthorityGuard {
                    key: key.clone(),
                    expected: expected.clone(),
                }])
            },
            &FixedClock::at(1_100),
            &context(&lease),
        )
        .unwrap();
    assert_eq!(
        engine.storage.control().get(&key).unwrap().unwrap(),
        expected
    );
    let before = read(&engine, &lease, &request);
    let attempt = before.attempt.as_ref().unwrap();
    assert_eq!(attempt.status, TaskAttemptStatus::Accepted);
    assert!(attempt.result.is_none());
    assert_eq!(attempt.endpoint_proposal, Some(proposal.clone()));
    assert_eq!(
        attempt.outcome_sha256,
        Some(operation_digest(&proposal).unwrap())
    );
    let budget = state(&engine, &lease, &request.task_id);
    assert_eq!(budget.reserved_steps, 1);
    assert!(budget.charged_elapsed_ms > 0);
    assert!(budget.active_attempt.is_none());
    assert!(!budget.execution_may_continue);
    drop(engine);
    let mut reopened = RrdEngine::open_existing(root.path(), instance(), TOKEN_KEY).unwrap();
    configure(&mut reopened, 3);
    assert_eq!(read(&reopened, &lease, &request), before);
    assert_eq!(state(&reopened, &lease, &request.task_id), budget);
    assert!(matches!(
        reserve(&reopened, &lease, &request, &cancellation, 1_200),
        Err(ServiceError::ReasoningAttemptRecorded {
            outcome_sha256: Some(_)
        })
    ));
    let mut corrupt = before;
    corrupt
        .attempt
        .as_mut()
        .unwrap()
        .endpoint_proposal
        .as_mut()
        .unwrap()
        .usage = None;
    assert!(corrupt.validate().is_err());
}

#[test]
fn invalidation_charges_spent_effort_without_reset_or_permanent_task_stop() {
    let (root, engine, lease, _, request, _, proposal) = proposal::tuple_fixture();
    let cancellation = EmbeddingCancellation::default();
    let reservation = reserve(&engine, &lease, &request, &cancellation, 1_000).unwrap();
    let original = state(&engine, &lease, &request.task_id);
    assert!(matches!(
        engine.finish_admitted_task_attempt(
            reservation,
            Ok(proposal.into()),
            |_| Err(ServiceError::TaskAttemptInvalidated(
                TaskAttemptInvalidation::InputsChanged
            )),
            &FixedClock::at(1_100),
            &context(&lease)
        ),
        Err(ServiceError::TaskAttemptInvalidated(
            TaskAttemptInvalidation::InputsChanged
        ))
    ));
    let receipt = read(&engine, &lease, &request).attempt.unwrap();
    assert_eq!(receipt.status, TaskAttemptStatus::Invalidated);
    assert_eq!(
        receipt.invalidation,
        Some(TaskAttemptInvalidation::InputsChanged)
    );
    assert!(receipt.endpoint_proposal.is_none() && receipt.outcome_sha256.is_none());
    let spent = state(&engine, &lease, &request.task_id);
    assert!(spent.stopped.is_none() && spent.active_attempt.is_none());
    assert!(spent.charged_elapsed_ms > original.charged_elapsed_ms);
    assert_eq!(spent.reserved_steps, 1);
    drop(engine);
    let mut engine = RrdEngine::open_existing(root.path(), instance(), TOKEN_KEY).unwrap();
    configure(&mut engine, 3);
    assert_eq!(state(&engine, &lease, &request.task_id), spent);
    assert!(matches!(
        reserve(&engine, &lease, &request, &cancellation, 1_200),
        Err(ServiceError::ReasoningAttemptRecorded {
            outcome_sha256: None
        })
    ));
    let mut successor = request.clone();
    successor.attempt_id = id("successor-proposal-ledger-test");
    drop(reserve(&engine, &lease, &successor, &cancellation, 1_200).unwrap());
    let next = state(&engine, &lease, &request.task_id);
    assert_eq!(next.deadline_unix_ms, original.deadline_unix_ms);
    assert_eq!(next.started_at_unix_ms, original.started_at_unix_ms);
    assert_eq!(next.charged_elapsed_ms, spent.charged_elapsed_ms);
    assert_eq!(next.reserved_steps, 2);
}

#[test]
fn session_change_between_guard_read_and_commit_cannot_publish_accepted_output() {
    let (_root, engine, lease, _, request, _, proposal) = proposal::tuple_fixture();
    let reservation = reserve(
        &engine,
        &lease,
        &request,
        &EmbeddingCancellation::default(),
        1_000,
    )
    .unwrap();
    let key = crate::engine::session::session_key(&engine.instance, &lease.session_id);
    let expected = engine.storage.control().get(&key).unwrap().unwrap();
    let mut checks = 0;
    let result = engine.finish_admitted_task_attempt(
        reservation,
        Ok(proposal.into()),
        |_| {
            checks += 1;
            if engine.storage.control().get(&key)?.as_deref() != Some(expected.as_slice()) {
                return Err(ServiceError::TaskAttemptInvalidated(
                    TaskAttemptInvalidation::AuthorizationChanged,
                ));
            }
            // A real session transition occurs after the check and before the
            // completion CAS. The callback deliberately returns the now-stale guard.
            engine.close_session(
                &lease.session_id,
                &lease.token,
                &rrd_contract::CloseSession {},
                &id("guard-session-close"),
                60,
                "guard-session-close",
                "guard-session-close",
            )?;
            Ok(vec![AttemptAuthorityGuard {
                key: key.clone(),
                expected: expected.clone(),
            }])
        },
        &FixedClock::at(1_100),
        &context(&lease),
    );
    assert!(matches!(
        result,
        Err(ServiceError::TaskAttemptInvalidated(
            TaskAttemptInvalidation::AuthorizationChanged
        ))
    ));
    assert_eq!(
        checks, 2,
        "stale control guard must conflict and re-admit completion"
    );
    assert_ne!(
        engine.storage.control().get(&key).unwrap().unwrap(),
        expected
    );
    let reader = engine
        .create_session(
            &session_request(9_000, 4),
            &id("guard-reader"),
            70,
            "guard-reader",
            "guard-reader",
        )
        .unwrap();
    let attempt = read(&engine, &reader, &request).attempt.unwrap();
    assert_eq!(attempt.status, TaskAttemptStatus::Invalidated);
    assert!(attempt.endpoint_proposal.is_none());
    assert!(state(&engine, &reader, &request.task_id).stopped.is_none());
}

#[test]
fn task_closure_between_freshness_read_and_commit_invalidates_the_attempt() {
    let (_root, engine, lease, _, request, _, proposal) = proposal::tuple_fixture();
    let reservation = reserve(
        &engine,
        &lease,
        &request,
        &EmbeddingCancellation::default(),
        1_000,
    )
    .unwrap();
    let mut checks = 0;
    let result = engine.finish_admitted_task_attempt(
        reservation,
        Ok(proposal.into()),
        |read| {
            checks += 1;
            if matches!(
                engine.open_task_at(read, &request.scope, &request.task_id),
                Err(ServiceError::TaskClosed)
            ) {
                return Err(ServiceError::TaskAttemptInvalidated(
                    TaskAttemptInvalidation::InputsChanged,
                ));
            }
            engine.close_task(
                &lease.session_id,
                &lease.token,
                &rrd_contract::CloseTask {
                    scope: request.scope.clone(),
                    task_id: request.task_id.clone(),
                    status: rrd_contract::TaskStatus::Abandoned,
                    outcome: None,
                },
                60,
                "guard-task-close",
                "guard-task-close",
            )?;
            Ok(Vec::new())
        },
        &FixedClock::at(1_100),
        &context(&lease),
    );
    assert!(matches!(
        result,
        Err(ServiceError::TaskAttemptInvalidated(
            TaskAttemptInvalidation::InputsChanged
        ))
    ));
    assert_eq!(checks, 2);
    assert_eq!(
        read(&engine, &lease, &request).attempt.unwrap().status,
        TaskAttemptStatus::Invalidated
    );
}

#[test]
fn uncertain_failure_and_cancellation_never_retain_successful_output() {
    for cancel in [false, true] {
        let (_root, engine, lease, _, request, _, proposal) = proposal::tuple_fixture();
        let cancellation = EmbeddingCancellation::default();
        let reservation = reserve(&engine, &lease, &request, &cancellation, 1_000).unwrap();
        let result = if cancel {
            cancellation.cancel();
            Ok(TaskAttemptOutput::from(proposal))
        } else {
            // Controlled classification input, not a newly observed provider failure.
            Err(ServiceError::EndpointCallFailed {
                reason: "test unresolved transport outcome",
                execution_may_continue: true,
            })
        };
        assert!(engine
            .finish_admitted_task_attempt(
                reservation,
                result,
                |_| if cancel {
                    Err(ServiceError::TaskAttemptInvalidated(
                        TaskAttemptInvalidation::InputsChanged,
                    ))
                } else {
                    Ok(Vec::new())
                },
                &FixedClock::at(1_100),
                &context(&lease)
            )
            .is_err());
        let attempt = read(&engine, &lease, &request).attempt.unwrap();
        assert_eq!(attempt.status, TaskAttemptStatus::Stopped);
        assert!(attempt.endpoint_proposal.is_none() && attempt.outcome_sha256.is_none());
        let budget = state(&engine, &lease, &request.task_id);
        assert_eq!(budget.execution_may_continue, !cancel);
        assert_eq!(
            budget.stopped,
            Some(if cancel {
                ReasoningBudgetStop::Cancelled
            } else {
                ReasoningBudgetStop::AttemptFailed
            })
        );
    }
}

#[test]
fn capacity_and_task_reservation_commit_together_and_survive_uncertain_failure() {
    // Fault classification is injected into the real ledger, never represented
    // as a newly observed provider failure or exported for learning.
    for uncertain in [false, true] {
        let (root, engine, lease, _, request, packet, _) = proposal::tuple_fixture();
        let capacity = CapacityLease {
            request: request.clone(),
            controller_binding_id: packet.controller_binding_id,
        };
        let cancellation = EmbeddingCancellation::default();
        let admission = TaskCompletionAdmission {
            writes: vec![engine
                .endpoint_capacity_transition(&capacity, Some((1, 1)))
                .unwrap()],
            ..Default::default()
        };
        let key = admission.writes[0].key.clone();
        assert!(engine.storage.control().get(&key).unwrap().is_none());
        let reservation = engine
            .reserve_controlled_task_attempt(
                &TaskAttemptAdmission {
                    started: Instant::now(),
                    scope: &request.scope,
                    task_id: &request.task_id,
                    attempt_id: &request.attempt_id,
                    request_sha256: &operation_digest(&request).unwrap(),
                    cancellation: &cancellation,
                },
                || panic!("no adapter resolution"),
                &admission,
                &FixedClock::at(1_000),
                &context(&lease),
            )
            .unwrap();
        assert_eq!(
            engine.storage.control().get(&key).unwrap(),
            Some(admission.writes[0].replacement.clone())
        );
        let mut other = capacity.clone();
        other.request.attempt_id = id("capacity-other-attempt");
        assert!(matches!(
            engine.endpoint_capacity_transition(&other, Some((1, 1))),
            Err(ServiceError::InferenceBusy)
        ));
        let mut cleanup_calls = 0;
        assert!(engine
            .finish_controlled_task_attempt(
                reservation,
                Err(ServiceError::EndpointCallFailed {
                    reason: "controlled completion classification",
                    execution_may_continue: uncertain
                }),
                |_| {
                    cleanup_calls += 1;
                    Ok(TaskCompletionAdmission {
                        writes: if uncertain {
                            Vec::new()
                        } else {
                            vec![engine.endpoint_capacity_transition(&capacity, None)?]
                        },
                        // An authority change cannot erase remote uncertainty.
                        invalidation: Some(TaskAttemptInvalidation::AuthorizationChanged),
                        ..Default::default()
                    })
                },
                &FixedClock::at(1_100),
                &context(&lease),
            )
            .is_err());
        assert_eq!(cleanup_calls, 1);
        assert_eq!(
            state(&engine, &lease, &request.task_id).execution_may_continue,
            uncertain
        );
        let before = engine.storage.control().get(&key).unwrap();
        drop(engine);
        let mut engine = RrdEngine::open_existing(root.path(), instance(), TOKEN_KEY).unwrap();
        configure(&mut engine, 3);
        assert_eq!(engine.storage.control().get(&key).unwrap(), before);
        if uncertain {
            assert!(matches!(
                engine.endpoint_capacity_transition(&other, Some((1, 1))),
                Err(ServiceError::InferenceBusy)
            ));
        } else {
            assert!(engine
                .endpoint_capacity_transition(&other, Some((1, 1)))
                .is_ok());
        }
    }
}

#[test]
fn competing_capacity_snapshot_cannot_publish_a_task_reservation() {
    let (_root, engine, lease, _, request, packet, proposal) = proposal::tuple_fixture();
    let capacity = CapacityLease {
        request: request.clone(),
        controller_binding_id: packet.controller_binding_id,
    };
    let stale = TaskCompletionAdmission {
        writes: vec![engine
            .endpoint_capacity_transition(&capacity, Some((1, 1)))
            .unwrap()],
        ..Default::default()
    };
    let cancellation = EmbeddingCancellation::default();
    let first = engine
        .reserve_controlled_task_attempt(
            &TaskAttemptAdmission {
                started: Instant::now(),
                scope: &request.scope,
                task_id: &request.task_id,
                attempt_id: &request.attempt_id,
                request_sha256: &operation_digest(&request).unwrap(),
                cancellation: &cancellation,
            },
            || panic!("no adapter resolution"),
            &stale,
            &FixedClock::at(1_000),
            &context(&lease),
        )
        .unwrap();
    engine
        .finish_controlled_task_attempt(
            first,
            Ok(proposal.into()),
            |_| {
                Ok(TaskCompletionAdmission {
                    writes: vec![engine.endpoint_capacity_transition(&capacity, None)?],
                    ..Default::default()
                })
            },
            &FixedClock::at(1_100),
            &context(&lease),
        )
        .unwrap();
    let before = state(&engine, &lease, &request.task_id);
    let second_id = id("stale-capacity-attempt");
    assert!(matches!(
        engine.reserve_controlled_task_attempt(
            &TaskAttemptAdmission {
                started: Instant::now(),
                scope: &request.scope,
                task_id: &request.task_id,
                attempt_id: &second_id,
                request_sha256: &operation_digest(&request).unwrap(),
                cancellation: &cancellation,
            },
            || panic!("no adapter resolution"),
            &stale,
            &FixedClock::at(1_200),
            &context(&lease),
        ),
        Err(ServiceError::StorageConflict(_))
    ));
    assert_eq!(state(&engine, &lease, &request.task_id), before);
}

#[test]
fn public_proposal_requires_initialized_authority_before_any_reservation() {
    let (_root, engine, lease, _, request, packet, _) = proposal::tuple_fixture();
    assert!(matches!(
        engine.propose_task_endpoint_with_clock(
            &lease.session_id,
            &lease.token,
            &request,
            "proposal-denied",
            "proposal-denied",
            &FixedClock::at(1_000),
        ),
        Err(ServiceError::PermissionDenied)
    ));
    assert!(engine
        .inspect_task_reasoning_budget(
            &lease.session_id,
            &lease.token,
            &request.scope,
            &request.task_id,
            1_001,
            "inspect",
            "inspect"
        )
        .unwrap()
        .is_none());
    let capacity = CapacityLease {
        request,
        controller_binding_id: packet.controller_binding_id,
    };
    assert!(engine
        .endpoint_capacity_transition(&capacity, Some((1, 1)))
        .unwrap()
        .expected
        .is_none());
}

#[test]
fn each_required_proposal_permission_is_enforced_before_configuration_or_capacity() {
    // These are real authorization denials in isolated engrams. No transport or
    // fabricated model answer is involved. The secret is an engine-issued token.
    let required = [
        SecurityAction::EndpointPropose,
        SecurityAction::TaskRead,
        SecurityAction::ReasoningWrite,
    ];
    for denied in required {
        let (_root, engine, lease, _, request, packet, _) = proposal::tuple_fixture();
        let principal_id = engine.instance.clone();
        let principal = rrd_security::Principal {
            id: principal_id.clone(),
            kind: rrd_security::PrincipalKind::Service,
            credential_sha256: digest::sha256_hex(lease.token.as_str().as_bytes()),
            credential_revision: 1,
            not_before_unix_ms: 1,
            expires_at_unix_ms: u64::MAX,
            disabled: false,
            role_ids: Default::default(),
            grants: std::iter::once(SecurityAction::SessionCreate)
                .chain(required.into_iter().filter(|action| *action != denied))
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
                50,
                "proposal-permission-test",
                "initialize",
                "initialize",
            )
            .unwrap();
        let authenticated = engine
            .create_authenticated_session(
                &principal_id,
                lease.token.as_str().as_bytes(),
                &session_request(9_000, 4),
                &id("permission-proposal-session"),
                100,
                "session",
                "session",
            )
            .unwrap();
        assert!(matches!(
            engine.propose_task_endpoint_with_clock(
                &authenticated.session_id,
                &authenticated.token,
                &request,
                "permission-proposal",
                "permission-proposal",
                &FixedClock::at(1_000),
            ),
            Err(ServiceError::PermissionDenied)
        ));
        assert!(engine
            .storage
            .control()
            .get(&engine.budget_key(&request.scope, &request.task_id))
            .unwrap()
            .is_none());
        let capacity = CapacityLease {
            request,
            controller_binding_id: packet.controller_binding_id,
        };
        assert!(engine
            .endpoint_capacity_transition(&capacity, Some((1, 1)))
            .unwrap()
            .expected
            .is_none());
    }
}
