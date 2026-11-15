//! Admission probes, not model-quality observations or learning records.
mod observation;
mod resume;
mod source_bound;
mod terminal_response;
use super::*;
use crate::engine::operation_digest;
use rrd_contract::{
    ApplyEndpointConfiguration, EndpointConfiguration, EndpointGenerationSettings, EndpointLocator,
    EndpointProfileInput, EndpointResponseFormat, EndpointSelectionChoice,
    EndpointSelectionRequest, EndpointTokenParameter, EndpointWorkerBinding, EndpointWorkerPolicy,
    InstalledEngramIdentity, PreviewEndpointConfiguration, ReadTaskAttempt, TaskAttemptStatus,
};
use rrd_contract::{EndpointSelectionTask, ReadTask, TaskWorkerRequest, MAX_TASK_STORAGE_KEYS};
use std::io::{Read, Write};
use std::net::TcpListener;

fn admitted_fixture(
    url: &str,
) -> (
    tempfile::TempDir,
    RrdEngine,
    SessionLease,
    EndpointSelectionRequest,
) {
    let (root, mut engine, lease, task_id, _) = actual_request_fixture();
    configure(&mut engine, 8);
    engine.installed_engram = Some(InstalledEngramIdentity {
        project_id: CanonicalId::new("worker-project").unwrap(),
        engram_id: CanonicalId::new("worker-engram").unwrap(),
        instance_id: instance(),
    });
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
            50,
            "worker-protocol-test",
            "initialize",
            "initialize",
        )
        .unwrap();
    let lease = engine
        .create_authenticated_session(
            &principal_id,
            lease.token.as_str().as_bytes(),
            &session_request(9_000, 4),
            &id("worker-authorized-session"),
            100,
            "session",
            "session",
        )
        .unwrap();
    let profile = proposal::configured().profile.unwrap();
    let selector = profile.selection_policy.unwrap();
    let candidate = &selector.candidates[0];
    let mut bindings = profile.bindings;
    for binding in &mut bindings {
        binding.endpoint = EndpointLocator::Url { url: url.into() };
        binding.credential = None;
        binding.limits.timeout_ms = 5_000;
    }
    let input = EndpointProfileInput {
        contract_version: profile.contract_version,
        name: profile.name,
        bindings,
        selection_policy: Some(selector.clone()),
        observation_policy: None,
        assessment_policy: None,
        worker_policy: Some(EndpointWorkerPolicy {
            contract_version: 1,
            allow_network: true,
            allow_context_export: true,
            allow_source_export: false,
            allow_unverified_identity: true,
            maximum_concurrent_calls: 1,
            bindings: vec![EndpointWorkerBinding {
                binding_id: candidate.binding_id.clone(),
                protocol_evidence_sha256: digest::sha256_hex(
                    b"controlled protocol fixture, not provider qualification",
                ),
                adapter: Default::default(),
                system_prompt: "Return a protocol-test draft, no project effects.".into(),
                generation: EndpointGenerationSettings {
                    maximum_output_tokens: 256,
                    token_parameter: EndpointTokenParameter::MaxTokens,
                    temperature_milli: None,
                    response_format: EndpointResponseFormat::Text,
                },
                maximum_response_bytes: 16_384,
                effort_values: Default::default(),
                allow_default_effort: true,
            }],
        }),
    };
    let plan = engine
        .preview_endpoint_configuration(
            &lease.session_id,
            &lease.token,
            &PreviewEndpointConfiguration {
                expected_revision: 0,
                profile: Some(input),
            },
            150,
            "preview",
            "preview",
        )
        .unwrap();
    let configuration = engine
        .apply_endpoint_configuration(
            &lease.session_id,
            &lease.token,
            &ApplyEndpointConfiguration {
                approved_plan_sha256: plan.plan_sha256.clone(),
                plan,
            },
            160,
            "apply",
            "apply",
        )
        .unwrap();
    let task = engine
        .read_task(
            &lease.session_id,
            &lease.token,
            &ReadTask {
                scope: SCOPE.into(),
                task_id: task_id.clone(),
                max_storage_keys: MAX_TASK_STORAGE_KEYS,
            },
            170,
            "task",
            "task",
        )
        .unwrap();
    let request = EndpointSelectionRequest {
        scope: SCOPE.into(),
        task_id,
        attempt_id: id("protocol-selection"),
        max_storage_keys: MAX_TASK_STORAGE_KEYS,
        expected_task_sha256: EndpointSelectionTask::from_task(&task.task)
            .unwrap()
            .task_sha256,
        expected_configuration_sha256: configuration.sha256().unwrap(),
        flight: None,
        routing: None,
    };
    (root, engine, lease, request)
}

