//! Head, index, ignore-rule and staging changes around a selected source.

use super::*;

#[test]
pub(super) fn unrelated_accepted_head_advance_before_dispatch_keeps_selected_worker_source() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let mut fixture = source_worker_fixture(&listener, true);
    let original_snapshot = crate::engine::attunement::load_accepted_source(
        &fixture.engine,
        &fixture.installed.preview.installation.target.project_id,
        8 * 1024 * 1024,
        10_000,
    )
    .unwrap()
    .proposal_sha256;
    let current_snapshot = refresh_unrelated(
        fixture.installed.root.path(),
        &fixture.engine,
        &fixture.lease,
        &fixture.installed.preview.installation.target.project_id,
        &mut fixture.job,
        "beta.txt",
    );
    fixture.now = fixture.job.updated_at_unix_ms + 1;
    assert_ne!(current_snapshot, original_snapshot);
    let mut request = selected_request(
        &fixture.engine,
        &fixture.lease,
        &listener,
        &fixture.selection,
        fixture.now,
        false,
    );
    request.source = Some(fixture.source.clone());
    let budget_before = inspect_budget(&fixture, fixture.now + 1);
    let (output, (body, _response)) = std::thread::scope(|scope| {
        let server = scope.spawn(|| {
            let (mut socket, body) = read_wire(&listener);
            let worker_wire: serde_json::Value = serde_json::from_slice(&body).unwrap();
            assert_eq!(
                worker_packet_from_chat_wire(&worker_wire)["source"]["content"],
                fixture.source_content
            );
            assert_eq!(
                worker_packet_from_chat_wire(&worker_wire)["source"]["provenance"]
                    ["captured_snapshot_sha256"],
                current_snapshot
            );
            let response = reply(
                &mut socket,
                worker_wire["model"].as_str().unwrap(),
                "Reviewed one file.",
            );
            (body, response)
        });
        let output = fixture
            .engine
            .execute_task_worker_with_clock(
                &fixture.lease.session_id,
                &fixture.lease.token,
                &request,
                "source-worker-execute",
                "source-worker-execute",
                &FixedClock::at(fixture.now + 1),
            )
            .unwrap();
        (output, server.join().unwrap())
    });
    let wire: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        worker_packet_from_chat_wire(&wire)["source"]["content"],
        fixture.source_content
    );
    let provenance = output.source.as_ref().unwrap();
    assert_eq!(provenance.captured_snapshot_sha256, current_snapshot);
    assert_eq!(provenance.validated_snapshot_sha256, current_snapshot);
    assert_eq!(output.content, "Reviewed one file.");
    let budget_after = inspect_budget(&fixture, fixture.now + 2);
    assert_eq!(
        budget_after.reserved_steps,
        budget_before.reserved_steps + 1
    );
    assert_eq!(
        budget_after.deadline_unix_ms,
        budget_before.deadline_unix_ms
    );
    assert!(!budget_after.execution_may_continue);
    assert!(budget_after.active_attempt.is_none());
    drop(fixture.engine);
    let reopened = fixture.installed.reopen(fixture.now + 3);
    let readback = reopened
        .read_task_attempt(
            &fixture.lease.session_id,
            &fixture.lease.token,
            &ReadTaskAttempt {
                scope: request.scope.clone(),
                task_id: request.task_id.clone(),
                attempt_id: request.attempt_id.clone(),
            },
            fixture.now + 3,
            "source-worker-readback",
            "source-worker-readback",
        )
        .unwrap();
    assert_eq!(readback.attempt.unwrap().worker_output.unwrap(), output);
    let reauthenticated = reopened
        .create_authenticated_session(
            &fixture.principal_id,
            fixture.worker_token.as_str().as_bytes(),
            &crate::engine::tests::session_request(10_000, 2),
            &id("source-worker-reauthenticated-session"),
            fixture.now + 4,
            "source-worker-reauthenticated-session",
            "source-worker-reauthenticated-session",
        )
        .unwrap();
    let reauthenticated_readback = reopened
        .read_task_attempt(
            &reauthenticated.session_id,
            &reauthenticated.token,
            &ReadTaskAttempt {
                scope: request.scope.clone(),
                task_id: request.task_id.clone(),
                attempt_id: request.attempt_id.clone(),
            },
            fixture.now + 5,
            "source-worker-readback-reauthenticated",
            "source-worker-readback-reauthenticated",
        )
        .unwrap();
    assert_eq!(
        reauthenticated_readback
            .attempt
            .unwrap()
            .worker_output
            .unwrap(),
        output
    );
    assert_eq!(
        std::fs::read_to_string(fixture.installed.root.path().join(&fixture.source_path)).unwrap(),
        fixture.source_content
    );
    std::fs::write(
        fixture.installed.root.path().join(&fixture.source_path),
        "later authorized source edit\n",
    )
    .unwrap();
    let historical = reopened
        .read_task_attempt(
            &reauthenticated.session_id,
            &reauthenticated.token,
            &ReadTaskAttempt {
                scope: request.scope.clone(),
                task_id: request.task_id.clone(),
                attempt_id: request.attempt_id.clone(),
            },
            fixture.now + 6,
            "source-worker-readback-after-content-change",
            "source-worker-readback-after-content-change",
        )
        .unwrap();
    let historical_output = historical.attempt.unwrap().worker_output.unwrap();
    assert_eq!(historical_output.content, "Reviewed one file.");
    assert_eq!(
        historical_output.source.unwrap().content_sha256,
        output.source.as_ref().unwrap().content_sha256
    );
    assert_ne!(
        digest::sha256_hex(b"later authorized source edit\n"),
        output.source.as_ref().unwrap().content_sha256
    );
    set_worker_export_flags(
        &reopened,
        &reauthenticated,
        fixture.now + 7,
        true,
        false,
        "source-disabled",
    );
    let source_export_hidden = reopened
        .read_task_attempt(
            &reauthenticated.session_id,
            &reauthenticated.token,
            &ReadTaskAttempt {
                scope: request.scope.clone(),
                task_id: request.task_id.clone(),
                attempt_id: request.attempt_id.clone(),
            },
            fixture.now + 8,
            "source-worker-readback-source-export-disabled",
            "source-worker-readback-source-export-disabled",
        )
        .unwrap();
    assert!(source_export_hidden
        .attempt
        .unwrap()
        .worker_output
        .is_none());
    set_worker_export_flags(
        &reopened,
        &reauthenticated,
        fixture.now + 9,
        false,
        true,
        "context-disabled",
    );
    let context_export_hidden = reopened
        .read_task_attempt(
            &reauthenticated.session_id,
            &reauthenticated.token,
            &ReadTaskAttempt {
                scope: request.scope.clone(),
                task_id: request.task_id.clone(),
                attempt_id: request.attempt_id.clone(),
            },
            fixture.now + 10,
            "source-worker-readback-context-export-disabled",
            "source-worker-readback-context-export-disabled",
        )
        .unwrap();
    assert!(context_export_hidden
        .attempt
        .unwrap()
        .worker_output
        .is_none());
    set_worker_export_flags(
        &reopened,
        &reauthenticated,
        fixture.now + 11,
        true,
        true,
        "exports-restored",
    );
    revoke_source_read(
        &reopened,
        &reauthenticated,
        &fixture.principal_id,
        fixture.now + 12,
    );
    let revoked_readback = reopened
        .read_task_attempt(
            &reauthenticated.session_id,
            &reauthenticated.token,
            &ReadTaskAttempt {
                scope: request.scope.clone(),
                task_id: request.task_id.clone(),
                attempt_id: request.attempt_id.clone(),
            },
            fixture.now + 13,
            "source-worker-readback-revoked",
            "source-worker-readback-revoked",
        )
        .unwrap();
    let revoked_attempt = revoked_readback.attempt.unwrap();
    assert_eq!(revoked_attempt.status, TaskAttemptStatus::Accepted);
    assert!(revoked_attempt.worker_output.is_none());
}

