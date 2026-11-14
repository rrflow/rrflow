//! Engine-owned, derived cold-history publication. Source history remains
//! authoritative; publication never reclaims or replaces it.

use super::clock::{observe_required, HostSystemClock};
use super::*;
use rrd_contract::{
    ColdHistoryCompression, ColdHistoryFailure, ColdHistoryFailureCode,
    ColdHistoryGenerationSnapshot, ColdHistoryPolicy, ColdHistoryReadResult, ReadColdHistory,
};
use rrd_query::{
    decode_cold_history_page, encode_cold_history_page, ColdHistoryReadExpectation,
    ColdHistoryReadLimits, ParquetCompression, ParquetEncoding, ParquetExportLimits,
};
use rrd_store::filesystem::{AnchoredPath, OpenOptions};
use std::io::{Read, Write};
use std::sync::TryLockError;

mod files;
mod maintain;
mod state;

use files::*;
use state::*;

#[cfg(test)]
mod tests;
