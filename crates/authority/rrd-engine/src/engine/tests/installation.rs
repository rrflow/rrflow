use super::*;
use crate::engine::clock::FixedClock;
use crate::{
    ClockAssessmentStatus, ClockObservationFailure, ClockSourceKind, ClockTrustLevel,
    InstallationPreview, InstallationVerificationReport, InstallationVerificationStatus,
};
use rrd_contract::{installation_plan_sha256, InstallationTargetKind, SessionLease};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn inventory(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn visit(root: &Path, current: &Path, output: &mut BTreeMap<PathBuf, Vec<u8>>) {
        let mut entries = std::fs::read_dir(current)
            .unwrap()
            .collect::<std::io::Result<Vec<_>>>()
            .unwrap();
        entries.sort_by_key(std::fs::DirEntry::file_name);
        for entry in entries {
            let path = entry.path();
            if path.is_dir() {
                visit(root, &path, output);
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

fn plan(project: &Path) -> InstallationPreview {
    RrdEngine::plan_installation(
        project,
        InstallationTargetKind::ExistingProject,
        &"1".repeat(64),
        "default",
        None,
        &test_executable(),
    )
    .unwrap()
}

fn test_executable() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml")
}

fn open_at(project: &Path, executable: &Path, at_unix_ms: u64) -> crate::Result<RrdEngine> {
    RrdEngine::open_installed_with_clock(project, executable, &FixedClock::at(at_unix_ms))
}

fn inspect_at(
    project: &Path,
    executable: &Path,
    at_unix_ms: u64,
) -> crate::Result<InstallationVerificationReport> {
    RrdEngine::inspect_installed_with_clock(project, executable, &FixedClock::at(at_unix_ms))
}

#[test]
fn independent_installations_do_not_reuse_content_identity() {
    for mode in [
        InstallationTargetKind::FreshProject,
        InstallationTargetKind::ExistingProject,
    ] {
        let first = tempfile::tempdir().unwrap();
        let second = tempfile::tempdir().unwrap();
        if mode == InstallationTargetKind::ExistingProject {
            for project in [first.path(), second.path()] {
                std::fs::write(project.join("README.md"), "identical source\n").unwrap();
            }
        }
        let executable = test_executable();
        let first_plan = RrdEngine::plan_installation(
            first.path(),
            mode,
            &RrdEngine::generate_installation_identity().unwrap(),
            "default",
            None,
            &executable,
        )
        .unwrap();
        let second_plan = RrdEngine::plan_installation(
            second.path(),
            mode,
            &RrdEngine::generate_installation_identity().unwrap(),
            "default",
            None,
            &executable,
        )
        .unwrap();
        assert_eq!(
            first_plan.installation.project_precondition_sha256,
            second_plan.installation.project_precondition_sha256
        );
        assert_ne!(
            first_plan.installation.target, second_plan.installation.target,
            "distinct projects with identical content must not share an estate identity"
        );
        let before = inventory(second.path());
        assert!(matches!(
            RrdEngine::apply_installation_at(
                second.path(),
                mode,
                &first_plan,
                &first_plan.installation.plan_sha256,
                1_800_000_000_000,
                &executable,
            ),
            Err(ServiceError::ProjectBindingMismatch)
        ));
        assert_eq!(
            inventory(second.path()),
            before,
            "foreign plan must have no effects"
        );
        for (project, preview) in [(first.path(), &first_plan), (second.path(), &second_plan)] {
            RrdEngine::apply_installation_at(
                project,
                mode,
                preview,
                &preview.installation.plan_sha256,
                1_800_000_000_000,
                &executable,
            )
            .unwrap();
            let reopened = open_at(project, &executable, 1_800_000_000_001).unwrap();
            assert_eq!(
                reopened.installed_estate_identity(),
                Some(&preview.installation.target)
            );
        }
        let first_key = RrdEngine::read_installed_api_key(first.path()).unwrap();
        let second_key = RrdEngine::read_installed_api_key(second.path()).unwrap();
        assert_ne!(first_key.credential(), second_key.credential());
        let engine = open_at(second.path(), &executable, 1_800_000_000_001).unwrap();
        let session = rrd_contract::CreateSession {
            limits: rrd_contract::SessionLimits {
                idle_timeout_ms: 30_000,
                absolute_timeout_ms: 60_000,
                max_open_transactions: 1,
            },
        };
        assert!(engine
            .create_authenticated_session(
                &first_key.principal_id,
                first_key.credential().as_bytes(),
                &session,
                &id("foreign-session"),
                1_800_000_000_002,
                "foreign-request",
                "foreign-operation",
            )
            .is_err());
        assert!(engine
            .create_authenticated_session(
                &second_key.principal_id,
                second_key.credential().as_bytes(),
                &session,
                &id("own-session"),
                1_800_000_000_003,
                "own-request",
                "own-operation",
            )
            .is_ok());
    }
}

#[test]
fn installation_identity_and_root_are_bound_before_effects() {
    let project = tempfile::tempdir().unwrap();
    let executable = test_executable();
    for invalid in [
        String::new(),
        "0".repeat(63),
        "A".repeat(64),
        "z".repeat(64),
    ] {
        assert!(RrdEngine::plan_installation(
            project.path(),
            InstallationTargetKind::FreshProject,
            &invalid,
            "default",
            None,
            &executable
        )
        .is_err());
        assert!(inventory(project.path()).is_empty());
    }
    let preview = plan(project.path());
    for change_identity in [true, false] {
        let mut forged = preview.clone();
        if change_identity {
            forged.installation.installation_identity = "2".repeat(64);
        } else {
            forged.installation.project_root_sha256 = "2".repeat(64);
        }
        forged.installation.plan_sha256 = installation_plan_sha256(&forged.installation).unwrap();
        assert!(RrdEngine::apply_installation_at(
            project.path(),
            InstallationTargetKind::ExistingProject,
            &forged,
            &forged.installation.plan_sha256,
            1_800_000_000_000,
            &executable
        )
        .is_err());
        assert!(inventory(project.path()).is_empty());
    }
}

#[test]
fn planning_is_byte_identical_and_has_no_project_effect() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("README.md"), "walking product\n").unwrap();
    let before = inventory(project.path());
    let first = plan(project.path());
    let second = plan(project.path());
    assert_eq!(first.installation.initial_grants, SecurityAction::ALL);
    assert_eq!(first.installation.managed_paths.len(), 13);
    assert!(serde_json::to_value(&first.installation)
        .unwrap()
        .get("project_root")
        .is_none());
    assert_eq!(first, second);
    assert_eq!(
        serde_json::to_vec(&first).unwrap(),
        serde_json::to_vec(&second).unwrap()
    );
    assert_eq!(inventory(project.path()), before);
}

#[test]
fn configuration_input_rejects_unknown_oversized_and_symbolic_sources() {
    let invalid = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(
        invalid.path(),
        "format_version = 1\nunknown_authority = true\n",
    )
    .unwrap();
    assert!(crate::load_estate_configuration(Some(invalid.path())).is_err());

    let oversized = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(oversized.path(), vec![b' '; 64 * 1024 + 1]).unwrap();
    assert!(crate::load_estate_configuration(Some(oversized.path())).is_err());

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;

        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("configuration.toml");
        let symbolic = directory.path().join("selected.toml");
        std::fs::write(&target, "format_version = 1\n").unwrap();
        symlink(target, &symbolic).unwrap();
        assert!(crate::load_estate_configuration(Some(&symbolic)).is_err());
    }
}

