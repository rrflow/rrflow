//! Deterministic TurboQuant codec for dense vector artifacts
//! (arXiv:2504.19874).
//!
//! Both estimators rotate with a seeded randomized orthogonal Hadamard
//! transform and normalize each vector's length onto fixed standard normal
//! Lloyd-Max codebooks, and both score queries asymmetrically at full
//! precision.
//!
//! - **MSE** (TurboQuant_mse) packs one codebook index per coordinate and
//!   rescales the reconstruction to the vector's length, a per-vector
//!   centroid-norm correction.
//! - **Inner product** (TurboQuant_prod, Algorithm 2) packs indices from a
//!   codebook one bit narrower, followed by the signs of the residual's QJL
//!   projection (`qjl`), and stores the residual's length. The paper's
//!   unbiasedness result assumes an independent Gaussian projection; this
//!   seeded finite-precision adaptation is evaluated by fixture, not proved
//!   unbiased by those observations.

use crate::contract::invalid;
use crate::qjl::QjlProjection;
use crate::{QuantizedKernel, ScoreMetric};
use rrd_core::Result;
use serde::{Deserialize, Serialize};

mod codes;
mod math;
mod query;

pub use codes::*;
use math::*;
pub use query::*;

#[cfg(test)]
mod tests;
