use super::*;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use rrd_contract::{
    ExportQuery, QueryExport, QueryExportSource, MAX_PARQUET_EXPORT_COLUMNS,
    MAX_PARQUET_EXPORT_INPUT_BYTES, MAX_PARQUET_EXPORT_ROWS, MAX_PARQUET_ROW_GROUP_ROWS,
    PARQUET_EXPORT_PROFILE,
};
use std::time::{Duration, Instant};

impl RrdEngine {
    #[allow(clippy::too_many_arguments)]
    pub fn export_query_scoped(
        &self,
        session_id: &CorrelationId,
        token: &CorrelationId,
        request: &ExportQuery,
        resource: &ResourcePath,
        now: u64,
        request_id: &str,
        operation_id: &str,
    ) -> Result<QueryExport> {
        let started = Instant::now();
        request
            .validate()
            .map_err(|error| ServiceError::Query(error.to_string()))?;
        let deadline = started
            .checked_add(Duration::from_millis(request.query.budget.max_elapsed_ms))
            .ok_or(ServiceError::DeadlineExceeded)?;
        let result = self.execute_query_scoped(
            session_id,
            token,
            &request.query,
            resource,
            now,
            request_id,
            operation_id,
        )?;
        if result.execution.truncated || result.rows.len() > MAX_PARQUET_EXPORT_ROWS {
            return Err(ServiceError::ConfigurationLimit(
                "Parquet export requires an untruncated result of at most 1024 rows".into(),
            ));
        }
        let source = QueryExportSource {
            canonical_query: result.canonical_query,
            scope: result.scope,
            read_manifest_sha256: result.read_manifest_sha256,
            known_at_cursor: result.known_at_cursor,
            schema_revision: result.schema_revision,
            plan: result.plan,
            execution: result.execution,
        };
        let rows = result
            .rows
            .iter()
            .map(|row| {
                Ok(rrd_query::QueryRow {
                    identity: row.identity.clone(),
                    values: row
                        .values
                        .iter()
                        .map(|(key, value)| Ok((key.clone(), runtime_value(value)?)))
                        .collect::<Result<_>>()?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let metadata = std::collections::BTreeMap::from([
            (
                "rrflow.export.profile".into(),
                PARQUET_EXPORT_PROFILE.into(),
            ),
            (
                "rrflow.export.source".into(),
                serde_json::to_string(&source)
                    .map_err(|error| ServiceError::Query(error.to_string()))?,
            ),
        ]);
        let artifact = rrd_query::export_parquet(
            &rows,
            &metadata,
            rrd_query::ParquetExportLimits {
                max_rows: MAX_PARQUET_EXPORT_ROWS,
                max_columns: MAX_PARQUET_EXPORT_COLUMNS,
                max_input_bytes: MAX_PARQUET_EXPORT_INPUT_BYTES,
                max_output_bytes: request.max_output_bytes as usize,
                row_group_rows: MAX_PARQUET_ROW_GROUP_ROWS,
                max_memory_bytes: request.query.budget.max_memory_bytes as usize,
                deadline,
            },
        )
        .map_err(|error| {
            if Instant::now() >= deadline {
                ServiceError::DeadlineExceeded
            } else {
                match error {
                    rrd_query::Error::StorageAdmission(message) => {
                        ServiceError::StorageAdmission(message)
                    }
                    rrd_query::Error::Budget(message) => ServiceError::ConfigurationLimit(message),
                    other => ServiceError::Query(other.to_string()),
                }
            }
        })?;
        let export = QueryExport {
            profile: PARQUET_EXPORT_PROFILE.into(),
            media_type: "application/vnd.apache.parquet".into(),
            source,
            columns: artifact.columns as u64,
            row_groups: artifact.row_groups as u64,
            input_bytes: artifact.input_bytes as u64,
            reserved_working_bytes: artifact.reserved_working_bytes as u64,
            observed_writer_bytes: artifact.observed_writer_bytes as u64,
            elapsed_micros: started.elapsed().as_micros().min(u128::from(u64::MAX)) as u64,
            artifact_bytes: artifact.bytes.len() as u64,
            artifact_sha256: digest::sha256_hex(&artifact.bytes),
            artifact_base64: STANDARD.encode(&artifact.bytes),
        };
        if Instant::now() >= deadline {
            return Err(ServiceError::DeadlineExceeded);
        }
        Ok(export)
    }
}
