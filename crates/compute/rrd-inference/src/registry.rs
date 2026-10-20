use super::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// Process-owned executable adapters and their immutable admitted descriptors.
/// This is not a durable scheduler. Selection never executes an adapter.
#[derive(Default)]
pub struct EmbeddingBackendRegistry {
    revision: u64,
    backends: BTreeMap<String, RegisteredBackend>,
    // Preserve the previous single executing batch per registry. There is no
    // waiting queue and no concurrency increase before G-01a qualification.
    occupied: Arc<AtomicBool>,
}

struct RegisteredBackend {
    descriptor: EmbeddingBackendDescriptor,
    backend: Arc<Mutex<Box<dyn EmbeddingBackend + Send>>>,
}

#[derive(Debug)]
pub enum EmbeddingDispatchError {
    Busy,
    UnhealthyBackend,
    Rejected(Error),
}

impl std::fmt::Display for EmbeddingDispatchError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Busy => formatter.write_str("embedding execution capacity is occupied"),
            Self::UnhealthyBackend => formatter.write_str("embedding backend lock is poisoned"),
            Self::Rejected(error) => std::fmt::Display::fmt(error, formatter),
        }
    }
}

impl std::error::Error for EmbeddingDispatchError {}

impl From<Error> for EmbeddingDispatchError {
    fn from(error: Error) -> Self {
        Self::Rejected(error)
    }
}

impl EmbeddingBackendRegistry {
    pub fn install(&mut self, backend: Box<dyn EmbeddingBackend + Send>) -> Result<u64> {
        let descriptor = backend.descriptor().clone();
        descriptor.validate()?;
        if self.backends.contains_key(&descriptor.id) {
            return invalid(format!(
                "embedding backend {} is already installed",
                descriptor.id
            ));
        }
        self.revision = self
            .revision
            .checked_add(1)
            .ok_or_else(|| Error::InvalidRuntime {
                reason: "embedding registry revision overflowed u64".into(),
            })?;
        self.backends.insert(
            descriptor.id.clone(),
            RegisteredBackend {
                descriptor,
                backend: Arc::new(Mutex::new(backend)),
            },
        );
        Ok(self.revision)
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn descriptors(&self) -> Vec<EmbeddingBackendDescriptor> {
        self.backends
            .values()
            .map(|entry| entry.descriptor.clone())
            .collect()
    }

    pub fn descriptor(&self, backend_id: &str) -> Option<&EmbeddingBackendDescriptor> {
        self.backends.get(backend_id).map(|entry| &entry.descriptor)
    }

    /// Reserves the existing single slot without waiting. Drop the registry
    /// lock before preparing inputs or executing the returned lease. There is
    /// no queue, backend call, or input copy on a busy response.
    pub fn try_acquire(
        &self,
        backend_id: &str,
    ) -> std::result::Result<EmbeddingExecution, EmbeddingDispatchError> {
        let entry = self
            .backends
            .get(backend_id)
            .ok_or_else(|| Error::InvalidRuntime {
                reason: format!("embedding backend {backend_id} is not installed"),
            })?;
        self.occupied
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .map_err(|_| EmbeddingDispatchError::Busy)?;
        // Own the permit before any subsequent fallible work. It is deliberately
        // not Clone: only this lease can release this admission.
        let permit = ExecutionPermit(Arc::clone(&self.occupied));
        Ok(EmbeddingExecution {
            registry_revision: self.revision,
            descriptor: entry.descriptor.clone(),
            backend: Arc::clone(&entry.backend),
            _permit: permit,
        })
    }
}

struct ExecutionPermit(Arc<AtomicBool>);

impl Drop for ExecutionPermit {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

/// One process-local, single-use admission with pinned catalogue provenance.
/// Dropping a lease releases capacity; this does not cancel running backend
/// code, and a caller must retain the lease until execution actually returns.
pub struct EmbeddingExecution {
    registry_revision: u64,
    descriptor: EmbeddingBackendDescriptor,
    backend: Arc<Mutex<Box<dyn EmbeddingBackend + Send>>>,
    _permit: ExecutionPermit,
}

impl EmbeddingExecution {
    pub fn descriptor(&self) -> &EmbeddingBackendDescriptor {
        &self.descriptor
    }

    pub fn embed_batch(
        self,
        model: &EmbeddingModelSpec,
        network_policy: NetworkPolicy,
        requests: &[EmbeddingRequest],
    ) -> std::result::Result<EmbeddingBatchResult, EmbeddingDispatchError> {
        if &self.descriptor.model != model {
            return Err(Error::InvalidRuntime {
                reason: "embedding backend model differs from the requested model".into(),
            }
            .into());
        }
        // The single permit excludes all other execution leases. This lock
        // protects adapter ownership and confines panic poisoning to that adapter.
        let mut backend = self
            .backend
            .lock()
            .map_err(|_| EmbeddingDispatchError::UnhealthyBackend)?;
        self.require_descriptor(backend.as_ref())?;
        let vectors = dispatch_embedding_batch(backend.as_mut(), network_policy, requests)?;
        self.require_descriptor(backend.as_ref())?;
        Ok(EmbeddingBatchResult {
            registry_revision: self.registry_revision,
            backend: self.descriptor.clone(),
            vectors,
        })
    }

    fn require_descriptor(&self, backend: &dyn EmbeddingBackend) -> Result<()> {
        if backend.descriptor() != &self.descriptor {
            return invalid("embedding backend descriptor changed after installation");
        }
        Ok(())
    }
}
