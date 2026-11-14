//! The Raft state machine, snapshot files and database state.

use super::*;

impl RaftStateMachine<RrdRaftTypeConfig> for RrdRaftStateMachine {
    type SnapshotBuilder = Self;

    async fn applied_state(
        &mut self,
    ) -> std::result::Result<
        (Option<LogId<u64>>, StoredMembership<u64, RrdRaftNode>),
        StorageError<u64>,
    > {
        let state = self.read_state()?;
        Ok((state.last_applied, state.last_membership))
    }

    async fn apply<I>(
        &mut self,
        entries: I,
    ) -> std::result::Result<Vec<RrdRaftResponse>, StorageError<u64>>
    where
        I: IntoIterator<Item = RrdRaftEntry> + Send,
        I::IntoIter: Send,
    {
        let mut database = lock_database(
            &self.state_database,
            ErrorSubject::StateMachine,
            ErrorVerb::Write,
        )?;
        let mut state = read_state_from_database(&database)?;
        let mut responses = Vec::new();
        for entry in entries {
            let mut operations = Vec::new();
            let response = match entry.payload {
                EntryPayload::Blank => blank_response(&state, entry.log_id),
                EntryPayload::Membership(membership) => {
                    let prior_binding = membership_binding(&state.last_membership)
                        .map(|voters| membership_binding_digest(&voters));
                    state.last_membership = StoredMembership::new(Some(entry.log_id), membership);
                    let next_binding = membership_binding(&state.last_membership)
                        .map(|voters| membership_binding_digest(&voters));
                    if prior_binding != next_binding {
                        state.voter_membership_generation = state
                            .voter_membership_generation
                            .checked_add(1)
                            .ok_or_else(|| {
                                storage_error(
                                    ErrorSubject::Apply(entry.log_id),
                                    ErrorVerb::Write,
                                    "voter membership generation overflowed",
                                )
                            })?;
                    }
                    blank_response(&state, entry.log_id)
                }
                EntryPayload::Normal(command) => {
                    let application =
                        apply_command(&database, self.shard, &mut state, entry.log_id, command)?;
                    operations.extend(application.operations);
                    application.response
                }
            };
            state.last_applied = Some(entry.log_id);
            prune_request_history(&mut state, entry.log_id.index);
            operations.push(put_json_storage(
                KEY_STATE,
                &state,
                ErrorSubject::StateMachine,
            )?);
            let batch = WriteBatch::new(operations).map_err(|error| {
                storage_error(
                    ErrorSubject::Apply(entry.log_id),
                    ErrorVerb::Write,
                    error.to_string(),
                )
            })?;
            database
                .write_owned(batch, Durability::Authoritative)
                .map_err(|error| {
                    storage_error(
                        ErrorSubject::Apply(entry.log_id),
                        ErrorVerb::Write,
                        error.to_string(),
                    )
                })?;
            responses.push(response);
        }
        Ok(responses)
    }

    async fn get_snapshot_builder(&mut self) -> Self::SnapshotBuilder {
        self.clone()
    }

    async fn begin_receiving_snapshot(
        &mut self,
    ) -> std::result::Result<
        Box<<RrdRaftTypeConfig as RaftTypeConfig>::SnapshotData>,
        StorageError<u64>,
    > {
        let path = snapshot_spool_path(&self.snapshot_spool, "receive");
        RrdSnapshotData::create_ephemeral(path)
            .await
            .map(Box::new)
            .map_err(|error| {
                storage_error(
                    ErrorSubject::Snapshot(None),
                    ErrorVerb::Write,
                    error.to_string(),
                )
            })
    }

