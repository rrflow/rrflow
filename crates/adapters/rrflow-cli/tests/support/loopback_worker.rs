//! Loopback model endpoints, raw public HTTP sessions and the installed worker driver for lifecycle tests.

use super::*;

pub(super) fn unix_ms() -> u64 {
    u64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis(),
    )
    .unwrap()
}

pub(super) fn loopback_worker_profile(
    listener: &TcpListener,
    allow_context_export: bool,
    allow_source_export: bool,
) -> rrd_contract::EndpointProfileInput {
    let mut configuration: rrd_contract::EndpointConfiguration = serde_json::from_slice(
        &super::observed_configuration::project(LOOPBACK_CONFIGURATION_BYTES),
    )
    .unwrap();
    let allowance: rrd_contract::EndpointSelectionAllowance =
        serde_json::from_slice(LOOPBACK_ALLOWANCE_BYTES).unwrap();
    let profile = configuration.profile.take().unwrap();
    let mut bindings = profile.bindings;
    let controller_service = bindings[0].service_id.clone();
    let controller_binding_id = bindings
        .iter()
        .find(|binding| binding.service_id == controller_service)
        .unwrap()
        .id
        .clone();
    let maximum_concurrent_proposals = bindings
        .iter()
        .find(|binding| binding.id == controller_binding_id)
        .unwrap()
        .limits
        .max_concurrency;
    let service_id = rrd_contract::CanonicalId::new("controlled-loopback-worker").unwrap();
    let binding_id = rrd_contract::CanonicalId::new("controlled-loopback-binding").unwrap();
    let manifest_sha256 = rrd_core::digest::sha256_hex(b"controlled loopback endpoint fixture");
    for binding in &mut bindings {
        binding.id = binding_id.clone();
        binding.endpoint = rrd_contract::EndpointLocator::Url {
            url: format!("http://{}/v1", listener.local_addr().unwrap()),
        };
        binding.credential = None;
        binding.manifest_sha256 = Some(manifest_sha256.clone());
        binding.service_id = service_id.clone();
        binding.limits.timeout_ms = 5_000;
    }
    let mut candidates = allowance.candidates;
    for candidate in &mut candidates {
        candidate.binding_id = binding_id.clone();
        candidate.manifest_sha256 = Some(manifest_sha256.clone());
        candidate.service_id = service_id.clone();
    }
    let selection_policy = rrd_contract::EndpointSelectionPolicy {
        contract_version: allowance.contract_version,
        purpose: rrd_contract::EndpointSelectionPurpose::ExperimentalProposal,
        allow_unverified_identity: true,
        allow_network: true,
        maximum_concurrent_proposals,
        controller_binding_id: binding_id,
        workload_class: allowance.workload_class,
        limits: allowance.limits,
        candidates,
        system_prompt: "Select the single explicitly allowed controlled loopback test binding."
            .into(),
        maximum_output_tokens: 64,
        temperature_milli: 0,
        lfg: None,
    };
    let worker_binding = selection_policy.candidates[0].binding_id.clone();
    rrd_contract::EndpointProfileInput {
        contract_version: profile.contract_version,
        name: rrd_contract::CanonicalId::new("controlled-installed-source-worker").unwrap(),
        bindings,
        selection_policy: Some(selection_policy),
        observation_policy: None,
        assessment_policy: None,
        worker_policy: Some(rrd_contract::EndpointWorkerPolicy {
            contract_version: rrd_contract::ENDPOINT_WORKER_VERSION,
            allow_network: true,
            allow_context_export,
            allow_source_export,
            allow_unverified_identity: true,
            maximum_concurrent_calls: 1,
            bindings: vec![rrd_contract::EndpointWorkerBinding {
                binding_id: worker_binding,
                protocol_evidence_sha256: rrd_core::digest::sha256_hex(
                    b"controlled loopback protocol fixture; no model or quality claim",
                ),
                adapter: rrd_contract::EndpointWorkerAdapter::default(),
                system_prompt: "Return a bounded draft. Do not mutate project files.".into(),
                generation: rrd_contract::EndpointGenerationSettings {
                    maximum_output_tokens: 128,
                    token_parameter: rrd_contract::EndpointTokenParameter::MaxTokens,
                    temperature_milli: None,
                    response_format: rrd_contract::EndpointResponseFormat::Text,
                },
                maximum_response_bytes: 16_384,
                effort_values: Default::default(),
                allow_default_effort: true,
            }],
        }),
    }
}

