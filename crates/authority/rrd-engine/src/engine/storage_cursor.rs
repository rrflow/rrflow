//! Engine-owned ephemeral physical readers; no durable or client lifecycle authority.
use super::*;
use rrd_contract::{InspectStorageCursor, StorageCursorAction, StorageCursorSnapshot};
use rrd_store::StorageInspectionCursor;

pub(super) struct RetainedCursor {
    cursor: StorageInspectionCursor,
    opened: StorageCursorAction,
    pub(super) deadline: Instant,
}
pub(super) type CursorRegistry = BTreeMap<(String, String), RetainedCursor>;

impl RrdEngine {
    /// Called by the installed server's owned sweep, or an embedded host.
    /// Expiry drops engine-owned buffers only; no externally owned buffer is invalidated.
    pub fn reap_storage_cursors(&self) -> Result<()> {
        self.storage_cursors
            .lock()
            .map_err(|_| ServiceError::Storage("storage cursor registry poisoned".into()))?
            .retain(|_, entry| Instant::now() < entry.deadline);
        Ok(())
    }
    pub(super) fn release_session_storage_cursors(&self, session: &CorrelationId) -> Result<()> {
        self.storage_cursors
            .lock()
            .map_err(|_| ServiceError::Storage("storage cursor registry poisoned".into()))?
            .retain(|(owner, _), _| owner != session.as_str());
        Ok(())
    }
    pub fn inspect_storage_cursor(
        &self,
        session: &CorrelationId,
        token: &CorrelationId,
        request: &InspectStorageCursor,
        now: u64,
        request_id: &str,
        operation_id: &str,
    ) -> Result<StorageCursorSnapshot> {
        request
            .validate()
            .map_err(|e| ServiceError::Contract(e.to_string()))?;
        self.authorize(
            session,
            token,
            SecurityAction::DiagnosticsRead,
            now,
            request_id,
            operation_id,
        )?;
        self.query_scope(&request.scope)?;
        let mut registry = self
            .storage_cursors
            .lock()
            .map_err(|_| ServiceError::Storage("storage cursor registry poisoned".into()))?;
        registry.retain(|_, entry| Instant::now() < entry.deadline);
        // Recheck after taking registry ownership: a concurrent close cannot leave a new orphan.
        let (_, state) = self.authorize(
            session,
            token,
            SecurityAction::DiagnosticsRead,
            now,
            request_id,
            operation_id,
        )?;
        let key = (
            session.as_str().to_owned(),
            request.cursor_id.as_str().to_owned(),
        );
        if let StorageCursorAction::Release {} = request.action {
            registry.remove(&key);
            return Ok(StorageCursorSnapshot {
                cursor_id: request.cursor_id.clone(),
                released: true,
                source_open: false,
                snapshot_sequence: 0,
                page_ordinal: 0,
                rows: 0,
                allocated_bytes: 0,
                page_sha256: digest::sha256_hex(b"rrflow-storage-cursor-released-v1"),
            });
        }
        if let StorageCursorAction::Open {
            batch_rows,
            batch_bytes,
            lifetime_ms,
        } = request.action
        {
            if let Some(entry) = registry.get(&key) {
                if entry.opened != request.action {
                    return Err(ServiceError::IdempotencyConflict);
                }
            } else {
                let limit = self.estate_configuration.storage.active_read_views.min(64) as usize;
                if registry.len() >= limit {
                    return Err(ServiceError::StorageAdmission(
                        "storage inspection cursor slots exhausted".into(),
                    ));
                }
                let store = self.storage.as_rrflow_kv().ok_or_else(|| {
                    ServiceError::Contract("physical inspection requires rrflowKV".into())
                })?;
                let lifetime = u64::from(lifetime_ms)
                    .min(state.idle_expires_at_unix_ms.saturating_sub(now))
                    .min(state.absolute_expires_at_unix_ms.saturating_sub(now));
                if lifetime == 0 {
                    return Err(ServiceError::SessionNotFound);
                }
                let deadline = Instant::now() + Duration::from_millis(lifetime);
                let cursor =
                    store.inspect_storage(usize::from(batch_rows), batch_bytes as usize)?;
                if Instant::now() >= deadline {
                    return Err(ServiceError::Contract(
                        "storage cursor lifetime elapsed during capture".into(),
                    ));
                }
                registry.insert(
                    key.clone(),
                    RetainedCursor {
                        cursor,
                        opened: request.action.clone(),
                        deadline,
                    },
                );
            }
        }
        let entry = registry
            .get_mut(&key)
            .ok_or_else(|| ServiceError::Contract("storage cursor absent or expired".into()))?;
        match request.action {
            StorageCursorAction::Advance { expected_ordinal } => {
                let page = entry.cursor.page()?;
                if expected_ordinal != page.ordinal
                    && expected_ordinal.checked_add(1) != Some(page.ordinal)
                {
                    return Err(ServiceError::Contract(
                        "stale storage cursor ordinal".into(),
                    ));
                }
                if expected_ordinal == page.ordinal && !page.source_open {
                    return Err(ServiceError::Contract(
                        "storage cursor source already closed".into(),
                    ));
                }
                if let Err(error) = entry.cursor.advance(expected_ordinal) {
                    registry.remove(&key);
                    return Err(error.into());
                }
            }
            StorageCursorAction::CancelSource {} => entry.cursor.cancel_source()?,
            _ => {}
        }
        let page = entry.cursor.page()?;
        let result = StorageCursorSnapshot {
            cursor_id: request.cursor_id.clone(),
            released: false,
            source_open: page.source_open,
            snapshot_sequence: page.sequence,
            page_ordinal: page.ordinal,
            rows: page.rows,
            allocated_bytes: page.allocated_bytes,
            page_sha256: page.sha256,
        };
        result
            .validate()
            .map_err(|e| ServiceError::Contract(e.to_string()))?;
        Ok(result)
    }
}
