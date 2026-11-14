//! Merging summaries, qualification requirements, bounded IO and campaign verification.

use super::*;

pub(super) fn merge_summary(
    totals: &mut BTreeMap<(Id, u64, LoadPhase), OperatingPointResult>,
    value: OperatingPointResult,
    drain_samples: usize,
) -> Result<()> {
    let key = (value.point.clone(), value.seed, value.phase);
    if let Some(total) = totals.get_mut(&key) {
        require(
            value.ownership_baseline.is_none(),
            "ownership baseline repeated",
        )?;
        // Keep only the frozen finite tail, including missing observations.
        // Every segment uses the same policy; short segments accumulate until
        // the maximum 64-sample window is available.
        total.drain_tail.extend(value.drain_tail.iter().cloned());
        if total.drain_tail.len() > drain_samples {
            total
                .drain_tail
                .drain(..total.drain_tail.len() - drain_samples);
        }
        require(
            value.corpus.is_none(),
            "corpus receipt repeated within operating point",
        )?;
        require(
            total.next_sequence == value.first_sequence
                && total.schedule_origin_ns == value.schedule_origin_ns,
            "operating point omitted/repeated attempts or changed offered schedule",
        )?;
        total.next_sequence = value.next_sequence;
        require(
            value.started_ns >= total.started_ns && value.finished_ns >= total.finished_ns,
            "operating point monotonic interval moved backwards",
        )?;
        total.finished_ns = value.finished_ns;
        total.elapsed_ns = total.finished_ns - total.started_ns;
        for (a, b) in [
            (&mut total.attempts, value.attempts),
            (&mut total.failures, value.failures),
            (&mut total.unresolved, value.unresolved),
            (&mut total.latency_over_budget, value.latency_over_budget),
            (&mut total.forced_reopens, value.forced_reopens),
        ] {
            *a = a
                .checked_add(b)
                .filter(|n| *n <= MAX_EXACT_INTEGER)
                .ok_or("campaign aggregate overflow")?;
        }
        for (total, count) in total
            .acknowledged_transitions
            .iter_mut()
            .zip(value.acknowledged_transitions)
        {
            *total = total
                .checked_add(count)
                .filter(|n| *n <= MAX_EXACT_INTEGER)
                .ok_or("transition count overflow")?;
        }
        total.maximum_scheduled_latency_ns = total
            .maximum_scheduled_latency_ns
            .max(value.maximum_scheduled_latency_ns);
        total.maximum_resident_bytes = total
            .maximum_resident_bytes
            .max(value.maximum_resident_bytes);
        total.missing_metrics.extend(value.missing_metrics);
        total.missing_metrics.sort();
        total.missing_metrics.dedup();
        total.lanes.extend(value.lanes);
        total.lanes.sort();
        total.lanes.dedup();
    } else {
        require(
            value.first_sequence == 0,
            "operating point does not start at sequence zero",
        )?;
        totals.insert(key, value);
    }
    Ok(())
}

pub(super) fn validate_next(
    campaign: &StorageCampaign,
    segment: &SustainedSegment,
    index: u64,
    previous: &Option<Digest>,
    last_finished: u64,
    totals: &BTreeMap<(Id, u64, LoadPhase), OperatingPointResult>,
    backups: &BackupTracker,
) -> Result<(OperatingPointResult, BackupTracker)> {
    require(
        segment.index == index
            && &segment.previous_sha256 == previous
            && segment.finished_ns >= last_finished,
        "segment chain reordered or changed",
    )?;
    let mut next_backups = backups.clone();
    let summary = validate_segment_with_backups(campaign, segment, &mut next_backups)?;
    // Fixed-count aggregates, bounded to 32 points * 32 seeds * five phases.
    // Validate on a copy before publishing evidence or mutating totals.
    merge_summary(
        &mut totals.clone(),
        summary.clone(),
        campaign.drain.stable_samples.get() as usize,
    )?;
    Ok((summary, next_backups))
}

pub(super) fn qualifies(campaign: &StorageCampaign, result: &CampaignResult) -> Result<bool> {
    if campaign.purpose != CampaignPurpose::Qualification || campaign.allocation.estates.get() != 1
    {
        return Ok(false);
    }
    satisfies_operating_requirements(campaign, result)
}

