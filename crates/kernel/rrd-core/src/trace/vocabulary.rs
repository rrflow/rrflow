//! Trace phases, outcomes, data classes, boundaries, operations and attributes.

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TracePhase {
    Start,
    Annotation,
    Finish,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TraceOutcome {
    Running,
    Ok,
    Error,
    Denied,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TraceDataClass {
    /// Digests, counts, timings, cursors, and other kernel control evidence.
    Control,
    /// Project/operator-authored knowledge safe under the instance policy.
    Operator,
    /// Prompt, document, model, or tool content requiring explicit retention.
    Content,
}

/// The fixed first-level operation boundary used by every RRFlow trace.
///
/// A boundary identifies the cooperating engine layer that performed the
/// work. It is not a product, process, storage authority, or lifecycle owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TraceBoundary {
    Ingress,
    Engine,
    Kv,
    Ql,
    Graph,
    Lexical,
    Vector,
    #[serde(rename = "datafusion")]
    DataFusion,
    Inference,
    Attunement,
    Routine,
    Adapter,
    Delivery,
}

impl TraceBoundary {
    pub const ALL: [Self; 13] = [
        Self::Ingress,
        Self::Engine,
        Self::Kv,
        Self::Ql,
        Self::Graph,
        Self::Lexical,
        Self::Vector,
        Self::DataFusion,
        Self::Inference,
        Self::Attunement,
        Self::Routine,
        Self::Adapter,
        Self::Delivery,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ingress => "ingress",
            Self::Engine => "engine",
            Self::Kv => "kv",
            Self::Ql => "ql",
            Self::Graph => "graph",
            Self::Lexical => "lexical",
            Self::Vector => "vector",
            Self::DataFusion => "datafusion",
            Self::Inference => "inference",
            Self::Attunement => "attunement",
            Self::Routine => "routine",
            Self::Adapter => "adapter",
            Self::Delivery => "delivery",
        }
    }
}

/// Closed low-cardinality operation catalogue for new instrumentation.
///
/// Dynamic record, route, provider, model, project, query, and error values
/// belong in typed links or bounded attributes. Adding an operation is an
/// explicit contract change; callers cannot manufacture operation names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TraceOperation {
    IngressRequest,
    IngressFrame,
    EngineOperation,
    EngineAuthenticate,
    EngineAuthorize,
    EngineValidate,
    EngineCommit,
    EngineContextAssemble,
    EngineFunctionExecute,
    KvPointRead,
    KvRangeScan,
    KvWriteBatch,
    KvWalAppend,
    KvSnapshot,
    KvPageScan,
    KvFlush,
    KvCompact,
    KvRecover,
    QlParse,
    QlBind,
    QlPlan,
    QlExecute,
    GraphMaintain,
    GraphTraverse,
    LexicalMaintain,
    LexicalSearch,
    VectorMaintain,
    VectorSearch,
    VectorRerank,
    VectorProjection,
    DataFusionPlan,
    DataFusionScan,
    DataFusionExecute,
    DataFusionSpill,
    InferenceEmbed,
    InferenceRoute,
    AttunementJob,
    AttunementPhase,
    RoutineTrigger,
    RoutineActivation,
    RoutineStep,
    RoutineActivity,
    RoutineCompensation,
    AdapterResolve,
    AdapterInvoke,
    AdapterSynchronize,
    AdapterTransfer,
    DeliveryConnect,
    DeliveryPublish,
    DeliveryAcknowledge,
    DeliveryHeartbeat,
    DeliveryResume,
    DeliveryClose,
}

