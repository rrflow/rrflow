//! Resharding plans.

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReshardState {
    Planned,
    Copying,
    CaughtUp,
    Cutover,
    Retired,
}

/// A reshard cannot cut over by wall clock. The metadata shard commits this
/// plan, and every source must be represented in the exact cutover vector.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReshardPlan {
    pub contract_version: u16,
    pub operation_id: String,
    pub metadata_index: u64,
    pub source_shards: BTreeSet<ShardId>,
    pub targets: Vec<ShardPlacement>,
    pub cutover: SnapshotVector,
    pub state: ReshardState,
}

impl ReshardPlan {
    pub fn validate(&self) -> Result<()> {
        if self.contract_version != CLUSTER_CONTRACT_VERSION
            || self.operation_id.is_empty()
            || self.operation_id.len() > 128
            || self.metadata_index == 0
            || self.source_shards.is_empty()
            || self.targets.is_empty()
        {
            return Err(ClusterError::Invalid("malformed reshard plan".into()));
        }
        self.cutover.validate()?;
        let cutover_shards: BTreeSet<_> = self.cutover.shards.keys().copied().collect();
        if cutover_shards != self.source_shards {
            return Err(ClusterError::Invalid(
                "reshard cutover vector must cover exactly the source shards".into(),
            ));
        }
        let source_epoch = self
            .cutover
            .shards
            .values()
            .map(|stamp| stamp.placement_epoch)
            .max()
            .expect("validated snapshot vector is non-empty");
        let mut targets = BTreeSet::new();
        for target in &self.targets {
            target.validate()?;
            if target.epoch <= source_epoch
                || !targets.insert(target.shard)
                || self.source_shards.contains(&target.shard)
            {
                return Err(ClusterError::Invalid(
                    "reshard targets require a newer epoch and identities unique from sources"
                        .into(),
                ));
            }
        }
        Ok(())
    }
}

impl ReplicaTransferPlan {
    pub fn validate(&self) -> Result<()> {
        self.grounded_snapshot.validate()?;
        if self.contract_version != CLUSTER_CONTRACT_VERSION
            || self.placement_epoch != self.grounded_snapshot.placement_epoch
            || self.source == self.target
            || self.wal_from_exclusive != self.grounded_snapshot.commit_index
            || self.wal_through_inclusive < self.wal_from_exclusive
            || self
                .artifact_digests
                .iter()
                .any(|digest| !is_sha256(digest))
        {
            return Err(ClusterError::Invalid(
                "invalid snapshot-plus-WAL replica transfer plan".into(),
            ));
        }
        Ok(())
    }
}

pub(super) fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
