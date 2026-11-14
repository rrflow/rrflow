//! Choice scoring, abstention and the guards that deny or withhold assessment output.

use super::*;

#[test]
pub(super) fn engine_executes_choice_score_and_evidence_limited_abstention() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let mut fixture = source_worker_assessment_fixture(&listener);
    configure_assessment_budget(&mut fixture);
    publish_assessment_policy(&mut fixture, true, true, true);

    let choice_request = assessment_request(
        &fixture,
        "engine-choice",
        TaskAssessmentDescriptor::Choice {
            question: "Which declared mode matches the supplied evidence?".into(),
            options: vec![
                rrd_contract::AssessmentOption {
                    id: CanonicalId::new("supported").unwrap(),
                    description: "Directly supported".into(),
                },
                rrd_contract::AssessmentOption {
                    id: CanonicalId::new("uncertain").unwrap(),
                    description: "Insufficient evidence".into(),
                },
            ],
        },
    );
    let (choice_output, choice_packet) = run_assessment(
        &fixture,
        &listener,
        &choice_request,
        serde_json::json!({
            "outcome": "answered",
            "value": { "kind": "choice", "option_id": "supported" }
        }),
        Some("source"),
    );
    assert_eq!(choice_packet["source_content"], fixture.source_content);
    assert!(matches!(
        &choice_output.decision,
        TaskAssessmentDecision::Answered {
            value: TaskAssessmentValue::Choice { option_id }
        } if option_id.as_str() == "supported"
    ));

    let score_request = assessment_request(
        &fixture,
        "engine-score",
        TaskAssessmentDescriptor::Score {
            rubric: "Use a bounded score for the supplied rubric".into(),
            minimum: 0.0,
            maximum: 5.0,
            direction: AssessmentScoreDirection::HigherIsBetter,
        },
    );
    let (score_output, score_packet) = run_assessment(
        &fixture,
        &listener,
        &score_request,
        serde_json::json!({
            "outcome": "answered",
            "value": { "kind": "score", "value": 3.0 }
        }),
        Some("practice"),
    );
    assert_eq!(
        score_packet["practice_sha256"],
        score_output.practice_sha256
    );
    assert!(matches!(
        score_output.decision,
        TaskAssessmentDecision::Answered {
            value: TaskAssessmentValue::Score { value: 3.0 }
        }
    ));
    assert_eq!(
        score_output.references[0].input,
        AssessmentInputKind::Practice
    );

    let abstention_request = assessment_request(&fixture, "engine-abstention", binary_descriptor());
    let (abstention_output, _) = run_assessment(
        &fixture,
        &listener,
        &abstention_request,
        serde_json::json!({
            "outcome": "abstained",
            "reason": "The required evidence is absent from the selected bytes"
        }),
        None,
    );
    assert!(matches!(
        abstention_output.decision,
        TaskAssessmentDecision::Abstained { .. }
    ));
    assert!(abstention_output.references.is_empty());
}

#[test]
pub(super) fn disabled_assessment_flags_deny_before_budget_reservation_or_provider_dispatch() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let mut fixture = source_worker_assessment_fixture(&listener);
    configure_assessment_budget(&mut fixture);
    for (suffix, network, source, context) in [
        ("network", false, true, true),
        ("source", true, false, true),
        ("context", true, true, false),
    ] {
        publish_assessment_policy(&mut fixture, network, source, context);
        let request = assessment_request(&fixture, suffix, binary_descriptor());
        let before = inspect_budget(&fixture, fixture.now + 1);
        assert!(matches!(
            fixture.engine.execute_task_assessment_with_clock(
                &fixture.lease.session_id,
                &fixture.lease.token,
                &request,
                &format!("assessment-denied-{suffix}"),
                &format!("assessment-denied-{suffix}"),
                &crate::engine::clock::FixedClock::at(fixture.now + 1),
            ),
            Err(ServiceError::PermissionDenied)
        ));
        let after = inspect_budget(&fixture, fixture.now + 2);
        assert_eq!(after, before);
        listener.set_nonblocking(true).unwrap();
        assert!(matches!(
            listener.accept(),
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock
        ));
        listener.set_nonblocking(false).unwrap();
    }
}