#[test]
fn exact_apply_reopens_and_quick_verification_changes_no_bytes() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("README.md"), "walking product\n").unwrap();
    let preview = plan(project.path());
    let executable = test_executable();
    let result = RrdEngine::apply_installation_at(
        project.path(),
        InstallationTargetKind::ExistingProject,
        &preview,
        &preview.installation.plan_sha256,
        1_800_000_000_000,
        &executable,
    )
    .unwrap();
    assert!(!result.idempotent_replay);
    assert_eq!(result.runtime_cursor, 2);
    assert!(project.path().join(".rrflow/config.toml").is_file());

    assert!(open_at(
        project.path(),
        &project.path().join("README.md"),
        1_800_000_000_001
    )
    .is_err());
    let engine = open_at(project.path(), &executable, 1_800_000_000_001).unwrap();
    assert!(engine.security_enforced().unwrap());
    assert_eq!(
        engine.installed_estate_identity(),
        Some(&preview.installation.target)
    );
    assert_eq!(
        engine.installed_deployment_profile(),
        Some(&preview.installation.deployment)
    );
    assert_eq!(
        engine.estate_configuration(),
        &preview.installation.configuration
    );
    assert_eq!(
        engine.readiness(1_800_000_000_001).unwrap().runtime_cursor,
        2
    );
    drop(engine);

    let before = inventory(project.path());
    let report = inspect_at(project.path(), &executable, 1_800_000_000_001).unwrap();
    assert_eq!(report.status, InstallationVerificationStatus::Passed);
    assert_eq!(report.clock.status, ClockAssessmentStatus::Current);
    assert_eq!(report.clock.anchor.source, ClockSourceKind::HostSystemTime);
    assert_eq!(report.clock.anchor.trust, ClockTrustLevel::Unverified);
    assert_eq!(
        report.clock.anchor.observed_at_unix_ms,
        result.applied_at_unix_ms
    );
    assert!(report.attunement_source_current);
    assert!(report.checks.iter().all(|check| check.passed));
    assert_eq!(inventory(project.path()), before);

    std::fs::write(
        project.path().join("README.md"),
        "walking product under active development\n",
    )
    .unwrap();
    let drift = inspect_at(project.path(), &executable, 1_800_000_000_002).unwrap();
    assert_eq!(drift.status, InstallationVerificationStatus::Passed);
    assert!(!drift.attunement_source_current);
    assert_ne!(
        drift.project_inventory_sha256,
        preview.installation.project_precondition_sha256
    );

    let replay = RrdEngine::apply_installation_at(
        project.path(),
        InstallationTargetKind::ExistingProject,
        &preview,
        &preview.installation.plan_sha256,
        1_800_000_000_010,
        &executable,
    )
    .unwrap();
    assert!(replay.idempotent_replay);
    assert_eq!(replay.plan_sha256, result.plan_sha256);
}

#[test]
fn installed_open_and_verify_never_repair_a_torn_or_missing_active_wal() {
    use std::io::Write as _;

    for missing in [false, true] {
        let project = tempfile::tempdir().unwrap();
        std::fs::write(project.path().join("README.md"), b"project-owned\n").unwrap();
        let preview = plan(project.path());
        let executable = test_executable();
        RrdEngine::apply_installation_at(
            project.path(),
            InstallationTargetKind::ExistingProject,
            &preview,
            &preview.installation.plan_sha256,
            1_800_000_000_000,
            &executable,
        )
        .unwrap();
        let storage = project.path().join(
            &preview
                .installation
                .managed_path(rrd_contract::InstallationManagedPathKind::StorageRoot)
                .relative_path,
        );
        let files = std::fs::read_dir(storage.join("wal"))
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect::<Vec<_>>();
        assert_eq!(files.len(), 1, "fixture must have one committed active WAL");
        if missing {
            std::fs::remove_file(&files[0]).unwrap();
        } else {
            let mut wal = std::fs::OpenOptions::new()
                .append(true)
                .open(&files[0])
                .unwrap();
            wal.write_all(b"incomplete-tail").unwrap();
            wal.sync_all().unwrap();
        }
        let damaged = inventory(project.path());
        assert!(open_at(project.path(), &executable, 1_800_000_000_001).is_err());
        assert_eq!(
            inventory(project.path()),
            damaged,
            "installed open repaired WAL"
        );
        assert!(inspect_at(project.path(), &executable, 1_800_000_000_001).is_err());
        assert_eq!(
            inventory(project.path()),
            damaged,
            "verification repaired WAL"
        );
    }
}

#[test]
fn installed_tail_repair_preserves_the_original_and_resumes_publication_boundaries() {
    use std::io::Write as _;
    for boundary in [
        "candidate-copied",
        "active-quarantined",
        "candidate-published",
        "receipt-committed",
    ] {
        let project = tempfile::tempdir().unwrap();
        std::fs::write(project.path().join("README.md"), b"project-owned\n").unwrap();
        let preview = plan(project.path());
        let executable = test_executable();
        let now = crate::engine::clock::observe_required(
            &crate::engine::clock::HostSystemClock,
            "repair.test",
        )
        .unwrap()
        .observed_at_unix_ms;
        RrdEngine::apply_installation_at(
            project.path(),
            InstallationTargetKind::ExistingProject,
            &preview,
            &preview.installation.plan_sha256,
            now,
            &executable,
        )
        .unwrap();
        let locator = RrdEngine::read_project_locator(project.path()).unwrap();
        let active = project.path().join(&locator.storage_root);
        let wal = std::fs::read_dir(active.join("wal"))
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        let mut file = std::fs::OpenOptions::new().append(true).open(&wal).unwrap();
        file.write_all(b"incomplete-tail").unwrap();
        file.sync_all().unwrap();
        drop(file);
        let damaged = inventory(&active);
        let before_plan = inventory(project.path());
        let repair = RrdEngine::plan_installed_tail_repair(project.path(), &executable).unwrap();
        assert_eq!(
            inventory(project.path()),
            before_plan,
            "repair planning changed bytes"
        );
        assert!(RrdEngine::apply_installed_tail_repair(
            project.path(),
            &executable,
            &repair,
            &"0".repeat(64)
        )
        .is_err());
        assert_eq!(
            inventory(project.path()),
            before_plan,
            "unapproved repair changed bytes"
        );
        let mut reached = false;
        let result = crate::engine::tail_repair::publication::apply_with_observer(
            project.path(),
            &executable,
            &repair,
            &mut |point| {
                if point == boundary {
                    reached = true;
                    Err(ServiceError::Storage("owned repair interruption".into()))
                } else {
                    Ok(())
                }
            },
        );
        assert!(
            reached && result.is_err(),
            "repair boundary not reached: {boundary}"
        );
        let quarantine = project.path().join(&repair.publication.quarantine_root);
        let preserved = if quarantine.exists() {
            &quarantine
        } else {
            &active
        };
        assert_eq!(
            inventory(preserved),
            damaged,
            "repair mutated its damaged input at {boundary}"
        );
        let receipt = RrdEngine::apply_installed_tail_repair(
            project.path(),
            &executable,
            &repair,
            &repair.plan_sha256,
        )
        .unwrap();
        assert_eq!(receipt.runtime_cursor, 2);
        assert_eq!(receipt.storage.discarded_bytes, 15);
        assert_eq!(
            inventory(&quarantine),
            damaged,
            "quarantined evidence changed"
        );
        let report = RrdEngine::inspect_installed(project.path(), &executable).unwrap();
        assert_eq!(report.status, InstallationVerificationStatus::Passed);
        let completed = inventory(project.path());
        let replay = RrdEngine::apply_installed_tail_repair(
            project.path(),
            &executable,
            &repair,
            &repair.plan_sha256,
        )
        .unwrap();
        assert!(replay.idempotent_replay);
        assert_eq!(replay.plan_sha256, receipt.plan_sha256);
        assert_eq!(
            inventory(project.path()),
            completed,
            "completed retry wrote another repair"
        );
        assert_eq!(
            std::fs::read(project.path().join("README.md")).unwrap(),
            b"project-owned\n"
        );
    }
}

