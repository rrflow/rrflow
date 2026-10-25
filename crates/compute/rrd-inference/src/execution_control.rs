use crate::EmbeddingDispatchError;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// One-way, process-local cancellation for caller-owned work. Clones share
/// the stop signal; there is intentionally no reset operation.
#[derive(Clone, Default)]
pub struct EmbeddingCancellation(Arc<AtomicBool>);

impl EmbeddingCancellation {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

/// Immutable bounds supplied to a cooperative adapter, not durable run state
/// or authorization for an effect or retry.
pub struct EmbeddingExecutionControl {
    deadline: Instant,
    cancellation: EmbeddingCancellation,
}

impl EmbeddingExecutionControl {
    pub fn new(deadline: Instant, cancellation: EmbeddingCancellation) -> Self {
        Self {
            deadline,
            cancellation,
        }
    }
    pub fn deadline(&self) -> Instant {
        self.deadline
    }
    /// Derive a stricter ceiling while retaining the same sticky stop signal.
    pub fn with_deadline(&self, deadline: Instant) -> Self {
        Self::new(self.deadline.min(deadline), self.cancellation.clone())
    }
    pub(crate) fn cancellation(&self) -> EmbeddingCancellation {
        self.cancellation.clone()
    }
    /// An adapter returning at this checkpoint has stopped its synchronous
    /// call. A caller outside that adapter must report outstanding work.
    pub fn check(&self) -> Result<(), EmbeddingDispatchError> {
        self.check_outstanding(false)
    }
    pub(crate) fn check_outstanding(
        &self,
        execution_may_continue: bool,
    ) -> Result<(), EmbeddingDispatchError> {
        if self.cancellation.is_cancelled() {
            return Err(EmbeddingDispatchError::Cancelled {
                execution_may_continue,
            });
        }
        if Instant::now() >= self.deadline {
            return Err(EmbeddingDispatchError::DeadlineExceeded {
                execution_may_continue,
            });
        }
        Ok(())
    }
}