#[test]
pub(super) fn unrelated_git_index_and_nonmatching_ignore_changes_keep_selected_worker_source() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let mut fixture = source_worker_fixture_with_git(&listener, true, true);
    let original_snapshot = crate::engine::attunement::load_accepted_source(
        &fixture.engine,
        &fixture.installed.preview.installation.target.project_id,
        8 * 1024 * 1024,
        10_000,
    )
    .unwrap()
    .proposal_sha256;
    std::fs::write(
        fixture.installed.root.path().join(".git/index"),
        include_bytes!(
            "../../../../../../../../compute/rrd-attunement/tests/fixtures/git-index/sha1-v3.index"
        ),
    )
    .unwrap();
    std::fs::remove_file(fixture.installed.root.path().join(".gitignore")).unwrap();
    std::fs::remove_file(fixture.installed.root.path().join("nested/.gitignore")).unwrap();
    let current_snapshot = refresh_unrelated(
        fixture.installed.root.path(),
        &fixture.engine,
        &fixture.lease,
        &fixture.installed.preview.installation.target.project_id,
        &mut fixture.job,
        "beta.txt",
    );
    fixture.now = fixture.job.updated_at_unix_ms + 1;
    assert_ne!(current_snapshot, original_snapshot);
    let mut request = selected_request(
        &fixture.engine,
        &fixture.lease,
        &listener,
        &fixture.selection,
        fixture.now,
        false,
    );
    request.source = Some(fixture.source.clone());
    let (output, (body, response)) = std::thread::scope(|scope| {
        let server = scope.spawn(|| {
            let (mut socket, body) = read_wire(&listener);
            let wire: serde_json::Value = serde_json::from_slice(&body).unwrap();
            assert_eq!(
                worker_packet_from_chat_wire(&wire)["source"]["content"],
                fixture.source_content
            );
            let response = reply(
                &mut socket,
                wire["model"].as_str().unwrap(),
                "Reviewed one file.",
            );
            (body, response)
        });
        let output = fixture
            .engine
            .execute_task_worker_with_clock(
                &fixture.lease.session_id,
                &fixture.lease.token,
                &request,
                "source-worker-git-before",
                "source-worker-git-before",
                &FixedClock::at(fixture.now + 1),
            )
            .unwrap();
        (output, server.join().unwrap())
    });
    let wire: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        worker_packet_from_chat_wire(&wire)["source"]["content"],
        fixture.source_content
    );
    assert_eq!(output.content, "Reviewed one file.");
    assert_eq!(
        output.source.unwrap().captured_snapshot_sha256,
        current_snapshot
    );
    assert!(!response.is_empty());
}

