//! Synthetic HTTP services exercise the real engine composition, not model quality.
use super::*;
use rrd_contract::{
    ApplyEndpointConfiguration, EndpointLocator, EndpointProfileInput, InstalledEngramIdentity,
    LfgAnalysisPolicy, PreviewEndpointConfiguration, ReadTaskAttempt,
};
use std::io::{Read, Write};
use std::net::TcpListener;

type DiagnosticSink = Arc<std::sync::Mutex<Vec<serde_json::Value>>>;

thread_local! {
    /// The sink the capturing test installed on this thread, if any.
    static LFG_SINK: std::cell::RefCell<Option<DiagnosticSink>> = const { std::cell::RefCell::new(None) };
}

/// Records one event's fields the way `tracing_subscriber::fmt().json()` does.
#[derive(Default)]
struct JsonFields(serde_json::Map<String, serde_json::Value>);

impl tracing::field::Visit for JsonFields {
    fn record_f64(&mut self, field: &tracing::field::Field, value: f64) {
        self.0.insert(field.name().to_owned(), value.into());
    }
    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        self.0.insert(field.name().to_owned(), value.into());
    }
    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        self.0.insert(field.name().to_owned(), value.into());
    }
    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        self.0.insert(field.name().to_owned(), value.into());
    }
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        self.0.insert(field.name().to_owned(), value.into());
    }
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.0
            .insert(field.name().to_owned(), format!("{value:?}").into());
    }
}

/// Routes `rrd_engine::lfg` events to the sink of the thread that emits them.
///
/// A per-test scoped subscriber is not enough in a parallel test binary:
/// `tracing` keeps one process-wide maximum level, rebuilt only when the first
/// scoped subscriber appears or a global one is set, so a DEBUG capture that
/// starts while another test's scoped subscriber is alive is filtered out
/// before it reaches the thread's subscriber. One global subscriber whose hint
/// admits DEBUG keeps the filter open for every thread; tests that still use a
/// scoped subscriber keep precedence on their own threads.
struct LfgSinkLayer;

impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for LfgSinkLayer {
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _context: tracing_subscriber::layer::Context<'_, S>,
    ) {
        if event.metadata().target() != "rrd_engine::lfg" {
            return;
        }
        LFG_SINK.with(|sink| {
            if let Some(sink) = sink.borrow().as_ref() {
                let mut fields = JsonFields::default();
                event.record(&mut fields);
                sink.lock().unwrap().push(serde_json::json!({
                    "level": event.metadata().level().to_string(),
                    "target": event.metadata().target(),
                    "fields": fields.0,
                }));
            }
        });
    }
}

fn install_diagnostic_subscriber() {
    use tracing_subscriber::layer::SubscriberExt;
    static INSTALLED: std::sync::OnceLock<()> = std::sync::OnceLock::new();
    INSTALLED.get_or_init(|| {
        // Another global subscriber, if one ever exists, keeps precedence; the
        // capture then depends on it admitting DEBUG for this target.
        let _ = tracing::subscriber::set_global_default(
            tracing_subscriber::registry().with(LfgSinkLayer),
        );
    });
}

fn serve_once(listener: TcpListener, respond: impl FnOnce(serde_json::Value) -> serde_json::Value) {
    listener.set_nonblocking(true).unwrap();
    let deadline = Instant::now() + Duration::from_secs(15);
    let (mut stream, _) = loop {
        match listener.accept() {
            Ok(value) => break value,
            Err(error)
                if error.kind() == std::io::ErrorKind::WouldBlock && Instant::now() < deadline =>
            {
                std::thread::sleep(Duration::from_millis(10))
            }
            Err(error) => panic!("listener did not receive request: {error}"),
        }
    };
    // On Windows the accepted socket can inherit the listener's nonblocking mode.
    stream.set_nonblocking(false).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let mut bytes = Vec::new();
    let mut chunk = [0u8; 4096];
    let request = loop {
        let count = stream.read(&mut chunk).unwrap();
        assert!(count > 0);
        bytes.extend_from_slice(&chunk[..count]);
        if let Some(split) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
            let headers = String::from_utf8_lossy(&bytes[..split]);
            let length: usize = headers
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse().unwrap())
                })
                .unwrap();
            if bytes.len() >= split + 4 + length {
                break serde_json::from_slice(&bytes[split + 4..split + 4 + length]).unwrap();
            }
        }
    };
    let body = serde_json::to_vec(&respond(request)).unwrap();
    write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )
    .unwrap();
    stream.write_all(&body).unwrap();
}

