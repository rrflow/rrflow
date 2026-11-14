//! Malformed requests, session close and abort, socket rotation and commit races.

use super::*;

#[test]
pub(super) fn malformed_oversized_deadline_and_resource_fail_before_mutation() {
    let (_temporary, _root, server) = start_root();
    let (status, malformed) = http(
        server.address,
        "POST",
        "/v1/sessions",
        &[("Content-Type", "application/json")],
        b"{",
    );
    assert_eq!(status, 400);
    assert_eq!(malformed["outcome"]["error"]["code"], "invalid_argument");

    let oversized = vec![b' '; RRD_MAX_BODY_BYTES + 1];
    let (status, exhausted) = http(
        server.address,
        "POST",
        "/v1/sessions",
        &[("Content-Type", "application/json")],
        &oversized,
    );
    assert_eq!(status, 429);
    assert_eq!(exhausted["outcome"]["error"]["code"], "resource_exhausted");

    let expired = envelope(
        json!({
            "limits": {
                "idle_timeout_ms": 60_000,
                "absolute_timeout_ms": 300_000,
                "max_open_transactions": 2,
            }
        }),
        Some("expired-key"),
        Some(1),
    );
    let (status, deadline) = post(&server, "/v1/sessions", &expired, None);
    assert_eq!(status, 504);
    assert_eq!(deadline["outcome"]["error"]["code"], "deadline_exceeded");

    let mut wrong_resource = expired;
    wrong_resource["context"]["deadline_unix_ms"] = Value::Null;
    wrong_resource["resource"]["segments"][0]["id"] = json!("other-instance");
    let (status, denied) = post(&server, "/v1/sessions", &wrong_resource, None);
    assert_eq!(status, 412);
    assert_eq!(denied["outcome"]["error"]["code"], "failed_precondition");

    let (status, ready) = http(server.address, "GET", "/v1/health/ready", &[], &[]);
    assert_eq!(status, 200);
    assert_eq!(payload(&ready)["claim_sequence"], 0);
}

#[test]
pub(super) fn close_and_abort_paths_require_matching_session_identity() {
    let (_temporary, _root, server) = start_root();
    let create = envelope(
        json!({
            "limits": {
                "idle_timeout_ms": 60_000,
                "absolute_timeout_ms": 300_000,
                "max_open_transactions": 2,
            }
        }),
        Some("create-key"),
        None,
    );
    let (_, created) = post(&server, "/v1/sessions", &create, None);
    let session_id = payload(&created)["session_id"].as_str().unwrap();
    let token = payload(&created)["token"].as_str().unwrap();
    let close = envelope(json!({}), Some("close-key"), None);
    let (status, denied) = delete(
        &server,
        "/v1/sessions/different-session",
        &close,
        (session_id, token),
    );
    assert_eq!(status, 403);
    assert_eq!(denied["outcome"]["error"]["code"], "permission_denied");
    let (status, closed) = delete(
        &server,
        &format!("/v1/sessions/{session_id}"),
        &close,
        (session_id, token),
    );
    assert_eq!(status, 200);
    assert_eq!(payload(&closed)["state"], "closed");
    let (status, replay) = delete(
        &server,
        &format!("/v1/sessions/{session_id}"),
        &close,
        (session_id, token),
    );
    assert_eq!(status, 200);
    assert_eq!(payload(&replay)["idempotent_replay"], true);
}

#[test]
pub(super) fn public_correlation_ids_remain_header_safe() {
    assert!(CorrelationId::new("session:test_1-token").is_ok());
}

