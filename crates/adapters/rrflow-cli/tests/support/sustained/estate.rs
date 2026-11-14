//! Running the estate under a sustained campaign.

use super::*;

pub(super) async fn run(
    campaign: &StorageCampaign,
    writer: &mut CampaignWriter,
) -> DriverResult<()> {
    run_estate(campaign, writer, None).await
}
pub(super) async fn run_estate(
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
            let seeded = corpus::seed(
                &estate,
                &mut work,
                campaign,
                &ProgressContext { writer, point },
                clock,
                &mut session_ordinal,
                &mut session_opened,
            )
            .await;
            if seeded.is_err() {
                // Preserve a single supplemental observation before stopping
                // the owned server. This is not a workload
                // segment and cannot qualify the campaign or hide its error.
                let status = if let Some(child) = &child {
                    match tokio::time::timeout(
                        Duration::from_secs(5),
                        sample(&work, child.id(), clock, &allocation),
                    )
                    .await
                    {
                        Ok(Ok(observation)) => {
                            if writer
                                .record_event(&ProgressEvent::Resource {
                                    segment: writer.next_index(),
                                    sample: Box::new(observation),
                                })
                                .and_then(|_| writer.checkpoint_progress())
                                .is_ok()
                            {
                                "observed"
                            } else {
                                "artifact-denied"
                            }
                        }
                        Ok(Err(_)) => "unavailable",
                        Err(_) => "timeout",
                    }
                } else {
                    "unavailable"
                };
                eprintln!("native_setup_failure_observation={status}");
                if let Some(mut stopped) = child.take() {
                    stopped.terminate();
                    // finish joins the monitor and reaps the child even when
                    // bounded output capture reports a failure. Drop remains
                    // the process guard for an early monitor error.
                    let _ = stopped.finish();
                    drop(stopped);
                }
                eprintln!(
                    "native_setup_failure_estate={}",
                    serde_json::json!({"path": estate.temp.retain(), "private_state": true})
                );
            }
            let mut corpus = Some(seeded?);
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
            let reopens_result = lifecycle::reopens(
                &estate,
                &mut child,
                &mut work,
                campaign,
                (clock, Duration::from_millis(lifetime_ms)),
                &ProgressContext { writer, point },
                (&mut session_ordinal, &mut session_opened),
            )
            .await;
            let mut reopens = Some(retain_failed_lifecycle(&estate.temp, reopens_result)?);
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
                previous_resources: Vec::new(),
                backup: management::ManagementTask::new(),
                sample_gate: None,
            };
            for phase in [
                LoadPhase::Warmup,
                LoadPhase::Measured,
                LoadPhase::Burst,
                LoadPhase::Drain,
                LoadPhase::Soak,
            ] {
                if phase == LoadPhase::Soak {
                    writer.admit_soak(&point.id, *seed)?;
                }
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
                    if !driver.backup.is_pending() {
                        rotate_session(
                            &estate,
                            &mut driver.workload,
                            campaign,
                            point.clients.get(),
                            &mut session_ordinal,
                            &mut session_opened,
                        )
                        .await?;
                    }
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
                    let result = tokio::time::timeout(
                        Duration::from_millis(campaign.storage.budgets.run_ms.get().min(180_000)),
                        driver.segment(writer, phase, origin, next, take, next + take == count),
                    )
                    .await
                    .map_err(|_| "installed segment exceeded its frozen deadline".to_owned())
                    .and_then(std::convert::identity);
                    let cleanup = driver.workload.native_worker.shutdown().await;
                    let mut segment = match (result, cleanup) {
                        (Ok(value), Ok(())) => value,
                        (Err(error), Ok(())) | (Ok(_), Err(error)) => return Err(error),
                        (Err(error), Err(cleanup)) => {
                            return Err(format!("{error}; native worker cleanup: {cleanup}"))
                        }
                    };
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
                    let killed =
                        super::super::kill(child.take().ok_or("missing final server")?, clock);
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
