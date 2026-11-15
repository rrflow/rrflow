//! Shared encoded reads over one captured storage snapshot.

use crate::key_codec::{prefix_end, KeyCodec};
use crate::keyspaces::{self, Space};
use crate::{Error, Result, StorageTransaction};
use rrd_core::{
    ReadStamp, RuntimeChange, RuntimeChangePage, RuntimeLogAccumulator, RuntimeMutation,
    RuntimeReadValidation, RuntimeSchemaRegistry, ScopeId,
};
use serde::de::DeserializeOwned;

pub(crate) fn checked_key(space: Space, key: &[u8]) -> Result<Vec<u8>> {
    keyspaces::validate_space(KeyCodec, space, key)?;
    Ok(key.to_vec())
}

pub(crate) trait AccessRead {
    fn read_key(&self, key: &[u8]) -> Result<Option<Vec<u8>>>;

    fn scan_range(&self, start: &[u8], end: &[u8], limit: usize)
        -> Result<Vec<(Vec<u8>, Vec<u8>)>>;

    /// True when writes prepared from this reader are being measured, not
    /// committed; see `RuntimeRepository::measure_commit_with_control`.
    fn measuring(&self) -> bool {
        false
    }
}

impl AccessRead for dyn StorageTransaction + '_ {
    fn read_key(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        self.get(key)
    }

    fn scan_range(
        &self,
        start: &[u8],
        end: &[u8],
        limit: usize,
    ) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        self.scan(start, end, limit)
    }

    fn measuring(&self) -> bool {
        self.is_measuring()
    }
}

pub(crate) fn get(
    reader: &(impl AccessRead + ?Sized),
    space: Space,
    key: &[u8],
) -> Result<Option<Vec<u8>>> {
    reader.read_key(&checked_key(space, key)?)
}

pub(crate) fn get_json<T: DeserializeOwned>(
    reader: &(impl AccessRead + ?Sized),
    space: Space,
    key: &[u8],
) -> Result<Option<T>> {
    get(reader, space, key)?
        .map(|bytes| serde_json::from_slice(&bytes).map_err(Error::from))
        .transpose()
}

pub(crate) fn read_sequence(reader: &(impl AccessRead + ?Sized), key: &[u8]) -> Result<u64> {
    get(reader, keyspaces::META, key)?
        .as_deref()
        .map(decode_sequence)
        .transpose()
        .map(Option::unwrap_or_default)
}

pub(crate) fn decode_sequence(value: &[u8]) -> Result<u64> {
    std::str::from_utf8(value)
        .map_err(|error| Error::CorruptWatermark(error.to_string()))?
        .parse::<u64>()
        .map_err(|error| Error::CorruptWatermark(error.to_string()))
}

pub(crate) fn put_sequence(
    transaction: &mut dyn StorageTransaction,
    key: &[u8],
    sequence: u64,
) -> Result<()> {
    transaction.put(
        checked_key(keyspaces::META, key)?,
        sequence.to_string().into_bytes(),
    )
}

pub(crate) fn scan_space(
    reader: &(impl AccessRead + ?Sized),
    space: Space,
    prefix: &[u8],
) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
    let start = if prefix.is_empty() {
        keyspaces::space_prefix(space)
    } else {
        checked_key(space, prefix)?
    };
    let end = prefix_end(&start)
        .ok_or_else(|| Error::Substrate("canonical key prefix has no upper bound".into()))?;
    reader.scan_range(&start, &end, usize::MAX)
}

pub(crate) fn scan_space_bounded(
    reader: &(impl AccessRead + ?Sized),
    space: Space,
    prefix: &[u8],
    limit: usize,
) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
    if limit == 0 {
        return Err(Error::Substrate(
            "canonical key-prefix scan limit must be greater than zero".into(),
        ));
    }
    let start = if prefix.is_empty() {
        keyspaces::space_prefix(space)
    } else {
        checked_key(space, prefix)?
    };
    let end = prefix_end(&start)
        .ok_or_else(|| Error::Substrate("canonical key prefix has no upper bound".into()))?;
    reader.scan_range(&start, &end, limit)
}

