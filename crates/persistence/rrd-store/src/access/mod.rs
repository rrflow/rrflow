//! Canonical encoded access paths shared by semantic repositories.
//!
//! This boundary owns physical key selection, stamped reads, and preparation
//! of canonical semantic write plans. It does not authenticate,
//! authorize, select a storage profile, or publish a transaction.

mod function;
mod index;
mod proof_nodes;
pub(crate) mod runtime_state;
mod semantic_commit;
mod vector;
mod versioned_read;

pub(crate) use function::{
    encode_function_invocation_receipts, put_standalone_function_receipt, validate_coordinate,
};
pub use function::{
    FunctionArtifactMediaTypeRecord, FunctionArtifactRecord, FunctionCatalogueHeadRecord,
    FunctionCatalogueMembershipRecord, FunctionCataloguePublication, FunctionCatalogueSnapshot,
    FunctionDefinitionRecord, FunctionInvocationReceiptRecord, TransactionFunctionBindingRecord,
    FUNCTION_CATALOGUE_STATE_FORMAT_VERSION, FUNCTION_INVOCATION_RECEIPT_FORMAT_VERSION,
};
pub(crate) use index::{
    encode_record_index_effects, index_source_deltas, synchronize_index_commit_bindings,
};
pub use index::{
    IndexCommitBindingDefinition, IndexCommitBindingKind, IndexSourceDelta, IndexSourceRow,
    INDEX_COMMIT_BINDING_CONTRACT_VERSION, INDEX_SOURCE_DELTA_CONTRACT_VERSION,
};
pub(crate) use semantic_commit::prepare_semantic_commit;
pub use semantic_commit::SemanticCommitPlan;
pub(crate) use vector::{
    encode_vector_retirement, encode_vector_source_effects, encode_vector_version,
    vector_source_deltas,
};
pub use vector::{
    VectorSourceAddress, VectorSourceDelta, VectorSourceVersion,
    VECTOR_SOURCE_DELTA_CONTRACT_VERSION,
};
pub(crate) use versioned_read::{read_versioned, read_versioned_expanding, schema_at_read};
pub use versioned_read::{
    RuntimeAdjacencyDirection, RuntimeReadAccessPath, RuntimeReadBudget, RuntimeReadEvidence,
    RuntimeReadPathEvidence, RuntimeVersionedRead, RuntimeVersionedSource,
    RUNTIME_VERSIONED_READ_CONTRACT_VERSION,
};
