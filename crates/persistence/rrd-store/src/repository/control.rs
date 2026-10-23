use crate::access::runtime_state::{
    checked_key, get, put_sequence, read_sequence, scan_space_bounded_from,
};
use crate::access::synchronize_index_commit_bindings;
use crate::control::{
    validate_control_batch, validate_control_key, verify_control_page, verify_control_tail,
    ControlJournalEntry, ControlTransition,
};
use crate::key_codec::KeyCodec;
use crate::keyspaces::{self, Durability};
use crate::{Error, IndexCommitBindingDefinition, Result, StorageEngine, StorageTransaction};
use rrd_core::{digest, ScopeId};

const CONTROL_TRANSACTION_ATTEMPTS: usize = 16;
const INDEX_CATALOGUE_CONTROL_PREFIX: &str = "server/state/index-catalogue/";

#[cfg(test)]
#[path = "control_page_tests.rs"]
mod page_tests;

struct IndexBindingProjection<'a> {
    expected_schema_revision: u64,
    definitions: &'a [IndexCommitBindingDefinition],
    source_control_sha256: String,
}

/// Materialized control state and its hash-chained journal over one storage
/// transaction authority.
pub struct ControlRepository<'a> {
    storage: &'a dyn StorageEngine,
}

impl<'a> ControlRepository<'a> {
    pub(crate) const fn new(storage: &'a dyn StorageEngine) -> Self {
        Self { storage }
    }

