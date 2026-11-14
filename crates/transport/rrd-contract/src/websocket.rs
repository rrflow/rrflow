//! Closed, transport-neutral application protocol carried by one RRD WebSocket.
//!
//! RFC 6455 framing remains a carriage concern: one RRFlow application message
//! is one encoded [`WebSocketFrame`], independent of fragmentation performed by
//! a WebSocket implementation. Durable subscription cursors and generations
//! remain engine semantics; connection sequence and correlation state remain
//! transport-local.

use super::*;
use std::collections::BTreeSet;

mod frames;
mod subscriptions;

pub use frames::*;
pub use subscriptions::*;
