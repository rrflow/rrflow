//! Replan tests: staleness, fencing, competing acceptances, crash replay and principal-scoped receipts.

use super::*;

#[test]
fn stale_attunement_can_be_replanned_without_reinstalling_or_losing_receipts() {
    let (installed, engine, session) = Installed::new(InstallationTargetKind::ExistingProject);
    std::fs::write(installed.root.path().join("README.md"), "first edit\n").unwrap();
    assert!(matches!(
        call(
            &engine,
            &session,
            &installed.start(30_000),
            "old-start",
            BASE + 1_000
        ),
        Err(ServiceError::AttunementSourceChanged)
    ));
    assert_eq!(
        installed.read(&engine, &session, BASE + 1_100).job.revision,
        1
    );
    assert!(engine
        .read_attunement_receipt(
            &session.session_id,
            &session.token,
            &installed.job,
            &id("old-start"),
            BASE + 1_110,
            "request-no-old-start-receipt",
            "operation-no-old-start-receipt"
        )
        .unwrap()
        .is_none());
    let proposal = engine
        .preview_attunement_replan(
            &session.session_id,
            &session.token,
            &installed.job,
            1,
            BASE + 1_200,
            "request-preview",
            "operation-preview",
        )
        .unwrap();
    let accepted = engine
        .accept_attunement_replan(
            &session.session_id,
            &session.token,
            &proposal,
            &proposal.proposal_sha256,
            &context("accept"),
            BASE + 1_300,
        )
        .unwrap();
    assert_ne!(accepted.successor.job.id, installed.job);
    assert_ne!(
        accepted.plan.plan_sha256,
        installed.preview.attunement.plan_sha256
    );
    assert_eq!(accepted.predecessor.job.state, State::CancelRequested);
    assert_eq!(accepted.successor.job.state, State::Pending);
    let start = AttunementCommand::Start {
        job_id: accepted.successor.job.id.clone(),
        expected_revision: 1,
        plan_sha256: accepted.plan.plan_sha256.clone(),
        lease_ms: 30_000,
    };
    let running = call(
        &engine,
        &session,
        &start,
        "new-start",
        accepted.successor.job.updated_at_unix_ms + 1,
    )
    .unwrap();
    let running_at = running.status.job.updated_at_unix_ms;
    std::fs::write(installed.root.path().join("README.md"), "second edit\n").unwrap();
    let work = step(&running.status.job);
    assert!(matches!(
        call(&engine, &session, &work, "stale-connect", running_at + 1),
        Err(ServiceError::AttunementSourceChanged)
    ));
    let current = engine
        .read_attunement(
            &session.session_id,
            &session.token,
            &accepted.successor.job.id,
            running_at + 2,
            "request-new-read",
            "operation-new-read",
        )
        .unwrap();
    assert!(current.job.checkpoints.is_empty());
    assert!(engine
        .read_attunement_receipt(
            &session.session_id,
            &session.token,
            &accepted.successor.job.id,
            &id("stale-connect"),
            running_at + 3,
            "request-no-stale-receipt",
            "operation-no-stale-receipt"
        )
        .unwrap()
        .is_none());
    drop(engine);
    let engine = installed.reopen(running_at + 4);
    let renewed = installed.session(&engine, "replan-reopened", running_at + 5);
    let replay = engine
        .accept_attunement_replan(
            &renewed.session_id,
            &renewed.token,
            &proposal,
            &proposal.proposal_sha256,
            &context("accept"),
            running_at + 6,
        )
        .unwrap();
    assert!(replay.idempotent_replay);
    assert_eq!(replay.successor, accepted.successor);
    assert_eq!(replay.predecessor, accepted.predecessor);
}

