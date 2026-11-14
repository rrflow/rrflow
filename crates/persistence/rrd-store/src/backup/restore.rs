//! Restoring a catalogued backup and verifying the restored objects.

use super::*;

pub fn restore_catalogued_backup(
    catalogue_root: &(impl AnchorInput + ?Sized),
    backup_id: &str,
    target: &(impl AnchorInput + ?Sized),
    at: u64,
) -> Result<LogicalRestoreReport> {
    restore_catalogued_backup_with_options(
        catalogue_root,
        backup_id,
        target,
        at,
        Default::default(),
    )
}

/// Restores using the same native policy for private staging and every reopen.
pub fn restore_catalogued_backup_with_options(
    catalogue_root: &(impl AnchorInput + ?Sized),
    backup_id: &str,
    target: &(impl AnchorInput + ?Sized),
    at: u64,
    options: crate::DatabaseOptions,
) -> Result<LogicalRestoreReport> {
    let catalogue_root = &catalogue_root.anchor().map_err(backup_io)?;
    let target = &target.anchor().map_err(backup_io)?;
    let catalogue = verify_backup_catalogue(catalogue_root)?;
    let entry = catalogue
        .backups
        .iter()
        .find(|entry| entry.backup_id == backup_id)
        .ok_or_else(|| Error::Archive(format!("backup is not catalogued: {backup_id}")))?;
    let archive = resolve_archive(catalogue_root, &entry.archive_file)?;
    if entry.application_complete {
        let object_manifest = load_object_manifest_for_entry(catalogue_root, entry)?;
        let catalogue_manifest = load_catalogue_manifest_for_entry(catalogue_root, entry)?;
        let (control_archive, control_inventory) =
            load_control_archive_for_entry(catalogue_root, entry)?;
        restore_logical_archive_to_new_root_with_options(
            &archive,
            target,
            at,
            options,
            |staging| {
                restore_catalogues(&catalogue_manifest, staging, options)?;
                restore_object_payloads(catalogue_root, &object_manifest, staging)?;
                restore_control_journal(&control_archive, &control_inventory, staging, options)
            },
            |_| Ok(()),
        )
    } else {
        restore_logical_archive_to_new_root_with_options(
            &archive,
            target,
            at,
            options,
            |_| Ok(()),
            |_| Ok(()),
        )
    }
}

/// Verifies, reading only, that an already published restore still contains
/// the complete immutable payload closure, catalogue state, and replayed
/// control journal of a catalogued backup.
pub fn verify_restored_backup_objects(
    catalogue_root: &(impl AnchorInput + ?Sized),
    backup_id: &str,
    restored_root: &(impl AnchorInput + ?Sized),
) -> Result<()> {
    verify_restored_backup_objects_with_options(
        catalogue_root,
        backup_id,
        restored_root,
        Default::default(),
    )
}

/// Verifies a published application restore under the caller's native policy.
pub fn verify_restored_backup_objects_with_options(
    catalogue_root: &(impl AnchorInput + ?Sized),
    backup_id: &str,
    restored_root: &(impl AnchorInput + ?Sized),
    options: crate::DatabaseOptions,
) -> Result<()> {
    let catalogue_root = &catalogue_root.anchor().map_err(backup_io)?;
    let restored_root = &restored_root.anchor().map_err(backup_io)?;
    let catalogue = verify_backup_catalogue(catalogue_root)?;
    let entry = catalogue
        .backups
        .iter()
        .find(|entry| entry.backup_id == backup_id)
        .ok_or_else(|| Error::Archive(format!("backup is not catalogued: {backup_id}")))?;
    if !entry.application_complete {
        return Err(Error::Archive(
            "logical-only backup cannot verify application-complete restore payloads".into(),
        ));
    }
    let manifest = load_object_manifest_for_entry(catalogue_root, entry)?;
    let restored = LocalObjectStore::open_existing_anchored(&restored_root.join("immutable"))?;
    for object in &manifest.objects {
        let verified = restored.verify(&object.sha256)?;
        if verified.length != object.length {
            return Err(Error::ObjectLengthMismatch {
                expected: object.length,
                actual: verified.length,
            });
        }
    }
    let catalogues = load_catalogue_manifest_for_entry(catalogue_root, entry)?;
    verify_restored_catalogues(&catalogues, restored_root, options)?;
    let (control_archive, control_inventory) =
        load_control_archive_for_entry(catalogue_root, entry)?;
    verify_restored_control_journal(&control_archive, &control_inventory, restored_root, options)
}

/// Reads the control-journal replay progress a restore left in a root, if any.
pub fn control_restore_progress(store: &RrflowKvStore) -> Result<Option<ControlRestoreProgress>> {
    let transaction = store.begin_transaction()?;
    read_control_restore_progress(&*transaction)
}

pub(super) fn read_control_restore_progress(
    transaction: &dyn crate::StorageTransaction,
) -> Result<Option<ControlRestoreProgress>> {
    get(
        transaction,
        keyspaces::META,
        &keyspaces::control_restore_progress_key(),
    )?
    .map(|bytes| {
        let progress: ControlRestoreProgress = serde_json::from_slice(&bytes)?;
        if progress.format_version != CONTROL_RESTORE_PROGRESS_FORMAT {
            return Err(Error::Archive(format!(
                "unsupported control restore progress version {}",
                progress.format_version
            )));
        }
        Ok(progress)
    })
    .transpose()
}

/// Replays the archived control-journal entries into the staging root, in
/// order, each as one control transition committed together with the replay
/// progress marker. An interrupted replay resumes after the last committed
/// entry; a target whose control state diverges from an entry's recorded
/// before-state fails closed before any further mutation.
pub(super) fn restore_control_journal(
    archive: &AnchoredPath,
    expected: &ControlArchiveInventory,
    staging_root: &AnchoredPath,
    options: crate::DatabaseOptions,
) -> Result<()> {
    let inspected = inspect_control_archive(archive)?;
    if &inspected != expected {
        return Err(Error::Archive(
            "control-journal archive inventory does not match its catalogue entry".into(),
        ));
    }
    let engine = RrflowKvStore::open_existing_anchored(staging_root, options)?;
    replay_control_entries(archive, expected, &engine, None)?;
    drop(engine);
    verify_restored_control_journal(archive, expected, staging_root, options)
}