pub(super) fn accept_loopback_chat(listener: &TcpListener) -> (TcpStream, serde_json::Value) {
    listener.set_nonblocking(true).unwrap();
    let deadline = std::time::Instant::now() + Duration::from_secs(15);
    let (mut socket, _) = loop {
        match listener.accept() {
            Ok(socket) => break socket,
            Err(error)
                if error.kind() == std::io::ErrorKind::WouldBlock
                    && std::time::Instant::now() < deadline =>
            {
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(error) => panic!("bounded loopback provider accept failed: {error}"),
        }
    };
    socket.set_nonblocking(false).unwrap();
    socket
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let mut header = Vec::new();
    while !header.ends_with(b"\r\n\r\n") {
        let mut byte = [0];
        socket.read_exact(&mut byte).unwrap();
        header.push(byte[0]);
        assert!(header.len() < 8_192);
    }
    let header = String::from_utf8(header).unwrap();
    assert!(header.starts_with("POST /v1/chat/completions HTTP/1.1"));
    let length: usize = header
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse().unwrap())
        })
        .unwrap();
    assert!(length < 524_288);
    let mut body = vec![0; length];
    socket.read_exact(&mut body).unwrap();
    (socket, serde_json::from_slice(&body).unwrap())
}

pub(super) fn reply_loopback_chat(socket: &mut TcpStream, model: &str, content: &str) {
    let body = serde_json::to_vec(&serde_json::json!({
        "model": model,
        "choices": [{"finish_reason": "stop", "message": {"role": "assistant", "content": content}}]
    }))
    .unwrap();
    write!(socket, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len()).unwrap();
    socket.write_all(&body).unwrap();
}

pub(super) fn execute_installed_assessment(
    runtime: &tokio::runtime::Runtime,
    client: &RrdClient,
    session: &rrd_client::Session,
    listener: &TcpListener,
    request: &TaskAssessmentRequest,
    decision: serde_json::Value,
    reference_input: Option<&str>,
) -> (rrd_contract::TaskAssessmentOutput, serde_json::Value) {
    std::thread::scope(|scope| {
        let server = scope.spawn(|| {
            let (mut socket, wire) = accept_loopback_chat(listener);
            assert_eq!(wire["messages"][0]["role"], "system");
            assert_eq!(
                wire["messages"][0]["content"],
                "Assess only the supplied packet; this controlled fixture is not a model result."
            );
            let packet: serde_json::Value =
                serde_json::from_str(wire["messages"][1]["content"].as_str().unwrap()).unwrap();
            assert!(packet.get("context").is_none());
            let response = assessment_reply_for_packet(&packet, decision, reference_input);
            reply_loopback_chat(&mut socket, wire["model"].as_str().unwrap(), &response);
            packet
        });
        let output = runtime
            .block_on(
                client.call_operation::<rrd_client::operations::TaskAssessmentExecute>(
                    session,
                    request.clone(),
                    RequestOptions::mutation(
                        &format!("assessment-execute-{}", request.attempt_id.as_str()),
                        &format!("assessment-execute-{}", request.attempt_id.as_str()),
                        &format!("assessment-execute-{}", request.attempt_id.as_str()),
                    )
                    .unwrap(),
                ),
            )
            .unwrap();
        let packet = server.join().unwrap();
        assert_listener_empty(listener);
        (output, packet)
    })
}

