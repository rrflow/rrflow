//! Retaining, verifying and restoring object payloads.

use super::*;

pub(super) fn retain_object_payloads(
    source: &impl ImmutableObjectStore,
    catalogue_root: &AnchoredPath,
    references: &[ObjectReference],
) -> Result<(String, ObjectPayloadManifestInventory)> {
    if references.len() > MAX_ARCHIVE_OBJECTS {
        return Err(Error::Archive(
            "backup object closure exceeds the v1 entry bound".into(),
        ));
    }
    let retained = LocalObjectStore::open_anchored(&catalogue_root.join(OBJECT_PAYLOAD_DIRECTORY))?;
    let mut entries = Vec::with_capacity(references.len());
    let mut payload_bytes = 0u64;
    for reference in references {
        let mut reader = source.open_verified(reference)?;
        let verified =
            retained.put_verified_stream(&reference.sha256, reference.length, reader.as_mut())?;
        if verified.length != reference.length {
            return Err(Error::ObjectLengthMismatch {
                expected: reference.length,
                actual: verified.length,
            });
        }
        payload_bytes = payload_bytes
            .checked_add(reference.length)
            .ok_or_else(|| Error::Archive("backup object payload bytes overflowed u64".into()))?;
        entries.push(ObjectPayloadEntry {
            sha256: reference.sha256.clone(),
            length: reference.length,
        });
    }
    entries.sort_by(|left, right| left.sha256.cmp(&right.sha256));
    let payload = ObjectPayloadManifest {
        format_version: OBJECT_MANIFEST_VERSION,
        objects: entries,
    };
    let payload_sha256 = digest::sha256_hex(&serde_json::to_vec(&payload)?);
    let stored = StoredObjectPayloadManifest {
        payload,
        payload_sha256: payload_sha256.clone(),
    };
    let manifests = catalogue_root.join(OBJECT_MANIFEST_DIRECTORY);
    manifests.create_dir_all().map_err(backup_io)?;
    let file_name = format!("{payload_sha256}.rrd-objects.json");
    let retained_path = manifests.join(&file_name);
    if retained_path.try_exists().map_err(backup_io)? {
        let actual = load_object_manifest(&retained_path)?;
        if actual != stored.payload {
            return Err(Error::Archive(
                "content-addressed object manifest conflicts with retained payload".into(),
            ));
        }
    } else {
        let id = BACKUP_TEMP_ID.fetch_add(1, Ordering::Relaxed);
        let staging = manifests.join(format!(".object-manifest-{}-{id}.tmp", std::process::id()));
        let bytes = serde_json::to_vec_pretty(&stored)?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&staging)
            .map_err(backup_io)?;
        file.write_all(&bytes).map_err(backup_io)?;
        file.sync_all().map_err(backup_io)?;
        drop(file);
        staging.rename_to(&retained_path).map_err(backup_io)?;
    }
    Ok((
        format!("{OBJECT_MANIFEST_DIRECTORY}/{file_name}"),
        ObjectPayloadManifestInventory {
            format_version: OBJECT_MANIFEST_VERSION,
            manifest_sha256: payload_sha256,
            object_count: references.len() as u64,
            payload_bytes,
        },
    ))
}

pub(super) fn verify_object_payloads(
    catalogue_root: &AnchoredPath,
    entry: &BackupEntry,
) -> Result<()> {
    let manifest = load_object_manifest_for_entry(catalogue_root, entry)?;
    let payloads =
        LocalObjectStore::open_existing_anchored(&catalogue_root.join(OBJECT_PAYLOAD_DIRECTORY))?;
    for object in manifest.objects {
        let verified = payloads.verify(&object.sha256)?;
        if verified.length != object.length {
            return Err(Error::ObjectLengthMismatch {
                expected: object.length,
                actual: verified.length,
            });
        }
    }
    Ok(())
}