#[test]
fn unavailable_or_rolled_back_clock_blocks_effects_but_not_read_only_diagnosis() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("README.md"), "clock boundary\n").unwrap();
    let preview = plan(project.path());
    let executable = test_executable();
    let before = inventory(project.path());
    let unavailable = FixedClock::unavailable(ClockObservationFailure::BeforeUnixEpoch);
    let error = RrdEngine::apply_installation_with_test_clock(
        project.path(),
        InstallationTargetKind::ExistingProject,
        &preview,
        &preview.installation.plan_sha256,
        &executable,
        &unavailable,
    )
    .unwrap_err();
    assert!(matches!(error, ServiceError::ClockUnavailable(_)));
    assert_eq!(inventory(project.path()), before);

    let installed = RrdEngine::apply_installation_at(
        project.path(),
        InstallationTargetKind::ExistingProject,
        &preview,
        &preview.installation.plan_sha256,
        10_000,
        &executable,
    )
    .unwrap();
    assert_eq!(installed.applied_at_unix_ms, 10_000);
    let installed_bytes = inventory(project.path());

    let replay_error = RrdEngine::apply_installation_at(
        project.path(),
        InstallationTargetKind::ExistingProject,
        &preview,
        &preview.installation.plan_sha256,
        9_999,
        &executable,
    )
    .unwrap_err();
    assert!(matches!(
        replay_error,
        ServiceError::ClockRollback {
            observed_at_unix_ms: 9_999,
            anchor_unix_ms: 10_000,
            maximum_rollback_ms: 0
        }
    ));
    assert_eq!(inventory(project.path()), installed_bytes);

    let error = open_at(project.path(), &executable, 9_999)
        .err()
        .expect("rolled-back clock must reject installed open");
    assert!(matches!(
        error,
        ServiceError::ClockRollback {
            observed_at_unix_ms: 9_999,
            anchor_unix_ms: 10_000,
            maximum_rollback_ms: 0
        }
    ));
    let rolled_back = inspect_at(project.path(), &executable, 9_999).unwrap();
    assert_eq!(rolled_back.status, InstallationVerificationStatus::Failed);
    assert_eq!(
        rolled_back.clock.status,
        ClockAssessmentStatus::RollbackExceeded
    );
    assert_eq!(rolled_back.clock.rollback_ms, Some(1));
    assert!(
        !rolled_back
            .checks
            .iter()
            .find(|check| check.id.as_str() == "clock")
            .unwrap()
            .passed
    );
    assert_eq!(inventory(project.path()), installed_bytes);

    let unavailable = RrdEngine::inspect_installed_with_clock(
        project.path(),
        &executable,
        &FixedClock::unavailable(ClockObservationFailure::Unrepresentable),
    )
    .unwrap();
    assert_eq!(unavailable.status, InstallationVerificationStatus::Failed);
    assert_eq!(unavailable.clock.status, ClockAssessmentStatus::Unavailable);
    assert_eq!(
        unavailable.clock.failure,
        Some(ClockObservationFailure::Unrepresentable)
    );
    assert_eq!(inventory(project.path()), installed_bytes);
}

#[test]
fn changed_project_or_credential_permissions_fail_closed() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("README.md"), "before\n").unwrap();
    let preview = plan(project.path());
    std::fs::write(project.path().join("README.md"), "after\n").unwrap();
    assert!(RrdEngine::apply_installation_at(
        project.path(),
        InstallationTargetKind::ExistingProject,
        &preview,
        &preview.installation.plan_sha256,
        1_800_000_000_000,
        &test_executable(),
    )
    .is_err());
    assert!(!project.path().join(".rrflow").exists());

    let clean = tempfile::tempdir().unwrap();
    let preview = plan(clean.path());
    let executable = test_executable();
    RrdEngine::apply_installation_at(
        clean.path(),
        InstallationTargetKind::ExistingProject,
        &preview,
        &preview.installation.plan_sha256,
        1_800_000_000_000,
        &executable,
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let credential = clean.path().join(
            &preview
                .installation
                .managed_path(rrd_contract::InstallationManagedPathKind::OperatorCredential)
                .relative_path,
        );
        std::fs::set_permissions(&credential, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(RrdEngine::inspect_installed(clean.path(), &executable).is_err());
    }
}

#[test]
fn installed_secret_aliases_deny_open_verify_and_replay_without_effects() {
    let project = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let executable = test_executable();
    let preview = plan(project.path());
    let at = 1_800_000_000_000;
    let applied = RrdEngine::apply_installation_at(
        project.path(),
        InstallationTargetKind::ExistingProject,
        &preview,
        &preview.installation.plan_sha256,
        at,
        &executable,
    )
    .unwrap();
    let before = inventory(project.path());
    for kind in [
        rrd_contract::InstallationManagedPathKind::TokenKey,
        rrd_contract::InstallationManagedPathKind::OperatorCredential,
    ] {
        let secret = project
            .path()
            .join(&preview.installation.managed_path(kind).relative_path);
        let alias = outside.path().join("foreign-alias");
        std::fs::hard_link(&secret, &alias).unwrap();
        assert!(open_at(project.path(), &executable, at + 1).is_err());
        assert!(inspect_at(project.path(), &executable, at + 1).is_err());
        assert!(RrdEngine::apply_installation_at(
            project.path(),
            InstallationTargetKind::ExistingProject,
            &preview,
            &preview.installation.plan_sha256,
            at + 1,
            &executable,
        )
        .is_err());
        assert_eq!(
            inventory(project.path()),
            before,
            "denial changed installed state"
        );
        assert_eq!(
            std::fs::read(&alias).unwrap(),
            std::fs::read(&secret).unwrap()
        );
        std::fs::remove_file(&alias).unwrap();
        let report = inspect_at(project.path(), &executable, at + 1).unwrap();
        assert_eq!(report.status, InstallationVerificationStatus::Passed);
        assert_eq!(report.runtime_cursor, applied.runtime_cursor);
        assert_eq!(
            report.control_journal_sequence,
            applied.control_journal_sequence
        );
    }
}

#[cfg(unix)]
#[test]
fn installed_open_and_inspection_reject_symbolic_object_paths() {
    use std::os::unix::fs::symlink;

    let project = tempfile::tempdir().unwrap();
    let executable = test_executable();
    let preview = plan(project.path());
    RrdEngine::apply_installation_at(
        project.path(),
        InstallationTargetKind::ExistingProject,
        &preview,
        &preview.installation.plan_sha256,
        1_800_000_000_000,
        &executable,
    )
    .unwrap();
    let storage = project.path().join(
        &preview
            .installation
            .managed_path(rrd_contract::InstallationManagedPathKind::StorageRoot)
            .relative_path,
    );
    let staging = storage.join("immutable/staging");
    let redirected = storage.join("immutable/redirected-staging");
    std::fs::rename(&staging, &redirected).unwrap();
    symlink(&redirected, &staging).unwrap();

    let instance = preview.installation.target.instance_id.clone();
    assert!(RrdEngine::open_existing(&storage, instance, [0; 32]).is_err());
    assert!(RrdEngine::inspect_installed(project.path(), &executable).is_err());
}

