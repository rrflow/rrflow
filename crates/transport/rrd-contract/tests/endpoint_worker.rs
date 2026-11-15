//! Explicit configuration perturbations, never provider qualification records.
#[path = "support/observed_configuration.rs"]
mod observed_configuration;
use rrd_contract::*;
use serde::Serialize;
use serde_json::Value;
use std::{collections::BTreeMap, path::PathBuf};

fn fixture(name: &str) -> Vec<u8> {
    observed_configuration::read(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/endpoint-selection-observed/available"),
        name,
    )
}

fn proposed() -> (EndpointConfiguration, EndpointWorkerPolicy) {
    let mut configuration: EndpointConfiguration =
        serde_json::from_slice(&fixture("configuration.json")).unwrap();
    let allowance: EndpointSelectionAllowance =
        serde_json::from_slice(&fixture("allowance.json")).unwrap();
    let request: Value = serde_json::from_slice(&fixture("choice-request.json")).unwrap();
    let profile = configuration.profile.as_mut().unwrap();
    let controller = profile
        .bindings
        .iter()
        .find(|binding| binding.service_id.as_str() == request["model"].as_str().unwrap())
        .unwrap();
    let candidate = &allowance.candidates[0];
    let EndpointEffortSupport::Observed {
        evidence_sha256,
        allow_default,
        values,
    } = &candidate.effort
    else {
        panic!("fixture needs observed effort")
    };
    let worker = EndpointWorkerPolicy {
        contract_version: ENDPOINT_WORKER_VERSION,
        allow_network: true,
        allow_context_export: true,
        allow_source_export: false,
        allow_unverified_identity: true,
        maximum_concurrent_calls: 1,
        bindings: vec![EndpointWorkerBinding {
            binding_id: candidate.binding_id.clone(),
            adapter: EndpointWorkerAdapter::default(),
            // Digest-shaped protocol probe, not evidence of worker compatibility.
            protocol_evidence_sha256: evidence_sha256.clone(),
            system_prompt: "Return a draft; do not claim project effects.".into(),
            generation: EndpointGenerationSettings {
                maximum_output_tokens: 256,
                token_parameter: EndpointTokenParameter::MaxCompletionTokens,
                temperature_milli: None,
                response_format: EndpointResponseFormat::Text,
            },
            maximum_response_bytes: 16_384,
            effort_values: values
                .iter()
                .map(|value| (value.clone(), value.clone()))
                .collect::<BTreeMap<_, _>>(),
            allow_default_effort: *allow_default,
        }],
    };
    profile.selection_policy = Some(EndpointSelectionPolicy {
        contract_version: allowance.contract_version,
        purpose: EndpointSelectionPurpose::ExperimentalProposal,
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
    });
    profile.profile_sha256 = endpoint_profile_sha256(profile).unwrap();
    configuration.validate().unwrap();
    (configuration, worker)
}

#[test]
fn worker_absence_preserves_identity_and_present_settings_require_a_new_approval() {
    let original: EndpointConfiguration =
        serde_json::from_slice(&fixture("configuration.json")).unwrap();
    let receipt: Value = serde_json::from_slice(&fixture("receipt.json")).unwrap();
    assert_eq!(original.sha256().unwrap(), receipt["configurationSha256"]);
    assert!(original.profile.as_ref().unwrap().worker_policy.is_none());
    assert!(serde_json::to_value(original).unwrap()["profile"]
        .get("worker_policy")
        .is_none());
    let (mut configuration, worker) = proposed();
    let before = configuration.sha256().unwrap();
    let profile = configuration.profile.as_mut().unwrap();
    profile.worker_policy = Some(worker);
    assert!(profile.validate().is_err());
    profile.profile_sha256 = endpoint_profile_sha256(profile).unwrap();
    configuration.validate().unwrap();
    assert_ne!(configuration.sha256().unwrap(), before);
    let profile = configuration.profile.as_mut().unwrap();
    profile.worker_policy.as_mut().unwrap().bindings[0]
        .generation
        .maximum_output_tokens += 1;
    assert!(profile.validate().is_err());
}

