use super::*;
use crate::{InstallationPreview, InstallationVerificationStatus};
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
        "default",
        None,
        &test_executable(),
    )
    .unwrap()
}

fn test_executable() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml")
}

#[test]
fn planning_is_byte_identical_and_has_no_project_effect() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("README.md"), "walking product\n").unwrap();
    let before = inventory(project.path());
    let first = plan(project.path());
    let second = plan(project.path());
    assert_eq!(first.installation.initial_grants, SecurityAction::ALL);
    assert_eq!(first.installation.managed_paths.len(), 8);
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
    let result = RrdEngine::apply_installation(
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

    assert!(RrdEngine::open_installed(project.path(), &project.path().join("README.md")).is_err());
    let engine = RrdEngine::open_installed(project.path(), &executable).unwrap();
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
    let report = RrdEngine::inspect_installed(project.path(), &executable).unwrap();
    assert_eq!(report.status, InstallationVerificationStatus::Passed);
    assert!(report.attunement_source_current);
    assert!(report.checks.iter().all(|check| check.passed));
    assert_eq!(inventory(project.path()), before);

    std::fs::write(
        project.path().join("README.md"),
        "walking product under active development\n",
    )
    .unwrap();
    let drift = RrdEngine::inspect_installed(project.path(), &executable).unwrap();
    assert_eq!(drift.status, InstallationVerificationStatus::Passed);
    assert!(!drift.attunement_source_current);
    assert_ne!(
        drift.project_inventory_sha256,
        preview.installation.project_precondition_sha256
    );

    let replay = RrdEngine::apply_installation(
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
fn changed_project_or_credential_permissions_fail_closed() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("README.md"), "before\n").unwrap();
    let preview = plan(project.path());
    std::fs::write(project.path().join("README.md"), "after\n").unwrap();
    assert!(RrdEngine::apply_installation(
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
    RrdEngine::apply_installation(
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

#[cfg(unix)]
#[test]
fn installed_open_and_inspection_reject_symbolic_object_paths() {
    use std::os::unix::fs::symlink;

    let project = tempfile::tempdir().unwrap();
    let executable = test_executable();
    let preview = plan(project.path());
    RrdEngine::apply_installation(
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
    forged.installation.plan_sha256 = installation_plan_sha256(&forged.installation).unwrap();
    forged.validate().unwrap();

    assert!(RrdEngine::apply_installation(
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
        "default",
        None,
        &executable,
    )
    .unwrap();

    let first = RrdEngine::apply_installation(
        project.path(),
        InstallationTargetKind::FreshProject,
        &preview,
        &preview.installation.plan_sha256,
        1_800_000_000_000,
        &executable,
    )
    .unwrap();
    assert!(!first.idempotent_replay);

    let replay = RrdEngine::apply_installation(
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
        r#"format_version = 1

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
        "default",
        Some(input.path()),
        &executable,
    )
    .unwrap();
    assert_eq!(preview.installation.configuration.query.max_rows, 9);
    std::fs::write(input.path(), "this no longer parses as TOML = [").unwrap();

    RrdEngine::apply_installation(
        project.path(),
        InstallationTargetKind::ExistingProject,
        &preview,
        &preview.installation.plan_sha256,
        1_800_000_000_000,
        &executable,
    )
    .unwrap();
    let engine = RrdEngine::open_installed(project.path(), &executable).unwrap();
    assert_eq!(engine.estate_configuration().query.max_rows, 9);
    drop(engine);
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
    assert!(RrdEngine::open_installed(project.path(), &executable).is_err());
    assert!(RrdEngine::inspect_installed(project.path(), &executable).is_err());
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
        "default",
        None,
        &executable,
    )
    .unwrap();
    RrdEngine::apply_installation(
        &original,
        InstallationTargetKind::ExistingProject,
        &preview,
        &preview.installation.plan_sha256,
        1_800_000_000_000,
        &executable,
    )
    .unwrap();

    std::fs::rename(&original, &moved).unwrap();
    let relocated = RrdEngine::open_installed(&moved, &executable).unwrap();
    assert_eq!(
        relocated.installed_estate_identity(),
        Some(&preview.installation.target)
    );
    drop(relocated);

    let nested = moved.join("nested");
    std::fs::create_dir(&nested).unwrap();
    assert!(RrdEngine::open_installed(&nested, &executable).is_err());

    std::fs::create_dir(&foreign).unwrap();
    std::fs::write(foreign.join("README.md"), "different project\n").unwrap();
    let foreign_preview = RrdEngine::plan_installation(
        &foreign,
        InstallationTargetKind::ExistingProject,
        "default",
        None,
        &executable,
    )
    .unwrap();
    assert_ne!(
        foreign_preview.installation.target,
        preview.installation.target
    );
    RrdEngine::apply_installation(
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
    assert!(RrdEngine::open_installed(&moved, &executable).is_err());
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
        1_800_000_000_000,
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
    validate_recovery_estate(project.path(), &layout)
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
        1_800_000_000_000,
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