pub(in super::super) fn satisfies_operating_requirements(
    campaign: &StorageCampaign,
    result: &CampaignResult,
) -> Result<bool> {
    if result.status != CampaignStatus::Completed
        || !result.limitations.is_empty()
        || result.segments == 0
        || result.failed_segment.is_some()
    {
        return Ok(false);
    }
    for point in &campaign.points {
        for &seed in &campaign.seeds {
            if !satisfies_point_requirements(
                campaign,
                point,
                seed,
                result.operating_points.iter(),
                &phases(),
            )? {
                return Ok(false);
            }
        }
    }
    Ok(true)
}

pub(super) fn satisfies_point_requirements<'a>(
    campaign: &StorageCampaign,
    point: &OperatingPoint,
    seed: u64,
    rows: impl Iterator<Item = &'a OperatingPointResult> + Clone,
    required_phases: &[LoadPhase],
) -> Result<bool> {
    let mut lanes = BTreeSet::new();
    let mut reopens = 0u64;
    let mut transitions = [false; 4];
    for &phase in required_phases {
        let Some(row) = rows
            .clone()
            .find(|r| r.point == point.id && r.seed == seed && r.phase == phase)
        else {
            return Ok(false);
        };
        if row.elapsed_ns < campaign.schedule.duration_ms(phase) * NANOSECONDS_PER_MS
            || (phase == LoadPhase::Warmup && row.corpus.is_none())
            || row.unresolved != 0
            || !row.missing_metrics.is_empty()
            || (phase != LoadPhase::Drain && row.attempts == 0)
            || row.latency_over_budget as u128 * 100 > row.attempts as u128
            || (phase != LoadPhase::Drain
                && (row.attempts as u128)
                    < (campaign.schedule.duration_ms(phase) as u128
                        * point.requests_per_second.get() as u128
                        * if phase == LoadPhase::Burst { 2 } else { 1 })
                    .div_ceil(1000))
            || row.failures as u128 * 10_000
                > row.attempts as u128 * point.maximum_error_basis_points as u128
        {
            return Ok(false);
        }
        for (seen, count) in transitions.iter_mut().zip(row.acknowledged_transitions) {
            *seen |= count > 0;
        }
        lanes.extend(row.lanes.iter().copied());
        reopens = reopens
            .checked_add(row.forced_reopens)
            .ok_or("reopen count overflow")?;
    }
    if !campaign
        .required_lanes
        .iter()
        .all(|lane| lanes.contains(lane))
        || reopens < campaign.schedule.forced_reopens.get()
        || transitions.iter().any(|seen| !seen)
    {
        return Ok(false);
    }
    Ok(true)
}

pub(in super::super) fn encode_bounded(value: &impl Serialize, limit: u64) -> Result<Vec<u8>> {
    struct Capped {
        bytes: Vec<u8>,
        limit: u64,
    }
    impl Write for Capped {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            if self.bytes.len() as u64 + b.len() as u64 > self.limit {
                return Err(std::io::Error::other("evidence encoding exceeds bound"));
            }
            self.bytes.extend_from_slice(b);
            Ok(b.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut output = Capped {
        bytes: Vec::new(),
        limit,
    };
    serde_json::to_writer(&mut output, value).map_err(|e| e.to_string())?;
    Ok(output.bytes)
}
pub(in super::super) fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|e| e.to_string())
}
pub(in super::super) fn read_bounded(path: &Path, limit: u64) -> Result<Vec<u8>> {
    let metadata = std::fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    require(
        metadata.is_file() && !metadata.file_type().is_symlink() && metadata.len() <= limit,
        "evidence is not a bounded regular file",
    )?;
    let mut bytes = Vec::new();
    File::open(path)
        .map_err(|e| e.to_string())?
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    require(
        bytes.len() as u64 == metadata.len(),
        "evidence changed while reading",
    )?;
    Ok(bytes)
}