#[test]
fn real_engine_composes_bounded_lfg_then_controller_and_reopens_receipt() {
    run_flow(Scenario::Normal, None);
}

#[test]
fn lfg_selection_drives_a_separate_governed_worker_and_both_reopen() {
    run_flow(Scenario::Worker, None);
}

#[test]
fn changed_configuration_during_lfg_records_result_without_controller_dispatch() {
    run_flow(Scenario::ConfigurationChanged, None);
}

#[test]
fn task_closed_during_lfg_records_result_without_controller_dispatch() {
    run_flow(Scenario::TaskClosed, None);
}

fn captured_diagnostics(scenario: Scenario) -> Vec<serde_json::Value> {
    install_diagnostic_subscriber();
    let sink: DiagnosticSink = Arc::new(std::sync::Mutex::new(Vec::new()));
    LFG_SINK.with(|slot| *slot.borrow_mut() = Some(Arc::clone(&sink)));
    run_flow(scenario, None);
    LFG_SINK.with(|slot| *slot.borrow_mut() = None);
    let events = sink.lock().unwrap().clone();
    events
}

#[test]
fn lfg_diagnostics_are_bounded_and_report_invalidated_checkpoint() {
    let events = captured_diagnostics(Scenario::TaskClosed);
    assert_eq!(events.len(), 2, "captured LFG diagnostics: {events:?}");
    let transport = events[0]["fields"].as_object().unwrap();
    let checkpoint = events[1]["fields"].as_object().unwrap();
    assert_eq!(transport["outcome"], "transport_completed");
    assert!(transport["elapsed_ms"].is_u64());
    assert_eq!(
        checkpoint["outcome"], "checkpoint_committed",
        "captured LFG diagnostics: {events:?}"
    );
    assert_eq!(checkpoint["invalidated"], true);
    assert_eq!(
        transport["operation_sha256"],
        checkpoint["operation_sha256"]
    );
    for fields in [transport, checkpoint] {
        for key in fields.keys() {
            assert!(matches!(
                key.as_str(),
                "message"
                    | "operation_sha256"
                    | "outcome"
                    | "elapsed_ms"
                    | "analyzer_request_sha256"
                    | "analyzer_response_sha256"
                    | "invalidated"
            ));
        }
    }
}

#[test]
fn lfg_diagnostics_report_uncertain_transport_failure_without_payload() {
    let events = captured_diagnostics(Scenario::InvalidLfg);
    assert_eq!(events.len(), 1);
    let fields = events[0]["fields"].as_object().unwrap();
    assert_eq!(fields["outcome"], "transport_failed");
    assert_eq!(fields["execution_may_continue"], true);
    assert!(fields["elapsed_ms"].is_u64());
    for key in fields.keys() {
        assert!(matches!(
            key.as_str(),
            "message" | "operation_sha256" | "outcome" | "elapsed_ms" | "execution_may_continue"
        ));
    }
}

#[test]
fn controller_response_failure_keeps_lfg_checkpoint_and_uncertain_capacity() {
    run_flow(Scenario::InvalidController, None);
}

#[test]
#[ignore = "requires an explicitly configured local LFG service with the fixture policy"]
fn real_lfg_service_flows_through_engine_controller_and_reopen() {
    let url = std::env::var("RRFLOW_TEST_LFG_URL").expect("configure the local LFG service origin");
    run_flow(Scenario::Normal, Some(url));
}

