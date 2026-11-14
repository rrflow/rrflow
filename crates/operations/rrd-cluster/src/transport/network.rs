//! The Raft network factory and client, artifact sources and RPCs.

use super::*;

#[derive(Clone)]
pub struct RrdRaftNetworkFactory {
    binding: RrdTransportBinding,
    connector: TlsConnector,
    gate: RrdTransportGate,
    reloader: Option<RrdTlsReloader>,
    artifact_source: Option<RrdArtifactSource>,
}

#[derive(Clone)]
pub(super) struct RrdArtifactSource {
    state_machine: RrdRaftStateMachine,
    objects: LocalObjectStore,
    scope: ScopeId,
    observer: Option<Arc<dyn ArtifactTransferObserver>>,
    attempt_counter: Arc<AtomicU64>,
}

pub(super) async fn observe_artifact(
    source: &RrdArtifactSource,
    observation: ArtifactTransferObservation,
) -> ClusterResult<()> {
    observation.validate()?;
    match &source.observer {
        Some(observer) => observer.observe(observation).await,
        None => Ok(()),
    }
}

impl RrdRaftNetworkFactory {
    pub fn new(binding: RrdTransportBinding, client: Arc<ClientConfig>) -> ClusterResult<Self> {
        Self::new_with_gate(binding, client, RrdTransportGate::enabled())
    }

    pub fn new_with_gate(
        binding: RrdTransportBinding,
        client: Arc<ClientConfig>,
        gate: RrdTransportGate,
    ) -> ClusterResult<Self> {
        binding.validate()?;
        Ok(Self {
            binding,
            connector: TlsConnector::from(client),
            gate,
            reloader: None,
            artifact_source: None,
        })
    }

    pub fn new_reloadable(
        binding: RrdTransportBinding,
        credentials: RrdTlsReloader,
        gate: RrdTransportGate,
    ) -> ClusterResult<Self> {
        if binding != credentials.binding {
            return Err(ClusterError::Denied(
                "TLS credential binding differs from the Raft network binding".into(),
            ));
        }
        let client = credentials.client_config()?;
        binding.validate()?;
        Ok(Self {
            binding,
            connector: TlsConnector::from(client),
            gate,
            reloader: Some(credentials),
            artifact_source: None,
        })
    }

    pub fn new_reloadable_with_artifacts(
        binding: RrdTransportBinding,
        credentials: RrdTlsReloader,
        gate: RrdTransportGate,
        state_machine: RrdRaftStateMachine,
        objects: LocalObjectStore,
        scope: ScopeId,
    ) -> ClusterResult<Self> {
        let mut factory = Self::new_reloadable(binding, credentials, gate)?;
        factory.artifact_source = Some(RrdArtifactSource {
            state_machine,
            objects,
            scope,
            observer: None,
            attempt_counter: Arc::new(AtomicU64::new(0)),
        });
        Ok(factory)
    }

    pub fn with_artifact_observer(mut self, observer: Arc<dyn ArtifactTransferObserver>) -> Self {
        if let Some(source) = &mut self.artifact_source {
            source.observer = Some(observer);
        }
        self
    }

    pub async fn submit_runtime_commit(
        &self,
        target: u64,
        node: &RrdRaftNode,
        command: RrdRaftCommand,
    ) -> std::result::Result<ClientWriteResponse<RrdRaftTypeConfig>, RrdConsensusCommitError> {
        let mut factory = self.clone();
        let client = factory.new_client(target, node).await;
        client.submit_runtime_commit(command).await
    }
}

pub struct RrdRaftNetworkClient {
    source: RrdTransportBinding,
    target_id: u64,
    target: RrdRaftNode,
    connector: TlsConnector,
    gate: RrdTransportGate,
    reloader: Option<RrdTlsReloader>,
    artifact_source: Option<RrdArtifactSource>,
    hydrated_snapshot: Option<String>,
}

impl RaftNetworkFactory<RrdRaftTypeConfig> for RrdRaftNetworkFactory {
    type Network = RrdRaftNetworkClient;

