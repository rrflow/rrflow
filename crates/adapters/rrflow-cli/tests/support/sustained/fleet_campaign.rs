//! Bounded coordinator over the existing single-estate campaign implementation.
use super::super::*;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Mutex,
};
use tokio::sync::Barrier;

pub(super) struct Shared {
    barrier: Barrier,
    works: Mutex<Vec<Option<Workload>>>,
    boundaries: Mutex<Vec<FleetBoundary>>,
    origin: Mutex<u64>,
    quiet_turn: AtomicU64,
    estates: u64,
}
#[derive(Clone)]
pub(crate) struct Participation {
    shared: Arc<Shared>,
    pub clock: Instant,
    pub estate: u64,
    pub trial: FleetTrial,
    wait: Duration,
}
impl Participation {
    pub fn quiet_extra_ms(&self, measured_ms: u64) -> u64 {
        if self.trial == FleetTrial::Quiet {
            (self.shared.estates - 1) * (measured_ms + 120_000)
        } else {
            0
        }
    }
    pub async fn wait_quiet(
        &self,
        target: u64,
        driver: &mut PointRun<'_>,
        writer: &mut CampaignWriter,
        lease: (&Estate, &mut u64, &mut Instant),
    ) -> DriverResult<()> {
        let (estate, ordinal, opened) = lease;
        while self.shared.quiet_turn.load(Ordering::Acquire) < target {
            rotate_session(
                estate,
                &mut driver.workload,
                driver.campaign,
                driver.point.clients.get(),
                ordinal,
                opened,
            )
            .await?;
            writer.admit_next()?;
            let duration = driver
                .campaign
                .idle_segment_ms(driver.campaign.schedule.measured_ms.get())?;
            writer.append(&driver.drain(writer, duration).await?)?;
        }
        Ok(())
    }
    pub fn finish_quiet(&self) -> DriverResult<u64> {
        self.shared
            .quiet_turn
            .compare_exchange(
                self.estate,
                self.estate + 1,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .map_err(|_| "isolated estate completed out of order")?;
        Ok(self.shared.estates)
    }
    /// All engines finish the previous phase before a new phase is admitted.
    /// Session credentials stay in process memory and are never serialized.
    pub async fn boundary(
        &self,
        stage: &str,
        estate: &Estate,
        driver: &mut PointRun<'_>,
        ordinal: &mut u64,
        opened: &mut Instant,
    ) -> DriverResult<u64> {
        // A slow neighbour may still be ingesting or reopening. Rotate this
        // joined member's lease while waiting so expiry cannot masquerade as
        // a foreign-session isolation result.
        let ready = tokio::time::timeout(self.wait, self.shared.barrier.wait());
        tokio::pin!(ready);
        loop {
            tokio::select! {
                result=&mut ready => {result.map_err(|_|"fleet phase readiness deadline")?;break;},
                _=tokio::time::sleep(Duration::from_secs(60)) => {
                    rotate_session(estate,&mut driver.workload,driver.campaign,driver.point.clients.get(),ordinal,opened).await?;
                }
            }
        }
        let sample = sample(&driver.workload, driver.pid, self.clock, &driver.allocation).await?;
        self.shared
            .works
            .lock()
            .map_err(|_| "fleet session registry poisoned")?[self.estate as usize] =
            Some(driver.workload.clone());
        tokio::time::timeout(self.wait, self.shared.barrier.wait())
            .await
            .map_err(|_| "fleet ready barrier deadline")?;
        let foreign = {
            let works = self
                .shared
                .works
                .lock()
                .map_err(|_| "fleet session registry poisoned")?;
            works[(self.estate as usize + 1) % works.len()]
                .clone()
                .ok_or("foreign fleet session missing")?
        };
        let response = tokio::time::timeout(
            Duration::from_secs(30),
            driver.workload.client.read_storage_diagnostics(
                &foreign.session,
                ReadStorageDiagnostics {
                    scope: format!("instance:{}", driver.workload.client.instance_id()),
                },
                RequestOptions::read("fleet-foreign-session", "fleet-foreign-session")
                    .map_err(|e| e.to_string())?,
            ),
        )
        .await
        .map_err(|_| "foreign-session denial deadline")?;
        let denial = match response {
            Err(rrd_client::Error::Api { error, .. })
                if matches!(
                    error.code,
                    rrd_contract::ErrorCode::Unauthenticated
                        | rrd_contract::ErrorCode::PermissionDenied
                        | rrd_contract::ErrorCode::NotFound
                ) =>
            {
                error.code
            }
            _ => {
                return Err(
                    "foreign fleet session did not receive explicit authorization denial".into(),
                )
            }
        };
        let boundary = FleetBoundary {
            estate: self.estate,
            stage: evidence_id(stage),
            instance: driver.workload.client.instance_id().to_string(),
            foreign_instance: foreign.client.instance_id().to_string(),
            denial,
            sample,
        };
        {
            let mut rows = self
                .shared
                .boundaries
                .lock()
                .map_err(|_| "fleet boundary registry poisoned")?;
            if rows.len() >= 70 {
                return Err("fleet boundary count exceeded".into());
            }
            rows.push(boundary);
        }
        let ready = tokio::time::timeout(self.wait, self.shared.barrier.wait())
            .await
            .map_err(|_| "fleet authorization barrier deadline")?;
        if ready.is_leader() {
            *self
                .shared
                .origin
                .lock()
                .map_err(|_| "fleet origin poisoned")? = ns(self.clock);
        }
        tokio::time::timeout(self.wait, self.shared.barrier.wait())
            .await
            .map_err(|_| "fleet origin barrier deadline")?;
        Ok(*self
            .shared
            .origin
            .lock()
            .map_err(|_| "fleet origin poisoned")?)
    }
}

pub(crate) fn execute(campaign: StorageCampaign, evidence: PathBuf) {
    let mut writer =
        FleetWriter::create(&evidence, campaign.clone()).expect("bounded fleet manifest admission");
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(campaign.allocation.estates.get() as usize + 2)
            .max_blocking_threads(campaign.allocation.estates.get() as usize)
            .enable_all()
            .build()
            .unwrap()
            .block_on(run_fleet(&campaign, &evidence, &mut writer))
    }))
    .unwrap_or_else(|_| Err("fleet driver panicked; raw child progress retained".into()));
    let status = if result.is_ok() {
        CampaignStatus::Completed
    } else {
        CampaignStatus::Failed
    };
    let report = writer.finish(status).expect("seal bounded fleet result");
    println!(
        "native_fleet_result={}",
        serde_json::to_string(&report).unwrap()
    );
    assert!(result.is_ok(), "installed fleet failed: {result:?}");
    assert!(
        campaign.purpose != CampaignPurpose::Qualification || report.qualification_passed,
        "fleet completed without satisfying qualification requirements"
    );
}