#[test]
pub(super) fn configuration_change_before_assessment_dispatch_is_rejected_without_budget_or_provider_use(
) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let mut fixture = source_worker_assessment_fixture(&listener);
    configure_assessment_budget(&mut fixture);
    publish_assessment_policy(&mut fixture, true, true, true);
    let request = assessment_request(&fixture, "stale-configuration", binary_descriptor());
    let before = inspect_budget(&fixture, fixture.now + 1);
    assert_eq!(before.reserved_steps, 0);

    let changed_configuration = publish_assessment_policy_with_prompt(
        &mut fixture,
        true,
        true,
        true,
        "Use the currently approved typed-assessment rubric.",
    );
    assert_ne!(
        changed_configuration.sha256().unwrap(),
        request.expected_configuration_sha256
    );
    assert!(matches!(
        fixture.engine.execute_task_assessment_with_clock(
            &fixture.lease.session_id,
            &fixture.lease.token,
            &request,
            "assessment-stale-configuration",
            "assessment-stale-configuration",
            &crate::engine::clock::FixedClock::at(fixture.now + 1),
        ),
        Err(ServiceError::TaskAttemptInvalidated(
            TaskAttemptInvalidation::InputsChanged
        ))
    ));
    let after = inspect_budget(&fixture, fixture.now + 2);
    assert_eq!(after, before);
    listener.set_nonblocking(true).unwrap();
    assert!(matches!(
        listener.accept(),
        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock
    ));
}

#[test]
pub(super) fn closed_task_before_assessment_dispatch_is_rejected_without_budget_or_provider_use() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let mut fixture = source_worker_assessment_fixture(&listener);
    configure_assessment_budget(&mut fixture);
    publish_assessment_policy(&mut fixture, true, true, true);
    let request = assessment_request(&fixture, "closed-task", binary_descriptor());
    let before = inspect_budget(&fixture, fixture.now + 1);
    assert_eq!(before.reserved_steps, 0);

    fixture.now += 2;
    fixture
        .engine
        .close_task(
            &fixture.lease.session_id,
            &fixture.lease.token,
            &rrd_contract::CloseTask {
                scope: request.scope.clone(),
                task_id: request.task_id.clone(),
                status: rrd_contract::TaskStatus::Abandoned,
                outcome: None,
            },
            fixture.now,
            "assessment-close-task-before-dispatch",
            "assessment-close-task-before-dispatch",
        )
        .unwrap();
    assert!(matches!(
        fixture.engine.execute_task_assessment_with_clock(
            &fixture.lease.session_id,
            &fixture.lease.token,
            &request,
            "assessment-closed-task",
            "assessment-closed-task",
            &crate::engine::clock::FixedClock::at(fixture.now + 3),
        ),
        Err(ServiceError::Contract(_))
    ));
    let after = inspect_budget(&fixture, fixture.now + 4);
    assert_eq!(after, before);
    listener.set_nonblocking(true).unwrap();
    assert!(matches!(
        listener.accept(),
        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock
    ));
}

