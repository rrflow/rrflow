//! The installed binary replans and reads receipts through the client.

use super::*;

pub(super) async fn inventory_through_client(
    client: &RrdClient,
    session: &rrd_client::Session,
    pending: &rrd_contract::AttunementJob,
    plan: &rrd_contract::AttunementPlan,
    prefix: &str,
) -> rrd_contract::AttunementJob {
    let options = |suffix: &str| {
        let key = format!("{prefix}-{suffix}");
        RequestOptions::mutation(&key, &key, &key).unwrap()
    };
    let mut job = client
        .start_attunement(
            session,
            rrd_contract::StartAttunement {
                contract_version: rrd_contract::ATTUNEMENT_CONTRACT_VERSION,
                job_id: pending.id.clone(),
                plan_sha256: pending.plan_sha256.clone(),
                expected_revision: pending.revision,
                lease_ms: 60_000,
            },
            plan,
            options("start"),
        )
        .await
        .unwrap()
        .status
        .job;
    for phase in [
        rrd_contract::AttunementPhase::Connect,
        rrd_contract::AttunementPhase::Inventory,
    ] {
        assert_eq!(job.current_phase, Some(phase));
        job = client
            .step_attunement(
                session,
                rrd_contract::StepAttunement {
                    contract_version: rrd_contract::ATTUNEMENT_CONTRACT_VERSION,
                    job_id: job.id.clone(),
                    expected_revision: job.revision,
                    lease_generation: job.lease.as_ref().unwrap().generation,
                },
                plan,
                options(&format!("{phase:?}")),
            )
            .await
            .unwrap()
            .status
            .job;
        assert_eq!(job.checkpoints.last().unwrap().phase, phase);
    }
    job
}

