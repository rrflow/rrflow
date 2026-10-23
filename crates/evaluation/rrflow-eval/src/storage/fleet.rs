//! Independent-estate interference over the canonical storage campaign oracle.
//! Child campaigns are mechanical views; only the aggregate can qualify a fleet.
use super::sustained::{encode_bounded, read_bounded, write_new};
use super::*;
#[path = "fleet_latency.rs"]
mod latency;
pub use latency::FleetLatencyBand;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FleetTrial {
    Quiet,
    Equal,
    Abusive,
}
impl FleetTrial {
    pub const ALL: [Self; 3] = [Self::Quiet, Self::Equal, Self::Abusive];
    pub fn name(self) -> &'static str {
        match self {
            Self::Quiet => "quiet",
            Self::Equal => "equal",
            Self::Abusive => "abusive",
        }
    }
    pub fn active(self, estate: u64) -> bool {
        self != Self::Quiet || estate == 0
    }
}

/// No caller-selected paths, workload overrides or additional profile authority.
pub fn fleet_child(
    campaign: &StorageCampaign,
    point: usize,
    seed: usize,
    trial: FleetTrial,
    estate: u64,
) -> Result<StorageCampaign> {
    validate_fleet(campaign)?;
    require(
        point < campaign.points.len()
            && seed < campaign.seeds.len()
            && estate < campaign.allocation.estates.get(),
        "fleet child coordinate outside manifest",
    )?;
    let estates = campaign.allocation.estates.get();
    let mut child = campaign.clone();
    child.purpose = CampaignPurpose::Characterization;
    child.allocation.estates = 1u64.try_into()?;
    child.storage.budgets.estates = 1u64.try_into()?;
    child.allocation.records = (campaign.allocation.records.get() / estates).try_into()?;
    child.allocation.edges = (campaign.allocation.edges.get() / estates).try_into()?;
    child.allocation.vectors = (campaign.allocation.vectors.get() / estates).try_into()?;
    child.allocation.logical_source_bytes = campaign
        .allocation
        .logical_source_bytes
        .get()
        .div_ceil(estates)
        .max(1)
        .try_into()?;
    let mut operating = campaign.points[point].clone();
    operating.clients = (operating.clients.get() / estates).try_into()?;
    operating.requests_per_second = ((operating.requests_per_second.get() / estates)
        * if trial == FleetTrial::Abusive && estate == 0 {
            2
        } else {
            1
        })
    .try_into()?;
    child.points = vec![operating];
    child.seeds = vec![campaign.seeds[seed]
        .checked_add(estate)
        .filter(|n| *n <= MAX_EXACT_INTEGER)
        .ok_or("fleet seed overflow")?];
    // Reserve a root manifest/result and per-trial metadata outside all children.
    let children = campaign.points.len() as u64 * campaign.seeds.len() as u64 * 3 * estates;
    let root_reserve = MAX_DOCUMENT_BYTES
        .checked_mul(2 + campaign.points.len() as u64 * campaign.seeds.len() as u64 * 3)
        .and_then(|n| n.checked_add(MAX_LAUNCH_RECEIPT_BYTES))
        .ok_or("fleet evidence reserve overflow")?;
    let total = campaign
        .evidence
        .total_bytes
        .get()
        .checked_sub(root_reserve)
        .ok_or("fleet root has no evidence reservation")?
        / children;
    require(
        total >= child.evidence.segment_bytes.get() + 2 * MAX_DOCUMENT_BYTES,
        "fleet child has insufficient reserved evidence",
    )?;
    child.evidence.total_bytes = total.try_into()?;
    child.validate()?;
    Ok(child)
}

