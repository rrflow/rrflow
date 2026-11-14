//! Sealed configuration, locator relocation, leftover folders and inherited estates.

use super::*;

#[test]
pub(super) fn explicit_configuration_is_sealed_and_apply_does_not_reread_it() {
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
source_page_bytes = 268435456
native_transactions = 64
native_transaction_bytes = 67108864
checkpoint_entries = 1024
active_read_views = 1024
pinned_read_bytes = 4294967296
page_cache_bytes = 67108864
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
pub(super) fn installed_locator_relocates_but_never_walks_ancestors_or_pairs_foreign_state() {
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

/// A leftover `.rrflow` folder from earlier tools or runs, created with the
/// platform's ordinary inherited permissions and holding no locator.
pub(super) fn leftover_rrflow(project: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    std::fs::create_dir_all(project.join(".rrflow/dev/old")).unwrap();
    std::fs::create_dir_all(project.join(".rrflow/rrd")).unwrap();
    std::fs::write(project.join(".rrflow/instance.toml"), "format = 1\n").unwrap();
    std::fs::write(project.join(".rrflow/dev/old/CURRENT"), "manifest-7\n").unwrap();
    std::fs::write(project.join(".rrflow/rrd/stale.bin"), [7u8; 64]).unwrap();
    std::fs::write(project.join("README.md"), "# project\n").unwrap();
    inventory(&project.join(".rrflow"))
}

#[test]
pub(super) fn a_leftover_rrflow_folder_is_adopted_in_place_and_keeps_every_file() {
    let project = tempfile::tempdir().unwrap();
    let executable = test_executable();
    let leftover = leftover_rrflow(project.path());
    let before = inventory(project.path());

    // Planning reports the adoption and changes nothing.
    let preview = plan(project.path());
    assert_eq!(
        preview.installation.adopted_estate_entries.as_deref(),
        Some(
            &[
                "dev".to_owned(),
                "instance.toml".to_owned(),
                "rrd".to_owned()
            ][..]
        )
    );
    assert_eq!(inventory(project.path()), before);

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

    // Every leftover byte is retained below `.rrflow/previous/<plan id>/`.
    let retained = project
        .path()
        .join(".rrflow/previous")
        .join(preview.installation.id.as_str());
    assert_eq!(inventory(&retained), leftover);
    assert!(!project.path().join(".rrflow/instance.toml").exists());
    assert!(!project.path().join(".rrflow/dev").exists());
    assert!(!project.path().join(".rrflow/rrd/stale.bin").exists());
    crate::engine::token_key::validate_private_directory(&project.path().join(".rrflow")).unwrap();

    // The adopted estate opens, verifies and replays like any other.
    open_at(project.path(), &executable, 1_800_000_000_001).unwrap();
    let report = inspect_at(project.path(), &executable, 1_800_000_000_002).unwrap();
    assert_eq!(report.status, InstallationVerificationStatus::Passed);
    let replay = RrdEngine::apply_installation_at(
        project.path(),
        InstallationTargetKind::ExistingProject,
        &preview,
        &preview.installation.plan_sha256,
        1_800_000_000_003,
        &executable,
    )
    .unwrap();
    assert!(replay.idempotent_replay);
}

#[test]
pub(super) fn a_changed_leftover_rrflow_folder_refuses_the_stale_plan_without_effects() {
    let project = tempfile::tempdir().unwrap();
    leftover_rrflow(project.path());
    let preview = plan(project.path());
    std::fs::write(
        project.path().join(".rrflow/late.txt"),
        "added after review\n",
    )
    .unwrap();
    let before = inventory(project.path());
    assert!(RrdEngine::apply_installation_at(
        project.path(),
        InstallationTargetKind::ExistingProject,
        &preview,
        &preview.installation.plan_sha256,
        1_800_000_000_000,
        &test_executable(),
    )
    .is_err());
    assert_eq!(inventory(project.path()), before);
    assert!(!project.path().join(".rrflow/previous").exists());
}

#[test]
pub(super) fn an_installed_estate_is_never_adopted() {
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
    let before = inventory(project.path());
    assert!(RrdEngine::plan_installation(
        project.path(),
        InstallationTargetKind::ExistingProject,
        &RrdEngine::generate_installation_identity().unwrap(),
        "default",
        None,
        &executable,
    )
    .is_err());
    assert_eq!(inventory(project.path()), before);
}

#[test]
pub(super) fn installed_backups_and_restore_staging_live_beside_the_storage_roots() {
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
        &super::super::super::estate_layout::EstateFilesystem::admit(
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

/// #247 characterized the pre-#236 installer: an existing `.rrflow` folder
/// with ordinary inherited permissions was rejected before preview, and the
/// storage error had to name the directory. #236 adopts that folder instead,
/// so the same shape now previews as an adoption with no entries and still
/// has no file effects. The in-place preservation oracle #247 asked for stays
/// below under POAM-006.
#[cfg(windows)]
#[test]
pub(super) fn inherited_estate_preview_adopts_the_folder_without_effects() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join(".rrflow")).unwrap();
    let before = inventory(root.path());
    let preview = RrdEngine::plan_installation(
        root.path(),
        InstallationTargetKind::ExistingProject,
        &"1".repeat(64),
        "default",
        None,
        &test_executable(),
    )
    .unwrap();
    assert_eq!(
        preview.installation.adopted_estate_entries.as_deref(),
        Some(&[][..]),
        "an empty inherited .rrflow is adopted with no entries"
    );
    assert_eq!(inventory(root.path()), before);
}

#[cfg(windows)]
#[test]
#[ignore = "POAM-006: the installer still rejects an existing .rrflow with inherited permissions; #236 adopts it by relocation, this oracle asks for in-place preservation of bytes and ACLs, and no repair for that exists yet"]
pub(super) fn existing_project_preserves_inherited_estate_and_opens_after_replay() {
    use std::process::Command;

    let root = tempfile::tempdir().unwrap();
    let estate = root.path().join(".rrflow");
    let old_storage = estate.join("rrd");
    std::fs::create_dir_all(&old_storage).unwrap();
    std::fs::write(root.path().join("source.txt"), b"existing project").unwrap();
    std::fs::write(estate.join("user-notes.txt"), b"leave notes unchanged").unwrap();
    std::fs::write(old_storage.join("legacy-state"), b"leave state unchanged").unwrap();
    let before = inventory(root.path());
    let acl = |path: &Path| {
        let result = Command::new("icacls").arg(path).output().unwrap();
        assert!(result.status.success());
        result.stdout
    };
    let estate_acl = acl(&estate);
    let storage_acl = acl(&old_storage);
    assert!(estate_acl.windows(3).any(|part| part == b"(I)"));
    assert!(storage_acl.windows(3).any(|part| part == b"(I)"));
    let preview = plan(root.path());
    assert_eq!(
        inventory(root.path()),
        before,
        "preview must not modify existing data"
    );
    let executable = test_executable();
    let apply = || {
        RrdEngine::apply_installation_at(
            root.path(),
            InstallationTargetKind::ExistingProject,
            &preview,
            &preview.installation.plan_sha256,
            1_800_000_000_000,
            &executable,
        )
        .unwrap()
    };
    apply();
    let engine = open_at(root.path(), &executable, 1_800_000_001_000).unwrap();
    assert_eq!(
        engine.installed_estate_identity(),
        Some(&preview.installation.target)
    );
    drop(engine);
    assert!(apply().idempotent_replay);
    for (path, bytes) in before {
        assert_eq!(std::fs::read(root.path().join(path)).unwrap(), bytes);
    }
    assert_eq!(acl(&estate), estate_acl);
    assert_eq!(acl(&old_storage), storage_acl);
}
