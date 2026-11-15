use super::*;
use rrd_contract::{
    ApplyEndpointConfiguration, BindingKind, BindingLimits, CredentialReference, DeclaredFootprint,
    EndpointBinding, EndpointConfiguration, EndpointConfigurationPlan, EndpointLocator,
    EndpointProfileInput, InstalledEngramIdentity, PreviewEndpointConfiguration, SessionLease,
    WorkloadClass,
};

fn bind(engine: &mut RrdEngine, project: &str) {
    engine.installed_engram = Some(InstalledEngramIdentity {
        project_id: CanonicalId::new(project).unwrap(),
        engram_id: CanonicalId::new("endpoint-engram").unwrap(),
        instance_id: instance(),
    });
}
fn profile(name: &str) -> EndpointProfileInput {
    EndpointProfileInput {
        contract_version: 1,
        name: CanonicalId::new(name).unwrap(),
        selection_policy: None,
        worker_policy: None,
        observation_policy: None,
        assessment_policy: None,
        bindings: vec![EndpointBinding {
            id: CanonicalId::new("knowledge-source").unwrap(),
            class: WorkloadClass::VectorStore,
            kind: BindingKind::Api,
            // An intentionally unreachable address proves configuration needs no probe.
            endpoint: EndpointLocator::Url {
                url: "https://not-a-provider.invalid/".into(),
            },
            service_id: CanonicalId::new("unqualified-vector-service").unwrap(),
            manifest_sha256: None,
            credential: Some(CredentialReference {
                provider: CanonicalId::new("unconfigured-secret-provider").unwrap(),
                name: "knowledge-read-key".into(),
            }),
            placement: None,
            footprint: DeclaredFootprint {
                memory_bytes: 1,
                gpu: rrd_contract::GpuRequirement::None,
            },
            limits: BindingLimits {
                max_concurrency: 1,
                timeout_ms: 1_000,
                max_input_bytes: 4096,
            },
        }],
    }
}
fn session(engine: &RrdEngine, at: u64) -> SessionLease {
    engine
        .create_session(
            &session_request(5_000, 2),
            &id(&format!("endpoint-session-{at}")),
            at,
            "session-request",
            "session-operation",
        )
        .unwrap()
}
fn preview(
    engine: &RrdEngine,
    lease: &SessionLease,
    revision: u64,
    name: Option<&str>,
) -> EndpointConfigurationPlan {
    engine
        .preview_endpoint_configuration(
            &lease.session_id,
            &lease.token,
            &PreviewEndpointConfiguration {
                expected_revision: revision,
                profile: name.map(profile),
            },
            1_001,
            "preview-request",
            "preview-operation",
        )
        .unwrap()
}
fn apply(
    engine: &RrdEngine,
    lease: &SessionLease,
    plan: &EndpointConfigurationPlan,
) -> crate::Result<EndpointConfiguration> {
    engine.apply_endpoint_configuration(
        &lease.session_id,
        &lease.token,
        &ApplyEndpointConfiguration {
            plan: plan.clone(),
            approved_plan_sha256: plan.plan_sha256.clone(),
        },
        1_002,
        "apply-request",
        "apply-operation",
    )
}
fn read(engine: &RrdEngine, lease: &SessionLease) -> EndpointConfiguration {
    engine
        .read_endpoint_configuration(
            &lease.session_id,
            &lease.token,
            1_003,
            "read-request",
            "read-operation",
        )
        .unwrap()
}

#[test]
fn actual_selector_settings_follow_existing_configuration_approval_and_reopen() {
    configuration_policy_reopens(false, false);
}

#[test]
fn worker_settings_follow_existing_configuration_approval_and_reopen() {
    configuration_policy_reopens(true, false);
}

#[test]
fn worker_and_observation_settings_share_configuration_approval_and_reopen() {
    configuration_policy_reopens(true, true);
}