pub(super) fn installed_binary_replans_and_reads_receipt_after_forced_restart(expect_parse: bool) {
    use serde_json::{json, Value};
    use std::io::{Read, Write};
    use std::net::TcpStream;

    fn post(address: &str, path: &str, body: &Value, headers: &[(&str, String)]) -> (u16, Value) {
        let bytes = serde_json::to_vec(body).unwrap();
        let mut stream = TcpStream::connect(address).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(30)))
            .unwrap();
        write!(stream, "POST {path} HTTP/1.1\r\nHost: {address}\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: {}\r\n", bytes.len()).unwrap();
        for (name, value) in headers {
            write!(stream, "{name}: {value}\r\n").unwrap();
        }
        stream.write_all(b"\r\n").unwrap();
        stream.write_all(&bytes).unwrap();
        let mut response = Vec::new();
        stream.read_to_end(&mut response).unwrap();
        let split = response
            .windows(4)
            .position(|part| part == b"\r\n\r\n")
            .unwrap();
        let status = String::from_utf8_lossy(&response[..split])
            .lines()
            .next()
            .unwrap()
            .split_whitespace()
            .nth(1)
            .unwrap()
            .parse()
            .unwrap();
        (
            status,
            serde_json::from_slice(&response[split + 4..]).unwrap(),
        )
    }
    fn payload(response: &Value) -> &Value {
        &response["outcome"]["payload"]
    }

    let root = tempfile::tempdir().unwrap();
    let project = root.path().join("project");
    std::fs::create_dir(&project).unwrap();
    std::fs::write(project.join("README.md"), "original source\n").unwrap();
    std::fs::write(
        project.join("sample.rs"),
        "pub fn project_answer() -> u32 { 42 }\n",
    )
    .unwrap();
    let proposal_file = root.path().join("source-attunement.json");
    let step = |phase| {
        json!({
            "phase": phase, "description": phase,
            "estimated_items": 0, "estimated_input_bytes": 0
        })
    };
    let mut parse = step("parse");
    parse["parse_policy"] = json!({
        "format_version": 1,
        "bindings": [{"extension": "rs", "language": "rust"}],
        "max_files": 64, "max_file_bytes": 1048576,
        "max_input_bytes": 4194304, "max_nodes": 10000,
        "max_depth": 64, "max_output_bytes": 1048576,
        "max_commit_bytes": 262144, "max_elapsed_ms": 10000,
        "max_memory_bytes": 268435456
    });
    std::fs::write(
        &proposal_file,
        serde_json::to_vec(&json!({
            "format_version": 1,
            "steps": [step("connect"), step("inventory"), parse, step("verify")],
            "estimated_min_duration_ms": 0, "estimated_max_duration_ms": 30000
        }))
        .unwrap(),
    )
    .unwrap();
    let identity = invoke(&["install", "identity"]);
    assert_success(&identity, "identity");
    let identity: Value = serde_json::from_slice(&identity.stdout).unwrap();
    let project_arg = project.to_str().unwrap();
    let plan = invoke(&[
        "install",
        "plan",
        "--identity",
        identity["installation_identity"].as_str().unwrap(),
        "--mode",
        "existing",
        "--project",
        project_arg,
        "--attunement-proposal",
        proposal_file.to_str().unwrap(),
    ]);
    assert_success(&plan, "install plan");
    let preview: Value = serde_json::from_slice(&plan.stdout).unwrap();
    let plan_file = root.path().join("plan.json");
    std::fs::write(&plan_file, &plan.stdout).unwrap();
    let applied = invoke(&[
        "install",
        "apply",
        "--mode",
        "existing",
        "--project",
        project_arg,
        "--plan",
        plan_file.to_str().unwrap(),
        "--expect",
        preview["installation"]["plan_sha256"].as_str().unwrap(),
    ]);
    assert_success(&applied, "install apply");
    let instance = preview["installation"]["target"]["instance_id"]
        .as_str()
        .unwrap();
    let plan_sha = preview["attunement"]["plan_sha256"].as_str().unwrap();
    let job = format!("job-{}", &plan_sha[..24]);
    let credential = RrdEngine::read_installed_api_key(&project).unwrap();
    let make = |payload: Value, operation: &str, mutation: bool| {
        json!({
            "protocol":"rrd", "protocol_version":1,
            "context":{"request_id":format!("request-{operation}"),"operation_id":format!("operation-{operation}"),"idempotency_key":if mutation { Some(operation) } else { None },"deadline_unix_ms":null},
            "resource":{"segments":[{"kind":"instance","id":instance}]}, "payload":payload
        })
    };
    std::fs::write(project.join("README.md"), "edited source\n").unwrap();
    let mut captured = Vec::new();
    let mut acceptance = Value::Null;
    let mut successor = Value::Null;
    for pass in 0..2 {
        let (mut child, address) = RunningEngine::start(&project, &mut captured);
        let create = make(
            json!({"limits":{"idle_timeout_ms":60000,"absolute_timeout_ms":300000,"max_open_transactions":1}}),
            &format!("binary-session-{pass}"),
            true,
        );
        let (status, session) = post(
            &address,
            "/v1/sessions",
            &create,
            &[
                (
                    "X-RRD-Principal",
                    credential.principal_id.as_str().to_owned(),
                ),
                (
                    "Authorization",
                    format!("ApiKey {}", credential.credential()),
                ),
            ],
        );
        assert_eq!(status, 200, "session creation failed");
        let headers = [
            (
                "X-RRD-Session",
                payload(&session)["session_id"].as_str().unwrap().to_owned(),
            ),
            (
                "Authorization",
                format!("Bearer {}", payload(&session)["token"].as_str().unwrap()),
            ),
        ];
        if pass == 0 {
            let start = make(
                json!({"contract_version":rrd_contract::ATTUNEMENT_CONTRACT_VERSION,"job_id":job,"plan_sha256":plan_sha,"expected_revision":1,"lease_ms":60000}),
                "binary-stale-start",
                true,
            );
            let (status, stale) = post(&address, "/v1/attunement/start", &start, &headers);
            assert_eq!(status, 412, "{stale}");
            let proposal = make(
                json!({"contract_version":rrd_contract::ATTUNEMENT_CONTRACT_VERSION,"job_id":job,"expected_revision":1}),
                "binary-preview",
                false,
            );
            let (status, proposed) = post(
                &address,
                "/v1/attunement/replan/preview",
                &proposal,
                &headers,
            );
            assert_eq!(status, 200, "{proposed}");
            acceptance = make(
                json!({"contract_version":rrd_contract::ATTUNEMENT_CONTRACT_VERSION,"preview":payload(&proposed),"proposal_sha256":payload(&proposed)["proposal_sha256"]}),
                "binary-accept",
                true,
            );
            let (status, accepted) = post(
                &address,
                "/v1/attunement/replan/accept",
                &acceptance,
                &headers,
            );
            assert_eq!(status, 200, "{accepted}");
            assert_eq!(payload(&accepted)["idempotent_replay"], false);
            successor = payload(&accepted)["successor"]["job"].clone();
            child.terminate(&mut captured);
        } else {
            let (status, replay) = post(
                &address,
                "/v1/attunement/replan/accept",
                &acceptance,
                &headers,
            );
            assert_eq!(status, 200, "{replay}");
            assert_eq!(payload(&replay)["idempotent_replay"], true);
            assert_eq!(payload(&replay)["successor"]["job"], successor);
            let read = make(
                json!({"contract_version":rrd_contract::ATTUNEMENT_CONTRACT_VERSION,"job_id":job,"idempotency_key":"binary-accept"}),
                "binary-receipt",
                false,
            );
            let (status, receipt) = post(
                &address,
                "/v1/attunement/replan/receipt/read",
                &read,
                &headers,
            );
            assert_eq!(status, 200, "{receipt}");
            assert_eq!(
                payload(&receipt)["receipt"]["result"]["successor"]["job"],
                successor
            );
            child.stop(&mut captured);
        }
    }

    // Exercise the same durable successor through the generated Rust client,
    // with a second source edit and a forced primary-process restart.
    let old_plan: rrd_contract::AttunementPlan =
        serde_json::from_value(acceptance["payload"]["preview"]["plan"].clone()).unwrap();
    let old_job: rrd_contract::AttunementJob = serde_json::from_value(successor).unwrap();
    std::fs::write(project.join("README.md"), "typed client edit\n").unwrap();
    let mut typed_acceptance = None;
    let mut typed_successor = None;
    let mut typed_inventory = None;
    for pass in 0..2 {
        let (mut child, address) = RunningEngine::start(&project, &mut captured);
        let client = RrdClient::connect_local(
            address.parse().unwrap(),
            old_plan.target.instance_id.clone(),
            ClientConfig::default(),
        )
        .unwrap();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            let session = client
                .create_session(
                    credential.principal_id.clone(),
                    credential.credential(),
                    CreateSession {
                        limits: SessionLimits {
                            idle_timeout_ms: 60_000,
                            absolute_timeout_ms: 300_000,
                            max_open_transactions: 1,
                        },
                    },
                    RequestOptions::mutation(
                        &format!("typed-session-{pass}"),
                        &format!("typed-session-{pass}"),
                        &format!("typed-session-{pass}"),
                    )
                    .unwrap(),
                )
                .await
                .unwrap();
            if pass == 0 {
                let preview = client
                    .preview_attunement_replan(
                        &session,
                        rrd_contract::PreviewAttunementReplan {
                            contract_version: rrd_contract::ATTUNEMENT_CONTRACT_VERSION,
                            job_id: old_job.id.clone(),
                            expected_revision: old_job.revision,
                        },
                        &old_plan,
                        RequestOptions::read("typed-preview-request", "typed-preview-operation")
                            .unwrap(),
                    )
                    .await
                    .unwrap();
                let request = rrd_contract::AcceptAttunementReplan {
                    contract_version: rrd_contract::ATTUNEMENT_CONTRACT_VERSION,
                    proposal_sha256: preview.proposal_sha256.clone(),
                    preview,
                };
                let result = client
                    .accept_attunement_replan(
                        &session,
                        request.clone(),
                        &old_plan,
                        RequestOptions::mutation(
                            "typed-accept-request",
                            "typed-accept-operation",
                            "typed-binary-accept",
                        )
                        .unwrap(),
                    )
                    .await
                    .unwrap();
                assert!(!result.idempotent_replay);
                typed_inventory = Some(
                    inventory_through_client(
                        &client,
                        &session,
                        &result.successor.job,
                        &request.preview.plan,
                        "binary-inventory",
                    )
                    .await,
                );
                typed_successor = Some(result.successor.job);
                typed_acceptance = Some(request);
            } else {
                let result = client
                    .accept_attunement_replan(
                        &session,
                        typed_acceptance.clone().unwrap(),
                        &old_plan,
                        RequestOptions::mutation(
                            "typed-accept-request",
                            "typed-accept-operation",
                            "typed-binary-accept",
                        )
                        .unwrap(),
                    )
                    .await
                    .unwrap();
                assert!(result.idempotent_replay);
                assert_eq!(Some(result.successor.job), typed_successor);
                let receipt = client
                    .read_attunement_replan_receipt(
                        &session,
                        rrd_contract::ReadAttunementReceipt {
                            contract_version: rrd_contract::ATTUNEMENT_CONTRACT_VERSION,
                            job_id: old_job.id.clone(),
                            idempotency_key: rrd_contract::CorrelationId::new(
                                "typed-binary-accept",
                            )
                            .unwrap(),
                        },
                        RequestOptions::read("typed-receipt-request", "typed-receipt-operation")
                            .unwrap(),
                    )
                    .await
                    .unwrap();
                assert_eq!(
                    Some(receipt.receipt.unwrap().result.successor.job),
                    typed_successor
                );
                let inventoried = typed_inventory.as_ref().unwrap();
                let current_plan = &typed_acceptance.as_ref().unwrap().preview.plan;
                let read = client
                    .read_attunement(
                        &session,
                        rrd_contract::ReadAttunement {
                            contract_version: rrd_contract::ATTUNEMENT_CONTRACT_VERSION,
                            job_id: inventoried.id.clone(),
                        },
                        RequestOptions::read("binary-inventory-read", "binary-inventory-read")
                            .unwrap(),
                    )
                    .await
                    .unwrap();
                assert_eq!(
                    read.status.job, *inventoried,
                    "acknowledged Inventory survives process death"
                );
                // Derive the fixture identities independently of the returned
                // rows. A job checkpoint alone does not prove that its source
                // records are reachable through the public query boundary.
                #[derive(serde::Serialize)]
                struct SourceScope<'a> {
                    estate: &'a str,
                    project: &'a str,
                    root: &'a str,
                }
                let scope = SourceScope {
                    estate: current_plan.target.estate_id.as_str(),
                    project: current_plan.target.project_id.as_str(),
                    root: "project-source",
                };
                let checkpoint = &inventoried.checkpoints[1];
                let result = client
                    .execute_query(
                        &session,
                        ExecuteQuery {
                            scope: format!("instance:{}", client.instance_id()),
                            query: format!(
                                "FROM traverse:source-tree-contains START source-tree-snapshot:\"{}\" DIRECTION OUTGOING DEPTH 2 AT VALID {} KNOWN {} PROJECT node_id, depth",
                                checkpoint.output_sha256,
                                checkpoint.committed_at_unix_ms,
                                checkpoint.coordinates.runtime_cursor,
                            ),
                            parameters: BTreeMap::new(),
                            budget: QueryBudget::default(),
                        },
                        RequestOptions::read("binary-source-readback", "binary-source-readback")
                            .unwrap(),
                    )
                    .await
                    .unwrap();
                assert_eq!(result.known_at_cursor, checkpoint.coordinates.runtime_cursor);
                for path in ["README.md", "sample.rs"] {
                    let bytes = serde_json::to_vec(&("rrflow-source-entry-v1", &scope, path))
                        .unwrap();
                    let identity = rrd_engine::digest::sha256_hex(&bytes);
                    assert!(
                        result.rows.iter().any(|row| {
                            row.values.get("node_id") == Some(&QueryValue::String(identity.clone()))
                        }),
                        "committed Inventory does not reach fixture source {path}"
                    );
                }
                eprintln!("installed inventory query readback verified after restart");
                std::fs::write(project.join("README.md"), "edit after durable Inventory\n")
                    .unwrap();
                let preview = client
                    .preview_attunement_replan(
                        &session,
                        rrd_contract::PreviewAttunementReplan {
                            contract_version: rrd_contract::ATTUNEMENT_CONTRACT_VERSION,
                            job_id: inventoried.id.clone(),
                            expected_revision: inventoried.revision,
                        },
                        current_plan,
                        RequestOptions::read("binary-inventory-replan", "binary-inventory-replan")
                            .unwrap(),
                    )
                    .await
                    .unwrap();
                let successor_plan = preview.plan.clone();
                let preview_evidence = preview.evidence.clone();
                let result = client
                    .accept_attunement_replan(
                        &session,
                        rrd_contract::AcceptAttunementReplan {
                            contract_version: rrd_contract::ATTUNEMENT_CONTRACT_VERSION,
                            proposal_sha256: preview.proposal_sha256.clone(),
                            preview,
                        },
                        current_plan,
                        RequestOptions::mutation(
                            "binary-inventory-accept",
                            "binary-inventory-accept",
                            "binary-inventory-accept",
                        )
                        .unwrap(),
                    )
                    .await
                    .unwrap();
                // P1 of the D-02 plan: after the committed Inventory the
                // successor is derived from evidence, not copied from the
                // operator's proposal. The fixture holds `sample.rs`, so the
                // evidence detects Rust with a bound grammar, and the plan
                // selects only the capabilities this engine can execute.
                let evidence = preview_evidence
                    .as_ref()
                    .expect("a replan after a committed Inventory carries evidence");
                assert_eq!(
                    evidence
                        .languages
                        .iter()
                        .find(|(language, _)| language.as_str() == "rust")
                        .map(|(_, files)| *files),
                    Some(1)
                );
                assert!(evidence.bound_grammars.iter().any(|language| language.as_str() == "rust"));
                let derived: Vec<rrd_contract::AttunementPhase> =
                    successor_plan.phases.iter().map(|step| step.phase).collect();
                let next = inventory_through_client(
                    &client,
                    &session,
                    &result.successor.job,
                    &successor_plan,
                    "binary-second-inventory",
                )
                .await;
                assert_ne!(
                    next.checkpoints[1].output_sha256,
                    inventoried.checkpoints[1].output_sha256
                );
                assert!(
                    next.checkpoints[1].coordinates.runtime_cursor
                        > inventoried.checkpoints[1].coordinates.runtime_cursor
                );
                if expect_parse {
                    // The D-04 acceptance oracle, recorded in POAM-006: once
                    // the Parse executor is installed the evidence makes it
                    // available, the derived plan selects it for the Rust
                    // source, and the step checkpoints. The offline harness
                    // requires every listed case to run and pass, so this
                    // branch is reached only when that executor lands.
                    assert!(derived.contains(&rrd_contract::AttunementPhase::Parse));
                    assert_eq!(
                        next.current_phase,
                        Some(rrd_contract::AttunementPhase::Parse)
                    );
                    let parsed = client
                        .step_attunement(
                            &session,
                            rrd_contract::StepAttunement {
                                contract_version: rrd_contract::ATTUNEMENT_CONTRACT_VERSION,
                                job_id: next.id.clone(),
                                expected_revision: next.revision,
                                lease_generation: next.lease.as_ref().unwrap().generation,
                            },
                            &successor_plan,
                            RequestOptions::mutation("binary-parse", "binary-parse", "binary-parse")
                                .unwrap(),
                        )
                        .await
                        .unwrap()
                        .status
                        .job;
                    assert_eq!(
                        parsed.checkpoints.last().map(|checkpoint| checkpoint.phase),
                        Some(rrd_contract::AttunementPhase::Parse),
                        "installed source parsing did not checkpoint: {:?}",
                        parsed.failure
                    );
                } else {
                    // Without a native parse supervisor the executable set is
                    // Connect and Inventory, so the derived plan omits Parse,
                    // reported unavailable rather than planned, the successor succeeds
                    // after its Inventory checkpoint, and no job reaches
                    // `phase-unavailable`.
                    assert_eq!(
                        derived,
                        vec![
                            rrd_contract::AttunementPhase::Connect,
                            rrd_contract::AttunementPhase::Inventory
                        ]
                    );
                    assert_eq!(next.state, rrd_contract::AttunementJobState::Succeeded);
                    assert!(next.failure.is_none());
                    assert_eq!(next.checkpoints.len(), 2);
                }
            }
        });
        if pass == 0 {
            child.terminate(&mut captured);
        } else {
            child.stop(&mut captured);
        }
    }
}
