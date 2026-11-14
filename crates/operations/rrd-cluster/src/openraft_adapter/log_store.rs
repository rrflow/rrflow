//! The Raft log reader, log storage and snapshot builder.

use super::*;

impl RaftLogReader<RrdRaftTypeConfig> for RrdRaftLogStore {
    async fn try_get_log_entries<RB: RangeBounds<u64> + Clone + Debug + Send>(
        &mut self,
        range: RB,
    ) -> std::result::Result<Vec<RrdRaftEntry>, StorageError<u64>> {
        let database = lock_database(&self.local_database, ErrorSubject::Logs, ErrorVerb::Read)?;
        let rows = database
            .scan(
                LOG_PREFIX,
                prefix_end(LOG_PREFIX).as_deref(),
                database.snapshot(),
            )
            .map_err(|error| {
                storage_error(ErrorSubject::Logs, ErrorVerb::Read, error.to_string())
            })?;
        let mut entries = Vec::new();
        for (key, value) in rows {
            let index = decode_log_key(&key)?;
            if range_contains(&range, index) {
                entries.push(decode_json(
                    &value,
                    ErrorSubject::LogIndex(index),
                    ErrorVerb::Read,
                )?);
            }
        }
        Ok(entries)
    }
}

impl RaftLogStorage<RrdRaftTypeConfig> for RrdRaftLogStore {
    type LogReader = Self;

    async fn get_log_state(
        &mut self,
    ) -> std::result::Result<LogState<RrdRaftTypeConfig>, StorageError<u64>> {
        let last_purged_log_id = read_json(&self.local_database, KEY_PURGED, ErrorSubject::Logs)?;
        let mut reader = self.clone();
        let last_present = reader
            .try_get_log_entries(..)
            .await?
            .into_iter()
            .next_back()
            .map(|entry| entry.log_id);
        Ok(LogState {
            last_log_id: last_present.or(last_purged_log_id),
            last_purged_log_id,
        })
    }

    async fn get_log_reader(&mut self) -> Self::LogReader {
        self.clone()
    }

    async fn save_vote(&mut self, vote: &Vote<u64>) -> std::result::Result<(), StorageError<u64>> {
        let mut database =
            lock_database(&self.local_database, ErrorSubject::Vote, ErrorVerb::Write)?;
        let current: Option<Vote<u64>> = database
            .get(KEY_VOTE, database.snapshot())
            .map_err(|error| storage_error(ErrorSubject::Vote, ErrorVerb::Read, error.to_string()))?
            .map(|bytes| decode_json(&bytes, ErrorSubject::Vote, ErrorVerb::Read))
            .transpose()?;
        if current
            .as_ref()
            .is_some_and(|current| vote.partial_cmp(current).is_none_or(|order| order.is_lt()))
        {
            return Err(storage_error(
                ErrorSubject::Vote,
                ErrorVerb::Write,
                "refusing to persist a regressing or incomparable vote",
            ));
        }
        let operation = put_json_storage(KEY_VOTE, vote, ErrorSubject::Vote)?;
        let batch = WriteBatch::new(vec![operation]).map_err(|error| {
            storage_error(ErrorSubject::Vote, ErrorVerb::Write, error.to_string())
        })?;
        database
            .write_owned(batch, Durability::Authoritative)
            .map_err(|error| {
                storage_error(ErrorSubject::Vote, ErrorVerb::Write, error.to_string())
            })?;
        Ok(())
    }

    async fn read_vote(&mut self) -> std::result::Result<Option<Vote<u64>>, StorageError<u64>> {
        read_json(&self.local_database, KEY_VOTE, ErrorSubject::Vote)
    }

    async fn save_committed(
        &mut self,
        committed: Option<LogId<u64>>,
    ) -> std::result::Result<(), StorageError<u64>> {
        write_json(
            &self.local_database,
            KEY_COMMITTED,
            &committed,
            ErrorSubject::Logs,
        )
    }

    async fn read_committed(
        &mut self,
    ) -> std::result::Result<Option<LogId<u64>>, StorageError<u64>> {
        Ok(read_json::<Option<LogId<u64>>>(
            &self.local_database,
            KEY_COMMITTED,
            ErrorSubject::Logs,
        )?
        .flatten())
    }

    async fn append<I>(
        &mut self,
        entries: I,
        callback: openraft::storage::LogFlushed<RrdRaftTypeConfig>,
    ) -> std::result::Result<(), StorageError<u64>>
    where
        I: IntoIterator<Item = RrdRaftEntry> + Send,
        I::IntoIter: Send,
    {
        let entries: Vec<_> = entries.into_iter().collect();
        let result = persist_entries(&self.local_database, &entries);
        match result {
            Ok(()) => {
                callback.log_io_completed(Ok(()));
                Ok(())
            }
            Err(error) => {
                callback.log_io_completed(Err(std::io::Error::other(error.to_string())));
                Err(error)
            }
        }
    }

    async fn truncate(&mut self, log_id: LogId<u64>) -> std::result::Result<(), StorageError<u64>> {
        delete_log_range(&self.local_database, log_id.index..)
    }

    async fn purge(&mut self, log_id: LogId<u64>) -> std::result::Result<(), StorageError<u64>> {
        let mut operations = log_delete_operations(&self.local_database, ..=log_id.index)?;
        operations.push(put_json_storage(KEY_PURGED, &log_id, ErrorSubject::Logs)?);
        write_operations(&self.local_database, operations, ErrorSubject::Logs)
    }
}

impl RaftSnapshotBuilder<RrdRaftTypeConfig> for RrdRaftStateMachine {
    async fn build_snapshot(
        &mut self,
    ) -> std::result::Result<Snapshot<RrdRaftTypeConfig>, StorageError<u64>> {
        let spool = snapshot_spool_path(&self.snapshot_spool, "build");
        let mut spool_cleanup = SnapshotCleanup(Some(spool.clone()));
        let (state, bundle) = {
            let mut database = lock_database(
                &self.state_database,
                ErrorSubject::Snapshot(None),
                ErrorVerb::Read,
            )?;
            let state = read_state_from_database(&database)?;
            let at = state.last_applied.map_or(0, |log_id| log_id.index);
            let bundle = database.export_snapshot_file(at, &spool).map_err(|error| {
                storage_error(
                    ErrorSubject::Snapshot(None),
                    ErrorVerb::Read,
                    error.to_string(),
                )
            })?;
            (state, bundle)
        };
        let snapshot_id = expected_snapshot_file_id(&state, &bundle);
        let meta = SnapshotMeta {
            last_log_id: state.last_applied,
            last_membership: state.last_membership,
            snapshot_id,
        };
        publish_snapshot_file(&self.local_database, &self.snapshot_objects, &meta, &bundle)?;
        let file = bundle.independent_file().map_err(|error| {
            storage_error(
                ErrorSubject::Snapshot(Some(meta.signature())),
                ErrorVerb::Read,
                error.to_string(),
            )
        })?;
        let snapshot = RrdSnapshotData::from_file(file, bundle.path().to_owned(), Some(spool));
        spool_cleanup.0.take();
        Ok(Snapshot {
            meta,
            snapshot: Box::new(snapshot),
        })
    }
}
