use super::*;
use rrd_contract::{
    AssessmentInputKind, AssessmentPracticeIdentity, AssessmentScoreDirection,
    EndpointAssessmentPolicy, EndpointConfiguration, EndpointGenerationSettings,
    EndpointProfileInput, EndpointResponseFormat, EndpointTokenParameter,
    PreviewEndpointConfiguration, ReadTaskAssessment, TaskAssessmentDecision,
    TaskAssessmentDescriptor, TaskAssessmentRequest, TaskAssessmentValue, TASK_ASSESSMENT_VERSION,
};
use std::sync::atomic::{AtomicU64, Ordering};

struct AssessmentClock(AtomicU64);

struct AssessmentPolicyOptions<'a> {
    allow_network: bool,
    allow_source_export: bool,
    allow_context_export: bool,
    system_prompt: &'a str,
}

impl AssessmentClock {
    fn at(at: u64) -> Self {
        Self(AtomicU64::new(at))
    }

    fn advance_to(&self, at: u64) {
        self.0.fetch_max(at, Ordering::SeqCst);
    }
}

impl crate::engine::clock::ClockSource for AssessmentClock {
    fn observe(
        &self,
    ) -> std::result::Result<
        crate::engine::clock::ClockObservation,
        crate::engine::clock::ClockObservationFailure,
    > {
        Ok(crate::engine::clock::ClockObservation {
            format_version: crate::engine::clock::CLOCK_OBSERVATION_FORMAT_VERSION,
            source: crate::engine::clock::ClockSourceKind::HostSystemTime,
            trust: crate::engine::clock::ClockTrustLevel::Unverified,
            observed_at_unix_ms: self.0.load(Ordering::SeqCst),
        })
    }
}

fn publish_assessment_policy(
    fixture: &mut SourceWorkerFixture,
    allow_network: bool,
    allow_source_export: bool,
    allow_context_export: bool,
) -> EndpointConfiguration {
    publish_assessment_policy_with_prompt(
        fixture,
        allow_network,
        allow_source_export,
        allow_context_export,
        "Return a typed advisory assessment of the supplied packet.",
    )
}

fn publish_assessment_policy_with_prompt(
    fixture: &mut SourceWorkerFixture,
    allow_network: bool,
    allow_source_export: bool,
    allow_context_export: bool,
    system_prompt: &str,
) -> EndpointConfiguration {
    publish_assessment_policy_with_prompt_for(
        &fixture.engine,
        &fixture.lease,
        &mut fixture.now,
        &mut fixture.selection,
        AssessmentPolicyOptions {
            allow_network,
            allow_source_export,
            allow_context_export,
            system_prompt,
        },
    )
}

fn publish_assessment_policy_with_prompt_for(
    engine: &RrdEngine,
    lease: &SessionLease,
    fixture_now: &mut u64,
    selection: &mut EndpointSelectionRequest,
    options: AssessmentPolicyOptions<'_>,
) -> EndpointConfiguration {
    let read_at = *fixture_now + 1;
    let current = engine
        .read_endpoint_configuration(
            &lease.session_id,
            &lease.token,
            read_at,
            &format!("assessment-config-read-{read_at}"),
            &format!("assessment-config-read-{read_at}"),
        )
        .unwrap();
    let profile = current.profile.unwrap();
    let mut selection_policy = profile.selection_policy.clone().unwrap();
    selection_policy.maximum_concurrent_proposals = 1;
    let controller = selection_policy.controller_binding_id.clone();
    let mut bindings = profile.bindings;
    for binding in &mut bindings {
        binding.limits.max_concurrency = 1;
    }
    let mut worker_policy = profile.worker_policy.clone().unwrap();
    worker_policy.maximum_concurrent_calls = 1;
    let binding_limit = bindings
        .iter()
        .find(|binding| binding.id == controller)
        .unwrap()
        .limits
        .max_input_bytes;
    let assessment_policy = EndpointAssessmentPolicy {
        contract_version: TASK_ASSESSMENT_VERSION,
        allow_network: options.allow_network,
        allow_source_export: options.allow_source_export,
        allow_context_export: options.allow_context_export,
        system_prompt: options.system_prompt.into(),
        generation: EndpointGenerationSettings {
            maximum_output_tokens: 256,
            token_parameter: EndpointTokenParameter::MaxTokens,
            temperature_milli: None,
            response_format: EndpointResponseFormat::JsonObject,
        },
        maximum_input_bytes: binding_limit.min(64 * 1024),
        maximum_response_bytes: 16 * 1024,
    };
    let input = EndpointProfileInput {
        contract_version: profile.contract_version,
        name: profile.name,
        bindings,
        selection_policy: Some(selection_policy),
        worker_policy: Some(worker_policy),
        observation_policy: profile.observation_policy,
        assessment_policy: Some(assessment_policy),
    };
    let preview_at = read_at + 1;
    let plan = engine
        .preview_endpoint_configuration(
            &lease.session_id,
            &lease.token,
            &PreviewEndpointConfiguration {
                expected_revision: current.revision,
                profile: Some(input),
            },
            preview_at,
            &format!("assessment-config-preview-{preview_at}"),
            &format!("assessment-config-preview-{preview_at}"),
        )
        .unwrap();
    let applied = engine
        .apply_endpoint_configuration(
            &lease.session_id,
            &lease.token,
            &rrd_contract::ApplyEndpointConfiguration {
                approved_plan_sha256: plan.plan_sha256.clone(),
                plan,
            },
            preview_at + 1,
            &format!("assessment-config-apply-{}", preview_at + 1),
            &format!("assessment-config-apply-{}", preview_at + 1),
        )
        .unwrap();
    *fixture_now = preview_at + 2;
    selection.expected_configuration_sha256 = applied.sha256().unwrap();
    applied
}