pub(super) fn assessment_reply_for_packet(
    packet: &serde_json::Value,
    decision: serde_json::Value,
    reference_input: Option<&str>,
) -> String {
    let references = reference_input
        .map(|input| {
            let (sha256, end_byte) = match input {
                "source" => (packet["source"]["content_sha256"].as_str().unwrap(), 7),
                "practice" => (packet["practice_sha256"].as_str().unwrap(), 6),
                other => panic!("unsupported controlled assessment reference input {other}"),
            };
            serde_json::json!([{
                "input": input,
                "input_sha256": sha256,
                "start_byte": 0,
                "end_byte": end_byte
            }])
        })
        .unwrap_or_else(|| serde_json::json!([]));
    serde_json::json!({
        "contract_version": TASK_ASSESSMENT_VERSION,
        "decision": decision,
        "references": references
    })
    .to_string()
}

pub(super) fn assert_listener_empty(listener: &TcpListener) {
    listener.set_nonblocking(true).unwrap();
    assert!(matches!(
        listener.accept(),
        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock
    ));
}

pub(super) fn raw_envelope<T: serde::Serialize>(
    instance_id: &CanonicalId,
    payload: T,
    request_id: &str,
    operation_id: &str,
    idempotency_key: Option<&str>,
) -> rrd_contract::RequestEnvelope<T> {
    rrd_contract::RequestEnvelope {
        protocol: rrd_contract::PROTOCOL.into(),
        protocol_version: rrd_contract::PROTOCOL_VERSION,
        context: rrd_contract::RequestContext {
            request_id: rrd_contract::CorrelationId::new(request_id).unwrap(),
            operation_id: rrd_contract::CorrelationId::new(operation_id).unwrap(),
            idempotency_key: idempotency_key
                .map(|key| rrd_contract::CorrelationId::new(key).unwrap()),
            deadline_unix_ms: None,
        },
        resource: rrd_contract::ResourcePath {
            segments: vec![rrd_contract::ResourceId::new(
                rrd_contract::ResourceKind::Instance,
                instance_id.as_str(),
            )
            .unwrap()],
        },
        payload,
    }
}

pub(super) fn raw_public_http(
    address: &str,
    method: &str,
    path: &str,
    envelope: &impl serde::Serialize,
    headers: &[(&str, String)],
) -> (u16, serde_json::Value) {
    let body = serde_json::to_vec(envelope).unwrap();
    let mut stream = TcpStream::connect(address).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    write!(stream, "{method} {path} HTTP/1.1\r\nHost: {address}\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: {}\r\n", body.len()).unwrap();
    for (name, value) in headers {
        write!(stream, "{name}: {value}\r\n").unwrap();
    }
    stream.write_all(b"\r\n").unwrap();
    stream.write_all(&body).unwrap();
    let mut response = Vec::new();
    stream.read_to_end(&mut response).unwrap();
    let split = response
        .windows(4)
        .position(|part| part == b"\r\n\r\n")
        .unwrap();
    let status = String::from_utf8_lossy(&response[..split])
        .lines()
        .next()
        .unwrap()
        .split_whitespace()
        .nth(1)
        .unwrap()
        .parse()
        .unwrap();
    (
        status,
        serde_json::from_slice(&response[split + 4..]).unwrap(),
    )
}

pub(super) fn open_raw_public_session(
    address: &str,
    instance_id: &CanonicalId,
    principal_id: &CanonicalId,
    api_key: &str,
) -> rrd_contract::SessionLease {
    let envelope = raw_envelope(
        instance_id,
        CreateSession {
            limits: SessionLimits {
                idle_timeout_ms: 60_000,
                absolute_timeout_ms: 300_000,
                max_open_transactions: 1,
            },
        },
        "assessment-raw-session-request",
        "assessment-raw-session-operation",
        Some("assessment-raw-session-idempotency"),
    );
    let (status, response) = raw_public_http(
        address,
        "POST",
        "/v1/sessions",
        &envelope,
        &[
            ("X-RRD-Principal", principal_id.as_str().to_owned()),
            ("Authorization", format!("ApiKey {api_key}")),
        ],
    );
    assert_eq!(status, 200);
    assert_eq!(response["outcome"]["status"], "ok");
    serde_json::from_value(response["outcome"]["payload"].clone()).unwrap()
}

