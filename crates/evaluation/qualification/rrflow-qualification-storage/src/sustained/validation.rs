//! Validating a sustained segment, including backup evidence.

use super::*;

pub(super) fn validate_segment_with_backups(
    campaign: &StorageCampaign,
    segment: &SustainedSegment,
    backups: &mut BackupTracker,
) -> Result<OperatingPointResult> {
    require(
        segment.format_version == 1 && segment.campaign_sha256 == campaign.sha256()?,
        "segment campaign identity differs",
    )?;
    let point = campaign
        .points
        .iter()
        .find(|p| p.id == segment.point)
        .ok_or("unknown operating point")?;
    require(
        campaign.seeds.contains(&segment.seed)
            && segment.started_ns < segment.finished_ns
            && segment.finished_ns <= MAX_EXACT_INTEGER,
        "invalid segment seed/time",
    )?;
    require(
        digest("candidate", &segment.manifest.candidate)?
            == digest("candidate", &campaign.storage.candidate)?
            && matches!(
                segment.manifest.evidence_class,
                EvidenceClass::NativeCapacity
            )
            && segment.manifest.durability == campaign.storage.durability
            && segment.manifest.corpus_sha256 == campaign.storage.corpus_sha256
            && digest("configuration", &segment.manifest.installed_configuration)?
                == digest("configuration", &campaign.storage.installed_configuration)?
            && digest("bindings", &segment.manifest.bindings)?
                == digest("bindings", &campaign.storage.bindings)?
            && digest("environment", &segment.manifest.environment)?
                == digest("environment", &campaign.storage.environment)?
            && digest("budgets", &segment.manifest.budgets)?
                == digest("budgets", &campaign.storage.budgets)?,
        "segment changed candidate, environment or ceilings",
    )?;
    let report = validate_result(&segment.manifest, &segment.result)?;
    require(
        segment
            .result
            .cases
            .iter()
            .all(|case| case.seed == segment.seed),
        "segment changed the frozen workload seed",
    )?;
    require(
        report.contract_passed,
        "segment failed independent storage ledger",
    )?;
    require(
        (segment.phase == LoadPhase::Drain || !segment.attempts.is_empty())
            && (segment.phase != LoadPhase::Drain || segment.attempts.is_empty())
            && segment.attempts.len() as u64 <= campaign.evidence.segment_attempts.get()
            && !segment.resources.is_empty()
            && segment.resources.len() as u64 <= campaign.evidence.samples_per_segment.get()
            && segment.lanes.len() <= 5,
        "segment evidence missing or exceeds bound",
    )?;
    let mut summary = OperatingPointResult {
        point: point.id.clone(),
        seed: segment.seed,
        phase: segment.phase,
        elapsed_ns: segment.finished_ns - segment.started_ns,
        started_ns: segment.started_ns,
        finished_ns: segment.finished_ns,
        attempts: segment.attempts.len() as u64,
        acknowledged_transitions: [0; 4],
        first_sequence: segment.attempts.first().map_or(0, |a| a.sequence),
        next_sequence: 0,
        schedule_origin_ns: None,
        failures: 0,
        unresolved: 0,
        latency_over_budget: 0,
        maximum_scheduled_latency_ns: 0,
        maximum_resident_bytes: 0,
        missing_metrics: Vec::new(),
        lanes: Vec::new(),
        forced_reopens: 0,
        corpus: segment.corpus.clone(),
        ownership_baseline: None,
        drain_tail: Vec::new(),
    };
    if segment.phase == LoadPhase::Warmup && summary.first_sequence == 0 {
        let first = &segment.resources[0];
        require(
            segment
                .attempts
                .iter()
                .all(|attempt| first.elapsed_ns <= attempt.started_ns),
            "ownership baseline follows a warmup effect",
        )?;
        summary.ownership_baseline = Some(first.into());
    }
    if segment.phase == LoadPhase::Drain {
        let keep = campaign.drain.stable_samples.get() as usize;
        summary.drain_tail = segment
            .resources
            .iter()
            .skip(segment.resources.len().saturating_sub(keep))
            .map(super::super::DrainSample::from)
            .collect();
    }
    if let Some(corpus) = &segment.corpus {
        require(
            segment.phase == LoadPhase::Warmup && summary.first_sequence == 0,
            "corpus receipt must precede the first warmup segment",
        )?;
        corpus.validate(campaign, segment.started_ns)?;
    }
    if let Some(reopens) = &segment.reopens {
        require(
            segment.phase == LoadPhase::Warmup && summary.first_sequence == 0,
            "reopen receipt must precede first warmup segment",
        )?;
        reopens.validate(campaign, segment.started_ns)?;
        require(
            segment.lanes.iter().any(|lane| {
                lane.lane == CapacityLane::ForcedReopens
                    && lane.repetitions.get() == reopens.cycles.len() as u64
                    && digest("c07g.reopen-evidence", reopens).as_ref() == Ok(&lane.receipt_sha256)
            }),
            "reopen lane does not bind its raw observations",
        )?;
    } else {
        require(
            !segment
                .lanes
                .iter()
                .any(|lane| lane.lane == CapacityLane::ForcedReopens),
            "reopen lane omitted its raw observations",
        )?;
    }
    if let Some(cold) = &segment.cold_start {
        require(
            segment.phase == LoadPhase::Warmup
                && summary.first_sequence == 0
                && cold.started_ns <= cold.evicted_ns
                && cold.evicted_ns <= cold.ready_ns
                && cold.ready_ns <= cold.finished_ns
                && cold.finished_ns <= segment.started_ns
                && cold.finished_ns - cold.started_ns
                    <= campaign.schedule.lifecycle_ms.get() * NANOSECONDS_PER_MS
                && cold.files > 0
                && cold.files
                    <= campaign
                        .storage
                        .installed_configuration
                        .storage
                        .recovery_files
                && cold.file_bytes > 0
                && cold.file_bytes
                    <= campaign
                        .storage
                        .installed_configuration
                        .storage
                        .recovery_read_bytes
                && cold.pages > 0
                && cold.resident_pages == 0
                && cold.page_bytes.is_power_of_two()
                && (4096..=65536).contains(&cold.page_bytes)
                && (cold.pages as u128) * (cold.page_bytes as u128) >= cold.file_bytes as u128
                && (cold.pages as u128) * (cold.page_bytes as u128)
                    <= cold.file_bytes as u128 + cold.files as u128 * (cold.page_bytes - 1) as u128
                && segment.lanes.iter().any(|lane| {
                    lane.lane == CapacityLane::Cold
                        && lane.repetitions.get() == 1
                        && digest("c07g.cold-start", cold).as_ref() == Ok(&lane.receipt_sha256)
                }),
            "cold lane lacks verified eviction, bounded startup or matching readback",
        )?;
    } else {
        require(
            !segment
                .lanes
                .iter()
                .any(|lane| lane.lane == CapacityLane::Cold),
            "cold lane omitted its native residency evidence",
        )?;
    }
    if let Some(reader) = &segment.reader {
        reader.validate()?;
        require(
            reader.opened_ns >= segment.started_ns && reader.released_ns <= segment.finished_ns,
            "reader interference lies outside its segment",
        )?;
    }
    for lane in segment
        .lanes
        .iter()
        .filter(|lane| lane.lane == CapacityLane::ReaderPinInterference)
    {
        let reader = segment
            .reader
            .as_ref()
            .ok_or("reader lane omitted raw ownership evidence")?;
        require(
            reader.qualifies(&segment.attempts)
                && lane.repetitions.get() == 1
                && digest("c07g.reader-interference", reader)? == lane.receipt_sha256,
            "reader lane lacks overlapping ACK, maintenance or matching receipt",
        )?;
    }
    for lane in &segment.lanes {
        if lane.lane == CapacityLane::Warm {
            require(
                segment.phase == LoadPhase::Measured
                    && summary.first_sequence == 0
                    && digest("c07g.warm-readback", &segment.result)? == lane.receipt_sha256,
                "warm lane lacks the first measured independent ledger",
            )?;
        }
    }
    let mut last = None;
    let mut written_slots = BTreeSet::new();
    let rate = point.requests_per_second.get()
        * if segment.phase == LoadPhase::Burst {
            2
        } else {
            1
        };
    for attempt in &segment.attempts {
        let offset = (attempt.sequence as u128 * 1_000_000_000 / rate as u128)
            .try_into()
            .map_err(|_| "offered schedule overflow")?;
        let origin = attempt
            .scheduled_ns
            .checked_sub(offset)
            .ok_or("offered schedule precedes time origin")?;
        require(
            summary
                .schedule_origin_ns
                .is_none_or(|value| value == origin),
            "offered schedule changed or coordinated omission",
        )?;
        summary.schedule_origin_ns = Some(origin);
        require(
            attempt.kind == storage_load_kind(attempt.sequence)
                && attempt.sequence < MAX_EXACT_INTEGER
                && last.is_none_or(|n| attempt.sequence == n + 1)
                && attempt.scheduled_ns >= segment.started_ns
                && attempt.scheduled_ns <= attempt.started_ns
                && attempt.started_ns <= attempt.finished_ns
                && attempt.finished_ns <= segment.finished_ns,
            "attempt mix, sequence or monotonic interval differs",
        )?;
        if let Some(verified) = attempt.verification_finished_ns {
            require(
                verified >= attempt.finished_ns && verified <= segment.finished_ns,
                "verification completion is outside the response/segment interval",
            )?;
        }
        match attempt.write {
            Some(write) => {
                require(
                    attempt.kind == StorageLoadKind::MixedWrite
                        && write.slot < campaign.evidence.segment_attempts.get()
                        && written_slots.insert(write.slot)
                        && write.generation <= MAX_EXACT_INTEGER,
                    "write transition exceeds frozen slot admission or repeats within segment",
                )?;
                if attempt.outcome == LoadOutcome::Acknowledged {
                    summary.acknowledged_transitions[write.counter_index()] += 1;
                }
            }
            None => require(
                attempt.kind != StorageLoadKind::MixedWrite,
                "mixed write omitted its slot transition",
            )?,
        }
        let recorded = segment
            .result
            .cases
            .iter()
            .flat_map(|case| &case.ledger.attempts)
            .find(|record| {
                record.logical_id == attempt.logical_id
                    && record.request_sha256 == attempt.request_sha256
                    && record.started_ns == attempt.started_ns - segment.started_ns
                    && record.finished_ns == attempt.finished_ns - segment.started_ns
            })
            .ok_or("scheduled attempt missing from independent ledger")?;
        require(
            match (&attempt.outcome, &recorded.outcome) {
                (LoadOutcome::Acknowledged, AttemptOutcome::Acknowledged { receipt_sha256 }) => {
                    receipt_sha256 == &attempt.result_sha256
                }
                (
                    LoadOutcome::Rejected | LoadOutcome::Cancelled,
                    AttemptOutcome::Rejected { .. },
                ) => true,
                (
                    LoadOutcome::Uncertain | LoadOutcome::Failed,
                    AttemptOutcome::Uncertain { .. },
                ) => true,
                _ => false,
            },
            "scheduled attempt outcome differs from ledger",
        )?;
        last = Some(attempt.sequence);
        summary.next_sequence = attempt.sequence + 1;
        if attempt.finished_ns - attempt.scheduled_ns > point.maximum_p99_ns.get() {
            summary.latency_over_budget += 1;
        }
        summary.maximum_scheduled_latency_ns = summary
            .maximum_scheduled_latency_ns
            .max(attempt.finished_ns - attempt.scheduled_ns);
        if attempt.outcome != LoadOutcome::Acknowledged {
            summary.failures += 1;
        }
        if attempt.outcome == LoadOutcome::Uncertain {
            summary.unresolved += 1;
        }
        require(
            attempt.finished_ns - attempt.started_ns
                <= campaign.schedule.operation_deadline_ms.get() * NANOSECONDS_PER_MS,
            "operation exceeded frozen deadline",
        )?;
    }
    let mut missing = BTreeSet::new();
    let mut previous = segment.started_ns;
    let mut allocation_identity = None;
    let sample_bound = campaign.schedule.sample_ms.get() * NANOSECONDS_PER_MS * 2;
    for sample in &segment.resources {
        if let Some(receipt) = &sample.allocation_receipt {
            receipt.validate(campaign)?;
            require(
                digest("allocation-enforcement", &sample.allocation_enforcement)?
                    == digest("allocation-enforcement", &receipt.enforcement()?)?,
                "native allocation receipt digest differs",
            )?;
            let identity = receipt.sha256()?;
            require(
                allocation_identity
                    .as_ref()
                    .is_none_or(|prior| prior == &identity),
                "native allocation changed within measured segment",
            )?;
            allocation_identity = Some(identity);
        } else {
            require(
                matches!(
                    sample.allocation_enforcement,
                    AllocationEnforcement::ObservedOnly
                ),
                "native enforcement claim omitted its launcher receipt",
            )?;
        }
        require(
            sample.pid > 0
                && sample.elapsed_ns >= previous
                && sample.elapsed_ns <= segment.finished_ns
                && sample.elapsed_ns - previous <= sample_bound,
            "resource series has a gap or invalid process identity",
        )?;
        previous = sample.elapsed_ns;
        for (name, metric) in [
            ("resident", &sample.resident_bytes),
            ("private", &sample.private_bytes),
            ("mapped", &sample.mapped_bytes),
            ("host-resident", &sample.host_resident_bytes),
            ("read-io", &sample.read_bytes),
            ("write-io", &sample.written_bytes),
            ("cpu", &sample.cpu_time_ns),
            ("retained", &sample.retained_bytes),
            ("queue", &sample.queue_requests),
            ("maintenance-debt", &sample.maintenance_debt_segments),
        ] {
            match metric.observed() {
                Some(value) => {
                    require(value <= MAX_EXACT_INTEGER, "resource exceeds exact integer")?
                }
                None => {
                    missing.insert(name.to_owned().try_into()?);
                }
            }
        }
        if matches!(
            sample.allocation_enforcement,
            AllocationEnforcement::ObservedOnly
        ) {
            missing.insert("native-allocation-enforcement".to_owned().try_into()?);
        }
        if let Some(rss) = sample.resident_bytes.observed() {
            require(
                rss <= campaign.allocation.engine_bytes.get(),
                "engine RSS ceiling exceeded",
            )?;
            summary.maximum_resident_bytes = summary.maximum_resident_bytes.max(rss);
        }
        if let Some(host) = sample.host_resident_bytes.observed() {
            require(
                host <= campaign.allocation.host_bytes.get(),
                "host ceiling exceeded",
            )?;
        }
        if let Some(retained) = sample.retained_bytes.observed() {
            require(
                retained <= campaign.storage.budgets.retained_bytes.get(),
                "retained-owner ceiling exceeded",
            )?;
        }
        if let Some(queue) = sample.queue_requests.observed() {
            require(
                queue <= campaign.storage.budgets.queue_requests.get(),
                "queue ceiling exceeded",
            )?;
        }
        if let Some(storage) = &sample.storage {
            storage.validate().map_err(|e| e.to_string())?;
            if let Some(admission) = &storage.admission {
                let limits = &campaign.storage.installed_configuration.storage;
                require(
                    admission.checkpoint_limit == limits.checkpoint_entries
                        && admission.checkpoint_entries <= limits.checkpoint_entries
                        && admission.native_transactions <= limits.native_transactions
                        && admission.native_transaction_bytes <= limits.native_transaction_bytes
                        && admission.active_read_views <= limits.active_read_views
                        && admission.pinned_read_bytes <= limits.pinned_read_bytes,
                    "physical owners exceed or differ from installed retention admission",
                )?;
            } else {
                missing.insert("physical-read-ownership".to_owned().try_into()?);
            }
        } else {
            missing.insert("engine-storage-snapshot".to_owned().try_into()?);
        }
    }
    require(
        segment.finished_ns - previous <= sample_bound,
        "resource series ends before measured interval",
    )?;
    summary.missing_metrics = missing.into_iter().collect();
    let mut lanes = BTreeSet::new();
    for lane in &segment.lanes {
        require(
            lanes.insert(lane.lane) && lane.acknowledged_readback_verified,
            "lane is duplicated or lacks exact readback",
        )?;
        if lane.lane == CapacityLane::ForcedReopens {
            summary.forced_reopens = lane.repetitions.get();
        }
    }
    summary.lanes = lanes.into_iter().collect();
    backups.observe(campaign, segment)?;
    Ok(summary)
}