async fn run_fleet(
    campaign: &StorageCampaign,
    evidence: &std::path::Path,
    writer: &mut FleetWriter,
) -> DriverResult<()> {
    validate_fleet(campaign)?;
    // This receipt is checked again against actual membership for every engine
    // before seeding, at every resource sample, and after each process reopen.
    let allocation = native::receipt(campaign)?;
    if campaign.purpose == CampaignPurpose::Qualification && allocation.is_none() {
        return Err("fleet qualification requires native aggregate allocation".into());
    }
    let mut index = 0;
    for point in 0..campaign.points.len() {
        for seed in 0..campaign.seeds.len() {
            for trial in FleetTrial::ALL {
                let n = campaign.allocation.estates.get() as usize;
                let shared = Arc::new(Shared {
                    barrier: Barrier::new(n),
                    works: Mutex::new(vec![None; n]),
                    boundaries: Mutex::new(Vec::with_capacity(n * 7)),
                    origin: Mutex::new(0),
                    quiet_turn: AtomicU64::new(0),
                    estates: n as u64,
                });
                let clock = Instant::now();
                let mut tasks = JoinSet::new();
                for estate in 0..n {
                    let child = fleet_child(campaign, point, seed, trial, estate as u64)?;
                    let directory = fleet_member_directory(evidence, index, estate as u64);
                    let mut child_writer = CampaignWriter::create(&directory, child.clone())?;
                    let participant = Participation {
                        shared: shared.clone(),
                        clock,
                        estate: estate as u64,
                        trial,
                        wait: Duration::from_millis(
                            campaign.schedule.setup_ms.get()
                                + campaign.schedule.lifecycle_ms.get()
                                + 600_000,
                        ),
                    };
                    tasks.spawn(async move {
                        let result =
                            run_estate(&child, &mut child_writer, Some(&participant)).await;
                        let (status, limitations) = if result.is_ok() {
                            (CampaignStatus::Completed, vec![])
                        } else {
                            let _ = child_writer.record_event(&ProgressEvent::Failure {
                                reason: evidence_id("fleet-member-failed"),
                            });
                            (
                                CampaignStatus::Failed,
                                vec![evidence_id("fleet-member-failed")],
                            )
                        };
                        let report = child_writer.finish(status, limitations)?;
                        result?;
                        Ok::<_, String>((estate, digest("rrflow-fleet-child-result-v1", &report)?))
                    });
                }
                let mut receipts = vec![None; n];
                let mut failure = None;
                while let Some(completed) = tasks.join_next().await {
                    match completed {
                        Ok(Ok((estate, hash))) => receipts[estate] = Some(hash),
                        Ok(Err(error)) => {
                            failure = Some(error);
                            tasks.abort_all();
                        }
                        Err(error) => {
                            if failure.is_none() {
                                failure = Some(format!("fleet member failed: {error}"));
                            }
                            tasks.abort_all();
                        }
                    }
                }
                // Aborted members retain their immutable campaign and raw progress; a
                // missing sealed child can never become successful fleet evidence.
                let boundaries = shared
                    .boundaries
                    .lock()
                    .map_err(|_| "fleet boundary registry poisoned")?
                    .clone();
                writer.append(FleetTrialResult {
                    format_version: 1,
                    campaign_sha256: campaign.sha256()?,
                    index,
                    point,
                    seed,
                    trial,
                    status: if failure.is_some() {
                        CampaignStatus::Failed
                    } else {
                        CampaignStatus::Completed
                    },
                    child_results: receipts.into_iter().flatten().collect(),
                    boundaries,
                    sum_member_peak_resident_bytes: 0,
                    measured_latency: vec![],
                    qualification_passed: false,
                })?;
                if let Some(error) = failure {
                    return Err(error);
                }
                index += 1;
            }
        }
    }
    Ok(())
}
