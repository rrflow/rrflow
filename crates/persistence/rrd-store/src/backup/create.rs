//! Creating logical and application-complete backups.

use super::*;

/// Creates or reuses a content-addressed archive and atomically adds its
/// coverage record to the catalogue.
pub fn create_logical_backup<E: StorageEngine>(
    engine: &E,
    catalogue_root: &(impl AnchorInput + ?Sized),
    label: &str,
    created_at: u64,
) -> Result<BackupEntry> {
    let catalogue_root = &catalogue_root.anchor().map_err(backup_io)?;
    validate_label(label)?;
    initialize_catalogue(catalogue_root)?;
    let archives = catalogue_root.join(ARCHIVE_DIRECTORY);
    archives.create_dir_all().map_err(backup_io)?;
    let id = BACKUP_TEMP_ID.fetch_add(1, Ordering::Relaxed);
    let staging = archives.join(format!(".logical-backup-{}-{id}.tmp", std::process::id()));
    let inventory = export_logical_archive(engine, &staging)?;
    let archive_name = format!("{}.rrd-archive", inventory.archive_sha256);
    let archive_path = archives.join(&archive_name);
    if archive_path.try_exists().map_err(backup_io)? {
        let retained = inspect_logical_archive(&archive_path)?;
        if retained != inventory {
            let _ = staging.remove_file();
            return Err(Error::Archive(
                "content-addressed backup archive conflicts with retained inventory".into(),
            ));
        }
        staging.remove_file().map_err(backup_io)?;
    } else {
        staging.rename_to(&archive_path).map_err(backup_io)?;
    }

    let backup_id = digest::sha256_hex(
        format!(
            "rrd-logical-backup-v1\0{label}\0{created_at}\0{}",
            inventory.archive_sha256
        )
        .as_bytes(),
    );
    let entry = BackupEntry {
        backup_id,
        label: label.into(),
        created_at,
        archive_file: format!("{ARCHIVE_DIRECTORY}/{archive_name}"),
        archive: inventory,
        object_manifest_file: None,
        object_manifest: None,
        catalogue_manifest_file: None,
        catalogue_manifest: None,
        control_archive_file: None,
        control_archive: None,
        claims: BackupCoverage::Included,
        typed_runtime: BackupCoverage::Included,
        catalogues: BackupCoverage::Excluded,
        installed_control: BackupCoverage::Excluded,
        object_payloads: BackupCoverage::ReferencedOnly,
        projections: BackupCoverage::RebuildRequired,
        invocation_telemetry: BackupCoverage::Excluded,
        snapshot_leases: BackupCoverage::Excluded,
        application_complete: false,
    };

    let mut catalogue = load_backup_catalogue(catalogue_root)?;
    deny_pruned_identity_reuse(&catalogue, &entry.backup_id)?;
    if let Some(existing) = catalogue
        .backups
        .iter()
        .find(|existing| existing.backup_id == entry.backup_id)
    {
        if existing == &entry {
            return Ok(existing.clone());
        }
        return Err(Error::Archive("backup identity collision".into()));
    }
    catalogue.revision = catalogue
        .revision
        .checked_add(1)
        .ok_or_else(|| Error::Archive("backup catalogue revision overflow".into()))?;
    catalogue.backups.push(entry.clone());
    catalogue.backups.sort_by(|left, right| {
        (left.created_at, &left.backup_id).cmp(&(right.created_at, &right.backup_id))
    });
    write_catalogue(catalogue_root, catalogue)?;
    Ok(entry)
}

