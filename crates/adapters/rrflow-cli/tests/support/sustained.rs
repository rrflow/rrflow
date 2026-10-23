//! Ordinary installed campaign. Explicit environment admission keeps a long
//! campaign out of ordinary correctness CI; failure artifacts are never green.
use super::Estate;
use rrd_client::RequestOptions;
use rrd_contract::{CloseSession, CreateSession, ReadStorageDiagnostics, SessionLimits};
use rrflow_eval::storage::*;
use std::{
    io::Read,
    path::PathBuf,
    time::{Duration, Instant},
};
use tokio::task::JoinSet;
#[path = "sustained/cold.rs"]
mod cold;
#[path = "sustained/corpus.rs"]
mod corpus;
#[path = "sustained/damage.rs"]
mod damage;
#[path = "sustained/fleet.rs"]
mod fleet;
#[path = "sustained/lifecycle.rs"]
mod lifecycle;
#[path = "sustained/native.rs"]
mod native;
#[path = "sustained/reader.rs"]
mod reader;
#[path = "sustained/workload.rs"]
mod workload;
use workload::{Prepared, Workload};

type DriverResult<T> = std::result::Result<T, String>;
fn evidence_id(value: impl Into<String>) -> Id {
    value.into().try_into().unwrap()
}
fn ns(clock: Instant) -> u64 {
    clock
        .elapsed()
        .as_nanos()
        .try_into()
        .expect("bounded campaign clock")
}
fn bounded_json<T: serde::de::DeserializeOwned>(path: &std::path::Path) -> DriverResult<T> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(MAX_DOCUMENT_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    decode(&bytes)
}

async fn session(
    estate: &Estate,
    client: rrd_client::RrdClient,
    seed: u64,
    campaign: &StorageCampaign,
    clients: u64,
    ordinal: u64,
) -> DriverResult<Workload> {
    let credential = rrd_engine::RrdEngine::read_installed_api_key(&estate.project)
        .map_err(|e| e.to_string())?;
    let name = format!("capacity-session-{ordinal}");
    let session = client
        .create_session(
            credential.principal_id.clone(),
            credential.credential(),
            CreateSession {
                limits: SessionLimits {
                    idle_timeout_ms: 600_000,
                    absolute_timeout_ms: 3_600_000,
                    max_open_transactions: clients
                        .try_into()
                        .map_err(|_| "client count overflow")?,
                },
            },
            RequestOptions::new(&name, &name, Some(&name), Some(u64::MAX)).unwrap(),
        )
        .await
        .map_err(|e| e.to_string())?;
    Ok(Workload {
        client,
        session,
        valid_at: estate.at,
        dimensions: campaign
            .allocation
            .vector_dimensions
            .get()
            .try_into()
            .map_err(|_| "vector dimensions overflow")?,
        seed,
        records: campaign.allocation.records.get(),
        read_distribution: campaign.read_distribution.clone(),
        edges_per_record: campaign
            .allocation
            .edges
            .get()
            .div_ceil(campaign.allocation.records.get())
            .try_into()
            .map_err(|_| "edge fanout overflow")?,
        vectors_per_record: campaign
            .allocation
            .vectors
            .get()
            .div_ceil(campaign.allocation.records.get())
            .try_into()
            .map_err(|_| "vector fanout overflow")?,
        payload_bytes: campaign
            .allocation
            .logical_source_bytes
            .get()
            .div_ceil(campaign.allocation.records.get())
            .max(1024)
            .try_into()
            .map_err(|_| "source payload overflow")?,
        query_budget: serde_json::from_value(estate.plan["configuration"]["query"].clone())
            .map_err(|e| e.to_string())?,
    })
}

/// Rotate only between bounded segments, after every in-flight request has
/// joined. Closing the old lease prevents a day of accumulated live sessions.
async fn rotate_session(
    estate: &Estate,
    work: &mut Workload,
    campaign: &StorageCampaign,
    clients: u64,
    ordinal: &mut u64,
    opened: &mut Instant,
) -> DriverResult<()> {
    if opened.elapsed() < Duration::from_secs(120) {
        return Ok(());
    }
    let key = format!("capacity-close-{ordinal}");
    work.client
        .close_session(
            &work.session,
            CloseSession {},
            RequestOptions::new(&key, &key, Some(&key), Some(u64::MAX)).unwrap(),
        )
        .await
        .map_err(|e| e.to_string())?;
    *ordinal = ordinal.checked_add(1).ok_or("session ordinal overflow")?;
    *work = session(
        estate,
        work.client.clone(),
        work.seed,
        campaign,
        clients,
        *ordinal,
    )
    .await?;
    *opened = Instant::now();
    Ok(())
}

