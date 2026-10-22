//! C-07's early storage subset of the J-04 measurement contract.
//! Validation admits evidence structure; it cannot authenticate a measurement
//! or prove an engine enforces the manifest's proposed resource envelope.
pub mod cli;
mod ledger;
mod model;
mod policy;
mod validation;

pub use ledger::*;
pub use model::*;
pub use policy::*;
pub use validation::*;

pub type Result<T> = std::result::Result<T, String>;

pub(crate) fn require(condition: bool, message: &str) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(message.to_owned())
    }
}
