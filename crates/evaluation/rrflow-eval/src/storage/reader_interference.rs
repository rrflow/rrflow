//! Raw, independently checkable installed reader ownership transitions.
use super::*;
use rrd_contract::{
    DiagnosticMaintenanceSnapshot, DiagnosticStorageAdmission, StorageCursorSnapshot,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReaderInterferenceEvidence {
    pub opened_ns: u64,
    pub observed_ns: u64,
    pub released_ns: u64,
    pub opened: StorageCursorSnapshot,
    pub observed: StorageCursorSnapshot,
    pub cancelled: StorageCursorSnapshot,
    pub released: StorageCursorSnapshot,
    pub baseline: DiagnosticStorageAdmission,
    pub retained: DiagnosticStorageAdmission,
    pub source_released: DiagnosticStorageAdmission,
    pub drained: DiagnosticStorageAdmission,
    pub maintenance_before: DiagnosticMaintenanceSnapshot,
    pub maintenance_after: DiagnosticMaintenanceSnapshot,
}
impl ReaderInterferenceEvidence {
    pub fn validate(&self) -> Result<()> {
        for page in [
            &self.opened,
            &self.observed,
            &self.cancelled,
            &self.released,
        ] {
            page.validate().map_err(|e| e.to_string())?;
        }
        require(
            self.opened_ns <= self.observed_ns
                && self.observed_ns <= self.released_ns
                && self.released_ns - self.opened_ns <= 60_000_000_000,
            "reader interference times exceed lease lifetime",
        )?;
        require(
            self.opened == self.observed
                && self.opened.source_open
                && self.opened.rows > 0
                && self.opened.allocated_bytes > 0
                && !self.opened.released,
            "retained page changed or no live reader/output was held",
        )?;
        let mut expected_cancel = self.opened.clone();
        expected_cancel.source_open = false;
        require(
            self.cancelled == expected_cancel
                && self.released.released
                && self.released.cursor_id == self.opened.cursor_id,
            "source cancellation changed copied page or release identity",
        )?;
        require(
            self.retained.active_read_views > self.baseline.active_read_views
                && self.retained.retained_projected_bytes > self.baseline.retained_projected_bytes
                && self.source_released.active_read_views <= self.baseline.active_read_views
                && self.source_released.pinned_read_bytes <= self.baseline.pinned_read_bytes
                && self.source_released.retained_projected_bytes
                    > self.baseline.retained_projected_bytes
                && self.drained.active_read_views <= self.baseline.active_read_views
                && self.drained.pinned_read_bytes <= self.baseline.pinned_read_bytes
                && self.drained.retained_projected_bytes <= self.baseline.retained_projected_bytes,
            "reader source/output owners did not follow distinct release lifetimes",
        )
    }
    pub fn qualifies(&self, attempts: &[LoadAttempt]) -> bool {
        (self.maintenance_after.automatic_flushes > self.maintenance_before.automatic_flushes
            || self.maintenance_after.automatic_compactions
                > self.maintenance_before.automatic_compactions)
            && attempts.iter().any(|a| {
                a.kind == StorageLoadKind::MixedWrite
                    && a.outcome == LoadOutcome::Acknowledged
                    && a.started_ns >= self.opened_ns
                    && a.finished_ns <= self.observed_ns
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retained_evidence_rejects_changed_bytes_and_premature_release() {
        let owners = || DiagnosticStorageAdmission {
            native_transactions: 0,
            native_transaction_bytes: 0,
            active_read_views: 0,
            pinned_read_bytes: 0,
            retained_mutable_bytes: 0,
            retained_projected_bytes: 0,
            outstanding_requests: 0,
            outstanding_logical_bytes: 0,
            queued_requests: 0,
            active: false,
            fenced: false,
        };
        let maintenance = DiagnosticMaintenanceSnapshot {
            automatic_flushes: 0,
            failed_flushes: 0,
            write_stalls: 0,
            oversized_batches: 0,
            automatic_compactions: 0,
            failed_compactions: 0,
            compaction_input_bytes: 0,
            compaction_output_bytes: 0,
            peak_compaction_buffer_bytes: 0,
        };
        let page = StorageCursorSnapshot {
            cursor_id: rrd_contract::CorrelationId::new("reader").unwrap(),
            released: false,
            source_open: true,
            snapshot_sequence: 1,
            page_ordinal: 0,
            rows: 1,
            allocated_bytes: 128,
            page_sha256: "a".repeat(64),
        };
        let mut cancelled = page.clone();
        cancelled.source_open = false;
        let mut released = cancelled.clone();
        released.rows = 0;
        released.allocated_bytes = 0;
        released.released = true;
        let mut retained = owners();
        retained.active_read_views = 1;
        retained.pinned_read_bytes = 4096;
        retained.retained_projected_bytes = 128;
        let mut source_released = owners();
        source_released.retained_projected_bytes = 128;
        let valid = ReaderInterferenceEvidence {
            opened_ns: 1,
            observed_ns: 2,
            released_ns: 3,
            opened: page.clone(),
            observed: page,
            cancelled,
            released,
            baseline: owners(),
            retained,
            source_released,
            drained: owners(),
            maintenance_before: maintenance.clone(),
            maintenance_after: maintenance,
        };
        valid.validate().unwrap();
        assert!(!valid.qualifies(&[]));
        for defect in 0..5 {
            let mut invalid = valid.clone();
            match defect {
                0 => invalid.observed.page_sha256 = "b".repeat(64),
                1 => invalid.source_released.retained_projected_bytes = 0,
                2 => invalid.drained.pinned_read_bytes = 1,
                3 => invalid.retained.active_read_views = 0,
                _ => invalid.released_ns = 60_000_000_002,
            }
            assert!(invalid.validate().is_err());
        }
    }
}