    async fn new_client(&mut self, target: u64, node: &RrdRaftNode) -> Self::Network {
        RrdRaftNetworkClient {
            source: self.binding.clone(),
            target_id: target,
            target: node.clone(),
            connector: self.connector.clone(),
            gate: self.gate.clone(),
            reloader: self.reloader.clone(),
            artifact_source: self.artifact_source.clone(),
            hydrated_snapshot: None,
        }
    }
}

impl RaftNetwork<RrdRaftTypeConfig> for RrdRaftNetworkClient {
    async fn append_entries(
        &mut self,
        request: AppendEntriesRequest<RrdRaftTypeConfig>,
        option: RPCOption,
    ) -> std::result::Result<AppendEntriesResponse<u64>, RPCError<u64, RrdRaftNode, RaftError<u64>>>
    {
        match self
            .call_with_timeout(
                WireRequest::Append(request),
                &option,
                RPCTypes::AppendEntries,
            )
            .await
            .map_err(|error| *error)?
        {
            WireResponse::Append(result) => {
                remote_result(self.target_id, self.target.clone(), result)
            }
            _ => Err(unreachable(
                "transport response kind did not match append request",
            )),
        }
    }

    async fn install_snapshot(
        &mut self,
        request: InstallSnapshotRequest<RrdRaftTypeConfig>,
        option: RPCOption,
    ) -> std::result::Result<
        InstallSnapshotResponse<u64>,
        RPCError<u64, RrdRaftNode, RaftError<u64, InstallSnapshotError>>,
    > {
        // `full_snapshot` hydrates the closure once per transfer attempt before
        // OpenRaft starts its per-chunk deadline. Keep this fallback for direct
        // `install_snapshot` callers, but never repeat hydration merely because
        // the first chunk has offset zero: doing so moves even an idempotent
        // target round-trip back inside OpenRaft's short chunk timeout.
        if artifact_hydration_required(
            self.artifact_source.is_some(),
            self.hydrated_snapshot.as_deref(),
            &request.meta.snapshot_id,
        ) {
            let attempt = self
                .next_artifact_attempt()
                .map_err(|error| unreachable(error.to_string()))?;
            self.hydrate_snapshot_artifacts(&request.meta, attempt)
                .await
                .map_err(|error| unreachable(error.to_string()))?;
            self.hydrated_snapshot = Some(request.meta.snapshot_id.clone());
        }
        match self
            .call_with_timeout(
                WireRequest::Snapshot(request),
                &option,
                RPCTypes::InstallSnapshot,
            )
            .await
            .map_err(|error| *error)?
        {
            WireResponse::Snapshot(result) => {
                remote_result(self.target_id, self.target.clone(), result)
            }
            _ => Err(unreachable(
                "transport response kind did not match snapshot request",
            )),
        }
    }

    async fn vote(
        &mut self,
        request: VoteRequest<u64>,
        option: RPCOption,
    ) -> std::result::Result<VoteResponse<u64>, RPCError<u64, RrdRaftNode, RaftError<u64>>> {
        match self
            .call_with_timeout(WireRequest::Vote(request), &option, RPCTypes::Vote)
            .await
            .map_err(|error| *error)?
        {
            WireResponse::Vote(result) => {
                remote_result(self.target_id, self.target.clone(), result)
            }
            _ => Err(unreachable(
                "transport response kind did not match vote request",
            )),
        }
    }

    async fn full_snapshot(
        &mut self,
        vote: Vote<u64>,
        snapshot: Snapshot<RrdRaftTypeConfig>,
        cancel: impl Future<Output = ReplicationClosed> + OptionalSend + 'static,
        option: RPCOption,
    ) -> std::result::Result<SnapshotResponse<u64>, StreamingError<RrdRaftTypeConfig, Fatal<u64>>>
    {
        let mut cancel = Box::pin(cancel);
        if self.artifact_source.is_some() {
            let attempt = self
                .next_artifact_attempt()
                .map_err(|error| NetworkError::new(&io::Error::other(error.to_string())))?;
            tokio::select! {
                closed = cancel.as_mut() => return Err(closed.into()),
                hydrated = self.hydrate_snapshot_artifacts(&snapshot.meta, attempt) => {
                    if let Err(error) = hydrated {
                        let error = io::Error::other(error.to_string());
                        return Err(NetworkError::new(&error).into());
                    }
                }
            }
            self.hydrated_snapshot = Some(snapshot.meta.snapshot_id.clone());
        }
        Chunked::send_snapshot(self, vote, snapshot, cancel, option).await
    }
}

