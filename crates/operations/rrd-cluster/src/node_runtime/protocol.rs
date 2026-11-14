//! Node configuration, commands, requests, status and replies.

use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RrdNodeConfig {
    pub version: u16,
    pub trust_domain: String,
    pub cluster: ClusterId,
    pub shard: ShardId,
    pub project_scope: ScopeId,
    pub raft_node_id: u64,
    pub data_root: PathBuf,
    pub raft_listen: String,
    pub nodes: BTreeMap<u64, RrdRaftNode>,
    pub certificate_der: PathBuf,
    pub private_key_der: PathBuf,
    pub trust_root_der: PathBuf,
    #[serde(default)]
    pub transport_admission: RrdTransportAdmissionPolicy,
    #[serde(default)]
    pub raft_timing: RrdRaftTimingPolicy,
}

impl RrdNodeConfig {
    pub fn load(path: &Path) -> ClusterResult<Self> {
        let metadata = fs::metadata(path)
            .map_err(|error| ClusterError::Unavailable(format!("node config metadata: {error}")))?;
        if metadata.len() == 0 || metadata.len() > RRFLOW_NODE_MAX_CONFIG_BYTES {
            return Err(ClusterError::Invalid(
                "node config must contain 1..=1048576 bytes".into(),
            ));
        }
        let bytes = fs::read(path)
            .map_err(|error| ClusterError::Unavailable(format!("read node config: {error}")))?;
        let config: Self = serde_json::from_slice(&bytes)
            .map_err(|error| ClusterError::Invalid(format!("decode node config: {error}")))?;
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> ClusterResult<()> {
        if self.version != RRFLOW_NODE_CONFIG_VERSION
            || self.raft_node_id == 0
            || self.nodes.is_empty()
            || self.nodes.len() > 1024
            || self.data_root.as_os_str().is_empty()
        {
            return Err(ClusterError::Invalid(
                "node config version, identity, inventory, or data root is invalid".into(),
            ));
        }
        let local = self.nodes.get(&self.raft_node_id).ok_or_else(|| {
            ClusterError::Invalid("node inventory does not contain the local Raft id".into())
        })?;
        for node in self.nodes.values() {
            node.validate()?;
        }
        self.raft_listen
            .parse::<std::net::SocketAddr>()
            .map_err(|error| {
                ClusterError::Invalid(format!("invalid Raft listen address: {error}"))
            })?;
        RrdTransportBinding {
            trust_domain: self.trust_domain.clone(),
            cluster: self.cluster.clone(),
            shard: self.shard,
            raft_node_id: self.raft_node_id,
            canonical_node_id: NodeId::new(local.canonical_id.clone())?,
        }
        .validate()?;
        self.transport_admission.validate()?;
        self.raft_timing.validate()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case", deny_unknown_fields)]
pub enum RrdNodeCommand {
    Status,
    Initialize,
    Elect,
    AddLearner {
        node_id: u64,
    },
    ChangeMembership {
        voters: BTreeSet<u64>,
    },
    PlacementTransition {
        request_id: String,
        placement: ShardPlacement,
        expected_commit_index: Option<u64>,
    },
    Probe {
        request_id: String,
        placement_epoch: u64,
        expected_commit_index: Option<u64>,
        payload: Vec<u8>,
    },
    RuntimeCommit {
        request_id: String,
        placement_epoch: u64,
        expected_commit_index: Option<u64>,
        commit: RuntimeCommit,
    },
    TriggerSnapshot,
    PurgeLog {
        index: u64,
    },
    WaitApplied {
        index: u64,
        timeout_millis: u64,
    },
    SetTransportEnabled {
        enabled: bool,
    },
    RotateCredentials {
        expected_generation: u64,
        files: RrdTlsFiles,
    },
    Shutdown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RrdTlsFiles {
    pub certificate_der: PathBuf,
    pub private_key_der: PathBuf,
    pub trust_root_ders: Vec<PathBuf>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub revocation_list_ders: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RrdNodeRequest {
    pub version: u16,
    pub request_id: String,
    pub command: RrdNodeCommand,
}

impl RrdNodeRequest {
    pub fn validate(&self) -> ClusterResult<()> {
        if self.version != RRFLOW_NODE_CONTROL_VERSION
            || self.request_id.is_empty()
            || self.request_id.len() > 128
            || self.request_id.as_bytes().contains(&0)
        {
            return Err(ClusterError::Invalid(
                "control request version or identity is invalid".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RrdNodeStatus {
    pub project_scope: ScopeId,
    pub cluster: ClusterId,
    pub shard: ShardId,
    pub raft_node_id: u64,
    pub canonical_node_id: NodeId,
    pub current_term: u64,
    pub current_leader: Option<u64>,
    pub last_log_index: Option<u64>,
    pub last_applied_index: Option<u64>,
    pub snapshot_index: Option<u64>,
    pub purged_index: Option<u64>,
    pub state: String,
    pub credentials: RrdTlsGeneration,
    pub telemetry: RrdNodeTelemetrySnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RrdNodeTelemetrySnapshot {
    pub observed_at: u64,
    pub transport_ingress: RrdTransportTelemetrySnapshot,
    pub artifacts: crate::ArtifactTransferTelemetrySnapshot,
    pub consensus_traces: RrdConsensusTraceTelemetrySnapshot,
}

impl RrdConsensusTraceTelemetrySnapshot {
    pub fn validate(&self) -> ClusterResult<()> {
        if self.started_at > self.observed_at {
            return Err(ClusterError::Invalid(
                "consensus trace telemetry observation predates its process".into(),
            ));
        }
        Ok(())
    }
}

impl RrdNodeTelemetrySnapshot {
    pub fn validate(&self) -> ClusterResult<()> {
        if self.transport_ingress.observed_at != self.observed_at
            || self.artifacts.observed_at != self.observed_at
            || self.consensus_traces.observed_at != self.observed_at
        {
            return Err(ClusterError::Invalid(
                "node telemetry sections do not share one observation time".into(),
            ));
        }
        self.transport_ingress.validate()?;
        self.artifacts.validate()?;
        self.consensus_traces.validate()
    }
}

impl RrdNodeStatus {
    pub fn validate(&self) -> ClusterResult<()> {
        if self.raft_node_id == 0
            || self.state.trim().is_empty()
            || self.state.len() > 64
            || self.credentials.generation == 0
            || self.credentials.leaf_digest.len() != 64
            || !self
                .credentials
                .leaf_digest
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
            || self.last_applied_index > self.last_log_index
            || self.purged_index > self.last_log_index
            || self.snapshot_index > self.last_log_index
        {
            return Err(ClusterError::Invalid(
                "node status is outside its bounded contract".into(),
            ));
        }
        self.telemetry.validate()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "result", rename_all = "snake_case", deny_unknown_fields)]
pub enum RrdNodeResult {
    Ready {
        raft_node_id: u64,
    },
    Ack,
    Status {
        status: Box<RrdNodeStatus>,
    },
    Write {
        log_index: u64,
        response: RrdRaftResponse,
    },
    Credentials {
        credentials: RrdTlsGeneration,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RrdNodeReply {
    pub version: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    pub ok: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<RrdNodeResult>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl RrdNodeReply {
    pub(super) fn success(request_id: Option<String>, value: RrdNodeResult) -> Self {
        Self {
            version: RRFLOW_NODE_CONTROL_VERSION,
            request_id,
            ok: true,
            value: Some(value),
            error: None,
        }
    }

    pub(super) fn failure(request_id: Option<String>, error: impl ToString) -> Self {
        Self {
            version: RRFLOW_NODE_CONTROL_VERSION,
            request_id,
            ok: false,
            value: None,
            error: Some(error.to_string()),
        }
    }
}
