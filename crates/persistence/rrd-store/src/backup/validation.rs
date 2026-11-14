//! Entry, label and relative-path validation and archive resolution.

use super::*;

pub(super) fn validate_entries(entries: &[BackupEntry]) -> Result<()> {
    let mut previous: Option<(u64, &str)> = None;
    let mut identities = std::collections::BTreeSet::new();
    for entry in entries {
        validate_label(&entry.label)?;
        if entry.backup_id.len() != 64
            || !entry
                .backup_id
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
            || !identities.insert(&entry.backup_id)
        {
            return Err(Error::Archive(
                "backup catalogue has an invalid or duplicate identity".into(),
            ));
        }
        let order = (entry.created_at, entry.backup_id.as_str());
        if previous.is_some_and(|prior| prior >= order) {
            return Err(Error::Archive(
                "backup catalogue entries are not canonically ordered".into(),
            ));
        }
        previous = Some(order);
        validate_relative_archive(&entry.archive_file)?;
        let common_coverage = entry.claims == BackupCoverage::Included
            && entry.typed_runtime == BackupCoverage::Included
            && entry.projections == BackupCoverage::RebuildRequired
            && entry.invocation_telemetry == BackupCoverage::Excluded
            && entry.snapshot_leases == BackupCoverage::Excluded;
        let logical_only = entry.object_payloads == BackupCoverage::ReferencedOnly
            && entry.catalogues == BackupCoverage::Excluded
            && entry.installed_control == BackupCoverage::Excluded
            && !entry.application_complete
            && entry.object_manifest_file.is_none()
            && entry.object_manifest.is_none()
            && entry.catalogue_manifest_file.is_none()
            && entry.catalogue_manifest.is_none()
            && entry.control_archive_file.is_none()
            && entry.control_archive.is_none();
        let application_complete = entry.object_payloads == BackupCoverage::Included
            && entry.catalogues == BackupCoverage::Included
            && entry.installed_control == BackupCoverage::Included
            && entry.application_complete
            && entry.object_manifest_file.is_some()
            && entry.object_manifest.is_some()
            && entry.catalogue_manifest_file.is_some()
            && entry.catalogue_manifest.is_some()
            && entry.control_archive_file.is_some()
            && entry.control_archive.is_some();
        if !common_coverage || (!logical_only && !application_complete) {
            return Err(Error::Archive(
                "backup v2 coverage declaration is not canonical".into(),
            ));
        }
        if let (Some(file), Some(inventory)) = (&entry.object_manifest_file, &entry.object_manifest)
        {
            validate_relative_object_manifest(file)?;
            if inventory.format_version != OBJECT_MANIFEST_VERSION
                || inventory.manifest_sha256.len() != 64
                || !inventory
                    .manifest_sha256
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
            {
                return Err(Error::Archive(
                    "backup object manifest inventory is invalid".into(),
                ));
            }
            let expected = format!(
                "{OBJECT_MANIFEST_DIRECTORY}/{}.rrd-objects.json",
                inventory.manifest_sha256
            );
            if file != &expected {
                return Err(Error::Archive(
                    "backup object manifest path does not match its content identity".into(),
                ));
            }
        } else if entry.object_manifest_file.is_some() || entry.object_manifest.is_some() {
            return Err(Error::Archive(
                "backup object manifest path and inventory must be present together".into(),
            ));
        }
        if let (Some(file), Some(inventory)) =
            (&entry.catalogue_manifest_file, &entry.catalogue_manifest)
        {
            validate_relative_catalogue_manifest(file)?;
            if inventory.format_version != CATALOGUE_MANIFEST_VERSION
                || inventory.manifest_sha256.len() != 64
                || !inventory
                    .manifest_sha256
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
            {
                return Err(Error::Archive(
                    "backup catalogue manifest inventory is invalid".into(),
                ));
            }
            let expected = format!(
                "{CATALOGUE_MANIFEST_DIRECTORY}/{}.rrd-catalogues.json",
                inventory.manifest_sha256
            );
            if file != &expected {
                return Err(Error::Archive(
                    "backup catalogue manifest path does not match its content identity".into(),
                ));
            }
        } else if entry.catalogue_manifest_file.is_some() || entry.catalogue_manifest.is_some() {
            return Err(Error::Archive(
                "backup catalogue manifest path and inventory must be present together".into(),
            ));
        }
        if let (Some(file), Some(inventory)) = (&entry.control_archive_file, &entry.control_archive)
        {
            validate_relative_control_archive(file)?;
            if inventory.format_version != CONTROL_ARCHIVE_VERSION {
                return Err(Error::Archive(
                    "backup control-journal archive inventory is invalid".into(),
                ));
            }
            validate_sha256_text(&inventory.archive_sha256, "backup control-journal archive")?;
            validate_control_families(&inventory.families)?;
            let empty_inventory_consistent = inventory.entry_count != 0
                || (inventory.first_sequence.is_none() && inventory.last_sequence.is_none());
            let populated_inventory_consistent = inventory.entry_count == 0
                || (inventory.first_sequence.is_some()
                    && inventory.last_sequence.is_some()
                    && inventory.last_sequence <= Some(inventory.source_control_sequence)
                    && inventory.first_sequence <= inventory.last_sequence);
            if !empty_inventory_consistent || !populated_inventory_consistent {
                return Err(Error::Archive(
                    "backup control-journal archive inventory is invalid".into(),
                ));
            }
            let expected = format!(
                "{CONTROL_ARCHIVE_DIRECTORY}/{}.rrd-control-archive",
                inventory.archive_sha256
            );
            if file != &expected {
                return Err(Error::Archive(
                    "backup control-journal archive path does not match its content identity"
                        .into(),
                ));
            }
        } else if entry.control_archive_file.is_some() || entry.control_archive.is_some() {
            return Err(Error::Archive(
                "backup control-journal archive path and inventory must be present together".into(),
            ));
        }
        if entry.claims != BackupCoverage::Included
            || entry.typed_runtime != BackupCoverage::Included
            || entry.projections != BackupCoverage::RebuildRequired
            || entry.invocation_telemetry != BackupCoverage::Excluded
            || entry.snapshot_leases != BackupCoverage::Excluded
        {
            return Err(Error::Archive(
                "backup coverage declaration is not canonical".into(),
            ));
        }
    }
    Ok(())
}