fn read_wire(listener: &TcpListener) -> (std::net::TcpStream, Vec<u8>) {
    read_wire_for(listener, "/v1/chat/completions")
}

fn read_wire_for(listener: &TcpListener, path: &str) -> (std::net::TcpStream, Vec<u8>) {
    // A bounded accept prevents a failed admission from hanging the test process.
    listener.set_nonblocking(true).unwrap();
    let deadline = Instant::now() + Duration::from_secs(15);
    let mut socket = loop {
        match listener.accept() {
            Ok((socket, _)) => break socket,
            Err(error)
                if error.kind() == std::io::ErrorKind::WouldBlock && Instant::now() < deadline =>
            {
                std::thread::sleep(Duration::from_millis(10))
            }
            Err(error) => panic!("bounded protocol accept failed: {error}"),
        }
    };
    socket.set_nonblocking(false).unwrap();
    socket
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let mut head = Vec::new();
    while !head.ends_with(b"\r\n\r\n") {
        let mut byte = [0];
        socket.read_exact(&mut byte).unwrap();
        head.push(byte[0]);
        assert!(head.len() < 8192);
    }
    let head = String::from_utf8(head).unwrap();
    assert!(head.starts_with(&format!("POST {path} HTTP/1.1")));
    let length: usize = head
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
    (socket, body)
}

