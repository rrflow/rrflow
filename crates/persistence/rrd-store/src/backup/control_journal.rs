//! Replaying, verifying and retaining the control journal.

use super::*;

/// Applies the archived entries after the recorded progress. `stop_after`
/// ends the replay early without the completion marker; it exists so a test
/// can interrupt a replay the way a crash would and resume it.
pub(super) fn replay_control_entries(
    archive: &AnchoredPath,
    expected: &ControlArchiveInventory,
    engine: &RrflowKvStore,
    stop_after: Option<u64>,
) -> Result<()> {
    let mut progress = match control_restore_progress(engine)? {
        Some(progress) => {
            if progress.archive_sha256 != expected.archive_sha256
                || progress.entry_count != expected.entry_count
                || progress.applied > progress.entry_count
            {
                return Err(Error::Archive(
                    "restore target carries another control-journal replay".into(),
                ));
            }
            progress
        }
        None => ControlRestoreProgress {
            format_version: CONTROL_RESTORE_PROGRESS_FORMAT,
            archive_sha256: expected.archive_sha256.clone(),
            entry_count: expected.entry_count,
            applied: 0,
            complete: false,
            first_restored_sequence: None,
            last_restored_sequence: None,
        },
    };
    if progress.complete {
        return Ok(());
    }
    let resume_after = progress.applied;
    let mut ordinal = 0u64;
    read_control_archive(archive, |entry| {
        ordinal += 1;
        if ordinal <= resume_after || stop_after.is_some_and(|stop| ordinal > stop) {
            return Ok(());
        }
        let mut transaction = engine.begin_transaction()?;
        let record_key = keyspaces::control_record_key(&entry.key);
        let current = get(&*transaction, keyspaces::META, &record_key)?;
        if current.as_deref().map(digest::sha256_hex) != entry.before_sha256 {
            return Err(Error::Archive(format!(
                "control-journal replay diverged at source sequence {}: {} does not hold its recorded before-state",
                entry.sequence, entry.key
            )));
        }
        let transition = ControlTransition {
            key: entry.key.clone(),
            expected: current,
            replacement: entry.replacement.clone(),
            at: entry.at,
            actor: entry.actor.clone(),
            action: entry.action.clone(),
            request_id: entry.request_id.clone(),
            operation_id: entry.operation_id.clone(),
        };
        let committed =
            prepare_control_batch(&mut *transaction, std::slice::from_ref(&transition))?;
        let restored_sequence = committed
            .first()
            .map(|entry| entry.sequence)
            .ok_or_else(|| Error::Archive("control replay committed no entry".into()))?;
        progress.applied = ordinal;
        progress.complete = ordinal == expected.entry_count;
        progress
            .first_restored_sequence
            .get_or_insert(restored_sequence);
        progress.last_restored_sequence = Some(restored_sequence);
        transaction.put(
            checked_key(keyspaces::META, &keyspaces::control_restore_progress_key())?,
            serde_json::to_vec(&progress)?,
        )?;
        transaction.commit(Durability::Authoritative)?;
        Ok(())
    })?;
    if ordinal != expected.entry_count {
        return Err(Error::Archive(
            "control-journal archive entry count changed between inspection and replay".into(),
        ));
    }
    if stop_after.is_some_and(|stop| stop < expected.entry_count) {
        return Ok(());
    }
    if !progress.complete {
        // An empty scope replays nothing; the marker still records that the
        // archive was applied.
        progress.complete = true;
        let mut transaction = engine.begin_transaction()?;
        transaction.put(
            checked_key(keyspaces::META, &keyspaces::control_restore_progress_key())?,
            serde_json::to_vec(&progress)?,
        )?;
        transaction.commit(Durability::Authoritative)?;
    }
    Ok(())
}