impl TraceOperation {
    pub const ALL: [Self; 53] = [
        Self::IngressRequest,
        Self::IngressFrame,
        Self::EngineOperation,
        Self::EngineAuthenticate,
        Self::EngineAuthorize,
        Self::EngineValidate,
        Self::EngineCommit,
        Self::EngineContextAssemble,
        Self::EngineFunctionExecute,
        Self::KvPointRead,
        Self::KvRangeScan,
        Self::KvWriteBatch,
        Self::KvWalAppend,
        Self::KvSnapshot,
        Self::KvPageScan,
        Self::KvFlush,
        Self::KvCompact,
        Self::KvRecover,
        Self::QlParse,
        Self::QlBind,
        Self::QlPlan,
        Self::QlExecute,
        Self::GraphMaintain,
        Self::GraphTraverse,
        Self::LexicalMaintain,
        Self::LexicalSearch,
        Self::VectorMaintain,
        Self::VectorSearch,
        Self::VectorRerank,
        Self::VectorProjection,
        Self::DataFusionPlan,
        Self::DataFusionScan,
        Self::DataFusionExecute,
        Self::DataFusionSpill,
        Self::InferenceEmbed,
        Self::InferenceRoute,
        Self::AttunementJob,
        Self::AttunementPhase,
        Self::RoutineTrigger,
        Self::RoutineActivation,
        Self::RoutineStep,
        Self::RoutineActivity,
        Self::RoutineCompensation,
        Self::AdapterResolve,
        Self::AdapterInvoke,
        Self::AdapterSynchronize,
        Self::AdapterTransfer,
        Self::DeliveryConnect,
        Self::DeliveryPublish,
        Self::DeliveryAcknowledge,
        Self::DeliveryHeartbeat,
        Self::DeliveryResume,
        Self::DeliveryClose,
    ];

    pub const fn boundary(self) -> TraceBoundary {
        match self {
            Self::IngressRequest | Self::IngressFrame => TraceBoundary::Ingress,
            Self::EngineOperation
            | Self::EngineAuthenticate
            | Self::EngineAuthorize
            | Self::EngineValidate
            | Self::EngineCommit
            | Self::EngineContextAssemble
            | Self::EngineFunctionExecute => TraceBoundary::Engine,
            Self::KvPointRead
            | Self::KvRangeScan
            | Self::KvWriteBatch
            | Self::KvWalAppend
            | Self::KvSnapshot
            | Self::KvPageScan
            | Self::KvFlush
            | Self::KvCompact
            | Self::KvRecover => TraceBoundary::Kv,
            Self::QlParse | Self::QlBind | Self::QlPlan | Self::QlExecute => TraceBoundary::Ql,
            Self::GraphMaintain | Self::GraphTraverse => TraceBoundary::Graph,
            Self::LexicalMaintain | Self::LexicalSearch => TraceBoundary::Lexical,
            Self::VectorMaintain
            | Self::VectorSearch
            | Self::VectorRerank
            | Self::VectorProjection => TraceBoundary::Vector,
            Self::DataFusionPlan
            | Self::DataFusionScan
            | Self::DataFusionExecute
            | Self::DataFusionSpill => TraceBoundary::DataFusion,
            Self::InferenceEmbed | Self::InferenceRoute => TraceBoundary::Inference,
            Self::AttunementJob | Self::AttunementPhase => TraceBoundary::Attunement,
            Self::RoutineTrigger
            | Self::RoutineActivation
            | Self::RoutineStep
            | Self::RoutineActivity
            | Self::RoutineCompensation => TraceBoundary::Routine,
            Self::AdapterResolve
            | Self::AdapterInvoke
            | Self::AdapterSynchronize
            | Self::AdapterTransfer => TraceBoundary::Adapter,
            Self::DeliveryConnect
            | Self::DeliveryPublish
            | Self::DeliveryAcknowledge
            | Self::DeliveryHeartbeat
            | Self::DeliveryResume
            | Self::DeliveryClose => TraceBoundary::Delivery,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::IngressRequest => "rrflow.ingress.request",
            Self::IngressFrame => "rrflow.ingress.frame",
            Self::EngineOperation => "rrflow.engine.operation",
            Self::EngineAuthenticate => "rrflow.engine.authenticate",
            Self::EngineAuthorize => "rrflow.engine.authorize",
            Self::EngineValidate => "rrflow.engine.validate",
            Self::EngineCommit => "rrflow.engine.commit",
            Self::EngineContextAssemble => "rrflow.engine.context_assemble",
            Self::EngineFunctionExecute => "rrflow.engine.function_execute",
            Self::KvPointRead => "rrflow.kv.point_read",
            Self::KvRangeScan => "rrflow.kv.range_scan",
            Self::KvWriteBatch => "rrflow.kv.write_batch",
            Self::KvWalAppend => "rrflow.kv.wal_append",
            Self::KvSnapshot => "rrflow.kv.snapshot",
            Self::KvPageScan => "rrflow.kv.page_scan",
            Self::KvFlush => "rrflow.kv.flush",
            Self::KvCompact => "rrflow.kv.compact",
            Self::KvRecover => "rrflow.kv.recover",
            Self::QlParse => "rrflow.ql.parse",
            Self::QlBind => "rrflow.ql.bind",
            Self::QlPlan => "rrflow.ql.plan",
            Self::QlExecute => "rrflow.ql.execute",
            Self::GraphMaintain => "rrflow.graph.maintain",
            Self::GraphTraverse => "rrflow.graph.traverse",
            Self::LexicalMaintain => "rrflow.lexical.maintain",
            Self::LexicalSearch => "rrflow.lexical.search",
            Self::VectorMaintain => "rrflow.vector.maintain",
            Self::VectorSearch => "rrflow.vector.search",
            Self::VectorRerank => "rrflow.vector.rerank",
            Self::VectorProjection => "rrflow.vector.projection",
            Self::DataFusionPlan => "rrflow.datafusion.plan",
            Self::DataFusionScan => "rrflow.datafusion.scan",
            Self::DataFusionExecute => "rrflow.datafusion.execute",
            Self::DataFusionSpill => "rrflow.datafusion.spill",
            Self::InferenceEmbed => "rrflow.inference.embed",
            Self::InferenceRoute => "rrflow.inference.route",
            Self::AttunementJob => "rrflow.attunement.job",
            Self::AttunementPhase => "rrflow.attunement.phase",
            Self::RoutineTrigger => "rrflow.routine.trigger",
            Self::RoutineActivation => "rrflow.routine.activation",
            Self::RoutineStep => "rrflow.routine.step",
            Self::RoutineActivity => "rrflow.routine.activity",
            Self::RoutineCompensation => "rrflow.routine.compensation",
            Self::AdapterResolve => "rrflow.adapter.resolve",
            Self::AdapterInvoke => "rrflow.adapter.invoke",
            Self::AdapterSynchronize => "rrflow.adapter.synchronize",
            Self::AdapterTransfer => "rrflow.adapter.transfer",
            Self::DeliveryConnect => "rrflow.delivery.connect",
            Self::DeliveryPublish => "rrflow.delivery.publish",
            Self::DeliveryAcknowledge => "rrflow.delivery.acknowledge",
            Self::DeliveryHeartbeat => "rrflow.delivery.heartbeat",
            Self::DeliveryResume => "rrflow.delivery.resume",
            Self::DeliveryClose => "rrflow.delivery.close",
        }
    }