#[test]
pub(super) fn persisted_legacy_inventory_reopens_and_eligible_replan_publishes_source_file() {
    let _legacy =
        crate::engine::attunement::LegacySourceFileProjectionGuard::omit_for_current_thread();
    let (installed, engine, lease, mut job, scope, _) =
        first_inventory_for_worker_with_setup_at(|_| {}, crate::engine::tests::attunement::BASE);
    let project_id = installed.preview.installation.target.project_id.clone();
    let legacy = crate::engine::attunement::load_accepted_source(
        &engine,
        &project_id,
        8 * 1024 * 1024,
        10_000,
    )
    .unwrap();
    let source_entry = legacy
        .proposal
        .entries()
        .iter()
        .find(|entry| entry.path == "alpha.txt")
        .unwrap()
        .clone();
    let source_sha256 = match &source_entry.kind {
        rrd_attunement::inventory::EntryKind::File { content_sha256, .. } => content_sha256.clone(),
        _ => panic!("fixture source entry is a regular file"),
    };
    let legacy_digest = legacy.proposal_sha256.clone();
    let entry_type = rrd_core::RuntimeType::new("source-tree-entry").unwrap();
    let legacy_schema = engine.storage.runtime().schema(&scope).unwrap().unwrap();
    assert!(!legacy_schema.records[&entry_type]
        .properties
        .contains_key("source_file"));
    let unavailable = engine.execute_query(
        &lease.session_id,
        &lease.token,
        &ExecuteQuery {
            scope: format!("instance:{}", engine.instance_id()),
            query: format!(
                "FROM record:source-tree-entry AT VALID {} KNOWN HEAD WHERE identity = \"{}\" PROJECT source_file",
                job.updated_at_unix_ms + 1,
                source_entry.identity
            ),
            parameters: Default::default(),
            budget: QueryBudget::default(),
        },
        job.updated_at_unix_ms + 2,
        "source-worker-legacy-file-unavailable",
        "source-worker-legacy-file-unavailable",
    );
    assert!(unavailable.is_err());

    drop(_legacy);
    drop(engine);
    let reopened = installed.reopen(job.updated_at_unix_ms + 3);
    let reopened_legacy = crate::engine::attunement::load_accepted_source(
        &reopened,
        &project_id,
        8 * 1024 * 1024,
        10_000,
    )
    .unwrap();
    assert_eq!(reopened_legacy.proposal_sha256, legacy_digest);
    assert_eq!(
        reopened_legacy.proposal.entries(),
        legacy.proposal.entries()
    );

    let replan_floor = job.updated_at_unix_ms + 3;
    refresh_inventory_without_source_edits(&reopened, &lease, &mut job, replan_floor);
    let refreshed = crate::engine::attunement::load_accepted_source(
        &reopened,
        &project_id,
        8 * 1024 * 1024,
        10_000,
    )
    .unwrap();
    let legacy_value = serde_json::to_value(&legacy.proposal).unwrap();
    let refreshed_value = serde_json::to_value(&refreshed.proposal).unwrap();
    let legacy_fields = legacy_value.as_object().unwrap();
    let refreshed_fields = refreshed_value.as_object().unwrap();
    let changed_fields = legacy_fields
        .iter()
        .filter_map(|(name, value)| {
            (refreshed_fields.get(name) != Some(value)).then_some(name.clone())
        })
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        changed_fields,
        std::collections::BTreeSet::from([
            "previous_sha256".to_owned(),
            "changes".to_owned(),
            "proposal_sha256".to_owned(),
        ])
    );
    assert_eq!(legacy.proposal.entries(), refreshed.proposal.entries());
    assert_eq!(
        refreshed
            .proposal
            .entries()
            .iter()
            .find(|entry| entry.path == "alpha.txt")
            .unwrap()
            .identity,
        source_entry.identity
    );
    let refreshed_schema = reopened.storage.runtime().schema(&scope).unwrap().unwrap();
    assert!(refreshed_schema.records[&entry_type]
        .properties
        .contains_key("source_file"));
    let available = reopened
        .execute_query(
            &lease.session_id,
            &lease.token,
            &ExecuteQuery {
                scope: format!("instance:{}", reopened.instance_id()),
                query: format!(
                    "FROM record:source-tree-entry AT VALID {} KNOWN HEAD WHERE identity = \"{}\" PROJECT identity, path, source_file",
                    job.updated_at_unix_ms + 1,
                    source_entry.identity
                ),
                parameters: Default::default(),
                budget: QueryBudget::default(),
            },
            job.updated_at_unix_ms + 2,
            "source-worker-legacy-file-republished",
            "source-worker-legacy-file-republished",
        )
        .unwrap();
    assert_eq!(available.rows.len(), 1);
    let projection = match available.rows[0].values.get("source_file") {
        Some(QueryValue::Map(values)) => values,
        value => panic!("unexpected typed source_file projection: {value:?}"),
    };
    assert!(matches!(
        projection.get("content_sha256"),
        Some(QueryValue::String(value) | QueryValue::Digest(value)) if value == &source_sha256
    ));
    assert!(matches!(
        projection.get("bytes"),
        Some(QueryValue::Unsigned(value)) if *value == "source alpha.txt\n".len() as u64
    ));
}