pub(crate) fn scan_space_bounded_from(
    reader: &(impl AccessRead + ?Sized),
    space: Space,
    prefix: &[u8],
    from: &[u8],
    limit: usize,
) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
    if limit == 0 {
        return Err(Error::Substrate(
            "canonical key-prefix scan limit must be greater than zero".into(),
        ));
    }
    let prefix = checked_key(space, prefix)?;
    let from = checked_key(space, from)?;
    let end = prefix_end(&prefix)
        .ok_or_else(|| Error::Substrate("canonical key prefix has no upper bound".into()))?;
    if from < prefix || from >= end {
        return Err(Error::Substrate(
            "canonical range start escaped its key prefix".into(),
        ));
    }
    reader.scan_range(&from, &end, limit)
}

pub(crate) fn scan_space_from(
    reader: &(impl AccessRead + ?Sized),
    space: Space,
    from: &[u8],
) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
    let start = checked_key(space, from)?;
    let prefix = keyspaces::space_prefix(space);
    let end = prefix_end(&prefix)
        .ok_or_else(|| Error::Substrate("canonical key space has no upper bound".into()))?;
    reader.scan_range(&start, &end, usize::MAX)
}

pub(crate) fn read_stamp_with(
    reader: &(impl AccessRead + ?Sized),
    scope: &ScopeId,
) -> Result<ReadStamp> {
    let commit_cursor = read_sequence(reader, &keyspaces::runtime_cursor_key())?;
    let schema_revision = get_json::<RuntimeSchemaRegistry>(
        reader,
        keyspaces::RUNTIME_SCHEMAS,
        &keyspaces::runtime_schema_key(scope),
    )?
    .map(|schema| schema.revision);
    let catalog_revision = read_sequence(reader, &keyspaces::catalog_revision_key(scope))?;
    let head_digest = get(
        reader,
        keyspaces::META,
        &keyspaces::runtime_last_digest_key(),
    )?
    .map(String::from_utf8)
    .transpose()
    .map_err(|error| Error::CorruptWatermark(error.to_string()))?
    .filter(|digest| !digest.is_empty());
    let accumulator = load_accumulator(reader, commit_cursor)?.ok_or_else(|| {
        Error::Substrate(format!(
            "runtime accumulator is absent at non-empty cursor {commit_cursor}"
        ))
    })?;
    ReadStamp::authenticated(
        scope.clone(),
        schema_revision,
        catalog_revision,
        commit_cursor,
        head_digest,
        accumulator.root,
    )
    .map_err(Error::from)
}

pub(crate) fn validate_read_stamp(
    reader: &(impl AccessRead + ?Sized),
    read: &ReadStamp,
) -> Result<RuntimeReadValidation> {
    read.validate()?;
    let current = read_sequence(reader, &keyspaces::runtime_cursor_key())?;
    if read.commit_cursor > current {
        return Err(Error::ReadStampUnavailable(read.manifest_id.clone()));
    }
    if read.commit_cursor == current && read.accumulator_root.is_some() {
        let accumulator = load_accumulator(reader, current)?
            .ok_or_else(|| Error::ReadStampMismatch(read.manifest_id.clone()))?;
        let head_digest = get(
            reader,
            keyspaces::META,
            &keyspaces::runtime_last_digest_key(),
        )?
        .map(String::from_utf8)
        .transpose()
        .map_err(|error| Error::CorruptWatermark(error.to_string()))?
        .filter(|digest| !digest.is_empty());
        let schema_revision = get_json::<RuntimeSchemaRegistry>(
            reader,
            keyspaces::RUNTIME_SCHEMAS,
            &keyspaces::runtime_schema_key(&read.scope),
        )?
        .map(|schema| schema.revision);
        let catalog_revision =
            read_sequence(reader, &keyspaces::catalog_revision_key(&read.scope))?;
        if read.catalog_revision != catalog_revision
            || read.head_digest != head_digest
            || read.schema_revision != schema_revision
            || read.accumulator_root.as_deref() != Some(accumulator.root.as_str())
        {
            return Err(Error::ReadStampMismatch(read.manifest_id.clone()));
        }
        return Ok(RuntimeReadValidation::new(
            "authenticated_current_head",
            0,
            0,
        ));
    }
    let root = read
        .accumulator_root
        .as_deref()
        .ok_or_else(|| Error::ReadStampMismatch(read.manifest_id.clone()))?;
    let accumulator =
        RuntimeLogAccumulator::from_nodes(read.commit_cursor, root, |level, index| {
            read_accumulator_node(reader, level, index)
        })?;
    let mut change_reads = 0_u64;
    let mut proof_nodes = 0_usize;
    let retained_head = if read.commit_cursor == 0 {
        None
    } else {
        let change: RuntimeChange = get_json(
            reader,
            keyspaces::RUNTIME_CHANGES,
            &keyspaces::runtime_change_key(read.commit_cursor),
        )?
        .ok_or_else(|| Error::ReadStampUnavailable(read.manifest_id.clone()))?;
        if change.cursor != read.commit_cursor || !change.verify_digest() {
            return Err(Error::Substrate(format!(
                "runtime change {} failed cursor/digest verification",
                read.commit_cursor
            )));
        }
        let proof = accumulator.inclusion_proof(read.commit_cursor - 1, |level, index| {
            read_accumulator_node(reader, level, index)
        })?;
        proof_nodes = proof_nodes.saturating_add(proof.path.len());
        proof.verify_change(&change, root)?;
        change_reads = change_reads.saturating_add(1);
        Some(change)
    };
    let schema_change = schema_version_at(reader, &read.scope, read.commit_cursor)?;
    let schema_revision = schema_change
        .as_ref()
        .and_then(|change| match &change.mutation {
            RuntimeMutation::Schema { registry } => Some(registry.revision),
            _ => None,
        });
    if let Some(change) = &schema_change {
        change_reads = change_reads.saturating_add(1);
        let head_already_proved = retained_head
            .as_ref()
            .is_some_and(|head| head.cursor == change.cursor && head.digest == change.digest);
        if !head_already_proved {
            let proof = accumulator.inclusion_proof(change.cursor - 1, |level, index| {
                read_accumulator_node(reader, level, index)
            })?;
            proof_nodes = proof_nodes.saturating_add(proof.path.len());
            proof.verify_change(change, root)?;
        }
    }
    if !catalog_revision_in_effect(reader, read)?
        || read.head_digest != retained_head.as_ref().map(|change| change.digest.clone())
        || read.schema_revision != schema_revision
    {
        return Err(Error::ReadStampMismatch(read.manifest_id.clone()));
    }
    Ok(RuntimeReadValidation::new(
        "rfc9162_direct_versions",
        change_reads,
        proof_nodes,
    ))
}

