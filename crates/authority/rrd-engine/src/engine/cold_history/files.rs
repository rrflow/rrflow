//! Encoding, digests, transitions and verified file publication.

use super::*;

/// Background publication is no client invocation; these name its gate reservation.
pub(super) const COLD_REQUEST_ID: &str = "cold-history-maintenance";
pub(super) const COLD_OPERATION_ID: &str = "cold-history-publication";

pub(super) fn native(value: u64) -> Result<usize> {
    usize::try_from(value).map_err(|_| {
        ServiceError::ConfigurationLimit("cold history limit exceeds native address space".into())
    })
}

pub(super) fn cold_source_error(error: rrd_store::Error) -> ServiceError {
    match error {
        rrd_store::Error::ColdHistorySourceDeadline => ServiceError::DeadlineExceeded,
        rrd_store::Error::ColdHistorySourceLimit { .. } => {
            ServiceError::ConfigurationLimit(error.to_string())
        }
        other => other.into(),
    }
}

pub(super) fn cold_query_error(error: rrd_query::Error, deadline: Instant) -> ServiceError {
    if matches!(error, rrd_query::Error::Budget(_)) && Instant::now() >= deadline {
        ServiceError::DeadlineExceeded
    } else {
        error.into()
    }
}

pub(super) fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

pub(super) fn limits(policy: &ColdHistoryPolicy, deadline: Instant) -> Result<ParquetExportLimits> {
    Ok(ParquetExportLimits {
        max_rows: native(policy.max_changes)?,
        max_columns: 14,
        max_input_bytes: native(policy.max_input_bytes)?,
        max_output_bytes: native(policy.max_output_bytes)?,
        row_group_rows: native(policy.row_group_rows)?,
        max_memory_bytes: native(policy.max_memory_bytes)?,
        deadline,
    })
}

pub(super) fn encoding(policy: &ColdHistoryPolicy) -> ParquetEncoding {
    ParquetEncoding {
        compression: match policy.compression {
            ColdHistoryCompression::Uncompressed => ParquetCompression::Uncompressed,
            ColdHistoryCompression::Zstd { level } => ParquetCompression::Zstd {
                level: i32::from(level),
            },
        },
        dictionary: policy.dictionary,
        statistics: policy.statistics,
    }
}

pub(super) fn encode(
    stamp: &ReadStamp,
    page: &rrd_core::RuntimeChangePage,
    policy: &ColdHistoryPolicy,
    deadline: Instant,
) -> Result<rrd_query::ColdHistoryPage> {
    encode_cold_history_page(stamp, page, limits(policy, deadline)?, encoding(policy))
        .map_err(|error| cold_query_error(error, deadline))
}

pub(super) fn source_digest(stamp: &ReadStamp, page: &rrd_core::RuntimeChangePage) -> String {
    source_digest_parts(
        stamp,
        page.requested_after,
        page.through_cursor,
        page.head_cursor,
        &page.changes,
    )
}

pub(super) fn source_digest_parts(
    stamp: &ReadStamp,
    requested_after: u64,
    through_cursor: u64,
    head_cursor: u64,
    changes: &[RuntimeChange],
) -> String {
    let mut hash = digest::Sha256::new();
    hash.update(b"rrflow-cold-history-page-v1\0");
    hash.update(stamp.manifest_id.as_bytes());
    for cursor in [requested_after, through_cursor, head_cursor] {
        hash.update(&cursor.to_be_bytes());
    }
    for change in changes {
        hash.update(&change.cursor.to_be_bytes());
        hash.update(change.digest.as_bytes());
    }
    hash.finalize_hex()
}

