//! The campaign writer and segment validation entry point.

use super::*;

impl CampaignWriter {
    pub fn create(directory: &Path, campaign: StorageCampaign) -> Result<Self> {
        let identity = campaign.sha256()?;
        let bytes = encode_bounded(&campaign, MAX_DOCUMENT_BYTES)?;
        require(
            (bytes.len() as u64)
                .checked_add(MAX_DOCUMENT_BYTES + MAX_LAUNCH_RECEIPT_BYTES)
                .and_then(|n| n.checked_add(4096))
                .is_some_and(|n| n <= campaign.evidence.total_bytes.get()),
            "campaign identity exceeds artifact budget",
        )?;
        std::fs::create_dir(directory)
            .map_err(|e| format!("campaign requires absent evidence directory: {e}"))?;
        write_new(&directory.join("campaign.json"), &bytes)?;
        let progress = Mutex::new(ProgressJournal::create(directory, &identity)?);
        Ok(Self {
            directory: directory.to_owned(),
            campaign,
            identity,
            count: 0,
            bytes: bytes.len() as u64,
            previous: None,
            last_finished_ns: 0,
            totals: BTreeMap::new(),
            failed: None,
            progress,
            backups: BackupTracker::default(),
        })
    }
    pub fn next_index(&self) -> u64 {
        self.count
    }
    pub fn previous_sha256(&self) -> Option<Digest> {
        self.previous.clone()
    }
    pub fn identity(&self) -> &Digest {
        &self.identity
    }
    pub fn record_event(&self, event: &ProgressEvent) -> Result<()> {
        let maximum = self
            .campaign
            .evidence
            .total_bytes
            .get()
            .checked_sub(self.bytes)
            .and_then(|n| n.checked_sub(MAX_DOCUMENT_BYTES + MAX_LAUNCH_RECEIPT_BYTES))
            .and_then(|n| n.checked_sub(self.campaign.evidence.segment_bytes.get()))
            .ok_or("progress has no reserved evidence capacity")?;
        self.progress
            .lock()
            .map_err(|_| "progress writer poisoned")?
            .append(event, maximum)
    }
    pub fn checkpoint_progress(&self) -> Result<()> {
        self.progress
            .lock()
            .map_err(|_| "progress writer poisoned")?
            .checkpoint()
    }
    fn progress_bytes(&self) -> Result<u64> {
        Ok(self
            .progress
            .lock()
            .map_err(|_| "progress writer poisoned")?
            .bytes())
    }
    /// Reserve enough evidence capacity before invoking the next operating
    /// point. Actual append still checks its encoded size and native writes.
    pub fn admit_next(&self) -> Result<()> {
        require(
            self.failed.is_none() && self.count < self.campaign.evidence.segments.get(),
            "campaign cannot admit another segment",
        )?;
        require(
            self.bytes
                .checked_add(self.campaign.evidence.segment_bytes.get())
                .and_then(|n| n.checked_add(self.progress_bytes().ok()?))
                .and_then(|n| n.checked_add(MAX_DOCUMENT_BYTES + MAX_LAUNCH_RECEIPT_BYTES))
                .is_some_and(|n| n <= self.campaign.evidence.total_bytes.get()),
            "next operating point has no reserved evidence capacity",
        )
    }
    pub fn append(&mut self, segment: &SustainedSegment) -> Result<Digest> {
        require(
            self.failed.is_none(),
            "campaign already retained a failed segment",
        )?;
        require(
            self.count < self.campaign.evidence.segments.get(),
            "campaign segment quota exhausted",
        )?;
        let bytes = encode_bounded(segment, self.campaign.evidence.segment_bytes.get())?;
        let next = self
            .bytes
            .checked_add(bytes.len() as u64)
            .ok_or("campaign artifact size overflow")?;
        // Reserve one bounded final result so a quota failure cannot consume its
        // reporting space. Failed OS writes remain failed, never counted complete.
        require(
            next.checked_add(self.progress_bytes()?)
                .and_then(|n| n.checked_add(MAX_DOCUMENT_BYTES + MAX_LAUNCH_RECEIPT_BYTES))
                .is_some_and(|n| n <= self.campaign.evidence.total_bytes.get()),
            "campaign artifact quota exhausted",
        )?;
        let hash = digest("rrflow-storage-segment-v1", segment)?;
        let validation = validate_next(
            &self.campaign,
            segment,
            self.count,
            &self.previous,
            self.last_finished_ns,
            &self.totals,
            &self.backups,
        );
        let (summary, backups) = match validation {
            Ok(summary) => summary,
            Err(reason) => {
                write_new(&self.directory.join("failed-segment.json"), &bytes)?;
                self.bytes = next;
                self.failed = Some(FailedSegment {
                    sha256: hash,
                    reason: reason.clone(),
                });
                return Err(reason);
            }
        };
        write_new(
            &self
                .directory
                .join(format!("segment-{:020}.json", self.count)),
            &bytes,
        )?;
        self.bytes = next;
        self.count += 1;
        self.previous = Some(hash.clone());
        self.last_finished_ns = segment.finished_ns;
        self.backups = backups;
        merge_summary(
            &mut self.totals,
            summary,
            self.campaign.drain.stable_samples.get() as usize,
        )?;
        Ok(hash)
    }
    /// Do not spend a qualification soak on an already failed operating point.
    /// Characterization intentionally retains unsuccessful operating points.
    pub fn admit_soak(&self, point: &Id, seed: u64) -> Result<()> {
        if self.campaign.purpose != CampaignPurpose::Qualification {
            return Ok(());
        }
        require(self.failed.is_none(), "failed segment cannot admit soak")?;
        let point = self
            .campaign
            .points
            .iter()
            .find(|p| &p.id == point)
            .ok_or("soak point is not in frozen campaign")?;
        require(
            self.campaign.seeds.contains(&seed),
            "soak seed is not in frozen campaign",
        )?;
        require(
            satisfies_point_requirements(
                &self.campaign,
                point,
                seed,
                self.totals.values(),
                &phases()[..4],
            )?,
            "soak denied: prerequisite phases do not satisfy the frozen operating requirements",
        )?;
        self.verify_drain(&point.id, seed)
    }