pub(super) fn raw_close_public_session(
    address: &str,
    instance_id: &CanonicalId,
    lease: &rrd_contract::SessionLease,
) {
    let envelope = raw_envelope(
        instance_id,
        CloseSession {},
        "assessment-raw-session-close-request",
        "assessment-raw-session-close-operation",
        Some("assessment-raw-session-close-idempotency"),
    );
    let (status, response) = raw_public_http(
        address,
        "DELETE",
        &format!("/v1/sessions/{}", lease.session_id.as_str()),
        &envelope,
        &[
            ("X-RRD-Session", lease.session_id.as_str().to_owned()),
            ("Authorization", format!("Bearer {}", lease.token.as_str())),
        ],
    );
    assert_eq!(status, 200);
    assert_eq!(response["outcome"]["status"], "ok");
}

pub(super) fn install_loopback_profile(
    runtime: &tokio::runtime::Runtime,
    client: &RrdClient,
    session: &rrd_client::Session,
    listener: &TcpListener,
    allow_context_export: bool,
    allow_source_export: bool,
    suffix: &str,
) -> rrd_contract::EndpointConfiguration {
    let profile = loopback_worker_profile(listener, allow_context_export, allow_source_export);
    apply_loopback_profile(runtime, client, session, suffix, profile)
}

pub(super) fn install_loopback_assessment_profile(
    runtime: &tokio::runtime::Runtime,
    client: &RrdClient,
    session: &rrd_client::Session,
    listener: &TcpListener,
    allow_context_export: bool,
    allow_source_export: bool,
    suffix: &str,
) -> rrd_contract::EndpointConfiguration {
    let mut profile = loopback_worker_profile(listener, allow_context_export, allow_source_export);
    profile.assessment_policy = Some(EndpointAssessmentPolicy {
        contract_version: TASK_ASSESSMENT_VERSION,
        allow_network: true,
        allow_source_export,
        allow_context_export,
        system_prompt:
            "Assess only the supplied packet; this controlled fixture is not a model result.".into(),
        generation: EndpointGenerationSettings {
            maximum_output_tokens: 128,
            token_parameter: EndpointTokenParameter::MaxTokens,
            temperature_milli: None,
            response_format: EndpointResponseFormat::JsonObject,
        },
        maximum_input_bytes: 16_384,
        maximum_response_bytes: 16_384,
    });
    apply_loopback_profile(runtime, client, session, suffix, profile)
}

pub(super) fn apply_loopback_profile(
    runtime: &tokio::runtime::Runtime,
    client: &RrdClient,
    session: &rrd_client::Session,
    suffix: &str,
    profile: rrd_contract::EndpointProfileInput,
) -> rrd_contract::EndpointConfiguration {
    let current = runtime
        .block_on(
            client.read_endpoint_configuration(
                session,
                rrd_contract::ReadEndpointConfiguration {},
                RequestOptions::read(
                    &format!("source-choice-profile-read-{suffix}"),
                    &format!("source-choice-profile-read-{suffix}"),
                )
                .unwrap(),
            ),
        )
        .unwrap();
    let plan = runtime
        .block_on(
            client.preview_endpoint_configuration(
                session,
                rrd_contract::PreviewEndpointConfiguration {
                    expected_revision: current.revision,
                    profile: Some(profile),
                },
                RequestOptions::mutation(
                    &format!("source-choice-profile-preview-{suffix}"),
                    &format!("source-choice-profile-preview-{suffix}"),
                    &format!("source-choice-profile-preview-{suffix}"),
                )
                .unwrap(),
            ),
        )
        .unwrap();
    runtime
        .block_on(
            client.apply_endpoint_configuration(
                session,
                rrd_contract::ApplyEndpointConfiguration {
                    approved_plan_sha256: plan.plan_sha256.clone(),
                    plan,
                },
                RequestOptions::mutation(
                    &format!("source-choice-profile-apply-{suffix}"),
                    &format!("source-choice-profile-apply-{suffix}"),
                    &format!("source-choice-profile-apply-{suffix}"),
                )
                .unwrap(),
            ),
        )
        .unwrap()
}

