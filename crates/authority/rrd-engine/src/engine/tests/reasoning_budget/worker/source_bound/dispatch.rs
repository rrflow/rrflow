//! Retained observations, dispatch revalidation, export denial and changed selected files.

use super::*;

#[test]
pub(super) fn retained_source_observation_is_denied_on_exclusion_and_available_after_restore() {
    let worker = TcpListener::bind("127.0.0.1:0").unwrap();
    let sink = TcpListener::bind("127.0.0.1:0").unwrap();
    let mut fixture = source_worker_fixture_with_git(&worker, true, true);
    let mut request = selected_request(
        &fixture.engine,
        &fixture.lease,
        &worker,
        &fixture.selection,
        fixture.now,
        false,
    );
    request.source = Some(fixture.source.clone());
    let (output, _) = complete_worker(
        &fixture.engine,
        &fixture.lease,
        &worker,
        &request,
        fixture.now + 1,
    );
    assert!(output.source.is_some());
    let read_attempt = ReadTaskAttempt {
        scope: request.scope.clone(),
        task_id: request.task_id.clone(),
        attempt_id: request.attempt_id.clone(),
    };
    let retained = fixture
        .engine
        .read_task_attempt(
            &fixture.lease.session_id,
            &fixture.lease.token,
            &read_attempt,
            fixture.now + 2,
            "source-observation-retained-attempt",
            "source-observation-retained-attempt",
        )
        .unwrap();
    let retained_output = retained
        .attempt
        .as_ref()
        .unwrap()
        .worker_output
        .as_ref()
        .unwrap();
    assert_eq!(retained_output.content, output.content);
    assert!(retained_output.source.is_some());

    let credential = CredentialReference {
        provider: CanonicalId::new("test-process-registry").unwrap(),
        name: "source-observation-sink".into(),
    };
    fixture
        .engine
        .install_endpoint_credentials(
            crate::EndpointCredentials::new(vec![(
                credential.clone(),
                "controlled-observation-secret".into(),
            )])
            .unwrap(),
        )
        .unwrap();
    let current = fixture.engine.load_endpoint_configuration().unwrap().1;
    let profile = current.profile.unwrap();
    let mut bindings = profile.bindings;
    let mut binding = bindings[0].clone();
    binding.id = CanonicalId::new("source-observation-sink").unwrap();
    binding.class = rrd_contract::WorkloadClass::BehaviorUpdates;
    binding.kind = rrd_contract::BindingKind::Api;
    binding.placement = None;
    binding.endpoint = EndpointLocator::Url {
        url: format!("http://{}", sink.local_addr().unwrap()),
    };
    binding.credential = Some(credential);
    binding.limits.max_concurrency = 1;
    binding.limits.max_input_bytes = 16_384;
    let binding_id = binding.id.clone();
    bindings.push(binding);
    let input = EndpointProfileInput {
        contract_version: profile.contract_version,
        name: profile.name,
        bindings,
        selection_policy: profile.selection_policy,
        worker_policy: profile.worker_policy,
        observation_policy: Some(ObservationExportPolicy {
            contract_version: 1,
            binding_id,
            expected_devspace: "test-devspace".into(),
            protocol_evidence_sha256: digest::sha256_hex(b"controlled protocol"),
            allow_network: true,
            allow_metadata_export: true,
            allowed_scopes: vec![request.scope.clone()],
            max_retained_deliveries: 8,
            retention_ms: 60_000,
            retire_expired_uncertain: false,
            max_attempts: 1,
            retry_delay_ms: 1_000,
            max_response_bytes: 16_384,
        }),
        assessment_policy: None,
    };
    let configured_at = fixture.now + 3;
    let plan = fixture
        .engine
        .preview_endpoint_configuration(
            &fixture.lease.session_id,
            &fixture.lease.token,
            &PreviewEndpointConfiguration {
                expected_revision: current.revision,
                profile: Some(input),
            },
            configured_at,
            "source-observation-config-preview",
            "source-observation-config-preview",
        )
        .unwrap();
    let configuration = fixture
        .engine
        .apply_endpoint_configuration(
            &fixture.lease.session_id,
            &fixture.lease.token,
            &rrd_contract::ApplyEndpointConfiguration {
                approved_plan_sha256: plan.plan_sha256.clone(),
                plan,
            },
            configured_at + 1,
            "source-observation-config-apply",
            "source-observation-config-apply",
        )
        .unwrap();
    let delivery_id = id("source-observation-retained");
    let prepared = fixture
        .engine
        .prepare_observation_delivery_at(
            &fixture.lease.session_id,
            &fixture.lease.token,
            &PrepareObservationDelivery {
                contract_version: 1,
                delivery_id: delivery_id.clone(),
                expected_configuration_sha256: configuration.sha256().unwrap(),
                source: ObservationSource::TaskAttempt {
                    request: read_attempt,
                    expected_status: rrd_contract::TaskAttemptStatus::Accepted,
                    expected_outcome_sha256: retained.attempt.unwrap().outcome_sha256,
                },
            },
            configured_at + 2,
            "source-observation-prepare",
            "source-observation-prepare",
        )
        .unwrap();
    let read = ReadObservationDelivery {
        contract_version: 1,
        delivery_id,
    };
    let before_exclusion = fixture
        .engine
        .read_observation_delivery_at(
            &fixture.lease.session_id,
            &fixture.lease.token,
            &read,
            configured_at + 3,
            "source-observation-read-authorized",
            "source-observation-read-authorized",
        )
        .unwrap()
        .source_response_json
        .unwrap();
    let prepared_source: rrd_contract::TaskAttemptView =
        serde_json::from_str(&before_exclusion).unwrap();
    assert_eq!(
        prepared_source
            .attempt
            .unwrap()
            .worker_output
            .unwrap()
            .content,
        output.content
    );
    assert_eq!(prepared.delivery_id, read.delivery_id);

    set_worker_export_flags(
        &fixture.engine,
        &fixture.lease,
        configured_at + 7,
        true,
        false,
        "observation-source-export-revoked",
    );
    assert!(matches!(
        fixture.engine.read_observation_delivery_at(
            &fixture.lease.session_id,
            &fixture.lease.token,
            &read,
            configured_at + 9,
            "source-observation-read-source-export-revoked",
            "source-observation-read-source-export-revoked",
        ),
        Err(ServiceError::PermissionDenied)
    ));
    set_worker_export_flags(
        &fixture.engine,
        &fixture.lease,
        configured_at + 10,
        true,
        true,
        "observation-source-export-restored",
    );
    assert_eq!(
        fixture
            .engine
            .read_observation_delivery_at(
                &fixture.lease.session_id,
                &fixture.lease.token,
                &read,
                configured_at + 12,
                "source-observation-read-source-export-restored",
                "source-observation-read-source-export-restored",
            )
            .unwrap()
            .source_response_json
            .as_deref(),
        Some(before_exclusion.as_str())
    );
    set_worker_export_flags(
        &fixture.engine,
        &fixture.lease,
        configured_at + 13,
        false,
        true,
        "observation-context-export-revoked",
    );
    assert!(matches!(
        fixture.engine.read_observation_delivery_at(
            &fixture.lease.session_id,
            &fixture.lease.token,
            &read,
            configured_at + 15,
            "source-observation-read-context-export-revoked",
            "source-observation-read-context-export-revoked",
        ),
        Err(ServiceError::PermissionDenied)
    ));
    set_worker_export_flags(
        &fixture.engine,
        &fixture.lease,
        configured_at + 16,
        true,
        true,
        "observation-context-export-restored",
    );
    assert_eq!(
        fixture
            .engine
            .read_observation_delivery_at(
                &fixture.lease.session_id,
                &fixture.lease.token,
                &read,
                configured_at + 18,
                "source-observation-read-context-export-restored",
                "source-observation-read-context-export-restored",
            )
            .unwrap()
            .source_response_json
            .as_deref(),
        Some(before_exclusion.as_str())
    );
    let principal_id = fixture.principal_id.clone();
    let project_id = fixture
        .installed
        .preview
        .installation
        .target
        .project_id
        .clone();
    revoke_source_read(
        &fixture.engine,
        &fixture.lease,
        &principal_id,
        configured_at + 19,
    );
    assert!(matches!(
        fixture.engine.read_observation_delivery_at(
            &fixture.lease.session_id,
            &fixture.lease.token,
            &read,
            configured_at + 20,
            "source-observation-read-grant-revoked",
            "source-observation-read-grant-revoked",
        ),
        Err(ServiceError::PermissionDenied)
    ));
    restore_source_read(
        &fixture.engine,
        &fixture.lease,
        &principal_id,
        &project_id,
        &fixture.source.entry_identity,
        configured_at + 21,
    );
    assert_eq!(
        fixture
            .engine
            .read_observation_delivery_at(
                &fixture.lease.session_id,
                &fixture.lease.token,
                &read,
                configured_at + 22,
                "source-observation-read-grant-restored",
                "source-observation-read-grant-restored",
            )
            .unwrap()
            .source_response_json
            .as_deref(),
        Some(before_exclusion.as_str())
    );

    std::fs::write(
        fixture.installed.root.path().join("nested/.gitignore"),
        b"alpha.txt\n",
    )
    .unwrap();
    assert!(matches!(
        fixture.engine.read_observation_delivery_at(
            &fixture.lease.session_id,
            &fixture.lease.token,
            &read,
            configured_at + 23,
            "source-observation-read-excluded",
            "source-observation-read-excluded",
        ),
        Err(ServiceError::PermissionDenied)
    ));

    std::fs::write(
        fixture.installed.root.path().join("nested/.gitignore"),
        b"*.tmp\n",
    )
    .unwrap();
    let restored = fixture
        .engine
        .read_observation_delivery_at(
            &fixture.lease.session_id,
            &fixture.lease.token,
            &read,
            configured_at + 24,
            "source-observation-read-restored",
            "source-observation-read-restored",
        )
        .unwrap();
    assert_eq!(
        restored.source_response_json.as_deref(),
        Some(before_exclusion.as_str())
    );
    assert_eq!(restored.delivery.unwrap().delivery_id, prepared.delivery_id);
    sink.set_nonblocking(true).unwrap();
    assert!(matches!(sink.accept(), Err(error) if error.kind() == std::io::ErrorKind::WouldBlock));
}

