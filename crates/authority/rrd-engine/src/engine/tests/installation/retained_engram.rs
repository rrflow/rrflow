//! An existing `.rrflow` and `.rrflow/rrd` are the project's own: they are
//! retained untouched, bound by native identity and security, and the engram
//! installs beside them. Each test fixes one way that binding could be lost.

use super::*;

#[cfg(windows)]
#[test]
pub(super) fn retained_engram_does_not_accept_an_identical_replacement_as_its_bootstrap() {
    let root = tempfile::tempdir().unwrap();
    let engram = root.path().join(".rrflow");
    std::fs::create_dir(&engram).unwrap();
    let preview = plan(root.path());
    let executable = test_executable();
    RrdEngine::apply_installation_at(
        root.path(),
        InstallationTargetKind::ExistingProject,
        &preview,
        &preview.installation.plan_sha256,
        1_800_000_000_000,
        &executable,
    )
    .unwrap();
    let bootstrap = engram.join("bootstrap.md");
    let original = engram.join("original-bootstrap.md");
    let contents = std::fs::read(&bootstrap).unwrap();
    std::fs::rename(&bootstrap, &original).unwrap();
    std::fs::write(&bootstrap, &contents).unwrap();
    assert!(open_at(root.path(), &executable, 1_800_000_001_000).is_err());
    assert_eq!(std::fs::read(&bootstrap).unwrap(), contents);
    assert_eq!(std::fs::read(&original).unwrap(), contents);
}

#[cfg(windows)]
#[test]
pub(super) fn retained_engram_preserves_a_concurrent_project_pending_file() {
    use crate::engine::installation::recovery::InstallApplyStage;
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join(".rrflow")).unwrap();
    let preview = plan(root.path());
    let pending = root.path().join(format!(
        ".rrflow/.bootstrap.{}.pending",
        preview.installation.storage_root_id
    ));
    let content = preview.installation.bootstrap.rendered_instructions();
    let result = RrdEngine::apply_installation_with_test_observer(
        root.path(),
        &preview,
        &test_executable(),
        &mut |stage, _| {
            if stage == InstallApplyStage::IntentPublished {
                std::fs::write(&pending, content.as_bytes()).unwrap();
            }
            Ok(())
        },
    );
    result.expect("a user-owned file outside the planned private staging must be preserved");
    assert_eq!(std::fs::read(&pending).unwrap(), content.as_bytes());
    assert!(root.path().join(".rrflow/bootstrap.md").exists());
}

#[cfg(windows)]
#[test]
pub(super) fn retained_engram_refuses_unproven_private_staging_without_removing_it() {
    use crate::engine::installation::recovery::InstallApplyStage;
    for complete in [false, true] {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join(".rrflow")).unwrap();
        let preview = plan(root.path());
        let pending = root.path().join(
            &preview
                .installation
                .managed_path(rrd_contract::InstallationManagedPathKind::BootstrapPending)
                .relative_path,
        );
        let contents = if complete {
            preview
                .installation
                .bootstrap
                .rendered_instructions()
                .into_bytes()
        } else {
            b"incomplete staging".to_vec()
        };
        let result = RrdEngine::apply_installation_with_test_observer(
            root.path(),
            &preview,
            &test_executable(),
            &mut |stage, _| {
                if stage == InstallApplyStage::EngramDirectoriesReady {
                    std::fs::write(&pending, &contents).unwrap();
                }
                Ok(())
            },
        );
        assert!(result.is_err());
        assert_eq!(std::fs::read(&pending).unwrap(), contents);
        assert!(!root.path().join(".rrflow/bootstrap.md").exists());
    }
}

