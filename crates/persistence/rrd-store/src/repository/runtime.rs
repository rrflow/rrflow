use crate::access::runtime_state::{
    authenticated_point_page, change_page, checked_key, get, get_json, read_sequence,
    read_stamp_with, scan_space, scan_space_from,
    validate_read_stamp as validate_runtime_read_stamp,
};
use crate::access::{
    index_source_deltas, prepare_semantic_commit, read_versioned as read_versioned_access,
    read_versioned_expanding as read_versioned_expanding_access, schema_at_read,
    vector_source_deltas,
};
use crate::keyspaces::{self, Durability};
use crate::{
    ControlJournalEntry, ControlTransition, Error, FunctionInvocationReceiptRecord,
    IndexSourceDelta, Result, RuntimeReadBudget, RuntimeReadEvidence, RuntimeVersionedRead,
    RuntimeVersionedSource, StorageEngine, VectorSourceAddress, VectorSourceDelta,
};
use rrd_core::{
    AuditEnvelope, DataTransaction, DataTransactionView, Millis, ProjectionWork, ReadStamp,
    RetentionPin, RuntimeChange, RuntimeChangePage, RuntimeCommit, RuntimeCommitOutcome,
    RuntimeDataSnapshot, RuntimeGraphSnapshot, RuntimeMutation, RuntimeReadValidation,
    RuntimeSchemaRegistry, ScopeId, SnapshotHandle, SnapshotId,
};

/// Canonical multi-model runtime repository shared by rrflowMX and rrflowKV.
/// It plans semantic effects against one captured physical snapshot and emits
/// the complete prepared plan through that transaction's single commit.
pub struct RuntimeRepository<'a> {
    storage: &'a dyn StorageEngine,
}

/// One all-model snapshot and the complete direct-read evidence that produced
/// its transaction-visible base. For a prospective transaction preview,
/// `selected_versions` and `read_evidence` describe the authenticated base;
/// the proposed mutations remain explicit in the caller's transaction.
#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeDataSnapshotRead {
    pub read: ReadStamp,
    pub selected_versions: u64,
    pub read_evidence: RuntimeReadEvidence,
    pub snapshot: RuntimeDataSnapshot,
}

impl<'a> RuntimeRepository<'a> {
    pub(crate) const fn new(storage: &'a dyn StorageEngine) -> Self {
        Self { storage }
    }

    pub fn cursor(&self) -> Result<u64> {
        let transaction = self.storage.begin_transaction()?;
        read_sequence(&*transaction, &keyspaces::runtime_cursor_key())
    }

    pub fn schema(&self, scope: &ScopeId) -> Result<Option<RuntimeSchemaRegistry>> {
        let transaction = self.storage.begin_transaction()?;
        get_json(
            &*transaction,
            keyspaces::RUNTIME_SCHEMAS,
            &keyspaces::runtime_schema_key(scope),
        )
    }

    pub fn read_stamp(&self, scope: &ScopeId) -> Result<ReadStamp> {
        let transaction = self.storage.begin_transaction()?;
        read_stamp_with(&*transaction, scope)
    }

    /// Authenticates one previously captured read stamp without opening the
    /// causal log as a range source. Current heads use only current-state point
    /// reads; historical stamps use bounded accumulator and semantic-version
    /// point proofs.
    pub fn validate_read_stamp(&self, read: &ReadStamp) -> Result<RuntimeReadValidation> {
        let transaction = self.storage.begin_transaction()?;
        validate_runtime_read_stamp(&*transaction, read)
    }

    /// Reads authenticated semantic versions through typed, budgeted ranges.
    /// This is the direct-access contract adopted by normal repository and
    /// query paths in the following C-04 packages; explicit log APIs remain
    /// separate.
    pub fn read_versioned(
        &self,
        read: &ReadStamp,
        sources: &[RuntimeVersionedSource],
        budget: RuntimeReadBudget,
    ) -> Result<RuntimeVersionedRead> {
        let transaction = self.storage.begin_transaction()?;
        read_versioned_access(&*transaction, read, sources, budget)
    }

