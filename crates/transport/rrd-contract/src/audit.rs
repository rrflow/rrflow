//! Security actions and the audit chain.

use crate::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum SecurityAction {
    ServiceInspect,
    UnknownRequest,
    SessionCreate,
    SessionRenew,
    SessionClose,
    QueryExecute,
    QueryLivePoll,
    QueryIndexEnsure,
    QueryIndexList,
    TransactionBegin,
    TransactionPreview,
    TransactionCommit,
    TransactionAbort,
    ChangefeedRead,
    ChangefeedFollow,
    WebSocketConnect,
    SubscriptionOpen,
    SubscriptionConnect,
    SubscriptionAck,
    SubscriptionClose,
    VectorCollectionEnsure,
    VectorCollectionList,
    VectorCollectionDelete,
    VectorPayloadIndexEnsure,
    VectorPayloadIndexList,
    VectorPayloadIndexDelete,
    VectorPointRetrieve,
    VectorPointScroll,
    VectorSearch,
    EmbeddingModelList,
    EmbeddingGenerate,
    EmbeddingSearch,
    BackupCreate,
    BackupList,
    RestoreCreate,
    EngramRead,
    EngramAdmin,
    AuditRead,
    AuditExport,
    FunctionCatalogueRead,
    FunctionCatalogueWrite,
    FunctionExecute,
    DiagnosticsRead,
    FlightLaunch,
    FlightRead,
    TaskWrite,
    TaskRead,
    SecurityAdmin,
    MemoryContextRead,
    ReasoningRead,
    ReasoningWrite,
    EndpointPropose,
    WorkerExecute,
    ProjectSourceRead,
    ObservationPrepare,
    ObservationDeliver,
    ObservationRead,
    KnowledgeTreeRead,
    KnowledgeTreeWrite,
}

