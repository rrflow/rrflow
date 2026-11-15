//! Served engrams and configured cold history across restarts.

use super::*;

/// H-07h: an instance started from the project folder records its endpoint,
/// another process proves it through `ready` without an address, and a second
/// serve of the same estate fails before binding.
#[test]
fn a_served_engram_is_found_proved_and_not_served_twice() {
    let root = tempfile::tempdir().unwrap();
    let project = root.path().join("project");
    std::fs::create_dir(&project).unwrap();
    std::fs::write(project.join("README.md"), "served endpoint record\n").unwrap();
    let project_arg = project.to_str().unwrap();
    let identity = invoke(&["install", "identity"]);
    assert_success(&identity, "allocate installation identity");
    let identity: serde_json::Value = serde_json::from_slice(&identity.stdout).unwrap();
    let plan = invoke(&[
        "install",
        "plan",
        "--identity",
        identity["installation_identity"].as_str().unwrap(),
        "--mode",
        "existing",
        "--project",
        project_arg,
    ]);
    assert_success(&plan, "install plan");
    let plan_file = root.path().join("install-plan.json");
    std::fs::write(&plan_file, &plan.stdout).unwrap();
    let preview: serde_json::Value = serde_json::from_slice(&plan.stdout).unwrap();
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
    let locator = RrdEngine::read_project_locator(&project).unwrap();
    let record_path = project.join(".rrflow").join("serving.json");
    let read_record = || -> serde_json::Value {
        serde_json::from_slice(&std::fs::read(&record_path).unwrap()).unwrap()
    };
    let ready_from_record = || invoke(&["--json", "ready", "--project", project_arg]);
    let mut captured = Vec::new();

    let unrecorded = ready_from_record();
    assert!(!unrecorded.status.success(), "nothing has been served yet");
    assert!(String::from_utf8_lossy(&unrecorded.stderr).contains("no served endpoint is recorded"));

    let (mut first, address) = RunningEngine::start(&project, &mut captured);
    let record = read_record();
    assert_eq!(record["url"], format!("http://{address}"));
    assert_eq!(record["instance_id"], locator.identity.instance_id.as_str());
    assert_eq!(record["process_id"], first.child.id());
    let ready = ready_from_record();
    capture_output(&ready, &mut captured);
    assert_success(&ready, "ready from the served endpoint record");
    let ready: serde_json::Value = serde_json::from_slice(&ready.stdout).unwrap();
    assert_eq!(ready["status"], "ready");
    assert_eq!(ready["url"], format!("http://{address}"));
    assert_eq!(ready["authentication"]["session_closed"], true);

    let second = invoke(&["serve", "--project", project_arg, "--bind", "127.0.0.1:0"]);
    capture_output(&second, &mut captured);
    assert!(!second.status.success(), "a second serve of an open estate");
    let refusal = String::from_utf8_lossy(&second.stderr);
    assert!(
        refusal.contains("already open in another process"),
        "{refusal}"
    );
    assert!(refusal.contains(&format!("http://{address}")), "{refusal}");
    assert_eq!(read_record()["url"], format!("http://{address}"));
    assert_success(&ready_from_record(), "the first server still proves ready");

    // A killed server cannot withdraw its record; the record alone is not readiness.
    first.terminate(&mut captured);
    assert!(record_path.exists());
    let stale = ready_from_record();
    assert!(!stale.status.success(), "a stale record proved ready");
    assert!(String::from_utf8_lossy(&stale.stderr).contains("is not ready"));

    let (mut restarted, replacement) = RunningEngine::start(&project, &mut captured);
    assert_eq!(read_record()["url"], format!("http://{replacement}"));
    assert_eq!(read_record()["process_id"], restarted.child.id());
    assert_success(
        &ready_from_record(),
        "ready after the stale record is replaced",
    );
    restarted.stop(&mut captured);
    #[cfg(unix)]
    assert!(
        !record_path.exists(),
        "a clean shutdown withdraws the record"
    );

    let locator_text =
        std::fs::read_to_string(project.join(".rrflow").join("config.toml")).unwrap();
    assert!(
        !locator_text.contains(&address),
        "the locator holds no endpoint"
    );
    let credential = RrdEngine::read_installed_api_key(&project).unwrap();
    assert!(!String::from_utf8_lossy(&captured).contains(credential.credential()));
    assert!(!std::fs::read_to_string(&record_path)
        .unwrap_or_default()
        .contains(credential.credential()));
}

