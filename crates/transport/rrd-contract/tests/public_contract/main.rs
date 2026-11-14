use rrd_contract::{
    transaction_operation_sha256, ActivateVectorQuantizationArtifact, BeginTransaction,
    BuildVectorQuantizationArtifact, CanonicalId, CapabilityDescriptor, CapabilityStatus,
    CloseSession, CommitReceipt, CommitTransaction, CorrelationId, CreateInstanceBackup,
    DataDomainSchema, DataEstateOrganisation, DataLogicalModel, DataRecordSchema, DataReference,
    DataSchemaMode, DataSchemaRegistry, DataSnapshot, DataTableSchema, DataTarget,
    DeleteVectorCollection, DeleteVectorPayloadIndex, DeploymentForm, DeploymentProfile,
    EmbedAndSearchVectors, EmbeddingInput, EmbeddingNetworkPolicy, EndpointPresentation,
    EnsureQueryIndex, EnsureVectorCollection, EnsureVectorIndex, EnsureVectorPayloadIndex,
    ErrorBody, ErrorCode, EstateActivityPolicySnapshot, EstateAuthoritySnapshot,
    EstateBackupJobSnapshot, EstateBackupJobState, EstateBackupJobsSnapshot,
    EstateBackupRecoveryPolicySnapshot, EstateConfiguration, EstateMutationResult, EstateSnapshot,
    ExecuteQuery, ExecuteQueryTransaction, ExecuteRetrievalQuery, FollowChangefeed,
    ForwardRollbackCounts, ForwardRollbackRequest, GenerateEmbeddings, HybridFusion,
    IdempotencyBinding, InstalledEstateIdentity, ListQueryIndexes, ListVectorCollections,
    ListVectorPayloadIndexes, ListVectorQuantizationArtifacts, Liveness, NamedVectorDefinition,
    PollLiveQuery, PreviewTransaction, ProductCapability, ProductCapabilityCatalogue,
    ProductSurface, QueryBudget, QueryExecutionAnalysisSnapshot, QueryExecutionSnapshot,
    QueryIndexKind, QueryPlanCandidate, QueryPlanSnapshot, QueryResult, QueryRowSnapshot,
    QueryValue, ReadAccessPath, ReadAudit, ReadChangefeed, ReadDataSnapshot, ReadEstate,
    ReadEvidence, ReadPathEvidence, ReadStampValidationEvidence, ReadStampValidationMethod,
    Readiness, RenewSession, RequestContext, RequestEnvelope, ResourceId, ResourceKind,
    ResourcePath, ResponseEnvelope, ResponseOutcome, RestoreInstanceBackup,
    RetireVectorQuantizationArtifact, RetrievalFusion, RetrievalPrefetch, RetrievalQuery,
    RetrievalResultShape, SearchHybrid, SearchVectors, ServiceCapabilities, SessionEndState,
    SessionLease, SessionLimits, SessionTermination, StorageProfileKind, SurfaceBinding,
    SurfaceDisposition, TransactionMutation, TransactionPreview, TransactionState,
    VectorIndexBuildPolicy, VectorIndexConfiguration, VectorMemoryTier, VectorPayloadCondition,
    VectorPayloadFilter, VectorPayloadIndexKind, VectorPayloadOperator, VectorProductCompression,
    VectorQuantizationMethod, VectorSearchMetric, VectorSearchMode, VectorSearchQuery,
    VectorValueKind, PROTOCOL, PROTOCOL_VERSION,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

mod catalogue;
mod estate_and_query;
mod flight_and_tasks;
mod golden;
mod schema_and_retrieval;
mod storage_cursor;
mod vectors_and_audit;

use catalogue::*;
use estate_and_query::*;
use golden::*;
use vectors_and_audit::*;
