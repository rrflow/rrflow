//! Typed, retained assessments across unrelated advances and lost responses.

use super::*;

#[test]
pub(super) fn controlled_assessment_is_typed_retained_and_currently_authorized_on_readback() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let mut fixture = source_worker_assessment_fixture(&listener);
    configure_assessment_budget(&mut fixture);
    publish_assessment_policy(&mut fixture, true, true, true);
    let request = assessment_request(&fixture, "happy", binary_descriptor());
    assert!(
        fixture
            .engine
            .load_endpoint_configuration()
            .unwrap()
            .1
            .profile
            .unwrap()
            .assessment_policy
            .unwrap()
            .allow_network
    );
    let budget_before = inspect_budget(&fixture, fixture.now + 1);
    assert_eq!(budget_before.reserved_steps, 0);
    let assessment_at = fixture.now + 1;
    assert!(assessment_at > budget_before.started_at_unix_ms);
    assert!(assessment_at < budget_before.deadline_unix_ms);
    assert!(assessment_at < fixture.lease.absolute_expires_at_unix_ms);
    let (output, (packet, response)) = std::thread::scope(|scope| {
        let server = scope.spawn(|| {
            let (mut socket, packet, model) = receive_assessment(&listener);
            assert_eq!(packet["source_content"], fixture.source_content);
            assert!(packet.get("context").is_none());
            let body = assessment_reply(&packet, true);
            let response = reply(&mut socket, &model, &body);
            (packet, response)
        });
        let output = fixture
            .engine
            .execute_task_assessment_with_clock(
                &fixture.lease.session_id,
                &fixture.lease.token,
                &request,
                "assessment-execute-happy",
                "assessment-execute-happy",
                &crate::engine::clock::FixedClock::at(assessment_at),
            )
            .unwrap();
        let (packet, response) = server.join().unwrap();
        (output, (packet, response))
    });
    assert_eq!(packet["source_content"], fixture.source_content);
    assert_eq!(output.source.entry_identity, fixture.source.entry_identity);
    assert_eq!(
        output.source.content_sha256,
        fixture.source.expected_content_sha256
    );
    assert!(matches!(
        &output.decision,
        TaskAssessmentDecision::Answered {
            value: TaskAssessmentValue::Binary { value: true }
        }
    ));
    assert_eq!(output.references.len(), 1);
    assert_eq!(output.references[0].input, AssessmentInputKind::Source);
    assert!(!response.is_empty());
    let budget_after = inspect_budget(&fixture, fixture.now + 2);
    assert_eq!(
        budget_after.reserved_steps,
        budget_before.reserved_steps + 1
    );
    assert_eq!(budget_after.reserved_steps, 1);
    assert!(budget_after.active_attempt.is_none());
    assert!(!budget_after.execution_may_continue);
    assert_eq!(
        budget_after.deadline_unix_ms,
        budget_before.deadline_unix_ms
    );
    listener.set_nonblocking(true).unwrap();
    assert!(matches!(
        listener.accept(),
        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock
    ));
    listener.set_nonblocking(false).unwrap();

    let read_request = ReadTaskAssessment {
        contract_version: TASK_ASSESSMENT_VERSION,
        scope: request.scope.clone(),
        task_id: request.task_id.clone(),
        attempt_id: request.attempt_id.clone(),
    };
    std::fs::write(
        fixture.installed.root.path().join(&fixture.source_path),
        "same selected path, changed ordinary contents\n",
    )
    .unwrap();
    let retained = fixture
        .engine
        .read_task_assessment(
            &fixture.lease.session_id,
            &fixture.lease.token,
            &read_request,
            fixture.now + 3,
            "assessment-read-after-edit",
            "assessment-read-after-edit",
        )
        .unwrap();
    assert_eq!(retained.assessment, Some(output.clone()));

    publish_assessment_policy(&mut fixture, true, false, true);
    assert!(matches!(
        fixture.engine.read_task_assessment(
            &fixture.lease.session_id,
            &fixture.lease.token,
            &read_request,
            fixture.now + 1,
            "assessment-read-source-export-disabled",
            "assessment-read-source-export-disabled",
        ),
        Err(ServiceError::PermissionDenied)
    ));
    publish_assessment_policy(&mut fixture, true, true, false);
    assert!(matches!(
        fixture.engine.read_task_assessment(
            &fixture.lease.session_id,
            &fixture.lease.token,
            &read_request,
            fixture.now + 1,
            "assessment-read-context-export-disabled",
            "assessment-read-context-export-disabled",
        ),
        Err(ServiceError::PermissionDenied)
    ));
    publish_assessment_policy(&mut fixture, true, true, true);
    assert_eq!(
        fixture
            .engine
            .read_task_assessment(
                &fixture.lease.session_id,
                &fixture.lease.token,
                &read_request,
                fixture.now + 1,
                "assessment-read-export-restored",
                "assessment-read-export-restored",
            )
            .unwrap()
            .assessment,
        Some(output.clone())
    );

    revoke_source_read(
        &fixture.engine,
        &fixture.lease,
        &fixture.principal_id,
        fixture.now + 2,
    );
    assert!(matches!(
        fixture.engine.read_task_assessment(
            &fixture.lease.session_id,
            &fixture.lease.token,
            &read_request,
            fixture.now + 3,
            "assessment-read-source-revoked",
            "assessment-read-source-revoked",
        ),
        Err(ServiceError::PermissionDenied)
    ));
    restore_source_read(
        &fixture.engine,
        &fixture.lease,
        &fixture.principal_id,
        &fixture.installed.preview.installation.target.project_id,
        &fixture.source.entry_identity,
        fixture.now + 4,
    );
    let restored = fixture
        .engine
        .read_task_assessment(
            &fixture.lease.session_id,
            &fixture.lease.token,
            &read_request,
            fixture.now + 5,
            "assessment-read-restored",
            "assessment-read-restored",
        )
        .unwrap();
    assert_eq!(restored.assessment, Some(output));

    let metadata_configuration = publish_metadata_observation_policy(&mut fixture);
    let attempt_request = rrd_contract::ReadTaskAttempt {
        scope: request.scope.clone(),
        task_id: request.task_id.clone(),
        attempt_id: request.attempt_id.clone(),
    };
    let attempt_view = fixture
        .engine
        .read_task_attempt(
            &fixture.lease.session_id,
            &fixture.lease.token,
            &attempt_request,
            fixture.now + 1,
            "assessment-attempt-metadata-read",
            "assessment-attempt-metadata-read",
        )
        .unwrap();
    let attempt = attempt_view.attempt.as_ref().unwrap();
    assert_eq!(attempt.status, rrd_contract::TaskAttemptStatus::Accepted);
    assert!(attempt.outcome_sha256.is_some());
    assert!(attempt.endpoint_proposal.is_none() && attempt.worker_output.is_none());
    let attempt_json = serde_json::to_value(&attempt_view).unwrap();
    assert!(attempt_json.pointer("/attempt/assessment_output").is_none());
    assert!(attempt_json.pointer("/attempt/decision").is_none());

    let delivery_id = id("assessment-metadata-only-observation");
    let prepare_request = rrd_contract::PrepareObservationDelivery {
        contract_version: rrd_contract::OBSERVATION_DELIVERY_VERSION,
        delivery_id: delivery_id.clone(),
        source: rrd_contract::ObservationSource::TaskAttempt {
            request: attempt_request.clone(),
            expected_status: rrd_contract::TaskAttemptStatus::Accepted,
            expected_outcome_sha256: attempt.outcome_sha256.clone(),
        },
        expected_configuration_sha256: metadata_configuration.sha256().unwrap(),
    };
    let prepared = fixture
        .engine
        .prepare_observation_delivery_at(
            &fixture.lease.session_id,
            &fixture.lease.token,
            &prepare_request,
            fixture.now + 2,
            "assessment-metadata-observation-prepare",
            "assessment-metadata-observation-prepare",
        )
        .unwrap();
    let observation_key = format!(
        "server/state/{}/observation-delivery/record/{}",
        fixture.engine.instance,
        operation_digest(&(&fixture.principal_id, &delivery_id)).unwrap()
    );
    let observation_bytes = fixture
        .engine
        .storage
        .control()
        .get(&observation_key)
        .unwrap()
        .unwrap();
    let observation_record: serde_json::Value = serde_json::from_slice(&observation_bytes).unwrap();
    for field in ["source_json", "payload_json"] {
        let encoded = observation_record[field].as_str().unwrap();
        assert!(!encoded.contains("decision"));
        assert!(!encoded.contains("references"));
        assert!(!encoded.contains(&fixture.source_content));
        assert!(!encoded.contains(&request.practice_text));
    }
    assert_eq!(
        observation_record["snapshot"]["payload_sha256"],
        prepared.payload_sha256
    );

    revoke_source_read(
        &fixture.engine,
        &fixture.lease,
        &fixture.principal_id,
        fixture.now + 3,
    );
    assert!(matches!(
        fixture.engine.read_task_assessment(
            &fixture.lease.session_id,
            &fixture.lease.token,
            &read_request,
            fixture.now + 4,
            "assessment-direct-output-denied-after-revoke",
            "assessment-direct-output-denied-after-revoke",
        ),
        Err(ServiceError::PermissionDenied)
    ));
    let metadata_after_revoke = fixture
        .engine
        .read_task_attempt(
            &fixture.lease.session_id,
            &fixture.lease.token,
            &attempt_request,
            fixture.now + 5,
            "assessment-attempt-metadata-after-revoke",
            "assessment-attempt-metadata-after-revoke",
        )
        .unwrap()
        .attempt
        .unwrap();
    assert_eq!(
        metadata_after_revoke.status,
        rrd_contract::TaskAttemptStatus::Accepted
    );
    assert_eq!(metadata_after_revoke.outcome_sha256, attempt.outcome_sha256);
    assert!(metadata_after_revoke.worker_output.is_none());
    let observed_metadata = fixture
        .engine
        .read_observation_delivery_at(
            &fixture.lease.session_id,
            &fixture.lease.token,
            &rrd_contract::ReadObservationDelivery {
                contract_version: rrd_contract::OBSERVATION_DELIVERY_VERSION,
                delivery_id,
            },
            fixture.now + 6,
            "assessment-metadata-observation-read-after-revoke",
            "assessment-metadata-observation-read-after-revoke",
        )
        .unwrap()
        .delivery
        .unwrap();
    assert_eq!(observed_metadata, prepared);
}

