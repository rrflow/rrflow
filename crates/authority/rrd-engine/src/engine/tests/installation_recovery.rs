use super::*;
use crate::engine::clock::FixedClock;
use crate::engine::estate_layout::EstateLayout;
use crate::engine::installation::recovery::InstallApplyStage;
use crate::engine::token_key::{read_api_key_document, read_token_key};
use crate::InstallationPreview;
use rrd_contract::{InstallationManagedPathKind, InstallationTargetKind};
use rrd_core::digest;
use rrd_store::ControlTransition;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const FIRST_APPLY_MS: u64 = 1_800_000_000_000;

fn test_executable() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml")
}

fn project_fixture() -> tempfile::TempDir {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("README.md"), "walking product\n").unwrap();
    project
}

fn plan(project: &Path, executable: &Path) -> InstallationPreview {
    RrdEngine::plan_installation(
        project,
        InstallationTargetKind::ExistingProject,
        "default",
        None,
        executable,
    )
    .unwrap()
}

fn layout(preview: &InstallationPreview) -> EstateLayout {
    EstateLayout::new(
        &preview.installation.storage_root_id,
        &preview.installation.initial_principal_id,
    )
    .unwrap()
}

fn managed_path(
    project: &Path,
    preview: &InstallationPreview,
    kind: InstallationManagedPathKind,
) -> PathBuf {
    project.join(&preview.installation.managed_path(kind).relative_path)
}

fn file_inventory(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn visit(root: &Path, current: &Path, output: &mut BTreeMap<PathBuf, Vec<u8>>) {
        let mut entries = std::fs::read_dir(current)
            .unwrap()
            .collect::<std::io::Result<Vec<_>>>()
            .unwrap();
        entries.sort_by_key(std::fs::DirEntry::file_name);
        for entry in entries {
            let path = entry.path();
            let metadata = std::fs::symlink_metadata(&path).unwrap();
            if metadata.is_dir() && !metadata.file_type().is_symlink() {
                visit(root, &path, output);
            } else if metadata.file_type().is_symlink() {
                output.insert(
                    path.strip_prefix(root).unwrap().to_owned(),
                    std::fs::read_link(&path)
                        .unwrap()
                        .as_os_str()
                        .as_encoded_bytes()
                        .to_vec(),
                );
            } else {
                output.insert(
                    path.strip_prefix(root).unwrap().to_owned(),
                    std::fs::read(path).unwrap(),
                );
            }
        }
    }

    let mut output = BTreeMap::new();
    visit(root, root, &mut output);
    output
}

fn count_named_files(root: &Path, expected_name: &str) -> usize {
    fn visit(current: &Path, expected_name: &str, count: &mut usize) {
        for entry in std::fs::read_dir(current).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            let metadata = std::fs::symlink_metadata(&path).unwrap();
            if metadata.is_dir() && !metadata.file_type().is_symlink() {
                visit(&path, expected_name, count);
            } else if entry.file_name() == expected_name {
                *count += 1;
            }
        }
    }

    let mut count = 0;
    visit(root, expected_name, &mut count);
    count
}

fn intent_evidence(path: &Path) -> (String, String, u64) {
    let value: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    (
        value["token_key_sha256"].as_str().unwrap().to_owned(),
        value["credential_sha256"].as_str().unwrap().to_owned(),
        value["clock_anchor"]["observed_at_unix_ms"]
            .as_u64()
            .unwrap(),
    )
}

fn materialized_secret_evidence(project: &Path, preview: &InstallationPreview) -> (String, String) {
    let token = read_token_key(&managed_path(
        project,
        preview,
        InstallationManagedPathKind::TokenKey,
    ))
    .unwrap();
    let credential = read_api_key_document(&managed_path(
        project,
        preview,
        InstallationManagedPathKind::OperatorCredential,
    ))
    .unwrap();
    (
        digest::sha256_hex(&token),
        digest::sha256_hex(credential.credential().as_bytes()),
    )
}

