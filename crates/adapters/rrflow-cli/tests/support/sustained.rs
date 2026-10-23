//! Ordinary installed campaign. Explicit environment admission keeps a long
//! campaign out of ordinary correctness CI; failure artifacts are never green.
use super::Estate;
use rrd_client::RequestOptions;
use rrd_contract::{CreateSession, ReadDiagnosticSnapshot, SessionLimits};
use rrflow_eval::storage::*;
use std::{
    io::Read,
    path::PathBuf,
    time::{Duration, Instant},
};
use tokio::task::JoinSet;
#[path = "sustained/native.rs"]
mod native;
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
    dimensions: u32,
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
        dimensions,
        seed,
    })
}

async fn sample(workload: &Workload, pid: u32, clock: Instant) -> DriverResult<ResourceSample> {
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
        .read_diagnostic_snapshot(
            &workload.session,
            ReadDiagnosticSnapshot {
                scope: format!("instance:{}", workload.client.instance_id()),
                graph_valid_at_unix_ms: workload.valid_at,
                graph_known_at_cursor: None,
                graph_compare_cursor: 0,
                runtime_max_scanned_changes: 1_000_000,
                changes_after_cursor: 0,
                change_limit: 1,
                audit_after_sequence: 0,
                audit_limit: 1,
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
        retained_bytes: match &snapshot.storage.admission {
            Some(value) => NativeMetric::Observed {
                value: value
                    .retained_mutable_bytes
                    .checked_add(value.retained_projected_bytes)
                    .ok_or("retained sum overflow")?,
            },
            None => missing("public-retained-gauge-unavailable"),
        },
        queue_requests: match &snapshot.storage.admission {
            Some(value) => NativeMetric::Observed {
                value: value.outstanding_requests,
            },
            None => missing("public-queue-gauge-unavailable"),
        },
        maintenance_debt_segments: match &snapshot.storage.rrflow_kv {
            Some(value) => NativeMetric::Observed {
                value: value.segment_set.compaction_debt_segments,
            },
            None => missing("public-debt-gauge-unavailable"),
        },
        allocation_enforcement: native::enforcement(),
        storage: Some(snapshot.storage),
    })
}

struct PointRun<'a> {
    campaign: &'a StorageCampaign,
    point: &'a OperatingPoint,
    workload: Workload,
    pid: u32,
    clock: Instant,
}

