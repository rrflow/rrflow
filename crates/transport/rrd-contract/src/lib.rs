//! Stable, transport-neutral public contracts for RRD.
//!
//! Internal storage, query, cluster, and UI types must not leak into this
//! boundary. HTTP, gRPC, embedded clients, SDKs, and Connectome consume the
//! same serialized vocabulary. Version 1 intentionally freezes only the
//! coordinates needed before those outward surfaces are implemented.

mod attunement;
mod capability_surface;
mod cold_history;
mod deployment;
mod diagnostic;
mod storage_cursor;
pub use storage_cursor::{InspectStorageCursor, StorageCursorAction, StorageCursorSnapshot};
mod endpoint_binding;
mod endpoint_configuration;
mod endpoint_generation;
mod endpoint_worker;
pub use endpoint_generation::{
    EndpointGenerationSettings, EndpointResponseFormat, EndpointTokenParameter,
};
pub use endpoint_worker::{
    EndpointWorkerAdapter, EndpointWorkerBinding, EndpointWorkerPolicy, TaskWorkerContext,
    TaskWorkerOutput, TaskWorkerRequest, TaskWorkerSourceProvenance, TaskWorkerSourceRequest,
    ENDPOINT_WORKER_VERSION, TASK_WORKER_SOURCE_VERSION,
};
mod endpoint_policy;
mod endpoint_proposal;
mod endpoint_selection;
mod task_assessment;
pub use endpoint_policy::{EndpointSelectionPolicy, LfgAnalysisPolicy};
pub use endpoint_proposal::*;
pub use endpoint_selection::{
    EndpointAbstentionReason, EndpointEffortSupport, EndpointSelectionAllowance,
    EndpointSelectionCandidate, EndpointSelectionChoice, EndpointSelectionLimits,
    ENDPOINT_SELECTION_VERSION,
};
pub use task_assessment::*;
mod observation_source;
pub use observation_source::*;
mod observation_delivery;
pub use observation_delivery::*;
mod flight;
mod function;
#[path = "generated/signal_catalogue.rs"]
mod generated_signal_catalogue;
mod inference;
mod knowledge;
mod knowledge_tree;
mod memory_context;
mod memory_engram;
mod model_manifest;
mod operation_reference;
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
    attunement_capability, attunement_capability_configuration_sha256,
    attunement_checkpoint_sha256, attunement_plan_sha256, attunement_replan_accept_sha256,
    attunement_replan_preview_sha256, attunement_replan_seed, attunement_verification_sha256,
    default_source_parse_policy, derive_attunement_phases, derive_attunement_plan,
    installation_initial_grants, installation_plan_sha256, installation_result_sha256,
    validate_adopted_engram_entries, AcceptAttunementReplan, AttunementCancelResult,
    AttunementCapability, AttunementControlResponse, AttunementEvidence, AttunementFailure,
    AttunementJob, AttunementJobPage, AttunementJobState, AttunementJobSummary, AttunementLease,
    AttunementPhase, AttunementPhaseCheckpoint, AttunementPhasePlan, AttunementPlan,
    AttunementPrecondition, AttunementReadResult, AttunementReceipt, AttunementReceiptReadResult,
    AttunementReplanPreview, AttunementReplanReceiptReadResult, AttunementReplanReceiptView,
    AttunementReplanResult, AttunementRuntimeCoordinates, AttunementSourceVerification,
    AttunementStatus, AttunementVerification, AttunementVerificationCheck,
    AttunementVerificationStatus, CancelAttunement, InstallationActionDisposition,
    InstallationActionKind, InstallationActionResult, InstallationBootstrap,
    InstallationBootstrapPolicy, InstallationManagedPath, InstallationManagedPathKind,
    InstallationPlan, InstallationPlanAction, InstallationPublicationProof, InstallationRecordKind,
    InstallationRemovalRule, InstallationResult, InstallationSpecialization,
    InstallationStorageGeneration, InstallationTargetKind, ListAttunementJobs, PauseAttunement,
    PreviewAttunementReplan, ReadAttunement, ReadAttunementReceipt, RecoverExpiredAttunement,
    ResumeAttunement, ResumeAttunementWithLease, SourceParseBinding, SourceParsePolicy,
    StartAttunement, StepAttunement, ADOPTED_ENGRAM_DIRECTORY, ATTUNEMENT_CAPABILITIES,
    ATTUNEMENT_CONTRACT_VERSION, ATTUNEMENT_EVIDENCE_FORMAT_VERSION,
    INSTALLATION_BOOTSTRAP_VERSION, INSTALLATION_LEASE_PATH, INSTALLATION_LEASE_PRECONDITION,
    INSTALLATION_PLAN_CONTRACT_VERSION, INSTALL_ATTUNEMENT_CONTRACT_VERSION,
    MAX_ADOPTED_ENGRAM_ENTRIES, MAX_ATTUNEMENT_CANCEL_REASON_BYTES,
    MAX_ATTUNEMENT_DESCRIPTION_BYTES, MAX_ATTUNEMENT_FAILURE_BYTES, MAX_ATTUNEMENT_JOB_PAGE,
    MAX_ATTUNEMENT_JOB_PAGE_BYTES, MAX_ATTUNEMENT_PHASES, MAX_ATTUNEMENT_VERIFICATION_CHECKS,
    MAX_BOOTSTRAP_RECORD_BYTES, MAX_INSTALLATION_ACTIONS,
};
pub use capability_surface::{
    ProductCapability, ProductCapabilityCatalogue, ProductSurface, SurfaceBinding,
    SurfaceDisposition,
};
pub use cold_history::{
    ColdHistoryFailure, ColdHistoryFailureCode, ColdHistoryGenerationSnapshot,
    ColdHistoryReadResult, ReadColdHistory,
};
pub use deployment::{
    engram_configuration_sha256, AttunementFollowPolicy, ClockPolicy, ColdHistoryCompression,
    ColdHistoryPolicy, DeploymentForm, DeploymentProfile, EndpointPresentation,
    EngramConfiguration, EngramConfigurationInput, InstalledEngramIdentity, ReasoningLimits,
    RecallLimits, StorageLimits, StorageProfileKind, DEPLOYMENT_PROFILE_CONTRACT_VERSION,
    ENGRAM_CONFIGURATION_FORMAT_VERSION, MAX_CLOCK_ROLLBACK_MS, MAX_REASONING_RUN_ELAPSED_MS,
    MAX_REASONING_STEPS, MAX_REASONING_STEP_ELAPSED_MS,
};
pub use diagnostic::{
    DiagnosticAuthority, DiagnosticCoverage, DiagnosticGraphDifference,
    DiagnosticGraphRecordChange, DiagnosticGraphRecordSnapshot, DiagnosticGraphRelationChange,
    DiagnosticGraphRelationSnapshot, DiagnosticGraphSnapshot, DiagnosticMaintenanceSnapshot,
    DiagnosticMemtableSnapshot, DiagnosticModelCatalogueSnapshot, DiagnosticModelKind,
    DiagnosticModelSnapshot, DiagnosticPageCacheSnapshot, DiagnosticReadStamp,
    DiagnosticRetentionPin, DiagnosticRetentionSnapshot, DiagnosticRrflowCldSnapshot,
    DiagnosticRrflowKvSnapshot, DiagnosticRuntimeReference, DiagnosticSectionSnapshot,
    DiagnosticSegmentIoMode, DiagnosticSegmentIoSnapshot, DiagnosticSegmentSetSnapshot,
    DiagnosticSnapshot, DiagnosticSnapshotLease, DiagnosticStorageAdmission,
    DiagnosticStorageEvidenceLevel, DiagnosticStorageSnapshot,
    DiagnosticVectorArtifactCatalogueSnapshot, DiagnosticVectorArtifactKind,
    DiagnosticVectorArtifactSnapshot, DiagnosticWriteAheadLogSnapshot, ReadDiagnosticSnapshot,
    ReadStorageDiagnostics, DIAGNOSTIC_SNAPSHOT_FORMAT_VERSION, MAX_DIAGNOSTIC_AUDIT_RECORDS,
    MAX_DIAGNOSTIC_GRAPH_RECORDS, MAX_DIAGNOSTIC_RUNTIME_SCANNED_CHANGES,
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
pub use memory_engram::{
    MemoryEngramPlan, MemorySeatDefinition, MemoryWarp, PersistMemoryEngram,
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
pub use operation_reference::{
    operation_families, referenced_operations, OperationFamily, OperationFamilyDescriptor,
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
    SdkConformanceBackup, SdkConformanceChangefeed, SdkConformanceColdHistory,
    SdkConformanceCorpus, SdkConformanceExpected, SdkConformanceIdentity, SdkConformanceSession,
    SdkConformanceTransaction, SdkConformanceVector, SDK_CONFORMANCE_FORMAT_VERSION,
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
use serde::{Deserialize, Serialize};

mod audit;
mod backup;
mod catalogue;
mod changefeed;
mod commit;
mod conformance;
mod data;
mod engram_authority;
mod envelope;
mod error;
mod health;
mod identity;
mod openapi;
mod points;
mod protocol;
mod query;
mod retrieval;
mod session;
mod transaction;
mod validation;
mod vector;
mod vector_search;

pub use audit::*;
pub use backup::*;
pub use catalogue::*;
pub use changefeed::*;
pub use commit::*;
pub use conformance::*;
pub use data::*;
pub use engram_authority::*;
pub use envelope::*;
pub use error::*;
pub use health::*;
pub use identity::*;
pub use openapi::*;
pub use points::*;
pub use protocol::*;
pub use query::*;
pub use retrieval::*;
pub use session::*;
pub use transaction::*;
pub use validation::*;
pub use vector::*;
pub use vector_search::*;
