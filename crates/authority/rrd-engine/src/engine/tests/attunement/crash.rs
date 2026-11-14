//! Crash and native worker kill recovery.

use super::*;

// This ignored entry point is invoked by the native parent test below, not
// counted as independent qualification. It uses the real installed engine.
#[test]
#[ignore = "native attunement subprocess fixture"]
pub(super) fn attunement_crash_child() {
    let root = PathBuf::from(
        std::env::var_os("RRFLOW_ATTUNEMENT_TEST_ROOT").expect("parent fixture root"),
    );
    let ready =
        PathBuf::from(std::env::var_os("RRFLOW_ATTUNEMENT_TEST_READY").expect("parent ready path"));
    let job = CanonicalId::new(std::env::var("RRFLOW_ATTUNEMENT_TEST_JOB").unwrap()).unwrap();
    let plan_sha256 = std::env::var("RRFLOW_ATTUNEMENT_TEST_PLAN").unwrap();
    let executable = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let engine =
        RrdEngine::open_installed_with_clock(&root, &executable, &FixedClock::at(BASE + 1_000))
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
            &id("crashed-worker-session"),
            BASE + 1_100,
            "request-crashed-session",
            "operation-crashed-session",
        )
        .unwrap();
    let claim = AttunementCommand::Start {
        job_id: job,
        plan_sha256,
        expected_revision: 1,
        lease_ms: 30_000,
    };
    call(&engine, &session, &claim, "crashed-start", BASE + 2_000).unwrap();
    // Signal only after the authoritative commit; the parent has not received
    // the accepted response. It terminates us with the engine still open.
    std::fs::write(ready, "claimed").unwrap();
    std::thread::sleep(std::time::Duration::from_secs(30));
    panic!("parent failed to terminate the attunement fixture");
}

#[test]
pub(super) fn attunement_native_worker_kill_recovers_claim_and_replays_lost_response() {
    let (installed, engine, _) = Installed::new(InstallationTargetKind::ExistingProject);
    drop(engine);
    let artifacts = tempfile::tempdir().unwrap();
    let ready = artifacts.path().join("claimed");
    let output_path = artifacts.path().join("child.log");
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
                "engine::tests::attunement::crash::attunement_crash_child",
                "--ignored",
                "--nocapture",
            ])
            .env("RRFLOW_ATTUNEMENT_TEST_ROOT", installed.root.path())
            .env("RRFLOW_ATTUNEMENT_TEST_READY", &ready)
            .env("RRFLOW_ATTUNEMENT_TEST_JOB", installed.job.as_str())
            .env(
                "RRFLOW_ATTUNEMENT_TEST_PLAN",
                &installed.preview.attunement.plan_sha256,
            )
            .stdout(output.try_clone().unwrap())
            .stderr(output)
            .spawn()
            .unwrap(),
    );
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    while !ready.exists() {
        assert!(
            child.0.try_wait().unwrap().is_none() && std::time::Instant::now() < deadline,
            "child failed before claim: {}",
            std::fs::read_to_string(&output_path).unwrap()
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    child.0.kill().unwrap();
    child.0.wait().unwrap();
    let engine = installed.reopen(BASE + 40_000);
    let session = installed.session(&engine, "surviving-worker", BASE + 40_100);
    let running = installed.read(&engine, &session, BASE + 41_000).job;
    assert_eq!(running.state, State::Running);
    assert_eq!(running.revision, 2);
    assert!(running.checkpoints.is_empty());
    let replay = call(
        &engine,
        &session,
        &installed.start(30_000),
        "crashed-start",
        BASE + 42_000,
    )
    .unwrap();
    assert!(replay.idempotent_replay);
    assert_eq!(replay.status.job, running);
    // The killed child claimed this lease, so its expiry first returns in this read.
    let recover_at = (BASE + 43_000).max(running.lease.as_ref().unwrap().expires_at_unix_ms + 1);
    let paused = call(
        &engine,
        &session,
        &AttunementCommand::RecoverExpired {
            job_id: installed.job.clone(),
            expected_revision: 2,
        },
        "recover-kill",
        recover_at,
    )
    .unwrap();
    let running = call(
        &engine,
        &session,
        &resume(&paused.status.job, paused.status.job.updated_at_unix_ms + 1),
        "resume-kill",
        paused.status.job.updated_at_unix_ms + 1,
    )
    .unwrap();
    let connected = call(
        &engine,
        &session,
        &step(&running.status.job),
        "connect-after-kill",
        running.status.job.updated_at_unix_ms + 1,
    )
    .unwrap();
    assert_eq!(connected.status.job.checkpoints.len(), 1);
    assert_eq!(connected.status.job.current_phase, Some(Phase::Inventory));
    let replay = call(
        &engine,
        &session,
        &step(&running.status.job),
        "connect-after-kill",
        connected.status.job.updated_at_unix_ms + 1,
    )
    .unwrap();
    assert!(replay.idempotent_replay);
    assert_eq!(replay.status, connected.status);
}