impl PointRun<'_> {
    async fn drain(
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
            resources.push(sample(&self.workload, self.pid, self.clock).await?);
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
        self.workload.verify(&baseline).await?;
        let ledger = AcknowledgementLedger {
            expected: vec![expected],
            attempts: vec![],
            readback: vec![Readback::Present {
                logical_id: evidence_id(baseline.id),
                request_sha256: baseline.request_sha256,
                effect_sha256: baseline.effect_sha256.clone(),
                receipt_sha256: digest("c07g.drain-readback", &baseline.effect_sha256)?,
                applications: 1u64.try_into().unwrap(),
            }],
        };
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
            manifest,
            result,
            attempts: vec![],
            resources,
            lanes: vec![],
        })
    }
    async fn segment(
        &self,
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
            let operation = self.workload.prepare(
                format!("load-{}-{sequence}", writer.next_index()),
                storage_load_kind(sequence),
                sequence,
            )?;
            ledger.expected.push(ExpectedOperation {
                logical_id: evidence_id(&operation.id),
                request_sha256: operation.request_sha256.clone(),
                effect_sha256: operation.effect_sha256.clone(),
                resolution: ExpectedResolution::Either,
            });
            prepared.push(operation);
        }
        let mut manifest = campaign.storage.clone();
        manifest.required_cases = vec![RequiredCase {
            id: evidence_id(&key),
            operation: evidence_id("transaction-commit"),
            seed: self.workload.seed,
            expected_ledger_sha256: expected_digest(&ledger.expected)?,
        }];
        let mut resources = vec![sample(&self.workload, self.pid, self.clock).await?];
        let mut attempts = Vec::new();
        let mut joins = JoinSet::new();
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
                        collect(result.ok_or("missing scheduled task")?.map_err(|_|"scheduled task panicked")?,started,&mut attempts,&mut ledger)?;
                    }
                    _=tokio::time::sleep_until(next_sample) => {
                        if resources.len() as u64>=campaign.evidence.samples_per_segment.get() {return Err("resource sample cap exhausted".into());}
                        resources.push(sample(&self.workload,self.pid,self.clock).await?);
                        next_sample=tokio::time::Instant::now()+Duration::from_millis(campaign.schedule.sample_ms.get());
                    }
                    _=tokio::time::sleep_until(due.into()) => break,
                }
            }
            let work = self.workload.clone();
            let operation = operation.clone();
            let clock = self.clock;
            if joins.len() as u64 >= self.point.clients.get() {
                let at = ns(clock);
                collect(
                    (sequence, scheduled(sequence), at, at, operation, None, true),
                    started,
                    &mut attempts,
                    &mut ledger,
                )?;
            } else {
                let at = scheduled(sequence);
                joins.spawn(async move {
                    let began = ns(clock);
                    let result = tokio::time::timeout(deadline, work.execute(&operation))
                        .await
                        .ok()
                        .and_then(std::result::Result::ok);
                    (sequence, at, began, ns(clock), operation, result, false)
                });
            }
        }
        while !joins.is_empty() {
            tokio::select! {
                result=joins.join_next() => collect(result.ok_or("missing scheduled task")?.map_err(|_|"scheduled task panicked")?,started,&mut attempts,&mut ledger)?,
                _=tokio::time::sleep_until(next_sample) => {
                    if resources.len() as u64>=campaign.evidence.samples_per_segment.get(){return Err("resource sample cap exhausted".into());}
                    resources.push(sample(&self.workload,self.pid,self.clock).await?);
                    next_sample=tokio::time::Instant::now()+Duration::from_millis(campaign.schedule.sample_ms.get());
                }
            }
        }
        let mut verified = true;
        for operation in &prepared {
            if tokio::time::Instant::now() >= next_sample {
                if resources.len() as u64 + 1 >= campaign.evidence.samples_per_segment.get() {
                    return Err("readback resource sample cap exhausted".into());
                }
                resources.push(sample(&self.workload, self.pid, self.clock).await?);
                next_sample = tokio::time::Instant::now()
                    + Duration::from_millis(campaign.schedule.sample_ms.get());
            }
            if let Some(attempt) = attempts
                .iter()
                .find(|a| a.logical_id.as_str() == operation.id)
            {
                match attempt.outcome {
                    LoadOutcome::Acknowledged => {
                        if self.workload.verify(operation).await.is_ok() {
                            ledger.readback.push(Readback::Present {
                                logical_id: attempt.logical_id.clone(),
                                request_sha256: operation.request_sha256.clone(),
                                effect_sha256: operation.effect_sha256.clone(),
                                receipt_sha256: attempt.result_sha256.clone(),
                                applications: 1u64.try_into().unwrap(),
                            });
                        } else {
                            verified = false;
                        }
                    }
                    LoadOutcome::Rejected => ledger.readback.push(Readback::Absent {
                        logical_id: attempt.logical_id.clone(),
                    }),
                    _ => verified = false,
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
            resources.push(sample(&self.workload, self.pid, self.clock).await?);
        }
        resources.push(sample(&self.workload, self.pid, self.clock).await?);
        attempts.sort_by_key(|a| a.sequence);
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
                faults: vec![],
                limitation: if verified {
                    None
                } else {
                    Some(evidence_id("independent-readback-failed"))
                },
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
            phase,
            started_ns: started,
            finished_ns: ns(self.clock),
            manifest,
            result,
            attempts,
            resources,
            lanes: vec![],
        })
    }
}

type Completion = (
    u64,
    u64,
    u64,
    u64,
    Prepared,
    Option<workload::Completed>,
    bool,
);
fn collect(
    (sequence, scheduled_ns, started_ns, finished_ns, operation, result, rejected): Completion,
    segment_start: u64,
    attempts: &mut Vec<LoadAttempt>,
    ledger: &mut AcknowledgementLedger,
) -> DriverResult<()> {
    let (outcome, result_sha256, recorded) = match result {
        Some(result) => (
            LoadOutcome::Acknowledged,
            result.receipt_sha256.clone(),
            AttemptOutcome::Acknowledged {
                receipt_sha256: result.receipt_sha256,
            },
        ),
        None if rejected => (
            LoadOutcome::Rejected,
            digest("c07g.saturated", &sequence)?,
            AttemptOutcome::Rejected {
                reason: evidence_id("driver-concurrency-limit"),
            },
        ),
        None => (
            LoadOutcome::Uncertain,
            digest("c07g.uncertain", &sequence)?,
            AttemptOutcome::Uncertain {
                reason: evidence_id("request-failed-or-deadline"),
            },
        ),
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
        scheduled_ns,
        started_ns,
        finished_ns,
        outcome,
        logical_id: evidence_id(operation.id),
        request_sha256: operation.request_sha256,
        result_sha256,
    });
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
    let mut writer = CampaignWriter::create(&evidence, campaign.clone()).unwrap();
    let result = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(run(&campaign, &mut writer));
    let (status, limitations) = if result.is_ok() {
        (CampaignStatus::Completed, vec![])
    } else {
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
}