fn configuration_policy_reopens(include_worker: bool, include_observation: bool) {
    // A configuration proposal derived from actual captured settings; this
    // fresh test engram does not claim to be the captured production identity.
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../transport/rrd-contract/tests/fixtures/endpoint-selection-observed/available");
    let configuration: EndpointConfiguration = serde_json::from_slice(
        &super::observed_configuration::read(&fixture, "configuration.json"),
    )
    .unwrap();
    let captured = configuration.profile.unwrap();
    let allowance: rrd_contract::EndpointSelectionAllowance = serde_json::from_slice(
        &super::observed_configuration::read(&fixture, "allowance.json"),
    )
    .unwrap();
    let request: serde_json::Value = serde_json::from_slice(&super::observed_configuration::read(
        &fixture,
        "choice-request.json",
    ))
    .unwrap();
    let controller = captured
        .bindings
        .iter()
        .find(|b| b.service_id.as_str() == request["model"].as_str().unwrap())
        .unwrap();
    let policy = rrd_contract::EndpointSelectionPolicy {
        contract_version: allowance.contract_version,
        purpose: rrd_contract::EndpointSelectionPurpose::ExperimentalProposal,
        allow_unverified_identity: true,
        allow_network: true,
        maximum_concurrent_proposals: controller.limits.max_concurrency,
        controller_binding_id: controller.id.clone(),
        workload_class: allowance.workload_class,
        limits: allowance.limits,
        candidates: allowance.candidates,
        system_prompt: request["messages"][0]["content"].as_str().unwrap().into(),
        maximum_output_tokens: request["max_tokens"].as_u64().unwrap().try_into().unwrap(),
        temperature_milli: (request["temperature"].as_f64().unwrap() * 1000.0) as u16,
        lfg: None,
    };
    let worker_policy = include_worker.then(|| {
        let candidate = &policy.candidates[0];
        let rrd_contract::EndpointEffortSupport::Observed {
            evidence_sha256,
            allow_default,
            values,
        } = &candidate.effort
        else {
            panic!("fixture requires observed support");
        };
        rrd_contract::EndpointWorkerPolicy {
            contract_version: rrd_contract::ENDPOINT_WORKER_VERSION,
            allow_network: true,
            allow_context_export: true,
            allow_source_export: false,
            allow_unverified_identity: true,
            maximum_concurrent_calls: 1,
            bindings: vec![rrd_contract::EndpointWorkerBinding {
                binding_id: candidate.binding_id.clone(),
                // Protocol persistence probe, not a new capability observation.
                protocol_evidence_sha256: evidence_sha256.clone(),
                adapter: Default::default(),
                system_prompt: "Return a bounded draft.".into(),
                generation: rrd_contract::EndpointGenerationSettings {
                    maximum_output_tokens: 256,
                    token_parameter: rrd_contract::EndpointTokenParameter::MaxCompletionTokens,
                    temperature_milli: None,
                    response_format: rrd_contract::EndpointResponseFormat::Text,
                },
                maximum_response_bytes: 16_384,
                effort_values: values
                    .iter()
                    .map(|value| (value.clone(), value.clone()))
                    .collect(),
                allow_default_effort: *allow_default,
            }],
        }
    });
    // Controlled observation binding, never an actual sink or learning record.
    let mut bindings = captured.bindings;
    let observation_policy = include_observation.then(|| {
        let mut sink = bindings[0].clone();
        sink.id = CanonicalId::new("test-observation-sink").unwrap();
        sink.class = rrd_contract::WorkloadClass::BehaviorUpdates;
        sink.kind = rrd_contract::BindingKind::Api;
        sink.placement = None;
        sink.endpoint = rrd_contract::EndpointLocator::Url {
            url: "https://sink.invalid".into(),
        };
        sink.credential = Some(rrd_contract::CredentialReference {
            provider: CanonicalId::new("environment").unwrap(),
            name: "TEST_OBSERVATION_KEY".into(),
        });
        sink.limits.max_input_bytes = 16384;
        let binding_id = sink.id.clone();
        bindings.push(sink);
        rrd_contract::ObservationExportPolicy {
            contract_version: 1,
            binding_id,
            expected_devspace: "test-devspace".into(),
            protocol_evidence_sha256: digest::sha256_hex(b"controlled test protocol"),
            allow_network: true,
            allow_metadata_export: true,
            allowed_scopes: vec!["default".into()],
            max_retained_deliveries: 8,
            retention_ms: 60000,
            retire_expired_uncertain: false,
            max_attempts: 3,
            retry_delay_ms: 1000,
            max_response_bytes: 4096,
        }
    });
    let input = EndpointProfileInput {
        contract_version: captured.contract_version,
        name: captured.name,
        bindings,
        selection_policy: Some(policy.clone()),
        worker_policy: worker_policy.clone(),
        observation_policy: observation_policy.clone(),
        assessment_policy: None,
    };
    let (root, mut engine) = isolated_engine();
    bind(&mut engine, "selector-policy-persistence");
    let lease = session(&engine, 1_000);
    let before = read(&engine, &lease);
    let plan = engine
        .preview_endpoint_configuration(
            &lease.session_id,
            &lease.token,
            &PreviewEndpointConfiguration {
                expected_revision: 0,
                profile: Some(input),
            },
            1_001,
            "policy-preview",
            "policy-preview",
        )
        .unwrap();
    assert_eq!(read(&engine, &lease), before);
    let mut tampered = plan.clone();
    tampered
        .profile
        .as_mut()
        .unwrap()
        .selection_policy
        .as_mut()
        .unwrap()
        .maximum_output_tokens += 1;
    assert!(apply(&engine, &lease, &tampered).is_err());
    assert_eq!(read(&engine, &lease), before);
    if include_worker {
        let mut tampered = plan.clone();
        tampered
            .profile
            .as_mut()
            .unwrap()
            .worker_policy
            .as_mut()
            .unwrap()
            .bindings[0]
            .generation
            .maximum_output_tokens += 1;
        assert!(apply(&engine, &lease, &tampered).is_err());
        assert_eq!(read(&engine, &lease), before);
    }
    if include_observation {
        let mut tampered = plan.clone();
        tampered
            .profile
            .as_mut()
            .unwrap()
            .observation_policy
            .as_mut()
            .unwrap()
            .expected_devspace = "changed-space".into();
        assert!(apply(&engine, &lease, &tampered).is_err());
        assert_eq!(read(&engine, &lease), before);
    }
    let installed = apply(&engine, &lease, &plan).unwrap();
    assert_eq!(
        installed.profile.as_ref().unwrap().observation_policy,
        observation_policy
    );
    assert_eq!(
        installed.profile.as_ref().unwrap().worker_policy,
        worker_policy
    );
    assert_eq!(
        installed
            .profile
            .as_ref()
            .unwrap()
            .selection_policy
            .as_ref(),
        Some(&policy)
    );
    let projected = policy.allowance(&installed).unwrap();
    drop(engine);
    let mut engine = RrdEngine::open(root.path(), instance(), TOKEN_KEY).unwrap();
    bind(&mut engine, "selector-policy-persistence");
    assert_eq!(read(&engine, &lease), installed);
    assert_eq!(apply(&engine, &lease, &plan).unwrap(), installed);
    let revoked = apply(&engine, &lease, &preview(&engine, &lease, 1, None)).unwrap();
    assert!(projected.validate_for(&revoked).is_err());
    assert!(policy.allowance(&revoked).is_err());
    assert!(apply(&engine, &lease, &plan).is_err());
}

