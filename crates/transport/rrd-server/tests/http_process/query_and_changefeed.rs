//! Authenticated rrflowQL queries, changefeed follow, managed backup and restore.

use super::*;

#[test]
pub(super) fn authenticated_query_exposes_exact_rrflowql_rrd_query_executor_contract() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("instance");
    seed_query_fixture(&root);
    let server = start(&root);
    let create = envelope(
        json!({
            "limits": {
                "idle_timeout_ms": 60_000,
                "absolute_timeout_ms": 300_000,
                "max_open_transactions": 2
            }
        }),
        Some("query-session"),
        None,
    );
    let (status, created) = post(&server, "/v1/sessions", &create, None);
    assert_eq!(status, 200);
    let session_id = payload(&created)["session_id"].as_str().unwrap();
    let token = payload(&created)["token"].as_str().unwrap();
    let query = envelope(
        json!({
            "scope": "instance:socket-test",
            "query": "FROM record:document AT VALID 100 KNOWN HEAD PROJECT id, title EXPLAIN CONTRACT",
            "parameters": {},
            "budget": {
                "max_storage_keys": 100,
                "max_rows": 10,
                "max_output_bytes": 16384,
                "max_batch_rows": 10
            }
        }),
        None,
        None,
    );
    let (status, denied) = post(&server, "/v1/query", &query, None);
    assert_eq!(status, 401);
    assert_eq!(denied["outcome"]["error"]["code"], "unauthenticated");

    let (status, result) = post(&server, "/v1/query", &query, Some((session_id, token)));
    assert_eq!(status, 200, "{result}");
    let result = payload(&result);
    assert_eq!(result["scope"], "instance:socket-test");
    assert_eq!(result["schema_revision"], 1);
    assert_eq!(result["plan"]["exact"], true);
    assert_eq!(result["plan"]["candidates"][0]["selected"], true);
    assert_eq!(result["execution"]["returned_rows"], 1);
    assert!(result["execution"]["selected_versions"].as_u64().unwrap() > 0);
    assert_direct_read_evidence(
        &result["execution"]["read_evidence"],
        ReadAccessPath::RecordVersions,
    );
    assert_eq!(result["rows"][0]["identity"], "record:document:alpha");
    assert_eq!(result["rows"][0]["values"]["title"]["type"], "string");
    assert_eq!(result["rows"][0]["values"]["title"]["value"], "Alpha");

    let mut wrong_scope = query;
    wrong_scope["payload"]["scope"] = json!("instance:other");
    let (status, wrong) = post(
        &server,
        "/v1/query",
        &wrong_scope,
        Some((session_id, token)),
    );
    assert_eq!(status, 400);
    assert_eq!(wrong["outcome"]["error"]["code"], "invalid_argument");
}

