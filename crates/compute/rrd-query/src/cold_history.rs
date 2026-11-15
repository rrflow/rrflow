//! Bounded encoding of a stamped semantic-history page.
//!
//! This codec does not authenticate page completeness, publish files, manage
//! retention, or authorize access. Those effects belong to the engine.

use crate::{
    export_parquet_encoded, record_batch_to_rows, Error, ParquetEncoding, ParquetExport,
    ParquetExportLimits, QueryFieldTypes, QueryRow, Result,
};
use bytes::Bytes;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use rrd_core::{
    ReadStamp, RuntimeChange, RuntimeChangePage, RuntimeMutation, RuntimeValue, RuntimeValueType,
};
use std::{
    collections::BTreeMap,
    io::{self, Write},
    time::Instant,
};

pub const COLD_HISTORY_FORMAT_VERSION: u16 = 1;

#[derive(Debug)]
pub struct ColdHistoryPage {
    pub requested_after: u64,
    pub through_cursor: u64,
    pub head_cursor: u64,
    pub source_manifest: String,
    pub artifact: ParquetExport,
}

#[derive(Debug, Clone)]
pub struct ColdHistoryReadExpectation {
    pub stamp: ReadStamp,
    pub requested_after: u64,
    pub through_cursor: u64,
    pub rows: u64,
    pub artifact_sha256: String,
}

#[derive(Debug, Clone, Copy)]
pub struct ColdHistoryReadLimits {
    pub max_rows: usize,
    pub max_encoded_bytes: usize,
    pub max_decoded_bytes: usize,
    pub max_memory_bytes: usize,
    pub max_batch_rows: usize,
    pub deadline: Instant,
}

#[derive(Debug)]
pub struct DecodedColdHistoryPage {
    pub changes: Vec<RuntimeChange>,
    pub encoded_bytes: usize,
    pub row_groups: usize,
    pub declared_uncompressed_bytes: u64,
    pub observed_arrow_bytes: usize,
}

/// Failure identity from decoding an admitted cold-history artifact.
#[derive(Debug)]
pub enum ColdHistoryReadError {
    DeadlineExceeded,
    Query(Error),
}

impl std::fmt::Display for ColdHistoryReadError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DeadlineExceeded => formatter.write_str("cold history read deadline exceeded"),
            Self::Query(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for ColdHistoryReadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::DeadlineExceeded => None,
            Self::Query(error) => Some(error),
        }
    }
}

impl From<Error> for ColdHistoryReadError {
    fn from(error: Error) -> Self {
        Self::Query(error)
    }
}

impl From<serde_json::Error> for ColdHistoryReadError {
    fn from(error: serde_json::Error) -> Self {
        Self::Query(error.into())
    }
}

/// Verify a complete bounded artifact against an engine-owned expectation and
/// return typed changes. This checks recorded rows, not scoped completeness.
pub fn decode_cold_history_page(
    bytes: Vec<u8>,
    expected: &ColdHistoryReadExpectation,
    limits: ColdHistoryReadLimits,
) -> std::result::Result<DecodedColdHistoryPage, ColdHistoryReadError> {
    decode_cold_history_page_with_clock(bytes, expected, limits, Instant::now)
}

