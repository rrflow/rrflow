use super::*;
use crate::engine::estate_control::EngineLocalBackupDriver;
use rrd_contract::InstallationTargetKind;
use rrd_estate::{BackupDriverRequest, DriverErrorKind, EstateBackupDriver};
use rrd_store::BackupCoverage;
use std::path::{Path, PathBuf};

fn test_executable() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml")
}

fn foreign_executable() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs")
}

fn canonical(value: &str) -> CanonicalId {
    CanonicalId::new(value).unwrap()
}

fn state_root(temporary: &tempfile::TempDir) -> PathBuf {
    let root = temporary.path().join("state");
    for path in [root.clone(), root.join("instances"), root.join("processes")] {
        std::fs::create_dir(&path).unwrap();
    }
    std::fs::canonicalize(root).unwrap()
}

fn request(instance: &str) -> BackupDriverRequest {
    BackupDriverRequest {
        estate_id: canonical("estate-a"),
        job_id: canonical("backup-one"),
        instance_id: canonical(instance),
        source_generation: 1,
        label: "daily.0001".into(),
        created_at: 500,
    }
}

/// Installs `instances/<instance>` canonically and returns the installed
/// engine's own instance identity, which names the catalogue root.
fn install(state_root: &Path, instance: &str) -> CanonicalId {
    let project = state_root.join("instances").join(instance);
    std::fs::create_dir(&project).unwrap();
    let preview = RrdEngine::plan_installation(
        &project,
        InstallationTargetKind::ExistingProject,
        "default",
        None,
        &test_executable(),
    )
    .unwrap();
    RrdEngine::apply_installation(
        &project,
        InstallationTargetKind::ExistingProject,
        &preview,
        &preview.installation.plan_sha256,
        100,
        &test_executable(),
    )
    .unwrap();
    preview.installation.target.instance_id
}

#[test]
fn a_managed_instance_backup_lands_in_its_own_installed_catalogue() {
    let temporary = tempfile::tempdir().unwrap();
    let state_root = state_root(&temporary);
    let installed = install(&state_root, "project-a");
    let mut driver = EngineLocalBackupDriver::new(&state_root, test_executable()).unwrap();

    let result = driver.create(&request("project-a")).unwrap();

    let project = state_root.join("instances/project-a");
    let catalogue_root = project.join(".rrflow/rrd/backups").join(installed.as_str());
    let catalogue = rrd_store::verify_backup_catalogue(&catalogue_root).unwrap();
    assert_eq!(catalogue.catalogue_sha256, result.catalogue_sha256);
    let entry = catalogue
        .backups
        .iter()
        .find(|entry| entry.backup_id == result.backup_id)
        .expect("the estate backup is catalogued where the served engine lists backups");
    assert_eq!(entry.archive.archive_sha256, result.archive_sha256);
    assert!(entry.application_complete);
    assert_eq!(entry.installed_control, BackupCoverage::Included);
    assert!(
        !state_root.join("backups").exists(),
        "no estate-level catalogue lane exists beside the instance"
    );
    // The instance is untouched and still opens as the installed estate.
    drop(RrdEngine::open_installed(&project, &test_executable()).unwrap());
}

#[test]
fn a_pre_canonical_component_root_is_refused_without_a_backup() {
    let temporary = tempfile::tempdir().unwrap();
    let state_root = state_root(&temporary);
    // The layout the driver used to accept: a bare store at `.rrflow/rrd`
    // with no locator, no installed record, and no token key.
    let project = state_root.join("instances/project-b");
    let component = project.join(".rrflow/rrd");
    std::fs::create_dir_all(&component).unwrap();
    drop(RrdEngine::create_new(&component, canonical("project-b"), [0_u8; 32]).unwrap());
    let before = std::fs::read_dir(&component).unwrap().count();
    let mut driver = EngineLocalBackupDriver::new(&state_root, test_executable()).unwrap();

    let error = driver.create(&request("project-b")).unwrap_err();

    assert_eq!(error.kind, DriverErrorKind::Permanent);
    assert!(
        error.message.contains("installed instance open failed"),
        "{}",
        error.message
    );
    assert_eq!(std::fs::read_dir(&component).unwrap().count(), before);
    assert!(!component.join("backups").exists());
    assert!(!state_root.join("backups").exists());
}

#[test]
fn a_foreign_distribution_cannot_back_up_a_managed_instance() {
    let temporary = tempfile::tempdir().unwrap();
    let state_root = state_root(&temporary);
    let installed = install(&state_root, "project-c");
    let mut driver = EngineLocalBackupDriver::new(&state_root, foreign_executable()).unwrap();

    let error = driver.create(&request("project-c")).unwrap_err();

    assert_eq!(error.kind, DriverErrorKind::Permanent);
    assert!(
        error.message.contains("ProjectBindingMismatch"),
        "{}",
        error.message
    );
    assert!(!state_root
        .join("instances/project-c/.rrflow/rrd/backups")
        .join(installed.as_str())
        .exists());
}

#[test]
fn an_absent_managed_instance_is_refused_before_anything_is_created() {
    let temporary = tempfile::tempdir().unwrap();
    let state_root = state_root(&temporary);
    let mut driver = EngineLocalBackupDriver::new(&state_root, test_executable()).unwrap();

    let error = driver.create(&request("project-d")).unwrap_err();

    assert_eq!(error.kind, DriverErrorKind::Permanent);
    assert!(!state_root.join("instances/project-d").exists());
}