    pub fn verify_drain(&self, point: &Id, seed: u64) -> Result<()> {
        super::super::drain::check(
            &self.campaign,
            &self.totals.values().collect::<Vec<_>>(),
            point,
            seed,
        )
    }
    pub fn finish(self, status: CampaignStatus, limitations: Vec<Id>) -> Result<CampaignResult> {
        require(
            limitations.len() <= MAX_CASES,
            "too many campaign limitations",
        )?;
        require(
            self.failed.is_none()
                || matches!(status, CampaignStatus::Failed | CampaignStatus::Interrupted),
            "failed operating point cannot be reported complete",
        )?;
        let progress = self
            .progress
            .into_inner()
            .map_err(|_| "progress writer poisoned")?
            .finish()?;
        let progress_audit = super::super::progress::verify_progress(
            &self.directory,
            &progress,
            &self.campaign,
            status,
        )?;
        for row in self.totals.values() {
            progress_audit.check_readback_order(&row.point, row.seed, row.finished_ns)?;
        }
        let artifact_bytes = self
            .bytes
            .checked_add(progress.bytes)
            .ok_or("progress artifact overflow")?;
        let mut result = CampaignResult {
            format_version: 1,
            campaign_sha256: self.identity,
            status,
            segments: self.count,
            last_segment_sha256: self.previous,
            artifact_bytes,
            progress,
            failed_segment: self.failed,
            operating_points: self.totals.into_values().collect(),
            limitations,
            qualification_passed: false,
        };
        if status == CampaignStatus::Completed {
            self.backups.require_settled()?;
            progress_audit.check_backup_starts(self.backups.start_markers())?;
            super::super::drain::check_completed(&self.campaign, &result.operating_points)?;
        }
        result.qualification_passed = qualifies(&self.campaign, &result)?;
        let bytes = encode_bounded(&result, MAX_DOCUMENT_BYTES)?;
        require(
            artifact_bytes
                .checked_add(bytes.len() as u64 + MAX_LAUNCH_RECEIPT_BYTES)
                .is_some_and(|n| n <= self.campaign.evidence.total_bytes.get()),
            "final result exceeds artifact quota",
        )?;
        write_new(&self.directory.join("result.json"), &bytes)?;
        Ok(result)
    }
}

pub fn validate_segment(
    campaign: &StorageCampaign,
    segment: &SustainedSegment,
) -> Result<OperatingPointResult> {
    validate_segment_with_backups(campaign, segment, &mut BackupTracker::default())
}