#[test]
pub(super) fn bounded_changefeed_follow_wakes_on_a_commit_and_times_out_at_the_same_cursor() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("instance");
    seed_query_fixture(&root);
    let server = start(&root);
    let create = envelope(
        json!({
            "limits": {
                "idle_timeout_ms": 60_000,
                "absolute_timeout_ms": 300_000,
                "max_open_transactions": 2
            }
        }),
        Some("follow-session"),
        None,
    );
    let (status, created) = post(&server, "/v1/sessions", &create, None);
    assert_eq!(status, 200, "{created}");
    let session_id = payload(&created)["session_id"].as_str().unwrap().to_owned();
    let token = payload(&created)["token"].as_str().unwrap().to_owned();
    let follow = envelope(
        json!({
            "read": {
                "scope": "instance:socket-test",
                "after_cursor": 2,
                "limit": 8
            },
            "wait_timeout_ms": 5_000
        }),
        None,
        None,
    );
    let address = server.address;
    let follow_body = serde_json::to_vec(&follow).unwrap();
    let follow_session = session_id.clone();
    let follow_token = token.clone();
    let waiting = std::thread::spawn(move || {
        let authorization = format!("Bearer {follow_token}");
        http(
            address,
            "POST",
            "/v1/changes/follow",
            &[
                ("Content-Type", "application/json"),
                ("X-RRD-Session", &follow_session),
                ("Authorization", &authorization),
            ],
            &follow_body,
        )
    });
    std::thread::sleep(Duration::from_millis(50));

    let begin = envelope(
        json!({"scope": "data", "timeout_ms": 10_000}),
        Some("follow-begin"),
        None,
    );
    let (status, began) = post(
        &server,
        "/v1/transactions",
        &begin,
        Some((&session_id, &token)),
    );
    assert_eq!(status, 200, "{began}");
    let transaction = payload(&began)["transaction_id"].as_str().unwrap();
    let mutations = json!([{
        "mutation": "append_event",
        "kind": "observed",
        "subject": {"kind": "document", "id": "alpha"},
        "properties": {"source": {"type": "string", "value": "follow-test"}}
    }]);
    let typed: Vec<TransactionMutation> = serde_json::from_value(mutations.clone()).unwrap();
    let commit = envelope(
        json!({
            "operation_sha256": transaction_operation_sha256(&typed),
            "mutations": mutations
        }),
        Some("follow-commit"),
        Some(u64::MAX),
    );
    let (status, committed) = post(
        &server,
        &format!("/v1/transactions/{transaction}/commit"),
        &commit,
        Some((&session_id, &token)),
    );
    assert_eq!(status, 200, "{committed}");
    assert_eq!(payload(&committed)["last_runtime_cursor"], 3);

    let (status, followed) = waiting.join().unwrap();
    assert_eq!(status, 200, "{followed}");
    assert_eq!(payload(&followed)["timed_out"], false);
    assert_eq!(payload(&followed)["page"]["through_cursor"], 3);
    assert_eq!(payload(&followed)["page"]["changes"][0]["cursor"], 3);
    assert_eq!(
        payload(&followed)["page"]["changes"][0]["mutation"]["mutation"]["mutation"],
        "append_event"
    );

    let timeout = envelope(
        json!({
            "read": {
                "scope": "instance:socket-test",
                "after_cursor": 3,
                "limit": 8
            },
            "wait_timeout_ms": 50
        }),
        None,
        None,
    );
    let (status, timed_out) = post(
        &server,
        "/v1/changes/follow",
        &timeout,
        Some((&session_id, &token)),
    );
    assert_eq!(status, 200, "{timed_out}");
    assert_eq!(payload(&timed_out)["timed_out"], true);
    assert_eq!(payload(&timed_out)["page"]["through_cursor"], 3);
    assert!(payload(&timed_out)["page"]["changes"]
        .as_array()
        .unwrap()
        .is_empty());

    let live = envelope(
        json!({
            "scope": "instance:socket-test",
            "query": "FROM record:document AT VALID 100 KNOWN HEAD PROJECT title",
            "parameters": {},
            "after_cursor": 3,
            "result_identity_version": rrd_core::result_identity::RESULT_IDENTITY_VERSION,
            "budget": {
                "max_storage_keys": 100,
                "max_rows": 10,
                "max_output_bytes": 4096,
                "max_batch_rows": 10
            },
            "max_delta_rows": 10,
            "wait_timeout_ms": 5_000
        }),
        None,
        None,
    );
    let mut old_live = live.clone();
    old_live["payload"]
        .as_object_mut()
        .unwrap()
        .remove("result_identity_version");
    old_live["payload"]["wait_timeout_ms"] = json!(0);
    let (status, refused) = post(
        &server,
        "/v1/query/live/poll",
        &old_live,
        Some((&session_id, &token)),
    );
    assert_eq!(status, 400, "{refused}");
    assert!(refused.to_string().contains("result identity version"));
    let address = server.address;
    let live_body = serde_json::to_vec(&live).unwrap();
    let live_session = session_id.clone();
    let live_token = token.clone();
    let waiting = std::thread::spawn(move || {
        let authorization = format!("Bearer {live_token}");
        http(
            address,
            "POST",
            "/v1/query/live/poll",
            &[
                ("Content-Type", "application/json"),
                ("X-RRD-Session", &live_session),
                ("Authorization", &authorization),
            ],
            &live_body,
        )
    });
    std::thread::sleep(Duration::from_millis(50));
    let begin = envelope(
        json!({"scope": "data", "timeout_ms": 10_000}),
        Some("live-follow-begin"),
        None,
    );
    let (status, began) = post(
        &server,
        "/v1/transactions",
        &begin,
        Some((&session_id, &token)),
    );
    assert_eq!(status, 200, "{began}");
    let transaction = payload(&began)["transaction_id"].as_str().unwrap();
    let mutations = json!([{
        "mutation": "put_record",
        "reference": {"kind": "document", "id": "alpha"},
        "valid_from": 100,
        "properties": {"title": {"type": "string", "value": "Alpha updated"}}
    }]);
    let typed: Vec<TransactionMutation> = serde_json::from_value(mutations.clone()).unwrap();
    let commit = envelope(
        json!({
            "operation_sha256": transaction_operation_sha256(&typed),
            "mutations": mutations
        }),
        Some("live-follow-commit"),
        Some(u64::MAX),
    );
    let (status, committed) = post(
        &server,
        &format!("/v1/transactions/{transaction}/commit"),
        &commit,
        Some((&session_id, &token)),
    );
    assert_eq!(status, 200, "{committed}");
    assert_eq!(payload(&committed)["last_runtime_cursor"], 4);
    let (status, followed) = waiting.join().unwrap();
    assert_eq!(status, 200, "{followed}");
    assert_eq!(payload(&followed)["timed_out"], false);
    assert_eq!(payload(&followed)["through_cursor"], 4);
    assert_eq!(payload(&followed)["updated"].as_array().unwrap().len(), 1);
    assert_eq!(
        payload(&followed)["updated"][0]["after"]["values"]["title"]["value"],
        "Alpha updated"
    );

    let live_timeout = envelope(
        json!({
            "scope": "instance:socket-test",
            "query": "FROM record:document AT VALID 100 KNOWN HEAD PROJECT title",
            "parameters": {},
            "after_cursor": 4,
            "result_identity_version": rrd_core::result_identity::RESULT_IDENTITY_VERSION,
            "budget": {
                "max_storage_keys": 100,
                "max_rows": 10,
                "max_output_bytes": 4096,
                "max_batch_rows": 10
            },
            "max_delta_rows": 10,
            "wait_timeout_ms": 50
        }),
        None,
        None,
    );
    let (status, timed_out) = post(
        &server,
        "/v1/query/live/poll",
        &live_timeout,
        Some((&session_id, &token)),
    );
    assert_eq!(status, 200, "{timed_out}");
    assert_eq!(payload(&timed_out)["timed_out"], true);
    assert_eq!(payload(&timed_out)["through_cursor"], 4);
    assert!(payload(&timed_out)["waited_ms"].as_u64().unwrap() >= 50);
}