    /// Reads authenticated semantic versions in dependent steps under one
    /// read stamp, key budget, stamp validation, and accumulator. `expand`
    /// receives each step's authenticated changes in global cursor order and
    /// returns the next typed ranges; a range already read is not read again
    /// and an empty selection ends the read. Native adjacency traversal
    /// selects each frontier's ranges from the previous step this way.
    pub fn read_versioned_expanding(
        &self,
        read: &ReadStamp,
        sources: &[RuntimeVersionedSource],
        budget: RuntimeReadBudget,
        expand: &mut dyn FnMut(&[RuntimeChange]) -> Vec<RuntimeVersionedSource>,
    ) -> Result<RuntimeVersionedRead> {
        let transaction = self.storage.begin_transaction()?;
        read_versioned_expanding_access(&*transaction, read, sources, budget, expand)
    }

    pub fn open_snapshot(
        &self,
        scope: &ScopeId,
        owner: &str,
        now: Millis,
        ttl: Millis,
    ) -> Result<SnapshotHandle> {
        let mut transaction = self.storage.begin_transaction()?;
        let handle = SnapshotHandle::new(read_stamp_with(&*transaction, scope)?, owner, now, ttl)?;
        let key = keyspaces::runtime_snapshot_key(handle.id.as_str());
        if let Some(persisted) =
            get_json::<SnapshotHandle>(&*transaction, keyspaces::RUNTIME_SNAPSHOTS, &key)?
        {
            if persisted != handle {
                return Err(Error::SnapshotMismatch(handle.id.to_string()));
            }
            return Ok(handle);
        }
        transaction.put(
            checked_key(keyspaces::RUNTIME_SNAPSHOTS, &key)?,
            serde_json::to_vec(&handle)?,
        )?;
        transaction.commit(Durability::Authoritative)?;
        Ok(handle)
    }

    pub fn snapshot_changes(
        &self,
        handle: &SnapshotHandle,
        after: u64,
        limit: usize,
        now: Millis,
    ) -> Result<RuntimeChangePage> {
        handle.validate()?;
        let transaction = self.storage.begin_transaction()?;
        let persisted: SnapshotHandle = get_json(
            &*transaction,
            keyspaces::RUNTIME_SNAPSHOTS,
            &keyspaces::runtime_snapshot_key(handle.id.as_str()),
        )?
        .ok_or_else(|| Error::SnapshotNotFound(handle.id.to_string()))?;
        if &persisted != handle {
            return Err(Error::SnapshotMismatch(handle.id.to_string()));
        }
        if handle.is_expired(now) {
            return Err(Error::SnapshotExpired {
                id: handle.id.to_string(),
                expired_at: handle.expires_at,
            });
        }
        change_page(
            &*transaction,
            handle.read.commit_cursor,
            after,
            limit,
            Some(&handle.read.scope),
        )
    }

    pub fn release_snapshot(&self, id: &SnapshotId) -> Result<bool> {
        let mut transaction = self.storage.begin_transaction()?;
        let key = keyspaces::runtime_snapshot_key(id.as_str());
        if get(&*transaction, keyspaces::RUNTIME_SNAPSHOTS, &key)?.is_none() {
            return Ok(false);
        }
        transaction.delete(checked_key(keyspaces::RUNTIME_SNAPSHOTS, &key)?)?;
        transaction.commit(Durability::Authoritative)?;
        Ok(true)
    }

    pub fn snapshots(&self, now: Millis) -> Result<Vec<SnapshotHandle>> {
        let transaction = self.storage.begin_transaction()?;
        let mut handles = scan_space(&*transaction, keyspaces::RUNTIME_SNAPSHOTS, &[])?
            .into_iter()
            .map(|(_, bytes)| serde_json::from_slice::<SnapshotHandle>(&bytes).map_err(Error::from))
            .collect::<Result<Vec<_>>>()?;
        for handle in &handles {
            handle.validate()?;
        }
        handles.retain(|handle| !handle.is_expired(now));
        handles.sort_by(|left, right| left.id.cmp(&right.id));
        Ok(handles)
    }

    pub fn retention_pins(&self, now: Millis) -> Result<Vec<RetentionPin>> {
        self.snapshots(now)?
            .iter()
            .map(RetentionPin::from_snapshot)
            .collect::<rrd_core::Result<Vec<_>>>()
            .map_err(Error::from)
    }

