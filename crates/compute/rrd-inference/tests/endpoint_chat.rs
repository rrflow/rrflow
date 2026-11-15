#![cfg(feature = "endpoint-chat")]
//! Actual retained HTTP records; denial mutations are in-memory probes only.
#[path = "support/observed_configuration.rs"]
mod observed_configuration;

use rrd_contract::{endpoint_profile_sha256, EndpointConfiguration, EndpointSelectionAllowance};
use rrd_inference::{
    endpoint_chat::{EndpointChatRequest, PreparedEndpointChat},
    EmbeddingCancellation, EmbeddingExecutionControl, NetworkPolicy,
};
use serde_json::Value;
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

fn read(name: &str, file: &str) -> Vec<u8> {
    observed_configuration::read(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../transport/rrd-contract/tests/fixtures/endpoint-selection-observed")
            .join(name),
        file,
    )
}
fn inputs(
    name: &str,
) -> (
    EndpointConfiguration,
    EndpointSelectionAllowance,
    EndpointChatRequest,
) {
    (
        serde_json::from_slice(&read(name, "configuration.json")).unwrap(),
        serde_json::from_slice(&read(name, "allowance.json")).unwrap(),
        serde_json::from_slice(&read(name, "choice-request.json")).unwrap(),
    )
}
fn response(name: &str) -> Value {
    serde_json::from_slice(&read(name, "choice-response.json")).unwrap()
}
fn control() -> EmbeddingExecutionControl {
    EmbeddingExecutionControl::new(
        Instant::now() + Duration::from_secs(1),
        EmbeddingCancellation::default(),
    )
}

#[test]
fn actual_responses_decode_without_invented_transport_measurements() {
    for name in ["available", "withheld"] {
        let (configuration, allowance, request) = inputs(name);
        let binding = &configuration.profile.as_ref().unwrap().bindings[0].id;
        let prepared =
            PreparedEndpointChat::new(&configuration, binding, &allowance, &request).unwrap();
        let observed = prepared
            .decode_response(read(name, "choice-response.json"))
            .unwrap();
        let receipt: Value = serde_json::from_slice(&read(name, "receipt.json")).unwrap();
        assert_eq!(
            observed.response_sha256,
            receipt["choiceCall"]["responseSha256"]
        );
        assert_eq!(
            observed.content_sha256,
            receipt["choiceCall"]["contentSha256"]
        );
        assert_eq!(observed.service_id, receipt["choiceCall"]["model"]);
        assert!(observed.http_status.is_none());
        assert!(observed.elapsed_ms.is_none());
        assert_eq!(
            serde_json::to_value(&observed.usage).unwrap(),
            serde_json::json!({
            "prompt_tokens":receipt["choiceCall"]["usage"]["prompt_tokens"],
            "completion_tokens":receipt["choiceCall"]["usage"]["completion_tokens"],
            "total_tokens":receipt["choiceCall"]["usage"]["total_tokens"]})
        );
    }
}

#[test]
fn stale_config_and_foreign_request_never_prepare() {
    let (configuration, mut allowance, mut request) = inputs("available");
    let binding = &configuration.profile.as_ref().unwrap().bindings[0].id;
    request.model = binding.clone();
    assert!(PreparedEndpointChat::new(&configuration, binding, &allowance, &request).is_err());
    let (_, _, request) = inputs("available");
    allowance.configuration_sha256 = allowance.candidates[0].manifest_sha256.clone().unwrap();
    assert!(PreparedEndpointChat::new(&configuration, binding, &allowance, &request).is_err());
}

#[test]
fn network_denial_expiry_and_cancellation_precede_credentials_and_io() {
    let (configuration, allowance, request) = inputs("available");
    let binding = &configuration.profile.as_ref().unwrap().bindings[0].id;
    let prepared =
        PreparedEndpointChat::new(&configuration, binding, &allowance, &request).unwrap();
    let denied = prepared
        .execute(NetworkPolicy::Deny, &control(), |_, _| {
            panic!("credential resolution after denial")
        })
        .unwrap_err();
    assert!(!denied.dispatch_may_have_occurred());
    let cancelled = EmbeddingCancellation::default();
    cancelled.cancel();
    let cancelled =
        EmbeddingExecutionControl::new(Instant::now() + Duration::from_secs(1), cancelled);
    let expired = EmbeddingExecutionControl::new(Instant::now(), EmbeddingCancellation::default());
    for control in [cancelled, expired] {
        let failure = prepared
            .execute(NetworkPolicy::Allow, &control, |_, _| {
                panic!("resolution after stop")
            })
            .unwrap_err();
        assert!(!failure.dispatch_may_have_occurred());
    }
    let failure = prepared
        .execute(NetworkPolicy::Allow, &control(), |reference, _| {
            Err(reference.name.clone())
        })
        .unwrap_err();
    assert_eq!(failure.reason, "credential unavailable");
    assert!(!failure.dispatch_may_have_occurred());
    assert!(!serde_json::to_string(&failure).unwrap().contains(
        &configuration.profile.as_ref().unwrap().bindings[0]
            .credential
            .as_ref()
            .unwrap()
            .name
    ));
}

