//! Assembling a diagnostic snapshot at one stamp.

use super::*;

pub(super) const MAX_DIAGNOSTIC_ASSEMBLY_ATTEMPTS: u8 = 3;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct EngineDiagnosticStamp {
    pub(super) claim_sequence: u64,
    pub(super) control_sequence: u64,
    pub(super) runtime: ReadStamp,
    pub(super) retention_sha256: String,
}

impl RrdEngine {
    /// Observe physical counters without traversing runtime/graph history.
    /// Authorization and instance scope are identical to full diagnostics;
    /// these counters are observations, not a cross-model consistency claim.
    pub fn read_storage_diagnostics(
        &self,
        session_id: &CorrelationId,
        token: &CorrelationId,
        request: &rrd_contract::ReadStorageDiagnostics,
        now: u64,
        request_id: &str,
        operation_id: &str,
    ) -> Result<DiagnosticStorageSnapshot> {
        request
            .validate()
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        self.authorize(
            session_id,
            token,
            SecurityAction::DiagnosticsRead,
            now,
            request_id,
            operation_id,
        )?;
        self.query_scope(&request.scope)?;
        let snapshot = diagnostic_storage(self, now)?;
        snapshot
            .validate()
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        Ok(snapshot)
    }

    /// Assembles a bounded cross-model diagnostic view under one verified RRD
    /// read stamp. Concurrent mutations cause a retry; a response is never
    /// returned with coordinates from two different runtime states.
    #[allow(clippy::too_many_arguments)]
    pub fn read_diagnostic_snapshot(
        &self,
        session_id: &CorrelationId,
        token: &CorrelationId,
        request: &ReadDiagnosticSnapshot,
        now: u64,
        request_id: &str,
        operation_id: &str,
    ) -> Result<DiagnosticSnapshot> {
        self.read_diagnostic_snapshot_with_observer(
            session_id,
            token,
            request,
            now,
            request_id,
            operation_id,
            &mut |_| Ok(()),
        )
    }