    pub fn read_changes(
        &self,
        read: &ReadStamp,
        after: u64,
        limit: usize,
    ) -> Result<RuntimeChangePage> {
        let transaction = self.storage.begin_transaction()?;
        let validation = validate_runtime_read_stamp(&*transaction, read)?;
        if limit == 1 && after < read.commit_cursor && read.accumulator_root.is_some() {
            let mut page = authenticated_point_page(&*transaction, read, after + 1)?;
            if validation.method != "authenticated_current_head" {
                page.validation.change_reads = page
                    .validation
                    .change_reads
                    .saturating_add(validation.change_reads);
                page.validation.proof_nodes = page
                    .validation
                    .proof_nodes
                    .saturating_add(validation.proof_nodes);
                page.validation.method = format!("{}_then_rfc9162_inclusion", validation.method);
            }
            return Ok(page);
        }
        let mut page = change_page(
            &*transaction,
            read.commit_cursor,
            after,
            limit,
            Some(&read.scope),
        )?;
        page.validation = validation;
        Ok(page)
    }

    pub fn data_snapshot(
        &self,
        scope: &ScopeId,
        valid_at: Millis,
        max_keys: usize,
    ) -> Result<RuntimeDataSnapshotRead> {
        let transaction = self.storage.begin_transaction()?;
        let read = read_stamp_with(&*transaction, scope)?;
        let sources = RuntimeVersionedSource::all();
        let direct = read_versioned_access(
            &*transaction,
            &read,
            &sources,
            read_budget(max_keys, "runtime data snapshot")?,
        )?;
        let snapshot = if read.schema_revision.is_some() {
            let schema = schema_at_read(&read, &direct.changes)?;
            RuntimeDataSnapshot::from_changes(
                &direct.changes,
                &schema,
                scope.clone(),
                valid_at,
                read.commit_cursor,
            )?
        } else {
            RuntimeDataSnapshot::from_claim_changes(
                &direct.changes,
                scope.clone(),
                valid_at,
                read.commit_cursor,
            )?
        };
        Ok(RuntimeDataSnapshotRead {
            read,
            selected_versions: selected_version_count(&direct.changes)?,
            read_evidence: direct.evidence,
            snapshot,
        })
    }

    pub fn commit(&self, commit: &RuntimeCommit) -> Result<RuntimeCommitOutcome> {
        self.commit_at_read(commit, None, None, None)
    }

    /// Cold-start composition boundary for runtime and control authority. Both
    /// plans are validated against one physical snapshot and become visible
    /// through one authoritative commit or not at all.
    pub fn commit_with_control_transitions(
        &self,
        commit: &RuntimeCommit,
        transitions: &[ControlTransition],
    ) -> Result<(RuntimeCommitOutcome, Vec<ControlJournalEntry>)> {
        commit.validate()?;
        let mut transaction = self.storage.begin_transaction()?;
        let plan = prepare_semantic_commit(&*transaction, commit, None, None, None)?;
        let entries = super::control::prepare_control_batch(&mut *transaction, transitions)?;
        let outcome = plan.apply(&mut *transaction)?;
        match transaction.commit(Durability::Authoritative) {
            Ok(_) => Ok((outcome, entries)),
            Err(Error::TransactionConflict { .. }) => Err(Error::RuntimeConflict {
                expected: commit.expected_cursor,
                actual: self.cursor()?,
            }),
            Err(error) => Err(error),
        }
    }

    pub fn commit_data_transaction(
        &self,
        transaction: &DataTransaction,
    ) -> Result<RuntimeCommitOutcome> {
        transaction.validate()?;
        self.commit_at_read(&transaction.commit, Some(&transaction.read), None, None)
    }

    /// Commits prepared governed-function receipts through the same physical
    /// transaction as their domain, graph, index, event, outbox, audit, cursor,
    /// and outcome effects. The receipt records are already validated and
    /// runtime-build-bound by `RrdEngine`; this boundary additionally proves
    /// that each one names this exact runtime commit.
    pub fn commit_data_transaction_with_function_receipts(
        &self,
        transaction: &DataTransaction,
        instance: &str,
        receipts: &[FunctionInvocationReceiptRecord],
    ) -> Result<RuntimeCommitOutcome> {
        transaction.validate()?;
        self.commit_at_read(
            &transaction.commit,
            Some(&transaction.read),
            None,
            Some((instance, receipts)),
        )
    }

    pub(crate) fn restore_commit(
        &self,
        commit: &RuntimeCommit,
        audit: &AuditEnvelope,
    ) -> Result<RuntimeCommitOutcome> {
        self.commit_at_read(commit, None, Some(audit), None)
    }