fn reply(socket: &mut std::net::TcpStream, model: &str, content: &str) -> Vec<u8> {
    let bytes = serde_json::to_vec(&serde_json::json!({"model":model,"choices":[{
        "finish_reason":"stop","message":{"role":"assistant","content":content}}]}))
    .unwrap();
    write!(socket, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", bytes.len()).unwrap();
    socket.write_all(&bytes).unwrap();
    bytes
}

fn budget(engine: &RrdEngine, lease: &SessionLease, task: &CanonicalId) -> TaskReasoningBudget {
    engine
        .inspect_task_reasoning_budget(
            &lease.session_id,
            &lease.token,
            SCOPE,
            task,
            1_200,
            "inspect",
            "inspect",
        )
        .unwrap()
        .unwrap()
}

#[test]
fn worker_consumes_retained_selection_reserves_before_http_and_reopens_without_replay() {
    // Two controlled wire exchanges with real engine admission and persistence.
    // These outputs are not from an LLM and must never enter learning records.
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let (root, engine, lease, selection_request) =
        admitted_fixture(&format!("http://{}/v1", listener.local_addr().unwrap()));
    let configuration: EndpointConfiguration = engine
        .read_endpoint_configuration(&lease.session_id, &lease.token, 180, "read", "read")
        .unwrap();
    let candidate = configuration
        .profile
        .as_ref()
        .unwrap()
        .selection_policy
        .as_ref()
        .unwrap()
        .candidates[0]
        .clone();
    let (request, output, before, wires) = std::thread::scope(|scope| {
        let server = scope.spawn(|| {
            let mut wires = Vec::new();
            for ordinal in 1..=2 {
                let (mut socket, body) = read_wire(&listener);
                let observed_budget = budget(&engine, &lease, &selection_request.task_id);
                assert_eq!(observed_budget.reserved_steps, ordinal);
                assert!(observed_budget.active_attempt.is_some());
                assert!(observed_budget.execution_may_continue);
                let wire: serde_json::Value = serde_json::from_slice(&body).unwrap();
                let content = if ordinal == 1 {
                    serde_json::to_string(&EndpointSelectionChoice::Select {
                        binding_id: candidate.binding_id.clone(),
                        effort: None,
                    })
                    .unwrap()
                } else {
                    "Controlled worker draft; no project result.".into()
                };
                let response = reply(&mut socket, wire["model"].as_str().unwrap(), &content);
                wires.push((body, response));
            }
            wires
        });
        let selection = engine
            .propose_task_endpoint_with_clock(
                &lease.session_id,
                &lease.token,
                &selection_request,
                "select",
                "select",
                &FixedClock::at(1_000),
            )
            .unwrap();
        let request = TaskWorkerRequest {
            scope: SCOPE.into(),
            task_id: selection_request.task_id.clone(),
            attempt_id: id("accepted-worker"),
            selection_attempt_id: selection.attempt_id.clone(),
            expected_selection_sha256: operation_digest(&selection).unwrap(),
            expected_task_sha256: selection.task_sha256,
            expected_configuration_sha256: selection.configuration_sha256,
            max_storage_keys: MAX_TASK_STORAGE_KEYS,
            context: None,
            source: None,
        };
        let before = budget(&engine, &lease, &request.task_id);
        let output = engine
            .execute_task_worker_with_clock(
                &lease.session_id,
                &lease.token,
                &request,
                "worker",
                "worker",
                &FixedClock::at(1_100),
            )
            .unwrap();
        (request, output, before, server.join().unwrap())
    });
    assert_eq!(
        output.content,
        "Controlled worker draft; no project result."
    );
    assert_eq!(output.request_sha256, digest::sha256_hex(&wires[1].0));
    assert_eq!(output.response_sha256, digest::sha256_hex(&wires[1].1));
    assert!(output.usage.is_none());
    assert_eq!(output.http_status, Some(200));
    let after = budget(&engine, &lease, &request.task_id);
    assert_eq!(after.reserved_steps, 2);
    assert_eq!(after.deadline_unix_ms, before.deadline_unix_ms);
    assert!(!after.execution_may_continue);
    assert!(after.active_attempt.is_none());
    let read = ReadTaskAttempt {
        scope: request.scope.clone(),
        task_id: request.task_id.clone(),
        attempt_id: request.attempt_id.clone(),
    };
    let retained = engine
        .read_task_attempt(
            &lease.session_id,
            &lease.token,
            &read,
            1_200,
            "read",
            "read",
        )
        .unwrap();
    let attempt = retained.attempt.as_ref().unwrap();
    assert_eq!(attempt.status, TaskAttemptStatus::Accepted);
    assert_eq!(attempt.worker_output.as_ref(), Some(&output));
    assert!(matches!(
        engine.execute_task_worker_with_clock(
            &lease.session_id,
            &lease.token,
            &request,
            "replay",
            "replay",
            &FixedClock::at(1_200)
        ),
        Err(ServiceError::ReasoningAttemptRecorded {
            outcome_sha256: Some(_)
        })
    ));
    assert_eq!(budget(&engine, &lease, &request.task_id), after);
    drop(engine);
    let mut engine = RrdEngine::open_existing(root.path(), instance(), TOKEN_KEY).unwrap();
    engine.installed_engram = Some(configuration.identity.clone());
    configure(&mut engine, 8);
    assert_eq!(
        engine
            .read_task_attempt(
                &lease.session_id,
                &lease.token,
                &read,
                1_300,
                "reopen",
                "reopen"
            )
            .unwrap(),
        retained
    );
    assert_eq!(budget(&engine, &lease, &request.task_id), after);
}

#[test]
fn worker_requires_initialized_authority_before_reservation() {
    let (_root, engine, lease, task_id, _) = actual_request_fixture();
    let task = engine
        .read_task(
            &lease.session_id,
            &lease.token,
            &ReadTask {
                scope: SCOPE.into(),
                task_id: task_id.clone(),
                max_storage_keys: MAX_TASK_STORAGE_KEYS,
            },
            50,
            "read",
            "read",
        )
        .unwrap();
    let request = TaskWorkerRequest {
        scope: SCOPE.into(),
        task_id: task_id.clone(),
        attempt_id: id("worker-attempt"),
        selection_attempt_id: id("absent-selection"),
        expected_selection_sha256: digest::sha256_hex(b"explicit absent selection probe"),
        expected_task_sha256: EndpointSelectionTask::from_task(&task.task)
            .unwrap()
            .task_sha256,
        expected_configuration_sha256: digest::sha256_hex(b"explicit absent configuration probe"),
        max_storage_keys: MAX_TASK_STORAGE_KEYS,
        context: None,
        source: None,
    };
    assert!(matches!(
        engine.execute_task_worker_with_clock(
            &lease.session_id,
            &lease.token,
            &request,
            "worker",
            "worker",
            &FixedClock::at(1_000),
        ),
        Err(ServiceError::PermissionDenied)
    ));
    assert!(engine
        .storage
        .control()
        .get(&engine.budget_key(SCOPE, &task_id))
        .unwrap()
        .is_none());
}

fn selected_request(
    engine: &RrdEngine,
    lease: &SessionLease,
    listener: &TcpListener,
    selection: &EndpointSelectionRequest,
    at: u64,
    abstain: bool,
) -> TaskWorkerRequest {
    let configuration = engine
        .read_endpoint_configuration(&lease.session_id, &lease.token, at, "read", "read")
        .unwrap();
    let candidate = configuration
        .profile
        .unwrap()
        .selection_policy
        .unwrap()
        .candidates[0]
        .clone();
    let proposal = std::thread::scope(|scope| {
        let server = scope.spawn(|| {
            let (mut socket, body) = read_wire(listener);
            let wire: serde_json::Value = serde_json::from_slice(&body).unwrap();
            let choice = if abstain {
                EndpointSelectionChoice::Abstain {
                    reason: rrd_contract::EndpointAbstentionReason::CannotDecide,
                }
            } else {
                EndpointSelectionChoice::Select {
                    binding_id: candidate.binding_id,
                    effort: None,
                }
            };
            reply(
                &mut socket,
                wire["model"].as_str().unwrap(),
                &serde_json::to_string(&choice).unwrap(),
            );
        });
        let result = engine.propose_task_endpoint_with_clock(
            &lease.session_id,
            &lease.token,
            selection,
            "select",
            "select",
            &FixedClock::at(at),
        );
        server.join().unwrap();
        result.unwrap()
    });
    TaskWorkerRequest {
        scope: proposal.scope.clone(),
        task_id: proposal.task_id.clone(),
        attempt_id: id("fault-worker"),
        selection_attempt_id: proposal.attempt_id.clone(),
        expected_selection_sha256: operation_digest(&proposal).unwrap(),
        expected_task_sha256: proposal.task_sha256,
        expected_configuration_sha256: proposal.configuration_sha256,
        max_storage_keys: MAX_TASK_STORAGE_KEYS,
        context: None,
        source: None,
    }
}

fn replace_profile(
    engine: &RrdEngine,
    lease: &SessionLease,
    edit: impl FnOnce(&mut EndpointProfileInput),
) -> EndpointConfiguration {
    let current = engine
        .read_endpoint_configuration(
            &lease.session_id,
            &lease.token,
            1_200,
            "profile-read",
            "profile-read",
        )
        .unwrap();
    let profile = current.profile.unwrap();
    let mut input = EndpointProfileInput {
        contract_version: profile.contract_version,
        name: profile.name,
        bindings: profile.bindings,
        selection_policy: profile.selection_policy,
        observation_policy: profile.observation_policy,
        assessment_policy: profile.assessment_policy,
        worker_policy: profile.worker_policy,
    };
    edit(&mut input);
    let plan = engine
        .preview_endpoint_configuration(
            &lease.session_id,
            &lease.token,
            &PreviewEndpointConfiguration {
                expected_revision: current.revision,
                profile: Some(input),
            },
            1_200,
            "profile-preview",
            "profile-preview",
        )
        .unwrap();
    engine
        .apply_endpoint_configuration(
            &lease.session_id,
            &lease.token,
            &ApplyEndpointConfiguration {
                approved_plan_sha256: plan.plan_sha256.clone(),
                plan,
            },
            1_201,
            "profile-apply",
            "profile-apply",
        )
        .unwrap()
}

fn revoke(engine: &RrdEngine, action: SecurityAction) {
    let repository =
        rrd_security::SecurityRepository::new(&engine.storage, engine.instance.clone());
    let mut replacement = repository.load().unwrap().unwrap();
    let revision = replacement.revision;
    replacement.revision += 1;
    for principal in replacement.principals.values_mut() {
        principal.grants.retain(|grant| grant.action != action);
    }
    repository
        .replace(
            revision,
            replacement,
            1_200,
            "worker-protocol-test",
            "revoke",
            "revoke",
        )
        .unwrap();
}

#[test]
fn wrong_stale_or_abstaining_selection_never_dispatches_or_spends_worker_effort() {
    for abstain in [false, true] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let (_root, engine, lease, selection) =
            admitted_fixture(&format!("http://{}/v1", listener.local_addr().unwrap()));
        let request = selected_request(&engine, &lease, &listener, &selection, 1_000, abstain);
        let before = budget(&engine, &lease, &request.task_id);
        let mut probes = Vec::new();
        if abstain {
            probes.push(request.clone());
        } else {
            let mut wrong = request.clone();
            wrong.expected_selection_sha256 = digest::sha256_hex(b"wrong selection digest");
            probes.push(wrong);
            let mut absent = request.clone();
            absent.selection_attempt_id = id("unrecorded-selection");
            probes.push(absent);
            let mut stale = request.clone();
            stale.expected_task_sha256 = digest::sha256_hex(b"changed task projection");
            probes.push(stale);
            let mut stale = request.clone();
            stale.expected_configuration_sha256 = digest::sha256_hex(b"changed configuration");
            probes.push(stale);
        }
        for probe in probes {
            assert!(engine
                .execute_task_worker_with_clock(
                    &lease.session_id,
                    &lease.token,
                    &probe,
                    "deny",
                    "deny",
                    &FixedClock::at(1_100)
                )
                .is_err());
            assert_eq!(budget(&engine, &lease, &request.task_id), before);
            assert_eq!(
                listener.accept().unwrap_err().kind(),
                std::io::ErrorKind::WouldBlock
            );
        }
    }
}

