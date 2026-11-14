//! The artifact transfer receiver.

use super::*;

/// Durable receiver for the authenticated chunk protocol. Session identity is
/// the manifest digest, so reconnects and process restarts resume at the exact
/// fsynced offset instead of creating a second mutable upload identity.
#[derive(Debug, Clone)]
pub struct ArtifactTransferReceiver {
    store: LocalObjectStore,
    sessions: PathBuf,
    policy: ArtifactTransferSessionPolicy,
    lifecycle: Arc<RwLock<()>>,
    session_locks: Arc<Mutex<BTreeMap<String, Weak<Mutex<()>>>>>,
    telemetry: Arc<Mutex<ArtifactTransferTelemetryState>>,
}

impl ArtifactTransferReceiver {
    pub fn open(store: LocalObjectStore) -> Result<Self> {
        Self::open_with_policy(store, ArtifactTransferSessionPolicy::default())
    }

    pub fn open_with_policy(
        store: LocalObjectStore,
        policy: ArtifactTransferSessionPolicy,
    ) -> Result<Self> {
        Self::open_with_policy_at(store, policy, receiver_now_millis())
    }

    /// Deterministic-clock constructor used by failure simulation and contract
    /// tests. Production callers should use [`Self::open_with_policy`].
    pub fn open_with_policy_at(
        store: LocalObjectStore,
        policy: ArtifactTransferSessionPolicy,
        started_at: u64,
    ) -> Result<Self> {
        policy.validate()?;
        let sessions = store.root().join(TRANSFER_SESSION_DIRECTORY);
        fs::create_dir_all(&sessions).map_err(cluster_io_error)?;
        sync_directory(&sessions)?;
        let receiver = Self {
            store,
            sessions,
            policy,
            lifecycle: Arc::new(RwLock::new(())),
            session_locks: Arc::new(Mutex::new(BTreeMap::new())),
            telemetry: Arc::new(Mutex::new(ArtifactTransferTelemetryState {
                started_at,
                begin_requests: 0,
                chunk_requests: 0,
                complete_requests: 0,
                begin_responses: 0,
                accepted_chunks: 0,
                completed_responses: 0,
                completed_receipt_replays: 0,
                denied: 0,
                failed: 0,
                quota_denials: 0,
                gc_runs: 0,
                gc_removed_incomplete: 0,
                gc_removed_completed: 0,
                gc_reclaimed_partial_bytes: 0,
                overflowed: false,
            })),
        };
        receiver.collect_garbage(started_at)?;
        Ok(receiver)
    }

    pub fn store(&self) -> &LocalObjectStore {
        &self.store
    }

    pub fn handle(
        &self,
        authenticated_source: &NodeId,
        local_target: &NodeId,
        request: ArtifactTransferRpc,
    ) -> Result<ArtifactTransferRpcResult> {
        self.handle_at(
            authenticated_source,
            local_target,
            request,
            receiver_now_millis(),
        )
    }

    pub fn handle_at(
        &self,
        authenticated_source: &NodeId,
        local_target: &NodeId,
        request: ArtifactTransferRpc,
        now: u64,
    ) -> Result<ArtifactTransferRpcResult> {
        let kind = match &request.operation {
            ArtifactTransferOperation::Begin { .. } => ArtifactTransferRequestKind::Begin,
            ArtifactTransferOperation::Chunk { .. } => ArtifactTransferRequestKind::Chunk,
            ArtifactTransferOperation::Complete { .. } => ArtifactTransferRequestKind::Complete,
        };
        self.record_request(kind)?;
        let result = (|| {
            request.validate()?;
            match request.operation {
                ArtifactTransferOperation::Begin { manifest } => {
                    let _lifecycle = self.lifecycle.write().map_err(lock_error)?;
                    let lock = self.session_lock(&manifest.manifest_digest)?;
                    let _session = lock.lock().map_err(lock_error)?;
                    self.begin(authenticated_source, local_target, *manifest, now)
                }
                ArtifactTransferOperation::Chunk {
                    manifest_digest,
                    sha256,
                    offset,
                    bytes,
                    ..
                } => {
                    let _lifecycle = self.lifecycle.read().map_err(lock_error)?;
                    let lock = self.session_lock(&manifest_digest)?;
                    let _session = lock.lock().map_err(lock_error)?;
                    self.chunk(
                        (authenticated_source, local_target),
                        &manifest_digest,
                        &sha256,
                        offset,
                        &bytes,
                        now,
                    )
                }
                ArtifactTransferOperation::Complete {
                    manifest_digest,
                    completed_at,
                } => {
                    let _lifecycle = self.lifecycle.read().map_err(lock_error)?;
                    let lock = self.session_lock(&manifest_digest)?;
                    let _session = lock.lock().map_err(lock_error)?;
                    self.complete(
                        authenticated_source,
                        local_target,
                        &manifest_digest,
                        completed_at,
                        now,
                    )
                }
            }
        })();
        self.record_result(&result)?;
        result
    }