#[test]
#[ignore = "requires an explicitly configured local LFG service with the fixture policy"]
fn real_lfg_selection_drives_governed_worker_and_reopens() {
    let url = std::env::var("RRFLOW_TEST_LFG_URL").expect("configure the local LFG service origin");
    run_flow(Scenario::Worker, Some(url));
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Scenario {
    Normal,
    Worker,
    ConfigurationChanged,
    TaskClosed,
    InvalidLfg,
    InvalidController,
}

fn run_flow(scenario: Scenario, real_lfg: Option<String>) {
    let execute_worker = scenario == Scenario::Worker;
    let change_configuration = scenario == Scenario::ConfigurationChanged;
    let close_task = scenario == Scenario::TaskClosed;
    let reject_lfg = scenario == Scenario::InvalidLfg;
    let reject_controller = scenario == Scenario::InvalidController;
    let (root, mut engine, bootstrap, mut configuration, mut request, _, _) =
        proposal::tuple_fixture();
    let fixture_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../compute/rrd-inference/tests/fixtures/lfg-synthetic-observation.json");
    let fixture: serde_json::Value =
        serde_json::from_slice(&std::fs::read(fixture_path).unwrap()).unwrap();
    let analyzer = if real_lfg.is_none() {
        Some(TcpListener::bind("127.0.0.1:0").unwrap())
    } else {
        None
    };
    let controller = TcpListener::bind("127.0.0.1:0").unwrap();
    let worker_listener = execute_worker.then(|| controller.try_clone().unwrap());
    engine.installed_engram = Some(InstalledEngramIdentity {
        project_id: CanonicalId::new("lfg-flow-project").unwrap(),
        engram_id: CanonicalId::new("lfg-flow-engram").unwrap(),
        instance_id: instance(),
    });
    let profile = configuration.profile.as_mut().unwrap();
    let policy = profile.selection_policy.as_mut().unwrap();
    policy.lfg = Some(LfgAnalysisPolicy {
        base_url: real_lfg.unwrap_or_else(|| {
            format!(
                "http://{}",
                analyzer.as_ref().unwrap().local_addr().unwrap()
            )
        }),
        assessment_sha256: fixture["request"]["expectedAssessmentSha256"]
            .as_str()
            .unwrap()
            .into(),
        rules_sha256: fixture["request"]["expectedRulesSha256"]
            .as_str()
            .unwrap()
            .into(),
        timeout_ms: 3_000,
        maximum_input_bytes: 65_536,
        maximum_request_bytes: 131_072,
        maximum_response_bytes: 65_536,
    });
    let binding = profile
        .bindings
        .iter_mut()
        .find(|item| item.id == policy.controller_binding_id)
        .unwrap();
    binding.endpoint = EndpointLocator::Url {
        url: format!("http://{}", controller.local_addr().unwrap()),
    };
    binding.credential = None;
    let model = binding.service_id.to_string();
    profile.worker_policy = execute_worker.then(|| rrd_contract::EndpointWorkerPolicy {
        contract_version: 1,
        allow_network: true,
        allow_context_export: true,
        allow_source_export: false,
        allow_unverified_identity: true,
        maximum_concurrent_calls: 1,
        bindings: vec![rrd_contract::EndpointWorkerBinding {
            binding_id: policy.candidates[0].binding_id.clone(),
            adapter: Default::default(),
            protocol_evidence_sha256: digest::sha256_hex(b"synthetic worker protocol only"),
            system_prompt: "Return a synthetic draft without project effects.".into(),
            generation: rrd_contract::EndpointGenerationSettings {
                maximum_output_tokens: 256,
                token_parameter: rrd_contract::EndpointTokenParameter::MaxTokens,
                temperature_milli: None,
                response_format: rrd_contract::EndpointResponseFormat::Text,
            },
            maximum_response_bytes: 16_384,
            effort_values: Default::default(),
            allow_default_effort: true,
        }],
    });
    let input = EndpointProfileInput {
        contract_version: profile.contract_version,
        name: profile.name.clone(),
        bindings: profile.bindings.clone(),
        selection_policy: profile.selection_policy.clone(),
        observation_policy: profile.observation_policy.clone(),
        worker_policy: profile.worker_policy.clone(),
        assessment_policy: None,
    };
    let mut next_input = input.clone();
    next_input
        .selection_policy
        .as_mut()
        .unwrap()
        .system_prompt
        .push_str(" updated");
    let plan = engine
        .preview_endpoint_configuration(
            &bootstrap.session_id,
            &bootstrap.token,
            &PreviewEndpointConfiguration {
                expected_revision: 0,
                profile: Some(input),
            },
            60,
            "preview",
            "preview",
        )
        .unwrap();
    configuration = engine
        .apply_endpoint_configuration(
            &bootstrap.session_id,
            &bootstrap.token,
            &ApplyEndpointConfiguration {
                approved_plan_sha256: plan.plan_sha256.clone(),
                plan,
            },
            61,
            "apply",
            "apply",
        )
        .unwrap();
    request.expected_configuration_sha256 = configuration.sha256().unwrap();

    let principal_id = engine.instance.clone();
    let principal = rrd_security::Principal {
        id: principal_id.clone(),
        kind: rrd_security::PrincipalKind::Service,
        credential_sha256: digest::sha256_hex(bootstrap.token.as_str().as_bytes()),
        credential_revision: 1,
        not_before_unix_ms: 1,
        expires_at_unix_ms: u64::MAX,
        disabled: false,
        role_ids: Default::default(),
        grants: [
            SecurityAction::SessionCreate,
            SecurityAction::EndpointPropose,
            SecurityAction::WorkerExecute,
            SecurityAction::TaskRead,
            SecurityAction::TaskWrite,
            SecurityAction::ReasoningWrite,
            SecurityAction::ReasoningRead,
            SecurityAction::EngramAdmin,
        ]
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
            70,
            "test",
            "initialize",
            "initialize",
        )
        .unwrap();
    let engine = Arc::new(engine);
    let edit_engine = Arc::clone(&engine);
    let lease = engine
        .create_authenticated_session(
            &principal_id,
            bootstrap.token.as_str().as_bytes(),
            &session_request(9_000, 4),
            &id("lfg-flow-session"),
            100,
            "session",
            "session",
        )
        .unwrap();
    let edit_lease = lease.clone();
    let task_to_close = rrd_contract::CloseTask {
        scope: request.scope.clone(),
        task_id: request.task_id.clone(),
        status: rrd_contract::TaskStatus::Abandoned,
        outcome: None,
    };

    let analyzer_thread = analyzer.map(|analyzer| {
        std::thread::spawn(move || {
            serve_once(analyzer, |input| {
                assert_eq!(
                    digest::sha256_hex(input["text"].as_str().unwrap().as_bytes()),
                    input["inputSha256"]
                );
                if change_configuration {
                    let plan = edit_engine
                        .preview_endpoint_configuration(
                            &edit_lease.session_id,
                            &edit_lease.token,
                            &PreviewEndpointConfiguration {
                                expected_revision: 1,
                                profile: Some(next_input),
                            },
                            900,
                            "change-preview",
                            "change-preview",
                        )
                        .unwrap();
                    edit_engine
                        .apply_endpoint_configuration(
                            &edit_lease.session_id,
                            &edit_lease.token,
                            &ApplyEndpointConfiguration {
                                approved_plan_sha256: plan.plan_sha256.clone(),
                                plan,
                            },
                            901,
                            "change-apply",
                            "change-apply",
                        )
                        .unwrap();
                }
                if close_task {
                    edit_engine
                        .close_task(
                            &edit_lease.session_id,
                            &edit_lease.token,
                            &task_to_close,
                            902,
                            "task-close",
                            "task-close",
                        )
                        .unwrap();
                }
                let mut output = fixture["observation"].clone();
                for field in ["requestId", "revision", "inputSha256"] {
                    output[field] = input[field].clone();
                }
                output["assessmentSha256"] = input["expectedAssessmentSha256"].clone();
                output["rulesSha256"] = input["expectedRulesSha256"].clone();
                if reject_lfg {
                    output["inputSha256"] = serde_json::Value::String("0".repeat(64));
                }
                output
            })
        })
    });
    let response_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../transport/rrd-contract/tests/fixtures/endpoint-selection-observed/available/choice-response.json");
    let mut response: serde_json::Value =
        serde_json::from_slice(&crate::engine::tests::observed_configuration::read(
            response_path.parent().unwrap(),
            "choice-response.json",
        ))
        .unwrap();
    if reject_controller {
        response = serde_json::json!({"synthetic": "invalid controller response"});
    }
    let (tx, rx) = mpsc::channel();
    let (controller_thread, idle_controller) = if change_configuration || close_task || reject_lfg {
        controller.set_nonblocking(true).unwrap();
        (None, Some(controller))
    } else {
        (
            Some(std::thread::spawn(move || {
                serve_once(controller, |input| {
                    let packet: serde_json::Value =
                        serde_json::from_str(input["messages"][1]["content"].as_str().unwrap())
                            .unwrap();
                    tx.send(packet).unwrap();
                    response
                })
            })),
            None,
        )
    };
    let proposal = engine.propose_task_endpoint_with_clock(
        &lease.session_id,
        &lease.token,
        &request,
        "lfg-flow",
        "lfg-flow",
        &FixedClock::at(1_000),
    );
    if let Some(analyzer_thread) = analyzer_thread {
        analyzer_thread.join().unwrap();
    }
    let proposal = if reject_lfg {
        assert!(matches!(
            proposal,
            Err(ServiceError::EndpointCallFailed {
                execution_may_continue: true,
                ..
            })
        ));
        assert!(
            matches!(idle_controller.unwrap().accept(), Err(error) if error.kind() == std::io::ErrorKind::WouldBlock)
        );
        None
    } else if change_configuration || close_task {
        assert!(matches!(
            proposal,
            Err(ServiceError::TaskAttemptInvalidated(
                rrd_contract::TaskAttemptInvalidation::InputsChanged
            ))
        ));
        assert!(
            matches!(idle_controller.unwrap().accept(), Err(error) if error.kind() == std::io::ErrorKind::WouldBlock)
        );
        None
    } else {
        controller_thread.unwrap().join().unwrap();
        let packet = rx.recv().unwrap();
        assert_eq!(packet["lfg"]["request_id"], request.attempt_id.as_str());
        if reject_controller {
            assert!(matches!(
                proposal,
                Err(ServiceError::EndpointCallFailed {
                    execution_may_continue: true,
                    ..
                })
            ));
            None
        } else {
            let proposal = proposal.unwrap();
            assert_eq!(
                packet["lfg"]["response_sha256"],
                proposal.lfg.as_ref().unwrap().response_sha256
            );
            Some(proposal)
        }
    };
    let read_request = ReadTaskAttempt {
        scope: request.scope.clone(),
        task_id: request.task_id.clone(),
        attempt_id: request.attempt_id.clone(),
    };
    let before = engine
        .read_task_attempt(
            &lease.session_id,
            &lease.token,
            &read_request,
            1_001,
            "read",
            "read",
        )
        .unwrap();
    assert_eq!(before.attempt.as_ref().unwrap().lfg.is_some(), !reject_lfg);
    assert_eq!(before.attempt.as_ref().unwrap().endpoint_proposal, proposal);
    let worker = if let Some(listener) = worker_listener {
        let selected = proposal.as_ref().unwrap();
        let worker_request = rrd_contract::TaskWorkerRequest {
            scope: request.scope.clone(),
            task_id: request.task_id.clone(),
            attempt_id: id("lfg-followup-worker"),
            selection_attempt_id: selected.attempt_id.clone(),
            expected_selection_sha256: crate::engine::operation_digest(selected).unwrap(),
            expected_task_sha256: selected.task_sha256.clone(),
            expected_configuration_sha256: selected.configuration_sha256.clone(),
            max_storage_keys: rrd_contract::MAX_TASK_STORAGE_KEYS,
            context: None,
            source: None,
        };
        let expected_selection = worker_request.expected_selection_sha256.clone();
        let worker_thread = std::thread::spawn(move || {
            serve_once(listener, |input| {
                let packet: serde_json::Value =
                    serde_json::from_str(input["messages"][1]["content"].as_str().unwrap())
                        .unwrap();
                assert_eq!(packet["selection_sha256"], expected_selection);
                serde_json::json!({
                    "id": "synthetic-worker", "object": "chat.completion", "model": model,
                    "choices": [{"index": 0, "message": {"role": "assistant",
                        "content": "Synthetic worker draft."}, "finish_reason": "stop"}]
                })
            })
        });
        let output = engine
            .execute_task_worker_with_clock(
                &lease.session_id,
                &lease.token,
                &worker_request,
                "worker",
                "worker",
                &FixedClock::at(1_100),
            )
            .unwrap();
        worker_thread.join().unwrap();
        assert_eq!(output.content, "Synthetic worker draft.");
        let read_worker = ReadTaskAttempt {
            scope: request.scope.clone(),
            task_id: request.task_id.clone(),
            attempt_id: worker_request.attempt_id.clone(),
        };
        let snapshot = engine
            .read_task_attempt(
                &lease.session_id,
                &lease.token,
                &read_worker,
                1_101,
                "read-worker",
                "read-worker",
            )
            .unwrap();
        assert_eq!(
            snapshot.attempt.as_ref().unwrap().worker_output.as_ref(),
            Some(&output)
        );
        assert!(snapshot.attempt.as_ref().unwrap().lfg.is_none());
        let mut forged = snapshot.clone();
        let mut advisory = before.attempt.as_ref().unwrap().lfg.clone().unwrap();
        advisory.request_id = worker_request.attempt_id.clone();
        forged.attempt.as_mut().unwrap().lfg = Some(advisory);
        assert!(forged.validate().is_err());
        Some((worker_request, read_worker, snapshot))
    } else {
        None
    };
    if change_configuration || close_task {
        assert_eq!(
            before.attempt.as_ref().unwrap().status,
            rrd_contract::TaskAttemptStatus::Invalidated
        );
        let budget = state(&engine, &lease, &request.task_id);
        assert!(budget.active_attempt.is_none() && budget.stopped.is_none());
    }
    if reject_controller || reject_lfg {
        assert_eq!(
            before.attempt.as_ref().unwrap().status,
            rrd_contract::TaskAttemptStatus::Stopped
        );
        assert!(state(&engine, &lease, &request.task_id).execution_may_continue);
    }
    drop(engine);
    let mut reopened = RrdEngine::open_existing(root.path(), instance(), TOKEN_KEY).unwrap();
    configure(&mut reopened, 3);
    assert_eq!(
        reopened
            .read_task_attempt(
                &lease.session_id,
                &lease.token,
                &read_request,
                1_002,
                "read",
                "read"
            )
            .unwrap(),
        before
    );
    if let Some((worker_request, read_worker, snapshot)) = worker {
        assert_eq!(
            reopened
                .read_task_attempt(
                    &lease.session_id,
                    &lease.token,
                    &read_worker,
                    1_102,
                    "read-worker",
                    "read-worker",
                )
                .unwrap(),
            snapshot
        );
        assert!(reopened
            .execute_task_worker_with_clock(
                &lease.session_id,
                &lease.token,
                &worker_request,
                "replay-worker",
                "replay-worker",
                &FixedClock::at(1_200),
            )
            .is_err());
    }
    assert!(reopened
        .propose_task_endpoint_with_clock(
            &lease.session_id,
            &lease.token,
            &request,
            "replay",
            "replay",
            &FixedClock::at(1_100)
        )
        .is_err());
}