    async fn install_snapshot(
        &mut self,
        meta: &SnapshotMeta<u64, RrdRaftNode>,
        snapshot: Box<<RrdRaftTypeConfig as RaftTypeConfig>::SnapshotData>,
    ) -> std::result::Result<(), StorageError<u64>> {
        let subject = ErrorSubject::Snapshot(Some(meta.signature()));
        snapshot
            .sync_all()
            .await
            .map_err(|error| storage_error(subject.clone(), ErrorVerb::Write, error.to_string()))?;
        let bundle = SnapshotBundleFile::open_file(
            snapshot.independent_file().await.map_err(|error| {
                storage_error(subject.clone(), ErrorVerb::Read, error.to_string())
            })?,
            snapshot.path(),
        )
        .map_err(|error| storage_error(subject.clone(), ErrorVerb::Read, error.to_string()))?;
        let state = state_from_snapshot_file(&bundle, self.shard, subject.clone())?;
        if state.last_applied != meta.last_log_id || state.last_membership != meta.last_membership {
            return Err(storage_error(
                subject,
                ErrorVerb::Write,
                "snapshot metadata does not match its state bytes",
            ));
        }
        if meta.snapshot_id != expected_snapshot_file_id(&state, &bundle) {
            return Err(storage_error(
                ErrorSubject::Snapshot(Some(meta.signature())),
                ErrorVerb::Write,
                "snapshot id does not bind the exact RRD LSM bundle",
            ));
        }
        let mut verified_digests = BTreeSet::new();
        for object in rrflow_kv_snapshot_all_object_references(&bundle)
            .map_err(|error| storage_error(subject.clone(), ErrorVerb::Read, error.to_string()))?
        {
            if !verified_digests.insert(object.sha256.clone()) {
                continue;
            }
            let verified = self
                .application_objects
                .verify(&object.sha256)
                .map_err(|error| {
                    storage_error(subject.clone(), ErrorVerb::Read, error.to_string())
                })?;
            if verified.length != object.length {
                return Err(storage_error(
                    subject.clone(),
                    ErrorVerb::Read,
                    "snapshot artifact length differs from its canonical reference",
                ));
            }
        }
        let at = meta.last_log_id.map_or(0, |log_id| log_id.index);
        lock_database(
            &self.state_database,
            ErrorSubject::Snapshot(Some(meta.signature())),
            ErrorVerb::Write,
        )?
        .install_snapshot_file(&bundle, at)
        .map_err(|error| {
            storage_error(
                ErrorSubject::Snapshot(Some(meta.signature())),
                ErrorVerb::Write,
                error.to_string(),
            )
        })?;
        publish_snapshot_file(&self.local_database, &self.snapshot_objects, meta, &bundle)
    }

    async fn get_current_snapshot(
        &mut self,
    ) -> std::result::Result<Option<Snapshot<RrdRaftTypeConfig>>, StorageError<u64>> {
        let stored: Option<StoredSnapshot> = read_json(
            &self.local_database,
            KEY_SNAPSHOT,
            ErrorSubject::Snapshot(None),
        )?;
        let Some(snapshot) = stored else {
            return Ok(None);
        };
        let subject = ErrorSubject::Snapshot(Some(snapshot.meta.signature()));
        let file = self
            .snapshot_objects
            .verified_file(&snapshot.object)
            .map_err(|error| storage_error(subject.clone(), ErrorVerb::Read, error.to_string()))?;
        let path = self
            .snapshot_objects
            .root()
            .join(&snapshot.object.receipt.key);
        let bundle = SnapshotBundleFile::open_file(file, &path)
            .map_err(|error| storage_error(subject.clone(), ErrorVerb::Read, error.to_string()))?;
        let state = state_from_snapshot_file(&bundle, self.shard, subject.clone())?;
        if state.last_applied != snapshot.meta.last_log_id
            || state.last_membership != snapshot.meta.last_membership
            || snapshot.meta.snapshot_id != expected_snapshot_file_id(&state, &bundle)
        {
            return Err(storage_error(
                subject.clone(),
                ErrorVerb::Read,
                "cached snapshot metadata does not match its authenticated bundle",
            ));
        }
        let data = RrdSnapshotData::from_file(
            bundle
                .independent_file()
                .map_err(|error| storage_error(subject, ErrorVerb::Read, error.to_string()))?,
            path,
            None,
        );
        Ok(Some(Snapshot {
            meta: snapshot.meta,
            snapshot: Box::new(data),
        }))
    }
}

impl RrdRaftStateMachine {
    /// Returns a non-owning lifecycle handle for a supervisor that must prove
    /// storage release before allowing a same-process restart.
    pub fn storage_lifecycle(&self) -> RrdRaftStorageLifecycle {
        RrdRaftStorageLifecycle {
            state_database: Arc::downgrade(&self.state_database),
            local_database: Arc::downgrade(&self.local_database),
        }
    }

    pub fn application_objects(&self) -> LocalObjectStore {
        self.application_objects.clone()
    }