/// Verifies that a restored root's control journal carries every archived
/// entry, contiguously and in order, with the same key, before-state, value,
/// time, actor, action, and identities. Nothing but the replay writes control
/// state into a staged root, so contiguity holds until publication.
pub(super) fn verify_restored_control_journal(
    archive: &AnchoredPath,
    expected: &ControlArchiveInventory,
    restored_root: &AnchoredPath,
    options: crate::DatabaseOptions,
) -> Result<()> {
    let engine = RrflowKvStore::open_existing_anchored(restored_root, options)?;
    let progress = control_restore_progress(&engine)?.ok_or_else(|| {
        Error::Archive("restored root carries no control-journal replay marker".into())
    })?;
    if progress.archive_sha256 != expected.archive_sha256
        || progress.entry_count != expected.entry_count
        || progress.applied != expected.entry_count
        || !progress.complete
    {
        return Err(Error::Archive(
            "restored root's control-journal replay is incomplete or belongs to another archive"
                .into(),
        ));
    }
    if expected.entry_count == 0 {
        if progress.first_restored_sequence.is_some() || progress.last_restored_sequence.is_some() {
            return Err(Error::Archive(
                "control-journal replay marker is inconsistent".into(),
            ));
        }
        return Ok(());
    }
    let first = progress
        .first_restored_sequence
        .ok_or_else(|| Error::Archive("control-journal replay marker is inconsistent".into()))?;
    if first == 0 || progress.last_restored_sequence != first.checked_add(expected.entry_count - 1)
    {
        return Err(Error::Archive(
            "control-journal replay marker is inconsistent".into(),
        ));
    }
    let mut page = Vec::new().into_iter();
    let mut pages = ControlJournalPages::new();
    let mut cursor = first - 1;
    let mut ordinal = 0u64;
    read_control_archive(archive, |entry| {
        let restored = match page.next() {
            Some(restored) => restored,
            None => {
                page = pages.read(&engine, cursor)?.into_iter();
                page.next().ok_or_else(|| {
                    Error::Archive("restored control journal ends before the archive".into())
                })?
            }
        };
        cursor = restored.sequence;
        ordinal += 1;
        if restored.sequence != first + ordinal - 1
            || restored.key != entry.key
            || restored.before_sha256 != entry.before_sha256
            || restored.replacement != entry.replacement
            || restored.at != entry.at
            || restored.actor != entry.actor
            || restored.action != entry.action
            || restored.request_id != entry.request_id
            || restored.operation_id != entry.operation_id
        {
            return Err(Error::Archive(format!(
                "restored control journal differs from the archive at source sequence {}",
                entry.sequence
            )));
        }
        Ok(())
    })?;
    Ok(())
}

/// Streams the control-journal entries of the declared families from the
/// start of the journal to `source_control_sequence` into a content-addressed
/// archive.
pub(super) fn retain_control_journal(
    engine: &impl StorageEngine,
    catalogue_root: &AnchoredPath,
    source_control_sequence: u64,
    families: &[String],
) -> Result<(String, ControlArchiveInventory, CatalogueCapture)> {
    let archives = catalogue_root.join(CONTROL_ARCHIVE_DIRECTORY);
    archives.create_dir_all().map_err(backup_io)?;
    let id = BACKUP_TEMP_ID.fetch_add(1, Ordering::Relaxed);
    let staging = archives.join(format!(".control-archive-{}-{id}.tmp", std::process::id()));
    let mut writer = ControlArchiveWriter::create(&staging, source_control_sequence, families)?;
    let mut catalogues = CatalogueCapture::default();
    let result = (|| {
        let mut after = 0u64;
        let mut pages = ControlJournalPages::new();
        while after < source_control_sequence {
            let page = pages.read(engine, after)?;
            if page.is_empty() {
                return Err(Error::Archive(
                    "control journal ended before the captured backup watermark".into(),
                ));
            }
            for entry in page {
                if entry.sequence != after + 1 || entry.sequence > source_control_sequence {
                    return Err(Error::Archive(
                        "control journal changed or became discontinuous during control capture"
                            .into(),
                    ));
                }
                after = entry.sequence;
                if control_family_of(families, &entry.key).is_some() {
                    writer.entry(&entry)?;
                }
                // Fold catalogue state from the same already-verified entry.
                // Excluded control families are still chain-validated, but
                // their payloads are not retained for a second traversal.
                catalogues.observe(entry)?;
            }
        }
        if engine.control().sequence()? != source_control_sequence {
            return Err(Error::Archive(
                "control journal changed while capturing control state".into(),
            ));
        }
        writer.finish()
    })();
    let inventory = match result {
        Ok(inventory) => inventory,
        Err(error) => {
            let _ = staging.remove_file();
            return Err(error);
        }
    };
    let file_name = format!("{}.rrd-control-archive", inventory.archive_sha256);
    let retained_path = archives.join(&file_name);
    if retained_path.try_exists().map_err(backup_io)? {
        let actual = inspect_control_archive(&retained_path)?;
        if actual != inventory {
            let _ = staging.remove_file();
            return Err(Error::Archive(
                "content-addressed control-journal archive conflicts with retained inventory"
                    .into(),
            ));
        }
        staging.remove_file().map_err(backup_io)?;
    } else {
        staging.rename_to(&retained_path).map_err(backup_io)?;
    }
    Ok((
        format!("{CONTROL_ARCHIVE_DIRECTORY}/{file_name}"),
        inventory,
        catalogues,
    ))
}
