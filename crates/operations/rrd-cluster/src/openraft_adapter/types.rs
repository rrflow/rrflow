//! Raft node, operation, command and response types, adapter state and the store.

use super::*;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RrdRaftNode {
    pub canonical_id: String,
    pub zone: String,
    pub endpoint: String,
}

impl RrdRaftNode {
    pub fn validate(&self) -> ClusterResult<()> {
        NodeId::new(self.canonical_id.as_str())?;
        for (label, value) in [
            ("zone", self.zone.as_str()),
            ("endpoint", self.endpoint.as_str()),
        ] {
            if value.is_empty() || value.len() > 512 || value.as_bytes().contains(&0) {
                return Err(ClusterError::Invalid(format!(
                    "raft node {label} must contain 1..=512 non-NUL bytes"
                )));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case")]
pub enum RrdRaftOperation {
    PlacementTransition {
        placement: ShardPlacement,
    },
    Probe {
        payload_digest: String,
        payload: Vec<u8>,
    },
    RuntimeCommit {
        commit: RuntimeCommit,
    },
}

impl RrdRaftOperation {
    fn validate(&self) -> ClusterResult<()> {
        match self {
            Self::PlacementTransition { placement } => placement.validate(),
            Self::Probe {
                payload_digest,
                payload,
            } if !payload.is_empty() && payload_digest == &sha256_hex(payload) => Ok(()),
            Self::Probe { .. } => Err(ClusterError::Invalid(
                "raft probe payload or digest is invalid".into(),
            )),
            Self::RuntimeCommit { commit } => commit
                .validate()
                .map_err(|error| ClusterError::Invalid(error.to_string())),
        }
    }

    pub(super) fn digest(&self) -> String {
        let mut bytes = b"rrflow.raft.operation.v2".to_vec();
        match self {
            Self::PlacementTransition { placement } => {
                bytes.push(1);
                bytes.extend_from_slice(
                    placement
                        .digest()
                        .expect("validated placement has a canonical digest")
                        .as_bytes(),
                );
            }
            Self::Probe { payload_digest, .. } => {
                bytes.push(2);
                bytes.extend_from_slice(payload_digest.as_bytes());
            }
            Self::RuntimeCommit { commit } => {
                bytes.push(3);
                bytes.extend_from_slice(commit.digest().as_bytes());
            }
        }
        sha256_hex(&bytes)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RrdRaftCommand {
    pub request_id: String,
    pub shard: ShardId,
    pub placement_epoch: u64,
    pub expected_commit_index: Option<u64>,
    pub operation: RrdRaftOperation,
}

impl RrdRaftCommand {
    pub fn placement_transition(
        request_id: impl Into<String>,
        placement: ShardPlacement,
        expected_commit_index: Option<u64>,
    ) -> ClusterResult<Self> {
        let command = Self {
            request_id: request_id.into(),
            shard: placement.shard,
            placement_epoch: placement.epoch,
            expected_commit_index,
            operation: RrdRaftOperation::PlacementTransition { placement },
        };
        command.validate()?;
        Ok(command)
    }

    pub fn new(
        request_id: impl Into<String>,
        shard: ShardId,
        placement_epoch: u64,
        expected_commit_index: Option<u64>,
        payload: Vec<u8>,
    ) -> ClusterResult<Self> {
        let command = Self {
            request_id: request_id.into(),
            shard,
            placement_epoch,
            expected_commit_index,
            operation: RrdRaftOperation::Probe {
                payload_digest: sha256_hex(&payload),
                payload,
            },
        };
        command.validate()?;
        Ok(command)
    }

    pub fn runtime_commit(
        request_id: impl Into<String>,
        shard: ShardId,
        placement_epoch: u64,
        expected_commit_index: Option<u64>,
        commit: RuntimeCommit,
    ) -> ClusterResult<Self> {
        let command = Self {
            request_id: request_id.into(),
            shard,
            placement_epoch,
            expected_commit_index,
            operation: RrdRaftOperation::RuntimeCommit { commit },
        };
        command.validate()?;
        Ok(command)
    }

    pub fn validate(&self) -> ClusterResult<()> {
        if self.request_id.is_empty()
            || self.request_id.len() > 256
            || self.request_id.as_bytes().contains(&0)
            || self.placement_epoch == 0
        {
            return Err(ClusterError::Invalid(
                "raft command identity or placement epoch is invalid".into(),
            ));
        }
        self.operation.validate()?;
        if let RrdRaftOperation::PlacementTransition { placement } = &self.operation {
            if placement.shard != self.shard || placement.epoch != self.placement_epoch {
                return Err(ClusterError::Invalid(
                    "placement transition command and placement identity differ".into(),
                ));
            }
        }
        let encoded = serde_json::to_vec(&self.operation)
            .map_err(|error| ClusterError::Invalid(error.to_string()))?;
        if encoded.len() > MAX_COMMAND_BYTES {
            return Err(ClusterError::Invalid(format!(
                "raft command operation exceeds {MAX_COMMAND_BYTES} encoded bytes"
            )));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RrdRaftResponse {
    pub accepted: bool,
    pub duplicate: bool,
    pub term: u64,
    pub index: u64,
    pub state_digest: String,
    pub reason: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_outcome: Option<RuntimeCommitOutcome>,
}

openraft::declare_raft_types!(
    pub RrdRaftTypeConfig:
        D = RrdRaftCommand,
        R = RrdRaftResponse,
        NodeId = u64,
        Node = RrdRaftNode,
        SnapshotData = RrdSnapshotData,
);

pub type RrdRaftEntry = Entry<RrdRaftTypeConfig>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AdapterConfig {
    pub(super) format_version: u16,
    pub(super) shard: ShardId,
    pub(super) domain: AdapterDomain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum AdapterDomain {
    CanonicalState,
    LocalRaft,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AppliedRequest {
    pub(super) first_applied_index: u64,
    pub(super) command_digest: String,
    pub(super) response: RrdRaftResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct StateMachineData {
    pub(super) last_applied: Option<LogId<u64>>,
    pub(super) last_membership: StoredMembership<u64, RrdRaftNode>,
    #[serde(default)]
    pub(super) placement_epoch: Option<u64>,
    #[serde(default)]
    pub(super) placement_digest: Option<String>,
    #[serde(default)]
    pub(super) placement_membership_digest: Option<String>,
    #[serde(default)]
    pub(super) voter_membership_generation: u64,
    #[serde(default)]
    pub(super) placement_membership_generation: Option<u64>,
    #[serde(default)]
    pub(super) runtime_commit_count: u64,
    pub(super) state_digest: String,
    pub(super) requests: BTreeMap<String, AppliedRequest>,
}

impl Default for StateMachineData {
    fn default() -> Self {
        Self {
            last_applied: None,
            last_membership: StoredMembership::default(),
            placement_epoch: None,
            placement_digest: None,
            placement_membership_digest: None,
            voter_membership_generation: 0,
            placement_membership_generation: None,
            runtime_commit_count: 0,
            state_digest: sha256_hex(b"rrflow.raft.state.v1"),
            requests: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct StoredSnapshot {
    pub(super) meta: SnapshotMeta<u64, RrdRaftNode>,
    pub(super) object: ObjectReference,
}

pub(super) type SharedDatabase = Arc<Mutex<Database>>;

/// Non-owning evidence that both physical databases opened for one Raft node
/// have been released.
///
/// OpenRaft joins its core task during shutdown, but its state-machine worker
/// exits after the core drops the worker channel. Callers that may restart a
/// node in the same process must therefore wait for this boundary before
/// reopening the storage root.
#[derive(Clone)]
pub struct RrdRaftStorageLifecycle {
    pub(super) state_database: Weak<Mutex<Database>>,
    pub(super) local_database: Weak<Mutex<Database>>,
}

impl RrdRaftStorageLifecycle {
    /// Returns true only after every log-store and state-machine owner has
    /// released both native database writer locks.
    pub fn is_released(&self) -> bool {
        self.state_database.strong_count() == 0 && self.local_database.strong_count() == 0
    }
}

#[derive(Clone)]
pub struct RrdRaftLogStore {
    pub(super) local_database: SharedDatabase,
}

#[derive(Clone)]
pub struct RrdRaftStateMachine {
    pub(super) state_database: SharedDatabase,
    pub(super) local_database: SharedDatabase,
    pub(super) snapshot_objects: LocalObjectStore,
    pub(super) application_objects: LocalObjectStore,
    pub(super) snapshot_spool: AnchoredPath,
    pub(super) shard: ShardId,
}

pub struct RrdRaftStore;

impl RrdRaftStore {
    pub fn open(
        root: &Path,
        shard: ShardId,
    ) -> ClusterResult<(RrdRaftLogStore, RrdRaftStateMachine)> {
        let root = AnchoredPath::from_path(root)
            .map_err(|error| ClusterError::Unavailable(error.to_string()))?;
        let mut state_database = open_adapter_database(&root, AdapterDomain::CanonicalState)?;
        ensure_adapter_database(
            &mut state_database,
            KEY_STATE_CONFIG,
            AdapterConfig {
                format_version: ADAPTER_FORMAT_VERSION,
                shard,
                domain: AdapterDomain::CanonicalState,
            },
            Some(put_json(KEY_STATE, &StateMachineData::default())?),
        )?;

        let root = state_database.anchored_root().clone();
        let local_root = root.join(LOCAL_DATABASE_DIRECTORY);
        let mut local_database = open_adapter_database(&local_root, AdapterDomain::LocalRaft)?;
        ensure_adapter_database(
            &mut local_database,
            KEY_LOCAL_CONFIG,
            AdapterConfig {
                format_version: ADAPTER_FORMAT_VERSION,
                shard,
                domain: AdapterDomain::LocalRaft,
            },
            None,
        )?;
        let snapshot_objects = LocalObjectStore::open_anchored(
            &local_database
                .anchored_root()
                .join(SNAPSHOT_OBJECT_DIRECTORY),
        )
        .map_err(|error| ClusterError::Unavailable(error.to_string()))?;
        let application_objects =
            LocalObjectStore::open_anchored(&root.join(APPLICATION_OBJECT_DIRECTORY))
                .map_err(|error| ClusterError::Unavailable(error.to_string()))?;
        let snapshot_spool = local_database
            .anchored_root()
            .join(SNAPSHOT_SPOOL_DIRECTORY);
        clean_snapshot_spool(&snapshot_spool)?;

        let database = Arc::new(Mutex::new(state_database));
        let local_database = Arc::new(Mutex::new(local_database));
        Ok((
            RrdRaftLogStore {
                local_database: local_database.clone(),
            },
            RrdRaftStateMachine {
                state_database: database,
                local_database,
                snapshot_objects,
                application_objects,
                snapshot_spool,
                shard,
            },
        ))
    }
}
