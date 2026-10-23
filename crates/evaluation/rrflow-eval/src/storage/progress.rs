//! Bounded append-only qualification progress. This is evidence, never engine
//! state: missing outcomes remain uncertain and cannot authorize replay.
use super::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use std::{
    collections::BTreeMap,
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, BufWriter, Read, Write},
    path::Path,
};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ProgressEvent {
    Campaign {
        campaign_sha256: Digest,
    },
    CorpusIntent {
        point: Id,
        seed: u64,
        ordinal: u64,
        request_sha256: Digest,
        effect_sha256: Digest,
    },
    CorpusAcknowledged {
        point: Id,
        seed: u64,
        ordinal: u64,
        receipt_sha256: Digest,
    },
    CorpusVerified {
        point: Id,
        seed: u64,
        ordinal: u64,
        effect_sha256: Digest,
    },
    LifecycleIntent {
        point: Id,
        seed: u64,
        cycle: u64,
        request_sha256: Digest,
        effect_sha256: Digest,
    },
    LifecycleAcknowledged {
        point: Id,
        seed: u64,
        cycle: u64,
        receipt_sha256: Digest,
    },
    LoadIntent {
        segment: u64,
        point: Id,
        seed: u64,
        phase: LoadPhase,
        expected: ExpectedOperation,
        write: Option<WriteTransition>,
    },
    LoadOutcome {
        segment: u64,
        attempt: LoadAttempt,
    },
    Readback {
        segment: u64,
        observation: Readback,
    },
    Resource {
        segment: u64,
        sample: Box<ResourceSample>,
    },
    Reopen {
        point: Id,
        seed: u64,
        observation: ReopenObservation,
    },
    ReaderInterference {
        segment: u64,
        observation: Box<ReaderInterferenceEvidence>,
    },
    Backup {
        segment: u64,
        observation: BackupEvidence,
    },
    Failure {
        reason: Id,
    },
    ColdFile {
        point: Id,
        seed: u64,
        file_identity_sha256: Digest,
        bytes: u64,
        page_bytes: u64,
        pages: u64,
        resident_pages: u64,
    },
    ColdStart {
        point: Id,
        seed: u64,
        observation: ColdStartEvidence,
    },
    FinalReadback {
        point: Id,
        seed: u64,
        killed_pid: u32,
        reopened_pid: u32,
        seed_records: u64,
        working_slots: u64,
        states_sha256: Digest,
        started_ns: u64,
        finished_ns: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProgressReceipt {
    pub bytes: u64,
    pub records: u64,
    pub sha256: Digest,
}

pub(super) struct ProgressJournal {
    output: BufWriter<File>,
    hash: Sha256,
    bytes: u64,
    records: u64,
}
impl ProgressJournal {
    pub fn create(directory: &Path, identity: &Digest) -> Result<Self> {
        let output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(directory.join("progress.jsonl"))
            .map_err(|e| e.to_string())?;
        let mut journal = Self {
            output: BufWriter::with_capacity(64 * 1024, output),
            hash: Sha256::new(),
            bytes: 0,
            records: 0,
        };
        journal.append(
            &ProgressEvent::Campaign {
                campaign_sha256: identity.clone(),
            },
            4096,
        )?;
        journal
            .output
            .flush()
            .and_then(|()| journal.output.get_ref().sync_all())
            .map_err(|e| e.to_string())?;
        Ok(journal)
    }
    pub fn bytes(&self) -> u64 {
        self.bytes
    }
    pub fn checkpoint(&mut self) -> Result<()> {
        self.output
            .flush()
            .and_then(|()| self.output.get_ref().sync_all())
            .map_err(|e| e.to_string())
    }
    pub fn append(&mut self, event: &ProgressEvent, maximum_bytes: u64) -> Result<()> {
        let mut bytes = super::sustained::encode_bounded(event, MAX_DOCUMENT_BYTES - 1)?;
        bytes.push(b'\n');
        let next = self
            .bytes
            .checked_add(bytes.len() as u64)
            .ok_or("progress byte overflow")?;
        require(
            next <= maximum_bytes,
            "progress evidence quota exhausted before append",
        )?;
        let records = self
            .records
            .checked_add(1)
            .filter(|n| *n <= MAX_EXACT_INTEGER)
            .ok_or("progress count overflow")?;
        self.output.write_all(&bytes).map_err(|e| e.to_string())?;
        self.hash.update(&bytes);
        self.bytes = next;
        self.records = records;
        Ok(())
    }
    pub fn finish(mut self) -> Result<ProgressReceipt> {
        self.output
            .flush()
            .and_then(|()| self.output.get_ref().sync_all())
            .map_err(|e| e.to_string())?;
        Ok(ProgressReceipt {
            bytes: self.bytes,
            records: self.records,
            sha256: self
                .hash
                .finalize()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
                .try_into()?,
        })
    }
}

fn scan(
    directory: &Path,
    receipt: &ProgressReceipt,
    identity: &Digest,
    total_bound: u64,
    mut observe: impl FnMut(&ProgressEvent) -> Result<()>,
) -> Result<()> {
    require(
        receipt.bytes <= total_bound && receipt.records > 0 && receipt.records <= MAX_EXACT_INTEGER,
        "progress receipt exceeds admission",
    )?;
    let path = directory.join("progress.jsonl");
    let metadata = std::fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
    require(
        metadata.is_file() && !metadata.file_type().is_symlink() && metadata.len() == receipt.bytes,
        "progress file missing, aliased or changed in size",
    )?;
    let file = File::open(path).map_err(|e| e.to_string())?;
    let mut input = BufReader::with_capacity(64 * 1024, file);
    let mut hash = Sha256::new();
    let mut count = 0u64;
    let mut size = 0u64;
    let mut line = Vec::new();
    loop {
        line.clear();
        (&mut input)
            .take(MAX_DOCUMENT_BYTES + 1)
            .read_until(b'\n', &mut line)
            .map_err(|e| e.to_string())?;
        if line.is_empty() {
            break;
        }
        require(
            line.len() as u64 <= MAX_DOCUMENT_BYTES && line.last() == Some(&b'\n'),
            "progress record truncated or exceeds bound",
        )?;
        let event: ProgressEvent = decode(&line)?;
        if count == 0 {
            require(
                matches!(&event,ProgressEvent::Campaign{campaign_sha256} if campaign_sha256==identity),
                "progress belongs to another campaign",
            )?;
        } else {
            require(
                !matches!(event, ProgressEvent::Campaign { .. }),
                "progress repeated its campaign header",
            )?;
        }
        observe(&event)?;
        count += 1;
        size = size
            .checked_add(line.len() as u64)
            .ok_or("progress byte overflow")?;
        require(
            count <= receipt.records && size <= receipt.bytes,
            "progress exceeded its receipt",
        )?;
        hash.update(&line);
    }
    let actual = hash
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    require(
        count == receipt.records && size == receipt.bytes && actual == receipt.sha256.as_str(),
        "progress records or digest differ",
    )
}

#[derive(Default)]
struct PointAudit {
    files: u64,
    bytes: u64,
    pages: u64,
    page_bytes: Option<u64>,
    resident_pages: u64,
    inventory: Option<Digest>,
    cold: Option<Digest>,
    final_readback_started: Option<u64>,
}

/// One small accumulator per declared point/seed, never one per file/attempt.
pub(super) struct ProgressAudit {
    points: BTreeMap<(Id, u64), PointAudit>,
    failed: bool,
    reader_observed: Option<Digest>,
    reader_bound: Option<Digest>,
}

impl ProgressAudit {
    fn new(campaign: &StorageCampaign) -> Self {
        Self {
            points: campaign
                .points
                .iter()
                .flat_map(|point| {
                    campaign
                        .seeds
                        .iter()
                        .map(move |seed| ((point.id.clone(), *seed), PointAudit::default()))
                })
                .collect(),
            failed: false,
            reader_observed: None,
            reader_bound: None,
        }
    }

    fn observe(&mut self, campaign: &StorageCampaign, event: &ProgressEvent) -> Result<()> {
        match event {
            ProgressEvent::Failure { .. } => self.failed = true,
            ProgressEvent::ReaderInterference {
                segment,
                observation,
            } => {
                observation.validate()?;
                self.reader_observed = Some(digest(
                    "c07g.reader-progress",
                    &(&self.reader_observed, segment, observation),
                )?);
            }
            ProgressEvent::ColdFile {
                point,
                seed,
                file_identity_sha256,
                bytes,
                page_bytes,
                pages,
                resident_pages,
            } => {
                let state = self
                    .points
                    .get_mut(&(point.clone(), *seed))
                    .ok_or("cold file names an undeclared point or seed")?;
                require(
                    state.cold.is_none() && state.final_readback_started.is_none(),
                    "cold file follows completed observation",
                )?;
                require(
                    page_bytes.is_power_of_two()
                        && (4096..=65536).contains(page_bytes)
                        && *resident_pages <= *pages
                        && *pages == bytes.div_ceil(*page_bytes)
                        && state.page_bytes.is_none_or(|prior| prior == *page_bytes),
                    "cold file page counts are impossible",
                )?;
                state.page_bytes = Some(*page_bytes);
                for (total, value) in [
                    (&mut state.files, 1),
                    (&mut state.bytes, *bytes),
                    (&mut state.pages, *pages),
                    (&mut state.resident_pages, *resident_pages),
                ] {
                    *total = total
                        .checked_add(value)
                        .ok_or("cold progress count overflow")?;
                }
                let policy = &campaign.storage.installed_configuration.storage;
                require(
                    state.files <= policy.recovery_files
                        && state.bytes <= policy.recovery_read_bytes,
                    "cold progress exceeds frozen inventory admission",
                )?;
                let prior = match state.inventory.take() {
                    Some(hash) => hash,
                    None => digest("c07g.cold-inventory", seed)?,
                };
                state.inventory = Some(digest(
                    "c07g.cold-inventory",
                    &(prior, file_identity_sha256),
                )?);
            }
            ProgressEvent::ColdStart {
                point,
                seed,
                observation,
            } => {
                let state = self
                    .points
                    .get_mut(&(point.clone(), *seed))
                    .ok_or("cold receipt names an undeclared point or seed")?;
                require(
                    state.cold.is_none()
                        && state.final_readback_started.is_none()
                        && state.files > 0
                        && state.pages > 0
                        && state.resident_pages == 0
                        && observation.files == state.files
                        && observation.file_bytes == state.bytes
                        && observation.pages == state.pages
                        && Some(observation.page_bytes) == state.page_bytes
                        && observation.resident_pages == state.resident_pages
                        && state.inventory.as_ref() == Some(&observation.inventory_sha256),
                    "cold receipt differs from raw file observations",
                )?;
                state.cold = Some(digest("c07g.cold-start", observation)?);
            }
            ProgressEvent::FinalReadback {
                point,
                seed,
                killed_pid,
                reopened_pid,
                seed_records,
                working_slots,
                started_ns,
                finished_ns,
                ..
            } => {
                let state = self
                    .points
                    .get_mut(&(point.clone(), *seed))
                    .ok_or("final readback names an undeclared point or seed")?;
                require(
                    state.final_readback_started.is_none()
                        && *killed_pid > 0
                        && *reopened_pid > 0
                        && *seed_records == campaign.allocation.records.get()
                        && *working_slots <= campaign.evidence.segment_attempts.get()
                        && started_ns <= finished_ns
                        && finished_ns - started_ns
                            <= campaign.schedule.lifecycle_ms.get() * 1_000_000,
                    "final readback is duplicate, incomplete or outside lifecycle admission",
                )?;
                state.final_readback_started = Some(*started_ns);
            }
            _ => {}
        }
        Ok(())
    }

    fn finish(&self, campaign: &StorageCampaign, status: CampaignStatus) -> Result<()> {
        if status == CampaignStatus::Completed {
            require(
                !self.failed
                    && self.points.values().all(|point| {
                        point.final_readback_started.is_some()
                            && (point.files == 0 || point.cold.is_some())
                    }),
                "completed campaign omitted final readback or retained a failure",
            )?;
            if campaign.required_lanes.contains(&CapacityLane::Cold) {
                require(
                    self.points.values().all(|point| point.cold.is_some()),
                    "completed campaign omitted required cold observations",
                )?;
            }
        }
        Ok(())
    }

    pub(super) fn bind_segment(&mut self, segment: &SustainedSegment) -> Result<()> {
        self.check_readback_order(&segment.point, segment.seed, segment.finished_ns)?;
        if let Some(reader) = &segment.reader {
            self.reader_bound = Some(digest(
                "c07g.reader-progress",
                &(&self.reader_bound, segment.index, reader),
            )?);
        }
        if let Some(cold) = &segment.cold_start {
            let state = self
                .points
                .get_mut(&(segment.point.clone(), segment.seed))
                .ok_or("cold segment names undeclared point or seed")?;
            require(
                state.cold.take().as_ref() == Some(&digest("c07g.cold-start", cold)?),
                "cold segment differs from progress observations or repeats them",
            )?;
        }
        Ok(())
    }

    pub(super) fn check_readback_order(
        &self,
        point: &Id,
        seed: u64,
        finished_ns: u64,
    ) -> Result<()> {
        let state = self
            .points
            .get(&(point.clone(), seed))
            .ok_or("segment names undeclared point or seed")?;
        require(
            state
                .final_readback_started
                .is_none_or(|start| start >= finished_ns),
            "final readback precedes completed offered load",
        )
    }

    pub(super) fn require_bound(&self, status: CampaignStatus) -> Result<()> {
        if status == CampaignStatus::Completed {
            require(
                self.reader_observed == self.reader_bound,
                "reader receipts differ from raw progress",
            )?;
        }
        if status == CampaignStatus::Completed {
            require(
                self.points.values().all(|point| point.cold.is_none()),
                "completed campaign omitted observed cold receipt from segments",
            )?;
        }
        Ok(())
    }
}

pub(super) fn verify_progress(
    directory: &Path,
    receipt: &ProgressReceipt,
    campaign: &StorageCampaign,
    status: CampaignStatus,
) -> Result<ProgressAudit> {
    let identity = campaign.sha256()?;
    let mut audit = ProgressAudit::new(campaign);
    scan(
        directory,
        receipt,
        &identity,
        campaign.evidence.total_bytes.get(),
        |event| audit.observe(campaign, event),
    )?;
    audit.finish(campaign, status)?;
    Ok(audit)
}

#[cfg(test)]
fn verify(
    directory: &Path,
    receipt: &ProgressReceipt,
    identity: &Digest,
    total_bound: u64,
) -> Result<()> {
    scan(directory, receipt, identity, total_bound, |_| Ok(()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_survives_failed_run_and_rejects_missing_truncated_or_changed_bytes() {
        let temporary = tempfile::tempdir().unwrap();
        let identity = digest("progress-fixture", &1).unwrap();
        let mut writer = ProgressJournal::create(temporary.path(), &identity).unwrap();
        writer
            .append(
                &ProgressEvent::Failure {
                    reason: "driver-deadline".to_owned().try_into().unwrap(),
                },
                MAX_DOCUMENT_BYTES,
            )
            .unwrap();
        writer.checkpoint().unwrap();
        let receipt = writer.finish().unwrap();
        assert_eq!(receipt.records, 2);
        verify(temporary.path(), &receipt, &identity, MAX_DOCUMENT_BYTES).unwrap();
        assert!(verify(
            temporary.path(),
            &receipt,
            &digest("other", &1).unwrap(),
            MAX_DOCUMENT_BYTES
        )
        .is_err());
        let path = temporary.path().join("progress.jsonl");
        let original = std::fs::read(&path).unwrap();
        let mut changed = original.clone();
        let at = changed
            .windows(b"driver-deadline".len())
            .position(|s| s == b"driver-deadline")
            .unwrap();
        changed[at] = b'x';
        std::fs::write(&path, changed).unwrap();
        assert!(verify(temporary.path(), &receipt, &identity, MAX_DOCUMENT_BYTES).is_err());
        std::fs::write(&path, &original[..original.len() - 1]).unwrap();
        assert!(verify(temporary.path(), &receipt, &identity, MAX_DOCUMENT_BYTES).is_err());
        std::fs::remove_file(&path).unwrap();
        assert!(verify(temporary.path(), &receipt, &identity, MAX_DOCUMENT_BYTES).is_err());
    }

    #[test]
    fn progress_quota_rejection_preserves_last_checkpoint() {
        let temporary = tempfile::tempdir().unwrap();
        let identity = digest("progress-fixture", &1).unwrap();
        let mut writer = ProgressJournal::create(temporary.path(), &identity).unwrap();
        let before = std::fs::read(temporary.path().join("progress.jsonl")).unwrap();
        assert!(writer
            .append(
                &ProgressEvent::Failure {
                    reason: "not-admitted".to_owned().try_into().unwrap(),
                },
                writer.bytes()
            )
            .is_err());
        let receipt = writer.finish().unwrap();
        assert_eq!(
            std::fs::read(temporary.path().join("progress.jsonl")).unwrap(),
            before
        );
        assert_eq!(receipt.records, 1);
        verify(temporary.path(), &receipt, &identity, MAX_DOCUMENT_BYTES).unwrap();
    }
}
