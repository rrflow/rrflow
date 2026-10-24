//! Admission for copied projected outputs, independent of source-view pins.

use crate::{Error, ProjectedReadResource, Result};
use arrow_buffer::{alloc::Allocation, Buffer, MutableBuffer};
use serde::{Deserialize, Serialize};
use std::ptr::NonNull;
use std::sync::{Arc, Mutex};

pub const DEFAULT_PROJECTED_OUTPUT_BYTES: usize = 128 * 1024 * 1024;

/// One coherent observation of this database-open instance's output pool.
/// Bytes include reserved/in-construction and retained Arrow allocations, not
/// allocator metadata, source pages, caller-created copies or process RSS.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectedOutputStats {
    pub limit_bytes: usize,
    pub reserved_bytes: usize,
    pub peak_reserved_bytes: usize,
    pub denied_batches: u64,
}

/// Read-only diagnostics that remain valid after the database closes. Cloning
/// this handle retains no buffers and does not reserve memory. A separately
/// reopened database has its own pool; this is not a process-global quota.
#[derive(Debug, Clone)]
pub struct ProjectedOutputMemory(Arc<Mutex<ProjectedOutputStats>>);

impl ProjectedOutputMemory {
    pub(crate) fn new(limit_bytes: usize) -> Self {
        Self(Arc::new(Mutex::new(ProjectedOutputStats {
            limit_bytes,
            ..ProjectedOutputStats::default()
        })))
    }

    pub fn stats(&self) -> ProjectedOutputStats {
        *self
            .0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub(crate) fn reserve(&self, bytes: usize) -> Result<OutputReservation> {
        let mut state = self
            .0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        // Subtraction admits even usize::MAX limits without addition overflow.
        if bytes > state.limit_bytes - state.reserved_bytes {
            state.denied_batches = state.denied_batches.saturating_add(1);
            return Err(Error::ProjectedReadLimit {
                resource: ProjectedReadResource::RetainedOutputBytes,
                limit: state.limit_bytes as u64,
                observed: state.reserved_bytes.saturating_add(bytes) as u64,
            });
        }
        state.reserved_bytes += bytes;
        state.peak_reserved_bytes = state.peak_reserved_bytes.max(state.reserved_bytes);
        Ok(OutputReservation {
            pool: self.clone(),
            bytes,
        })
    }
}

#[derive(Debug)]
pub(crate) struct OutputReservation {
    pool: ProjectedOutputMemory,
    bytes: usize,
}

impl OutputReservation {
    /// Transfer part of a preallocation reservation to an immutable allocation.
    /// Only exact-capacity buffers constructed by the reader may enter here.
    pub(crate) fn wrap(&mut self, buffer: MutableBuffer) -> Result<Buffer> {
        let bytes = buffer.capacity();
        if buffer.len() != bytes || bytes > self.bytes {
            return Err(Error::InvalidProjectedRead(
                "projected output allocation differs from its reservation".into(),
            ));
        }
        if bytes == 0 {
            return Ok(Buffer::default());
        }
        let buffer = Buffer::from(buffer);
        let pointer = NonNull::new(buffer.as_ptr().cast_mut()).ok_or_else(|| {
            Error::InvalidProjectedRead("projected output allocation has a null pointer".into())
        })?;
        self.bytes -= bytes;
        let owner: Arc<dyn Allocation> = Arc::new(ReservedAllocation {
            _buffer: buffer,
            _reservation: Self {
                pool: self.pool.clone(),
                bytes,
            },
        });
        // SAFETY: the owner retains the original immutable Buffer, whose
        // pointer is valid for exactly `bytes` initialized bytes. It is neither
        // mutated nor exposed separately. Arrow clones/slices retain this
        // owner; custom allocations cannot be converted to MutableBuffer or
        // reallocated in place. Field order frees the underlying allocation
        // before releasing its charge. The pointer has Arrow's original
        // alignment; this wrapper does not copy or change the allocation.
        Ok(unsafe { Buffer::from_custom_allocation(pointer, bytes, owner) })
    }
}

impl Drop for OutputReservation {
    fn drop(&mut self) {
        let mut state = self
            .pool
            .0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.reserved_bytes -= self.bytes;
    }
}

#[derive(Debug)]
struct ReservedAllocation {
    // Rust drops fields in declaration order: free bytes, then release charge.
    _buffer: Buffer,
    _reservation: OutputReservation,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reservation_overflow_partial_transfer_and_failed_wrap_release_exactly() {
        let pool = ProjectedOutputMemory::new(usize::MAX);
        let full = pool.reserve(usize::MAX).unwrap();
        assert!(matches!(
            pool.reserve(1),
            Err(Error::ProjectedReadLimit { .. })
        ));
        assert_eq!(pool.stats().reserved_bytes, usize::MAX);
        drop(full);
        assert_eq!(pool.stats().reserved_bytes, 0);

        let mut reservation = pool.reserve(8).unwrap();
        let buffer = reservation.wrap(MutableBuffer::from_len_zeroed(4)).unwrap();
        assert!(reservation.wrap(MutableBuffer::from_len_zeroed(5)).is_err());
        assert_eq!(pool.stats().reserved_bytes, 8);
        drop(reservation);
        assert_eq!(pool.stats().reserved_bytes, 4);
        let slice = buffer.slice_with_length(1, 1);
        drop(buffer);
        assert_eq!(pool.stats().reserved_bytes, 4);
        assert_eq!(slice.as_slice(), &[0]);
        assert!(slice.into_mutable().is_err());
        assert_eq!(pool.stats().reserved_bytes, 0);
    }
}
