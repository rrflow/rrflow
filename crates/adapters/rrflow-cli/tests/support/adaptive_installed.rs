use super::*;

async fn assert_source_at_checkpoint(
    client: &RrdClient,
    session: &rrd_client::Session,
    plan: &rrd_contract::AttunementPlan,
    job: &rrd_contract::AttunementJob,
) {
    #[derive(serde::Serialize)]
    struct SourceScope<'a> {
        engram: &'a str,
        project: &'a str,
        root: &'a str,
    }
    let source_scope = SourceScope {
        engram: plan.target.engram_id.as_str(),
        project: plan.target.project_id.as_str(),
        root: "project-source",
    };
    let expected = rrd_engine::digest::sha256_hex(
        &serde_json::to_vec(&("rrflow-source-entry-v1", &source_scope, "README.md")).unwrap(),
    );
    let checkpoint = job.checkpoints.last().unwrap();
    assert_eq!(checkpoint.phase, rrd_contract::AttunementPhase::Inventory);
    let result = client.execute_query(
        session,
        ExecuteQuery {
            scope: format!("instance:{}", client.instance_id()),
            query: format!(
                "FROM traverse:source-tree-contains START source-tree-snapshot:\"{}\" DIRECTION OUTGOING DEPTH 2 AT VALID {} KNOWN {} PROJECT node_id, depth",
                checkpoint.output_sha256, checkpoint.committed_at_unix_ms, checkpoint.coordinates.runtime_cursor,
            ),
            parameters: BTreeMap::new(),
            budget: QueryBudget::default(),
        },
        RequestOptions::read("adaptive-source", "adaptive-source").unwrap(),
    ).await.unwrap();
    assert_eq!(
        result.known_at_cursor,
        checkpoint.coordinates.runtime_cursor
    );
    assert!(result
        .rows
        .iter()
        .any(|row| row.values.get("node_id") == Some(&QueryValue::String(expected.clone()))));
}

