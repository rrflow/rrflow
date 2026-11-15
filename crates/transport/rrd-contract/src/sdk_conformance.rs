use crate::{
    endpoint_catalogue, transaction_operation_sha256, AbortTransaction,
    ActivateVectorQuantizationArtifact, BeginTransaction, BuildVectorQuantizationArtifact,
    CanonicalId, CloseSession, CommitTransaction, ContractError, CreateInstanceBackup,
    CreateSession, DeleteVectorPayloadIndex, EnsureVectorCollection, EnsureVectorIndex,
    EnsureVectorPayloadIndex, ErrorCode, ExecuteQuery, ListInstanceBackups, PreviewTransaction,
    ReadChangefeed, ReadColdHistory, ReadEngram, RenewSession, Result, SearchVectors,
    VectorQuantizationMethod, VectorSearchMode, PROTOCOL, PROTOCOL_VERSION,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const SDK_CONFORMANCE_FORMAT_VERSION: u16 = 1;

const REQUIRED_DOMAINS: [&str; 14] = [
    "auth",
    "backup",
    "cancellation",
    "cold_history",
    "crud",
    "engram",
    "live_feeds",
    "query",
    "retries",
    "sessions",
    "transactions",
    "typed_errors",
    "vectors",
    "versions",
];

/// One semantic corpus consumed unchanged by every supported language client.
///
/// Transport fault endpoints and ephemeral session/transaction identities are
/// supplied by the harness. Every request payload and observable expectation
/// remains checked in here so a language binding cannot silently qualify a
/// smaller protocol.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SdkConformanceCorpus {
    pub format_version: u16,
    pub protocol: String,
    pub protocol_version: u16,
    pub incompatible_protocol_version: u16,
    pub required_domains: BTreeSet<CanonicalId>,
    pub identity: SdkConformanceIdentity,
    pub session: SdkConformanceSession,
    pub transaction: SdkConformanceTransaction,
    pub query: ExecuteQuery,
    pub vector: SdkConformanceVector,
    pub changefeed: SdkConformanceChangefeed,
    pub cold_history: SdkConformanceColdHistory,
    pub backup: SdkConformanceBackup,
    pub engram: ReadEngram,
    pub expected: SdkConformanceExpected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SdkConformanceIdentity {
    pub instance: CanonicalId,
    pub engram: CanonicalId,
    pub principal: CanonicalId,
    pub api_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SdkConformanceSession {
    pub create: CreateSession,
    pub renew: RenewSession,
    pub close: CloseSession,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SdkConformanceTransaction {
    pub preview_begin: BeginTransaction,
    pub preview: PreviewTransaction,
    pub abort: AbortTransaction,
    pub commit_begin: BeginTransaction,
    pub commit_deadline_timeout_ms: u64,
    pub commit: CommitTransaction,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SdkConformanceVector {
    pub ensure: EnsureVectorCollection,
    pub search: SearchVectors,
    pub quantization: SdkConformanceQuantization,
    pub index: SdkConformanceVectorIndex,
}

/// The graph index of the corpus's second named vector: declare the payload
/// index it filters on, build it, search through it, and delete an unrelated
/// payload index. It has its own named vector, so the TurboQuant search above
/// keeps its access path.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SdkConformanceVectorIndex {
    pub payload_index: EnsureVectorPayloadIndex,
    pub ensure: EnsureVectorIndex,
    pub search: SearchVectors,
    pub spare_payload_index: EnsureVectorPayloadIndex,
    pub delete_payload_index: DeleteVectorPayloadIndex,
}

/// The quantization lifecycle on the corpus's vector: build a TurboQuant
/// generation, activate it, and require it. `activate` names the artifact the
/// build must return; the runners share one engram, so each one activates
/// the generation its own build returned.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SdkConformanceQuantization {
    pub build: BuildVectorQuantizationArtifact,
    pub activate: ActivateVectorQuantizationArtifact,
    pub search: SearchVectors,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SdkConformanceChangefeed {
    pub read: ReadChangefeed,
    pub follow_wait_timeout_ms: u64,
    pub cancellation_wait_timeout_ms: u64,
    pub cancel_after_ms: u64,
}

/// The cold-history read every SDK client makes. The harness installs with
/// the product's default configuration, where cold history is on, and
/// publishes one generation of its seeded history before any client runs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SdkConformanceColdHistory {
    pub read: ReadColdHistory,
    /// The actor the harness seeds history as; a generation must hold it.
    pub expected_actor: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SdkConformanceBackup {
    pub create: CreateInstanceBackup,
    pub list: ListInstanceBackups,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SdkConformanceExpected {
    pub endpoint_count: u16,
    pub query_identity: String,
    pub vector_reference: String,
    pub quantized_access_path: CanonicalId,
    pub indexed_access_path: CanonicalId,
    pub indexed_reference: String,
    pub engram_revision: u64,
    pub typed_error: ErrorCode,
    pub retry_attempts: u8,
}

impl SdkConformanceCorpus {
    pub fn validate(&self) -> Result<()> {
        if self.format_version != SDK_CONFORMANCE_FORMAT_VERSION {
            return invalid(format!(
                "SDK conformance format must be {SDK_CONFORMANCE_FORMAT_VERSION}"
            ));
        }
        if self.protocol != PROTOCOL || self.protocol_version != PROTOCOL_VERSION {
            return invalid("SDK conformance protocol must match the public RRD contract");
        }
        if self.incompatible_protocol_version == PROTOCOL_VERSION {
            return invalid("SDK conformance incompatible version must actually differ");
        }
        let required = REQUIRED_DOMAINS
            .into_iter()
            .map(|domain| CanonicalId::new(domain).expect("static SDK domain is canonical"))
            .collect::<BTreeSet<_>>();
        if self.required_domains != required {
            return invalid("SDK conformance required-domain coverage differs");
        }
        if self.identity.api_key.is_empty()
            || self.identity.api_key.len() > 128
            || self.identity.api_key.contains('\0')
        {
            return invalid("SDK conformance API key is invalid");
        }
        self.session.create.limits.validate()?;
        self.transaction.preview_begin.validate()?;
        self.transaction.preview.validate()?;
        self.transaction.commit_begin.validate()?;
        self.transaction.commit.validate()?;
        if self.transaction.commit_deadline_timeout_ms == 0
            || self.transaction.commit_deadline_timeout_ms > 300_000
        {
            return invalid("SDK conformance commit deadline window is invalid");
        }
        if self.transaction.commit.operation_sha256
            != transaction_operation_sha256(&self.transaction.commit.mutations)
        {
            return invalid("SDK conformance transaction digest differs from its mutations");
        }
        self.query.validate()?;
        self.vector.ensure.validate()?;
        self.vector.search.validate()?;
        self.vector.quantization.build.validate()?;
        self.vector.quantization.activate.validate()?;
        self.vector.quantization.search.validate()?;
        self.vector.index.payload_index.validate()?;
        self.vector.index.ensure.validate()?;
        self.vector.index.search.validate()?;
        self.vector.index.spare_payload_index.validate()?;
        self.vector.index.delete_payload_index.validate()?;
        self.changefeed.read.validate()?;
        self.cold_history.read.validate()?;
        if self.cold_history.read.generation != Some(1)
            || self.cold_history.expected_actor.is_empty()
            || self.cold_history.expected_actor.len() > 128
            || self.cold_history.expected_actor.contains('\0')
        {
            return invalid("SDK cold-history generation or seeded actor is invalid");
        }
        self.backup.create.validate()?;
        let scope = format!("instance:{}", self.identity.instance);
        if self.cold_history.read.scope != scope {
            return invalid("SDK cold-history scope differs from the corpus instance");
        }
        if self.query.scope != scope
            || self.vector.ensure.scope != scope
            || self.vector.search.scope != scope
            || self.vector.quantization.build.scope != scope
            || self.vector.quantization.activate.scope != scope
            || self.vector.quantization.search.scope != scope
            || self.vector.index.payload_index.scope != scope
            || self.vector.index.ensure.scope != scope
            || self.vector.index.search.scope != scope
            || self.vector.index.spare_payload_index.scope != scope
            || self.vector.index.delete_payload_index.scope != scope
            || self.changefeed.read.scope != scope
        {
            return invalid("SDK conformance requests must target the fixture instance scope");
        }
        if self.vector.search.collection_id.as_ref() != Some(&self.vector.ensure.collection_id)
            || self.vector.search.vector_name.as_ref()
                != self
                    .vector
                    .ensure
                    .vectors
                    .first()
                    .map(|vector| &vector.name)
        {
            return invalid("SDK conformance vector ensure/search addresses differ");
        }
        let quantization = &self.vector.quantization;
        if quantization.build.collection_id != self.vector.ensure.collection_id
            || Some(&quantization.build.vector_name) != self.vector.search.vector_name.as_ref()
            || quantization.search.collection_id != self.vector.search.collection_id
            || quantization.search.vector_name != self.vector.search.vector_name
            || !matches!(
                quantization.build.method,
                VectorQuantizationMethod::TurboQuant { .. }
            )
            || !matches!(
                quantization.search.mode,
                VectorSearchMode::RequireApproximate { .. }
            )
            || self.expected.quantized_access_path.as_str() != "turboquant"
        {
            return invalid(
                "SDK conformance quantization must build, activate and require TurboQuant on the corpus vector",
            );
        }
        let index = &self.vector.index;
        if index.ensure.collection_id != self.vector.ensure.collection_id
            || Some(&index.ensure.vector_name) == self.vector.search.vector_name.as_ref()
            || index.search.collection_id.as_ref() != Some(&index.ensure.collection_id)
            || index.search.vector_name.as_ref() != Some(&index.ensure.vector_name)
            || index.payload_index.collection_id != index.ensure.collection_id
            || index.search.filter.is_none()
            || index.spare_payload_index.field != index.delete_payload_index.field
            || index.spare_payload_index.field == index.payload_index.field
            || !matches!(
                index.search.mode,
                VectorSearchMode::RequireApproximate { .. }
            )
            || self.expected.indexed_access_path.as_str() != "hnsw"
            || self.expected.indexed_reference.is_empty()
        {
            return invalid(
                "SDK conformance vector index must filter and require HNSW on its own named vector",
            );
        }
        if self.changefeed.follow_wait_timeout_ms == 0
            || self.changefeed.follow_wait_timeout_ms > 5_000
            || self.changefeed.cancellation_wait_timeout_ms
                <= self.changefeed.follow_wait_timeout_ms
            || self.changefeed.cancel_after_ms == 0
            || self.changefeed.cancel_after_ms >= self.changefeed.cancellation_wait_timeout_ms
        {
            return invalid("SDK conformance changefeed wait/cancellation bounds are invalid");
        }
        if usize::from(self.expected.endpoint_count) != endpoint_catalogue().endpoints.len()
            || self.expected.query_identity.is_empty()
            || self.expected.vector_reference.is_empty()
            || self.expected.engram_revision == 0
            || self.expected.typed_error != ErrorCode::Unauthenticated
            || self.expected.retry_attempts != 2
        {
            return invalid("SDK conformance observable expectations are invalid");
        }
        Ok(())
    }
}

fn invalid<T>(message: impl Into<String>) -> Result<T> {
    Err(ContractError(message.into()))
}
