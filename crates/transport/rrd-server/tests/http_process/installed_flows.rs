//! Installed attunement replans, endpoint configuration and quantization over HTTP.

use super::*;

#[test]
pub(super) fn installed_attunement_replan_over_http_survives_restart_and_replay() {
    let temporary = tempfile::tempdir().unwrap();
    let project = temporary.path().join("project");
    std::fs::create_dir(&project).unwrap();
    std::fs::write(project.join("README.md"), "initial source\n").unwrap();
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
    let job = format!("job-{}", &installation.attunement.plan_sha256[..24]);
    let key = RrdEngine::read_installed_api_key(&project).unwrap();
    let make = |payload, operation: &str, mutation: bool| {
        let mut value = envelope(payload, mutation.then_some(operation), None);
        value["resource"]["segments"][0]["id"] = json!(instance.as_str());
        value["context"]["request_id"] = json!(format!("request-{operation}"));
        value["context"]["operation_id"] = json!(format!("operation-{operation}"));
        value
    };
    std::fs::write(project.join("README.md"), "ordinary edit\n").unwrap();
    let mut accepted_request = Value::Null;
    let mut successor = Value::Null;
    let mut expired = Value::Null;
    for pass in 0..3 {
        let engine = RrdEngine::open_installed(&project, &executable).unwrap();
        let fixture = acquire_server_fixture();
        let server = RrdHttpServer::bind(engine, "127.0.0.1:0".parse().unwrap()).unwrap();
        let address = server.local_addr();
        let (shutdown, receiver) = tokio::sync::oneshot::channel();
        let thread = std::thread::spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(server.serve_until(async move {
                    let _ = receiver.await;
                }))
        });
        let server = RunningServer {
            address,
            shutdown: Some(shutdown),
            thread: Some(thread),
            _fixture: fixture,
        };
        let create = make(
            json!({"limits": {"idle_timeout_ms": 60000, "absolute_timeout_ms": 300000, "max_open_transactions": 1}}),
            &format!("replan-session-{pass}"),
            true,
        );
        let (status, created) = post_with_api_key(
            &server,
            "/v1/sessions",
            &create,
            key.principal_id.as_str(),
            key.credential(),
        );
        assert_eq!(status, 200, "{created}");
        let session = (
            payload(&created)["session_id"].as_str().unwrap(),
            payload(&created)["token"].as_str().unwrap(),
        );
        if pass == 0 {
            let start = make(
                json!({"contract_version": rrd_contract::ATTUNEMENT_CONTRACT_VERSION, "job_id": job, "plan_sha256": installation.attunement.plan_sha256, "expected_revision": 1, "lease_ms": 60000}),
                "stale-start",
                true,
            );
            let (status, stale) = post(&server, "/v1/attunement/start", &start, Some(session));
            assert_eq!(status, 412, "{stale}");
            assert_eq!(stale["outcome"]["error"]["code"], "failed_precondition");
            assert_eq!(
                stale["outcome"]["error"]["details"]["reason"],
                "attunement-source-changed"
            );
            assert_eq!(
                stale["outcome"]["error"]["details"]["next-action"],
                "preview-attunement-replan"
            );
            let read = make(
                json!({"contract_version": rrd_contract::ATTUNEMENT_CONTRACT_VERSION, "job_id": job}),
                "old-read",
                false,
            );
            let (_, old) = post(&server, "/v1/attunement/read", &read, Some(session));
            assert_eq!(payload(&old)["status"]["job"]["revision"], 1);
            let preview_request = make(
                json!({"contract_version": rrd_contract::ATTUNEMENT_CONTRACT_VERSION, "job_id": job, "expected_revision": 1}),
                "replan-preview",
                false,
            );
            let (status, denied) = post(
                &server,
                "/v1/attunement/replan/preview",
                &preview_request,
                None,
            );
            assert_eq!(status, 401, "{denied}");
            let (status, preview) = post(
                &server,
                "/v1/attunement/replan/preview",
                &preview_request,
                Some(session),
            );
            assert_eq!(status, 200, "{preview}");
            assert_ne!(
                payload(&preview)["plan"]["source_sha256"],
                installation.attunement.source_sha256
            );
            let obsolete = make(
                json!({"contract_version": rrd_contract::ATTUNEMENT_CONTRACT_VERSION, "preview": payload(&preview), "proposal_sha256": payload(&preview)["proposal_sha256"]}),
                "obsolete-replan",
                true,
            );
            std::fs::write(project.join("README.md"), "another ordinary edit\n").unwrap();
            let (status, stale_preview) = post(
                &server,
                "/v1/attunement/replan/accept",
                &obsolete,
                Some(session),
            );
            assert_eq!(status, 412, "{stale_preview}");
            let (status, preview) = post(
                &server,
                "/v1/attunement/replan/preview",
                &preview_request,
                Some(session),
            );
            assert_eq!(status, 200, "{preview}");
            accepted_request = make(
                json!({"contract_version": rrd_contract::ATTUNEMENT_CONTRACT_VERSION, "preview": payload(&preview), "proposal_sha256": payload(&preview)["proposal_sha256"]}),
                "accept-replan",
                true,
            );
            let (status, accepted) = post(
                &server,
                "/v1/attunement/replan/accept",
                &accepted_request,
                Some(session),
            );
            assert_eq!(status, 200, "{accepted}");
            assert_eq!(payload(&accepted)["idempotent_replay"], false);
            successor = payload(&accepted)["successor"]["job"].clone();
            let mut changed_replay = accepted_request.clone();
            changed_replay["payload"]["preview"]["generation"] = json!(99);
            let (status, conflict) = post(
                &server,
                "/v1/attunement/replan/accept",
                &changed_replay,
                Some(session),
            );
            assert_eq!(status, 409, "{conflict}");
        } else if pass == 1 {
            let (status, replay) = post(
                &server,
                "/v1/attunement/replan/accept",
                &accepted_request,
                Some(session),
            );
            assert_eq!(status, 200, "{replay}");
            assert_eq!(payload(&replay)["idempotent_replay"], true);
            assert_eq!(payload(&replay)["successor"]["job"], successor);
            let receipt_request = make(
                json!({"contract_version": rrd_contract::ATTUNEMENT_CONTRACT_VERSION, "job_id": job, "idempotency_key": "accept-replan"}),
                "receipt-read",
                false,
            );
            let (status, receipt) = post(
                &server,
                "/v1/attunement/replan/receipt/read",
                &receipt_request,
                Some(session),
            );
            assert_eq!(status, 200, "{receipt}");
            assert_eq!(
                payload(&receipt)["receipt"]["result"]["successor"]["job"],
                successor
            );
            let start = make(
                json!({"contract_version": rrd_contract::ATTUNEMENT_CONTRACT_VERSION, "job_id": successor["id"], "plan_sha256": successor["plan_sha256"], "expected_revision": successor["revision"], "lease_ms": 60000}),
                "successor-start",
                true,
            );
            let (status, started) = post(&server, "/v1/attunement/start", &start, Some(session));
            assert_eq!(status, 200, "{started}");
            let running = &payload(&started)["status"]["job"];
            let pause = make(
                json!({"contract_version": rrd_contract::ATTUNEMENT_CONTRACT_VERSION, "job_id": running["id"], "expected_revision": running["revision"], "lease_generation": running["lease"]["generation"]}),
                "pause",
                true,
            );
            let (status, paused) = post(&server, "/v1/attunement/pause", &pause, Some(session));
            assert_eq!(status, 200, "{paused}");
            let paused_job = &payload(&paused)["status"]["job"];
            assert_eq!(paused_job["state"], "paused");
            let resume = make(
                json!({"request": {"contract_version": rrd_contract::ATTUNEMENT_CONTRACT_VERSION, "job_id": paused_job["id"], "plan_sha256": paused_job["plan_sha256"], "expected_job_revision": paused_job["revision"], "last_checkpoint_sha256": null, "resume_from_phase": "connect", "requested_at_unix_ms": paused_job["updated_at_unix_ms"]}, "lease_ms": 2000}),
                "resume-short",
                true,
            );
            let (status, resumed) = post(&server, "/v1/attunement/resume", &resume, Some(session));
            assert_eq!(status, 200, "{resumed}");
            expired = payload(&resumed)["status"]["job"].clone();
        } else {
            let recover = make(
                json!({"contract_version": rrd_contract::ATTUNEMENT_CONTRACT_VERSION, "job_id": expired["id"], "expected_revision": expired["revision"]}),
                "recover-expired",
                true,
            );
            let (status, recovered) = post(
                &server,
                "/v1/attunement/recover-expired",
                &recover,
                Some(session),
            );
            assert_eq!(status, 200, "{recovered}");
            let recovered_job = &payload(&recovered)["status"]["job"];
            assert_eq!(recovered_job["state"], "paused");
            let resume = make(
                json!({"request": {"contract_version": rrd_contract::ATTUNEMENT_CONTRACT_VERSION, "job_id": recovered_job["id"], "plan_sha256": recovered_job["plan_sha256"], "expected_job_revision": recovered_job["revision"], "last_checkpoint_sha256": null, "resume_from_phase": "connect", "requested_at_unix_ms": recovered_job["updated_at_unix_ms"]}, "lease_ms": 60000}),
                "resume-recovered",
                true,
            );
            let (status, resumed) = post(&server, "/v1/attunement/resume", &resume, Some(session));
            assert_eq!(status, 200, "{resumed}");
            let running = &payload(&resumed)["status"]["job"];
            let step = make(
                json!({"contract_version": rrd_contract::ATTUNEMENT_CONTRACT_VERSION, "job_id": running["id"], "expected_revision": running["revision"], "lease_generation": running["lease"]["generation"]}),
                "successor-step",
                true,
            );
            let (status, advanced) = post(&server, "/v1/attunement/step", &step, Some(session));
            assert_eq!(status, 200, "{advanced}");
            assert_eq!(
                payload(&advanced)["status"]["job"]["checkpoints"]
                    .as_array()
                    .unwrap()
                    .len(),
                1
            );
        }
        server.stop();
        if pass == 1 {
            std::thread::sleep(Duration::from_millis(2100));
        }
    }
}

