//! Authoritative RRFlow engine composition and operations.

use hmac::{Hmac, KeyInit, Mac};
use rrd_contract::{
    transaction_operation_sha256, AbortTransaction, ActivateVectorQuantizationArtifact,
    ActivateVectorQuantizationArtifactResult, AuditDecision, AuditExport, AuditPage, AuditPhase,
    AuditRecordSnapshot, BeginTransaction, BuildVectorQuantizationArtifact,
    BuildVectorQuantizationArtifactResult, CanonicalId, ChangeMutationSnapshot,
    ChangefeedFollowResult, ChangefeedPage, ChangefeedValidation, ClaimChangeSnapshot,
    ClaimPromotionSnapshot, ClaimTierSnapshot, CloseSession, CloseSubscription,
    CloseSubscriptionResult, CommitReceipt, CommitTransaction, CorrelationId, CreateInstanceBackup,
    CreateInstanceBackupResult, CreateSession, DataCatalogueIdentity, DataEmbeddingProvenance,
    DataEventSchema, DataGeoPoint, DataGeoValue, DataLogicalModel, DataObjectReceipt,
    DataProperties, DataPropertySchema, DataRecordSchema, DataReference, DataRelationSchema,
    DataSchemaMode, DataSchemaRegistry, DataSeriesValue, DataSnapshot, DataSnapshotEntry,
    DataTableSchema, DataTarget, DataValueType, DataVectorNormalization, DataVectorValue,
    DeleteVectorCollection, DeleteVectorCollectionResult, DeleteVectorPayloadIndex,
    DeleteVectorPayloadIndexResult, DeploymentProfile, DiagnosticAuthority, DiagnosticCoverage,
    DiagnosticGraphDifference, DiagnosticGraphRecordChange, DiagnosticGraphRecordSnapshot,
    DiagnosticGraphRelationChange, DiagnosticGraphRelationSnapshot, DiagnosticGraphSnapshot,
    DiagnosticModelCatalogueSnapshot, DiagnosticModelKind, DiagnosticModelSnapshot,
    DiagnosticReadStamp, DiagnosticRetentionPin, DiagnosticRetentionSnapshot,
    DiagnosticRuntimeReference, DiagnosticSectionSnapshot, DiagnosticSnapshot,
    DiagnosticSnapshotLease, DiagnosticVectorArtifactCatalogueSnapshot,
    DiagnosticVectorArtifactKind, DiagnosticVectorArtifactSnapshot, EnsureQueryIndex,
    EnsureQueryIndexResult, EnsureVectorCollection, EnsureVectorCollectionResult,
    EnsureVectorIndex, EnsureVectorIndexResult, EnsureVectorPayloadIndex,
    EnsureVectorPayloadIndexResult, EstateConfiguration, ExecuteFunction, ExecuteQuery,
    ExecuteQueryTransaction, ExecuteRetrievalQuery, ExportAudit, FollowChangefeed,
    ForwardRollbackCounts, ForwardRollbackPlan, ForwardRollbackRequest, FunctionArtifact,
    FunctionArtifactMediaType, FunctionCatalogue, FunctionDefinition, FunctionExecutionResult,
    FunctionInvocationProposal, FunctionInvocationReceipt, FunctionRuntime, FunctionRuntimeKind,
    HybridFusion, HybridSearchHit, HybridSearchResult, InstalledEstateIdentity,
    InstanceBackupCatalogueSnapshot, InstanceBackupSnapshot, ListInstanceBackups, ListQueryIndexes,
    ListVectorCollections, ListVectorPayloadIndexes, ListVectorQuantizationArtifacts,
    ListVectorQuantizationArtifactsResult, LiveQueryDeltaResult, LiveQueryRowChange,
    LogicalArchiveSnapshot, NamedVectorDefinition, OpenSubscription, OpenSubscriptionResult,
    PollLiveQuery, PreviewTransaction, QueryExecutionAnalysisSnapshot, QueryExecutionSnapshot,
    QueryFullTextConfiguration, QueryIndexCatalogueSnapshot, QueryIndexKind,
    QueryIndexMaintenanceSnapshot, QueryIndexSnapshot, QueryIndexState, QueryPlanCandidate,
    QueryPlanSnapshot, QueryResult, QueryRowSnapshot, QueryTextAnalyzer, QueryTextStemmer,
    QueryTextTokenizer, QueryTransactionResult, QueryValue, ReadAudit, ReadChangefeed,
    ReadDataSnapshot, ReadDiagnosticSnapshot, Readiness, RenewSession, ReplaceFunctionCatalogue,
    RequestContext, ResourceId, ResourceKind, ResourcePath, RestoreInstanceBackup,
    RestoreInstanceBackupResult, RetireVectorQuantizationArtifact,
    RetireVectorQuantizationArtifactResult, RetrievalContextPair, RetrievalContribution,
    RetrievalFacet, RetrievalFusion, RetrievalGroup, RetrievalHit, RetrievalMatrixCell,
    RetrievalOutput, RetrievalPrefetch, RetrievalQuery, RetrievalQueryResult,
    RetrievalRecommendStrategy, RetrievalRerankStage, RetrievalResultShape, RetrievalStageEvidence,
    RetrievalVectorExample, RetrieveVectorPoints, RuntimeChangeSnapshot, ScrollVectorPoints,
    SearchHybrid, SearchVectors, SecurityAction, SessionEndState, SessionLease, SessionLimits,
    SessionTermination, StorageProfileKind, SubscriptionAcknowledgement, SubscriptionDelivery,
    SubscriptionDeliveryBatch, SubscriptionLeaseCoordinate, SubscriptionPoll, SubscriptionResume,
    SubscriptionSnapshot, SubscriptionStatus, SubscriptionStream, TransactionFunctionBinding,
    TransactionFunctionEffect, TransactionLease, TransactionMutation, TransactionMutationKind,
    TransactionPreview, TransactionState, VectorCollectionCatalogueSnapshot,
    VectorCollectionSnapshot, VectorEmbeddingModel, VectorIndexBuildEvidence,
    VectorIndexBuildPolicy, VectorIndexBuildResourceEvidence, VectorIndexBuildTarget,
    VectorIndexConfiguration, VectorIndexDifferentialStatus, VectorIndexMaintenanceMode,
    VectorIndexMaintenanceSnapshot, VectorIndexSnapshot, VectorMemoryTier, VectorPayloadFilter,
    VectorPayloadIndexCatalogueSnapshot, VectorPayloadIndexKind, VectorPayloadIndexSnapshot,
    VectorPayloadOperator, VectorPointBatch, VectorPointPage, VectorPointSnapshot,
    VectorProductCompression, VectorQuantizationArtifactSnapshot, VectorQuantizationArtifactState,
    VectorQuantizationBits, VectorQuantizationMethod, VectorSearchHit, VectorSearchMetric,
    VectorSearchMode, VectorSearchQuery, VectorSearchResourceEvidence, VectorSearchResult,
    VectorValueKind, DIAGNOSTIC_SNAPSHOT_FORMAT_VERSION,
};
use rrd_contract::{
    EmbedAndSearchVectors, EmbedAndSearchVectorsResult, EmbeddingBackendSnapshot,
    EmbeddingExecutionTarget, EmbeddingModelCatalogue, EmbeddingNetworkPolicy,
    EmbeddingResourceLimits, EmbeddingTrustBoundary, GenerateEmbeddings, GenerateEmbeddingsResult,
    GeneratedEmbedding, ListEmbeddingModels,
};
use rrd_core::{
    digest, Claim, DataTransaction, EmbeddingProvenance, GeoPoint, GeoValue, ObjectReceipt,
    ObjectReference, Predicate, Producer, ProjectionId, ProjectionState, PromotionState, ReadStamp,
    RetentionPin, RuntimeCatalogueIdentity, RuntimeChange, RuntimeCommit, RuntimeDataSnapshot,
    RuntimeEvent, RuntimeEventSchema, RuntimeGeo, RuntimeGraphSnapshot, RuntimeLogicalModel,
    RuntimeMutation, RuntimeProperties, RuntimePropertySchema, RuntimeRecord, RuntimeRecordSchema,
    RuntimeRef, RuntimeRelation, RuntimeRelationSchema, RuntimeSchemaMode, RuntimeSchemaRegistry,
    RuntimeSeriesSample, RuntimeTableSchema, RuntimeType, RuntimeValue, RuntimeValueType,
    RuntimeVector, ScopeId, SeriesValue, Subject, Tier, VectorNormalization, VectorValue,
};
use rrd_query::{ComparisonOperator, CursorExpr, Filter, Projection, TimeExpr, ValueExpr};
use rrd_store::{
    ControlTransition, MemoryObjectStore, ObjectStoreBox, RrflowKvStore, RuntimeDataSnapshotRead,
    StorageEngine, StorageProfile,
};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

