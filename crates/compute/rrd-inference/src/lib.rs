//! Provider-neutral model admission and embedding jobs with exact provenance.
//!
//! Inference is never authoritative by itself. A prepared vector is accepted
//! only when the source bytes match the job's expected digest before and after
//! inference, the backend exactly matches the requested model contract, and
//! the returned shape/normalization validates as a canonical `RuntimeVector`.

use rrd_contract::{
    RouterArtifactDescriptor, RouterBackendDescriptor, RouterModelHandshake, RouterModelManifest,
};
use rrd_core::{
    digest, DataTransaction, EmbeddingProvenance, Error, Millis, ReadStamp, Result, RuntimeCommit,
    RuntimeId, RuntimeMutation, RuntimeProperties, RuntimeRef, RuntimeValue, RuntimeVector,
    ScopeId, VectorCollectionAddress, VectorNormalization, VectorValue,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

mod registry;
pub use registry::{EmbeddingBackendRegistry, EmbeddingDispatchError, EmbeddingExecution};
mod execution_control;
pub use execution_control::{EmbeddingCancellation, EmbeddingExecutionControl};

pub mod lfg_observation;

#[cfg(feature = "endpoint-chat")]
pub mod lfg_transport;

#[cfg(feature = "endpoint-chat")]
pub mod endpoint_chat;

#[cfg(feature = "fastembed-local")]
mod fastembed_local;

#[cfg(feature = "fastembed-local")]
pub use fastembed_local::{fastembed_model_digest, FastEmbedLocalBackend, FastEmbedLocalIdentity};

mod batch;
mod coordinator;
mod feature_hash;
mod model;
mod router;
mod validation;

pub use batch::*;
pub use coordinator::*;
pub use feature_hash::*;
pub use model::*;
pub use router::*;
use validation::*;

#[cfg(test)]
mod cooperative_feature_hash_tests {
    use super::*;

    #[test]
    fn cancellation_interrupts_real_feature_hash_before_output() {
        let bytes = (0..100)
            .map(|index| format!("word{index} "))
            .collect::<String>()
            .into_bytes();
        let request = EmbeddingRequest {
            job_id: RuntimeId::new("cancel-source").unwrap(),
            job_digest: digest::sha256_hex(b"cancel-job"),
            source_digest: digest::sha256_hex(&bytes),
            media_type: "text/plain".into(),
            bytes,
        };
        let mut backend = FeatureHashBackend::new(32, 99).unwrap();
        let cancellation = EmbeddingCancellation::default();
        let control = EmbeddingExecutionControl::new(
            std::time::Instant::now() + std::time::Duration::from_secs(5),
            cancellation.clone(),
        );
        let mut checkpoints = 0;
        let stopped = backend.embed_with_checkpoint(&request, || {
            checkpoints += 1;
            if checkpoints == 20 {
                cancellation.cancel();
            }
            control.check().map_err(|error| Error::InvalidRuntime {
                reason: error.to_string(),
            })
        });
        assert!(stopped.is_err());
        assert_eq!(
            checkpoints, 20,
            "real computation continued past cancellation"
        );
        assert!(
            backend.embed(&request).is_ok(),
            "cancellation corrupted the reusable model"
        );
    }
}