pub(super) struct CountOutput {
    pub(super) limit: u64,
    pub(super) used: u64,
}
impl Write for CountOutput {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let next = self
            .used
            .checked_add(bytes.len() as u64)
            .ok_or_else(|| std::io::Error::other("output length overflow"))?;
        if next > self.limit {
            return Err(std::io::Error::other("output limit exceeded"));
        }
        self.used = next;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

pub(super) fn transition<T: Serialize>(
    key: &str,
    expected: Option<Vec<u8>>,
    replacement: &T,
    now: u64,
    action: &str,
) -> Result<ControlTransition> {
    Ok(transition_bytes(
        key,
        expected,
        serde_json::to_vec(replacement).map_err(contract_json)?,
        now,
        action,
    ))
}

pub(super) fn transition_bytes(
    key: &str,
    expected: Option<Vec<u8>>,
    replacement: Vec<u8>,
    now: u64,
    action: &str,
) -> ControlTransition {
    ControlTransition {
        key: key.into(),
        expected,
        replacement: Some(replacement),
        at: now,
        actor: "system:maintenance".into(),
        action: action.into(),
        request_id: "cold-history-maintenance".into(),
        operation_id: "cold-history-maintenance".into(),
    }
}

pub(super) fn check_deadline(deadline: Instant) -> Result<()> {
    if Instant::now() >= deadline {
        Err(ServiceError::DeadlineExceeded)
    } else {
        Ok(())
    }
}

pub(super) fn io_error(error: std::io::Error) -> ServiceError {
    ServiceError::Storage(format!("cold history file operation failed: {error}"))
}

pub(super) fn verify_head(
    engine: &RrdEngine,
    root: &AnchoredPath,
    state: &ColdState,
    deadline: Instant,
) -> Result<()> {
    let key = format!(
        "server/state/{}/cold-history/generation/{:020}",
        engine.instance, state.generation
    );
    let bytes = engine
        .storage
        .control()
        .get(&key)?
        .ok_or_else(|| ServiceError::Storage("cold history head receipt is missing".into()))?;
    if Some(digest::sha256_hex(&bytes).as_str()) != state.last_receipt_sha256.as_deref() {
        return Err(ServiceError::Storage(
            "cold history head receipt digest mismatch".into(),
        ));
    }
    let receipt: ColdReceipt = serde_json::from_slice(&bytes).map_err(contract_json)?;
    if receipt.version != COLD_STATE_VERSION
        || receipt.generation != state.generation
        || receipt.pending.through_cursor != state.through_cursor
        || !valid_sha256(&receipt.pending.artifact_sha256)
        || receipt.pending.artifact_bytes == 0
        || receipt.pending.artifact_bytes > 1_073_741_824
        || (receipt.generation == 1) != receipt.previous_receipt_sha256.is_none()
        || receipt
            .previous_receipt_sha256
            .as_ref()
            .is_some_and(|hash| !valid_sha256(hash))
    {
        return Err(ServiceError::Storage(
            "cold history head receipt is invalid".into(),
        ));
    }
    let file = root
        .join("cold-history")
        .join(format!("{}.parquet", receipt.pending.artifact_sha256));
    verify_file(
        &file,
        &receipt.pending.artifact_sha256,
        native(receipt.pending.artifact_bytes)?,
        deadline,
    )
}

pub(super) fn verify_file(
    path: &AnchoredPath,
    expected: &str,
    expected_bytes: usize,
    deadline: Instant,
) -> Result<()> {
    let mut file = path.open_read().map_err(io_error)?;
    if file.metadata().map_err(io_error)?.len() != expected_bytes as u64 {
        return Err(ServiceError::Storage(
            "cold history artifact length mismatch".into(),
        ));
    }
    let mut hash = digest::Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        check_deadline(deadline)?;
        let count = file.read(&mut buffer).map_err(io_error)?;
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
    }
    if hash.finalize_hex() != expected {
        return Err(ServiceError::Storage(
            "cold history artifact digest mismatch".into(),
        ));
    }
    Ok(())
}

pub(super) fn publish_file(
    directory: &AnchoredPath,
    final_path: &AnchoredPath,
    digest: &str,
    bytes: &[u8],
    deadline: Instant,
) -> Result<()> {
    if final_path.try_exists().map_err(io_error)? {
        return verify_file(final_path, digest, bytes.len(), deadline);
    }
    let staging = directory.join(format!(".{digest}.tmp"));
    if staging.try_exists().map_err(io_error)? {
        staging.remove_file().map_err(io_error)?;
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&staging)
        .map_err(io_error)?;
    file.write_all(bytes).map_err(io_error)?;
    check_deadline(deadline)?;
    file.sync_all().map_err(io_error)?;
    drop(file);
    check_deadline(deadline)?;
    staging.rename_new_to(final_path).map_err(io_error)?;
    verify_file(final_path, digest, bytes.len(), deadline)
}
