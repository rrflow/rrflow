//! Pure projection/validation of a real task opened in a fresh engine and retained
//! provider bytes. No provider executes here, no constructed proposal is persisted,
//! and these derived tuples are neither observed engine receipts nor learning data.
use super::*;
use rrd_contract::{
    endpoint_profile_sha256, EndpointConfiguration, EndpointSelectionAllowance,
    EndpointSelectionChoice, EndpointSelectionPacket, EndpointSelectionPolicy,
    EndpointSelectionProposal, EndpointSelectionPurpose, EndpointSelectionRequest,
    EndpointSelectionTask, EndpointTokenUsage, ReadTask, ENDPOINT_SELECTION_VERSION,
    MAX_TASK_STORAGE_KEYS,
};

fn captured(file: &str) -> Vec<u8> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../transport/rrd-contract/tests/fixtures/endpoint-selection-observed/available");
    let bytes = std::fs::read(root.join(file)).unwrap();
    let hashes: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("files.json")).unwrap()).unwrap();
    // The recorded bytes must be the recorded observation; what the test then
    // uses is that observation projected to the current names.
    assert_eq!(digest::sha256_hex(&bytes), hashes[file]);
    crate::engine::tests::observed_configuration::read(&root, file)
}

pub(super) fn configured() -> EndpointConfiguration {
    let mut configuration: EndpointConfiguration =
        serde_json::from_slice(&captured("configuration.json")).unwrap();
    let allowance: EndpointSelectionAllowance =
        serde_json::from_slice(&captured("allowance.json")).unwrap();
    let request: serde_json::Value =
        serde_json::from_slice(&captured("choice-request.json")).unwrap();
    let profile = configuration.profile.as_mut().unwrap();
    let controller = profile
        .bindings
        .iter()
        .find(|binding| binding.service_id.as_str() == request["model"].as_str().unwrap())
        .unwrap();
    profile.selection_policy = Some(EndpointSelectionPolicy {
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
    });
    profile.profile_sha256 = endpoint_profile_sha256(profile).unwrap();
    configuration.validate().unwrap();
    configuration
}

#[test]
fn proposal_packet_binds_actual_task_and_rejects_changed_projection() {
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
            "proposal-task",
            "proposal-task",
        )
        .unwrap();
    let configuration = configured();
    let policy = configuration
        .profile
        .as_ref()
        .unwrap()
        .selection_policy
        .as_ref()
        .unwrap();
    let packet = EndpointSelectionPacket {
        contract_version: ENDPOINT_SELECTION_VERSION,
        purpose: EndpointSelectionPurpose::ExperimentalProposal,
        task: EndpointSelectionTask::from_task(&task.task).unwrap(),
        read_manifest_sha256: task.read.runtime_manifest_sha256.clone(),
        read_cursor: task.read.runtime_cursor,
        controller_binding_id: policy.controller_binding_id.clone(),
        allowance: policy.allowance(&configuration).unwrap(),
        lfg: None,
        routing: None,
    };
    packet.validate_for(&configuration).unwrap();
    packet.validate_task(&task.task).unwrap();
    let encoded = serde_json::to_value(&packet).unwrap();
    assert!(encoded["task"].get("idempotency_key").is_none());
    assert!(encoded.get("credential").is_none());
    let mut changed = packet.clone();
    changed.task.goal.push('\n');
    assert!(changed.validate_task(&task.task).is_err());
    assert_ne!(
        changed.sha256(&configuration).unwrap(),
        packet.sha256(&configuration).unwrap()
    );
    changed = packet.clone();
    changed.read_cursor = task.task.known_at_cursor - 1;
    assert!(changed.validate_for(&configuration).is_err());
    changed = packet.clone();
    changed.allowance.candidates.clear();
    assert!(changed.validate_for(&configuration).is_err());
    engine
        .close_task(
            &lease.session_id,
            &lease.token,
            &rrd_contract::CloseTask {
                scope: SCOPE.into(),
                task_id: task_id.clone(),
                status: rrd_contract::TaskStatus::Abandoned,
                outcome: None,
            },
            60,
            "close-projection-test",
            "close-projection-test",
        )
        .unwrap();
    let closed = engine
        .read_task(
            &lease.session_id,
            &lease.token,
            &ReadTask {
                scope: SCOPE.into(),
                task_id,
                max_storage_keys: MAX_TASK_STORAGE_KEYS,
            },
            70,
            "closed-proposal-task",
            "closed-proposal-task",
        )
        .unwrap();
    assert!(packet.validate_task(&closed.task).is_err());
    assert!(EndpointSelectionTask::from_task(&closed.task).is_err());
}

