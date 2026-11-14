//! Authoritative publication and restart reconstruction for vector artifacts.
//!
//! The in-process `VectorRuntime` is only a serving view. This module makes a
//! strict runtime record and a verified immutable object reference the source
//! of truth, committed together through `RRD storage coordinator` and surrounded by a durable
//! control-plane trace.

use super::{DurableTraceSpan, TraceIdentity};
use rrd_core::{
    DataTransaction, Millis, ObjectReference, ProjectionId, ReadStamp, RuntimeChange,
    RuntimeCommit, RuntimeCommitOutcome, RuntimeId, RuntimeLogicalModel, RuntimeMutation,
    RuntimeProperties, RuntimePropertySchema, RuntimeRecord, RuntimeRecordSchema, RuntimeRef,
    RuntimeSchemaRegistry, RuntimeTableSchema, RuntimeType, RuntimeValue, RuntimeValueType,
    ScopeId, TraceBoundary, TraceDataClass, TraceLink, TraceOutcome, VectorCollectionAddress,
};
use rrd_store::{
    DataRuntimeAccess, Error as StoreError, ImmutableObjectStore, RuntimeReadBudget,
    RuntimeVersionedRead, RuntimeVersionedSource, StorageEngine,
};
use rrd_vector::{
    QuantizationArtifactCatalogue, QuantizationArtifactEntry, QuantizationArtifactState,
    QuantizationLifecycleAction, QuantizationLifecycleEvent, QuantizationOperationReceipt,
    VectorArtifact, VectorArtifactCatalogEntry, VectorCandidate, VectorRuntime,
    QUANTIZATION_ARTIFACT_RECORD_TYPE, QUANTIZATION_LIFECYCLE_RECORD_TYPE,
    VECTOR_ARTIFACT_RECORD_TYPE,
};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

// Control-plane inspection/publication has a finite ceiling. Search uses its
// caller-supplied budget, shared with candidates and object authentication.

mod catalog;
mod publication;
mod quantization;
mod trace;

pub use catalog::*;
pub use publication::*;
pub use quantization::*;
use trace::*;