async fn sample(
    workload: &Workload,
    pid: u32,
    clock: Instant,
    allocation: &Option<NativeAllocationReceipt>,
) -> DriverResult<ResourceSample> {
    let mut system = sysinfo::System::new();
    let process_id = sysinfo::Pid::from_u32(pid);
    system.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[process_id]), true);
    system.refresh_memory();
    let process = system
        .process(process_id)
        .ok_or("owned engine exited during resource sampling")?;
    let io = process.disk_usage();
    let snapshot = workload
        .client
        .read_storage_diagnostics(
            &workload.session,
            ReadStorageDiagnostics {
                scope: format!("instance:{}", workload.client.instance_id()),
            },
            RequestOptions::read("capacity-resource", "capacity-resource").unwrap(),
        )
        .await
        .map_err(|e| e.to_string())?;
    let missing = |reason: &str| NativeMetric::Unavailable {
        reason: evidence_id(reason),
    };
    let (private_bytes, mapped_bytes) = native::memory(pid);
    Ok(ResourceSample {
        elapsed_ns: ns(clock),
        pid,
        resident_bytes: NativeMetric::Observed {
            value: process.memory(),
        },
        private_bytes,
        mapped_bytes,
        host_resident_bytes: NativeMetric::Observed {
            value: system.used_memory(),
        },
        read_bytes: NativeMetric::Observed {
            value: io.total_read_bytes,
        },
        written_bytes: NativeMetric::Observed {
            value: io.total_written_bytes,
        },
        cpu_time_ns: NativeMetric::Observed {
            value: process.accumulated_cpu_time() * 1_000_000,
        },
        retained_bytes: match &snapshot.admission {
            Some(value) => NativeMetric::Observed {
                value: value
                    .retained_mutable_bytes
                    .checked_add(value.retained_projected_bytes)
                    .and_then(|bytes| bytes.checked_add(value.native_transaction_bytes))
                    .ok_or("retained sum overflow")?,
            },
            None => missing("public-retained-gauge-unavailable"),
        },
        queue_requests: match &snapshot.admission {
            Some(value) => NativeMetric::Observed {
                value: value.outstanding_requests,
            },
            None => missing("public-queue-gauge-unavailable"),
        },
        maintenance_debt_segments: match &snapshot.rrflow_kv {
            Some(value) => NativeMetric::Observed {
                value: value.segment_set.compaction_debt_segments,
            },
            None => missing("public-debt-gauge-unavailable"),
        },
        allocation_enforcement: native::enforcement(pid, allocation.as_ref())?,
        allocation_receipt: allocation.clone(),
        storage: Some(snapshot),
    })
}

struct ProgressContext<'a> {
    writer: &'a CampaignWriter,
    point: &'a OperatingPoint,
}

struct PointRun<'a> {
    campaign: &'a StorageCampaign,
    point: &'a OperatingPoint,
    workload: Workload,
    pid: u32,
    clock: Instant,
    allocation: Option<NativeAllocationReceipt>,
    generations: Vec<u64>,
}