#[test]
pub(super) fn managed_backup_and_restore_are_authenticated_replay_safe_and_path_closed() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("instance");
    seed_query_fixture(&root);
    let server = start(&root);
    let create_session = envelope(
        json!({
            "limits": {
                "idle_timeout_ms": 60_000,
                "absolute_timeout_ms": 300_000,
                "max_open_transactions": 2
            }
        }),
        Some("backup-session"),
        None,
    );
    let (status, created) = post(&server, "/v1/sessions", &create_session, None);
    assert_eq!(status, 200, "{created}");
    let session_id = payload(&created)["session_id"].as_str().unwrap().to_owned();
    let token = payload(&created)["token"].as_str().unwrap().to_owned();
    let backup = envelope(
        json!({"label": "before-upgrade", "created_at_unix_ms": 500}),
        Some("backup-create"),
        None,
    );
    let (status, denied) = post(&server, "/v1/backups", &backup, None);
    assert_eq!(status, 401, "{denied}");
    let (status, backed_up) = post(&server, "/v1/backups", &backup, Some((&session_id, &token)));
    assert_eq!(status, 200, "{backed_up}");
    assert_eq!(payload(&backed_up)["idempotent_replay"], false);
    assert_eq!(
        payload(&backed_up)["backup"]["archive"]["runtime_cursor"],
        2
    );
    assert_eq!(payload(&backed_up)["backup"]["object_payloads"], "included");
    assert_eq!(payload(&backed_up)["backup"]["catalogues"], "included");
    assert_eq!(
        payload(&backed_up)["backup"]["installed_control"],
        "included"
    );
    assert_eq!(payload(&backed_up)["backup"]["application_complete"], true);
    let backup_id = payload(&backed_up)["backup"]["backup_sha256"]
        .as_str()
        .unwrap()
        .to_owned();
    let (status, backup_replay) =
        post(&server, "/v1/backups", &backup, Some((&session_id, &token)));
    assert_eq!(status, 200, "{backup_replay}");
    assert_eq!(payload(&backup_replay)["idempotent_replay"], true);
    assert_eq!(
        payload(&backup_replay)["backup"]["backup_sha256"],
        backup_id
    );
    let mut backup_collision = backup.clone();
    backup_collision["payload"]["label"] = json!("different");
    let (status, collision) = post(
        &server,
        "/v1/backups",
        &backup_collision,
        Some((&session_id, &token)),
    );
    assert_eq!(status, 409, "{collision}");

    let list = envelope(json!({"verify_archives": true}), None, None);
    let (status, listed) = post(
        &server,
        "/v1/backups/list",
        &list,
        Some((&session_id, &token)),
    );
    assert_eq!(status, 200, "{listed}");
    assert_eq!(payload(&listed)["archives_verified"], true);
    assert_eq!(payload(&listed)["revision"], 1);
    assert_eq!(payload(&listed)["backups"].as_array().unwrap().len(), 1);
    assert_eq!(payload(&listed)["backups"][0]["backup_sha256"], backup_id);

    let restore = envelope(
        json!({
            "backup_sha256": backup_id,
            "restore_id": "restore-a",
            "restored_at_unix_ms": 600
        }),
        Some("restore-create"),
        None,
    );
    let (status, restored) = post(
        &server,
        "/v1/restores",
        &restore,
        Some((&session_id, &token)),
    );
    assert_eq!(status, 200, "{restored}");
    assert_eq!(payload(&restored)["restore_id"], "restore-a");
    assert_eq!(payload(&restored)["reopened"], true);
    assert_eq!(payload(&restored)["idempotent_replay"], false);
    let (status, restore_replay) = post(
        &server,
        "/v1/restores",
        &restore,
        Some((&session_id, &token)),
    );
    assert_eq!(status, 200, "{restore_replay}");
    assert_eq!(payload(&restore_replay)["idempotent_replay"], true);
    server.stop();

    let restored_root = temporary
        .path()
        .join("rrd-service/socket-test/restores/restore-a");
    assert!(restored_root.join("CURRENT").is_file());
    let restored_engine = RrflowKvStore::open(&restored_root).unwrap();
    assert_eq!(restored_engine.claims().sequence().unwrap(), 0);
    assert_eq!(restored_engine.runtime().cursor().unwrap(), 2);
    drop(restored_engine);

    let server = start(&root);
    let (status, restart_backup) =
        post(&server, "/v1/backups", &backup, Some((&session_id, &token)));
    assert_eq!(status, 200, "{restart_backup}");
    assert_eq!(payload(&restart_backup)["idempotent_replay"], true);
    let (status, restart_restore) = post(
        &server,
        "/v1/restores",
        &restore,
        Some((&session_id, &token)),
    );
    assert_eq!(status, 200, "{restart_restore}");
    assert_eq!(payload(&restart_restore)["idempotent_replay"], true);
}