#[test]
pub(super) fn selected_source_change_during_assessment_dispatch_withholds_output_without_retry() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let mut fixture = source_worker_assessment_fixture(&listener);
    configure_assessment_budget(&mut fixture);
    publish_assessment_policy(&mut fixture, true, true, true);
    let request = assessment_request(&fixture, "source-drift", binary_descriptor());
    let (entered_tx, entered_rx) = std::sync::mpsc::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let (result, (packet, response)) = std::thread::scope(|scope| {
        let expected_source = fixture.source_content.clone();
        let listener_ref = &listener;
        let server = scope.spawn(move || {
            let (mut socket, packet, model) = receive_assessment(listener_ref);
            assert_eq!(packet["source_content"], expected_source);
            entered_tx.send(()).unwrap();
            release_rx
                .recv_timeout(std::time::Duration::from_secs(10))
                .unwrap();
            let response = reply(&mut socket, &model, &assessment_reply(&packet, true));
            (packet, response)
        });
        let worker = scope.spawn(|| {
            fixture.engine.execute_task_assessment_with_clock(
                &fixture.lease.session_id,
                &fixture.lease.token,
                &request,
                "assessment-source-drift-during",
                "assessment-source-drift-during",
                &crate::engine::clock::FixedClock::at(fixture.now + 1),
            )
        });
        entered_rx
            .recv_timeout(std::time::Duration::from_secs(10))
            .unwrap();
        std::fs::write(
            fixture.installed.root.path().join(&fixture.source_path),
            "changed while the single controlled request was pending\n",
        )
        .unwrap();
        release_tx.send(()).unwrap();
        (worker.join().unwrap(), server.join().unwrap())
    });
    assert!(matches!(
        result,
        Err(ServiceError::TaskAttemptInvalidated(
            TaskAttemptInvalidation::InputsChanged
        ))
    ));
    assert_eq!(packet["source_content"], fixture.source_content);
    assert!(!response.is_empty());
    listener.set_nonblocking(true).unwrap();
    assert!(matches!(
        listener.accept(),
        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock
    ));
    let read = fixture
        .engine
        .read_task_assessment(
            &fixture.lease.session_id,
            &fixture.lease.token,
            &ReadTaskAssessment {
                contract_version: TASK_ASSESSMENT_VERSION,
                scope: request.scope.clone(),
                task_id: request.task_id.clone(),
                attempt_id: request.attempt_id.clone(),
            },
            fixture.now + 5,
            "assessment-source-drift-read",
            "assessment-source-drift-read",
        )
        .unwrap();
    assert!(read.assessment.is_none());
}

#[test]
pub(super) fn current_task_close_during_assessment_withholds_output_without_retry() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let mut fixture = source_worker_assessment_fixture(&listener);
    configure_assessment_budget(&mut fixture);
    publish_assessment_policy(&mut fixture, true, true, true);
    let request = assessment_request(&fixture, "task-close-during", binary_descriptor());
    let before = inspect_budget(&fixture, fixture.now + 1);
    assert_eq!(before.reserved_steps, 0);
    let start_at = fixture.now + 1;
    assert!(start_at > before.started_at_unix_ms);
    assert!(start_at < before.deadline_unix_ms);
    assert!(start_at < fixture.lease.absolute_expires_at_unix_ms);
    let clock = AssessmentClock::at(start_at);
    let engine = &fixture.engine;
    let session = fixture.lease.session_id.clone();
    let token = fixture.lease.token.clone();
    let (entered_tx, entered_rx) = std::sync::mpsc::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let (result, (packet, response)) = std::thread::scope(|scope| {
        let expected_content = fixture.source_content.clone();
        let listener_ref = &listener;
        let server = scope.spawn(move || {
            let (mut socket, packet, model) = receive_assessment(listener_ref);
            assert_eq!(packet["source_content"], expected_content);
            entered_tx.send(()).unwrap();
            release_rx
                .recv_timeout(std::time::Duration::from_secs(10))
                .unwrap();
            let response = reply(&mut socket, &model, &assessment_reply(&packet, true));
            (packet, response)
        });
        let clock_ref = &clock;
        let worker = scope.spawn(|| {
            engine.execute_task_assessment_with_clock(
                &session,
                &token,
                &request,
                "assessment-task-close-during",
                "assessment-task-close-during",
                clock_ref,
            )
        });
        entered_rx
            .recv_timeout(std::time::Duration::from_secs(10))
            .unwrap();
        let close_at = host_unix_ms().max(start_at + 1);
        assert!(close_at < before.deadline_unix_ms);
        assert!(close_at < fixture.lease.absolute_expires_at_unix_ms);
        engine
            .close_task(
                &session,
                &token,
                &rrd_contract::CloseTask {
                    scope: request.scope.clone(),
                    task_id: request.task_id.clone(),
                    status: rrd_contract::TaskStatus::Abandoned,
                    outcome: None,
                },
                close_at,
                "assessment-task-close-during-state-change",
                "assessment-task-close-during-state-change",
            )
            .unwrap();
        clock.advance_to(close_at + 1);
        release_tx.send(()).unwrap();
        (worker.join().unwrap(), server.join().unwrap())
    });
    assert!(matches!(
        result,
        Err(ServiceError::TaskAttemptInvalidated(
            TaskAttemptInvalidation::InputsChanged
        ))
    ));
    assert_eq!(packet["source_content"], fixture.source_content);
    assert!(!response.is_empty());
    let after = inspect_budget(&fixture, clock.0.load(Ordering::SeqCst) + 1);
    assert_eq!(after.reserved_steps, 1);
    assert_eq!(after.deadline_unix_ms, before.deadline_unix_ms);
    assert!(after.active_attempt.is_none());
    listener.set_nonblocking(true).unwrap();
    assert!(matches!(
        listener.accept(),
        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock
    ));
    assert!(fixture
        .engine
        .read_task_assessment(
            &session,
            &token,
            &ReadTaskAssessment {
                contract_version: TASK_ASSESSMENT_VERSION,
                scope: request.scope.clone(),
                task_id: request.task_id.clone(),
                attempt_id: request.attempt_id.clone(),
            },
            clock.0.load(Ordering::SeqCst) + 2,
            "assessment-task-close-during-read",
            "assessment-task-close-during-read",
        )
        .unwrap()
        .assessment
        .is_none());
}

