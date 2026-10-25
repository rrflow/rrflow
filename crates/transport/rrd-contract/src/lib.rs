//! Stable, transport-neutral public contracts for RRD.
//!
//! Internal storage, query, cluster, and UI types must not leak into this
//! boundary. HTTP, gRPC, embedded clients, SDKs, and Connectome consume the
//! same serialized vocabulary. Version 1 intentionally freezes only the
//! coordinates needed before those outward surfaces are implemented.

mod attunement;
mod capability_surface;
mod deployment;
mod diagnostic;
mod storage_cursor;
pub use storage_cursor::{InspectStorageCursor, StorageCursorAction, StorageCursorSnapshot};
mod endpoint_binding;
mod endpoint_configuration;
mod flight;
mod function;
#[path = "generated/signal_catalogue.rs"]
mod generated_signal_catalogue;
mod inference;
mod knowledge;
mod knowledge_tree;
mod memory_context;
mod memory_estate;
mod model_manifest;
mod platform;
mod query_export;
mod read;
mod reasoning_budget;
pub use reasoning_budget::*;
mod reasoning_tree;
mod restore_publication;
mod router;
mod sdk_conformance;
mod task;
mod websocket;

pub use attunement::{
    attunement_checkpoint_sha256, attunement_plan_sha256, attunement_verification_sha256,
    installation_plan_sha256, installation_result_sha256, AttunementFailure, AttunementJob,
    AttunementJobState, AttunementLease, AttunementPhase, AttunementPhaseCheckpoint,
    AttunementPhasePlan, AttunementPlan, AttunementRuntimeCoordinates, AttunementStatus,
    AttunementVerification, AttunementVerificationCheck, AttunementVerificationStatus,
    CancelAttunement, InstallationActionDisposition, InstallationActionKind,
    InstallationActionResult, InstallationBootstrap, InstallationBootstrapPolicy,
    InstallationManagedPath, InstallationManagedPathKind, InstallationPlan, InstallationPlanAction,
    InstallationRecordKind, InstallationRemovalRule, InstallationResult,
    InstallationSpecialization, InstallationStorageGeneration, InstallationTargetKind,
    ResumeAttunement, ATTUNEMENT_PHASES, INSTALLATION_BOOTSTRAP_VERSION, INSTALLATION_LEASE_PATH,
    INSTALLATION_LEASE_PRECONDITION, INSTALLATION_PLAN_CONTRACT_VERSION,
    INSTALL_ATTUNEMENT_CONTRACT_VERSION, MAX_ATTUNEMENT_CANCEL_REASON_BYTES,
    MAX_ATTUNEMENT_FAILURE_BYTES, MAX_ATTUNEMENT_PHASES, MAX_ATTUNEMENT_VERIFICATION_CHECKS,
    MAX_BOOTSTRAP_RECORD_BYTES, MAX_BOOTSTRAP_TEXT_BYTES, MAX_INSTALLATION_ACTIONS,
};
pub use capability_surface::{
    ProductCapability, ProductCapabilityCatalogue, ProductSurface, SurfaceBinding,
    SurfaceDisposition,
};
pub use deployment::{
    estate_configuration_sha256, ClockPolicy, DeploymentForm, DeploymentProfile,
    EndpointPresentation, EstateConfiguration, EstateConfigurationInput, InstalledEstateIdentity,
    ReasoningLimits, RecallLimits, StorageLimits, StorageProfileKind,
    DEPLOYMENT_PROFILE_CONTRACT_VERSION, ESTATE_CONFIGURATION_FORMAT_VERSION,
    MAX_CLOCK_ROLLBACK_MS, MAX_REASONING_RUN_ELAPSED_MS, MAX_REASONING_STEPS,
    MAX_REASONING_STEP_ELAPSED_MS,
};
pub use diagnostic::{
    DiagnosticAuthority, DiagnosticCoverage, DiagnosticGraphDifference,
    DiagnosticGraphRecordChange, DiagnosticGraphRecordSnapshot, DiagnosticGraphRelationChange,
    DiagnosticGraphRelationSnapshot, DiagnosticGraphSnapshot, DiagnosticMaintenanceSnapshot,
    DiagnosticMemtableSnapshot, DiagnosticModelCatalogueSnapshot, DiagnosticModelKind,
    DiagnosticModelSnapshot, DiagnosticPageCacheSnapshot, DiagnosticReadStamp,
    DiagnosticRetentionPin, DiagnosticRetentionSnapshot, DiagnosticRrflowKvSnapshot,
    DiagnosticRuntimeReference, DiagnosticSectionSnapshot, DiagnosticSegmentIoMode,
    DiagnosticSegmentIoSnapshot, DiagnosticSegmentSetSnapshot, DiagnosticSnapshot,
    DiagnosticSnapshotLease, DiagnosticStorageAdmission, DiagnosticStorageEvidenceLevel,
    DiagnosticStorageSnapshot, DiagnosticVectorArtifactCatalogueSnapshot,
    DiagnosticVectorArtifactKind, DiagnosticVectorArtifactSnapshot,
    DiagnosticWriteAheadLogSnapshot, ReadDiagnosticSnapshot, ReadStorageDiagnostics,
    DIAGNOSTIC_SNAPSHOT_FORMAT_VERSION, MAX_DIAGNOSTIC_AUDIT_RECORDS,
    MAX_DIAGNOSTIC_RUNTIME_SCANNED_CHANGES,
};
pub use endpoint_binding::{
    endpoint_profile_sha256, BindingKind, BindingLimits, CredentialReference, DeclaredFootprint,
    EndpointBinding, EndpointLocator, EndpointProfile, EndpointProfileInput, GpuCapacity,
    GpuRequirement, HardwareInventory, MachineCapacity, MachineLoad, WorkloadClass,
    ENDPOINT_PROFILE_CONTRACT_VERSION, HARDWARE_INVENTORY_CONTRACT_VERSION, MAX_BINDINGS_PER_CLASS,
    MAX_BINDING_CONCURRENCY, MAX_BINDING_INPUT_BYTES, MAX_BINDING_TIMEOUT_MS,
    MAX_CREDENTIAL_NAME_BYTES, MAX_DECLARED_BYTES, MAX_ENDPOINT_URL_BYTES, MAX_INVENTORY_MACHINES,
    MAX_PROFILE_BINDINGS,
};
pub use endpoint_configuration::{
    ApplyEndpointConfiguration, EndpointConfiguration, EndpointConfigurationPlan,
    PreviewEndpointConfiguration, ReadEndpointConfiguration, ENDPOINT_CONFIGURATION_PREVIEW_TTL_MS,
    ENDPOINT_CONFIGURATION_VERSION, MAX_ENDPOINT_CONFIGURATION_BYTES,
};
pub use flight::{
    prompt_cohort_sha256, LaunchPromptFlight, LaunchPromptFlightResult, ListPromptFlights,
    PromptFlight, PromptFlightContextMode, PromptFlightContextRequest, PromptFlightContextSummary,
    PromptFlightEvent, PromptFlightMetrics, PromptFlightPage, PromptFlightProvider,
    PromptFlightReadResult, PromptFlightRecord, PromptFlightStage, PromptFlightStatus,
    PromptFlightUse, ReadPromptFlight, MAX_PROMPT_FLIGHT_ACCEPTANCE_MARKER_BYTES,
    MAX_PROMPT_FLIGHT_EVENTS, MAX_PROMPT_FLIGHT_EVENT_DATA_BYTES,
    MAX_PROMPT_FLIGHT_EVENT_DETAIL_BYTES, MAX_PROMPT_FLIGHT_EVENT_KIND_BYTES,
    MAX_PROMPT_FLIGHT_EVENT_LABEL_BYTES, MAX_PROMPT_FLIGHT_PAGE, MAX_PROMPT_FLIGHT_PROMPT_BYTES,
    MAX_PROMPT_FLIGHT_STORAGE_KEYS, PROMPT_FLIGHT_EVENT_KIND, PROMPT_FLIGHT_FORMAT_VERSION,
    PROMPT_FLIGHT_KIND, PROMPT_FLIGHT_USE_KIND,
};
pub use function::{
    validate_function_value, ExecuteFunction, FunctionArtifact, FunctionArtifactMediaType,
    FunctionCapability, FunctionCatalogue, FunctionDefinition, FunctionExecutionResult,
    FunctionInvocationProposal, FunctionInvocationReceipt, FunctionLimits, FunctionRuntime,
    FunctionRuntimeKind, FunctionValueSchema, FunctionValueShape, ListFunctionCatalogue,
    ReplaceFunctionCatalogue, TransactionFunctionBinding, TransactionFunctionEffect,
    TransactionMutationKind, WebAssemblyAbi, FUNCTION_CONTRACT_VERSION,
    MAX_FUNCTIONS_PER_CATALOGUE, MAX_FUNCTION_CATALOGUE_ARTIFACT_BYTES,
    MAX_FUNCTION_CATALOGUE_ENCODED_BYTES, MAX_FUNCTION_INPUT_BYTES, MAX_FUNCTION_JSON_SAFE_INTEGER,
    MAX_FUNCTION_OUTPUT_BYTES, MAX_FUNCTION_SCHEMA_DEPTH, MAX_FUNCTION_SCHEMA_ITEMS,
    MAX_FUNCTION_SOURCE_BYTES, MAX_FUNCTION_VALUE_DEPTH, MAX_FUNCTION_VALUE_ITEMS,
    MAX_FUNCTION_WASM_BYTES, MAX_TRANSACTION_FUNCTION_BINDINGS_PER_CATALOGUE,
};
pub use inference::{
    EmbedAndSearchVectors, EmbedAndSearchVectorsResult, EmbeddingBackendSnapshot,
    EmbeddingExecutionTarget, EmbeddingInput, EmbeddingModality, EmbeddingModelCatalogue,
    EmbeddingNetworkPolicy, EmbeddingResourceLimits, EmbeddingTrustBoundary, GenerateEmbeddings,
    GenerateEmbeddingsResult, GeneratedEmbedding, ListEmbeddingModels, MAX_EMBEDDING_BATCH_BYTES,
    MAX_EMBEDDING_BATCH_INPUTS, MAX_EMBEDDING_INPUT_BYTES,
};
pub use knowledge::{
    knowledge_package_sha256, knowledge_record_sha256, KnowledgeClassification,
    KnowledgeExclusionV1, KnowledgeManifestDispositionV1, KnowledgeManifestEntryV1,
    KnowledgePackageV1, KnowledgeProvenance, KnowledgeRecordV1, KnowledgeSourceInventoryEntryV1,
    KNOWLEDGE_PACKAGE_CONTRACT_VERSION, MAX_KNOWLEDGE_BODY_BYTES, MAX_KNOWLEDGE_COORDINATE_BYTES,
    MAX_KNOWLEDGE_EXCLUSIONS, MAX_KNOWLEDGE_EXCLUSION_REASON_BYTES,
    MAX_KNOWLEDGE_PACKAGE_BODY_BYTES, MAX_KNOWLEDGE_RECORDS, MAX_KNOWLEDGE_REVISION_BYTES,
    MAX_KNOWLEDGE_SOURCE_PATH_BYTES,
};
pub use knowledge_tree::{
    KnowledgePath, KnowledgePathBinding, KnowledgeTree, KnowledgeTreeCatalogue, ListKnowledgeTrees,
    ReplaceKnowledgeTrees, ReplaceKnowledgeTreesResult, KNOWLEDGE_TREE_CONTRACT_VERSION,
    MAX_KNOWLEDGE_CATALOGUE_BYTES, MAX_KNOWLEDGE_NAME_BYTES, MAX_KNOWLEDGE_PATHS,
    MAX_KNOWLEDGE_PATHS_PER_TREE, MAX_KNOWLEDGE_PATH_DEPTH,
    MAX_KNOWLEDGE_PROVIDER_IDENTIFIER_BYTES, MAX_KNOWLEDGE_TREES,
};
pub use memory_context::{
    context_packet_sha256, context_plan_sha256, context_plan_stage_sha256, context_request_sha256,
    AssembleContext, ContextAccessPath, ContextEvidence, ContextEvidenceKind, ContextItem,
    ContextPacket, ContextPlanSnapshot, ContextPlanStage, ContextPlanStageKind,
    ContextPlanStageStatus, ContextReadStamp, MAX_CONTEXT_GRAPH_DEPTH, MAX_CONTEXT_ITEMS,
    MAX_CONTEXT_OUTPUT_BYTES, MAX_CONTEXT_QUERY_BYTES, MAX_CONTEXT_SEEDS, MAX_CONTEXT_STORAGE_KEYS,
};
pub use memory_estate::{
    MemoryEstatePlan, MemorySeatDefinition, MemoryWarp, PersistMemoryEstate,
    ProviderRepresentation, ProviderRepresentationDefinition, ResolveMemoryWarp,
    ResolveSeatIdentity, ResolvedMemoryWarp, SeatIdentity, MAX_PROVIDER_REPRESENTATIONS,
    MEMORY_PROVIDER_IDENTITY_KIND, MEMORY_REPRESENTS_KIND, MEMORY_SEAT_KIND,
};
pub use model_manifest::{
    router_model_handshake_sha256, router_model_manifest_sha256, RouterArtifactDescriptor,
    RouterGrammarBinding, RouterModelHandshake, RouterModelLimits, RouterModelManifest,
    RouterQuantizationBinding, RouterRuntimeBinding, MAX_ROUTER_ARTIFACT_MEDIA_TYPE_BYTES,
    MAX_ROUTER_CONTEXT_TOKENS, MAX_ROUTER_GRAMMAR_ARTIFACT_BYTES, MAX_ROUTER_MODEL_ARTIFACT_BYTES,
    MAX_ROUTER_OUTPUT_TOKENS, MAX_ROUTER_RESIDENT_BYTES, MAX_ROUTER_RUNTIME_ARTIFACT_BYTES,
    MAX_ROUTER_THREADS, MAX_ROUTER_TOKENIZER_ARTIFACT_BYTES,
    ROUTER_MODEL_MANIFEST_CONTRACT_VERSION,
};
pub use platform::{PlatformTermDefinition, PlatformTermRole, PLATFORM_TERMS};
pub use query_export::{
    ExportQuery, QueryExport, QueryExportSource, MAX_PARQUET_EXPORT_BYTES,
    MAX_PARQUET_EXPORT_COLUMNS, MAX_PARQUET_EXPORT_INPUT_BYTES, MAX_PARQUET_EXPORT_ROWS,
    MAX_PARQUET_ROW_GROUP_ROWS, PARQUET_EXPORT_PROFILE,
};
pub use read::{
    ReadAccessPath, ReadEvidence, ReadPathEvidence, ReadStampValidationEvidence,
    ReadStampValidationMethod, READ_EVIDENCE_CONTRACT_VERSION,
};
pub use reasoning_tree::{
    ReasoningActiveCursor, ReasoningCondition, ReasoningConditionEvaluation,
    ReasoningConditionPredicate, ReasoningCursorAdvance, ReasoningDecisionEvidence, ReasoningEdge,
    ReasoningEvidence, ReasoningNode, ReasoningReadStamp, ReasoningRecipe,
    ReasoningRecipeSelection, ReasoningTree, ReasoningVerificationResult,
    ReasoningVerificationStatus, MAX_REASONING_EDGE_CONDITIONS, MAX_REASONING_EVIDENCE_ITEMS,
    MAX_REASONING_EVIDENCE_SOURCE_BYTES, MAX_REASONING_EVIDENCE_SUMMARY_BYTES,
    MAX_REASONING_TREE_EDGES, MAX_REASONING_TREE_NODES, MAX_REASONING_TREE_RECIPES,
    REASONING_TREE_CONTRACT_VERSION,
};
pub use restore_publication::{
    restore_publication_plan_sha256, restore_publication_result_sha256, RestorePublicationAction,
    RestorePublicationActionKind, RestorePublicationActionResult, RestorePublicationPlan,
    RestorePublicationResult, RestoreRootEvidence, MAX_RESTORE_PUBLICATION_ACTIONS,
    RESTORE_PUBLICATION_ACTIONS,
};
pub use router::{
    route_step_decision_schema_sha256, route_step_decision_sha256, route_step_request_sha256,
    router_backend_descriptor_sha256, RouteContextAllowance, RouteContextBudget, RouteDecisionKind,
    RouteParameterValue, RouteSignal, RouteSignalValue, RouteStepDecision, RouteStepRequest,
    RouterBackendDescriptor, RouterBackendLimits, MAX_ROUTE_BRANCH_CANDIDATES,
    MAX_ROUTE_EXECUTION_MS, MAX_ROUTE_INTENT_BYTES, MAX_ROUTE_PARAMETERS,
    MAX_ROUTE_PARAMETER_DEPTH, MAX_ROUTE_PARAMETER_ITEMS, MAX_ROUTE_RECIPE_CANDIDATES,
    MAX_ROUTE_REQUEST_BYTES, MAX_ROUTE_RESPONSE_BYTES, MAX_ROUTE_SIGNALS, MAX_ROUTE_SIGNAL_BYTES,
    ROUTER_CONTRACT_VERSION,
};
pub use sdk_conformance::{
    SdkConformanceBackup, SdkConformanceChangefeed, SdkConformanceCorpus, SdkConformanceExpected,
    SdkConformanceIdentity, SdkConformanceSession, SdkConformanceTransaction, SdkConformanceVector,
    SDK_CONFORMANCE_FORMAT_VERSION,
};
pub use task::{
    CloseTask, ListTasks, OpenTask, ReadTask, TaskPage, TaskReadResult, TaskRecord, TaskResult,
    TaskStatus, MAX_TASK_GOAL_BYTES, MAX_TASK_OUTCOME_BYTES, MAX_TASK_PAGE, MAX_TASK_STORAGE_KEYS,
    MAX_TASK_TITLE_BYTES, TASK_FORMAT_VERSION, TASK_INCLUDES_KIND, TASK_KIND,
};
pub use websocket::{
    decode_websocket_frame, encode_websocket_frame, validate_subscription_snapshot,
    validate_websocket_cancellation_correlation, validate_websocket_response_correlation,
    validate_websocket_subscription_correlation, SubscriptionAcknowledgement, SubscriptionDelivery,
    SubscriptionDeliveryBatch, SubscriptionLeaseCoordinate, SubscriptionPoll, SubscriptionResume,
    WebSocketAcknowledged, WebSocketBackpressure, WebSocketBackpressureTarget, WebSocketCancel,
    WebSocketCancellation, WebSocketCancellationDisposition, WebSocketConnected, WebSocketDelivery,
    WebSocketError, WebSocketErrorTarget, WebSocketFrame, WebSocketHeartbeat, WebSocketLimits,
    WebSocketPayload, WebSocketPeer, WebSocketReceiveState, WebSocketRequest,
    WebSocketRequestTarget, WebSocketResponse, WebSocketSendState, WebSocketSubscribe,
    WebSocketSubscribed, WebSocketUnsubscribe, WebSocketUnsubscribed, MAX_WEBSOCKET_BUFFER_BYTES,
    MAX_WEBSOCKET_CANCEL_REASON_BYTES, MAX_WEBSOCKET_ERROR_BYTES, MAX_WEBSOCKET_ERROR_DETAILS,
    MAX_WEBSOCKET_FRAME_BYTES, MAX_WEBSOCKET_IN_FLIGHT_REQUESTS, MAX_WEBSOCKET_JSON_DEPTH,
    MAX_WEBSOCKET_JSON_ITEMS, MAX_WEBSOCKET_MESSAGE_BYTES, MAX_WEBSOCKET_RETRY_AFTER_MS,
    MAX_WEBSOCKET_SUBSCRIPTIONS, MAX_WEBSOCKET_TIMEOUT_MS, MAX_WEBSOCKET_WRITE_BUFFER_BYTES,
    MIN_WEBSOCKET_HEARTBEAT_MS, WEBSOCKET_CONTRACT_VERSION,
};

use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub const PROTOCOL: &str = "rrd";
pub const PROTOCOL_VERSION: u16 = 1;
pub const OPENAPI_DOCUMENT_SHA256: &str =
    "06cd649d80ec2de5fe152da49daa00533d2e5dbf56e0321fea4d86f7b432a7b3";