#[test]
fn endpoint_configuration_flow_rejects_stale_replay_and_persists_revocation() {
    let (root, persistent) = isolated_engine();
    for (mut engine, reopen) in [
        (RrdEngine::rrflow_mx(instance(), TOKEN_KEY), false),
        (persistent, true),
    ] {
        bind(&mut engine, "endpoint-project");
        let lease = session(&engine, 1_000);
        let initial = read(&engine, &lease);
        let first = preview(&engine, &lease, 0, Some("first"));
        let competing = preview(&engine, &lease, 0, Some("competing"));
        assert_eq!(
            read(&engine, &lease),
            initial,
            "preview changed canonical profile"
        );
        let configured = apply(&engine, &lease, &first).unwrap();
        assert_eq!(configured.revision, 1);
        assert_eq!(
            apply(&engine, &lease, &first).unwrap(),
            configured,
            "lost ACK retry changed the head"
        );
        assert!(matches!(
            apply(&engine, &lease, &competing),
            Err(ServiceError::StorageConflict(_))
        ));
        if reopen {
            drop(engine);
            engine = RrdEngine::open(root.path(), instance(), TOKEN_KEY).unwrap();
            bind(&mut engine, "endpoint-project");
            assert_eq!(read(&engine, &lease), configured);
            assert_eq!(apply(&engine, &lease, &first).unwrap(), configured);
        }
        let revoked = apply(&engine, &lease, &preview(&engine, &lease, 1, None)).unwrap();
        assert_eq!(revoked.revision, 2);
        assert!(revoked.profile.is_none());
        assert!(
            matches!(
                apply(&engine, &lease, &first),
                Err(ServiceError::StorageConflict(_))
            ),
            "old plan resurrected revoked binding"
        );
        if reopen {
            drop(engine);
            engine = RrdEngine::open(root.path(), instance(), TOKEN_KEY).unwrap();
            bind(&mut engine, "endpoint-project");
            assert_eq!(read(&engine, &lease), revoked);
        }
        let replacement = apply(
            &engine,
            &lease,
            &preview(&engine, &lease, 2, Some("replacement")),
        )
        .unwrap();
        assert_eq!(replacement.revision, 3);
        assert_eq!(replacement.profile.unwrap().name.as_str(), "replacement");
    }
}

