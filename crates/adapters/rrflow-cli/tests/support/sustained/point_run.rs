//! Point runs, readback failures and failed campaign causes.

use super::*;

impl PointRun<'_> {
    async fn sample(&self, writer: &CampaignWriter) -> DriverResult<ResourceSample> {
        let value = sample(&self.workload, self.pid, self.clock, &self.allocation).await?;
        Self::record_sample(writer, value)
    }
    fn record_sample(
        writer: &CampaignWriter,
        value: ResourceSample,
    ) -> DriverResult<ResourceSample> {
        writer.record_event(&ProgressEvent::Resource {
            segment: writer.next_index(),
            sample: Box::new(value.clone()),
        })?;
        Ok(value)
    }
    fn start_sample(&self, task: &mut sample_task::SampleTask<ResourceSample>) -> DriverResult<()> {
        self.start_sample_at(task, None)
    }
    fn start_sample_at(
        &self,
        task: &mut sample_task::SampleTask<ResourceSample>,
        not_before: Option<Instant>,
    ) -> DriverResult<()> {
        let work = self.workload.clone();
        let pid = self.pid;
        let clock = self.clock;
        let allocation = self.allocation.clone();
        let gate = self.sample_gate.clone();
        task.start(async move {
            if let Some(not_before) = not_before {
                tokio::time::sleep_until(not_before.into()).await;
            }
            if let Some(gate) = gate {
                gate.wait().await;
            }
            sample(&work, pid, clock, &allocation).await
        })
    }
    pub(super) async fn drain(
        &self,
        writer: &CampaignWriter,
        duration_ms: u64,
    ) -> DriverResult<SustainedSegment> {
        if self.campaign.idle_segment_ms(duration_ms)? != duration_ms {
            return Err("idle duration exceeds its frozen segment admission".into());
        }
        let result = tokio::time::timeout(
            Duration::from_millis(self.campaign.storage.budgets.run_ms.get().min(180_000)),
            self.drain_inner(writer, duration_ms),
        )
        .await
        .map_err(|_| "idle segment exceeded its frozen deadline".to_owned())
        .and_then(std::convert::identity);
        let cleanup = self.workload.native_worker.shutdown().await;
        match (result, cleanup) {
            (Ok(value), Ok(())) => Ok(value),
            (Err(error), Ok(())) | (Ok(_), Err(error)) => Err(error),
            (Err(error), Err(cleanup)) => Err(format!("{error}; native worker cleanup: {cleanup}")),
        }
    }

    async fn drain_inner(
        &self,
        writer: &CampaignWriter,
        duration_ms: u64,
    ) -> DriverResult<SustainedSegment> {
        let started = ns(self.clock);
        let baseline =
            self.workload
                .prepare("drain-baseline".into(), StorageLoadKind::PointRead, 0)?;
        let expected = ExpectedOperation {
            logical_id: evidence_id(&baseline.id),
            request_sha256: baseline.request_sha256.clone(),
            effect_sha256: baseline.effect_sha256.clone(),
            resolution: ExpectedResolution::Present,
        };
        let key = evidence_id(format!("drain-{}", writer.next_index()));
        let mut manifest = self.campaign.storage.clone();
        manifest.required_cases = vec![RequiredCase {
            id: key.clone(),
            operation: evidence_id("transaction-commit"),
            seed: self.workload.seed,
            expected_ledger_sha256: expected_digest(std::slice::from_ref(&expected))?,
        }];
        let mut resources = Vec::new();
        let deadline =
            self.clock + Duration::from_nanos(started) + Duration::from_millis(duration_ms);
        let period = Duration::from_millis(self.campaign.schedule.sample_ms.get());
        let mut next_sample = Instant::now();
        loop {
            resources.push(self.sample(writer).await?);
            if Instant::now() >= deadline {
                break;
            }
            if resources.len() as u64 >= self.campaign.evidence.samples_per_segment.get() {
                return Err("drain resource sample cap exhausted".into());
            }
            next_sample =
                resource_window::next_sample_deadline(next_sample, period, Instant::now())?;
            tokio::time::sleep_until(next_sample.min(deadline).into()).await;
        }
        writer.record_event(&ProgressEvent::LoadIntent {
            segment: writer.next_index(),
            point: self.point.id.clone(),
            seed: self.workload.seed,
            phase: LoadPhase::Drain,
            expected: expected.clone(),
            write: None,
        })?;
        writer.checkpoint_progress()?;
        let probe_started = ns(self.clock);
        let (receipt, probe_finished) = tokio::time::timeout(
            Duration::from_millis(self.campaign.schedule.operation_deadline_ms.get()),
            async {
                let receipt = self.workload.execute(&baseline).await?;
                let finished = ns(self.clock);
                self.workload.verify(&baseline).await?;
                Ok::<_, String>((receipt, finished))
            },
        )
        .await
        .map_err(|_| "drain probe deadline")??;
        let ledger = AcknowledgementLedger {
            expected: vec![expected],
            // This is the actual drain probe, not offered load or throughput.
            attempts: vec![Attempt {
                attempt_id: evidence_id(format!("drain-probe-{}", writer.next_index())),
                logical_id: evidence_id(&baseline.id),
                request_sha256: baseline.request_sha256.clone(),
                started_ns: probe_started - started,
                finished_ns: probe_finished - started,
                outcome: AttemptOutcome::Acknowledged {
                    receipt_sha256: receipt.receipt_sha256.clone(),
                },
            }],
            readback: vec![Readback::Present {
                logical_id: evidence_id(baseline.id),
                request_sha256: baseline.request_sha256,
                effect_sha256: baseline.effect_sha256.clone(),
                receipt_sha256: receipt.receipt_sha256,
                applications: 1u64.try_into().unwrap(),
            }],
        };
        writer.record_event(&ProgressEvent::Readback {
            segment: writer.next_index(),
            observation: ledger.readback[0].clone(),
        })?;
        writer.checkpoint_progress()?;
        let result = StorageResult {
            format_version: 1,
            manifest_sha256: manifest_digest(&manifest)?,
            cases: vec![CaseResult {
                id: key,
                seed: self.workload.seed,
                status: CaseStatus::Executed,
                faults: vec![],
                limitation: None,
                ledger,
            }],
        };
        Ok(SustainedSegment {
            format_version: 1,
            campaign_sha256: writer.identity().clone(),
            index: writer.next_index(),
            previous_sha256: writer.previous_sha256(),
            point: self.point.id.clone(),
            seed: self.workload.seed,
            phase: LoadPhase::Drain,
            started_ns: started,
            finished_ns: ns(self.clock),
            corpus: None,
            cold_start: None,
            reopens: None,
            backup_started_ns: None,
            backup: None,
            reader: None,
            manifest,
            result,
            attempts: vec![],
            resources,
            lanes: vec![],
        })
    }
    pub(super) async fn segment(
        &mut self,
        writer: &CampaignWriter,
        phase: LoadPhase,
        origin: u64,
        first: u64,
        count: u64,
        final_segment: bool,
    ) -> DriverResult<SustainedSegment> {
        let campaign = self.campaign;
        let rate =
            self.point.requests_per_second.get() * if phase == LoadPhase::Burst { 2 } else { 1 };
        let scheduled =
            |sequence: u64| origin + (sequence as u128 * 1_000_000_000 / rate as u128) as u64;
        let started = if phase == LoadPhase::Drain {
            ns(self.clock)
        } else {
            scheduled(first)
        };
        let key = format!(
            "point-{}-seed-{}-segment-{}",
            self.point.id.as_str(),
            self.workload.seed,
            writer.next_index()
        );
        let mut prepared = Vec::new();
        let mut ledger = AcknowledgementLedger::default();
        // This complete expected ledger is frozen before any segment operation.
        for offset in 0..count {
            let sequence = first + offset;
            let name = format!("load-{}-{sequence}", writer.next_index());
            let kind = storage_load_kind(sequence);
            let operation = if kind == StorageLoadKind::MixedWrite {
                self.workload.prepare_load(
                    name,
                    WriteTransition {
                        slot: offset,
                        generation: self.generations[offset as usize],
                    },
                )?
            } else {
                self.workload.prepare(name, kind, sequence)?
            };
            ledger.expected.push(ExpectedOperation {
                logical_id: evidence_id(&operation.id),
                request_sha256: operation.request_sha256.clone(),
                effect_sha256: operation.effect_sha256.clone(),
                resolution: ExpectedResolution::Either,
            });
            prepared.push(operation);
        }
        for (expected, operation) in ledger.expected.iter().zip(&prepared) {
            writer.record_event(&ProgressEvent::LoadIntent {
                segment: writer.next_index(),
                point: self.point.id.clone(),
                seed: self.workload.seed,
                phase,
                expected: expected.clone(),
                write: operation.write,
            })?;
        }
        writer.checkpoint_progress()?;
        let mut manifest = campaign.storage.clone();
        manifest.required_cases = vec![RequiredCase {
            id: evidence_id(&key),
            operation: evidence_id("transaction-commit"),
            seed: self.workload.seed,
            expected_ledger_sha256: expected_digest(&ledger.expected)?,
        }];
        // Offered windows may overlap when the preceding segment drains late.
        // Its actual observations still cover that time; never invent a new
        // origin or lose their evidence merely because the ledger changed.
        let mut resources = resource_window::overlapping_samples(
            &self.previous_resources,
            started,
            self.pid,
            campaign.evidence.samples_per_segment.get() as usize,
        )?;
        for value in &resources {
            writer.record_event(&ProgressEvent::Resource {
                segment: writer.next_index(),
                sample: Box::new(value.clone()),
            })?;
        }
        let mut sample_task = sample_task::SampleTask::new();
        let mut initial_sample_pending = !(phase == LoadPhase::Warmup && first == 0);
        if !initial_sample_pending {
            // The evaluator requires this real ownership baseline before any
            // initial warmup effect; later segment samples need no such fence.
            resources.push(self.sample(writer).await?);
        } else {
            let not_before = self
                .clock
                .checked_add(Duration::from_nanos(started))
                .ok_or("initial sample deadline overflow")?;
            self.start_sample_at(&mut sample_task, Some(not_before))?;
        }
        let held_reader = if phase == LoadPhase::Measured
            && campaign
                .required_lanes
                .contains(&CapacityLane::ReaderPinInterference)
            && prepared.iter().any(|operation| operation.write.is_some())
        {
            Some(
                reader::HeldReader::open(
                    &self.workload,
                    writer.next_index(),
                    self.clock,
                    campaign.reader_selection.as_ref(),
                )
                .await?,
            )
        } else {
            None
        };
        let mut attempts = Vec::new();
        let mut failure = ReadbackFailures::default();
        let mut joins = JoinSet::new();
        let mut backup_started_ns = None;
        let deadline = Duration::from_millis(campaign.schedule.operation_deadline_ms.get());
        let period = Duration::from_millis(campaign.schedule.sample_ms.get());
        let last_observed = self.clock
            + Duration::from_nanos(resources.last().map_or(started, |value| value.elapsed_ns));
        let mut next_sample =
            resource_window::next_sample_deadline(last_observed, period, Instant::now())?;
        for (offset, operation) in prepared.iter().enumerate() {
            let sequence = first + offset as u64;
            let due = self.clock + Duration::from_nanos(scheduled(sequence));
            loop {
                let sample_pending = sample_task.is_pending();
                tokio::select! {
                    biased;
                    result=joins.join_next(), if !joins.is_empty() => {
                        collect(result.ok_or("missing scheduled task")?.map_err(|_|"scheduled task panicked")?,started,&mut attempts,&mut ledger,&mut failure,writer)?;
                    }
                    _=tokio::time::sleep_until(due.into()) => break,
                    value=sample_task.finish(), if sample_pending => {
                        let value=value?.ok_or("missing resource sample")?;
                        let observed=self.clock.checked_add(Duration::from_nanos(value.elapsed_ns)).ok_or("sample timestamp overflow")?;
                        resources.push(Self::record_sample(writer,value)?);
                        next_sample=resource_window::after_sample_deadline(next_sample,period,observed,Instant::now(),initial_sample_pending)?;
                        initial_sample_pending=false;
                    }
                    _=tokio::time::sleep_until(next_sample.into()), if !sample_pending => {
                        if resources.len() as u64>=campaign.evidence.samples_per_segment.get() {return Err("resource sample cap exhausted".into());}
                        self.start_sample(&mut sample_task)?;
                    }
                }
            }
            let work = self.workload.clone();
            let operation = operation.clone();
            let clock = self.clock;
            if backup_due(campaign, phase, sequence) {
                let work = work.clone();
                let name = format!("capacity-backup-{}", writer.next_index());
                let started_ns = ns(clock);
                backup_started_ns = Some(started_ns);
                writer.record_event(&ProgressEvent::BackupStarted {
                    segment: writer.next_index(),
                    point: self.point.id.clone(),
                    seed: self.workload.seed,
                    started_ns,
                })?;
                writer.checkpoint_progress()?;
                self.backup.start(
                    clock + Duration::from_nanos(started_ns) + deadline,
                    lifecycle::backup_started(work, name, clock, started_ns),
                )?;
            }
            if joins.len() as u64 >= self.point.clients.get() {
                let at = ns(clock);
                collect(
                    (
                        sequence,
                        scheduled(sequence),
                        at,
                        at,
                        at,
                        operation,
                        Err("driver-concurrency-limit".into()),
                        None,
                        true,
                    ),
                    started,
                    &mut attempts,
                    &mut ledger,
                    &mut failure,
                    writer,
                )?;
            } else {
                let at = scheduled(sequence);
                joins.spawn(async move {
                    let began = ns(clock);
                    let expires = tokio::time::Instant::now() + deadline;
                    let result = tokio::time::timeout_at(expires, work.execute(&operation))
                        .await
                        .map_err(|_| "operation-deadline".to_owned())
                        .and_then(std::convert::identity);
                    // API latency ends at its actual response. Independent oracle
                    // work stays inside this same admitted client slot/deadline,
                    // so it cannot pause the next offered segment or grow tasks.
                    let finished = ns(clock);
                    let verification = if result.is_ok() {
                        Some(
                            tokio::time::timeout_at(expires, work.verify(&operation))
                                .await
                                .map_err(|_| "readback-deadline".to_owned())
                                .and_then(std::convert::identity),
                        )
                    } else {
                        None
                    };
                    (
                        sequence,
                        at,
                        began,
                        finished,
                        ns(clock),
                        operation,
                        result,
                        verification,
                        false,
                    )
                });
            }
        }
        while !joins.is_empty() {
            let sample_pending = sample_task.is_pending();
            tokio::select! {
                result=joins.join_next() => collect(result.ok_or("missing scheduled task")?.map_err(|_|"scheduled task panicked")?,started,&mut attempts,&mut ledger,&mut failure,writer)?,
                value=sample_task.finish(), if sample_pending => {
                    let value=value?.ok_or("missing resource sample")?;
                    let observed=self.clock.checked_add(Duration::from_nanos(value.elapsed_ns)).ok_or("sample timestamp overflow")?;
                    resources.push(Self::record_sample(writer,value)?);
                    next_sample=resource_window::after_sample_deadline(next_sample,period,observed,Instant::now(),initial_sample_pending)?;
                    initial_sample_pending=false;
                }
                _=tokio::time::sleep_until(next_sample.into()), if !sample_pending => {
                    if resources.len() as u64>=campaign.evidence.samples_per_segment.get(){return Err("resource sample cap exhausted".into());}
                    self.start_sample(&mut sample_task)?;
                }
            }
        }
        // All offers have been issued and joined. Complete the single owned
        // sample before the reader, backup or final resource boundary.
        if sample_task.is_pending() {
            let value = sample_task
                .finish()
                .await?
                .ok_or("missing resource sample")?;
            let observed = self
                .clock
                .checked_add(Duration::from_nanos(value.elapsed_ns))
                .ok_or("sample timestamp overflow")?;
            resources.push(Self::record_sample(writer, value)?);
            next_sample = resource_window::after_sample_deadline(
                next_sample,
                period,
                observed,
                Instant::now(),
                initial_sample_pending,
            )?;
        }
        let reader_receipt = match held_reader {
            Some(reader) => Some(reader.finish(&self.workload, self.clock).await?),
            None => None,
        };
        if let Some(observation) = &reader_receipt {
            writer.record_event(&ProgressEvent::ReaderInterference {
                segment: writer.next_index(),
                observation: Box::new(observation.clone()),
            })?;
            writer.checkpoint_progress()?;
        }
        // An evidence boundary must not join management work while more offers
        // remain. The task keeps its original absolute deadline across segments.
        let mut backup_receipt = self.backup.try_finish()?;
        while final_segment && self.backup.is_pending() {
            tokio::select! {
                value=self.backup.finish() => { backup_receipt=value?; }
                _=tokio::time::sleep_until(next_sample.into()) => {
                    if resources.len() as u64+1>=campaign.evidence.samples_per_segment.get() {
                        return Err("backup resource sample cap exhausted".into());
                    }
                    resources.push(self.sample(writer).await?);
                    next_sample=resource_window::next_sample_deadline(next_sample,period,Instant::now())?;
                }
            }
        }
        let verified = failure.count == 0;
        // Each working slot appears at most once in this admitted segment.
        // Only an independently verified response advances its next generation.
        for operation in &prepared {
            if ledger.readback.iter().any(|observation| {
                matches!(observation,
                Readback::Present { logical_id, .. } if logical_id.as_str() == operation.id)
            }) {
                if let Some(write) = operation.write {
                    self.generations[write.slot as usize] = write
                        .generation
                        .checked_add(1)
                        .filter(|n| *n <= MAX_EXACT_INTEGER)
                        .ok_or("working slot generation overflow")?;
                }
            }
        }
        let offered_end = self.clock + Duration::from_nanos(scheduled(first + count));
        while Instant::now() < offered_end {
            tokio::time::sleep_until(next_sample.min(offered_end).into()).await;
            if resources.len() as u64 + 1 >= campaign.evidence.samples_per_segment.get() {
                return Err("offered interval resource sample cap exhausted".into());
            }
            resources.push(self.sample(writer).await?);
            next_sample =
                resource_window::next_sample_deadline(next_sample, period, Instant::now())?;
        }
        resources.push(self.sample(writer).await?);
        if backup_receipt.is_none() {
            backup_receipt = self.backup.try_finish()?;
        }
        if let Some(observation) = &backup_receipt {
            writer.record_event(&ProgressEvent::Backup {
                segment: writer.next_index(),
                observation: observation.clone(),
            })?;
        }
        attempts.sort_by_key(|a| a.sequence);
        writer.checkpoint_progress()?;
        let mut lanes = match &backup_receipt {
            Some(raw) => vec![LaneEvidence {
                lane: CapacityLane::BackupUnderWrite,
                repetitions: 1u64.try_into()?,
                receipt_sha256: digest("c07g.backup-evidence", raw)?,
                acknowledged_readback_verified: verified,
            }],
            None => vec![],
        };
        if let Some(raw) = &reader_receipt {
            if raw.qualifies(&attempts) {
                lanes.push(LaneEvidence {
                    lane: CapacityLane::ReaderPinInterference,
                    repetitions: 1u64.try_into()?,
                    receipt_sha256: digest("c07g.reader-interference", raw)?,
                    acknowledged_readback_verified: verified,
                });
            }
        }
        let result = StorageResult {
            format_version: 1,
            manifest_sha256: manifest_digest(&manifest)?,
            cases: vec![CaseResult {
                id: evidence_id(key),
                seed: self.workload.seed,
                status: if verified {
                    CaseStatus::Executed
                } else {
                    CaseStatus::Failed
                },
                faults: failure.finish(),
                limitation: if verified {
                    None
                } else {
                    Some(evidence_id("independent-readback-failed"))
                },
                ledger,
            }],
        };
        if phase == LoadPhase::Measured && first == 0 {
            lanes.push(LaneEvidence {
                lane: CapacityLane::Warm,
                repetitions: 1u64.try_into()?,
                receipt_sha256: digest("c07g.warm-readback", &result)?,
                acknowledged_readback_verified: verified,
            });
        }
        // One admitted segment only. A failure to append/validate the segment
        // stops the campaign, so unvalidated observations cannot advance it.
        self.previous_resources.clone_from(&resources);
        Ok(SustainedSegment {
            format_version: 1,
            campaign_sha256: writer.identity().clone(),
            index: writer.next_index(),
            previous_sha256: writer.previous_sha256(),
            point: self.point.id.clone(),
            seed: self.workload.seed,
            phase,
            started_ns: started,
            finished_ns: ns(self.clock),
            corpus: None,
            cold_start: None,
            reopens: None,
            backup_started_ns,
            backup: backup_receipt,
            reader: reader_receipt,
            manifest,
            result,
            attempts,
            resources,
            lanes,
        })
    }
}