fn decode_cold_history_page_with_clock(
    bytes: Vec<u8>,
    expected: &ColdHistoryReadExpectation,
    limits: ColdHistoryReadLimits,
    mut now: impl FnMut() -> Instant,
) -> std::result::Result<DecodedColdHistoryPage, ColdHistoryReadError> {
    let mut check = || {
        if now() >= limits.deadline {
            Err(ColdHistoryReadError::DeadlineExceeded)
        } else {
            Ok(())
        }
    };
    check()?;
    if limits.max_rows == 0
        || limits.max_batch_rows == 0
        || limits.max_encoded_bytes == 0
        || limits.max_decoded_bytes == 0
        || limits.max_memory_bytes == 0
        || bytes.len() > limits.max_encoded_bytes
        || expected.rows > limits.max_rows as u64
        || expected.requested_after > expected.through_cursor
        || expected.through_cursor > expected.stamp.commit_cursor
    {
        return Err(Error::Budget("invalid or exceeded cold history read limits".into()).into());
    }
    expected
        .stamp
        .validate()
        .map_err(|error| Error::Integrity(error.to_string()))?;
    if rrd_core::digest::sha256_hex(&bytes) != expected.artifact_sha256 {
        return Err(Error::Integrity("cold history artifact digest mismatch".into()).into());
    }
    let encoded_bytes = bytes.len();
    let builder = ParquetRecordBatchReaderBuilder::try_new(Bytes::from(bytes))
        .map_err(|error| Error::Integrity(format!("invalid cold history Parquet: {error}")))?;
    let metadata = builder.metadata();
    if metadata.file_metadata().num_rows() < 0
        || metadata.file_metadata().num_rows() as u64 != expected.rows
    {
        return Err(Error::Integrity("cold history row count differs from receipt".into()).into());
    }
    let entries = metadata
        .file_metadata()
        .key_value_metadata()
        .ok_or_else(|| Error::Integrity("cold history metadata is absent".into()))?;
    let metadata_value = |key: &str| -> Result<&str> {
        let mut matches = entries.iter().filter(|entry| entry.key == key);
        let value = matches
            .next()
            .and_then(|entry| entry.value.as_deref())
            .ok_or_else(|| Error::Integrity(format!("cold history metadata {key} is absent")))?;
        if matches.next().is_some() {
            return Err(Error::Integrity(format!(
                "duplicate cold history metadata {key}"
            )));
        }
        Ok(value)
    };
    if metadata_value("rrflow.cold.format_version")? != COLD_HISTORY_FORMAT_VERSION.to_string()
        || serde_json::from_str::<ReadStamp>(metadata_value("rrflow.cold.read_stamp")?)?
            != expected.stamp
        || metadata_value("rrflow.cold.requested_after")? != expected.requested_after.to_string()
        || metadata_value("rrflow.cold.through_cursor")? != expected.through_cursor.to_string()
        || metadata_value("rrflow.cold.head_cursor")? != expected.stamp.commit_cursor.to_string()
    {
        return Err(Error::Integrity("cold history metadata differs from receipt".into()).into());
    }
    let row_groups = metadata.row_groups().len();
    let mut uncompressed = 0u64;
    for group in metadata.row_groups() {
        for column in group.columns() {
            let size = u64::try_from(column.uncompressed_size())
                .map_err(|_| Error::Integrity("negative Parquet column size".into()))?;
            uncompressed = uncompressed
                .checked_add(size)
                .ok_or_else(|| Error::Budget("cold history decoded size overflow".into()))?;
        }
    }
    if uncompressed > limits.max_decoded_bytes as u64 {
        return Err(Error::Budget("cold history decoded-byte budget exceeded".into()).into());
    }
    // An admission estimate, not an allocator-level peak-RSS guarantee.
    let reserved = usize::try_from(uncompressed)
        .ok()
        .and_then(|size| size.checked_mul(32))
        .and_then(|size| size.checked_add(encoded_bytes))
        .ok_or_else(|| Error::Budget("cold history read reservation overflow".into()))?;
    if reserved > limits.max_memory_bytes {
        return Err(
            Error::Budget("cold history read reservation exceeds memory budget".into()).into(),
        );
    }
    let mut changes = Vec::new();
    changes
        .try_reserve_exact(expected.rows as usize)
        .map_err(|_| Error::Budget("cold history result allocation failed".into()))?;
    let mut observed_arrow_bytes = 0usize;
    let mut previous = expected.requested_after;
    let reader = builder
        .with_batch_size(limits.max_batch_rows)
        .build()
        .map_err(|error| Error::Integrity(format!("cold history reader failed: {error}")))?;
    for batch in reader {
        check()?;
        let batch = batch
            .map_err(|error| Error::Integrity(format!("cold history batch failed: {error}")))?;
        observed_arrow_bytes = observed_arrow_bytes
            .checked_add(batch.get_array_memory_size())
            .ok_or_else(|| Error::Budget("cold history observed bytes overflow".into()))?;
        if observed_arrow_bytes > limits.max_decoded_bytes {
            return Err(
                Error::Budget("cold history observed Arrow bytes exceed budget".into()).into(),
            );
        }
        for row in record_batch_to_rows(&batch)? {
            check()?;
            let Some(RuntimeValue::String(json)) = row.values.get("change_json") else {
                return Err(
                    Error::Integrity("cold history change payload is absent".into()).into(),
                );
            };
            let change: RuntimeChange = serde_json::from_str(json)?;
            if change.scope != expected.stamp.scope
                || change.cursor <= previous
                || change.cursor > expected.through_cursor
                || !change.verify_digest()
                || change_row(&change, json.clone()) != row
            {
                return Err(Error::Integrity(
                    "cold history row does not match its typed change".into(),
                )
                .into());
            }
            previous = change.cursor;
            changes.push(change);
        }
    }
    if changes.len() as u64 != expected.rows {
        return Err(
            Error::Integrity("cold history decoded row count differs from receipt".into()).into(),
        );
    }
    check()?;
    Ok(DecodedColdHistoryPage {
        changes,
        encoded_bytes,
        row_groups,
        declared_uncompressed_bytes: uncompressed,
        observed_arrow_bytes,
    })
}