#[test]
pub(super) fn newly_applicable_ignore_rule_denies_source_before_provider_dispatch() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let fixture = source_worker_fixture_with_git(&listener, true, true);
    let mut request = selected_request(
        &fixture.engine,
        &fixture.lease,
        &listener,
        &fixture.selection,
        fixture.now,
        false,
    );
    request.source = Some(fixture.source.clone());
    std::fs::write(
        fixture.installed.root.path().join("nested/.gitignore"),
        b"alpha.txt\n",
    )
    .unwrap();
    let before = inspect_budget(&fixture, fixture.now + 1);
    assert!(matches!(
        fixture.engine.execute_task_worker_with_clock(
            &fixture.lease.session_id,
            &fixture.lease.token,
            &request,
            "source-worker-git-excluded",
            "source-worker-git-excluded",
            &FixedClock::at(fixture.now + 1),
        ),
        Err(ServiceError::ProjectBindingMismatch)
    ));
    let after = inspect_budget(&fixture, fixture.now + 2);
    assert_eq!(after.reserved_steps, before.reserved_steps);
    assert!(
        matches!(listener.accept(), Err(error) if error.kind() == std::io::ErrorKind::WouldBlock)
    );
}

#[test]
pub(super) fn tracked_index_entry_overrides_ignore_for_selected_source() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let fixture = source_worker_fixture_for_path(&listener, true, true, "ignored/tracked.txt");
    let mut request = selected_request(
        &fixture.engine,
        &fixture.lease,
        &listener,
        &fixture.selection,
        fixture.now,
        false,
    );
    request.source = Some(fixture.source.clone());
    let (output, body) = std::thread::scope(|scope| {
        let server = scope.spawn(|| {
            let (mut socket, body) = read_wire(&listener);
            let wire: serde_json::Value = serde_json::from_slice(&body).unwrap();
            assert_eq!(
                worker_packet_from_chat_wire(&wire)["source"]["content"],
                fixture.source_content
            );
            let response = reply(
                &mut socket,
                wire["model"].as_str().unwrap(),
                "Reviewed tracked file.",
            );
            (body, response)
        });
        let output = fixture
            .engine
            .execute_task_worker_with_clock(
                &fixture.lease.session_id,
                &fixture.lease.token,
                &request,
                "source-worker-tracked-override",
                "source-worker-tracked-override",
                &FixedClock::at(fixture.now + 1),
            )
            .unwrap();
        (output, server.join().unwrap())
    });
    let wire: serde_json::Value = serde_json::from_slice(&body.0).unwrap();
    assert_eq!(
        worker_packet_from_chat_wire(&wire)["source"]["content"],
        fixture.source_content
    );
    assert_eq!(output.content, "Reviewed tracked file.");
    assert!(!body.1.is_empty());
}