async fn run(campaign: &StorageCampaign, writer: &mut CampaignWriter) -> DriverResult<()> {
    if campaign.allocation.estates.get() != 1 {
        return Err("fleet requires native aggregate estate driver".into());
    }
    if campaign.purpose == CampaignPurpose::Qualification {
        return Err("native allocation enforcement and complete resource gauges required before qualification execution".into());
    }
    let configuration = std::env::var_os("RRFLOW_SUSTAINED_CONFIGURATION").map(PathBuf::from);
    let clock = Instant::now();
    for point in &campaign.points {
        for seed in &campaign.seeds {
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
            .try_fold(600_000u64, |n, p| {
                n.checked_add(campaign.schedule.duration_ms(p))
            })
            .ok_or("campaign deadline overflow")?;
            let (child, client, _) = estate.start_for(clock, Duration::from_millis(lifetime_ms));
            let work = session(
                &estate,
                client,
                *seed,
                campaign
                    .allocation
                    .vector_dimensions
                    .get()
                    .try_into()
                    .map_err(|_| "vector dimensions overflow")?,
                point.clients.get(),
                writer.next_index(),
            )
            .await?;
            work.initialize().await?;
            // Characterization is currently the one-record linked corpus. Reject a
            // larger frozen allocation rather than label it as having been seeded.
            if campaign.allocation.records.get() != 1
                || campaign.allocation.edges.get() != 1
                || campaign.allocation.vectors.get() != 1
            {
                return Err(
                    "requested profile corpus has not been seeded by this characterization driver"
                        .into(),
                );
            }
            let driver = PointRun {
                campaign,
                point,
                workload: work,
                pid: child.id(),
                clock,
            };
            for phase in [
                LoadPhase::Warmup,
                LoadPhase::Measured,
                LoadPhase::Burst,
                LoadPhase::Drain,
                LoadPhase::Soak,
            ] {
                if phase == LoadPhase::Drain {
                    let mut remaining = campaign.schedule.drain_ms.get();
                    let cap = campaign
                        .schedule
                        .sample_ms
                        .get()
                        .checked_mul(
                            campaign
                                .evidence
                                .samples_per_segment
                                .get()
                                .saturating_sub(2),
                        )
                        .filter(|v| *v > 0)
                        .ok_or("drain sample capacity must exceed two")?;
                    while remaining > 0 {
                        writer.admit_next()?;
                        let duration = remaining.min(cap);
                        writer.append(&driver.drain(writer, duration).await?)?;
                        remaining -= duration;
                    }
                    continue;
                }
                let rate =
                    point.requests_per_second.get() * if phase == LoadPhase::Burst { 2 } else { 1 };
                let count =
                    (campaign.schedule.duration_ms(phase) as u128 * rate as u128).div_ceil(1000);
                let count: u64 = count.try_into().map_err(|_| "offered count overflow")?;
                let origin = ns(clock);
                let mut next = 0;
                while next < count {
                    writer.admit_next()?;
                    let active_ms = campaign
                        .storage
                        .budgets
                        .run_ms
                        .get()
                        .checked_sub(campaign.schedule.operation_deadline_ms.get())
                        .filter(|v| *v > 0)
                        .ok_or("segment must reserve an operation deadline")?;
                    let timed_cap = (u128::from(rate) * u128::from(active_ms) / 1000)
                        .max(1)
                        .min(u128::from(u64::MAX)) as u64;
                    let take = (count - next)
                        .min(campaign.evidence.segment_attempts.get())
                        .min(timed_cap);
                    let segment = driver.segment(writer, phase, origin, next, take).await?;
                    writer.append(&segment)?;
                    next += take;
                }
            }
            drop(driver);
            drop(child);
        }
    }
    Ok(())
}