    pub fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        validate_control_key(key)?;
        let transaction = self.storage.begin_transaction()?;
        get(
            &*transaction,
            keyspaces::META,
            &keyspaces::control_record_key(key),
        )
    }

    pub fn commit(&self, transition: &ControlTransition) -> Result<ControlJournalEntry> {
        self.commit_one(transition, None, None)
            .map(|(_, entry)| entry)
    }

    pub fn commit_catalog(
        &self,
        scope: &ScopeId,
        transition: &ControlTransition,
    ) -> Result<(u64, ControlJournalEntry)> {
        let (revision, entry) = self.commit_one(transition, Some(scope), None)?;
        Ok((
            revision.expect("catalogue transition assigns a revision"),
            entry,
        ))
    }

    /// Commits the authoritative rrflowQL catalogue replacement and its
    /// storage-facing derived bindings in one physical transaction.
    pub fn commit_catalog_with_index_bindings(
        &self,
        scope: &ScopeId,
        expected_schema_revision: u64,
        transition: &ControlTransition,
        definitions: &[IndexCommitBindingDefinition],
    ) -> Result<(u64, ControlJournalEntry)> {
        let expected_key = format!("{INDEX_CATALOGUE_CONTROL_PREFIX}{scope}");
        if transition.key != expected_key {
            return Err(Error::IndexConstraint(
                "index commit bindings require the scope's canonical index catalogue key".into(),
            ));
        }
        let replacement = transition.replacement.as_ref().ok_or_else(|| {
            Error::IndexConstraint(
                "index catalogue binding transition requires replacement bytes".into(),
            )
        })?;
        let projection = IndexBindingProjection {
            expected_schema_revision,
            definitions,
            source_control_sha256: digest::sha256_hex(replacement),
        };
        let (revision, entry) = self.commit_one(transition, Some(scope), Some(&projection))?;
        Ok((
            revision.expect("index catalogue transition assigns a revision"),
            entry,
        ))
    }

    fn commit_one(
        &self,
        transition: &ControlTransition,
        catalog_scope: Option<&ScopeId>,
        index_bindings: Option<&IndexBindingProjection<'_>>,
    ) -> Result<(Option<u64>, ControlJournalEntry)> {
        transition.validate()?;
        for _ in 0..CONTROL_TRANSACTION_ATTEMPTS {
            match self.commit_one_attempt(transition, catalog_scope, index_bindings) {
                Err(Error::TransactionConflict { .. }) => continue,
                result => return result,
            }
        }
        Err(Error::Substrate(
            "control journal contention exceeded its retry bound".into(),
        ))
    }

    fn commit_one_attempt(
        &self,
        transition: &ControlTransition,
        catalog_scope: Option<&ScopeId>,
        index_bindings: Option<&IndexBindingProjection<'_>>,
    ) -> Result<(Option<u64>, ControlJournalEntry)> {
        let mut transaction = self.storage.begin_transaction()?;
        let record_key = keyspaces::control_record_key(&transition.key);
        let current = get(&*transaction, keyspaces::META, &record_key)?;
        if current.as_deref() != transition.expected.as_deref() {
            return Err(Error::ControlConflict(transition.key.clone()));
        }
        let current_sequence =
            read_sequence(&*transaction, &keyspaces::control_journal_sequence_key())?;
        let previous_digest = get(
            &*transaction,
            keyspaces::META,
            &keyspaces::control_journal_last_digest_key(),
        )?
        .map(String::from_utf8)
        .transpose()
        .map_err(|error| Error::CorruptWatermark(error.to_string()))?;
        let previous_entry = if current_sequence == 0 {
            None
        } else {
            get(
                &*transaction,
                keyspaces::META,
                &keyspaces::control_journal_key(current_sequence),
            )?
            .map(|bytes| serde_json::from_slice(&bytes))
            .transpose()?
        };
        verify_control_tail(
            current_sequence,
            previous_digest.as_deref(),
            previous_entry.as_ref(),
        )?;
        let sequence = current_sequence
            .checked_add(1)
            .ok_or(Error::SequenceOverflow)?;
        let catalog_revision = catalog_scope
            .map(|scope| {
                read_sequence(&*transaction, &keyspaces::catalog_revision_key(scope))?
                    .checked_add(1)
                    .ok_or(Error::SequenceOverflow)
            })
            .transpose()?;
        let entry = ControlJournalEntry::committed(sequence, transition, previous_digest);
        match &transition.replacement {
            Some(value) => {
                transaction.put(checked_key(keyspaces::META, &record_key)?, value.clone())?
            }
            None => transaction.delete(checked_key(keyspaces::META, &record_key)?)?,
        }
        if let Some(projection) = index_bindings {
            synchronize_index_commit_bindings(
                &mut *transaction,
                catalog_scope.expect("index binding projection has a catalogue scope"),
                projection.expected_schema_revision,
                catalog_revision.expect("index binding projection has a catalogue revision"),
                &transition.key,
                &projection.source_control_sha256,
                projection.definitions,
            )?;
        }
        transaction.put(
            checked_key(keyspaces::META, &keyspaces::control_journal_key(sequence))?,
            serde_json::to_vec(&entry)?,
        )?;
        put_sequence(
            &mut *transaction,
            &keyspaces::control_journal_sequence_key(),
            sequence,
        )?;
        transaction.put(
            checked_key(
                keyspaces::META,
                &keyspaces::control_journal_last_digest_key(),
            )?,
            entry.digest.as_bytes().to_vec(),
        )?;
        if let (Some(scope), Some(revision)) = (catalog_scope, catalog_revision) {
            put_sequence(
                &mut *transaction,
                &keyspaces::catalog_revision_key(scope),
                revision,
            )?;
        }
        transaction.commit(Durability::Authoritative)?;
        Ok((catalog_revision, entry))
    }

    pub fn commit_batch(
        &self,
        transitions: &[ControlTransition],
    ) -> Result<Vec<ControlJournalEntry>> {
        validate_control_batch(transitions)?;
        for _ in 0..CONTROL_TRANSACTION_ATTEMPTS {
            match self.commit_batch_attempt(transitions) {
                Err(Error::TransactionConflict { .. }) => continue,
                result => return result,
            }
        }
        Err(Error::Substrate(
            "control journal contention exceeded its retry bound".into(),
        ))
    }

    fn commit_batch_attempt(
        &self,
        transitions: &[ControlTransition],
    ) -> Result<Vec<ControlJournalEntry>> {
        let mut transaction = self.storage.begin_transaction()?;
        let entries = prepare_control_batch(&mut *transaction, transitions)?;
        transaction.commit(Durability::Authoritative)?;
        Ok(entries)
    }

    pub fn journal_since(&self, after: u64, limit: usize) -> Result<Vec<ControlJournalEntry>> {
        if limit == 0 {
            return Err(Error::Substrate(
                "control journal limit must be non-zero".into(),
            ));
        }
        let transaction = self.storage.begin_transaction()?;
        let head = read_sequence(&*transaction, &keyspaces::control_journal_sequence_key())?;
        if after > head {
            return Ok(Vec::new());
        }
        let anchor_digest = if after == 0 {
            None
        } else {
            let bytes = get(
                &*transaction,
                keyspaces::META,
                &keyspaces::control_journal_key(after),
            )?
            .ok_or_else(|| Error::Substrate("control journal anchor is missing".into()))?;
            let entry: ControlJournalEntry = serde_json::from_slice(&bytes)?;
            if !entry.verify() || entry.sequence != after {
                return Err(Error::Substrate("control journal anchor is corrupt".into()));
            }
            Some(entry.digest)
        };
        if after == head {
            return Ok(Vec::new());
        }
        // Admit only this page within the journal family. Reading the whole
        // META tail before taking `limit` makes backup cost depend on unrelated
        // state and defeats the caller's pagination under native byte limits.
        let count = limit.min(usize::try_from(head - after).unwrap_or(usize::MAX));
        let entries = scan_space_bounded_from(
            &*transaction,
            keyspaces::META,
            &keyspaces::control_journal_prefix(),
            &keyspaces::control_journal_key(after + 1),
            count,
        )?
        .into_iter()
        .map(|(key, bytes)| {
            let entry: ControlJournalEntry = serde_json::from_slice(&bytes)?;
            if !keyspaces::is_control_journal_key(KeyCodec, &key)
                || key != keyspaces::control_journal_key(entry.sequence)
            {
                return Err(Error::Substrate(
                    "control journal key does not match its entry".into(),
                ));
            }
            Ok(entry)
        })
        .collect::<Result<Vec<ControlJournalEntry>>>()?;
        if entries.len() != count {
            return Err(Error::Substrate(
                "control journal ended before its captured watermark".into(),
            ));
        }
        verify_control_page(after, anchor_digest, &entries)?;
        Ok(entries)
    }

    pub fn sequence(&self) -> Result<u64> {
        let transaction = self.storage.begin_transaction()?;
        read_sequence(&*transaction, &keyspaces::control_journal_sequence_key())
    }
}