#[test]
pub(super) fn unrelated_accepted_head_advance_during_dispatch_revalidates_without_redispatch() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let mut fixture = source_worker_fixture_with_git(&listener, true, true);
    let captured_snapshot = crate::engine::attunement::load_accepted_source(
        &fixture.engine,
        &fixture.installed.preview.installation.target.project_id,
        8 * 1024 * 1024,
        10_000,
    )
    .unwrap()
    .proposal_sha256;
    let mut request = selected_request(
        &fixture.engine,
        &fixture.lease,
        &listener,
        &fixture.selection,
        fixture.now,
        false,
    );
    request.source = Some(fixture.source.clone());
    let budget_before = inspect_budget(&fixture, fixture.now + 1);
    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let (output, (worker_body, worker_response)) = std::thread::scope(|scope| {
        let source_content = fixture.source_content.clone();
        let server = scope.spawn(move || {
            let (mut socket, body) = read_wire(&listener);
            let worker_wire: serde_json::Value = serde_json::from_slice(&body).unwrap();
            assert_eq!(
                worker_packet_from_chat_wire(&worker_wire)["source"]["content"],
                source_content
            );
            entered_tx.send(()).unwrap();
            release_rx.recv_timeout(Duration::from_secs(10)).unwrap();
            let response = reply(
                &mut socket,
                worker_wire["model"].as_str().unwrap(),
                "Reviewed one file.",
            );
            (body, response)
        });
        let worker_engine = &fixture.engine;
        let worker_lease = &fixture.lease;
        let worker_now = fixture.now;
        let worker_request = &request;
        let worker = scope.spawn(move || {
            worker_engine.execute_task_worker_with_clock(
                &worker_lease.session_id,
                &worker_lease.token,
                worker_request,
                "source-worker-execute-during",
                "source-worker-execute-during",
                &FixedClock::at(worker_now + 1),
            )
        });
        entered_rx.recv_timeout(Duration::from_secs(10)).unwrap();
        std::fs::write(
            fixture.installed.root.path().join(".git/index"),
            include_bytes!(
                "../../../../../../../../compute/rrd-attunement/tests/fixtures/git-index/sha1-v3.index"
            ),
        )
        .unwrap();
        std::fs::write(
            fixture.installed.root.path().join(".gitignore"),
            b"*.tmp\nignored/\n",
        )
        .unwrap();
        std::fs::write(
            fixture.installed.root.path().join("nested/.gitignore"),
            b"*.bak\n",
        )
        .unwrap();
        let validated_snapshot = refresh_unrelated(
            fixture.installed.root.path(),
            &fixture.engine,
            &fixture.lease,
            &fixture.installed.preview.installation.target.project_id,
            &mut fixture.job,
            "gamma.txt",
        );
        fixture.now = fixture.job.updated_at_unix_ms + 1;
        assert_ne!(captured_snapshot, validated_snapshot);
        release_tx.send(()).unwrap();
        let output = worker.join().unwrap().unwrap();
        let server = server.join().unwrap();
        (output, server)
    });
    let wire: serde_json::Value = serde_json::from_slice(&worker_body).unwrap();
    assert_eq!(
        worker_packet_from_chat_wire(&wire)["source"]["content"],
        fixture.source_content
    );
    assert_eq!(output.content, "Reviewed one file.");
    let provenance = output.source.as_ref().unwrap();
    assert_eq!(provenance.captured_snapshot_sha256, captured_snapshot);
    assert_ne!(provenance.validated_snapshot_sha256, captured_snapshot);
    let budget_after = inspect_budget(&fixture, fixture.now + 2);
    assert_eq!(
        budget_after.reserved_steps,
        budget_before.reserved_steps + 1
    );
    assert_eq!(
        budget_after.deadline_unix_ms,
        budget_before.deadline_unix_ms
    );
    assert!(!budget_after.execution_may_continue);
    assert!(budget_after.active_attempt.is_none());
    let readback = fixture
        .engine
        .read_task_attempt(
            &fixture.lease.session_id,
            &fixture.lease.token,
            &ReadTaskAttempt {
                scope: request.scope.clone(),
                task_id: request.task_id.clone(),
                attempt_id: request.attempt_id.clone(),
            },
            fixture.now + 2,
            "source-worker-readback-during",
            "source-worker-readback-during",
        )
        .unwrap();
    assert_eq!(readback.attempt.unwrap().worker_output.unwrap(), output);
    assert!(!worker_response.is_empty());
    assert_eq!(
        std::fs::read_to_string(fixture.installed.root.path().join(&fixture.source_path)).unwrap(),
        fixture.source_content
    );
    std::fs::write(
        fixture.installed.root.path().join("nested/.gitignore"),
        b"alpha.txt\n",
    )
    .unwrap();
    let hidden = fixture
        .engine
        .read_task_attempt(
            &fixture.lease.session_id,
            &fixture.lease.token,
            &ReadTaskAttempt {
                scope: request.scope.clone(),
                task_id: request.task_id.clone(),
                attempt_id: request.attempt_id.clone(),
            },
            fixture.now + 3,
            "source-worker-readback-excluded",
            "source-worker-readback-excluded",
        )
        .unwrap();
    let hidden_attempt = hidden.attempt.unwrap();
    assert_eq!(hidden_attempt.status, TaskAttemptStatus::Accepted);
    assert!(hidden_attempt.worker_output.is_none());
    let (_, durable) = fixture
        .engine
        .read_task_attempt_record(&ReadTaskAttempt {
            scope: request.scope.clone(),
            task_id: request.task_id.clone(),
            attempt_id: request.attempt_id.clone(),
        })
        .unwrap();
    assert!(durable.attempt.unwrap().worker_output.is_some());
}