#[test]
fn rehashed_profile_field_forgery_fails_before_any_project_effect() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("README.md"), "walking product\n").unwrap();
    let mut forged = plan(project.path());
    forged.installation.credential_bytes = 64;
    forged.installation.bootstrap.credential_entropy_bytes = 64;
    forged.installation.plan_sha256 = installation_plan_sha256(&forged.installation).unwrap();
    forged.validate().unwrap();

    assert!(RrdEngine::apply_installation_at(
        project.path(),
        InstallationTargetKind::ExistingProject,
        &forged,
        &forged.installation.plan_sha256,
        1_800_000_000_000,
        &test_executable(),
    )
    .is_err());
    assert!(!project.path().join(".rrflow").exists());
}

#[test]
fn fresh_project_apply_and_exact_replay_are_supported() {
    let project = tempfile::tempdir().unwrap();
    let executable = test_executable();
    let preview = RrdEngine::plan_installation(
        project.path(),
        InstallationTargetKind::FreshProject,
        &RrdEngine::generate_installation_identity().unwrap(),
        "default",
        None,
        &executable,
    )
    .unwrap();

    let first = RrdEngine::apply_installation_at(
        project.path(),
        InstallationTargetKind::FreshProject,
        &preview,
        &preview.installation.plan_sha256,
        1_800_000_000_000,
        &executable,
    )
    .unwrap();
    assert!(!first.idempotent_replay);

    let replay = RrdEngine::apply_installation_at(
        project.path(),
        InstallationTargetKind::FreshProject,
        &preview,
        &preview.installation.plan_sha256,
        1_800_000_000_001,
        &executable,
    )
    .unwrap();
    assert!(replay.idempotent_replay);
    assert_eq!(replay.plan_sha256, first.plan_sha256);
}

#[test]
fn explicit_configuration_is_sealed_and_apply_does_not_reread_it() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("README.md"), "configured product\n").unwrap();
    let input = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(
        input.path(),
        r#"format_version = 3

[storage]
recovery_reserve_bytes = 16777216
mutable_accounted_bytes = 134217728
mutable_versions = 1048576
projected_output_bytes = 134217728
native_transactions = 64
native_transaction_bytes = 67108864
active_read_views = 1024
pinned_read_bytes = 4294967296
page_cache_bytes = 4194304
wal_flush_target_bytes = 67108864
memtable_flush_target_versions = 524288
compaction_input_bytes = 1073741824
compaction_input_logical_bytes = 268435456
compaction_input_versions = 4194304
compaction_output_segments = 256
compaction_elapsed_ms = 30000
compaction_l0_segments = 64
retained_mutable_bytes = 536870912
retained_mutable_versions = 4194304
retained_mutable_generations = 64
reclamation_inventory_entries = 262144
reclamation_metadata_bytes = 67108864
reclamation_delete_files = 1024
reclamation_delete_bytes = 1073741824
reclamation_elapsed_ms = 30000
transaction_admission_requests = 64
transaction_admission_bytes = 67108864
transaction_admission_wait_ms = 30000
recovery_metadata_bytes = 67108864
recovery_files = 262144
recovery_read_bytes = 1099511627776
recovery_wal_bytes = 268435456
recovery_wal_frames = 1048576
recovery_elapsed_ms = 120000

[clock]
maximum_rollback_ms = 250

[reasoning]
max_run_elapsed_ms = 120000
max_steps = 32
max_step_elapsed_ms = 30000

[recall]
max_graph_depth = 2
max_items = 32
max_output_bytes = 131072
max_storage_keys = 5000

[query]
max_storage_keys = 5000
max_rows = 9
max_output_bytes = 131072
max_batch_rows = 8
max_memory_bytes = 16777216
max_spill_bytes = 33554432
max_elapsed_ms = 5000
"#,
    )
    .unwrap();
    let executable = test_executable();
    let preview = RrdEngine::plan_installation(
        project.path(),
        InstallationTargetKind::ExistingProject,
        &RrdEngine::generate_installation_identity().unwrap(),
        "default",
        Some(input.path()),
        &executable,
    )
    .unwrap();
    assert_eq!(preview.installation.configuration.query.max_rows, 9);
    assert_eq!(
        preview.installation.configuration.clock.maximum_rollback_ms,
        250
    );
    std::fs::write(input.path(), "this no longer parses as TOML = [").unwrap();

    RrdEngine::apply_installation_at(
        project.path(),
        InstallationTargetKind::ExistingProject,
        &preview,
        &preview.installation.plan_sha256,
        1_800_000_000_000,
        &executable,
    )
    .unwrap();
    let engine = open_at(project.path(), &executable, 1_799_999_999_750).unwrap();
    assert_eq!(engine.estate_configuration().query.max_rows, 9);
    drop(engine);
    let tolerated = inspect_at(project.path(), &executable, 1_799_999_999_750).unwrap();
    assert_eq!(
        tolerated.clock.status,
        ClockAssessmentStatus::RollbackWithinTolerance
    );
    assert_eq!(tolerated.clock.rollback_ms, Some(250));
    assert_eq!(tolerated.status, InstallationVerificationStatus::Passed);
    let mut locator = RrdEngine::read_project_locator(project.path()).unwrap();
    assert_eq!(locator.configuration_revision, 1);
    assert_eq!(
        locator.configuration_sha256,
        preview.installation.configuration.configuration_sha256
    );
    assert!(!toml::to_string(&locator).unwrap().contains("project_root"));

    locator.configuration_revision = 2;
    std::fs::write(
        project.path().join(".rrflow/config.toml"),
        toml::to_string(&locator).unwrap(),
    )
    .unwrap();
    assert!(open_at(project.path(), &executable, 1_800_000_000_001).is_err());
    assert!(inspect_at(project.path(), &executable, 1_800_000_000_001).is_err());
}

