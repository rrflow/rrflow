//! Local protocol fixtures only. No actual engine result or learning label is asserted.
use rrd_contract::*;
use rrd_observation::*;
use serde_json::json;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::time::{Duration, Instant};

const EVENT: &str = "11111111-1111-4111-8111-111111111111";
const CAPTURE: &str = "22222222-2222-4222-8222-222222222222";

fn configuration(url: String) -> (EndpointBinding, ObservationExportPolicy) {
    let binding = serde_json::from_value(
        json!({"id":"fixture-sink","class":"behavior_updates","kind":"api",
        "endpoint":{"kind":"url","url":url},"service_id":"fixture-service","manifest_sha256":null,
        "credential":{"provider":"fixture","name":"fixture-key"},"placement":null,
        "footprint":{"memory_bytes":1024,"gpu":{"kind":"none"}},
        "limits":{"max_concurrency":1,"timeout_ms":1000,"max_input_bytes":16384}}),
    )
    .unwrap();
    let policy = serde_json::from_value(json!({"contract_version":1,"binding_id":"fixture-sink",
        "expected_devspace":"fixture-space","protocol_evidence_sha256":"a".repeat(64),
        "allow_network":true,"allow_metadata_export":true,"allowed_scopes":["default"],
        "max_retained_deliveries":8,"retention_ms":60000,"max_attempts":3,"retry_delay_ms":10,
        "max_response_bytes":4096}))
    .unwrap();
    (binding, policy)
}
fn references() -> EngineReferences {
    EngineReferences {
        version: 1,
        instance_id: "fixture-instance".into(),
        scope: "default".into(),
        operation: "task-attempt-read".into(),
        response_sha256: "a".repeat(64),
        task_id: Some("fixture-task".into()),
        attempt_id: Some("fixture-attempt".into()),
        job_id: None,
        plan_sha256: None,
        source_sha256: None,
        configuration_sha256: Some("b".repeat(64)),
        selector_policy_id: None,
        selector_routing_sha256: None,
        selector_narrowed_candidate_count: None,
    }
}
#[test]
fn selector_references_are_validated_and_stay_absent_when_unrouted() {
    let unrouted = references();
    unrouted.validate().unwrap();
    let encoded = serde_json::to_value(&unrouted).unwrap();
    assert!(encoded.get("selector_policy_id").is_none());
    assert!(encoded.get("selector_routing_sha256").is_none());
    assert!(encoded.get("selector_narrowed_candidate_count").is_none());

    let mut routed = references();
    routed.selector_policy_id = Some("semantic-router-v1".into());
    routed.selector_routing_sha256 = Some("c".repeat(64));
    routed.selector_narrowed_candidate_count = Some(2);
    routed.validate().unwrap();
    let reopened: EngineReferences =
        serde_json::from_value(serde_json::to_value(&routed).unwrap()).unwrap();
    assert_eq!(reopened, routed);

    for corrupt in [
        |references: &mut EngineReferences| references.selector_policy_id = Some(String::new()),
        |references: &mut EngineReferences| {
            references.selector_routing_sha256 = Some("short".into())
        },
        |references: &mut EngineReferences| references.selector_narrowed_candidate_count = Some(0),
    ] {
        let mut invalid = routed.clone();
        corrupt(&mut invalid);
        assert!(invalid.validate().is_err());
    }
}
fn payload() -> Vec<u8> {
    encode_event(
        EVENT,
        CAPTURE,
        "fixture-delivery",
        1793491200000,
        &references(),
    )
    .unwrap()
}
fn request(stream: &mut TcpStream) -> Vec<u8> {
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    let mut bytes = Vec::new();
    let end = loop {
        let mut byte = [0];
        stream.read_exact(&mut byte).unwrap();
        bytes.push(byte[0]);
        assert!(bytes.len() < 16384);
        if bytes.ends_with(b"\r\n\r\n") {
            break bytes.len();
        }
    };
    let headers = String::from_utf8(bytes.clone()).unwrap();
    let length: usize = headers
        .lines()
        .find_map(|line| {
            line.split_once(':')
                .filter(|(name, _)| name.eq_ignore_ascii_case("content-length"))
                .map(|(_, value)| value.trim().parse().unwrap())
        })
        .unwrap();
    assert!(length < 16384);
    bytes.resize(end + length, 0);
    stream.read_exact(&mut bytes[end..]).unwrap();
    bytes
}
fn reply(stream: &mut TcpStream, status: u16, body: &str, extra: &str) {
    write!(
        stream,
        "HTTP/1.1 {status} fixture\r\nContent-Length: {}\r\nConnection: close\r\n{extra}\r\n{body}",
        body.len()
    )
    .unwrap();
}
fn ack() -> String {
    json!({"devspace":"fixture-space","accepted_ids":[EVENT]}).to_string()
}