    pub fn from_name(value: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|operation| operation.as_str() == value)
    }
}

impl fmt::Display for TraceOperation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl From<TraceOperation> for String {
    fn from(value: TraceOperation) -> Self {
        value.as_str().into()
    }
}

/// Closed names for bounded attributes added by canonical instrumentation.
///
/// Stable identities and revisions belong in [`TraceLink`]. These names are
/// reserved for bounded measurements and decisions shared across engine
/// boundaries. Existing pre-convergence producers are admitted only through
/// the finite private inventory below and must converge with their behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TraceAttribute {
    Stage,
    Attempt,
    Phase,
    Action,
    Selected,
    SkipReason,
    Exact,
    Fallback,
    Truncated,
    CacheStatus,
    InputItems,
    InputBytes,
    MaxStorageKeys,
    OutputItems,
    OutputBytes,
    SelectedVersions,
    KeysExamined,
    PagesExamined,
    RowsExamined,
    ReadPointReads,
    ReadRangeScans,
    ReadValuesDecoded,
    ReadStampValidation,
    ReadStampValidationChangeReads,
    ReadStampValidationProofNodes,
    GraphSteps,
    CandidatesExamined,
    MappedBytes,
    ReadBytes,
    DecodedBytes,
    DecompressedBytes,
    BorrowedBytes,
    CopiedBytes,
    AllocatedBytes,
    ElapsedMicros,
    MemoryPeakBytes,
    SpillBytes,
    InputTokens,
    OutputTokens,
    FailedStage,
    ErrorClass,
    ErrorDigest,
    Retryable,
    Rank,
    Score,
    RrfContribution,
    SelectedSourceCount,
    SkippedSourceCount,
    CompactedBytes,
    CompactedTokens,
    VerificationStatus,
    PropagationStatus,
    PropagationDigest,
    StartCursor,
}