#[test]
pub(super) fn unrelated_accepted_inventory_advance_preserves_selected_assessment_input() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let mut fixture = source_worker_fixture_for_path_at(
        &listener,
        true,
        true,
        "nested/alpha.txt",
        host_unix_ms().saturating_sub(30_000),
    );
    let project_id = fixture
        .installed
        .preview
        .installation
        .target
        .project_id
        .clone();
    let original = crate::engine::attunement::load_accepted_source(
        &fixture.engine,
        &project_id,
        8 * 1024 * 1024,
        10_000,
    )
    .unwrap();
    let original_selected = original
        .proposal
        .entries()
        .iter()
        .find(|entry| entry.path == fixture.source_path)
        .unwrap()
        .clone();

    let refreshed_sha256 = refresh_unrelated(
        fixture.installed.root.path(),
        &fixture.engine,
        &fixture.lease,
        &project_id,
        &mut fixture.job,
        "unrelated-assessment.txt",
    );
    assert_ne!(refreshed_sha256, original.proposal_sha256);
    let refreshed = crate::engine::attunement::load_accepted_source(
        &fixture.engine,
        &project_id,
        8 * 1024 * 1024,
        10_000,
    )
    .unwrap();
    let refreshed_selected = refreshed
        .proposal
        .entries()
        .iter()
        .find(|entry| entry.path == fixture.source_path)
        .unwrap();
    assert_eq!(refreshed_selected.identity, original_selected.identity);
    assert_eq!(refreshed_selected.kind, original_selected.kind);
    fixture.now = host_unix_ms().max(fixture.job.updated_at_unix_ms + 1);
    configure_assessment_budget(&mut fixture);
    publish_assessment_policy(&mut fixture, true, true, true);

    let request = assessment_request(&fixture, "unrelated-head", binary_descriptor());
    let before = inspect_budget(&fixture, fixture.now + 1);
    assert_eq!(before.reserved_steps, 0);
    let (output, packet) = run_assessment(
        &fixture,
        &listener,
        &request,
        serde_json::json!({
            "outcome": "answered",
            "value": { "kind": "binary", "value": true }
        }),
        Some("source"),
    );
    assert_eq!(
        packet["source"]["entry_identity"],
        fixture.source.entry_identity
    );
    assert_eq!(packet["source_content"], fixture.source_content);
    assert_eq!(output.source.entry_identity, original_selected.identity);
    assert_eq!(
        output.source.content_sha256,
        fixture.source.expected_content_sha256
    );
    let after = inspect_budget(&fixture, fixture.now + 2);
    assert_eq!(after.reserved_steps, before.reserved_steps + 1);
    assert_eq!(after.deadline_unix_ms, before.deadline_unix_ms);
}