#[test]
pub(super) fn current_configuration_change_during_assessment_withholds_output_without_retry() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let mut fixture = source_worker_assessment_fixture(&listener);
    configure_assessment_budget(&mut fixture);
    publish_assessment_policy(&mut fixture, true, true, true);
    let request = assessment_request(&fixture, "configuration-change-during", binary_descriptor());
    let before = inspect_budget(&fixture, fixture.now + 1);
    assert_eq!(before.reserved_steps, 0);
    let start_at = fixture.now + 1;
    assert!(start_at > before.started_at_unix_ms);
    assert!(start_at < before.deadline_unix_ms);
    assert!(start_at < fixture.lease.absolute_expires_at_unix_ms);
    let clock = AssessmentClock::at(start_at);
    let engine = &fixture.engine;
    let session = fixture.lease.session_id.clone();
    let token = fixture.lease.token.clone();
    let (entered_tx, entered_rx) = std::sync::mpsc::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let (result, (packet, response)) = std::thread::scope(|scope| {
        let expected_content = fixture.source_content.clone();
        let listener_ref = &listener;
        let server = scope.spawn(move || {
            let (mut socket, packet, model) = receive_assessment(listener_ref);
            assert_eq!(packet["source_content"], expected_content);
            entered_tx.send(()).unwrap();
            release_rx
                .recv_timeout(std::time::Duration::from_secs(10))
                .unwrap();
            let response = reply(&mut socket, &model, &assessment_reply(&packet, true));
            (packet, response)
        });
        let clock_ref = &clock;
        let worker = scope.spawn(|| {
            engine.execute_task_assessment_with_clock(
                &session,
                &token,
                &request,
                "assessment-configuration-change-during",
                "assessment-configuration-change-during",
                clock_ref,
            )
        });
        entered_rx
            .recv_timeout(std::time::Duration::from_secs(10))
            .unwrap();
        let changed = publish_assessment_policy_with_prompt_for(
            engine,
            &fixture.lease,
            &mut fixture.now,
            &mut fixture.selection,
            AssessmentPolicyOptions {
                allow_network: true,
                allow_source_export: true,
                allow_context_export: true,
                system_prompt: "Use the currently approved alternate advisory rubric.",
            },
        );
        assert_ne!(
            changed.sha256().unwrap(),
            request.expected_configuration_sha256
        );
        let completion_at = host_unix_ms().max(fixture.now + 1);
        assert!(completion_at < before.deadline_unix_ms);
        assert!(completion_at < fixture.lease.absolute_expires_at_unix_ms);
        clock.advance_to(completion_at);
        release_tx.send(()).unwrap();
        (worker.join().unwrap(), server.join().unwrap())
    });
    assert!(matches!(
        result,
        Err(ServiceError::TaskAttemptInvalidated(
            TaskAttemptInvalidation::InputsChanged
        ))
    ));
    assert_eq!(packet["source_content"], fixture.source_content);
    assert!(!response.is_empty());
    let after = inspect_budget(&fixture, clock.0.load(Ordering::SeqCst) + 1);
    assert_eq!(after.reserved_steps, 1);
    assert_eq!(after.deadline_unix_ms, before.deadline_unix_ms);
    assert!(after.active_attempt.is_none());
    listener.set_nonblocking(true).unwrap();
    assert!(matches!(
        listener.accept(),
        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock
    ));
    assert!(fixture
        .engine
        .read_task_assessment(
            &session,
            &token,
            &ReadTaskAssessment {
                contract_version: TASK_ASSESSMENT_VERSION,
                scope: request.scope.clone(),
                task_id: request.task_id.clone(),
                attempt_id: request.attempt_id.clone(),
            },
            clock.0.load(Ordering::SeqCst) + 2,
            "assessment-configuration-change-during-read",
            "assessment-configuration-change-during-read",
        )
        .unwrap()
        .assessment
        .is_none());
}