pub fn validate_fleet(campaign: &StorageCampaign) -> Result<()> {
    campaign.validate()?;
    let n = campaign.allocation.estates.get();
    require(
        (2..=10).contains(&n) && campaign.points.len() * campaign.seeds.len() <= 32,
        "fleet requires 2..10 estates and at most 32 point/seed coordinates",
    )?;
    require(
        campaign.allocation.records.get().is_multiple_of(n)
            && campaign.allocation.edges.get().is_multiple_of(n)
            && campaign.allocation.vectors.get().is_multiple_of(n),
        "fleet corpus must divide exactly into independent estates",
    )?;
    for point in &campaign.points {
        require(
            point.clients.get().is_multiple_of(n)
                && point.clients.get() / n == 4
                && point.requests_per_second.get().is_multiple_of(n)
                && point.requests_per_second.get() / n <= 500_000,
            "fleet requires four clients per estate and an exactly divisible admitted rate",
        )?;
    }
    campaign
        .read_distribution
        .validate(campaign.allocation.records.get() / n)?;
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FleetBoundary {
    pub estate: u64,
    /// Warmup/measured/burst/drain/soak/loaded/reopened; fixed coordinator stages.
    pub stage: Id,
    #[schemars(length(min = 1, max = 256))]
    pub instance: String,
    #[schemars(length(min = 1, max = 256))]
    pub foreign_instance: String,
    pub denial: rrd_contract::ErrorCode,
    pub sample: ResourceSample,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FleetTrialResult {
    pub format_version: u16,
    pub campaign_sha256: Digest,
    pub index: u64,
    pub point: usize,
    pub seed: usize,
    pub trial: FleetTrial,
    pub status: CampaignStatus,
    /// One digest per independently verified child result, in estate order.
    #[schemars(length(max = 10))]
    pub child_results: Vec<Digest>,
    /// Bounded pre-phase and post-reopen kernel/authorization observations.
    #[schemars(length(max = 70))]
    pub boundaries: Vec<FleetBoundary>,
    /// Conservative sum of per-estate peaks, not a simultaneous RSS sample.
    pub sum_member_peak_resident_bytes: u64,
    #[schemars(length(max = 10))]
    pub measured_latency: Vec<FleetLatencyBand>,
    pub qualification_passed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FleetResult {
    pub format_version: u16,
    pub campaign_sha256: Digest,
    pub status: CampaignStatus,
    #[schemars(length(max = 96))]
    pub trials: Vec<Digest>,
    pub isolated_latency_passed: bool,
    pub qualification_passed: bool,
}

pub fn fleet_member_directory(root: &Path, trial: u64, estate: u64) -> PathBuf {
    root.join(format!("trial-{trial:04}-estate-{estate:02}"))
}

fn check_boundaries(campaign: &StorageCampaign, trial: &FleetTrialResult) -> Result<()> {
    let n = campaign.allocation.estates.get() as usize;
    let stages = [
        "warmup", "measured", "burst", "drain", "soak", "loaded", "reopened",
    ];
    require(
        trial.boundaries.len() == n * stages.len(),
        "fleet boundary coverage incomplete",
    )?;
    let mut prior = vec![0; n];
    let mut identities = vec![String::new(); n];
    let mut shared_allocation = None;
    let mut loaded_pids = vec![0; n];
    for stage in stages {
        let rows = trial
            .boundaries
            .iter()
            .filter(|b| b.stage.as_str() == stage)
            .collect::<Vec<_>>();
        require(
            rows.len() == n,
            "fleet stage has missing or duplicate members",
        )?;
        let mut pids = BTreeSet::new();
        let mut instances = BTreeSet::new();
        let mut seen = BTreeSet::new();
        for row in &rows {
            let i = row.estate as usize;
            require(
                i < n
                    && seen.insert(i)
                    && pids.insert(row.sample.pid)
                    && row.sample.pid > 0
                    && instances.insert(&row.instance)
                    && !row.instance.is_empty()
                    && row.instance.len() <= 256,
                "fleet members do not identify distinct live engines",
            )?;
            require(
                row.sample.elapsed_ns > prior[i]
                    && row.sample.elapsed_ns <= MAX_EXACT_INTEGER
                    && row.foreign_instance.len() <= 256,
                "fleet boundary time did not advance",
            )?;
            prior[i] = row.sample.elapsed_ns;
            if stage == "loaded" {
                loaded_pids[i] = row.sample.pid;
            }
            if stage == "reopened" {
                require(
                    loaded_pids[i] != row.sample.pid,
                    "fleet final reopen did not replace the process",
                )?;
            }
            if let Some(storage) = &row.sample.storage {
                storage.validate().map_err(|e| e.to_string())?;
            }
            if let Some(rss) = row.sample.resident_bytes.observed() {
                require(
                    rss <= campaign.allocation.engine_bytes.get(),
                    "fleet boundary RSS ceiling exceeded",
                )?;
            }
            if let Some(host) = row.sample.host_resident_bytes.observed() {
                require(
                    host <= campaign.allocation.host_bytes.get(),
                    "fleet boundary host ceiling exceeded",
                )?;
            }
            if stage == "warmup" {
                identities[i] = row.instance.clone();
            } else {
                require(
                    identities[i] == row.instance,
                    "fleet estate identity changed",
                )?;
            }
            require(
                matches!(
                    row.denial,
                    rrd_contract::ErrorCode::Unauthenticated
                        | rrd_contract::ErrorCode::PermissionDenied
                        | rrd_contract::ErrorCode::NotFound
                ),
                "foreign session was not explicitly denied",
            )?;
            let next = rows
                .iter()
                .find(|r| r.estate == (row.estate + 1) % n as u64)
                .ok_or("foreign member missing")?;
            require(
                row.foreign_instance == next.instance && row.instance != row.foreign_instance,
                "foreign denial did not bind the next independent estate",
            )?;
            if campaign.purpose == CampaignPurpose::Qualification {
                let receipt = row
                    .sample
                    .allocation_receipt
                    .as_ref()
                    .ok_or("fleet lacks native aggregate allocation")?;
                receipt.validate(campaign)?;
                require(
                    row.sample.allocation_enforcement == receipt.enforcement()?,
                    "fleet native membership not verified",
                )?;
                let hash = receipt.sha256()?;
                if let Some(expected) = &shared_allocation {
                    require(
                        *expected == hash,
                        "fleet members use different native allocations",
                    )?;
                } else {
                    shared_allocation = Some(hash);
                }
                require(
                    matches!(row.sample.resident_bytes, NativeMetric::Observed { .. }),
                    "fleet native RSS unavailable",
                )?;
            }
        }
    }
    Ok(())
}

pub fn assess_fleet_trial(
    root: &Path,
    campaign: &StorageCampaign,
    trial: &mut FleetTrialResult,
) -> Result<()> {
    validate_fleet(campaign)?;
    require(
        trial.format_version == 1
            && trial.campaign_sha256 == campaign.sha256()?
            && trial.point < campaign.points.len()
            && trial.seed < campaign.seeds.len(),
        "fleet trial identity differs",
    )?;
    let expected_index = ((trial.point * campaign.seeds.len() + trial.seed) * 3
        + FleetTrial::ALL
            .iter()
            .position(|t| *t == trial.trial)
            .ok_or("unknown trial")?) as u64;
    require(trial.index == expected_index, "fleet trial order differs")?;
    require(
        trial.boundaries.len() <= 70
            && trial.child_results.len() <= campaign.allocation.estates.get() as usize
            && trial.measured_latency.len() <= campaign.allocation.estates.get() as usize,
        "fleet trial metadata exceeds bound",
    )?;
    if trial.status != CampaignStatus::Completed {
        trial.qualification_passed = false;
        return Ok(());
    }
    check_boundaries(campaign, trial)?;
    require(
        trial.child_results.len() == campaign.allocation.estates.get() as usize,
        "fleet child coverage incomplete",
    )?;
    let mut peak = 0u64;
    let mut qualifies = true;
    let mut measured_latency = Vec::new();
    let mut isolated_intervals = Vec::new();
    let mut overlap = std::collections::BTreeMap::<LoadPhase, (u64, u64)>::new();
    for estate in 0..campaign.allocation.estates.get() {
        let child = fleet_child(campaign, trial.point, trial.seed, trial.trial, estate)?;
        let directory = fleet_member_directory(root, trial.index, estate);
        let actual: StorageCampaign = decode(&read_bounded(
            &directory.join("campaign.json"),
            MAX_DOCUMENT_BYTES,
        )?)?;
        require(
            actual.sha256()? == child.sha256()?,
            "fleet child changed the frozen derived workload",
        )?;
        let result = verify_campaign(&directory)?;
        require(
            result.status == CampaignStatus::Completed
                && result.limitations.is_empty()
                && digest("rrflow-fleet-child-result-v1", &result)?
                    == trial.child_results[estate as usize],
            "fleet child failed or result changed",
        )?;
        let loaded = trial
            .boundaries
            .iter()
            .find(|b| b.estate == estate && b.stage.as_str() == "loaded")
            .ok_or("loaded boundary missing")?;
        let before = trial
            .boundaries
            .iter()
            .find(|b| b.estate == estate && b.stage.as_str() == "warmup")
            .ok_or("warmup boundary missing")?;
        let mut histogram = super::latency::Histogram::new();
        for index in 0..result.segments {
            let segment: SustainedSegment = decode(&read_bounded(
                &directory.join(format!("segment-{index:020}.json")),
                child.evidence.segment_bytes.get(),
            )?)?;
            require(
                segment.started_ns >= before.sample.elapsed_ns
                    && segment.finished_ns <= loaded.sample.elapsed_ns,
                "child load lies outside fleet boundary interval",
            )?;
            if segment.phase == LoadPhase::Measured {
                for attempt in &segment.attempts {
                    histogram.record(
                        attempt
                            .finished_ns
                            .checked_sub(attempt.scheduled_ns)
                            .ok_or("negative fleet latency")?,
                    )?;
                }
            }
            for sample in &segment.resources {
                require(
                    sample.pid == loaded.sample.pid,
                    "child resource sample belongs to another process",
                )?;
                require(
                    digest("fleet-allocation", &sample.allocation_receipt)?
                        == digest("fleet-allocation", &loaded.sample.allocation_receipt)?,
                    "child sample changed shared fleet allocation",
                )?;
            }
        }
        let band = histogram.percentile(99)?;
        measured_latency.push(FleetLatencyBand {
            estate,
            samples: band.samples,
            p99_lower_ns: band.lower_ns,
            p99_upper_ns: band.upper_ns,
        });
        if trial.trial == FleetTrial::Quiet {
            let row = result
                .operating_points
                .iter()
                .find(|r| r.phase == LoadPhase::Measured)
                .ok_or("quiet estate measured baseline missing")?;
            isolated_intervals.push((row.started_ns, row.finished_ns));
        }
        if trial.trial.active(estate) {
            for row in &result.operating_points {
                overlap
                    .entry(row.phase)
                    .and_modify(|(start, end)| {
                        *start = (*start).max(row.started_ns);
                        *end = (*end).min(row.finished_ns);
                    })
                    .or_insert((row.started_ns, row.finished_ns));
            }
        }
        peak = peak
            .checked_add(
                result
                    .operating_points
                    .iter()
                    .map(|r| r.maximum_resident_bytes)
                    .max()
                    .unwrap_or(0),
            )
            .ok_or("fleet peak sum overflow")?;
        if trial.trial.active(estate) {
            qualifies &= super::sustained::satisfies_operating_requirements(&child, &result)?;
        } else {
            require(
                result.operating_points.iter().all(|r| {
                    matches!(
                        r.phase,
                        LoadPhase::Warmup | LoadPhase::Measured | LoadPhase::Drain
                    )
                }),
                "quiet neighbour executed offered load",
            )?;
            qualifies &= result
                .operating_points
                .iter()
                .all(|r| r.missing_metrics.is_empty() && r.unresolved == 0 && r.failures == 0);
            let idle = result
                .operating_points
                .iter()
                .find(|r| r.phase == LoadPhase::Drain)
                .ok_or("quiet neighbour was not observed")?;
            let expected = [LoadPhase::Burst, LoadPhase::Drain, LoadPhase::Soak]
                .iter()
                .map(|p| child.schedule.duration_ms(*p))
                .sum::<u64>()
                * 1_000_000;
            let baseline = result
                .operating_points
                .iter()
                .find(|r| r.phase == LoadPhase::Measured)
                .ok_or("quiet baseline absent")?;
            let point = &child.points[0];
            qualifies &= baseline.elapsed_ns >= child.schedule.measured_ms.get() * 1_000_000
                && baseline.attempts as u128
                    >= (u128::from(child.schedule.measured_ms.get())
                        * u128::from(point.requests_per_second.get()))
                    .div_ceil(1000)
                && baseline.latency_over_budget as u128 * 100 <= baseline.attempts as u128
                && baseline.failures as u128 * 10_000
                    <= baseline.attempts as u128 * u128::from(point.maximum_error_basis_points);
            qualifies &= idle.elapsed_ns >= expected;
        }
    }
    for phase in [
        LoadPhase::Warmup,
        LoadPhase::Measured,
        LoadPhase::Burst,
        LoadPhase::Drain,
        LoadPhase::Soak,
    ] {
        let (start, end) = overlap.get(&phase).ok_or("fleet active phase missing")?;
        // Drain starts immediately after the common barrier and may differ by
        // one sampling interval; offered-load phases use one exact origin.
        let allowance = if phase == LoadPhase::Drain {
            campaign.schedule.sample_ms.get() * 1_000_000
        } else {
            0
        };
        qualifies &= end.saturating_sub(*start).saturating_add(allowance)
            >= campaign.schedule.duration_ms(phase) * 1_000_000;
    }
    if trial.trial == FleetTrial::Quiet {
        require(
            isolated_intervals.len() == campaign.allocation.estates.get() as usize,
            "quiet isolated baseline coverage missing",
        )?;
        for pair in isolated_intervals.windows(2) {
            require(
                pair[0].1 <= pair[1].0,
                "quiet baselines overlap or are not in estate order",
            )?;
        }
    }
    trial.measured_latency = measured_latency;
    trial.sum_member_peak_resident_bytes = peak;
    trial.qualification_passed = campaign.purpose == CampaignPurpose::Qualification
        && qualifies
        && peak <= campaign.allocation.engine_bytes.get();
    Ok(())
}

pub struct FleetWriter {
    directory: PathBuf,
    campaign: StorageCampaign,
    trials: Vec<Digest>,
}
impl FleetWriter {
    pub fn create(directory: &Path, campaign: StorageCampaign) -> Result<Self> {
        validate_fleet(&campaign)?;
        // Check every mechanical child's quota before any engine is installed.
        for point in 0..campaign.points.len() {
            for seed in 0..campaign.seeds.len() {
                for trial in FleetTrial::ALL {
                    for estate in 0..campaign.allocation.estates.get() {
                        fleet_child(&campaign, point, seed, trial, estate)?;
                    }
                }
            }
        }
        std::fs::create_dir(directory).map_err(|e| e.to_string())?;
        write_new(
            &directory.join("campaign.json"),
            &encode_bounded(&campaign, MAX_DOCUMENT_BYTES)?,
        )?;
        Ok(Self {
            directory: directory.to_owned(),
            campaign,
            trials: vec![],
        })
    }
    pub fn append(&mut self, mut trial: FleetTrialResult) -> Result<()> {
        require(
            trial.index == self.trials.len() as u64
                && self.trials.len() < self.campaign.points.len() * self.campaign.seeds.len() * 3,
            "fleet trial append order or bound differs",
        )?;
        assess_fleet_trial(&self.directory, &self.campaign, &mut trial)?;
        write_new(
            &self
                .directory
                .join(format!("trial-{:04}.json", trial.index)),
            &encode_bounded(&trial, MAX_DOCUMENT_BYTES)?,
        )?;
        self.trials.push(digest("rrflow-fleet-trial-v1", &trial)?);
        Ok(())
    }
    pub fn finish(self, status: CampaignStatus) -> Result<FleetResult> {
        let mut result = FleetResult {
            format_version: 1,
            campaign_sha256: self.campaign.sha256()?,
            status,
            trials: self.trials,
            isolated_latency_passed: false,
            qualification_passed: false,
        };
        (result.qualification_passed, result.isolated_latency_passed) =
            verify_fleet_parts(&self.directory, &self.campaign, &result)?;
        write_new(
            &self.directory.join("fleet-result.json"),
            &encode_bounded(&result, MAX_DOCUMENT_BYTES)?,
        )?;
        Ok(result)
    }
}
fn verify_fleet_parts(
    root: &Path,
    campaign: &StorageCampaign,
    result: &FleetResult,
) -> Result<(bool, bool)> {
    validate_fleet(campaign)?;
    let count = campaign.points.len() * campaign.seeds.len() * 3;
    require(
        result.format_version == 1
            && result.campaign_sha256 == campaign.sha256()?
            && result.trials.len() <= count,
        "fleet result identity/count differs",
    )?;
    require(
        result.status != CampaignStatus::Completed || result.trials.len() == count,
        "complete fleet is missing trials",
    )?;
    let mut passed = result.status == CampaignStatus::Completed
        && campaign.purpose == CampaignPurpose::Qualification;
    let mut quiet = None;
    let mut isolation_passed = result.status == CampaignStatus::Completed;
    for (index, expected) in result.trials.iter().enumerate() {
        let raw = read_bounded(
            &root.join(format!("trial-{index:04}.json")),
            MAX_DOCUMENT_BYTES,
        )?;
        let mut trial: FleetTrialResult = decode(&raw)?;
        require(
            trial.index == index as u64 && digest("rrflow-fleet-trial-v1", &trial)? == *expected,
            "fleet trial identity/hash differs",
        )?;
        let before = digest("rrflow-fleet-trial-v1", &trial)?;
        assess_fleet_trial(root, campaign, &mut trial)?;
        require(
            digest("rrflow-fleet-trial-v1", &trial)? == before,
            "fleet claimed aggregate or qualification differs from evidence",
        )?;
        require(
            result.status != CampaignStatus::Completed || trial.status == CampaignStatus::Completed,
            "completed fleet contains a failed trial",
        )?;
        if trial.trial == FleetTrial::Quiet {
            quiet = Some(trial.measured_latency.clone());
        }
        if trial.trial == FleetTrial::Abusive && trial.status == CampaignStatus::Completed {
            isolation_passed &= latency::isolated_ratio(
                quiet
                    .as_ref()
                    .ok_or("quiet baseline missing before abusive trial")?,
                &trial.measured_latency,
                campaign.allocation.estates.get(),
            )?;
        }
        passed &= trial.qualification_passed;
    }
    Ok((passed && isolation_passed, isolation_passed))
}
pub fn verify_fleet(root: &Path) -> Result<FleetResult> {
    let campaign: StorageCampaign = decode(&read_bounded(
        &root.join("campaign.json"),
        MAX_DOCUMENT_BYTES,
    )?)?;
    let result: FleetResult = decode(&read_bounded(
        &root.join("fleet-result.json"),
        MAX_DOCUMENT_BYTES,
    )?)?;
    require(
        (result.qualification_passed, result.isolated_latency_passed)
            == verify_fleet_parts(root, &campaign, &result)?,
        "fleet qualification claim differs",
    )?;
    Ok(result)
}

#[cfg(test)]
#[path = "fleet_tests.rs"]
mod tests;