#[test]
pub(super) fn worker_source_export_is_denied_by_default_before_provider_dispatch() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let fixture = source_worker_fixture(&listener, false);
    let mut request = selected_request(
        &fixture.engine,
        &fixture.lease,
        &listener,
        &fixture.selection,
        fixture.now,
        false,
    );
    request.source = Some(fixture.source.clone());
    let before = fixture
        .engine
        .inspect_task_reasoning_budget(
            &fixture.lease.session_id,
            &fixture.lease.token,
            &request.scope,
            &request.task_id,
            fixture.now + 1,
            "source-worker-budget-before-denial",
            "source-worker-budget-before-denial",
        )
        .unwrap();
    let result = fixture.engine.execute_task_worker_with_clock(
        &fixture.lease.session_id,
        &fixture.lease.token,
        &request,
        "source-worker-denied",
        "source-worker-denied",
        &FixedClock::at(fixture.now + 1),
    );
    assert!(matches!(result, Err(ServiceError::PermissionDenied)));
    let after = fixture
        .engine
        .inspect_task_reasoning_budget(
            &fixture.lease.session_id,
            &fixture.lease.token,
            &request.scope,
            &request.task_id,
            fixture.now + 2,
            "source-worker-budget-after-denial",
            "source-worker-budget-after-denial",
        )
        .unwrap();
    assert_eq!(after, before);
    let mut socket = None;
    listener.set_nonblocking(true).unwrap();
    match listener.accept() {
        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
        Ok((accepted, _)) => socket = Some(accepted),
        Err(error) => panic!("unexpected listener error: {error}"),
    }
    assert!(
        socket.is_none(),
        "source denial must not make a provider request"
    );
}