pub(crate) fn schema_version_at(
    reader: &(impl AccessRead + ?Sized),
    scope: &ScopeId,
    at_cursor: u64,
) -> Result<Option<RuntimeChange>> {
    if at_cursor == 0 {
        return Ok(None);
    }
    let prefix = keyspaces::runtime_scope_prefix(keyspaces::RUNTIME_SCHEMA_VERSIONS, scope);
    let start = keyspaces::runtime_schema_version_key(scope, at_cursor);
    let Some((stored_key, bytes)) = scan_space_bounded_from(
        reader,
        keyspaces::RUNTIME_SCHEMA_VERSIONS,
        &prefix,
        &start,
        1,
    )?
    .into_iter()
    .next() else {
        return Ok(None);
    };
    let change: RuntimeChange = serde_json::from_slice(&bytes)?;
    if change.scope != *scope
        || change.cursor > at_cursor
        || !change.verify_digest()
        || !matches!(change.mutation, RuntimeMutation::Schema { .. })
        || stored_key != keyspaces::runtime_schema_version_key(scope, change.cursor)
    {
        return Err(Error::Substrate(format!(
            "schema version key is invalid for scope {scope} at cursor {at_cursor}"
        )));
    }
    Ok(Some(change))
}

pub(crate) fn current_accumulator(
    reader: &(impl AccessRead + ?Sized),
    expected_size: u64,
) -> Result<RuntimeLogAccumulator> {
    load_accumulator(reader, expected_size)?.ok_or_else(|| {
        Error::Substrate(format!(
            "runtime accumulator is absent at non-empty cursor {expected_size}"
        ))
    })
}

fn load_accumulator(
    reader: &(impl AccessRead + ?Sized),
    expected_size: u64,
) -> Result<Option<RuntimeLogAccumulator>> {
    let stored: Option<RuntimeLogAccumulator> = get_json(
        reader,
        keyspaces::META,
        &keyspaces::runtime_accumulator_state_key(),
    )?;
    match stored {
        Some(accumulator) => {
            accumulator.validate()?;
            if accumulator.tree_size != expected_size {
                return Err(Error::Substrate(format!(
                    "runtime accumulator size {} differs from cursor {expected_size}",
                    accumulator.tree_size
                )));
            }
            Ok(Some(accumulator))
        }
        None if expected_size == 0 => Ok(Some(RuntimeLogAccumulator::new())),
        None => Ok(None),
    }
}

