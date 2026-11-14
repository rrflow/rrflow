//! Inventory checkpoints, batches, bounds, interruptions and partial recovery.

use super::*;

#[test]
pub(super) fn inventory_checkpoint_and_source_records_commit_reopen_and_replan_together() {
    let proposal = crate::AttunementProposal {
        format_version: crate::ATTUNEMENT_PROPOSAL_FORMAT_VERSION,
        steps: [Phase::Connect, Phase::Inventory, Phase::Verify]
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
    let (installed, engine, session) = Installed::with_source(
        InstallationTargetKind::ExistingProject,
        Some(&proposal),
        |root| {
            std::fs::write(root.join("before.txt"), "rename content\n").unwrap();
            std::fs::write(root.join("removed.txt"), "remove content\n").unwrap();
        },
    );
    let running = call(
        &engine,
        &session,
        &installed.start(30_000),
        "inventory-start",
        BASE + 1_000,
    )
    .unwrap();
    let connected = call(
        &engine,
        &session,
        &step(&running.status.job),
        "inventory-connect",
        running.status.job.updated_at_unix_ms + 1,
    )
    .unwrap();
    let inventoried = call(
        &engine,
        &session,
        &step(&connected.status.job),
        "inventory-step",
        connected.status.job.updated_at_unix_ms + 1,
    )
    .unwrap();
    let inventoried_at = inventoried.status.job.updated_at_unix_ms;
    assert_eq!(inventoried.status.job.state, State::Running);
    assert_eq!(inventoried.status.job.checkpoints.len(), 2);
    let checkpoint = inventoried.status.job.checkpoints.last().unwrap();
    assert_eq!(checkpoint.phase, Phase::Inventory);
    let scope = ScopeId::new(format!("instance:{}", engine.instance_id())).unwrap();
    let read = engine.storage.runtime().read_stamp(&scope).unwrap();
    assert_eq!(checkpoint.coordinates.runtime_cursor, read.commit_cursor);
    assert_eq!(
        checkpoint.coordinates.runtime_manifest_sha256,
        read.manifest_id
    );
    let replay = call(
        &engine,
        &session,
        &step(&connected.status.job),
        "inventory-step",
        inventoried_at + 1,
    )
    .unwrap();
    assert!(replay.idempotent_replay);
    assert_eq!(replay.status, inventoried.status);
    assert_eq!(engine.storage.runtime().read_stamp(&scope).unwrap(), read);
    let changes = engine
        .storage
        .runtime()
        .read_changes(&read, 0, 512)
        .unwrap();
    assert!(changes.changes.iter().any(|change| matches!(&change.mutation,
        RuntimeMutation::Record { record } if record.reference.kind.as_str() == "source-tree-snapshot")));
    assert!(changes.changes.iter().any(|change| matches!(&change.mutation,
        RuntimeMutation::Relation { relation } if relation.reference.kind.as_str() == "source-tree-contains")));
    drop(engine);
    let engine = installed.reopen(inventoried_at + 2);
    let session = installed.session(&engine, "inventory-reopen", inventoried_at + 3);
    assert_eq!(
        installed
            .read(&engine, &session, inventoried_at + 4)
            .job
            .checkpoints,
        inventoried.status.job.checkpoints
    );
    std::fs::rename(
        installed.root.path().join("before.txt"),
        installed.root.path().join("after.txt"),
    )
    .unwrap();
    std::fs::remove_file(installed.root.path().join("removed.txt")).unwrap();
    std::fs::write(installed.root.path().join("added.txt"), "added content\n").unwrap();
    std::fs::write(
        installed.root.path().join("README.md"),
        "edited after Inventory\n",
    )
    .unwrap();
    assert!(matches!(
        call(
            &engine,
            &session,
            &step(&inventoried.status.job),
            "inventory-stale",
            inventoried_at + 5
        ),
        Err(ServiceError::AttunementSourceChanged)
    ));
    let replan = engine
        .preview_attunement_replan(
            &session.session_id,
            &session.token,
            &installed.job,
            inventoried.status.job.revision,
            inventoried_at + 6,
            "request-inventory-replan",
            "operation-inventory-replan",
        )
        .unwrap();
    let accepted = engine
        .accept_attunement_replan(
            &session.session_id,
            &session.token,
            &replan,
            &replan.proposal_sha256,
            &context("inventory-accept"),
            inventoried_at + 7,
        )
        .unwrap();
    let successor = &accepted.successor.job;
    let started = call(
        &engine,
        &session,
        &AttunementCommand::Start {
            job_id: successor.id.clone(),
            plan_sha256: successor.plan_sha256.clone(),
            expected_revision: successor.revision,
            lease_ms: 30_000,
        },
        "inventory-successor-start",
        successor.updated_at_unix_ms + 1,
    )
    .unwrap();
    let connected = call(
        &engine,
        &session,
        &step(&started.status.job),
        "inventory-successor-connect",
        started.status.job.updated_at_unix_ms + 1,
    )
    .unwrap();
    let next = call(
        &engine,
        &session,
        &step(&connected.status.job),
        "inventory-successor-step",
        connected.status.job.updated_at_unix_ms + 1,
    )
    .unwrap();
    assert_eq!(next.status.job.checkpoints.len(), 2);
    assert_ne!(
        next.status.job.checkpoints[1].output_sha256,
        checkpoint.output_sha256
    );
    let new_read = engine.storage.runtime().read_stamp(&scope).unwrap();
    let delta = engine
        .storage
        .runtime()
        .read_changes(&new_read, read.commit_cursor, 512)
        .unwrap();
    assert!(delta.changes.iter().any(|change| matches!(&change.mutation,
        RuntimeMutation::Record { record } if record.reference.kind.as_str() == "source-tree-change-set"
            && matches!(record.properties.get("modified"), Some(RuntimeValue::List(values)) if !values.is_empty()))));
    let change_set = delta
        .changes
        .iter()
        .find_map(|change| match &change.mutation {
            RuntimeMutation::Record { record }
                if record.reference.kind.as_str() == "source-tree-change-set" =>
            {
                Some(record)
            }
            _ => None,
        })
        .unwrap();
    for field in ["added", "removed"] {
        assert!(
            matches!(change_set.properties.get(field), Some(RuntimeValue::List(values)) if values.len() == 2)
        );
    }
    assert!(
        matches!(change_set.properties.get("rename_candidates"), Some(RuntimeValue::List(values)) if values.len() == 1)
    );
    let retired: Vec<_> = delta
        .changes
        .iter()
        .filter_map(|change| match &change.mutation {
            RuntimeMutation::Retire { retirement }
                if retirement.reference.kind.as_str() == "source-tree-entry" =>
            {
                Some(retirement.reference.clone())
            }
            _ => None,
        })
        .collect();
    assert_eq!(retired.len(), 2);
    for path in ["before.txt", "removed.txt"] {
        let original = changes
            .changes
            .iter()
            .find_map(|change| match &change.mutation {
                RuntimeMutation::Record { record }
                    if record.reference.kind.as_str() == "source-tree-entry"
                        && record.properties.get("path")
                            == Some(&RuntimeValue::String(path.into())) =>
                {
                    Some(&record.reference)
                }
                _ => None,
            })
            .unwrap();
        assert!(retired.contains(original));
    }
}

#[test]
pub(super) fn inventory_source_rejection_preserves_runtime_and_allows_retry() {
    let (installed, engine, session) = Installed::new(InstallationTargetKind::ExistingProject);
    let started = call(
        &engine,
        &session,
        &installed.start(30_000),
        "retry-start",
        BASE + 1_000,
    )
    .unwrap();
    let connected = call(
        &engine,
        &session,
        &step(&started.status.job),
        "retry-connect",
        started.status.job.updated_at_unix_ms + 1,
    )
    .unwrap();
    let connected_at = connected.status.job.updated_at_unix_ms;
    let scope = ScopeId::new(format!("instance:{}", engine.instance_id())).unwrap();
    let before = engine.storage.runtime().read_stamp(&scope).unwrap();
    let head = format!("server/state/{}/inventory/head", engine.instance_id());
    let original = std::fs::read(installed.root.path().join("README.md")).unwrap();
    std::fs::write(installed.root.path().join("README.md"), "changed input\n").unwrap();
    assert!(matches!(
        call(
            &engine,
            &session,
            &step(&connected.status.job),
            "retry-inventory",
            connected_at + 1
        ),
        Err(ServiceError::AttunementSourceChanged)
    ));
    assert_eq!(engine.storage.runtime().read_stamp(&scope).unwrap(), before);
    assert!(engine.storage.control().get(&head).unwrap().is_none());
    assert_eq!(
        installed.read(&engine, &session, connected_at + 2).job,
        connected.status.job
    );
    std::fs::write(installed.root.path().join("README.md"), original).unwrap();
    let accepted = call(
        &engine,
        &session,
        &step(&connected.status.job),
        "retry-inventory",
        connected_at + 3,
    )
    .unwrap();
    assert!(
        !accepted.idempotent_replay,
        "a failed attempt must not consume its receipt key"
    );
    assert_eq!(accepted.status.job.checkpoints.len(), 2);
    assert!(engine.storage.control().get(&head).unwrap().is_some());
}

#[test]
pub(super) fn inventory_larger_than_one_physical_batch_publishes_and_reopens() {
    let (installed, engine, session) =
        Installed::with_source(InstallationTargetKind::ExistingProject, None, |root| {
            let source = root.join("src");
            std::fs::create_dir(&source).unwrap();
            for index in 0..2048 {
                std::fs::write(
                    source.join(format!("entry-{index:04}.txt")),
                    format!("source entry {index}\n"),
                )
                .unwrap();
            }
        });
    let started = call(
        &engine,
        &session,
        &installed.start(60_000),
        "large-start",
        BASE + 1_000,
    )
    .unwrap();
    let connected = call(
        &engine,
        &session,
        &step(&started.status.job),
        "large-connect",
        started.status.job.updated_at_unix_ms + 1,
    )
    .unwrap();
    let inventoried = call(
        &engine,
        &session,
        &step(&connected.status.job),
        "large-inventory",
        connected.status.job.updated_at_unix_ms + 1,
    )
    .unwrap();
    assert_eq!(inventoried.status.job.checkpoints.len(), 2);
    assert_eq!(
        inventoried.status.job.checkpoints[1].phase,
        Phase::Inventory
    );
    let groups = engine
        .storage
        .control()
        .journal_since(0, 1024)
        .unwrap()
        .into_iter()
        .filter(|entry| entry.action == "attunement.inventory_group_published")
        .count();
    assert!(
        groups > 1,
        "the whole input must cross the native atomic batch boundary"
    );
    let reopened_at = inventoried.status.job.updated_at_unix_ms + 1_000;
    drop(engine);
    let engine = installed.reopen(reopened_at);
    let session = installed.session(&engine, "large-reopen", reopened_at + 100);
    assert_eq!(
        installed.read(&engine, &session, reopened_at + 200).job,
        inventoried.status.job
    );
}

#[test]
pub(super) fn inventory_output_bound_preserves_runtime_checkpoint_and_head_after_reopen() {
    let defaults = rrd_contract::EstateConfiguration::default();
    let input = rrd_contract::EstateConfigurationInput {
        format_version: defaults.format_version,
        clock: defaults.clock,
        storage: rrd_contract::StorageLimits {
            transaction_admission_bytes: 16_384,
            ..defaults.storage
        },
        reasoning: defaults.reasoning,
        recall: defaults.recall,
        query: defaults.query,
        attunement_follow: None,
    };
    let file = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(file.path(), toml::to_string(&input).unwrap()).unwrap();
    let (installed, engine, session) = Installed::with_configuration(
        InstallationTargetKind::ExistingProject,
        None,
        |root| {
            for index in 0..64 {
                std::fs::write(root.join(format!("source-{index}.txt")), "bounded source\n")
                    .unwrap();
            }
        },
        Some(file.path()),
    );
    let started = call(
        &engine,
        &session,
        &installed.start(30_000),
        "bound-start",
        BASE + 1_000,
    )
    .unwrap();
    let connected = call(
        &engine,
        &session,
        &step(&started.status.job),
        "bound-connect",
        started.status.job.updated_at_unix_ms + 1,
    )
    .unwrap();
    let connected_at = connected.status.job.updated_at_unix_ms;
    let scope = ScopeId::new(format!("instance:{}", engine.instance_id())).unwrap();
    let before = engine.storage.runtime().read_stamp(&scope).unwrap();
    let head = format!("server/state/{}/inventory/head", engine.instance_id());
    assert!(matches!(
        call(
            &engine,
            &session,
            &step(&connected.status.job),
            "bound-inventory",
            connected_at + 1
        ),
        Err(ServiceError::ConfigurationLimit(_))
    ));
    assert_eq!(engine.storage.runtime().read_stamp(&scope).unwrap(), before);
    assert!(engine.storage.control().get(&head).unwrap().is_none());
    assert_eq!(
        installed.read(&engine, &session, connected_at + 2).job,
        connected.status.job
    );
    assert_eq!(
        engine
            .transaction_admission_snapshot()
            .unwrap()
            .logical_bytes,
        0
    );
    drop(engine);
    let engine = installed.reopen(connected_at + 3);
    let session = installed.session(&engine, "bound-reopen", connected_at + 4);
    assert_eq!(engine.storage.runtime().read_stamp(&scope).unwrap(), before);
    assert!(engine.storage.control().get(&head).unwrap().is_none());
    assert_eq!(
        installed.read(&engine, &session, connected_at + 5).job,
        connected.status.job
    );
}

#[test]
pub(super) fn inventory_interruption_preserves_incomplete_groups_and_resumes_after_reopen() {
    partial_inventory_recovery(false);
}

#[test]
pub(super) fn inventory_edit_during_publication_rejects_stale_completion_and_allows_replan() {
    partial_inventory_recovery(true);
}

pub(super) fn partial_inventory_recovery(edit_source: bool) {
    let (installed, engine, session) =
        Installed::with_source(InstallationTargetKind::ExistingProject, None, |root| {
            let source = root.join("src");
            std::fs::create_dir(&source).unwrap();
            for index in 0..2048 {
                std::fs::write(
                    source.join(format!("entry-{index:04}.txt")),
                    format!("source entry {index}\n"),
                )
                .unwrap();
            }
        });
    let started = call(
        &engine,
        &session,
        &installed.start(60_000),
        "interrupt-start",
        BASE + 1_000,
    )
    .unwrap();
    let connected = call(
        &engine,
        &session,
        &step(&started.status.job),
        "interrupt-connect",
        started.status.job.updated_at_unix_ms + 1,
    )
    .unwrap();
    let at = connected.status.job.updated_at_unix_ms + 1;
    let progress_key = format!(
        "server/state/{}/inventory/progress/{}/1",
        engine.instance_id(),
        installed.job
    );
    let head_key = format!("server/state/{}/inventory/head", engine.instance_id());
    let scope_id = ScopeId::new(format!("instance:{}", engine.instance_id())).unwrap();
    std::thread::scope(|scope| {
        let (sender, receiver) = std::sync::mpsc::channel();
        let engine = &engine;
        let session = &session;
        let command = step(&connected.status.job);
        let worker = scope.spawn(move || {
            let authorized = engine
                .begin_invocation(
                    Invocation {
                        context: context("interrupt-inventory"),
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
                    RrdOperation::EstateAdmin,
                    InvocationCredential::Session {
                        session_id: &session.session_id,
                        token: &session.token,
                    },
                )
                .unwrap();
            sender.send(authorized).unwrap();
            call(engine, session, &command, "interrupt-inventory", at)
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
                "Inventory finished before recording progress"
            );
            assert!(deadline.elapsed() < std::time::Duration::from_secs(60));
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        if edit_source {
            std::fs::write(
                installed.root.path().join("README.md"),
                "edited during publication\n",
            )
            .unwrap();
            assert!(matches!(
                worker.join().unwrap(),
                Err(ServiceError::AttunementSourceChanged)
            ));
        } else {
            authorized.cancel_pending();
            assert!(matches!(
                worker.join().unwrap(),
                Err(ServiceError::AdmissionCancelled)
            ));
        }
    });
    assert!(engine.storage.control().get(&head_key).unwrap().is_none());
    let partial = engine.storage.runtime().read_stamp(&scope_id).unwrap();
    assert!(
        partial.commit_cursor
            > connected.status.job.checkpoints[0]
                .coordinates
                .runtime_cursor
    );
    assert_eq!(
        installed.read(&engine, &session, at + 30_000).job,
        connected.status.job
    );
    let partial_rows = engine
        .storage
        .runtime()
        .read_changes(&partial, 0, 10_000)
        .unwrap();
    assert!(!partial_rows.changes.iter().any(|change| matches!(&change.mutation,
        RuntimeMutation::Record { record } if record.reference.kind.as_str() == "source-tree-snapshot")));
    let reopened_at = connected
        .status
        .job
        .lease
        .as_ref()
        .unwrap()
        .expires_at_unix_ms
        + 1_000;
    drop(engine);
    let engine = installed.reopen(reopened_at);
    let session = installed.session(&engine, "interrupt-reopen", reopened_at + 100);
    let recovered = call(
        &engine,
        &session,
        &AttunementCommand::RecoverExpired {
            job_id: installed.job.clone(),
            expected_revision: connected.status.job.revision,
        },
        "interrupt-recover",
        reopened_at + 200,
    )
    .unwrap();
    let recovered_at = recovered.status.job.updated_at_unix_ms;
    let resumed = call(
        &engine,
        &session,
        &resume(&recovered.status.job, recovered_at + 1),
        "interrupt-resume",
        recovered_at + 1,
    );
    if edit_source {
        assert!(matches!(
            resumed,
            Err(ServiceError::AttunementSourceChanged)
        ));
        assert!(engine.storage.control().get(&head_key).unwrap().is_none());
        let replan = engine
            .preview_attunement_replan(
                &session.session_id,
                &session.token,
                &installed.job,
                recovered.status.job.revision,
                recovered_at + 2,
                "request-partial-replan",
                "operation-partial-replan",
            )
            .unwrap();
        let accepted = engine
            .accept_attunement_replan(
                &session.session_id,
                &session.token,
                &replan,
                &replan.proposal_sha256,
                &context("partial-accept"),
                recovered_at + 3,
            )
            .unwrap();
        assert_ne!(
            accepted.successor.job.plan_sha256,
            connected.status.job.plan_sha256
        );
        assert!(accepted.successor.job.checkpoints.is_empty());
        assert!(engine.storage.control().get(&head_key).unwrap().is_none());
        return;
    }
    let resumed = resumed.unwrap();
    let complete = call(
        &engine,
        &session,
        &step(&resumed.status.job),
        "interrupt-continue",
        resumed.status.job.updated_at_unix_ms + 1,
    )
    .unwrap();
    assert_eq!(complete.status.job.checkpoints.len(), 2);
    assert!(engine
        .storage
        .control()
        .get(&progress_key)
        .unwrap()
        .is_none());
    assert!(engine.storage.control().get(&head_key).unwrap().is_some());
    let read = engine.storage.runtime().read_stamp(&scope_id).unwrap();
    let changes = engine
        .storage
        .runtime()
        .read_changes(&read, 0, 10_000)
        .unwrap();
    let identities: Vec<_> = changes
        .changes
        .iter()
        .filter_map(|change| match &change.mutation {
            RuntimeMutation::Record { record }
                if record.reference.kind.as_str() == "source-tree-entry" =>
            {
                Some(record.reference.clone())
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        identities.len(),
        2052, // Root, src, README, the excluded .rrflow marker, and 2,048 files.
        "resume must not republish already committed groups"
    );
    assert_eq!(
        identities
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        identities.len()
    );
}

#[test]
pub(super) fn git_selected_plan_ignores_irrelevant_edits_replans_tracked_edits_and_reopens() {
    selected_plan_replans_after_progress_and_reopens(true);
}