#[test]
fn replan_rechecks_inputs_and_fences_the_old_worker() {
    let (installed, engine, session) = Installed::new(InstallationTargetKind::ExistingProject);
    let running = call(
        &engine,
        &session,
        &installed.start(30_000),
        "old-running",
        BASE + 1_000,
    )
    .unwrap();
    let running_at = running.status.job.updated_at_unix_ms;
    let old_step = step(&running.status.job);
    std::fs::write(installed.root.path().join("README.md"), "new source\n").unwrap();
    let preview = engine
        .preview_attunement_replan(
            &session.session_id,
            &session.token,
            &installed.job,
            running.status.job.revision,
            running_at + 1,
            "request-preview-1",
            "operation-preview-1",
        )
        .unwrap();
    std::fs::write(installed.root.path().join("README.md"), "changed again\n").unwrap();
    assert!(matches!(
        engine.accept_attunement_replan(
            &session.session_id,
            &session.token,
            &preview,
            &preview.proposal_sha256,
            &context("stale-accept"),
            running_at + 2
        ),
        Err(ServiceError::AttunementSourceChanged)
    ));
    assert_eq!(
        installed.read(&engine, &session, running_at + 3).job,
        running.status.job
    );
    assert!(matches!(
        engine.preview_attunement_replan(
            &session.session_id,
            &id("wrong-token"),
            &installed.job,
            running.status.job.revision,
            running_at + 4,
            "request-denied",
            "operation-denied"
        ),
        Err(ServiceError::Unauthenticated)
    ));
    let replacement = engine
        .preview_attunement_replan(
            &session.session_id,
            &session.token,
            &installed.job,
            running.status.job.revision,
            running_at + 5,
            "request-preview-2",
            "operation-preview-2",
        )
        .unwrap();
    let mut malformed = replacement.clone();
    malformed.generation += 1;
    assert!(matches!(
        engine.accept_attunement_replan(
            &session.session_id,
            &session.token,
            &malformed,
            &malformed.proposal_sha256,
            &context("invalid-proposal"),
            running_at + 6
        ),
        Err(ServiceError::OperationDigestMismatch)
    ));
    let before_sequence = engine.storage.control().sequence().unwrap();
    let accepted = engine
        .accept_attunement_replan(
            &session.session_id,
            &session.token,
            &replacement,
            &replacement.proposal_sha256,
            &context("replacement"),
            running_at + 7,
        )
        .unwrap();
    let accepted_at = accepted.predecessor.job.updated_at_unix_ms;
    let journal = engine
        .storage
        .control()
        .journal_since(before_sequence, 32)
        .unwrap();
    for action in [
        "attunement.replan_fenced",
        "attunement.replan_plan_created",
        "attunement.replan_job_created",
        "attunement.replan_generation",
        "attunement.replan_linked",
        "attunement.replan_receipted",
    ] {
        assert_eq!(
            journal
                .iter()
                .filter(|entry| entry.action == action)
                .count(),
            1
        );
    }
    assert!(matches!(
        call(&engine, &session, &old_step, "old-worker", accepted_at + 1),
        Err(ServiceError::StorageConflict(_))
    ));
    let mut changed = replacement.clone();
    changed.generation += 1;
    assert!(matches!(
        engine.accept_attunement_replan(
            &session.session_id,
            &session.token,
            &changed,
            &changed.proposal_sha256,
            &context("replacement"),
            accepted_at + 2
        ),
        Err(ServiceError::IdempotencyConflict)
    ));
    let cancellation = call(
        &engine,
        &session,
        &step(&accepted.predecessor.job),
        "cancel-old",
        accepted_at + 3,
    )
    .unwrap();
    assert_eq!(cancellation.status.job.state, State::Cancelled);
    assert!(cancellation.status.job.checkpoints.is_empty());
}