/// The contract admits adapters beyond the native chat ones, but the engine
/// runs only those it implements. A worker bound to any other adapter must be
/// refused before a reservation, a spend or a provider call.
#[test]
fn a_worker_adapter_the_engine_does_not_implement_is_refused_before_any_effect() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let (_root, engine, lease, mut selection) =
        admitted_fixture(&format!("http://{}/v1", listener.local_addr().unwrap()));
    let configuration = replace_profile(&engine, &lease, |profile| {
        profile.worker_policy.as_mut().unwrap().bindings[0].adapter =
            rrd_contract::EndpointWorkerAdapter::Other {
                adapter_id: CanonicalId::new("unimplemented-adapter").unwrap(),
                request_schema_sha256: digest::sha256_hex(b"request schema"),
                response_schema_sha256: digest::sha256_hex(b"response schema"),
            };
    });
    selection.expected_configuration_sha256 = configuration.sha256().unwrap();
    let request = selected_request(&engine, &lease, &listener, &selection, 1_500, false);
    let before = budget(&engine, &lease, &request.task_id);
    let key = engine.budget_key(SCOPE, &request.task_id);
    let stored = engine.storage.control().get(&key).unwrap();
    let refused = engine.execute_task_worker_with_clock(
        &lease.session_id,
        &lease.token,
        &request,
        "unimplemented",
        "unimplemented",
        &FixedClock::at(1_600),
    );
    assert!(
        matches!(
            refused,
            Err(ServiceError::EndpointCallFailed {
                execution_may_continue: false,
                ..
            })
        ),
        "{refused:?}"
    );
    assert_eq!(budget(&engine, &lease, &request.task_id), before);
    assert_eq!(engine.storage.control().get(&key).unwrap(), stored);
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
}

