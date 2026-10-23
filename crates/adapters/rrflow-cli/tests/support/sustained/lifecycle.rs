//! Native forced-process reopen on the same installed/configured estate used
//! by offered load. Readback precedes retry, so replay cannot repair lost data.
use super::*;

/// One bounded management client overlaps offered writes. Archive verification
/// validates the actual saved bytes; segment readback checks all live ACKs.
pub(super) async fn backup(
    work: Workload,
    key: String,
    clock: Instant,
) -> DriverResult<BackupEvidence> {
    let started_ns = ns(clock);
    let receipt = work
        .client
        .create_backup(
            &work.session,
            rrd_contract::CreateInstanceBackup {
                label: key.clone(),
                created_at_unix_ms: work.valid_at,
            },
            RequestOptions::new(&key, &key, Some(&key), Some(u64::MAX)).unwrap(),
        )
        .await
        .map_err(|e| e.to_string())?;
    let finished_ns = ns(clock);
    let verified = work
        .client
        .list_backups(
            &work.session,
            rrd_contract::ListInstanceBackups {
                verify_archives: true,
            },
            RequestOptions::read("capacity-backup-verify", "capacity-backup-verify").unwrap(),
        )
        .await
        .map_err(|e| e.to_string())?;
    if !verified.archives_verified
        || !receipt.backup.application_complete
        || !verified
            .backups
            .iter()
            .any(|backup| backup == &receipt.backup)
    {
        return Err("saved archive did not match complete backup receipt".into());
    }
    Ok(BackupEvidence {
        started_ns,
        finished_ns,
        verified_ns: ns(clock),
        receipt,
        verification_catalogue_sha256: verified.catalogue_sha256.try_into()?,
    })
}

pub(super) async fn reopens(
    estate: &Estate,
    child: &mut Option<super::super::process::Process>,
    work: &mut Workload,
    campaign: &StorageCampaign,
    clock: Instant,
    lifetime: Duration,
    progress: &ProgressContext<'_>,
) -> DriverResult<ReopenEvidence> {
    tokio::time::timeout(
        Duration::from_millis(campaign.schedule.lifecycle_ms.get()),
        reopen_inner(estate, child, work, campaign, clock, lifetime, progress),
    )
    .await
    .map_err(|_| "installed reopen lane exceeded lifecycle deadline".to_owned())?
}

async fn reopen_inner(
    estate: &Estate,
    child: &mut Option<super::super::process::Process>,
    work: &mut Workload,
    campaign: &StorageCampaign,
    clock: Instant,
    lifetime: Duration,
    progress: &ProgressContext<'_>,
) -> DriverResult<ReopenEvidence> {
    let started_ns = ns(clock);
    let mut cycles = Vec::new();
    let mut receipts = Vec::new();
    for cycle in 0..campaign.schedule.forced_reopens.get() {
        let request = work.prepare(
            format!("lifecycle-reopen-{cycle}"),
            StorageLoadKind::MixedWrite,
            cycle,
        )?;
        progress
            .writer
            .record_event(&ProgressEvent::LifecycleIntent {
                point: progress.point.id.clone(),
                seed: work.seed,
                cycle,
                request_sha256: request.request_sha256.clone(),
                effect_sha256: request.effect_sha256.clone(),
            })?;
        progress.writer.checkpoint_progress()?;
        let acknowledged = work.execute(&request).await?;
        if acknowledged.idempotent_replay {
            return Err("fresh reopen operation unexpectedly replayed".into());
        }
        progress
            .writer
            .record_event(&ProgressEvent::LifecycleAcknowledged {
                point: progress.point.id.clone(),
                seed: work.seed,
                cycle,
                receipt_sha256: acknowledged.receipt_sha256.clone(),
            })?;
        progress.writer.checkpoint_progress()?;
        receipts.push(acknowledged.receipt_sha256);
        let killed = super::super::kill(child.take().ok_or("missing owned server")?, clock);
        let (reopened, client, identity) = estate.start_for(clock, lifetime);
        *child = Some(reopened);
        work.client = client;
        let mut expected = digest("c07g.reopen.expected", &work.seed)?;
        let mut original = digest("c07g.reopen.receipts", &work.seed)?;
        let mut replayed = original.clone();
        let mut readback = digest("c07g.reopen.readback", &work.seed)?;
        for prior in 0..=cycle {
            let operation = work.prepare(
                format!("lifecycle-reopen-{prior}"),
                StorageLoadKind::MixedWrite,
                prior,
            )?;
            // The old session and transaction identities survive reconnect.
            // Retry begins the exact same transaction, never a replacement.
            work.verify(&operation).await?;
            let receipt = work.execute(&operation).await?;
            if !receipt.idempotent_replay || receipt.receipt_sha256 != receipts[prior as usize] {
                return Err("reopen changed an acknowledged transaction receipt".into());
            }
            expected = digest(
                "c07g.reopen.expected",
                &(
                    expected,
                    prior,
                    &operation.request_sha256,
                    &operation.effect_sha256,
                ),
            )?;
            original = digest(
                "c07g.reopen.receipts",
                &(original, prior, &receipts[prior as usize]),
            )?;
            replayed = digest(
                "c07g.reopen.receipts",
                &(replayed, prior, receipt.receipt_sha256),
            )?;
            readback = digest(
                "c07g.reopen.readback",
                &(readback, prior, &operation.effect_sha256),
            )?;
        }
        cycles.push(ReopenObservation {
            cycle,
            killed_pid: killed["pid"]
                .as_u64()
                .ok_or("kill PID missing")?
                .try_into()
                .map_err(|_| "PID overflow")?,
            reopened_pid: child.as_ref().ok_or("reopened server missing")?.id(),
            killed_ns: killed["killed_ns"].as_u64().ok_or("kill time missing")?,
            ready_ns: identity["ready_ns"].as_u64().ok_or("ready time missing")?,
            finished_ns: ns(clock),
            checked_operations: cycle + 1,
            expected_sha256: expected,
            original_receipts_sha256: original,
            replay_receipts_sha256: replayed,
            readback_sha256: readback,
        });
        progress.writer.record_event(&ProgressEvent::Reopen {
            point: progress.point.id.clone(),
            seed: work.seed,
            observation: cycles.last().ok_or("missing reopen observation")?.clone(),
        })?;
        progress.writer.checkpoint_progress()?;
    }
    // Verify the entire pre-existing seeded corpus after the last kill. These
    // immutable seed keys were never rewritten by retry or the lifecycle lane.
    let mut readback = digest("c07g.reopened-corpus", &(work.seed, work.records))?;
    for record in 0..work.records {
        let operation = work.seed_record(record)?;
        work.verify(&operation).await?;
        readback = digest(
            "c07g.reopened-corpus",
            &(readback, record, &operation.effect_sha256),
        )?;
    }
    Ok(ReopenEvidence {
        started_ns,
        finished_ns: ns(clock),
        corpus_records_verified: work.records,
        corpus_readback_sha256: readback,
        cycles,
    })
}
