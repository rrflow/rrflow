//! Fixed-size-per-point validation of backup work spanning evidence segments.
//! No old segment, ledger, archive or request payload is retained here.
use super::*;
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
struct Activity {
    started_ns: u64,
    earliest_write_started_ns: Option<u64>,
    completed: bool,
}

#[derive(Debug, Clone, Default)]
pub(super) struct BackupTracker {
    points: BTreeMap<(Id, u64), Activity>,
    start_markers: Option<Digest>,
}

impl BackupTracker {
    // Call only after this segment's independent ledger and attempt bindings
    // validate. A scalar minimum is enough: every candidate ACK ended after
    // backup start, so the minimum start proves overlap with its eventual end.
    pub(super) fn observe(
        &mut self,
        campaign: &StorageCampaign,
        segment: &SustainedSegment,
    ) -> Result<()> {
        let key = (segment.point.clone(), segment.seed);
        require(
            self.points.iter().all(|(owner, activity)| {
                activity.completed || (owner == &key && segment.phase == LoadPhase::Measured)
            }),
            "unfinished backup crossed its point, seed or measured phase",
        )?;
        let lane = segment
            .lanes
            .iter()
            .find(|lane| lane.lane == CapacityLane::BackupUnderWrite);
        let declared = segment.backup_started_ns;
        // Existing self-contained receipts still prove their start and overlap
        // in one segment. A cross-segment receipt always needs an earlier marker.
        let local_start = segment
            .backup
            .as_ref()
            .filter(|backup| backup.started_ns >= segment.started_ns)
            .map(|backup| backup.started_ns);
        if let Some(started_ns) = declared.or_else(|| {
            if self.points.contains_key(&key) {
                None
            } else {
                local_start
            }
        }) {
            require(
                segment.phase == LoadPhase::Measured
                    && !self.points.contains_key(&key)
                    && started_ns >= segment.started_ns
                    && started_ns <= segment.finished_ns,
                "backup start is duplicate or outside its measured segment",
            )?;
            self.points.insert(
                key.clone(),
                Activity {
                    started_ns,
                    earliest_write_started_ns: None,
                    completed: false,
                },
            );
            if declared.is_some() {
                self.start_markers = Some(super::progress::backup_start_digest(
                    &self.start_markers,
                    segment.index,
                    &segment.point,
                    segment.seed,
                    started_ns,
                )?);
            }
        }
        let Some(activity) = self.points.get_mut(&key) else {
            return require(
                segment.backup.is_none() && lane.is_none(),
                "backup completion has no authenticated start segment",
            );
        };
        if activity.completed {
            return require(
                segment.backup.is_none() && lane.is_none(),
                "backup completion or lane is duplicated",
            );
        }
        for attempt in &segment.attempts {
            if attempt.kind == StorageLoadKind::MixedWrite
                && attempt.outcome == LoadOutcome::Acknowledged
                && attempt.finished_ns > activity.started_ns
            {
                activity.earliest_write_started_ns = Some(
                    activity
                        .earliest_write_started_ns
                        .map_or(attempt.started_ns, |prior| prior.min(attempt.started_ns)),
                );
            }
        }
        let deadline = u128::from(activity.started_ns)
            + u128::from(campaign.schedule.operation_deadline_ms.get()) * 1_000_000;
        match &segment.backup {
            Some(backup) => {
                require(
                    backup.started_ns == activity.started_ns
                        && backup.started_ns <= backup.finished_ns
                        && backup.finished_ns <= backup.verified_ns
                        && backup.verified_ns <= segment.finished_ns
                        && u128::from(backup.verified_ns) <= deadline
                        && backup.receipt.backup.application_complete
                        && activity
                            .earliest_write_started_ns
                            .is_some_and(|started| started < backup.finished_ns)
                        && lane.is_some_and(|lane| {
                            lane.repetitions.get() == 1
                                && digest("c07g.backup-evidence", backup).as_ref()
                                    == Ok(&lane.receipt_sha256)
                        }),
                    "backup lacks matching start, deadline, overlapping ACK or verified receipt",
                )?;
                activity.completed = true;
                Ok(())
            }
            None => require(
                lane.is_none() && u128::from(segment.finished_ns) <= deadline,
                "pending backup exceeded its deadline or claimed a completed lane",
            ),
        }
    }

    pub(super) fn start_markers(&self) -> &Option<Digest> {
        &self.start_markers
    }

    pub(super) fn require_settled(&self) -> Result<()> {
        require(
            self.points.values().all(|activity| activity.completed),
            "completed campaign omitted an unfinished backup receipt",
        )
    }
}
