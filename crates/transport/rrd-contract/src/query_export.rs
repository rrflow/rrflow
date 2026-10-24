//! The first bounded query-result interchange profile. Import and streaming
//! are separate capabilities, not implied by an export artifact.
use base64::{engine::general_purpose::STANDARD, Engine as _};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    invalid, sha256_bytes, validate_sha256, ExecuteQuery, QueryExecutionSnapshot,
    QueryPlanSnapshot, Result,
};

pub const PARQUET_EXPORT_PROFILE: &str = "rrflow-parquet-1";
pub const MAX_PARQUET_EXPORT_ROWS: usize = 1024;
pub const MAX_PARQUET_EXPORT_COLUMNS: usize = 64;
pub const MAX_PARQUET_EXPORT_INPUT_BYTES: usize = 262144;
pub const MAX_PARQUET_EXPORT_BYTES: usize = 524288;
pub const MAX_PARQUET_ROW_GROUP_ROWS: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExportQuery {
    pub query: ExecuteQuery,
    pub max_output_bytes: u64,
}

impl ExportQuery {
    pub fn validate(&self) -> Result<()> {
        self.query.validate()?;
        if self.max_output_bytes == 0
            || self.max_output_bytes > MAX_PARQUET_EXPORT_BYTES as u64
            || self.max_output_bytes > self.query.budget.max_output_bytes
        {
            return invalid("Parquet output limit must fit the export profile and query budget");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct QueryExportSource {
    pub canonical_query: String,
    pub scope: String,
    pub read_manifest_sha256: String,
    pub known_at_cursor: u64,
    pub schema_revision: u64,
    pub plan: QueryPlanSnapshot,
    pub execution: QueryExecutionSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct QueryExport {
    pub profile: String,
    pub media_type: String,
    pub source: QueryExportSource,
    pub columns: u64,
    pub row_groups: u64,
    pub input_bytes: u64,
    pub reserved_working_bytes: u64,
    pub observed_writer_bytes: u64,
    pub elapsed_micros: u64,
    pub artifact_bytes: u64,
    pub artifact_sha256: String,
    pub artifact_base64: String,
}

impl QueryExport {
    /// Validate the envelope and artifact integrity. Full format decoding
    /// belongs to a Parquet reader; framing alone is not format conformance.
    pub fn decode_artifact(&self) -> Result<Vec<u8>> {
        if self.profile != PARQUET_EXPORT_PROFILE
            || self.media_type != "application/vnd.apache.parquet"
            || self.source.execution.truncated
            || self.source.execution.returned_rows > MAX_PARQUET_EXPORT_ROWS as u64
            || self.columns == 0
            || self.columns > MAX_PARQUET_EXPORT_COLUMNS as u64 + 1
            || self.row_groups
                != self
                    .source
                    .execution
                    .returned_rows
                    .div_ceil(MAX_PARQUET_ROW_GROUP_ROWS as u64)
            || self.input_bytes > MAX_PARQUET_EXPORT_INPUT_BYTES as u64
            || self.artifact_bytes > MAX_PARQUET_EXPORT_BYTES as u64
            || self.artifact_base64.len() > MAX_PARQUET_EXPORT_BYTES.div_ceil(3) * 4
        {
            return invalid("invalid Parquet export profile or bounds");
        }
        validate_sha256(&self.source.read_manifest_sha256, "export read manifest")?;
        validate_sha256(&self.source.plan.plan_sha256, "export query plan")?;
        validate_sha256(
            &self.source.plan.authorization_sha256,
            "export authorization",
        )?;
        validate_sha256(&self.artifact_sha256, "export artifact")?;
        let bytes = STANDARD
            .decode(&self.artifact_base64)
            .map_err(|_| crate::ContractError("invalid Parquet base64".into()))?;
        if bytes.len() < 12
            || bytes.len() as u64 != self.artifact_bytes
            || !bytes.starts_with(b"PAR1")
            || !bytes.ends_with(b"PAR1")
            || sha256_bytes(&bytes) != self.artifact_sha256
            || STANDARD.encode(&bytes) != self.artifact_base64
        {
            return invalid("Parquet artifact framing, length, encoding or digest mismatch");
        }
        Ok(bytes)
    }
}