#[test]
fn interrupted_apply_recovers_at_every_durable_stage_without_duplicate_authority() {
    let stages = [
        InstallApplyStage::IntentPublished,
        InstallApplyStage::EstateDirectoriesReady,
        InstallApplyStage::StorageReady,
        InstallApplyStage::TokenReady,
        InstallApplyStage::CredentialReady,
        InstallApplyStage::EngineCommitted,
        InstallApplyStage::LocatorPublished,
        InstallApplyStage::IntentAcknowledged,
    ];

    for stage in stages {
        let project = project_fixture();
        let executable = test_executable();
        let preview = plan(project.path(), &executable);
        let estate = layout(&preview);
        let intent_path = project.path().join(&estate.install_intent);

        let error = RrdEngine::apply_installation_with_failure(
            project.path(),
            InstallationTargetKind::ExistingProject,
            &preview,
            &preview.installation.plan_sha256,
            FIRST_APPLY_MS,
            &executable,
            stage,
        )
        .unwrap_err();
        assert!(error.to_string().contains("test installation failpoint"));

        let (token_sha256, credential_sha256, applied_at_unix_ms) = if intent_path.exists() {
            intent_evidence(&intent_path)
        } else {
            let (token, credential) = materialized_secret_evidence(project.path(), &preview);
            (token, credential, FIRST_APPLY_MS)
        };

        let recovered = RrdEngine::apply_installation_at(
            project.path(),
            InstallationTargetKind::ExistingProject,
            &preview,
            &preview.installation.plan_sha256,
            FIRST_APPLY_MS + 77,
            &executable,
        )
        .unwrap();
        assert_eq!(recovered.applied_at_unix_ms, applied_at_unix_ms);
        assert_eq!(recovered.credential_sha256, credential_sha256);
        assert_eq!(
            recovered.idempotent_replay,
            matches!(
                stage,
                InstallApplyStage::EngineCommitted
                    | InstallApplyStage::LocatorPublished
                    | InstallApplyStage::IntentAcknowledged
            )
        );
        assert!(!intent_path.exists());
        assert!(!project.path().join(&estate.locator_pending).exists());

        let (materialized_token, materialized_credential) =
            materialized_secret_evidence(project.path(), &preview);
        assert_eq!(materialized_token, token_sha256);
        assert_eq!(materialized_credential, credential_sha256);
        assert_eq!(count_named_files(project.path(), "RRD.TOKEN"), 1);
        let credential_name = Path::new(&estate.operator_credential)
            .file_name()
            .unwrap()
            .to_str()
            .unwrap();
        assert_eq!(count_named_files(project.path(), credential_name), 1);

        let report = RrdEngine::inspect_installed_with_clock(
            project.path(),
            &executable,
            &FixedClock::at(FIRST_APPLY_MS + 88),
        )
        .unwrap();
        assert_eq!(report.runtime_cursor, recovered.runtime_cursor);
        assert_eq!(
            report.control_journal_sequence,
            recovered.control_journal_sequence
        );
        assert!(report.attunement_source_current);

        let before_replay = file_inventory(project.path());
        let replay = RrdEngine::apply_installation_at(
            project.path(),
            InstallationTargetKind::ExistingProject,
            &preview,
            &preview.installation.plan_sha256,
            FIRST_APPLY_MS + 99,
            &executable,
        )
        .unwrap();
        assert!(replay.idempotent_replay);
        assert_eq!(
            replay.installed_record_sha256,
            recovered.installed_record_sha256
        );
        assert_eq!(
            replay.runtime_manifest_sha256,
            recovered.runtime_manifest_sha256
        );
        assert_eq!(file_inventory(project.path()), before_replay);
    }
}

