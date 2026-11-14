//! Process boundary for one durable RRD Raft node.
//!
//! Raft traffic uses the authenticated transport. Administrative lifecycle
//! commands use a bounded, versioned JSON-lines protocol over the process's
//! inherited stdin/stdout. This is current cluster characterization, not a
//! canonical lifecycle or administrative authority; the distributed
//! convergence gate must place administration behind `RrdEngine`.

use crate::transport::RrdConsensusCommitError;
use crate::{
    artifact_transfer_trace_event, ArtifactTransferObservation, ArtifactTransferObserver,
    ArtifactTransferReceiver, ClusterError, ClusterId, NodeId, Result as ClusterResult,
    RrdRaftCommand, RrdRaftNetworkFactory, RrdRaftNode, RrdRaftResponse, RrdRaftStateMachine,
    RrdRaftStore, RrdRaftTimingPolicy, RrdRaftTlsServer, RrdTlsGeneration, RrdTlsMaterial,
    RrdTlsReloader, RrdTransportAdmissionPolicy, RrdTransportBinding, RrdTransportGate,
    RrdTransportTelemetrySnapshot, RrdTransportTrust, ShardId, ShardPlacement,
};
use openraft::metrics::Metric;
use openraft::{Config, Raft, SnapshotPolicy};
use rrd_core::{RuntimeCommit, ScopeId};
use rustls::pki_types::{
    CertificateDer, CertificateRevocationListDer, PrivateKeyDer, PrivatePkcs8KeyDer,
};
use rustls::RootCertStore;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;

mod protocol;
mod runtime;
mod telemetry;

pub use protocol::*;
pub use runtime::*;
pub use telemetry::*;

#[cfg(test)]
mod tests;
