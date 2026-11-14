//! Reading and writing the catalogue file and validating prune plans.

use super::*;

pub(super) fn empty_catalogue_payload() -> CataloguePayload {
    CataloguePayload {
        format_version: BACKUP_CATALOGUE_VERSION,
        revision: 0,
        backups: Vec::new(),
        prune_receipts: Vec::new(),
    }
}

/// Retained artifacts exist only for catalogued entries, so a catalogue file
/// that is absent beside them was lost; reading it as empty would hide that.
pub(super) fn require_no_orphan_artifacts(catalogue_root: &AnchoredPath) -> Result<()> {
    for directory in [
        ARCHIVE_DIRECTORY,
        OBJECT_MANIFEST_DIRECTORY,
        CATALOGUE_MANIFEST_DIRECTORY,
        CONTROL_ARCHIVE_DIRECTORY,
        OBJECT_PAYLOAD_DIRECTORY,
    ] {
        if catalogue_root
            .join(directory)
            .try_exists()
            .map_err(backup_io)?
        {
            return Err(Error::Archive(format!(
                "backup catalogue is absent while retained {directory} exist beside it"
            )));
        }
    }
    Ok(())
}

/// Publishes the empty catalogue before the first artifact is retained, so
/// every retained artifact is younger than the catalogue that owns it and an
/// absent catalogue beside artifacts is always a loss.
pub(super) fn initialize_catalogue(catalogue_root: &AnchoredPath) -> Result<()> {
    if catalogue_root
        .join(CATALOGUE_FILE)
        .try_exists()
        .map_err(backup_io)?
    {
        return Ok(());
    }
    require_no_orphan_artifacts(catalogue_root)?;
    catalogue_root.create_dir_all().map_err(backup_io)?;
    write_catalogue(
        catalogue_root,
        catalogue_from_payload(empty_catalogue_payload())?,
    )
}

pub(super) fn catalogue_from_payload(payload: CataloguePayload) -> Result<BackupCatalogue> {
    Ok(BackupCatalogue {
        format_version: payload.format_version,
        revision: payload.revision,
        catalogue_sha256: payload_digest(&payload)?,
        backups: payload.backups,
        prune_receipts: payload.prune_receipts,
    })
}

pub(super) fn write_catalogue(root: &AnchoredPath, catalogue: BackupCatalogue) -> Result<()> {
    let payload = CataloguePayload {
        format_version: BACKUP_CATALOGUE_VERSION,
        revision: catalogue.revision,
        backups: catalogue.backups,
        prune_receipts: catalogue.prune_receipts,
    };
    validate_entries(&payload.backups)?;
    validate_prune_receipts(&payload.prune_receipts)?;
    let stored = StoredCatalogue {
        payload_sha256: payload_digest(&payload)?,
        payload,
    };
    let bytes = serde_json::to_vec_pretty(&stored)?;
    let id = BACKUP_TEMP_ID.fetch_add(1, Ordering::Relaxed);
    let temporary = root.join(format!(".catalogue-{}-{id}.tmp", std::process::id()));
    let path = root.join(CATALOGUE_FILE);
    crate::filesystem::persist_rename(&temporary, &path, &bytes).map_err(backup_io)
}

pub(super) fn payload_digest(payload: &CataloguePayload) -> Result<String> {
    Ok(digest::sha256_hex(&serde_json::to_vec(payload)?))
}

pub(super) fn validate_prune_plan(plan: &BackupPrunePlan) -> Result<()> {
    validate_sha256_text(
        &plan.expected_catalogue_sha256,
        "backup prune expected catalogue",
    )?;
    if plan.prune_candidate_backup_ids.is_empty() {
        return Err(Error::Archive(
            "backup prune requires at least one candidate".into(),
        ));
    }
    validate_ordered_backup_ids(&plan.retained_backup_ids, "retained backup")?;
    validate_ordered_backup_ids(&plan.prune_candidate_backup_ids, "backup prune candidate")
}

pub(super) fn validate_prune_receipts(receipts: &[BackupPruneReceipt]) -> Result<()> {
    if receipts.len() > MAX_BACKUP_PRUNE_RECEIPTS {
        return Err(Error::Archive(
            "backup prune receipt history exceeds its v1 bound".into(),
        ));
    }
    let mut requests = BTreeSet::new();
    let mut removed_identities = BTreeSet::new();
    for receipt in receipts {
        validate_sha256_text(&receipt.request_sha256, "backup prune request")?;
        validate_sha256_text(
            &receipt.expected_catalogue_sha256,
            "backup prune expected catalogue",
        )?;
        if !requests.insert(receipt.request_sha256.as_str()) {
            return Err(Error::Archive(
                "backup prune receipt request identity is duplicated".into(),
            ));
        }
        validate_ordered_backup_ids(&receipt.retained_backup_ids, "retained backup")?;
        if receipt.removed_backups.is_empty() {
            return Err(Error::Archive(
                "backup prune receipt has no removed backup".into(),
            ));
        }
        validate_entries(&receipt.removed_backups)?;
        for entry in &receipt.removed_backups {
            if receipt
                .retained_backup_ids
                .binary_search(&entry.backup_id)
                .is_ok()
                || !removed_identities.insert(entry.backup_id.as_str())
            {
                return Err(Error::Archive(
                    "backup prune receipt identity is inconsistent".into(),
                ));
            }
        }
    }
    Ok(())
}