impl PointRun<'_> {
    async fn sample(&self, writer: &CampaignWriter) -> DriverResult<ResourceSample> {
        let value = sample(&self.workload, self.pid, self.clock, &self.allocation).await?;
        writer.record_event(&ProgressEvent::Resource {
            segment: writer.next_index(),
            sample: Box::new(value.clone()),
        })?;
        Ok(value)
    }
    async fn drain(
        &self,
        writer: &CampaignWriter,
        duration_ms: u64,
    ) -> DriverResult<SustainedSegment> {
        if self.campaign.idle_segment_ms(duration_ms)? != duration_ms {
            return Err("idle duration exceeds its frozen segment admission".into());
        }
        tokio::time::timeout(
            Duration::from_millis(self.campaign.storage.budgets.run_ms.get().min(180_000)),
            self.drain_inner(writer, duration_ms),
        )
        .await
        .map_err(|_| "idle segment exceeded its frozen deadline")?
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
        loop {
            resources.push(self.sample(writer).await?);
            if Instant::now() >= deadline {
                break;
            }
            if resources.len() as u64 >= self.campaign.evidence.samples_per_segment.get() {
                return Err("drain resource sample cap exhausted".into());
            }
            tokio::time::sleep_until(
                (Instant::now() + Duration::from_millis(self.campaign.schedule.sample_ms.get()))
                    .min(deadline)
                    .into(),
            )
            .await;
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
            backup: None,
            reader: None,
            manifest,
            result,
            attempts: vec![],
            resources,
            lanes: vec![],
        })
    }
    async fn segment(
        &mut self,
        writer: &CampaignWriter,
        phase: LoadPhase,
        origin: u64,
        first: u64,
        count: u64,
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
        let mut resources = vec![self.sample(writer).await?];
        let held_reader = if phase == LoadPhase::Measured
            && campaign
                .required_lanes
                .contains(&CapacityLane::ReaderPinInterference)
            && prepared.iter().any(|operation| operation.write.is_some())
        {
            Some(reader::HeldReader::open(&self.workload, writer.next_index(), self.clock).await?)
        } else {
            None
        };
        let mut attempts = Vec::new();
        let mut failure = ReadbackFailures::default();
        let mut joins = JoinSet::new();
        let mut backup = JoinSet::new();
        let deadline = Duration::from_millis(campaign.schedule.operation_deadline_ms.get());
        let mut next_sample =
            tokio::time::Instant::now() + Duration::from_millis(campaign.schedule.sample_ms.get());
        for (offset, operation) in prepared.iter().enumerate() {
            let sequence = first + offset as u64;
            let due = self.clock + Duration::from_nanos(scheduled(sequence));
            loop {
                tokio::select! {
                    biased;
                    result=joins.join_next(), if !joins.is_empty() => {
                        collect(result.ok_or("missing scheduled task")?.map_err(|_|"scheduled task panicked")?,started,&mut attempts,&mut ledger,&mut failure,writer)?;
                    }
                    _=tokio::time::sleep_until(next_sample) => {
                        if resources.len() as u64>=campaign.evidence.samples_per_segment.get() {return Err("resource sample cap exhausted".into());}
                        resources.push(self.sample(writer).await?);
                        next_sample=tokio::time::Instant::now()+Duration::from_millis(campaign.schedule.sample_ms.get());
                    }
                    _=tokio::time::sleep_until(due.into()) => break,
                }
            }
            let work = self.workload.clone();
            let operation = operation.clone();
            let clock = self.clock;
            if phase == LoadPhase::Measured && sequence == 14 {
                let work = work.clone();
                let name = format!("capacity-backup-{}", writer.next_index());
                backup.spawn(async move {
                    tokio::time::timeout(deadline, lifecycle::backup(work, name, clock))
                        .await
                        .map_err(|_| "backup interference deadline".to_owned())?
                });
            }
            if joins.len() as u64 >= self.point.clients.get() {
                let at = ns(clock);
                collect(
                    (
                        sequence,
                        scheduled(sequence),
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
                        operation,
                        result,
                        verification,
                        false,
                    )
                });
            }
        }
        while !joins.is_empty() {
            tokio::select! {
                result=joins.join_next() => collect(result.ok_or("missing scheduled task")?.map_err(|_|"scheduled task panicked")?,started,&mut attempts,&mut ledger,&mut failure,writer)?,
                _=tokio::time::sleep_until(next_sample) => {
                    if resources.len() as u64>=campaign.evidence.samples_per_segment.get(){return Err("resource sample cap exhausted".into());}
                    resources.push(self.sample(writer).await?);
                    next_sample=tokio::time::Instant::now()+Duration::from_millis(campaign.schedule.sample_ms.get());
                }
            }
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
        let mut backup_receipt = None;
        while !backup.is_empty() {
            tokio::select! {
                value=backup.join_next() => {
                    backup_receipt=Some(value.ok_or("backup task missing")?.map_err(|_|"backup task panicked")??);
                }
                _=tokio::time::sleep_until(next_sample) => {
                    if resources.len() as u64+1>=campaign.evidence.samples_per_segment.get() {
                        return Err("backup resource sample cap exhausted".into());
                    }
                    resources.push(self.sample(writer).await?);
                    next_sample=tokio::time::Instant::now()+Duration::from_millis(campaign.schedule.sample_ms.get());
                }
            }
        }
        let verified = failure.count == 0;
        if let Some(observation) = &backup_receipt {
            writer.record_event(&ProgressEvent::Backup {
                segment: writer.next_index(),
                observation: observation.clone(),
            })?;
        }
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
            tokio::time::sleep_until(
                (Instant::now() + Duration::from_millis(campaign.schedule.sample_ms.get()))
                    .min(offered_end)
                    .into(),
            )
            .await;
            if resources.len() as u64 + 1 >= campaign.evidence.samples_per_segment.get() {
                return Err("offered interval resource sample cap exhausted".into());
            }
            resources.push(self.sample(writer).await?);
        }
        resources.push(self.sample(writer).await?);
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
struct ReadbackFailures {
    count: u64,
    digest: Option<Digest>,
}
impl ReadbackFailures {
    fn record(&mut self, logical_id: &str, cause: &str) -> DriverResult<()> {
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
fn failure_code(error: &str) -> Id {
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
        "/../../evaluation/rrflow-eval/tests/fixtures/storage/manifest.json"
    )))
    .unwrap();
    let mut result: StorageResult = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../evaluation/rrflow-eval/tests/fixtures/storage/result.json"
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