pub(super) fn artifact_hydration_required(
    artifact_source_enabled: bool,
    hydrated_snapshot: Option<&str>,
    requested_snapshot: &str,
) -> bool {
    artifact_source_enabled && hydrated_snapshot != Some(requested_snapshot)
}

impl RrdRaftNetworkClient {
    async fn submit_runtime_commit(
        &self,
        command: RrdRaftCommand,
    ) -> std::result::Result<ClientWriteResponse<RrdRaftTypeConfig>, RrdConsensusCommitError> {
        command
            .validate()
            .map_err(|error| RrdConsensusCommitError::Rejected(error.to_string()))?;
        if !matches!(
            command.operation,
            crate::RrdRaftOperation::RuntimeCommit { .. }
        ) {
            return Err(RrdConsensusCommitError::Rejected(
                "internal consensus route accepts only runtime commits".into(),
            ));
        }
        let response = tokio::time::timeout(
            RRFLOW_CONSENSUS_COMMIT_RPC_TIMEOUT,
            self.call::<std::io::Error>(WireRequest::RuntimeCommit(Box::new(command))),
        )
        .await
        .map_err(|_| RrdConsensusCommitError::Unavailable("consensus commit RPC timed out".into()))?
        .map_err(|error| RrdConsensusCommitError::Unavailable(error.to_string()))?;
        match response {
            WireResponse::RuntimeCommit(Ok(response)) => Ok(response),
            WireResponse::RuntimeCommit(Err(ConsensusCommitWireError::Raft(error)))
                if matches!(
                    error.api_error(),
                    Some(ClientWriteError::ForwardToLeader(_))
                ) =>
            {
                Err(RrdConsensusCommitError::ForwardToLeader)
            }
            WireResponse::RuntimeCommit(Err(ConsensusCommitWireError::Denied(error))) => {
                Err(RrdConsensusCommitError::Rejected(error))
            }
            WireResponse::RuntimeCommit(Err(ConsensusCommitWireError::Raft(error))) => {
                Err(RrdConsensusCommitError::Rejected(error.to_string()))
            }
            _ => Err(RrdConsensusCommitError::Unavailable(
                "consensus commit response kind did not match request".into(),
            )),
        }
    }