/// A selection that names its prompt flight is admitted only when the retained
/// flight is that task's and matches the identity the request states. Anything
/// else is refused before the controller is called.
#[test]
fn a_selection_must_match_the_prompt_flight_it_names() {
    use rrd_contract::{
        EndpointSelectionFlightIdentity, LaunchPromptFlight, PromptFlightContextMode,
        PromptFlightContextRequest, PromptFlightProvider,
    };
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let (_root, engine, lease, selection) =
        admitted_fixture(&format!("http://{}/v1", listener.local_addr().unwrap()));
    let launch = |task_id: Option<CanonicalId>, key: &str| {
        engine
            .launch_prompt_flight(
                &lease.session_id,
                &lease.token,
                &id(key),
                &LaunchPromptFlight {
                    scope: SCOPE.into(),
                    prompt: "protocol-test prompt".into(),
                    provider: PromptFlightProvider::Observe,
                    context_mode: PromptFlightContextMode::Fresh,
                    context: PromptFlightContextRequest {
                        valid_at: 1_200,
                        seeds: Vec::new(),
                        max_graph_depth: 2,
                        max_items: 8,
                        max_output_bytes: 65_536,
                        max_storage_keys: 4_096,
                    },
                    acceptance_marker: None,
                    task_id,
                },
                1_300,
                key,
                key,
            )
            .unwrap()
            .flight
            .record
    };
    let bound = launch(Some(selection.task_id.clone()), "flight-bound");
    let unbound = launch(None, "flight-unbound");
    let identity = |record: &rrd_contract::PromptFlightRecord| EndpointSelectionFlightIdentity {
        flight_id: record.flight_id.clone(),
        prompt_cohort_sha256: record.cohort_sha256.clone(),
        context_request_sha256: record
            .context
            .as_ref()
            .map(|context| context.request_sha256.clone()),
    };
    let propose = |flight: EndpointSelectionFlightIdentity| {
        let mut request = selection.clone();
        request.flight = Some(flight);
        engine.propose_task_endpoint_with_clock(
            &lease.session_id,
            &lease.token,
            &request,
            "flight-select",
            "flight-select",
            &FixedClock::at(1_400),
        )
    };
    let mut wrong_cohort = identity(&bound);
    wrong_cohort.prompt_cohort_sha256 = digest::sha256_hex(b"another cohort");
    let mut wrong_context = identity(&bound);
    wrong_context.context_request_sha256 = Some(digest::sha256_hex(b"another context request"));
    for (name, flight) in [
        ("cohort differs", wrong_cohort),
        ("context request differs", wrong_context),
        ("flight belongs to no task", identity(&unbound)),
    ] {
        assert!(
            matches!(
                propose(flight),
                Err(ServiceError::TaskAttemptInvalidated(
                    rrd_contract::TaskAttemptInvalidation::InputsChanged
                ))
            ),
            "{name}"
        );
    }
    listener.set_nonblocking(true).unwrap();
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock,
        "a refused flight must not reach the controller"
    );
}