#[test]
fn interrupted_recovery_rejects_clock_rollback_without_changing_owned_state() {
    let project = project_fixture();
    let executable = test_executable();
    let preview = plan(project.path(), &executable);

    RrdEngine::apply_installation_with_failure(
        project.path(),
        InstallationTargetKind::ExistingProject,
        &preview,
        &preview.installation.plan_sha256,
        FIRST_APPLY_MS,
        &executable,
        InstallApplyStage::IntentPublished,
    )
    .unwrap_err();
    let owned_state = file_inventory(project.path());

    let error = RrdEngine::apply_installation_at(
        project.path(),
        InstallationTargetKind::ExistingProject,
        &preview,
        &preview.installation.plan_sha256,
        FIRST_APPLY_MS - 1,
        &executable,
    )
    .unwrap_err();
    assert!(matches!(
        error,
        ServiceError::ClockRollback {
            observed_at_unix_ms,
            anchor_unix_ms,
            maximum_rollback_ms: 0
        } if observed_at_unix_ms == FIRST_APPLY_MS - 1 && anchor_unix_ms == FIRST_APPLY_MS
    ));
    assert_eq!(file_inventory(project.path()), owned_state);

    let recovered = RrdEngine::apply_installation_at(
        project.path(),
        InstallationTargetKind::ExistingProject,
        &preview,
        &preview.installation.plan_sha256,
        FIRST_APPLY_MS + 1,
        &executable,
    )
    .unwrap();
    assert_eq!(recovered.applied_at_unix_ms, FIRST_APPLY_MS);
}

#[test]
fn owned_intent_rejects_project_and_executable_drift_without_writes() {
    let project = project_fixture();
    let distribution = tempfile::tempdir().unwrap();
    let executable = distribution.path().join("rrflow-test-binary");
    std::fs::write(&executable, b"distribution-v1").unwrap();
    let preview = plan(project.path(), &executable);
    let estate = layout(&preview);

    RrdEngine::apply_installation_with_failure(
        project.path(),
        InstallationTargetKind::ExistingProject,
        &preview,
        &preview.installation.plan_sha256,
        FIRST_APPLY_MS,
        &executable,
        InstallApplyStage::IntentPublished,
    )
    .unwrap_err();

    let before_second_plan = file_inventory(project.path());
    assert!(RrdEngine::plan_installation(
        project.path(),
        InstallationTargetKind::ExistingProject,
        "default",
        None,
        &executable,
    )
    .is_err());
    assert_eq!(file_inventory(project.path()), before_second_plan);

    std::fs::write(&executable, b"distribution-v2").unwrap();
    let before_executable_rejection = file_inventory(project.path());
    assert!(RrdEngine::apply_installation_at(
        project.path(),
        InstallationTargetKind::ExistingProject,
        &preview,
        &preview.installation.plan_sha256,
        FIRST_APPLY_MS + 1,
        &executable,
    )
    .is_err());
    assert_eq!(file_inventory(project.path()), before_executable_rejection);
    std::fs::write(&executable, b"distribution-v1").unwrap();

    std::fs::write(project.path().join("README.md"), "project drift\n").unwrap();
    let before_project_rejection = file_inventory(project.path());
    assert!(RrdEngine::apply_installation_at(
        project.path(),
        InstallationTargetKind::ExistingProject,
        &preview,
        &preview.installation.plan_sha256,
        FIRST_APPLY_MS + 2,
        &executable,
    )
    .is_err());
    assert_eq!(file_inventory(project.path()), before_project_rejection);
    assert!(!project.path().join(".rrflow").exists());
    assert!(project.path().join(&estate.install_intent).is_file());

    std::fs::write(project.path().join("README.md"), "walking product\n").unwrap();
    let recovered = RrdEngine::apply_installation_at(
        project.path(),
        InstallationTargetKind::ExistingProject,
        &preview,
        &preview.installation.plan_sha256,
        FIRST_APPLY_MS + 3,
        &executable,
    )
    .unwrap();
    assert_eq!(recovered.applied_at_unix_ms, FIRST_APPLY_MS);
}

