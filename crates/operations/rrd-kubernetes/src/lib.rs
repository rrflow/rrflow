//! Kubernetes-native deployment contract for a secured RRD instance.
//!
//! The first version intentionally deploys one durable RRD process. The
//! separate `rrd-cluster` consensus engine is not yet wired into the public
//! RRD data plane, so this crate refuses to represent multiple independent RRD
//! pods as a distributed database.

use kube::{CustomResource, CustomResourceExt};
use rrd_core::digest;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;

pub mod controller;

pub const RRD_INSTANCE_API_VERSION: &str = "rrflow.io/v1alpha1";
pub const RRD_INSTANCE_KIND: &str = "RrdInstance";
pub const RRD_INSTANCE_FINALIZER: &str = "rrflow.io/rrd-instance-protection";
pub const RRD_OPERATOR_FIELD_MANAGER: &str = "rrd-kubernetes-operator";
pub const RRD_CLIENT_PORT: u16 = 9477;
pub const RRD_KUBERNETES_CONTRACT_VERSION: u16 = 1;

mod resources;
mod spec;
mod validation;

pub use resources::*;
pub use spec::*;
use validation::*;
