//! A single bounded HTTP exchange. No storage, policy authority, retry or task execution.
use rrd_contract::{
    EndpointBinding, EndpointLocator, ObservationDeliveryDisposition, ObservationExportPolicy,
    MAX_OBSERVATION_EVENT_BYTES,
};
use rrd_core::digest::sha256_hex;
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::net::IpAddr;
use std::time::{Duration, Instant};

mod export;
mod references;

pub use export::*;
pub use references::*;