type Completion = (
    u64,
    u64,
    u64,
    u64,
    Prepared,
    DriverResult<workload::Completed>,
    Option<DriverResult<()>>,
    bool,
);
fn collect(
    (sequence, scheduled_ns, started_ns, finished_ns, operation, result, verification, rejected): Completion,
    segment_start: u64,
    attempts: &mut Vec<LoadAttempt>,
    ledger: &mut AcknowledgementLedger,
    failure: &mut ReadbackFailures,
    writer: &CampaignWriter,
) -> DriverResult<()> {
    let (outcome, result_sha256, recorded) = match result {
        Ok(result) => (
            LoadOutcome::Acknowledged,
            result.receipt_sha256.clone(),
            AttemptOutcome::Acknowledged {
                receipt_sha256: result.receipt_sha256,
            },
        ),
        Err(_) if rejected => (
            LoadOutcome::Rejected,
            digest("c07g.saturated", &sequence)?,
            AttemptOutcome::Rejected {
                reason: evidence_id("driver-concurrency-limit"),
            },
        ),
        Err(error) if error.starts_with("rejected-") => (
            LoadOutcome::Rejected,
            digest("c07g.rejected-request", &(sequence, &error))?,
            AttemptOutcome::Rejected {
                reason: failure_code(error.strip_prefix("rejected-").expect("matched prefix")),
            },
        ),
        Err(error) => (
            LoadOutcome::Uncertain,
            digest("c07g.request-failure", &(sequence, &error))?,
            AttemptOutcome::Uncertain {
                reason: failure_code(&error),
            },
        ),
    };
    let readback = match outcome {
        LoadOutcome::Acknowledged => match verification {
            Some(Ok(())) => Some(Readback::Present {
                logical_id: evidence_id(&operation.id),
                request_sha256: operation.request_sha256.clone(),
                effect_sha256: operation.effect_sha256.clone(),
                receipt_sha256: result_sha256.clone(),
                applications: 1u64.try_into().unwrap(),
            }),
            Some(Err(error)) => {
                failure.record(&operation.id, &error)?;
                None
            }
            None => return Err("acknowledged worker omitted independent verification".into()),
        },
        LoadOutcome::Rejected => Some(Readback::Absent {
            logical_id: evidence_id(&operation.id),
        }),
        _ => {
            failure.record(&operation.id, result_sha256.as_str())?;
            None
        }
    };
    ledger.attempts.push(Attempt {
        attempt_id: evidence_id(format!("attempt-{sequence}")),
        logical_id: evidence_id(&operation.id),
        request_sha256: operation.request_sha256.clone(),
        started_ns: started_ns - segment_start,
        finished_ns: finished_ns - segment_start,
        outcome: recorded,
    });
    attempts.push(LoadAttempt {
        sequence,
        kind: operation.kind,
        write: operation.write,
        scheduled_ns,
        started_ns,
        finished_ns,
        outcome,
        logical_id: evidence_id(operation.id),
        request_sha256: operation.request_sha256,
        result_sha256,
    });
    writer.record_event(&ProgressEvent::LoadOutcome {
        segment: writer.next_index(),
        attempt: attempts.last().ok_or("missing completed attempt")?.clone(),
    })?;
    if let Some(observation) = readback {
        writer.record_event(&ProgressEvent::Readback {
            segment: writer.next_index(),
            observation: observation.clone(),
        })?;
        ledger.readback.push(observation);
    }
    Ok(())
}

#[test]
#[ignore = "explicit frozen campaign and new evidence directory required; never substitutes for 24-hour evidence"]
fn installed_sustained_capacity() {
    let campaign_path = PathBuf::from(
        std::env::var_os("RRFLOW_SUSTAINED_CAMPAIGN").expect("explicit campaign path"),
    );
    let evidence =
        PathBuf::from(std::env::var_os("RRFLOW_SUSTAINED_EVIDENCE").expect("new evidence path"));
    let campaign: StorageCampaign = bounded_json(&campaign_path).unwrap();
    campaign.validate().unwrap();
    if campaign.allocation.estates.get() > 1 {
        return fleet::execute(campaign, evidence);
    }
    let mut writer = CampaignWriter::create(&evidence, campaign.clone()).unwrap();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(run(&campaign, &mut writer))
    }))
    .unwrap_or_else(|_| {
        Err("installed driver panicked; retained progress is incomplete evidence".into())
    });
    let (status, limitations) = if result.is_ok() {
        (CampaignStatus::Completed, vec![])
    } else {
        if let Err(error) = writer.record_event(&ProgressEvent::Failure {
            reason: evidence_id("installed-driver-failed"),
        }) {
            eprintln!("failure event could not be appended: {error}; sealing failed result");
        }
        (
            CampaignStatus::Failed,
            vec![evidence_id("installed-driver-failed")],
        )
    };
    let report = writer.finish(status, limitations).unwrap();
    println!(
        "native_sustained_result={}",
        serde_json::to_string(&report).unwrap()
    );
    assert!(
        result.is_ok(),
        "installed sustained driver failed: {result:?}"
    );
    assert!(
        campaign.purpose != CampaignPurpose::Qualification || report.qualification_passed,
        "campaign completed but did not satisfy its frozen qualification requirements"
    );
}

