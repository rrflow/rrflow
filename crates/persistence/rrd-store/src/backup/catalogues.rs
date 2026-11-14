//! Capturing, retaining, validating and restoring catalogue manifests.

use super::*;

#[derive(Default)]
pub(super) struct CatalogueCapture {
    records: std::collections::BTreeMap<String, CatalogueRecordSnapshot>,
    scopes: BTreeSet<ScopeId>,
}

impl CatalogueCapture {
    pub(super) fn observe(&mut self, entry: crate::ControlJournalEntry) -> Result<()> {
        let Some(scope) = catalogue_scope(&entry.key)? else {
            return Ok(());
        };
        self.scopes.insert(scope.clone());
        match entry.replacement {
            Some(value) => {
                if !self.records.contains_key(&entry.key)
                    && self.records.len() == MAX_ARCHIVE_CATALOGUE_RECORDS
                {
                    return Err(Error::Archive(
                        "backup catalogue closure exceeds the v1 entry bound".into(),
                    ));
                }
                self.records.insert(
                    entry.key.clone(),
                    CatalogueRecordSnapshot {
                        scope,
                        key: entry.key,
                        value,
                    },
                );
            }
            None => {
                self.records.remove(&entry.key);
            }
        }
        Ok(())
    }
}

pub(super) fn retain_catalogues(
    engine: &impl StorageEngine,
    catalogue_root: &AnchoredPath,
    source_control_sequence: u64,
    capture: CatalogueCapture,
) -> Result<(String, CatalogueManifestInventory)> {
    if engine.control().sequence()? != source_control_sequence {
        return Err(Error::Archive(
            "control journal changed while capturing catalogue state".into(),
        ));
    }
    let scopes = capture
        .scopes
        .into_iter()
        .map(|scope| {
            Ok(CatalogueScopeSnapshot {
                revision: engine.runtime().read_stamp(&scope)?.catalog_revision,
                scope,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let payload = CatalogueManifest {
        format_version: CATALOGUE_MANIFEST_VERSION,
        source_control_sequence,
        scopes,
        records: capture.records.into_values().collect(),
    };
    validate_catalogue_manifest(&payload)?;
    let inventory = catalogue_manifest_inventory(&payload)?;
    let stored = StoredCatalogueManifest {
        payload,
        payload_sha256: inventory.manifest_sha256.clone(),
    };
    let manifests = catalogue_root.join(CATALOGUE_MANIFEST_DIRECTORY);
    manifests.create_dir_all().map_err(backup_io)?;
    let file_name = format!("{}.rrd-catalogues.json", inventory.manifest_sha256);
    let retained_path = manifests.join(&file_name);
    if retained_path.try_exists().map_err(backup_io)? {
        let actual = load_catalogue_manifest(&retained_path)?;
        if actual != stored.payload {
            return Err(Error::Archive(
                "content-addressed catalogue manifest conflicts with retained payload".into(),
            ));
        }
    } else {
        let id = BACKUP_TEMP_ID.fetch_add(1, Ordering::Relaxed);
        let staging = manifests.join(format!(
            ".catalogue-manifest-{}-{id}.tmp",
            std::process::id()
        ));
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
        format!("{CATALOGUE_MANIFEST_DIRECTORY}/{file_name}"),
        inventory,
    ))
}

pub(super) fn catalogue_scope(key: &str) -> Result<Option<ScopeId>> {
    for family in CATALOGUE_CONTROL_FAMILIES {
        if key.len() > family.len() && control_family_covers(family, key) {
            return ScopeId::new(&key[family.len() + 1..])
                .map(Some)
                .map_err(Error::from);
        }
    }
    Ok(None)
}

pub(super) fn load_catalogue_manifest_for_entry(
    catalogue_root: &AnchoredPath,
    entry: &BackupEntry,
) -> Result<CatalogueManifest> {
    let file = entry.catalogue_manifest_file.as_deref().ok_or_else(|| {
        Error::Archive("application-complete backup has no catalogue manifest path".into())
    })?;
    let expected = entry.catalogue_manifest.as_ref().ok_or_else(|| {
        Error::Archive("application-complete backup has no catalogue manifest inventory".into())
    })?;
    let manifest = load_catalogue_manifest(&resolve_catalogue_manifest(catalogue_root, file)?)?;
    let actual = catalogue_manifest_inventory(&manifest)?;
    if &actual != expected {
        return Err(Error::Archive(format!(
            "backup {} catalogue manifest inventory does not match catalogue",
            entry.backup_id
        )));
    }
    Ok(manifest)
}

pub(super) fn load_catalogue_manifest(path: &AnchoredPath) -> Result<CatalogueManifest> {
    let bytes = read_manifest_bytes(path)?;
    let stored: StoredCatalogueManifest = serde_json::from_slice(&bytes)?;
    validate_catalogue_manifest(&stored.payload)?;
    let actual = digest::sha256_hex(&serde_json::to_vec(&stored.payload)?);
    if actual != stored.payload_sha256 {
        return Err(Error::Archive(
            "catalogue manifest payload digest does not match".into(),
        ));
    }
    Ok(stored.payload)
}

pub(super) fn catalogue_manifest_inventory(
    manifest: &CatalogueManifest,
) -> Result<CatalogueManifestInventory> {
    validate_catalogue_manifest(manifest)?;
    let payload_bytes = manifest.records.iter().try_fold(0u64, |total, record| {
        total
            .checked_add(record.value.len() as u64)
            .ok_or_else(|| Error::Archive("catalogue manifest bytes overflowed u64".into()))
    })?;
    Ok(CatalogueManifestInventory {
        format_version: manifest.format_version,
        manifest_sha256: digest::sha256_hex(&serde_json::to_vec(manifest)?),
        source_control_sequence: manifest.source_control_sequence,
        scope_count: manifest.scopes.len() as u64,
        record_count: manifest.records.len() as u64,
        payload_bytes,
    })
}

pub(super) fn validate_catalogue_manifest(manifest: &CatalogueManifest) -> Result<()> {
    if manifest.format_version != CATALOGUE_MANIFEST_VERSION {
        return Err(Error::Archive(format!(
            "unsupported catalogue manifest version {}",
            manifest.format_version
        )));
    }
    if manifest
        .scopes
        .windows(2)
        .any(|pair| pair[0].scope >= pair[1].scope)
        || manifest
            .records
            .windows(2)
            .any(|pair| pair[0].key >= pair[1].key)
    {
        return Err(Error::Archive(
            "catalogue manifest entries are not uniquely ordered".into(),
        ));
    }
    if manifest.records.len() > MAX_ARCHIVE_CATALOGUE_RECORDS {
        return Err(Error::Archive(
            "backup catalogue closure exceeds the v1 entry bound".into(),
        ));
    }
    let scope_revisions = manifest
        .scopes
        .iter()
        .map(|scope| (&scope.scope, scope.revision))
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut record_counts = std::collections::BTreeMap::<&ScopeId, u64>::new();
    for record in &manifest.records {
        if record.value.is_empty() || record.value.len() > 1024 * 1024 {
            return Err(Error::Archive(
                "catalogue manifest record violates control-state bounds".into(),
            ));
        }
        let parsed = catalogue_scope(&record.key)?.ok_or_else(|| {
            Error::Archive("catalogue manifest contains a non-catalogue key".into())
        })?;
        if parsed != record.scope || !scope_revisions.contains_key(&record.scope) {
            return Err(Error::Archive(
                "catalogue manifest key, scope, or inventory differs".into(),
            ));
        }
        *record_counts.entry(&record.scope).or_default() += 1;
    }
    for scope in &manifest.scopes {
        let records = record_counts.get(&scope.scope).copied().unwrap_or_default();
        if scope.revision < records || (scope.revision > 0 && records == 0) {
            return Err(Error::Archive(
                "catalogue scope revision cannot be reconstructed from its records".into(),
            ));
        }
    }
    Ok(())
}

pub(super) fn restore_catalogues(
    manifest: &CatalogueManifest,
    staging_root: &AnchoredPath,
    options: crate::DatabaseOptions,
) -> Result<()> {
    validate_catalogue_manifest(manifest)?;
    let engine = RrflowKvStore::open_existing_anchored(staging_root, options)?;
    for scope in &manifest.scopes {
        let records = manifest
            .records
            .iter()
            .filter(|record| record.scope == scope.scope)
            .collect::<Vec<_>>();
        let first = records.first().ok_or_else(|| {
            Error::Archive("catalogue revision has no materialized restore record".into())
        })?;
        let mut revision = engine.runtime().read_stamp(&scope.scope)?.catalog_revision;
        if revision > scope.revision {
            return Err(Error::Archive(format!(
                "restored catalogue revision {revision} exceeds {} for {}",
                scope.revision, scope.scope
            )));
        }
        for (ordinal, record) in records.iter().enumerate() {
            let existing = engine.control().get(&record.key)?;
            if (ordinal as u64) < revision {
                if existing.as_deref() != Some(record.value.as_slice()) {
                    return Err(Error::Archive(format!(
                        "partially restored catalogue record {} differs",
                        record.key
                    )));
                }
                continue;
            }
            if existing.is_some() {
                return Err(Error::Archive(format!(
                    "catalogue record {} exists ahead of its revision",
                    record.key
                )));
            }
            engine.control().commit_catalog(
                &scope.scope,
                &ControlTransition {
                    key: record.key.clone(),
                    expected: None,
                    replacement: Some(record.value.clone()),
                    at: 1,
                    actor: "rrd-restore".into(),
                    action: "catalogue.restored".into(),
                    request_id: "rrd-restore".into(),
                    operation_id: "rrd-restore".into(),
                },
            )?;
            revision += 1;
        }
        while revision < scope.revision {
            engine.control().commit_catalog(
                &scope.scope,
                &ControlTransition {
                    key: first.key.clone(),
                    expected: Some(first.value.clone()),
                    replacement: Some(first.value.clone()),
                    at: 1,
                    actor: "rrd-restore".into(),
                    action: "catalogue.revision_restored".into(),
                    request_id: "rrd-restore".into(),
                    operation_id: "rrd-restore".into(),
                },
            )?;
            revision += 1;
        }
    }
    drop(engine);
    verify_restored_catalogues(manifest, staging_root, options)
}

pub(super) fn verify_restored_catalogues(
    manifest: &CatalogueManifest,
    restored_root: &AnchoredPath,
    options: crate::DatabaseOptions,
) -> Result<()> {
    let engine = RrflowKvStore::open_existing_anchored(restored_root, options)?;
    for record in &manifest.records {
        if engine.control().get(&record.key)?.as_deref() != Some(record.value.as_slice()) {
            return Err(Error::Archive(format!(
                "restored catalogue record {} differs from its backup",
                record.key
            )));
        }
    }
    for scope in &manifest.scopes {
        let actual = engine.runtime().read_stamp(&scope.scope)?.catalog_revision;
        if actual != scope.revision {
            return Err(Error::Archive(format!(
                "restored catalogue revision {actual} differs from {} for {}",
                scope.revision, scope.scope
            )));
        }
    }
    Ok(())
}
