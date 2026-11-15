use super::*;
use rrd_core::{ReadStamp, RuntimeCommit, RuntimeLogicalModel, RuntimeType};
use rrd_store::{RuntimeReadBudget, RuntimeVersionedSource};

#[test]
#[ignore = "POAM-006: a cancelled successor refresh leaves KNOWN HEAD reading a partially refreshed root while the completed checkpoint still reads the old one; the repair is not written"]
fn interrupted_refresh_does_not_expose_partial_facts_as_completed_snapshot() {
    let (installed, engine, session) =
        Installed::with_source(InstallationTargetKind::ExistingProject, None, |root| {
            std::fs::create_dir(root.join("src")).unwrap();
            for index in 0..2048 {
                std::fs::write(root.join(format!("src/file-{index:04}.txt")), b"before\n").unwrap();
            }
        });
    let started = call(
        &engine,
        &session,
        &installed.start(60_000),
        "visibility-start",
        BASE + 1_000,
    )
    .unwrap();
    let connected = call(
        &engine,
        &session,
        &step(&started.status.job),
        "visibility-connect",
        started.status.job.updated_at_unix_ms + 1,
    )
    .unwrap();
    let first = call(
        &engine,
        &session,
        &step(&connected.status.job),
        "visibility-first",
        connected.status.job.updated_at_unix_ms + 1,
    )
    .unwrap();
    let first_job = first.status.job;
    let scope = ScopeId::new(format!("instance:{}", engine.instance_id())).unwrap();
    let first_read = engine.storage.runtime().read_stamp(&scope).unwrap();
    let root_record = engine
        .storage
        .runtime()
        .read_versioned(
            &first_read,
            &[RuntimeVersionedSource::Records {
                kind: Some(RuntimeType::new("source-tree-entry").unwrap()),
            }],
            RuntimeReadBudget::new(100_000).unwrap(),
        )
        .unwrap()
        .changes
        .into_iter()
        .find_map(|change| match change.mutation {
            RuntimeMutation::Record { record }
                if record.properties.get("path") == Some(&RuntimeValue::String(String::new())) =>
            {
                Some(record)
            }
            _ => None,
        })
        .unwrap();
    let head_key = format!("server/state/{}/inventory/head", engine.instance_id());
    let first_head = engine.storage.control().get(&head_key).unwrap().unwrap();
    for index in 0..2048 {
        std::fs::write(
            installed
                .root
                .path()
                .join(format!("src/file-{index:04}.txt")),
            b"after\n",
        )
        .unwrap();
    }
    let preview = engine
        .preview_attunement_replan(
            &session.session_id,
            &session.token,
            &installed.job,
            first_job.revision,
            first_job.updated_at_unix_ms + 1,
            "request-visibility-preview",
            "operation-visibility-preview",
        )
        .unwrap();
    let accepted = engine
        .accept_attunement_replan(
            &session.session_id,
            &session.token,
            &preview,
            &preview.proposal_sha256,
            &context("visibility-replan"),
            first_job.updated_at_unix_ms + 2,
        )
        .unwrap();
    let successor = accepted.successor.job;
    let next_start = AttunementCommand::Start {
        job_id: successor.id.clone(),
        plan_sha256: successor.plan_sha256.clone(),
        expected_revision: successor.revision,
        lease_ms: 60_000,
    };
    let running = call(
        &engine,
        &session,
        &next_start,
        "visibility-next-start",
        successor.updated_at_unix_ms + 1,
    )
    .unwrap();
    let next = call(
        &engine,
        &session,
        &step(&running.status.job),
        "visibility-next-connect",
        running.status.job.updated_at_unix_ms + 1,
    )
    .unwrap();
    let at = next.status.job.updated_at_unix_ms + 1;
    let progress_key = format!(
        "server/state/{}/inventory/progress/{}/1",
        engine.instance_id(),
        successor.id
    );
    std::thread::scope(|scope| {
        let (sender, receiver) = std::sync::mpsc::channel();
        let engine = &engine;
        let session = &session;
        let command = step(&next.status.job);
        let worker = scope.spawn(move || {
            let authorized = engine
                .begin_invocation(
                    Invocation {
                        context: context("visibility-interrupt"),
                        resource: ResourcePath {
                            segments: vec![rrd_contract::ResourceId::new(
                                rrd_contract::ResourceKind::Instance,
                                engine.instance_id().as_str(),
                            )
                            .unwrap()],
                        },
                        observed_at_unix_ms: at,
                        attempt: 1,
                        request_sha256: digest::sha256_hex(&serde_json::to_vec(&command).unwrap()),
                    },
                    RrdOperation::EngramAdmin,
                    InvocationCredential::Session {
                        session_id: &session.session_id,
                        token: &session.token,
                    },
                )
                .unwrap();
            sender.send(authorized).unwrap();
            call(engine, session, &command, "visibility-interrupt", at)
        });
        let authorized = receiver
            .recv_timeout(std::time::Duration::from_secs(30))
            .unwrap();
        let deadline = std::time::Instant::now();
        while engine
            .storage
            .control()
            .get(&progress_key)
            .unwrap()
            .is_none()
        {
            assert!(
                !worker.is_finished(),
                "refresh completed before interruption"
            );
            assert!(deadline.elapsed() < std::time::Duration::from_secs(60));
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        authorized.cancel_pending();
        assert!(matches!(
            worker.join().unwrap(),
            Err(ServiceError::AdmissionCancelled)
        ));
    });
    assert_eq!(
        engine.storage.control().get(&head_key).unwrap().unwrap(),
        first_head
    );
    let partial_read = engine.storage.runtime().read_stamp(&scope).unwrap();
    assert!(partial_read.commit_cursor > first_read.commit_cursor);
    let unrelated_at = at + 30_000;
    let transaction = engine
        .begin_transaction(
            &session.session_id,
            &session.token,
            &rrd_contract::BeginTransaction {
                scope: CanonicalId::new("data").unwrap(),
                timeout_ms: 1_000,
            },
            &context("visibility-unrelated-begin"),
            unrelated_at,
        )
        .unwrap();
    let unrelated_mutations = vec![rrd_contract::TransactionMutation::AssertClaim {
        subject: CanonicalId::new("visibility-subject").unwrap(),
        predicate: CanonicalId::new("visibility-marker").unwrap(),
        object: "unrelated-latest".into(),
        valid_from: unrelated_at,
        tx_time: unrelated_at,
        producer: CanonicalId::new("visibility-fixture").unwrap(),
        confidence: None,
    }];
    engine
        .commit_transaction(
            &session.session_id,
            &session.token,
            &transaction.transaction_id,
            &id("visibility-unrelated-commit"),
            &rrd_contract::CommitTransaction {
                operation_sha256: rrd_contract::transaction_operation_sha256(&unrelated_mutations),
                mutations: unrelated_mutations,
            },
            unrelated_at + 1,
            "request-visibility-unrelated-commit",
            "operation-visibility-unrelated-commit",
        )
        .unwrap();
    let read_unrelated = |engine: &RrdEngine, session: &SessionLease, now: u64, suffix: &str| {
        let result = engine.execute_query(
            &session.session_id, &session.token,
            &rrd_contract::ExecuteQuery {
                scope: scope.to_string(),
                query: format!("FROM claim:visibility-marker AT VALID {now} KNOWN HEAD PROJECT subject, object"),
                parameters: Default::default(), budget: rrd_contract::QueryBudget::default(),
            }, now, &format!("request-visibility-unrelated-{suffix}"),
            &format!("operation-visibility-unrelated-{suffix}"),
        ).unwrap();
        assert_eq!(result.rows.len(), 1);
        assert_eq!(
            result.rows[0].values.get("object"),
            Some(&rrd_contract::QueryValue::String("unrelated-latest".into()))
        );
    };
    read_unrelated(&engine, &session, unrelated_at + 2, "before-reopen");
    // The all-model public snapshot has its own storage consumer. Capturing it
    // separately prevents a query-only fix from masking a mixed source view.
    let read_snapshot_root =
        |engine: &RrdEngine, session: &SessionLease, now: u64, suffix: &str| {
            let snapshot = engine
                .read_data_snapshot(
                    &session.session_id,
                    &session.token,
                    &rrd_contract::ReadDataSnapshot {
                        valid_at: unrelated_at,
                        max_storage_keys: rrd_contract::MAX_DATA_SNAPSHOT_STORAGE_KEYS,
                    },
                    now,
                    &format!("request-visibility-snapshot-{suffix}"),
                    &format!("operation-visibility-snapshot-{suffix}"),
                )
                .unwrap();
            snapshot
                .entries
                .into_iter()
                .find_map(|entry| match entry.value {
                    rrd_contract::TransactionMutation::PutRecord {
                        reference,
                        properties,
                        ..
                    } if reference.kind.as_str() == "source-tree-entry"
                        && reference.id.as_str() == root_record.reference.id.as_str() =>
                    {
                        properties.get("subtree_sha256").cloned()
                    }
                    _ => None,
                })
                .expect("completed source root in public data snapshot")
        };
    let snapshot_root = read_snapshot_root(&engine, &session, at + 30_003, "before-reopen");
    let query = format!("FROM record:source-tree-entry AT VALID {} KNOWN HEAD WHERE id = \"{}\" PROJECT path, subtree_sha256", at + 30_000, root_record.reference.id);
    let result = engine
        .execute_query(
            &session.session_id,
            &session.token,
            &rrd_contract::ExecuteQuery {
                scope: scope.to_string(),
                query,
                parameters: Default::default(),
                budget: rrd_contract::QueryBudget::default(),
            },
            at + 30_003,
            "request-visibility-query",
            "operation-visibility-query",
        )
        .unwrap();
    assert_eq!(result.rows.len(), 1);
    let expected = match root_record.properties.get("subtree_sha256").unwrap() {
        RuntimeValue::String(value) => rrd_contract::QueryValue::String(value.clone()),
        other => panic!("unexpected root digest: {other:?}"),
    };
    let pinned = engine
        .execute_query(
            &session.session_id,
            &session.token,
            &rrd_contract::ExecuteQuery {
                scope: scope.to_string(),
                query: format!("FROM record:source-tree-entry AT VALID {} KNOWN {} WHERE id = \"{}\" PROJECT path, subtree_sha256", at + 30_000, first_read.commit_cursor, root_record.reference.id),
                parameters: Default::default(),
                budget: rrd_contract::QueryBudget::default(),
            },
            at + 30_004,
            "request-visibility-pinned",
            "operation-visibility-pinned",
        )
        .unwrap();
    assert_eq!(pinned.rows.len(), 1);
    assert_eq!(pinned.rows[0].values.get("subtree_sha256"), Some(&expected));
    let traversal = engine
        .execute_query(
            &session.session_id,
            &session.token,
            &rrd_contract::ExecuteQuery {
                scope: scope.to_string(),
                query: format!("FROM traverse:source-tree-contains START source-tree-snapshot:\"{}\" DIRECTION OUTGOING DEPTH 1 AT VALID {} KNOWN HEAD PROJECT node_id", first_job.checkpoints[1].output_sha256, at + 30_000),
                parameters: Default::default(),
                budget: rrd_contract::QueryBudget::default(),
            },
            at + 30_005,
            "request-visibility-traversal",
            "operation-visibility-traversal",
        )
        .unwrap();
    assert!(traversal.rows.iter().any(|row| row.values.get("node_id")
        == Some(&rrd_contract::QueryValue::String(
            root_record.reference.id.to_string()
        ))));
    drop(engine);
    let reopened = installed.reopen(at + 30_006);
    let reopened_session = installed.session(&reopened, "visibility-reopened", at + 30_007);
    read_unrelated(&reopened, &reopened_session, at + 30_008, "after-reopen");
    let reopened_snapshot_root =
        read_snapshot_root(&reopened, &reopened_session, at + 30_008, "after-reopen");
    assert_eq!(
        reopened.storage.control().get(&head_key).unwrap().unwrap(),
        first_head
    );
    let after_reopen = reopened
        .execute_query(
            &reopened_session.session_id,
            &reopened_session.token,
            &rrd_contract::ExecuteQuery {
                scope: scope.to_string(),
                query: format!("FROM record:source-tree-entry AT VALID {} KNOWN HEAD WHERE id = \"{}\" PROJECT path, subtree_sha256", at + 30_000, root_record.reference.id),
                parameters: Default::default(),
                budget: rrd_contract::QueryBudget::default(),
            },
            at + 30_009,
            "request-visibility-reopened",
            "operation-visibility-reopened",
        )
        .unwrap();
    assert_eq!(after_reopen.rows, result.rows);
    // A cancelled publication must remain recoverable. Keep this part of the
    // joined journey ahead of the known failing visibility assertion below.
    let recovery_at = next.status.job.lease.as_ref().unwrap().expires_at_unix_ms + 1_000;
    let recovery_session = installed.session(&reopened, "visibility-recovery", recovery_at);
    let recovered = call(
        &reopened,
        &recovery_session,
        &AttunementCommand::RecoverExpired {
            job_id: successor.id.clone(),
            expected_revision: next.status.job.revision,
        },
        "visibility-recover",
        recovery_at + 1,
    )
    .unwrap();
    let resumed_at = recovered.status.job.updated_at_unix_ms + 1;
    let mut resume_command = resume(&recovered.status.job, resumed_at);
    if let AttunementCommand::Resume { lease_ms, .. } = &mut resume_command {
        *lease_ms = 60_000;
    }
    let resumed = call(
        &reopened,
        &recovery_session,
        &resume_command,
        "visibility-resume",
        resumed_at,
    )
    .unwrap();
    let completion_command = step(&resumed.status.job);
    let completed = call(
        &reopened,
        &recovery_session,
        &completion_command,
        "visibility-complete",
        resumed.status.job.updated_at_unix_ms + 1,
    )
    .unwrap();
    assert_eq!(completed.status.job.checkpoints.len(), 2);
    let final_head = reopened.storage.control().get(&head_key).unwrap().unwrap();
    assert_ne!(final_head, first_head);
    assert!(reopened
        .storage
        .control()
        .get(&progress_key)
        .unwrap()
        .is_none());
    let final_at = completed.status.job.updated_at_unix_ms + 1;
    let read_root = |engine: &RrdEngine, cursor: &str, suffix: &str| {
        engine.execute_query(
            &recovery_session.session_id, &recovery_session.token,
            &rrd_contract::ExecuteQuery {
                scope: scope.to_string(),
                query: format!("FROM record:source-tree-entry AT VALID {final_at} KNOWN {cursor} WHERE id = \"{}\" PROJECT subtree_sha256", root_record.reference.id),
                parameters: Default::default(), budget: rrd_contract::QueryBudget::default(),
            },
            final_at, &format!("request-visibility-{suffix}"), &format!("operation-visibility-{suffix}"),
        ).unwrap()
    };
    let final_root = read_root(&reopened, "HEAD", "complete-head");
    assert_eq!(final_root.rows.len(), 1);
    assert_ne!(
        final_root.rows[0].values.get("subtree_sha256"),
        Some(&expected)
    );
    let old_cursor = first_read.commit_cursor.to_string();
    assert_eq!(
        read_root(&reopened, &old_cursor, "complete-old").rows[0]
            .values
            .get("subtree_sha256"),
        Some(&expected)
    );
    drop(reopened);
    let final_reopened = installed.reopen(final_at + 1);
    let final_session = installed.session(&final_reopened, "visibility-final-reopen", final_at + 2);
    let replay = call(
        &final_reopened,
        &final_session,
        &completion_command,
        "visibility-complete",
        final_at + 3,
    )
    .unwrap();
    assert!(replay.idempotent_replay);
    read_unrelated(
        &final_reopened,
        &final_session,
        final_at + 4,
        "after-replay",
    );
    assert_eq!(replay.status.job, completed.status.job);
    assert_eq!(
        final_reopened
            .storage
            .control()
            .get(&head_key)
            .unwrap()
            .unwrap(),
        final_head
    );
    for (cursor, wanted, suffix) in [
        (
            "HEAD",
            final_root.rows[0].values.get("subtree_sha256"),
            "replay-new",
        ),
        (old_cursor.as_str(), Some(&expected), "replay-old"),
    ] {
        let query = rrd_contract::ExecuteQuery {
            scope: scope.to_string(),
            query: format!("FROM record:source-tree-entry AT VALID {final_at} KNOWN {cursor} WHERE id = \"{}\" PROJECT subtree_sha256", root_record.reference.id),
            parameters: Default::default(), budget: rrd_contract::QueryBudget::default(),
        };
        let value = final_reopened
            .execute_query(
                &final_session.session_id,
                &final_session.token,
                &query,
                final_at + 4,
                &format!("request-visibility-{suffix}"),
                &format!("operation-visibility-{suffix}"),
            )
            .unwrap();
        assert_eq!(value.rows.len(), 1);
        assert_eq!(value.rows[0].values.get("subtree_sha256"), wanted);
    }
    assert_eq!(
        reopened_snapshot_root, snapshot_root,
        "the interrupted snapshot root must survive reopen unchanged"
    );
    assert_eq!(
        snapshot_root, expected,
        "a completed snapshot must not expose an incomplete refresh's root through data snapshot"
    );
    assert_eq!(
        result.rows[0].values.get("subtree_sha256"),
        Some(&expected),
        "a completed snapshot must not expose an incomplete refresh's root"
    );
}

pub(in crate::engine::tests) fn first_inventory_for_worker() -> (
    Installed,
    RrdEngine,
    SessionLease,
    AttunementJob,
    ScopeId,
    ReadStamp,
) {
    first_inventory_for_worker_with_setup(|_| {})
}

pub(in crate::engine::tests) fn first_inventory_for_worker_with_setup(
    setup: impl FnOnce(&std::path::Path),
) -> (
    Installed,
    RrdEngine,
    SessionLease,
    AttunementJob,
    ScopeId,
    ReadStamp,
) {
    first_inventory_for_worker_with_setup_at(setup, BASE)
}

pub(in crate::engine::tests) fn first_inventory_for_worker_with_setup_at(
    setup: impl FnOnce(&std::path::Path),
    base: u64,
) -> (
    Installed,
    RrdEngine,
    SessionLease,
    AttunementJob,
    ScopeId,
    ReadStamp,
) {
    let proposal = crate::AttunementProposal {
        format_version: crate::ATTUNEMENT_PROPOSAL_FORMAT_VERSION,
        steps: [
            Phase::Connect,
            Phase::Inventory,
            Phase::Inventory,
            Phase::Inventory,
            Phase::Verify,
        ]
        .into_iter()
        .map(|phase| crate::AttunementProposalStep {
            phase,
            description: format!("{phase:?}"),
            estimated_items: 0,
            estimated_input_bytes: 0,
            parse_policy: None,
        })
        .collect(),
        estimated_min_duration_ms: 0,
        estimated_max_duration_ms: 30_000,
    };
    let (installed, engine, session) = Installed::with_source_at(
        InstallationTargetKind::ExistingProject,
        Some(&proposal),
        |root| {
            for name in ["alpha.txt", "beta.txt", "gamma.txt"] {
                std::fs::write(root.join(name), format!("source {name}\n")).unwrap();
            }
            setup(root);
        },
        base,
    );
    let started = call(
        &engine,
        &session,
        &installed.start(30_000),
        "delta-start",
        base + 1_000,
    )
    .unwrap();
    let connected = call(
        &engine,
        &session,
        &step(&started.status.job),
        "delta-connect",
        started.status.job.updated_at_unix_ms + 1,
    )
    .unwrap();
    let first = call(
        &engine,
        &session,
        &step(&connected.status.job),
        "delta-first",
        connected.status.job.updated_at_unix_ms + 1,
    )
    .unwrap();
    let scope = ScopeId::new(format!("instance:{}", engine.instance_id())).unwrap();
    let read = engine.storage.runtime().read_stamp(&scope).unwrap();
    (installed, engine, session, first.status.job, scope, read)
}

#[test]
fn unchanged_inventory_publishes_only_completion_metadata() {
    let (installed, engine, session, mut job, scope, mut previous_read) =
        first_inventory_for_worker();
    for key in ["delta-second", "delta-third"] {
        let next = call(
            &engine,
            &session,
            &step(&job),
            key,
            job.updated_at_unix_ms + 1,
        )
        .unwrap();
        let read = engine.storage.runtime().read_stamp(&scope).unwrap();
        assert_eq!(
            read.commit_cursor - previous_read.commit_cursor,
            3,
            "unchanged entries and containment must survive without new versions; only the snapshot, change set and root link are new"
        );
        assert_eq!(next.status.job.checkpoints.len(), job.checkpoints.len() + 1);
        assert_eq!(
            next.status
                .job
                .checkpoints
                .last()
                .unwrap()
                .coordinates
                .runtime_cursor,
            read.commit_cursor
        );
        previous_read = read;
        job = next.status.job;
    }
    assert_eq!(job.current_phase, Some(Phase::Verify));
    let reopen_at = job.updated_at_unix_ms + 1;
    drop(engine);
    let engine = installed.reopen(reopen_at);
    let session = installed.session(&engine, "delta-reopened", reopen_at + 1);
    assert_eq!(installed.read(&engine, &session, reopen_at + 2).job, job);
    assert_eq!(
        engine.storage.runtime().read_stamp(&scope).unwrap(),
        previous_read
    );
}

#[test]
fn inventory_reuse_observes_current_records_edges_and_retirements() {
    for changed_kind in ["record", "relation", "retirement"] {
        let (installed, engine, session, job, scope, before) = first_inventory_for_worker();
        let facts = engine
            .storage
            .runtime()
            .read_versioned(
                &before,
                &[
                    RuntimeVersionedSource::Records {
                        kind: Some(RuntimeType::new("source-tree-entry").unwrap()),
                    },
                    RuntimeVersionedSource::Relations {
                        kind: Some(RuntimeType::new("source-tree-contains").unwrap()),
                    },
                ],
                RuntimeReadBudget::new(10_000).unwrap(),
            )
            .unwrap();
        let original = facts
            .changes
            .iter()
            .find_map(|change| match &change.mutation {
                RuntimeMutation::Record { record }
                    if record.properties.get("path")
                        == Some(&RuntimeValue::String("alpha.txt".into())) =>
                {
                    Some(record.clone())
                }
                _ => None,
            })
            .unwrap();
        let original_edge = facts
            .changes
            .iter()
            .find_map(|change| match &change.mutation {
                RuntimeMutation::Relation { relation } if relation.to == original.reference => {
                    Some(relation.clone())
                }
                _ => None,
            })
            .unwrap();
        let at = job.updated_at_unix_ms + 1;
        let mutation = match changed_kind {
            "record" => {
                let mut record = original.clone();
                record.valid_from = at;
                record.properties.insert(
                    "subtree_sha256".into(),
                    RuntimeValue::String("0".repeat(64)),
                );
                RuntimeMutation::Record { record }
            }
            "relation" => {
                let mut relation = original_edge.clone();
                relation.valid_from = at;
                relation.from = original.reference.clone();
                RuntimeMutation::Relation { relation }
            }
            "retirement" => RuntimeMutation::Retire {
                retirement: rrd_core::RuntimeRetirement {
                    model: RuntimeLogicalModel::GraphNode,
                    reference: original.reference.clone(),
                    effective_at: at,
                },
            },
            _ => unreachable!(),
        };
        engine
            .storage
            .runtime()
            .commit(&RuntimeCommit {
                scope: scope.clone(),
                at,
                actor: "test-fixture".into(),
                expected_cursor: before.commit_cursor,
                mutations: vec![mutation],
            })
            .unwrap();
        let intervening = engine.storage.runtime().read_stamp(&scope).unwrap();
        let repaired = call(&engine, &session, &step(&job), "delta-repair", at + 1).unwrap();
        let after = engine.storage.runtime().read_stamp(&scope).unwrap();
        assert_eq!(
            after.commit_cursor - intervening.commit_cursor,
            5,
            "{changed_kind}: republish the changed entry/edge group and completion metadata"
        );
        let current = engine
            .storage
            .runtime()
            .read_versioned(
                &after,
                &[
                    RuntimeVersionedSource::Identity {
                        model: RuntimeLogicalModel::GraphNode,
                        reference: original.reference.clone(),
                    },
                    RuntimeVersionedSource::Identity {
                        model: RuntimeLogicalModel::GraphRelation,
                        reference: original_edge.reference.clone(),
                    },
                ],
                RuntimeReadBudget::new(10_000).unwrap(),
            )
            .unwrap();
        let latest_record = current
            .changes
            .iter()
            .filter_map(|change| match &change.mutation {
                RuntimeMutation::Record { record } => Some((change.cursor, record)),
                _ => None,
            })
            .max_by_key(|(cursor, _)| *cursor)
            .unwrap()
            .1;
        assert_eq!(latest_record.properties, original.properties);
        let latest_edge = current
            .changes
            .iter()
            .filter_map(|change| match &change.mutation {
                RuntimeMutation::Relation { relation } => Some((change.cursor, relation)),
                _ => None,
            })
            .max_by_key(|(cursor, _)| *cursor)
            .unwrap()
            .1;
        assert_eq!(latest_edge.from, original_edge.from);
        assert_eq!(latest_edge.to, original_edge.to);
        // Reconstruct the new complete snapshot before another unchanged run.
        let repeated = call(
            &engine,
            &session,
            &step(&repaired.status.job),
            "delta-repaired-again",
            repaired.status.job.updated_at_unix_ms + 1,
        )
        .unwrap();
        assert_eq!(
            engine
                .storage
                .runtime()
                .read_stamp(&scope)
                .unwrap()
                .commit_cursor
                - after.commit_cursor,
            3
        );
        assert_eq!(
            installed
                .read(
                    &engine,
                    &session,
                    repeated.status.job.updated_at_unix_ms + 1
                )
                .job,
            repeated.status.job
        );
    }
}