    /// Returns one internally consistent canonical read/schema pair for a
    /// coordinator that will submit the resulting runtime transaction through
    /// Raft rather than writing the state database directly.
    pub fn runtime_commit_context(
        &self,
        scope: &ScopeId,
    ) -> ClusterResult<(ReadStamp, Option<RuntimeSchemaRegistry>)> {
        let database = lock_database(
            &self.state_database,
            ErrorSubject::StateMachine,
            ErrorVerb::Read,
        )
        .map_err(|error| ClusterError::Unavailable(error.to_string()))?;
        rrflow_kv_commit_context(&database, scope)
            .map_err(|error| ClusterError::Unavailable(error.to_string()))
    }

    /// Returns the metadata of the exact authenticated snapshot persisted by
    /// this node. OpenRaft's `metrics.snapshot` describes locally built
    /// snapshots and is not evidence that a learner activated a received one.
    pub fn persisted_snapshot_meta(&self) -> ClusterResult<Option<SnapshotMeta<u64, RrdRaftNode>>> {
        let stored: Option<StoredSnapshot> = read_json(
            &self.local_database,
            KEY_SNAPSHOT,
            ErrorSubject::Snapshot(None),
        )
        .map_err(|error| ClusterError::Unavailable(error.to_string()))?;
        let Some(snapshot) = stored else {
            return Ok(None);
        };
        let subject = ErrorSubject::Snapshot(Some(snapshot.meta.signature()));
        let file = self
            .snapshot_objects
            .verified_file(&snapshot.object)
            .map_err(|error| ClusterError::Unavailable(error.to_string()))?;
        let bundle = SnapshotBundleFile::open_file(
            file,
            self.snapshot_objects
                .root()
                .join(&snapshot.object.receipt.key),
        )
        .map_err(|error| ClusterError::Unavailable(error.to_string()))?;
        let state = state_from_snapshot_file(&bundle, self.shard, subject)
            .map_err(|error| ClusterError::Unavailable(error.to_string()))?;
        if state.last_applied != snapshot.meta.last_log_id
            || state.last_membership != snapshot.meta.last_membership
            || snapshot.meta.snapshot_id != expected_snapshot_file_id(&state, &bundle)
        {
            return Err(ClusterError::Unavailable(
                "persisted snapshot metadata does not match its authenticated bundle".into(),
            ));
        }
        Ok(Some(snapshot.meta))
    }

    /// Binds the exact cached physical snapshot to the project-scoped object
    /// closure that must be hydrated before a target may activate it.
    pub fn artifact_manifest_for_cached_snapshot(
        &self,
        meta: &SnapshotMeta<u64, RrdRaftNode>,
        scope: &rrd_core::ScopeId,
        source: NodeId,
        target: NodeId,
    ) -> std::result::Result<Option<ArtifactTransferManifest>, StorageError<u64>> {
        let subject = ErrorSubject::Snapshot(Some(meta.signature()));
        let stored: Option<StoredSnapshot> =
            read_json(&self.local_database, KEY_SNAPSHOT, subject.clone())?;
        let stored = stored.ok_or_else(|| {
            storage_error(
                subject.clone(),
                ErrorVerb::Read,
                "artifact transfer requires the exact cached snapshot",
            )
        })?;
        if &stored.meta != meta {
            return Err(storage_error(
                subject.clone(),
                ErrorVerb::Read,
                "cached snapshot metadata differs from the artifact transfer snapshot",
            ));
        }
        let file = self
            .snapshot_objects
            .verified_file(&stored.object)
            .map_err(|error| storage_error(subject.clone(), ErrorVerb::Read, error.to_string()))?;
        let bundle = SnapshotBundleFile::open_file(
            file,
            self.snapshot_objects
                .root()
                .join(&stored.object.receipt.key),
        )
        .map_err(|error| storage_error(subject.clone(), ErrorVerb::Read, error.to_string()))?;
        let state = state_from_snapshot_file(&bundle, self.shard, subject.clone())?;
        if state.last_applied != meta.last_log_id
            || state.last_membership != meta.last_membership
            || meta.snapshot_id != expected_snapshot_file_id(&state, &bundle)
        {
            return Err(storage_error(
                subject,
                ErrorVerb::Read,
                "cached snapshot state differs from its transfer metadata",
            ));
        }
        let (read, objects) =
            rrflow_kv_snapshot_artifact_view(&bundle, scope).map_err(|error| {
                storage_error(
                    ErrorSubject::Snapshot(Some(meta.signature())),
                    ErrorVerb::Read,
                    error.to_string(),
                )
            })?;
        if objects.is_empty() {
            return Ok(None);
        }
        let placement_epoch = state.placement_epoch.ok_or_else(|| {
            storage_error(
                ErrorSubject::Snapshot(Some(meta.signature())),
                ErrorVerb::Read,
                "artifact-bearing snapshot has no applied placement epoch",
            )
        })?;
        let commit_index = meta.last_log_id.map_or(0, |log_id| log_id.index);
        let term = meta.last_log_id.map_or(0, |log_id| log_id.leader_id.term);
        let artifact_digests = objects.iter().map(|object| object.sha256.clone()).collect();
        ArtifactTransferManifest::new(
            ReplicaTransferPlan {
                contract_version: CLUSTER_CONTRACT_VERSION,
                shard: self.shard,
                placement_epoch,
                source,
                target,
                grounded_snapshot: ShardReadStamp {
                    term,
                    commit_index,
                    placement_epoch,
                    state_digest: state.state_digest,
                },
                wal_from_exclusive: commit_index,
                wal_through_inclusive: commit_index,
                artifact_digests,
            },
            scope.clone(),
            read,
            objects,
        )
        .map(Some)
        .map_err(|error| {
            storage_error(
                ErrorSubject::Snapshot(Some(meta.signature())),
                ErrorVerb::Read,
                error.to_string(),
            )
        })
    }

