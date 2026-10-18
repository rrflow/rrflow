//! Canonical semantic repositories over the one physical storage transaction.
//!
//! Repositories own key selection, decoding, validation, and semantic mutation
//! planning. They borrow a [`StorageEngine`](crate::StorageEngine), but the
//! engine exposes no semantic read or write methods of its own. rrflowMX and
//! rrflowKV therefore execute the same repository code and differ only in
//! volatility and physical durability.

mod claims;
mod control;
mod function;
mod invocation;
mod projection;
mod runtime;

pub use claims::ClaimRepository;
pub(crate) use control::prepare_control_batch;
pub use control::ControlRepository;
pub use function::FunctionCatalogueRepository;
pub use invocation::InvocationRepository;
pub use projection::ProjectionRepository;
pub use runtime::{RuntimeDataSnapshotRead, RuntimeRepository};
