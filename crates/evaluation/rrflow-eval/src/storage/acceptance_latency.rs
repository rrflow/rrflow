//! Owner-threshold checks over already bound raw measured segments. Physical
//! range timing is a C-07 proxy, not graph/context or model-quality qualification.
use super::super::latency::Histogram;
use super::super::sustained::read_bounded;
use super::super::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::Path};
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NominalObservation {
    pub directory: Id,
    pub profile: Id,
    pub point: Id,
    pub clients: u64,
    pub requests_per_second: u64,
    pub seed: u64,
    pub kind: StorageLoadKind,
    pub p95: LatencyBounds,
    pub p99: LatencyBounds,
    pub total_attempts: u64,
    pub terminal_errors: u64,
    pub maximum_p95_ns: u64,
    pub maximum_p99_ns: u64,
    pub maximum_error_basis_points: u16,
    pub passed: bool,
}
struct Cohort {
    histograms: [Histogram; 3],
    attempts: u64,
    errors: u64,
}
fn nominal_passes(
    p95: &LatencyBounds,
    p99: &LatencyBounds,
    attempts: u64,
    errors: u64,
    limit: &LatencyThreshold,
    error_bps: u16,
) -> bool {
    attempts > 0
        && errors <= attempts
        && p95.samples == p99.samples
        && p95.samples > 0
        && p95.upper_ns <= limit.p95_ns
        && p99.upper_ns <= limit.p99_ns
        && u128::from(errors) * 10_000 <= u128::from(attempts) * u128::from(error_bps)
}
pub(super) fn measure(
    directory: &Path,
    label: &Id,
    campaign: &StorageCampaign,
    result: &CampaignResult,
) -> Result<Vec<NominalObservation>> {
    let projection = capacity_profiles()?;
    let thresholds = &projection.thresholds;
    let Some(rate) = thresholds
        .nominal_rates
        .iter()
        .find(|r| r.profile == campaign.storage.profile.as_str())
    else {
        return Ok(vec![]);
    };
    if result.status != CampaignStatus::Completed {
        return Ok(vec![]);
    }
    let points = campaign
        .points
        .iter()
        .filter(|p| p.requests_per_second.get() == rate.requests_per_second)
        .collect::<Vec<_>>();
    require(
        points.len() * campaign.seeds.len() <= 128,
        "nominal timing reducer exceeds 128 fixed cohorts",
    )?;
    if points.is_empty() {
        return Ok(vec![]);
    }
    let mut cohorts = BTreeMap::new();
    for point in &points {
        for seed in &campaign.seeds {
            cohorts.insert(
                (point.id.clone(), *seed),
                Cohort {
                    histograms: std::array::from_fn(|_| Histogram::new()),
                    attempts: 0,
                    errors: 0,
                },
            );
        }
    }
    let mut previous = None;
    for index in 0..result.segments {
        let segment: SustainedSegment = decode(&read_bounded(
            &directory.join(format!("segment-{index:020}.json")),
            campaign.evidence.segment_bytes.get(),
        )?)?;
        require(
            segment.index == index && segment.previous_sha256 == previous,
            "nominal read changed segment chain",
        )?;
        previous = Some(digest("rrflow-storage-segment-v1", &segment)?);
        if segment.phase != LoadPhase::Measured {
            continue;
        }
        let Some(cohort) = cohorts.get_mut(&(segment.point, segment.seed)) else {
            continue;
        };
        for attempt in segment.attempts {
            cohort.attempts = cohort
                .attempts
                .checked_add(1)
                .filter(|n| *n <= MAX_EXACT_INTEGER)
                .ok_or("nominal attempt count overflow")?;
            if attempt.outcome != LoadOutcome::Acknowledged {
                cohort.errors += 1;
            }
            let index = match attempt.kind {
                StorageLoadKind::PointRead => 0,
                StorageLoadKind::MixedWrite => 1,
                StorageLoadKind::RangeRead => 2,
                StorageLoadKind::Maintenance => continue,
            };
            cohort.histograms[index].record(
                attempt
                    .finished_ns
                    .checked_sub(attempt.scheduled_ns)
                    .ok_or("nominal timing reversed")?,
            )?;
        }
    }
    require(
        previous == result.last_segment_sha256,
        "nominal timing source differs from verified result",
    )?;
    let mut observations = Vec::new();
    for ((point, seed), cohort) in cohorts {
        let operating = points
            .iter()
            .find(|p| p.id == point)
            .ok_or("nominal point missing")?;
        for (index, (kind, limit)) in [
            (StorageLoadKind::PointRead, &thresholds.point_read),
            (StorageLoadKind::MixedWrite, &thresholds.mixed_write),
            (StorageLoadKind::RangeRead, &thresholds.range_read),
        ]
        .into_iter()
        .enumerate()
        {
            let p95 = cohort.histograms[index].percentile(95)?;
            let p99 = cohort.histograms[index].percentile(99)?;
            let passed = nominal_passes(
                &p95,
                &p99,
                cohort.attempts,
                cohort.errors,
                limit,
                thresholds.maximum_error_basis_points,
            );
            observations.push(NominalObservation {
                directory: label.clone(),
                profile: campaign.storage.profile.clone(),
                point: point.clone(),
                clients: operating.clients.get(),
                requests_per_second: operating.requests_per_second.get(),
                seed,
                kind,
                p95,
                p99,
                total_attempts: cohort.attempts,
                terminal_errors: cohort.errors,
                maximum_p95_ns: limit.p95_ns,
                maximum_p99_ns: limit.p99_ns,
                maximum_error_basis_points: thresholds.maximum_error_basis_points,
                passed,
            });
        }
    }
    Ok(observations)
}
pub(super) fn complete(observations: &[NominalObservation]) -> Result<bool> {
    let projection = capacity_profiles()?;
    if observations.iter().any(|row| !row.passed) {
        return Ok(false);
    }
    for nominal in &projection.thresholds.nominal_rates {
        let profile = projection
            .profiles
            .iter()
            .find(|p| p.id == nominal.profile)
            .ok_or("nominal profile missing from owner")?;
        for clients in &profile.clients {
            for kind in [
                StorageLoadKind::PointRead,
                StorageLoadKind::MixedWrite,
                StorageLoadKind::RangeRead,
            ] {
                let seeds = observations
                    .iter()
                    .filter(|row| {
                        row.profile.as_str() == nominal.profile
                            && row.clients == *clients
                            && row.requests_per_second == nominal.requests_per_second
                            && row.kind == kind
                    })
                    .map(|r| r.seed)
                    .collect::<std::collections::BTreeSet<_>>();
                if seeds.len() < 3 {
                    return Ok(false);
                }
            }
        }
    }
    Ok(true)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nominal_limits_reject_optimistic_bins_and_excess_errors() {
        let limit = LatencyThreshold {
            p95_ns: 50_000_000,
            p99_ns: 200_000_000,
        };
        let mut p95 = LatencyBounds {
            samples: 10000,
            lower_ns: 49_000_000,
            upper_ns: 50_000_000,
        };
        let p99 = LatencyBounds {
            samples: 10000,
            lower_ns: 199_000_000,
            upper_ns: 200_000_000,
        };
        assert!(nominal_passes(&p95, &p99, 10000, 10, &limit, 10));
        assert!(!nominal_passes(&p95, &p99, 10000, 11, &limit, 10));
        p95.upper_ns += 1;
        assert!(!nominal_passes(&p95, &p99, 10000, 0, &limit, 10));
        assert!(!complete(&[]).unwrap());
    }
}