// One rolling failure digest bounds retained provenance independently of
// segment attempt count. It includes each failed logical identity and cause.
#[derive(Default)]
pub(super) struct ReadbackFailures {
    count: u64,
    digest: Option<Digest>,
}
impl ReadbackFailures {
    pub(super) fn record(&mut self, logical_id: &str, cause: &str) -> DriverResult<()> {
        self.count = self.count.checked_add(1).ok_or("failure count overflow")?;
        self.digest = Some(digest(
            "c07g.readback-failures",
            &(&self.digest, logical_id, cause),
        )?);
        Ok(())
    }
    fn finish(self) -> Vec<Failure> {
        self.digest.map_or_else(Vec::new, |artifact_sha256| {
            vec![Failure {
                phase: FailurePhase::Readback,
                code: evidence_id("independent-readback-failed"),
                occurrence: self.count.try_into().expect("nonempty failure history"),
                artifact_sha256,
            }]
        })
    }
}
pub(super) fn failure_code(error: &str) -> Id {
    if error == "operation-deadline" {
        return evidence_id(error);
    }
    if let Some((code, detail)) = error.split_once(':') {
        if (code.starts_with("api-") || code.starts_with("client-"))
            && code.len() <= 80
            && code.bytes().all(|b| b.is_ascii_lowercase() || b == b'-')
            && detail.len() == 64
            && detail.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return evidence_id(code);
        }
    }
    evidence_id("request-or-oracle-failed")
}

