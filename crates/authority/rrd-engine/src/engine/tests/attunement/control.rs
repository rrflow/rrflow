//! Attunement detail, connect failures, competing workers, expiry, cancel and control encoding.

use super::*;

#[test]
pub(super) fn attunement_detail_reports_failure_and_does_not_claim_source_verification() {
    let proposal = proposal_with_unavailable_parse();
    let (installed, engine, session) =
        Installed::with_proposal(InstallationTargetKind::ExistingProject, Some(&proposal));
    let request = rrd_contract::ReadAttunement {
        contract_version: rrd_contract::ATTUNEMENT_CONTRACT_VERSION,
        job_id: installed.job.clone(),
    };
    let started = call(
        &engine,
        &session,
        &installed.start(30_000),
        "detail-start",
        BASE + 1_000,
    )
    .unwrap();
    let connected = call(
        &engine,
        &session,
        &step(&started.status.job),
        "detail-connect",
        started.status.job.updated_at_unix_ms + 1,
    )
    .unwrap();
    let inventoried = call(
        &engine,
        &session,
        &step(&connected.status.job),
        "detail-inventory",
        connected.status.job.updated_at_unix_ms + 1,
    )
    .unwrap();
    assert_eq!(inventoried.status.job.current_phase, Some(Phase::Parse));
    let failed = call(
        &engine,
        &session,
        &step(&inventoried.status.job),
        "detail-parse",
        inventoried.status.job.updated_at_unix_ms + 1,
    )
    .unwrap();
    let failure_at = failed.status.job.updated_at_unix_ms;
    std::fs::write(installed.root.path().join("README.md"), "changed source\n").unwrap();
    let read = engine
        .read_attunement_detail(
            &session.session_id,
            &session.token,
            &request,
            failure_at + 1,
            "request-detail",
            "operation-detail",
        )
        .unwrap();
    assert_eq!(read.plan, installed.preview.attunement);
    assert_eq!(read.status.job, failed.status.job);
    assert_eq!(read.status.job.state, State::Failed);
    assert_eq!(
        read.status.job.failure.as_ref().unwrap().code.as_str(),
        "phase-unavailable"
    );
    assert_eq!(
        read.source_verification,
        rrd_contract::AttunementSourceVerification::NotChecked
    );
    let invalid_token = id("invalid-token");
    assert!(engine
        .read_attunement_detail(
            &session.session_id,
            &invalid_token,
            &request,
            failure_at + 2,
            "request-denied",
            "operation-denied"
        )
        .is_err());
}

#[test]
pub(super) fn attunement_installed_connect_failure_and_exact_receipts_survive_reopen() {
    for mode in [
        InstallationTargetKind::FreshProject,
        InstallationTargetKind::ExistingProject,
    ] {
        let proposal = proposal_with_unavailable_parse();
        let (installed, engine, session) = Installed::with_proposal(mode, Some(&proposal));
        let pending = installed.read(&engine, &session, BASE + 500);
        assert_eq!(pending.job.state, State::Pending);
        let start = call(
            &engine,
            &session,
            &installed.start(30_000),
            "start",
            BASE + 1_000,
        )
        .unwrap();
        assert_eq!(start.status.job.state, State::Running);
        let work = step(&start.status.job);
        let connected = call(
            &engine,
            &session,
            &work,
            "connect",
            start.status.job.updated_at_unix_ms + 1,
        )
        .unwrap();
        assert_eq!(connected.status.job.current_phase, Some(Phase::Inventory));
        assert_eq!(connected.status.job.checkpoints.len(), 1);
        let checkpoint = &connected.status.job.checkpoints[0];
        assert_eq!(checkpoint.phase, Phase::Connect);
        let scope = rrd_core::ScopeId::new(format!(
            "instance:{}",
            installed.preview.attunement.target.instance_id
        ))
        .unwrap();
        let read = engine.storage.runtime().read_stamp(&scope).unwrap();
        assert_eq!(
            checkpoint.coordinates.runtime_manifest_sha256,
            read.manifest_id
        );
        assert_eq!(checkpoint.coordinates.runtime_cursor, read.commit_cursor);
        let inventoried = call(
            &engine,
            &session,
            &step(&connected.status.job),
            "inventory",
            connected.status.job.updated_at_unix_ms + 1,
        )
        .unwrap();
        assert_eq!(inventoried.status.job.current_phase, Some(Phase::Parse));
        let failed = call(
            &engine,
            &session,
            &step(&inventoried.status.job),
            "parse",
            inventoried.status.job.updated_at_unix_ms + 1,
        )
        .unwrap();
        assert_eq!(failed.status.job.state, State::Failed);
        assert_eq!(
            failed.status.job.failure.as_ref().unwrap().code.as_str(),
            "phase-unavailable"
        );
        assert_eq!(
            failed.status.job.checkpoints,
            inventoried.status.job.checkpoints
        );
        assert!(failed.status.job.lease.is_none());
        let failure_at = failed.status.job.updated_at_unix_ms;
        drop(engine);
        let engine = installed.reopen(failure_at + 1);
        let renewed = installed.session(&engine, "second-session", failure_at + 2);
        let replay = call(&engine, &renewed, &work, "connect", failure_at + 3).unwrap();
        assert!(replay.idempotent_replay);
        assert_eq!(
            replay.status, connected.status,
            "replay returns the original receipt even after later failure"
        );
        assert_eq!(
            installed.read(&engine, &renewed, failure_at + 4).job,
            failed.status.job
        );
        let resumed = call(
            &engine,
            &renewed,
            &resume(&failed.status.job, failure_at + 5),
            "resume",
            failure_at + 5,
        )
        .unwrap();
        assert_eq!(resumed.status.job.current_phase, Some(Phase::Parse));
        assert!(
            resumed.status.job.lease.as_ref().unwrap().generation
                > start.status.job.lease.as_ref().unwrap().generation
        );
        let failed_again = call(
            &engine,
            &renewed,
            &step(&resumed.status.job),
            "parse-again",
            resumed.status.job.updated_at_unix_ms + 1,
        )
        .unwrap();
        assert_eq!(
            failed_again.status.job.checkpoints.len(),
            2,
            "unsupported Parse must not fabricate a checkpoint"
        );
        drop(engine);
        let report = RrdEngine::inspect_installed_with_clock(
            installed.root.path(),
            &installed.executable,
            &FixedClock::at(failed_again.status.job.updated_at_unix_ms + 1),
        )
        .unwrap();
        assert_eq!(report.status, crate::InstallationVerificationStatus::Passed);
    }
}