#[test]
pub(super) fn installed_endpoint_configuration_http_flow_survives_restart_and_revocation() {
    let temporary = tempfile::tempdir().unwrap();
    let project = temporary.path().join("endpoint-project");
    std::fs::create_dir(&project).unwrap();
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
    let key = RrdEngine::read_installed_api_key(&project).unwrap();
    let make_envelope = |payload, operation: &str, mutation: bool| {
        let mut value = envelope(payload, mutation.then_some(operation), None);
        value["resource"]["segments"][0]["id"] = json!(instance.as_str());
        value["context"]["request_id"] = json!(format!("request-{operation}"));
        value["context"]["operation_id"] = json!(format!("operation-{operation}"));
        value
    };
    let mut first_apply = Value::Null;
    for pass in 0..2 {
        let engine = RrdEngine::open_installed(&project, &executable).unwrap();
        let fixture = acquire_server_fixture();
        let server = RrdHttpServer::bind(engine, "127.0.0.1:0".parse().unwrap()).unwrap();
        let address = server.local_addr();
        let (shutdown, receiver) = tokio::sync::oneshot::channel();
        let thread = std::thread::spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(server.serve_until(async move {
                    let _ = receiver.await;
                }))
        });
        let server = RunningServer {
            address,
            shutdown: Some(shutdown),
            thread: Some(thread),
            _fixture: fixture,
        };
        let create = make_envelope(
            json!({"limits": {"idle_timeout_ms": 60_000, "absolute_timeout_ms": 300_000, "max_open_transactions": 1}}),
            &format!("configuration-session-{pass}"),
            true,
        );
        let (status, created) = post_with_api_key(
            &server,
            "/v1/sessions",
            &create,
            key.principal_id.as_str(),
            key.credential(),
        );
        assert_eq!(status, 200, "{created}");
        let session = (
            payload(&created)["session_id"].as_str().unwrap(),
            payload(&created)["token"].as_str().unwrap(),
        );
        let read_request = make_envelope(json!({}), "configuration-read", false);
        let (status, before) = post(
            &server,
            "/v1/configuration/endpoints/read",
            &read_request,
            Some(session),
        );
        assert_eq!(status, 200, "{before}");
        assert_eq!(payload(&before)["revision"], pass);
        let desired = if pass == 0 {
            json!({"contract_version": 1, "name": "project-knowledge", "bindings": [{
                "id": "vector-source", "class": "vector_store", "kind": "api",
                "endpoint": {"kind": "url", "url": "https://unreachable.invalid/"},
                "service_id": "unqualified-service", "manifest_sha256": null,
                "credential": {"provider": "unconfigured-provider", "name": "read-key"},
                "placement": null, "footprint": {"memory_bytes": 1, "gpu": {"kind": "none"}},
                "limits": {"max_concurrency": 1, "timeout_ms": 1000, "max_input_bytes": 4096}
            }]})
        } else {
            Value::Null
        };
        let preview_request = make_envelope(
            json!({"expected_revision": pass, "profile": desired}),
            "configuration-preview",
            false,
        );
        let (denied_status, _) = post(
            &server,
            "/v1/configuration/endpoints/preview",
            &preview_request,
            None,
        );
        assert_eq!(denied_status, 401);
        let (status, preview) = post(
            &server,
            "/v1/configuration/endpoints/preview",
            &preview_request,
            Some(session),
        );
        assert_eq!(status, 200, "{preview}");
        let (_, unchanged) = post(
            &server,
            "/v1/configuration/endpoints/read",
            &read_request,
            Some(session),
        );
        assert_eq!(payload(&unchanged), payload(&before));
        let apply_request = make_envelope(
            json!({"plan": payload(&preview), "approved_plan_sha256": payload(&preview)["plan_sha256"]}),
            &format!("configuration-apply-{pass}"),
            true,
        );
        let (status, applied) = post(
            &server,
            "/v1/configuration/endpoints/apply",
            &apply_request,
            Some(session),
        );
        assert_eq!(status, 200, "{applied}");
        assert_eq!(payload(&applied)["revision"], pass + 1);
        let (status, replay) = post(
            &server,
            "/v1/configuration/endpoints/apply",
            &apply_request,
            Some(session),
        );
        assert_eq!(status, 200, "{replay}");
        assert_eq!(payload(&replay), payload(&applied));
        if pass == 0 {
            first_apply = apply_request;
        } else {
            assert!(payload(&applied)["profile"].is_null());
            first_apply["context"] =
                make_envelope(json!({}), "stale-configuration-apply", true)["context"].clone();
            let (status, stale) = post(
                &server,
                "/v1/configuration/endpoints/apply",
                &first_apply,
                Some(session),
            );
            assert_eq!(status, 409, "{stale}");
        }
        server.stop();
    }
    let reopened = RrdEngine::open_installed(&project, &executable).unwrap();
    assert_eq!(
        reopened.installed_estate_identity(),
        Some(&installation.installation.target)
    );
}