    /// Runs `during_assembly` inside every assembly attempt, after its first
    /// consistency stamp is captured, so a test can place concurrent work in
    /// the window that attempt compares.
    #[cfg(test)]
    #[allow(clippy::too_many_arguments)]
    pub(in super::super) fn read_diagnostic_snapshot_during_assembly(
        &self,
        session_id: &CorrelationId,
        token: &CorrelationId,
        request: &ReadDiagnosticSnapshot,
        now: u64,
        request_id: &str,
        operation_id: &str,
        during_assembly: &mut dyn FnMut(u8) -> Result<()>,
    ) -> Result<DiagnosticSnapshot> {
        self.read_diagnostic_snapshot_with_observer(
            session_id,
            token,
            request,
            now,
            request_id,
            operation_id,
            during_assembly,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn read_diagnostic_snapshot_with_observer(
        &self,
        session_id: &CorrelationId,
        token: &CorrelationId,
        request: &ReadDiagnosticSnapshot,
        now: u64,
        request_id: &str,
        operation_id: &str,
        observer: &mut dyn FnMut(u8) -> Result<()>,
    ) -> Result<DiagnosticSnapshot> {
        request
            .validate()
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        self.authorize(
            session_id,
            token,
            SecurityAction::DiagnosticsRead,
            now,
            request_id,
            operation_id,
        )?;
        let scope = self.query_scope(&request.scope)?;

        for attempt in 1..=MAX_DIAGNOSTIC_ASSEMBLY_ATTEMPTS {
            let before = capture_diagnostic_stamp(self, &scope, now)?;
            observer(attempt)?;
            let runtime_changes = match diagnostic_runtime_changes(
                self,
                &before.runtime,
                request.runtime_max_scanned_changes,
            ) {
                Ok(changes) => changes,
                Err(error) if error.retryable() => continue,
                Err(error) => return Err(error),
            };
            let schema = self
                .storage
                .runtime()
                .schema(&scope)?
                .as_ref()
                .map(super::super::model::public_schema)
                .transpose()?;
            let models = diagnostic_models(&request.scope, schema.as_ref())?;
            let (graph, graph_difference) = match diagnostic_graph(
                &before.runtime,
                &runtime_changes,
                &request.scope,
                request.graph_valid_at_unix_ms,
                request.graph_known_at_cursor,
                request.graph_compare_cursor,
                request.graph_max_records,
            ) {
                Ok(graph) => graph,
                Err(error) if error.retryable() => continue,
                Err(error) => return Err(error),
            };
            let vector_artifacts =
                diagnostic_vector_artifacts(&runtime_changes, &scope, &request.scope)?;
            let retention = diagnostic_retention(self, now)?;
            let retention_sha256 = diagnostic_retention_sha256(&retention)?;
            let query_indexes = diagnostic_query_indexes(self, &scope, &request.scope)?;
            let vector_collections = diagnostic_vector_collections(self, &scope, &request.scope)?;
            let estate = rrd_estate::EstateRepository::new(&self.storage, self.instance.clone())
                .load()?
                .as_ref()
                .map(rrd_estate::public_snapshot);
            let changes = self.read_changefeed_page(&ReadChangefeed {
                scope: request.scope.clone(),
                after_cursor: request.changes_after_cursor,
                limit: request.change_limit,
            })?;
            let audit = diagnostic_audit(
                self,
                request.audit_after_sequence,
                usize::from(request.audit_limit),
            )?;
            let product_capabilities = crate::product_capability_catalogue();
            let readiness = self.readiness(now)?;
            let after = capture_diagnostic_stamp(self, &scope, now)?;

            if before != after
                || readiness.claim_sequence != after.claim_sequence
                || readiness.runtime_cursor != after.runtime.commit_cursor
                || changes.head_cursor != after.runtime.commit_cursor
                || retention_sha256 != before.retention_sha256
                || retention_sha256 != after.retention_sha256
            {
                continue;
            }

            // Physical counters are an observation of this process, taken once
            // after the logical coordinates verified; they never gate a retry.
            let storage = diagnostic_storage(self, now)?;
            let sections = diagnostic_sections(
                after.runtime.commit_cursor,
                &product_capabilities,
                schema.as_ref(),
                &models,
                &graph,
                &graph_difference,
                &retention,
                &vector_artifacts,
                &query_indexes,
                &vector_collections,
                estate.as_ref(),
                &storage,
                &changes,
                &audit,
            )?;
            let snapshot = DiagnosticSnapshot {
                format_version: DIAGNOSTIC_SNAPSHOT_FORMAT_VERSION,
                observed_at_unix_ms: now,
                instance: ResourceId::new(ResourceKind::Instance, self.instance.as_str())
                    .expect("engine instance identity is canonical"),
                scope: request.scope.clone(),
                read: DiagnosticReadStamp {
                    claim_sequence: after.claim_sequence,
                    control_sequence: after.control_sequence,
                    runtime_cursor: after.runtime.commit_cursor,
                    schema_revision: after.runtime.schema_revision,
                    catalogue_revision: after.runtime.catalog_revision,
                    runtime_manifest_sha256: after.runtime.manifest_id,
                    retention_sha256: after.retention_sha256,
                    assembly_attempts: attempt,
                },
                readiness,
                product_capabilities,
                schema,
                models,
                graph,
                graph_difference,
                retention,
                vector_artifacts,
                query_indexes,
                vector_collections,
                estate,
                storage,
                changes,
                audit,
                sections,
            };
            snapshot
                .validate()
                .map_err(|error| ServiceError::Contract(error.to_string()))?;
            return Ok(snapshot);
        }

        Err(ServiceError::StorageConflict(format!(
            "diagnostic snapshot changed during all {MAX_DIAGNOSTIC_ASSEMBLY_ATTEMPTS} assembly attempts"
        )))
    }
}