    fn commit_at_read(
        &self,
        commit: &RuntimeCommit,
        read: Option<&ReadStamp>,
        archived_audit: Option<&AuditEnvelope>,
        function_receipts: Option<(&str, &[FunctionInvocationReceiptRecord])>,
    ) -> Result<RuntimeCommitOutcome> {
        commit.validate()?;
        let mut transaction = self.storage.begin_transaction()?;
        let plan = prepare_semantic_commit(
            &*transaction,
            commit,
            read,
            archived_audit,
            function_receipts,
        )?;
        let outcome = plan.apply(&mut *transaction)?;
        match transaction.commit(Durability::Authoritative) {
            Ok(_) => Ok(outcome),
            Err(Error::TransactionConflict { .. }) => Err(Error::RuntimeConflict {
                expected: commit.expected_cursor,
                actual: self.cursor()?,
            }),
            Err(error) => Err(error),
        }
    }

    pub fn changes_since(
        &self,
        after: u64,
        limit: usize,
        scope: Option<&ScopeId>,
    ) -> Result<RuntimeChangePage> {
        let transaction = self.storage.begin_transaction()?;
        let head = read_sequence(&*transaction, &keyspaces::runtime_cursor_key())?;
        change_page(&*transaction, head, after, limit, scope)
    }

    pub fn outbox_since(&self, after: u64, limit: usize) -> Result<Vec<ProjectionWork>> {
        if limit == 0 {
            return Err(Error::Substrate(
                "runtime outbox page limit must be greater than zero".into(),
            ));
        }
        let transaction = self.storage.begin_transaction()?;
        let start =
            keyspaces::runtime_outbox_key(after.checked_add(1).ok_or(Error::SequenceOverflow)?);
        scan_space_from(&*transaction, keyspaces::RUNTIME_OUTBOX, &start)?
            .into_iter()
            .take(limit)
            .map(|(_, bytes)| {
                let work: ProjectionWork = serde_json::from_slice(&bytes)?;
                work.validate()?;
                Ok(work)
            })
            .collect()
    }

    /// Durable projection-work identities committed with their canonical
    /// mutations. These are distinct from outbox delivery records even though
    /// both initially carry the same validated work identity; C-03b adds the
    /// family-specific old/new index deltas behind these identities.
    pub fn projection_deltas_since(&self, after: u64, limit: usize) -> Result<Vec<ProjectionWork>> {
        if limit == 0 {
            return Err(Error::Substrate(
                "runtime projection-delta page limit must be greater than zero".into(),
            ));
        }
        let transaction = self.storage.begin_transaction()?;
        let start = keyspaces::runtime_projection_delta_start(
            after.checked_add(1).ok_or(Error::SequenceOverflow)?,
        );
        scan_space_from(&*transaction, keyspaces::RUNTIME_PROJECTION_DELTAS, &start)?
            .into_iter()
            .take(limit)
            .map(|(_, bytes)| {
                let work: ProjectionWork = serde_json::from_slice(&bytes)?;
                work.validate()?;
                Ok(work)
            })
            .collect()
    }

    /// Exact old/new record fields committed for one schema-bound native
    /// index. Consumers checkpoint `source_cursor`; they never replay the full
    /// runtime log to infer maintenance work.
    pub fn index_source_deltas_since(
        &self,
        scope: &ScopeId,
        index: &rrd_core::ProjectionId,
        after: u64,
        limit: usize,
    ) -> Result<Vec<IndexSourceDelta>> {
        let transaction = self.storage.begin_transaction()?;
        index_source_deltas(&*transaction, scope, index, after, limit)
    }

    /// Exact vector-version pointer changes for one collection/name/field
    /// source. Accelerator builders consume this stream outside the commit
    /// path and publish only through their own verified gate.
    pub fn vector_source_deltas_since(
        &self,
        scope: &ScopeId,
        source: &VectorSourceAddress,
        after: u64,
        limit: usize,
    ) -> Result<Vec<VectorSourceDelta>> {
        let transaction = self.storage.begin_transaction()?;
        vector_source_deltas(&*transaction, scope, source, after, limit)
    }

    pub fn audit(&self, commit_id: &str) -> Result<Option<AuditEnvelope>> {
        let transaction = self.storage.begin_transaction()?;
        let audit: Option<AuditEnvelope> = get_json(
            &*transaction,
            keyspaces::RUNTIME_AUDIT,
            &keyspaces::runtime_audit_key(commit_id),
        )?;
        if let Some(value) = &audit {
            value.validate()?;
        }
        Ok(audit)
    }