    fn next_artifact_attempt(&self) -> ClusterResult<u64> {
        let source = self.artifact_source.as_ref().ok_or_else(|| {
            ClusterError::Unavailable("artifact transfer source is not configured".into())
        })?;
        source
            .attempt_counter
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |attempt| {
                attempt.checked_add(1)
            })
            .map(|attempt| attempt + 1)
            .map_err(|_| {
                ClusterError::Unavailable("artifact transfer attempt counter overflowed".into())
            })
    }

    async fn hydrate_snapshot_artifacts(
        &self,
        meta: &openraft::SnapshotMeta<u64, RrdRaftNode>,
        attempt: u64,
    ) -> ClusterResult<()> {
        let Some(source) = self.artifact_source.clone() else {
            return Ok(());
        };
        let state_machine = source.state_machine.clone();
        let meta = meta.clone();
        let scope = source.scope.clone();
        let source_id = self.source.canonical_node_id.clone();
        let target_id = NodeId::new(self.target.canonical_id.clone())?;
        let manifest = tokio::task::spawn_blocking(move || {
            state_machine
                .artifact_manifest_for_cached_snapshot(&meta, &scope, source_id, target_id)
                .map_err(|error| ClusterError::Unavailable(error.to_string()))
        })
        .await
        .map_err(|error| ClusterError::Unavailable(format!("artifact manifest task: {error}")))??;
        let Some(manifest) = manifest else {
            return Ok(());
        };
        observe_artifact(
            &source,
            ArtifactTransferObservation::prepared(&manifest, attempt, now_millis())?,
        )
        .await?;
        let started = Instant::now();
        match self
            .transfer_artifact_manifest(&source, &manifest, attempt)
            .await
        {
            Ok(receipt) => {
                observe_artifact(
                    &source,
                    ArtifactTransferObservation::completed(
                        &manifest,
                        attempt,
                        now_millis(),
                        started.elapsed().as_micros().min(u128::from(u64::MAX)) as u64,
                        &receipt,
                    )?,
                )
                .await
            }
            Err(error) => {
                let rendered = error.to_string();
                let observation = ArtifactTransferObservation::failed(
                    &manifest,
                    attempt,
                    now_millis(),
                    started.elapsed().as_micros().min(u128::from(u64::MAX)) as u64,
                    &rendered,
                )?;
                if let Err(observer_error) = observe_artifact(&source, observation).await {
                    return Err(ClusterError::Unavailable(format!(
                        "{rendered}; artifact failure observation: {observer_error}"
                    )));
                }
                Err(error)
            }
        }
    }

    async fn transfer_artifact_manifest(
        &self,
        source: &RrdArtifactSource,
        manifest: &ArtifactTransferManifest,
        attempt: u64,
    ) -> ClusterResult<ArtifactTransferReceipt> {
        manifest.validate()?;
        let progress = self
            .call_artifact(ArtifactTransferRpc::begin(manifest.clone())?)
            .await?;
        let ArtifactTransferRpcResult::Progress {
            manifest_digest,
            objects,
        } = progress
        else {
            return Err(ClusterError::Unavailable(
                "artifact begin response had the wrong result kind".into(),
            ));
        };
        if manifest_digest != manifest.manifest_digest {
            return Err(ClusterError::Denied(
                "artifact begin response changed the manifest digest".into(),
            ));
        }
        let mut progress = objects
            .into_iter()
            .map(|object| (object.sha256.clone(), object))
            .collect::<BTreeMap<_, _>>();
        let distinct = manifest
            .objects
            .iter()
            .map(|object| (object.sha256.clone(), object.clone()))
            .collect::<BTreeMap<_, _>>();
        if progress.len() != distinct.len()
            || progress.keys().ne(distinct.keys())
            || progress.iter().any(|(digest, state)| {
                state.expected_length != distinct[digest].length
                    || state.next_offset > state.expected_length
                    || state.complete != (state.next_offset == state.expected_length)
            })
        {
            return Err(ClusterError::Denied(
                "artifact begin response differs from the manifest closure".into(),
            ));
        }
        for (sha256, object) in distinct {
            let mut state = progress
                .remove(&sha256)
                .expect("validated progress contains every digest");
            if state.complete {
                continue;
            }
            let reference = object.clone();
            let source_store = source.objects.clone();
            let file = tokio::task::spawn_blocking(move || source_store.verified_file(&reference))
                .await
                .map_err(|error| {
                    ClusterError::Unavailable(format!("artifact source verify task: {error}"))
                })?
                .map_err(|error| ClusterError::Unavailable(error.to_string()))?;
            let mut file = tokio::fs::File::from_std(file);
            while !state.complete {
                file.seek(std::io::SeekFrom::Start(state.next_offset))
                    .await
                    .map_err(|error| ClusterError::Unavailable(error.to_string()))?;
                let remaining = state.expected_length - state.next_offset;
                let chunk_length = remaining.min(ARTIFACT_TRANSFER_CHUNK_MAX_BYTES as u64) as usize;
                let mut bytes = vec![0u8; chunk_length];
                file.read_exact(&mut bytes)
                    .await
                    .map_err(|error| ClusterError::Unavailable(error.to_string()))?;
                let sent_offset = state.next_offset;
                let response = self
                    .call_artifact(ArtifactTransferRpc::chunk(
                        manifest.manifest_digest.clone(),
                        sha256.clone(),
                        sent_offset,
                        bytes,
                    )?)
                    .await?;
                let ArtifactTransferRpcResult::ChunkAccepted {
                    manifest_digest,
                    object: next,
                } = response
                else {
                    return Err(ClusterError::Unavailable(
                        "artifact chunk response had the wrong result kind".into(),
                    ));
                };
                if manifest_digest != manifest.manifest_digest
                    || next.sha256 != sha256
                    || next.expected_length != object.length
                    || next.next_offset > object.length
                    || next.complete != (next.next_offset == object.length)
                    || next.next_offset == sent_offset
                {
                    return Err(ClusterError::Denied(
                        "artifact chunk response did not make valid progress".into(),
                    ));
                }
                observe_artifact(
                    source,
                    ArtifactTransferObservation::progress(manifest, attempt, now_millis(), &next)?,
                )
                .await?;
                state = next;
            }
        }
        let completed_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let response = self
            .call_artifact(ArtifactTransferRpc::complete(
                manifest.manifest_digest.clone(),
                completed_at,
            )?)
            .await?;
        let ArtifactTransferRpcResult::Completed { receipt } = response else {
            return Err(ClusterError::Unavailable(
                "artifact completion response had the wrong result kind".into(),
            ));
        };
        receipt.validate(manifest)?;
        Ok(receipt)
    }

    async fn call_artifact(
        &self,
        request: ArtifactTransferRpc,
    ) -> ClusterResult<ArtifactTransferRpcResult> {
        let response = tokio::time::timeout(
            RRFLOW_ARTIFACT_RPC_TIMEOUT,
            self.call::<std::io::Error>(WireRequest::Artifact(request)),
        )
        .await
        .map_err(|_| ClusterError::Unavailable("artifact RPC timed out".into()))?
        .map_err(|error| ClusterError::Unavailable(error.to_string()))?;
        match response {
            WireResponse::Artifact(Ok(response)) => Ok(response),
            WireResponse::Artifact(Err(error)) => Err(ClusterError::Unavailable(error)),
            _ => Err(ClusterError::Unavailable(
                "artifact response kind did not match request".into(),
            )),
        }
    }

    async fn call_with_timeout<E>(
        &self,
        request: WireRequest,
        option: &RPCOption,
        action: RPCTypes,
    ) -> WireRpcResult<E>
    where
        E: std::error::Error,
    {
        match tokio::time::timeout(option.hard_ttl(), self.call(request)).await {
            Ok(result) => result,
            Err(_) => Err(Box::new(RPCError::Timeout(Timeout {
                action,
                id: self.source.raft_node_id,
                target: self.target_id,
                timeout: option.hard_ttl(),
            }))),
        }
    }

    async fn call<E>(&self, request: WireRequest) -> WireRpcResult<E>
    where
        E: std::error::Error,
    {
        if !self.gate.is_enabled() {
            return Err(Box::new(unreachable("local Raft transport is disabled")));
        }
        self.target
            .validate()
            .map_err(|error| Box::new(unreachable(error.to_string())))?;
        let endpoint = RrdTlsEndpoint::parse(&self.target.endpoint)
            .map_err(|error| Box::new(unreachable(error.to_string())))?;
        let stream = TcpStream::connect(&endpoint.address)
            .await
            .map_err(|error| Box::new(RPCError::Unreachable(Unreachable::new(&error))))?;
        let server_name = ServerName::try_from(endpoint.server_name.clone())
            .map_err(|error| Box::new(unreachable(format!("invalid TLS server name: {error}"))))?;
        let connector = self
            .reloader
            .as_ref()
            .map(RrdTlsReloader::client_config)
            .transpose()
            .map_err(|error| Box::new(unreachable(error.to_string())))?
            .map(TlsConnector::from)
            .unwrap_or_else(|| self.connector.clone());
        let mut stream = connector
            .connect(server_name, stream)
            .await
            .map_err(|error| Box::new(RPCError::Unreachable(Unreachable::new(&error))))?;
        let expected_peer = peer_binding_for_node(&self.source, self.target_id, &self.target)
            .map_err(|error| Box::new(unreachable(error.to_string())))?;
        verify_peer_identity(stream.get_ref().1.peer_certificates(), &expected_peer)
            .map_err(|error| Box::new(unreachable(error.to_string())))?;
        let envelope = WireEnvelope::new(&self.source, self.target_id, &self.target, request)
            .map_err(|error| Box::new(unreachable(error.to_string())))?;
        write_frame(&mut stream, &envelope)
            .await
            .map_err(|error| Box::new(RPCError::Unreachable(Unreachable::new(&error))))?;
        read_frame(&mut stream)
            .await
            .map_err(|error| Box::new(RPCError::Unreachable(Unreachable::new(&error))))
    }
}
