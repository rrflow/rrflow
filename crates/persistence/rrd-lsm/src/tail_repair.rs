//! Offline, explicitly authorized removal of an incomplete final WAL frame.
//! The ordinary reader still rejects torn tails; complete corruption always fails.
use crate::database::wal_path;
use crate::filesystem::{AnchoredPath, OpenOptions};
use crate::recovery_budget::RecoveryBudget;
use crate::{DatabaseInspection, DatabaseOptions, Error, ReadOnlyDatabase, Result};
use rrd_core::digest::{sha256_hex, Sha256};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TailRepairPlan {
    pub format_version: u16,
    pub before: DatabaseInspection,
    pub wal_relative_path: String,
    pub valid_bytes: u64,
    pub prefix_sha256: String,
    pub discarded_bytes: u64,
    pub plan_sha256: String,
}

impl TailRepairPlan {
    pub fn calculated_sha256(&self) -> Result<String> {
        Ok(sha256_hex(&serde_json::to_vec(&(
            "rrflow-native-tail-repair-v1",
            self.format_version,
            &self.before,
            &self.wal_relative_path,
            self.valid_bytes,
            &self.prefix_sha256,
            self.discarded_bytes,
        ))?))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TailRepairReceipt {
    pub plan_sha256: String,
    pub retained_bytes: u64,
    pub discarded_bytes: u64,
    /// The exact approved prefix was already present. Sync is still retried.
    pub idempotent_replay: bool,
}

/// Holds the exclusive native lease from inspection through truncation/sync.
/// Reading the valid prefix does not authorize its mutation.
pub struct TailRepair {
    reader: ReadOnlyDatabase,
    wal: File,
    budget: RecoveryBudget,
    tail: Option<u64>,
    wal_relative_path: String,
    prefix_sha256: String,
}

impl TailRepair {
    pub fn open_anchored(root: &AnchoredPath, options: DatabaseOptions) -> Result<Self> {
        let root = root.pin_directory()?;
        let (reader, tail, mut budget) = ReadOnlyDatabase::open_inner(&root, options, true)?;
        let path = wal_path(&root, reader.manifest().wal_start_sequence);
        let mut wal = OpenOptions::new().read(true).write(true).open(&path)?;
        let prefix_sha256 =
            hash_prefix(&mut wal, reader.inspection().wal_valid_bytes, &mut budget)?;
        Ok(Self {
            reader,
            wal,
            budget,
            tail,
            wal_relative_path: path.relative_path().to_string_lossy().replace('\\', "/"),
            prefix_sha256,
        })
    }

    pub fn reader(&self) -> &ReadOnlyDatabase {
        &self.reader
    }

    pub fn plan(&self) -> Result<TailRepairPlan> {
        let valid_bytes = self.tail.ok_or_else(|| {
            Error::InvalidManifest("active WAL has no incomplete final frame to repair".into())
        })?;
        let before = self.reader.inspection().clone();
        let bytes = before
            .files
            .iter()
            .find(|f| f.relative_path == self.wal_relative_path)
            .ok_or_else(|| Error::InvalidManifest("active WAL inventory missing".into()))?
            .bytes;
        let discarded_bytes = bytes
            .checked_sub(valid_bytes)
            .filter(|n| *n != 0)
            .ok_or_else(|| Error::InvalidManifest("invalid incomplete tail extent".into()))?;
        let mut plan = TailRepairPlan {
            format_version: 1,
            before,
            wal_relative_path: self.wal_relative_path.clone(),
            valid_bytes,
            prefix_sha256: self.prefix_sha256.clone(),
            discarded_bytes,
            plan_sha256: String::new(),
        };
        plan.plan_sha256 = plan.calculated_sha256()?;
        Ok(plan)
    }

    pub fn apply(
        mut self,
        plan: &TailRepairPlan,
        expected_sha256: &str,
    ) -> Result<TailRepairReceipt> {
        if plan.format_version != 1
            || plan.plan_sha256 != expected_sha256
            || plan.plan_sha256 != plan.calculated_sha256()?
            || plan.discarded_bytes == 0
            || plan.wal_relative_path != self.wal_relative_path
            || plan.valid_bytes != self.reader.inspection().wal_valid_bytes
            || plan.prefix_sha256 != self.prefix_sha256
        {
            return Err(Error::InvalidManifest(
                "tail repair approval or prefix differs".into(),
            ));
        }
        let replay = self.tail.is_none();
        if replay {
            let mut after = plan.before.clone();
            let wal = after
                .files
                .iter_mut()
                .find(|f| f.relative_path == self.wal_relative_path)
                .ok_or_else(|| Error::InvalidManifest("approved WAL inventory missing".into()))?;
            if wal.bytes.checked_sub(plan.valid_bytes) != Some(plan.discarded_bytes) {
                return Err(Error::InvalidManifest(
                    "approved tail extent differs".into(),
                ));
            }
            wal.bytes = plan.valid_bytes;
            wal.sha256.clone_from(&plan.prefix_sha256);
            after.total_bytes = after
                .total_bytes
                .checked_sub(plan.discarded_bytes)
                .ok_or_else(|| {
                    Error::InvalidManifest("approved inventory size underflow".into())
                })?;
            if &after != self.reader.inspection() {
                return Err(Error::InvalidManifest(
                    "repair replay physical closure differs".into(),
                ));
            }
        } else if self.plan()? != *plan {
            return Err(Error::InvalidManifest("repair plan is stale".into()));
        }
        // Check the retained handle again immediately before the effect. A
        // cooperative writer cannot enter while the inspection lease is held.
        let bytes = self.wal.metadata()?.len();
        let expected_bytes = if replay {
            plan.valid_bytes
        } else {
            plan.valid_bytes
                .checked_add(plan.discarded_bytes)
                .ok_or_else(|| Error::InvalidManifest("repair length overflow".into()))?
        };
        if bytes != expected_bytes {
            return Err(Error::InvalidManifest(
                "repair WAL changed after inspection".into(),
            ));
        }
        let actual = hash_prefix(&mut self.wal, bytes, &mut self.budget)?;
        let expected = if replay {
            &plan.prefix_sha256
        } else {
            &plan
                .before
                .files
                .iter()
                .find(|f| f.relative_path == self.wal_relative_path)
                .ok_or_else(|| Error::InvalidManifest("approved WAL missing".into()))?
                .sha256
        };
        if &actual != expected {
            return Err(Error::InvalidManifest("repair WAL digest changed".into()));
        }
        self.budget.tick()?;
        if !replay {
            self.wal.set_len(plan.valid_bytes)?;
        }
        self.wal.sync_all()?;
        Ok(TailRepairReceipt {
            plan_sha256: plan.plan_sha256.clone(),
            retained_bytes: plan.valid_bytes,
            discarded_bytes: plan.discarded_bytes,
            idempotent_replay: replay,
        })
    }
}

fn hash_prefix(file: &mut File, bytes: u64, budget: &mut RecoveryBudget) -> Result<String> {
    budget.file(bytes)?;
    file.seek(SeekFrom::Start(0))?;
    let mut remaining = bytes;
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    while remaining != 0 {
        budget.tick()?;
        let count = remaining.min(buffer.len() as u64) as usize;
        file.read_exact(&mut buffer[..count])?;
        hash.update(&buffer[..count]);
        remaining -= count as u64;
    }
    budget.tick()?;
    Ok(hash.finalize_hex())
}
