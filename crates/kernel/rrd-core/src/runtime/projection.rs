//! Projection state, families, work and stamps.

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectionState {
    Building,
    Ready,
    Quarantined,
    Retiring,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectionFamily {
    Scalar,
    Graph,
    Text,
    Vector,
    TimeSeries,
    Geo,
    Object,
}

/// Durable work emitted in the same transaction as its canonical source.
/// Consumers acknowledge work in a later transaction; a projection is usable
/// only when its published stamp covers the required source cursor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectionWork {
    pub contract_version: u16,
    pub id: OutboxId,
    pub scope: ScopeId,
    pub source_cursor: u64,
    pub commit_id: String,
    pub commit_ordinal: u64,
    pub family: ProjectionFamily,
}

impl ProjectionWork {
    pub fn for_change(
        scope: ScopeId,
        source_cursor: u64,
        commit_id: impl Into<String>,
        commit_ordinal: u64,
        family: ProjectionFamily,
    ) -> Result<Self> {
        let commit_id = commit_id.into();
        validate_digest("outbox commit", &commit_id)?;
        if source_cursor == 0 {
            return Err(Error::InvalidRuntime {
                reason: "outbox source cursor must be greater than zero".into(),
            });
        }
        let mut bytes = b"rrflow-projection-work-v1\0".to_vec();
        text(&mut bytes, scope.as_str());
        bytes.extend_from_slice(&source_cursor.to_be_bytes());
        text(&mut bytes, &commit_id);
        bytes.extend_from_slice(&commit_ordinal.to_be_bytes());
        bytes.push(projection_family_tag(family));
        let id = OutboxId::new(digest::sha256_hex(&bytes))?;
        Ok(Self {
            contract_version: DATA_RUNTIME_CONTRACT_VERSION,
            id,
            scope,
            source_cursor,
            commit_id,
            commit_ordinal,
            family,
        })
    }

    pub fn validate(&self) -> Result<()> {
        let rebuilt = Self::for_change(
            self.scope.clone(),
            self.source_cursor,
            self.commit_id.clone(),
            self.commit_ordinal,
            self.family,
        )?;
        if self.contract_version != DATA_RUNTIME_CONTRACT_VERSION || rebuilt.id != self.id {
            return Err(Error::InvalidRuntime {
                reason: "outbox identity or contract version is invalid".into(),
            });
        }
        Ok(())
    }
}

pub(super) fn projection_family_tag(family: ProjectionFamily) -> u8 {
    match family {
        ProjectionFamily::Scalar => 0,
        ProjectionFamily::Graph => 1,
        ProjectionFamily::Text => 2,
        ProjectionFamily::Vector => 3,
        ProjectionFamily::TimeSeries => 4,
        ProjectionFamily::Geo => 5,
        ProjectionFamily::Object => 6,
    }
}

/// Projection work required by a canonical mutation. Schema and claim changes
/// are consumed directly by their existing authoritative paths.
pub fn projection_family(mutation: &RuntimeMutation) -> Option<ProjectionFamily> {
    match mutation {
        RuntimeMutation::Record { .. } => Some(ProjectionFamily::Scalar),
        RuntimeMutation::Relation { .. } | RuntimeMutation::Event { .. } => {
            Some(ProjectionFamily::Graph)
        }
        RuntimeMutation::Vector { .. } => Some(ProjectionFamily::Vector),
        RuntimeMutation::SeriesSample { .. } => Some(ProjectionFamily::TimeSeries),
        RuntimeMutation::Geo { .. } => Some(ProjectionFamily::Geo),
        RuntimeMutation::Object { .. } => Some(ProjectionFamily::Object),
        RuntimeMutation::Retire { retirement } => match retirement.model {
            crate::RuntimeLogicalModel::Document
            | crate::RuntimeLogicalModel::Relational
            | crate::RuntimeLogicalModel::GraphNode
            | crate::RuntimeLogicalModel::KeyValue
            | crate::RuntimeLogicalModel::ReasoningRecord
            | crate::RuntimeLogicalModel::LifecycleRecord => Some(ProjectionFamily::Scalar),
            crate::RuntimeLogicalModel::GraphRelation
            | crate::RuntimeLogicalModel::Event
            | crate::RuntimeLogicalModel::ReasoningEvent
            | crate::RuntimeLogicalModel::LifecycleEvent => Some(ProjectionFamily::Graph),
            crate::RuntimeLogicalModel::Vector => Some(ProjectionFamily::Vector),
            crate::RuntimeLogicalModel::TimeSeries => Some(ProjectionFamily::TimeSeries),
            crate::RuntimeLogicalModel::Geo => Some(ProjectionFamily::Geo),
            crate::RuntimeLogicalModel::Object => Some(ProjectionFamily::Object),
            crate::RuntimeLogicalModel::ReasoningClaim => None,
        },
        RuntimeMutation::Claim { .. } | RuntimeMutation::Schema { .. } => None,
    }
}

/// Common freshness and provenance identity for every derived index family.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectionStamp {
    pub contract_version: u16,
    pub id: ProjectionId,
    pub generation: u64,
    pub source_cursor: u64,
    pub config_digest: String,
    pub artifact_digest: String,
    pub state: ProjectionState,
}

impl ProjectionStamp {
    pub fn validate(&self) -> Result<()> {
        if self.contract_version != DATA_RUNTIME_CONTRACT_VERSION || self.generation == 0 {
            return Err(Error::InvalidRuntime {
                reason: "projection contract version and generation must be valid".into(),
            });
        }
        validate_digest("projection configuration", &self.config_digest)?;
        validate_digest("projection artifact", &self.artifact_digest)
    }
}
