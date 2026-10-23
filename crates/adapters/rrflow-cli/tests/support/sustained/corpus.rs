//! Bounded ingestion and a complete second readback pass. No input/result list
//! grows with corpus size: each rolling digest retains one record at a time.
use super::*;

pub(super) async fn seed(
    estate: &Estate,
    work: &mut Workload,
    campaign: &StorageCampaign,
    progress: &ProgressContext<'_>,
    clock: Instant,
    ordinal: &mut u64,
    opened: &mut Instant,
) -> DriverResult<CorpusEvidence> {
    tokio::time::timeout(
        Duration::from_millis(campaign.schedule.setup_ms.get()),
        seed_inner(estate, work, campaign, progress, clock, ordinal, opened),
    )
    .await
    .map_err(|_| "corpus setup exceeded its frozen deadline".to_owned())?
}

async fn seed_inner(
    estate: &Estate,
    work: &mut Workload,
    campaign: &StorageCampaign,
    progress: &ProgressContext<'_>,
    clock: Instant,
    ordinal: &mut u64,
    opened: &mut Instant,
) -> DriverResult<CorpusEvidence> {
    let started_ns = ns(clock);
    // Bound a single generated record BEFORE its payload/vector allocation.
    if work.payload_bytes > 256 * 1024
        || work.edges_per_record > 64
        || work.vectors_per_record > 64
        || work.dimensions > 4096
        || work.query_budget.max_rows < u64::from(work.edges_per_record).max(16)
        || work.query_budget.max_output_bytes < (work.payload_bytes as u64 + 2048) * 2
    {
        return Err("corpus shape exceeds bounded single-record/readback admission".into());
    }
    let mut input = digest("c07g.corpus.input", &(work.seed, work.records))?;
    let mut acknowledgements = digest("c07g.corpus.acknowledgements", &input)?;
    let mut readback = digest("c07g.corpus.readback", &input)?;
    let mut encoded = 0u64;
    let operation_deadline = Duration::from_millis(campaign.schedule.operation_deadline_ms.get());
    work.initialize().await?;
    // Persist each bounded batch of intents before its first database effect.
    // ACK checkpoints bound the evidence-loss window without retaining the corpus.
    for first in (0..work.records).step_by(64) {
        let end = first.saturating_add(64).min(work.records);
        for record in first..end {
            let request = work.seed_record(record)?;
            let bytes =
                serde_json::to_vec(request.mutation.as_ref().ok_or("missing corpus mutation")?)
                    .map_err(|e| e.to_string())?
                    .len() as u64;
            if bytes > campaign.storage.budgets.atomic_batch_bytes.get() {
                return Err("corpus record exceeds frozen atomic batch bytes".into());
            }
            encoded = encoded
                .checked_add(bytes)
                .ok_or("encoded corpus overflow")?;
            input = digest(
                "c07g.corpus.input",
                &(input, record, &request.request_sha256),
            )?;
            progress.writer.record_event(&ProgressEvent::CorpusIntent {
                point: progress.point.id.clone(),
                seed: work.seed,
                ordinal: record,
                request_sha256: request.request_sha256,
                effect_sha256: request.effect_sha256,
            })?;
        }
        progress.writer.checkpoint_progress()?;
        for record in first..end {
            rotate_session(
                estate,
                work,
                campaign,
                progress.point.clients.get(),
                ordinal,
                opened,
            )
            .await?;
            let request = work.seed_record(record)?;
            let receipt = tokio::time::timeout(operation_deadline, work.execute(&request))
                .await
                .map_err(|_| "corpus write deadline")??;
            progress
                .writer
                .record_event(&ProgressEvent::CorpusAcknowledged {
                    point: progress.point.id.clone(),
                    seed: work.seed,
                    ordinal: record,
                    receipt_sha256: receipt.receipt_sha256.clone(),
                })?;
            acknowledgements = digest(
                "c07g.corpus.acknowledgements",
                &(acknowledgements, record, receipt.receipt_sha256),
            )?;
        }
        progress.writer.checkpoint_progress()?;
    }
    // Only a complete independent readback can attest the seeded allocation.
    // Expected values are regenerated, never learned from the database answer.
    for record in 0..work.records {
        rotate_session(
            estate,
            work,
            campaign,
            progress.point.clients.get(),
            ordinal,
            opened,
        )
        .await?;
        let request = work.seed_record(record)?;
        tokio::time::timeout(operation_deadline, work.verify(&request))
            .await
            .map_err(|_| "corpus readback deadline")??;
        progress
            .writer
            .record_event(&ProgressEvent::CorpusVerified {
                point: progress.point.id.clone(),
                seed: work.seed,
                ordinal: record,
                effect_sha256: request.effect_sha256.clone(),
            })?;
        if record % 64 == 63 {
            progress.writer.checkpoint_progress()?;
        }
        readback = digest(
            "c07g.corpus.readback",
            &(readback, record, &request.effect_sha256),
        )?;
    }
    progress.writer.checkpoint_progress()?;
    Ok(CorpusEvidence {
        records: work.records,
        anchor_records: 1,
        edges: work
            .records
            .checked_mul(u64::from(work.edges_per_record))
            .ok_or("edge count overflow")?,
        vectors: work
            .records
            .checked_mul(u64::from(work.vectors_per_record))
            .ok_or("vector count overflow")?,
        vector_dimensions: u64::from(work.dimensions),
        logical_source_bytes: work
            .records
            .checked_mul(work.payload_bytes as u64)
            .ok_or("source byte overflow")?,
        encoded_mutation_bytes: encoded,
        verified_records: work.records,
        started_ns,
        finished_ns: ns(clock),
        input_sha256: input,
        acknowledgement_sha256: acknowledgements,
        readback_sha256: readback,
    })
}
