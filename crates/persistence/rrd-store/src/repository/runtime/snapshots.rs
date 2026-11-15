//! Versioned reads, snapshots, retention pins, change pages and data snapshots.

use super::*;

impl<'a> RuntimeRepository<'a> {
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

    /// Read a stamped scope's global cursor interval with a bound on serialized
    /// log entries retained for this page. The underlying point-read API can
    /// materialize one full entry before its length is known, and stamp
    /// validation has separate reads; this is not a total allocation bound.
    pub fn read_changes_bounded_bytes(
        &self,
        read: &ReadStamp,
        after: u64,
        limit: usize,
        max_bytes: u64,
        deadline: std::time::Instant,
    ) -> Result<RuntimeChangePage> {
        if std::time::Instant::now() >= deadline {
            return Err(Error::ColdHistorySourceDeadline);
        }
        if after > read.commit_cursor {
            return Err(Error::Substrate(
                "cold history cursor exceeds its read stamp".into(),
            ));
        }
        let transaction = self.storage.begin_transaction()?;
        let validation = validate_runtime_read_stamp(&*transaction, read)?;
        let mut page =
            change_page_bounded_bytes(&*transaction, read, after, limit, max_bytes, deadline)?;
        page.validation.change_reads = page
            .validation
            .change_reads
            .saturating_add(validation.change_reads);
        page.validation.proof_nodes = page
            .validation
            .proof_nodes
            .saturating_add(validation.proof_nodes);
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
        let snapshot = reduce_data_snapshot(&read, &direct.changes, valid_at)?;
        Ok(RuntimeDataSnapshotRead {
            read,
            selected_versions: selected_version_count(&direct.changes)?,
            read_evidence: direct.evidence,
            snapshot,
        })
    }

    /// A data snapshot at a captured read stamp that reads every source
    /// except the records of `excluded` kinds, plus the listed identities.
    /// A caller that answered some kinds from commit-maintained indexes at
    /// this stamp reads only the identities those indexes selected, and the
    /// rest of the engram at the same stamp, so one answer never mixes two
    /// states. Every version read is authenticated as in [`Self::data_snapshot`].
    pub fn data_snapshot_at(
        &self,
        read: &ReadStamp,
        valid_at: Millis,
        max_keys: usize,
        excluded: &std::collections::BTreeSet<rrd_core::RuntimeType>,
        identities: &[RuntimeVersionedSource],
    ) -> Result<RuntimeDataSnapshotRead> {
        if identities
            .iter()
            .any(|source| !matches!(source, RuntimeVersionedSource::Identity { .. }))
        {
            return Err(Error::Substrate(
                "a selective snapshot names identities only".into(),
            ));
        }
        let transaction = self.storage.begin_transaction()?;
        let schema: Option<RuntimeSchemaRegistry> = get_json(
            &*transaction,
            keyspaces::RUNTIME_SCHEMAS,
            &keyspaces::runtime_schema_key(&read.scope),
        )?;
        if schema.as_ref().map(|schema| schema.revision) != read.schema_revision {
            return Err(Error::ReadStampMismatch(read.manifest_id.clone()));
        }
        let mut sources = Vec::new();
        for source in RuntimeVersionedSource::all() {
            match source {
                RuntimeVersionedSource::Records { kind: None } if !excluded.is_empty() => {
                    for kind in schema.iter().flat_map(|schema| schema.records.keys()) {
                        if !excluded.contains(kind) {
                            sources.push(RuntimeVersionedSource::Records {
                                kind: Some(kind.clone()),
                            });
                        }
                    }
                }
                source => sources.push(source),
            }
        }
        sources.extend(identities.iter().cloned());
        let direct = read_versioned_access(
            &*transaction,
            read,
            &sources,
            read_budget(max_keys, "runtime data snapshot")?,
        )?;
        let snapshot = reduce_data_snapshot(read, &direct.changes, valid_at)?;
        Ok(RuntimeDataSnapshotRead {
            read: read.clone(),
            selected_versions: selected_version_count(&direct.changes)?,
            read_evidence: direct.evidence,
            snapshot,
        })
    }
}