impl SecurityAction {
    /// Canonical policy-action catalogue. Installation profiles and other
    /// projections consume this list instead of maintaining parallel copies.
    pub const ALL: [Self; 59] = [
        Self::ServiceInspect,
        Self::UnknownRequest,
        Self::SessionCreate,
        Self::SessionRenew,
        Self::SessionClose,
        Self::QueryExecute,
        Self::QueryLivePoll,
        Self::QueryIndexEnsure,
        Self::QueryIndexList,
        Self::TransactionBegin,
        Self::TransactionPreview,
        Self::TransactionCommit,
        Self::TransactionAbort,
        Self::ChangefeedRead,
        Self::ChangefeedFollow,
        Self::WebSocketConnect,
        Self::SubscriptionOpen,
        Self::SubscriptionConnect,
        Self::SubscriptionAck,
        Self::SubscriptionClose,
        Self::VectorCollectionEnsure,
        Self::VectorCollectionList,
        Self::VectorCollectionDelete,
        Self::VectorPayloadIndexEnsure,
        Self::VectorPayloadIndexList,
        Self::VectorPayloadIndexDelete,
        Self::VectorPointRetrieve,
        Self::VectorPointScroll,
        Self::VectorSearch,
        Self::EmbeddingModelList,
        Self::EmbeddingGenerate,
        Self::EmbeddingSearch,
        Self::BackupCreate,
        Self::BackupList,
        Self::RestoreCreate,
        Self::EngramRead,
        Self::EngramAdmin,
        Self::AuditRead,
        Self::AuditExport,
        Self::FunctionCatalogueRead,
        Self::FunctionCatalogueWrite,
        Self::FunctionExecute,
        Self::DiagnosticsRead,
        Self::FlightLaunch,
        Self::FlightRead,
        Self::TaskWrite,
        Self::TaskRead,
        Self::SecurityAdmin,
        Self::MemoryContextRead,
        Self::ReasoningRead,
        Self::ReasoningWrite,
        Self::EndpointPropose,
        Self::WorkerExecute,
        Self::ProjectSourceRead,
        Self::ObservationPrepare,
        Self::ObservationDeliver,
        Self::ObservationRead,
        Self::KnowledgeTreeRead,
        Self::KnowledgeTreeWrite,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AuditPhase {
    Authorized,
    Completed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AuditDecision {
    Allowed,
    Denied,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReadAudit {
    pub after_sequence: u64,
    pub limit: u16,
}

impl ReadAudit {
    pub fn validate(&self) -> Result<()> {
        if self.limit == 0 || self.limit > 1_024 {
            return invalid("audit page limit must be in 1..=1024");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AuditRecordSnapshot {
    pub sequence: u64,
    pub audit_id: CanonicalId,
    pub at_unix_ms: u64,
    pub principal_id: Option<CanonicalId>,
    pub action: SecurityAction,
    pub resource: ResourcePath,
    pub request_id: String,
    pub operation_id: String,
    pub phase: AuditPhase,
    pub decision: AuditDecision,
    pub status_code: u16,
    pub request_sha256: String,
    pub response_sha256: String,
    pub previous_audit_sha256: Option<String>,
    pub audit_sha256: String,
}

impl AuditRecordSnapshot {
    pub fn validate(&self) -> Result<()> {
        self.resource.validate()?;
        validate_sha256(&self.request_sha256, "audit request_sha256")?;
        validate_sha256(&self.response_sha256, "audit response_sha256")?;
        if let Some(previous) = &self.previous_audit_sha256 {
            validate_sha256(previous, "previous audit digest")?;
        }
        validate_sha256(&self.audit_sha256, "audit digest")?;
        if self.sequence == 0
            || self.at_unix_ms == 0
            || self.request_id.is_empty()
            || self.operation_id.is_empty()
            || self.request_id.len() > 256
            || self.operation_id.len() > 256
            || !self.request_id.is_ascii()
            || !self.operation_id.is_ascii()
            || !(100..=599).contains(&self.status_code)
            || (self.phase == AuditPhase::Authorized
                && (self.decision != AuditDecision::Allowed || self.status_code != 100))
            || (self.phase == AuditPhase::Completed && self.status_code < 200)
        {
            return invalid("audit record coordinates are invalid");
        }
        let expected = sha256_bytes(
            &serde_json::to_vec(&(
                &self.audit_id,
                self.at_unix_ms,
                &self.principal_id,
                self.action,
                &self.resource,
                &self.request_id,
                &self.operation_id,
                self.phase,
                self.decision,
                self.status_code,
                &self.request_sha256,
                &self.response_sha256,
                &self.previous_audit_sha256,
            ))
            .map_err(|error| ContractError(error.to_string()))?,
        );
        if expected != self.audit_sha256 {
            return invalid("audit record digest is invalid");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AuditPage {
    pub requested_after_sequence: u64,
    pub through_sequence: u64,
    pub chain_anchor_sha256: Option<String>,
    pub chain_head_sha256: Option<String>,
    pub records: Vec<AuditRecordSnapshot>,
}

impl AuditPage {
    pub fn validate(&self) -> Result<()> {
        if self.through_sequence < self.requested_after_sequence || self.records.len() > 1_024 {
            return invalid("audit page coordinates are invalid");
        }
        validate_audit_chain(
            &self.records,
            self.chain_anchor_sha256.as_deref(),
            self.chain_head_sha256.as_deref(),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExportAudit {
    pub after_sequence: u64,
    pub limit: u16,
}

impl ExportAudit {
    pub fn validate(&self) -> Result<()> {
        ReadAudit {
            after_sequence: self.after_sequence,
            limit: self.limit,
        }
        .validate()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AuditExport {
    pub requested_after_sequence: u64,
    pub through_sequence: u64,
    pub chain_anchor_sha256: Option<String>,
    pub chain_head_sha256: Option<String>,
    pub record_count: u16,
    pub media_type: String,
    pub content_sha256: String,
    pub json_lines: String,
}

impl AuditExport {
    pub fn validate(&self) -> Result<()> {
        if self.through_sequence < self.requested_after_sequence
            || self.record_count > 1_024
            || self.media_type != "application/x-ndjson; profile=rrd-audit-v1"
            || self.json_lines.len() > 8 * 1024 * 1024
        {
            return invalid("audit export coordinates are invalid");
        }
        validate_sha256(&self.content_sha256, "audit export content_sha256")?;
        for digest in self
            .chain_anchor_sha256
            .iter()
            .chain(&self.chain_head_sha256)
        {
            validate_sha256(digest, "audit export chain digest")?;
        }
        if sha256_bytes(self.json_lines.as_bytes()) != self.content_sha256 {
            return invalid("audit export content digest does not match JSON Lines");
        }
        if (!self.json_lines.is_empty() && !self.json_lines.ends_with('\n'))
            || (self.json_lines.is_empty() && self.record_count != 0)
        {
            return invalid("audit export JSON Lines framing is invalid");
        }
        let records = self
            .json_lines
            .lines()
            .map(|line| {
                serde_json::from_str::<AuditRecordSnapshot>(line)
                    .map_err(|error| ContractError(error.to_string()))
            })
            .collect::<Result<Vec<_>>>()?;
        if records.len() != usize::from(self.record_count) {
            return invalid("audit export record count differs from JSON Lines");
        }
        validate_audit_chain(
            &records,
            self.chain_anchor_sha256.as_deref(),
            self.chain_head_sha256.as_deref(),
        )?;
        Ok(())
    }
}

pub(crate) fn validate_audit_chain(
    records: &[AuditRecordSnapshot],
    anchor: Option<&str>,
    head: Option<&str>,
) -> Result<()> {
    if records.is_empty() {
        if anchor.is_some() || head.is_some() {
            return invalid("empty audit range cannot advertise chain coordinates");
        }
        return Ok(());
    }
    if records[0].previous_audit_sha256.as_deref() != anchor
        || records.last().map(|record| record.audit_sha256.as_str()) != head
    {
        return invalid("audit range chain coordinates are invalid");
    }
    let mut previous = anchor;
    for record in records {
        record.validate()?;
        if record.previous_audit_sha256.as_deref() != previous {
            return invalid("audit record lineage is discontinuous");
        }
        previous = Some(record.audit_sha256.as_str());
    }
    Ok(())
}