#[test]
pub(super) fn attunement_competing_workers_only_one_claims_and_foreign_worker_cannot_publish() {
    let (installed, engine, first) = Installed::new(InstallationTargetKind::ExistingProject);
    let second = installed.session(&engine, "other-worker", BASE + 200);
    let engine = Arc::new(engine);
    let barrier = Arc::new(Barrier::new(2));
    let results = std::thread::scope(|scope| {
        let mut handles = Vec::new();
        for (session, key) in [(&first, "first-claim"), (&second, "second-claim")] {
            let engine = Arc::clone(&engine);
            let barrier = Arc::clone(&barrier);
            let command = installed.start(30_000);
            handles.push(scope.spawn(move || {
                barrier.wait();
                call(&engine, session, &command, key, BASE + 1_000)
            }));
        }
        handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect::<Vec<_>>()
    });
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    let winner = if results[0].is_ok() { &first } else { &second };
    let loser = if results[0].is_ok() { &second } else { &first };
    let claimed_at = results
        .iter()
        .find_map(|result| result.as_ref().ok())
        .unwrap()
        .status
        .job
        .updated_at_unix_ms;
    let running = installed.read(&engine, winner, claimed_at + 1).job;
    assert!(matches!(
        call(
            &engine,
            loser,
            &step(&running),
            "foreign-step",
            claimed_at + 2
        ),
        Err(ServiceError::StorageConflict(_))
    ));
    assert_eq!(installed.read(&engine, winner, claimed_at + 3).job, running);
    assert!(call(
        &engine,
        winner,
        &step(&running),
        "winner-step",
        claimed_at + 4
    )
    .is_ok());
}

