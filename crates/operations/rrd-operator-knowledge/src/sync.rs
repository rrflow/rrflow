//! Sync work, receipts, writers and the reference writer.

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperatorSyncOperation {
    UpsertVector,
    DeleteVector,
}

/// Idempotent work derived from RRD's already-committed projection outbox.
/// Payload bytes remain caller-owned; the durable identity binds the exact
/// canonical mutation and payload digest that an adapter must apply.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperatorSyncWork {
    pub contract_version: u16,
    pub id: String,
    pub binding_digest: String,
    pub scope: ScopeId,
    pub source_cursor: u64,
    pub source_commit_id: String,
    pub source_commit_ordinal: u64,
    pub source_outbox_id: String,
    pub source_change_digest: String,
    pub payload_digest: String,
    pub operation: OperatorSyncOperation,
}

impl OperatorSyncWork {
    pub fn for_vector(
        binding: &OperatorKnowledgeBinding,
        source: &ProjectionWork,
        source_change_digest: impl Into<String>,
        payload_digest: impl Into<String>,
    ) -> Result<Self> {
        Self::for_vector_operation(
            binding,
            source,
            source_change_digest,
            payload_digest,
            OperatorSyncOperation::UpsertVector,
        )
    }

    pub fn for_vector_delete(
        binding: &OperatorKnowledgeBinding,
        source: &ProjectionWork,
        source_change_digest: impl Into<String>,
        payload_digest: impl Into<String>,
    ) -> Result<Self> {
        Self::for_vector_operation(
            binding,
            source,
            source_change_digest,
            payload_digest,
            OperatorSyncOperation::DeleteVector,
        )
    }

    fn for_vector_operation(
        binding: &OperatorKnowledgeBinding,
        source: &ProjectionWork,
        source_change_digest: impl Into<String>,
        payload_digest: impl Into<String>,
        operation: OperatorSyncOperation,
    ) -> Result<Self> {
        binding.validate()?;
        source.validate()?;
        if source.family != ProjectionFamily::Vector || source.scope != binding.scope {
            return invalid("operator sync source is not vector work for the bound scope");
        }
        let source_change_digest = source_change_digest.into();
        let payload_digest = payload_digest.into();
        validate_digest("operator sync source change", &source_change_digest)?;
        validate_digest("operator sync payload", &payload_digest)?;
        let mut work = Self {
            contract_version: OPERATOR_KNOWLEDGE_CONTRACT_VERSION,
            id: String::new(),
            binding_digest: binding.digest()?,
            scope: source.scope.clone(),
            source_cursor: source.source_cursor,
            source_commit_id: source.commit_id.clone(),
            source_commit_ordinal: source.commit_ordinal,
            source_outbox_id: source.id.to_string(),
            source_change_digest,
            payload_digest,
            operation,
        };
        work.id = work.expected_id()?;
        work.validate()?;
        Ok(work)
    }

    pub fn validate(&self) -> Result<()> {
        validate_version(self.contract_version)?;
        for (kind, value) in [
            ("operator sync identity", &self.id),
            ("operator sync binding", &self.binding_digest),
            ("operator sync commit", &self.source_commit_id),
            ("operator sync outbox", &self.source_outbox_id),
            ("operator sync source change", &self.source_change_digest),
            ("operator sync payload", &self.payload_digest),
        ] {
            validate_digest(kind, value)?;
        }
        if self.source_cursor == 0 || self.id != self.expected_id()? {
            return invalid("operator sync identity or source cursor is invalid");
        }
        Ok(())
    }