#[test]
fn endpoint_configuration_rejects_foreign_tampered_unapproved_and_expired_plans() {
    let mut engine = RrdEngine::rrflow_mx(instance(), TOKEN_KEY);
    bind(&mut engine, "endpoint-project");
    let lease = session(&engine, 1_000);
    let plan = preview(&engine, &lease, 0, Some("first"));
    let before = read(&engine, &lease);
    let mut foreign = plan.clone();
    foreign.identity.project_id = CanonicalId::new("different-project").unwrap();
    foreign.plan_sha256 = foreign.sha256().unwrap();
    assert!(matches!(
        apply(&engine, &lease, &foreign),
        Err(ServiceError::ProjectBindingMismatch)
    ));
    let mut tampered = plan.clone();
    tampered.profile.as_mut().unwrap().bindings[0]
        .limits
        .max_concurrency += 1;
    assert!(apply(&engine, &lease, &tampered).is_err());
    let request = ApplyEndpointConfiguration {
        plan: plan.clone(),
        approved_plan_sha256: "0".repeat(64),
    };
    assert!(engine
        .apply_endpoint_configuration(
            &lease.session_id,
            &lease.token,
            &request,
            1_002,
            "bad-approval",
            "bad-approval"
        )
        .is_err());
    assert!(engine
        .apply_endpoint_configuration(
            &lease.session_id,
            &id("wrong-token"),
            &ApplyEndpointConfiguration {
                plan: plan.clone(),
                approved_plan_sha256: plan.plan_sha256.clone()
            },
            1_002,
            "bad-token",
            "bad-token"
        )
        .is_err());
    assert_eq!(read(&engine, &lease), before);
    let later = session(&engine, plan.expires_at_unix_ms);
    assert!(matches!(
        engine.apply_endpoint_configuration(
            &later.session_id,
            &later.token,
            &ApplyEndpointConfiguration {
                approved_plan_sha256: plan.plan_sha256.clone(),
                plan: plan.clone()
            },
            plan.expires_at_unix_ms,
            "expired",
            "expired"
        ),
        Err(ServiceError::DeadlineExceeded)
    ));
    // Expiry failure leaves the state absent, including its control key.
    assert!(engine
        .storage
        .control()
        .get(&format!(
            "server/state/{}/endpoint-configuration/active",
            instance()
        ))
        .unwrap()
        .is_none());
}

