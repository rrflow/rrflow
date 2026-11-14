//! Exact apply, open and verify, tail repair, clocks, permissions and secret aliases.

use super::*;

#[test]
pub(super) fn exact_apply_reopens_and_quick_verification_changes_no_bytes() {
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
pub(super) fn installed_open_and_verify_never_repair_a_torn_or_missing_active_wal() {
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
pub(super) fn installed_tail_repair_preserves_the_original_and_resumes_publication_boundaries() {
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
pub(super) fn unavailable_or_rolled_back_clock_blocks_effects_but_not_read_only_diagnosis() {
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
pub(super) fn changed_project_or_credential_permissions_fail_closed() {
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
pub(super) fn installed_secret_aliases_deny_open_verify_and_replay_without_effects() {
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
pub(super) fn installed_open_and_inspection_reject_symbolic_object_paths() {
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
pub(super) fn rehashed_profile_field_forgery_fails_before_any_project_effect() {
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
pub(super) fn fresh_project_apply_and_exact_replay_are_supported() {
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