#[test]
fn unencrypted_nonloopback_and_oversized_requests_do_not_prepare() {
    let (mut configuration, mut allowance, request) = inputs("available");
    let profile = configuration.profile.as_mut().unwrap();
    if let rrd_contract::EndpointLocator::Url { url } = &mut profile.bindings[0].endpoint {
        *url = url.replace("127.0.0.1", "0.0.0.0");
    }
    profile.profile_sha256 = endpoint_profile_sha256(profile).unwrap();
    allowance.configuration_sha256 = configuration.sha256().unwrap();
    let binding = &configuration.profile.as_ref().unwrap().bindings[0].id;
    assert!(PreparedEndpointChat::new(&configuration, binding, &allowance, &request).is_err());
    let (mut configuration, mut allowance, request) = inputs("available");
    let profile = configuration.profile.as_mut().unwrap();
    profile.bindings[0].limits.max_input_bytes = 1;
    profile.profile_sha256 = endpoint_profile_sha256(profile).unwrap();
    allowance.configuration_sha256 = configuration.sha256().unwrap();
    let binding = &configuration.profile.as_ref().unwrap().bindings[0].id;
    assert!(PreparedEndpointChat::new(&configuration, binding, &allowance, &request).is_err());
}

#[test]
fn altered_model_truncation_unsupported_effects_and_bad_usage_fail() {
    let (configuration, allowance, request) = inputs("available");
    let binding = &configuration.profile.as_ref().unwrap().bindings[0].id;
    let prepared =
        PreparedEndpointChat::new(&configuration, binding, &allowance, &request).unwrap();
    let original = response("available");
    let mut changed = original.clone();
    changed["model"] = Value::String(binding.to_string());
    let mut truncated = original.clone();
    truncated["choices"][0]["finish_reason"] = Value::Null;
    let mut tools = original.clone();
    tools["choices"][0]["message"]["tool_calls"] =
        serde_json::json!([original["choices"][0]["message"]["content"]]);
    let mut usage = original.clone();
    let mut legacy = original.clone();
    legacy["choices"][0]["message"]["function_call"] =
        original["choices"][0]["message"]["content"].clone();
    let mut reasoning = original.clone();
    reasoning["choices"][0]["message"]["reasoning_content"] =
        original["choices"][0]["message"]["content"].clone();
    usage["usage"]["total_tokens"] = serde_json::json!(u64::MAX);
    let mut choice = original.clone();
    choice["choices"][0]["message"]["content"] = Value::String(String::new());
    for value in [changed, truncated, tools, legacy, reasoning, usage, choice] {
        let failure = prepared
            .decode_response(serde_json::to_vec(&value).unwrap())
            .unwrap_err();
        assert!(failure.dispatch_may_have_occurred());
    }
    let bytes = read("available", "choice-response.json");
    assert!(prepared
        .decode_response(bytes[..bytes.len() - 1].to_vec())
        .is_err());
}

#[test]
fn missing_usage_is_unknown_and_outer_response_is_bounded() {
    let (configuration, mut allowance, request) = inputs("available");
    let binding = &configuration.profile.as_ref().unwrap().bindings[0].id;
    let prepared =
        PreparedEndpointChat::new(&configuration, binding, &allowance, &request).unwrap();
    let mut output = response("available");
    output.as_object_mut().unwrap().remove("usage");
    assert!(prepared
        .decode_response(serde_json::to_vec(&output).unwrap())
        .unwrap()
        .usage
        .is_none());
    let bytes = read("available", "choice-response.json");
    allowance.limits.maximum_response_bytes = bytes.len() as u64 - 1;
    allowance.limits.maximum_effort_bytes = allowance.limits.maximum_response_bytes as u32;
    let prepared =
        PreparedEndpointChat::new(&configuration, binding, &allowance, &request).unwrap();
    assert_eq!(
        prepared.decode_response(bytes).unwrap_err().reason,
        "chat response exceeds byte limit"
    );
}

