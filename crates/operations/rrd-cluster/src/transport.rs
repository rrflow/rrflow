//! Mutually authenticated, bounded OpenRaft RPC transport.
//!
//! Transport v2 uses one request per TLS connection, disables application
//! bearer credentials, and binds the TLS URI SAN to the canonical cluster/node
//! identity carried by the replicated membership. OpenRaft retains ownership of
//! retry, ordering, duplicate, and chunked-snapshot semantics.

use crate::{
    ArtifactTransferManifest, ArtifactTransferObservation, ArtifactTransferObserver,
    ArtifactTransferReceipt, ArtifactTransferReceiver, ArtifactTransferRpc,
    ArtifactTransferRpcResult, ClusterError, ClusterId, NodeId, Result as ClusterResult,
    RrdRaftCommand, RrdRaftNode, RrdRaftStateMachine, RrdRaftTypeConfig,
    RrdTransportAdmissionPolicy, RrdTransportOperation, RrdTransportOutcome, RrdTransportTelemetry,
    RrdTransportTelemetrySnapshot, ShardId, ARTIFACT_TRANSFER_CHUNK_MAX_BYTES,
};
use openraft::error::{
    ClientWriteError, Fatal, InstallSnapshotError, NetworkError, RPCError, RaftError, RemoteError,
    ReplicationClosed, StreamingError, Timeout, Unreachable,
};
use openraft::network::snapshot_transport::{Chunked, SnapshotTransport};
use openraft::network::{RPCOption, RPCTypes, RaftNetwork, RaftNetworkFactory};
use openraft::raft::{
    AppendEntriesRequest, AppendEntriesResponse, ClientWriteResponse, InstallSnapshotRequest,
    InstallSnapshotResponse, SnapshotResponse, VoteRequest, VoteResponse,
};
use openraft::{OptionalSend, Raft, Snapshot, Vote};
use rrd_core::digest::sha256_hex;
use rrd_core::ScopeId;
use rrd_store::LocalObjectStore;
use rustls::client::WebPkiServerVerifier;
use rustls::pki_types::{CertificateDer, CertificateRevocationListDer, PrivateKeyDer, ServerName};
use rustls::server::WebPkiClientVerifier;
use rustls::{ClientConfig, RootCertStore, ServerConfig};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::future::Future;
use std::io;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncSeekExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Semaphore;
use tokio_rustls::{TlsAcceptor, TlsConnector};
use x509_parser::extensions::GeneralName;
use x509_parser::prelude::{FromDer, X509Certificate};

mod config;
mod network;
mod server;
mod wire;

pub use config::*;
pub use network::*;
pub use server::*;
pub use wire::*;

#[cfg(test)]
mod tests;
