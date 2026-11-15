#![cfg(feature = "endpoint-chat")]
//! Protocol correctness fixtures, not worker quality or training examples.
#[path = "support/observed_configuration.rs"]
mod observed_configuration;

use rrd_contract::{
    EndpointConfiguration, EndpointGenerationSettings, EndpointResponseFormat,
    EndpointTokenParameter,
};
use rrd_inference::endpoint_chat::{ChatMessage, ChatRole, ModelChatRequest, PreparedModelChat};
use rrd_inference::{EmbeddingCancellation, EmbeddingExecutionControl, NetworkPolicy};
use serde_json::Value;
use std::path::PathBuf;
use std::time::{Duration, Instant};

fn inputs() -> (EndpointConfiguration, ModelChatRequest, Value) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../transport/rrd-contract/tests/fixtures/endpoint-selection-observed/available");
    let configuration: EndpointConfiguration =
        serde_json::from_slice(&observed_configuration::read(&root, "configuration.json")).unwrap();
    let model = configuration.profile.as_ref().unwrap().bindings[0]
        .service_id
        .clone();
    let response =
        serde_json::from_slice(&observed_configuration::read(&root, "choice-response.json"))
            .unwrap();
    (
        configuration,
        ModelChatRequest {
            model,
            messages: vec![ChatMessage {
                role: ChatRole::User,
                content: "Protocol fixture only".into(),
            }],
            settings: EndpointGenerationSettings {
                maximum_output_tokens: 256,
                token_parameter: EndpointTokenParameter::MaxCompletionTokens,
                temperature_milli: None,
                response_format: EndpointResponseFormat::Text,
            },
            reasoning_effort: None,
        },
        response,
    )
}

#[test]
fn configured_generation_parameters_are_explicit_and_content_is_not_a_route_choice() {
    let (configuration, request, mut response) = inputs();
    let binding = &configuration.profile.as_ref().unwrap().bindings[0].id;
    let prepared = PreparedModelChat::new(&configuration, binding, &request, 16_384).unwrap();
    let wire: Value = serde_json::from_slice(prepared.request_bytes()).unwrap();
    assert_eq!(wire["max_completion_tokens"], 256);
    assert!(wire.get("max_tokens").is_none());
    assert!(wire.get("temperature").is_none());
    assert!(wire.get("reasoning_effort").is_none());
    assert_eq!(wire["response_format"]["type"], "text");
    let content = "A bounded draft is not an accepted project change.";
    response["choices"][0]["message"]["content"] = content.into();
    response.as_object_mut().unwrap().remove("usage");
    let observed = prepared
        .decode_response(serde_json::to_vec(&response).unwrap())
        .unwrap();
    assert_eq!(observed.content, content);
    assert_eq!(
        observed.content_sha256,
        rrd_core::digest::sha256_hex(content.as_bytes())
    );
    assert!(observed.usage.is_none());
    assert!(observed.elapsed_ms.is_none());
    assert!(observed.http_status.is_none());
}

#[test]
fn model_content_keeps_envelope_effect_and_identity_denials() {
    let (configuration, request, response) = inputs();
    let binding = &configuration.profile.as_ref().unwrap().bindings[0].id;
    let prepared = PreparedModelChat::new(&configuration, binding, &request, 16_384).unwrap();
    let mut wrong_model = response.clone();
    wrong_model["model"] = "unconfigured-model".into();
    let mut truncated = response.clone();
    truncated["choices"][0]["finish_reason"] = "length".into();
    let mut tools = response.clone();
    tools["choices"][0]["message"]["tool_calls"] = serde_json::json!([{}]);
    let mut reasoning = response.clone();
    reasoning["choices"][0]["message"]["reasoning_content"] = "unsupported field".into();
    for value in [wrong_model, truncated, tools, reasoning] {
        assert!(prepared
            .decode_response(serde_json::to_vec(&value).unwrap())
            .is_err());
    }
    assert!(prepared.decode_response(vec![b'x'; 16_385]).is_err());
    assert!(PreparedModelChat::new(&configuration, binding, &request, 0).is_err());
}

#[test]
fn rejected_content_distinguishes_completed_generation_from_unknown_outcome() {
    use rrd_inference::endpoint_chat::EndpointChatOutcome;
    let (configuration, mut request, response) = inputs();
    request.settings.response_format = EndpointResponseFormat::JsonObject;
    let binding = &configuration.profile.as_ref().unwrap().bindings[0].id;
    let prepared = PreparedModelChat::new(&configuration, binding, &request, 16_384).unwrap();
    let changes = [
        (
            "/choices/0/finish_reason",
            serde_json::json!("length"),
            false,
        ),
        (
            "/choices/0/finish_reason",
            serde_json::json!("content_filter"),
            false,
        ),
        (
            "/choices/0/message/content",
            serde_json::json!("not JSON"),
            false,
        ),
        ("/choices/0/message/content", Value::Null, false),
        ("/model", serde_json::json!("wrong-model"), true),
        (
            "/choices/0/finish_reason",
            serde_json::json!("unrecognized"),
            true,
        ),
        ("/choices/0/finish_reason", Value::Null, true),
        ("/choices/0/message/role", serde_json::json!("tool"), true),
    ];
    for (pointer, value, unknown) in changes {
        let mut changed = response.clone();
        *changed.pointer_mut(pointer).unwrap() = value;
        let error = prepared
            .decode_response(serde_json::to_vec(&changed).unwrap())
            .unwrap_err();
        assert_eq!(
            error.execution_may_continue(),
            unknown,
            "{pointer}: {changed}"
        );
        assert!(error.dispatch_may_have_occurred());
        assert_eq!(
            error.outcome,
            if unknown {
                EndpointChatOutcome::Unknown
            } else {
                EndpointChatOutcome::ResponseComplete
            }
        );
    }
    for field in ["tool_calls", "function_call"] {
        let mut changed = response.clone();
        changed["choices"][0]["message"][field] = if field == "tool_calls" {
            serde_json::json!([{}])
        } else {
            serde_json::json!({})
        };
        assert!(prepared
            .decode_response(serde_json::to_vec(&changed).unwrap())
            .unwrap_err()
            .execution_may_continue());
    }
    let mut usage = response.clone();
    usage["usage"] =
        serde_json::json!({"prompt_tokens": 1, "completion_tokens": 2, "total_tokens": 99});
    assert!(!prepared
        .decode_response(serde_json::to_vec(&usage).unwrap())
        .unwrap_err()
        .execution_may_continue());
    for bytes in [b"{".to_vec(), vec![b'x'; 16_385]] {
        assert!(prepared
            .decode_response(bytes)
            .unwrap_err()
            .execution_may_continue());
    }
}