/// Stream the declared sequence from disk; never enumerate/retain a day of
/// documents. A missing/altered/extra indexed segment or false completion fails.
pub fn verify_campaign(directory: &Path) -> Result<CampaignResult> {
    let campaign: StorageCampaign = decode(&read_bounded(
        &directory.join("campaign.json"),
        MAX_DOCUMENT_BYTES,
    )?)?;
    let identity = campaign.sha256()?;
    let result_bytes = read_bounded(&directory.join("result.json"), MAX_DOCUMENT_BYTES)?;
    let result: CampaignResult = decode(&result_bytes)?;
    let mut progress_audit = super::super::progress::verify_progress(
        directory,
        &result.progress,
        &campaign,
        result.status,
    )?;
    require(
        result.format_version == 1
            && result.campaign_sha256 == identity
            && result.segments <= campaign.evidence.segments.get()
            && result.limitations.len() <= MAX_CASES
            && result.operating_points.len() <= campaign.points.len() * campaign.seeds.len() * 5,
        "campaign result identity/count differs",
    )?;
    let mut previous = None;
    let mut last_finished = 0;
    let mut totals = BTreeMap::new();
    let mut backups = BackupTracker::default();
    let mut bytes = std::fs::metadata(directory.join("campaign.json"))
        .map_err(|e| e.to_string())?
        .len()
        .checked_add(result.progress.bytes)
        .ok_or("progress artifact byte overflow")?;
    for index in 0..result.segments {
        let raw = read_bounded(
            &directory.join(format!("segment-{index:020}.json")),
            campaign.evidence.segment_bytes.get(),
        )?;
        bytes = bytes
            .checked_add(raw.len() as u64)
            .ok_or("artifact byte overflow")?;
        require(
            bytes <= campaign.evidence.total_bytes.get(),
            "campaign artifact quota exceeded",
        )?;
        let segment: SustainedSegment = decode(&raw)?;
        progress_audit.bind_segment(&segment)?;
        let (summary, next_backups) = validate_next(
            &campaign,
            &segment,
            index,
            &previous,
            last_finished,
            &totals,
            &backups,
        )?;
        backups = next_backups;
        merge_summary(
            &mut totals,
            summary,
            campaign.drain.stable_samples.get() as usize,
        )?;
        previous = Some(digest("rrflow-storage-segment-v1", &segment)?);
        last_finished = segment.finished_ns;
    }
    progress_audit.require_bound(result.status)?;
    if let Some(failed) = &result.failed_segment {
        require(
            matches!(
                result.status,
                CampaignStatus::Failed | CampaignStatus::Interrupted
            ),
            "failed segment reported complete",
        )?;
        let raw = read_bounded(
            &directory.join("failed-segment.json"),
            campaign.evidence.segment_bytes.get(),
        )?;
        bytes = bytes
            .checked_add(raw.len() as u64)
            .ok_or("artifact byte overflow")?;
        let segment: SustainedSegment = decode(&raw)?;
        require(
            digest("rrflow-storage-segment-v1", &segment)? == failed.sha256,
            "failed operating point digest differs",
        )?;
        let reason = validate_next(
            &campaign,
            &segment,
            result.segments,
            &previous,
            last_finished,
            &totals,
            &backups,
        )
        .err()
        .ok_or("failed segment no longer fails validation")?;
        require(
            reason == failed.reason,
            "failed operating point reason differs",
        )?;
    } else {
        require(
            !directory.join("failed-segment.json").exists(),
            "result omitted failed operating point",
        )?;
    }
    require(
        bytes
            .checked_add(result_bytes.len() as u64 + MAX_LAUNCH_RECEIPT_BYTES)
            .is_some_and(|n| n <= campaign.evidence.total_bytes.get()),
        "campaign total evidence quota exceeded",
    )?;
    require(
        !directory
            .join(format!("segment-{:020}.json", result.segments))
            .exists(),
        "result omitted a written segment",
    )?;
    require(
        bytes == result.artifact_bytes
            && previous == result.last_segment_sha256
            && digest("point-results", &totals.into_values().collect::<Vec<_>>())?
                == digest("point-results", &result.operating_points)?,
        "result aggregates or segment digest differ",
    )?;
    if result.status == CampaignStatus::Completed {
        backups.require_settled()?;
        super::super::drain::check_completed(&campaign, &result.operating_points)?;
    }
    require(
        result.qualification_passed == qualifies(&campaign, &result)?,
        "campaign qualification claim differs from evidence",
    )?;
    Ok(result)
}