    fn begin(
        &self,
        source: &NodeId,
        target: &NodeId,
        manifest: ArtifactTransferManifest,
        now: u64,
    ) -> Result<ArtifactTransferRpcResult> {
        validate_peers(&manifest, source, target)?;
        self.collect_garbage_locked(now)?;
        let directory = self.session_directory(&manifest.manifest_digest);
        if !directory.exists() {
            self.admit(&manifest, now)?;
        }
        fs::create_dir_all(&directory).map_err(cluster_io_error)?;
        let manifest_path = directory.join("manifest.json");
        if manifest_path.exists() {
            let persisted = read_manifest(&manifest_path)?;
            if persisted != manifest {
                return Err(ClusterError::Denied(
                    "artifact session digest is already bound to another manifest".into(),
                ));
            }
        } else {
            publish_json(&manifest_path, &manifest)?;
        }
        self.touch_session(&directory, &manifest.manifest_digest, now, false)?;
        let objects = distinct_objects(&manifest)
            .into_iter()
            .map(|(sha256, expected_length)| self.progress(&directory, &sha256, expected_length))
            .collect::<Result<Vec<_>>>()?;
        Ok(ArtifactTransferRpcResult::Progress {
            manifest_digest: manifest.manifest_digest,
            objects,
        })
    }