pub(crate) fn authenticated_point_page(
    reader: &(impl AccessRead + ?Sized),
    read: &ReadStamp,
    cursor: u64,
) -> Result<RuntimeChangePage> {
    let root = read
        .accumulator_root
        .as_deref()
        .ok_or_else(|| Error::ReadStampMismatch(read.manifest_id.clone()))?;
    let accumulator = match load_accumulator(reader, read.commit_cursor) {
        Ok(Some(accumulator)) if accumulator.root == root => accumulator,
        Ok(_) | Err(_) => {
            RuntimeLogAccumulator::from_nodes(read.commit_cursor, root, |level, index| {
                read_accumulator_node(reader, level, index)
            })?
        }
    };
    let change: RuntimeChange = get_json(
        reader,
        keyspaces::RUNTIME_CHANGES,
        &keyspaces::runtime_change_key(cursor),
    )?
    .ok_or_else(|| Error::ReadStampUnavailable(read.manifest_id.clone()))?;
    let proof = accumulator.inclusion_proof(cursor - 1, |level, index| {
        read_accumulator_node(reader, level, index)
    })?;
    let proof_nodes = proof.path.len();
    proof.verify_change(&change, root)?;
    let selected = (change.scope == read.scope).then_some(change);
    Ok(RuntimeChangePage {
        requested_after: cursor - 1,
        through_cursor: cursor,
        head_cursor: read.commit_cursor,
        validation: RuntimeReadValidation::new("rfc9162_inclusion_proof", 1, proof_nodes),
        changes: selected.into_iter().collect(),
    })
}

pub(crate) fn read_accumulator_node(
    reader: &(impl AccessRead + ?Sized),
    level: u8,
    index: u64,
) -> rrd_core::Result<Option<String>> {
    get(
        reader,
        keyspaces::META,
        &keyspaces::runtime_accumulator_node_key(level, index),
    )
    .map_err(|error| rrd_core::Error::InvalidRuntime {
        reason: format!("cannot read runtime accumulator node: {error}"),
    })?
    .map(String::from_utf8)
    .transpose()
    .map_err(|error| rrd_core::Error::InvalidRuntime {
        reason: format!("runtime accumulator node is not UTF-8: {error}"),
    })
}

pub(crate) fn change_page(
    reader: &(impl AccessRead + ?Sized),
    head: u64,
    after: u64,
    limit: usize,
    scope: Option<&ScopeId>,
) -> Result<RuntimeChangePage> {
    if limit == 0 {
        return Err(Error::Substrate(
            "runtime change page limit must be greater than zero".into(),
        ));
    }
    if after == u64::MAX || after >= head {
        return Ok(RuntimeChangePage {
            requested_after: after,
            through_cursor: after,
            head_cursor: head,
            validation: RuntimeReadValidation::new("bounded_hash_chain_page", 0, 0),
            changes: Vec::new(),
        });
    }
    let mut previous_digest = if after == 0 {
        None
    } else {
        let prior: RuntimeChange = get_json(
            reader,
            keyspaces::RUNTIME_CHANGES,
            &keyspaces::runtime_change_key(after),
        )?
        .ok_or_else(|| Error::Substrate(format!("runtime log is missing cursor {after}")))?;
        if !prior.verify_digest() {
            return Err(Error::Substrate(format!(
                "runtime change {after} failed digest verification"
            )));
        }
        Some(prior.digest)
    };
    let mut through = after;
    let mut selected = Vec::new();
    for expected in after + 1..=head {
        if through.saturating_sub(after) as usize >= limit {
            break;
        }
        let change: RuntimeChange = get_json(
            reader,
            keyspaces::RUNTIME_CHANGES,
            &keyspaces::runtime_change_key(expected),
        )?
        .ok_or_else(|| Error::Substrate(format!("runtime log is missing cursor {expected}")))?;
        if change.cursor != expected
            || change.previous_digest != previous_digest
            || !change.verify_digest()
        {
            return Err(Error::Substrate(format!(
                "runtime change {expected} failed cursor/hash-chain verification"
            )));
        }
        through = expected;
        previous_digest = Some(change.digest.clone());
        if scope.is_none_or(|scope| scope == &change.scope) {
            selected.push(change);
        }
    }
    Ok(RuntimeChangePage {
        requested_after: after,
        through_cursor: through,
        head_cursor: head,
        validation: RuntimeReadValidation::new(
            "bounded_hash_chain_page",
            through.saturating_sub(after),
            0,
        ),
        changes: selected,
    })
}