#[test]
fn selected_inventory_completes_and_replans_across_primary_process_restarts() {
    let root = tempfile::tempdir().unwrap();
    let project = root.path().join("project");
    std::fs::create_dir(&project).unwrap();
    std::fs::write(project.join("README.md"), "first source\n").unwrap();
    let identity = invoke(&["install", "identity"]);
    assert_success(&identity, "identity");
    let identity: serde_json::Value = serde_json::from_slice(&identity.stdout).unwrap();
    let proposal = root.path().join("proposal.json");
    std::fs::write(&proposal, serde_json::to_vec(&serde_json::json!({
        "format_version": 1,
        "steps": [
            {"phase": "connect", "description": "local connection", "estimated_items": 1, "estimated_input_bytes": 0},
            {"phase": "inventory", "description": "source inventory", "estimated_items": 1, "estimated_input_bytes": 0}
        ],
        "estimated_min_duration_ms": 0, "estimated_max_duration_ms": 30000
    })).unwrap()).unwrap();
    let planned = invoke(&[
        "install",
        "plan",
        "--identity",
        identity["installation_identity"].as_str().unwrap(),
        "--mode",
        "existing",
        "--project",
        project.to_str().unwrap(),
        "--attunement-proposal",
        proposal.to_str().unwrap(),
    ]);
    assert_success(&planned, "short install plan");
    let preview: serde_json::Value = serde_json::from_slice(&planned.stdout).unwrap();
    let plan: rrd_contract::AttunementPlan =
        serde_json::from_value(preview["attunement"].clone()).unwrap();
    let plan_file = root.path().join("plan.json");
    std::fs::write(&plan_file, &planned.stdout).unwrap();
    let applied = invoke(&[
        "install",
        "apply",
        "--mode",
        "existing",
        "--project",
        project.to_str().unwrap(),
        "--plan",
        plan_file.to_str().unwrap(),
        "--expect",
        preview["installation"]["plan_sha256"].as_str().unwrap(),
    ]);
    assert_success(&applied, "apply short plan");
    let job_id = CanonicalId::new(format!("job-{}", &plan.plan_sha256[..24])).unwrap();
    let credential = RrdEngine::read_installed_api_key(&project).unwrap();
    let mut original = None;
    let mut successor = None;
    let mut captured = Vec::new();
    for pass in 0..3 {
        if pass == 1 {
            std::fs::write(project.join("README.md"), "edited source\n").unwrap();
        }
        let (mut child, address) = RunningEngine::start(&project, &mut captured);
        let client = RrdClient::connect_local(
            address.parse().unwrap(),
            plan.target.instance_id.clone(),
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
                        &format!("adaptive-session-{pass}"),
                        &format!("adaptive-session-{pass}"),
                        &format!("adaptive-session-{pass}"),
                    )
                    .unwrap(),
                )
                .await
                .unwrap();
            let read = client
                .read_attunement(
                    &session,
                    rrd_contract::ReadAttunement {
                        contract_version: rrd_contract::ATTUNEMENT_CONTRACT_VERSION,
                        job_id: job_id.clone(),
                    },
                    RequestOptions::read("adaptive-read", "adaptive-read").unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(read.plan, plan);
            match pass {
                0 => {
                    let job = inventory_through_client(
                        &client,
                        &session,
                        &read.status.job,
                        &plan,
                        "adaptive-first",
                    )
                    .await;
                    assert_eq!(job.state, rrd_contract::AttunementJobState::Succeeded);
                    original = Some(job);
                }
                1 => {
                    let prior = original.as_ref().unwrap();
                    assert_eq!(&read.status.job, prior);
                    assert_source_at_checkpoint(&client, &session, &plan, prior).await;
                    let preview = client
                        .preview_attunement_replan(
                            &session,
                            rrd_contract::PreviewAttunementReplan {
                                contract_version: rrd_contract::ATTUNEMENT_CONTRACT_VERSION,
                                job_id: job_id.clone(),
                                expected_revision: prior.revision,
                            },
                            &plan,
                            RequestOptions::read("adaptive-preview", "adaptive-preview").unwrap(),
                        )
                        .await
                        .unwrap();
                    assert_ne!(preview.plan.source_sha256, plan.source_sha256);
                    let result = client
                        .accept_attunement_replan(
                            &session,
                            rrd_contract::AcceptAttunementReplan {
                                contract_version: rrd_contract::ATTUNEMENT_CONTRACT_VERSION,
                                proposal_sha256: preview.proposal_sha256.clone(),
                                preview,
                            },
                            &plan,
                            RequestOptions::mutation(
                                "adaptive-accept",
                                "adaptive-accept",
                                "adaptive-accept",
                            )
                            .unwrap(),
                        )
                        .await
                        .unwrap();
                    assert_eq!(&result.predecessor.job, prior);
                    let job = inventory_through_client(
                        &client,
                        &session,
                        &result.successor.job,
                        &result.plan,
                        "adaptive-second",
                    )
                    .await;
                    assert_eq!(job.state, rrd_contract::AttunementJobState::Succeeded);
                    successor = Some(job);
                }
                _ => {
                    assert_eq!(Some(&read.status.job), original.as_ref());
                    let expected = successor.as_ref().unwrap();
                    let current = client
                        .read_attunement(
                            &session,
                            rrd_contract::ReadAttunement {
                                contract_version: rrd_contract::ATTUNEMENT_CONTRACT_VERSION,
                                job_id: expected.id.clone(),
                            },
                            RequestOptions::read("adaptive-successor", "adaptive-successor")
                                .unwrap(),
                        )
                        .await
                        .unwrap();
                    assert_eq!(&current.status.job, expected);
                    assert_source_at_checkpoint(&client, &session, &current.plan, expected).await;
                }
            }
        });
        if pass < 2 {
            child.terminate(&mut captured);
        } else {
            child.stop(&mut captured);
        }
    }
}