mod backup;
mod changefeed;
mod clock;
mod context;
mod control;
mod core;
mod data;
mod diagnostic;
mod distributed;
mod endpoint_configuration;
mod error;
mod estate;
mod estate_control;
mod estate_layout;
mod flight;
mod function;
mod inference;
mod installation;
mod invocation;
mod maintenance;
mod memory_estate;
mod model;
mod query;
mod query_export;
mod query_transaction;
mod restore_publication;
mod storage_cursor;
mod tail_repair;
pub use tail_repair::{InstalledTailRepairPlan, InstalledTailRepairReceipt};
mod retrieval;
mod retrieval_query;
mod rollback;
mod security;
mod session;
mod session_transactions;
mod subscription;
mod task;
mod token_key;
mod transaction;
mod transaction_admission;
mod vector;

pub use clock::{
    ClockAssessment, ClockAssessmentStatus, ClockObservation, ClockObservationFailure,
    ClockSourceKind, ClockTrustLevel, CLOCK_OBSERVATION_FORMAT_VERSION,
};
use control::*;
pub use core::RrdEngine;
use data::public_data_snapshot;
pub use distributed::{
    DistributedAuthorityRead, DistributedReadRoute, PreparedDistributedAuthority,
};
pub use error::{Result, ServiceError, ServiceErrorKind};
pub use estate_control::{
    EstateAdminAction, EstateAdminResult, EstateBackupReconcileOutcome, EstateReconcileOutcome,
};
pub use installation::{
    load_estate_configuration, DefaultAttunementPhase, DefaultInstallProfile, InstallationPreview,
    InstallationVerificationCheck, InstallationVerificationReport, InstallationVerificationStatus,
    InstalledEstateRecord, ProjectLocator,
};
pub use invocation::{
    AuthorizedInvocation, Invocation, InvocationCompletion, InvocationCredential, RrdOperation,
};
use model::*;
pub use security::{MAX_AUDIT_PAGE_RECORDS, MAX_JWT_CREDENTIAL_BYTES};
use session::*;
use token_key::load_or_create_token_key;
pub use token_key::TOKEN_KEY_BYTES;
pub use transaction_admission::TransactionAdmissionSnapshot;