/// Short executable integration of the campaign's actual workload adapter.
/// It deliberately emits no NativeCapacity/soak qualification claim.
#[test]
#[ignore = "explicit native capacity-driver smoke; candidate portability job"]
fn installed_capacity_workload_smoke() {
    let estate = Estate::install("fresh");
    let clock = Instant::now();
    let (child, client, _) = estate.start_for(clock, Duration::from_secs(300));
    let credential = rrd_engine::RrdEngine::read_installed_api_key(&estate.project).unwrap();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let lease = client
            .create_session(
                credential.principal_id.clone(),
                credential.credential(),
                CreateSession {
                    limits: SessionLimits {
                        idle_timeout_ms: 300_000,
                        absolute_timeout_ms: 600_000,
                        max_open_transactions: 4,
                    },
                },
                RequestOptions::mutation(
                    "capacity-smoke-session",
                    "capacity-smoke-session",
                    "capacity-smoke-session",
                )
                .unwrap(),
            )
            .await
            .unwrap();
        let mut work = Workload {
            client,
            session: lease,
            valid_at: estate.at,
            dimensions: 16,
            seed: 73,
            records: 8,
            read_distribution: ReadKeyDistribution::Uniform {},
            edges_per_record: 3,
            vectors_per_record: 2,
            payload_bytes: 1024,
            query_budget: serde_json::from_value(estate.plan["configuration"]["query"].clone())
                .unwrap(),
        };
        work.initialize().await.unwrap();
        let mut receipts = Vec::new();
        for ordinal in 0..work.records {
            let operation = work.seed_record(ordinal).unwrap();
            let receipt = work.execute(&operation).await.unwrap();
            assert!(!receipt.idempotent_replay);
            receipts.push(receipt.receipt_sha256);
        }
        for ordinal in 0..work.records {
            work.verify(&work.seed_record(ordinal).unwrap())
                .await
                .unwrap();
        }
        for (index, kind) in [
            StorageLoadKind::PointRead,
            StorageLoadKind::RangeRead,
            StorageLoadKind::Maintenance,
        ]
        .into_iter()
        .enumerate()
        {
            let operation = work
                .prepare(format!("smoke-read-{index}"), kind, index as u64)
                .unwrap();
            work.execute(&operation).await.unwrap();
            work.verify(&operation).await.unwrap();
        }
        let physical = work.client.read_storage_diagnostics(&work.session,
            ReadStorageDiagnostics {scope: format!("instance:{}",work.client.instance_id())},
            RequestOptions::read("smoke-storage", "smoke-storage").unwrap()).await.unwrap();
        assert!(physical.rrflow_kv.is_some() && physical.admission.is_some());
        let mut final_states = Vec::new();
        for slot in 0..3 {
            for generation in 0..=slot+2 {
                let operation = work.prepare_load(format!("smoke-slot-{slot}-{generation}"),
                    WriteTransition { slot, generation }).unwrap();
                let receipt = work.execute(&operation).await.unwrap();
                assert!(!receipt.idempotent_replay);
                work.verify(&operation).await.unwrap();
                if generation == slot+2 { final_states.push((operation, receipt.receipt_sha256)); }
            }
        }
        let write = work
            .prepare(
                "smoke-concurrent-write".into(),
                StorageLoadKind::MixedWrite,
                99,
            )
            .unwrap();
        let retained_reader = reader::HeldReader::open(&work, 0, clock).await.unwrap();
        let (written, backup) = tokio::join!(
            work.execute(&write),
            lifecycle::backup(work.clone(), "smoke-backup".into(), clock)
        );
        let written = written.unwrap();
        let backup = backup.unwrap();
        let reader_evidence = retained_reader.finish(&work, clock).await.unwrap();
        work.verify(&write).await.unwrap();
        let killed = super::kill(child, clock);
        let (reopened, client, identity) = estate.start_for(clock, Duration::from_secs(300));
        work.client = client;
        // Every old value is read before replay; exact old request identities
        // must return the stored receipt after this real process boundary.
        for ordinal in 0..work.records {
            let operation = work.seed_record(ordinal).unwrap();
            work.verify(&operation).await.unwrap();
            let replay = work.execute(&operation).await.unwrap();
            assert!(replay.idempotent_replay);
            assert_eq!(replay.receipt_sha256, receipts[ordinal as usize]);
        }
        for (operation, original) in &final_states {
            work.verify(operation).await.unwrap();
            let replay = work.execute(operation).await.unwrap();
            assert!(replay.idempotent_replay);
            assert_eq!(&replay.receipt_sha256, original);
        }
        work.verify(&write).await.unwrap();
        let replay = work.execute(&write).await.unwrap();
        assert!(replay.idempotent_replay);
        assert_eq!(replay.receipt_sha256, written.receipt_sha256);
        println!(
            "installed_capacity_workload_smoke={}",
            serde_json::json!({
            "build":estate.build,"records":8,"edges":24,"vectors":16,"dimensions":16,
            "replayed_transactions":12,"write_transitions":{"insert":3,"update":4,"retire":3,"recreate":2},"backup_sha256":backup.receipt.backup.backup_sha256,
            "reader_interference":reader_evidence,"termination":killed,"reopened":identity,"capacity_qualification":false})
        );
        super::kill(reopened, clock);
    });
}

