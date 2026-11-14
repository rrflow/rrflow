//! Persistent identity, deny-by-default authorization, and audit for RRD.
//!
//! `RrdEngine` is the only authority boundary: it authenticates, authorizes,
//! stamps, and commits. This crate supplies principal, role, grant, and
//! credential metadata, pure validation and policy compilation, audit value
//! semantics, and cryptographic helpers. Its public storage repository is
//! implementation inventory, not the accepted authority boundary; POAM-016
//! records it. Connectome renders and administers policy only through
//! authorized APIs.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use hmac::{Hmac, KeyInit, Mac};
use rrd_contract::{CanonicalId, ResourceId, ResourceKind, ResourcePath};
use rrd_core::{digest, RuntimeValue};
use rrd_store::{ControlJournalEntry, ControlTransition, StorageEngine};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

mod audit;
mod authorization;
mod codec;
mod model;
mod repository;
mod validation;

pub use audit::*;
pub use authorization::*;
use codec::*;
pub use model::*;
pub use repository::*;
use validation::*;
