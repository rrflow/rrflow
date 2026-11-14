//! Audit events, records, heads and pages.

use super::*;

pub use rrd_contract::{AuditDecision, AuditPhase};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuditEvent {
    pub audit_id: CanonicalId,
    pub at_unix_ms: u64,
    pub principal_id: Option<CanonicalId>,
    pub action: Action,
    pub resource: ResourcePath,
    pub request_id: String,
    pub operation_id: String,
    pub phase: AuditPhase,
    pub decision: AuditDecision,
    pub status_code: u16,
    pub request_sha256: String,
    pub response_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuditRecord {
    pub audit_id: CanonicalId,
    pub at_unix_ms: u64,
    pub principal_id: Option<CanonicalId>,
    pub action: Action,
    pub resource: ResourcePath,
    pub request_id: String,
    pub operation_id: String,
    pub phase: AuditPhase,
    pub decision: AuditDecision,
    pub status_code: u16,
    pub request_sha256: String,
    pub response_sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_audit_sha256: Option<String>,
    pub audit_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AuditHead {
    pub(super) audit_id: CanonicalId,
    pub(super) audit_sha256: String,
    pub(super) record_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditJournalPage {
    pub through_sequence: u64,
    pub chain_anchor_sha256: Option<String>,
    pub chain_head_sha256: Option<String>,
    pub records: Vec<(u64, AuditRecord)>,
}

impl AuditEvent {
    pub fn validate(&self) -> Result<()> {
        self.resource
            .validate()
            .map_err(|error| Error::Invalid(error.to_string()))?;
        validate_sha256(&self.request_sha256)?;
        validate_sha256(&self.response_sha256)?;
        if self.at_unix_ms == 0
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
            return Err(Error::Invalid("audit coordinates are invalid".into()));
        }
        Ok(())
    }
}

impl AuditRecord {
    pub(super) fn seal(event: AuditEvent, previous_audit_sha256: Option<String>) -> Result<Self> {
        event.validate()?;
        if let Some(previous) = &previous_audit_sha256 {
            validate_sha256(previous)?;
        }
        let mut record = Self {
            audit_id: event.audit_id,
            at_unix_ms: event.at_unix_ms,
            principal_id: event.principal_id,
            action: event.action,
            resource: event.resource,
            request_id: event.request_id,
            operation_id: event.operation_id,
            phase: event.phase,
            decision: event.decision,
            status_code: event.status_code,
            request_sha256: event.request_sha256,
            response_sha256: event.response_sha256,
            previous_audit_sha256,
            audit_sha256: String::new(),
        };
        record.audit_sha256 = record.expected_sha256()?;
        Ok(record)
    }

    pub fn validate(&self) -> Result<()> {
        self.as_event().validate()?;
        if let Some(previous) = &self.previous_audit_sha256 {
            validate_sha256(previous)?;
        }
        validate_sha256(&self.audit_sha256)?;
        if self.expected_sha256()? != self.audit_sha256 {
            return Err(Error::Invalid("audit record hash is invalid".into()));
        }
        Ok(())
    }

    pub(super) fn as_event(&self) -> AuditEvent {
        AuditEvent {
            audit_id: self.audit_id.clone(),
            at_unix_ms: self.at_unix_ms,
            principal_id: self.principal_id.clone(),
            action: self.action,
            resource: self.resource.clone(),
            request_id: self.request_id.clone(),
            operation_id: self.operation_id.clone(),
            phase: self.phase,
            decision: self.decision,
            status_code: self.status_code,
            request_sha256: self.request_sha256.clone(),
            response_sha256: self.response_sha256.clone(),
        }
    }

    fn expected_sha256(&self) -> Result<String> {
        Ok(digest::sha256_hex(
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
            .map_err(json_error)?,
        ))
    }
}