#[test]
pub(super) fn worker_denies_unenforceable_source_data_policy_without_provider_dispatch() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let fixture = source_worker_fixture(&listener, true);
    let mut request = selected_request(
        &fixture.engine,
        &fixture.lease,
        &listener,
        &fixture.selection,
        fixture.now,
        false,
    );
    request.source = Some(fixture.source.clone());
    attach_source_data_policy(&fixture, fixture.now + 1);
    assert!(matches!(
        fixture.engine.execute_task_worker_with_clock(
            &fixture.lease.session_id,
            &fixture.lease.token,
            &request,
            "source-worker-policy-denied",
            "source-worker-policy-denied",
            &FixedClock::at(fixture.now + 2),
        ),
        Err(ServiceError::PermissionDenied)
    ));
    listener.set_nonblocking(true).unwrap();
    assert!(matches!(
        listener.accept(),
        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock
    ));
}

#[test]
pub(super) fn changed_selected_file_is_rejected_before_dispatch_without_reserving_worker_attempt() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let fixture = source_worker_fixture(&listener, true);
    let mut request = selected_request(
        &fixture.engine,
        &fixture.lease,
        &listener,
        &fixture.selection,
        fixture.now,
        false,
    );
    request.source = Some(fixture.source.clone());
    let before = fixture
        .engine
        .inspect_task_reasoning_budget(
            &fixture.lease.session_id,
            &fixture.lease.token,
            &request.scope,
            &request.task_id,
            fixture.now + 1,
            "source-worker-drift-budget-before",
            "source-worker-drift-budget-before",
        )
        .unwrap();
    std::fs::write(
        fixture.installed.root.path().join(&fixture.source_path),
        "selected source changed\n",
    )
    .unwrap();
    assert!(matches!(
        fixture.engine.execute_task_worker_with_clock(
            &fixture.lease.session_id,
            &fixture.lease.token,
            &request,
            "source-worker-drift-before",
            "source-worker-drift-before",
            &FixedClock::at(fixture.now + 2),
        ),
        Err(ServiceError::ProjectBindingMismatch)
    ));
    let after = fixture
        .engine
        .inspect_task_reasoning_budget(
            &fixture.lease.session_id,
            &fixture.lease.token,
            &request.scope,
            &request.task_id,
            fixture.now + 3,
            "source-worker-drift-budget-after",
            "source-worker-drift-budget-after",
        )
        .unwrap();
    assert_eq!(before, after);
    listener.set_nonblocking(true).unwrap();
    assert!(matches!(
        listener.accept(),
        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock
    ));
}