async fn run(campaign: &StorageCampaign, writer: &mut CampaignWriter) -> DriverResult<()> {
    run_estate(campaign, writer, None).await
}
async fn run_estate(
    campaign: &StorageCampaign,
    writer: &mut CampaignWriter,
    fleet: Option<&fleet::Participation>,
) -> DriverResult<()> {
    if campaign
        .required_lanes
        .iter()
        .any(|lane| *lane == CapacityLane::Cold && !cfg!(target_os = "linux"))
    {
        return Err("native cold-cache control is not available on this platform".into());
    }
    if campaign
        .required_lanes
        .contains(&CapacityLane::BackupUnderWrite)
        && campaign.points.iter().any(|point| {
            u128::from(campaign.schedule.measured_ms.get())
                * u128::from(point.requests_per_second.get())
                < 15_000
        })
    {
        return Err(
            "backup interference requires the measured mix to reach its first mixed write".into(),
        );
    }
    if campaign.schedule.operation_deadline_ms.get() > 60_000 {
        return Err("installed request deadline exceeds the driver's one-minute bound".into());
    }
    if campaign.allocation.estates.get() != 1 {
        return Err("fleet requires native aggregate estate driver".into());
    }
    if campaign
        .required_lanes
        .contains(&CapacityLane::ReaderPinInterference)
        && campaign.schedule.operation_deadline_ms.get() > 30_000
    {
        return Err("retained-reader lane requires operation deadline at most 30 seconds".into());
    }
    let allocation = native::receipt(campaign)?;
    if campaign.purpose == CampaignPurpose::Qualification && allocation.is_none() {
        return Err("qualification requires launcher-owned native allocation receipt".into());
    }
    let configuration = std::env::var_os("RRFLOW_SUSTAINED_CONFIGURATION").map(PathBuf::from);
    let clock = fleet.map_or_else(Instant::now, |fleet| fleet.clock);
    for point in &campaign.points {
        for seed in &campaign.seeds {
            writer.admit_next()?;
            let estate = Estate::install_configured("fresh", configuration.as_deref());
            if estate.build["executable_sha256"].as_str()
                != Some(campaign.storage.candidate.executable_sha256.as_str())
                || estate.plan["configuration"]["configuration_sha256"].as_str()
                    != Some(campaign.storage.candidate.configuration_sha256.as_str())
            {
                return Err("installed binary/configuration differs from frozen campaign".into());
            }
            let lifetime_ms = [
                LoadPhase::Warmup,
                LoadPhase::Measured,
                LoadPhase::Burst,
                LoadPhase::Drain,
                LoadPhase::Soak,
            ]
            .into_iter()
            .try_fold(
                campaign
                    .schedule
                    .setup_ms
                    .get()
                    .checked_add(campaign.schedule.lifecycle_ms.get())
                    .and_then(|v| v.checked_add(600_000))
                    .ok_or("setup deadline overflow")?,
                |n, p| n.checked_add(campaign.schedule.duration_ms(p)),
            )
            .ok_or("campaign deadline overflow")?
            .checked_add(fleet.map_or(0, |f| f.quiet_extra_ms(campaign.schedule.measured_ms.get())))
            .ok_or("fleet isolated baseline deadline overflow")?;
            let (child, client, _) = estate.start_for(clock, Duration::from_millis(lifetime_ms));
            native::enforcement(child.id(), allocation.as_ref())?;
            let mut child = Some(child);
            let mut session_ordinal = 0;
            let mut session_opened = Instant::now();
            let mut work = session(
                &estate,
                client,
                *seed,
                campaign,
                point.clients.get(),
                session_ordinal,
            )
            .await?;
            let mut corpus = Some(
                corpus::seed(
                    &estate,
                    &mut work,
                    campaign,
                    &ProgressContext { writer, point },
                    clock,
                    &mut session_ordinal,
                    &mut session_opened,
                )
                .await?,
            );
            // Fresh session for the bounded lifecycle lane, avoiding inherited
            // setup age. Subsequent load still rotates joined session segments.
            session_opened = Instant::now() - Duration::from_secs(120);
            rotate_session(
                &estate,
                &mut work,
                campaign,
                point.clients.get(),
                &mut session_ordinal,
                &mut session_opened,
            )
            .await?;
            let mut reopens = Some(
                lifecycle::reopens(
                    &estate,
                    &mut child,
                    &mut work,
                    campaign,
                    clock,
                    Duration::from_millis(lifetime_ms),
                    &ProgressContext { writer, point },
                )
                .await?,
            );
            let mut cold_start = if campaign.required_lanes.contains(&CapacityLane::Cold) {
                Some(
                    cold::run(
                        &estate,
                        &mut child,
                        &mut work,
                        campaign,
                        clock,
                        Duration::from_millis(lifetime_ms),
                        &ProgressContext { writer, point },
                    )
                    .await?,
                )
            } else {
                None
            };
            let mut driver = PointRun {
                campaign,
                point,
                workload: work,
                pid: child
                    .as_ref()
                    .ok_or("owned server missing after lifecycle lane")?
                    .id(),
                clock,
                allocation: allocation.clone(),
                generations: vec![0; campaign.evidence.segment_attempts.get() as usize],
            };
            for phase in [
                LoadPhase::Warmup,
                LoadPhase::Measured,
                LoadPhase::Burst,
                LoadPhase::Drain,
                LoadPhase::Soak,
            ] {
                let mut phase_origin = if let Some(fleet) = fleet {
                    rotate_session(
                        &estate,
                        &mut driver.workload,
                        campaign,
                        point.clients.get(),
                        &mut session_ordinal,
                        &mut session_opened,
                    )
                    .await?;
                    let stage = match phase {
                        LoadPhase::Warmup => "warmup",
                        LoadPhase::Measured => "measured",
                        LoadPhase::Burst => "burst",
                        LoadPhase::Drain => "drain",
                        LoadPhase::Soak => "soak",
                    };
                    Some(
                        fleet
                            .boundary(
                                stage,
                                &estate,
                                &mut driver,
                                &mut session_ordinal,
                                &mut session_opened,
                            )
                            .await?,
                    )
                } else {
                    None
                };
                if phase == LoadPhase::Measured {
                    if let Some(fleet) = fleet.filter(|f| f.trial == FleetTrial::Quiet) {
                        fleet
                            .wait_quiet(
                                fleet.estate,
                                &mut driver,
                                writer,
                                (&estate, &mut session_ordinal, &mut session_opened),
                            )
                            .await?;
                        phase_origin = Some(ns(clock));
                    }
                }
                if phase == LoadPhase::Drain
                    || (phase != LoadPhase::Warmup
                        && fleet.is_some_and(|f| {
                            !f.trial.active(f.estate)
                                && !(phase == LoadPhase::Measured && f.trial == FleetTrial::Quiet)
                        }))
                {
                    let mut remaining = campaign.schedule.duration_ms(phase);
                    while remaining > 0 {
                        writer.admit_next()?;
                        rotate_session(
                            &estate,
                            &mut driver.workload,
                            campaign,
                            point.clients.get(),
                            &mut session_ordinal,
                            &mut session_opened,
                        )
                        .await?;
                        let duration = campaign.idle_segment_ms(remaining)?;
                        writer.append(&driver.drain(writer, duration).await?)?;
                        remaining -= duration;
                    }
                    writer.verify_drain(&point.id, *seed)?;
                    continue;
                }
                let rate =
                    point.requests_per_second.get() * if phase == LoadPhase::Burst { 2 } else { 1 };
                let count =
                    (campaign.schedule.duration_ms(phase) as u128 * rate as u128).div_ceil(1000);
                let count: u64 = count.try_into().map_err(|_| "offered count overflow")?;
                let origin = phase_origin.unwrap_or_else(|| ns(clock));
                let mut next = 0;
                while next < count {
                    writer.admit_next()?;
                    rotate_session(
                        &estate,
                        &mut driver.workload,
                        campaign,
                        point.clients.get(),
                        &mut session_ordinal,
                        &mut session_opened,
                    )
                    .await?;
                    let active_ms = campaign
                        .storage
                        .budgets
                        .run_ms
                        .get()
                        .checked_sub(campaign.schedule.operation_deadline_ms.get())
                        .filter(|v| *v > 0)
                        .ok_or("segment must reserve an operation deadline")?
                        .min(
                            if campaign
                                .required_lanes
                                .contains(&CapacityLane::ReaderPinInterference)
                            {
                                10_000
                            } else {
                                120_000
                            },
                        );
                    let timed_cap = (u128::from(rate) * u128::from(active_ms) / 1000)
                        .max(1)
                        .min(u128::from(u64::MAX)) as u64;
                    let take = (count - next)
                        .min(campaign.evidence.segment_attempts.get())
                        .min(timed_cap)
                        .min(
                            campaign
                                .storage
                                .budgets
                                .retained_bytes
                                .get()
                                .min(campaign.allocation.engine_bytes.get() / 4)
                                .saturating_sub(
                                    (driver.generations.len() as u64).saturating_mul(8),
                                )
                                / driver.workload.retained_request_bound().saturating_mul(2),
                        );
                    if take == 0 {
                        return Err("prepared corpus exceeds retained driver admission".into());
                    }
                    let mut segment = tokio::time::timeout(
                        Duration::from_millis(campaign.storage.budgets.run_ms.get().min(180_000)),
                        driver.segment(writer, phase, origin, next, take),
                    )
                    .await
                    .map_err(|_| "installed segment exceeded its frozen deadline")??;
                    if phase == LoadPhase::Warmup && next == 0 {
                        segment.corpus = corpus.take();
                        segment.cold_start = cold_start.take();
                        if let Some(raw) = &segment.cold_start {
                            segment.lanes.push(LaneEvidence {
                                lane: CapacityLane::Cold,
                                repetitions: 1u64.try_into()?,
                                receipt_sha256: digest("c07g.cold-start", raw)?,
                                acknowledged_readback_verified: true,
                            });
                        }
                        segment.reopens = reopens.take();
                        if let Some(raw) = &segment.reopens {
                            segment.lanes.push(LaneEvidence {
                                lane: CapacityLane::ForcedReopens,
                                repetitions: (raw.cycles.len() as u64).try_into()?,
                                receipt_sha256: digest("c07g.reopen-evidence", raw)?,
                                acknowledged_readback_verified: true,
                            });
                        }
                    }
                    writer.append(&segment)?;
                    next += take;
                }
                if phase == LoadPhase::Measured {
                    if let Some(fleet) = fleet.filter(|f| f.trial == FleetTrial::Quiet) {
                        let all = fleet.finish_quiet()?;
                        fleet
                            .wait_quiet(
                                all,
                                &mut driver,
                                writer,
                                (&estate, &mut session_ordinal, &mut session_opened),
                            )
                            .await?;
                    }
                }
            }
            if let Some(fleet) = fleet {
                rotate_session(
                    &estate,
                    &mut driver.workload,
                    campaign,
                    point.clients.get(),
                    &mut session_ordinal,
                    &mut session_opened,
                )
                .await?;
                fleet
                    .boundary(
                        "loaded",
                        &estate,
                        &mut driver,
                        &mut session_ordinal,
                        &mut session_opened,
                    )
                    .await?;
            }
            // Reopen after offered load and verify the final generated state
            // before any retry can rewrite it. Progress retains incomplete runs.
            tokio::time::timeout(
                Duration::from_millis(campaign.schedule.lifecycle_ms.get()),
                async {
                    let started_ns = ns(clock);
                    let killed = super::kill(child.take().ok_or("missing final server")?, clock);
                    let (reopened, client, _) =
                        estate.start_for(clock, Duration::from_millis(lifetime_ms));
                    let reopened_pid = reopened.id();
                    native::enforcement(reopened_pid, allocation.as_ref())?;
                    driver.pid = reopened_pid;
                    child = Some(reopened);
                    driver.workload.client = client;
                    session_opened = Instant::now() - Duration::from_secs(120);
                    rotate_session(
                        &estate,
                        &mut driver.workload,
                        campaign,
                        point.clients.get(),
                        &mut session_ordinal,
                        &mut session_opened,
                    )
                    .await?;
                    let mut states = digest("c07g.final-state", &(*seed, point.id.clone()))?;
                    for record in 0..driver.workload.records {
                        rotate_session(
                            &estate,
                            &mut driver.workload,
                            campaign,
                            point.clients.get(),
                            &mut session_ordinal,
                            &mut session_opened,
                        )
                        .await?;
                        let operation = driver.workload.seed_record(record)?;
                        driver.workload.verify(&operation).await?;
                        states = digest("c07g.final-state", &(states, operation.effect_sha256))?;
                    }
                    let mut working_slots = 0;
                    for (slot, generation) in driver
                        .generations
                        .iter()
                        .enumerate()
                        .filter(|(_, n)| **n > 0)
                    {
                        rotate_session(
                            &estate,
                            &mut driver.workload,
                            campaign,
                            point.clients.get(),
                            &mut session_ordinal,
                            &mut session_opened,
                        )
                        .await?;
                        let operation = driver.workload.prepare_load(
                            "final-readback".into(),
                            WriteTransition {
                                slot: slot as u64,
                                generation: generation - 1,
                            },
                        )?;
                        driver.workload.verify(&operation).await?;
                        states = digest("c07g.final-state", &(states, operation.effect_sha256))?;
                        working_slots += 1;
                    }
                    writer.record_event(&ProgressEvent::FinalReadback {
                        point: point.id.clone(),
                        seed: *seed,
                        killed_pid: killed["pid"]
                            .as_u64()
                            .ok_or("final killed PID missing")?
                            .try_into()
                            .map_err(|_| "final PID overflow")?,
                        reopened_pid,
                        seed_records: driver.workload.records,
                        working_slots,
                        states_sha256: states,
                        started_ns,
                        finished_ns: ns(clock),
                    })?;
                    writer.checkpoint_progress()?;
                    Ok::<(), String>(())
                },
            )
            .await
            .map_err(|_| "final readback exceeded lifecycle deadline")??;
            if let Some(fleet) = fleet {
                rotate_session(
                    &estate,
                    &mut driver.workload,
                    campaign,
                    point.clients.get(),
                    &mut session_ordinal,
                    &mut session_opened,
                )
                .await?;
                fleet
                    .boundary(
                        "reopened",
                        &estate,
                        &mut driver,
                        &mut session_ordinal,
                        &mut session_opened,
                    )
                    .await?;
            }
            drop(driver);
            drop(child);
        }
    }
    Ok(())
}