#[test]
fn endpoint_configuration_concurrent_apply_has_one_winner() {
    let (_root, mut engine) = isolated_engine();
    bind(&mut engine, "endpoint-project");
    let lease = session(&engine, 1_000);
    let a = preview(&engine, &lease, 0, Some("first"));
    let b = preview(&engine, &lease, 0, Some("second"));
    let barrier = std::sync::Barrier::new(2);
    let results = std::thread::scope(|scope| {
        let left = scope.spawn(|| {
            barrier.wait();
            apply(&engine, &lease, &a)
        });
        let right = scope.spawn(|| {
            barrier.wait();
            apply(&engine, &lease, &b)
        });
        [left.join().unwrap(), right.join().unwrap()]
    });
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|r| matches!(r, Err(ServiceError::StorageConflict(_))))
            .count(),
        1
    );
    assert_eq!(read(&engine, &lease).revision, 1);
}

#[test]
fn endpoint_configuration_read_permission_does_not_allow_preview_or_apply() {
    use std::collections::{BTreeMap, BTreeSet};
    let mut engine = RrdEngine::rrflow_mx(instance(), TOKEN_KEY);
    bind(&mut engine, "endpoint-project");
    let bootstrap = session(&engine, 1_000);
    let plan = preview(&engine, &bootstrap, 0, Some("first"));
    let principal_id = CanonicalId::new("configuration-reader").unwrap();
    let resource = ResourcePath {
        segments: vec![ResourceId::new(ResourceKind::Instance, instance().as_str()).unwrap()],
    };
    let principal = Principal {
        id: principal_id.clone(),
        kind: PrincipalKind::Service,
        credential_sha256: digest::sha256_hex(b"reader-key"),
        credential_revision: 1,
        not_before_unix_ms: 1,
        expires_at_unix_ms: u64::MAX,
        disabled: false,
        role_ids: BTreeSet::new(),
        grants: [SecurityAction::SessionCreate, SecurityAction::EngramRead]
            .into_iter()
            .map(|action| ResourceGrant {
                action,
                resource_prefix: resource.clone(),
                data_policy: None,
            })
            .collect(),
    };
    SecurityRepository::new(&engine.storage, instance())
        .initialize(
            SecurityState {
                format_version: rrd_security::SECURITY_FORMAT,
                revision: 1,
                principals: BTreeMap::from([(principal_id.clone(), principal)]),
                roles: BTreeMap::new(),
                identity_bindings: BTreeMap::new(),
                jwt_issuers: BTreeMap::new(),
            },
            1_000,
            "test",
            "security-request",
            "security-operation",
        )
        .unwrap();
    let reader = engine
        .create_authenticated_session(
            &principal_id,
            b"reader-key",
            &session_request(5_000, 2),
            &id("reader-session"),
            1_000,
            "reader-request",
            "reader-operation",
        )
        .unwrap();
    assert_eq!(read(&engine, &reader).revision, 0);
    assert!(engine
        .preview_endpoint_configuration(
            &reader.session_id,
            &reader.token,
            &PreviewEndpointConfiguration {
                expected_revision: 0,
                profile: Some(profile("unauthorized"))
            },
            1_001,
            "denied-preview",
            "denied-preview"
        )
        .is_err());
    assert!(apply(&engine, &reader, &plan).is_err());
    assert_eq!(read(&engine, &reader).revision, 0);
}