    fn chunk(
        &self,
        peers: (&NodeId, &NodeId),
        manifest_digest: &str,
        sha256: &str,
        offset: u64,
        bytes: &[u8],
        now: u64,
    ) -> Result<ArtifactTransferRpcResult> {
        let directory = self.session_directory(manifest_digest);
        let manifest = read_manifest(&directory.join("manifest.json"))?;
        validate_peers(&manifest, peers.0, peers.1)?;
        if manifest.manifest_digest != manifest_digest {
            return Err(ClusterError::Denied(
                "artifact chunk session differs from its persisted manifest".into(),
            ));
        }
        let expected_length = manifest
            .objects
            .iter()
            .find(|object| object.sha256 == sha256)
            .map(|object| object.length)
            .ok_or_else(|| {
                ClusterError::Denied("artifact chunk digest is absent from the manifest".into())
            })?;
        let current = self.progress(&directory, sha256, expected_length)?;
        if current.complete || current.next_offset != offset {
            return Ok(ArtifactTransferRpcResult::ChunkAccepted {
                manifest_digest: manifest_digest.to_owned(),
                object: current,
            });
        }
        let next = offset
            .checked_add(bytes.len() as u64)
            .ok_or_else(|| ClusterError::Invalid("artifact chunk offset overflowed u64".into()))?;
        if next > expected_length {
            return Err(ClusterError::Denied(
                "artifact chunk exceeds its manifest length".into(),
            ));
        }
        let part = directory.join(format!("{sha256}.part"));
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&part)
            .map_err(cluster_io_error)?;
        file.write_all(bytes).map_err(cluster_io_error)?;
        file.sync_data().map_err(cluster_io_error)?;
        drop(file);
        sync_directory(&directory)?;
        if next == expected_length {
            let mut source_file = File::open(&part).map_err(cluster_io_error)?;
            let published = self
                .store
                .put_verified_stream(sha256, expected_length, &mut source_file)
                .map_err(cluster_store_error);
            if let Err(error) = published {
                let _ = fs::remove_file(&part);
                return Err(error);
            }
            let marker = directory.join(format!("{sha256}.transferred"));
            if !marker.exists() {
                publish_bytes(&marker, b"transferred-v1")?;
            }
            fs::remove_file(&part).map_err(cluster_io_error)?;
            sync_directory(&directory)?;
        }
        self.touch_session(&directory, manifest_digest, now, false)?;
        Ok(ArtifactTransferRpcResult::ChunkAccepted {
            manifest_digest: manifest_digest.to_owned(),
            object: self.progress(&directory, sha256, expected_length)?,
        })
    }

    fn complete(
        &self,
        source: &NodeId,
        target: &NodeId,
        manifest_digest: &str,
        completed_at: u64,
        now: u64,
    ) -> Result<ArtifactTransferRpcResult> {
        let directory = self.session_directory(manifest_digest);
        let manifest = read_manifest(&directory.join("manifest.json"))?;
        validate_peers(&manifest, source, target)?;
        if manifest.manifest_digest != manifest_digest {
            return Err(ClusterError::Denied(
                "artifact completion session differs from its manifest".into(),
            ));
        }
        let receipt_path = directory.join("receipt.json");
        if receipt_path.exists() {
            let receipt: ArtifactTransferReceipt = read_json(&receipt_path, "artifact receipt")?;
            receipt.validate(&manifest)?;
            self.touch_session(&directory, manifest_digest, now, true)?;
            self.record_receipt_replay()?;
            return Ok(ArtifactTransferRpcResult::Completed { receipt });
        }
        let mut accounted = BTreeSet::new();
        let objects = manifest
            .objects
            .iter()
            .map(|object| {
                let verified = self
                    .store
                    .verify(&object.sha256)
                    .map_err(cluster_store_error)?;
                if verified.length != object.length {
                    return Err(ClusterError::Invalid(format!(
                        "completed artifact {} has length {}, expected {}",
                        object.sha256, verified.length, object.length
                    )));
                }
                Ok(ArtifactReplicaObjectReceipt {
                    reference: object.reference.clone(),
                    sha256: object.sha256.clone(),
                    length: object.length,
                    target: verified.receipt,
                    transferred: accounted.insert(object.sha256.clone())
                        && directory
                            .join(format!("{}.transferred", object.sha256))
                            .is_file(),
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let receipt = ArtifactTransferReceipt::new(&manifest, objects, completed_at)?;
        publish_json(&receipt_path, &receipt)?;
        self.touch_session(&directory, manifest_digest, now, true)?;
        Ok(ArtifactTransferRpcResult::Completed { receipt })
    }

    fn progress(
        &self,
        directory: &Path,
        sha256: &str,
        expected_length: u64,
    ) -> Result<ArtifactObjectProgress> {
        match self.store.verify(sha256) {
            Ok(verified) if verified.length == expected_length => {
                return Ok(ArtifactObjectProgress {
                    sha256: sha256.to_owned(),
                    expected_length,
                    next_offset: expected_length,
                    complete: true,
                })
            }
            Ok(verified) => {
                return Err(ClusterError::Invalid(format!(
                    "target object {sha256} has length {}, expected {expected_length}",
                    verified.length
                )))
            }
            Err(StoreError::ObjectMissing(_)) | Err(StoreError::ObjectCorrupt { .. }) => {}
            Err(error) => return Err(cluster_store_error(error)),
        }
        if expected_length == 0 {
            let mut empty = std::io::Cursor::new(Vec::<u8>::new());
            self.store
                .put_verified_stream(sha256, 0, &mut empty)
                .map_err(cluster_store_error)?;
            let marker = directory.join(format!("{sha256}.transferred"));
            if !marker.exists() {
                publish_bytes(&marker, b"transferred-v1")?;
            }
            return Ok(ArtifactObjectProgress {
                sha256: sha256.to_owned(),
                expected_length,
                next_offset: 0,
                complete: true,
            });
        }
        let part = directory.join(format!("{sha256}.part"));
        let next_offset = match fs::symlink_metadata(&part) {
            Ok(metadata) if metadata.is_file() => metadata.len(),
            Ok(_) => {
                return Err(ClusterError::Denied(
                    "artifact session part is not a regular file".into(),
                ))
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => 0,
            Err(error) => return Err(cluster_io_error(error)),
        };
        if next_offset > expected_length {
            return Err(ClusterError::Denied(
                "artifact session offset exceeds its manifest length".into(),
            ));
        }
        Ok(ArtifactObjectProgress {
            sha256: sha256.to_owned(),
            expected_length,
            next_offset,
            complete: false,
        })
    }

    fn session_directory(&self, manifest_digest: &str) -> PathBuf {
        self.sessions.join(manifest_digest)
    }

    fn session_lock(&self, manifest_digest: &str) -> Result<Arc<Mutex<()>>> {
        let mut locks = self.session_locks.lock().map_err(lock_error)?;
        locks.retain(|_, lock| lock.strong_count() > 0);
        if let Some(lock) = locks.get(manifest_digest).and_then(Weak::upgrade) {
            return Ok(lock);
        }
        let lock = Arc::new(Mutex::new(()));
        locks.insert(manifest_digest.to_owned(), Arc::downgrade(&lock));
        Ok(lock)
    }

    fn touch_session(
        &self,
        directory: &Path,
        manifest_digest: &str,
        now: u64,
        completed: bool,
    ) -> Result<()> {
        let path = directory.join(TRANSFER_SESSION_STATE_FILE);
        let mut state = if path.exists() {
            let state: ArtifactTransferSessionState = read_json(&path, "artifact session state")?;
            state.validate(manifest_digest)?;
            state
        } else {
            ArtifactTransferSessionState::active(manifest_digest.to_owned(), now)
        };
        state.last_activity_at = now;
        if completed && state.completed_at.is_none() {
            state.completed_at = Some(now);
        }
        publish_json(&path, &state)
    }

    fn admit(&self, manifest: &ArtifactTransferManifest, now: u64) -> Result<()> {
        let entries = self.session_entries(now)?;
        let inventory = summarize_sessions(&entries)?;
        if inventory.active_sessions >= self.policy.max_active_sessions {
            self.record_quota_denial()?;
            return Err(ClusterError::Unavailable(format!(
                "artifact receiver active-session quota {} is exhausted",
                self.policy.max_active_sessions
            )));
        }
        let reservation = self.manifest_reserved_bytes(manifest)?;
        let next = inventory
            .reserved_bytes
            .checked_add(reservation)
            .ok_or_else(|| ClusterError::Unavailable("artifact reservation overflowed".into()))?;
        if next > self.policy.max_reserved_bytes {
            self.record_quota_denial()?;
            return Err(ClusterError::Unavailable(format!(
                "artifact receiver reserved-byte quota {} would be exceeded",
                self.policy.max_reserved_bytes
            )));
        }
        Ok(())
    }

    fn manifest_reserved_bytes(&self, manifest: &ArtifactTransferManifest) -> Result<u64> {
        distinct_objects(manifest)
            .into_iter()
            .try_fold(0u64, |total, (sha256, length)| {
                let needed = match self.store.verify(&sha256) {
                    Ok(verified) if verified.length == length => 0,
                    Ok(verified) => {
                        return Err(ClusterError::Invalid(format!(
                            "target object {sha256} has length {}, expected {length}",
                            verified.length
                        )))
                    }
                    Err(StoreError::ObjectMissing(_)) | Err(StoreError::ObjectCorrupt { .. }) => {
                        length
                    }
                    Err(error) => return Err(cluster_store_error(error)),
                };
                total.checked_add(needed).ok_or_else(|| {
                    ClusterError::Invalid("artifact manifest reservation overflowed".into())
                })
            })
    }

    pub fn session_inventory(&self, now: u64) -> Result<ArtifactTransferSessionInventory> {
        let _lifecycle = self.lifecycle.write().map_err(lock_error)?;
        summarize_sessions(&self.session_entries(now)?)
    }

    pub fn telemetry_snapshot(&self, now: u64) -> Result<ArtifactTransferTelemetrySnapshot> {
        let started_at = self.telemetry.lock().map_err(lock_error)?.started_at;
        if now < started_at {
            return Err(ClusterError::Invalid(
                "artifact telemetry observation predates this receiver process".into(),
            ));
        }
        let inventory = self.session_inventory(now)?;
        let state = self.telemetry.lock().map_err(lock_error)?;
        Ok(ArtifactTransferTelemetrySnapshot {
            contract_version: ARTIFACT_TRANSFER_TELEMETRY_VERSION,
            started_at: state.started_at,
            observed_at: now,
            policy: self.policy.clone(),
            inventory,
            begin_requests: state.begin_requests,
            chunk_requests: state.chunk_requests,
            complete_requests: state.complete_requests,
            begin_responses: state.begin_responses,
            accepted_chunks: state.accepted_chunks,
            completed_responses: state.completed_responses,
            completed_receipt_replays: state.completed_receipt_replays,
            denied: state.denied,
            failed: state.failed,
            quota_denials: state.quota_denials,
            gc_runs: state.gc_runs,
            gc_removed_incomplete: state.gc_removed_incomplete,
            gc_removed_completed: state.gc_removed_completed,
            gc_reclaimed_partial_bytes: state.gc_reclaimed_partial_bytes,
            overflowed: state.overflowed,
        })
    }

    pub fn collect_garbage(&self, now: u64) -> Result<ArtifactTransferGcReport> {
        let _lifecycle = self.lifecycle.write().map_err(lock_error)?;
        self.collect_garbage_locked(now)
    }

    fn collect_garbage_locked(&self, now: u64) -> Result<ArtifactTransferGcReport> {
        let entries = self.session_entries(now)?;
        let scanned_sessions = entries.len();
        let mut remove = BTreeSet::new();
        for entry in &entries {
            if !entry.complete
                && now.saturating_sub(entry.state.last_activity_at)
                    >= self.policy.stale_incomplete_after_millis
            {
                remove.insert(entry.digest.clone());
            }
            if entry.complete
                && now.saturating_sub(
                    entry
                        .state
                        .completed_at
                        .unwrap_or(entry.state.last_activity_at),
                ) >= self.policy.completed_receipt_retention_millis
            {
                remove.insert(entry.digest.clone());
            }
        }
        let mut retained = entries
            .iter()
            .filter(|entry| entry.complete && !remove.contains(&entry.digest))
            .collect::<Vec<_>>();
        retained.sort_by_key(|entry| {
            (
                entry
                    .state
                    .completed_at
                    .unwrap_or(entry.state.last_activity_at),
                entry.digest.as_str(),
            )
        });
        let excess = retained
            .len()
            .saturating_sub(self.policy.max_retained_receipts);
        for entry in retained.into_iter().take(excess) {
            remove.insert(entry.digest.clone());
        }

        let mut removed_incomplete = 0usize;
        let mut removed_completed = 0usize;
        let mut reclaimed_partial_bytes = 0u64;
        for entry in entries
            .iter()
            .filter(|entry| remove.contains(&entry.digest))
        {
            if entry.complete {
                removed_completed += 1;
            } else {
                removed_incomplete += 1;
            }
            reclaimed_partial_bytes = reclaimed_partial_bytes
                .checked_add(entry.partial_bytes)
                .ok_or_else(|| {
                    ClusterError::Unavailable("artifact GC byte count overflowed".into())
                })?;
            remove_session_directory(&self.sessions, &entry.path, &entry.digest)?;
        }
        if !remove.is_empty() {
            sync_directory(&self.sessions)?;
            let mut locks = self.session_locks.lock().map_err(lock_error)?;
            for digest in &remove {
                locks.remove(digest);
            }
        }
        let remaining = summarize_sessions(&self.session_entries(now)?)?;
        let report = ArtifactTransferGcReport {
            scanned_sessions,
            removed_incomplete,
            removed_completed,
            reclaimed_partial_bytes,
            remaining,
        };
        self.record_gc(&report)?;
        Ok(report)
    }

    fn session_entries(&self, now: u64) -> Result<Vec<SessionInventoryEntry>> {
        let mut entries = Vec::new();
        for entry in fs::read_dir(&self.sessions).map_err(cluster_io_error)? {
            let entry = entry.map_err(cluster_io_error)?;
            let file_type = entry.file_type().map_err(cluster_io_error)?;
            let digest = entry
                .file_name()
                .into_string()
                .map_err(|_| ClusterError::Denied("artifact session name is not UTF-8".into()))?;
            if !file_type.is_dir() || !is_sha256(&digest) {
                return Err(ClusterError::Denied(
                    "artifact session root contains an unexpected entry".into(),
                ));
            }
            let path = entry.path();
            let manifest = read_manifest(&path.join("manifest.json"))?;
            if manifest.manifest_digest != digest {
                return Err(ClusterError::Denied(
                    "artifact session directory differs from its manifest".into(),
                ));
            }
            let receipt_path = path.join("receipt.json");
            let complete = match fs::symlink_metadata(&receipt_path) {
                Ok(metadata) if metadata.file_type().is_file() => true,
                Ok(_) => {
                    return Err(ClusterError::Denied(
                        "artifact receipt is not a regular file".into(),
                    ))
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
                Err(error) => return Err(cluster_io_error(error)),
            };
            if complete {
                let receipt: ArtifactTransferReceipt =
                    read_json(&receipt_path, "artifact receipt")?;
                receipt.validate(&manifest)?;
            }
            let state_path = path.join(TRANSFER_SESSION_STATE_FILE);
            let state_existed = state_path.exists();
            let mut state_changed = !state_existed;
            let mut state = if state_existed {
                let state: ArtifactTransferSessionState =
                    read_json(&state_path, "artifact session state")?;
                state.validate(&digest)?;
                state
            } else {
                ArtifactTransferSessionState::active(digest.clone(), now)
            };
            if complete && state.completed_at.is_none() {
                state.completed_at = Some(now);
                state_changed = true;
            } else if !complete && state.completed_at.is_some() {
                return Err(ClusterError::Denied(
                    "artifact session claims completion without a receipt".into(),
                ));
            }
            if state_changed {
                publish_json(&state_path, &state)?;
            }

            let expected = distinct_objects(&manifest);
            let mut partial_bytes = 0u64;
            for child in fs::read_dir(&path).map_err(cluster_io_error)? {
                let child = child.map_err(cluster_io_error)?;
                let name = child.file_name().into_string().map_err(|_| {
                    ClusterError::Denied("artifact session entry is not UTF-8".into())
                })?;
                let child_type = child.file_type().map_err(cluster_io_error)?;
                if name == "manifest.json"
                    || name == "receipt.json"
                    || name == TRANSFER_SESSION_STATE_FILE
                    || (name.ends_with(".transferred")
                        && expected.contains_key(name.trim_end_matches(".transferred")))
                {
                    if !child_type.is_file() {
                        return Err(ClusterError::Denied(
                            "artifact session metadata is not a regular file".into(),
                        ));
                    }
                    continue;
                }
                if let Some(sha256) = name.strip_suffix(".part") {
                    let Some(expected_length) = expected.get(sha256) else {
                        return Err(ClusterError::Denied(
                            "artifact session part is absent from its manifest".into(),
                        ));
                    };
                    let metadata = child.metadata().map_err(cluster_io_error)?;
                    if !child_type.is_file() || metadata.len() > *expected_length {
                        return Err(ClusterError::Denied(
                            "artifact session part is outside its manifest bound".into(),
                        ));
                    }
                    partial_bytes = partial_bytes.checked_add(metadata.len()).ok_or_else(|| {
                        ClusterError::Unavailable("artifact partial byte count overflowed".into())
                    })?;
                    continue;
                }
                if name.starts_with('.') && name.ends_with(".pending") && child_type.is_file() {
                    fs::remove_file(child.path()).map_err(cluster_io_error)?;
                    continue;
                }
                return Err(ClusterError::Denied(
                    "artifact session contains an unexpected entry".into(),
                ));
            }
            if complete && partial_bytes != 0 {
                return Err(ClusterError::Denied(
                    "completed artifact session retains partial bytes".into(),
                ));
            }
            let reserved_bytes = if complete {
                0
            } else {
                self.manifest_reserved_bytes(&manifest)?
            };
            entries.push(SessionInventoryEntry {
                digest,
                path,
                state,
                complete,
                reserved_bytes,
                partial_bytes,
            });
        }
        entries.sort_by(|left, right| left.digest.cmp(&right.digest));
        Ok(entries)
    }

    fn record_request(&self, kind: ArtifactTransferRequestKind) -> Result<()> {
        let mut state = self.telemetry.lock().map_err(lock_error)?;
        let mut overflowed = state.overflowed;
        let counter = match kind {
            ArtifactTransferRequestKind::Begin => &mut state.begin_requests,
            ArtifactTransferRequestKind::Chunk => &mut state.chunk_requests,
            ArtifactTransferRequestKind::Complete => &mut state.complete_requests,
        };
        telemetry_add(counter, 1, &mut overflowed);
        state.overflowed = overflowed;
        Ok(())
    }

    fn record_result(&self, result: &Result<ArtifactTransferRpcResult>) -> Result<()> {
        let mut state = self.telemetry.lock().map_err(lock_error)?;
        let mut overflowed = state.overflowed;
        match result {
            Ok(ArtifactTransferRpcResult::Progress { .. }) => {
                telemetry_add(&mut state.begin_responses, 1, &mut overflowed)
            }
            Ok(ArtifactTransferRpcResult::ChunkAccepted { .. }) => {
                telemetry_add(&mut state.accepted_chunks, 1, &mut overflowed)
            }
            Ok(ArtifactTransferRpcResult::Completed { .. }) => {
                telemetry_add(&mut state.completed_responses, 1, &mut overflowed)
            }
            Err(ClusterError::Invalid(_) | ClusterError::Denied(_)) => {
                telemetry_add(&mut state.denied, 1, &mut overflowed)
            }
            Err(_) => telemetry_add(&mut state.failed, 1, &mut overflowed),
        }
        state.overflowed = overflowed;
        Ok(())
    }

    fn record_receipt_replay(&self) -> Result<()> {
        let mut state = self.telemetry.lock().map_err(lock_error)?;
        let mut overflowed = state.overflowed;
        telemetry_add(&mut state.completed_receipt_replays, 1, &mut overflowed);
        state.overflowed = overflowed;
        Ok(())
    }

    fn record_quota_denial(&self) -> Result<()> {
        let mut state = self.telemetry.lock().map_err(lock_error)?;
        let mut overflowed = state.overflowed;
        telemetry_add(&mut state.quota_denials, 1, &mut overflowed);
        state.overflowed = overflowed;
        Ok(())
    }

    fn record_gc(&self, report: &ArtifactTransferGcReport) -> Result<()> {
        let mut state = self.telemetry.lock().map_err(lock_error)?;
        let mut overflowed = state.overflowed;
        telemetry_add(&mut state.gc_runs, 1, &mut overflowed);
        telemetry_add(
            &mut state.gc_removed_incomplete,
            report.removed_incomplete as u64,
            &mut overflowed,
        );
        telemetry_add(
            &mut state.gc_removed_completed,
            report.removed_completed as u64,
            &mut overflowed,
        );
        telemetry_add(
            &mut state.gc_reclaimed_partial_bytes,
            report.reclaimed_partial_bytes,
            &mut overflowed,
        );
        state.overflowed = overflowed;
        Ok(())
    }
}
