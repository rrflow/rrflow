//! Log persistence, JSON coding and log keys.

use super::*;

pub(super) fn persist_entries(
    database: &SharedDatabase,
    entries: &[RrdRaftEntry],
) -> std::result::Result<(), StorageError<u64>> {
    if entries.is_empty() {
        return Ok(());
    }
    for pair in entries.windows(2) {
        if pair[1].log_id.index != pair[0].log_id.index + 1 {
            return Err(storage_error(
                ErrorSubject::Logs,
                ErrorVerb::Write,
                "append batch contains a log index hole",
            ));
        }
    }
    let operations = entries
        .iter()
        .map(|entry| {
            put_json_storage(
                &log_key(entry.log_id.index),
                entry,
                ErrorSubject::Log(entry.log_id),
            )
        })
        .collect::<std::result::Result<Vec<_>, _>>()?;
    write_operations(database, operations, ErrorSubject::Logs)
}

pub(super) fn delete_log_range<R: RangeBounds<u64> + Clone + Debug + Send>(
    database: &SharedDatabase,
    range: R,
) -> std::result::Result<(), StorageError<u64>> {
    let operations = log_delete_operations(database, range)?;
    if operations.is_empty() {
        return Ok(());
    }
    write_operations(database, operations, ErrorSubject::Logs)
}

pub(super) fn log_delete_operations<R: RangeBounds<u64> + Clone + Debug + Send>(
    database: &SharedDatabase,
    range: R,
) -> std::result::Result<Vec<Mutation>, StorageError<u64>> {
    let database = lock_database(database, ErrorSubject::Logs, ErrorVerb::Read)?;
    let rows = database
        .scan(
            LOG_PREFIX,
            prefix_end(LOG_PREFIX).as_deref(),
            database.snapshot(),
        )
        .map_err(|error| storage_error(ErrorSubject::Logs, ErrorVerb::Read, error.to_string()))?;
    rows.into_iter()
        .filter_map(|(key, _)| match decode_log_key(&key) {
            Ok(index) if range_contains(&range, index) => Some(Ok(Mutation::Delete { key })),
            Ok(_) => None,
            Err(error) => Some(Err(error)),
        })
        .collect()
}

pub(super) fn write_json<T: Serialize>(
    database: &SharedDatabase,
    key: &[u8],
    value: &T,
    subject: ErrorSubject<u64>,
) -> std::result::Result<(), StorageError<u64>> {
    let operation = put_json_storage(key, value, subject.clone())?;
    write_operations(database, vec![operation], subject)
}

pub(super) fn read_json<T: DeserializeOwned>(
    database: &SharedDatabase,
    key: &[u8],
    subject: ErrorSubject<u64>,
) -> std::result::Result<Option<T>, StorageError<u64>> {
    let database = lock_database(database, subject.clone(), ErrorVerb::Read)?;
    database
        .get(key, database.snapshot())
        .map_err(|error| storage_error(subject.clone(), ErrorVerb::Read, error.to_string()))?
        .map(|bytes| decode_json(&bytes, subject, ErrorVerb::Read))
        .transpose()
}

pub(super) fn write_operations(
    database: &SharedDatabase,
    operations: Vec<Mutation>,
    subject: ErrorSubject<u64>,
) -> std::result::Result<(), StorageError<u64>> {
    let batch = WriteBatch::new(operations)
        .map_err(|error| storage_error(subject.clone(), ErrorVerb::Write, error.to_string()))?;
    lock_database(database, subject.clone(), ErrorVerb::Write)?
        .write_owned(batch, Durability::Authoritative)
        .map_err(|error| storage_error(subject, ErrorVerb::Write, error.to_string()))?;
    Ok(())
}

pub(super) fn lock_database<'a>(
    database: &'a SharedDatabase,
    subject: ErrorSubject<u64>,
    verb: ErrorVerb,
) -> std::result::Result<MutexGuard<'a, Database>, StorageError<u64>> {
    database
        .lock()
        .map_err(|_| storage_error(subject, verb, "raft database mutex is poisoned"))
}

pub(super) fn put_json<T: Serialize>(key: &[u8], value: &T) -> ClusterResult<Mutation> {
    let value = serde_json::to_vec(value)
        .map_err(|error| ClusterError::Invalid(format!("raft value encoding failed: {error}")))?;
    Ok(Mutation::Put {
        key: key.to_vec(),
        value,
    })
}

pub(super) fn put_json_storage<T: Serialize>(
    key: &[u8],
    value: &T,
    subject: ErrorSubject<u64>,
) -> std::result::Result<Mutation, StorageError<u64>> {
    Ok(Mutation::Put {
        key: key.to_vec(),
        value: encode_json(value, subject, ErrorVerb::Write)?,
    })
}

pub(super) fn encode_json<T: Serialize>(
    value: &T,
    subject: ErrorSubject<u64>,
    verb: ErrorVerb,
) -> std::result::Result<Vec<u8>, StorageError<u64>> {
    serde_json::to_vec(value).map_err(|error| storage_error(subject, verb, error.to_string()))
}

pub(super) fn decode_json<T: DeserializeOwned>(
    bytes: &[u8],
    subject: ErrorSubject<u64>,
    verb: ErrorVerb,
) -> std::result::Result<T, StorageError<u64>> {
    serde_json::from_slice(bytes).map_err(|error| storage_error(subject, verb, error.to_string()))
}

pub(super) fn storage_error(
    subject: ErrorSubject<u64>,
    verb: ErrorVerb,
    message: impl Into<String>,
) -> StorageError<u64> {
    StorageError::from_io_error(subject, verb, std::io::Error::other(message.into()))
}

pub(super) fn log_key(index: u64) -> Vec<u8> {
    let mut key = Vec::with_capacity(LOG_PREFIX.len() + 8);
    key.extend_from_slice(LOG_PREFIX);
    key.extend_from_slice(&index.to_be_bytes());
    key
}

pub(super) fn decode_log_key(key: &[u8]) -> std::result::Result<u64, StorageError<u64>> {
    let suffix = key.strip_prefix(LOG_PREFIX).ok_or_else(|| {
        storage_error(
            ErrorSubject::Logs,
            ErrorVerb::Read,
            "raft log key has an invalid prefix",
        )
    })?;
    let bytes: [u8; 8] = suffix.try_into().map_err(|_| {
        storage_error(
            ErrorSubject::Logs,
            ErrorVerb::Read,
            "raft log key has an invalid length",
        )
    })?;
    Ok(u64::from_be_bytes(bytes))
}

pub(super) fn prefix_end(prefix: &[u8]) -> Option<Vec<u8>> {
    let mut end = prefix.to_vec();
    for index in (0..end.len()).rev() {
        if end[index] != u8::MAX {
            end[index] += 1;
            end.truncate(index + 1);
            return Some(end);
        }
    }
    None
}

pub(super) fn range_contains<R: RangeBounds<u64>>(range: &R, value: u64) -> bool {
    let after_start = match range.start_bound() {
        Bound::Included(start) => value >= *start,
        Bound::Excluded(start) => value > *start,
        Bound::Unbounded => true,
    };
    let before_end = match range.end_bound() {
        Bound::Included(end) => value <= *end,
        Bound::Excluded(end) => value < *end,
        Bound::Unbounded => true,
    };
    after_start && before_end
}
