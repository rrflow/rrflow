//! Schema-captured GraphQL ingress for the canonical rrflowQL query model.
//!
//! This module is deliberately an adapter, not an executor. It parses one
//! bounded GraphQL query operation, validates the lowerable surface against a
//! [`Catalog`] captured at one [`ReadStamp`], and produces the same [`Query`]
//! and [`BoundQuery`] used by rrflowQL. It performs no storage I/O and owns no
//! resolver, authorization decision, read stamp, or response lifecycle.

use crate::plan::fields_for_source;
use crate::{
    bind, BoundQuery, Catalog, ComparisonOperator, CursorExpr, Error, Filter, Join, Parameters,
    Projection, Query, Result, Source, TemporalSelector, TimeExpr, TraversalDirection, ValueExpr,
    QUERY_CONTRACT_VERSION,
};
use async_graphql_parser::types::{
    BaseType, DocumentOperations, OperationDefinition, OperationType, Selection, Type,
};
use async_graphql_parser::{parse_query, Positioned};
use async_graphql_value::{ConstValue, Value};
use rrd_core::{
    digest, Predicate, ReadStamp, RuntimeId, RuntimeRef, RuntimeType, RuntimeValue,
    RuntimeValueType,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

mod lowering;
mod schema;
mod variables;

pub use lowering::*;
pub use schema::*;
use variables::*;