fn source_worker_assessment_fixture(listener: &TcpListener) -> SourceWorkerFixture {
    let base = host_unix_ms().saturating_sub(30_000);
    source_worker_fixture_for_path_at(listener, true, false, "alpha.txt", base)
}

fn configure_assessment_budget(fixture: &mut SourceWorkerFixture) {
    configure_assessment_budget_with_repetition(fixture, None);
}

fn configure_assessment_budget_with_repetition(
    fixture: &mut SourceWorkerFixture,
    max_equivalent_attempts: Option<u64>,
) {
    let at = fixture.now + 1;
    let configured = fixture
        .engine
        .configure_task_budget(
            &fixture.lease.session_id,
            &fixture.lease.token,
            &rrd_contract::ConfigureTaskBudget {
                scope: fixture.selection.scope.clone(),
                task_id: fixture.selection.task_id.clone(),
                limits: fixture.engine.estate_configuration.reasoning.clone(),
                repetition_policy: max_equivalent_attempts.map(|max_equivalent_attempts| {
                    rrd_contract::TaskRepetitionPolicy {
                        max_equivalent_attempts,
                    }
                }),
            },
            at,
            "assessment-budget-configure",
            "assessment-budget-configure",
        )
        .unwrap();
    let budget = configured.budget.expect("public budget was configured");
    fixture.now = at.max(budget.last_observed_at_unix_ms);
}

fn publish_metadata_observation_policy(fixture: &mut SourceWorkerFixture) -> EndpointConfiguration {
    let read_at = fixture.now + 1;
    let current = fixture
        .engine
        .read_endpoint_configuration(
            &fixture.lease.session_id,
            &fixture.lease.token,
            read_at,
            "assessment-observation-config-read",
            "assessment-observation-config-read",
        )
        .unwrap();
    let profile = current.profile.unwrap();
    let binding_id = CanonicalId::new("assessment-metadata-observer").unwrap();
    let credential = rrd_contract::CredentialReference {
        provider: CanonicalId::new("assessment-test-registry").unwrap(),
        name: "metadata-observation".into(),
    };
    let mut bindings = profile.bindings;
    bindings.push(rrd_contract::EndpointBinding {
        id: binding_id.clone(),
        class: rrd_contract::WorkloadClass::BehaviorUpdates,
        kind: rrd_contract::BindingKind::Api,
        endpoint: rrd_contract::EndpointLocator::Url {
            url: "http://127.0.0.1:9/".into(),
        },
        service_id: CanonicalId::new("assessment-metadata-sink").unwrap(),
        manifest_sha256: None,
        credential: Some(credential),
        placement: None,
        footprint: rrd_contract::DeclaredFootprint {
            memory_bytes: 1,
            gpu: rrd_contract::GpuRequirement::None,
        },
        limits: rrd_contract::BindingLimits {
            max_concurrency: 1,
            timeout_ms: 1_000,
            max_input_bytes: 16_384,
        },
    });
    let input = EndpointProfileInput {
        contract_version: profile.contract_version,
        name: profile.name,
        bindings,
        selection_policy: profile.selection_policy,
        worker_policy: profile.worker_policy,
        observation_policy: Some(rrd_contract::ObservationExportPolicy {
            contract_version: rrd_contract::OBSERVATION_DELIVERY_VERSION,
            binding_id,
            expected_devspace: "test-devspace".into(),
            protocol_evidence_sha256: digest::sha256_hex(
                b"controlled metadata-only observation preparation; no receiver call",
            ),
            allow_network: true,
            allow_metadata_export: true,
            allowed_scopes: vec![fixture.selection.scope.clone()],
            max_retained_deliveries: 1,
            retention_ms: 10_000,
            retire_expired_uncertain: false,
            max_attempts: 1,
            retry_delay_ms: 10,
            max_response_bytes: 1_024,
        }),
        assessment_policy: profile.assessment_policy,
    };
    let preview_at = read_at + 1;
    let plan = fixture
        .engine
        .preview_endpoint_configuration(
            &fixture.lease.session_id,
            &fixture.lease.token,
            &PreviewEndpointConfiguration {
                expected_revision: current.revision,
                profile: Some(input),
            },
            preview_at,
            "assessment-observation-config-preview",
            "assessment-observation-config-preview",
        )
        .unwrap();
    let applied = fixture
        .engine
        .apply_endpoint_configuration(
            &fixture.lease.session_id,
            &fixture.lease.token,
            &rrd_contract::ApplyEndpointConfiguration {
                approved_plan_sha256: plan.plan_sha256.clone(),
                plan,
            },
            preview_at + 1,
            "assessment-observation-config-apply",
            "assessment-observation-config-apply",
        )
        .unwrap();
    fixture.now = preview_at + 2;
    applied
}

