use super::{
    content_digest, invalid, quote_identifier, validate_name, validate_pg_identifier,
    validate_version, IterativeScanMode, OperatorAccessPath, OperatorAdapterDescriptor,
    OperatorKnowledgeAdapter, OperatorKnowledgeBinding, OperatorKnowledgeWriter,
    OperatorPlanEvidence, OperatorSearchHit, OperatorSearchRequest, OperatorSearchResult,
    OperatorSourceRevision, OperatorSyncOperation, OperatorSyncReceipt, OperatorSyncWork,
    OperatorVectorKind, PgvectorRelation, OPERATOR_KNOWLEDGE_CONTRACT_VERSION,
};
use native_tls::{Certificate, TlsConnector};
use postgres::{config::SslMode, Client, Config, GenericClient, IsolationLevel, Transaction};
use postgres_native_tls::MakeTlsConnector;
use rrd_core::{digest, Error, Result};
use rrd_vector::{ScoreMetric, VectorQuery};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::str::FromStr;
use std::time::Instant;

mod adapter;
mod catalog;

pub use adapter::*;
use catalog::*;

#[cfg(test)]
mod tests;