pub(super) fn restore_object_payloads(
    catalogue_root: &AnchoredPath,
    manifest: &ObjectPayloadManifest,
    staging_root: &AnchoredPath,
) -> Result<()> {
    let source =
        LocalObjectStore::open_existing_anchored(&catalogue_root.join(OBJECT_PAYLOAD_DIRECTORY))?;
    let target = LocalObjectStore::open_anchored(&staging_root.join("immutable"))?;
    for object in &manifest.objects {
        let verified = source.verify(&object.sha256)?;
        if verified.length != object.length {
            return Err(Error::ObjectLengthMismatch {
                expected: object.length,
                actual: verified.length,
            });
        }
        let reference = ObjectReference::for_verified(
            format!("backup-{}", object.sha256),
            None,
            "application/octet-stream",
            object.sha256.clone(),
            object.length,
            verified.receipt,
        )?;
        let mut reader = source.open_verified(&reference)?;
        target.put_verified_stream(&object.sha256, object.length, reader.as_mut())?;
    }
    Ok(())
}

pub(super) fn load_object_manifest_for_entry(
    catalogue_root: &AnchoredPath,
    entry: &BackupEntry,
) -> Result<ObjectPayloadManifest> {
    let file = entry.object_manifest_file.as_deref().ok_or_else(|| {
        Error::Archive("application-complete backup has no object manifest path".into())
    })?;
    let expected = entry.object_manifest.as_ref().ok_or_else(|| {
        Error::Archive("application-complete backup has no object manifest inventory".into())
    })?;
    let manifest = load_object_manifest(&resolve_object_manifest(catalogue_root, file)?)?;
    let actual = object_manifest_inventory(&manifest)?;
    if &actual != expected {
        return Err(Error::Archive(format!(
            "backup {} object manifest inventory does not match catalogue",
            entry.backup_id
        )));
    }
    Ok(manifest)
}

pub(super) fn load_object_manifest(path: &AnchoredPath) -> Result<ObjectPayloadManifest> {
    let bytes = read_manifest_bytes(path)?;
    let stored: StoredObjectPayloadManifest = serde_json::from_slice(&bytes)?;
    validate_object_manifest(&stored.payload)?;
    let actual = digest::sha256_hex(&serde_json::to_vec(&stored.payload)?);
    if actual != stored.payload_sha256 {
        return Err(Error::Archive(
            "object manifest payload digest does not match".into(),
        ));
    }
    Ok(stored.payload)
}

pub(super) fn read_manifest_bytes(path: &AnchoredPath) -> Result<Vec<u8>> {
    let file = path.open_read().map_err(backup_io)?;
    if file.metadata().map_err(backup_io)?.len() > MAX_ARCHIVE_MANIFEST_BYTES {
        return Err(Error::Archive(
            "backup manifest exceeds the v1 byte bound".into(),
        ));
    }
    let mut bytes = Vec::new();
    file.take(MAX_ARCHIVE_MANIFEST_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(backup_io)?;
    if bytes.len() as u64 > MAX_ARCHIVE_MANIFEST_BYTES {
        return Err(Error::Archive(
            "backup manifest exceeds the v1 byte bound".into(),
        ));
    }
    Ok(bytes)
}

pub(super) fn object_manifest_inventory(
    manifest: &ObjectPayloadManifest,
) -> Result<ObjectPayloadManifestInventory> {
    validate_object_manifest(manifest)?;
    let payload_bytes = manifest.objects.iter().try_fold(0u64, |total, object| {
        total
            .checked_add(object.length)
            .ok_or_else(|| Error::Archive("object manifest payload bytes overflowed u64".into()))
    })?;
    Ok(ObjectPayloadManifestInventory {
        format_version: manifest.format_version,
        manifest_sha256: digest::sha256_hex(&serde_json::to_vec(manifest)?),
        object_count: manifest.objects.len() as u64,
        payload_bytes,
    })
}

pub(super) fn validate_object_manifest(manifest: &ObjectPayloadManifest) -> Result<()> {
    if manifest.format_version != OBJECT_MANIFEST_VERSION {
        return Err(Error::Archive(format!(
            "unsupported object manifest version {}",
            manifest.format_version
        )));
    }
    if manifest.objects.len() > MAX_ARCHIVE_OBJECTS {
        return Err(Error::Archive(
            "backup object closure exceeds the v1 entry bound".into(),
        ));
    }
    let mut previous: Option<&str> = None;
    for object in &manifest.objects {
        ObjectReference::canonical_key(&object.sha256)?;
        if previous.is_some_and(|prior| prior >= object.sha256.as_str()) {
            return Err(Error::Archive(
                "object manifest entries are not uniquely ordered".into(),
            ));
        }
        previous = Some(&object.sha256);
    }
    Ok(())
}