#[test]
fn competing_replan_acceptances_publish_one_successor() {
    let (installed, engine, first) = Installed::new(InstallationTargetKind::ExistingProject);
    let second = installed.session(&engine, "replan-competitor", BASE + 200);
    std::fs::write(installed.root.path().join("README.md"), "edited\n").unwrap();
    let preview = engine
        .preview_attunement_replan(
            &first.session_id,
            &first.token,
            &installed.job,
            1,
            BASE + 500,
            "request-compete-preview",
            "operation-compete-preview",
        )
        .unwrap();
    let engine = Arc::new(engine);
    let barrier = Arc::new(Barrier::new(2));
    let results = std::thread::scope(|scope| {
        let mut handles = Vec::new();
        for (session, name) in [(&first, "accept-first"), (&second, "accept-second")] {
            let engine = Arc::clone(&engine);
            let barrier = Arc::clone(&barrier);
            let preview = preview.clone();
            handles.push(scope.spawn(move || {
                barrier.wait();
                engine.accept_attunement_replan(
                    &session.session_id,
                    &session.token,
                    &preview,
                    &preview.proposal_sha256,
                    &context(name),
                    BASE + 1_000,
                )
            }));
        }
        handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect::<Vec<_>>()
    });
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    let winner = results.into_iter().find_map(Result::ok).unwrap();
    assert_eq!(winner.predecessor.job.state, State::CancelRequested);
    let successor = engine
        .read_attunement(
            &first.session_id,
            &first.token,
            &winner.successor.job.id,
            winner.successor.job.updated_at_unix_ms + 1,
            "request-winning-job",
            "operation-winning-job",
        )
        .unwrap();
    assert_eq!(successor.job, winner.successor.job);
}

#[test]
fn replan_refuses_an_unreconciled_failure() {
    let (installed, engine, session) = Installed::with_proposal(
        InstallationTargetKind::ExistingProject,
        Some(&proposal_with_unavailable_parse()),
    );
    let started = call(
        &engine,
        &session,
        &installed.start(30_000),
        "unknown-start",
        BASE + 1_000,
    )
    .unwrap();
    let connected = call(
        &engine,
        &session,
        &step(&started.status.job),
        "unknown-connect",
        started.status.job.updated_at_unix_ms + 1,
    )
    .unwrap();
    let inventoried = call(
        &engine,
        &session,
        &step(&connected.status.job),
        "unknown-failure",
        connected.status.job.updated_at_unix_ms + 1,
    )
    .unwrap();
    assert_eq!(inventoried.status.job.current_phase, Some(Phase::Parse));
    let failed = call(
        &engine,
        &session,
        &step(&inventoried.status.job),
        "unknown-parse",
        inventoried.status.job.updated_at_unix_ms + 1,
    )
    .unwrap();
    let failure_at = failed.status.job.updated_at_unix_ms;
    assert!(engine
        .preview_attunement_replan(
            &session.session_id,
            &session.token,
            &installed.job,
            failed.status.job.revision,
            failure_at + 1,
            "request-known-failure",
            "operation-known-failure"
        )
        .is_ok());
    let key = crate::engine::installation::attunement_job_key(
        &installed.preview.attunement.target.instance_id,
        &installed.job,
    );
    let expected = engine.storage.control().get(&key).unwrap();
    let mut unknown = failed.status.job;
    let failure = unknown.failure.as_mut().unwrap();
    failure.code = CanonicalId::new("external-outcome-unknown").unwrap();
    failure.retryable = false;
    unknown.validate(&installed.preview.attunement).unwrap();
    engine
        .storage
        .control()
        .commit(&ControlTransition {
            key,
            expected,
            replacement: Some(serde_json::to_vec(&unknown).unwrap()),
            at: failure_at + 2,
            actor: "test-fixture".into(),
            action: "attunement.fixture_unknown".into(),
            request_id: "request-fixture-unknown".into(),
            operation_id: "operation-fixture-unknown".into(),
        })
        .unwrap();
    assert!(matches!(
        engine.preview_attunement_replan(
            &session.session_id,
            &session.token,
            &installed.job,
            unknown.revision,
            failure_at + 3,
            "request-unknown",
            "operation-unknown"
        ),
        Err(ServiceError::StorageConflict(_))
    ));
    assert_eq!(
        installed.read(&engine, &session, failure_at + 4).job,
        unknown
    );
}