#[test]
fn malformed_or_foreign_recovery_state_is_preserved_and_rejected() {
    let executable = test_executable();

    let malformed_project = project_fixture();
    let malformed_preview = plan(malformed_project.path(), &executable);
    let malformed_intent = malformed_project
        .path()
        .join(layout(&malformed_preview).install_intent);
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    use std::io::Write as _;
    options
        .open(&malformed_intent)
        .unwrap()
        .write_all(b"not-an-install-intent")
        .unwrap();
    let malformed_before = file_inventory(malformed_project.path());
    assert!(RrdEngine::apply_installation_at(
        malformed_project.path(),
        InstallationTargetKind::ExistingProject,
        &malformed_preview,
        &malformed_preview.installation.plan_sha256,
        FIRST_APPLY_MS,
        &executable,
    )
    .is_err());
    assert_eq!(file_inventory(malformed_project.path()), malformed_before);
    assert!(!malformed_project.path().join(".rrflow").exists());

    let unknown_project = project_fixture();
    let unknown_preview = plan(unknown_project.path(), &executable);
    RrdEngine::apply_installation_with_failure(
        unknown_project.path(),
        InstallationTargetKind::ExistingProject,
        &unknown_preview,
        &unknown_preview.installation.plan_sha256,
        FIRST_APPLY_MS,
        &executable,
        InstallApplyStage::IntentPublished,
    )
    .unwrap_err();
    std::fs::create_dir(unknown_project.path().join(".rrflow")).unwrap();
    std::fs::write(
        unknown_project.path().join(".rrflow/foreign-authority"),
        b"unowned",
    )
    .unwrap();
    let unknown_before = file_inventory(unknown_project.path());
    assert!(RrdEngine::apply_installation_at(
        unknown_project.path(),
        InstallationTargetKind::ExistingProject,
        &unknown_preview,
        &unknown_preview.installation.plan_sha256,
        FIRST_APPLY_MS + 1,
        &executable,
    )
    .is_err());
    assert_eq!(file_inventory(unknown_project.path()), unknown_before);
}

#[test]
fn partial_control_state_and_missing_committed_secret_fail_closed() {
    let executable = test_executable();

    let partial_project = project_fixture();
    let partial_preview = plan(partial_project.path(), &executable);
    RrdEngine::apply_installation_with_failure(
        partial_project.path(),
        InstallationTargetKind::ExistingProject,
        &partial_preview,
        &partial_preview.installation.plan_sha256,
        FIRST_APPLY_MS,
        &executable,
        InstallApplyStage::CredentialReady,
    )
    .unwrap_err();
    let storage_root = managed_path(
        partial_project.path(),
        &partial_preview,
        InstallationManagedPathKind::StorageRoot,
    );
    let token = read_token_key(&managed_path(
        partial_project.path(),
        &partial_preview,
        InstallationManagedPathKind::TokenKey,
    ))
    .unwrap();
    let engine = RrdEngine::open_existing(
        &storage_root,
        partial_preview.installation.target.instance_id.clone(),
        token,
    )
    .unwrap();
    engine
        .storage
        .control()
        .commit(&ControlTransition {
            key: "server/state/test/partial-install".into(),
            expected: None,
            replacement: Some(b"partial".to_vec()),
            at: FIRST_APPLY_MS + 1,
            actor: "test".into(),
            action: "partial.install".into(),
            request_id: "partial-request".into(),
            operation_id: "partial-operation".into(),
        })
        .unwrap();
    drop(engine);
    let partial_before = file_inventory(partial_project.path());
    assert!(RrdEngine::apply_installation_at(
        partial_project.path(),
        InstallationTargetKind::ExistingProject,
        &partial_preview,
        &partial_preview.installation.plan_sha256,
        FIRST_APPLY_MS + 2,
        &executable,
    )
    .is_err());
    assert_eq!(file_inventory(partial_project.path()), partial_before);

    let missing_project = project_fixture();
    let missing_preview = plan(missing_project.path(), &executable);
    RrdEngine::apply_installation_with_failure(
        missing_project.path(),
        InstallationTargetKind::ExistingProject,
        &missing_preview,
        &missing_preview.installation.plan_sha256,
        FIRST_APPLY_MS,
        &executable,
        InstallApplyStage::EngineCommitted,
    )
    .unwrap_err();
    let credential_path = managed_path(
        missing_project.path(),
        &missing_preview,
        InstallationManagedPathKind::OperatorCredential,
    );
    std::fs::remove_file(&credential_path).unwrap();
    let missing_before = file_inventory(missing_project.path());
    assert!(RrdEngine::apply_installation_at(
        missing_project.path(),
        InstallationTargetKind::ExistingProject,
        &missing_preview,
        &missing_preview.installation.plan_sha256,
        FIRST_APPLY_MS + 1,
        &executable,
    )
    .is_err());
    assert_eq!(file_inventory(missing_project.path()), missing_before);
    assert!(!credential_path.exists());
}