const SESSION_STATE_FORMAT: u16 = 3;
const MAX_SESSION_RENEWALS: usize = 64;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SessionState {
    format_version: u16,
    // Earlier strict session readers must reject state whose terminal records
    // they cannot discover. Absence denotes the existing inline working set;
    // every new transition serializes the current marker atomically with it.
    #[serde(default = "session_transactions::terminal_transaction_format")]
    terminal_transaction_format: u16,
    session_id: CorrelationId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    principal_id: Option<CanonicalId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    principal_credential_revision: Option<u64>,
    status: SessionStatus,
    issued_at_unix_ms: u64,
    idle_expires_at_unix_ms: u64,
    absolute_expires_at_unix_ms: u64,
    limits: SessionLimits,
    creation_idempotency_key: CorrelationId,
    creation_operation_sha256: String,
    creation_idle_expires_at_unix_ms: u64,
    token_sha256: String,
    token_generation: u64,
    renewals: BTreeMap<CorrelationId, RenewalRecord>,
    closure: Option<ClosureRecord>,
    transactions: BTreeMap<CorrelationId, TransactionRecord>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum SessionStatus {
    Active,
    Expired,
    Closed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RenewalRecord {
    operation_sha256: String,
    previous_token_sha256: String,
    token_generation: u64,
    idle_expires_at_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ClosureRecord {
    idempotency_key: CorrelationId,
    operation_sha256: String,
    previous_token_sha256: String,
    ended_at_unix_ms: u64,
    affected_open_transactions: u16,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TransactionRecord {
    lease: TransactionLease,
    read: ReadStamp,
    begin_idempotency_key: CorrelationId,
    begin_operation_sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    prepared: Option<PreparedRecord>,
    commit_intent: Option<CommitIntent>,
    commit_receipt: Option<CommitReceipt>,
    abort: Option<AbortRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PreparedRecord {
    idempotency_key: CorrelationId,
    operation_sha256: String,
    runtime_at_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CommitIntent {
    idempotency_key: CorrelationId,
    operation_sha256: String,
    runtime_at_unix_ms: u64,
    runtime_commit_sha256: String,
    function_catalogue_revision: u64,
    function_receipts: Vec<FunctionInvocationReceipt>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AbortRecord {
    idempotency_key: CorrelationId,
    operation_sha256: String,
}

#[cfg(test)]
mod tests;