#[test]
fn installed_locator_relocates_but_never_walks_ancestors_or_pairs_foreign_state() {
    let container = tempfile::tempdir().unwrap();
    let original = container.path().join("original");
    let moved = container.path().join("moved");
    let foreign = container.path().join("foreign");
    std::fs::create_dir(&original).unwrap();
    std::fs::write(original.join("README.md"), "relocatable project\n").unwrap();
    let executable = test_executable();
    let preview = RrdEngine::plan_installation(
        &original,
        InstallationTargetKind::ExistingProject,
        &RrdEngine::generate_installation_identity().unwrap(),
        "default",
        None,
        &executable,
    )
    .unwrap();
    RrdEngine::apply_installation_at(
        &original,
        InstallationTargetKind::ExistingProject,
        &preview,
        &preview.installation.plan_sha256,
        1_800_000_000_000,
        &executable,
    )
    .unwrap();

    std::fs::rename(&original, &moved).unwrap();
    let relocated = open_at(&moved, &executable, 1_800_000_000_001).unwrap();
    assert_eq!(
        relocated.installed_estate_identity(),
        Some(&preview.installation.target)
    );
    drop(relocated);

    let replay = RrdEngine::apply_installation_at(
        &moved,
        InstallationTargetKind::ExistingProject,
        &preview,
        &preview.installation.plan_sha256,
        1_800_000_000_001,
        &executable,
    )
    .unwrap();
    assert!(replay.idempotent_replay);

    let nested = moved.join("nested");
    std::fs::create_dir(&nested).unwrap();
    assert!(open_at(&nested, &executable, 1_800_000_000_001).is_err());

    std::fs::create_dir(&foreign).unwrap();
    std::fs::write(foreign.join("README.md"), "different project\n").unwrap();
    let foreign_preview = RrdEngine::plan_installation(
        &foreign,
        InstallationTargetKind::ExistingProject,
        &RrdEngine::generate_installation_identity().unwrap(),
        "default",
        None,
        &executable,
    )
    .unwrap();
    assert_ne!(
        foreign_preview.installation.target,
        preview.installation.target
    );
    RrdEngine::apply_installation_at(
        &foreign,
        InstallationTargetKind::ExistingProject,
        &foreign_preview,
        &foreign_preview.installation.plan_sha256,
        1_800_000_000_001,
        &executable,
    )
    .unwrap();
    std::fs::copy(
        foreign.join(".rrflow/config.toml"),
        moved.join(".rrflow/config.toml"),
    )
    .unwrap();
    assert!(open_at(&moved, &executable, 1_800_000_000_001).is_err());
}

#[test]
fn pre_canonical_rrflow_state_blocks_a_parallel_installation_authority() {
    let project = tempfile::tempdir().unwrap();
    std::fs::create_dir(project.path().join(".rrflow")).unwrap();
    std::fs::write(project.path().join(".rrflow/instance.toml"), "format = 1\n").unwrap();
    let before = inventory(project.path());
    assert!(RrdEngine::plan_installation(
        project.path(),
        InstallationTargetKind::ExistingProject,
        &RrdEngine::generate_installation_identity().unwrap(),
        "default",
        None,
        &test_executable(),
    )
    .is_err());
    assert_eq!(inventory(project.path()), before);
}

#[test]
fn installed_backups_and_restore_staging_live_beside_the_storage_roots() {
    use crate::engine::estate_layout::{validate_recovery_estate, EstateLayout};
    use rrd_contract::{
        CreateInstanceBackup, CreateSession, ListInstanceBackups, RestoreInstanceBackup,
        SessionLimits,
    };

    let project = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("README.md"), "walking product\n").unwrap();
    let preview = plan(project.path());
    let executable = test_executable();
    RrdEngine::apply_installation(
        project.path(),
        InstallationTargetKind::ExistingProject,
        &preview,
        &preview.installation.plan_sha256,
        &executable,
    )
    .unwrap();
    let instance = preview.installation.target.instance_id.clone();
    let storage_root_id = preview.installation.storage_root_id.clone();

    let engine = RrdEngine::open_installed(project.path(), &executable).unwrap();
    let api_key = RrdEngine::read_installed_api_key(project.path()).unwrap();
    let lease = engine
        .create_authenticated_session(
            &api_key.principal_id,
            api_key.credential().as_bytes(),
            &CreateSession {
                limits: SessionLimits {
                    idle_timeout_ms: 60_000,
                    absolute_timeout_ms: 60_000,
                    max_open_transactions: 1,
                },
            },
            &id("installed-backup-session"),
            1_800_000_000_100,
            "request-installed-session",
            "operation-installed-session",
        )
        .unwrap();

    let backup = engine
        .create_instance_backup(
            &lease.session_id,
            &lease.token,
            &id("installed-backup-key"),
            &CreateInstanceBackup {
                label: "installed".into(),
                created_at_unix_ms: 1_800_000_000_200,
            },
            1_800_000_000_200,
            "request-installed-backup",
            "operation-installed-backup",
        )
        .unwrap();
    assert!(backup.backup.application_complete);

    let catalogue = engine
        .list_instance_backups(
            &lease.session_id,
            &lease.token,
            &ListInstanceBackups {
                verify_archives: true,
            },
            1_800_000_000_300,
            "request-installed-backup-list",
            "operation-installed-backup-list",
        )
        .unwrap();
    assert!(catalogue.archives_verified);
    assert_eq!(catalogue.backups.len(), 1);
    assert_eq!(
        catalogue.backups[0].backup_sha256,
        backup.backup.backup_sha256
    );

    let restore_id = CanonicalId::new("restore-one").unwrap();
    let restored = engine
        .restore_instance_backup(
            &lease.session_id,
            &lease.token,
            &id("installed-restore-key"),
            &RestoreInstanceBackup {
                backup_sha256: backup.backup.backup_sha256.clone(),
                restore_id: restore_id.clone(),
                restored_at_unix_ms: 1_800_000_000_400,
            },
            1_800_000_000_400,
            "request-installed-restore",
            "operation-installed-restore",
        )
        .unwrap();
    assert!(restored.reopened);
    drop(engine);

    let container = project.path().join(".rrflow/rrd");
    assert!(container
        .join("backups")
        .join(instance.as_str())
        .join("catalogue.json")
        .is_file());
    assert!(container
        .join("restores")
        .join(instance.as_str())
        .join(restore_id.as_str())
        .is_dir());

    let mut roots = std::fs::read_dir(container.join("roots"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect::<Vec<_>>();
    roots.sort();
    assert_eq!(
        roots,
        [storage_root_id.to_string()],
        "roots/ holds only the storage root the locator selects"
    );
    assert!(
        !inventory(project.path()).keys().any(|path| path
            .components()
            .any(|component| component.as_os_str() == "rrd-service")),
        "an installed estate has no rrd-service tree"
    );

    let layout =
        EstateLayout::new(&storage_root_id, &preview.installation.initial_principal_id).unwrap();
    validate_recovery_estate(
        &super::super::estate_layout::EstateFilesystem::admit(
            rrd_store::filesystem::AnchoredPath::open_root(project.path()).unwrap(),
            false,
        )
        .unwrap(),
        &layout,
    )
    .expect("install recovery accepts an estate that holds backups and restore staging");

    let reopened = RrdEngine::open_installed(project.path(), &executable).unwrap();
    assert_eq!(
        reopened
            .readiness(1_800_000_000_500)
            .unwrap()
            .runtime_cursor,
        2
    );
}

struct InstalledEstate {
    _project: tempfile::TempDir,
    project_path: PathBuf,
    executable: PathBuf,
    instance: CanonicalId,
    active_root: PathBuf,
}

/// Installs a project, opens it as the product does, and authenticates with
/// the installed credential. The caller owns the session.
fn install_and_open() -> (InstalledEstate, RrdEngine, SessionLease) {
    use rrd_contract::{CreateSession, InstallationManagedPathKind, SessionLimits};

    let project = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("README.md"), "walking product\n").unwrap();
    let preview = plan(project.path());
    let executable = test_executable();
    RrdEngine::apply_installation(
        project.path(),
        InstallationTargetKind::ExistingProject,
        &preview,
        &preview.installation.plan_sha256,
        &executable,
    )
    .unwrap();
    let engine = RrdEngine::open_installed(project.path(), &executable).unwrap();
    let api_key = RrdEngine::read_installed_api_key(project.path()).unwrap();
    let lease = engine
        .create_authenticated_session(
            &api_key.principal_id,
            api_key.credential().as_bytes(),
            &CreateSession {
                limits: SessionLimits {
                    idle_timeout_ms: 60_000,
                    absolute_timeout_ms: 60_000,
                    max_open_transactions: 1,
                },
            },
            &id("installed-session"),
            1_800_000_000_100,
            "request-installed-session",
            "operation-installed-session",
        )
        .unwrap();
    let estate = InstalledEstate {
        project_path: project.path().to_path_buf(),
        active_root: project.path().join(
            &preview
                .installation
                .managed_path(InstallationManagedPathKind::StorageRoot)
                .relative_path,
        ),
        _project: project,
        executable,
        instance: preview.installation.target.instance_id.clone(),
    };
    (estate, engine, lease)
}