#[test]
fn replan_keeps_historical_control_receipt_after_restart() {
    let (installed, engine, session) = Installed::new(InstallationTargetKind::ExistingProject);
    let started = call(
        &engine,
        &session,
        &installed.start(30_000),
        "historical-start",
        BASE + 1_000,
    )
    .unwrap();
    let old_step = step(&started.status.job);
    let connected = call(
        &engine,
        &session,
        &old_step,
        "historical-connect",
        started.status.job.updated_at_unix_ms + 1,
    )
    .unwrap();
    std::fs::write(
        installed.root.path().join("README.md"),
        "changed after connect\n",
    )
    .unwrap();
    let preview = engine
        .preview_attunement_replan(
            &session.session_id,
            &session.token,
            &installed.job,
            connected.status.job.revision,
            connected.status.job.updated_at_unix_ms + 1,
            "request-historical-preview",
            "operation-historical-preview",
        )
        .unwrap();
    let accepted = engine
        .accept_attunement_replan(
            &session.session_id,
            &session.token,
            &preview,
            &preview.proposal_sha256,
            &context("historical-replan"),
            connected.status.job.updated_at_unix_ms + 2,
        )
        .unwrap();
    let accepted_at = accepted.predecessor.job.updated_at_unix_ms;
    assert_eq!(accepted.predecessor.job.state, State::CancelRequested);
    drop(engine);
    let engine = installed.reopen(accepted_at + 1);
    let renewed = installed.session(&engine, "historical-reopened", accepted_at + 2);
    let replay = call(
        &engine,
        &renewed,
        &old_step,
        "historical-connect",
        accepted_at + 3,
    )
    .unwrap();
    assert!(replay.idempotent_replay);
    assert_eq!(replay.status, connected.status);
    let receipt = engine
        .read_attunement_receipt(
            &renewed.session_id,
            &renewed.token,
            &installed.job,
            &id("historical-connect"),
            accepted_at + 4,
            "request-historical-receipt",
            "operation-historical-receipt",
        )
        .unwrap()
        .unwrap();
    assert_eq!(receipt.status, connected.status);
    assert_eq!(receipt.plan, installed.preview.attunement);
}

#[test]
fn repeated_ordinary_edits_are_not_stopped_by_a_fixed_generation_limit() {
    let (installed, engine, session) = Installed::new(InstallationTargetKind::ExistingProject);
    let mut current = installed.job.clone();
    let mut at = BASE + 1_000;
    for generation in 1_u64..=33 {
        std::fs::write(
            installed.root.path().join("README.md"),
            format!("edit {generation}\n"),
        )
        .unwrap();
        let preview = engine
            .preview_attunement_replan(
                &session.session_id,
                &session.token,
                &current,
                1,
                at,
                "request-generation-preview",
                "operation-generation-preview",
            )
            .unwrap();
        assert_eq!(preview.generation, generation);
        let key = format!("generation-{generation}");
        let result = engine
            .accept_attunement_replan(
                &session.session_id,
                &session.token,
                &preview,
                &preview.proposal_sha256,
                &context(&key),
                at + 1,
            )
            .unwrap();
        at = result.successor.job.updated_at_unix_ms + 1_000;
        current = result.successor.job.id;
    }
}

#[test]
#[ignore = "native replan lost-response subprocess fixture"]
fn attunement_replan_crash_child() {
    let root = PathBuf::from(std::env::var_os("RRFLOW_REPLAN_TEST_ROOT").unwrap());
    let ready = PathBuf::from(std::env::var_os("RRFLOW_REPLAN_TEST_READY").unwrap());
    let preview_path = PathBuf::from(std::env::var_os("RRFLOW_REPLAN_TEST_PREVIEW").unwrap());
    let preview: crate::AttunementReplanPreview =
        serde_json::from_slice(&std::fs::read(preview_path).unwrap()).unwrap();
    let executable = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let engine =
        RrdEngine::open_installed_with_clock(&root, &executable, &FixedClock::at(BASE + 2_000))
            .unwrap();
    let credential = RrdEngine::read_installed_api_key(&root).unwrap();
    let session = engine
        .create_authenticated_session(
            &credential.principal_id,
            credential.credential().as_bytes(),
            &CreateSession {
                limits: SessionLimits {
                    idle_timeout_ms: 120_000,
                    absolute_timeout_ms: 120_000,
                    max_open_transactions: 2,
                },
            },
            &id("replan-crash-session"),
            BASE + 2_100,
            "request-replan-crash-session",
            "operation-replan-crash-session",
        )
        .unwrap();
    engine
        .accept_attunement_replan(
            &session.session_id,
            &session.token,
            &preview,
            &preview.proposal_sha256,
            &context("replan-lost-response"),
            BASE + 2_200,
        )
        .unwrap();
    std::fs::write(ready, "committed").unwrap();
    std::thread::sleep(std::time::Duration::from_secs(30));
    panic!("parent failed to terminate the replan fixture");
}