/// Validates and applies a complete control batch to a caller-owned physical
/// transaction. This function never commits; bootstrap composition remains
/// responsible for the one durability acknowledgement shared with runtime
/// state.
pub(crate) fn prepare_control_batch(
    transaction: &mut dyn StorageTransaction,
    transitions: &[ControlTransition],
) -> Result<Vec<ControlJournalEntry>> {
    validate_control_batch(transitions)?;
    for transition in transitions {
        let current = get(
            transaction,
            keyspaces::META,
            &keyspaces::control_record_key(&transition.key),
        )?;
        if current.as_deref() != transition.expected.as_deref() {
            return Err(Error::ControlConflict(transition.key.clone()));
        }
    }
    let current_sequence = read_sequence(transaction, &keyspaces::control_journal_sequence_key())?;
    let mut previous_digest = get(
        transaction,
        keyspaces::META,
        &keyspaces::control_journal_last_digest_key(),
    )?
    .map(String::from_utf8)
    .transpose()
    .map_err(|error| Error::CorruptWatermark(error.to_string()))?;
    let previous_entry = if current_sequence == 0 {
        None
    } else {
        get(
            transaction,
            keyspaces::META,
            &keyspaces::control_journal_key(current_sequence),
        )?
        .map(|bytes| serde_json::from_slice(&bytes))
        .transpose()?
    };
    verify_control_tail(
        current_sequence,
        previous_digest.as_deref(),
        previous_entry.as_ref(),
    )?;
    let mut entries = Vec::with_capacity(transitions.len());
    for (offset, transition) in transitions.iter().enumerate() {
        let sequence = current_sequence
            .checked_add(offset as u64 + 1)
            .ok_or(Error::SequenceOverflow)?;
        let entry = ControlJournalEntry::committed(sequence, transition, previous_digest.clone());
        previous_digest = Some(entry.digest.clone());
        entries.push(entry);
    }
    for (transition, entry) in transitions.iter().zip(&entries) {
        let record_key = keyspaces::control_record_key(&transition.key);
        match &transition.replacement {
            Some(value) => {
                transaction.put(checked_key(keyspaces::META, &record_key)?, value.clone())?
            }
            None => transaction.delete(checked_key(keyspaces::META, &record_key)?)?,
        }
        transaction.put(
            checked_key(
                keyspaces::META,
                &keyspaces::control_journal_key(entry.sequence),
            )?,
            serde_json::to_vec(entry)?,
        )?;
    }
    let last = entries.last().expect("validated non-empty control batch");
    put_sequence(
        transaction,
        &keyspaces::control_journal_sequence_key(),
        last.sequence,
    )?;
    transaction.put(
        checked_key(
            keyspaces::META,
            &keyspaces::control_journal_last_digest_key(),
        )?,
        last.digest.as_bytes().to_vec(),
    )?;
    Ok(entries)
}