#[test]
pub(super) fn attunement_expiry_recovery_and_pause_fence_prior_workers() {
    let (installed, engine, first) = Installed::new(InstallationTargetKind::FreshProject);
    let second = installed.session(&engine, "recovery-worker", BASE + 200);
    let started = call(
        &engine,
        &first,
        &installed.start(2_000),
        "start",
        BASE + 1_000,
    )
    .unwrap();
    let old_work = step(&started.status.job);
    let recovery = AttunementCommand::RecoverExpired {
        job_id: installed.job.clone(),
        expected_revision: 2,
    };
    let started_at = started.status.job.updated_at_unix_ms;
    let expires_at = started
        .status
        .job
        .lease
        .as_ref()
        .unwrap()
        .expires_at_unix_ms;
    assert!(call(
        &engine,
        &second,
        &recovery,
        "early-recovery",
        started_at + 1
    )
    .is_err());
    assert!(call(&engine, &first, &old_work, "expired-step", expires_at + 1).is_err());
    let paused = call(&engine, &second, &recovery, "recovery", expires_at + 2).unwrap();
    assert_eq!(paused.status.job.state, State::Paused);
    assert!(paused.status.job.checkpoints.is_empty());
    let paused_at = paused.status.job.updated_at_unix_ms;
    drop(engine);
    let engine = installed.reopen(paused_at + 1);
    let resumed = call(
        &engine,
        &second,
        &resume(&paused.status.job, paused_at + 2),
        "resume",
        paused_at + 2,
    )
    .unwrap();
    let resumed_at = resumed.status.job.updated_at_unix_ms;
    assert!(call(&engine, &first, &old_work, "zombie", resumed_at + 1).is_err());
    let mut forged = step(&resumed.status.job);
    if let AttunementCommand::Step {
        lease_generation, ..
    } = &mut forged
    {
        *lease_generation = 2;
    }
    assert!(call(&engine, &second, &forged, "old-generation", resumed_at + 2).is_err());
    let lease = resumed.status.job.lease.as_ref().unwrap();
    let paused = call(
        &engine,
        &second,
        &AttunementCommand::Pause {
            job_id: installed.job.clone(),
            expected_revision: resumed.status.job.revision,
            lease_generation: lease.generation,
        },
        "pause",
        resumed_at + 3,
    )
    .unwrap();
    assert_eq!(paused.status.job.state, State::Paused);
    assert!(call(
        &engine,
        &second,
        &step(&resumed.status.job),
        "paused-worker",
        paused.status.job.updated_at_unix_ms + 1
    )
    .is_err());
}

#[test]
pub(super) fn attunement_cancel_wins_over_stale_work_and_is_durable() {
    let (installed, engine, session) = Installed::new(InstallationTargetKind::ExistingProject);
    let running = call(
        &engine,
        &session,
        &installed.start(30_000),
        "start",
        BASE + 1_000,
    )
    .unwrap();
    let running_at = running.status.job.updated_at_unix_ms;
    let old_work = step(&running.status.job);
    let command = cancel(&running.status.job, running_at + 1);
    let requested = call(&engine, &session, &command, "cancel", running_at + 1).unwrap();
    let requested_at = requested.status.job.updated_at_unix_ms;
    assert_eq!(requested.status.job.state, State::CancelRequested);
    assert!(call(
        &engine,
        &session,
        &old_work,
        "stale-after-cancel",
        requested_at + 1
    )
    .is_err());
    drop(engine);
    let engine = installed.reopen(requested_at + 2);
    let complete = call(
        &engine,
        &session,
        &step(&requested.status.job),
        "ack-cancel",
        requested_at + 3,
    )
    .unwrap();
    let complete_at = complete.status.job.updated_at_unix_ms;
    assert_eq!(complete.status.job.state, State::Cancelled);
    assert!(complete.status.job.lease.is_none());
    assert!(complete.status.job.checkpoints.is_empty());
    let replay = call(&engine, &session, &command, "cancel", complete_at + 1).unwrap();
    assert_eq!(replay.status, requested.status);
    assert!(replay.idempotent_replay);
    assert_eq!(
        installed.read(&engine, &session, complete_at + 2).job,
        complete.status.job
    );
    assert!(call(
        &engine,
        &session,
        &installed.start(30_000),
        "restart-cancelled",
        complete_at + 3
    )
    .is_err());
}

#[test]
pub(super) fn attunement_rejects_invalid_identity_auth_budget_and_changed_idempotency() {
    let (installed, engine, session) = Installed::new(InstallationTargetKind::FreshProject);
    let original = installed.read(&engine, &session, BASE + 500).job;
    let mut wrong_plan = installed.start(30_000);
    if let AttunementCommand::Start { plan_sha256, .. } = &mut wrong_plan {
        *plan_sha256 = "0".repeat(64);
    }
    assert!(call(&engine, &session, &wrong_plan, "wrong-plan", BASE + 1_000).is_err());
    for (lease_ms, key) in [(0, "zero-lease"), (60_001, "huge-lease")] {
        assert!(call(
            &engine,
            &session,
            &installed.start(lease_ms),
            key,
            BASE + 2_000
        )
        .is_err());
    }
    let mut deadline = context("expired-deadline");
    deadline.deadline_unix_ms = Some(BASE + 2_999);
    assert!(matches!(
        engine.control_attunement(
            &session.session_id,
            &session.token,
            &installed.start(30_000),
            &deadline,
            BASE + 3_000
        ),
        Err(ServiceError::DeadlineExceeded)
    ));
    assert!(engine
        .control_attunement(
            &session.session_id,
            &id("invalid-token"),
            &installed.start(30_000),
            &context("bad-token"),
            BASE + 3_000
        )
        .is_err());
    assert!(engine
        .read_attunement(
            &session.session_id,
            &session.token,
            &CanonicalId::new("foreign-job").unwrap(),
            BASE + 3_000,
            "request-foreign",
            "operation-foreign"
        )
        .is_err());
    assert_eq!(
        installed.read(&engine, &session, BASE + 3_500).job,
        original
    );
    let accepted = call(
        &engine,
        &session,
        &installed.start(30_000),
        "accepted-key",
        BASE + 4_000,
    )
    .unwrap();
    let accepted_at = accepted.status.job.updated_at_unix_ms;
    assert!(matches!(
        call(
            &engine,
            &session,
            &installed.start(29_000),
            "accepted-key",
            accepted_at + 1
        ),
        Err(ServiceError::IdempotencyConflict)
    ));
    assert!(
        engine
            .control_attunement(
                &session.session_id,
                &id("invalid-token"),
                &installed.start(30_000),
                &context("accepted-key"),
                accepted_at + 2
            )
            .is_err(),
        "receipt cannot bypass auth"
    );
}