#[test]
fn replan_lost_response_survives_process_kill_and_replays() {
    let (installed, engine, session) = Installed::new(InstallationTargetKind::ExistingProject);
    std::fs::write(
        installed.root.path().join("README.md"),
        "edited before replan\n",
    )
    .unwrap();
    let preview = engine
        .preview_attunement_replan(
            &session.session_id,
            &session.token,
            &installed.job,
            1,
            BASE + 1_000,
            "request-crash-preview",
            "operation-crash-preview",
        )
        .unwrap();
    drop(engine);
    let artifacts = tempfile::tempdir().unwrap();
    let preview_path = artifacts.path().join("preview.json");
    let ready = artifacts.path().join("committed");
    let output_path = artifacts.path().join("child.log");
    std::fs::write(&preview_path, serde_json::to_vec(&preview).unwrap()).unwrap();
    let output = std::fs::File::create(&output_path).unwrap();
    struct ChildGuard(std::process::Child);
    impl Drop for ChildGuard {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let mut child = ChildGuard(
        std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "engine::tests::attunement::replan::attunement_replan_crash_child",
                "--ignored",
                "--nocapture",
            ])
            .env("RRFLOW_REPLAN_TEST_ROOT", installed.root.path())
            .env("RRFLOW_REPLAN_TEST_READY", &ready)
            .env("RRFLOW_REPLAN_TEST_PREVIEW", &preview_path)
            .stdout(output.try_clone().unwrap())
            .stderr(output)
            .spawn()
            .unwrap(),
    );
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    while !ready.exists() {
        assert!(
            child.0.try_wait().unwrap().is_none() && std::time::Instant::now() < deadline,
            "child failed before replan commit: {}",
            std::fs::read_to_string(&output_path).unwrap()
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    child.0.kill().unwrap();
    child.0.wait().unwrap();
    let engine = installed.reopen(BASE + 4_000);
    let renewed = installed.session(&engine, "replan-after-kill", BASE + 4_100);
    let replay = engine
        .accept_attunement_replan(
            &renewed.session_id,
            &renewed.token,
            &preview,
            &preview.proposal_sha256,
            &context("replan-lost-response"),
            BASE + 4_200,
        )
        .unwrap();
    assert!(replay.idempotent_replay);
    assert_eq!(replay.predecessor.job.state, State::CancelRequested);
    // The killed child committed this job, so its completion first returns here.
    let successor = engine
        .read_attunement(
            &renewed.session_id,
            &renewed.token,
            &replay.successor.job.id,
            (BASE + 4_300).max(replay.successor.job.updated_at_unix_ms + 1),
            "request-after-kill",
            "operation-after-kill",
        )
        .unwrap();
    assert_eq!(successor.job, replay.successor.job);
}

#[test]
fn replan_receipt_read_is_principal_scoped_and_does_not_replay() {
    let (installed, engine, session) = Installed::new(InstallationTargetKind::ExistingProject);
    std::fs::write(installed.root.path().join("README.md"), "receipt source\n").unwrap();
    let preview = engine
        .preview_attunement_replan(
            &session.session_id,
            &session.token,
            &installed.job,
            1,
            BASE + 1_000,
            "request-replan-read-preview",
            "operation-replan-read-preview",
        )
        .unwrap();
    assert!(engine
        .read_attunement_replan_receipt(
            &session.session_id,
            &session.token,
            &installed.job,
            &id("read-replan"),
            BASE + 1_100,
            "request-missing-replan",
            "operation-missing-replan"
        )
        .unwrap()
        .is_none());
    let accepted = engine
        .accept_attunement_replan(
            &session.session_id,
            &session.token,
            &preview,
            &preview.proposal_sha256,
            &context("read-replan"),
            BASE + 1_200,
        )
        .unwrap();
    // Effects include real elapsed work. Continue from their committed time,
    // rather than assuming the previous call finished within a fixed window.
    let receipt_at = accepted.predecessor.job.updated_at_unix_ms + 1;
    let before = engine.storage.control().journal_since(0, 1024).unwrap();
    let receipt = engine
        .read_attunement_replan_receipt(
            &session.session_id,
            &session.token,
            &installed.job,
            &id("read-replan"),
            receipt_at,
            "request-read-replan",
            "operation-read-replan",
        )
        .unwrap()
        .unwrap();
    assert_eq!(receipt.preview, preview);
    assert_eq!(receipt.predecessor_plan, installed.preview.attunement);
    assert_eq!(receipt.result, accepted);
    assert!(engine
        .read_attunement_replan_receipt(
            &session.session_id,
            &id("bad-token"),
            &installed.job,
            &id("read-replan"),
            receipt_at + 1,
            "request-bad-replan",
            "operation-bad-replan"
        )
        .is_err());
    let after = engine.storage.control().journal_since(0, 1024).unwrap();
    let count = |rows: &[rrd_store::ControlJournalEntry]| {
        rows.iter()
            .filter(|row| row.action.starts_with("attunement.replan_"))
            .count()
    };
    assert_eq!(count(&before), count(&after));
    assert_eq!(
        installed.read(&engine, &session, receipt_at + 2).job,
        accepted.predecessor.job
    );
    let successor_start = AttunementCommand::Start {
        job_id: accepted.successor.job.id.clone(),
        plan_sha256: accepted.plan.plan_sha256.clone(),
        expected_revision: 1,
        lease_ms: 30_000,
    };
    let successor = call(
        &engine,
        &session,
        &successor_start,
        "receipt-successor-start",
        receipt_at + 3,
    )
    .unwrap();
    std::fs::write(installed.root.path().join("README.md"), "later edit\n").unwrap();
    let later_at = successor.status.job.updated_at_unix_ms + 1;
    let later = engine
        .preview_attunement_replan(
            &session.session_id,
            &session.token,
            &successor.status.job.id,
            successor.status.job.revision,
            later_at,
            "request-later-preview",
            "operation-later-preview",
        )
        .unwrap();
    let later_accepted = engine
        .accept_attunement_replan(
            &session.session_id,
            &session.token,
            &later,
            &later.proposal_sha256,
            &context("later-replan"),
            later_at + 1,
        )
        .unwrap();
    let historical_at = later_accepted.predecessor.job.updated_at_unix_ms + 1;
    let historical = engine
        .read_attunement_replan_receipt(
            &session.session_id,
            &session.token,
            &installed.job,
            &id("read-replan"),
            historical_at,
            "request-historical-replan-read",
            "operation-historical-replan-read",
        )
        .unwrap()
        .unwrap();
    assert_eq!(historical, receipt);
    drop(engine);
    let engine = installed.reopen(historical_at + 1);
    let renewed = installed.session(&engine, "replan-read-reopen", historical_at + 2);
    let reopened = engine
        .read_attunement_replan_receipt(
            &renewed.session_id,
            &renewed.token,
            &installed.job,
            &id("read-replan"),
            historical_at + 3,
            "request-replan-reopen",
            "operation-replan-reopen",
        )
        .unwrap()
        .unwrap();
    assert_eq!(reopened, receipt);
}

#[test]
fn replan_receipt_and_replay_reject_a_corrupt_successor_link() {
    let (installed, engine, session) = Installed::new(InstallationTargetKind::ExistingProject);
    let preview = engine
        .preview_attunement_replan(
            &session.session_id,
            &session.token,
            &installed.job,
            1,
            BASE + 1_000,
            "request-corrupt-preview",
            "operation-corrupt-preview",
        )
        .unwrap();
    let accepted = engine
        .accept_attunement_replan(
            &session.session_id,
            &session.token,
            &preview,
            &preview.proposal_sha256,
            &context("corrupt-replan"),
            BASE + 1_200,
        )
        .unwrap();
    let accepted_at = accepted.predecessor.job.updated_at_unix_ms;
    let link = format!(
        "{}/successor",
        crate::engine::installation::attunement_job_key(engine.instance_id(), &installed.job)
    );
    let expected = engine.storage.control().get(&link).unwrap();
    let mut changed = preview.clone();
    changed.generation += 1;
    engine
        .storage
        .control()
        .commit(&ControlTransition {
            key: link,
            expected,
            replacement: Some(serde_json::to_vec(&changed).unwrap()),
            at: accepted_at + 1,
            actor: "test-fixture".into(),
            action: "attunement.fixture_corrupt_link".into(),
            request_id: "request-corrupt-link".into(),
            operation_id: "operation-corrupt-link".into(),
        })
        .unwrap();
    assert!(engine
        .read_attunement_replan_receipt(
            &session.session_id,
            &session.token,
            &installed.job,
            &id("corrupt-replan"),
            accepted_at + 2,
            "request-corrupt-read",
            "operation-corrupt-read"
        )
        .is_err());
    assert!(engine
        .accept_attunement_replan(
            &session.session_id,
            &session.token,
            &preview,
            &preview.proposal_sha256,
            &context("corrupt-replan"),
            accepted_at + 3
        )
        .is_err());
}

#[test]
fn replan_receipt_is_not_visible_to_another_authorized_principal() {
    let (installed, engine, owner) = Installed::new(InstallationTargetKind::ExistingProject);
    let preview = engine
        .preview_attunement_replan(
            &owner.session_id,
            &owner.token,
            &installed.job,
            1,
            BASE + 1_000,
            "request-private-preview",
            "operation-private-preview",
        )
        .unwrap();
    let accepted = engine
        .accept_attunement_replan(
            &owner.session_id,
            &owner.token,
            &preview,
            &preview.proposal_sha256,
            &context("private-replan"),
            BASE + 1_200,
        )
        .unwrap();
    let accepted_at = accepted.predecessor.job.updated_at_unix_ms;
    let repository = SecurityRepository::new(&engine.storage, engine.instance_id().clone());
    let mut authority = repository.load().unwrap().unwrap();
    let revision = authority.revision;
    authority.revision += 1;
    let principal_id = CanonicalId::new("other-replan-reader").unwrap();
    let resource = ResourcePath {
        segments: vec![
            ResourceId::new(ResourceKind::Instance, engine.instance_id().as_str()).unwrap(),
        ],
    };
    authority.principals.insert(
        principal_id.clone(),
        Principal {
            id: principal_id.clone(),
            kind: PrincipalKind::Service,
            credential_sha256: digest::sha256_hex(b"other-replan-reader"),
            credential_revision: 1,
            not_before_unix_ms: 1,
            expires_at_unix_ms: u64::MAX,
            disabled: false,
            role_ids: Default::default(),
            grants: [SecurityAction::SessionCreate, SecurityAction::EngramRead]
                .into_iter()
                .map(|action| ResourceGrant {
                    action,
                    resource_prefix: resource.clone(),
                    data_policy: None,
                })
                .collect(),
        },
    );
    repository
        .replace(
            revision,
            authority,
            accepted_at + 1,
            "test",
            "reader-update",
            "reader-update",
        )
        .unwrap();
    let reader = engine
        .create_authenticated_session(
            &principal_id,
            b"other-replan-reader",
            &CreateSession {
                limits: SessionLimits {
                    idle_timeout_ms: 120_000,
                    absolute_timeout_ms: 120_000,
                    max_open_transactions: 2,
                },
            },
            &id("other-replan-session"),
            accepted_at + 2,
            "request-other-replan-session",
            "operation-other-replan-session",
        )
        .unwrap();
    assert!(engine
        .read_attunement_replan_receipt(
            &reader.session_id,
            &reader.token,
            &installed.job,
            &id("private-replan"),
            accepted_at + 3,
            "request-foreign-replan",
            "operation-foreign-replan"
        )
        .unwrap()
        .is_none());
}