/// Page a hash-chained log while limiting serialized entries retained for the
/// page. The storage `get` API materializes one value before its size can be
/// checked; this limit is not a pre-read allocation or total RSS bound.
pub(crate) fn change_page_bounded_bytes(
    reader: &(impl AccessRead + ?Sized),
    read: &ReadStamp,
    after: u64,
    limit: usize,
    max_bytes: u64,
    deadline: std::time::Instant,
) -> Result<RuntimeChangePage> {
    let head = read.commit_cursor;
    if std::time::Instant::now() >= deadline {
        return Err(Error::ColdHistorySourceDeadline);
    }
    if limit == 0 || max_bytes == 0 {
        return Err(Error::Substrate(
            "cold history page limits must be positive".into(),
        ));
    }
    if after >= head {
        return Ok(RuntimeChangePage {
            requested_after: after,
            through_cursor: after,
            head_cursor: head,
            validation: RuntimeReadValidation::new("bounded_hash_chain_page", 0, 0),
            changes: Vec::new(),
        });
    }
    let root = read
        .accumulator_root
        .as_deref()
        .ok_or_else(|| Error::ReadStampMismatch(read.manifest_id.clone()))?;
    let accumulator = RuntimeLogAccumulator::from_nodes(head, root, |level, index| {
        read_accumulator_node(reader, level, index)
    })?;
    let mut used = 0u64;
    let mut proof_nodes = 0usize;
    let mut previous_digest = if after == 0 {
        None
    } else {
        let bytes = get(
            reader,
            keyspaces::RUNTIME_CHANGES,
            &keyspaces::runtime_change_key(after),
        )?
        .ok_or_else(|| Error::Substrate(format!("runtime log is missing cursor {after}")))?;
        let predecessor_bytes = u64::try_from(bytes.len())
            .map_err(|_| Error::Substrate("cold history byte count overflow".into()))?;
        if predecessor_bytes > max_bytes {
            return Err(Error::ColdHistorySourceLimit {
                limit: max_bytes,
                observed: predecessor_bytes,
            });
        }
        let prior: RuntimeChange = serde_json::from_slice(&bytes)?;
        if prior.cursor != after || !prior.verify_digest() {
            return Err(Error::Substrate(format!(
                "runtime change {after} failed digest verification"
            )));
        }
        Some(prior.digest)
    };
    let mut through = after;
    let mut selected = Vec::new();
    for expected in after + 1..=head {
        if std::time::Instant::now() >= deadline {
            return Err(Error::ColdHistorySourceDeadline);
        }
        if through.saturating_sub(after) as usize >= limit {
            break;
        }
        let bytes = get(
            reader,
            keyspaces::RUNTIME_CHANGES,
            &keyspaces::runtime_change_key(expected),
        )?
        .ok_or_else(|| Error::Substrate(format!("runtime log is missing cursor {expected}")))?;
        let observed = used
            .checked_add(
                u64::try_from(bytes.len())
                    .map_err(|_| Error::Substrate("cold history byte count overflow".into()))?,
            )
            .ok_or_else(|| Error::Substrate("cold history byte count overflow".into()))?;
        if observed > max_bytes {
            if through == after {
                return Err(Error::ColdHistorySourceLimit {
                    limit: max_bytes,
                    observed,
                });
            }
            break;
        }
        let change: RuntimeChange = serde_json::from_slice(&bytes)?;
        if change.cursor != expected
            || change.previous_digest != previous_digest
            || !change.verify_digest()
        {
            return Err(Error::Substrate(format!(
                "runtime change {expected} failed cursor/hash-chain verification"
            )));
        }
        let proof = accumulator.inclusion_proof(expected - 1, |level, index| {
            read_accumulator_node(reader, level, index)
        })?;
        proof.verify_change(&change, root)?;
        proof_nodes = proof_nodes.saturating_add(proof.path.len());
        used = observed;
        through = expected;
        previous_digest = Some(change.digest.clone());
        if change.scope == read.scope {
            selected.push(change);
        }
    }
    if std::time::Instant::now() >= deadline {
        return Err(Error::ColdHistorySourceDeadline);
    }
    Ok(RuntimeChangePage {
        requested_after: after,
        through_cursor: through,
        head_cursor: head,
        validation: RuntimeReadValidation::new(
            "bounded_authenticated_chain_page",
            through.saturating_sub(after),
            proof_nodes,
        ),
        changes: selected,
    })
}