#[test]
fn every_worker_permission_is_required_before_effect() {
    for action in [
        SecurityAction::WorkerExecute,
        SecurityAction::TaskRead,
        SecurityAction::ReasoningRead,
        SecurityAction::ReasoningWrite,
        SecurityAction::MemoryContextRead,
    ] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let (_root, engine, lease, selection) =
            admitted_fixture(&format!("http://{}/v1", listener.local_addr().unwrap()));
        let mut request = selected_request(&engine, &lease, &listener, &selection, 1_000, false);
        if action == SecurityAction::MemoryContextRead {
            request.context = Some(context_request());
        }
        let key = engine.budget_key(SCOPE, &request.task_id);
        let before = engine.storage.control().get(&key).unwrap();
        revoke(&engine, action);
        assert!(matches!(
            engine.execute_task_worker_with_clock(
                &lease.session_id,
                &lease.token,
                &request,
                "denied",
                "denied",
                &FixedClock::at(1_300)
            ),
            Err(ServiceError::PermissionDenied)
        ));
        assert_eq!(engine.storage.control().get(&key).unwrap(), before);
        assert_eq!(
            listener.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
    }
}

fn context_request() -> rrd_contract::AssembleContext {
    rrd_contract::AssembleContext {
        scope: SCOPE.into(),
        query: "task".into(),
        valid_at: 1_000,
        seeds: Vec::new(),
        max_graph_depth: 1,
        max_items: 8,
        max_output_bytes: 8192,
        max_storage_keys: 10_000,
    }
}

#[derive(Clone, Copy, Debug)]
enum Fault {
    Configuration,
    TaskClosed,
    Authority,
    Stop,
    LostResponse,
}

