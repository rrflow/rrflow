//! C-07's early storage subset of the J-04 measurement contract.
//! Validation admits evidence structure; it cannot authenticate a measurement
//! or prove an engine enforces the manifest's proposed resource envelope.
mod acceptance;
pub use acceptance::*;
mod capacity;
pub mod cli;
mod drain;
mod fleet;
pub use fleet::*;
mod latency;
pub use latency::LatencyBounds;
mod ledger;
mod model;
mod policy;
mod progress;
mod reader_interference;
pub use reader_interference::ReaderInterferenceEvidence;
mod sustained;
mod validation;

pub use capacity::{
    capacity_profiles, CapacityProfile, CapacityProfiles, CapacityThresholds, LatencyThreshold,
    NominalRate,
};
pub use drain::{DrainOwnership, DrainPolicy, DrainSample};
pub use ledger::*;
pub use model::*;
pub use policy::*;
pub use progress::{ProgressEvent, ProgressReceipt};
pub use sustained::*;
pub use validation::*;

pub type Result<T> = std::result::Result<T, String>;

pub(crate) fn require(condition: bool, message: &str) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(message.to_owned())
    }
}
