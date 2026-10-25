//! Bounded immutable-file ownership. Read-view pins, not this cache, govern GC.
use crate::filesystem::{AnchoredPath, FileIdentity};
use crate::{Error, Result};
use std::collections::BTreeMap;
use std::fs::File;
use std::marker::PhantomData;
use std::ops::Deref;
use std::sync::{Arc, Mutex};

#[derive(Debug, Default, Clone, Copy)]
pub(super) struct HandleStats {
    pub open: usize,
    pub peak: usize,
    pub evictions: u64,
    pub reopens: u64,
    pub denials: u64,
}

struct CachedFile {
    file: Arc<File>,
    touched: u64,
}

#[derive(Default)]
struct State {
    files: BTreeMap<u64, CachedFile>,
    next_id: u64,
    clock: u64,
    stats: HandleStats,
}

pub(crate) struct HandlePool {
    limit: usize,
    state: Mutex<State>,
}

/// A source identity retains the anchored root, but no per-segment descriptor.
/// Standalone segments have their own context, so their sole file stays pinned.
pub(crate) struct SegmentFile {
    pool: Arc<HandlePool>,
    id: u64,
    path: AnchoredPath,
    identity: FileIdentity,
}

impl std::fmt::Debug for SegmentFile {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SegmentFile")
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}

/// The borrow prevents source destruction while a native read holds its file.
pub(crate) struct FileLease<'a> {
    file: Arc<File>,
    _source: PhantomData<&'a SegmentFile>,
}

impl Deref for FileLease<'_> {
    type Target = File;
    fn deref(&self) -> &File {
        &self.file
    }
}

impl HandlePool {
    pub(super) fn new(limit: usize) -> Arc<Self> {
        Arc::new(Self {
            limit,
            state: Mutex::new(State::default()),
        })
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|error| error.into_inner())
    }

    fn admit(&self, state: &mut State) -> Result<()> {
        if state.files.len() < self.limit {
            return Ok(());
        }
        let idle = state
            .files
            .iter()
            .filter(|(_, entry)| Arc::strong_count(&entry.file) == 1)
            .min_by_key(|(_, entry)| entry.touched)
            .map(|(id, _)| *id);
        if let Some(id) = idle {
            state.files.remove(&id);
            state.stats.evictions = state.stats.evictions.saturating_add(1);
            Ok(())
        } else {
            state.stats.denials = state.stats.denials.saturating_add(1);
            Err(Error::SegmentHandleLimit {
                limit: self.limit,
                observed: self.limit + 1,
            })
        }
    }

    // Admission, native open and insertion share the lock: concurrent opens
    // cannot overshoot the bound, even during initial metadata validation.
    pub(crate) fn open(self: &Arc<Self>, path: &AnchoredPath) -> Result<SegmentFile> {
        let mut state = self.lock();
        let id = state
            .next_id
            .checked_add(1)
            .ok_or_else(|| Error::InvalidSegment("segment handle identity exhausted".into()))?;
        self.admit(&mut state)?;
        let file = path.open_read()?;
        let identity = FileIdentity::capture(&file)?;
        state.next_id = id;
        Self::insert(&mut state, id, Arc::new(file));
        Ok(SegmentFile {
            pool: self.clone(),
            id,
            path: path.clone(),
            identity,
        })
    }

    fn insert(state: &mut State, id: u64, file: Arc<File>) {
        state.clock = state.clock.saturating_add(1);
        state.files.insert(
            id,
            CachedFile {
                file,
                touched: state.clock,
            },
        );
        state.stats.peak = state.stats.peak.max(state.files.len());
    }

    pub(super) fn stats(&self) -> HandleStats {
        let state = self.lock();
        HandleStats {
            open: state.files.len(),
            ..state.stats
        }
    }
}

impl SegmentFile {
    pub(crate) fn lease(&self) -> Result<FileLease<'_>> {
        let mut state = self.pool.lock();
        if !state.files.contains_key(&self.id) {
            self.pool.admit(&mut state)?;
            let file = self.path.open_read()?;
            if FileIdentity::capture(&file)? != self.identity {
                return Err(Error::InvalidSegment(
                    "evicted segment source identity changed".into(),
                ));
            }
            HandlePool::insert(&mut state, self.id, Arc::new(file));
            state.stats.reopens = state.stats.reopens.saturating_add(1);
        }
        state.clock = state.clock.saturating_add(1);
        let touched = state.clock;
        let entry = state
            .files
            .get_mut(&self.id)
            .expect("admitted segment handle");
        entry.touched = touched;
        Ok(FileLease {
            file: entry.file.clone(),
            _source: PhantomData,
        })
    }
}

impl Drop for SegmentFile {
    fn drop(&mut self) {
        // A FileLease borrows this source, so all native reads have finished.
        self.pool.lock().files.remove(&self.id);
    }
}

#[cfg(test)]
mod tests;