#[test]
pub(super) fn socket_rotation_quota_abort_and_idle_expiry_are_enforced() {
    let (_temporary, _root, server) = start_root();
    let create = envelope(
        json!({
            "limits": {
                "idle_timeout_ms": 1_000,
                "absolute_timeout_ms": 5_000,
                "max_open_transactions": 1,
            }
        }),
        Some("create-key"),
        None,
    );
    let (_, created) = post(&server, "/v1/sessions", &create, None);
    let session_id = payload(&created)["session_id"].as_str().unwrap().to_owned();
    let original_token = payload(&created)["token"].as_str().unwrap().to_owned();
    let renew = envelope(json!({}), Some("renew-key"), None);
    let renewal_path = format!("/v1/sessions/{session_id}/renew");
    let (status, renewed) = post(
        &server,
        &renewal_path,
        &renew,
        Some((&session_id, &original_token)),
    );
    assert_eq!(status, 200);
    let renewed_token = payload(&renewed)["token"].as_str().unwrap().to_owned();
    assert_ne!(renewed_token, original_token);
    let (status, renewal_replay) = post(
        &server,
        &renewal_path,
        &renew,
        Some((&session_id, &original_token)),
    );
    assert_eq!(status, 200);
    assert_eq!(payload(&renewal_replay), payload(&renewed));

    let begin = envelope(
        json!({"scope": "claims", "timeout_ms": 5_000}),
        Some("begin-key"),
        None,
    );
    let (status, old_token_denied) = post(
        &server,
        "/v1/transactions",
        &begin,
        Some((&session_id, &original_token)),
    );
    assert_eq!(status, 401);
    assert_eq!(
        old_token_denied["outcome"]["error"]["code"],
        "unauthenticated"
    );
    let (status, began) = post(
        &server,
        "/v1/transactions",
        &begin,
        Some((&session_id, &renewed_token)),
    );
    assert_eq!(status, 200);
    let transaction_id = payload(&began)["transaction_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let mut over_quota = begin.clone();
    over_quota["context"]["idempotency_key"] = json!("begin-over-quota");
    let (status, quota) = post(
        &server,
        "/v1/transactions",
        &over_quota,
        Some((&session_id, &renewed_token)),
    );
    assert_eq!(status, 429);
    assert_eq!(quota["outcome"]["error"]["code"], "resource_exhausted");

    let abort = envelope(json!({}), Some("abort-key"), None);
    let abort_path = format!("/v1/transactions/{transaction_id}");
    let (status, aborted) = delete(&server, &abort_path, &abort, (&session_id, &renewed_token));
    assert_eq!(status, 200);
    assert_eq!(payload(&aborted)["state"], "aborted");
    let (status, abort_replay) =
        delete(&server, &abort_path, &abort, (&session_id, &renewed_token));
    assert_eq!(status, 200);
    assert_eq!(payload(&abort_replay), payload(&aborted));

    std::thread::sleep(Duration::from_millis(1_100));
    let mut after_expiry = begin;
    after_expiry["context"]["idempotency_key"] = json!("begin-after-expiry");
    let (status, expired) = post(
        &server,
        "/v1/transactions",
        &after_expiry,
        Some((&session_id, &renewed_token)),
    );
    assert_eq!(status, 412);
    assert_eq!(expired["outcome"]["error"]["code"], "failed_precondition");
}

#[test]
pub(super) fn concurrent_same_commit_is_single_acceptance_and_retry_converges() {
    let (_temporary, _root, server) = start_root();
    let create = envelope(
        json!({
            "limits": {
                "idle_timeout_ms": 60_000,
                "absolute_timeout_ms": 300_000,
                "max_open_transactions": 2,
            }
        }),
        Some("create-key"),
        None,
    );
    let (_, created) = post(&server, "/v1/sessions", &create, None);
    let session_id = payload(&created)["session_id"].as_str().unwrap().to_owned();
    let token = payload(&created)["token"].as_str().unwrap().to_owned();
    let begin = envelope(
        json!({"scope": "claims", "timeout_ms": 60_000}),
        Some("begin-key"),
        None,
    );
    let (_, began) = post(
        &server,
        "/v1/transactions",
        &begin,
        Some((&session_id, &token)),
    );
    let transaction_id = payload(&began)["transaction_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let mutations = json!([{
        "mutation": "assert_claim",
        "subject": "document-1",
        "predicate": "contains",
        "object": "concurrent",
        "valid_from": 1,
        "tx_time": 1,
        "producer": "socket-test"
    }]);
    let typed: Vec<TransactionMutation> = serde_json::from_value(mutations.clone()).unwrap();
    let commit = envelope(
        json!({
            "operation_sha256": transaction_operation_sha256(&typed),
            "mutations": mutations,
        }),
        Some("commit-key"),
        Some(u64::MAX),
    );
    let path = format!("/v1/transactions/{transaction_id}/commit");
    let address = server.address;
    let mut workers = Vec::new();
    for _ in 0..2 {
        let body = serde_json::to_vec(&commit).unwrap();
        let path = path.clone();
        let session_id = session_id.clone();
        let authorization = format!("Bearer {token}");
        workers.push(std::thread::spawn(move || {
            http(
                address,
                "POST",
                &path,
                &[
                    ("Content-Type", "application/json"),
                    ("X-RRD-Session", &session_id),
                    ("Authorization", &authorization),
                ],
                &body,
            )
        }));
    }
    let results = workers
        .into_iter()
        .map(|worker| worker.join().unwrap())
        .collect::<Vec<_>>();
    assert!(results.iter().any(|(status, _)| *status == 200));
    assert!(results
        .iter()
        .all(|(status, _)| matches!(*status, 200 | 409)));

    let (status, converged) = post(&server, &path, &commit, Some((&session_id, &token)));
    assert_eq!(status, 200);
    assert_eq!(payload(&converged)["idempotent_replay"], true);
    let (status, ready) = http(server.address, "GET", "/v1/health/ready", &[], &[]);
    assert_eq!(status, 200);
    assert_eq!(payload(&ready)["claim_sequence"], 1);
}