/// Encode a single already-read page. Source reads and publication have their
/// own budgets and must be authorized separately; validation here cannot prove
/// that a caller supplied every scope-matching change in the interval.
pub fn encode_cold_history_page(
    stamp: &ReadStamp,
    page: &RuntimeChangePage,
    limits: ParquetExportLimits,
    encoding: ParquetEncoding,
) -> Result<ColdHistoryPage> {
    if page.head_cursor != stamp.commit_cursor
        || page.requested_after > page.through_cursor
        || page.through_cursor > page.head_cursor
        || page.changes.len() > limits.max_rows
        || limits.max_input_bytes == 0
    {
        return Err(Error::Integrity(
            "cold history page coordinates or row limit are invalid".into(),
        ));
    }

    let mut remaining = limits.max_input_bytes;
    let stamp_json = bounded_json(stamp, &mut remaining, limits.deadline)?;
    stamp
        .validate()
        .map_err(|error| Error::Integrity(error.to_string()))?;
    let row_headers = page
        .changes
        .len()
        .checked_mul(std::mem::size_of::<QueryRow>())
        .ok_or_else(|| Error::Budget("cold history row allocation overflows".into()))?;
    if row_headers > limits.max_memory_bytes {
        return Err(Error::Budget(
            "cold history row allocation exceeds memory budget".into(),
        ));
    }
    let mut rows = Vec::new();
    rows.try_reserve_exact(page.changes.len())
        .map_err(|_| Error::Budget("cold history row allocation failed".into()))?;
    let mut previous = page.requested_after;
    for change in &page.changes {
        if Instant::now() >= limits.deadline {
            return Err(Error::DeadlineExceeded(
                "cold history deadline exceeded".into(),
            ));
        }
        if change.scope != stamp.scope
            || change.cursor <= previous
            || change.cursor > page.through_cursor
        {
            return Err(Error::Integrity(
                "cold history change scope, cursor, or digest is invalid".into(),
            ));
        }
        previous = change.cursor;
        let encoded = bounded_json(change, &mut remaining, limits.deadline)?;
        if !change.verify_digest() {
            return Err(Error::Integrity(
                "cold history change digest is invalid".into(),
            ));
        }
        rows.push(change_row(change, encoded));
    }

    let metadata = BTreeMap::from([
        (
            "rrflow.cold.format_version".into(),
            COLD_HISTORY_FORMAT_VERSION.to_string(),
        ),
        ("rrflow.cold.read_stamp".into(), stamp_json),
        (
            "rrflow.cold.requested_after".into(),
            page.requested_after.to_string(),
        ),
        (
            "rrflow.cold.through_cursor".into(),
            page.through_cursor.to_string(),
        ),
        (
            "rrflow.cold.head_cursor".into(),
            page.head_cursor.to_string(),
        ),
    ]);
    let artifact = export_parquet_encoded(&rows, &metadata, &cold_schema(), limits, encoding)?;
    Ok(ColdHistoryPage {
        requested_after: page.requested_after,
        through_cursor: page.through_cursor,
        head_cursor: page.head_cursor,
        source_manifest: stamp.manifest_id.clone(),
        artifact,
    })
}