pub(super) fn endpoint_selection_request(
    scope: &str,
    task_id: &rrd_contract::CanonicalId,
    task: &rrd_contract::TaskRecord,
    configuration: &rrd_contract::EndpointConfiguration,
    attempt_id: &str,
) -> rrd_contract::EndpointSelectionRequest {
    rrd_contract::EndpointSelectionRequest {
        scope: scope.to_owned(),
        task_id: task_id.clone(),
        attempt_id: rrd_contract::CorrelationId::new(attempt_id).unwrap(),
        max_storage_keys: rrd_contract::MAX_TASK_STORAGE_KEYS,
        expected_task_sha256: rrd_contract::EndpointSelectionTask::from_task(task)
            .unwrap()
            .task_sha256,
        expected_configuration_sha256: configuration.sha256().unwrap(),
        flight: None,
        routing: None,
    }
}

pub(super) fn propose_over_loopback(
    runtime: &tokio::runtime::Runtime,
    client: &RrdClient,
    session: &rrd_client::Session,
    listener: &TcpListener,
    request: rrd_contract::EndpointSelectionRequest,
    suffix: &str,
) -> rrd_contract::EndpointSelectionProposal {
    std::thread::scope(|scope| {
        let server = scope.spawn(|| {
            let (mut socket, wire) = accept_loopback_chat(listener);
            let binding_id = rrd_contract::CanonicalId::new("controlled-loopback-binding").unwrap();
            let choice = rrd_contract::EndpointSelectionChoice::Select {
                binding_id,
                effort: None,
            };
            let model = wire["model"].as_str().unwrap();
            reply_loopback_chat(&mut socket, model, &serde_json::to_string(&choice).unwrap());
        });
        let result = runtime
            .block_on(
                client.call_operation::<rrd_client::operations::TaskEndpointPropose>(
                    session,
                    request,
                    RequestOptions::mutation(
                        &format!("source-choice-proposal-{suffix}"),
                        &format!("source-choice-proposal-{suffix}"),
                        &format!("source-choice-proposal-{suffix}"),
                    )
                    .unwrap(),
                ),
            )
            .unwrap();
        server.join().unwrap();
        result
    })
}

pub(super) fn worker_request(
    proposal: &rrd_contract::EndpointSelectionProposal,
    scope: &str,
    task_id: &rrd_contract::CanonicalId,
    source: Option<(&str, &str)>,
    context: Option<rrd_contract::AssembleContext>,
    suffix: &str,
) -> rrd_contract::TaskWorkerRequest {
    rrd_contract::TaskWorkerRequest {
        scope: scope.to_owned(),
        task_id: task_id.clone(),
        attempt_id: rrd_contract::CorrelationId::new(format!("source-choice-worker-{suffix}"))
            .unwrap(),
        selection_attempt_id: proposal.attempt_id.clone(),
        expected_selection_sha256: rrd_core::digest::sha256_hex(
            &serde_json::to_vec(proposal).unwrap(),
        ),
        expected_task_sha256: proposal.task_sha256.clone(),
        expected_configuration_sha256: proposal.configuration_sha256.clone(),
        max_storage_keys: rrd_contract::MAX_TASK_STORAGE_KEYS,
        context,
        source: source.map(|(entry_identity, expected_content_sha256)| {
            rrd_contract::TaskWorkerSourceRequest {
                contract_version: rrd_contract::TASK_WORKER_SOURCE_VERSION,
                entry_identity: entry_identity.to_owned(),
                expected_content_sha256: expected_content_sha256.to_owned(),
                maximum_input_bytes: 64 * 1024,
            }
        }),
    }
}

