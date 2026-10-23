//! Actual installed physical stream/batch lifetime across concurrent load.
use super::*;
use rrd_contract::{
    DiagnosticStorageSnapshot, InspectStorageCursor, StorageCursorAction, StorageCursorSnapshot,
};
pub(super) struct HeldReader {
    id: String,
    opened_ns: u64,
    opened: StorageCursorSnapshot,
    baseline: DiagnosticStorageSnapshot,
}
async fn cursor(
    work: &Workload,
    id: &str,
    action: StorageCursorAction,
) -> DriverResult<StorageCursorSnapshot> {
    work.client
        .inspect_storage_cursor(
            &work.session,
            InspectStorageCursor {
                scope: format!("instance:{}", work.client.instance_id()),
                cursor_id: rrd_contract::CorrelationId::new(id).map_err(|e| e.to_string())?,
                action,
            },
            RequestOptions::read(id, id).map_err(|e| e.to_string())?,
        )
        .await
        .map_err(|e| e.to_string())
}
async fn diagnostics(work: &Workload, id: &str) -> DriverResult<DiagnosticStorageSnapshot> {
    work.client
        .read_storage_diagnostics(
            &work.session,
            ReadStorageDiagnostics {
                scope: format!("instance:{}", work.client.instance_id()),
            },
            RequestOptions::read(id, id).map_err(|e| e.to_string())?,
        )
        .await
        .map_err(|e| e.to_string())
}
impl HeldReader {
    pub(super) async fn open(work: &Workload, segment: u64, clock: Instant) -> DriverResult<Self> {
        let id = format!("capacity-reader-{segment}");
        let baseline = diagnostics(work, &id).await?;
        let opened = cursor(
            work,
            &id,
            StorageCursorAction::Open {
                batch_rows: 1,
                batch_bytes: 4 * 1024 * 1024,
                lifetime_ms: 60_000,
            },
        )
        .await?;
        if !opened.source_open || opened.rows == 0 {
            return Err("reader interference requires nonempty native source".into());
        }
        Ok(Self {
            id,
            opened_ns: ns(clock),
            opened,
            baseline,
        })
    }
    pub(super) async fn finish(
        self,
        work: &Workload,
        clock: Instant,
    ) -> DriverResult<ReaderInterferenceEvidence> {
        let observed = cursor(work, &self.id, StorageCursorAction::Observe {}).await?;
        let observed_ns = ns(clock);
        let retained = diagnostics(work, &self.id).await?;
        let cancelled = cursor(work, &self.id, StorageCursorAction::CancelSource {}).await?;
        let source_released = diagnostics(work, &self.id).await?;
        let released = cursor(work, &self.id, StorageCursorAction::Release {}).await?;
        let drained = diagnostics(work, &self.id).await?;
        let result = ReaderInterferenceEvidence {
            opened_ns: self.opened_ns,
            observed_ns,
            released_ns: ns(clock),
            opened: self.opened,
            observed,
            cancelled,
            released,
            baseline: self.baseline.admission.ok_or("missing baseline owners")?,
            retained: retained.admission.ok_or("missing retained owners")?,
            source_released: source_released
                .admission
                .ok_or("missing source release owners")?,
            drained: drained.admission.ok_or("missing drained owners")?,
            maintenance_before: self
                .baseline
                .rrflow_kv
                .ok_or("missing native baseline")?
                .maintenance,
            maintenance_after: retained
                .rrflow_kv
                .ok_or("missing native retained counters")?
                .maintenance,
        };
        result.validate()?;
        Ok(result)
    }
}
