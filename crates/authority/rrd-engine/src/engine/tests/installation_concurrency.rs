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
    spawn_at(fixture, role, InstallApplyStage::IntentPublished)
}

fn spawn_at(fixture: &Path, role: &str, stage: InstallApplyStage) -> OwnedChild {
    OwnedChild(
        Command::new(std::env::current_exe().unwrap())
            .args(["--exact", CHILD_TEST, "--ignored", "--nocapture"])
            .env("RRFLOW_INSTALL_TEST_FIXTURE", fixture)
            .env("RRFLOW_INSTALL_TEST_ROLE", role)
            .env(
                "RRFLOW_INSTALL_TEST_STAGE",
                serde_json::to_string(&stage).unwrap(),
            )
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    )
}

fn wait_file(path: &Path, child: &mut OwnedChild) {
    let deadline = Instant::now() + Duration::from_secs(120);
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
    let deadline = Instant::now() + Duration::from_secs(120);
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
    let stage_to_pause: InstallApplyStage =
        serde_json::from_str(&std::env::var("RRFLOW_INSTALL_TEST_STAGE").unwrap()).unwrap();
    let role_plan = fixture.join(format!("{role}.plan.json"));
    let preview: InstallationPreview = serde_json::from_slice(
        &std::fs::read(if role_plan.exists() {
            role_plan
        } else {
            fixture.join("plan.json")
        })
        .unwrap(),
    )
    .unwrap();
    let executable = if fixture.join("distribution-fixture").exists() {
        fixture.join("distribution-fixture")
    } else {
        std::env::current_exe().unwrap()
    };
    let result = RrdEngine::apply_installation_with_test_observer(
        &fixture.join("project"),
        &preview,
        &executable,
        &mut |stage, _| {
            if role == "owner" && stage == stage_to_pause {
                std::fs::write(fixture.join("paused"), b"ready").unwrap();
                let deadline = Instant::now() + Duration::from_secs(120);
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

fn fixture() -> tempfile::TempDir {
    let fixture = tempfile::tempdir().unwrap();
    let project = fixture.path().join("project");
    std::fs::create_dir(&project).unwrap();
    // The focused first oracle binds the actual test executable. The wider
    // engine-process matrix uses a small fixed distribution identity to avoid
    // repeatedly hashing 140MB of debug symbols. D-01.5 owns primary-binary proof.
    let executable = fixture.path().join("distribution-fixture");
    std::fs::write(&executable, b"installation concurrency identity fixture").unwrap();
    for (name, identity) in [("plan.json", "1"), ("different.plan.json", "2")] {
        let preview = RrdEngine::plan_installation(
            &project,
            InstallationTargetKind::FreshProject,
            &identity.repeat(64),
            "default",
            None,
            &executable,
        )
        .unwrap();
        std::fs::write(
            fixture.path().join(name),
            serde_json::to_vec(&preview).unwrap(),
        )
        .unwrap();
    }
    fixture
}

fn outcome(fixture: &Path, role: &str) -> String {
    std::fs::read_to_string(fixture.join(format!("{role}.result"))).unwrap()
}

fn tree_digests(root: &Path) -> std::collections::BTreeMap<PathBuf, String> {
    let mut result = std::collections::BTreeMap::new();
    for entry in std::fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            result.insert(path.clone(), "directory".into());
            result.extend(tree_digests(&path));
        } else {
            // LockFileEx can deny a zero-byte EOF read. Metadata still proves
            // the entire content of an empty lease; never skip nonempty data.
            let bytes = if std::fs::metadata(&path).unwrap().len() == 0 {
                Vec::new()
            } else {
                std::fs::read(&path).unwrap()
            };
            result.insert(path, digest::sha256_hex(&bytes));
        }
    }
    result
}

#[test]
fn competing_plans_and_recovery_are_excluded_at_every_installation_boundary() {
    for stage in [
        InstallApplyStage::LeaseAcquired,
        InstallApplyStage::IntentPublished,
        InstallApplyStage::EstateDirectoriesReady,
        InstallApplyStage::StorageReady,
        InstallApplyStage::TokenReady,
        InstallApplyStage::CredentialReady,
        InstallApplyStage::EngineCommitted,
        InstallApplyStage::LocatorPublished,
        InstallApplyStage::IntentAcknowledged,
    ] {
        let fixture = fixture();
        let project = fixture.path().join("project");
        let mut owner = spawn_at(fixture.path(), "owner", stage);
        wait_file(&fixture.path().join("paused"), &mut owner);
        let before = tree_digests(&project);
        for role in ["contender", "different"] {
            let mut contender = spawn(fixture.path(), role);
            wait_success(&mut contender);
            assert_eq!(
                outcome(fixture.path(), role),
                "conflict",
                "{stage:?} {role}"
            );
            assert_eq!(
                tree_digests(&project),
                before,
                "contender changed project at {stage:?}"
            );
        }
        std::fs::write(fixture.path().join("release"), b"continue").unwrap();
        wait_success(&mut owner);
        assert_eq!(outcome(fixture.path(), "owner"), "success");
        // Finishing an old owner must not remove the shared admission inode.
        let lease = project.join(rrd_contract::INSTALLATION_LEASE_PATH);
        assert_eq!(std::fs::metadata(&lease).unwrap().len(), 0);
        let completed = tree_digests(&project);
        let mut replay = spawn(fixture.path(), "contender");
        wait_success(&mut replay);
        assert_eq!(outcome(fixture.path(), "contender"), "success");
        assert_eq!(tree_digests(&project), completed);
    }
}

#[test]
fn process_death_releases_admission_without_regenerating_prepared_secrets() {
    for stage in [
        InstallApplyStage::LeaseAcquired,
        InstallApplyStage::IntentPublished,
        InstallApplyStage::EngineCommitted,
        InstallApplyStage::LocatorPublished,
        InstallApplyStage::IntentAcknowledged,
    ] {
        let fixture = fixture();
        let project = fixture.path().join("project");
        let preview: InstallationPreview =
            serde_json::from_slice(&std::fs::read(fixture.path().join("plan.json")).unwrap())
                .unwrap();
        let layout = crate::engine::estate_layout::EstateLayout::new(
            &preview.installation.storage_root_id,
            &preview.installation.initial_principal_id,
        )
        .unwrap();
        let mut owner = spawn_at(fixture.path(), "owner", stage);
        wait_file(&fixture.path().join("paused"), &mut owner);
        let intent: Option<Value> = std::fs::read(project.join(&layout.install_intent))
            .ok()
            .map(|bytes| serde_json::from_slice(&bytes).unwrap());
        owner.0.kill().unwrap();
        owner.0.wait().unwrap();
        let mut recovery = spawn(fixture.path(), "contender");
        wait_success(&mut recovery);
        assert_eq!(outcome(fixture.path(), "contender"), "success", "{stage:?}");
        assert!(!project.join(&layout.install_intent).exists());
        if let Some(intent) = intent {
            let token =
                crate::engine::token_key::read_token_key(&project.join(&layout.token_key)).unwrap();
            let credential = crate::engine::token_key::read_api_key_document(
                &project.join(&layout.operator_credential),
            )
            .unwrap();
            assert_eq!(digest::sha256_hex(&token), intent["token_key_sha256"]);
            assert_eq!(
                digest::sha256_hex(credential.credential().as_bytes()),
                intent["credential_sha256"]
            );
        }
    }
}

#[test]
fn unrelated_projects_can_install_while_another_project_is_paused() {
    let first = fixture();
    let second = fixture();
    let mut owner = spawn(first.path(), "owner");
    wait_file(&first.path().join("paused"), &mut owner);
    let mut independent = spawn(second.path(), "contender");
    wait_success(&mut independent);
    assert_eq!(outcome(second.path(), "contender"), "success");
    assert!(!first.path().join("project/.rrflow").exists());
    std::fs::write(first.path().join("release"), b"continue").unwrap();
    wait_success(&mut owner);
}

#[test]
fn lease_admission_rejects_nonempty_directory_and_hardlink_without_repair() {
    use crate::engine::token_key::write_private_new;
    for kind in ["nonempty", "directory", "hardlink"] {
        let root = tempfile::tempdir().unwrap();
        let project = root.path().join("project");
        std::fs::create_dir(&project).unwrap();
        let path = project.join(rrd_contract::INSTALLATION_LEASE_PATH);
        match kind {
            "nonempty" => write_private_new(&path, b"not a lease").unwrap(),
            "directory" => std::fs::create_dir(&path).unwrap(),
            _ => {
                write_private_new(&path, b"").unwrap();
                std::fs::hard_link(&path, root.path().join("alias")).unwrap();
            }
        }
        let before = tree_digests(&project);
        assert!(RrdEngine::plan_installation(
            &project,
            InstallationTargetKind::FreshProject,
            &"1".repeat(64),
            "default",
            None,
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("Cargo.toml")
                .as_path()
        )
        .is_err());
        assert_eq!(tree_digests(&project), before);
    }
}

#[test]
fn interrupted_admission_retains_an_empty_lease_and_deterministic_preview() {
    let project = tempfile::tempdir().unwrap();
    let executable = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let preview = RrdEngine::plan_installation(
        project.path(),
        InstallationTargetKind::FreshProject,
        &"1".repeat(64),
        "default",
        None,
        &executable,
    )
    .unwrap();
    let error = RrdEngine::apply_installation_with_failure(
        project.path(),
        InstallationTargetKind::FreshProject,
        &preview,
        &preview.installation.plan_sha256,
        1_800_000_000_000,
        &executable,
        InstallApplyStage::LeaseAcquired,
    )
    .unwrap_err();
    assert!(error.to_string().contains("test installation failpoint"));
    let lease = project.path().join(rrd_contract::INSTALLATION_LEASE_PATH);
    assert_eq!(std::fs::read_dir(project.path()).unwrap().count(), 1);
    assert_eq!(std::fs::metadata(&lease).unwrap().len(), 0);
    assert_eq!(
        RrdEngine::plan_installation(
            project.path(),
            InstallationTargetKind::FreshProject,
            &"1".repeat(64),
            "default",
            None,
            &executable
        )
        .unwrap(),
        preview
    );
    // A correctly named file is not enough: malformed lease state is preserved
    // and denied before any prepared credential or engine state is created.
    std::fs::write(&lease, b"foreign payload").unwrap();
    let before = tree_digests(project.path());
    assert!(RrdEngine::apply_installation_at(
        project.path(),
        InstallationTargetKind::FreshProject,
        &preview,
        &preview.installation.plan_sha256,
        1_800_000_000_000,
        &executable
    )
    .is_err());
    assert_eq!(tree_digests(project.path()), before);
}

#[test]
fn live_recovery_owns_the_same_project_lease_as_initial_apply() {
    let fixture = fixture();
    let mut initial = spawn(fixture.path(), "owner");
    wait_file(&fixture.path().join("paused"), &mut initial);
    initial.0.kill().unwrap();
    initial.0.wait().unwrap();
    std::fs::remove_file(fixture.path().join("paused")).unwrap();
    let mut recovery = spawn(fixture.path(), "owner");
    wait_file(&fixture.path().join("paused"), &mut recovery);
    drop(initial); // Closing an old owner must never unlink newer admission.
    let before = tree_digests(&fixture.path().join("project"));
    let mut contender = spawn(fixture.path(), "contender");
    wait_success(&mut contender);
    assert_eq!(outcome(fixture.path(), "contender"), "conflict");
    assert_eq!(tree_digests(&fixture.path().join("project")), before);
    std::fs::write(fixture.path().join("release"), b"continue").unwrap();
    wait_success(&mut recovery);
    assert_eq!(outcome(fixture.path(), "owner"), "success");
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
