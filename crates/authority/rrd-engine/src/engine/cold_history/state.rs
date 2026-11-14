//! Cold-history state, pending publication and receipts.

use super::*;

pub(super) const COLD_STATE_VERSION: u16 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ColdState {
    pub(super) version: u16,
    pub(super) generation: u64,
    pub(super) through_cursor: u64,
    pub(super) last_receipt_sha256: Option<String>,
    pub(super) pending: Option<ColdPending>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) failure: Option<ColdHistoryFailure>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ColdPending {
    pub(super) stamp: ReadStamp,
    pub(super) requested_after: u64,
    pub(super) through_cursor: u64,
    pub(super) scan_limit: u64,
    pub(super) source_sha256: String,
    pub(super) policy_sha256: String,
    pub(super) artifact_sha256: String,
    pub(super) artifact_bytes: u64,
    pub(super) rows: u64,
    pub(super) row_groups: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ColdReceipt {
    pub(super) version: u16,
    pub(super) generation: u64,
    pub(super) previous_receipt_sha256: Option<String>,
    pub(super) pending: ColdPending,
}
