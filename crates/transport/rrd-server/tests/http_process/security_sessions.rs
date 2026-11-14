//! Security authority initialization, sessions and JWT exchange.

use super::*;

#[test]
pub(super) fn initialized_security_authority_binds_sessions_and_denies_ungranted_routes() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("instance");
    seed_query_fixture(&root);
    let engine = RrflowKvStore::open(&root).unwrap();
    let instance = CanonicalId::new("socket-test").unwrap();
    let resource = rrd_contract::ResourcePath {
        segments: vec![rrd_contract::ResourceId::new(
            rrd_contract::ResourceKind::Instance,
            "socket-test",
        )
        .unwrap()],
    };
    let principal = Principal {
        id: CanonicalId::new("connectome-local").unwrap(),
        kind: PrincipalKind::User,
        credential_sha256: rrd_core::digest::sha256_hex(b"local-api-key"),
        credential_revision: 1,
        not_before_unix_ms: 1,
        expires_at_unix_ms: u64::MAX,
        disabled: false,
        role_ids: Default::default(),
        grants: vec![
            ResourceGrant {
                action: SecurityAction::SessionCreate,
                resource_prefix: resource.clone(),
                data_policy: None,
            },
            ResourceGrant {
                action: SecurityAction::QueryExecute,
                resource_prefix: resource,
                data_policy: None,
            },
            ResourceGrant {
                action: SecurityAction::AuditRead,
                resource_prefix: rrd_contract::ResourcePath {
                    segments: vec![rrd_contract::ResourceId::new(
                        rrd_contract::ResourceKind::Instance,
                        "socket-test",
                    )
                    .unwrap()],
                },
                data_policy: None,
            },
            ResourceGrant {
                action: SecurityAction::AuditExport,
                resource_prefix: rrd_contract::ResourcePath {
                    segments: vec![rrd_contract::ResourceId::new(
                        rrd_contract::ResourceKind::Instance,
                        "socket-test",
                    )
                    .unwrap()],
                },
                data_policy: None,
            },
        ],
    };
    SecurityRepository::new(&engine, instance)
        .initialize(
            SecurityState {
                format_version: SECURITY_FORMAT,
                revision: 1,
                principals: BTreeMap::from([(principal.id.clone(), principal)]),
                roles: BTreeMap::new(),
                identity_bindings: BTreeMap::new(),
                jwt_issuers: BTreeMap::new(),
            },
            1,
            "bootstrap",
            "request-bootstrap",
            "operation-bootstrap",
        )
        .unwrap();
    drop(engine);

    let server = start(&root);
    let (status, _) = http(server.address, "GET", "/v1/health/live", &[], &[]);
    assert_eq!(status, 200);
    let (status, _) = http(server.address, "GET", "/v1/not-a-route", &[], &[]);
    assert_eq!(status, 404);
    let create = envelope(
        json!({
            "limits": {
                "idle_timeout_ms": 60_000,
                "absolute_timeout_ms": 300_000,
                "max_open_transactions": 2
            }
        }),
        Some("secured-session"),
        None,
    );
    let (status, _) = post(&server, "/v1/sessions", &create, None);
    assert_eq!(status, 401);
    let (status, _) = post_with_api_key(
        &server,
        "/v1/sessions",
        &create,
        "connectome-local",
        "wrong",
    );
    assert_eq!(status, 401);
    let (status, created) = post_with_api_key(
        &server,
        "/v1/sessions",
        &create,
        "connectome-local",
        "local-api-key",
    );
    assert_eq!(status, 200, "{created}");
    let session = payload(&created)["session_id"].as_str().unwrap();
    let token = payload(&created)["token"].as_str().unwrap();

    let query = envelope(
        json!({
            "scope": "instance:socket-test",
            "query": "FROM record:document AT VALID 100 KNOWN HEAD PROJECT id, title EXPLAIN CONTRACT",
            "parameters": {},
            "budget": {
                "max_storage_keys": 100,
                "max_rows": 10,
                "max_output_bytes": 4096,
                "max_batch_rows": 10
            }
        }),
        Some("prepare-data"),
        None,
    );
    let (status, response) = post(&server, "/v1/query", &query, Some((session, token)));
    assert_eq!(status, 200, "{response}");

    let backup = envelope(
        json!({"label": "denied", "created_at_unix_ms": 500}),
        Some("denied-backup"),
        None,
    );
    let (status, denied) = post(&server, "/v1/backups", &backup, Some((session, token)));
    assert_eq!(status, 403, "{denied}");
    assert_eq!(denied["outcome"]["error"]["code"], "permission_denied");
    let (status, unauthenticated) = post(&server, "/v1/backups", &backup, None);
    assert_eq!(status, 401, "{unauthenticated}");

    let attunement = envelope(
        json!({"contract_version": rrd_contract::ATTUNEMENT_CONTRACT_VERSION, "job_id": "job-missing"}),
        None,
        None,
    );
    let (status, denied) = post(
        &server,
        "/v1/attunement/read",
        &attunement,
        Some((session, token)),
    );
    assert_eq!(status, 403, "{denied}");
    assert_eq!(denied["outcome"]["error"]["code"], "permission_denied");
    let (status, denied) = post(&server, "/v1/attunement/read", &attunement, None);
    assert_eq!(status, 401, "{denied}");

    let replan = envelope(
        json!({"contract_version": rrd_contract::ATTUNEMENT_CONTRACT_VERSION, "job_id": "job-missing", "expected_revision": 1}),
        None,
        None,
    );
    let (status, denied) = post(
        &server,
        "/v1/attunement/replan/preview",
        &replan,
        Some((session, token)),
    );
    assert_eq!(status, 403, "{denied}");
    assert_eq!(denied["outcome"]["error"]["code"], "permission_denied");

    // The quantization lifecycle is authorized under the vector collection
    // action, which this principal does not hold.
    let quantization = envelope(
        json!({
            "scope": "instance:socket-test",
            "collection_id": "documents",
            "vector_name": "title",
            "method": {"method": "turbo_quant", "bits": "bits4", "seed": 7},
            "max_storage_keys": 100
        }),
        Some("denied-quantization-build"),
        None,
    );
    let (status, denied) = post(
        &server,
        "/v1/vector/quantizations/build",
        &quantization,
        Some((session, token)),
    );
    assert_eq!(status, 403, "{denied}");
    assert_eq!(denied["outcome"]["error"]["code"], "permission_denied");

    let mut invalid_query = query.clone();
    invalid_query["payload"]["query"] = json!("NOT RRFLOWQL");
    let (status, failed) = post(&server, "/v1/query", &invalid_query, Some((session, token)));
    assert_eq!(status, 400, "{failed}");

    let oversized = "x".repeat(rrd_server::RRD_MAX_BODY_BYTES + 1);
    let (status, exhausted) = http(
        server.address,
        "POST",
        "/v1/query",
        &[("Content-Type", "application/json")],
        oversized.as_bytes(),
    );
    assert_eq!(status, 429, "{exhausted}");

    let audit = envelope(json!({"after_sequence": 0, "limit": 64}), None, None);
    let (status, audited) = post(&server, "/v1/audit/read", &audit, Some((session, token)));
    assert_eq!(status, 200, "{audited}");
    let records = payload(&audited)["records"].as_array().unwrap();
    assert!(records.len() >= 15, "{audited}");
    assert!(records
        .iter()
        .any(|record| record["action"] == "security_admin" && record["decision"] == "allowed"));
    assert!(records
        .iter()
        .any(|record| record["action"] == "service_inspect" && record["decision"] == "allowed"));
    assert!(records
        .iter()
        .any(|record| record["action"] == "unknown_request" && record["decision"] == "failed"));
    assert_eq!(
        records
            .iter()
            .filter(|record| {
                record["action"] == "session_create" && record["decision"] == "denied"
            })
            .count(),
        2
    );
    assert!(records
        .iter()
        .any(|record| { record["action"] == "session_create" && record["phase"] == "authorized" }));
    assert!(records.iter().any(|record| {
        record["action"] == "query_execute"
            && record["phase"] == "completed"
            && record["decision"] == "allowed"
    }));
    assert!(records.iter().any(|record| {
        record["action"] == "query_execute"
            && record["phase"] == "completed"
            && record["decision"] == "failed"
    }));
    assert!(records
        .iter()
        .any(|record| { record["action"] == "estate_read" && record["decision"] == "denied" }));
    assert!(records.iter().any(|record| {
        record["action"] == "vector_collection_ensure" && record["decision"] == "denied"
    }));
    assert_eq!(
        records
            .iter()
            .filter(|record| {
                record["action"] == "backup_create" && record["decision"] == "denied"
            })
            .count(),
        2
    );
    assert!(records
        .iter()
        .any(|record| { record["action"] == "audit_read" && record["phase"] == "authorized" }));
    assert!(records.iter().any(|record| {
        record["action"] == "query_execute"
            && record["phase"] == "completed"
            && record["status_code"] == 429
    }));
    assert!(records.windows(2).all(|pair| {
        pair[1]["previous_audit_sha256"].as_str() == pair[0]["audit_sha256"].as_str()
    }));
    let encoded = serde_json::to_string(records).unwrap();
    assert!(!encoded.contains("local-api-key"));
    assert!(!encoded.contains("wrong"));

    // Authorization adds later audit events. Compare only the stable prefix
    // already observed, not the moving end of the journal.
    for limit in 1..=14 {
        let request = envelope(json!({"after_sequence": 0, "limit": limit}), None, None);
        let (status, page) = post(&server, "/v1/audit/read", &request, Some((session, token)));
        assert_eq!(status, 200, "limit {limit}: {page}");
        let page = payload(&page);
        assert_eq!(page["records"].as_array().unwrap(), &records[..limit]);
        let (status, export) = post(
            &server,
            "/v1/audit/export",
            &request,
            Some((session, token)),
        );
        assert_eq!(status, 200, "limit {limit}: {export}");
        let export = payload(&export);
        let lines = export["json_lines"].as_str().unwrap();
        let exported: Vec<serde_json::Value> = lines
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(exported, records[..limit]);
        assert_eq!(export["through_sequence"], page["through_sequence"]);
        assert_eq!(export["chain_anchor_sha256"], page["chain_anchor_sha256"]);
        assert_eq!(export["chain_head_sha256"], page["chain_head_sha256"]);
        assert_eq!(
            export["content_sha256"],
            rrd_core::digest::sha256_hex(lines.as_bytes())
        );
    }

    let export = envelope(json!({"after_sequence": 0, "limit": 64}), None, None);
    let (status, exported) = post(&server, "/v1/audit/export", &export, Some((session, token)));
    assert_eq!(status, 200, "{exported}");
    let exported = payload(&exported);
    let json_lines = exported["json_lines"].as_str().unwrap();
    assert_eq!(
        exported["content_sha256"],
        rrd_core::digest::sha256_hex(json_lines.as_bytes())
    );
    assert_eq!(
        exported["record_count"].as_u64().unwrap() as usize,
        json_lines.lines().count()
    );
    assert!(!json_lines.contains("local-api-key"));
    assert!(!json_lines.contains(token));
    server.stop();
    let reopened = RrflowKvStore::open(&root).unwrap();
    assert_eq!(reopened.runtime().cursor().unwrap(), 2);
    let journal = reopened.control().journal_since(0, 64).unwrap();
    let encoded = serde_json::to_string(&journal).unwrap();
    assert!(!encoded.contains("local-api-key"));
    assert!(!encoded.contains("ApiKey"));
    drop(reopened);
    assert_tree_excludes(&root, &["local-api-key", token]);
}