    pub fn commit_outcome(&self, commit_id: &str) -> Result<Option<RuntimeCommitOutcome>> {
        let transaction = self.storage.begin_transaction()?;
        get_json(
            &*transaction,
            keyspaces::RUNTIME_COMMITS,
            &keyspaces::runtime_commit_key(commit_id),
        )
    }

    pub fn preview_transaction(
        &self,
        transaction: &DataTransaction,
        valid_at: Millis,
        max_keys: usize,
    ) -> Result<DataTransactionView> {
        transaction.validate()?;
        let storage = self.storage.begin_transaction()?;
        let sources = RuntimeVersionedSource::all();
        let direct = read_versioned_access(
            &*storage,
            &transaction.read,
            &sources,
            read_budget(max_keys, "transaction graph preview")?,
        )?;
        let base = RuntimeGraphSnapshot::from_changes(
            &direct.changes,
            transaction.read.scope.clone(),
            valid_at,
            transaction.read.commit_cursor,
        );
        transaction.preview(&base).map_err(Error::from)
    }

    pub fn preview_data_snapshot(
        &self,
        transaction: &DataTransaction,
        valid_at: Millis,
        max_keys: usize,
    ) -> Result<RuntimeDataSnapshotRead> {
        transaction.validate()?;
        if valid_at == 0 {
            return Err(Error::Substrate(
                "transaction data preview requires non-zero valid time".into(),
            ));
        }
        let storage = self.storage.begin_transaction()?;
        let sources = RuntimeVersionedSource::all();
        let direct = read_versioned_access(
            &*storage,
            &transaction.read,
            &sources,
            read_budget(max_keys, "transaction data preview")?,
        )?;
        let schema = transaction
            .commit
            .mutations
            .iter()
            .filter_map(|mutation| match mutation {
                RuntimeMutation::Schema { registry } => Some(registry.clone()),
                _ => None,
            })
            .next_back()
            .map(Ok)
            .or_else(|| {
                transaction
                    .read
                    .schema_revision
                    .map(|_| schema_at_read(&transaction.read, &direct.changes))
            })
            .transpose()?;
        if let Some(schema) = &schema {
            schema.validate()?;
        } else if !transaction
            .commit
            .mutations
            .iter()
            .all(|mutation| matches!(mutation, RuntimeMutation::Claim { .. }))
        {
            return Err(Error::Substrate(
                "schema-bound transaction preview requires a schema".into(),
            ));
        }
        let prospective_cursor = transaction
            .read
            .commit_cursor
            .checked_add(transaction.commit.mutations.len() as u64)
            .ok_or_else(|| Error::Substrate("transaction preview cursor overflowed".into()))?;
        let commit_id = transaction.commit.digest();
        let selected_versions = selected_version_count(&direct.changes)?;
        let mut changes = direct.changes;
        let mut previous_digest = transaction.read.head_digest.clone();
        for (ordinal, mutation) in transaction.commit.mutations.iter().cloned().enumerate() {
            let cursor = transaction.read.commit_cursor + ordinal as u64 + 1;
            let change = RuntimeChange::committed(
                cursor,
                &transaction.commit,
                &commit_id,
                ordinal as u64,
                mutation,
                previous_digest,
            );
            previous_digest = Some(change.digest.clone());
            changes.push(change);
        }
        let snapshot = if let Some(schema) = &schema {
            RuntimeDataSnapshot::from_changes(
                &changes,
                schema,
                transaction.read.scope.clone(),
                valid_at,
                prospective_cursor,
            )?
        } else {
            RuntimeDataSnapshot::from_claim_changes(
                &changes,
                transaction.read.scope.clone(),
                valid_at,
                prospective_cursor,
            )?
        };
        Ok(RuntimeDataSnapshotRead {
            read: transaction.read.clone(),
            selected_versions,
            read_evidence: direct.evidence,
            snapshot,
        })
    }
}

fn selected_version_count(changes: &[RuntimeChange]) -> Result<u64> {
    u64::try_from(changes.len())
        .map_err(|_| Error::Substrate("selected semantic-version count exceeds u64".into()))
}

fn read_budget(max_keys: usize, operation: &str) -> Result<RuntimeReadBudget> {
    let max_keys = u64::try_from(max_keys)
        .map_err(|_| Error::Substrate(format!("{operation} key budget exceeds u64")))?;
    RuntimeReadBudget::new(max_keys)
}