fn cold_schema() -> QueryFieldTypes {
    let mut fields = QueryFieldTypes::new();
    for name in [
        "scope",
        "commit_id",
        "family",
        "kind",
        "record_id",
        "subject",
        "predicate",
        "change_digest",
        "previous_digest",
        "change_json",
    ] {
        fields.insert(name.into(), vec![RuntimeValueType::String]);
    }
    for name in ["cursor", "commit_ordinal", "committed_at", "valid_at"] {
        fields.insert(name.into(), vec![RuntimeValueType::Unsigned]);
    }
    fields
}

fn change_row(change: &RuntimeChange, encoded: String) -> QueryRow {
    let (family, reference, subject, predicate, valid_at) = match &change.mutation {
        RuntimeMutation::Schema { .. } => ("schema", None, None, None, None),
        RuntimeMutation::Claim { claim } => (
            "claim",
            None,
            Some(claim.subject.as_str()),
            Some(claim.predicate.as_str()),
            Some(claim.valid_from),
        ),
        RuntimeMutation::Record { record } => (
            "record",
            Some(&record.reference),
            None,
            None,
            Some(record.valid_from),
        ),
        RuntimeMutation::Relation { relation } => (
            "relation",
            Some(&relation.reference),
            None,
            None,
            Some(relation.valid_from),
        ),
        RuntimeMutation::Event { event } => ("event", None, None, Some(event.kind.as_str()), None),
        RuntimeMutation::Vector { vector } => (
            "vector",
            Some(&vector.reference),
            None,
            None,
            Some(vector.valid_from),
        ),
        RuntimeMutation::SeriesSample { sample } => (
            "series_sample",
            Some(&sample.reference),
            None,
            None,
            Some(sample.observed_at),
        ),
        RuntimeMutation::Geo { geo } => (
            "geo",
            Some(&geo.reference),
            None,
            None,
            Some(geo.valid_from),
        ),
        RuntimeMutation::Object { object } => ("object", Some(&object.reference), None, None, None),
        RuntimeMutation::Retire { retirement } => (
            "retire",
            Some(&retirement.reference),
            None,
            None,
            Some(retirement.effective_at),
        ),
    };
    let text = |value: Option<&str>| {
        value.map_or(RuntimeValue::Null, |value| {
            RuntimeValue::String(value.into())
        })
    };
    let values = BTreeMap::from([
        (
            "scope".into(),
            RuntimeValue::String(change.scope.to_string()),
        ),
        ("cursor".into(), RuntimeValue::Unsigned(change.cursor)),
        (
            "commit_id".into(),
            RuntimeValue::String(change.commit_id.clone()),
        ),
        (
            "commit_ordinal".into(),
            RuntimeValue::Unsigned(change.commit_ordinal),
        ),
        ("committed_at".into(), RuntimeValue::Unsigned(change.at)),
        ("family".into(), RuntimeValue::String(family.into())),
        (
            "kind".into(),
            text(reference.map(|value| value.kind.as_str())),
        ),
        (
            "record_id".into(),
            text(reference.map(|value| value.id.as_str())),
        ),
        ("subject".into(), text(subject)),
        ("predicate".into(), text(predicate)),
        (
            "valid_at".into(),
            valid_at.map_or(RuntimeValue::Null, RuntimeValue::Unsigned),
        ),
        (
            "change_digest".into(),
            RuntimeValue::String(change.digest.clone()),
        ),
        (
            "previous_digest".into(),
            text(change.previous_digest.as_deref()),
        ),
        ("change_json".into(), RuntimeValue::String(encoded)),
    ]);
    QueryRow {
        identity: format!("change:{}", change.cursor),
        values,
    }
}

