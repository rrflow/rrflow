//! OpenRaft adapter over the RRD-native durable key/value substrate.
//!
//! Adapter v4 physically separates node-local Raft history from transferable
//! canonical state. Votes, logs, purge/commit cursors, and snapshot cache
//! references stay local; state-machine application and canonical runtime data
//! share one authoritative RRD LSM WAL frame.

// OpenRaft fixes `StorageError` as the error type in its storage traits. The
// enum deliberately carries rich Raft context and is larger than Clippy's
// generic threshold, so adapter helpers preserve that required type too.
#![allow(clippy::result_large_err)]

use crate::{
    transfer_artifacts, ArtifactTransferManifest, ArtifactTransferReceipt, ClusterError, NodeId,
    ReplicaTransferPlan, Result as ClusterResult, ShardId, ShardPlacement, ShardReadStamp,
    CLUSTER_CONTRACT_VERSION,
};
use openraft::storage::{RaftLogStorage, RaftStateMachine};
use openraft::{
    Entry, EntryPayload, ErrorSubject, ErrorVerb, LogId, LogState, RaftLogReader,
    RaftSnapshotBuilder, RaftTypeConfig, Snapshot, SnapshotMeta, StorageError, StoredMembership,
    Vote,
};
use rrd_core::{
    digest::sha256_hex, ObjectReference, ReadStamp, RuntimeCommit, RuntimeCommitOutcome,
    RuntimeSchemaRegistry, ScopeId,
};
use rrd_lsm::filesystem::{AnchorInput, AnchoredPath, OpenOptions};
use rrd_lsm::{
    Database, Durability, Mutation, SnapshotBundleFile, WriteBatch, SNAPSHOT_BUNDLE_MAX_BYTES,
};
use rrd_store::{
    create_rrflow_kv_database, is_rrflow_kv_database, prepare_rrflow_kv_commit,
    rrflow_kv_commit_context, rrflow_kv_commit_outcome, rrflow_kv_snapshot_all_object_references,
    rrflow_kv_snapshot_artifact_view, rrflow_kv_snapshot_object_references, Error as StoreError,
    LocalObjectStore,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Debug;
use std::io;
use std::ops::{Bound, RangeBounds};
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, Weak};
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncSeek, AsyncWrite, ReadBuf};

mod apply;
mod database;
mod log_store;
mod snapshot;
mod state_machine;
mod storage;
mod types;

use apply::*;
use database::*;
pub use snapshot::*;
use state_machine::*;
use storage::*;
pub use types::*;

#[cfg(test)]
mod tests;