pub(super) fn validate_ordered_backup_ids(values: &[String], name: &str) -> Result<()> {
    let mut previous: Option<&str> = None;
    for value in values {
        validate_sha256_text(value, name)?;
        if previous.is_some_and(|prior| prior >= value.as_str()) {
            return Err(Error::Archive(format!(
                "{name} identities are not uniquely ordered"
            )));
        }
        previous = Some(value);
    }
    Ok(())
}

pub(super) fn validate_sha256_text(value: &str, name: &str) -> Result<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err(Error::Archive(format!("{name} identity is invalid")));
    }
    Ok(())
}

pub(super) fn sorted_backup_ids(mut values: Vec<String>) -> Vec<String> {
    values.sort();
    values
}

pub(super) fn deny_pruned_identity_reuse(
    catalogue: &BackupCatalogue,
    backup_id: &str,
) -> Result<()> {
    if catalogue.prune_receipts.iter().any(|receipt| {
        receipt
            .removed_backups
            .iter()
            .any(|entry| entry.backup_id == backup_id)
    }) {
        return Err(Error::Archive(
            "a pruned backup identity cannot be resurrected".into(),
        ));
    }
    Ok(())
}

pub(super) fn cleanup_pruned_artifacts(
    catalogue_root: &AnchoredPath,
    removed: &[BackupEntry],
    retained: &[BackupEntry],
) -> Result<()> {
    let mut retained_files = BTreeSet::new();
    let mut retained_objects = BTreeSet::new();
    for entry in retained {
        retained_files.insert(entry.archive_file.clone());
        if let Some(path) = &entry.object_manifest_file {
            retained_files.insert(path.clone());
            retained_objects.extend(
                load_object_manifest_for_entry(catalogue_root, entry)?
                    .objects
                    .into_iter()
                    .map(|object| object.sha256),
            );
        }
        if let Some(path) = &entry.catalogue_manifest_file {
            retained_files.insert(path.clone());
        }
        if let Some(path) = &entry.control_archive_file {
            retained_files.insert(path.clone());
        }
    }
    let mut removed_files = BTreeSet::new();
    let mut removed_objects = BTreeSet::new();
    for entry in removed {
        removed_files.insert(entry.archive_file.clone());
        if let Some(path) = &entry.object_manifest_file {
            let manifest_path = resolve_object_manifest(catalogue_root, path)?;
            if manifest_path.try_exists().map_err(backup_io)? {
                removed_objects.extend(
                    load_object_manifest_for_entry(catalogue_root, entry)?
                        .objects
                        .into_iter()
                        .map(|object| object.sha256),
                );
            }
            removed_files.insert(path.clone());
        }
        if let Some(path) = &entry.catalogue_manifest_file {
            removed_files.insert(path.clone());
        }
        if let Some(path) = &entry.control_archive_file {
            removed_files.insert(path.clone());
        }
    }
    let unreachable_objects = removed_objects
        .difference(&retained_objects)
        .cloned()
        .collect::<BTreeSet<_>>();
    LocalObjectStore::open_anchored(&catalogue_root.join(OBJECT_PAYLOAD_DIRECTORY))?
        .reclaim_orphans(&unreachable_objects)?;
    for relative in removed_files.difference(&retained_files) {
        let path = if relative.starts_with(&format!("{ARCHIVE_DIRECTORY}/")) {
            resolve_archive(catalogue_root, relative)?
        } else if relative.starts_with(&format!("{OBJECT_MANIFEST_DIRECTORY}/")) {
            resolve_object_manifest(catalogue_root, relative)?
        } else if relative.starts_with(&format!("{CONTROL_ARCHIVE_DIRECTORY}/")) {
            resolve_control_archive(catalogue_root, relative)?
        } else {
            resolve_catalogue_manifest(catalogue_root, relative)?
        };
        match path.remove_file() {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(backup_io(error)),
        }
    }
    for directory in [
        ARCHIVE_DIRECTORY,
        OBJECT_MANIFEST_DIRECTORY,
        CATALOGUE_MANIFEST_DIRECTORY,
        CONTROL_ARCHIVE_DIRECTORY,
        OBJECT_PAYLOAD_DIRECTORY,
    ] {
        let path = catalogue_root.join(directory);
        if path.try_exists().map_err(backup_io)? && path.metadata().map_err(backup_io)?.is_dir() {
            path.sync_directory().map_err(backup_io)?;
        }
    }
    Ok(())
}
