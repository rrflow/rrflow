//! rrflowQL syntax, deterministic binding, planning, and execution.
//!
//! The implementation begins from exact, typed semantic version ranges at one
//! authenticated read stamp. Immutable projections may compete only when the
//! bound query proves their source/schema coordinates and artifact digest;
//! DataFusion evaluates the resulting rows but never owns transactional state.

mod arrow;
mod bm25;
mod catalog;
mod error;
mod execute;
mod fusion;
mod graphql;
mod index;
mod live;
/// Native physical access operators. The module root file is planned for F-03.
mod physical {
    pub(crate) mod graph;
}
mod pipeline;
mod plan;
mod syntax;

pub use arrow::{
    record_batch_to_rows, rows_to_arrow_snapshot, rows_to_record_batch, ArrowReadStamp,
    ArrowSnapshot,
};
pub use bm25::{
    highlight_offsets, Bm25Analyzer, Bm25Artifact, Bm25Config, Bm25Document, Bm25Hit, Bm25Offset,
    Bm25Posting, Bm25Stemmer, Bm25Tokenizer, BM25_ARTIFACT_CONTRACT_VERSION,
};
pub use catalog::{Catalog, SchemaVersion, SourceWatermarks};
pub use error::{Error, Result};
pub use execute::{execute, ExecutionBudget, QueryBatch, QueryExecution, QueryRow};
pub use fusion::{execute_snapshot, FusionAnalysis, FusionBudget};
pub use graphql::{
    bind_graphql, derive_graphql_schema, lower_graphql, GraphqlFieldDefinition, GraphqlRequest,
    GraphqlSchema, GraphqlSourceDefinition, GraphqlSourceFamily, LoweredGraphqlQuery,
    GRAPHQL_ADAPTER_CONTRACT_VERSION, MAX_GRAPHQL_DOCUMENT_BYTES, MAX_GRAPHQL_VARIABLES,
};
pub use index::{
    IndexAnalyticsArtifact, IndexArtifact, IndexArtifactPublication, IndexArtifactReference,
    IndexCatalogue, IndexCatalogueRepository, IndexCountGroup, IndexDefinition, IndexEntry,
    IndexKind, IndexMaintenanceEvidence, IndexMutationContext, IndexOperationReceipt,
    INDEX_ARTIFACT_CONTRACT_VERSION, INDEX_CATALOGUE_CONTRACT_VERSION,
};
pub use live::{poll_live_query, LiveQueryBudget, LiveQueryDelta, LiveRowChange};
pub use pipeline::{StampedQueryExecution, StampedQueryPipeline};
pub use plan::{
    bind, plan, BoundFilter, BoundIndexCandidate, BoundQuery, CandidatePath, ExecutionContract,
    LogicalOperator, LogicalPlan, Parameters, PhysicalOperator, PhysicalPlan, PlanExplanation,
    QueryFieldTypes,
};
pub use syntax::*;
