//! Loading, verifying and pruning the backup catalogue.

use super::*;

/// Loads the catalogue and verifies its payload digest. Archive bytes are
/// verified separately by `verify_backup_catalogue` so listing remains bounded.
///
/// A catalogue that was never written reads as empty. A catalogue file that
/// is absent while retained artifacts exist beside it fails closed: those
/// artifacts belonged to entries, and reading them as "no backups" would hide
/// the loss.
pub fn load_backup_catalogue(
    catalogue_root: &(impl AnchorInput + ?Sized),
) -> Result<BackupCatalogue> {
    let catalogue_root = &catalogue_root.anchor().map_err(backup_io)?;
    let path = catalogue_root.join(CATALOGUE_FILE);
    if !path.try_exists().map_err(backup_io)? {
        require_no_orphan_artifacts(catalogue_root)?;
        return catalogue_from_payload(empty_catalogue_payload());
    }
    let bytes = path.read().map_err(backup_io)?;
    let stored: StoredCatalogue = serde_json::from_slice(&bytes)?;
    if stored.payload.format_version != BACKUP_CATALOGUE_VERSION {
        return Err(Error::Archive(format!(
            "unsupported backup catalogue version {}",
            stored.payload.format_version
        )));
    }
    let actual = payload_digest(&stored.payload)?;
    if actual != stored.payload_sha256 {
        return Err(Error::Archive(
            "backup catalogue payload digest does not match".into(),
        ));
    }
    validate_entries(&stored.payload.backups)?;
    validate_prune_receipts(&stored.payload.prune_receipts)?;
    Ok(BackupCatalogue {
        format_version: stored.payload.format_version,
        revision: stored.payload.revision,
        catalogue_sha256: actual,
        backups: stored.payload.backups,
        prune_receipts: stored.payload.prune_receipts,
    })
}

/// Verifies the digests of the catalogue and of every retained artifact it
/// references, reading only: nothing is created or repaired. The digests
/// detect changed bytes; they do not authenticate a producer.
pub fn verify_backup_catalogue(
    catalogue_root: &(impl AnchorInput + ?Sized),
) -> Result<BackupCatalogue> {
    let catalogue_root = &catalogue_root.anchor().map_err(backup_io)?;
    let catalogue = load_backup_catalogue(catalogue_root)?;
    for entry in &catalogue.backups {
        let path = resolve_archive(catalogue_root, &entry.archive_file)?;
        let actual = inspect_logical_archive(&path)?;
        if actual != entry.archive {
            return Err(Error::Archive(format!(
                "backup {} archive inventory does not match catalogue",
                entry.backup_id
            )));
        }
        if entry.application_complete {
            load_catalogue_manifest_for_entry(catalogue_root, entry)?;
            load_control_archive_for_entry(catalogue_root, entry)?;
            verify_object_payloads(catalogue_root, entry)?;
        }
    }
    Ok(catalogue)
}

/// Publishes one digest-verified successor catalogue for a complete retention
/// partition, then reclaims only artifacts that no retained entry references.
/// Exact replay is recovered from the bounded receipt embedded in the
/// catalogue.
pub fn prune_backup_catalogue(
    catalogue_root: &(impl AnchorInput + ?Sized),
    plan: &BackupPrunePlan,
) -> Result<BackupPruneOutcome> {
    let catalogue_root = &catalogue_root.anchor().map_err(backup_io)?;
    validate_prune_plan(plan)?;
    let request_sha256 = digest::sha256_hex(&serde_json::to_vec(plan)?);
    let mut catalogue = verify_backup_catalogue(catalogue_root)?;
    if let Some(receipt) = catalogue
        .prune_receipts
        .iter()
        .find(|receipt| receipt.request_sha256 == request_sha256)
        .cloned()
    {
        cleanup_pruned_artifacts(catalogue_root, &receipt.removed_backups, &catalogue.backups)?;
        return Ok(BackupPruneOutcome {
            catalogue,
            pruned_backup_ids: sorted_backup_ids(
                receipt
                    .removed_backups
                    .iter()
                    .map(|entry| entry.backup_id.clone())
                    .collect(),
            ),
            idempotent_replay: true,
        });
    }
    if catalogue.catalogue_sha256 != plan.expected_catalogue_sha256 {
        return Err(Error::Archive(
            "backup prune expected catalogue digest is stale".into(),
        ));
    }
    if catalogue.prune_receipts.len() == MAX_BACKUP_PRUNE_RECEIPTS {
        return Err(Error::Archive(
            "backup prune receipt history is at its v1 bound".into(),
        ));
    }
    let current = catalogue
        .backups
        .iter()
        .map(|entry| entry.backup_id.clone())
        .collect::<BTreeSet<_>>();
    let retained = plan
        .retained_backup_ids
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let candidates = plan
        .prune_candidate_backup_ids
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    if !retained.is_disjoint(&candidates)
        || retained
            .union(&candidates)
            .cloned()
            .collect::<BTreeSet<_>>()
            != current
    {
        return Err(Error::Archive(
            "backup prune plan is not a complete disjoint catalogue partition".into(),
        ));
    }
    let removed_backups = catalogue
        .backups
        .iter()
        .filter(|entry| candidates.contains(&entry.backup_id))
        .cloned()
        .collect::<Vec<_>>();
    catalogue
        .backups
        .retain(|entry| retained.contains(&entry.backup_id));
    catalogue.revision = catalogue
        .revision
        .checked_add(1)
        .ok_or_else(|| Error::Archive("backup catalogue revision overflow".into()))?;
    catalogue.prune_receipts.push(BackupPruneReceipt {
        request_sha256,
        expected_catalogue_sha256: plan.expected_catalogue_sha256.clone(),
        retained_backup_ids: plan.retained_backup_ids.clone(),
        removed_backups: removed_backups.clone(),
    });
    write_catalogue(catalogue_root, catalogue)?;
    let catalogue = verify_backup_catalogue(catalogue_root)?;
    cleanup_pruned_artifacts(catalogue_root, &removed_backups, &catalogue.backups)?;
    Ok(BackupPruneOutcome {
        catalogue,
        pruned_backup_ids: plan.prune_candidate_backup_ids.clone(),
        idempotent_replay: false,
    })
}
