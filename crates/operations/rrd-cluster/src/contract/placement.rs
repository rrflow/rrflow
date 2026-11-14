//! Replica roles and health, placement, read and write consistency, snapshot vectors, routes and replica transfer plans.

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReplicaRole {
    Voter,
    Learner,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReplicaHealth {
    Active,
    Suspect,
    Recovering,
    Lost,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplicaPlacement {
    pub node: NodeId,
    pub zone: ZoneId,
    pub role: ReplicaRole,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlacementPolicy {
    pub voter_count: u8,
    pub minimum_voter_zones: u8,
    pub maximum_voters_per_zone: u8,
}

impl PlacementPolicy {
    pub fn validate(&self) -> Result<()> {
        if self.voter_count < 3 || self.voter_count.is_multiple_of(2) {
            return Err(ClusterError::Invalid(
                "voter_count must be an odd number of at least three".into(),
            ));
        }
        if self.minimum_voter_zones < 2 || self.minimum_voter_zones > self.voter_count {
            return Err(ClusterError::Invalid(
                "minimum_voter_zones must be in 2..=voter_count".into(),
            ));
        }
        if self.maximum_voters_per_zone == 0 {
            return Err(ClusterError::Invalid(
                "maximum_voters_per_zone must be non-zero".into(),
            ));
        }
        Ok(())
    }

    pub fn quorum(&self) -> usize {
        usize::from(self.voter_count / 2 + 1)
    }

    pub fn tolerated_failures(&self) -> usize {
        usize::from((self.voter_count - 1) / 2)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShardPlacement {
    pub contract_version: u16,
    pub cluster: ClusterId,
    pub shard: ShardId,
    pub epoch: u64,
    pub policy: PlacementPolicy,
    pub replicas: Vec<ReplicaPlacement>,
}

impl ShardPlacement {
    pub fn validate(&self) -> Result<()> {
        if self.contract_version != CLUSTER_CONTRACT_VERSION || self.epoch == 0 {
            return Err(ClusterError::Invalid(
                "unsupported contract version or zero placement epoch".into(),
            ));
        }
        self.policy.validate()?;
        if self
            .replicas
            .windows(2)
            .any(|pair| pair[0].node >= pair[1].node)
        {
            return Err(ClusterError::Invalid(
                "replicas must be strictly ordered by unique node id".into(),
            ));
        }
        let voters: Vec<_> = self
            .replicas
            .iter()
            .filter(|replica| replica.role == ReplicaRole::Voter)
            .collect();
        if voters.len() != usize::from(self.policy.voter_count) {
            return Err(ClusterError::Invalid(format!(
                "placement has {} voters but policy requires {}",
                voters.len(),
                self.policy.voter_count
            )));
        }
        let zones: BTreeSet<_> = voters.iter().map(|replica| &replica.zone).collect();
        if zones.len() < usize::from(self.policy.minimum_voter_zones) {
            return Err(ClusterError::Invalid(
                "placement does not satisfy voter zone diversity".into(),
            ));
        }
        let mut per_zone = BTreeMap::<&ZoneId, usize>::new();
        for voter in voters {
            *per_zone.entry(&voter.zone).or_default() += 1;
        }
        if per_zone
            .values()
            .any(|count| *count > usize::from(self.policy.maximum_voters_per_zone))
        {
            return Err(ClusterError::Invalid(
                "placement exceeds maximum voters per zone".into(),
            ));
        }
        Ok(())
    }

    pub fn voters(&self) -> impl Iterator<Item = &ReplicaPlacement> {
        self.replicas
            .iter()
            .filter(|replica| replica.role == ReplicaRole::Voter)
    }

    pub fn digest(&self) -> Result<String> {
        self.validate()?;
        let bytes = serde_json::to_vec(self).map_err(|error| {
            ClusterError::Invalid(format!("placement encoding failed: {error}"))
        })?;
        Ok(sha256_hex(&bytes))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ReadConsistency {
    Linearizable,
    BoundedStale { maximum_index_lag: u64 },
    ExactSnapshot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WriteConsistency {
    QuorumDurable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShardReadStamp {
    pub term: u64,
    pub commit_index: u64,
    pub placement_epoch: u64,
    pub state_digest: String,
}

impl ShardReadStamp {
    pub fn validate(&self) -> Result<()> {
        if self.term == 0 || self.placement_epoch == 0 || !is_sha256(&self.state_digest) {
            return Err(ClusterError::Invalid(
                "shard stamp requires non-zero term/epoch and lowercase SHA-256 state digest"
                    .into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VectorRelation {
    Equal,
    Dominates,
    Dominated,
    Concurrent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotVector {
    pub contract_version: u16,
    pub scope: String,
    pub shards: BTreeMap<ShardId, ShardReadStamp>,
}

impl SnapshotVector {
    pub fn validate(&self) -> Result<()> {
        if self.contract_version != CLUSTER_CONTRACT_VERSION
            || self.scope.is_empty()
            || self.scope.len() > 256
            || self.shards.is_empty()
        {
            return Err(ClusterError::Invalid(
                "snapshot vector requires v1, a bounded scope, and at least one shard".into(),
            ));
        }
        for stamp in self.shards.values() {
            stamp.validate()?;
        }
        Ok(())
    }

    /// Partial-order comparison. `Concurrent` is intentionally preserved;
    /// RRD never fabricates a total cluster cursor.
    pub fn relation(&self, other: &Self) -> Result<VectorRelation> {
        self.validate()?;
        other.validate()?;
        if self.scope != other.scope || self.shards.keys().ne(other.shards.keys()) {
            return Err(ClusterError::Invalid(
                "snapshot vectors must cover the same scope and shard set".into(),
            ));
        }
        let mut saw_less = false;
        let mut saw_greater = false;
        for (shard, left) in &self.shards {
            let right = &other.shards[shard];
            if left.placement_epoch != right.placement_epoch {
                return Err(ClusterError::Invalid(
                    "snapshot vectors cross a placement epoch".into(),
                ));
            }
            match left.commit_index.cmp(&right.commit_index) {
                Ordering::Less => saw_less = true,
                Ordering::Greater => saw_greater = true,
                Ordering::Equal => {
                    if left.state_digest != right.state_digest || left.term != right.term {
                        return Err(ClusterError::Denied(
                            "equal shard cursors disagree on term or state digest".into(),
                        ));
                    }
                }
            }
        }
        Ok(match (saw_less, saw_greater) {
            (false, false) => VectorRelation::Equal,
            (false, true) => VectorRelation::Dominates,
            (true, false) => VectorRelation::Dominated,
            (true, true) => VectorRelation::Concurrent,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TransactionScope {
    SingleShard { shard: ShardId },
    CrossShard { shards: BTreeSet<ShardId> },
}

impl TransactionScope {
    pub fn enforce_m7(&self) -> Result<ShardId> {
        match self {
            Self::SingleShard { shard } => Ok(*shard),
            Self::CrossShard { .. } => Err(ClusterError::Denied(
                "cross-shard writes require durable intents and a verified commit protocol".into(),
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteEvidence {
    pub contract_version: u16,
    pub shard: ShardId,
    pub placement_epoch: u64,
    pub requested_consistency: ReadConsistency,
    pub selected: Vec<NodeId>,
    pub replica_health: BTreeMap<NodeId, ReplicaHealth>,
    pub observed: Option<ShardReadStamp>,
    pub allowed: bool,
    pub reason: String,
}

impl RouteEvidence {
    pub fn validate(&self) -> Result<()> {
        if self.contract_version != CLUSTER_CONTRACT_VERSION
            || self.placement_epoch == 0
            || self.reason.is_empty()
            || self.selected.windows(2).any(|pair| pair[0] >= pair[1])
        {
            return Err(ClusterError::Invalid("malformed route evidence".into()));
        }
        if self.allowed && (self.selected.is_empty() || self.observed.is_none()) {
            return Err(ClusterError::Invalid(
                "an allowed route requires selected replicas and an observed stamp".into(),
            ));
        }
        if !self.allowed && self.observed.is_some() {
            return Err(ClusterError::Invalid(
                "a denied route cannot claim an observed snapshot".into(),
            ));
        }
        if self.allowed
            && self
                .selected
                .iter()
                .any(|node| self.replica_health.get(node).copied() != Some(ReplicaHealth::Active))
        {
            return Err(ClusterError::Invalid(
                "an allowed route may select only explicitly active replicas".into(),
            ));
        }
        if let Some(stamp) = &self.observed {
            stamp.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplicaTransferPlan {
    pub contract_version: u16,
    pub shard: ShardId,
    pub placement_epoch: u64,
    pub source: NodeId,
    pub target: NodeId,
    pub grounded_snapshot: ShardReadStamp,
    pub wal_from_exclusive: u64,
    pub wal_through_inclusive: u64,
    pub artifact_digests: BTreeSet<String>,
}