pub const MAX_ID_BYTES: usize = 128;
pub const MAX_MESSAGE_BYTES: usize = 4_096;
pub const MAX_CAPABILITIES: usize = 512;
pub const MAX_TRANSACTION_CLAIMS: usize = 4_096;
pub const MAX_DATA_SNAPSHOT_STORAGE_KEYS: u32 = 1_000_000;
pub const MAX_VECTOR_DIMENSIONS: usize = 1_048_576;
pub const MAX_QUERY_BYTES: usize = 64 * 1024;
pub const MAX_QUERY_PARAMETERS: usize = 128;
pub const MAX_QUERY_PARAMETER_BYTES: usize = 64 * 1024;
pub const MAX_QUERY_STORAGE_KEYS: u64 = 1_000_000;
pub const MAX_QUERY_ROWS: u64 = 100_000;
pub const MAX_QUERY_OUTPUT_BYTES: u64 = 768 * 1024;
pub const MAX_QUERY_BATCH_ROWS: u64 = 1_024;
pub const MAX_QUERY_MEMORY_BYTES: u64 = 1024 * 1024 * 1024;
pub const MAX_QUERY_SPILL_BYTES: u64 = 8 * 1024 * 1024 * 1024;
pub const MAX_QUERY_ELAPSED_MS: u64 = 300_000;
pub const MAX_QUERY_TRANSACTION_MUTATIONS: usize = 256;
pub const MAX_QUERY_TRANSACTION_BINDING_BYTES: usize = 1024 * 1024;
pub const MAX_LIVE_QUERY_DELTA_ROWS: u64 = 100_000;
pub const MAX_LIVE_QUERY_WAIT_MS: u64 = 5_000;
pub const MAX_VECTOR_STORAGE_KEYS: u64 = 1_000_000;
pub const MAX_VECTOR_SEARCH_WORK: u64 = 1_000_000;
pub const MAX_VECTOR_SEARCH_TOP_K: u64 = 100_000;
pub const MAX_VECTOR_POINT_PAGE: u64 = 4_096;
pub const MAX_CHANGEFEED_PAGE: u64 = 4_096;
pub const MAX_CHANGEFEED_WAIT_MS: u64 = 5_000;
pub const MAX_SUBSCRIPTION_IN_FLIGHT: u16 = 64;
pub const MAX_SUBSCRIPTION_RETENTION_CURSORS: u64 = 10_000_000;
pub const MIN_SUBSCRIPTION_HEARTBEAT_MS: u64 = 100;
pub const MAX_SUBSCRIPTION_HEARTBEAT_MS: u64 = 30_000;
pub const DEPLOYMENT_CONFORMANCE_FORMAT_VERSION: u16 = 1;
pub const MAX_DEPLOYMENT_CONFORMANCE_DOCUMENTS: usize = 1_024;
pub const MIN_LEASE_MS: u64 = 1_000;
pub const MAX_LEASE_MS: u64 = 3_600_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum QueryValue {
    Null,
    Bool(bool),
    Integer(i64),
    Unsigned(u64),
    Decimal(String),
    String(String),
    Digest(String),
    List(Vec<QueryValue>),
    Map(BTreeMap<String, QueryValue>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct QueryBudget {
    pub max_storage_keys: u64,
    pub max_rows: u64,
    pub max_output_bytes: u64,
    pub max_batch_rows: u64,
    #[serde(default = "default_query_memory_bytes")]
    pub max_memory_bytes: u64,
    #[serde(default = "default_query_spill_bytes")]
    pub max_spill_bytes: u64,
    #[serde(default = "default_query_elapsed_ms")]
    pub max_elapsed_ms: u64,
}

impl Default for QueryBudget {
    fn default() -> Self {
        Self {
            max_storage_keys: 100_000,
            max_rows: 10_000,
            max_output_bytes: 512 * 1024,
            max_batch_rows: 256,
            max_memory_bytes: default_query_memory_bytes(),
            max_spill_bytes: default_query_spill_bytes(),
            max_elapsed_ms: default_query_elapsed_ms(),
        }
    }
}

impl QueryBudget {
    pub fn validate(&self) -> Result<()> {
        for (name, value, maximum) in [
            (
                "max_storage_keys",
                self.max_storage_keys,
                MAX_QUERY_STORAGE_KEYS,
            ),
            ("max_rows", self.max_rows, MAX_QUERY_ROWS),
            (
                "max_output_bytes",
                self.max_output_bytes,
                MAX_QUERY_OUTPUT_BYTES,
            ),
            ("max_batch_rows", self.max_batch_rows, MAX_QUERY_BATCH_ROWS),
            (
                "max_memory_bytes",
                self.max_memory_bytes,
                MAX_QUERY_MEMORY_BYTES,
            ),
            (
                "max_spill_bytes",
                self.max_spill_bytes,
                MAX_QUERY_SPILL_BYTES,
            ),
            ("max_elapsed_ms", self.max_elapsed_ms, MAX_QUERY_ELAPSED_MS),
        ] {
            if value == 0 || value > maximum {
                return invalid(format!("query {name} must be in 1..={maximum}"));
            }
        }
        Ok(())
    }
}

const fn default_query_memory_bytes() -> u64 {
    64 * 1024 * 1024
}

const fn default_query_spill_bytes() -> u64 {
    256 * 1024 * 1024
}

const fn default_query_elapsed_ms() -> u64 {
    30_000
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExecuteQuery {
    pub scope: String,
    pub query: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub parameters: BTreeMap<String, QueryValue>,
    #[serde(default)]
    pub budget: QueryBudget,
}

impl ExecuteQuery {
    pub fn validate(&self) -> Result<()> {
        if self.scope.is_empty() || self.scope.len() > MAX_ID_BYTES || self.scope.contains('\0') {
            return invalid(format!(
                "query scope length must be in 1..={MAX_ID_BYTES} bytes and contain no NUL"
            ));
        }
        if self.query.trim().is_empty() || self.query.len() > MAX_QUERY_BYTES {
            return invalid(format!(
                "query text length must be in 1..={MAX_QUERY_BYTES} bytes"
            ));
        }
        if self.parameters.len() > MAX_QUERY_PARAMETERS {
            return invalid(format!(
                "query parameters may contain at most {MAX_QUERY_PARAMETERS} entries"
            ));
        }
        if self.parameters.values().any(|value| {
            !matches!(
                value,
                QueryValue::Null
                    | QueryValue::Bool(_)
                    | QueryValue::Integer(_)
                    | QueryValue::Unsigned(_)
                    | QueryValue::String(_)
            )
        }) {
            return invalid(
                "query parameters support only null, boolean, integer, unsigned, and string values",
            );
        }
        let parameter_bytes = serde_json::to_vec(&self.parameters)
            .map_err(|error| ContractError(error.to_string()))?
            .len();
        if parameter_bytes > MAX_QUERY_PARAMETER_BYTES {
            return invalid(format!(
                "query parameters may encode at most {MAX_QUERY_PARAMETER_BYTES} bytes"
            ));
        }
        self.budget.validate()
    }
}

/// A bounded rrflowQL transaction program plus canonical typed mutation
/// bindings. The program selects ordering and commit/cancel disposition; the
/// bindings reuse the one public transaction mutation vocabulary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExecuteQueryTransaction {
    pub scope: String,
    pub program: String,
    pub mutation_bindings: BTreeMap<String, TransactionMutation>,
    pub timeout_ms: u64,
}

impl ExecuteQueryTransaction {
    pub fn validate(&self) -> Result<()> {
        if self.scope.is_empty() || self.scope.len() > MAX_ID_BYTES || self.scope.contains('\0') {
            return invalid(format!(
                "query transaction scope length must be in 1..={MAX_ID_BYTES} bytes and contain no NUL"
            ));
        }
        if self.program.trim().is_empty() || self.program.len() > MAX_QUERY_BYTES {
            return invalid(format!(
                "query transaction program length must be in 1..={MAX_QUERY_BYTES} bytes"
            ));
        }
        if self.mutation_bindings.is_empty()
            || self.mutation_bindings.len() > MAX_QUERY_TRANSACTION_MUTATIONS
        {
            return invalid(format!(
                "query transaction binding count must be in 1..={MAX_QUERY_TRANSACTION_MUTATIONS}"
            ));
        }
        if self.mutation_bindings.keys().any(|name| {
            name.is_empty()
                || name.len() > MAX_ID_BYTES
                || !name
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.'))
        }) {
            return invalid("query transaction binding names must be bounded identifiers");
        }
        for mutation in self.mutation_bindings.values() {
            mutation.validate()?;
        }
        let bytes = serde_json::to_vec(&self.mutation_bindings)
            .map_err(|error| ContractError(error.to_string()))?
            .len();
        if bytes > MAX_QUERY_TRANSACTION_BINDING_BYTES {
            return invalid(format!(
                "query transaction bindings may encode at most {MAX_QUERY_TRANSACTION_BINDING_BYTES} bytes"
            ));
        }
        if !(MIN_LEASE_MS..=MAX_LEASE_MS).contains(&self.timeout_ms) {
            return invalid(format!(
                "query transaction timeout_ms must be in {MIN_LEASE_MS}..={MAX_LEASE_MS}"
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct QueryTransactionResult {
    pub canonical_program: String,
    pub transaction_id: CorrelationId,
    pub read_cursor: u64,
    pub state: TransactionState,
    pub mutation_count: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commit: Option<CommitReceipt>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct QueryPlanCandidate {
    pub name: String,
    pub selected: bool,
    pub exact: bool,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct QueryPlanSnapshot {
    pub plan_sha256: String,
    /// Revision of the security authority compiled before query binding. Zero
    /// denotes an explicitly unsecured loopback development engine.
    pub security_policy_revision: u64,
    pub authorization_sha256: String,
    pub exact: bool,
    pub deterministic_order: String,
    pub authorization_boundary: String,
    pub candidates: Vec<QueryPlanCandidate>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct QueryRowSnapshot {
    pub identity: String,
    pub values: BTreeMap<String, QueryValue>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct QueryExecutionSnapshot {
    pub selected_versions: u64,
    pub read_evidence: ReadEvidence,
    pub returned_rows: u64,
    pub output_bytes: u64,
    pub truncated: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub analysis: Option<QueryExecutionAnalysisSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct QueryExecutionAnalysisSnapshot {
    pub engine: String,
    pub provider_scans: u64,
    pub input_rows: u64,
    pub input_batches: u64,
    pub input_memory_bytes: u64,
    pub output_batches: u64,
    pub projection_pushdown: String,
    pub filter_pushdown: String,
    pub limit_pushdown: String,
    pub physical_operators: u64,
    pub peak_memory_bytes: u64,
    pub spill_count: u64,
    pub spilled_bytes: u64,
    pub spilled_rows: u64,
    pub elapsed_micros: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct QueryResult {
    pub canonical_query: String,
    pub scope: String,
    pub read_manifest_sha256: String,
    pub known_at_cursor: u64,
    pub schema_revision: u64,
    pub plan: QueryPlanSnapshot,
    pub execution: QueryExecutionSnapshot,
    pub rows: Vec<QueryRowSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PollLiveQuery {
    pub scope: String,
    pub query: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub parameters: BTreeMap<String, QueryValue>,
    pub after_cursor: u64,
    #[serde(default)]
    pub budget: QueryBudget,
    pub max_delta_rows: u64,
    #[serde(default)]
    pub wait_timeout_ms: u64,
}

impl PollLiveQuery {
    pub fn validate(&self) -> Result<()> {
        ExecuteQuery {
            scope: self.scope.clone(),
            query: self.query.clone(),
            parameters: self.parameters.clone(),
            budget: self.budget.clone(),
        }
        .validate()?;
        if self.max_delta_rows == 0 || self.max_delta_rows > MAX_LIVE_QUERY_DELTA_ROWS {
            return invalid(format!(
                "live query max_delta_rows must be in 1..={MAX_LIVE_QUERY_DELTA_ROWS}"
            ));
        }
        if self.wait_timeout_ms > MAX_LIVE_QUERY_WAIT_MS {
            return invalid(format!(
                "live query wait_timeout_ms must be in 0..={MAX_LIVE_QUERY_WAIT_MS}"
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LiveQueryRowChange {
    pub before: QueryRowSnapshot,
    pub after: QueryRowSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LiveQueryDeltaResult {
    pub timed_out: bool,
    pub waited_ms: u64,
    pub query_sha256: String,
    pub from_cursor: u64,
    pub through_cursor: u64,
    pub head_cursor: u64,
    pub added: Vec<QueryRowSnapshot>,
    pub updated: Vec<LiveQueryRowChange>,
    pub removed: Vec<QueryRowSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EnsureQueryIndex {
    pub scope: String,
    pub index_id: CanonicalId,
    pub definition_query: String,
    #[serde(default)]
    pub unique: bool,
    #[serde(default)]
    pub kind: QueryIndexKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub full_text: Option<QueryFullTextConfiguration>,
    #[serde(default)]
    pub budget: QueryBudget,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryIndexKind {
    #[default]
    Scalar,
    Count,
    Geo,
    MaterializedView,
    AggregateCount,
    Bm25,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryTextAnalyzer {
    #[default]
    UnicodeLowercase,
    UnicodeCaseSensitive,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryTextTokenizer {
    #[default]
    UnicodeAlphanumeric,
    Whitespace,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryTextStemmer {
    #[default]
    None,
    English,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct QueryFullTextConfiguration {
    #[serde(default)]
    pub analyzer: QueryTextAnalyzer,
    #[serde(default)]
    pub tokenizer: QueryTextTokenizer,
    #[serde(default)]
    pub ascii_folding: bool,
    #[serde(default)]
    pub stop_words: BTreeSet<String>,
    #[serde(default = "default_query_min_token_chars")]
    pub min_token_chars: u16,
    #[serde(default = "default_query_max_token_chars")]
    pub max_token_chars: u16,
    #[serde(default)]
    pub stemmer: QueryTextStemmer,
    #[serde(default = "default_query_bm25_k1_micros")]
    pub k1_micros: u32,
    #[serde(default = "default_query_bm25_b_micros")]
    pub b_micros: u32,
}

impl Default for QueryFullTextConfiguration {
    fn default() -> Self {
        Self {
            analyzer: QueryTextAnalyzer::default(),
            tokenizer: QueryTextTokenizer::default(),
            ascii_folding: false,
            stop_words: BTreeSet::new(),
            min_token_chars: default_query_min_token_chars(),
            max_token_chars: default_query_max_token_chars(),
            stemmer: QueryTextStemmer::default(),
            k1_micros: default_query_bm25_k1_micros(),
            b_micros: default_query_bm25_b_micros(),
        }
    }
}

impl QueryFullTextConfiguration {
    fn validate(&self) -> Result<()> {
        if self.min_token_chars == 0
            || self.min_token_chars > self.max_token_chars
            || self.max_token_chars > 40
            || self.k1_micros == 0
            || self.k1_micros > 10_000_000
            || self.b_micros > 1_000_000
            || self.stop_words.len() > 10_000
            || self.stop_words.iter().any(|word| word.is_empty())
        {
            return invalid("full-text index configuration is outside supported bounds");
        }
        Ok(())
    }
}

const fn default_query_min_token_chars() -> u16 {
    1
}
const fn default_query_max_token_chars() -> u16 {
    40
}
const fn default_query_bm25_k1_micros() -> u32 {
    1_200_000
}
const fn default_query_bm25_b_micros() -> u32 {
    750_000
}

impl EnsureQueryIndex {
    pub fn validate(&self) -> Result<()> {
        match (self.kind, &self.full_text) {
            (QueryIndexKind::Bm25, Some(config)) => config.validate()?,
            (QueryIndexKind::Bm25, None) | (_, None) => {}
            (_, Some(_)) => return invalid("full_text configuration requires kind bm25"),
        }
        ExecuteQuery {
            scope: self.scope.clone(),
            query: self.definition_query.clone(),
            parameters: BTreeMap::new(),
            budget: self.budget.clone(),
        }
        .validate()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ListQueryIndexes {
    pub scope: String,
}

impl ListQueryIndexes {
    pub fn validate(&self) -> Result<()> {
        if self.scope.is_empty() || self.scope.len() > MAX_ID_BYTES || self.scope.contains('\0') {
            return invalid(format!(
                "query index scope length must be in 1..={MAX_ID_BYTES} bytes and contain no NUL"
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryIndexState {
    Building,
    Ready,
    Quarantined,
    Retiring,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct QueryIndexSnapshot {
    pub index_id: CanonicalId,
    pub definition_query: String,
    pub unique: bool,
    #[serde(default)]
    pub kind: QueryIndexKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub full_text: Option<QueryFullTextConfiguration>,
    pub generation: u64,
    pub source_cursor: u64,
    pub built_valid_at: Option<u64>,
    pub artifact_rows: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub analytics_total_count: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub analytics_group_count: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maintenance: Option<QueryIndexMaintenanceSnapshot>,
    pub configuration_sha256: String,
    pub artifact_sha256: String,
    pub state: QueryIndexState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct QueryIndexMaintenanceSnapshot {
    pub mode: String,
    pub prior_source_cursor: Option<u64>,
    pub source_cursor: u64,
    pub inserted_rows: u64,
    pub updated_rows: u64,
    pub removed_rows: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct QueryIndexCatalogueSnapshot {
    pub scope: String,
    pub revision: u64,
    pub indexes: Vec<QueryIndexSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EnsureQueryIndexResult {
    pub index: QueryIndexSnapshot,
    pub catalogue_revision: u64,
    pub selected_versions: u64,
    pub read_evidence: ReadEvidence,
    pub idempotent_replay: bool,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum VectorSearchMetric {
    Cosine,
    Dot,
    Euclidean,
    Manhattan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MultiVectorComparator {
    MaxSim,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum VectorValueKind {
    Dense,
    Sparse,
    MultiDense,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum VectorMemoryTier {
    Pinned,
    Cached,
    Cold,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VectorEmbeddingModel {
    pub name: String,
    pub digest: String,
}

impl VectorEmbeddingModel {
    fn validate(&self) -> Result<()> {
        if self.name.trim().is_empty()
            || self.name.len() > MAX_MESSAGE_BYTES
            || self.name.contains('\0')
        {
            return invalid("vector embedding model name is invalid");
        }
        validate_sha256(&self.digest, "vector embedding model digest")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NamedVectorDefinition {
    pub name: CanonicalId,
    pub field: CanonicalId,
    pub kind: VectorValueKind,
    pub dimensions: u32,
    pub metric: VectorSearchMetric,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embedding_model: Option<VectorEmbeddingModel>,
    pub memory_tier: VectorMemoryTier,
}

impl NamedVectorDefinition {
    fn validate(&self) -> Result<()> {
        if self.dimensions == 0 || u64::from(self.dimensions) > MAX_VECTOR_DIMENSIONS as u64 {
            return invalid(format!(
                "named-vector dimensions must be in 1..={MAX_VECTOR_DIMENSIONS}"
            ));
        }
        if let Some(model) = &self.embedding_model {
            model.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EnsureVectorCollection {
    pub scope: String,
    pub collection_id: CanonicalId,
    pub vectors: Vec<NamedVectorDefinition>,
}

impl EnsureVectorCollection {
    pub fn validate(&self) -> Result<()> {
        validate_vector_scope(&self.scope)?;
        if self.vectors.is_empty() || self.vectors.len() > 64 {
            return invalid("vector collection must contain 1..=64 named vectors");
        }
        let mut names = BTreeSet::new();
        let mut fields = BTreeSet::new();
        for vector in &self.vectors {
            vector.validate()?;
            if !names.insert(vector.name.as_str()) || !fields.insert(vector.field.as_str()) {
                return invalid("vector collection names and fields must be unique");
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ListVectorCollections {
    pub scope: String,
}

impl ListVectorCollections {
    pub fn validate(&self) -> Result<()> {
        validate_vector_scope(&self.scope)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VectorCollectionSnapshot {
    pub collection_id: CanonicalId,
    pub vectors: Vec<NamedVectorDefinition>,
    #[serde(default)]
    pub payload_indexes: Vec<VectorPayloadIndexSnapshot>,
    pub generation: u64,
    pub created_at_unix_ms: u64,
    pub updated_at_unix_ms: u64,
    pub configuration_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VectorCollectionCatalogueSnapshot {
    pub scope: String,
    pub revision: u64,
    pub collections: Vec<VectorCollectionSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EnsureVectorCollectionResult {
    pub collection: VectorCollectionSnapshot,
    pub catalogue_revision: u64,
    pub idempotent_replay: bool,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum VectorPayloadIndexKind {
    Boolean,
    Integer,
    Unsigned,
    Decimal,
    Keyword,
    Digest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VectorPayloadIndexSnapshot {
    pub field: CanonicalId,
    pub kind: VectorPayloadIndexKind,
    pub generation: u64,
    pub created_at_unix_ms: u64,
    pub updated_at_unix_ms: u64,
    pub configuration_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EnsureVectorPayloadIndex {
    pub scope: String,
    pub collection_id: CanonicalId,
    pub field: CanonicalId,
    pub kind: VectorPayloadIndexKind,
}

impl EnsureVectorPayloadIndex {
    pub fn validate(&self) -> Result<()> {
        validate_vector_scope(&self.scope)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DeleteVectorPayloadIndex {
    pub scope: String,
    pub collection_id: CanonicalId,
    pub field: CanonicalId,
}

impl DeleteVectorPayloadIndex {
    pub fn validate(&self) -> Result<()> {
        validate_vector_scope(&self.scope)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ListVectorPayloadIndexes {
    pub scope: String,
    pub collection_id: CanonicalId,
}

impl ListVectorPayloadIndexes {
    pub fn validate(&self) -> Result<()> {
        validate_vector_scope(&self.scope)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VectorPayloadIndexCatalogueSnapshot {
    pub scope: String,
    pub collection_id: CanonicalId,
    pub collection_generation: u64,
    pub catalogue_revision: u64,
    pub indexes: Vec<VectorPayloadIndexSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EnsureVectorPayloadIndexResult {
    pub collection: VectorCollectionSnapshot,
    pub index: VectorPayloadIndexSnapshot,
    pub catalogue_revision: u64,
    pub idempotent_replay: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DeleteVectorPayloadIndexResult {
    pub collection: VectorCollectionSnapshot,
    pub deleted_index: VectorPayloadIndexSnapshot,
    pub catalogue_revision: u64,
    pub idempotent_replay: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DeleteVectorCollection {
    pub scope: String,
    pub collection_id: CanonicalId,
    pub valid_at: u64,
    pub max_storage_keys: u64,
}

impl DeleteVectorCollection {
    pub fn validate(&self) -> Result<()> {
        validate_vector_scope(&self.scope)?;
        if self.valid_at == 0 {
            return invalid("vector collection delete valid_at must be greater than zero");
        }
        if self.max_storage_keys == 0 || self.max_storage_keys > MAX_VECTOR_STORAGE_KEYS {
            return invalid(format!(
                "vector collection delete max_storage_keys must be in 1..={MAX_VECTOR_STORAGE_KEYS}"
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DeleteVectorCollectionResult {
    pub deleted_collection: VectorCollectionSnapshot,
    pub catalogue_revision: u64,
    pub selected_versions: u64,
    pub read_evidence: ReadEvidence,
    pub idempotent_replay: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum VectorIndexConfiguration {
    Hnsw {
        m: u32,
        ef_construction: u64,
        max_level: u8,
        seed: u64,
        #[serde(default)]
        filter_properties: Vec<CanonicalId>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum VectorQuantizationBits {
    Bits4,
    Bits2,
    Bits1_5,
    Bits1,
}

impl VectorIndexConfiguration {
    fn validate(&self) -> Result<()> {
        match self {
            Self::Hnsw {
                m,
                ef_construction,
                max_level,
                filter_properties,
                ..
            } => {
                if !(2..=128).contains(m)
                    || *ef_construction < u64::from(*m)
                    || *ef_construction > 1_000_000
                    || !(1..=32).contains(max_level)
                {
                    return invalid("HNSW index parameters are outside supported bounds");
                }
                let unique = filter_properties
                    .iter()
                    .map(CanonicalId::as_str)
                    .collect::<BTreeSet<_>>();
                if unique.len() != filter_properties.len() {
                    return invalid("HNSW filter properties must be unique");
                }
                Ok(())
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VectorIndexBuildResourceEvidence {
    pub input_vectors: u64,
    pub dimensions: u64,
    pub input_values: u64,
    pub cpu_artifact_bytes: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accelerator_artifact_bytes: Option<u64>,
    pub semantic_probe_queries: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum VectorIndexDifferentialStatus {
    NotRun,
    Passed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum VectorIndexBuildTarget {
    Cpu,
    Gpu { platform: String, device: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VectorIndexBuildEvidence {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requested_backend_id: Option<CanonicalId>,
    pub selected_backend_id: String,
    pub selected_target: VectorIndexBuildTarget,
    pub used_fallback: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback_reason: Option<String>,
    pub byte_differential: VectorIndexDifferentialStatus,
    pub semantic_differential: VectorIndexDifferentialStatus,
    pub resources: VectorIndexBuildResourceEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum VectorIndexBuildPolicy {
    #[default]
    Cpu,
    PreferGpu {
        backend_id: CanonicalId,
        allow_cpu_fallback: bool,
    },
    RequireGpu {
        backend_id: CanonicalId,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EnsureVectorIndex {
    pub scope: String,
    pub collection_id: CanonicalId,
    pub vector_name: CanonicalId,
    pub configuration: VectorIndexConfiguration,
    #[serde(default)]
    pub build_policy: VectorIndexBuildPolicy,
    pub max_storage_keys: u64,
}

impl EnsureVectorIndex {
    pub fn validate(&self) -> Result<()> {
        validate_vector_scope(&self.scope)?;
        self.configuration.validate()?;
        if self.max_storage_keys == 0 || self.max_storage_keys > MAX_VECTOR_STORAGE_KEYS {
            return invalid(format!(
                "vector index max_storage_keys must be in 1..={MAX_VECTOR_STORAGE_KEYS}"
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VectorIndexMaintenanceSnapshot {
    pub mode: VectorIndexMaintenanceMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_generation: Option<u64>,
    pub indexed_delta_vectors: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum VectorIndexMaintenanceMode {
    FullBuild,
    Incremental,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VectorIndexSnapshot {
    pub index_id: CanonicalId,
    pub collection_id: CanonicalId,
    pub vector_name: CanonicalId,
    pub kind: CanonicalId,
    pub generation: u64,
    pub source_cursor: u64,
    pub indexed_vectors: u64,
    pub maintenance: VectorIndexMaintenanceSnapshot,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub packed_vector_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub full_precision_vector_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub build_evidence: Option<VectorIndexBuildEvidence>,
    pub configuration_sha256: String,
    pub artifact_sha256: String,
    pub object_sha256: String,
    pub catalogue_revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EnsureVectorIndexResult {
    pub index: VectorIndexSnapshot,
    pub selected_versions: u64,
    pub read_evidence: ReadEvidence,
    pub idempotent_replay: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum VectorProductCompression {
    X4,
    X8,
    X16,
    X32,
    X64,
}

impl VectorProductCompression {
    pub const fn ratio(self) -> u64 {
        match self {
            Self::X4 => 4,
            Self::X8 => 8,
            Self::X16 => 16,
            Self::X32 => 32,
            Self::X64 => 64,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "method", rename_all = "snake_case", deny_unknown_fields)]
pub enum VectorQuantizationMethod {
    Scalar,
    Product {
        compression: VectorProductCompression,
    },
    Binary,
    TurboQuant {
        bits: VectorQuantizationBits,
        seed: u64,
    },
}

impl VectorQuantizationMethod {
    pub const fn maximum_compression_ratio(&self) -> u64 {
        match self {
            Self::Scalar => 4,
            Self::Product { compression } => compression.ratio(),
            Self::Binary
            | Self::TurboQuant {
                bits: VectorQuantizationBits::Bits1,
                ..
            } => 32,
            Self::TurboQuant {
                bits: VectorQuantizationBits::Bits1_5,
                ..
            } => 21,
            Self::TurboQuant {
                bits: VectorQuantizationBits::Bits2,
                ..
            } => 16,
            Self::TurboQuant {
                bits: VectorQuantizationBits::Bits4,
                ..
            } => 8,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum VectorQuantizationArtifactState {
    Ready,
    Active,
    Retired,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BuildVectorQuantizationArtifact {
    pub scope: String,
    pub collection_id: CanonicalId,
    pub vector_name: CanonicalId,
    pub method: VectorQuantizationMethod,
    #[serde(default)]
    pub filter_properties: Vec<CanonicalId>,
    pub max_storage_keys: u64,
}

impl BuildVectorQuantizationArtifact {
    pub fn validate(&self) -> Result<()> {
        validate_vector_scope(&self.scope)?;
        if self.max_storage_keys == 0 || self.max_storage_keys > MAX_VECTOR_STORAGE_KEYS {
            return invalid(format!(
                "quantization build max_storage_keys must be in 1..={MAX_VECTOR_STORAGE_KEYS}"
            ));
        }
        let unique = self
            .filter_properties
            .iter()
            .map(CanonicalId::as_str)
            .collect::<BTreeSet<_>>();
        if unique.len() != self.filter_properties.len() {
            return invalid("quantization filter properties must be unique");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ListVectorQuantizationArtifacts {
    pub scope: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collection_id: Option<CanonicalId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vector_name: Option<CanonicalId>,
    pub max_artifacts: u64,
}

impl ListVectorQuantizationArtifacts {
    pub fn validate(&self) -> Result<()> {
        validate_vector_scope(&self.scope)?;
        if self.max_artifacts == 0 || self.max_artifacts > 100_000 {
            return invalid("quantization list max_artifacts must be in 1..=100000");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ActivateVectorQuantizationArtifact {
    pub scope: String,
    pub artifact_id: CanonicalId,
    pub generation: u64,
}

impl ActivateVectorQuantizationArtifact {
    pub fn validate(&self) -> Result<()> {
        validate_vector_scope(&self.scope)?;
        if self.generation == 0 {
            return invalid("quantization activation generation must be greater than zero");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RetireVectorQuantizationArtifact {
    pub scope: String,
    pub artifact_id: CanonicalId,
    pub generation: u64,
}

impl RetireVectorQuantizationArtifact {
    pub fn validate(&self) -> Result<()> {
        validate_vector_scope(&self.scope)?;
        if self.generation == 0 {
            return invalid("quantization retirement generation must be greater than zero");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VectorQuantizationArtifactSnapshot {
    pub artifact_id: CanonicalId,
    pub collection_id: CanonicalId,
    pub vector_name: CanonicalId,
    pub method: VectorQuantizationMethod,
    pub state: VectorQuantizationArtifactState,
    pub generation: u64,
    pub source_cursor: u64,
    pub indexed_vectors: u64,
    pub packed_vector_bytes: u64,
    pub full_precision_vector_bytes: u64,
    pub auxiliary_bytes: u64,
    pub maximum_compression_ratio: u64,
    pub configuration_sha256: String,
    pub artifact_sha256: String,
    pub object_sha256: String,
    pub object_length: u64,
    pub built_at_unix_ms: u64,
    pub lifecycle_revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BuildVectorQuantizationArtifactResult {
    pub artifact: VectorQuantizationArtifactSnapshot,
    pub selected_versions: u64,
    pub read_evidence: ReadEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ListVectorQuantizationArtifactsResult {
    pub scope: String,
    pub lifecycle_revision: u64,
    pub artifacts: Vec<VectorQuantizationArtifactSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ActivateVectorQuantizationArtifactResult {
    pub artifact: VectorQuantizationArtifactSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RetireVectorQuantizationArtifactResult {
    pub artifact: VectorQuantizationArtifactSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "operator", rename_all = "snake_case", deny_unknown_fields)]
pub enum VectorPayloadOperator {
    Equals {
        value: QueryValue,
    },
    NotEquals {
        value: QueryValue,
    },
    In {
        values: Vec<QueryValue>,
    },
    Range {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        gt: Option<QueryValue>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        gte: Option<QueryValue>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        lt: Option<QueryValue>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        lte: Option<QueryValue>,
    },
    Exists {
        value: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VectorPayloadCondition {
    pub property: CanonicalId,
    pub operator: VectorPayloadOperator,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum VectorPayloadFilter {
    Condition { condition: VectorPayloadCondition },
    All { filters: Vec<VectorPayloadFilter> },
    Any { filters: Vec<VectorPayloadFilter> },
    Not { filter: Box<VectorPayloadFilter> },
}

impl VectorPayloadFilter {
    pub fn validate(&self) -> Result<()> {
        let mut nodes = 0_usize;
        self.validate_at(0, &mut nodes)
    }

    fn validate_at(&self, depth: usize, nodes: &mut usize) -> Result<()> {
        *nodes += 1;
        if depth > 32 || *nodes > 4_096 {
            return invalid("vector payload filter exceeds depth or node bounds");
        }
        match self {
            Self::Condition { condition } => match &condition.operator {
                VectorPayloadOperator::In { values } if values.is_empty() => {
                    invalid("vector payload in filter requires values")
                }
                VectorPayloadOperator::Range { gt, gte, lt, lte } => {
                    if (gt.is_some() && gte.is_some())
                        || (lt.is_some() && lte.is_some())
                        || (gt.is_none() && gte.is_none() && lt.is_none() && lte.is_none())
                    {
                        return invalid("vector payload range bounds are invalid");
                    }
                    Ok(())
                }
                _ => Ok(()),
            },
            Self::All { filters } | Self::Any { filters } => {
                if filters.is_empty() {
                    return invalid("vector payload all/any filter requires children");
                }
                for filter in filters {
                    filter.validate_at(depth + 1, nodes)?;
                }
                Ok(())
            }
            Self::Not { filter } => filter.validate_at(depth + 1, nodes),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum VectorSearchQuery {
    Dense {
        values: Vec<f32>,
    },
    Sparse {
        dimensions: u32,
        indices: Vec<u32>,
        values: Vec<f32>,
    },
    MultiDense {
        dimensions: u32,
        vectors: Vec<Vec<f32>>,
        comparator: MultiVectorComparator,
    },
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
pub enum VectorSearchMode {
    #[default]
    Exact,
    AllowApproximate {
        exact_rerank: u64,
        ef_search: u64,
    },
    RequireApproximate {
        exact_rerank: u64,
        ef_search: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SearchVectors {
    pub scope: String,
    pub valid_at: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collection_id: Option<CanonicalId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vector_name: Option<CanonicalId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub field: Option<CanonicalId>,
    pub query: VectorSearchQuery,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter: Option<VectorPayloadFilter>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metric: Option<VectorSearchMetric>,
    pub top_k: u64,
    #[serde(default)]
    pub mode: VectorSearchMode,
    pub max_storage_keys: u64,
}

impl SearchVectors {
    pub fn validate(&self) -> Result<()> {
        validate_vector_scope(&self.scope)?;
        let collection_address = self.collection_id.is_some() && self.vector_name.is_some();
        let field_address = self.field.is_some() && self.metric.is_some();
        if collection_address == field_address
            || self.collection_id.is_some() != self.vector_name.is_some()
            || self.field.is_some() != self.metric.is_some()
        {
            return invalid(
                "vector search must use exactly one complete collection/vector or field/metric address",
            );
        }
        if self.valid_at == 0 {
            return invalid("vector search valid_at must be greater than zero");
        }
        if self.top_k == 0 || self.top_k > MAX_VECTOR_SEARCH_TOP_K {
            return invalid(format!(
                "vector search top_k must be in 1..={MAX_VECTOR_SEARCH_TOP_K}"
            ));
        }
        match self.mode {
            VectorSearchMode::Exact => {}
            VectorSearchMode::AllowApproximate {
                exact_rerank,
                ef_search,
            }
            | VectorSearchMode::RequireApproximate {
                exact_rerank,
                ef_search,
            } => {
                if exact_rerank < self.top_k
                    || exact_rerank > MAX_VECTOR_SEARCH_WORK
                    || ef_search < exact_rerank
                    || ef_search > MAX_VECTOR_SEARCH_WORK
                {
                    return invalid(
                        "approximate vector search requires top_k <= exact_rerank <= ef_search within the search bound",
                    );
                }
            }
        }
        if self.max_storage_keys == 0 || self.max_storage_keys > MAX_VECTOR_STORAGE_KEYS {
            return invalid(format!(
                "vector search max_storage_keys must be in 1..={MAX_VECTOR_STORAGE_KEYS}"
            ));
        }
        let vector = match &self.query {
            VectorSearchQuery::Dense { values } => DataVectorValue::Dense {
                values: values.clone(),
            },
            VectorSearchQuery::Sparse {
                dimensions,
                indices,
                values,
            } => DataVectorValue::Sparse {
                dimensions: *dimensions,
                indices: indices.clone(),
                values: values.clone(),
            },
            VectorSearchQuery::MultiDense {
                dimensions,
                vectors,
                ..
            } => DataVectorValue::MultiDense {
                dimensions: *dimensions,
                vectors: vectors.clone(),
            },
        };
        validate_data_vector(&vector)?;
        if let Some(filter) = &self.filter {
            filter.validate()?;
        }
        if self.metric == Some(VectorSearchMetric::Cosine) {
            let zero = match &self.query {
                VectorSearchQuery::Dense { values } | VectorSearchQuery::Sparse { values, .. } => {
                    values.iter().all(|value| *value == 0.0)
                }
                VectorSearchQuery::MultiDense { vectors, .. } => vectors
                    .iter()
                    .any(|values| values.iter().all(|value| *value == 0.0)),
            };
            if zero {
                return invalid("cosine vector search rejects zero-norm query rows");
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VectorSearchHit {
    pub reference: DataReference,
    pub subject: DataReference,
    pub source_cursor: u64,
    pub score: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VectorSearchResult {
    pub scope: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collection_id: Option<CanonicalId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vector_name: Option<CanonicalId>,
    pub read_manifest_sha256: String,
    pub known_at_cursor: u64,
    pub selected_versions: u64,
    pub read_evidence: ReadEvidence,
    pub plan_sha256: String,
    pub access_path: CanonicalId,
    pub exact: bool,
    pub resources: VectorSearchResourceEvidence,
    pub hits: Vec<VectorSearchHit>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VectorSearchResourceEvidence {
    pub canonical_candidates: u64,
    pub selected_candidates: u64,
    pub ef_search: u64,
    pub exact_rerank: u64,
    pub overlay_candidates: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_generation: Option<u64>,
    pub loaded_artifact_bytes: u64,
    pub result_hits: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum HybridFusion {
    ReciprocalRank {
        rank_constant: u32,
        text_weight_millionths: u32,
        vector_weight_millionths: u32,
    },
}

impl Default for HybridFusion {
    fn default() -> Self {
        Self::ReciprocalRank {
            rank_constant: 60,
            text_weight_millionths: 1_000_000,
            vector_weight_millionths: 1_000_000,
        }
    }
}

impl HybridFusion {
    fn validate(self) -> Result<()> {
        match self {
            Self::ReciprocalRank {
                rank_constant,
                text_weight_millionths,
                vector_weight_millionths,
            } => {
                if !(1..=10_000).contains(&rank_constant)
                    || !(1..=1_000_000).contains(&text_weight_millionths)
                    || !(1..=1_000_000).contains(&vector_weight_millionths)
                {
                    return invalid(
                        "hybrid reciprocal-rank parameters exceed their supported bounds",
                    );
                }
                Ok(())
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SearchHybrid {
    pub scope: String,
    pub valid_at: u64,
    pub document_kind: CanonicalId,
    pub text_field: CanonicalId,
    pub text_query: String,
    pub collection_id: CanonicalId,
    pub vector_name: CanonicalId,
    pub vector_query: VectorSearchQuery,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vector_filter: Option<VectorPayloadFilter>,
    #[serde(default)]
    pub vector_mode: VectorSearchMode,
    #[serde(default)]
    pub fusion: HybridFusion,
    pub top_k: u64,
    pub candidate_k: u64,
    pub max_storage_keys: u64,
}

impl SearchHybrid {
    pub fn validate(&self) -> Result<()> {
        validate_vector_scope(&self.scope)?;
        if self.valid_at == 0 {
            return invalid("hybrid search valid_at must be greater than zero");
        }
        if self.text_query.trim().is_empty() || self.text_query.len() > MAX_MESSAGE_BYTES {
            return invalid(format!(
                "hybrid text query length must be in 1..={MAX_MESSAGE_BYTES} bytes"
            ));
        }
        if self.top_k == 0
            || self.top_k > MAX_VECTOR_SEARCH_TOP_K
            || self.candidate_k < self.top_k
            || self.candidate_k > MAX_VECTOR_SEARCH_TOP_K
        {
            return invalid(
                "hybrid search requires 1 <= top_k <= candidate_k within the vector result bound",
            );
        }
        self.fusion.validate()?;
        SearchVectors {
            scope: self.scope.clone(),
            valid_at: self.valid_at,
            collection_id: Some(self.collection_id.clone()),
            vector_name: Some(self.vector_name.clone()),
            field: None,
            query: self.vector_query.clone(),
            filter: self.vector_filter.clone(),
            metric: None,
            top_k: self.candidate_k,
            mode: self.vector_mode,
            max_storage_keys: self.max_storage_keys,
        }
        .validate()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct HybridSearchHit {
    pub subject: DataReference,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vector_reference: Option<DataReference>,
    pub fused_score: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_score: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vector_score: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_rank: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vector_rank: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct HybridSearchResult {
    pub scope: String,
    pub read_manifest_sha256: String,
    pub known_at_cursor: u64,
    pub selected_versions: u64,
    pub read_evidence: ReadEvidence,
    pub text_plan_sha256: String,
    pub vector_plan_sha256: String,
    pub fusion_plan_sha256: String,
    pub text_access_path: CanonicalId,
    pub vector_access_path: CanonicalId,
    pub vector_exact: bool,
    pub text_candidates: u64,
    pub vector_candidates: u64,
    pub fusion: HybridFusion,
    pub hits: Vec<HybridSearchHit>,
}

const MAX_RETRIEVAL_QUERY_DEPTH: usize = 8;
const MAX_RETRIEVAL_QUERY_NODES: usize = 128;
const MAX_RETRIEVAL_PREFETCHES: usize = 16;
const MAX_RETRIEVAL_EXAMPLES: usize = 256;
const MAX_RETRIEVAL_RERANK_STAGES: usize = 16;

/// A stored vector version or caller-supplied vector used by recommendation
/// and discovery queries. Stored examples are vector-reference addressed so a
/// point with several named modalities is never ambiguous.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RetrievalVectorExample {
    Reference { reference: DataReference },
    Vector { value: VectorSearchQuery },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RetrievalContextPair {
    pub positive: RetrievalVectorExample,
    pub negative: RetrievalVectorExample,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RetrievalRecommendStrategy {
    AverageVector,
    BestScore,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RetrievalFusion {
    ReciprocalRank {
        rank_constant: u32,
        weights_millionths: Vec<u32>,
    },
}

impl RetrievalFusion {
    fn validate(&self, branches: usize) -> Result<()> {
        match self {
            Self::ReciprocalRank {
                rank_constant,
                weights_millionths,
            } => {
                if !(1..=10_000).contains(rank_constant)
                    || weights_millionths.len() != branches
                    || weights_millionths
                        .iter()
                        .any(|weight| !(1..=1_000_000).contains(weight))
                {
                    return invalid(
                        "retrieval RRF requires one bounded positive weight per prefetch",
                    );
                }
                Ok(())
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RetrievalRerankStage {
    ScoreBoost {
        filter: VectorPayloadFilter,
        add_millionths: i32,
        multiply_millionths: u32,
    },
    Exact {
        using: CanonicalId,
        query: VectorSearchQuery,
    },
    Model {
        using: CanonicalId,
        model: VectorEmbeddingModel,
        query: VectorSearchQuery,
    },
    Mmr {
        using: CanonicalId,
        diversity_millionths: u32,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RetrievalPrefetch {
    pub query: Box<RetrievalQuery>,
    pub limit: u64,
}

/// Recursive retrieval algebra. Leaf sources read one authoritative snapshot;
/// fusion and reranking consume only their declared bounded prefetch results.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RetrievalQuery {
    Nearest {
        using: CanonicalId,
        query: VectorSearchQuery,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        filter: Option<VectorPayloadFilter>,
        #[serde(default)]
        mode: VectorSearchMode,
    },
    Keyword {
        document_kind: CanonicalId,
        text_field: CanonicalId,
        query: String,
    },
    Recommend {
        using: CanonicalId,
        positive: Vec<RetrievalVectorExample>,
        #[serde(default)]
        negative: Vec<RetrievalVectorExample>,
        strategy: RetrievalRecommendStrategy,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        filter: Option<VectorPayloadFilter>,
    },
    Discover {
        using: CanonicalId,
        target: RetrievalVectorExample,
        context: Vec<RetrievalContextPair>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        filter: Option<VectorPayloadFilter>,
    },
    Context {
        using: CanonicalId,
        context: Vec<RetrievalContextPair>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        filter: Option<VectorPayloadFilter>,
    },
    Fusion {
        prefetch: Vec<RetrievalPrefetch>,
        fusion: RetrievalFusion,
    },
    Rerank {
        prefetch: Box<RetrievalPrefetch>,
        stages: Vec<RetrievalRerankStage>,
    },
}

impl RetrievalQuery {
    fn validate_at(
        &self,
        request: &ExecuteRetrievalQuery,
        limit: u64,
        depth: usize,
        nodes: &mut usize,
    ) -> Result<()> {
        *nodes += 1;
        if depth > MAX_RETRIEVAL_QUERY_DEPTH || *nodes > MAX_RETRIEVAL_QUERY_NODES {
            return invalid("retrieval query exceeds its depth or node bound");
        }
        let validate_vector = |using: &CanonicalId,
                               query: &VectorSearchQuery,
                               filter: Option<VectorPayloadFilter>,
                               mode: VectorSearchMode| {
            SearchVectors {
                scope: request.scope.clone(),
                valid_at: request.valid_at,
                collection_id: Some(request.collection_id.clone()),
                vector_name: Some(using.clone()),
                field: None,
                query: query.clone(),
                filter,
                metric: None,
                top_k: limit,
                mode,
                max_storage_keys: request.max_storage_keys,
            }
            .validate()
        };
        let validate_example = validate_retrieval_example;
        let validate_filter = |filter: &Option<VectorPayloadFilter>| {
            if let Some(filter) = filter {
                filter.validate()?;
            }
            Ok(())
        };
        match self {
            Self::Nearest {
                using,
                query,
                filter,
                mode,
            } => validate_vector(using, query, filter.clone(), *mode),
            Self::Keyword { query, .. } => {
                if query.trim().is_empty() || query.len() > MAX_MESSAGE_BYTES {
                    return invalid("retrieval keyword query is empty or exceeds its byte bound");
                }
                Ok(())
            }
            Self::Recommend {
                positive,
                negative,
                filter,
                ..
            } => {
                if positive.is_empty()
                    || positive.len().saturating_add(negative.len()) > MAX_RETRIEVAL_EXAMPLES
                {
                    return invalid(
                        "retrieval recommendation requires bounded positive/negative examples",
                    );
                }
                for example in positive.iter().chain(negative) {
                    validate_example(example)?;
                }
                validate_filter(filter)
            }
            Self::Discover {
                target,
                context,
                filter,
                ..
            } => {
                validate_example(target)?;
                validate_context(context, &validate_example)?;
                validate_filter(filter)
            }
            Self::Context {
                context, filter, ..
            } => {
                validate_context(context, &validate_example)?;
                validate_filter(filter)
            }
            Self::Fusion { prefetch, fusion } => {
                if !(2..=MAX_RETRIEVAL_PREFETCHES).contains(&prefetch.len()) {
                    return invalid("retrieval fusion requires 2..=16 prefetch branches");
                }
                fusion.validate(prefetch.len())?;
                for branch in prefetch {
                    branch.validate_at(request, depth + 1, nodes)?;
                }
                Ok(())
            }
            Self::Rerank { prefetch, stages } => {
                if stages.is_empty() || stages.len() > MAX_RETRIEVAL_RERANK_STAGES {
                    return invalid("retrieval rerank requires 1..=16 governed stages");
                }
                prefetch.validate_at(request, depth + 1, nodes)?;
                for stage in stages {
                    match stage {
                        RetrievalRerankStage::ScoreBoost {
                            filter,
                            add_millionths,
                            multiply_millionths,
                        } => {
                            filter.validate()?;
                            if add_millionths.unsigned_abs() > 10_000_000
                                || *multiply_millionths > 10_000_000
                            {
                                return invalid(
                                    "retrieval score boost exceeds its fixed-point bound",
                                );
                            }
                        }
                        RetrievalRerankStage::Exact { using, query } => {
                            validate_vector(using, query, None, VectorSearchMode::Exact)?;
                        }
                        RetrievalRerankStage::Model {
                            using,
                            model,
                            query,
                        } => {
                            model.validate()?;
                            validate_vector(using, query, None, VectorSearchMode::Exact)?;
                        }
                        RetrievalRerankStage::Mmr {
                            diversity_millionths,
                            ..
                        } if *diversity_millionths > 1_000_000 => {
                            return invalid("retrieval MMR diversity must be in 0..=1000000");
                        }
                        RetrievalRerankStage::Mmr { .. } => {}
                    }
                }
                Ok(())
            }
        }
    }
}

impl RetrievalPrefetch {
    fn validate_at(
        &self,
        request: &ExecuteRetrievalQuery,
        depth: usize,
        nodes: &mut usize,
    ) -> Result<()> {
        if self.limit == 0 || self.limit > request.candidate_limit {
            return invalid("retrieval prefetch limit must be in 1..=candidate_limit");
        }
        self.query.validate_at(request, self.limit, depth, nodes)
    }
}

fn validate_context(
    context: &[RetrievalContextPair],
    validate_example: &dyn Fn(&RetrievalVectorExample) -> Result<()>,
) -> Result<()> {
    if context.is_empty() || context.len().saturating_mul(2) > MAX_RETRIEVAL_EXAMPLES {
        return invalid("retrieval context requires a bounded non-empty pair set");
    }
    for pair in context {
        validate_example(&pair.positive)?;
        validate_example(&pair.negative)?;
    }
    Ok(())
}

fn validate_retrieval_example(example: &RetrievalVectorExample) -> Result<()> {
    let RetrievalVectorExample::Vector { value } = example else {
        return Ok(());
    };
    let value = match value {
        VectorSearchQuery::Dense { values } => DataVectorValue::Dense {
            values: values.clone(),
        },
        VectorSearchQuery::Sparse {
            dimensions,
            indices,
            values,
        } => DataVectorValue::Sparse {
            dimensions: *dimensions,
            indices: indices.clone(),
            values: values.clone(),
        },
        VectorSearchQuery::MultiDense {
            dimensions,
            vectors,
            ..
        } => DataVectorValue::MultiDense {
            dimensions: *dimensions,
            vectors: vectors.clone(),
        },
    };
    validate_data_vector(&value).map(|_| ())
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RetrievalResultShape {
    #[default]
    Points,
    Groups {
        property: CanonicalId,
        max_groups: u64,
        hits_per_group: u64,
    },
    Facets {
        property: CanonicalId,
        limit: u64,
    },
    Matrix {
        using: CanonicalId,
        sample: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExecuteRetrievalQuery {
    pub scope: String,
    pub valid_at: u64,
    pub collection_id: CanonicalId,
    pub query: RetrievalQuery,
    #[serde(default)]
    pub result_shape: RetrievalResultShape,
    pub limit: u64,
    pub candidate_limit: u64,
    pub max_storage_keys: u64,
}

impl ExecuteRetrievalQuery {
    pub fn validate(&self) -> Result<()> {
        validate_vector_scope(&self.scope)?;
        if self.valid_at == 0 {
            return invalid("retrieval query valid_at must be greater than zero");
        }
        if self.limit == 0
            || self.limit > MAX_VECTOR_SEARCH_TOP_K
            || self.candidate_limit < self.limit
            || self.candidate_limit > MAX_VECTOR_SEARCH_TOP_K
        {
            return invalid(
                "retrieval query requires 1 <= limit <= candidate_limit within the result bound",
            );
        }
        if self.max_storage_keys == 0 || self.max_storage_keys > MAX_VECTOR_STORAGE_KEYS {
            return invalid("retrieval query max_storage_keys exceeds its bound");
        }
        match self.result_shape {
            RetrievalResultShape::Points => {}
            RetrievalResultShape::Groups {
                max_groups,
                hits_per_group,
                ..
            } => {
                if max_groups == 0
                    || hits_per_group == 0
                    || max_groups
                        .checked_mul(hits_per_group)
                        .is_none_or(|rows| rows > self.candidate_limit)
                {
                    return invalid("retrieval group result exceeds its candidate bound");
                }
            }
            RetrievalResultShape::Facets { limit, .. } => {
                if limit == 0 || limit > self.candidate_limit {
                    return invalid("retrieval facet limit exceeds its candidate bound");
                }
            }
            RetrievalResultShape::Matrix { sample, .. } => {
                if !(2..=1_024).contains(&sample) || sample > self.candidate_limit {
                    return invalid("retrieval matrix sample must be in 2..=1024 and bounded");
                }
            }
        }
        let mut nodes = 0;
        self.query
            .validate_at(self, self.candidate_limit, 0, &mut nodes)?;
        if self
            .candidate_limit
            .checked_mul(nodes as u64)
            .is_none_or(|work| work > MAX_VECTOR_SEARCH_WORK)
        {
            return invalid("retrieval query candidate-stage work exceeds its bound");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RetrievalContribution {
    pub stage_ordinal: u32,
    pub rank: u64,
    pub score: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RetrievalHit {
    pub subject: DataReference,
    #[serde(default)]
    pub vector_references: Vec<DataReference>,
    pub score: f64,
    #[serde(default)]
    pub contributions: Vec<RetrievalContribution>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RetrievalGroup {
    pub value: QueryValue,
    pub hits: Vec<RetrievalHit>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RetrievalFacet {
    pub value: QueryValue,
    pub count: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RetrievalMatrixCell {
    pub left: DataReference,
    pub right: DataReference,
    pub score: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RetrievalOutput {
    Points {
        hits: Vec<RetrievalHit>,
    },
    Groups {
        groups: Vec<RetrievalGroup>,
    },
    Facets {
        facets: Vec<RetrievalFacet>,
    },
    Matrix {
        subjects: Vec<DataReference>,
        cells: Vec<RetrievalMatrixCell>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RetrievalStageEvidence {
    pub ordinal: u32,
    pub kind: CanonicalId,
    pub input_candidates: u64,
    pub output_candidates: u64,
    pub exact: bool,
    pub plan_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RetrievalQueryResult {
    pub scope: String,
    pub collection_id: CanonicalId,
    pub read_manifest_sha256: String,
    pub known_at_cursor: u64,
    pub selected_versions: u64,
    pub read_evidence: ReadEvidence,
    pub query_plan_sha256: String,
    pub stages: Vec<RetrievalStageEvidence>,
    pub output: RetrievalOutput,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ScrollVectorPoints {
    pub scope: String,
    pub collection_id: CanonicalId,
    pub vector_name: CanonicalId,
    pub valid_at: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after_reference: Option<DataReference>,
    pub limit: u64,
    pub max_storage_keys: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter: Option<VectorPayloadFilter>,
}

impl ScrollVectorPoints {
    pub fn validate(&self) -> Result<()> {
        validate_vector_scope(&self.scope)?;
        if self.valid_at == 0 {
            return invalid("vector point scroll valid_at must be greater than zero");
        }
        if self.limit == 0 || self.limit > MAX_VECTOR_POINT_PAGE {
            return invalid(format!(
                "vector point scroll limit must be in 1..={MAX_VECTOR_POINT_PAGE}"
            ));
        }
        if self.max_storage_keys == 0 || self.max_storage_keys > MAX_VECTOR_STORAGE_KEYS {
            return invalid(format!(
                "vector point scroll max_storage_keys must be in 1..={MAX_VECTOR_STORAGE_KEYS}"
            ));
        }
        if let Some(filter) = &self.filter {
            filter.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VectorPointSnapshot {
    pub reference: DataReference,
    pub subject: DataReference,
    pub source_cursor: u64,
    pub value: DataVectorValue,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance: Option<DataEmbeddingProvenance>,
    pub payload: DataProperties,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VectorPointPage {
    pub scope: String,
    pub collection_id: CanonicalId,
    pub vector_name: CanonicalId,
    pub read_manifest_sha256: String,
    pub known_at_cursor: u64,
    pub selected_versions: u64,
    pub read_evidence: ReadEvidence,
    pub points: Vec<VectorPointSnapshot>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_after: Option<DataReference>,
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RetrieveVectorPoints {
    pub scope: String,
    pub collection_id: CanonicalId,
    pub vector_name: CanonicalId,
    pub valid_at: u64,
    pub references: Vec<DataReference>,
    pub max_storage_keys: u64,
}

impl RetrieveVectorPoints {
    pub fn validate(&self) -> Result<()> {
        validate_vector_scope(&self.scope)?;
        if self.valid_at == 0 {
            return invalid("vector point retrieve valid_at must be greater than zero");
        }
        if self.references.is_empty() || self.references.len() > MAX_VECTOR_POINT_PAGE as usize {
            return invalid(format!(
                "vector point retrieve references must contain 1..={MAX_VECTOR_POINT_PAGE} identities"
            ));
        }
        if self.references.iter().collect::<BTreeSet<_>>().len() != self.references.len() {
            return invalid("vector point retrieve references must be unique");
        }
        if self.max_storage_keys == 0 || self.max_storage_keys > MAX_VECTOR_STORAGE_KEYS {
            return invalid(format!(
                "vector point retrieve max_storage_keys must be in 1..={MAX_VECTOR_STORAGE_KEYS}"
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VectorPointBatch {
    pub scope: String,
    pub collection_id: CanonicalId,
    pub vector_name: CanonicalId,
    pub read_manifest_sha256: String,
    pub known_at_cursor: u64,
    pub selected_versions: u64,
    pub read_evidence: ReadEvidence,
    pub points: Vec<VectorPointSnapshot>,
    pub missing: Vec<DataReference>,
}

fn validate_vector_scope(scope: &str) -> Result<()> {
    if scope.is_empty() || scope.len() > MAX_ID_BYTES || scope.as_bytes().contains(&0) {
        return invalid("vector scope is invalid");
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReadChangefeed {
    pub scope: String,
    pub after_cursor: u64,
    pub limit: u64,
}

impl ReadChangefeed {
    pub fn validate(&self) -> Result<()> {
        if self.scope.is_empty()
            || self.scope.len() > MAX_ID_BYTES
            || self.scope.as_bytes().contains(&0)
        {
            return invalid("changefeed scope is invalid");
        }
        if self.limit == 0 || self.limit > MAX_CHANGEFEED_PAGE {
            return invalid(format!(
                "changefeed limit must be in 1..={MAX_CHANGEFEED_PAGE}"
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ClaimTierSnapshot {
    Local,
    Primary,
    Tenant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ClaimPromotionSnapshot {
    Unpromoted,
    Pending,
    Promoted,
    Denied,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ClaimChangeSnapshot {
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub valid_from: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valid_to: Option<u64>,
    pub tx_time: u64,
    pub producer: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_behalf_of: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supersedes_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
    pub tier: ClaimTierSnapshot,
    pub promotion: ClaimPromotionSnapshot,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "family", rename_all = "snake_case", deny_unknown_fields)]
pub enum ChangeMutationSnapshot {
    Claim { claim: ClaimChangeSnapshot },
    Data { mutation: TransactionMutation },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RuntimeChangeSnapshot {
    pub cursor: u64,
    pub commit_sha256: String,
    pub commit_ordinal: u64,
    pub scope: String,
    pub at_unix_ms: u64,
    pub actor: String,
    pub mutation: ChangeMutationSnapshot,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_change_sha256: Option<String>,
    pub change_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ChangefeedValidation {
    pub method: String,
    pub change_reads: u64,
    pub proof_nodes: u16,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ChangefeedPage {
    pub requested_after_cursor: u64,
    pub through_cursor: u64,
    pub head_cursor: u64,
    pub has_more: bool,
    pub validation: ChangefeedValidation,
    pub changes: Vec<RuntimeChangeSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FollowChangefeed {
    pub read: ReadChangefeed,
    pub wait_timeout_ms: u64,
}

impl FollowChangefeed {
    pub fn validate(&self) -> Result<()> {
        self.read.validate()?;
        if self.wait_timeout_ms == 0 || self.wait_timeout_ms > MAX_CHANGEFEED_WAIT_MS {
            return invalid(format!(
                "changefeed wait_timeout_ms must be in 1..={MAX_CHANGEFEED_WAIT_MS}"
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ChangefeedFollowResult {
    pub timed_out: bool,
    pub waited_ms: u64,
    pub page: ChangefeedPage,
}

/// The immutable source definition owned by a durable push subscription.
/// Delivery state is deliberately separate: reconnecting never changes which
/// data a subscription means.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "stream", rename_all = "snake_case", deny_unknown_fields)]
pub enum SubscriptionStream {
    Changefeed {
        scope: String,
    },
    LiveQuery {
        scope: String,
        query: String,
        #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
        parameters: BTreeMap<String, QueryValue>,
        #[serde(default)]
        budget: QueryBudget,
        max_delta_rows: u64,
    },
}

impl SubscriptionStream {
    pub fn validate(&self, after_cursor: u64, batch_size: u64) -> Result<()> {
        match self {
            Self::Changefeed { scope } => ReadChangefeed {
                scope: scope.clone(),
                after_cursor,
                limit: batch_size,
            }
            .validate(),
            Self::LiveQuery {
                scope,
                query,
                parameters,
                budget,
                max_delta_rows,
            } => PollLiveQuery {
                scope: scope.clone(),
                query: query.clone(),
                parameters: parameters.clone(),
                after_cursor,
                budget: budget.clone(),
                max_delta_rows: *max_delta_rows,
                wait_timeout_ms: 0,
            }
            .validate(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OpenSubscription {
    pub subscription_id: CorrelationId,
    pub stream: SubscriptionStream,
    pub after_cursor: u64,
    pub batch_size: u64,
    pub max_in_flight: u16,
    pub retention_cursor_window: u64,
    pub lease_ms: u64,
    pub heartbeat_interval_ms: u64,
}

impl OpenSubscription {
    pub fn validate(&self) -> Result<()> {
        if self.batch_size == 0 || self.batch_size > MAX_CHANGEFEED_PAGE {
            return invalid(format!(
                "subscription batch_size must be in 1..={MAX_CHANGEFEED_PAGE}"
            ));
        }
        self.stream.validate(self.after_cursor, self.batch_size)?;
        if self.max_in_flight == 0 || self.max_in_flight > MAX_SUBSCRIPTION_IN_FLIGHT {
            return invalid(format!(
                "subscription max_in_flight must be in 1..={MAX_SUBSCRIPTION_IN_FLIGHT}"
            ));
        }
        if self.retention_cursor_window < self.batch_size
            || self.retention_cursor_window > MAX_SUBSCRIPTION_RETENTION_CURSORS
        {
            return invalid(format!(
                "subscription retention_cursor_window must be between batch_size and {MAX_SUBSCRIPTION_RETENTION_CURSORS}"
            ));
        }
        if !(MIN_LEASE_MS..=MAX_LEASE_MS).contains(&self.lease_ms) {
            return invalid(format!(
                "subscription lease_ms must be in {MIN_LEASE_MS}..={MAX_LEASE_MS}"
            ));
        }
        if !(MIN_SUBSCRIPTION_HEARTBEAT_MS..=MAX_SUBSCRIPTION_HEARTBEAT_MS)
            .contains(&self.heartbeat_interval_ms)
        {
            return invalid(format!(
                "subscription heartbeat_interval_ms must be in {MIN_SUBSCRIPTION_HEARTBEAT_MS}..={MAX_SUBSCRIPTION_HEARTBEAT_MS}"
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SubscriptionStatus {
    Open,
    Closed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SubscriptionSnapshot {
    pub subscription_id: CorrelationId,
    pub stream_sha256: String,
    pub acknowledged_cursor: u64,
    pub retention_floor_cursor: u64,
    pub head_cursor: u64,
    pub batch_size: u64,
    pub max_in_flight: u16,
    pub connection_generation: u64,
    pub lease_expires_at_unix_ms: u64,
    pub heartbeat_interval_ms: u64,
    pub status: SubscriptionStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OpenSubscriptionResult {
    pub subscription: SubscriptionSnapshot,
    pub idempotent_replay: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CloseSubscription {
    pub subscription_id: CorrelationId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CloseSubscriptionResult {
    pub subscription: SubscriptionSnapshot,
    pub idempotent_replay: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BackupCoverageSnapshot {
    Included,
    ReferencedOnly,
    RebuildRequired,
    Excluded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LogicalArchiveSnapshot {
    pub format_version: u16,
    pub contract_version: u16,
    pub archive_sha256: String,
    pub action_count: u64,
    pub standalone_claims: u64,
    pub runtime_commits: u64,
    pub runtime_mutations: u64,
    pub payload_bytes: u64,
    pub claim_sequence: u64,
    pub runtime_cursor: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_audit_sha256: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InstanceBackupSnapshot {
    pub backup_sha256: String,
    pub label: String,
    pub created_at_unix_ms: u64,
    pub archive: LogicalArchiveSnapshot,
    pub claims: BackupCoverageSnapshot,
    pub typed_runtime: BackupCoverageSnapshot,
    pub catalogues: BackupCoverageSnapshot,
    /// The estate's control state outside the catalogues: installed record
    /// and checkpoint, active configuration, security policy and audit chain,
    /// attunement plan and job state, and estate documents. `included` means
    /// the backup carries them as a replayable control-journal archive; the
    /// token key and operator credential are never carried.
    pub installed_control: BackupCoverageSnapshot,
    pub object_payloads: BackupCoverageSnapshot,
    pub projections: BackupCoverageSnapshot,
    pub invocation_telemetry: BackupCoverageSnapshot,
    pub snapshot_leases: BackupCoverageSnapshot,
    pub application_complete: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InstanceBackupCatalogueSnapshot {
    pub format_version: u16,
    pub revision: u64,
    pub catalogue_sha256: String,
    pub archives_verified: bool,
    pub backups: Vec<InstanceBackupSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateInstanceBackup {
    pub label: String,
    pub created_at_unix_ms: u64,
}

impl CreateInstanceBackup {
    pub fn validate(&self) -> Result<()> {
        if self.label.is_empty()
            || self.label.len() > 96
            || self.label.trim() != self.label
            || !self
                .label
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
        {
            return invalid(
                "backup label must be 1-96 ASCII alphanumeric, '.', '-', or '_' characters",
            );
        }
        if self.created_at_unix_ms == 0 {
            return invalid("backup created_at_unix_ms must be greater than zero");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateInstanceBackupResult {
    pub backup: InstanceBackupSnapshot,
    pub catalogue_revision: u64,
    pub catalogue_sha256: String,
    pub idempotent_replay: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ListInstanceBackups {
    #[serde(default)]
    pub verify_archives: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RestoreInstanceBackup {
    pub backup_sha256: String,
    pub restore_id: CanonicalId,
    pub restored_at_unix_ms: u64,
}

impl RestoreInstanceBackup {
    pub fn validate(&self) -> Result<()> {
        validate_sha256(&self.backup_sha256, "backup_sha256")?;
        if self.restored_at_unix_ms == 0 {
            return invalid("restore restored_at_unix_ms must be greater than zero");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RestoreInstanceBackupResult {
    pub backup_sha256: String,
    pub restore_id: CanonicalId,
    pub inventory: LogicalArchiveSnapshot,
    pub reopened: bool,
    pub idempotent_replay: bool,
}

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
    EstateRead,
    EstateAdmin,
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
    KnowledgeTreeRead,
    KnowledgeTreeWrite,
}

impl SecurityAction {
    /// Canonical policy-action catalogue. Installation profiles and other
    /// projections consume this list instead of maintaining parallel copies.
    pub const ALL: [Self; 53] = [
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
        Self::EstateRead,
        Self::EstateAdmin,
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

fn validate_audit_chain(
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

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum HttpMethod {
    Get,
    Post,
    Delete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EndpointAuthentication {
    Public,
    ApiKey,
    SessionBearer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "source", rename_all = "snake_case")]
pub enum EndpointAction {
    Fixed { action: SecurityAction },
}

impl EndpointAction {
    pub const fn fixed(action: SecurityAction) -> Self {
        Self::Fixed { action }
    }

    pub const fn fixed_action(self) -> Option<SecurityAction> {
        match self {
            Self::Fixed { action } => Some(action),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EndpointDescriptor {
    pub operation: CanonicalId,
    pub method: HttpMethod,
    pub path: String,
    pub authentication: EndpointAuthentication,
    pub mutation: bool,
    pub action: EndpointAction,
    pub request_type: String,
    pub response_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WebSocketEndpointDescriptor {
    pub operation: CanonicalId,
    pub path: String,
    pub authentication: EndpointAuthentication,
    pub connect_action: SecurityAction,
    pub frame_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EndpointCatalogue {
    pub protocol: String,
    pub protocol_version: u16,
    pub endpoints: Vec<EndpointDescriptor>,
    pub websocket_endpoints: Vec<WebSocketEndpointDescriptor>,
}

impl EndpointCatalogue {
    /// Resolves one concrete HTTP request against the authoritative endpoint
    /// templates. Transports use this instead of maintaining a second route
    /// capability table.
    pub fn resolve_http(&self, method: HttpMethod, path: &str) -> Option<&EndpointDescriptor> {
        self.endpoints.iter().find(|endpoint| {
            endpoint.method == method && endpoint_path_matches(&endpoint.path, path)
        })
    }

    pub fn validate(&self) -> Result<()> {
        validate_protocol(&self.protocol, self.protocol_version)?;
        if self.endpoints.is_empty() || self.endpoints.len() > 256 {
            return invalid("endpoint catalogue must contain 1..=256 endpoints");
        }
        if self.websocket_endpoints.len() > 32 {
            return invalid("endpoint catalogue may contain at most 32 WebSocket endpoints");
        }
        let mut operations = BTreeSet::new();
        let mut routes = BTreeSet::new();
        for endpoint in &self.endpoints {
            if endpoint.path.is_empty()
                || endpoint.path.len() > 256
                || !endpoint.path.starts_with("/v1/")
                || !endpoint.path.is_ascii()
                || endpoint.request_type.is_empty()
                || endpoint.request_type.len() > 128
                || endpoint.response_type.is_empty()
                || endpoint.response_type.len() > 128
                || !endpoint.request_type.is_ascii()
                || !endpoint.response_type.is_ascii()
            {
                return invalid("endpoint descriptor contains an invalid path or type name");
            }
            if !operations.insert(endpoint.operation.clone())
                || !routes.insert((endpoint.method, endpoint.path.clone()))
            {
                return invalid("endpoint operations and method/path pairs must be unique");
            }
            if endpoint.mutation && endpoint.method == HttpMethod::Get {
                return invalid("mutating endpoints may not use GET");
            }
        }
        if self
            .endpoints
            .windows(2)
            .any(|pair| pair[0].operation > pair[1].operation)
        {
            return invalid("endpoint descriptors must be sorted by operation");
        }
        let mut websocket_operations = BTreeSet::new();
        let mut websocket_paths = BTreeSet::new();
        for endpoint in &self.websocket_endpoints {
            if endpoint.path.is_empty()
                || endpoint.path.len() > 256
                || !endpoint.path.starts_with("/v1/")
                || !endpoint.path.is_ascii()
                || endpoint.frame_type.is_empty()
                || endpoint.frame_type.len() > 128
                || !endpoint.frame_type.is_ascii()
                || endpoint.authentication != EndpointAuthentication::SessionBearer
            {
                return invalid("WebSocket endpoint descriptor is invalid");
            }
            if !websocket_operations.insert(endpoint.operation.clone())
                || !websocket_paths.insert(endpoint.path.clone())
            {
                return invalid("WebSocket endpoint operations and paths must be unique");
            }
        }
        if self
            .websocket_endpoints
            .windows(2)
            .any(|pair| pair[0].operation > pair[1].operation)
        {
            return invalid("WebSocket endpoint descriptors must be sorted by operation");
        }
        Ok(())
    }
}

fn endpoint_path_matches(template: &str, concrete: &str) -> bool {
    let mut template_segments = template.split('/');
    let mut concrete_segments = concrete.split('/');
    loop {
        match (template_segments.next(), concrete_segments.next()) {
            (Some(expected), Some(actual)) => {
                let parameter = expected.starts_with('{') && expected.ends_with('}');
                if (parameter && actual.is_empty()) || (!parameter && expected != actual) {
                    return false;
                }
            }
            (None, None) => return true,
            _ => return false,
        }
    }
}

pub fn endpoint_catalogue() -> EndpointCatalogue {
    let mut endpoints = vec![
        endpoint(
            "audit-export",
            HttpMethod::Post,
            "/v1/audit/export",
            EndpointAuthentication::SessionBearer,
            false,
            SecurityAction::AuditExport,
            "ExportAudit",
            "AuditExport",
        ),
        endpoint(
            "audit-read",
            HttpMethod::Post,
            "/v1/audit/read",
            EndpointAuthentication::SessionBearer,
            false,
            SecurityAction::AuditRead,
            "ReadAudit",
            "AuditPage",
        ),
        endpoint(
            "backup-create",
            HttpMethod::Post,
            "/v1/backups",
            EndpointAuthentication::SessionBearer,
            true,
            SecurityAction::BackupCreate,
            "CreateInstanceBackup",
            "CreateInstanceBackupResult",
        ),
        endpoint(
            "backup-list",
            HttpMethod::Post,
            "/v1/backups/list",
            EndpointAuthentication::SessionBearer,
            false,
            SecurityAction::BackupList,
            "ListInstanceBackups",
            "InstanceBackupCatalogueSnapshot",
        ),
        endpoint(
            "capabilities-read",
            HttpMethod::Get,
            "/v1/capabilities",
            EndpointAuthentication::Public,
            false,
            SecurityAction::ServiceInspect,
            "Empty",
            "ServiceCapabilities",
        ),
        endpoint(
            "changefeed-follow",
            HttpMethod::Post,
            "/v1/changes/follow",
            EndpointAuthentication::SessionBearer,
            false,
            SecurityAction::ChangefeedFollow,
            "FollowChangefeed",
            "ChangefeedFollowResult",
        ),
        endpoint(
            "changefeed-read",
            HttpMethod::Post,
            "/v1/changes/read",
            EndpointAuthentication::SessionBearer,
            false,
            SecurityAction::ChangefeedRead,
            "ReadChangefeed",
            "ChangefeedPage",
        ),
        endpoint(
            "context-assemble",
            HttpMethod::Post,
            "/v1/context/assemble",
            EndpointAuthentication::SessionBearer,
            false,
            SecurityAction::MemoryContextRead,
            "AssembleContext",
            "ContextPacket",
        ),
        endpoint(
            "diagnostics-storage-cursor",
            HttpMethod::Post,
            "/v1/diagnostics/storage/cursor",
            EndpointAuthentication::SessionBearer,
            false,
            SecurityAction::DiagnosticsRead,
            "InspectStorageCursor",
            "StorageCursorSnapshot",
        ),
        endpoint(
            "diagnostics-storage-read",
            HttpMethod::Post,
            "/v1/diagnostics/storage/read",
            EndpointAuthentication::SessionBearer,
            false,
            SecurityAction::DiagnosticsRead,
            "ReadStorageDiagnostics",
            "DiagnosticStorageSnapshot",
        ),
        endpoint(
            "diagnostics-read",
            HttpMethod::Post,
            "/v1/diagnostics/read",
            EndpointAuthentication::SessionBearer,
            false,
            SecurityAction::DiagnosticsRead,
            "ReadDiagnosticSnapshot",
            "DiagnosticSnapshot",
        ),
        endpoint(
            "flight-launch",
            HttpMethod::Post,
            "/v1/flights",
            EndpointAuthentication::SessionBearer,
            true,
            SecurityAction::FlightLaunch,
            "LaunchPromptFlight",
            "LaunchPromptFlightResult",
        ),
        endpoint(
            "flight-list",
            HttpMethod::Post,
            "/v1/flights/list",
            EndpointAuthentication::SessionBearer,
            false,
            SecurityAction::FlightRead,
            "ListPromptFlights",
            "PromptFlightPage",
        ),
        endpoint(
            "flight-read",
            HttpMethod::Post,
            "/v1/flights/read",
            EndpointAuthentication::SessionBearer,
            false,
            SecurityAction::FlightRead,
            "ReadPromptFlight",
            "PromptFlightReadResult",
        ),
        endpoint(
            "endpoint-configuration-read",
            HttpMethod::Post,
            "/v1/configuration/endpoints/read",
            EndpointAuthentication::SessionBearer,
            false,
            SecurityAction::EstateRead,
            "ReadEndpointConfiguration",
            "EndpointConfiguration",
        ),
        endpoint(
            "endpoint-configuration-preview",
            HttpMethod::Post,
            "/v1/configuration/endpoints/preview",
            EndpointAuthentication::SessionBearer,
            false,
            SecurityAction::EstateAdmin,
            "PreviewEndpointConfiguration",
            "EndpointConfigurationPlan",
        ),
        endpoint(
            "endpoint-configuration-apply",
            HttpMethod::Post,
            "/v1/configuration/endpoints/apply",
            EndpointAuthentication::SessionBearer,
            true,
            SecurityAction::EstateAdmin,
            "ApplyEndpointConfiguration",
            "EndpointConfiguration",
        ),
        endpoint(
            "estate-read",
            HttpMethod::Post,
            "/v1/estates/{estate}/read",
            EndpointAuthentication::SessionBearer,
            false,
            SecurityAction::EstateRead,
            "ReadEstate",
            "EstateSnapshot",
        ),
        endpoint(
            "endpoint-catalogue",
            HttpMethod::Get,
            "/v1/schema/endpoints",
            EndpointAuthentication::Public,
            false,
            SecurityAction::ServiceInspect,
            "Empty",
            "EndpointCatalogue",
        ),
        endpoint(
            "health-live",
            HttpMethod::Get,
            "/v1/health/live",
            EndpointAuthentication::Public,
            false,
            SecurityAction::ServiceInspect,
            "Empty",
            "Liveness",
        ),
        endpoint(
            "health-ready",
            HttpMethod::Get,
            "/v1/health/ready",
            EndpointAuthentication::Public,
            false,
            SecurityAction::ServiceInspect,
            "Empty",
            "Readiness",
        ),
        endpoint(
            "knowledge-tree-list",
            HttpMethod::Post,
            "/v1/knowledge/trees/list",
            EndpointAuthentication::SessionBearer,
            false,
            SecurityAction::KnowledgeTreeRead,
            "ListKnowledgeTrees",
            "KnowledgeTreeCatalogue",
        ),
        endpoint(
            "knowledge-tree-replace",
            HttpMethod::Post,
            "/v1/knowledge/trees/replace",
            EndpointAuthentication::SessionBearer,
            true,
            SecurityAction::KnowledgeTreeWrite,
            "ReplaceKnowledgeTrees",
            "ReplaceKnowledgeTreesResult",
        ),
        endpoint(
            "openapi-read",
            HttpMethod::Get,
            "/v1/schema/openapi",
            EndpointAuthentication::Public,
            false,
            SecurityAction::ServiceInspect,
            "Empty",
            "OpenApiDocument",
        ),
        endpoint(
            "data-schema-read",
            HttpMethod::Post,
            "/v1/data/schema",
            EndpointAuthentication::SessionBearer,
            false,
            SecurityAction::DiagnosticsRead,
            "ReadDataSchema",
            "DataSchemaSnapshot",
        ),
        endpoint(
            "query-export",
            HttpMethod::Post,
            "/v1/query/export",
            EndpointAuthentication::SessionBearer,
            false,
            SecurityAction::QueryExecute,
            "ExportQuery",
            "QueryExport",
        ),
        endpoint(
            "query-execute",
            HttpMethod::Post,
            "/v1/query",
            EndpointAuthentication::SessionBearer,
            false,
            SecurityAction::QueryExecute,
            "ExecuteQuery",
            "QueryResult",
        ),
        endpoint(
            "query-index-ensure",
            HttpMethod::Post,
            "/v1/query/indexes/ensure",
            EndpointAuthentication::SessionBearer,
            true,
            SecurityAction::QueryIndexEnsure,
            "EnsureQueryIndex",
            "EnsureQueryIndexResult",
        ),
        endpoint(
            "query-index-list",
            HttpMethod::Post,
            "/v1/query/indexes/list",
            EndpointAuthentication::SessionBearer,
            false,
            SecurityAction::QueryIndexList,
            "ListQueryIndexes",
            "QueryIndexCatalogueSnapshot",
        ),
        endpoint(
            "query-live-poll",
            HttpMethod::Post,
            "/v1/query/live/poll",
            EndpointAuthentication::SessionBearer,
            false,
            SecurityAction::QueryLivePoll,
            "PollLiveQuery",
            "LiveQueryDeltaResult",
        ),
        endpoint(
            "restore-create",
            HttpMethod::Post,
            "/v1/restores",
            EndpointAuthentication::SessionBearer,
            true,
            SecurityAction::RestoreCreate,
            "RestoreInstanceBackup",
            "RestoreInstanceBackupResult",
        ),
        endpoint(
            "session-close",
            HttpMethod::Delete,
            "/v1/sessions/{session}",
            EndpointAuthentication::SessionBearer,
            true,
            SecurityAction::SessionClose,
            "CloseSession",
            "SessionTermination",
        ),
        endpoint(
            "session-create",
            HttpMethod::Post,
            "/v1/sessions",
            EndpointAuthentication::ApiKey,
            true,
            SecurityAction::SessionCreate,
            "CreateSession",
            "SessionLease",
        ),
        endpoint(
            "session-renew",
            HttpMethod::Post,
            "/v1/sessions/{session}/renew",
            EndpointAuthentication::SessionBearer,
            true,
            SecurityAction::SessionRenew,
            "RenewSession",
            "SessionLease",
        ),
        endpoint(
            "subscription-close",
            HttpMethod::Post,
            "/v1/subscriptions/close",
            EndpointAuthentication::SessionBearer,
            true,
            SecurityAction::SubscriptionClose,
            "CloseSubscription",
            "CloseSubscriptionResult",
        ),
        endpoint(
            "subscription-open",
            HttpMethod::Post,
            "/v1/subscriptions/open",
            EndpointAuthentication::SessionBearer,
            true,
            SecurityAction::SubscriptionOpen,
            "OpenSubscription",
            "OpenSubscriptionResult",
        ),
        endpoint(
            "embedding-builtin-activate",
            HttpMethod::Post,
            "/v1/embedding/builtin/activate",
            EndpointAuthentication::SessionBearer,
            true,
            SecurityAction::ReasoningWrite,
            "ActivateBuiltinEmbedding",
            "EmbeddingModelCatalogue",
        ),
        endpoint(
            "task-budget-configure",
            HttpMethod::Post,
            "/v1/tasks/budget/configure",
            EndpointAuthentication::SessionBearer,
            true,
            SecurityAction::ReasoningWrite,
            "ConfigureTaskBudget",
            "TaskBudgetView",
        ),
        endpoint(
            "task-budget-read",
            HttpMethod::Post,
            "/v1/tasks/budget/read",
            EndpointAuthentication::SessionBearer,
            false,
            SecurityAction::ReasoningRead,
            "ReadTaskBudget",
            "TaskBudgetView",
        ),
        endpoint(
            "task-budget-stop",
            HttpMethod::Post,
            "/v1/tasks/budget/stop",
            EndpointAuthentication::SessionBearer,
            true,
            SecurityAction::ReasoningWrite,
            "StopTaskBudget",
            "TaskBudgetView",
        ),
        endpoint(
            "task-embedding-execute",
            HttpMethod::Post,
            "/v1/tasks/embedding/execute",
            EndpointAuthentication::SessionBearer,
            true,
            SecurityAction::ReasoningWrite,
            "ExecuteTaskEmbedding",
            "GenerateEmbeddingsResult",
        ),
        endpoint(
            "task-attempt-read",
            HttpMethod::Post,
            "/v1/tasks/attempt/read",
            EndpointAuthentication::SessionBearer,
            false,
            SecurityAction::ReasoningRead,
            "ReadTaskAttempt",
            "TaskAttemptView",
        ),
        endpoint(
            "task-close",
            HttpMethod::Post,
            "/v1/tasks/close",
            EndpointAuthentication::SessionBearer,
            true,
            SecurityAction::TaskWrite,
            "CloseTask",
            "TaskResult",
        ),
        endpoint(
            "task-list",
            HttpMethod::Post,
            "/v1/tasks/list",
            EndpointAuthentication::SessionBearer,
            false,
            SecurityAction::TaskRead,
            "ListTasks",
            "TaskPage",
        ),
        endpoint(
            "task-open",
            HttpMethod::Post,
            "/v1/tasks",
            EndpointAuthentication::SessionBearer,
            true,
            SecurityAction::TaskWrite,
            "OpenTask",
            "TaskResult",
        ),
        endpoint(
            "task-read",
            HttpMethod::Post,
            "/v1/tasks/read",
            EndpointAuthentication::SessionBearer,
            false,
            SecurityAction::TaskRead,
            "ReadTask",
            "TaskReadResult",
        ),
        endpoint(
            "transaction-abort",
            HttpMethod::Delete,
            "/v1/transactions/{transaction}",
            EndpointAuthentication::SessionBearer,
            true,
            SecurityAction::TransactionAbort,
            "AbortTransaction",
            "TransactionLease",
        ),
        endpoint(
            "transaction-begin",
            HttpMethod::Post,
            "/v1/transactions",
            EndpointAuthentication::SessionBearer,
            true,
            SecurityAction::TransactionBegin,
            "BeginTransaction",
            "TransactionLease",
        ),
        endpoint(
            "transaction-commit",
            HttpMethod::Post,
            "/v1/transactions/{transaction}/commit",
            EndpointAuthentication::SessionBearer,
            true,
            SecurityAction::TransactionCommit,
            "CommitTransaction",
            "CommitReceipt",
        ),
        endpoint(
            "transaction-preview",
            HttpMethod::Post,
            "/v1/transactions/{transaction}/preview",
            EndpointAuthentication::SessionBearer,
            true,
            SecurityAction::TransactionPreview,
            "PreviewTransaction",
            "TransactionPreview",
        ),
        endpoint(
            "vector-collection-ensure",
            HttpMethod::Post,
            "/v1/vector/collections/ensure",
            EndpointAuthentication::SessionBearer,
            true,
            SecurityAction::VectorCollectionEnsure,
            "EnsureVectorCollection",
            "EnsureVectorCollectionResult",
        ),
        endpoint(
            "vector-collection-list",
            HttpMethod::Post,
            "/v1/vector/collections/list",
            EndpointAuthentication::SessionBearer,
            false,
            SecurityAction::VectorCollectionList,
            "ListVectorCollections",
            "VectorCollectionCatalogueSnapshot",
        ),
        endpoint(
            "vector-point-retrieve",
            HttpMethod::Post,
            "/v1/vector/points/retrieve",
            EndpointAuthentication::SessionBearer,
            false,
            SecurityAction::VectorPointRetrieve,
            "RetrieveVectorPoints",
            "VectorPointBatch",
        ),
        endpoint(
            "vector-point-scroll",
            HttpMethod::Post,
            "/v1/vector/points/scroll",
            EndpointAuthentication::SessionBearer,
            false,
            SecurityAction::VectorPointScroll,
            "ScrollVectorPoints",
            "VectorPointPage",
        ),
        endpoint(
            "vector-search",
            HttpMethod::Post,
            "/v1/vector/search",
            EndpointAuthentication::SessionBearer,
            false,
            SecurityAction::VectorSearch,
            "SearchVectors",
            "VectorSearchResult",
        ),
    ];
    endpoints.sort_by(|left, right| left.operation.cmp(&right.operation));
    let catalogue = EndpointCatalogue {
        protocol: PROTOCOL.into(),
        protocol_version: PROTOCOL_VERSION,
        endpoints,
        websocket_endpoints: vec![WebSocketEndpointDescriptor {
            operation: CanonicalId::new("websocket-connect")
                .expect("static WebSocket operation is canonical"),
            path: "/v1/ws".into(),
            authentication: EndpointAuthentication::SessionBearer,
            connect_action: SecurityAction::WebSocketConnect,
            frame_type: "WebSocketFrame".into(),
        }],
    };
    debug_assert!(catalogue.validate().is_ok());
    catalogue
}

/// Deterministic OpenAPI 3.1 projection of the authoritative RRD v1 endpoint
/// catalogue and Rust wire types.
///
/// This document is the language-neutral generator input for supported SDKs;
/// it is not a separately maintained description of the protocol.
pub fn openapi_document() -> Result<serde_json::Value> {
    let catalogue = endpoint_catalogue();
    catalogue.validate()?;
    let signal_catalogue = serde_json::from_str::<serde_json::Value>(
        generated_signal_catalogue::SIGNAL_CATALOGUE_JSON,
    )
    .map_err(|error| ContractError(format!("generated signal catalogue is invalid: {error}")))?;
    let signal_catalogue_sha256 = generated_signal_catalogue::SIGNAL_CATALOGUE_SHA256;
    let query_value_schema = openapi_query_value_schema()?;
    let vector_payload_filter_schema = openapi_vector_payload_filter_schema()?;
    let mut websocket_frame_schema = schema_json::<WebSocketFrame>();
    rebase_local_schema_refs(
        &mut websocket_frame_schema,
        "#/components/schemas/WebSocketFrame",
    );
    let mut paths = serde_json::Map::new();
    for descriptor in &catalogue.endpoints {
        let method = match descriptor.method {
            HttpMethod::Get => "get",
            HttpMethod::Post => "post",
            HttpMethod::Delete => "delete",
        };
        let mut operation = serde_json::Map::new();
        operation.insert(
            "operationId".into(),
            serde_json::Value::String(descriptor.operation.as_str().into()),
        );
        operation.insert(
            "summary".into(),
            serde_json::Value::String(format!("RRD {}", descriptor.operation)),
        );
        operation.insert(
            "x-rrd-action".into(),
            serde_json::to_value(descriptor.action)
                .map_err(|error| ContractError(error.to_string()))?,
        );
        operation.insert(
            "x-rrd-mutation".into(),
            serde_json::Value::Bool(descriptor.mutation),
        );
        operation.insert(
            "security".into(),
            match descriptor.authentication {
                EndpointAuthentication::Public => serde_json::json!([]),
                EndpointAuthentication::ApiKey => serde_json::json!([{"rrdApiKey": []}]),
                EndpointAuthentication::SessionBearer => {
                    serde_json::json!([{"rrdBearer": []}])
                }
            },
        );
        let parameters = openapi_parameters(descriptor);
        if !parameters.is_empty() {
            operation.insert("parameters".into(), serde_json::Value::Array(parameters));
        }
        if descriptor.method != HttpMethod::Get {
            let mut request_schema = request_envelope_schema(&descriptor.request_type)?;
            rebase_local_schema_refs(
                &mut request_schema,
                &openapi_schema_pointer(
                    &descriptor.path,
                    method,
                    "requestBody/content/application~1json/schema",
                ),
            );
            operation.insert(
                "requestBody".into(),
                serde_json::json!({
                    "required": true,
                    "content": {
                        "application/json": {
                            "schema": request_schema
                        }
                    }
                }),
            );
        }
        let response_schema = response_envelope_schema(&descriptor.response_type)?;
        let mut success_schema = response_schema.clone();
        rebase_local_schema_refs(
            &mut success_schema,
            &openapi_schema_pointer(
                &descriptor.path,
                method,
                "responses/200/content/application~1json/schema",
            ),
        );
        let mut error_schema = response_schema;
        rebase_local_schema_refs(
            &mut error_schema,
            &openapi_schema_pointer(
                &descriptor.path,
                method,
                "responses/default/content/application~1json/schema",
            ),
        );
        operation.insert(
            "responses".into(),
            serde_json::json!({
                "200": {
                    "description": "Typed RRD response",
                    "content": {"application/json": {"schema": success_schema}}
                },
                "default": {
                    "description": "Typed RRD error response",
                    "content": {"application/json": {"schema": error_schema}}
                }
            }),
        );
        let path = paths
            .entry(descriptor.path.clone())
            .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
        path.as_object_mut()
            .expect("OpenAPI path item is constructed as an object")
            .insert(method.into(), serde_json::Value::Object(operation));
    }

    Ok(canonical_json(serde_json::json!({
        "openapi": "3.1.0",
        "info": {
            "title": "RRFlow Durable Runtime API",
            "version": format!("{PROTOCOL_VERSION}.0.0")
        },
        "servers": [{"url": "http://127.0.0.1:9477"}],
        "paths": paths,
        "components": {
            "schemas": {
                "QueryValue": query_value_schema,
                "WebSocketFrame": websocket_frame_schema,
                "VectorPayloadFilter": vector_payload_filter_schema
            },
            "securitySchemes": {
                "rrdApiKey": {
                    "type": "apiKey",
                    "in": "header",
                    "name": "Authorization",
                    "description": "Authorization: ApiKey <credential>; X-RRD-Principal is also required"
                },
                "rrdBearer": {
                    "type": "http",
                    "scheme": "bearer",
                    "description": "Session bearer; X-RRD-Session is also required"
                }
            }
        },
        "x-rrd-protocol": PROTOCOL,
        "x-rrd-protocol-version": PROTOCOL_VERSION,
        "x-rrd-endpoint-count": catalogue.endpoints.len(),
        "x-rrd-signal-catalogue": signal_catalogue,
        "x-rrd-signal-catalogue-sha256": signal_catalogue_sha256,
        "x-rrd-websocket-endpoint-count": catalogue.websocket_endpoints.len(),
        "x-rrd-websocket-endpoints": catalogue.websocket_endpoints
    })))
}

/// Normalize object insertion order so public protocol bytes are independent
/// of workspace feature unification. In particular, DataFusion enables
/// `serde_json/preserve_order`; RRD must not emit a different OpenAPI digest
/// merely because another workspace package activated that representation.
fn canonical_json(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Array(values) => {
            serde_json::Value::Array(values.into_iter().map(canonical_json).collect())
        }
        serde_json::Value::Object(values) => {
            let mut entries = values.into_iter().collect::<Vec<_>>();
            entries.sort_by(|left, right| left.0.cmp(&right.0));
            let mut canonical = serde_json::Map::with_capacity(entries.len());
            for (name, value) in entries {
                canonical.insert(name, canonical_json(value));
            }
            serde_json::Value::Object(canonical)
        }
        scalar => scalar,
    }
}

fn openapi_schema_pointer(path: &str, method: &str, suffix: &str) -> String {
    let escaped = path.replace('~', "~0").replace('/', "~1");
    format!("#/paths/{escaped}/{method}/{suffix}")
}

fn rebase_local_schema_refs(value: &mut serde_json::Value, schema_pointer: &str) {
    match value {
        serde_json::Value::Array(values) => {
            for value in values {
                rebase_local_schema_refs(value, schema_pointer);
            }
        }
        serde_json::Value::Object(object) => {
            if let Some(serde_json::Value::String(reference)) = object.get_mut("$ref") {
                if reference == "#" || reference == "#/$defs/QueryValue" {
                    *reference = "#/components/schemas/QueryValue".into();
                } else if reference == "#/$defs/VectorPayloadFilter" {
                    *reference = "#/components/schemas/VectorPayloadFilter".into();
                } else if let Some(suffix) = reference.strip_prefix("#/") {
                    *reference = format!("{schema_pointer}/{suffix}");
                }
            }
            for value in object.values_mut() {
                rebase_local_schema_refs(value, schema_pointer);
            }
            let remove_definitions =
                if let Some(serde_json::Value::Object(definitions)) = object.get_mut("$defs") {
                    definitions.remove("QueryValue");
                    definitions.remove("VectorPayloadFilter");
                    definitions.is_empty()
                } else {
                    false
                };
            if remove_definitions {
                object.remove("$defs");
            }
        }
        _ => {}
    }
}

fn openapi_query_value_schema() -> Result<serde_json::Value> {
    let generated = serde_json::to_value(schemars::schema_for!(QueryValue))
        .map_err(|error| ContractError(error.to_string()))?;
    let definition = generated
        .get("$defs")
        .and_then(serde_json::Value::as_object)
        .and_then(|definitions| definitions.get("QueryValue"))
        .cloned();
    let mut schema = definition.unwrap_or(generated);
    rebase_local_schema_refs(&mut schema, "#/components/schemas/QueryValue");
    Ok(schema)
}

fn openapi_vector_payload_filter_schema() -> Result<serde_json::Value> {
    let generated = serde_json::to_value(schemars::schema_for!(VectorPayloadFilter))
        .map_err(|error| ContractError(error.to_string()))?;
    let definition = generated
        .get("$defs")
        .and_then(serde_json::Value::as_object)
        .and_then(|definitions| definitions.get("VectorPayloadFilter"))
        .cloned();
    let mut schema = definition.unwrap_or(generated);
    rebase_local_schema_refs(&mut schema, "#/components/schemas/VectorPayloadFilter");
    Ok(schema)
}

fn openapi_parameters(descriptor: &EndpointDescriptor) -> Vec<serde_json::Value> {
    let mut parameters = Vec::new();
    for name in ["estate", "session", "transaction"] {
        if descriptor.path.contains(&format!("{{{name}}}")) {
            parameters.push(serde_json::json!({
                "name": name,
                "in": "path",
                "required": true,
                "schema": {"type": "string", "minLength": 1, "maxLength": MAX_ID_BYTES}
            }));
        }
    }
    match descriptor.authentication {
        EndpointAuthentication::Public => {}
        EndpointAuthentication::ApiKey => parameters.push(serde_json::json!({
            "name": "X-RRD-Principal",
            "in": "header",
            "required": true,
            "schema": {"type": "string", "minLength": 1, "maxLength": MAX_ID_BYTES}
        })),
        EndpointAuthentication::SessionBearer => parameters.push(serde_json::json!({
            "name": "X-RRD-Session",
            "in": "header",
            "required": true,
            "schema": {"type": "string", "minLength": 1, "maxLength": MAX_ID_BYTES}
        })),
    }
    parameters
}

fn schema_json<T: JsonSchema>() -> serde_json::Value {
    let schema = schemars::generate::SchemaSettings::draft2020_12()
        .with(|settings| settings.inline_subschemas = true)
        .into_generator()
        .into_root_schema_for::<T>();
    serde_json::to_value(schema).expect("JsonSchema output must serialize as JSON")
}

fn request_envelope_schema(name: &str) -> Result<serde_json::Value> {
    let schema = match name {
        "ActivateBuiltinEmbedding" => schema_json::<RequestEnvelope<ActivateBuiltinEmbedding>>(),
        "ConfigureTaskBudget" => schema_json::<RequestEnvelope<ConfigureTaskBudget>>(),
        "ExecuteTaskEmbedding" => schema_json::<RequestEnvelope<ExecuteTaskEmbedding>>(),
        "ReadTaskAttempt" => schema_json::<RequestEnvelope<ReadTaskAttempt>>(),
        "ReadTaskBudget" => schema_json::<RequestEnvelope<ReadTaskBudget>>(),
        "StopTaskBudget" => schema_json::<RequestEnvelope<StopTaskBudget>>(),
        "AbortTransaction" => schema_json::<RequestEnvelope<AbortTransaction>>(),
        "AssembleContext" => schema_json::<RequestEnvelope<AssembleContext>>(),
        "BeginTransaction" => schema_json::<RequestEnvelope<BeginTransaction>>(),
        "CloseSession" => schema_json::<RequestEnvelope<CloseSession>>(),
        "CommitTransaction" => schema_json::<RequestEnvelope<CommitTransaction>>(),
        "CreateInstanceBackup" => schema_json::<RequestEnvelope<CreateInstanceBackup>>(),
        "CreateSession" => schema_json::<RequestEnvelope<CreateSession>>(),
        "ExecuteQuery" => schema_json::<RequestEnvelope<ExecuteQuery>>(),
        "ReadEndpointConfiguration" => schema_json::<RequestEnvelope<ReadEndpointConfiguration>>(),
        "PreviewEndpointConfiguration" => {
            schema_json::<RequestEnvelope<PreviewEndpointConfiguration>>()
        }
        "ApplyEndpointConfiguration" => {
            schema_json::<RequestEnvelope<ApplyEndpointConfiguration>>()
        }
        "ExportQuery" => schema_json::<RequestEnvelope<ExportQuery>>(),
        "ExportAudit" => schema_json::<RequestEnvelope<ExportAudit>>(),
        "EnsureQueryIndex" => schema_json::<RequestEnvelope<EnsureQueryIndex>>(),
        "EnsureVectorCollection" => schema_json::<RequestEnvelope<EnsureVectorCollection>>(),
        "ListKnowledgeTrees" => schema_json::<RequestEnvelope<ListKnowledgeTrees>>(),
        "ReplaceKnowledgeTrees" => schema_json::<RequestEnvelope<ReplaceKnowledgeTrees>>(),
        "ListQueryIndexes" => schema_json::<RequestEnvelope<ListQueryIndexes>>(),
        "ListVectorCollections" => schema_json::<RequestEnvelope<ListVectorCollections>>(),
        "OpenSubscription" => schema_json::<RequestEnvelope<OpenSubscription>>(),
        "PollLiveQuery" => schema_json::<RequestEnvelope<PollLiveQuery>>(),
        "FollowChangefeed" => schema_json::<RequestEnvelope<FollowChangefeed>>(),
        "ListInstanceBackups" => schema_json::<RequestEnvelope<ListInstanceBackups>>(),
        "PreviewTransaction" => schema_json::<RequestEnvelope<PreviewTransaction>>(),
        "ReadAudit" => schema_json::<RequestEnvelope<ReadAudit>>(),
        "ReadChangefeed" => schema_json::<RequestEnvelope<ReadChangefeed>>(),
        "ReadDiagnosticSnapshot" => schema_json::<RequestEnvelope<ReadDiagnosticSnapshot>>(),
        "InspectStorageCursor" => schema_json::<RequestEnvelope<InspectStorageCursor>>(),
        "ReadStorageDiagnostics" => schema_json::<RequestEnvelope<ReadStorageDiagnostics>>(),
        "ReadDataSchema" => schema_json::<RequestEnvelope<ReadDataSchema>>(),
        "LaunchPromptFlight" => schema_json::<RequestEnvelope<LaunchPromptFlight>>(),
        "ListPromptFlights" => schema_json::<RequestEnvelope<ListPromptFlights>>(),
        "ReadPromptFlight" => schema_json::<RequestEnvelope<ReadPromptFlight>>(),
        "OpenTask" => schema_json::<RequestEnvelope<OpenTask>>(),
        "CloseTask" => schema_json::<RequestEnvelope<CloseTask>>(),
        "ListTasks" => schema_json::<RequestEnvelope<ListTasks>>(),
        "ReadTask" => schema_json::<RequestEnvelope<ReadTask>>(),
        "ReadEstate" => schema_json::<RequestEnvelope<ReadEstate>>(),
        "RenewSession" => schema_json::<RequestEnvelope<RenewSession>>(),
        "RestoreInstanceBackup" => schema_json::<RequestEnvelope<RestoreInstanceBackup>>(),
        "RetrieveVectorPoints" => schema_json::<RequestEnvelope<RetrieveVectorPoints>>(),
        "ScrollVectorPoints" => schema_json::<RequestEnvelope<ScrollVectorPoints>>(),
        "SearchVectors" => schema_json::<RequestEnvelope<SearchVectors>>(),
        "CloseSubscription" => schema_json::<RequestEnvelope<CloseSubscription>>(),
        _ => return invalid(format!("no public request schema for {name}")),
    };
    Ok(schema)
}

fn response_envelope_schema(name: &str) -> Result<serde_json::Value> {
    let schema = match name {
        "EmbeddingModelCatalogue" => schema_json::<ResponseEnvelope<EmbeddingModelCatalogue>>(),
        "GenerateEmbeddingsResult" => schema_json::<ResponseEnvelope<GenerateEmbeddingsResult>>(),
        "TaskAttemptView" => schema_json::<ResponseEnvelope<TaskAttemptView>>(),
        "TaskBudgetView" => schema_json::<ResponseEnvelope<TaskBudgetView>>(),
        "AuditExport" => schema_json::<ResponseEnvelope<AuditExport>>(),
        "AuditPage" => schema_json::<ResponseEnvelope<AuditPage>>(),
        "ChangefeedFollowResult" => schema_json::<ResponseEnvelope<ChangefeedFollowResult>>(),
        "ChangefeedPage" => schema_json::<ResponseEnvelope<ChangefeedPage>>(),
        "CommitReceipt" => schema_json::<ResponseEnvelope<CommitReceipt>>(),
        "ContextPacket" => schema_json::<ResponseEnvelope<ContextPacket>>(),
        "CloseSubscriptionResult" => schema_json::<ResponseEnvelope<CloseSubscriptionResult>>(),
        "CreateInstanceBackupResult" => {
            schema_json::<ResponseEnvelope<CreateInstanceBackupResult>>()
        }
        "DiagnosticSnapshot" => schema_json::<ResponseEnvelope<DiagnosticSnapshot>>(),
        "StorageCursorSnapshot" => schema_json::<ResponseEnvelope<StorageCursorSnapshot>>(),
        "DiagnosticStorageSnapshot" => schema_json::<ResponseEnvelope<DiagnosticStorageSnapshot>>(),
        "DataSchemaSnapshot" => schema_json::<ResponseEnvelope<DataSchemaSnapshot>>(),
        "LaunchPromptFlightResult" => schema_json::<ResponseEnvelope<LaunchPromptFlightResult>>(),
        "PromptFlightPage" => schema_json::<ResponseEnvelope<PromptFlightPage>>(),
        "PromptFlightReadResult" => schema_json::<ResponseEnvelope<PromptFlightReadResult>>(),
        "TaskResult" => schema_json::<ResponseEnvelope<TaskResult>>(),
        "TaskPage" => schema_json::<ResponseEnvelope<TaskPage>>(),
        "TaskReadResult" => schema_json::<ResponseEnvelope<TaskReadResult>>(),
        "EndpointCatalogue" => schema_json::<ResponseEnvelope<EndpointCatalogue>>(),
        "EstateSnapshot" => schema_json::<ResponseEnvelope<EstateSnapshot>>(),
        "InstanceBackupCatalogueSnapshot" => {
            schema_json::<ResponseEnvelope<InstanceBackupCatalogueSnapshot>>()
        }
        "KnowledgeTreeCatalogue" => schema_json::<ResponseEnvelope<KnowledgeTreeCatalogue>>(),
        "ReplaceKnowledgeTreesResult" => {
            schema_json::<ResponseEnvelope<ReplaceKnowledgeTreesResult>>()
        }
        "Liveness" => schema_json::<ResponseEnvelope<Liveness>>(),
        "LiveQueryDeltaResult" => schema_json::<ResponseEnvelope<LiveQueryDeltaResult>>(),
        "OpenApiDocument" => schema_json::<ResponseEnvelope<serde_json::Value>>(),
        "OpenSubscriptionResult" => schema_json::<ResponseEnvelope<OpenSubscriptionResult>>(),
        "QueryResult" => schema_json::<ResponseEnvelope<QueryResult>>(),
        "EndpointConfiguration" => schema_json::<ResponseEnvelope<EndpointConfiguration>>(),
        "EndpointConfigurationPlan" => schema_json::<ResponseEnvelope<EndpointConfigurationPlan>>(),
        "QueryExport" => schema_json::<ResponseEnvelope<QueryExport>>(),
        "EnsureQueryIndexResult" => schema_json::<ResponseEnvelope<EnsureQueryIndexResult>>(),
        "EnsureVectorCollectionResult" => {
            schema_json::<ResponseEnvelope<EnsureVectorCollectionResult>>()
        }
        "QueryIndexCatalogueSnapshot" => {
            schema_json::<ResponseEnvelope<QueryIndexCatalogueSnapshot>>()
        }
        "Readiness" => schema_json::<ResponseEnvelope<Readiness>>(),
        "RestoreInstanceBackupResult" => {
            schema_json::<ResponseEnvelope<RestoreInstanceBackupResult>>()
        }
        "ServiceCapabilities" => schema_json::<ResponseEnvelope<ServiceCapabilities>>(),
        "SessionLease" => schema_json::<ResponseEnvelope<SessionLease>>(),
        "SessionTermination" => schema_json::<ResponseEnvelope<SessionTermination>>(),
        "TransactionLease" => schema_json::<ResponseEnvelope<TransactionLease>>(),
        "TransactionPreview" => schema_json::<ResponseEnvelope<TransactionPreview>>(),
        "VectorPointBatch" => schema_json::<ResponseEnvelope<VectorPointBatch>>(),
        "VectorPointPage" => schema_json::<ResponseEnvelope<VectorPointPage>>(),
        "VectorCollectionCatalogueSnapshot" => {
            schema_json::<ResponseEnvelope<VectorCollectionCatalogueSnapshot>>()
        }
        "VectorSearchResult" => schema_json::<ResponseEnvelope<VectorSearchResult>>(),
        _ => return invalid(format!("no public response schema for {name}")),
    };
    Ok(schema)
}

#[allow(clippy::too_many_arguments)]
fn endpoint(
    operation: &str,
    method: HttpMethod,
    path: &str,
    authentication: EndpointAuthentication,
    mutation: bool,
    action: SecurityAction,
    request_type: &str,
    response_type: &str,
) -> EndpointDescriptor {
    EndpointDescriptor {
        operation: CanonicalId::new(operation).expect("static endpoint operation is canonical"),
        method,
        path: path.into(),
        authentication,
        mutation,
        action: EndpointAction::fixed(action),
        request_type: request_type.into(),
        response_type: response_type.into(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReadEstate {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EstateDesiredPhase {
    Running,
    Stopped,
    Absent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EstateObservedPhase {
    Unknown,
    Provisioning,
    Starting,
    Running,
    Stopping,
    Stopped,
    Deleting,
    Absent,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EstateActivityClass {
    Unknown,
    Active,
    Idle,
    Stale,
    Neglected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EstateOperationKind {
    Provision,
    Start,
    Stop,
    Restart,
    Upgrade,
    Delete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EstateOperationState {
    Pending,
    Leased,
    Prepared,
    Applied,
    Succeeded,
    Failed,
    Superseded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EstateReceiptBoundary {
    Prepared,
    Applied,
    Completed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EstateActivityPolicySnapshot {
    pub idle_after_ms: u64,
    pub stale_after_ms: u64,
    pub neglected_after_ms: u64,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum EstateAuthorityResourceKind {
    Organisation,
    Account,
    Entitlement,
    Project,
    Environment,
    Instance,
    Node,
    Shard,
    Job,
    Assignment,
    Health,
    SecretReference,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EstateAuthorityStatus {
    Pending,
    Ready,
    Degraded,
    Failed,
    Retired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EstateAuthorityReceiptBoundary {
    DesiredAccepted,
    Assigned,
    Applied,
    Observed,
    Completed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EstateAuthorityDesiredSnapshot {
    pub generation: u64,
    pub spec_sha256: String,
    pub updated_at_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EstateAuthorityObservedSnapshot {
    pub generation: u64,
    pub status: EstateAuthorityStatus,
    pub observed_at_unix_ms: u64,
    pub evidence_sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EstateAuthorityResourceSnapshot {
    pub id: CanonicalId,
    pub kind: EstateAuthorityResourceKind,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parent_ids: Vec<CanonicalId>,
    pub name: String,
    pub spec_sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub desired: Option<EstateAuthorityDesiredSnapshot>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observed: Option<EstateAuthorityObservedSnapshot>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub secret_reference_ids: Vec<CanonicalId>,
    pub created_at_unix_ms: u64,
    pub updated_at_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EstateAuthorityReceiptSnapshot {
    pub id: CanonicalId,
    pub resource_id: CanonicalId,
    pub operation_id: CanonicalId,
    pub lease_epoch: u64,
    pub boundary: EstateAuthorityReceiptBoundary,
    pub at_unix_ms: u64,
    pub evidence_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EstateAuthoritySnapshot {
    pub catalogue_sha256: String,
    pub resources: Vec<EstateAuthorityResourceSnapshot>,
    pub receipts: Vec<EstateAuthorityReceiptSnapshot>,
    pub idempotency_binding_count: u32,
}

impl EstateAuthoritySnapshot {
    pub fn empty() -> Self {
        Self {
            catalogue_sha256: sha256_bytes(
                br#"{"format":1,"resources":{},"receipts":{},"idempotency":{}}"#,
            ),
            resources: Vec::new(),
            receipts: Vec::new(),
            idempotency_binding_count: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EstateDesiredSnapshot {
    pub generation: u64,
    pub phase: EstateDesiredPhase,
    pub deployment_ref: CanonicalId,
    pub version: String,
    pub configuration_sha256: String,
    pub updated_at_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EstateObservedSnapshot {
    pub generation: u64,
    pub phase: EstateObservedPhase,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub process_id: Option<u32>,
    pub observed_at_unix_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EstateActivitySnapshot {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_meaningful_runtime_at_unix_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_heartbeat_at_unix_ms: Option<u64>,
    pub class: EstateActivityClass,
    pub evaluated_at_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EstateInstanceSnapshot {
    pub id: CanonicalId,
    pub desired: EstateDesiredSnapshot,
    pub observed: EstateObservedSnapshot,
    pub activity: EstateActivitySnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EstateLeaseSnapshot {
    pub owner: CanonicalId,
    pub epoch: u64,
    pub acquired_at_unix_ms: u64,
    pub expires_at_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EstateReceiptSnapshot {
    pub boundary: EstateReceiptBoundary,
    pub lease_epoch: u64,
    pub at_unix_ms: u64,
    pub evidence_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EstateOperationSnapshot {
    pub id: CanonicalId,
    pub instance_id: CanonicalId,
    pub kind: EstateOperationKind,
    pub desired_generation: u64,
    pub request_sha256: String,
    pub state: EstateOperationState,
    pub attempts: u32,
    pub created_at_unix_ms: u64,
    pub updated_at_unix_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lease: Option<EstateLeaseSnapshot>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub receipts: Vec<EstateReceiptSnapshot>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EstateSnapshot {
    pub format_version: u16,
    pub id: CanonicalId,
    pub revision: u64,
    pub created_at_unix_ms: u64,
    pub updated_at_unix_ms: u64,
    pub activity_policy: EstateActivityPolicySnapshot,
    pub authority: EstateAuthoritySnapshot,
    pub instances: Vec<EstateInstanceSnapshot>,
    pub operations: Vec<EstateOperationSnapshot>,
    pub idempotency_binding_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EstateMutationResult {
    pub estate: EstateSnapshot,
    pub idempotent_replay: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EstateBackupJobState {
    Pending,
    Leased,
    Prepared,
    Succeeded,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EstateBackupReceiptBoundary {
    Prepared,
    Completed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EstateBackupReceiptSnapshot {
    pub boundary: EstateBackupReceiptBoundary,
    pub lease_epoch: u64,
    pub at_unix_ms: u64,
    pub evidence_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EstateBackupRecoveryPolicySnapshot {
    pub revision: u64,
    pub max_rpo_ms: u64,
    pub max_rto_ms: u64,
    pub minimum_recovery_points: u16,
    pub retention_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EstateBackupJobSnapshot {
    pub id: CanonicalId,
    pub instance_id: CanonicalId,
    pub source_generation: u64,
    pub label: String,
    pub request_sha256: String,
    pub state: EstateBackupJobState,
    pub attempts: u32,
    pub created_at_unix_ms: u64,
    pub updated_at_unix_ms: u64,
    pub recovery_policy: EstateBackupRecoveryPolicySnapshot,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lease: Option<EstateLeaseSnapshot>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub receipts: Vec<EstateBackupReceiptSnapshot>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backup_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archive_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub catalogue_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EstateBackupJobsSnapshot {
    pub estate_id: CanonicalId,
    pub estate_revision: u64,
    pub jobs: Vec<EstateBackupJobSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EstateBackupMutationResult {
    pub estate: EstateSnapshot,
    pub job: EstateBackupJobSnapshot,
    pub idempotent_replay: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EstateRecoveryPolicySnapshot {
    pub instance_id: CanonicalId,
    pub revision: u64,
    pub max_rpo_ms: u64,
    pub max_rto_ms: u64,
    pub minimum_recovery_points: u16,
    pub retention_ms: u64,
    pub updated_at_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EstateRecoveryPointSnapshot {
    pub backup_sha256: String,
    pub instance_id: CanonicalId,
    pub backup_job_id: CanonicalId,
    pub source_generation: u64,
    pub archive_sha256: String,
    pub catalogue_sha256: String,
    pub source_cut_at_unix_ms: u64,
    pub completed_at_unix_ms: u64,
    pub policy_revision: u64,
    pub max_rpo_ms: u64,
    pub max_rto_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pruned_at_unix_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EstateRetentionPinKindSnapshot {
    Policy,
    ExplicitHold,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EstateRetentionPinSnapshot {
    pub id: CanonicalId,
    pub backup_sha256: String,
    pub kind: EstateRetentionPinKindSnapshot,
    pub created_at_unix_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at_unix_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub released_at_unix_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EstateRestoreEvidenceSnapshot {
    pub restore_id: CanonicalId,
    pub instance_id: CanonicalId,
    pub backup_sha256: String,
    pub started_at_unix_ms: u64,
    pub completed_at_unix_ms: u64,
    pub duration_ms: u64,
    pub recovery_point_age_ms: u64,
    pub restored_claim_sequence: u64,
    pub restored_runtime_cursor: u64,
    pub closure_sha256: String,
    pub policy_revision: u64,
    pub rpo_within_objective: bool,
    pub rto_within_objective: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EstateRecoveryPruneIntentSnapshot {
    pub id: CanonicalId,
    pub instance_id: CanonicalId,
    pub based_on_estate_revision: u64,
    pub evaluated_at_unix_ms: u64,
    pub expected_catalogue_sha256: String,
    pub retained_backup_ids: Vec<String>,
    pub prune_candidate_backup_ids: Vec<String>,
    pub created_at_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EstateRecoverySnapshot {
    pub estate_id: CanonicalId,
    pub estate_revision: u64,
    pub policies: Vec<EstateRecoveryPolicySnapshot>,
    pub recovery_points: Vec<EstateRecoveryPointSnapshot>,
    pub retention_pins: Vec<EstateRetentionPinSnapshot>,
    pub restore_evidence: Vec<EstateRestoreEvidenceSnapshot>,
    pub prune_intents: Vec<EstateRecoveryPruneIntentSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EstateRetentionDecisionSnapshot {
    pub estate_revision: u64,
    pub instance_id: CanonicalId,
    pub evaluated_at_unix_ms: u64,
    pub retained_backup_ids: Vec<String>,
    pub prune_candidate_backup_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EstateRecoveryMutationResult {
    pub recovery: EstateRecoverySnapshot,
    pub idempotent_replay: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EstateRecoveryPruneResult {
    pub recovery: EstateRecoverySnapshot,
    pub decision: EstateRetentionDecisionSnapshot,
    pub catalogue_revision: u64,
    pub catalogue_sha256: String,
    pub pruned_backup_ids: Vec<String>,
    pub idempotent_replay: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EstateRecoveryRestoreResult {
    pub recovery: EstateRecoverySnapshot,
    pub restore_id: CanonicalId,
    pub backup_sha256: String,
    pub inventory: LogicalArchiveSnapshot,
    pub reopened: bool,
    pub idempotent_replay: bool,
}

pub type Result<T> = std::result::Result<T, ContractError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractError(pub String);

impl fmt::Display for ContractError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for ContractError {}

/// A canonical public identifier component.
///
/// IDs are lowercase ASCII and intentionally URL/path safe. Human-facing
/// labels are separate data and may use arbitrary Unicode.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, JsonSchema)]
#[schemars(transparent)]
pub struct CanonicalId(String);

impl CanonicalId {
    pub fn new(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        if value.is_empty() || value.len() > MAX_ID_BYTES {
            return invalid(format!(
                "canonical id length must be in 1..={MAX_ID_BYTES} bytes"
            ));
        }
        if !value.bytes().enumerate().all(|(index, byte)| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || (index > 0 && matches!(byte, b'-' | b'_' | b'.'))
        }) {
            return invalid(
                "canonical id must start with lowercase ASCII or a digit and contain only lowercase ASCII, digits, '-', '_', or '.'",
            );
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

impl fmt::Display for CanonicalId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Serialize for CanonicalId {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for CanonicalId {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum ResourceKind {
    Organization,
    Estate,
    Project,
    Instance,
    Tenant,
    Namespace,
    Database,
    Table,
    Collection,
    Record,
    Point,
    Relation,
    Alias,
    Cluster,
    Node,
    Shard,
    Replica,
    Segment,
    Transaction,
    Snapshot,
    Backup,
    Operation,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResourceId {
    pub kind: ResourceKind,
    pub id: CanonicalId,
}

impl ResourceId {
    pub fn new(kind: ResourceKind, id: impl Into<String>) -> Result<Self> {
        Ok(Self {
            kind,
            id: CanonicalId::new(id)?,
        })
    }
}

/// A fully explicit hierarchical identity. No field is inferred from process
/// cwd, connection state, or a human label.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResourcePath {
    pub segments: Vec<ResourceId>,
}

impl ResourcePath {
    pub fn validate(&self) -> Result<()> {
        if self.segments.is_empty() {
            return invalid("resource path must contain at least one segment");
        }
        if self.segments.len() > 16 {
            return invalid("resource path must contain at most 16 segments");
        }
        let mut kinds = BTreeSet::new();
        for segment in &self.segments {
            if !kinds.insert(segment.kind) {
                return invalid("resource path must not repeat a resource kind");
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, JsonSchema)]
#[schemars(transparent)]
pub struct CorrelationId(String);

impl CorrelationId {
    pub fn new(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        if value.is_empty() || value.len() > MAX_ID_BYTES {
            return invalid(format!(
                "correlation id length must be in 1..={MAX_ID_BYTES} bytes"
            ));
        }
        if !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
        {
            return invalid(
                "correlation id must contain only ASCII letters, digits, '-', '_', '.', or ':'",
            );
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Serialize for CorrelationId {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for CorrelationId {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RequestContext {
    pub request_id: CorrelationId,
    pub operation_id: CorrelationId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<CorrelationId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deadline_unix_ms: Option<u64>,
}

impl RequestContext {
    pub fn validate(&self, mutation: bool) -> Result<()> {
        if mutation && self.idempotency_key.is_none() {
            return invalid("mutating requests require an idempotency key");
        }
        if self.deadline_unix_ms == Some(0) {
            return invalid("deadline_unix_ms must be greater than zero when present");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct IdempotencyBinding {
    pub key: CorrelationId,
    pub operation_sha256: String,
}

impl IdempotencyBinding {
    pub fn validate(&self) -> Result<()> {
        validate_sha256(&self.operation_sha256, "operation_sha256")
    }
}

/// One checked-in logical corpus used unchanged by every deployment adapter.
/// Physical cursors, transport evidence, and storage receipts remain
/// mode-specific; the expected logical identities may not diverge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DeploymentConformanceCorpus {
    pub format_version: u16,
    pub documents: Vec<DeploymentConformanceDocument>,
    pub query: DeploymentConformanceQuery,
    pub expected_ids: Vec<CanonicalId>,
}

impl DeploymentConformanceCorpus {
    pub fn validate(&self) -> Result<()> {
        if self.format_version != DEPLOYMENT_CONFORMANCE_FORMAT_VERSION {
            return invalid(format!(
                "deployment conformance format must be {DEPLOYMENT_CONFORMANCE_FORMAT_VERSION}"
            ));
        }
        if self.documents.is_empty() || self.documents.len() > MAX_DEPLOYMENT_CONFORMANCE_DOCUMENTS
        {
            return invalid(format!(
                "deployment conformance documents must contain 1..={MAX_DEPLOYMENT_CONFORMANCE_DOCUMENTS} entries"
            ));
        }
        let mut document_ids = BTreeSet::new();
        for document in &self.documents {
            document.validate()?;
            if !document_ids.insert(&document.id) {
                return invalid("deployment conformance document identities must be unique");
            }
        }
        self.query.validate()?;
        if self.expected_ids.is_empty()
            || self.expected_ids.len() > usize::from(self.query.top_k)
            || self
                .expected_ids
                .iter()
                .any(|identity| !document_ids.contains(identity))
        {
            return invalid(
                "deployment conformance expected identities must be non-empty, bounded by top_k, and present in the corpus",
            );
        }
        if self.expected_ids.windows(2).any(|pair| pair[0] >= pair[1]) {
            return invalid("deployment conformance expected identities must be unique and sorted");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DeploymentConformanceDocument {
    pub id: CanonicalId,
    pub text: String,
}

impl DeploymentConformanceDocument {
    pub fn validate(&self) -> Result<()> {
        if self.text.is_empty() || self.text.len() > MAX_MESSAGE_BYTES {
            return invalid(format!(
                "deployment conformance document text must contain 1..={MAX_MESSAGE_BYTES} bytes"
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DeploymentConformanceQuery {
    pub rrflowql: String,
    pub edge_text: String,
    pub valid_at: u64,
    pub top_k: u16,
}

impl DeploymentConformanceQuery {
    pub fn validate(&self) -> Result<()> {
        if self.rrflowql.is_empty() || self.rrflowql.len() > MAX_QUERY_BYTES {
            return invalid(format!(
                "deployment conformance rrflowQL must contain 1..={MAX_QUERY_BYTES} bytes"
            ));
        }
        if self.edge_text.is_empty() || self.edge_text.len() > MAX_MESSAGE_BYTES {
            return invalid(format!(
                "deployment conformance edge query must contain 1..={MAX_MESSAGE_BYTES} bytes"
            ));
        }
        if self.valid_at == 0 || self.top_k == 0 {
            return invalid("deployment conformance valid_at and top_k must be greater than zero");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityStatus {
    Unavailable,
    Experimental,
    Available,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CapabilityDescriptor {
    pub name: CanonicalId,
    pub contract_version: u16,
    pub status: CapabilityStatus,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub limits: BTreeMap<CanonicalId, u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limitation: Option<String>,
}

impl CapabilityDescriptor {
    pub fn validate(&self) -> Result<()> {
        if self.contract_version == 0 {
            return invalid("capability contract version must be greater than zero");
        }
        if self
            .limitation
            .as_ref()
            .is_some_and(|value| value.is_empty() || value.len() > MAX_MESSAGE_BYTES)
        {
            return invalid(format!(
                "capability limitation length must be in 1..={MAX_MESSAGE_BYTES} bytes"
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ServiceCapabilities {
    pub protocol: String,
    pub protocol_version: u16,
    pub implementation: CanonicalId,
    pub implementation_version: String,
    pub deployment: DeploymentProfile,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub installed_estate: Option<InstalledEstateIdentity>,
    pub configuration: EstateConfiguration,
    pub instance: ResourceId,
    pub capabilities: Vec<CapabilityDescriptor>,
    pub product_capabilities: ProductCapabilityCatalogue,
}

impl ServiceCapabilities {
    pub fn validate(&self) -> Result<()> {
        validate_protocol(&self.protocol, self.protocol_version)?;
        self.deployment.validate()?;
        self.configuration.validate()?;
        if self.instance.kind != ResourceKind::Instance {
            return invalid("service capability identity must be an instance resource");
        }
        if let Some(installed) = &self.installed_estate {
            installed.validate()?;
            if installed.instance_id != self.instance.id {
                return invalid(
                    "service capability instance differs from installed estate identity",
                );
            }
        }
        if self.implementation_version.is_empty()
            || self.implementation_version.len() > MAX_ID_BYTES
            || !self.implementation_version.is_ascii()
        {
            return invalid(format!(
                "implementation version must be 1..={MAX_ID_BYTES} ASCII bytes"
            ));
        }
        if self.capabilities.len() > MAX_CAPABILITIES {
            return invalid(format!(
                "service may advertise at most {MAX_CAPABILITIES} capabilities"
            ));
        }
        let mut names = BTreeSet::new();
        for capability in &self.capabilities {
            capability.validate()?;
            if !names.insert(&capability.name) {
                return invalid("service capability names must be unique");
            }
        }
        if self
            .capabilities
            .windows(2)
            .any(|pair| pair[0].name > pair[1].name)
        {
            return invalid("service capabilities must be sorted by canonical name");
        }
        self.product_capabilities.validate()?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RequestEnvelope<T> {
    pub protocol: String,
    pub protocol_version: u16,
    pub context: RequestContext,
    pub resource: ResourcePath,
    pub payload: T,
}

impl<T> RequestEnvelope<T> {
    pub fn validate(&self, mutation: bool) -> Result<()> {
        validate_protocol(&self.protocol, self.protocol_version)?;
        self.context.validate(mutation)?;
        self.resource.validate()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    InvalidArgument,
    NotFound,
    AlreadyExists,
    Conflict,
    FailedPrecondition,
    Unauthenticated,
    PermissionDenied,
    ResourceExhausted,
    DeadlineExceeded,
    Cancelled,
    Unavailable,
    Corruption,
    UnsupportedVersion,
    Internal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ErrorBody {
    pub code: ErrorCode,
    pub message: String,
    pub retryable: bool,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub details: BTreeMap<CanonicalId, String>,
}

impl ErrorBody {
    pub fn validate(&self) -> Result<()> {
        if self.message.is_empty() || self.message.len() > MAX_MESSAGE_BYTES {
            return invalid(format!(
                "error message length must be in 1..={MAX_MESSAGE_BYTES} bytes"
            ));
        }
        if self
            .details
            .values()
            .any(|value| value.len() > MAX_MESSAGE_BYTES)
        {
            return invalid(format!(
                "error detail values must be at most {MAX_MESSAGE_BYTES} bytes"
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ResponseOutcome<T> {
    Ok { payload: T },
    Error { error: ErrorBody },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResponseEnvelope<T> {
    pub protocol: String,
    pub protocol_version: u16,
    pub request_id: CorrelationId,
    pub operation_id: CorrelationId,
    pub outcome: ResponseOutcome<T>,
}

impl<T> ResponseEnvelope<T> {
    pub fn validate(&self) -> Result<()> {
        validate_protocol(&self.protocol, self.protocol_version)?;
        if let ResponseOutcome::Error { error } = &self.outcome {
            error.validate()?;
        }
        Ok(())
    }
}

/// Resource limits requested for one loopback transport session. These are
/// availability boundaries, not authentication or authorization policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SessionLimits {
    pub idle_timeout_ms: u64,
    pub absolute_timeout_ms: u64,
    pub max_open_transactions: u16,
}

impl SessionLimits {
    pub fn validate(&self) -> Result<()> {
        if !(MIN_LEASE_MS..=MAX_LEASE_MS).contains(&self.idle_timeout_ms)
            || !(MIN_LEASE_MS..=MAX_LEASE_MS).contains(&self.absolute_timeout_ms)
            || self.idle_timeout_ms > self.absolute_timeout_ms
        {
            return invalid("session timeouts are outside bounds or idle exceeds absolute");
        }
        if self.max_open_transactions == 0 || self.max_open_transactions > 256 {
            return invalid("max_open_transactions must be in 1..=256");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateSession {
    pub limits: SessionLimits,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SessionLease {
    pub session_id: CorrelationId,
    pub token: CorrelationId,
    pub issued_at_unix_ms: u64,
    pub idle_expires_at_unix_ms: u64,
    pub absolute_expires_at_unix_ms: u64,
    pub limits: SessionLimits,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RenewSession {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CloseSession {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AbortTransaction {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SessionEndState {
    Closed,
    Expired,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SessionTermination {
    pub session_id: CorrelationId,
    pub state: SessionEndState,
    pub ended_at_unix_ms: u64,
    pub affected_open_transactions: u16,
    pub idempotent_replay: bool,
}

impl SessionTermination {
    pub fn validate(&self) -> Result<()> {
        if self.ended_at_unix_ms == 0 {
            return invalid("session termination time must be greater than zero");
        }
        Ok(())
    }
}

impl SessionLease {
    pub fn validate(&self) -> Result<()> {
        self.limits.validate()?;
        if self.issued_at_unix_ms == 0
            || self.idle_expires_at_unix_ms <= self.issued_at_unix_ms
            || self.absolute_expires_at_unix_ms < self.idle_expires_at_unix_ms
        {
            return invalid("session lease timestamps are inconsistent");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BeginTransaction {
    pub scope: CanonicalId,
    pub timeout_ms: u64,
}

impl BeginTransaction {
    pub fn validate(&self) -> Result<()> {
        if !(MIN_LEASE_MS..=MAX_LEASE_MS).contains(&self.timeout_ms) {
            return invalid(format!(
                "transaction timeout must be in {MIN_LEASE_MS}..={MAX_LEASE_MS}"
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TransactionState {
    Open,
    Committed,
    Aborted,
    Expired,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TransactionLease {
    pub transaction_id: CorrelationId,
    pub session_id: CorrelationId,
    pub scope: CanonicalId,
    pub read_cursor: u64,
    pub expires_at_unix_ms: u64,
    pub state: TransactionState,
}

impl TransactionLease {
    pub fn validate(&self) -> Result<()> {
        if self.expires_at_unix_ms == 0 {
            return invalid("transaction expiry must be greater than zero");
        }
        Ok(())
    }
}

pub type DataValue = QueryValue;
pub type DataProperties = BTreeMap<String, DataValue>;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DataReference {
    pub kind: CanonicalId,
    pub id: CanonicalId,
}

/// Stable target identity for CRUD and retirement. Append-only events are
/// addressed by their authenticated-log cursor instead of a fabricated user
/// identifier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "identity", rename_all = "snake_case", deny_unknown_fields)]
pub enum DataTarget {
    Reference { reference: DataReference },
    Event { kind: CanonicalId, cursor: u64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DataValueType {
    Null,
    Bool,
    Integer,
    Unsigned,
    Decimal,
    String,
    Digest,
    List,
    Map,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DataPropertySchema {
    pub value_type: DataValueType,
    #[serde(default)]
    pub required: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DataRecordSchema {
    #[serde(default)]
    pub properties: BTreeMap<String, DataPropertySchema>,
    #[serde(default)]
    pub allow_additional_properties: bool,
    #[serde(default)]
    pub unique_properties: BTreeSet<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DataRelationSchema {
    #[serde(default)]
    pub from: BTreeSet<CanonicalId>,
    #[serde(default)]
    pub to: BTreeSet<CanonicalId>,
    #[serde(default)]
    pub properties: BTreeMap<String, DataPropertySchema>,
    #[serde(default)]
    pub allow_additional_properties: bool,
    #[serde(default)]
    pub unique_pair: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_outgoing: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_incoming: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DataEventSchema {
    #[serde(default)]
    pub subject_required: bool,
    #[serde(default)]
    pub subject_types: BTreeSet<CanonicalId>,
    #[serde(default)]
    pub properties: BTreeMap<String, DataPropertySchema>,
    #[serde(default)]
    pub allow_additional_properties: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DataLogicalModel {
    Document,
    Relational,
    GraphNode,
    GraphRelation,
    KeyValue,
    Vector,
    Event,
    TimeSeries,
    Geo,
    Object,
    ReasoningClaim,
    ReasoningRecord,
    ReasoningEvent,
    LifecycleRecord,
    LifecycleEvent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DataSchemaMode {
    Strict,
    Schemaless,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DataTableSchema {
    pub model: DataLogicalModel,
    pub mode: DataSchemaMode,
    #[serde(default)]
    pub properties: BTreeMap<String, DataPropertySchema>,
    #[serde(default)]
    pub allow_additional_properties: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DataCatalogueIdentity {
    pub namespace: CanonicalId,
    pub database: CanonicalId,
}

impl Default for DataCatalogueIdentity {
    fn default() -> Self {
        Self {
            namespace: CanonicalId::new("default").expect("static namespace is valid"),
            database: CanonicalId::new("default").expect("static database is valid"),
        }
    }
}

impl DataCatalogueIdentity {
    fn is_default(&self) -> bool {
        self == &Self::default()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DataSchemaRegistry {
    pub revision: u64,
    pub migration: String,
    #[serde(default, skip_serializing_if = "DataCatalogueIdentity::is_default")]
    pub catalogue: DataCatalogueIdentity,
    pub tables: BTreeMap<CanonicalId, DataTableSchema>,
    #[serde(default)]
    pub records: BTreeMap<CanonicalId, DataRecordSchema>,
    #[serde(default)]
    pub relations: BTreeMap<CanonicalId, DataRelationSchema>,
    #[serde(default)]
    pub events: BTreeMap<CanonicalId, DataEventSchema>,
}

/// Bounded, authenticated catalogue discovery for database administration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReadDataSchema {
    pub scope: String,
    pub max_storage_keys: u32,
}

impl ReadDataSchema {
    pub fn validate(&self) -> Result<()> {
        if self.scope.is_empty() || self.scope.len() > MAX_ID_BYTES || self.scope.contains('\0') {
            return invalid("data schema scope is invalid");
        }
        if self.max_storage_keys == 0 || self.max_storage_keys > MAX_DATA_SNAPSHOT_STORAGE_KEYS {
            return invalid("data schema storage key budget is out of bounds");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DataSchemaSnapshot {
    pub scope: String,
    pub known_at_cursor: u64,
    pub read_manifest_sha256: String,
    pub schema: Option<DataSchemaRegistry>,
    pub read_evidence: ReadEvidence,
}

impl DataSchemaSnapshot {
    pub fn validate(&self) -> Result<()> {
        ReadDataSchema {
            scope: self.scope.clone(),
            max_storage_keys: 1,
        }
        .validate()?;
        validate_sha256(&self.read_manifest_sha256, "schema read manifest")?;
        self.read_evidence.validate()?;
        if let Some(schema) = &self.schema {
            if schema.revision == 0 {
                return invalid("schema revision must be non-zero");
            }
            TransactionMutation::PutSchema {
                registry: schema.clone(),
            }
            .validate()?;
        }
        Ok(())
    }
}

/// Bounded request for one all-model snapshot at the engine's current
/// authenticated transaction cursor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReadDataSnapshot {
    pub valid_at: u64,
    pub max_storage_keys: u32,
}

impl ReadDataSnapshot {
    pub fn validate(&self) -> Result<()> {
        if self.valid_at == 0 {
            return invalid("data snapshot valid_at must be greater than zero");
        }
        if self.max_storage_keys == 0 || self.max_storage_keys > MAX_DATA_SNAPSHOT_STORAGE_KEYS {
            return invalid(format!(
                "data snapshot max_storage_keys must be in 1..={MAX_DATA_SNAPSHOT_STORAGE_KEYS}"
            ));
        }
        Ok(())
    }
}

/// One current value in an all-model snapshot. `value` uses the same exact
/// typed vocabulary as writes; the wrapper supplies the catalogue model and
/// stable reference (including cursor-derived event identities).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DataSnapshotEntry {
    pub model: DataLogicalModel,
    pub target: DataTarget,
    pub value: TransactionMutation,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DataSnapshot {
    pub scope: String,
    pub valid_at: u64,
    pub known_at_cursor: u64,
    pub schema_revision: u64,
    pub read_manifest_sha256: String,
    pub selected_versions: u64,
    pub read_evidence: ReadEvidence,
    pub entries: Vec<DataSnapshotEntry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum DataVectorValue {
    Dense {
        values: Vec<f32>,
    },
    Sparse {
        dimensions: u32,
        indices: Vec<u32>,
        values: Vec<f32>,
    },
    MultiDense {
        dimensions: u32,
        vectors: Vec<Vec<f32>>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DataVectorNormalization {
    None,
    UnitL2,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DataEmbeddingProvenance {
    pub source_sha256: String,
    pub model: String,
    pub model_sha256: String,
    pub dimensions: u32,
    pub normalization: DataVectorNormalization,
    #[serde(default)]
    pub generation_parameters: DataProperties,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum DataSeriesValue {
    Integer(i64),
    Unsigned(u64),
    Decimal(String),
    Bool(bool),
    String(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DataGeoPoint {
    pub longitude: f64,
    pub latitude: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum DataGeoValue {
    Point {
        point: DataGeoPoint,
    },
    BoundingBox {
        southwest: DataGeoPoint,
        northeast: DataGeoPoint,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DataObjectReceipt {
    pub backend: String,
    pub key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub etag: Option<String>,
}

/// Public multi-model mutation vocabulary. It is deliberately independent of
/// `rrd_core`; adapters lower these values into the authoritative runtime.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "mutation", rename_all = "snake_case", deny_unknown_fields)]
pub enum TransactionMutation {
    AssertClaim {
        subject: CanonicalId,
        predicate: CanonicalId,
        object: String,
        valid_from: u64,
        tx_time: u64,
        producer: CanonicalId,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        confidence: Option<f32>,
    },
    PutSchema {
        registry: DataSchemaRegistry,
    },
    PutRecord {
        reference: DataReference,
        valid_from: u64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        valid_to: Option<u64>,
        #[serde(default)]
        properties: DataProperties,
    },
    PutRelation {
        reference: DataReference,
        from: DataReference,
        to: DataReference,
        valid_from: u64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        valid_to: Option<u64>,
        #[serde(default)]
        properties: DataProperties,
    },
    AppendEvent {
        kind: CanonicalId,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        subject: Option<DataReference>,
        #[serde(default)]
        properties: DataProperties,
    },
    PutVector {
        reference: DataReference,
        subject: DataReference,
        collection_id: CanonicalId,
        vector_name: CanonicalId,
        field: CanonicalId,
        valid_from: u64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        valid_to: Option<u64>,
        value: DataVectorValue,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        provenance: Option<DataEmbeddingProvenance>,
        #[serde(default)]
        properties: DataProperties,
    },
    AppendSeriesSample {
        reference: DataReference,
        series: DataReference,
        observed_at: u64,
        value: DataSeriesValue,
        #[serde(default)]
        properties: DataProperties,
    },
    PutGeo {
        reference: DataReference,
        subject: DataReference,
        field: CanonicalId,
        valid_from: u64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        valid_to: Option<u64>,
        value: DataGeoValue,
        #[serde(default)]
        properties: DataProperties,
    },
    PublishObjectReference {
        reference: DataReference,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        subject: Option<DataReference>,
        sha256: String,
        length: u64,
        media_type: String,
        receipt: DataObjectReceipt,
        #[serde(default)]
        properties: DataProperties,
    },
    /// Retire one currently live catalogue identity at modeled valid time.
    /// Event correction is expressed as this mutation plus one replacement
    /// `append_event` in the same transaction.
    RetireData {
        model: DataLogicalModel,
        target: DataTarget,
        effective_at: u64,
    },
}

impl TransactionMutation {
    pub fn validate(&self) -> Result<()> {
        match self {
            Self::AssertClaim {
                object,
                valid_from,
                tx_time,
                confidence,
                ..
            } => {
                if object.is_empty() || object.len() > MAX_MESSAGE_BYTES {
                    return invalid(format!(
                        "claim object length must be in 1..={MAX_MESSAGE_BYTES} bytes"
                    ));
                }
                if *valid_from == 0 || *tx_time == 0 {
                    return invalid("claim timestamps must be greater than zero");
                }
                if confidence
                    .is_some_and(|value| !value.is_finite() || !(0.0..=1.0).contains(&value))
                {
                    return invalid("claim confidence must be finite and in 0..=1");
                }
                Ok(())
            }
            Self::PutSchema { registry } => validate_data_schema(registry),
            Self::PutRecord {
                valid_from,
                valid_to,
                properties,
                ..
            }
            | Self::PutRelation {
                valid_from,
                valid_to,
                properties,
                ..
            }
            | Self::PutGeo {
                valid_from,
                valid_to,
                properties,
                ..
            } => {
                validate_data_window(*valid_from, *valid_to)?;
                validate_data_properties(properties)
            }
            Self::AppendEvent { properties, .. } => validate_data_properties(properties),
            Self::PutVector {
                valid_from,
                valid_to,
                value,
                provenance,
                properties,
                ..
            } => {
                validate_data_window(*valid_from, *valid_to)?;
                let dimensions = validate_data_vector(value)?;
                if let Some(provenance) = provenance {
                    validate_sha256(&provenance.source_sha256, "embedding source_sha256")?;
                    validate_sha256(&provenance.model_sha256, "embedding model_sha256")?;
                    if provenance.model.trim().is_empty()
                        || provenance.model.len() > MAX_MESSAGE_BYTES
                        || provenance.dimensions as usize != dimensions
                    {
                        return invalid("embedding provenance is inconsistent with the vector");
                    }
                    validate_data_properties(&provenance.generation_parameters)?;
                }
                validate_data_properties(properties)
            }
            Self::AppendSeriesSample {
                observed_at,
                value,
                properties,
                ..
            } => {
                if *observed_at == 0 {
                    return invalid("series observed_at must be greater than zero");
                }
                if matches!(value, DataSeriesValue::Decimal(value) if value.trim().is_empty()) {
                    return invalid("series decimal must not be empty");
                }
                validate_data_properties(properties)
            }
            Self::PublishObjectReference {
                sha256,
                media_type,
                receipt,
                properties,
                ..
            } => {
                validate_sha256(sha256, "object sha256")?;
                if media_type.trim().is_empty()
                    || receipt.backend.trim().is_empty()
                    || receipt.key.trim().is_empty()
                {
                    return invalid(
                        "object media type, receipt backend, and key must not be empty",
                    );
                }
                validate_data_properties(properties)
            }
            Self::RetireData {
                model,
                target,
                effective_at,
                ..
            } => {
                if *model == DataLogicalModel::ReasoningClaim {
                    return invalid(
                        "reasoning claims retire through their bitemporal claim contract",
                    );
                }
                if *effective_at == 0 {
                    return invalid("data retirement effective_at must be greater than zero");
                }
                match (data_model_is_event(*model), target) {
                    (true, DataTarget::Event { cursor, .. }) if *cursor > 0 => {}
                    (false, DataTarget::Reference { .. }) => {}
                    (true, DataTarget::Event { .. }) => {
                        return invalid("event retirement cursor must be greater than zero");
                    }
                    (true, DataTarget::Reference { .. }) => {
                        return invalid("event models retire by authenticated event cursor");
                    }
                    (false, DataTarget::Event { .. }) => {
                        return invalid("non-event models retire by canonical reference");
                    }
                }
                Ok(())
            }
        }
    }
}

fn validate_data_window(valid_from: u64, valid_to: Option<u64>) -> Result<()> {
    if valid_from == 0 || valid_to.is_some_and(|end| end <= valid_from) {
        return invalid("data validity window must start above zero and end after its start");
    }
    Ok(())
}

fn validate_data_properties(properties: &DataProperties) -> Result<()> {
    if properties
        .keys()
        .any(|name| name.is_empty() || name.len() > MAX_ID_BYTES || name.as_bytes().contains(&0))
    {
        return invalid("data property names must be bounded, non-empty, and contain no NUL");
    }
    let bytes = serde_json::to_vec(properties).map_err(|error| ContractError(error.to_string()))?;
    if bytes.len() > MAX_QUERY_PARAMETER_BYTES {
        return invalid(format!(
            "data properties may encode at most {MAX_QUERY_PARAMETER_BYTES} bytes per object"
        ));
    }
    Ok(())
}

fn validate_data_schema(registry: &DataSchemaRegistry) -> Result<()> {
    if registry.revision == 0 || registry.migration.trim().is_empty() {
        return invalid("data schema requires a positive revision and migration description");
    }
    if registry.tables.is_empty() {
        return invalid("data schema requires an explicit non-empty table map");
    }
    for (kind, table) in &registry.tables {
        if table.mode == DataSchemaMode::Schemaless
            && (!table.properties.is_empty() || table.allow_additional_properties)
        {
            return invalid(format!(
                "schemaless table {kind} cannot declare a strict property contract"
            ));
        }
        let family = data_model_family(table.model);
        if matches!(family, "record" | "relation" | "event")
            && (!table.properties.is_empty() || table.allow_additional_properties)
        {
            return invalid(format!(
                "table {kind} must keep {family} properties in its specialized schema"
            ));
        }
        let specialized = match family {
            "record" => registry.records.contains_key(kind),
            "relation" => registry.relations.contains_key(kind),
            "event" => registry.events.contains_key(kind),
            _ => false,
        };
        if matches!(family, "record" | "relation" | "event") {
            match (table.mode, specialized) {
                (DataSchemaMode::Strict, false) => {
                    return invalid(format!(
                        "strict table {kind} lacks its specialized {family} schema"
                    ));
                }
                (DataSchemaMode::Schemaless, true) => {
                    return invalid(format!(
                        "schemaless table {kind} also declares a strict {family} schema"
                    ));
                }
                _ => {}
            }
        }
    }
    for kind in registry.records.keys() {
        validate_specialized_data_table(registry, kind, "record")?;
    }
    for kind in registry.relations.keys() {
        validate_specialized_data_table(registry, kind, "relation")?;
    }
    for kind in registry.events.keys() {
        validate_specialized_data_table(registry, kind, "event")?;
    }
    for schema in registry.records.values() {
        if schema
            .unique_properties
            .iter()
            .any(|name| !schema.properties.contains_key(name))
        {
            return invalid("record schema unique properties must be declared properties");
        }
    }
    for schema in registry.relations.values() {
        if schema.from.is_empty()
            || schema.to.is_empty()
            || schema.max_outgoing == Some(0)
            || schema.max_incoming == Some(0)
        {
            return invalid("relation schema requires endpoints and positive cardinality limits");
        }
    }
    Ok(())
}

fn data_model_family(model: DataLogicalModel) -> &'static str {
    match model {
        DataLogicalModel::Document
        | DataLogicalModel::Relational
        | DataLogicalModel::GraphNode
        | DataLogicalModel::KeyValue
        | DataLogicalModel::ReasoningRecord
        | DataLogicalModel::LifecycleRecord => "record",
        DataLogicalModel::GraphRelation => "relation",
        DataLogicalModel::Event
        | DataLogicalModel::ReasoningEvent
        | DataLogicalModel::LifecycleEvent => "event",
        DataLogicalModel::Vector => "vector",
        DataLogicalModel::TimeSeries => "time_series",
        DataLogicalModel::Geo => "geo",
        DataLogicalModel::Object => "object",
        DataLogicalModel::ReasoningClaim => "claim",
    }
}

fn data_model_is_event(model: DataLogicalModel) -> bool {
    matches!(
        model,
        DataLogicalModel::Event
            | DataLogicalModel::ReasoningEvent
            | DataLogicalModel::LifecycleEvent
    )
}

fn validate_specialized_data_table(
    registry: &DataSchemaRegistry,
    kind: &CanonicalId,
    family: &str,
) -> Result<()> {
    let table = registry.tables.get(kind).ok_or_else(|| {
        ContractError(format!(
            "specialized {family} schema {kind} has no explicit table entry"
        ))
    })?;
    if data_model_family(table.model) != family || table.mode != DataSchemaMode::Strict {
        return invalid(format!(
            "table {kind} conflicts with its strict specialized {family} schema"
        ));
    }
    Ok(())
}

fn validate_data_vector(value: &DataVectorValue) -> Result<usize> {
    let finite = |values: &[f32]| values.iter().all(|value| value.is_finite());
    let dimensions = match value {
        DataVectorValue::Dense { values } => {
            if !finite(values) {
                return invalid("dense vector values must be finite");
            }
            values.len()
        }
        DataVectorValue::Sparse {
            dimensions,
            indices,
            values,
        } => {
            if indices.is_empty()
                || indices.len() != values.len()
                || indices.iter().any(|index| index >= dimensions)
                || indices.windows(2).any(|pair| pair[0] >= pair[1])
                || !finite(values)
            {
                return invalid("sparse vector indices and values are invalid");
            }
            *dimensions as usize
        }
        DataVectorValue::MultiDense {
            dimensions,
            vectors,
        } => {
            if vectors.is_empty()
                || vectors
                    .iter()
                    .any(|vector| vector.len() != *dimensions as usize || !finite(vector))
            {
                return invalid("multi-dense vector rows are invalid");
            }
            *dimensions as usize
        }
    };
    if dimensions == 0 || dimensions > MAX_VECTOR_DIMENSIONS {
        return invalid(format!(
            "vector dimensions must be in 1..={MAX_VECTOR_DIMENSIONS}"
        ));
    }
    Ok(dimensions)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CommitTransaction {
    pub operation_sha256: String,
    pub mutations: Vec<TransactionMutation>,
}

impl CommitTransaction {
    pub fn validate(&self) -> Result<()> {
        validate_sha256(&self.operation_sha256, "operation_sha256")?;
        if self.mutations.is_empty() || self.mutations.len() > MAX_TRANSACTION_CLAIMS {
            return invalid(format!(
                "transaction mutation count must be in 1..={MAX_TRANSACTION_CLAIMS}"
            ));
        }
        for mutation in &self.mutations {
            mutation.validate()?;
        }
        Ok(())
    }

    pub fn computed_operation_sha256(&self) -> String {
        transaction_operation_sha256(&self.mutations)
    }
}

/// SHA-256 over the stable typed JSON representation of the ordered mutation
/// list. Clients must use this function's cross-language equivalent rather
/// than hashing arbitrary input-object key order.
pub fn transaction_operation_sha256(mutations: &[TransactionMutation]) -> String {
    let bytes = serde_json::to_vec(mutations).expect("public transaction mutations serialize");
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    const HEX: &[u8; 16] = b"0123456789abcdef";
    for byte in digest {
        output.push(HEX[usize::from(byte >> 4)] as char);
        output.push(HEX[usize::from(byte & 0x0f)] as char);
    }
    output
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PreviewTransaction {
    pub mutations: Vec<TransactionMutation>,
    /// Valid-time coordinate for the prospective read-your-writes snapshot.
    /// Omission selects the server's single request time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valid_at: Option<u64>,
    #[serde(default = "default_transaction_preview_changes")]
    pub max_storage_keys: u32,
}

impl PreviewTransaction {
    pub fn validate(&self) -> Result<()> {
        if self.mutations.is_empty() || self.mutations.len() > MAX_TRANSACTION_CLAIMS {
            return invalid(format!(
                "transaction mutation count must be in 1..={MAX_TRANSACTION_CLAIMS}"
            ));
        }
        for mutation in &self.mutations {
            mutation.validate()?;
        }
        if self.valid_at == Some(0)
            || self.max_storage_keys == 0
            || self.max_storage_keys > MAX_DATA_SNAPSHOT_STORAGE_KEYS
        {
            return invalid(format!(
                "transaction preview requires a non-zero valid_at when present and max_storage_keys in 1..={MAX_DATA_SNAPSHOT_STORAGE_KEYS}"
            ));
        }
        Ok(())
    }
}

fn default_transaction_preview_changes() -> u32 {
    10_000
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TransactionPreview {
    pub transaction_id: CorrelationId,
    pub read_cursor: u64,
    pub operation_sha256: String,
    pub mutations: Vec<TransactionMutation>,
    pub prospective: DataSnapshot,
    pub idempotent_replay: bool,
}

impl TransactionPreview {
    pub fn validate(&self) -> Result<()> {
        validate_sha256(&self.operation_sha256, "operation_sha256")?;
        PreviewTransaction {
            mutations: self.mutations.clone(),
            valid_at: Some(self.prospective.valid_at),
            max_storage_keys: MAX_DATA_SNAPSHOT_STORAGE_KEYS,
        }
        .validate()?;
        if self.prospective.known_at_cursor
            != self
                .read_cursor
                .checked_add(self.mutations.len() as u64)
                .ok_or_else(|| ContractError("transaction preview cursor overflowed".into()))?
        {
            return invalid(
                "transaction prospective cursor must cover the read cursor and every mutation",
            );
        }
        validate_sha256(
            &self.prospective.read_manifest_sha256,
            "prospective read manifest",
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CommitReceipt {
    pub transaction_id: CorrelationId,
    pub operation_sha256: String,
    pub first_claim_sequence: u64,
    pub last_claim_sequence: u64,
    pub mutation_count: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_commit_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_runtime_cursor: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_runtime_cursor: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claim_mutation_count: Option<u64>,
    pub idempotent_replay: bool,
}

/// Selects one retained historical structural state and the later instant at
/// which compensating versions become effective. The target cursor is
/// transaction time; `target_valid_at` is modeled valid time. They remain
/// independent so rollback never collapses RRFlow's two timelines.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ForwardRollbackRequest {
    pub target_valid_at: u64,
    pub target_known_at_cursor: u64,
    pub effective_at: u64,
    pub reason: String,
}

impl ForwardRollbackRequest {
    pub fn validate(&self) -> Result<()> {
        if self.target_valid_at == 0 || self.effective_at <= self.target_valid_at {
            return invalid(
                "rollback effective time must be greater than its positive target valid time",
            );
        }
        if self.reason.trim().is_empty() || self.reason.len() > MAX_MESSAGE_BYTES {
            return invalid(format!(
                "rollback reason length must be in 1..={MAX_MESSAGE_BYTES} bytes"
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ForwardRollbackCounts {
    pub restored_records: u64,
    pub retired_records: u64,
    pub restored_relations: u64,
    pub retired_relations: u64,
}

impl ForwardRollbackCounts {
    pub fn checked_mutation_count(self) -> Option<u64> {
        self.restored_records
            .checked_add(self.retired_records)?
            .checked_add(self.restored_relations)?
            .checked_add(self.retired_relations)
    }
}

/// Deterministic compensation plan bound to the transaction's original read
/// cursor. The final mutation is an evidence claim; `operation_sha256` covers
/// the complete ordered list and feeds the existing transaction idempotency
/// authority.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ForwardRollbackPlan {
    pub request: ForwardRollbackRequest,
    pub read_cursor: u64,
    pub target_state_sha256: String,
    pub counts: ForwardRollbackCounts,
    pub operation_sha256: String,
    pub mutations: Vec<TransactionMutation>,
}

impl ForwardRollbackPlan {
    pub fn validate(&self) -> Result<()> {
        self.request.validate()?;
        if self.request.target_known_at_cursor >= self.read_cursor {
            return invalid(
                "rollback target cursor must be older than the transaction read cursor",
            );
        }
        validate_sha256(&self.target_state_sha256, "target_state_sha256")?;
        validate_sha256(&self.operation_sha256, "operation_sha256")?;
        let expected_mutations = self
            .counts
            .checked_mutation_count()
            .and_then(|count| count.checked_add(1));
        if u64::try_from(self.mutations.len()).ok() != expected_mutations {
            return invalid("rollback mutation count does not match its structural counts");
        }
        for mutation in &self.mutations {
            mutation.validate()?;
        }
        if transaction_operation_sha256(&self.mutations) != self.operation_sha256 {
            return invalid("rollback operation digest does not match its mutations");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ForwardRollbackReceipt {
    pub request: ForwardRollbackRequest,
    pub read_cursor: u64,
    pub target_state_sha256: String,
    pub counts: ForwardRollbackCounts,
    pub commit: CommitReceipt,
}

impl ForwardRollbackReceipt {
    pub fn validate(&self) -> Result<()> {
        self.request.validate()?;
        if self.request.target_known_at_cursor >= self.read_cursor {
            return invalid("rollback receipt target must be older than its read cursor");
        }
        validate_sha256(&self.target_state_sha256, "target_state_sha256")?;
        self.commit.validate()?;
        let expected_mutations = self
            .counts
            .checked_mutation_count()
            .and_then(|count| count.checked_add(1));
        if Some(self.commit.mutation_count) != expected_mutations {
            return invalid("rollback receipt does not match its committed mutation counts");
        }
        Ok(())
    }
}

impl CommitReceipt {
    pub fn validate(&self) -> Result<()> {
        validate_sha256(&self.operation_sha256, "operation_sha256")?;
        match (
            &self.runtime_commit_sha256,
            self.first_runtime_cursor,
            self.last_runtime_cursor,
            self.claim_mutation_count,
        ) {
            (None, None, None, None) => {
                if self.first_claim_sequence == 0
                    || self.last_claim_sequence < self.first_claim_sequence
                    || self.mutation_count
                        != self.last_claim_sequence - self.first_claim_sequence + 1
                {
                    return invalid("commit receipt sequence interval is inconsistent");
                }
            }
            (Some(commit), Some(first), Some(last), Some(claims)) => {
                validate_sha256(commit, "runtime_commit_sha256")?;
                if first == 0 || last < first || self.mutation_count != last - first + 1 {
                    return invalid("runtime commit receipt cursor interval is inconsistent");
                }
                if (claims == 0
                    && (self.first_claim_sequence != 0 || self.last_claim_sequence != 0))
                    || (claims > 0
                        && (self.first_claim_sequence == 0
                            || self.last_claim_sequence - self.first_claim_sequence + 1 != claims))
                {
                    return invalid("runtime commit receipt claim interval is inconsistent");
                }
            }
            _ => return invalid("runtime commit receipt fields must be supplied together"),
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Liveness {
    pub observed_at_unix_ms: u64,
}

impl Liveness {
    pub fn validate(&self) -> Result<()> {
        if self.observed_at_unix_ms == 0 {
            return invalid("liveness observation time must be greater than zero");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Readiness {
    pub observed_at_unix_ms: u64,
    pub claim_sequence: u64,
    pub runtime_cursor: u64,
    pub backend: CanonicalId,
}

impl Readiness {
    pub fn validate(&self) -> Result<()> {
        if self.observed_at_unix_ms == 0 {
            return invalid("readiness observation time must be greater than zero");
        }
        Ok(())
    }
}

pub fn validate_protocol(protocol: &str, version: u16) -> Result<()> {
    if protocol != PROTOCOL {
        return invalid(format!(
            "unsupported protocol {protocol:?}; expected {PROTOCOL:?}"
        ));
    }
    if version != PROTOCOL_VERSION {
        return invalid(format!(
            "unsupported protocol version {version}; expected {PROTOCOL_VERSION}"
        ));
    }
    Ok(())
}

fn validate_sha256(value: &str, field: &str) -> Result<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return invalid(format!("{field} must be lowercase SHA-256 hex"));
    }
    Ok(())
}

fn sha256_bytes(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    const HEX: &[u8; 16] = b"0123456789abcdef";
    for byte in digest {
        output.push(HEX[usize::from(byte >> 4)] as char);
        output.push(HEX[usize::from(byte & 0x0f)] as char);
    }
    output
}

fn invalid<T>(message: impl Into<String>) -> Result<T> {
    Err(ContractError(message.into()))
}