#[test]
fn worker_binding_settings_and_effort_cannot_invent_selector_support() {
    let (configuration, policy) = proposed();
    let profile = configuration.profile.unwrap();
    policy
        .validate_for(&profile.bindings, profile.selection_policy.as_ref())
        .unwrap();
    assert!(policy.validate_for(&profile.bindings, None).is_err());
    assert!(policy.bindings[0]
        .wire_effort(Some("unobserved-effort"))
        .is_err());
    let mut changed = policy.clone();
    changed.bindings[0]
        .effort_values
        .insert("unobserved-effort".into(), "medium".into());
    assert!(changed
        .validate_for(&profile.bindings, profile.selection_policy.as_ref())
        .is_err());
    for field in ["contract_version", "maximum_concurrent_calls"] {
        let mut value = serde_json::to_value(&policy).unwrap();
        value[field] = 0.into();
        let changed: EndpointWorkerPolicy = serde_json::from_value(value).unwrap();
        assert!(changed
            .validate_for(&profile.bindings, profile.selection_policy.as_ref())
            .is_err());
    }
    let mut changed = policy.clone();
    changed.bindings.push(changed.bindings[0].clone());
    assert!(changed
        .validate_for(&profile.bindings, profile.selection_policy.as_ref())
        .is_err());
    let mut changed = policy.clone();
    changed.bindings[0].maximum_response_bytes = 0;
    assert!(changed
        .validate_for(&profile.bindings, profile.selection_policy.as_ref())
        .is_err());
    let mut value = serde_json::to_value(policy).unwrap();
    value["authorize_project_write"] = true.into();
    assert!(serde_json::from_value::<EndpointWorkerPolicy>(value).is_err());
}

#[test]
fn configured_effort_mapping_is_exact_and_default_is_never_inferred() {
    let (_, policy) = proposed();
    let mut binding = policy.bindings[0].clone();
    binding.allow_default_effort = false;
    binding.effort_values = BTreeMap::from([("selected".into(), "wire-value".into())]);
    assert_eq!(
        binding.wire_effort(Some("selected")).unwrap(),
        Some("wire-value")
    );
    assert!(binding.wire_effort(None).is_err());
    assert!(binding.wire_effort(Some("wire-value")).is_err());
    binding.allow_default_effort = true;
    assert_eq!(binding.wire_effort(None).unwrap(), None);
}

#[test]
fn worker_operation_has_explicit_authority_and_closed_input_identity() {
    let catalogue = endpoint_catalogue();
    let operation = catalogue
        .endpoints
        .iter()
        .find(|endpoint| endpoint.operation.as_str() == "task-worker-execute")
        .unwrap();
    assert_eq!(operation.path, "/v1/tasks/worker/execute");
    assert!(operation.mutation);
    assert_eq!(
        operation.action.fixed_action(),
        Some(SecurityAction::WorkerExecute)
    );
    let request = TaskWorkerRequest {
        scope: "instance:worker-test".into(),
        task_id: CanonicalId::new("worker-test-task").unwrap(),
        attempt_id: CorrelationId::new("worker-test-attempt").unwrap(),
        selection_attempt_id: CorrelationId::new("worker-test-selection").unwrap(),
        expected_selection_sha256: "1".repeat(64),
        expected_task_sha256: "2".repeat(64),
        expected_configuration_sha256: "3".repeat(64),
        max_storage_keys: 10_000,
        context: None,
        source: None,
    };
    request.validate().unwrap();
    #[derive(Serialize)]
    struct LegacyTaskWorkerRequest {
        scope: String,
        task_id: CanonicalId,
        attempt_id: CorrelationId,
        selection_attempt_id: CorrelationId,
        expected_selection_sha256: String,
        expected_task_sha256: String,
        expected_configuration_sha256: String,
        max_storage_keys: u64,
        context: Option<AssembleContext>,
    }
    let legacy_serialization = serde_json::to_value(LegacyTaskWorkerRequest {
        scope: request.scope.clone(),
        task_id: request.task_id.clone(),
        attempt_id: request.attempt_id.clone(),
        selection_attempt_id: request.selection_attempt_id.clone(),
        expected_selection_sha256: request.expected_selection_sha256.clone(),
        expected_task_sha256: request.expected_task_sha256.clone(),
        expected_configuration_sha256: request.expected_configuration_sha256.clone(),
        max_storage_keys: request.max_storage_keys,
        context: request.context.clone(),
    })
    .unwrap();
    assert_eq!(
        serde_json::to_value(&request).unwrap(),
        legacy_serialization
    );
    let mut legacy = serde_json::to_value(&request).unwrap();
    legacy.as_object_mut().unwrap().remove("source");
    assert_eq!(
        serde_json::from_value::<TaskWorkerRequest>(legacy)
            .unwrap()
            .source,
        None
    );
    let mut value = serde_json::to_value(&request).unwrap();
    value["endpoint"] = "http://127.0.0.1:1".into();
    assert!(serde_json::from_value::<TaskWorkerRequest>(value).is_err());
    let mut same_attempt = request.clone();
    same_attempt.selection_attempt_id = same_attempt.attempt_id.clone();
    assert!(same_attempt.validate().is_err());
    let mut foreign = request;
    foreign.context = Some(AssembleContext {
        scope: "instance:foreign".into(),
        query: "test".into(),
        valid_at: 1,
        seeds: Vec::new(),
        max_graph_depth: 0,
        max_items: 1,
        max_output_bytes: 1024,
        max_storage_keys: 100,
    });
    assert!(foreign.validate().is_err());
}