fn assessment_request(
    fixture: &SourceWorkerFixture,
    suffix: &str,
    descriptor: TaskAssessmentDescriptor,
) -> TaskAssessmentRequest {
    TaskAssessmentRequest {
        contract_version: TASK_ASSESSMENT_VERSION,
        scope: fixture.selection.scope.clone(),
        task_id: fixture.selection.task_id.clone(),
        attempt_id: id(&format!("assessment-{suffix}")),
        expected_task_sha256: fixture.selection.expected_task_sha256.clone(),
        expected_configuration_sha256: fixture.selection.expected_configuration_sha256.clone(),
        max_storage_keys: MAX_TASK_STORAGE_KEYS,
        source: fixture.source.clone(),
        descriptor,
        practice_identity: AssessmentPracticeIdentity {
            question_set_id: CanonicalId::new("assessment-set").unwrap(),
            question_id: CanonicalId::new("assessment-question").unwrap(),
            template_id: CanonicalId::new("assessment-template").unwrap(),
            template_revision: 1,
            template_sha256: digest::sha256_hex(b"controlled practice declaration"),
            requirement_id: CanonicalId::new("assessment-requirement").unwrap(),
        },
        practice_text: "Check only the selected source and the declared rubric.".into(),
    }
}

fn binary_descriptor() -> TaskAssessmentDescriptor {
    TaskAssessmentDescriptor::Binary {
        proposition: "The selected source contains the named declaration".into(),
    }
}

fn assessment_reply(packet: &serde_json::Value, value: bool) -> String {
    assessment_content(
        packet,
        serde_json::json!({ "outcome": "answered", "value": { "kind": "binary", "value": value } }),
        Some("source"),
    )
}

fn assessment_content(
    packet: &serde_json::Value,
    decision: serde_json::Value,
    evidence_input: Option<&str>,
) -> String {
    let references = evidence_input
        .map(|input| {
            let sha256 = match input {
                "source" => packet["source"]["content_sha256"].as_str().unwrap(),
                "practice" => packet["practice_sha256"].as_str().unwrap(),
                _ => panic!("test fixture uses only declared packet inputs"),
            };
            serde_json::json!([{
                "input": input,
                "input_sha256": sha256,
                "start_byte": 0,
                "end_byte": 1
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

fn receive_assessment(listener: &TcpListener) -> (std::net::TcpStream, serde_json::Value, String) {
    let (socket, body) = read_wire(listener);
    let wire: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let model = wire["model"].as_str().unwrap().to_owned();
    let packet: serde_json::Value =
        serde_json::from_str(wire["messages"][1]["content"].as_str().unwrap()).unwrap();
    (socket, packet, model)
}

fn run_assessment(
    fixture: &SourceWorkerFixture,
    listener: &TcpListener,
    request: &TaskAssessmentRequest,
    decision: serde_json::Value,
    evidence_input: Option<&str>,
) -> (rrd_contract::TaskAssessmentOutput, serde_json::Value) {
    std::thread::scope(|scope| {
        let server = scope.spawn(|| {
            let (mut socket, packet, model) = receive_assessment(listener);
            let body = assessment_content(&packet, decision, evidence_input);
            let response = reply(&mut socket, &model, &body);
            assert!(!response.is_empty());
            packet
        });
        let output = fixture
            .engine
            .execute_task_assessment_with_clock(
                &fixture.lease.session_id,
                &fixture.lease.token,
                request,
                request.attempt_id.as_str(),
                request.attempt_id.as_str(),
                &crate::engine::clock::FixedClock::at(fixture.now + 1),
            )
            .unwrap();
        (output, server.join().unwrap())
    })
}

mod dispatch_guards;
mod repetition;
mod retention;