#[test]
fn explicit_settings_and_json_format_are_enforced_without_default_substitution() {
    let (configuration, mut request, mut response) = inputs();
    let binding = &configuration.profile.as_ref().unwrap().bindings[0].id;
    request.settings.token_parameter = EndpointTokenParameter::MaxTokens;
    request.settings.temperature_milli = Some(125);
    request.settings.response_format = EndpointResponseFormat::JsonObject;
    request.reasoning_effort = Some("configured-effort".into());
    let prepared = PreparedModelChat::new(&configuration, binding, &request, 16_384).unwrap();
    let wire: Value = serde_json::from_slice(prepared.request_bytes()).unwrap();
    assert_eq!(wire["max_tokens"], 256);
    assert!(wire.get("max_completion_tokens").is_none());
    assert_eq!(wire["temperature"], 0.125);
    assert_eq!(wire["reasoning_effort"], "configured-effort");
    response["choices"][0]["message"]["content"] = "plain text".into();
    assert!(prepared
        .decode_response(serde_json::to_vec(&response).unwrap())
        .is_err());
    response["choices"][0]["message"]["content"] = "[]".into();
    assert!(prepared
        .decode_response(serde_json::to_vec(&response).unwrap())
        .is_err());
    response["choices"][0]["message"]["content"] = "{\"draft\":true}".into();
    prepared
        .decode_response(serde_json::to_vec(&response).unwrap())
        .unwrap();
    for tokens in [0, 65_537] {
        request.settings.maximum_output_tokens = tokens;
        assert!(PreparedModelChat::new(&configuration, binding, &request, 16_384).is_err());
    }
}

#[test]
fn worker_network_and_stop_denials_do_not_resolve_credentials() {
    let (configuration, request, _) = inputs();
    let binding = &configuration.profile.as_ref().unwrap().bindings[0].id;
    let prepared = PreparedModelChat::new(&configuration, binding, &request, 16_384).unwrap();
    let cancellation = EmbeddingCancellation::default();
    let control = EmbeddingExecutionControl::new(
        Instant::now() + Duration::from_secs(2),
        cancellation.clone(),
    );
    let denied = prepared
        .execute(NetworkPolicy::Deny, &control, |_, _| {
            panic!("denied resolver")
        })
        .unwrap_err();
    assert!(!denied.dispatch_may_have_occurred());
    cancellation.cancel();
    let stopped = prepared
        .execute(NetworkPolicy::Allow, &control, |_, _| {
            panic!("stopped resolver")
        })
        .unwrap_err();
    assert!(!stopped.dispatch_may_have_occurred());
}

#[test]
fn loopback_transport_records_exact_draft_and_wire_without_selector_decoding() {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    let (mut configuration, request, mut response) = inputs();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let profile = configuration.profile.as_mut().unwrap();
    profile.bindings[0].endpoint = rrd_contract::EndpointLocator::Url {
        url: format!("http://{}/v1", listener.local_addr().unwrap()),
    };
    profile.bindings[0].credential = None;
    profile.profile_sha256 = rrd_contract::endpoint_profile_sha256(profile).unwrap();
    response["choices"][0]["message"]["content"] =
        "Controlled transport draft; no task result.".into();
    let expected_response = serde_json::to_vec(&response).unwrap();
    let reply = expected_response.clone();
    let server = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
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
        assert!(head.starts_with("POST /v1/chat/completions HTTP/1.1"));
        let length: usize = head
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse().unwrap())
            })
            .unwrap();
        assert!(length < 16_384);
        let mut body = vec![0; length];
        socket.read_exact(&mut body).unwrap();
        write!(socket, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", reply.len()).unwrap();
        socket.write_all(&reply).unwrap();
        body
    });
    let binding = &configuration.profile.as_ref().unwrap().bindings[0].id;
    let prepared = PreparedModelChat::new(&configuration, binding, &request, 16_384).unwrap();
    let control = EmbeddingExecutionControl::new(
        Instant::now() + Duration::from_secs(5),
        EmbeddingCancellation::default(),
    );
    let result = prepared
        .execute(NetworkPolicy::Allow, &control, |_, _| {
            panic!("no credential configured")
        })
        .unwrap();
    assert_eq!(server.join().unwrap(), prepared.request_bytes());
    assert_eq!(result.response_bytes, expected_response);
    assert_eq!(
        result.response_sha256,
        rrd_core::digest::sha256_hex(&expected_response)
    );
    assert_eq!(
        result.request_sha256,
        rrd_core::digest::sha256_hex(prepared.request_bytes())
    );
    assert_eq!(result.configuration_sha256, configuration.sha256().unwrap());
    assert_eq!(result.http_status, Some(200));
    assert!(result.elapsed_ms.is_some());
    assert_eq!(
        result.content,
        "Controlled transport draft; no task result."
    );
}