#[test]
fn codec_is_closed_bounded_and_does_not_contain_source_content() {
    let encoded: serde_json::Value = serde_json::from_slice(&payload()).unwrap();
    let event = &encoded["events"][0];
    assert_eq!(event["kind"], "engine.observed");
    assert_eq!(event["observed_at"], "2026-11-01T00:00:00.000Z");
    for forbidden in ["payload", "usage", "model", "native_status", "exit_code"] {
        assert!(event.get(forbidden).is_none());
    }
    let mut invalid = references();
    invalid.scope = "x".repeat(129);
    assert!(encode_event(EVENT, CAPTURE, "fixture-delivery", 1, &invalid).is_err());
    assert!(encode_event("invalid", CAPTURE, "fixture-delivery", 1, &references()).is_err());
}

#[test]
fn exact_acknowledgement_is_required_and_reply_errors_expose_no_body() {
    for (body, expected) in [
        (ack(), ObservationDeliveryDisposition::Acknowledged),
        (
            json!({"devspace":"foreign","accepted_ids":[EVENT]}).to_string(),
            ObservationDeliveryDisposition::ReplyRejected,
        ),
        (
            json!({"devspace":"fixture-space","accepted_ids":["different"]}).to_string(),
            ObservationDeliveryDisposition::ReplyRejected,
        ),
        (
            "SECRET_RESPONSE".into(),
            ObservationDeliveryDisposition::ReplyRejected,
        ),
        (
            "x".repeat(4097),
            ObservationDeliveryDisposition::ReplyTooLarge,
        ),
    ] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let (binding, policy) = configuration(format!("http://{}", listener.local_addr().unwrap()));
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let bytes = request(&mut stream);
            reply(&mut stream, 200, &body, "");
            bytes
        });
        let bytes = payload();
        let prepared = PreparedExport::new(&binding, &policy, &bytes, EVENT).unwrap();
        let result = prepared.execute("fixture-secret", Instant::now() + Duration::from_secs(2));
        assert_eq!(result.disposition, expected);
        assert!(result.request_may_have_been_sent);
        assert!(!format!("{result:?}").contains("SECRET_RESPONSE"));
        let sent = server.join().unwrap();
        assert!(sent.ends_with(&bytes));
    }
}

#[test]
fn lost_ack_requires_explicit_retry_and_preserves_identical_payload() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let (binding, policy) = configuration(format!("http://{}", listener.local_addr().unwrap()));
    let server = std::thread::spawn(move || {
        let (mut first, _) = listener.accept().unwrap();
        let one = request(&mut first);
        drop(first);
        let (mut second, _) = listener.accept().unwrap();
        let two = request(&mut second);
        reply(&mut second, 200, &ack(), "");
        (one, two)
    });
    let bytes = payload();
    let prepared = PreparedExport::new(&binding, &policy, &bytes, EVENT).unwrap();
    let first = prepared.execute("fixture-secret", Instant::now() + Duration::from_secs(2));
    assert_eq!(
        first.disposition,
        ObservationDeliveryDisposition::TransportUnavailable
    );
    assert!(first.request_may_have_been_sent);
    let second = prepared.execute("fixture-secret", Instant::now() + Duration::from_secs(2));
    assert_eq!(
        second.disposition,
        ObservationDeliveryDisposition::Acknowledged
    );
    let (one, two) = server.join().unwrap();
    assert!(one.ends_with(&bytes));
    assert!(two.ends_with(&bytes));
}

#[test]
fn redirects_never_receive_a_second_request() {
    let destination = TcpListener::bind("127.0.0.1:0").unwrap();
    destination.set_nonblocking(true).unwrap();
    let location = format!(
        "Location: http://{}/collect\r\n",
        destination.local_addr().unwrap()
    );
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let (binding, policy) = configuration(format!("http://{}", listener.local_addr().unwrap()));
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        request(&mut stream);
        reply(&mut stream, 302, "", &location);
    });
    let bytes = payload();
    let prepared = PreparedExport::new(&binding, &policy, &bytes, EVENT).unwrap();
    let observed = prepared.execute("fixture-secret", Instant::now() + Duration::from_secs(2));
    assert_eq!(
        observed.disposition,
        ObservationDeliveryDisposition::ReplyRejected
    );
    server.join().unwrap();
    assert_eq!(
        destination.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
}

#[test]
fn denied_policy_unsafe_url_and_expired_deadline_do_not_dispatch() {
    for url in [
        "http://remote.invalid",
        "https://sink.invalid?secret=value",
        "https://sink.invalid/#fragment",
        "https://user:secret@sink.invalid",
    ] {
        let (binding, policy) = configuration(url.into());
        assert!(PreparedExport::new(&binding, &policy, &payload(), EVENT).is_err());
    }
    let (binding, mut policy) = configuration("https://sink.invalid".into());
    policy.allow_metadata_export = false;
    assert!(PreparedExport::new(&binding, &policy, &payload(), EVENT).is_err());
    policy.allow_metadata_export = true;
    let bytes = payload();
    let prepared = PreparedExport::new(&binding, &policy, &bytes, EVENT).unwrap();
    let observed = prepared.execute("fixture-secret", Instant::now() - Duration::from_secs(1));
    assert!(!observed.request_may_have_been_sent);
    assert_eq!(
        observed.disposition,
        ObservationDeliveryDisposition::DeadlineExceeded
    );
}