#[test]
pub(super) fn unrelated_accepted_head_advance_during_assessment_preserves_output_and_single_dispatch(
) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let mut fixture = source_worker_fixture_for_path_at(
        &listener,
        true,
        true,
        "nested/alpha.txt",
        host_unix_ms().saturating_sub(30_000),
    );
    configure_assessment_budget(&mut fixture);
    publish_assessment_policy(&mut fixture, true, true, true);
    let request = assessment_request(&fixture, "unrelated-head-during", binary_descriptor());
    let before_budget = inspect_budget(&fixture, fixture.now + 1);
    assert_eq!(before_budget.reserved_steps, 0);
    let before = crate::engine::attunement::load_accepted_source(
        &fixture.engine,
        &fixture.installed.preview.installation.target.project_id,
        8 * 1024 * 1024,
        10_000,
    )
    .unwrap();
    let before_entry = before
        .proposal
        .entries()
        .iter()
        .find(|entry| entry.path == fixture.source_path)
        .unwrap()
        .clone();
    let start_at = fixture.now + 1;
    assert!(start_at > before_budget.started_at_unix_ms);
    assert!(start_at < before_budget.deadline_unix_ms);
    assert!(start_at < fixture.lease.absolute_expires_at_unix_ms);
    let clock = AssessmentClock::at(start_at);
    let (entered_tx, entered_rx) = std::sync::mpsc::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let engine = &fixture.engine;
    let session = &fixture.lease.session_id;
    let token = &fixture.lease.token;
    let root = fixture.installed.root.path();
    let project_id = fixture
        .installed
        .preview
        .installation
        .target
        .project_id
        .clone();
    let selected_path = fixture.source_path.clone();
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
                session,
                token,
                &request,
                "assessment-unrelated-head-during",
                "assessment-unrelated-head-during",
                clock_ref,
            )
        });
        entered_rx
            .recv_timeout(std::time::Duration::from_secs(10))
            .unwrap();
        let profile = engine
            .load_endpoint_configuration()
            .unwrap()
            .1
            .profile
            .unwrap();
        let selector = profile.selection_policy.as_ref().unwrap();
        let controller = selector.controller_binding_id.clone();
        let binding_limit = profile
            .bindings
            .iter()
            .find(|binding| binding.id == controller)
            .unwrap()
            .limits
            .max_concurrency;
        assert_eq!(selector.maximum_concurrent_proposals, 1);
        assert_eq!(binding_limit, 1);
        let worker_binding = profile.worker_policy.as_ref().unwrap().bindings[0]
            .binding_id
            .clone();
        assert_eq!(worker_binding, controller);
        let capacity_key = format!(
            "server/state/{}/endpoint-proposal/capacity",
            engine.instance
        );
        let capacity_bytes = engine
            .storage
            .control()
            .get(&capacity_key)
            .unwrap()
            .unwrap();
        let capacity_value: serde_json::Value = serde_json::from_slice(&capacity_bytes).unwrap();
        assert_eq!(capacity_value["assessments"].as_object().unwrap().len(), 1);

        let mut competing_selection = fixture.selection.clone();
        competing_selection.attempt_id = id("assessment-capacity-competing-proposal");
        let competing_proposal = crate::engine::endpoint_proposal::capacity::CapacityLease {
            request: competing_selection,
            controller_binding_id: controller.clone(),
        };
        assert!(matches!(
            engine.endpoint_capacity_transition(
                &competing_proposal,
                Some((selector.maximum_concurrent_proposals, binding_limit))
            ),
            Err(ServiceError::InferenceBusy)
        ));
        let competing_worker = crate::engine::endpoint_proposal::capacity::WorkerCapacityLease {
            scope: fixture.selection.scope.clone(),
            task_id: fixture.selection.task_id.clone(),
            attempt_id: id("assessment-capacity-competing-worker"),
            binding_id: worker_binding,
        };
        assert!(matches!(
            engine.worker_capacity_transition(&competing_worker, Some((1, binding_limit))),
            Err(ServiceError::InferenceBusy)
        ));
        let new_head = refresh_unrelated_with_host_floor(
            root,
            engine,
            &fixture.lease,
            &project_id,
            &mut fixture.job,
            "unrelated-during-assessment.txt",
            Some(start_at),
        );
        assert_ne!(new_head, before.proposal_sha256);
        let after = crate::engine::attunement::load_accepted_source(
            &fixture.engine,
            &project_id,
            8 * 1024 * 1024,
            10_000,
        )
        .unwrap();
        let after_entry = after
            .proposal
            .entries()
            .iter()
            .find(|entry| entry.path == selected_path)
            .unwrap();
        assert_eq!(after_entry.identity, before_entry.identity);
        assert_eq!(after_entry.kind, before_entry.kind);
        let completion_at = host_unix_ms().max(fixture.job.updated_at_unix_ms + 1);
        assert!(completion_at < before_budget.deadline_unix_ms);
        assert!(completion_at < fixture.lease.absolute_expires_at_unix_ms);
        clock.advance_to(completion_at);
        release_tx.send(()).unwrap();
        (worker.join().unwrap(), server.join().unwrap())
    });
    let output = result.unwrap();
    assert_eq!(packet["source_content"], fixture.source_content);
    assert_ne!(
        output.source.captured_snapshot_sha256,
        output.source.validated_snapshot_sha256
    );
    assert_eq!(output.source.entry_identity, before_entry.identity);
    assert_eq!(
        output.source.content_sha256,
        fixture.source.expected_content_sha256
    );
    assert!(!response.is_empty());
    fixture.now = clock.0.load(Ordering::SeqCst);
    let after_budget = inspect_budget(&fixture, fixture.now + 1);
    assert_eq!(after_budget.reserved_steps, 1);
    assert_eq!(
        after_budget.deadline_unix_ms,
        before_budget.deadline_unix_ms
    );
    assert!(after_budget.active_attempt.is_none());
    assert!(!after_budget.execution_may_continue);
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
    let capacity_value: serde_json::Value = serde_json::from_slice(&capacity_bytes).unwrap();
    assert!(capacity_value.get("assessments").is_none());
    listener.set_nonblocking(true).unwrap();
    assert!(matches!(
        listener.accept(),
        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock
    ));
}