#[test]
fn changed_inputs_stop_and_response_loss_withhold_worker_output_and_survive_reopen() {
    for fault in [
        Fault::Configuration,
        Fault::TaskClosed,
        Fault::Authority,
        Fault::Stop,
        Fault::LostResponse,
    ] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let (root, engine, lease, selection) =
            admitted_fixture(&format!("http://{}/v1", listener.local_addr().unwrap()));
        let request = selected_request(&engine, &lease, &listener, &selection, 1_000, false);
        let before = budget(&engine, &lease, &request.task_id);
        let result = std::thread::scope(|scope| {
            let server = scope.spawn(|| {
                let (mut socket, body) = read_wire(&listener);
                match fault {
                    Fault::Configuration => {
                        replace_profile(&engine, &lease, |input| {
                            input.worker_policy.as_mut().unwrap().bindings[0]
                                .system_prompt
                                .push_str(" Changed during dispatch.");
                        });
                    }
                    Fault::TaskClosed => {
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
                                1_200,
                                "close",
                                "close",
                            )
                            .unwrap();
                    }
                    Fault::Authority => revoke(&engine, SecurityAction::WorkerExecute),
                    Fault::Stop => {
                        let current = budget(&engine, &lease, &request.task_id);
                        engine
                            .stop_task_budget(
                                &lease.session_id,
                                &lease.token,
                                &rrd_contract::StopTaskBudget {
                                    scope: SCOPE.into(),
                                    task_id: request.task_id.clone(),
                                    expected_budget_sha256: operation_digest(&current).unwrap(),
                                },
                                1_200,
                                "stop",
                                "stop",
                            )
                            .unwrap();
                    }
                    Fault::LostResponse => return, // Actually close the socket after receiving the full request.
                }
                let wire: serde_json::Value = serde_json::from_slice(&body).unwrap();
                reply(
                    &mut socket,
                    wire["model"].as_str().unwrap(),
                    "Controlled result after intervention.",
                );
            });
            let result = engine.execute_task_worker_with_clock(
                &lease.session_id,
                &lease.token,
                &request,
                "worker-fault",
                "worker-fault",
                &FixedClock::at(1_100),
            );
            server.join().unwrap();
            result
        });
        match fault {
            Fault::Configuration | Fault::TaskClosed => assert!(
                matches!(
                    result,
                    Err(ServiceError::TaskAttemptInvalidated(
                        rrd_contract::TaskAttemptInvalidation::InputsChanged
                    ))
                ),
                "{fault:?}: {result:?}"
            ),
            Fault::Authority => assert!(
                matches!(
                    result,
                    Err(ServiceError::TaskAttemptInvalidated(
                        rrd_contract::TaskAttemptInvalidation::AuthorizationChanged
                    ))
                ),
                "{result:?}"
            ),
            Fault::Stop | Fault::LostResponse => assert!(result.is_err(), "{fault:?}"),
        }
        let read = ReadTaskAttempt {
            scope: SCOPE.into(),
            task_id: request.task_id.clone(),
            attempt_id: request.attempt_id.clone(),
        };
        let retained = engine
            .read_task_attempt(
                &lease.session_id,
                &lease.token,
                &read,
                1_400,
                "read",
                "read",
            )
            .unwrap();
        let receipt = retained.attempt.as_ref().unwrap();
        assert!(receipt.worker_output.is_none() && receipt.outcome_sha256.is_none());
        let after = budget(&engine, &lease, &request.task_id);
        assert_eq!(after.reserved_steps, 2);
        assert_eq!(after.deadline_unix_ms, before.deadline_unix_ms);
        if matches!(
            fault,
            Fault::Configuration | Fault::TaskClosed | Fault::Authority
        ) {
            assert_eq!(receipt.status, TaskAttemptStatus::Invalidated);
            assert!(after.stopped.is_none());
        } else {
            assert_eq!(receipt.status, TaskAttemptStatus::Stopped);
            assert!(after.execution_may_continue);
        }
        let identity = engine.installed_engram.clone();
        drop(engine);
        let mut engine = RrdEngine::open_existing(root.path(), instance(), TOKEN_KEY).unwrap();
        engine.installed_engram = identity;
        configure(&mut engine, 8);
        assert_eq!(
            engine
                .read_task_attempt(
                    &lease.session_id,
                    &lease.token,
                    &read,
                    1_500,
                    "reopen",
                    "reopen"
                )
                .unwrap(),
            retained
        );
        assert_eq!(budget(&engine, &lease, &request.task_id), after);
        let capacity = crate::engine::endpoint_proposal::capacity::WorkerCapacityLease {
            scope: SCOPE.into(),
            task_id: request.task_id.clone(),
            attempt_id: id("different-worker"),
            binding_id: configuration_binding(&engine),
        };
        if matches!(fault, Fault::Stop | Fault::LostResponse) {
            assert!(matches!(
                engine.worker_capacity_transition(&capacity, Some((1, 1))),
                Err(ServiceError::InferenceBusy)
            ));
            let mut competing = selection.clone();
            competing.attempt_id = id("competing-controller");
            let controller = crate::engine::endpoint_proposal::capacity::CapacityLease {
                request: competing,
                controller_binding_id: capacity.binding_id.clone(),
            };
            assert!(matches!(
                engine.endpoint_capacity_transition(&controller, Some((1, 1))),
                Err(ServiceError::InferenceBusy)
            ));
        } else {
            assert!(engine
                .worker_capacity_transition(&capacity, Some((1, 1)))
                .is_ok());
        }
    }
}

fn configuration_binding(engine: &RrdEngine) -> CanonicalId {
    engine
        .load_endpoint_configuration()
        .unwrap()
        .1
        .profile
        .unwrap()
        .worker_policy
        .unwrap()
        .bindings[0]
        .binding_id
        .clone()
}

