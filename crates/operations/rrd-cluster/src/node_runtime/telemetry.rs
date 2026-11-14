//! Node runtime limits and consensus and artifact-transfer telemetry.

use super::*;

pub const RRFLOW_NODE_CONFIG_VERSION: u16 = 2;
pub const RRFLOW_NODE_CONTROL_VERSION: u16 = 4;
pub const RRFLOW_NODE_MAX_CONFIG_BYTES: u64 = 1024 * 1024;
pub const RRFLOW_NODE_MAX_CONTROL_LINE_BYTES: usize = 1024 * 1024;
pub(super) const LEARNER_CATCH_UP_TIMEOUT: Duration = Duration::from_secs(10);
pub(super) const STORAGE_RELEASE_TIMEOUT: Duration = Duration::from_secs(5);
pub(super) const CONSENSUS_TRACE_COMMIT_RETRIES: usize = 16;
pub(super) const CONSENSUS_TRACE_ROUTE_RETRIES: usize = 32;

pub(super) type RrdRaft = Raft<crate::RrdRaftTypeConfig>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RrdConsensusTraceTelemetrySnapshot {
    pub started_at: u64,
    pub observed_at: u64,
    pub prepared_observations: u64,
    pub chunk_observations: u64,
    pub completed_observations: u64,
    pub failed_observations: u64,
    pub commit_acknowledgements: u64,
    pub cursor_conflicts: u64,
    pub leader_changes: u64,
    pub leader_unavailable: u64,
    pub denied: u64,
    pub failed: u64,
    pub overflowed: bool,
}

#[derive(Debug)]
pub(super) struct ConsensusTraceTelemetry {
    started_at: u64,
    prepared_observations: AtomicU64,
    chunk_observations: AtomicU64,
    completed_observations: AtomicU64,
    failed_observations: AtomicU64,
    commit_acknowledgements: AtomicU64,
    cursor_conflicts: AtomicU64,
    leader_changes: AtomicU64,
    leader_unavailable: AtomicU64,
    denied: AtomicU64,
    failed: AtomicU64,
    overflowed: AtomicBool,
}

impl ConsensusTraceTelemetry {
    pub(super) fn new(started_at: u64) -> Self {
        Self {
            started_at,
            prepared_observations: AtomicU64::new(0),
            chunk_observations: AtomicU64::new(0),
            completed_observations: AtomicU64::new(0),
            failed_observations: AtomicU64::new(0),
            commit_acknowledgements: AtomicU64::new(0),
            cursor_conflicts: AtomicU64::new(0),
            leader_changes: AtomicU64::new(0),
            leader_unavailable: AtomicU64::new(0),
            denied: AtomicU64::new(0),
            failed: AtomicU64::new(0),
            overflowed: AtomicBool::new(false),
        }
    }

    fn record_observation(&self, observation: &ArtifactTransferObservation) {
        use crate::ArtifactTransferObservationPhase as Phase;
        let counter = match observation.phase {
            Phase::Prepared => &self.prepared_observations,
            Phase::ChunkAccepted => &self.chunk_observations,
            Phase::Completed => &self.completed_observations,
            Phase::Failed => &self.failed_observations,
        };
        telemetry_increment(counter, &self.overflowed);
    }

    pub(super) fn snapshot(
        &self,
        observed_at: u64,
    ) -> ClusterResult<RrdConsensusTraceTelemetrySnapshot> {
        if observed_at < self.started_at {
            return Err(ClusterError::Invalid(
                "consensus trace telemetry observation predates this process".into(),
            ));
        }
        Ok(RrdConsensusTraceTelemetrySnapshot {
            started_at: self.started_at,
            observed_at,
            prepared_observations: self.prepared_observations.load(Ordering::Relaxed),
            chunk_observations: self.chunk_observations.load(Ordering::Relaxed),
            completed_observations: self.completed_observations.load(Ordering::Relaxed),
            failed_observations: self.failed_observations.load(Ordering::Relaxed),
            commit_acknowledgements: self.commit_acknowledgements.load(Ordering::Relaxed),
            cursor_conflicts: self.cursor_conflicts.load(Ordering::Relaxed),
            leader_changes: self.leader_changes.load(Ordering::Relaxed),
            leader_unavailable: self.leader_unavailable.load(Ordering::Relaxed),
            denied: self.denied.load(Ordering::Relaxed),
            failed: self.failed.load(Ordering::Relaxed),
            overflowed: self.overflowed.load(Ordering::Relaxed),
        })
    }
}

#[derive(Clone)]
pub(super) struct RrdNodeTelemetrySources {
    pub(super) transport: RrdRaftTlsServer,
    pub(super) artifacts: ArtifactTransferReceiver,
    pub(super) traces: Arc<ConsensusTraceTelemetry>,
}

pub(super) struct ConsensusArtifactTransferObserver {
    pub(super) raft: Arc<OnceLock<RrdRaft>>,
    pub(super) state_machine: RrdRaftStateMachine,
    pub(super) network: RrdRaftNetworkFactory,
    pub(super) nodes: BTreeMap<u64, RrdRaftNode>,
    pub(super) local_node_id: u64,
    pub(super) project_scope: ScopeId,
    pub(super) actor: String,
    pub(super) telemetry: Arc<ConsensusTraceTelemetry>,
}