fn restore(
    engine: &RrdEngine,
    lease: &SessionLease,
    key: &str,
    backup_sha256: &str,
    restore_id: &str,
    now: u64,
) -> std::result::Result<rrd_contract::RestoreInstanceBackupResult, ServiceError> {
    use rrd_contract::RestoreInstanceBackup;

    engine.restore_instance_backup(
        &lease.session_id,
        &lease.token,
        &id(key),
        &RestoreInstanceBackup {
            backup_sha256: backup_sha256.into(),
            restore_id: CanonicalId::new(restore_id).unwrap(),
            restored_at_unix_ms: now,
        },
        now,
        &format!("request-{key}"),
        &format!("operation-{key}"),
    )
}

#[test]
fn a_restore_into_an_existing_verified_target_is_recovered_without_a_second_writer() {
    use rrd_contract::CreateInstanceBackup;

    let (estate, engine, lease) = install_and_open();
    let backup = engine
        .create_instance_backup(
            &lease.session_id,
            &lease.token,
            &id("backup-key"),
            &CreateInstanceBackup {
                label: "installed".into(),
                created_at_unix_ms: 1_800_000_000_200,
            },
            1_800_000_000_200,
            "request-backup",
            "operation-backup",
        )
        .unwrap();

    let fresh = restore(
        &engine,
        &lease,
        "restore-key-one",
        &backup.backup.backup_sha256,
        "restore-one",
        1_800_000_000_300,
    )
    .unwrap();
    assert!(!fresh.idempotent_replay);
    let target = estate
        .project_path
        .join(".rrflow/rrd/restores")
        .join(estate.instance.as_str())
        .join("restore-one");
    assert!(target.is_dir());

    // A different idempotency key for the same restore id finds the target
    // already staged. That is the crash-recovery path: the earlier operation
    // staged the root but its result was never recorded.
    let recovered = restore(
        &engine,
        &lease,
        "restore-key-two",
        &backup.backup.backup_sha256,
        "restore-one",
        1_800_000_000_400,
    )
    .expect("an existing verified target is recovered, not rebuilt");
    assert!(recovered.idempotent_replay);
    assert_eq!(recovered.inventory, fresh.inventory);
    assert_eq!(recovered.backup_sha256, fresh.backup_sha256);

    // An existing target that is not a restored store fails closed instead of
    // being created as a new database.
    let empty = estate
        .project_path
        .join(".rrflow/rrd/restores")
        .join(estate.instance.as_str())
        .join("restore-empty");
    std::fs::create_dir_all(&empty).unwrap();
    let refused = restore(
        &engine,
        &lease,
        "restore-key-three",
        &backup.backup.backup_sha256,
        "restore-empty",
        1_800_000_000_500,
    );
    assert!(
        refused.is_err(),
        "an empty target directory is not a restored store"
    );
    assert!(
        std::fs::read_dir(&empty).unwrap().next().is_none(),
        "a refused restore creates nothing inside the target"
    );
}

/// Collects a root's complete control journal in order.
fn control_journal(store: &rrd_store::RrflowKvStore) -> Vec<rrd_store::ControlJournalEntry> {
    let mut entries = Vec::new();
    let mut after = 0u64;
    loop {
        let page = store.control().journal_since(after, 256).unwrap();
        match page.last() {
            Some(last) => after = last.sequence,
            None => return entries,
        }
        entries.extend(page);
    }
}