pub(super) fn tuple_fixture() -> (
    tempfile::TempDir,
    RrdEngine,
    SessionLease,
    EndpointConfiguration,
    EndpointSelectionRequest,
    EndpointSelectionPacket,
    EndpointSelectionProposal,
) {
    let (root, engine, lease, task_id, _) = actual_request_fixture();
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
            "proposal-tuple",
            "proposal-tuple",
        )
        .unwrap();
    let configuration = configured();
    let policy = configuration
        .profile
        .as_ref()
        .unwrap()
        .selection_policy
        .as_ref()
        .unwrap();
    let packet = EndpointSelectionPacket {
        contract_version: ENDPOINT_SELECTION_VERSION,
        purpose: EndpointSelectionPurpose::ExperimentalProposal,
        task: EndpointSelectionTask::from_task(&task.task).unwrap(),
        read_manifest_sha256: task.read.runtime_manifest_sha256,
        read_cursor: task.read.runtime_cursor,
        controller_binding_id: policy.controller_binding_id.clone(),
        allowance: policy.allowance(&configuration).unwrap(),
        lfg: None,
        routing: None,
    };
    let request = EndpointSelectionRequest {
        scope: SCOPE.into(),
        task_id: task_id.clone(),
        attempt_id: id("proposal-tuple-test"),
        max_storage_keys: MAX_TASK_STORAGE_KEYS,
        expected_task_sha256: packet.task.task_sha256.clone(),
        expected_configuration_sha256: configuration.sha256().unwrap(),
        flight: None,
        routing: None,
    };
    let provider: serde_json::Value =
        serde_json::from_slice(&captured("choice-response.json")).unwrap();
    // This in-memory tuple exercises binding only. It does not assert that the
    // historical provider response was caused by this new engine packet.
    let proposal = EndpointSelectionProposal {
        contract_version: ENDPOINT_SELECTION_VERSION,
        purpose: packet.purpose,
        scope: SCOPE.into(),
        task_id,
        attempt_id: request.attempt_id.clone(),
        task_sha256: request.expected_task_sha256.clone(),
        packet_sha256: packet.sha256(&configuration).unwrap(),
        configuration_sha256: configuration.sha256().unwrap(),
        allowance_sha256: packet.allowance.sha256(&configuration).unwrap(),
        controller_binding_id: policy.controller_binding_id.clone(),
        service_id: CanonicalId::new(provider["model"].as_str().unwrap()).unwrap(),
        request_sha256: digest::sha256_hex(&captured("choice-request.json")),
        response_sha256: digest::sha256_hex(&captured("choice-response.json")),
        content_sha256: digest::sha256_hex(&captured("choice-content.json")),
        flight: None,
        routing: None,
        choice: EndpointSelectionChoice::decode(
            &captured("choice-content.json"),
            &packet.allowance,
            &configuration,
        )
        .unwrap(),
        usage: Some(EndpointTokenUsage {
            prompt_tokens: provider["usage"]["prompt_tokens"].as_u64(),
            completion_tokens: provider["usage"]["completion_tokens"].as_u64(),
            total_tokens: provider["usage"]["total_tokens"].as_u64(),
        }),
        elapsed_ms: None,
        http_status: None,
        lfg: None,
    };
    proposal
        .validate_for(&request, &packet, &configuration)
        .unwrap();
    (
        root,
        engine,
        lease,
        configuration,
        request,
        packet,
        proposal,
    )
}

#[test]
fn proposal_tuple_rejects_cross_attempt_config_and_usage_substitution() {
    let (_root, _engine, lease, configuration, request, packet, proposal) = tuple_fixture();
    let roundtrip: EndpointSelectionProposal =
        serde_json::from_slice(&serde_json::to_vec(&proposal).unwrap()).unwrap();
    assert_eq!(roundtrip, proposal);
    let mut changed = proposal.clone();
    changed.attempt_id = lease.session_id.clone();
    assert!(changed
        .validate_for(&request, &packet, &configuration)
        .is_err());
    changed = proposal.clone();
    changed.configuration_sha256 = changed.response_sha256.clone();
    assert!(changed
        .validate_for(&request, &packet, &configuration)
        .is_err());
    changed = proposal.clone();
    changed.packet_sha256 = changed.response_sha256.clone();
    assert!(changed
        .validate_for(&request, &packet, &configuration)
        .is_err());
    changed = proposal.clone();
    changed.usage.as_mut().unwrap().total_tokens = Some(0);
    assert!(changed.validate().is_err());
    changed = proposal.clone();
    changed.http_status = Some(503);
    assert!(changed.validate().is_err());
    let mut unknown = serde_json::to_value(&request).unwrap();
    unknown["endpoint"] = serde_json::Value::String("https://not-authorized.example".into());
    let decoded = serde_json::from_value::<EndpointSelectionRequest>(unknown).unwrap();
    assert_eq!(decoded, request);
    proposal
        .validate_for(&decoded, &packet, &configuration)
        .unwrap();
    let mut exhausted = request;
    exhausted.max_storage_keys = 0;
    assert!(exhausted.validate().is_err());
}
