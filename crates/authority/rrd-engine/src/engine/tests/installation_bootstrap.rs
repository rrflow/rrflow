//! Independent observations of installation's previewed effect boundary.
use super::*;
use crate::engine::installation::recovery::InstallApplyStage;
use rrd_contract::InstallationTargetKind;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

#[test]
fn installation_preview_covers_transient_intent_before_storage_creation() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("README.md"), b"project-owned content\n").unwrap();
    let original: BTreeSet<PathBuf> = std::fs::read_dir(project.path())
        .unwrap()
        .map(|entry| PathBuf::from(entry.unwrap().file_name()))
        .collect();
    let executable = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let preview = RrdEngine::plan_installation(
        project.path(),
        InstallationTargetKind::ExistingProject,
        &"1".repeat(64),
        "default",
        None,
        &executable,
    )
    .unwrap();
    let declared: BTreeSet<PathBuf> = preview
        .installation
        .managed_paths
        .iter()
        .map(|path| PathBuf::from(&path.relative_path))
        .collect();
    let mut observed_intent_boundary = false;
    let result = RrdEngine::apply_installation_with_test_observer(
        project.path(),
        &preview,
        &executable,
        &mut |stage, _| {
            if stage == InstallApplyStage::IntentPublished {
                observed_intent_boundary = true;
                // Enumerate actual effects, independently of EstateLayout and
                // the intent filename constructor. Never read secret payloads.
                for entry in std::fs::read_dir(project.path()).unwrap() {
                    let relative = PathBuf::from(entry.unwrap().file_name());
                    if !original.contains(&relative) {
                        assert!(
                            declared.contains(&relative),
                            "installation created an unpreviewed path: {}",
                            relative.display()
                        );
                    }
                }
                return Err(ServiceError::Storage("observed intent boundary".into()));
            }
            Ok(())
        },
    );
    assert!(observed_intent_boundary);
    assert!(
        result.is_err(),
        "observer must stop before storage creation"
    );
    assert!(!project.path().join(".rrflow").exists());
    assert_eq!(
        std::fs::read(project.path().join("README.md")).unwrap(),
        b"project-owned content\n"
    );
}