#[test]
pub(super) fn quantization_lifecycle_serves_turboquant_and_replays_over_http() {
    let (_temporary, _root, server) = start_root();
    let create = envelope(
        json!({
            "limits": {
                "idle_timeout_ms": 60_000,
                "absolute_timeout_ms": 300_000,
                "max_open_transactions": 2
            }
        }),
        Some("quantization-session"),
        None,
    );
    let (status, created) = post(&server, "/v1/sessions", &create, None);
    assert_eq!(status, 200, "{created}");
    let session_id = payload(&created)["session_id"].as_str().unwrap().to_owned();
    let token = payload(&created)["token"].as_str().unwrap().to_owned();
    let auth = Some((session_id.as_str(), token.as_str()));
    let ensure = envelope(
        json!({
            "scope": "instance:socket-test",
            "collection_id": "documents",
            "vectors": [{
                "name": "title",
                "field": "title_embedding",
                "kind": "dense",
                "dimensions": 2,
                "metric": "cosine",
                "memory_tier": "cached"
            }]
        }),
        Some("quantization-collection"),
        None,
    );
    let (status, ensured) = post(&server, "/v1/vector/collections/ensure", &ensure, auth);
    assert_eq!(status, 200, "{ensured}");
    let begin = envelope(
        json!({"scope": "data", "timeout_ms": 60_000}),
        Some("quantization-begin"),
        None,
    );
    let (status, began) = post(&server, "/v1/transactions", &begin, auth);
    assert_eq!(status, 200, "{began}");
    let transaction_id = payload(&began)["transaction_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let mut mutations = vec![json!({
        "mutation": "put_schema",
        "registry": {
            "revision": 1,
            "migration": "quantization fixture",
            "tables": {
                "document": {"model": "relational", "mode": "strict"},
                "embedding": {"model": "vector", "mode": "schemaless"}
            },
            "records": {
                "document": {
                    "properties": {},
                    "allow_additional_properties": true,
                    "unique_properties": []
                }
            },
            "relations": {},
            "events": {}
        }
    })];
    for (id, values) in [
        ("alpha", [0.6, 0.8]),
        ("beta", [1.0, 0.0]),
        ("gamma", [0.0, 1.0]),
    ] {
        mutations.push(json!({
            "mutation": "put_record",
            "reference": {"kind": "document", "id": id},
            "valid_from": 100,
            "properties": {}
        }));
        mutations.push(json!({
            "mutation": "put_vector",
            "reference": {"kind": "embedding", "id": format!("{id}-title")},
            "subject": {"kind": "document", "id": id},
            "collection_id": "documents",
            "vector_name": "title",
            "field": "title_embedding",
            "valid_from": 100,
            "value": {"kind": "dense", "values": values},
            "properties": {}
        }));
    }
    let typed: Vec<TransactionMutation> =
        serde_json::from_value(Value::Array(mutations.clone())).unwrap();
    let commit = envelope(
        json!({
            "operation_sha256": transaction_operation_sha256(&typed),
            "mutations": mutations
        }),
        Some("quantization-commit"),
        Some(u64::MAX),
    );
    let (status, committed) = post(
        &server,
        &format!("/v1/transactions/{transaction_id}/commit"),
        &commit,
        auth,
    );
    assert_eq!(status, 200, "{committed}");

    let build_request = json!({
        "scope": "instance:socket-test",
        "collection_id": "documents",
        "vector_name": "title",
        "method": {"method": "turbo_quant", "bits": "bits4", "seed": 7},
        "max_storage_keys": 1_000
    });
    let (status, unkeyed) = post(
        &server,
        "/v1/vector/quantizations/build",
        &envelope(build_request.clone(), None, None),
        auth,
    );
    assert_eq!(status, 400, "{unkeyed}");
    let build = envelope(build_request.clone(), Some("build-title"), None);
    let (status, built) = post(&server, "/v1/vector/quantizations/build", &build, auth);
    assert_eq!(status, 200, "{built}");
    let artifact = payload(&built)["artifact"].clone();
    assert_eq!(payload(&built)["idempotent_replay"], false);
    assert_eq!(artifact["artifact_id"], "quant-turboquant-documents-title");
    assert_eq!(artifact["generation"], 1);
    assert_eq!(artifact["state"], "ready");
    let (status, replayed) = post(&server, "/v1/vector/quantizations/build", &build, auth);
    assert_eq!(status, 200, "{replayed}");
    assert_eq!(payload(&replayed)["idempotent_replay"], true);
    assert_eq!(payload(&replayed)["artifact"], artifact);
    let mut changed = build_request.clone();
    changed["method"]["seed"] = json!(8);
    let (status, conflict) = post(
        &server,
        "/v1/vector/quantizations/build",
        &envelope(changed, Some("build-title"), None),
        auth,
    );
    assert_eq!(status, 409, "{conflict}");

    let list = envelope(
        json!({"scope": "instance:socket-test", "max_artifacts": 10}),
        None,
        None,
    );
    let (status, listed) = post(&server, "/v1/vector/quantizations/list", &list, auth);
    assert_eq!(status, 200, "{listed}");
    assert_eq!(payload(&listed)["artifacts"].as_array().unwrap().len(), 1);
    assert_eq!(payload(&listed)["artifacts"][0]["state"], "ready");

    let search = envelope(
        json!({
            "scope": "instance:socket-test",
            "valid_at": 200,
            "collection_id": "documents",
            "vector_name": "title",
            "query": {"kind": "dense", "values": [0.6, 0.8]},
            "top_k": 1,
            "mode": {"mode": "require_approximate", "exact_rerank": 3, "ef_search": 3},
            "max_storage_keys": 1_000
        }),
        None,
        None,
    );
    let (status, refused) = post(&server, "/v1/vector/search", &search, auth);
    assert_eq!(status, 400, "a ready generation does not serve: {refused}");
    let generation = json!({
        "scope": "instance:socket-test",
        "artifact_id": "quant-turboquant-documents-title",
        "generation": 1
    });
    let (status, activated) = post(
        &server,
        "/v1/vector/quantizations/activate",
        &envelope(generation.clone(), Some("activate-title"), None),
        auth,
    );
    assert_eq!(status, 200, "{activated}");
    assert_eq!(payload(&activated)["artifact"]["state"], "active");
    let (status, searched) = post(&server, "/v1/vector/search", &search, auth);
    assert_eq!(status, 200, "{searched}");
    assert_eq!(payload(&searched)["access_path"], "turboquant");
    assert_eq!(payload(&searched)["exact"], false);
    assert_eq!(
        payload(&searched)["hits"][0]["reference"]["id"],
        "alpha-title"
    );
    let (status, retired) = post(
        &server,
        "/v1/vector/quantizations/retire",
        &envelope(generation, Some("retire-title"), None),
        auth,
    );
    assert_eq!(status, 200, "{retired}");
    assert_eq!(payload(&retired)["artifact"]["state"], "retired");
    let (status, refused) = post(&server, "/v1/vector/search", &search, auth);
    assert_eq!(
        status, 400,
        "a retired generation does not serve: {refused}"
    );
}
