//! These inputs are retained actual endpoint calls and installed configuration.
//! Denial probes mutate those inputs in memory; they are not training examples,
//! model-produced results, authorization receipts or quality labels.
#[path = "support/observed_configuration.rs"]
mod observed_configuration;
use rrd_contract::{
    endpoint_profile_sha256, EndpointAbstentionReason, EndpointConfiguration,
    EndpointEffortSupport, EndpointSelectionAllowance, EndpointSelectionChoice,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::path::PathBuf;

fn root(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/endpoint-selection-observed")
        .join(name)
}
fn json(name: &str, file: &str) -> Value {
    serde_json::from_slice(&observed_configuration::read(&root(name), file)).unwrap()
}
fn observation(name: &str) -> (EndpointConfiguration, EndpointSelectionAllowance, Vec<u8>) {
    (
        serde_json::from_value(json(name, "configuration.json")).unwrap(),
        serde_json::from_value(json(name, "allowance.json")).unwrap(),
        observed_configuration::read(&root(name), "choice-content.json"),
    )
}
fn sha(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[test]
fn extra_request_fields_are_accepted_but_never_authoritative() {
    use rrd_contract::EndpointSelectionRequest;
    let schema = serde_json::to_value(schemars::schema_for!(EndpointSelectionRequest)).unwrap();
    assert_eq!(schema["additionalProperties"], true);
    let original = serde_json::json!({
        "scope": "project",
        "task_id": "task-1",
        "attempt_id": "attempt-1",
        "max_storage_keys": 1,
        "expected_task_sha256": "a".repeat(64),
        "expected_configuration_sha256": "b".repeat(64)
    });
    let expected: EndpointSelectionRequest = serde_json::from_value(original.clone()).unwrap();
    expected.validate().unwrap();
    let mut future = original;
    future["endpoint"] = serde_json::json!("https://not-an-authorized-endpoint.example");
    future["observation"] =
        serde_json::json!({"decision": "grant", "credential": "not-a-credential"});
    let decoded: EndpointSelectionRequest = serde_json::from_value(future).unwrap();
    decoded.validate().unwrap();
    assert_eq!(decoded, expected);
    assert_eq!(
        serde_json::to_value(decoded).unwrap(),
        serde_json::to_value(expected).unwrap()
    );
}

#[test]
fn retained_calls_bind_actual_bytes_and_no_learning_claim() {
    for name in ["available", "withheld"] {
        for (file, digest) in json(name, "files.json").as_object().unwrap() {
            assert_eq!(
                sha(&std::fs::read(root(name).join(file)).unwrap()),
                digest.as_str().unwrap()
            );
        }
        let (configuration, allowance, response) = observation(name);
        configuration.validate().unwrap();
        allowance.validate_for(&configuration).unwrap();
        let provider = json(name, "choice-response.json");
        assert_eq!(
            provider["choices"][0]["message"]["content"]
                .as_str()
                .unwrap()
                .as_bytes(),
            response
        );
        let receipt = json(name, "receipt.json");
        assert_eq!(
            receipt["configurationSha256"],
            configuration.sha256().unwrap()
        );
        assert_eq!(receipt["choiceCall"]["contentSha256"], sha(&response));
        assert_eq!(receipt["choiceCall"]["httpStatus"], 200);
        for (call, prefix) in [
            ("defaultEffortCall", "default-effort"),
            ("choiceCall", "choice"),
        ] {
            for (field, suffix) in [("requestSha256", "request"), ("responseSha256", "response")] {
                assert_eq!(
                    receipt[call][field],
                    sha(
                        &std::fs::read(root(name).join(format!("{prefix}-{suffix}.json"))).unwrap()
                    )
                );
            }
            let output = json(name, &format!("{prefix}-response.json"));
            assert_eq!(
                receipt[call]["contentSha256"],
                sha(output["choices"][0]["message"]["content"]
                    .as_str()
                    .unwrap()
                    .as_bytes())
            );
            assert_eq!(receipt[call]["model"], output["model"]);
        }
        let effort = json(name, "effort-observation.json");
        assert_eq!(effort["observation"], receipt["defaultEffortCall"]);
        assert_eq!(effort["engineAttested"], false);
        assert_eq!(effort["qualityQualified"], false);
        for field in [
            "workerDispatched",
            "trainingAdmitted",
            "humanReviewed",
            "routingQualified",
        ] {
            assert_eq!(receipt[field], false);
        }
        for candidate in &allowance.candidates {
            let EndpointEffortSupport::Observed {
                evidence_sha256, ..
            } = &candidate.effort
            else {
                panic!("actual observed default call is missing")
            };
            assert_eq!(
                *evidence_sha256,
                sha(&std::fs::read(root(name).join("effort-observation.json")).unwrap())
            );
        }
    }
}

#[test]
fn actual_model_choice_and_withheld_candidate_abstention_validate() {
    let (configuration, allowance, response) = observation("available");
    let choice = EndpointSelectionChoice::decode(&response, &allowance, &configuration).unwrap();
    assert!(
        matches!(choice, EndpointSelectionChoice::Select { ref binding_id, effort: None } if binding_id == &allowance.candidates[0].binding_id)
    );
    let (configuration, allowance, response) = observation("withheld");
    assert!(allowance.candidates.is_empty());
    assert_eq!(
        EndpointSelectionChoice::decode(&response, &allowance, &configuration).unwrap(),
        EndpointSelectionChoice::Abstain {
            reason: EndpointAbstentionReason::NoEligibleCandidate
        }
    );
}

#[test]
fn changed_configuration_does_not_rebind_an_existing_choice() {
    let (mut configuration, allowance, response) = observation("available");
    configuration.revision += 1;
    let profile = configuration.profile.as_mut().unwrap();
    profile.revision = configuration.revision;
    profile.profile_sha256 = endpoint_profile_sha256(profile).unwrap();
    configuration.validate().unwrap();
    assert!(
        EndpointSelectionChoice::decode(&response, &allowance, &configuration)
            .unwrap_err()
            .to_string()
            .contains("configuration changed")
    );
}

#[test]
fn missing_duplicate_or_changed_candidate_facts_are_rejected() {
    let (configuration, original, response) = observation("available");
    let mut allowance = original.clone();
    allowance.candidates.clear();
    assert!(EndpointSelectionChoice::decode(&response, &allowance, &configuration).is_err());
    let mut allowance = original.clone();
    allowance.limits.maximum_candidates += 1;
    allowance.candidates.push(allowance.candidates[0].clone());
    assert!(allowance
        .validate_for(&configuration)
        .unwrap_err()
        .to_string()
        .contains("sorted and unique"));
    let mut allowance = original.clone();
    allowance.candidates[0].binding_id = allowance.candidates[0].service_id.clone();
    assert!(allowance.validate_for(&configuration).is_err());
    let mut allowance = original.clone();
    allowance.candidates[0].service_id = allowance.candidates[0].binding_id.clone();
    assert!(allowance.validate_for(&configuration).is_err());
    let mut allowance = original;
    allowance.candidates[0].manifest_sha256 = None;
    assert!(allowance.validate_for(&configuration).is_err());
}

#[test]
fn unknown_effort_and_unobserved_values_do_not_become_defaults() {
    let (configuration, mut allowance, response) = observation("available");
    let mut choice = json("available", "choice-content.json");
    choice["effort"] = Value::String(allowance.candidates[0].service_id.to_string());
    assert!(EndpointSelectionChoice::decode(
        &serde_json::to_vec(&choice).unwrap(),
        &allowance,
        &configuration
    )
    .unwrap_err()
    .to_string()
    .contains("unsupported"));
    allowance.candidates[0].effort = EndpointEffortSupport::Unknown;
    allowance.validate_for(&configuration).unwrap();
    assert!(
        EndpointSelectionChoice::decode(&response, &allowance, &configuration)
            .unwrap_err()
            .to_string()
            .contains("unknown")
    );
}

#[test]
fn limits_apply_before_json_decoding_and_to_allowance_encoding() {
    let (configuration, mut allowance, response) = observation("available");
    allowance.limits.maximum_response_bytes = response.len() as u64 - 1;
    allowance.limits.maximum_effort_bytes = allowance.limits.maximum_response_bytes as u32;
    assert!(
        EndpointSelectionChoice::decode(&response, &allowance, &configuration)
            .unwrap_err()
            .to_string()
            .contains("response exceeds byte limit")
    );
    let (_, mut allowance, _) = observation("available");
    allowance.limits.maximum_allowance_bytes = 1;
    assert!(allowance.validate_for(&configuration).is_err());
}

#[test]
fn malformed_or_authority_bearing_output_never_returns_a_choice() {
    let (configuration, allowance, response) = observation("available");
    assert!(EndpointSelectionChoice::decode(
        &response[..response.len() - 1],
        &allowance,
        &configuration
    )
    .is_err());
    let mut choice = json("available", "choice-content.json");
    choice["authorization_granted"] = Value::Bool(true);
    assert!(EndpointSelectionChoice::decode(
        &serde_json::to_vec(&choice).unwrap(),
        &allowance,
        &configuration
    )
    .is_err());
    let mut repeated = response[..response.len() - 1].to_vec();
    repeated.extend_from_slice(b",\"kind\":\"select\"}");
    assert!(EndpointSelectionChoice::decode(&repeated, &allowance, &configuration).is_err());
}

#[test]
fn actual_empty_candidate_abstention_cannot_claim_nonempty_candidates_are_absent() {
    let (configuration, allowance, _) = observation("available");
    let (_, _, abstention) = observation("withheld");
    assert!(EndpointSelectionChoice::decode(&abstention, &allowance, &configuration).is_err());
}
