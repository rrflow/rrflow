//! Bounded append-only qualification progress. This is evidence, never engine
//! state: missing outcomes remain uncertain and cannot authorize replay.
use super::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use std::{
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

pub(super) fn verify(
    directory: &Path,
    receipt: &ProgressReceipt,
    identity: &Digest,
    total_bound: u64,
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