#[test]
fn rejected_selection_after_completed_generation_does_not_retain_unknown_execution() {
    let (configuration, allowance, request) = inputs("available");
    let binding = &configuration.profile.as_ref().unwrap().bindings[0].id;
    let prepared =
        PreparedEndpointChat::new(&configuration, binding, &allowance, &request).unwrap();
    let mut output = response("available");
    output["choices"][0]["message"]["content"] = serde_json::json!("{}");
    let error = prepared
        .decode_response(serde_json::to_vec(&output).unwrap())
        .unwrap_err();
    assert_eq!(error.reason, "invalid endpoint selection choice");
    assert!(error.dispatch_may_have_occurred());
    assert!(!error.execution_may_continue());
}

#[test]
fn native_live_calls_retain_actual_bytes_and_uncertain_failures() {
    let root =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/endpoint-chat-native");
    let hashes: std::collections::BTreeMap<String, String> =
        serde_json::from_slice(&std::fs::read(root.join("files.json")).unwrap()).unwrap();
    for (file, hash) in hashes {
        assert_eq!(
            rrd_core::digest::sha256_hex(&std::fs::read(root.join(file)).unwrap()),
            hash
        );
    }
    for name in [
        "available",
        "withheld",
        "deadline",
        "cancelled",
        "postdecode",
        "envelope",
    ] {
        let receipt: Value =
            serde_json::from_slice(&std::fs::read(root.join(name).join("receipt.json")).unwrap())
                .unwrap();
        for field in [
            "engine_authorized",
            "worker_dispatched",
            "training_admitted",
        ] {
            assert_eq!(receipt[field], false);
        }
        if name == "deadline" || name == "cancelled" {
            assert_eq!(receipt["failure"]["dispatch_may_have_occurred"], true);
            assert!(receipt.get("observation").is_none());
            continue;
        }
        let configuration =
            serde_json::from_slice(&observed_configuration::read(&root, "configuration.json"))
                .unwrap();
        let allowance = serde_json::from_slice(&observed_configuration::read(
            &root,
            &format!("{name}-allowance.json"),
        ))
        .unwrap();
        let request: EndpointChatRequest =
            serde_json::from_slice(&std::fs::read(root.join(name).join("request.json")).unwrap())
                .unwrap();
        let prepared =
            PreparedEndpointChat::new(&configuration, &request.model.clone(), &allowance, &request);
        // A service alias is not a binding id. Use the actual installed binding.
        assert!(prepared.is_err());
        let binding = &configuration.profile.as_ref().unwrap().bindings[0].id;
        let prepared =
            PreparedEndpointChat::new(&configuration, binding, &allowance, &request).unwrap();
        let observed = prepared
            .decode_response(std::fs::read(root.join(name).join("response.json")).unwrap())
            .unwrap();
        assert_eq!(
            serde_json::to_value(&observed.choice).unwrap(),
            receipt["observation"]["choice"]
        );
        assert_eq!(
            observed.request_sha256,
            receipt["observation"]["request_sha256"]
        );
        assert_eq!(
            observed.response_sha256,
            receipt["observation"]["response_sha256"]
        );
        assert_eq!(receipt["observation"]["http_status"], 200);
        assert!(receipt["observation"]["elapsed_ms"].as_u64().unwrap() > 0);
    }
}

#[test]
fn partial_reported_usage_preserves_unknowns_and_rejects_impossible_totals() {
    let (configuration, allowance, request) = inputs("available");
    let binding = &configuration.profile.as_ref().unwrap().bindings[0].id;
    let prepared =
        PreparedEndpointChat::new(&configuration, binding, &allowance, &request).unwrap();
    let original = response("available");
    let mut partial = original.clone();
    partial["usage"]
        .as_object_mut()
        .unwrap()
        .remove("completion_tokens");
    let observed = prepared
        .decode_response(serde_json::to_vec(&partial).unwrap())
        .unwrap();
    assert!(observed.usage.as_ref().unwrap().completion_tokens.is_none());
    assert_eq!(
        observed.usage.unwrap().prompt_tokens,
        original["usage"]["prompt_tokens"].as_u64()
    );
    // Explicit in-memory denial perturbations of the retained response. These
    // values are not provider observations and are never exported as examples.
    partial["usage"]["total_tokens"] = serde_json::json!(0);
    assert!(prepared
        .decode_response(serde_json::to_vec(&partial).unwrap())
        .is_err());
    let mut overflow = original;
    overflow["usage"]
        .as_object_mut()
        .unwrap()
        .remove("total_tokens");
    overflow["usage"]["prompt_tokens"] = serde_json::json!(u64::MAX);
    assert!(prepared
        .decode_response(serde_json::to_vec(&overflow).unwrap())
        .is_err());
}
