//! Real processes exercise the interval before rrflowKV writer admission.
use super::*;
use crate::engine::installation::recovery::InstallApplyStage;
use crate::InstallationPreview;
use rrd_contract::InstallationTargetKind;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

const CHILD_TEST: &str = "engine::tests::installation_concurrency::installation_child";

struct OwnedChild(Child);
impl Drop for OwnedChild {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn spawn(fixture: &Path, role: &str) -> OwnedChild {
    OwnedChild(
        Command::new(std::env::current_exe().unwrap())
            .args(["--exact", CHILD_TEST, "--ignored", "--nocapture"])
            .env("RRFLOW_INSTALL_TEST_FIXTURE", fixture)
            .env("RRFLOW_INSTALL_TEST_ROLE", role)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    )
}

fn wait_file(path: &Path, child: &mut OwnedChild) {
    let deadline = Instant::now() + Duration::from_secs(30);
    while !path.exists() {
        assert!(
            child.0.try_wait().unwrap().is_none(),
            "child exited before barrier"
        );
        assert!(Instant::now() < deadline, "child barrier deadline exceeded");
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn wait_success(child: &mut OwnedChild) {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        if let Some(status) = child.0.try_wait().unwrap() {
            assert!(status.success(), "installation child failed");
            return;
        }
        assert!(
            Instant::now() < deadline,
            "child completion deadline exceeded"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
#[ignore = "subprocess fixture; launched only by bounded concurrency tests"]
fn installation_child() {
    let fixture = PathBuf::from(std::env::var_os("RRFLOW_INSTALL_TEST_FIXTURE").unwrap());
    let role = std::env::var("RRFLOW_INSTALL_TEST_ROLE").unwrap();
    let preview: InstallationPreview =
        serde_json::from_slice(&std::fs::read(fixture.join("plan.json")).unwrap()).unwrap();
    let executable = std::env::current_exe().unwrap();
    let result = RrdEngine::apply_installation_with_test_observer(
        &fixture.join("project"),
        &preview,
        &executable,
        &mut |stage, _| {
            if role == "owner" && stage == InstallApplyStage::IntentPublished {
                std::fs::write(fixture.join("paused"), b"ready").unwrap();
                let deadline = Instant::now() + Duration::from_secs(30);
                while !fixture.join("release").exists() {
                    assert!(Instant::now() < deadline, "owner release deadline exceeded");
                    std::thread::sleep(Duration::from_millis(10));
                }
            }
            Ok(())
        },
    );
    let outcome = match result {
        Ok(_) => "success",
        Err(ServiceError::StorageConflict(_)) => "conflict",
        Err(_) => "other_error",
    };
    std::fs::write(fixture.join(format!("{role}.result")), outcome).unwrap();
}

#[test]
fn same_plan_contender_cannot_advance_an_active_installation_intent() {
    let fixture = tempfile::tempdir().unwrap();
    let project = fixture.path().join("project");
    std::fs::create_dir(&project).unwrap();
    let executable = std::env::current_exe().unwrap();
    let preview = RrdEngine::plan_installation(
        &project,
        InstallationTargetKind::FreshProject,
        &"1".repeat(64),
        "default",
        None,
        &executable,
    )
    .unwrap();
    std::fs::write(
        fixture.path().join("plan.json"),
        serde_json::to_vec(&preview).unwrap(),
    )
    .unwrap();
    let mut owner = spawn(fixture.path(), "owner");
    wait_file(&fixture.path().join("paused"), &mut owner);
    let mut contender = spawn(fixture.path(), "contender");
    wait_success(&mut contender);
    let outcome = std::fs::read_to_string(fixture.path().join("contender.result")).unwrap();
    // Never include credentials or raw installation-intent bytes in failures.
    assert_eq!(
        outcome, "conflict",
        "a live owner must exclude recovery before store admission"
    );
    assert!(!project.join(".rrflow").exists());
    std::fs::write(fixture.path().join("release"), b"continue").unwrap();
    wait_success(&mut owner);
    assert_eq!(
        std::fs::read_to_string(fixture.path().join("owner.result")).unwrap(),
        "success"
    );
}