    /// Hydrates the immutable object closure before activating its canonical
    /// snapshot. A failed snapshot install may leave content-addressed orphans,
    /// but the supported path never exposes canonical references first and
    /// hopes their bytes arrive later.
    pub async fn install_snapshot_with_artifacts<S, T>(
        &mut self,
        meta: &SnapshotMeta<u64, RrdRaftNode>,
        snapshot: Box<RrdSnapshotData>,
        source: &S,
        target: &T,
        manifest: &ArtifactTransferManifest,
        completed_at: u64,
    ) -> std::result::Result<ArtifactTransferReceipt, StorageError<u64>>
    where
        S: rrd_store::ImmutableObjectStore,
        T: rrd_store::ImmutableObjectStore,
    {
        let subject = ErrorSubject::Snapshot(Some(meta.signature()));
        snapshot
            .sync_all()
            .await
            .map_err(|error| storage_error(subject.clone(), ErrorVerb::Write, error.to_string()))?;
        let bundle = SnapshotBundleFile::open_file(
            snapshot.independent_file().await.map_err(|error| {
                storage_error(subject.clone(), ErrorVerb::Read, error.to_string())
            })?,
            snapshot.path(),
        )
        .map_err(|error| storage_error(subject.clone(), ErrorVerb::Read, error.to_string()))?;
        let state = state_from_snapshot_file(&bundle, self.shard, subject.clone())?;
        let snapshot_index = meta.last_log_id.map_or(0, |log_id| log_id.index);
        manifest
            .validate()
            .map_err(|error| storage_error(subject.clone(), ErrorVerb::Write, error.to_string()))?;
        let snapshot_objects = rrflow_kv_snapshot_object_references(&bundle, &manifest.scope)
            .map_err(|error| storage_error(subject.clone(), ErrorVerb::Read, error.to_string()))?;
        if state.last_applied != meta.last_log_id
            || state.last_membership != meta.last_membership
            || meta.snapshot_id != expected_snapshot_file_id(&state, &bundle)
            || manifest.plan.shard != self.shard
            || manifest.plan.grounded_snapshot.commit_index != snapshot_index
            || manifest.plan.grounded_snapshot.state_digest != state.state_digest
            || manifest.plan.wal_from_exclusive != snapshot_index
            || manifest.objects != snapshot_objects
        {
            return Err(storage_error(
                subject,
                ErrorVerb::Write,
                "artifact manifest does not bind the exact canonical snapshot state",
            ));
        }
        let receipt =
            transfer_artifacts(source, target, manifest, completed_at).map_err(|error| {
                storage_error(
                    ErrorSubject::Snapshot(Some(meta.signature())),
                    ErrorVerb::Write,
                    error.to_string(),
                )
            })?;
        for object in &manifest.objects {
            let verified = target.verify(&object.sha256).map_err(|error| {
                storage_error(
                    ErrorSubject::Snapshot(Some(meta.signature())),
                    ErrorVerb::Read,
                    error.to_string(),
                )
            })?;
            if verified.length != object.length {
                return Err(storage_error(
                    ErrorSubject::Snapshot(Some(meta.signature())),
                    ErrorVerb::Read,
                    "installed artifact object length differs from its manifest",
                ));
            }
        }
        <Self as RaftStateMachine<RrdRaftTypeConfig>>::install_snapshot(self, meta, snapshot)
            .await?;
        Ok(receipt)
    }

