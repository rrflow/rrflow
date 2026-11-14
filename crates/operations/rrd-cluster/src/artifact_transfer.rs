//! Backend-neutral immutable object hydration for a grounded replica transfer.

use crate::{
    ArtifactObjectProgress, ArtifactReplicaObjectReceipt, ArtifactTransferManifest,
    ArtifactTransferOperation, ArtifactTransferReceipt, ArtifactTransferRpc,
    ArtifactTransferRpcResult, ClusterError, NodeId, Result,
};
use rrd_core::{RuntimeMutation, ScopeId};
use rrd_store::{
    publish_durable_rename, sync_directory_metadata, Error as StoreError, ImmutableObjectStore,
    LocalObjectStore, StorageEngine,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock, Weak};
use std::time::{SystemTime, UNIX_EPOCH};

mod receiver;
mod session;
mod support;

pub use receiver::*;
pub use session::*;
use support::*;
