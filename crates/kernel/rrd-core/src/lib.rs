//! # rrd-core
//!
//! The rrflow kernel: semantic identities, bi-temporal claims, and supersession.
//!
//! ## Boundary
//!
//! This crate depends only on `serde` and a pure cryptographic primitive. It must
//! not depend on the substrate, transport or tier policy, or expose their types
//! in its public API. Adapters implement [`temporal::ClaimSource`].
//!
//! `SPEC.md` §5 states the modularity criterion: a module satisfies the
//! specification if and only if it compiles and passes its tests with every
//! outward module removed. [`reference::MemoryClaims`] is the in-crate
//! implementation that allows this crate to satisfy that criterion, and serves
//! as the grounding reference defined in `SPEC.md` §8.3.
//!
//! ## Timelines
//!
//! - **valid time** (`valid_from`, `valid_to`) — the interval during which a
//!   claim holds in the modelled domain
//! - **transaction time** (`tx_time`) — the instant the kernel recorded it
//!
//! Superseded claims are retired, not deleted. Retirement makes staleness
//! representable, which is the mechanism by which this specification prevents
//! drift.
//!
//! ## Clocks
//!
//! The kernel does not read a clock. Every operation requiring the current
//! instant takes it as a parameter, so that results are reproducible and tests
//! are deterministic.

pub mod authenticated_log;
pub mod claim;
pub mod data;
pub mod digest;
pub mod error;
pub mod ident;
pub mod reasoning_tree;
pub mod reference;
pub mod result_identity;
pub mod runtime;
pub mod schema;
pub mod telemetry;
pub mod temporal;
pub mod trace;

pub use authenticated_log::{
    RuntimeInclusionProof, RuntimeLogAccumulator, RuntimeMerkleNode,
    RUNTIME_LOG_ACCUMULATOR_VERSION,
};
pub use claim::{supersede, Claim, Millis, Producer, PromotionState, Tier};
pub use data::{
    EmbeddingProvenance, GeoPoint, GeoValue, ObjectReceipt, ObjectReference, RuntimeGeo,
    RuntimeSeriesSample, RuntimeVector, SeriesValue, VectorCollectionAddress, VectorNormalization,
    VectorValue,
};
pub use error::{Error, Result};
pub use ident::{Predicate, Reader, Subject};
pub use reasoning_tree::{
    ReasoningActiveCursor, ReasoningCondition, ReasoningConditionEvaluation,
    ReasoningConditionPredicate, ReasoningCursorAdvance, ReasoningDecisionEvidence, ReasoningEdge,
    ReasoningEvidence, ReasoningNode, ReasoningRecipe, ReasoningRecipeSelection, ReasoningTree,
    ReasoningVerificationResult, ReasoningVerificationStatus, MAX_REASONING_EDGE_CONDITIONS,
    MAX_REASONING_EVIDENCE_ITEMS, MAX_REASONING_EVIDENCE_SOURCE_BYTES,
    MAX_REASONING_EVIDENCE_SUMMARY_BYTES, MAX_REASONING_TREE_EDGES, MAX_REASONING_TREE_NODES,
    MAX_REASONING_TREE_RECIPES, REASONING_TREE_CONTRACT_VERSION,
};
pub use runtime::{
    projection_family, AuditDecision, AuditEnvelope, DataTransaction, DataTransactionView,
    OutboxId, ProjectionFamily, ProjectionId, ProjectionStamp, ProjectionState, ProjectionWork,
    ReadStamp, RetentionPin, RetentionPinId, RuntimeChange, RuntimeChangePage, RuntimeCommit,
    RuntimeCommitOutcome, RuntimeDataSnapshot, RuntimeEvent, RuntimeEventValue, RuntimeGraphDiff,
    RuntimeGraphSnapshot, RuntimeId, RuntimeModelValue, RuntimeMutation, RuntimeProperties,
    RuntimeReadValidation, RuntimeRecord, RuntimeRecordChange, RuntimeRef, RuntimeRelation,
    RuntimeRelationChange, RuntimeRetirement, RuntimeType, RuntimeValue, ScopeId, SnapshotHandle,
    SnapshotId, DATA_RUNTIME_CONTRACT_VERSION,
};
pub use schema::{
    RuntimeCategorySchema, RuntimeCategoryTargets, RuntimeDomainSchema, RuntimeEngramOrganisation,
    RuntimeEventSchema, RuntimeLogicalModel, RuntimePropertySchema, RuntimeRecordSchema,
    RuntimeRelationSchema, RuntimeSchemaMode, RuntimeSchemaRegistry, RuntimeTableSchema,
    RuntimeTagCardinality, RuntimeTagRef, RuntimeTagSchema, RuntimeValueType, DEFAULT_DOMAIN,
    ENGINE_DOMAIN, ENGINE_OWNED_PREFIX, MAX_CATEGORIES, MAX_DOMAINS, MAX_DOMAIN_DEPTH,
    MAX_ORGANISATION_DESCRIPTION_BYTES, MAX_ORGANISATION_TITLE_BYTES, MAX_TAGS_PER_CATEGORY,
    MAX_TAGS_PER_KIND,
};
pub use telemetry::{
    signal_catalogue, signal_catalogue_sha256, DiagnosticLevel, MetricAttribute, MetricDescriptor,
    MetricInstrument, MetricKind, MetricUnit, SignalCatalogue, TraceAttributeDescriptor,
    TraceOperationDescriptor, DEFAULT_METRIC_CARDINALITY_LIMIT, TELEMETRY_CONTRACT_VERSION,
};
pub use temporal::{changed_since, resolve_as_of, ClaimReader, ClaimSource};
pub use trace::{
    RuntimeTraceEvent, SpanId, TraceAttribute, TraceBoundary, TraceCausalRelation, TraceDataClass,
    TraceId, TraceLink, TraceOperation, TraceOutcome, TracePhase, RUNTIME_TRACE_CONTRACT_VERSION,
    RUNTIME_TRACE_EVENT_TYPE,
};

static CHILD_LAUNCH: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Held across every child-process launch in the engine. Windows pipes given to
/// a child are briefly inheritable, and a sibling launch inheriting all handles
/// in that window would receive them.
pub fn child_launch_lock() -> std::sync::MutexGuard<'static, ()> {
    CHILD_LAUNCH
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