#[test]
pub(super) fn lost_assessment_response_retains_uncertain_attempt_capacity_and_budget_across_reopen()
{
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let mut fixture = source_worker_assessment_fixture(&listener);
    configure_assessment_budget(&mut fixture);
    publish_assessment_policy(&mut fixture, true, true, true);
    let request = assessment_request(&fixture, "lost-response", binary_descriptor());
    let before = inspect_budget(&fixture, fixture.now + 1);
    assert_eq!(before.reserved_steps, 0);
    let assessment_at = fixture.now + 1;
    assert!(assessment_at < before.deadline_unix_ms);
    assert!(assessment_at < fixture.lease.absolute_expires_at_unix_ms);

    let (result, packet) = std::thread::scope(|scope| {
        let server = scope.spawn(|| {
            let (_socket, packet, _model) = receive_assessment(&listener);
            packet
        });
        let result = fixture.engine.execute_task_assessment_with_clock(
            &fixture.lease.session_id,
            &fixture.lease.token,
            &request,
            "assessment-lost-response",
            "assessment-lost-response",
            &crate::engine::clock::FixedClock::at(assessment_at),
        );
        (result, server.join().unwrap())
    });
    assert_eq!(packet["source_content"], fixture.source_content);
    assert!(matches!(
        result,
        Err(ServiceError::EndpointCallFailed {
            execution_may_continue: true,
            ..
        })
    ));
    let after = inspect_budget(&fixture, fixture.now + 2);
    assert_eq!(after.reserved_steps, before.reserved_steps + 1);
    assert_eq!(after.deadline_unix_ms, before.deadline_unix_ms);
    assert!(after.active_attempt.is_none());
    assert!(after.execution_may_continue);
    assert_eq!(
        after.stopped,
        Some(rrd_contract::ReasoningBudgetStop::AttemptFailed)
    );
    let read_attempt = rrd_contract::ReadTaskAttempt {
        scope: request.scope.clone(),
        task_id: request.task_id.clone(),
        attempt_id: request.attempt_id.clone(),
    };
    let attempt = fixture
        .engine
        .read_task_attempt(
            &fixture.lease.session_id,
            &fixture.lease.token,
            &read_attempt,
            fixture.now + 3,
            "assessment-lost-attempt-read",
            "assessment-lost-attempt-read",
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
            &fixture.lease.session_id,
            &fixture.lease.token,
            &ReadTaskAssessment {
                contract_version: TASK_ASSESSMENT_VERSION,
                scope: request.scope.clone(),
                task_id: request.task_id.clone(),
                attempt_id: request.attempt_id.clone(),
            },
            fixture.now + 4,
            "assessment-lost-output-read",
            "assessment-lost-output-read",
        )
        .unwrap()
        .assessment
        .is_none());
    let capacity_key = format!(
        "server/state/{}/endpoint-proposal/capacity",
        fixture.engine.instance
    );
    let capacity = fixture
        .engine
        .storage
        .control()
        .get(&capacity_key)
        .unwrap()
        .unwrap();
    let capacity_value: serde_json::Value = serde_json::from_slice(&capacity).unwrap();
    assert_eq!(capacity_value["assessments"].as_object().unwrap().len(), 1);

    let reopen_at = host_unix_ms();
    assert!(reopen_at < fixture.lease.absolute_expires_at_unix_ms);
    let SourceWorkerFixture {
        installed,
        engine,
        lease,
        job,
        principal_id,
        worker_token,
        selection,
        source,
        source_path,
        source_content,
        now: _,
    } = fixture;
    drop(engine);
    let reopened = installed.reopen(reopen_at);
    let fixture = SourceWorkerFixture {
        installed,
        engine: reopened,
        lease,
        job,
        principal_id,
        worker_token,
        selection,
        source,
        source_path,
        source_content,
        now: reopen_at,
    };
    assert_eq!(inspect_budget(&fixture, reopen_at + 1), after);
    assert_eq!(
        fixture
            .engine
            .read_task_attempt(
                &fixture.lease.session_id,
                &fixture.lease.token,
                &read_attempt,
                reopen_at + 2,
                "assessment-lost-attempt-reopened",
                "assessment-lost-attempt-reopened",
            )
            .unwrap()
            .attempt,
        Some(attempt)
    );
    assert!(fixture
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
            reopen_at + 3,
            "assessment-lost-output-reopened",
            "assessment-lost-output-reopened",
        )
        .unwrap()
        .assessment
        .is_none());
    assert_eq!(
        fixture.engine.storage.control().get(&capacity_key).unwrap(),
        Some(capacity.clone())
    );
    assert!(matches!(
        fixture.engine.execute_task_assessment_with_clock(
            &fixture.lease.session_id,
            &fixture.lease.token,
            &request,
            "assessment-lost-response-no-replay",
            "assessment-lost-response-no-replay",
            &crate::engine::clock::FixedClock::at(reopen_at + 4),
        ),
        Err(ServiceError::ReasoningAttemptRecorded {
            outcome_sha256: None
        })
    ));
    assert_eq!(
        fixture.engine.storage.control().get(&capacity_key).unwrap(),
        Some(capacity)
    );
    assert!(matches!(
        fixture.engine.resume_task_budget_with_clock(
            &fixture.lease.session_id,
            &fixture.lease.token,
            &rrd_contract::ResumeTaskBudget {
                scope: request.scope.clone(),
                task_id: request.task_id.clone(),
                expected_budget_sha256: operation_digest(&after).unwrap(),
            },
            "assessment-lost-budget-resume",
            "assessment-lost-budget-resume",
            &crate::engine::clock::FixedClock::at(reopen_at + 4),
        ),
        Err(ServiceError::ReasoningBudgetStopped(
            rrd_contract::ReasoningBudgetStop::UnresolvedAttempt
        ))
    ));
    assert_eq!(inspect_budget(&fixture, reopen_at + 5), after);
    listener.set_nonblocking(true).unwrap();
    assert!(matches!(
        listener.accept(),
        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock
    ));
}