#[test]
pub(super) fn attunement_control_encoding_is_bounded_and_cancel_remains_available_at_limit() {
    let (installed, engine, session) = Installed::new(InstallationTargetKind::FreshProject);
    let mut oversized = installed.start(30_000);
    if let AttunementCommand::Start { plan_sha256, .. } = &mut oversized {
        *plan_sha256 = "f".repeat(128 * 1024);
    }
    assert!(matches!(
        call(&engine, &session, &oversized, "oversized", BASE + 1_000),
        Err(ServiceError::ConfigurationLimit(_))
    ));
    assert_eq!(
        installed.read(&engine, &session, BASE + 1_500).job.state,
        State::Pending
    );
    let running = call(
        &engine,
        &session,
        &installed.start(30_000),
        "start",
        BASE + 2_000,
    )
    .unwrap();
    let running_at = running.status.job.updated_at_unix_ms;
    // Characterize the finite lifetime limit without manufacturing 1023 writes.
    // Only fixture setup writes this canonical control record directly.
    let key = crate::engine::installation::attunement_job_key(
        &installed.preview.attunement.target.instance_id,
        &installed.job,
    );
    let expected = engine.storage.control().get(&key).unwrap();
    let mut exhausted = running.status.job;
    exhausted.revision = 1024;
    exhausted.validate(&installed.preview.attunement).unwrap();
    engine
        .storage
        .control()
        .commit(&ControlTransition {
            key,
            expected,
            replacement: Some(serde_json::to_vec(&exhausted).unwrap()),
            at: running_at + 1,
            actor: "test-fixture".into(),
            action: "attunement.fixture_limit".into(),
            request_id: "fixture-limit".into(),
            operation_id: "fixture-limit".into(),
        })
        .unwrap();
    assert!(matches!(
        call(
            &engine,
            &session,
            &step(&exhausted),
            "limited-work",
            running_at + 2
        ),
        Err(ServiceError::ConfigurationLimit(_))
    ));
    let requested = call(
        &engine,
        &session,
        &cancel(&exhausted, running_at + 3),
        "cancel-limit",
        running_at + 3,
    )
    .unwrap();
    let cancelled = call(
        &engine,
        &session,
        &step(&requested.status.job),
        "ack-limit",
        requested.status.job.updated_at_unix_ms + 1,
    )
    .unwrap();
    assert_eq!(cancelled.status.job.state, State::Cancelled);
    assert_eq!(cancelled.status.job.revision, 1026);
    assert!(call(
        &engine,
        &session,
        &cancel(
            &cancelled.status.job,
            cancelled.status.job.updated_at_unix_ms + 1
        ),
        "cancel-terminal",
        cancelled.status.job.updated_at_unix_ms + 1
    )
    .is_err());
}

#[test]
pub(super) fn attunement_distinct_installed_projects_cannot_read_or_start_each_others_job() {
    let (first, first_engine, first_session) = Installed::new(InstallationTargetKind::FreshProject);
    let (second, second_engine, second_session) =
        Installed::new(InstallationTargetKind::FreshProject);
    assert_ne!(first.job, second.job);
    assert!(first_engine
        .read_attunement(
            &first_session.session_id,
            &first_session.token,
            &second.job,
            BASE + 1_000,
            "foreign-project",
            "foreign-project"
        )
        .is_err());
    assert!(call(
        &second_engine,
        &second_session,
        &first.start(30_000),
        "foreign-start",
        BASE + 2_000
    )
    .is_err());
    assert_eq!(
        first
            .read(&first_engine, &first_session, BASE + 3_000)
            .job
            .state,
        State::Pending
    );
    assert_eq!(
        second
            .read(&second_engine, &second_session, BASE + 3_000)
            .job
            .state,
        State::Pending
    );
}