#[test]
fn configured_cold_history_publishes_and_reads_after_process_restart() {
    let root = tempfile::tempdir().unwrap();
    let project = root.path().join("project");
    std::fs::create_dir(&project).unwrap();
    std::fs::write(project.join("README.md"), "cold history project\n").unwrap();
    let default = rrd_contract::EstateConfiguration::default();
    let mut input = rrd_contract::EstateConfigurationInput {
        format_version: default.format_version,
        clock: default.clock,
        storage: default.storage,
        reasoning: default.reasoning,
        recall: default.recall,
        query: default.query,
        attunement_follow: None,
    };
    input.storage.cold_history = Some(rrd_contract::ColdHistoryPolicy {
        interval_ms: 25,
        max_changes: 128,
        max_input_bytes: 1024 * 1024,
        max_output_bytes: 1024 * 1024,
        max_memory_bytes: 64 * 1024 * 1024,
        max_elapsed_ms: 30_000,
        row_group_rows: 16,
        compression: rrd_contract::ColdHistoryCompression::Zstd { level: 3 },
        dictionary: true,
        statistics: true,
    });
    let configuration = root.path().join("cold.toml");
    std::fs::write(&configuration, toml::to_string(&input).unwrap()).unwrap();
    let identity = invoke(&["install", "identity"]);
    assert_success(&identity, "allocate cold-history identity");
    let identity: serde_json::Value = serde_json::from_slice(&identity.stdout).unwrap();
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
        "--configuration",
        configuration.to_str().unwrap(),
    ]);
    assert_success(&plan, "cold-history install plan");
    let preview: serde_json::Value = serde_json::from_slice(&plan.stdout).unwrap();
    assert_eq!(
        preview["installation"]["configuration"]["storage"]["cold_history"]["interval_ms"],
        25
    );
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
    assert_success(&applied, "cold-history install apply");
    let locator = RrdEngine::read_project_locator(&project).unwrap();
    let credential = RrdEngine::read_installed_api_key(&project).unwrap();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let mut captured = Vec::new();
    let mut first: Option<Vec<rrd_contract::ColdHistoryReadResult>> = None;
    let mut expected = Vec::<(u64, TransactionMutation)>::new();
    let mut last_cursor = 0;
    for pass in 0..2 {
        let (mut child, address) = RunningEngine::start(&project, &mut captured);
        let client = RrdClient::connect_local(
            address.parse().unwrap(),
            locator.identity.instance_id.clone(),
            ClientConfig::default(),
        )
        .unwrap();
        let generations = runtime.block_on(async {
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
                    RequestOptions::mutation("cold-session", "cold-session", "cold-session")
                        .unwrap(),
                )
                .await
                .unwrap();
            let scope = format!("instance:{}", client.instance_id());
            if pass == 0 {
                let mut schema = client
                    .read_data_schema(
                        &session,
                        rrd_contract::ReadDataSchema {
                            scope: scope.clone(),
                            max_storage_keys: 10_000,
                        },
                        RequestOptions::read("cold-schema", "cold-schema").unwrap(),
                    )
                    .await
                    .unwrap()
                    .schema
                    .expect("installed schema must exist");
                schema.revision += 1;
                schema.migration = "add cold history fixture table".into();
                let kind = CanonicalId::new("cold-fixture").unwrap();
                schema.tables.insert(
                    kind.clone(),
                    rrd_contract::DataTableSchema {
                        model: rrd_contract::DataLogicalModel::Relational,
                        mode: rrd_contract::DataSchemaMode::Schemaless,
                        properties: BTreeMap::new(),
                        allow_additional_properties: false,
                    },
                );
                let reference = rrd_contract::DataReference {
                    kind,
                    id: CanonicalId::new("note-one").unwrap(),
                };
                let record = |body: &str, valid_from| TransactionMutation::PutRecord {
                    reference: reference.clone(),
                    valid_from,
                    valid_to: None,
                    properties: BTreeMap::from([("body".into(), QueryValue::String(body.into()))]),
                };
                let batches = vec![
                    vec![
                        TransactionMutation::PutSchema { registry: schema },
                        record("before", 100),
                    ],
                    vec![record("after", 200)],
                    vec![TransactionMutation::RetireData {
                        model: rrd_contract::DataLogicalModel::Relational,
                        target: rrd_contract::DataTarget::Reference { reference },
                        effective_at: 300,
                    }],
                ];
                for (ordinal, mutations) in batches.into_iter().enumerate() {
                    let begin = format!("cold-fixture-begin-{ordinal}");
                    let commit = format!("cold-fixture-commit-{ordinal}");
                    let lease = client
                        .begin_transaction(
                            &session,
                            BeginTransaction {
                                scope: CanonicalId::new("data").unwrap(),
                                timeout_ms: 10_000,
                            },
                            RequestOptions::mutation(&begin, &begin, &begin).unwrap(),
                        )
                        .await
                        .unwrap();
                    let expected_mutation = mutations.last().unwrap().clone();
                    let receipt = client
                        .commit_transaction(
                            &session,
                            &lease.transaction_id,
                            CommitTransaction {
                                operation_sha256: transaction_operation_sha256(&mutations),
                                mutations,
                            },
                            RequestOptions::new(&commit, &commit, Some(&commit), Some(u64::MAX))
                                .unwrap(),
                        )
                        .await
                        .unwrap();
                    assert!(!receipt.idempotent_replay);
                    last_cursor = receipt.last_runtime_cursor.expect("runtime cursor");
                    expected.push((last_cursor, expected_mutation));
                }
            }
            let mut request = rrd_contract::ReadColdHistory {
                scope,
                generation: None,
                max_artifact_bytes: 1024 * 1024,
                budget: QueryBudget::default(),
            };
            let deadline = std::time::Instant::now() + Duration::from_secs(30);
            let status = loop {
                let status = client
                    .read_cold_history(
                        &session,
                        request.clone(),
                        RequestOptions::read("cold-status", "cold-status").unwrap(),
                    )
                    .await
                    .unwrap();
                assert!(status.enabled);
                if status.current_generation > 0 && status.through_cursor >= last_cursor {
                    break status;
                }
                assert!(
                    std::time::Instant::now() < deadline,
                    "cold history did not cover acknowledged fixture cursors"
                );
                tokio::time::sleep(Duration::from_millis(100)).await;
            };
            let generation_count = first
                .as_ref()
                .map_or(status.current_generation, |prior| prior.len() as u64);
            assert!(generation_count > 0 && generation_count <= 64);
            assert!(status.current_generation >= generation_count);
            let mut results = Vec::new();
            for generation in 1..=generation_count {
                request.generation = Some(generation);
                results.push(
                    client
                        .read_cold_history(
                            &session,
                            request.clone(),
                            RequestOptions::read("cold-read", "cold-read").unwrap(),
                        )
                        .await
                        .unwrap(),
                );
            }
            results
        });
        assert_eq!(expected.len(), 3);
        for (cursor, mutation) in &expected {
            let matches = generations
                .iter()
                .flat_map(|generation| &generation.changes)
                .filter(|change| change.cursor == *cursor)
                .collect::<Vec<_>>();
            assert_eq!(matches.len(), 1, "fixture cursor must appear exactly once");
            match &matches[0].mutation {
                rrd_contract::ChangeMutationSnapshot::Data { mutation: actual } => {
                    assert_eq!(actual, mutation);
                }
                other => panic!("fixture cursor had unexpected mutation: {other:?}"),
            }
        }
        if pass == 0 {
            first = Some(generations);
            child.terminate(&mut captured);
        } else {
            let previous = first.as_ref().unwrap();
            assert_eq!(previous.len(), generations.len());
            for (before, after) in previous.iter().zip(&generations) {
                assert_eq!(before.generation, after.generation);
                assert_eq!(before.changes, after.changes);
            }
            child.stop(&mut captured);
        }
    }
    assert!(!String::from_utf8_lossy(&captured).contains(credential.credential()));
}