#[test]
pub(super) fn public_budget_stop_during_pending_assessment_withholds_output_and_retains_capacity() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let mut fixture = source_worker_assessment_fixture(&listener);
    configure_assessment_budget(&mut fixture);
    publish_assessment_policy(&mut fixture, true, true, true);
    let request = assessment_request(&fixture, "explicit-stop", binary_descriptor());
    let before = inspect_budget(&fixture, fixture.now + 1);
    assert_eq!(before.reserved_steps, 0);
    let start_at = fixture.now + 1;
    assert!(start_at > before.started_at_unix_ms);
    assert!(start_at < before.deadline_unix_ms);
    assert!(start_at < fixture.lease.absolute_expires_at_unix_ms);
    let clock = AssessmentClock::at(start_at);
    let engine = &fixture.engine;
    let session = fixture.lease.session_id.clone();
    let token = fixture.lease.token.clone();
    let (entered_tx, entered_rx) = std::sync::mpsc::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let (result, (packet, response)) = std::thread::scope(|scope| {
        let expected_content = fixture.source_content.clone();
        let listener_ref = &listener;
        let server = scope.spawn(move || {
            let (mut socket, packet, model) = receive_assessment(listener_ref);
            assert_eq!(packet["source_content"], expected_content);
            entered_tx.send(()).unwrap();
            release_rx
                .recv_timeout(std::time::Duration::from_secs(10))
                .unwrap();
            let response = reply(&mut socket, &model, &assessment_reply(&packet, true));
            (packet, response)
        });
        let clock_ref = &clock;
        let worker = scope.spawn(|| {
            engine.execute_task_assessment_with_clock(
                &session,
                &token,
                &request,
                "assessment-explicit-stop",
                "assessment-explicit-stop",
                clock_ref,
            )
        });
        entered_rx
            .recv_timeout(std::time::Duration::from_secs(10))
            .unwrap();
        let pending = inspect_budget(&fixture, start_at + 1);
        assert_eq!(pending.reserved_steps, 1);
        assert!(pending.active_attempt.is_some());
        let stop_at = host_unix_ms().max(start_at + 1);
        assert!(stop_at < before.deadline_unix_ms);
        assert!(stop_at < fixture.lease.absolute_expires_at_unix_ms);
        engine
            .stop_task_budget(
                &session,
                &token,
                &rrd_contract::StopTaskBudget {
                    scope: request.scope.clone(),
                    task_id: request.task_id.clone(),
                    expected_budget_sha256: operation_digest(&pending).unwrap(),
                },
                stop_at,
                "assessment-explicit-stop-budget",
                "assessment-explicit-stop-budget",
            )
            .unwrap();
        clock.advance_to(stop_at + 1);
        release_tx.send(()).unwrap();
        (worker.join().unwrap(), server.join().unwrap())
    });
    assert!(
        matches!(
            &result,
            Err(ServiceError::EndpointCallFailed {
                reason: "chat cancelled",
                execution_may_continue: true,
            })
        ),
        "unexpected explicit-stop result: {result:?}"
    );
    assert_eq!(packet["source_content"], fixture.source_content);
    assert!(!response.is_empty());
    let after = inspect_budget(&fixture, clock.0.load(Ordering::SeqCst) + 1);
    assert_eq!(after.reserved_steps, 1);
    assert_eq!(after.deadline_unix_ms, before.deadline_unix_ms);
    assert_eq!(
        after.stopped,
        Some(rrd_contract::ReasoningBudgetStop::Cancelled)
    );
    assert!(after.active_attempt.is_none());
    assert!(after.execution_may_continue);
    let attempt = fixture
        .engine
        .read_task_attempt(
            &session,
            &token,
            &rrd_contract::ReadTaskAttempt {
                scope: request.scope.clone(),
                task_id: request.task_id.clone(),
                attempt_id: request.attempt_id.clone(),
            },
            clock.0.load(Ordering::SeqCst) + 2,
            "assessment-explicit-stop-read-attempt",
            "assessment-explicit-stop-read-attempt",
        )
        .unwrap()
        .attempt
        .unwrap();
    assert_eq!(attempt.status, rrd_contract::TaskAttemptStatus::Stopped);
    assert!(attempt.outcome_sha256.is_none());
    assert!(attempt.worker_output.is_none() && attempt.endpoint_proposal.is_none());
    assert!(fixture
        .engine
        .read_task_assessment(
            &session,
            &token,
            &ReadTaskAssessment {
                contract_version: TASK_ASSESSMENT_VERSION,
                scope: request.scope.clone(),
                task_id: request.task_id.clone(),
                attempt_id: request.attempt_id.clone(),
            },
            clock.0.load(Ordering::SeqCst) + 3,
            "assessment-explicit-stop-read-output",
            "assessment-explicit-stop-read-output",
        )
        .unwrap()
        .assessment
        .is_none());
    let capacity_bytes = fixture
        .engine
        .storage
        .control()
        .get(&format!(
            "server/state/{}/endpoint-proposal/capacity",
            fixture.engine.instance
        ))
        .unwrap()
        .unwrap();
    let capacity: serde_json::Value = serde_json::from_slice(&capacity_bytes).unwrap();
    assert_eq!(capacity["assessments"].as_object().unwrap().len(), 1);
    assert!(matches!(
        fixture.engine.resume_task_budget_with_clock(
            &session,
            &token,
            &rrd_contract::ResumeTaskBudget {
                scope: request.scope.clone(),
                task_id: request.task_id.clone(),
                expected_budget_sha256: operation_digest(&after).unwrap(),
            },
            "assessment-explicit-stop-budget-resume",
            "assessment-explicit-stop-budget-resume",
            &crate::engine::clock::FixedClock::at(clock.0.load(Ordering::SeqCst) + 4),
        ),
        Err(ServiceError::ReasoningBudgetStopped(
            rrd_contract::ReasoningBudgetStop::UnresolvedAttempt
        ))
    ));
    assert!(matches!(
        fixture.engine.execute_task_assessment_with_clock(
            &session,
            &token,
            &request,
            "assessment-explicit-stop-no-replay",
            "assessment-explicit-stop-no-replay",
            &crate::engine::clock::FixedClock::at(clock.0.load(Ordering::SeqCst) + 5),
        ),
        Err(ServiceError::ReasoningAttemptRecorded {
            outcome_sha256: None
        })
    ));
    assert_eq!(
        fixture
            .engine
            .storage
            .control()
            .get(&format!(
                "server/state/{}/endpoint-proposal/capacity",
                fixture.engine.instance
            ))
            .unwrap(),
        Some(capacity_bytes)
    );
    listener.set_nonblocking(true).unwrap();
    assert!(matches!(
        listener.accept(),
        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock
    ));
}