#[test]
fn selected_worker_source_is_versioned_bounded_and_export_opt_in_defaults_off() {
    let source = TaskWorkerSourceRequest {
        contract_version: TASK_WORKER_SOURCE_VERSION,
        entry_identity: "5".repeat(64),
        expected_content_sha256: "6".repeat(64),
        maximum_input_bytes: 1024,
    };
    source.validate().unwrap();
    for removed_field in ["expected_snapshot_sha256", "expected_policy_sha256"] {
        let mut legacy = serde_json::to_value(&source).unwrap();
        legacy[removed_field] = serde_json::Value::String("4".repeat(64));
        assert!(serde_json::from_value::<TaskWorkerSourceRequest>(legacy).is_err());
    }
    let mut invalid = source.clone();
    invalid.maximum_input_bytes = 0;
    assert!(invalid.validate().is_err());
    invalid = source.clone();
    invalid.contract_version += 1;
    assert!(invalid.validate().is_err());

    let mut task = TaskWorkerRequest {
        scope: "instance:worker-test".into(),
        task_id: CanonicalId::new("worker-test-task").unwrap(),
        attempt_id: CorrelationId::new("worker-test-attempt").unwrap(),
        selection_attempt_id: CorrelationId::new("worker-test-selection").unwrap(),
        expected_selection_sha256: "1".repeat(64),
        expected_task_sha256: "2".repeat(64),
        expected_configuration_sha256: "3".repeat(64),
        max_storage_keys: 10_000,
        context: None,
        source: Some(source),
    };
    task.validate().unwrap();
    task.source.as_mut().unwrap().maximum_input_bytes = MAX_ROUTE_REQUEST_BYTES + 1;
    assert!(task.validate().is_err());

    let (mut configuration, policy) = proposed();
    assert!(!policy.allow_source_export);
    let profile = configuration.profile.as_mut().unwrap();
    profile.worker_policy = Some(policy.clone());
    profile.profile_sha256 = endpoint_profile_sha256(profile).unwrap();
    let denied = configuration.sha256().unwrap();
    let profile = configuration.profile.as_mut().unwrap();
    profile.worker_policy.as_mut().unwrap().allow_source_export = true;
    profile.profile_sha256 = endpoint_profile_sha256(profile).unwrap();
    assert_ne!(configuration.sha256().unwrap(), denied);
    let mut legacy = serde_json::to_value(policy).unwrap();
    legacy
        .as_object_mut()
        .unwrap()
        .remove("allow_source_export");
    let legacy_policy: EndpointWorkerPolicy = serde_json::from_value(legacy.clone()).unwrap();
    assert!(!legacy_policy.allow_source_export);
    assert_eq!(serde_json::to_value(legacy_policy).unwrap(), legacy);
}
