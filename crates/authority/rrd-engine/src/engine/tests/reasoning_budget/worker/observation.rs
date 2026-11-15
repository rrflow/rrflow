//! Controlled wire fixtures: no model-quality, task-completion or training labels.
use super::*;
use rrd_contract::{
    CredentialReference, DeliverObservation, ObservationDeliveryState, ObservationExportPolicy,
    ObservationSource, PrepareObservationDelivery, ReadObservationDelivery,
};

#[test]
fn worker_observation_exports_original_configuration_and_reopens_without_completing_task() {
    let model = TcpListener::bind("127.0.0.1:0").unwrap();
    let sink = TcpListener::bind("127.0.0.1:0").unwrap();
    let (root, mut engine, lease, selection) =
        admitted_fixture(&format!("http://{}/v1", model.local_addr().unwrap()));
    let request = selected_request(&engine, &lease, &model, &selection, 1_000, false);
    let (output, _) = complete_worker(&engine, &lease, &model, &request, 1_100);
    let worker_budget = budget(&engine, &lease, &request.task_id);
    let read_attempt = ReadTaskAttempt {
        scope: request.scope.clone(),
        task_id: request.task_id.clone(),
        attempt_id: request.attempt_id.clone(),
    };
    let retained = engine
        .read_task_attempt(
            &lease.session_id,
            &lease.token,
            &read_attempt,
            1_200,
            "source",
            "source",
        )
        .unwrap();
    let credential = CredentialReference {
        provider: CanonicalId::new("test-process-registry").unwrap(),
        name: "worker-observation".into(),
    };
    engine
        .install_endpoint_credentials(
            crate::EndpointCredentials::new(vec![(
                credential.clone(),
                "controlled-test-secret".into(),
            )])
            .unwrap(),
        )
        .unwrap();
    // Installing an export destination changes current configuration. The event
    // must still identify the earlier configuration that actually ran the worker.
    let configuration = replace_profile(&engine, &lease, |profile| {
        let mut binding = profile.bindings[0].clone();
        binding.id = CanonicalId::new("test-observation-sink").unwrap();
        binding.class = rrd_contract::WorkloadClass::BehaviorUpdates;
        binding.kind = rrd_contract::BindingKind::Api;
        binding.placement = None;
        binding.credential = Some(credential);
        binding.endpoint = EndpointLocator::Url {
            url: format!("http://{}", sink.local_addr().unwrap()),
        };
        binding.limits.max_concurrency = 1;
        binding.limits.max_input_bytes = 16384;
        let binding_id = binding.id.clone();
        profile.bindings.push(binding);
        profile.observation_policy = Some(ObservationExportPolicy {
            contract_version: 1,
            binding_id,
            expected_devspace: "test-devspace".into(),
            protocol_evidence_sha256: digest::sha256_hex(b"controlled protocol"),
            allow_network: true,
            allow_metadata_export: true,
            allowed_scopes: vec![SCOPE.into()],
            max_retained_deliveries: 8,
            retention_ms: 60_000,
            retire_expired_uncertain: false,
            max_attempts: 1,
            retry_delay_ms: 1000,
            max_response_bytes: 4096,
        });
    });
    let prepare = PrepareObservationDelivery {
        contract_version: 1,
        delivery_id: id("worker-observation"),
        expected_configuration_sha256: configuration.sha256().unwrap(),
        source: ObservationSource::TaskAttempt {
            request: read_attempt.clone(),
            expected_status: TaskAttemptStatus::Accepted,
            expected_outcome_sha256: retained.attempt.as_ref().unwrap().outcome_sha256.clone(),
        },
    };
    let prepared = engine
        .prepare_observation_delivery_at(
            &lease.session_id,
            &lease.token,
            &prepare,
            1_300,
            "prepare",
            "prepare",
        )
        .unwrap();
    let delivery = DeliverObservation {
        contract_version: 1,
        delivery_id: prepared.delivery_id.clone(),
        expected_payload_sha256: prepared.payload_sha256.clone(),
        expected_configuration_sha256: prepared.configuration_sha256.clone(),
    };
    let (accepted, wire) = std::thread::scope(|scope| {
        let server = scope.spawn(|| {
            let (mut socket, raw) = read_wire_for(&sink, "/v1/events");
            let value: serde_json::Value = serde_json::from_slice(&raw).unwrap();
            let body = serde_json::to_vec(&serde_json::json!({
                "devspace":"test-devspace",
                "accepted_ids":[value["events"][0]["event_id"]],
            })).unwrap();
            write!(socket, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len()).unwrap();
            socket.write_all(&body).unwrap();
            value
        });
        let accepted = engine
            .deliver_observation_with(
                &lease.session_id,
                &lease.token,
                &delivery,
                "deliver",
                "deliver",
                &FixedClock::at(1_400),
                |prepared, secret, deadline| prepared.execute(secret, deadline),
            )
            .unwrap();
        (accepted, server.join().unwrap())
    });
    assert_eq!(accepted.state, ObservationDeliveryState::Acknowledged);
    let references = &wire["events"][0]["engine"];
    assert_eq!(references["task_id"], request.task_id.as_str());
    assert_eq!(references["attempt_id"], request.attempt_id.as_str());
    assert_eq!(
        references["configuration_sha256"],
        output.configuration_sha256
    );
    assert_ne!(output.configuration_sha256, prepared.configuration_sha256);
    assert_eq!(
        references["response_sha256"],
        prepared.source_response_sha256
    );
    assert!(!wire.to_string().contains(&output.content));
    assert!(!wire.to_string().contains("controlled-test-secret"));
    assert_eq!(budget(&engine, &lease, &request.task_id), worker_budget);
    drop(engine);
    let mut engine = RrdEngine::open_existing(root.path(), instance(), TOKEN_KEY).unwrap();
    engine.installed_engram = Some(configuration.identity);
    configure(&mut engine, 8);
    let readback = engine
        .read_observation_delivery_at(
            &lease.session_id,
            &lease.token,
            &ReadObservationDelivery {
                contract_version: 1,
                delivery_id: prepared.delivery_id,
            },
            1_500,
            "read-observation",
            "read-observation",
        )
        .unwrap();
    assert_eq!(readback.delivery.as_ref(), Some(&accepted));
    let source: rrd_contract::TaskAttemptView =
        serde_json::from_str(readback.source_response_json.as_ref().unwrap()).unwrap();
    assert_eq!(source, retained);
    let task = engine
        .read_task(
            &lease.session_id,
            &lease.token,
            &ReadTask {
                scope: request.scope,
                task_id: request.task_id,
                max_storage_keys: MAX_TASK_STORAGE_KEYS,
            },
            1_500,
            "task",
            "task",
        )
        .unwrap();
    assert_eq!(task.task.status, rrd_contract::TaskStatus::Open);
}