#[test]
fn published_locator_recovers_only_its_exact_pending_link() {
    let executable = test_executable();

    let exact_project = project_fixture();
    let exact_preview = plan(exact_project.path(), &executable);
    let exact_layout = layout(&exact_preview);
    RrdEngine::apply_installation_with_failure(
        exact_project.path(),
        InstallationTargetKind::ExistingProject,
        &exact_preview,
        &exact_preview.installation.plan_sha256,
        FIRST_APPLY_MS,
        &executable,
        InstallApplyStage::LocatorPublished,
    )
    .unwrap_err();
    let locator = std::fs::read(exact_project.path().join(".rrflow/config.toml")).unwrap();
    let pending = exact_project.path().join(&exact_layout.locator_pending);
    std::fs::write(&pending, &locator).unwrap();
    let recovered = RrdEngine::apply_installation_at(
        exact_project.path(),
        InstallationTargetKind::ExistingProject,
        &exact_preview,
        &exact_preview.installation.plan_sha256,
        FIRST_APPLY_MS + 1,
        &executable,
    )
    .unwrap();
    assert!(recovered.idempotent_replay);
    assert!(!pending.exists());
    assert!(!exact_project
        .path()
        .join(&exact_layout.install_intent)
        .exists());
    assert_eq!(
        std::fs::read(exact_project.path().join(".rrflow/config.toml")).unwrap(),
        locator
    );

    let foreign_project = project_fixture();
    let foreign_preview = plan(foreign_project.path(), &executable);
    let foreign_layout = layout(&foreign_preview);
    RrdEngine::apply_installation_with_failure(
        foreign_project.path(),
        InstallationTargetKind::ExistingProject,
        &foreign_preview,
        &foreign_preview.installation.plan_sha256,
        FIRST_APPLY_MS,
        &executable,
        InstallApplyStage::LocatorPublished,
    )
    .unwrap_err();
    std::fs::write(
        foreign_project.path().join(&foreign_layout.locator_pending),
        b"foreign pending locator",
    )
    .unwrap();
    let before = file_inventory(foreign_project.path());
    assert!(RrdEngine::apply_installation_at(
        foreign_project.path(),
        InstallationTargetKind::ExistingProject,
        &foreign_preview,
        &foreign_preview.installation.plan_sha256,
        FIRST_APPLY_MS + 1,
        &executable,
    )
    .is_err());
    assert_eq!(file_inventory(foreign_project.path()), before);
}

#[cfg(unix)]
#[test]
fn symbolic_intent_collision_is_preserved_and_rejected() {
    use std::os::unix::fs::symlink;

    let project = project_fixture();
    let executable = test_executable();
    let preview = plan(project.path(), &executable);
    let target = project.path().join("foreign-intent");
    std::fs::write(&target, b"foreign").unwrap();
    let intent = project.path().join(layout(&preview).install_intent);
    symlink(&target, &intent).unwrap();
    let before = file_inventory(project.path());
    assert!(RrdEngine::apply_installation_at(
        project.path(),
        InstallationTargetKind::ExistingProject,
        &preview,
        &preview.installation.plan_sha256,
        FIRST_APPLY_MS,
        &executable,
    )
    .is_err());
    assert_eq!(file_inventory(project.path()), before);
}