#[test]
pub(super) fn unrelated_git_stage_does_not_reset_source_repetition_ceiling() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = host_unix_ms().saturating_sub(30_000);
    let mut fixture =
        source_worker_fixture_for_path_at(&listener, true, true, "nested/alpha.txt", base);
    fixture
        .engine
        .configure_task_budget(
            &fixture.lease.session_id,
            &fixture.lease.token,
            &rrd_contract::ConfigureTaskBudget {
                scope: fixture.selection.scope.clone(),
                task_id: fixture.selection.task_id.clone(),
                limits: fixture.engine.estate_configuration.reasoning.clone(),
                repetition_policy: Some(rrd_contract::TaskRepetitionPolicy {
                    max_equivalent_attempts: 1,
                }),
            },
            host_unix_ms(),
            "source-worker-repetition-configure",
            "source-worker-repetition-configure",
        )
        .unwrap();
    let mut request = selected_request_host(
        &fixture.engine,
        &fixture.lease,
        &listener,
        &fixture.selection,
    );
    request.source = Some(fixture.source.clone());
    let (first_output, (body, _response)) = std::thread::scope(|scope| {
        let server = scope.spawn(|| {
            let (mut socket, body) = read_wire(&listener);
            let wire: serde_json::Value = serde_json::from_slice(&body).unwrap();
            assert_eq!(
                worker_packet_from_chat_wire(&wire)["source"]["content"],
                fixture.source_content
            );
            let response = reply(
                &mut socket,
                wire["model"].as_str().unwrap(),
                "One source draft.",
            );
            (body, response)
        });
        let output = fixture
            .engine
            .execute_task_worker(
                &fixture.lease.session_id,
                &fixture.lease.token,
                &request,
                "source-worker-repetition-first",
                "source-worker-repetition-first",
            )
            .unwrap();
        (output, server.join().unwrap())
    });
    let first_wire: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(first_output.content, "One source draft.");
    assert!(!first_output
        .source
        .as_ref()
        .unwrap()
        .captured_snapshot_sha256
        .is_empty());
    assert!(!first_wire["model"].as_str().unwrap().is_empty());
    let budget_after_first = inspect_budget(&fixture, host_unix_ms());
    assert!(budget_after_first.reserved_steps > 0);
    assert!(budget_after_first.stopped.is_none());
    let project_id = fixture
        .installed
        .preview
        .installation
        .target
        .project_id
        .clone();
    let original = crate::engine::attunement::load_accepted_source(
        &fixture.engine,
        &project_id,
        8 * 1024 * 1024,
        10_000,
    )
    .unwrap();
    let original_entry = original
        .proposal
        .entries()
        .iter()
        .find(|entry| entry.path == fixture.source_path)
        .unwrap()
        .clone();
    std::fs::write(
        fixture.installed.root.path().join(".git/index"),
        include_bytes!(
            "../../../../../../../../compute/rrd-attunement/tests/fixtures/git-index/sha1-v3.index"
        ),
    )
    .unwrap();
    std::fs::write(
        fixture.installed.root.path().join(".gitignore"),
        b"*.tmp\nignored/\n",
    )
    .unwrap();
    std::fs::write(
        fixture.installed.root.path().join("nested/.gitignore"),
        b"*.bak\n",
    )
    .unwrap();
    let current = refresh_unrelated_with_host_floor(
        fixture.installed.root.path(),
        &fixture.engine,
        &fixture.lease,
        &fixture.installed.preview.installation.target.project_id,
        &mut fixture.job,
        "beta.txt",
        Some(host_unix_ms()),
    );
    fixture.now = host_unix_ms().max(fixture.job.updated_at_unix_ms + 1);
    assert_ne!(current, original.proposal_sha256);
    let refreshed = crate::engine::attunement::load_accepted_source(
        &fixture.engine,
        &project_id,
        8 * 1024 * 1024,
        10_000,
    )
    .unwrap();
    assert_eq!(current, refreshed.proposal_sha256);
    assert_ne!(
        original.proposal.revisions().policy_sha256,
        refreshed.proposal.revisions().policy_sha256
    );
    let refreshed_entry = refreshed
        .proposal
        .entries()
        .iter()
        .find(|entry| entry.path == fixture.source_path)
        .unwrap();
    assert_eq!(refreshed_entry.identity, original_entry.identity);
    assert_eq!(refreshed_entry.kind, original_entry.kind);
    assert_eq!(
        first_output.source.as_ref().unwrap().entry_identity,
        fixture.source.entry_identity
    );
    let mut equivalent = request.clone();
    equivalent.attempt_id = id("source-worker-repetition-second");
    let before_second = inspect_budget(&fixture, host_unix_ms());
    assert_eq!(
        before_second.reserved_steps,
        budget_after_first.reserved_steps
    );
    assert_eq!(
        before_second.charged_elapsed_ms,
        budget_after_first.charged_elapsed_ms
    );
    assert!(matches!(
        fixture.engine.execute_task_worker(
            &fixture.lease.session_id,
            &fixture.lease.token,
            &equivalent,
            "source-worker-repetition-second",
            "source-worker-repetition-second",
        ),
        Err(ServiceError::ReasoningBudgetStopped(
            rrd_contract::ReasoningBudgetStop::RepeatedWork
        ))
    ));
    let after_second = inspect_budget(&fixture, host_unix_ms());
    assert_eq!(after_second.reserved_steps, before_second.reserved_steps);
    assert_eq!(
        after_second.charged_elapsed_ms,
        before_second.charged_elapsed_ms
    );
    assert_eq!(
        after_second.stopped,
        Some(rrd_contract::ReasoningBudgetStop::RepeatedWork)
    );
    listener.set_nonblocking(true).unwrap();
    assert!(matches!(
        listener.accept(),
        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock
    ));
}