impl TraceAttribute {
    pub const ALL: &'static [Self] = &[
        Self::Stage,
        Self::Attempt,
        Self::Phase,
        Self::Action,
        Self::Selected,
        Self::SkipReason,
        Self::Exact,
        Self::Fallback,
        Self::Truncated,
        Self::CacheStatus,
        Self::InputItems,
        Self::InputBytes,
        Self::MaxStorageKeys,
        Self::OutputItems,
        Self::OutputBytes,
        Self::SelectedVersions,
        Self::KeysExamined,
        Self::PagesExamined,
        Self::RowsExamined,
        Self::ReadPointReads,
        Self::ReadRangeScans,
        Self::ReadValuesDecoded,
        Self::ReadStampValidation,
        Self::ReadStampValidationChangeReads,
        Self::ReadStampValidationProofNodes,
        Self::GraphSteps,
        Self::CandidatesExamined,
        Self::MappedBytes,
        Self::ReadBytes,
        Self::DecodedBytes,
        Self::DecompressedBytes,
        Self::BorrowedBytes,
        Self::CopiedBytes,
        Self::AllocatedBytes,
        Self::ElapsedMicros,
        Self::MemoryPeakBytes,
        Self::SpillBytes,
        Self::InputTokens,
        Self::OutputTokens,
        Self::FailedStage,
        Self::ErrorClass,
        Self::ErrorDigest,
        Self::Retryable,
        Self::Rank,
        Self::Score,
        Self::RrfContribution,
        Self::SelectedSourceCount,
        Self::SkippedSourceCount,
        Self::CompactedBytes,
        Self::CompactedTokens,
        Self::VerificationStatus,
        Self::PropagationStatus,
        Self::PropagationDigest,
        Self::StartCursor,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Stage => "stage",
            Self::Attempt => "attempt",
            Self::Phase => "phase",
            Self::Action => "action",
            Self::Selected => "selected",
            Self::SkipReason => "skip_reason",
            Self::Exact => "exact",
            Self::Fallback => "fallback",
            Self::Truncated => "truncated",
            Self::CacheStatus => "cache_status",
            Self::InputItems => "input_items",
            Self::InputBytes => "input_bytes",
            Self::MaxStorageKeys => "max_storage_keys",
            Self::OutputItems => "output_items",
            Self::OutputBytes => "output_bytes",
            Self::SelectedVersions => "selected_versions",
            Self::KeysExamined => "keys_examined",
            Self::PagesExamined => "pages_examined",
            Self::RowsExamined => "rows_examined",
            Self::ReadPointReads => "read_point_reads",
            Self::ReadRangeScans => "read_range_scans",
            Self::ReadValuesDecoded => "read_values_decoded",
            Self::ReadStampValidation => "read_stamp_validation",
            Self::ReadStampValidationChangeReads => "read_stamp_validation_change_reads",
            Self::ReadStampValidationProofNodes => "read_stamp_validation_proof_nodes",
            Self::GraphSteps => "graph_steps",
            Self::CandidatesExamined => "candidates_examined",
            Self::MappedBytes => "mapped_bytes",
            Self::ReadBytes => "read_bytes",
            Self::DecodedBytes => "decoded_bytes",
            Self::DecompressedBytes => "decompressed_bytes",
            Self::BorrowedBytes => "borrowed_bytes",
            Self::CopiedBytes => "copied_bytes",
            Self::AllocatedBytes => "allocated_bytes",
            Self::ElapsedMicros => "elapsed_micros",
            Self::MemoryPeakBytes => "memory_peak_bytes",
            Self::SpillBytes => "spill_bytes",
            Self::InputTokens => "input_tokens",
            Self::OutputTokens => "output_tokens",
            Self::FailedStage => "failed_stage",
            Self::ErrorClass => "error_class",
            Self::ErrorDigest => "error_digest",
            Self::Retryable => "retryable",
            Self::Rank => "rank",
            Self::Score => "score",
            Self::RrfContribution => "rrf_contribution",
            Self::SelectedSourceCount => "selected_source_count",
            Self::SkippedSourceCount => "skipped_source_count",
            Self::CompactedBytes => "compacted_bytes",
            Self::CompactedTokens => "compacted_tokens",
            Self::VerificationStatus => "verification_status",
            Self::PropagationStatus => "propagation_status",
            Self::PropagationDigest => "propagation_digest",
            Self::StartCursor => "start_cursor",
        }
    }

    pub fn from_name(value: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|attribute| attribute.as_str() == value)
    }
}

impl fmt::Display for TraceAttribute {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl From<TraceAttribute> for String {
    fn from(value: TraceAttribute) -> Self {
        value.as_str().into()
    }
}