/// POAM-031 closure, first half: a backup of an installed estate carries the
/// installed control state as replayable control-journal entries, so a
/// restored root holds the installed record, the install checkpoint, the
/// active configuration, the security policy, and an audit chain that reads
/// back from the restored root. Secrets stay files outside the store, and the
/// transient families (session leases, operation receipts) are not carried.
/// The second half, opening the restored root through `open_installed`, is
/// restore publication's evidence.
#[test]
fn a_restored_root_carries_the_installed_control_state() {
    use rrd_contract::CreateInstanceBackup;

    let (estate, engine, lease) = install_and_open();
    let backup = engine
        .create_instance_backup(
            &lease.session_id,
            &lease.token,
            &id("backup-key"),
            &CreateInstanceBackup {
                label: "installed".into(),
                created_at_unix_ms: 1_800_000_000_200,
            },
            1_800_000_000_200,
            "request-backup",
            "operation-backup",
        )
        .unwrap();
    assert!(backup.backup.application_complete);
    let families = super::super::backup::installed_control_families(&estate.instance);
    rrd_store::validate_control_families(&families).unwrap();
    let catalogue = rrd_store::load_backup_catalogue(
        &estate
            .project_path
            .join(".rrflow/rrd/backups")
            .join(estate.instance.as_str()),
    )
    .unwrap();
    assert_eq!(
        catalogue.format_version,
        rrd_store::BACKUP_CATALOGUE_VERSION
    );
    let entry = catalogue
        .backups
        .iter()
        .find(|entry| entry.backup_id == backup.backup.backup_sha256)
        .unwrap();
    assert_eq!(entry.installed_control, rrd_store::BackupCoverage::Included);
    let archive = entry.control_archive.as_ref().unwrap();
    assert_eq!(archive.families, families);
    assert!(
        archive.entry_count >= 8,
        "install writes at least eight control entries"
    );

    restore(
        &engine,
        &lease,
        "restore-key",
        &backup.backup.backup_sha256,
        "restore-one",
        1_800_000_000_300,
    )
    .unwrap();
    drop(engine);

    let active = rrd_store::RrflowKvStore::open_existing(&estate.active_root).unwrap();
    let restored = rrd_store::RrflowKvStore::open_existing(
        &estate
            .project_path
            .join(".rrflow/rrd/restores")
            .join(estate.instance.as_str())
            .join("restore-one"),
    )
    .unwrap();

    // Installed state that the estate has not changed since install is
    // byte-identical in the restored root.
    let configuration_key = format!("server/state/{}/configuration/active", estate.instance);
    let security_key = format!("server/state/{}/security/policy", estate.instance);
    for key in [
        "server/state/install/record",
        "server/state/install/checkpoint",
        configuration_key.as_str(),
        security_key.as_str(),
    ] {
        let value = active.control().get(key).unwrap();
        assert!(value.is_some(), "the active root holds {key}");
        assert_eq!(
            restored.control().get(key).unwrap(),
            value,
            "the restored root carries {key}"
        );
    }
    assert_eq!(
        restored.runtime().cursor().unwrap(),
        active.runtime().cursor().unwrap(),
        "the restored root carries every runtime commit"
    );

    // Every in-scope control-journal entry up to the backup's watermark is
    // replayed in order with the same transition; nothing outside the scope
    // is carried, and nothing was invented.
    let source_entries = control_journal(&active)
        .into_iter()
        .filter(|entry| entry.sequence <= archive.source_control_sequence)
        .filter(|entry| rrd_store::control_family_of(&families, &entry.key).is_some())
        .collect::<Vec<_>>();
    assert_eq!(source_entries.len() as u64, archive.entry_count);
    let restored_entries = control_journal(&restored);
    let replayed = restored_entries
        .iter()
        .filter(|entry| rrd_store::control_family_of(&families, &entry.key).is_some())
        .collect::<Vec<_>>();
    assert_eq!(replayed.len(), source_entries.len());
    for (restored_entry, source_entry) in replayed.iter().zip(&source_entries) {
        assert_eq!(restored_entry.key, source_entry.key);
        assert_eq!(restored_entry.before_sha256, source_entry.before_sha256);
        assert_eq!(restored_entry.replacement, source_entry.replacement);
        assert_eq!(restored_entry.at, source_entry.at);
        assert_eq!(restored_entry.actor, source_entry.actor);
        assert_eq!(restored_entry.action, source_entry.action);
        assert_eq!(restored_entry.request_id, source_entry.request_id);
        assert_eq!(restored_entry.operation_id, source_entry.operation_id);
    }
    let excluded = [
        format!("server/state/{}/session/", estate.instance),
        format!("server/state/{}/backup/", estate.instance),
        format!("server/state/{}/restore/", estate.instance),
    ];
    assert!(
        control_journal(&active)
            .iter()
            .any(|entry| entry.key.starts_with(&excluded[0])),
        "the active root holds the session lease"
    );
    assert!(
        restored_entries
            .iter()
            .all(|entry| excluded.iter().all(|prefix| !entry.key.starts_with(prefix))),
        "session leases and operation receipts are not carried"
    );
    let foreign = restored_entries
        .iter()
        .filter(|entry| rrd_store::control_family_of(&families, &entry.key).is_none())
        .map(|entry| entry.key.as_str())
        .collect::<Vec<_>>();
    assert!(
        foreign.iter().all(
            |key| key.starts_with("server/state/vector-collection-catalogue/")
                || key.starts_with("server/state/index-catalogue/")
        ),
        "only catalogue restores sit beside the replayed control state: {foreign:?}"
    );

    // The audit chain reads back from the restored root and closes on its
    // durable head.
    let page = SecurityRepository::new(&restored, estate.instance.clone())
        .audit_since(0, 64)
        .unwrap();
    let head: serde_json::Value = serde_json::from_slice(
        &restored
            .control()
            .get(&format!("server/state/{}/audit-head", estate.instance))
            .unwrap()
            .expect("the restored root holds the audit head"),
    )
    .unwrap();
    assert_eq!(
        head["record_count"].as_u64().unwrap(),
        page.records.len() as u64
    );
    assert!(
        page.records.len() >= 2,
        "install genesis plus the session's audits"
    );
    assert_eq!(
        page.chain_head_sha256.as_deref(),
        head["audit_sha256"].as_str()
    );
    assert!(
        rrd_security::SecurityRepository::new(&restored, estate.instance.clone())
            .is_initialized()
            .unwrap(),
        "the restored root has a security authority"
    );
    let progress = rrd_store::control_restore_progress(&restored)
        .unwrap()
        .unwrap();
    assert!(progress.complete);
    assert_eq!(progress.applied, archive.entry_count);
    assert!(
        !estate
            .project_path
            .join(".rrflow/rrd/restores")
            .join(estate.instance.as_str())
            .join("restore-one")
            .join("RRD.TOKEN")
            .exists(),
        "the token key is never part of a backup"
    );
    drop(restored);
    drop(active);
    assert!(
        RrdEngine::open_installed(&estate.project_path, &estate.executable).is_ok(),
        "the active estate still opens"
    );
}

/// Installs, backs up, stages a restore of that backup, then performs more
/// audited work on the active estate so the restored state is visibly older
/// than the active one. Returns the backup identity and the restore id.
fn install_backup_and_stage() -> (InstalledEstate, String, CanonicalId) {
    use rrd_contract::CreateInstanceBackup;

    let (estate, engine, lease) = install_and_open();
    let backup = engine
        .create_instance_backup(
            &lease.session_id,
            &lease.token,
            &id("backup-key"),
            &CreateInstanceBackup {
                label: "publish".into(),
                created_at_unix_ms: 1_800_000_000_200,
            },
            1_800_000_000_200,
            "request-backup",
            "operation-backup",
        )
        .unwrap();
    let restore_id = CanonicalId::new("restore-publish").unwrap();
    restore(
        &engine,
        &lease,
        "restore-key",
        &backup.backup.backup_sha256,
        restore_id.as_str(),
        1_800_000_000_300,
    )
    .unwrap();
    // Later work on the active estate: another backup, which appends audit
    // records and operation receipts the restored state must not carry.
    engine
        .create_instance_backup(
            &lease.session_id,
            &lease.token,
            &id("backup-key-later"),
            &CreateInstanceBackup {
                label: "later".into(),
                created_at_unix_ms: 1_800_000_000_400,
            },
            1_800_000_000_400,
            "request-backup-later",
            "operation-backup-later",
        )
        .unwrap();
    drop(engine);
    (estate, backup.backup.backup_sha256, restore_id)
}

fn authenticated_readiness(estate: &InstalledEstate, now: u64) -> rrd_contract::Readiness {
    use rrd_contract::{CreateSession, SessionLimits};

    let engine = RrdEngine::open_installed(&estate.project_path, &estate.executable).unwrap();
    let api_key = RrdEngine::read_installed_api_key(&estate.project_path).unwrap();
    engine
        .create_authenticated_session(
            &api_key.principal_id,
            api_key.credential().as_bytes(),
            &CreateSession {
                limits: SessionLimits {
                    idle_timeout_ms: 60_000,
                    absolute_timeout_ms: 60_000,
                    max_open_transactions: 1,
                },
            },
            &id(&format!("published-session-{now}")),
            now,
            "request-published-session",
            "operation-published-session",
        )
        .expect("the published estate authenticates its operator credential");
    engine.readiness(now).unwrap()
}