#[test]
fn failed_campaign_preserves_closed_cause_and_readback_provenance() {
    assert_eq!(
        failure_code("operation-deadline").as_str(),
        "operation-deadline"
    );
    let sanitized = format!("api-conflict:{}", "a".repeat(64));
    assert_eq!(failure_code(&sanitized).as_str(), "api-conflict");
    assert_eq!(
        failure_code("api-secret:private detail").as_str(),
        "request-or-oracle-failed"
    );
    let manifest: StorageManifest = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../evaluation/qualification/rrflow-qualification-storage/tests/fixtures/storage/manifest.json"
    )))
    .unwrap();
    let mut result: StorageResult = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../evaluation/qualification/rrflow-qualification-storage/tests/fixtures/storage/result.json"
    )))
    .unwrap();
    result.manifest_sha256 = manifest_digest(&manifest).unwrap();
    result.cases[0].status = CaseStatus::Failed;
    result.cases[0].faults.clear();
    assert_eq!(
        validate_result(&manifest, &result).unwrap_err(),
        "failed case must retain failure provenance"
    );
    let mut failure = ReadbackFailures::default();
    failure.record("logical-a", &sanitized).unwrap();
    failure.record("logical-b", "readback differs").unwrap();
    result.cases[0].faults = failure.finish();
    assert_eq!(result.cases[0].faults[0].occurrence.get(), 2);
    let report = validate_result(&manifest, &result).unwrap();
    assert_eq!(report.failed, 1);
    assert!(!report.contract_passed);
}
