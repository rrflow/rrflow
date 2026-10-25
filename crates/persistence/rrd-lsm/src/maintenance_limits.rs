//! Hard admission bounds for synchronous immutable maintenance. Logical input
//! accounts decoded page payloads, not allocator overhead or process RSS.

use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompactionLimits {
    pub input_bytes: u64,
    pub input_logical_bytes: u64,
    pub input_versions: u64,
    pub output_segments: u64,
    pub elapsed_ms: u64,
    pub l0_segments: u64,
}

impl Default for CompactionLimits {
    fn default() -> Self {
        Self {
            input_bytes: 1024 * 1024 * 1024,
            input_logical_bytes: 256 * 1024 * 1024,
            input_versions: 4_194_304,
            output_segments: 256,
            elapsed_ms: 30_000,
            l0_segments: 64,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MaintenanceResource {
    InputBytes,
    InputLogicalBytes,
    InputVersions,
    OutputSegments,
    ElapsedMs,
    L0Segments,
}

impl MaintenanceResource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InputBytes => "input_bytes",
            Self::InputLogicalBytes => "input_logical_bytes",
            Self::InputVersions => "input_versions",
            Self::OutputSegments => "output_segments",
            Self::ElapsedMs => "elapsed_ms",
            Self::L0Segments => "l0_segments",
        }
    }
}

impl CompactionLimits {
    pub(crate) fn validate(self) -> Result<()> {
        if self.input_bytes == 0
            || self.input_logical_bytes == 0
            || self.input_versions == 0
            || self.output_segments == 0
            || self.elapsed_ms == 0
            || self.l0_segments < 2
        {
            return Err(Error::InvalidConfiguration(
                "compaction limits must be positive and L0 capacity at least two".into(),
            ));
        }
        Ok(())
    }

    pub(crate) fn check(self, resource: MaintenanceResource, observed: u64) -> Result<()> {
        let limit = match resource {
            MaintenanceResource::InputBytes => self.input_bytes,
            MaintenanceResource::InputLogicalBytes => self.input_logical_bytes,
            MaintenanceResource::InputVersions => self.input_versions,
            MaintenanceResource::OutputSegments => self.output_segments,
            MaintenanceResource::ElapsedMs => self.elapsed_ms,
            MaintenanceResource::L0Segments => self.l0_segments,
        };
        if observed > limit {
            return Err(Error::MaintenanceLimit {
                resource,
                limit,
                observed,
            });
        }
        Ok(())
    }

    /// Cooperative deadline: checked between bounded units and before
    /// publication; cannot interrupt an operating-system read or sync call.
    pub(crate) fn check_elapsed(self, started: Instant) -> Result<()> {
        self.check(
            MaintenanceResource::ElapsedMs,
            u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
        )
    }
}