pub(super) struct InstalledWorker<'a> {
    pub(super) runtime: &'a tokio::runtime::Runtime,
    pub(super) client: &'a RrdClient,
    pub(super) session: &'a rrd_client::Session,
    pub(super) listener: &'a TcpListener,
}

pub(super) struct WorkerAttempt<'a> {
    pub(super) proposal: &'a rrd_contract::EndpointSelectionProposal,
    pub(super) scope: &'a str,
    pub(super) task_id: &'a rrd_contract::CanonicalId,
    pub(super) source: Option<(&'a str, &'a str)>,
    pub(super) context: Option<rrd_contract::AssembleContext>,
    pub(super) suffix: &'a str,
}

impl InstalledWorker<'_> {
    pub(super) fn assert_denied(
        &self,
        attempt: WorkerAttempt<'_>,
        expected_error: rrd_contract::ErrorCode,
    ) {
        let request = worker_request(
            attempt.proposal,
            attempt.scope,
            attempt.task_id,
            attempt.source,
            attempt.context,
            attempt.suffix,
        );
        let result = self.runtime.block_on(
            self.client.execute_task_worker(
                self.session,
                request,
                RequestOptions::mutation(
                    &format!("source-choice-worker-deny-{}", attempt.suffix),
                    &format!("source-choice-worker-deny-{}", attempt.suffix),
                    &format!("source-choice-worker-deny-{}", attempt.suffix),
                )
                .unwrap(),
            ),
        );
        assert!(matches!(
            result,
            Err(rrd_client::Error::Api { error, .. }) if error.code == expected_error
        ));
        self.listener.set_nonblocking(true).unwrap();
        assert!(matches!(
            self.listener.accept(),
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock
        ));
    }

    pub(super) fn propose_and_execute(
        &self,
        selection_request: rrd_contract::EndpointSelectionRequest,
        scope: &str,
        task_id: &rrd_contract::CanonicalId,
        source: Option<(&str, &str)>,
        context: Option<rrd_contract::AssembleContext>,
    ) -> (rrd_contract::TaskWorkerOutput, Option<String>) {
        std::thread::scope(|scope_thread| {
            let server = scope_thread.spawn(|| {
                let (mut selection_socket, selection_wire) = accept_loopback_chat(self.listener);
                let choice = rrd_contract::EndpointSelectionChoice::Select {
                    binding_id: rrd_contract::CanonicalId::new("controlled-loopback-binding")
                        .unwrap(),
                    effort: None,
                };
                let model = selection_wire["model"].as_str().unwrap();
                reply_loopback_chat(
                    &mut selection_socket,
                    model,
                    &serde_json::to_string(&choice).unwrap(),
                );
                let (mut worker_socket, worker_wire) = accept_loopback_chat(self.listener);
                let packet: serde_json::Value =
                    serde_json::from_str(worker_wire["messages"][1]["content"].as_str().unwrap())
                        .unwrap();
                let source_text = packet["source"]["content"].as_str().map(str::to_owned);
                let model = worker_wire["model"].as_str().unwrap();
                reply_loopback_chat(&mut worker_socket, model, "Reviewed one selected file.");
                source_text
            });
            let proposal = self
                .runtime
                .block_on(
                    self.client
                        .call_operation::<rrd_client::operations::TaskEndpointPropose>(
                            self.session,
                            selection_request,
                            RequestOptions::mutation(
                                "source-choice-ready-proposal",
                                "source-choice-ready-proposal",
                                "source-choice-ready-proposal",
                            )
                            .unwrap(),
                        ),
                )
                .unwrap();
            let request = worker_request(&proposal, scope, task_id, source, context, "ready");
            let output = self
                .runtime
                .block_on(
                    self.client.execute_task_worker(
                        self.session,
                        request,
                        RequestOptions::mutation(
                            "source-choice-ready-worker",
                            "source-choice-ready-worker",
                            "source-choice-ready-worker",
                        )
                        .unwrap(),
                    ),
                )
                .unwrap();
            let source_text = server.join().unwrap();
            (output, source_text)
        })
    }
}
