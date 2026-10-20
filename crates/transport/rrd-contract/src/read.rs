//! Provider-neutral evidence for one bounded, read-stamped semantic read.
//!
//! These types expose the logical I/O performed by rrflowMX or rrflowKV
//! without exposing a storage implementation, key encoding, file path, or
//! provider. Normal state reads use this contract; changefeed, archive,
//! rollback, recovery, and diagnostics keep their explicit log-read evidence.

use crate::{invalid, Result};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const READ_EVIDENCE_CONTRACT_VERSION: u16 = 1;

/// The semantic physical path charged during a normal state read.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum ReadAccessPath {
    ReadStamp,
    SchemaVersions,
    ClaimVersions,
    RecordVersions,
    RelationVersions,
    AdjacencyVersions,
    EventVersions,
    VectorVersions,
    SeriesVersions,
    GeoVersions,
    ObjectVersions,
    AccumulatorProof,
}

/// The authentication strategy applied to the requested read stamp.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReadStampValidationMethod {
    AuthenticatedCurrentHead,
    Rfc9162DirectVersions,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReadStampValidationEvidence {
    pub method: ReadStampValidationMethod,
    /// Authenticated log-head/schema changes needed only to prove a retained
    /// historical stamp. This is not the source of normal query rows.
    pub change_reads: u64,
    pub proof_nodes: u16,
}

impl ReadStampValidationEvidence {
    pub fn validate(&self) -> Result<()> {
        if self.method == ReadStampValidationMethod::AuthenticatedCurrentHead
            && (self.change_reads != 0 || self.proof_nodes != 0)
        {
            return invalid(
                "current-head read-stamp validation cannot report historical change or proof reads",
            );
        }
        Ok(())
    }
}

/// Counters attributed to one closed direct-read access path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReadPathEvidence {
    pub path: ReadAccessPath,
    pub point_reads: u64,
    pub range_scans: u64,
    pub keys_examined: u64,
    pub values_decoded: u64,
    pub decoded_bytes: u64,
}

/// Complete logical I/O evidence for normal semantic reads at one stamp.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReadEvidence {
    pub contract_version: u16,
    pub key_budget: u64,
    pub point_reads: u64,
    pub range_scans: u64,
    pub keys_examined: u64,
    pub values_decoded: u64,
    pub decoded_bytes: u64,
    pub stamp_validation: ReadStampValidationEvidence,
    pub paths: Vec<ReadPathEvidence>,
}

impl ReadEvidence {
    pub fn validate(&self) -> Result<()> {
        if self.contract_version != READ_EVIDENCE_CONTRACT_VERSION {
            return invalid(format!(
                "read evidence contract version must be {READ_EVIDENCE_CONTRACT_VERSION}"
            ));
        }
        if self.key_budget == 0
            || self.keys_examined > self.key_budget
            || self.values_decoded > self.keys_examined
            || self.paths.is_empty()
        {
            return invalid("read evidence header is invalid");
        }
        self.stamp_validation.validate()?;

        let mut prior = None;
        let mut point_reads = 0_u64;
        let mut range_scans = 0_u64;
        let mut keys_examined = 0_u64;
        let mut values_decoded = 0_u64;
        let mut decoded_bytes = 0_u64;
        for path in &self.paths {
            if prior.is_some_and(|value| value >= path.path)
                || path.values_decoded > path.keys_examined
            {
                return invalid("read path evidence is duplicated, unsorted, or inconsistent");
            }
            prior = Some(path.path);
            point_reads = checked_add(point_reads, path.point_reads)?;
            range_scans = checked_add(range_scans, path.range_scans)?;
            keys_examined = checked_add(keys_examined, path.keys_examined)?;
            values_decoded = checked_add(values_decoded, path.values_decoded)?;
            decoded_bytes = checked_add(decoded_bytes, path.decoded_bytes)?;
        }
        if point_reads != self.point_reads
            || range_scans != self.range_scans
            || keys_examined != self.keys_examined
            || values_decoded != self.values_decoded
            || decoded_bytes != self.decoded_bytes
        {
            return invalid("read evidence totals do not match its path counters");
        }
        Ok(())
    }
}

fn checked_add(left: u64, right: u64) -> Result<u64> {
    left.checked_add(right)
        .ok_or_else(|| crate::ContractError("read evidence counter overflowed".into()))
}
