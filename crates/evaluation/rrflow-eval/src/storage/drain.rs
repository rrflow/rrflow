//! Bounded post-burst ownership recovery, derived from actual storage samples.
use super::sustained::{LoadPhase, OperatingPointResult, ResourceSample, StorageCampaign};
use super::{require, Result};
use super::{Id, Limit};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DrainPolicy {
    pub maximum_mutable_growth_bytes: u64,
    pub maximum_projected_growth_bytes: u64,
    pub maximum_debt_segments: u64,
    pub stable_samples: Limit,
}

impl DrainPolicy {
    pub(super) fn validate(&self, campaign: &StorageCampaign) -> Result<()> {
        require(
            (2..=64).contains(&self.stable_samples.get())
                && self.maximum_mutable_growth_bytes
                    <= campaign.storage.budgets.mutable_bytes.get()
                && self.maximum_projected_growth_bytes
                    <= campaign.storage.budgets.retained_bytes.get()
                && self.maximum_debt_segments <= super::MAX_EXACT_INTEGER
                && (self.stable_samples.get() - 1)
                    .checked_mul(campaign.schedule.sample_ms.get())
                    .is_some_and(|time| time <= campaign.schedule.drain_ms.get()),
            "drain policy exceeds ownership bounds or its observation window",
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DrainOwnership {
    pub native_transactions: u64,
    pub native_transaction_bytes: u64,
    pub active_read_views: u64,
    pub pinned_read_bytes: u64,
    pub mutable_bytes: u64,
    pub projected_bytes: u64,
    pub debt_segments: u64,
    pub outstanding_requests: u64,
    pub outstanding_bytes: u64,
    pub queued_requests: u64,
    pub active: bool,
    pub fenced: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DrainSample {
    pub elapsed_ns: u64,
    /// Missing native admission/counters are retained, never converted to zero.
    pub ownership: Option<DrainOwnership>,
}

impl From<&ResourceSample> for DrainSample {
    fn from(sample: &ResourceSample) -> Self {
        let ownership = sample.storage.as_ref().and_then(|storage| {
            let admission = storage.admission.as_ref()?;
            let native = storage.rrflow_kv.as_ref()?;
            Some(DrainOwnership {
                native_transactions: admission.native_transactions,
                native_transaction_bytes: admission.native_transaction_bytes,
                active_read_views: admission.active_read_views,
                pinned_read_bytes: admission.pinned_read_bytes,
                mutable_bytes: admission.retained_mutable_bytes,
                projected_bytes: admission.retained_projected_bytes,
                debt_segments: native.segment_set.compaction_debt_segments,
                outstanding_requests: admission.outstanding_requests,
                outstanding_bytes: admission.outstanding_logical_bytes,
                queued_requests: admission.queued_requests,
                active: admission.active,
                fenced: admission.fenced,
            })
        });
        Self {
            elapsed_ns: sample.elapsed_ns,
            ownership,
        }
    }
}

pub(super) fn check(
    campaign: &StorageCampaign,
    rows: &[&OperatingPointResult],
    point: &Id,
    seed: u64,
) -> Result<()> {
    let warmup = rows
        .iter()
        .find(|row| row.point == *point && row.seed == seed && row.phase == LoadPhase::Warmup)
        .ok_or("drain lacks pre-load warmup baseline")?;
    let baseline = warmup
        .ownership_baseline
        .as_ref()
        .and_then(|sample| sample.ownership.as_ref())
        .ok_or("drain baseline ownership unavailable")?;
    require(
        !baseline.active
            && !baseline.fenced
            && baseline.outstanding_requests == 0
            && baseline.outstanding_bytes == 0
            && baseline.queued_requests == 0,
        "drain baseline is not quiescent",
    )?;
    let drain = rows
        .iter()
        .find(|row| row.point == *point && row.seed == seed && row.phase == LoadPhase::Drain)
        .ok_or("drain observations missing")?;
    require(
        warmup.finished_ns <= drain.started_ns,
        "drain precedes its measured warmup baseline",
    )?;
    require(
        drain.drain_tail.len() as u64 == campaign.drain.stable_samples.get(),
        "drain has too few settled observations",
    )?;
    let mutable = baseline
        .mutable_bytes
        .checked_add(campaign.drain.maximum_mutable_growth_bytes)
        .ok_or("mutable drain threshold overflow")?;
    let projected = baseline
        .projected_bytes
        .checked_add(campaign.drain.maximum_projected_growth_bytes)
        .ok_or("projected drain threshold overflow")?;
    let mut prior = None;
    for sample in &drain.drain_tail {
        if let Some(before) = prior {
            require(
                sample.elapsed_ns > before,
                "drain samples repeat or reverse time",
            )?;
        }
        prior = Some(sample.elapsed_ns);
        let owner = sample
            .ownership
            .as_ref()
            .ok_or("drain ownership observation unavailable")?;
        require(
            owner.native_transactions <= baseline.native_transactions
                && owner.native_transaction_bytes <= baseline.native_transaction_bytes
                && owner.active_read_views <= baseline.active_read_views
                && owner.pinned_read_bytes <= baseline.pinned_read_bytes
                && owner.mutable_bytes <= mutable
                && owner.projected_bytes <= projected
                && owner.debt_segments <= campaign.drain.maximum_debt_segments
                && owner.outstanding_requests == 0
                && owner.outstanding_bytes == 0
                && owner.queued_requests == 0
                && !owner.active
                && !owner.fenced,
            "drain did not return owners and maintenance debt to frozen thresholds",
        )?;
    }
    Ok(())
}

pub(super) fn check_completed(
    campaign: &StorageCampaign,
    rows: &[OperatingPointResult],
) -> Result<()> {
    let references: Vec<_> = rows.iter().collect();
    for row in rows.iter().filter(|row| row.phase == LoadPhase::Drain) {
        check(campaign, &references, &row.point, row.seed)?;
    }
    Ok(())
}