/// POAM-031 closure, second half (Decision 6 A): a verified restore staging
/// root is published into the storage root's own name, the previous root is
/// quarantined beside the staging roots, the token key travels with the
/// estate rather than with the backup, and the published estate opens through
/// `open_installed` and reaches authenticated readiness at the backup's state.
#[test]
fn a_staged_restore_is_published_in_place_and_the_estate_opens_from_the_backup() {
    use crate::engine::estate_layout::{validate_recovery_estate, EstateLayout};

    let (estate, backup_sha256, restore_id) = install_backup_and_stage();
    let locator = RrdEngine::read_project_locator(&estate.project_path).unwrap();
    let candidate = estate
        .project_path
        .join(".rrflow/rrd/restores")
        .join(estate.instance.as_str())
        .join(restore_id.as_str());
    let active_before = rrd_store::RrflowKvInspector::open(&estate.active_root).unwrap();
    let active_control_sequence = active_before.inspection().control_journal_sequence;
    drop(active_before);

    // Planning fails while the estate is served and writes nothing.
    let served = RrdEngine::open_installed(&estate.project_path, &estate.executable).unwrap();
    assert!(
        RrdEngine::plan_restore_publication(
            &estate.project_path,
            &restore_id,
            &backup_sha256,
            &estate.executable,
        )
        .is_err(),
        "a served estate cannot be published over"
    );
    drop(served);
    let before = inventory(&estate.project_path);

    let plan = RrdEngine::plan_restore_publication(
        &estate.project_path,
        &restore_id,
        &backup_sha256,
        &estate.executable,
    )
    .unwrap();
    assert_eq!(
        inventory(&estate.project_path),
        before,
        "planning has no effect"
    );
    plan.validate().unwrap();
    assert_eq!(plan.target, locator.identity);
    assert_eq!(plan.active_root, locator.storage_root);
    assert_eq!(
        plan.candidate_root,
        format!(".rrflow/rrd/restores/{}/{}", estate.instance, restore_id)
    );
    assert_eq!(
        plan.quarantine_root,
        format!(
            ".rrflow/rrd/restores/{}/{}.quarantined-{}",
            estate.instance, restore_id, plan.storage_root_id
        )
    );
    assert_eq!(
        plan.installed_record_sha256,
        locator.installed_record_sha256
    );
    assert!(
        plan.active.control_journal_sequence > plan.candidate.control_journal_sequence,
        "the active estate has moved past the backup"
    );
    assert_eq!(plan.active.runtime_cursor, plan.candidate.runtime_cursor);
    let replanned = RrdEngine::plan_restore_publication(
        &estate.project_path,
        &restore_id,
        &backup_sha256,
        &estate.executable,
    )
    .unwrap();
    assert_eq!(replanned, plan, "planning is deterministic");
    assert!(
        RrdEngine::apply_restore_publication(
            &estate.project_path,
            &plan,
            &"0".repeat(64),
            1_800_000_000_500,
            &estate.executable,
        )
        .is_err(),
        "an unaccepted digest applies nothing"
    );
    assert_eq!(inventory(&estate.project_path), before);

    let result = RrdEngine::apply_restore_publication(
        &estate.project_path,
        &plan,
        &plan.plan_sha256,
        1_800_000_000_500,
        &estate.executable,
    )
    .unwrap();
    result.validate_for(&plan).unwrap();
    assert!(!result.idempotent_replay);
    assert_eq!(result.published_root, plan.active_root);
    assert_eq!(result.quarantine_root, plan.quarantine_root);
    assert_eq!(result.runtime_cursor, plan.candidate.runtime_cursor);
    assert!(!candidate.exists(), "the candidate moved into roots/");
    let quarantine = estate.project_path.join(&plan.quarantine_root);
    assert!(quarantine.is_dir(), "the previous root is quarantined");
    let quarantined = rrd_store::RrflowKvInspector::open(&quarantine).unwrap();
    assert_eq!(
        quarantined.inspection().control_journal_sequence,
        active_control_sequence,
        "the quarantined root is the previous active root, untouched"
    );
    drop(quarantined);
    let mut roots = std::fs::read_dir(estate.project_path.join(".rrflow/rrd/roots"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect::<Vec<_>>();
    roots.sort();
    assert_eq!(roots, [plan.storage_root_id.to_string()]);
    assert!(
        !estate
            .project_path
            .join(".rrflow/rrd/restores")
            .join(estate.instance.as_str())
            .join(format!(".publish-{restore_id}.json"))
            .exists(),
        "the intent is acknowledged"
    );
    let token = std::fs::read(estate.active_root.join("RRD.TOKEN")).unwrap();
    assert_eq!(digest::sha256_hex(&token), plan.token_key_sha256);
    assert_eq!(
        std::fs::read(quarantine.join("RRD.TOKEN")).unwrap(),
        token,
        "the token key travels with the estate"
    );
    let layout = EstateLayout::new(
        &plan.storage_root_id,
        &RrdEngine::read_installed_api_key(&estate.project_path)
            .unwrap()
            .principal_id,
    )
    .unwrap();
    validate_recovery_estate(
        &super::super::estate_layout::EstateFilesystem::admit(
            rrd_store::filesystem::AnchoredPath::open_root(&estate.project_path).unwrap(),
            false,
        )
        .unwrap(),
        &layout,
    )
    .expect("install recovery accepts the published estate");

    // POAM-031 closure: the published root is the installed estate.
    let readiness = authenticated_readiness(&estate, 1_800_000_000_600);
    assert_eq!(readiness.runtime_cursor, plan.candidate.runtime_cursor);
    let report = RrdEngine::inspect_installed(&estate.project_path, &estate.executable).unwrap();
    assert_eq!(report.status, InstallationVerificationStatus::Passed);
    let published = rrd_store::RrflowKvStore::open_existing(&estate.active_root).unwrap();
    assert!(
        published
            .control()
            .get(&format!(
                "server/state/{}/restore/publication/{}",
                estate.instance, plan.id
            ))
            .unwrap()
            .is_some(),
        "the published root carries the publication receipt"
    );
    assert!(published
        .control()
        .get("server/state/install/record")
        .unwrap()
        .is_some());
    drop(published);

    // Replay returns the recorded receipt; a new plan for the consumed
    // candidate is refused.
    let replay = RrdEngine::apply_restore_publication(
        &estate.project_path,
        &plan,
        &plan.plan_sha256,
        1_800_000_000_700,
        &estate.executable,
    )
    .unwrap();
    assert!(replay.idempotent_replay);
    assert_eq!(replay.plan_sha256, result.plan_sha256);
    assert_eq!(replay.action_results, result.action_results);
    assert_eq!(replay.applied_at_unix_ms, result.applied_at_unix_ms);
    assert!(RrdEngine::plan_restore_publication(
        &estate.project_path,
        &restore_id,
        &backup_sha256,
        &estate.executable,
    )
    .is_err());
}

#[test]
fn an_interrupted_publication_resumes_from_its_recorded_stage() {
    use super::super::restore_publication::PublicationStage;

    for fail_after in [
        PublicationStage::CandidateVerified,
        PublicationStage::TokenCarried,
        PublicationStage::ActiveQuarantined,
        PublicationStage::CandidatePublished,
    ] {
        let (estate, backup_sha256, restore_id) = install_backup_and_stage();
        let plan = RrdEngine::plan_restore_publication(
            &estate.project_path,
            &restore_id,
            &backup_sha256,
            &estate.executable,
        )
        .unwrap();
        let interrupted = RrdEngine::apply_restore_publication_with_failure(
            &estate.project_path,
            &plan,
            &plan.plan_sha256,
            1_800_000_000_500,
            &estate.executable,
            fail_after,
        );
        assert!(interrupted.is_err(), "failpoint after {fail_after:?} fires");
        let intent = estate
            .project_path
            .join(".rrflow/rrd/restores")
            .join(estate.instance.as_str())
            .join(format!(".publish-{restore_id}.json"));
        assert!(intent.is_file(), "the intent survives the interruption");
        assert!(
            RrdEngine::plan_restore_publication(
                &estate.project_path,
                &restore_id,
                &backup_sha256,
                &estate.executable,
            )
            .is_err(),
            "an in-progress publication refuses a second plan"
        );

        let result = RrdEngine::apply_restore_publication(
            &estate.project_path,
            &plan,
            &plan.plan_sha256,
            1_800_000_000_600,
            &estate.executable,
        )
        .unwrap_or_else(|error| panic!("resume after {fail_after:?} failed: {error}"));
        result.validate_for(&plan).unwrap();
        assert!(!result.idempotent_replay);
        assert!(!intent.exists());
        assert!(estate.project_path.join(&plan.quarantine_root).is_dir());
        assert!(!estate.project_path.join(&plan.candidate_root).exists());
        let readiness = authenticated_readiness(&estate, 1_800_000_000_700);
        assert_eq!(readiness.runtime_cursor, plan.candidate.runtime_cursor);
    }
}
