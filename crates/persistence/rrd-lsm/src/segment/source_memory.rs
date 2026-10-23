//! Admission for live owned source-page allocations, including cache misses.
use crate::{Error, Result};
use arrow_buffer::{alloc::Allocation, Buffer, MutableBuffer};
use serde::{Deserialize, Serialize};
use std::{
    ptr::NonNull,
    sync::{Arc, Mutex},
};

pub const DEFAULT_SOURCE_PAGE_BYTES: usize = 256 * 1024 * 1024;

/// Per-open private page allocations, including construction and retained
/// buffers. Excludes borrowed mmap pages, allocator metadata and process RSS.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourcePageStats {
    pub limit_bytes: usize,
    pub reserved_bytes: usize,
    pub peak_reserved_bytes: usize,
    pub denied_allocations: u64,
}

/// Diagnostics survive database close without retaining any page allocation.
#[derive(Debug, Clone)]
pub struct SourcePageMemory(Arc<Mutex<SourcePageStats>>);
impl SourcePageMemory {
    pub(crate) fn new(limit_bytes: usize) -> Self {
        Self(Arc::new(Mutex::new(SourcePageStats {
            limit_bytes,
            ..Default::default()
        })))
    }
    pub fn stats(&self) -> SourcePageStats {
        *self.0.lock().unwrap_or_else(|e| e.into_inner())
    }
    pub(super) fn reserve(&self, bytes: usize) -> Result<SourceReservation> {
        let mut state = self.0.lock().unwrap_or_else(|e| e.into_inner());
        if bytes > state.limit_bytes - state.reserved_bytes {
            state.denied_allocations = state.denied_allocations.saturating_add(1);
            return Err(Error::SourcePageLimit {
                limit: state.limit_bytes,
                observed: state.reserved_bytes.saturating_add(bytes),
            });
        }
        state.reserved_bytes += bytes;
        state.peak_reserved_bytes = state.peak_reserved_bytes.max(state.reserved_bytes);
        Ok(SourceReservation {
            pool: self.clone(),
            bytes,
        })
    }
}

pub(super) struct SourceReservation {
    pool: SourcePageMemory,
    bytes: usize,
}
impl SourceReservation {
    pub(super) fn wrap(self, buffer: MutableBuffer) -> Result<Buffer> {
        if buffer.capacity() != self.bytes || buffer.len() != self.bytes {
            return Err(Error::InvalidSegment(
                "source page allocation differs from reservation".into(),
            ));
        }
        if self.bytes == 0 {
            return Ok(Buffer::default());
        }
        let len = buffer.len();
        let buffer = Buffer::from(buffer);
        let pointer = NonNull::new(buffer.as_ptr().cast_mut()).ok_or_else(|| {
            Error::InvalidSegment("source page allocation has a null pointer".into())
        })?;
        let owner: Arc<dyn Allocation> = Arc::new(ReservedPage {
            _buffer: buffer,
            _reservation: self,
        });
        // SAFETY: owner retains the original immutable, aligned allocation for
        // all initialized len bytes. Clones/slices retain the owner and cannot
        // recover mutable access. Field drop order frees memory before charge.
        Ok(unsafe { Buffer::from_custom_allocation(pointer, len, owner) })
    }
}
impl Drop for SourceReservation {
    fn drop(&mut self) {
        self.pool
            .0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .reserved_bytes -= self.bytes;
    }
}
struct ReservedPage {
    _buffer: Buffer,
    _reservation: SourceReservation,
}
