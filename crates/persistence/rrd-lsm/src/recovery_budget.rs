//! Admission for recovery and read-only inspection; denial never repairs bytes.
use crate::filesystem::{file_io, AnchoredPath};
use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoveryLimits {
    pub metadata_bytes: u64,
    pub files: u64,
    pub read_bytes: u64,
    pub wal_bytes: u64,
    pub wal_frames: u64,
    pub elapsed_ms: u64,
}

impl Default for RecoveryLimits {
    fn default() -> Self {
        Self {
            metadata_bytes: 64 * 1024 * 1024,
            files: 262_144,
            read_bytes: 1 << 40,
            wal_bytes: 256 * 1024 * 1024,
            wal_frames: 1_048_576,
            elapsed_ms: 120_000,
        }
    }
}

impl RecoveryLimits {
    pub(crate) fn validate(self) -> Result<()> {
        if [
            self.metadata_bytes,
            self.files,
            self.read_bytes,
            self.wal_bytes,
            self.wal_frames,
            self.elapsed_ms,
        ]
        .contains(&0)
        {
            return Err(Error::InvalidConfiguration(
                "recovery limits must be positive".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryResource {
    MetadataBytes,
    Files,
    ReadBytes,
    WalBytes,
    WalFrames,
    ElapsedMs,
}

impl RecoveryResource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MetadataBytes => "metadata_bytes",
            Self::Files => "files",
            Self::ReadBytes => "read_bytes",
            Self::WalBytes => "wal_bytes",
            Self::WalFrames => "wal_frames",
            Self::ElapsedMs => "elapsed_ms",
        }
    }
}

pub(crate) struct RecoveryBudget {
    limits: RecoveryLimits,
    files: u64,
    read_bytes: u64,
    metadata_bytes: u64,
    frames: u64,
    started: Instant,
}

impl RecoveryBudget {
    pub(crate) fn new(limits: RecoveryLimits) -> Self {
        Self {
            limits,
            files: 0,
            read_bytes: 0,
            metadata_bytes: 0,
            frames: 0,
            started: Instant::now(),
        }
    }

    fn check(&self, resource: RecoveryResource, limit: u64, observed: u64) -> Result<()> {
        if observed > limit {
            return Err(Error::RecoveryLimit {
                resource,
                limit,
                observed,
            });
        }
        Ok(())
    }

    pub(crate) fn tick(&self) -> Result<()> {
        self.check(
            RecoveryResource::ElapsedMs,
            self.limits.elapsed_ms,
            u64::try_from(self.started.elapsed().as_millis()).unwrap_or(u64::MAX),
        )
    }

    /// Conservative whole-file charge before opening/decoding. Re-reading a
    /// file for the inspector's digest consumes another admitted file/read.
    pub(crate) fn file(&mut self, bytes: u64) -> Result<()> {
        self.tick()?;
        let files = self
            .files
            .checked_add(1)
            .ok_or_else(|| Error::InvalidManifest("recovery file count overflow".into()))?;
        let read = self
            .read_bytes
            .checked_add(bytes)
            .ok_or_else(|| Error::InvalidManifest("recovery read charge overflow".into()))?;
        self.check(RecoveryResource::Files, self.limits.files, files)?;
        self.check(RecoveryResource::ReadBytes, self.limits.read_bytes, read)?;
        self.files = files;
        self.read_bytes = read;
        Ok(())
    }

    pub(crate) fn wal(&mut self, bytes: u64) -> Result<()> {
        self.check(RecoveryResource::WalBytes, self.limits.wal_bytes, bytes)?;
        self.file(bytes)
    }

    pub(crate) fn frame(&mut self) -> Result<()> {
        self.tick()?;
        let frames = self
            .frames
            .checked_add(1)
            .ok_or_else(|| Error::InvalidManifest("recovery frame count overflow".into()))?;
        self.check(RecoveryResource::WalFrames, self.limits.wal_frames, frames)?;
        self.frames = frames;
        Ok(())
    }

    pub(crate) fn metadata(&mut self, path: &AnchoredPath) -> Result<Vec<u8>> {
        self.tick()?;
        let file = path.open_read()?;
        let metadata = file.metadata()?;
        if !metadata.is_file() {
            return Err(Error::InvalidManifest(
                "recovery metadata is not a regular file".into(),
            ));
        }
        let next = self
            .metadata_bytes
            .checked_add(metadata.len())
            .ok_or_else(|| Error::InvalidManifest("recovery metadata charge overflow".into()))?;
        self.check(
            RecoveryResource::MetadataBytes,
            self.limits.metadata_bytes,
            next,
        )?;
        self.file(metadata.len())?;
        // One extra byte detects a growing file without an unbounded read.
        let bytes = file_io::read_to_limit(
            file,
            path.diagnostic_path(),
            metadata.len().saturating_add(1),
        )?;
        if bytes.len() as u64 != metadata.len() {
            return Err(Error::InvalidManifest(
                "recovery metadata changed while reading".into(),
            ));
        }
        self.metadata_bytes = next;
        self.tick()?;
        Ok(bytes)
    }
}
