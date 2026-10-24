//! Finite admission at the existing in-process transaction serialization owner.
use super::{Result, RrdEngine, ServiceError};
use serde::Serialize;
use std::collections::VecDeque;
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

#[derive(Debug)]
pub(super) struct InvocationAdmission {
    cancelled: AtomicBool,
    started: Instant,
    deadline: Option<Duration>,
}

// Authorization equality is reservation identity, never mutable cancellation state.
impl PartialEq for InvocationAdmission {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl Eq for InvocationAdmission {}

impl InvocationAdmission {
    pub(super) fn new(started: Instant, deadline_ms: Option<u64>) -> Self {
        Self {
            cancelled: AtomicBool::new(false),
            started,
            deadline: deadline_ms.map(Duration::from_millis),
        }
    }
    pub(super) fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }
    fn check(&self) -> Result<()> {
        if self.cancelled.load(Ordering::Acquire) {
            return Err(ServiceError::AdmissionCancelled);
        }
        if self
            .deadline
            .is_some_and(|limit| self.started.elapsed() >= limit)
        {
            return Err(ServiceError::DeadlineExceeded);
        }
        Ok(())
    }
}

#[derive(Debug, Default)]
struct State {
    active: bool,
    fenced: bool,
    outstanding: u64,
    bytes: u64,
    next_ticket: u64,
    peak_requests: u64,
    peak_bytes: u64,
    queue: VecDeque<u64>,
}

#[derive(Debug, Default)]
pub(crate) struct TransactionGate {
    state: Mutex<State>,
    changed: Condvar,
}

pub(crate) struct TransactionPermit<'a> {
    gate: &'a TransactionGate,
    ticket: u64,
    bytes: u64,
    active: bool,
}

impl Drop for TransactionPermit<'_> {
    fn drop(&mut self) {
        // Release actual ownership even during unwinding. Future admissions
        // still reject a poisoned gate rather than performing new effects.
        let mut state = self
            .gate
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if self.active {
            // The previous Mutex guard poisoned serialization if effect code
            // panicked. Preserve that fail-closed behavior with an owned permit.
            state.fenced |= std::thread::panicking();
            state.active = false;
        } else {
            state.queue.retain(|ticket| *ticket != self.ticket);
        }
        state.outstanding -= 1;
        state.bytes -= self.bytes;
        self.gate.changed.notify_all();
    }
}

impl TransactionGate {
    fn acquire<T: Serialize + ?Sized>(
        &self,
        policy: &rrd_contract::StorageLimits,
        request: &T,
        control: Option<&InvocationAdmission>,
    ) -> Result<TransactionPermit<'_>> {
        let started = Instant::now();
        if let Some(control) = control {
            control.check()?;
        }
        let bytes = request_charge(request, policy.transaction_admission_bytes)?;
        let mut state = self.state.lock().map_err(|_| poisoned())?;
        if state.fenced {
            return Err(poisoned());
        }
        if state.outstanding >= policy.transaction_admission_requests
            || bytes
                > policy
                    .transaction_admission_bytes
                    .saturating_sub(state.bytes)
        {
            return Err(ServiceError::StorageAdmission(
                "transaction admission count or logical-byte limit".into(),
            ));
        }
        let ticket = state.next_ticket;
        state.next_ticket = ticket.checked_add(1).ok_or_else(|| {
            ServiceError::StorageAdmission("transaction admission ticket space exhausted".into())
        })?;
        state.outstanding += 1;
        state.bytes += bytes;
        state.peak_requests = state.peak_requests.max(state.outstanding);
        state.peak_bytes = state.peak_bytes.max(state.bytes);
        state.queue.push_back(ticket);
        // Drop the state lock before constructing the guard, so every error
        // path can release its reservation without recursively locking state.
        drop(state);
        let mut permit = TransactionPermit {
            gate: self,
            ticket,
            bytes,
            active: false,
        };
        let wait_limit = Duration::from_millis(policy.transaction_admission_wait_ms);
        loop {
            if let Some(control) = control {
                control.check()?;
            }
            let elapsed = started.elapsed();
            if elapsed >= wait_limit {
                return Err(ServiceError::DeadlineExceeded);
            }
            let mut state = self.state.lock().map_err(|_| poisoned())?;
            if state.fenced {
                drop(state);
                return Err(poisoned());
            }
            if !state.active && state.queue.front() == Some(&ticket) {
                // Check again after locking, before granting durable-effect access.
                let checked = control.map_or(Ok(()), InvocationAdmission::check);
                if let Err(error) = checked {
                    drop(state);
                    return Err(error);
                }
                if started.elapsed() >= wait_limit {
                    drop(state);
                    return Err(ServiceError::DeadlineExceeded);
                }
                state.queue.pop_front();
                state.active = true;
                permit.active = true;
                drop(state);
                return Ok(permit);
            }
            // Periodic wake bounds cancellation latency without transferring
            // mutation authority to the cancelling caller.
            let wait = wait_limit
                .saturating_sub(elapsed)
                .min(Duration::from_millis(10));
            let waited = self
                .changed
                .wait_timeout(state, wait)
                .map_err(|_| poisoned())?;
            drop(waited);
        }
    }
}

fn poisoned() -> ServiceError {
    ServiceError::Storage("engine transaction gate is poisoned".into())
}

