use super::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

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
    /// The result was not accepted. A dispatched blocking adapter may still run.
    DeadlineExceeded {
        execution_may_continue: bool,
    },
    Cancelled {
        execution_may_continue: bool,
    },
    WorkerUnavailable,
    Rejected(Error),
}

impl std::fmt::Display for EmbeddingDispatchError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Busy => formatter.write_str("embedding execution capacity is occupied"),
            Self::UnhealthyBackend => formatter.write_str("embedding backend lock is poisoned"),
            Self::DeadlineExceeded {
                execution_may_continue,
            } => write!(
                formatter,
                "embedding deadline exceeded; execution may continue: {execution_may_continue}"
            ),
            Self::WorkerUnavailable => formatter.write_str("embedding worker unavailable"),
            Self::Cancelled {
                execution_may_continue,
            } => write!(
                formatter,
                "embedding cancelled; execution may continue: {execution_may_continue}"
            ),
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
        self.embed_batch_inner(model, network_policy, requests, None)
    }

    /// Bound caller waiting and result acceptance by one absolute monotonic
    /// deadline. This does not forcibly stop synchronous adapter code. The
    /// owned worker retains the existing single admission until it returns;
    /// timeout never permits another worker to accumulate behind it.
    pub fn embed_batch_until(
        self,
        model: EmbeddingModelSpec,
        network_policy: NetworkPolicy,
        requests: Vec<EmbeddingRequest>,
        deadline: Instant,
    ) -> std::result::Result<EmbeddingBatchResult, EmbeddingDispatchError> {
        self.embed_batch_with_control(
            model,
            network_policy,
            requests,
            EmbeddingExecutionControl::new(deadline, EmbeddingCancellation::default()),
        )
    }

    pub fn embed_batch_with_control(
        self,
        model: EmbeddingModelSpec,
        network_policy: NetworkPolicy,
        requests: Vec<EmbeddingRequest>,
        control: EmbeddingExecutionControl,
    ) -> std::result::Result<EmbeddingBatchResult, EmbeddingDispatchError> {
        control.check()?;
        let waiting = EmbeddingExecutionControl::new(control.deadline(), control.cancellation());
        let (sender, receiver) = std::sync::mpsc::sync_channel(1);
        std::thread::Builder::new()
            .name("rrflow-inference".into())
            .spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    self.embed_batch_inner(&model, network_policy, &requests, Some(&control))
                }))
                .unwrap_or(Err(EmbeddingDispatchError::WorkerUnavailable));
                // Never block on delivery. A timed-out receiver discards late
                // output, while the lease has remained with actual execution.
                let _ = sender.send(result);
            })
            .map_err(|_| EmbeddingDispatchError::WorkerUnavailable)?;
        loop {
            waiting.check_outstanding(true)?;
            let remaining = waiting.deadline().saturating_duration_since(Instant::now());
            // Blocking waits, not spinning. The cancellation wait quantum is
            // finite; host scheduling can add latency. Deadline never resets.
            match receiver.recv_timeout(remaining.min(std::time::Duration::from_millis(10))) {
                Ok(result) => {
                    waiting.check_outstanding(true)?;
                    return result;
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                    return Err(EmbeddingDispatchError::WorkerUnavailable)
                }
            }
        }
    }

    fn embed_batch_inner(
        self,
        model: &EmbeddingModelSpec,
        network_policy: NetworkPolicy,
        requests: &[EmbeddingRequest],
        control: Option<&EmbeddingExecutionControl>,
    ) -> std::result::Result<EmbeddingBatchResult, EmbeddingDispatchError> {
        check_control(control)?;
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
        validate_embedding_batch(&self.descriptor, network_policy, requests)?;
        check_control(control)?;
        let result = match control {
            Some(control) => backend.embed_batch_controlled(requests, control),
            None => backend.embed_batch(requests).map_err(Into::into),
        };
        check_control(control)?;
        let vectors = result?;
        validate_embedding_batch_output(&self.descriptor, requests.len(), &vectors)?;
        self.require_descriptor(backend.as_ref())?;
        check_control(control)?;
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

fn check_control(
    control: Option<&EmbeddingExecutionControl>,
) -> std::result::Result<(), EmbeddingDispatchError> {
    control.map_or(Ok(()), EmbeddingExecutionControl::check)
}