    fn read_state(&self) -> std::result::Result<StateMachineData, StorageError<u64>> {
        let database = lock_database(
            &self.state_database,
            ErrorSubject::StateMachine,
            ErrorVerb::Read,
        )?;
        read_state_from_database(&database)
    }
}

pub(super) fn state_from_snapshot_file(
    bundle: &SnapshotBundleFile,
    shard: ShardId,
    subject: ErrorSubject<u64>,
) -> std::result::Result<StateMachineData, StorageError<u64>> {
    let mut values = bundle
        .get_many(&[KEY_STATE_CONFIG, KEY_LOCAL_CONFIG, KEY_STATE])
        .map_err(|error| storage_error(subject.clone(), ErrorVerb::Read, error.to_string()))?
        .into_iter();
    let config_bytes = values.next().flatten().ok_or_else(|| {
        storage_error(
            subject.clone(),
            ErrorVerb::Read,
            "snapshot bundle has no canonical-state adapter config",
        )
    })?;
    let config: AdapterConfig = decode_json(&config_bytes, subject.clone(), ErrorVerb::Read)?;
    let expected = AdapterConfig {
        format_version: ADAPTER_FORMAT_VERSION,
        shard,
        domain: AdapterDomain::CanonicalState,
    };
    if config != expected {
        return Err(storage_error(
            subject.clone(),
            ErrorVerb::Read,
            "snapshot bundle adapter format, shard, or storage domain does not match",
        ));
    }
    if values.next().flatten().is_some() {
        return Err(storage_error(
            subject.clone(),
            ErrorVerb::Read,
            "snapshot bundle illegally contains node-local Raft configuration",
        ));
    }
    let state_bytes = values.next().flatten().ok_or_else(|| {
        storage_error(
            subject.clone(),
            ErrorVerb::Read,
            "snapshot bundle has no state-machine record",
        )
    })?;
    decode_json(&state_bytes, subject, ErrorVerb::Read)
}

pub(super) fn expected_snapshot_file_id(
    state: &StateMachineData,
    bundle: &SnapshotBundleFile,
) -> String {
    format!(
        "v4-{}-{}",
        state.last_applied.map_or(0, |log_id| log_id.index),
        bundle.digest
    )
}

pub(super) fn publish_snapshot_file(
    local_database: &SharedDatabase,
    objects: &LocalObjectStore,
    meta: &SnapshotMeta<u64, RrdRaftNode>,
    bundle: &SnapshotBundleFile,
) -> std::result::Result<(), StorageError<u64>> {
    let subject = ErrorSubject::Snapshot(Some(meta.signature()));
    let verified =
        objects
            .put_reader(&mut bundle.independent_file().map_err(|error| {
                storage_error(subject.clone(), ErrorVerb::Read, error.to_string())
            })?)
            .map_err(|error| storage_error(subject.clone(), ErrorVerb::Write, error.to_string()))?;
    if verified.length != bundle.length {
        return Err(storage_error(
            subject,
            ErrorVerb::Write,
            "published snapshot object length differs from its authenticated bundle",
        ));
    }
    let object = ObjectReference::for_verified(
        format!("raft-snapshot-{}", meta.snapshot_id),
        None,
        SNAPSHOT_MEDIA_TYPE,
        verified.sha256,
        verified.length,
        verified.receipt,
    )
    .map_err(|error| storage_error(subject.clone(), ErrorVerb::Write, error.to_string()))?;
    write_json(
        local_database,
        KEY_SNAPSHOT,
        &StoredSnapshot {
            meta: meta.clone(),
            object,
        },
        subject,
    )
}

pub(super) fn read_state_from_database(
    database: &Database,
) -> std::result::Result<StateMachineData, StorageError<u64>> {
    database
        .get(KEY_STATE, database.snapshot())
        .map_err(|error| {
            storage_error(
                ErrorSubject::StateMachine,
                ErrorVerb::Read,
                error.to_string(),
            )
        })?
        .map(|bytes| decode_json(&bytes, ErrorSubject::StateMachine, ErrorVerb::Read))
        .transpose()?
        .ok_or_else(|| {
            storage_error(
                ErrorSubject::StateMachine,
                ErrorVerb::Read,
                "state machine key is absent",
            )
        })
}