/// Creates an application-complete logical backup by retaining the exact
/// immutable-object closure referenced by the exported runtime cut, the
/// catalogue control state, and the control-journal entries of the declared
/// `control_families`.
///
/// The caller owns the control-key layout and declares which families are
/// estate control state. Secrets never live in control values (the token key
/// and operator credential are files outside every family), and transient
/// families such as sessions, subscriptions, and operation receipts stay out
/// of the scope. Logical replay, object bytes, catalogues, and control entries
/// remain separate physical artifacts, but one digest-verified catalogue entry
/// binds their content identities. All repositories read one admitted physical
/// snapshot, retained through publication; concurrent writes belong to a later
/// cut and cannot mix into this archive.
pub fn create_application_backup<E: StorageEngine, O: ImmutableObjectStore>(
    engine: &E,
    objects: &O,
    catalogue_root: &(impl AnchorInput + ?Sized),
    label: &str,
    created_at: u64,
    control_families: &[String],
) -> Result<BackupEntry> {
    let snapshot = crate::snapshot::SnapshotStore::capture(engine)?;
    let engine = &snapshot;
    let catalogue_root = &catalogue_root.anchor().map_err(backup_io)?;
    validate_label(label)?;
    validate_control_families(control_families)?;
    initialize_catalogue(catalogue_root)?;
    let control_sequence = engine.control().sequence()?;
    let archives = catalogue_root.join(ARCHIVE_DIRECTORY);
    archives.create_dir_all().map_err(backup_io)?;
    let id = BACKUP_TEMP_ID.fetch_add(1, Ordering::Relaxed);
    let staging = archives.join(format!(".logical-backup-{}-{id}.tmp", std::process::id()));
    let inventory = export_logical_archive(engine, &staging)?;
    let archive_name = format!("{}.rrd-archive", inventory.archive_sha256);
    let archive_path = archives.join(&archive_name);
    retain_content_addressed_file(&staging, &archive_path, &inventory)?;

    let references = object_references_at(engine, inventory.runtime_cursor)?;
    if engine.claims().sequence()? != inventory.claim_sequence
        || engine.runtime().cursor()? != inventory.runtime_cursor
    {
        return Err(Error::Archive(
            "source watermarks changed while retaining object payloads; retry from a stable cut"
                .into(),
        ));
    }
    let (manifest_file, manifest_inventory) =
        retain_object_payloads(objects, catalogue_root, &references)?;
    let (control_archive_file, control_archive, catalogues) =
        retain_control_journal(engine, catalogue_root, control_sequence, control_families)?;
    let (catalogue_manifest_file, catalogue_manifest) =
        retain_catalogues(engine, catalogue_root, control_sequence, catalogues)?;
    if engine.claims().sequence()? != inventory.claim_sequence
        || engine.runtime().cursor()? != inventory.runtime_cursor
        || engine.control().sequence()? != control_sequence
        || catalogue_manifest.source_control_sequence != control_sequence
        || control_archive.source_control_sequence != control_sequence
    {
        return Err(Error::Archive(
            "source data or control watermarks changed before application-complete backup publication; retry"
                .into(),
        ));
    }

    let backup_id = digest::sha256_hex(
        format!(
            "rrd-application-backup-v2\0{label}\0{created_at}\0{}\0{}\0{}\0{}",
            inventory.archive_sha256,
            manifest_inventory.manifest_sha256,
            catalogue_manifest.manifest_sha256,
            control_archive.archive_sha256
        )
        .as_bytes(),
    );
    let entry = BackupEntry {
        backup_id,
        label: label.into(),
        created_at,
        archive_file: format!("{ARCHIVE_DIRECTORY}/{archive_name}"),
        archive: inventory,
        object_manifest_file: Some(manifest_file),
        object_manifest: Some(manifest_inventory),
        catalogue_manifest_file: Some(catalogue_manifest_file),
        catalogue_manifest: Some(catalogue_manifest),
        control_archive_file: Some(control_archive_file),
        control_archive: Some(control_archive),
        claims: BackupCoverage::Included,
        typed_runtime: BackupCoverage::Included,
        catalogues: BackupCoverage::Included,
        installed_control: BackupCoverage::Included,
        object_payloads: BackupCoverage::Included,
        projections: BackupCoverage::RebuildRequired,
        invocation_telemetry: BackupCoverage::Excluded,
        snapshot_leases: BackupCoverage::Excluded,
        application_complete: true,
    };
    retain_catalogue_entry(catalogue_root, entry)
}