#[test]
pub(super) fn jwt_session_exchange_reopens_and_credential_rotation_revokes_token_and_lease() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("jwt-instance");
    seed_query_fixture(&root);
    let signing_key = b"rrd-jwt-test-signing-key-material";
    let issued_at = u64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis(),
    )
    .unwrap();
    let principal_id = CanonicalId::new("jwt-client").unwrap();
    let issuer_id = CanonicalId::new("rrd-test-issuer").unwrap();
    let resource = rrd_contract::ResourcePath {
        segments: vec![rrd_contract::ResourceId::new(
            rrd_contract::ResourceKind::Instance,
            "socket-test",
        )
        .unwrap()],
    };
    let principal = Principal {
        id: principal_id.clone(),
        kind: PrincipalKind::Service,
        credential_sha256: rrd_core::digest::sha256_hex(b"jwt-api-key"),
        credential_revision: 1,
        not_before_unix_ms: 1,
        expires_at_unix_ms: u64::MAX,
        disabled: false,
        role_ids: Default::default(),
        grants: vec![
            ResourceGrant {
                action: SecurityAction::SessionCreate,
                resource_prefix: resource.clone(),
                data_policy: None,
            },
            ResourceGrant {
                action: SecurityAction::QueryExecute,
                resource_prefix: resource.clone(),
                data_policy: Some(DataPolicy {
                    tenant: None,
                    rows: Vec::new(),
                    allowed_fields: Some(BTreeSet::from(["title".into()])),
                }),
            },
        ],
    };
    let issuer = JwtIssuer {
        id: issuer_id.clone(),
        issuer: "https://rrd.test".into(),
        audience: "rrflow-client".into(),
        key_id: CanonicalId::new("jwt-key-1").unwrap(),
        signing_key_sha256: rrd_core::digest::sha256_hex(signing_key),
        not_before_unix_ms: 1,
        expires_at_unix_ms: u64::MAX,
        disabled: false,
    };
    let storage = RrflowKvStore::open(&root).unwrap();
    SecurityRepository::new(&storage, CanonicalId::new("socket-test").unwrap())
        .initialize(
            SecurityState {
                format_version: SECURITY_FORMAT,
                revision: 1,
                principals: BTreeMap::from([(principal_id.clone(), principal)]),
                roles: BTreeMap::new(),
                identity_bindings: BTreeMap::new(),
                jwt_issuers: BTreeMap::from([(issuer_id.clone(), issuer)]),
            },
            1,
            "bootstrap",
            "request-jwt-bootstrap",
            "operation-jwt-bootstrap",
        )
        .unwrap();
    drop(storage);
    let engine = RrdEngine::open(
        &root,
        CanonicalId::new("socket-test").unwrap(),
        COMPONENT_TOKEN_KEY,
    )
    .unwrap();
    let jwt = engine
        .issue_principal_jwt(
            &principal_id,
            b"jwt-api-key",
            signing_key,
            &JwtIssueRequest {
                issuer_id,
                token_id: CanonicalId::new("jwt-exchange-1").unwrap(),
                issued_at_unix_ms: issued_at,
                not_before_unix_ms: issued_at,
                expires_at_unix_ms: issued_at + 60_000,
            },
        )
        .unwrap();
    drop(engine);

    let create = envelope(
        json!({
            "limits": {
                "idle_timeout_ms": 60_000,
                "absolute_timeout_ms": 120_000,
                "max_open_transactions": 2
            }
        }),
        Some("jwt-session-create"),
        None,
    );
    let create_body = serde_json::to_vec(&create).unwrap();
    let authorization = format!("Bearer {}", jwt.token);
    let server = start_with_jwt(&root, signing_key);
    let (status, capabilities) = http(server.address, "GET", "/v1/capabilities", &[], &[]);
    assert_eq!(status, 200);
    let advertised = payload(&capabilities)["capabilities"].as_array().unwrap();
    assert!(advertised.iter().any(|entry| {
        entry["name"] == "jwt-session-credentials" && entry["status"] == "available"
    }));
    let (status, created) = http(
        server.address,
        "POST",
        "/v1/sessions",
        &[
            ("Content-Type", "application/json"),
            ("Authorization", &authorization),
        ],
        &create_body,
    );
    assert_eq!(status, 200, "{created}");
    let session_id = payload(&created)["session_id"].as_str().unwrap().to_owned();
    let session_token = payload(&created)["token"].as_str().unwrap().to_owned();
    let query = envelope(
        json!({
            "scope": "instance:socket-test",
            "query": "FROM record:document AT VALID 100 KNOWN HEAD PROJECT title",
            "parameters": {},
            "budget": {
                "max_storage_keys": 100,
                "max_rows": 10,
                "max_output_bytes": 4096,
                "max_batch_rows": 10
            }
        }),
        None,
        None,
    );
    let (status, queried) = post(
        &server,
        "/v1/query",
        &query,
        Some((&session_id, &session_token)),
    );
    assert_eq!(status, 200, "{queried}");
    let mut forbidden_field_query = query.clone();
    forbidden_field_query["payload"]["query"] =
        json!("FROM record:document AT VALID 100 KNOWN HEAD PROJECT secret");
    let (status, denied) = post(
        &server,
        "/v1/query",
        &forbidden_field_query,
        Some((&session_id, &session_token)),
    );
    assert_eq!(status, 403, "{denied}");
    assert_eq!(denied["outcome"]["error"]["code"], "permission_denied");
    server.stop();

    let server = start_with_jwt(&root, signing_key);
    let (status, replay) = http(
        server.address,
        "POST",
        "/v1/sessions",
        &[
            ("Content-Type", "application/json"),
            ("Authorization", &authorization),
        ],
        &create_body,
    );
    assert_eq!(status, 200, "{replay}");
    assert_eq!(payload(&replay)["session_id"], session_id);
    server.stop();

    let storage = RrflowKvStore::open(&root).unwrap();
    let repository = SecurityRepository::new(&storage, CanonicalId::new("socket-test").unwrap());
    let audit = repository.audit_since(0, 64).unwrap();
    assert!(audit.records.iter().any(|(_, record)| {
        record.action == SecurityAction::QueryExecute
            && record.phase == AuditPhase::Completed
            && record.decision == AuditDecision::Denied
            && record.status_code == 403
    }));
    let mut rotated = repository.load().unwrap().unwrap();
    rotated.revision = 2;
    let principal = rotated.principals.get_mut(&principal_id).unwrap();
    principal.credential_revision = 2;
    principal.credential_sha256 = rrd_core::digest::sha256_hex(b"rotated-jwt-api-key");
    repository
        .replace(
            1,
            rotated,
            issued_at + 1,
            "security-admin",
            "request-jwt-rotate",
            "operation-jwt-rotate",
        )
        .unwrap();
    drop(storage);

    let server = start_with_jwt(&root, signing_key);
    let (status, _) = http(
        server.address,
        "POST",
        "/v1/sessions",
        &[
            ("Content-Type", "application/json"),
            ("Authorization", &authorization),
        ],
        &create_body,
    );
    assert_eq!(status, 401);
    let (status, _) = post(
        &server,
        "/v1/query",
        &query,
        Some((&session_id, &session_token)),
    );
    assert_eq!(status, 401);
    server.stop();
    assert_tree_excludes(
        &root,
        &[&jwt.token, std::str::from_utf8(signing_key).unwrap()],
    );
}