pub(super) fn validate_label(label: &str) -> Result<()> {
    if label.is_empty()
        || label.len() > 128
        || label.trim() != label
        || !label
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return Err(Error::Archive(
            "backup label must be 1-128 ASCII alphanumeric, '.', '-', or '_' characters".into(),
        ));
    }
    Ok(())
}

pub(super) fn validate_relative_archive(value: &str) -> Result<()> {
    let path = Path::new(value);
    let components: Vec<_> = path.components().collect();
    if components.len() != 2
        || components[0] != Component::Normal(ARCHIVE_DIRECTORY.as_ref())
        || !matches!(components[1], Component::Normal(_))
        || !value.ends_with(".rrd-archive")
    {
        return Err(Error::Archive(
            "backup archive path is not canonical".into(),
        ));
    }
    Ok(())
}

pub(super) fn validate_relative_object_manifest(value: &str) -> Result<()> {
    let path = Path::new(value);
    let components: Vec<_> = path.components().collect();
    if components.len() != 2
        || components[0] != Component::Normal(OBJECT_MANIFEST_DIRECTORY.as_ref())
        || !matches!(components[1], Component::Normal(_))
        || !value.ends_with(".rrd-objects.json")
    {
        return Err(Error::Archive(
            "backup object manifest path is not canonical".into(),
        ));
    }
    Ok(())
}

pub(super) fn validate_relative_catalogue_manifest(value: &str) -> Result<()> {
    let path = Path::new(value);
    let components: Vec<_> = path.components().collect();
    if components.len() != 2
        || components[0] != Component::Normal(CATALOGUE_MANIFEST_DIRECTORY.as_ref())
        || !matches!(components[1], Component::Normal(_))
        || !value.ends_with(".rrd-catalogues.json")
    {
        return Err(Error::Archive(
            "backup catalogue manifest path is not canonical".into(),
        ));
    }
    Ok(())
}

pub(super) fn validate_relative_control_archive(value: &str) -> Result<()> {
    let path = Path::new(value);
    let components: Vec<_> = path.components().collect();
    if components.len() != 2
        || components[0] != Component::Normal(CONTROL_ARCHIVE_DIRECTORY.as_ref())
        || !matches!(components[1], Component::Normal(_))
        || !value.ends_with(".rrd-control-archive")
    {
        return Err(Error::Archive(
            "backup control-journal archive path is not canonical".into(),
        ));
    }
    Ok(())
}

pub(super) fn resolve_archive(root: &AnchoredPath, value: &str) -> Result<AnchoredPath> {
    validate_relative_archive(value)?;
    Ok(root.join(value))
}

pub(super) fn resolve_control_archive(root: &AnchoredPath, value: &str) -> Result<AnchoredPath> {
    validate_relative_control_archive(value)?;
    Ok(root.join(value))
}

pub(super) fn resolve_object_manifest(root: &AnchoredPath, value: &str) -> Result<AnchoredPath> {
    validate_relative_object_manifest(value)?;
    Ok(root.join(value))
}

pub(super) fn resolve_catalogue_manifest(root: &AnchoredPath, value: &str) -> Result<AnchoredPath> {
    validate_relative_catalogue_manifest(value)?;
    Ok(root.join(value))
}

pub(super) fn backup_io(error: std::io::Error) -> Error {
    Error::Archive(error.to_string())
}