fn request_charge<T: Serialize + ?Sized>(request: &T, limit: u64) -> Result<u64> {
    struct Counter {
        bytes: u64,
        limit: u64,
        exceeded: bool,
    }
    impl Write for Counter {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            if bytes.len() as u64 > self.limit.saturating_sub(self.bytes) {
                self.exceeded = true;
                return Err(io::Error::other("transaction admission byte limit"));
            }
            self.bytes += bytes.len() as u64;
            Ok(bytes.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let mut counter = Counter {
        bytes: 0,
        limit,
        exceeded: false,
    };
    if let Err(error) = serde_json::to_writer(&mut counter, request) {
        return Err(if counter.exceeded {
            ServiceError::StorageAdmission(
                "transaction request exceeds logical-byte admission".into(),
            )
        } else {
            ServiceError::Contract(error.to_string())
        });
    }
    Ok(counter.bytes)
}

/// Content-free per-engine admission observation. Bytes are serialized logical
/// request charge, not heap or RSS; the active request remains charged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct TransactionAdmissionSnapshot {
    pub outstanding_requests: u64,
    pub logical_bytes: u64,
    pub queued_requests: usize,
    pub active: bool,
    pub fenced: bool,
    pub peak_requests: u64,
    pub peak_logical_bytes: u64,
}

impl RrdEngine {
    pub fn transaction_admission_snapshot(&self) -> Result<TransactionAdmissionSnapshot> {
        let state = self.transaction_gate.state.lock().map_err(|_| poisoned())?;
        Ok(TransactionAdmissionSnapshot {
            outstanding_requests: state.outstanding,
            logical_bytes: state.bytes,
            queued_requests: state.queue.len(),
            active: state.active,
            fenced: state.fenced,
            peak_requests: state.peak_requests,
            peak_logical_bytes: state.peak_bytes,
        })
    }

    pub(crate) fn admit_transaction<T: Serialize + ?Sized>(
        &self,
        request: &T,
        request_id: &str,
        operation_id: &str,
    ) -> Result<TransactionPermit<'_>> {
        let control: Option<Arc<InvocationAdmission>> =
            self.invocation_admission(request_id, operation_id)?;
        self.transaction_gate.acquire(
            &self.estate_configuration.storage,
            request,
            control.as_deref(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expired_invocation_and_panicked_writer_cannot_grant_new_effects() {
        let gate = TransactionGate::default();
        let policy = rrd_contract::StorageLimits::default();
        let expired = InvocationAdmission::new(Instant::now(), Some(0));
        assert!(matches!(
            gate.acquire(&policy, &"expired", Some(&expired)),
            Err(ServiceError::DeadlineExceeded)
        ));
        assert_eq!(gate.state.lock().unwrap().outstanding, 0);
        let result = std::panic::catch_unwind(|| {
            let _permit = gate.acquire(&policy, &"active", None).unwrap();
            panic!("simulate an effect-owner panic");
        });
        assert!(result.is_err());
        assert_eq!(gate.state.lock().unwrap().outstanding, 0);
        assert!(matches!(
            gate.acquire(&policy, &"after", None),
            Err(ServiceError::Storage(_))
        ));
    }

    #[test]
    fn count_bytes_deadline_and_cancellation_release_the_same_gate() {
        let gate = TransactionGate::default();
        let mut policy = rrd_contract::StorageLimits {
            transaction_admission_requests: 1,
            transaction_admission_bytes: 8,
            ..rrd_contract::StorageLimits::default()
        };
        let first = gate.acquire(&policy, &"ab", None).unwrap();
        assert!(matches!(
            gate.acquire(&policy, &"c", None),
            Err(ServiceError::StorageAdmission(_))
        ));
        policy.transaction_admission_requests = 2;
        assert!(matches!(
            gate.acquire(&policy, &"long", None),
            Err(ServiceError::StorageAdmission(_))
        ));
        policy.transaction_admission_wait_ms = 1;
        assert!(matches!(
            gate.acquire(&policy, &"c", None),
            Err(ServiceError::DeadlineExceeded)
        ));
        assert_eq!(gate.state.lock().unwrap().outstanding, 1);
        assert_eq!(gate.state.lock().unwrap().bytes, 4);
        let control = InvocationAdmission::new(Instant::now(), None);
        control.cancel();
        assert!(matches!(
            gate.acquire(&policy, &"c", Some(&control)),
            Err(ServiceError::AdmissionCancelled)
        ));
        drop(first);
        assert_eq!(gate.state.lock().unwrap().outstanding, 0);
        assert_eq!(gate.state.lock().unwrap().bytes, 0);
        policy.transaction_admission_wait_ms = 30_000;
        assert!(gate.acquire(&policy, &"next", None).is_ok());
    }

    #[test]
    fn queued_cancellation_releases_reservation_without_releasing_active_writer() {
        let gate = TransactionGate::default();
        let policy = rrd_contract::StorageLimits::default();
        let control = InvocationAdmission::new(Instant::now(), None);
        let first = gate.acquire(&policy, &"first", None).unwrap();
        std::thread::scope(|scope| {
            let waiter = scope.spawn(|| gate.acquire(&policy, &"queued", Some(&control)).map(drop));
            let deadline = Instant::now();
            while gate.state.lock().unwrap().outstanding != 2 {
                assert!(
                    deadline.elapsed() < Duration::from_secs(10),
                    "waiter never admitted"
                );
                std::thread::yield_now();
            }
            control.cancel();
            assert!(matches!(
                waiter.join().unwrap(),
                Err(ServiceError::AdmissionCancelled)
            ));
        });
        assert_eq!(gate.state.lock().unwrap().outstanding, 1);
        assert!(gate.state.lock().unwrap().active);
        drop(first);
        assert!(gate.acquire(&policy, &"after", None).is_ok());
    }
}