pub(crate) fn values_for_scope<T: DeserializeOwned>(
    reader: &(impl AccessRead + ?Sized),
    space: Space,
    scope: &ScopeId,
) -> Result<Vec<T>> {
    let prefix = keyspaces::runtime_scope_prefix(space, scope);
    scan_space(reader, space, &prefix)?
        .into_iter()
        .map(|(_, value)| serde_json::from_slice(&value).map_err(Error::from))
        .collect()
}

/// True when the catalogue revision a historical stamp names was in effect at
/// some moment the runtime cursor stood at the stamp's cursor: it took effect
/// at or before that cursor, and the next revision, if any, took effect at or
/// after it. A revision without recorded history is accepted only while it is
/// still current, as before history was recorded.
fn catalog_revision_in_effect(
    reader: &(impl AccessRead + ?Sized),
    read: &ReadStamp,
) -> Result<bool> {
    let current = read_sequence(reader, &keyspaces::catalog_revision_key(&read.scope))?;
    if read.catalog_revision > current {
        return Ok(false);
    }
    let cursor_of = |revision: u64| -> Result<Option<u64>> {
        let key = keyspaces::catalog_revision_cursor_key(&read.scope, revision);
        get(reader, keyspaces::META, &key)?
            .as_deref()
            .map(decode_sequence)
            .transpose()
    };
    let took_effect = if read.catalog_revision == 0 {
        Some(0)
    } else {
        cursor_of(read.catalog_revision)?
    };
    if read.catalog_revision == current {
        return Ok(took_effect.is_none_or(|cursor| cursor <= read.commit_cursor));
    }
    let (Some(took_effect), Some(replaced)) = (took_effect, cursor_of(read.catalog_revision + 1)?)
    else {
        return Ok(false);
    };
    Ok(took_effect <= read.commit_cursor && read.commit_cursor <= replaced)
}

#[cfg(test)]
mod cold_history_authentication_tests {
    use super::*;
    use crate::{Durability, RrflowKvStore, StorageEngine};
    use rrd_core::{
        RuntimeCommit, RuntimeLogicalModel, RuntimeRecord, RuntimeRef, RuntimeTableSchema,
        RuntimeType,
    };
    use std::time::{Duration, Instant};

    #[test]
    fn bounded_page_rejects_a_self_consistent_change_outside_the_captured_root() {
        let directory = tempfile::tempdir().unwrap();
        let store = RrflowKvStore::open(&directory.path().join("engram")).unwrap();
        let scope = ScopeId::new("instance:proof-test").unwrap();
        let mut registry = RuntimeSchemaRegistry::empty(1, "proof test");
        registry.tables.insert(
            RuntimeType::new("note").unwrap(),
            RuntimeTableSchema::schemaless(RuntimeLogicalModel::Relational),
        );
        let commit = RuntimeCommit {
            scope: scope.clone(),
            at: 10,
            actor: "original".into(),
            expected_cursor: 0,
            mutations: vec![
                RuntimeMutation::Schema { registry },
                RuntimeMutation::Record {
                    record: RuntimeRecord {
                        reference: RuntimeRef::new("note", "one").unwrap(),
                        valid_from: 10,
                        valid_to: None,
                        properties: Default::default(),
                    },
                },
            ],
        };
        store.runtime().commit(&commit).unwrap();
        let stamp = store.runtime().read_stamp(&scope).unwrap();
        let key = keyspaces::runtime_change_key(1);
        let mut transaction = store.begin_transaction().unwrap();
        let original: RuntimeChange =
            serde_json::from_slice(&transaction.get(&key).unwrap().unwrap()).unwrap();
        let mut altered_commit = commit;
        altered_commit.actor = "altered".into();
        let altered = RuntimeChange::committed(
            original.cursor,
            &altered_commit,
            &original.commit_id,
            original.commit_ordinal,
            original.mutation,
            original.previous_digest,
        );
        assert!(altered.verify_digest());
        transaction
            .put(key, serde_json::to_vec(&altered).unwrap())
            .unwrap();
        transaction.commit(Durability::Authoritative).unwrap();
        assert!(store
            .runtime()
            .read_changes_bounded_bytes(
                &stamp,
                0,
                1,
                1024 * 1024,
                Instant::now() + Duration::from_secs(10)
            )
            .is_err());
    }
}