    fn expected_id(&self) -> Result<String> {
        content_digest(
            b"rrd-operator-knowledge-sync-work-v1\0",
            &(
                &self.binding_digest,
                &self.scope,
                self.source_cursor,
                &self.source_commit_id,
                self.source_commit_ordinal,
                &self.source_outbox_id,
                &self.source_change_digest,
                &self.payload_digest,
                self.operation,
            ),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperatorSyncReceipt {
    pub contract_version: u16,
    pub work_id: String,
    pub revision: OperatorSourceRevision,
    pub applied_now: bool,
    pub idempotent_replay: bool,
}

impl OperatorSyncReceipt {
    pub fn validate(&self) -> Result<()> {
        validate_version(self.contract_version)?;
        validate_digest("operator sync receipt work", &self.work_id)?;
        self.revision.validate()?;
        if self.applied_now == self.idempotent_replay {
            return invalid(
                "operator sync receipt must be either a new apply or an idempotent replay",
            );
        }
        Ok(())
    }
}

pub trait OperatorKnowledgeWriter {
    fn descriptor(&self) -> &OperatorAdapterDescriptor;

    fn apply(
        &mut self,
        binding: &OperatorKnowledgeBinding,
        work: &OperatorSyncWork,
        payload: &[u8],
    ) -> Result<OperatorSyncReceipt>;
}

pub fn execute_operator_sync<W: OperatorKnowledgeWriter>(
    writer: &mut W,
    binding: &OperatorKnowledgeBinding,
    work: &OperatorSyncWork,
    payload: &[u8],
) -> Result<OperatorSyncReceipt> {
    binding.validate()?;
    work.validate()?;
    let descriptor = writer.descriptor().clone();
    descriptor.validate()?;
    if descriptor.adapter != binding.adapter || work.binding_digest != binding.digest()? {
        return invalid("operator sync writer or work uses another project binding");
    }
    if work.scope != binding.scope || digest::sha256_hex(payload) != work.payload_digest {
        return invalid("operator sync payload or scope differs from its durable work identity");
    }
    let receipt = writer.apply(binding, work, payload)?;
    if writer.descriptor() != &descriptor {
        return invalid("operator sync writer changed its descriptor during execution");
    }
    receipt.validate()?;
    if receipt.work_id != work.id
        || receipt.revision.adapter != binding.adapter
        || receipt.revision.project_id != binding.project_id
        || receipt.revision.source_identity_digest != binding.source_identity_digest
    {
        return invalid("operator sync receipt differs from its work or project binding");
    }
    Ok(receipt)
}

/// Deterministic idempotency oracle for external writers. It models the
/// adapter-side unique work-id table required by a pgvector implementation.
pub struct ReferenceOperatorWriter {
    descriptor: OperatorAdapterDescriptor,
    binding_digest: String,
    revision: OperatorSourceRevision,
    applied: BTreeMap<String, OperatorSyncReceipt>,
    apply_count: u64,
}

impl ReferenceOperatorWriter {
    pub fn new(
        descriptor: OperatorAdapterDescriptor,
        binding: &OperatorKnowledgeBinding,
        revision: OperatorSourceRevision,
    ) -> Result<Self> {
        descriptor.validate()?;
        binding.validate()?;
        revision.validate()?;
        Ok(Self {
            descriptor,
            binding_digest: binding.digest()?,
            revision,
            applied: BTreeMap::new(),
            apply_count: 0,
        })
    }

    pub const fn apply_count(&self) -> u64 {
        self.apply_count
    }
}

impl OperatorKnowledgeWriter for ReferenceOperatorWriter {
    fn descriptor(&self) -> &OperatorAdapterDescriptor {
        &self.descriptor
    }

    fn apply(
        &mut self,
        binding: &OperatorKnowledgeBinding,
        work: &OperatorSyncWork,
        _payload: &[u8],
    ) -> Result<OperatorSyncReceipt> {
        if binding.digest()? != self.binding_digest {
            return invalid("reference operator writer is bound to another project");
        }
        if let Some(receipt) = self.applied.get(&work.id) {
            let mut replay = receipt.clone();
            replay.applied_now = false;
            replay.idempotent_replay = true;
            return Ok(replay);
        }
        self.apply_count =
            self.apply_count
                .checked_add(1)
                .ok_or_else(|| Error::InvalidRuntime {
                    reason: "reference operator apply count overflow".into(),
                })?;
        let receipt = OperatorSyncReceipt {
            contract_version: OPERATOR_KNOWLEDGE_CONTRACT_VERSION,
            work_id: work.id.clone(),
            revision: self.revision.clone(),
            applied_now: true,
            idempotent_replay: false,
        };
        self.applied.insert(work.id.clone(), receipt.clone());
        Ok(receipt)
    }
}