#[cfg(windows)]
#[test]
pub(super) fn retained_engram_rejects_a_concurrent_identical_bootstrap_file() {
    use crate::engine::installation::recovery::InstallApplyStage;
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join(".rrflow")).unwrap();
    let preview = plan(root.path());
    let bootstrap = root.path().join(".rrflow/bootstrap.md");
    let content = preview.installation.bootstrap.rendered_instructions();
    let result = RrdEngine::apply_installation_with_test_observer(
        root.path(),
        &preview,
        &test_executable(),
        &mut |stage, _| {
            if stage == InstallApplyStage::IntentPublished {
                std::fs::write(&bootstrap, content.as_bytes()).unwrap();
            }
            Ok(())
        },
    );
    assert!(
        result.is_err(),
        "matching bytes do not prove RRFlow owns a concurrent file"
    );
    assert_eq!(std::fs::read(&bootstrap).unwrap(), content.as_bytes());
}

#[cfg(windows)]
#[test]
pub(super) fn retained_engram_rejects_changed_ancestors_before_writes_and_can_replan() {
    let root = tempfile::tempdir().unwrap();
    let engram = root.path().join(".rrflow");
    std::fs::create_dir(&engram).unwrap();
    std::fs::write(engram.join("user-note"), b"preserved").unwrap();
    let preview = plan(root.path());
    std::fs::write(engram.join("ordinary-edit"), b"new project content").unwrap();
    let before = inventory(root.path());
    let executable = test_executable();
    assert!(RrdEngine::apply_installation_at(
        root.path(),
        InstallationTargetKind::ExistingProject,
        &preview,
        &preview.installation.plan_sha256,
        1_800_000_000_000,
        &executable
    )
    .is_err());
    assert_eq!(
        inventory(root.path()),
        before,
        "stale preview must not write"
    );
    let replacement = plan(root.path());
    assert_ne!(
        preview.installation.plan_sha256,
        replacement.installation.plan_sha256
    );
    RrdEngine::apply_installation_at(
        root.path(),
        InstallationTargetKind::ExistingProject,
        &replacement,
        &replacement.installation.plan_sha256,
        1_800_000_000_001,
        &executable,
    )
    .unwrap();
    assert_eq!(
        std::fs::read(engram.join("ordinary-edit")).unwrap(),
        b"new project content"
    );
    assert_eq!(
        std::fs::read(engram.join("user-note")).unwrap(),
        b"preserved"
    );
}

#[cfg(windows)]
#[test]
pub(super) fn retained_engram_rejects_replaced_directory_before_mutation() {
    let root = tempfile::tempdir().unwrap();
    let engram = root.path().join(".rrflow");
    std::fs::create_dir(&engram).unwrap();
    std::fs::write(engram.join("note"), b"same bytes").unwrap();
    let preview = plan(root.path());
    std::fs::rename(&engram, root.path().join("retained-old-directory")).unwrap();
    std::fs::create_dir(&engram).unwrap();
    std::fs::write(engram.join("note"), b"same bytes").unwrap();
    let before = inventory(root.path());
    let result = RrdEngine::apply_installation_at(
        root.path(),
        InstallationTargetKind::ExistingProject,
        &preview,
        &preview.installation.plan_sha256,
        1_800_000_000_000,
        &test_executable(),
    );
    assert!(result.is_err());
    assert_eq!(inventory(root.path()), before);
}

#[cfg(windows)]
#[test]
pub(super) fn retained_engram_rejects_changed_acl_before_mutation() {
    let root = tempfile::tempdir().unwrap();
    let engram = root.path().join(".rrflow");
    std::fs::create_dir(&engram).unwrap();
    let preview = plan(root.path());
    let output = std::process::Command::new("icacls")
        .arg(&engram)
        .arg("/inheritance:d")
        .output()
        .unwrap();
    assert!(output.status.success(), "fixture ACL change failed");
    let before = inventory(root.path());
    let result = RrdEngine::apply_installation_at(
        root.path(),
        InstallationTargetKind::ExistingProject,
        &preview,
        &preview.installation.plan_sha256,
        1_800_000_000_000,
        &test_executable(),
    );
    assert!(
        result.is_err(),
        "a changed permissions descriptor must invalidate the preview"
    );
    assert_eq!(inventory(root.path()), before);
}
