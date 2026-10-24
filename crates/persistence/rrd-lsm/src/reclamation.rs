//! Bounded work for the existing manifest-reachability garbage collector.
use crate::filesystem::AnchoredPath;
use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReclamationLimits {
    pub inventory_entries: u64,
    pub metadata_bytes: u64,
    pub delete_files: u64,
    pub delete_bytes: u64,
    pub elapsed_ms: u64,
}

impl Default for ReclamationLimits {
    fn default() -> Self {
        Self {
            inventory_entries: 262_144,
            metadata_bytes: 64 * 1024 * 1024,
            delete_files: 1024,
            delete_bytes: 1024 * 1024 * 1024,
            elapsed_ms: 30_000,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReclamationResource {
    InventoryEntries,
    MetadataBytes,
    DeleteBytes,
    ElapsedMs,
}

impl ReclamationResource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InventoryEntries => "inventory_entries",
            Self::MetadataBytes => "metadata_bytes",
            Self::DeleteBytes => "delete_bytes",
            Self::ElapsedMs => "elapsed_ms",
        }
    }
}

impl ReclamationLimits {
    pub(crate) fn validate(self) -> Result<()> {
        if self.inventory_entries == 0
            || self.metadata_bytes == 0
            || self.delete_files == 0
            || self.delete_bytes == 0
            || self.elapsed_ms == 0
        {
            return Err(Error::InvalidConfiguration(
                "reclamation limits must be positive".into(),
            ));
        }
        Ok(())
    }
}

pub(crate) struct ReclamationBudget {
    pub limits: ReclamationLimits,
    pub entries: u64,
    pub metadata_bytes: u64,
    pub deleted_files: u64,
    pub deleted_bytes: u64,
    started: Instant,
}

impl ReclamationBudget {
    pub(crate) fn new(limits: ReclamationLimits) -> Self {
        Self {
            limits,
            entries: 0,
            metadata_bytes: 0,
            deleted_files: 0,
            deleted_bytes: 0,
            started: Instant::now(),
        }
    }

    pub(crate) fn tick(&self) -> Result<()> {
        self.check(
            ReclamationResource::ElapsedMs,
            self.limits.elapsed_ms,
            u64::try_from(self.started.elapsed().as_millis()).unwrap_or(u64::MAX),
        )
    }

    fn check(&self, resource: ReclamationResource, limit: u64, observed: u64) -> Result<()> {
        if observed > limit {
            return Err(Error::ReclamationLimit {
                resource,
                limit,
                observed,
            });
        }
        Ok(())
    }

    /// Charge examined references and names before retaining them, including
    /// duplicate references and ignored directory entries.
    pub(crate) fn entry(&mut self) -> Result<()> {
        self.charge_entries(1)
    }

    pub(crate) fn charge_entries(&mut self, entries: u64) -> Result<()> {
        self.tick()?;
        let next = self
            .entries
            .checked_add(entries)
            .ok_or_else(|| Error::InvalidManifest("reclamation inventory overflow".into()))?;
        self.check(
            ReclamationResource::InventoryEntries,
            self.limits.inventory_entries,
            next,
        )?;
        self.entries = next;
        Ok(())
    }

    pub(crate) fn read_metadata(&mut self, path: &AnchoredPath) -> Result<Vec<u8>> {
        self.tick()?;
        let file = path.open_read()?;
        let metadata = file.metadata()?;
        if !metadata.is_file() {
            return Err(Error::InvalidManifest(
                "reclamation metadata is not a regular file".into(),
            ));
        }
        let remaining = self.limits.metadata_bytes - self.metadata_bytes;
        self.check(
            ReclamationResource::MetadataBytes,
            self.limits.metadata_bytes,
            self.metadata_bytes.saturating_add(metadata.len()),
        )?;
        let bytes = crate::filesystem::file_io::read_to_limit(
            file,
            path.diagnostic_path(),
            remaining.saturating_add(1),
        )?;
        let observed = self.metadata_bytes.saturating_add(bytes.len() as u64);
        self.check(
            ReclamationResource::MetadataBytes,
            self.limits.metadata_bytes,
            observed,
        )?;
        if bytes.len() as u64 != metadata.len() {
            return Err(Error::InvalidManifest(
                "reclamation metadata changed while reading".into(),
            ));
        }
        self.metadata_bytes = observed;
        self.tick()?;
        Ok(bytes)
    }

    /// A full cycle may return a partial, safely reclaimable prefix. A single
    /// impossible file is explicit pressure, never silently skipped forever.
    pub(crate) fn admit_delete(&mut self, bytes: u64) -> Result<bool> {
        self.check(
            ReclamationResource::DeleteBytes,
            self.limits.delete_bytes,
            bytes,
        )?;
        if self.tick().is_err()
            || self.deleted_files == self.limits.delete_files
            || bytes > self.limits.delete_bytes - self.deleted_bytes
        {
            return Ok(false);
        }
        self.deleted_files += 1;
        self.deleted_bytes += bytes;
        Ok(true)
    }
}
