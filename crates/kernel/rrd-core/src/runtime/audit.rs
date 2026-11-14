//! Audit decisions and envelopes.

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditDecision {
    Allow,
    Deny,
}

/// Portable JSON audit envelope. Persistence and archival policy belong to
/// `RRD storage coordinator`; this type freezes the information every adapter must preserve.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditEnvelope {
    pub contract_version: u16,
    pub request_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_request_id: Option<String>,
    pub at: Millis,
    pub actor: String,
    pub scope: ScopeId,
    pub operation: String,
    pub resource: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub read: Option<ReadStamp>,
    pub decision: AuditDecision,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outcome_cursor: Option<u64>,
    pub duration_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_digest: Option<String>,
    pub digest: String,
}

impl AuditEnvelope {
    pub fn accepted_commit(
        commit: &RuntimeCommit,
        commit_id: &str,
        outcome_cursor: u64,
        previous_digest: Option<String>,
    ) -> Result<Self> {
        Self::accepted_commit_at_read(commit, None, commit_id, outcome_cursor, previous_digest)
    }

    pub fn accepted_commit_at_read(
        commit: &RuntimeCommit,
        read: Option<&ReadStamp>,
        commit_id: &str,
        outcome_cursor: u64,
        previous_digest: Option<String>,
    ) -> Result<Self> {
        Self {
            contract_version: DATA_RUNTIME_CONTRACT_VERSION,
            request_id: commit_id.to_owned(),
            parent_request_id: None,
            at: commit.at,
            actor: commit.actor.clone(),
            scope: commit.scope.clone(),
            operation: "runtime.commit".into(),
            resource: format!("transaction:{commit_id}"),
            read: read.cloned(),
            decision: AuditDecision::Allow,
            outcome_cursor: Some(outcome_cursor),
            duration_ms: 0,
            previous_digest,
            digest: String::new(),
        }
        .seal()
    }

    pub fn seal(mut self) -> Result<Self> {
        self.validate_components()?;
        self.digest = digest::sha256_hex(&self.bytes_without_digest());
        Ok(self)
    }

    pub fn validate(&self) -> Result<()> {
        self.validate_components()?;
        validate_digest("audit envelope", &self.digest)?;
        if self.digest != digest::sha256_hex(&self.bytes_without_digest()) {
            return Err(Error::InvalidRuntime {
                reason: "audit envelope digest does not match its fields".into(),
            });
        }
        Ok(())
    }

    fn validate_components(&self) -> Result<()> {
        if self.contract_version != DATA_RUNTIME_CONTRACT_VERSION {
            return Err(Error::InvalidRuntime {
                reason: format!(
                    "unsupported audit contract version {}",
                    self.contract_version
                ),
            });
        }
        for (kind, value) in [
            ("audit request id", self.request_id.as_str()),
            ("audit actor", self.actor.as_str()),
            ("audit operation", self.operation.as_str()),
            ("audit resource", self.resource.as_str()),
        ] {
            validate_text(kind, value)?;
        }
        if let Some(parent) = &self.parent_request_id {
            validate_text("audit parent request id", parent)?;
        }
        if let Some(read) = &self.read {
            read.validate()?;
            if read.scope != self.scope {
                return Err(Error::InvalidRuntime {
                    reason: "audit read stamp scope differs from audit scope".into(),
                });
            }
        }
        if let Some(previous) = &self.previous_digest {
            validate_digest("previous audit envelope", previous)?;
        }
        Ok(())
    }

    fn bytes_without_digest(&self) -> Vec<u8> {
        let mut out = b"rrflow-audit-envelope-v1\0".to_vec();
        out.extend_from_slice(&self.contract_version.to_be_bytes());
        text(&mut out, &self.request_id);
        optional_text(&mut out, self.parent_request_id.as_deref());
        out.extend_from_slice(&self.at.to_be_bytes());
        text(&mut out, &self.actor);
        text(&mut out, self.scope.as_str());
        text(&mut out, &self.operation);
        text(&mut out, &self.resource);
        out.push(u8::from(self.read.is_some()));
        if let Some(read) = &self.read {
            let bytes = read.canonical_bytes();
            out.extend_from_slice(&(bytes.len() as u64).to_be_bytes());
            out.extend_from_slice(&bytes);
        }
        out.push(match self.decision {
            AuditDecision::Allow => 1,
            AuditDecision::Deny => 2,
        });
        encode_optional_u64(&mut out, self.outcome_cursor);
        out.extend_from_slice(&self.duration_ms.to_be_bytes());
        optional_text(&mut out, self.previous_digest.as_deref());
        out
    }
}
