//! Remote bind refusal, the standalone daemon and real socket lifecycle.

use super::*;

#[test]
pub(super) fn server_denies_remote_bind_before_opening_a_listener() {
    let temporary = tempfile::tempdir().unwrap();
    let engine = RrdEngine::open(
        &temporary.path().join("instance"),
        CanonicalId::new("socket-test").unwrap(),
        [7; 32],
    )
    .unwrap();
    let result = RrdHttpServer::bind(engine, "0.0.0.0:0".parse().unwrap());
    assert!(matches!(result, Err(HttpError::RemoteBindDenied(_))));
}

#[test]
pub(super) fn binary_refuses_remote_bind() {
    let temporary = tempfile::tempdir().unwrap();
    let project = temporary.path().join("project");
    std::fs::create_dir(&project).unwrap();
    let server_binary = PathBuf::from(env!("CARGO_BIN_EXE_rrd-server"));
    let executable = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let installation = RrdEngine::plan_installation(
        &project,
        InstallationTargetKind::ExistingProject,
        &RrdEngine::generate_installation_identity().unwrap(),
        "default",
        None,
        &executable,
    )
    .unwrap();
    RrdEngine::apply_installation(
        &project,
        InstallationTargetKind::ExistingProject,
        &installation,
        &installation.installation.plan_sha256,
        &executable,
    )
    .unwrap();
    let output = Command::new(&server_binary)
        .args([
            "--project",
            project.to_str().unwrap(),
            "--distribution-executable",
            executable.to_str().unwrap(),
            "--bind",
            "0.0.0.0:0",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("denied without authenticated TLS"),
        "unexpected stderr: {stderr}"
    );
}

#[test]
pub(super) fn standalone_daemon_process_passes_the_shared_corpus_and_exclusively_owns_its_root() {
    let _fixture = acquire_server_fixture();
    let temporary = tempfile::tempdir().unwrap();
    let project = temporary.path().join("project");
    std::fs::create_dir(&project).unwrap();
    let server_binary = PathBuf::from(env!("CARGO_BIN_EXE_rrd-server"));
    let executable = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let installation = RrdEngine::plan_installation(
        &project,
        InstallationTargetKind::ExistingProject,
        &RrdEngine::generate_installation_identity().unwrap(),
        "default",
        None,
        &executable,
    )
    .unwrap();
    RrdEngine::apply_installation(
        &project,
        InstallationTargetKind::ExistingProject,
        &installation,
        &installation.installation.plan_sha256,
        &executable,
    )
    .unwrap();
    let instance = installation.installation.target.instance_id.clone();
    let scope = format!("instance:{instance}");
    let root = project.join(
        &installation
            .installation
            .managed_path(InstallationManagedPathKind::StorageRoot)
            .relative_path,
    );
    let api_key = RrdEngine::read_installed_api_key(&project).unwrap();
    extend_installed_schema_for_deployment_corpus(&root, &scope);
    let installed_envelope = |payload, idempotency_key, deadline| {
        let mut value = envelope(payload, idempotency_key, deadline);
        value["resource"]["segments"][0]["id"] = json!(instance.as_str());
        value
    };

    let process = start_installed_process(
        &server_binary,
        &project,
        &executable,
        temporary.path(),
        "first",
    );

    assert!(RrflowKvStore::open(&root).is_err());
    let (status, capabilities) = http(process.address, "GET", "/v1/capabilities", &[], &[]);
    assert_eq!(status, 200);
    assert_eq!(
        payload(&capabilities)["deployment"]["deployment_form"],
        "single_node_server"
    );
    assert_eq!(
        payload(&capabilities)["deployment"]["storage_profile"],
        "rrflow_kv"
    );
    assert_eq!(
        payload(&capabilities)["deployment"]["endpoint_presentation"],
        "loopback_http_websocket"
    );
    assert_eq!(
        payload(&capabilities)["installed_estate"]["instance_id"],
        instance.as_str()
    );

    let create = installed_envelope(
        json!({
            "limits": {
                "idle_timeout_ms": 60_000,
                "absolute_timeout_ms": 300_000,
                "max_open_transactions": 1
            }
        }),
        Some("deployment-process-session"),
        None,
    );
    let create_bytes = serde_json::to_vec(&create).unwrap();
    let (status, created) = http(
        process.address,
        "POST",
        "/v1/sessions",
        &[
            ("Content-Type", "application/json"),
            ("X-RRD-Principal", api_key.principal_id.as_str()),
            ("Authorization", &format!("ApiKey {}", api_key.credential())),
        ],
        &create_bytes,
    );
    assert_eq!(status, 200, "{created}");
    let session = payload(&created)["session_id"].as_str().unwrap();
    let token = payload(&created)["token"].as_str().unwrap();
    let authorization = format!("Bearer {token}");
    let begin = installed_envelope(
        json!({"scope": "data", "timeout_ms": 10_000}),
        Some("deployment-process-begin"),
        None,
    );
    let begin_bytes = serde_json::to_vec(&begin).unwrap();
    let (status, began) = http(
        process.address,
        "POST",
        "/v1/transactions",
        &[
            ("Content-Type", "application/json"),
            ("X-RRD-Session", session),
            ("Authorization", &authorization),
        ],
        &begin_bytes,
    );
    assert_eq!(status, 200, "{began}");
    let transaction = payload(&began)["transaction_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let mut mutations = deployment_mutations()
        .into_iter()
        .skip(1)
        .collect::<Vec<_>>();
    mutations.push(
        serde_json::from_value(json!({
            "mutation": "put_relation",
            "reference": {"kind": "deployment_link", "id": "alpha-gamma"},
            "from": {"kind": "document", "id": "alpha"},
            "to": {"kind": "document", "id": "gamma"},
            "valid_from": 100,
            "properties": {}
        }))
        .unwrap(),
    );
    let operation_sha256 = transaction_operation_sha256(&mutations);
    let commit = installed_envelope(
        json!({
            "operation_sha256": operation_sha256,
            "mutations": mutations
        }),
        Some("deployment-process-commit"),
        Some(u64::MAX),
    );
    let commit_bytes = serde_json::to_vec(&commit).unwrap();
    let (status, committed) = http(
        process.address,
        "POST",
        &format!("/v1/transactions/{transaction}/commit"),
        &[
            ("Content-Type", "application/json"),
            ("X-RRD-Session", session),
            ("Authorization", &authorization),
        ],
        &commit_bytes,
    );
    assert_eq!(status, 200, "{committed}");
    assert_eq!(payload(&committed)["first_runtime_cursor"], 4);
    assert_eq!(payload(&committed)["last_runtime_cursor"], 6);
    assert_eq!(payload(&committed)["mutation_count"], 3);

    let corpus = deployment_corpus();
    let query = installed_envelope(
        json!({
            "scope": scope,
            "query": corpus.query.rrflowql,
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
    let query_bytes = serde_json::to_vec(&query).unwrap();
    let (status, result) = http(
        process.address,
        "POST",
        "/v1/query",
        &[
            ("Content-Type", "application/json"),
            ("X-RRD-Session", session),
            ("Authorization", &authorization),
        ],
        &query_bytes,
    );
    assert_eq!(status, 200, "{result}");
    let actual = payload(&result)["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            row["identity"]
                .as_str()
                .unwrap()
                .strip_prefix("record:document:")
                .unwrap()
        })
        .collect::<Vec<_>>();
    let expected = corpus
        .expected_ids
        .iter()
        .map(CanonicalId::as_str)
        .collect::<Vec<_>>();
    assert_eq!(actual, expected);
    assert_eq!(
        payload(&result)["rows"][0]["values"]["body"]["value"],
        corpus.documents[0].text
    );
    assert!(
        payload(&result)["execution"]["selected_versions"]
            .as_u64()
            .unwrap()
            > 0
    );
    assert_direct_read_evidence(
        &payload(&result)["execution"]["read_evidence"],
        ReadAccessPath::RecordVersions,
    );

    let relationship_query = installed_envelope(
        json!({
            "scope": scope,
            "query": "SELECT left.id, right.to_id FROM record:document JOIN relation:deployment_link ON id = from_id AT VALID 100 KNOWN HEAD WHERE left.id = \"alpha\"",
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
    let (status, relationship) = http(
        process.address,
        "POST",
        "/v1/query",
        &[
            ("Content-Type", "application/json"),
            ("X-RRD-Session", session),
            ("Authorization", &authorization),
        ],
        &serde_json::to_vec(&relationship_query).unwrap(),
    );
    assert_eq!(status, 200, "{relationship}");
    let relationship = payload(&relationship);
    assert_eq!(relationship["known_at_cursor"], 6);
    assert_eq!(relationship["rows"].as_array().unwrap().len(), 1);
    assert_eq!(
        relationship["rows"][0]["values"]["left.id"]["value"],
        "alpha"
    );
    assert_eq!(
        relationship["rows"][0]["values"]["right.to_id"]["value"],
        "gamma"
    );
    let expected_relationship_rows = relationship["rows"].clone();

    process.stop();
    let reopened = RrflowKvStore::open(&root).unwrap();
    assert_eq!(reopened.runtime().cursor().unwrap(), 6);
    drop(reopened);
    let process = start_installed_process(
        &server_binary,
        &project,
        &executable,
        temporary.path(),
        "restarted",
    );
    let (status, restored_relationship) = http(
        process.address,
        "POST",
        "/v1/query",
        &[
            ("Content-Type", "application/json"),
            ("X-RRD-Session", session),
            ("Authorization", &authorization),
        ],
        &serde_json::to_vec(&relationship_query).unwrap(),
    );
    assert_eq!(status, 200, "{restored_relationship}");
    assert_eq!(payload(&restored_relationship)["known_at_cursor"], 6);
    assert_eq!(
        payload(&restored_relationship)["rows"],
        expected_relationship_rows
    );
    process.stop();
}

#[test]
pub(super) fn real_socket_exercises_lifecycle_commit_and_restart_replay() {
    let (_temporary, root, server) = start_root();
    let expected_catalogue = rrd_contract::endpoint_catalogue();
    let (status, live) = http(server.address, "GET", "/v1/health/live", &[], &[]);
    assert_eq!(status, 200);
    assert_eq!(live["outcome"]["status"], "ok");
    let (status, capabilities) = http(server.address, "GET", "/v1/capabilities", &[], &[]);
    assert_eq!(status, 200);
    assert_eq!(
        payload(&capabilities)["deployment"]["deployment_form"],
        "single_node_server"
    );
    assert_eq!(
        payload(&capabilities)["deployment"]["storage_profile"],
        "rrflow_kv"
    );
    assert_eq!(
        payload(&capabilities)["deployment"]["endpoint_presentation"],
        "loopback_http_websocket"
    );
    assert_eq!(
        payload(&capabilities)["configuration"]["reasoning"]["max_run_elapsed_ms"],
        900_000
    );
    assert!(payload(&capabilities)["capabilities"]
        .as_array()
        .unwrap()
        .iter()
        .any(|capability| {
            capability["name"] == "remote-listen" && capability["status"] == "unavailable"
        }));
    let (status, catalogue) = http(server.address, "GET", "/v1/schema/endpoints", &[], &[]);
    assert_eq!(status, 200, "{catalogue}");
    assert_eq!(payload(&catalogue)["protocol_version"], 1);
    assert_eq!(
        payload(&catalogue)["endpoints"].as_array().unwrap().len(),
        expected_catalogue.endpoints.len()
    );
    assert_eq!(
        payload(&catalogue)["websocket_endpoints"]
            .as_array()
            .unwrap()
            .len(),
        expected_catalogue.websocket_endpoints.len()
    );
    let (status, openapi) = http(server.address, "GET", "/v1/schema/openapi", &[], &[]);
    assert_eq!(status, 200, "{openapi}");
    assert_eq!(payload(&openapi)["openapi"], "3.1.0");
    assert_eq!(
        payload(&openapi)["x-rrd-endpoint-count"],
        expected_catalogue.endpoints.len()
    );
    assert_eq!(
        payload(&openapi)["x-rrd-websocket-endpoint-count"],
        expected_catalogue.websocket_endpoints.len()
    );
    assert!(
        payload(&openapi)["paths"]["/v1/query"]["post"]["requestBody"]["content"]
            ["application/json"]["schema"]
            .is_object()
    );
    assert!(
        payload(&openapi)["paths"]["/v1/query/live/poll"]["post"]["requestBody"]["content"]
            ["application/json"]["schema"]
            .is_object()
    );
    assert!(
        payload(&openapi)["paths"]["/v1/query/indexes/ensure"]["post"]["requestBody"]["content"]
            ["application/json"]["schema"]
            .is_object()
    );

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
    let (status, created) = post(&server, "/v1/sessions", &create, None);
    assert_eq!(status, 200);
    let session_id = payload(&created)["session_id"].as_str().unwrap().to_owned();
    let token = payload(&created)["token"].as_str().unwrap().to_owned();
    let (status, create_replay) = post(&server, "/v1/sessions", &create, None);
    assert_eq!(status, 200);
    assert_eq!(payload(&create_replay), payload(&created));

    let begin = envelope(
        json!({"scope": "claims", "timeout_ms": 60_000}),
        Some("begin-key"),
        None,
    );
    let (status, began) = post(
        &server,
        "/v1/transactions",
        &begin,
        Some((&session_id, &token)),
    );
    assert_eq!(status, 200);
    let transaction_id = payload(&began)["transaction_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let mutations = json!([{
        "mutation": "assert_claim",
        "subject": "document-1",
        "predicate": "contains",
        "object": "socket verified",
        "valid_from": 1,
        "tx_time": 1,
        "producer": "socket-test",
        "confidence": 0.95,
    }]);
    let preview = envelope(
        json!({"mutations": mutations.clone()}),
        Some("prepare-claims"),
        None,
    );
    let (status, previewed) = post(
        &server,
        &format!("/v1/transactions/{transaction_id}/preview"),
        &preview,
        Some((&session_id, &token)),
    );
    assert_eq!(status, 200);
    assert_eq!(payload(&previewed)["mutations"], mutations);

    let typed_mutations: Vec<TransactionMutation> =
        serde_json::from_value(mutations.clone()).unwrap();
    let operation_sha256 = transaction_operation_sha256(&typed_mutations);
    let commit = envelope(
        json!({
            "operation_sha256": operation_sha256,
            "mutations": mutations,
        }),
        Some("commit-key"),
        Some(u64::MAX),
    );
    let commit_path = format!("/v1/transactions/{transaction_id}/commit");
    let mut missing_deadline = commit.clone();
    missing_deadline["context"]["deadline_unix_ms"] = Value::Null;
    let (status, denied) = post(
        &server,
        &commit_path,
        &missing_deadline,
        Some((&session_id, &token)),
    );
    assert_eq!(status, 400);
    assert_eq!(denied["outcome"]["error"]["code"], "invalid_argument");
    let (status, committed) = post(&server, &commit_path, &commit, Some((&session_id, &token)));
    assert_eq!(status, 200, "{committed}");
    assert_eq!(payload(&committed)["idempotent_replay"], false);
    let (status, replay) = post(&server, &commit_path, &commit, Some((&session_id, &token)));
    assert_eq!(status, 200);
    assert_eq!(payload(&replay)["idempotent_replay"], true);
    server.stop();

    let server = start(&root);
    let (status, ready) = http(server.address, "GET", "/v1/health/ready", &[], &[]);
    assert_eq!(status, 200);
    assert_eq!(payload(&ready)["claim_sequence"], 1);
    let (status, restart_replay) =
        post(&server, &commit_path, &commit, Some((&session_id, &token)));
    assert_eq!(status, 200);
    assert_eq!(payload(&restart_replay)["idempotent_replay"], true);

    let mut collision = commit.clone();
    collision["context"]["idempotency_key"] = json!("different-key");
    let (status, conflict) = post(
        &server,
        &commit_path,
        &collision,
        Some((&session_id, &token)),
    );
    assert_eq!(status, 409);
    assert_eq!(conflict["outcome"]["error"]["code"], "conflict");
}