#[test]
pub(super) fn cancelled_data_commit_restarts_and_converges_on_durable_retry() {
    let (_temporary, root, server) = start_root();
    let create = envelope(
        json!({
            "limits": {
                "idle_timeout_ms": 60_000,
                "absolute_timeout_ms": 300_000,
                "max_open_transactions": 2,
            }
        }),
        Some("create-key"),
        None,
    );
    let (_, created) = post(&server, "/v1/sessions", &create, None);
    let session_id = payload(&created)["session_id"].as_str().unwrap().to_owned();
    let token = payload(&created)["token"].as_str().unwrap().to_owned();
    let begin = envelope(
        json!({"scope": "data", "timeout_ms": 60_000}),
        Some("begin-key"),
        None,
    );
    let (_, began) = post(
        &server,
        "/v1/transactions",
        &begin,
        Some((&session_id, &token)),
    );
    let transaction_id = payload(&began)["transaction_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let typed = deployment_mutations();
    let mutations = serde_json::to_value(&typed).unwrap();
    let commit = envelope(
        json!({
            "operation_sha256": transaction_operation_sha256(&typed),
            "mutations": mutations,
        }),
        Some("commit-key"),
        Some(u64::MAX),
    );
    let commit_path = format!("/v1/transactions/{transaction_id}/commit");
    let body = serde_json::to_vec(&commit).unwrap();
    let authorization = format!("Bearer {token}");
    let mut stream = TcpStream::connect(server.address).unwrap();
    write!(
        stream,
        "POST {commit_path} HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nX-RRD-Session: {session_id}\r\nAuthorization: {authorization}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        server.address,
        body.len()
    )
    .unwrap();
    stream.write_all(&body).unwrap();
    stream.flush().unwrap();
    stream.shutdown(Shutdown::Write).unwrap();
    std::thread::sleep(Duration::from_millis(50));
    drop(stream);
    server.stop();

    let server = start(&root);
    let (status, first_retry) = post(&server, &commit_path, &commit, Some((&session_id, &token)));
    assert_eq!(status, 200);
    assert!(payload(&first_retry)["idempotent_replay"].is_boolean());
    let (status, converged) = post(&server, &commit_path, &commit, Some((&session_id, &token)));
    assert_eq!(status, 200);
    assert_eq!(payload(&converged)["idempotent_replay"], true);
    let (_, ready) = http(server.address, "GET", "/v1/health/ready", &[], &[]);
    assert_eq!(payload(&ready)["runtime_cursor"], 3);
    assert_eq!(payload(&ready)["claim_sequence"], 0);
}
