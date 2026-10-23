//! Ephemeral, bounded inspection of an installed physical storage snapshot.
use crate::{invalid, CorrelationId, ReadStorageDiagnostics, Result};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InspectStorageCursor {
    pub scope: String,
    pub cursor_id: CorrelationId,
    pub action: StorageCursorAction,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum StorageCursorAction {
    Open {
        batch_rows: u16,
        batch_bytes: u32,
        lifetime_ms: u32,
    },
    Observe {},
    Advance {
        expected_ordinal: u64,
    },
    CancelSource {},
    Release {},
}
impl InspectStorageCursor {
    pub fn validate(&self) -> Result<()> {
        ReadStorageDiagnostics {
            scope: self.scope.clone(),
        }
        .validate()?;
        if let StorageCursorAction::Open {
            batch_rows,
            batch_bytes,
            lifetime_ms,
        } = self.action
        {
            if !(1..=4096).contains(&batch_rows)
                || !(1..=4 * 1024 * 1024).contains(&batch_bytes)
                || !(1..=60_000).contains(&lifetime_ms)
            {
                return invalid("invalid storage cursor batch or lifetime budget");
            }
        }
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StorageCursorSnapshot {
    pub cursor_id: CorrelationId,
    pub released: bool,
    pub source_open: bool,
    pub snapshot_sequence: u64,
    pub page_ordinal: u64,
    pub rows: u64,
    pub allocated_bytes: u64,
    pub page_sha256: String,
}
impl StorageCursorSnapshot {
    pub fn validate(&self) -> Result<()> {
        crate::validate_sha256(&self.page_sha256, "page_sha256")?;
        if self.rows > 4096
            || self.allocated_bytes > 4 * 1024 * 1024
            || (self.released && (self.source_open || self.rows != 0 || self.allocated_bytes != 0))
        {
            return invalid("invalid storage cursor ownership snapshot");
        }
        Ok(())
    }
}