impl ConsensusArtifactTransferObserver {
    async fn submit(
        &self,
        raft: &RrdRaft,
        command: RrdRaftCommand,
    ) -> ClusterResult<openraft::raft::ClientWriteResponse<crate::RrdRaftTypeConfig>> {
        let mut last_error: Option<String> = None;
        for _ in 0..CONSENSUS_TRACE_ROUTE_RETRIES {
            let metrics = raft.metrics().borrow().clone();
            let Some(leader) = metrics.current_leader else {
                telemetry_increment(
                    &self.telemetry.leader_unavailable,
                    &self.telemetry.overflowed,
                );
                last_error = Some("no current leader".into());
                tokio::time::sleep(Duration::from_millis(25)).await;
                continue;
            };
            let result = if leader == self.local_node_id {
                match raft.client_write(command.clone()).await {
                    Ok(response) => Ok(response),
                    Err(error)
                        if matches!(
                            error.api_error(),
                            Some(openraft::error::ClientWriteError::ForwardToLeader(_))
                        ) =>
                    {
                        Err(RrdConsensusCommitError::ForwardToLeader)
                    }
                    Err(error) if error.fatal().is_some() => {
                        Err(RrdConsensusCommitError::Unavailable(error.to_string()))
                    }
                    Err(error) => Err(RrdConsensusCommitError::Rejected(error.to_string())),
                }
            } else {
                let node = self.nodes.get(&leader).ok_or_else(|| {
                    ClusterError::Unavailable(format!(
                        "current leader {leader} is absent from the node inventory"
                    ))
                })?;
                self.network
                    .submit_runtime_commit(leader, node, command.clone())
                    .await
            };
            match result {
                Ok(response) => return Ok(response),
                Err(RrdConsensusCommitError::ForwardToLeader) => {
                    telemetry_increment(&self.telemetry.leader_changes, &self.telemetry.overflowed);
                    last_error = Some("leader changed while routing".into());
                    tokio::time::sleep(Duration::from_millis(25)).await;
                }
                Err(RrdConsensusCommitError::Rejected(error)) => {
                    return Err(ClusterError::Denied(error))
                }
                Err(RrdConsensusCommitError::Unavailable(error)) => {
                    return Err(ClusterError::Unavailable(error))
                }
            }
        }
        Err(ClusterError::Unavailable(format!(
            "consensus trace could not route to a stable leader after {CONSENSUS_TRACE_ROUTE_RETRIES} attempts: {}",
            last_error.unwrap_or_else(|| "no routing evidence".into())
        )))
    }

    async fn commit(&self, observation: ArtifactTransferObservation) -> ClusterResult<()> {
        observation.validate()?;
        if observation.scope != self.project_scope {
            return Err(ClusterError::Denied(
                "artifact trace scope differs from the configured project".into(),
            ));
        }
        let event = artifact_transfer_trace_event(&observation)?;
        let event_digest = rrd_core::digest::sha256_hex(
            &serde_json::to_vec(&event)
                .map_err(|error| ClusterError::Invalid(error.to_string()))?,
        );
        let raft = self.raft.get().cloned().ok_or_else(|| {
            ClusterError::Unavailable("consensus trace writer is not attached to Raft".into())
        })?;
        let mut last_conflict = None;
        for _ in 0..CONSENSUS_TRACE_COMMIT_RETRIES {
            let (read, schema) = self
                .state_machine
                .runtime_commit_context(&observation.scope)?;
            let commit = event
                .prepare_commit(&read, schema.as_ref(), &self.actor)
                .map_err(|error| ClusterError::Invalid(error.to_string()))?;
            let request_id = format!("artifact-trace:{event_digest}:{}", commit.expected_cursor);
            let command = RrdRaftCommand::runtime_commit(
                request_id,
                observation.shard,
                observation.placement_epoch,
                None,
                commit,
            )?;
            let response = self.submit(&raft, command).await?;
            if response.data.accepted && response.data.runtime_outcome.is_some() {
                telemetry_increment(
                    &self.telemetry.commit_acknowledgements,
                    &self.telemetry.overflowed,
                );
                return Ok(());
            }
            if response
                .data
                .reason
                .contains("runtime commit conflict: expected cursor")
            {
                telemetry_increment(&self.telemetry.cursor_conflicts, &self.telemetry.overflowed);
                last_conflict = Some(response.data.reason);
                continue;
            }
            return Err(ClusterError::Denied(format!(
                "consensus artifact trace commit was rejected: {}",
                response.data.reason
            )));
        }
        Err(ClusterError::Unavailable(format!(
            "consensus artifact trace exhausted {CONSENSUS_TRACE_COMMIT_RETRIES} cursor retries: {}",
            last_conflict.unwrap_or_else(|| "no conflict evidence".into())
        )))
    }
}

impl ArtifactTransferObserver for ConsensusArtifactTransferObserver {
    fn observe(
        &self,
        observation: ArtifactTransferObservation,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ClusterResult<()>> + Send + '_>> {
        Box::pin(async move {
            self.telemetry.record_observation(&observation);
            let result = self.commit(observation).await;
            if let Err(error) = &result {
                match error {
                    ClusterError::Denied(_) | ClusterError::Invalid(_) => {
                        telemetry_increment(&self.telemetry.denied, &self.telemetry.overflowed)
                    }
                    ClusterError::Unavailable(_) | ClusterError::NotFound(_) => {
                        telemetry_increment(&self.telemetry.failed, &self.telemetry.overflowed)
                    }
                }
            }
            result
        })
    }
}