struct BoundedJson {
    bytes: Vec<u8>,
    limit: usize,
    deadline: Instant,
}

impl Write for BoundedJson {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if Instant::now() >= self.deadline {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "cold history JSON deadline exceeded",
            ));
        }
        if bytes.len() > self.limit.saturating_sub(self.bytes.len()) {
            return Err(io::Error::other("cold history JSON budget exceeded"));
        }
        self.bytes
            .try_reserve(bytes.len())
            .map_err(io::Error::other)?;
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn bounded_json<T: serde::Serialize>(
    value: &T,
    remaining: &mut usize,
    deadline: Instant,
) -> Result<String> {
    let mut output = BoundedJson {
        bytes: Vec::new(),
        limit: *remaining,
        deadline,
    };
    serde_json::to_writer(&mut output, value).map_err(|error| {
        if error.io_error_kind() == Some(io::ErrorKind::TimedOut) {
            Error::DeadlineExceeded(error.to_string())
        } else {
            Error::Budget(error.to_string())
        }
    })?;
    *remaining -= output.bytes.len();
    String::from_utf8(output.bytes).map_err(|error| Error::Integrity(error.to_string()))
}

#[cfg(test)]
mod deadline_tests {
    use super::*;
    use rrd_core::{RuntimeReadValidation, ScopeId};
    use std::time::Duration;

    #[test]
    fn cold_json_preserves_deadline_and_resource_identity() {
        let mut limit = 1024;
        assert!(matches!(
            bounded_json(&"payload", &mut limit, Instant::now()),
            Err(Error::DeadlineExceeded(_))
        ));
        let mut limit = 1;
        assert!(matches!(
            bounded_json(
                &"payload",
                &mut limit,
                Instant::now() + Duration::from_secs(30)
            ),
            Err(Error::Budget(_))
        ));
    }

    #[test]
    fn empty_artifact_cannot_finish_after_deadline() {
        let stamp =
            ReadStamp::new(ScopeId::new("instance:cold").unwrap(), None, 0, 0, None).unwrap();
        let page = RuntimeChangePage {
            requested_after: 0,
            through_cursor: 0,
            head_cursor: 0,
            validation: RuntimeReadValidation::new("test", 0, 0),
            changes: Vec::new(),
        };
        let end = Instant::now() + Duration::from_secs(30);
        let encoded = encode_cold_history_page(
            &stamp,
            &page,
            ParquetExportLimits {
                max_rows: 1,
                max_columns: 32,
                max_input_bytes: 1024 * 1024,
                max_output_bytes: 1024 * 1024,
                row_group_rows: 1,
                max_memory_bytes: 64 * 1024 * 1024,
                deadline: end,
            },
            ParquetEncoding::default(),
        )
        .unwrap()
        .artifact
        .bytes;
        let expected = ColdHistoryReadExpectation {
            stamp,
            requested_after: 0,
            through_cursor: 0,
            rows: 0,
            artifact_sha256: rrd_core::digest::sha256_hex(&encoded),
        };
        let limits = ColdHistoryReadLimits {
            max_rows: 1,
            max_encoded_bytes: 1024 * 1024,
            max_decoded_bytes: 1024 * 1024,
            max_memory_bytes: 64 * 1024 * 1024,
            max_batch_rows: 1,
            deadline: end,
        };
        let mut calls = 0;
        let result = decode_cold_history_page_with_clock(encoded, &expected, limits, || {
            calls += 1;
            if calls == 1 {
                end - Duration::from_secs(1)
            } else {
                end
            }
        });
        assert!(matches!(
            result,
            Err(ColdHistoryReadError::DeadlineExceeded)
        ));
        assert!(calls >= 2);
    }
}
