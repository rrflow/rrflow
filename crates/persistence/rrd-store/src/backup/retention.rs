//! Retaining archives, content-addressed files and catalogue entries.

use super::*;

pub(super) fn load_control_archive_for_entry(
    catalogue_root: &AnchoredPath,
    entry: &BackupEntry,
) -> Result<(AnchoredPath, ControlArchiveInventory)> {
    let file = entry.control_archive_file.as_deref().ok_or_else(|| {
        Error::Archive("application-complete backup has no control-journal archive path".into())
    })?;
    let expected = entry.control_archive.as_ref().ok_or_else(|| {
        Error::Archive(
            "application-complete backup has no control-journal archive inventory".into(),
        )
    })?;
    let path = resolve_control_archive(catalogue_root, file)?;
    let actual = inspect_control_archive(&path)?;
    if &actual != expected {
        return Err(Error::Archive(format!(
            "backup {} control-journal archive inventory does not match catalogue",
            entry.backup_id
        )));
    }
    Ok((path, actual))
}

pub(super) fn retain_content_addressed_file(
    staging: &AnchoredPath,
    retained: &AnchoredPath,
    inventory: &LogicalArchiveInventory,
) -> Result<()> {
    if retained.try_exists().map_err(backup_io)? {
        let actual = inspect_logical_archive(retained)?;
        if &actual != inventory {
            let _ = staging.remove_file();
            return Err(Error::Archive(
                "content-addressed backup archive conflicts with retained inventory".into(),
            ));
        }
        staging.remove_file().map_err(backup_io)
    } else {
        staging.rename_to(retained).map_err(backup_io)
    }
}

pub(super) fn retain_catalogue_entry(
    catalogue_root: &AnchoredPath,
    entry: BackupEntry,
) -> Result<BackupEntry> {
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

pub(super) fn object_references_at(
    engine: &impl StorageEngine,
    head: u64,
) -> Result<Vec<ObjectReference>> {
    let mut after = 0u64;
    let mut objects = std::collections::BTreeMap::<String, ObjectReference>::new();
    while after < head {
        let page = engine.runtime().changes_since(after, PAGE_SIZE, None)?;
        if page.head_cursor != head || page.requested_after != after {
            return Err(Error::Archive(
                "runtime watermark changed while collecting backup object references".into(),
            ));
        }
        if page.through_cursor <= after || page.through_cursor > head {
            return Err(Error::Archive(
                "runtime object-reference scan did not advance canonically".into(),
            ));
        }
        for change in page.changes {
            let RuntimeMutation::Object { object } = change.mutation else {
                continue;
            };
            object.validate()?;
            if let Some(existing) = objects.get(&object.sha256) {
                if existing.length != object.length {
                    return Err(Error::Archive(format!(
                        "object digest {} has conflicting lengths in the backup cut",
                        object.sha256
                    )));
                }
            } else {
                if objects.len() == MAX_ARCHIVE_OBJECTS {
                    return Err(Error::Archive(
                        "backup object closure exceeds the v1 entry bound".into(),
                    ));
                }
                objects.insert(object.sha256.clone(), object);
            }
        }
        after = page.through_cursor;
    }
    Ok(objects.into_values().collect())
}
