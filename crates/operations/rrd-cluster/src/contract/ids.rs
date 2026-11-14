//! Cluster contract version, Raft timing, errors and typed identifiers.

use super::*;

pub const CLUSTER_CONTRACT_VERSION: u16 = 1;
pub const METADATA_SHARD_ID: ShardId = ShardId(0);
pub const ARTIFACT_TRANSFER_CHUNK_MAX_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RrdRaftTimingPolicy {
    pub heartbeat_interval_millis: u64,
    pub election_timeout_min_millis: u64,
    pub election_timeout_max_millis: u64,
}

impl Default for RrdRaftTimingPolicy {
    fn default() -> Self {
        Self {
            heartbeat_interval_millis: 250,
            election_timeout_min_millis: 1_000,
            election_timeout_max_millis: 2_000,
        }
    }
}

impl RrdRaftTimingPolicy {
    pub fn validate(&self) -> Result<()> {
        if self.heartbeat_interval_millis == 0
            || self.heartbeat_interval_millis > 60_000
            || self.election_timeout_min_millis <= self.heartbeat_interval_millis
            || self.election_timeout_max_millis <= self.election_timeout_min_millis
            || self.election_timeout_max_millis > 300_000
        {
            return Err(ClusterError::Invalid(
                "Raft timing policy is outside its bounded contract".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClusterError {
    Invalid(String),
    Denied(String),
    Unavailable(String),
    NotFound(String),
}

impl fmt::Display for ClusterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(message) => write!(f, "invalid cluster contract: {message}"),
            Self::Denied(message) => write!(f, "cluster operation denied: {message}"),
            Self::Unavailable(message) => write!(f, "cluster unavailable: {message}"),
            Self::NotFound(message) => write!(f, "cluster object not found: {message}"),
        }
    }
}

impl std::error::Error for ClusterError {}

pub type Result<T> = std::result::Result<T, ClusterError>;

macro_rules! string_id {
    ($name:ident, $label:literal) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(try_from = "String", into = "String")]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self> {
                let value = value.into();
                if let Some(problem) = rrd_core::ident::canonical_id_problem(&value) {
                    return Err(ClusterError::Invalid(format!(
                        "{} {value:?} is invalid: {problem}",
                        $label
                    )));
                }
                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl TryFrom<String> for $name {
            type Error = ClusterError;

            fn try_from(value: String) -> Result<Self> {
                Self::new(value)
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> Self {
                value.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

string_id!(ClusterId, "cluster id");
string_id!(NodeId, "node id");
string_id!(ZoneId, "zone id");
string_id!(RegionId, "region id");
string_id!(TenantId, "tenant id");
string_id!(LogicalTableId, "logical table id");

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ShardId(pub u64);