fn complete_worker(
    engine: &RrdEngine,
    lease: &SessionLease,
    listener: &TcpListener,
    request: &TaskWorkerRequest,
    at: u64,
) -> (rrd_contract::TaskWorkerOutput, serde_json::Value) {
    std::thread::scope(|scope| {
        let server = scope.spawn(|| {
            let (mut socket, body) = read_wire(listener);
            let wire: serde_json::Value = serde_json::from_slice(&body).unwrap();
            reply(
                &mut socket,
                wire["model"].as_str().unwrap(),
                "Controlled admitted draft.",
            );
            wire
        });
        let output = engine.execute_task_worker_with_clock(
            &lease.session_id,
            &lease.token,
            request,
            "worker",
            "worker",
            &FixedClock::at(at),
        );
        assert!(
            output.is_ok(),
            "worker admission failed before protocol response: {output:?}"
        );
        let wire = server.join().unwrap();
        (output.unwrap(), wire)
    })
}

#[test]
fn changed_configuration_allows_a_new_selection_and_worker_without_resetting_effort() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let (_root, engine, lease, mut selection) =
        admitted_fixture(&format!("http://{}/v1", listener.local_addr().unwrap()));
    let request = selected_request(&engine, &lease, &listener, &selection, 1_000, false);
    let before = budget(&engine, &lease, &request.task_id);
    std::thread::scope(|scope| {
        let server = scope.spawn(|| {
            let (mut socket, body) = read_wire(&listener);
            replace_profile(&engine, &lease, |input| {
                input.worker_policy.as_mut().unwrap().bindings[0]
                    .system_prompt
                    .push_str(" Revised protocol instruction.")
            });
            let wire: serde_json::Value = serde_json::from_slice(&body).unwrap();
            reply(
                &mut socket,
                wire["model"].as_str().unwrap(),
                "Stale controlled draft.",
            );
        });
        assert!(matches!(
            engine.execute_task_worker_with_clock(
                &lease.session_id,
                &lease.token,
                &request,
                "stale",
                "stale",
                &FixedClock::at(1_100)
            ),
            Err(ServiceError::TaskAttemptInvalidated(
                rrd_contract::TaskAttemptInvalidation::InputsChanged
            ))
        ));
        server.join().unwrap();
    });
    selection.expected_configuration_sha256 = engine
        .load_endpoint_configuration()
        .unwrap()
        .1
        .sha256()
        .unwrap();
    selection.attempt_id = id("fresh-selection");
    let mut successor = selected_request(&engine, &lease, &listener, &selection, 1_400, false);
    successor.attempt_id = id("fresh-worker");
    complete_worker(&engine, &lease, &listener, &successor, 1_500);
    let after = budget(&engine, &lease, &request.task_id);
    assert_eq!(after.reserved_steps, 4);
    assert_eq!(after.deadline_unix_ms, before.deadline_unix_ms);
    assert_eq!(after.limits, before.limits);
    assert!(after.stopped.is_none());
}

#[test]
fn context_is_engine_assembled_and_derived_readback_requires_current_context_permission() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let (_root, engine, lease, selection) =
        admitted_fixture(&format!("http://{}/v1", listener.local_addr().unwrap()));
    let mut request = selected_request(&engine, &lease, &listener, &selection, 1_000, false);
    let mut context = context_request();
    context.query.clear();
    context.seeds.push(rrd_contract::DataReference {
        kind: CanonicalId::new("rrflow-task").unwrap(),
        id: request.task_id.clone(),
    });
    request.context = Some(context.clone());
    let (output, wire) = complete_worker(&engine, &lease, &listener, &request, 1_100);
    let packet: serde_json::Value =
        serde_json::from_str(wire["messages"][1]["content"].as_str().unwrap()).unwrap();
    let captured: rrd_contract::ContextPacket =
        serde_json::from_value(packet["context"].clone()).unwrap();
    captured.validate().unwrap();
    assert!(!captured.items.is_empty());
    let provenance = output.context.as_ref().unwrap();
    assert_eq!(provenance.packet_sha256, captured.packet_sha256);
    assert_eq!(
        provenance.request_sha256,
        rrd_contract::context_request_sha256(&context).unwrap()
    );
    let read = ReadTaskAttempt {
        scope: SCOPE.into(),
        task_id: request.task_id.clone(),
        attempt_id: request.attempt_id.clone(),
    };
    assert_eq!(
        engine
            .read_task_attempt(
                &lease.session_id,
                &lease.token,
                &read,
                1_200,
                "read",
                "read"
            )
            .unwrap()
            .attempt
            .unwrap()
            .worker_output,
        Some(output)
    );
    revoke(&engine, SecurityAction::MemoryContextRead);
    assert!(matches!(
        engine.read_task_attempt(
            &lease.session_id,
            &lease.token,
            &read,
            1_300,
            "denied-read",
            "denied-read"
        ),
        Err(ServiceError::PermissionDenied)
    ));
}