#[test]
pub(super) fn changed_selected_file_during_dispatch_withholds_output_without_redispatch() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let fixture = source_worker_fixture(&listener, true);
    let mut request = selected_request(
        &fixture.engine,
        &fixture.lease,
        &listener,
        &fixture.selection,
        fixture.now,
        false,
    );
    request.source = Some(fixture.source.clone());
    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let (result, (worker_body, response)) = std::thread::scope(|scope| {
        let source_content = fixture.source_content.clone();
        let server = scope.spawn(move || {
            let (mut socket, body) = read_wire(&listener);
            let wire: serde_json::Value = serde_json::from_slice(&body).unwrap();
            assert_eq!(
                worker_packet_from_chat_wire(&wire)["source"]["content"],
                source_content
            );
            entered_tx.send(()).unwrap();
            release_rx.recv_timeout(Duration::from_secs(10)).unwrap();
            let response = reply(
                &mut socket,
                wire["model"].as_str().unwrap(),
                "Stale source draft.",
            );
            (body, response)
        });
        let worker_engine = &fixture.engine;
        let worker_lease = &fixture.lease;
        let worker_request = &request;
        let worker_now = fixture.now;
        let worker = scope.spawn(move || {
            worker_engine.execute_task_worker_with_clock(
                &worker_lease.session_id,
                &worker_lease.token,
                worker_request,
                "source-worker-drift-during",
                "source-worker-drift-during",
                &FixedClock::at(worker_now + 1),
            )
        });
        entered_rx.recv_timeout(Duration::from_secs(10)).unwrap();
        std::fs::write(
            fixture.installed.root.path().join(&fixture.source_path),
            "selected source changed during request\n",
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
    assert!(!response.is_empty());
    let retained = fixture
        .engine
        .read_task_attempt(
            &fixture.lease.session_id,
            &fixture.lease.token,
            &ReadTaskAttempt {
                scope: request.scope.clone(),
                task_id: request.task_id.clone(),
                attempt_id: request.attempt_id.clone(),
            },
            fixture.now + 2,
            "source-worker-drift-readback",
            "source-worker-drift-readback",
        )
        .unwrap();
    let retained = retained.attempt.unwrap();
    assert_eq!(retained.status, TaskAttemptStatus::Invalidated);
    assert!(retained.worker_output.is_none());
    let wire: serde_json::Value = serde_json::from_slice(&worker_body).unwrap();
    assert_eq!(
        worker_packet_from_chat_wire(&wire)["source"]["content"],
        fixture.source_content
    );
}
