//! Ownership-based admission for active and retained mutable generations.
//! Reservations cover logical key/value/version charge, not heap overhead/RSS.
use crate::mutable::MutableCharge;
use crate::{Error, Memtable, MutableResource, Result, WriteBatch};
use serde::{Deserialize, Serialize};
use std::ops::Deref;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutableRetentionLimits {
    pub accounted_bytes: usize,
    pub versions: usize,
    pub generations: usize,
}

impl Default for MutableRetentionLimits {
    fn default() -> Self {
        Self {
            accounted_bytes: 512 * 1024 * 1024,
            versions: 4_194_304,
            generations: 64,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutableMemoryStats {
    pub accounted_bytes: usize,
    pub versions: usize,
    pub generations: usize,
    pub peak_accounted_bytes: usize,
    pub peak_generations: usize,
    pub denied_writes: u64,
}

#[derive(Debug)]
struct Pool {
    limits: MutableRetentionLimits,
    stats: Mutex<MutableMemoryStats>,
}

/// One open-instance pool. The handle and reservations survive database close;
/// a separately reopened database has its own pool, not a global process quota.
#[derive(Debug, Clone)]
pub struct MutableMemory(Arc<Pool>);

impl MutableMemory {
    pub(crate) fn new(limits: MutableRetentionLimits) -> Self {
        Self(Arc::new(Pool {
            limits,
            stats: Mutex::new(MutableMemoryStats::default()),
        }))
    }

    pub fn stats(&self) -> MutableMemoryStats {
        *self
            .0
            .stats
            .lock()
            .unwrap_or_else(|error| error.into_inner())
    }

    pub fn limits(&self) -> MutableRetentionLimits {
        self.0.limits
    }

    fn reserve(&self, charge: MutableCharge, generations: usize) -> Result<Reservation> {
        let mut state = self
            .0
            .stats
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        for (resource, used, added, limit) in [
            (
                MutableResource::RetainedAccountedBytes,
                state.accounted_bytes,
                charge.bytes,
                self.0.limits.accounted_bytes,
            ),
            (
                MutableResource::RetainedVersions,
                state.versions,
                charge.versions,
                self.0.limits.versions,
            ),
            (
                MutableResource::Generations,
                state.generations,
                generations,
                self.0.limits.generations,
            ),
        ] {
            if added > limit.saturating_sub(used) {
                state.denied_writes = state.denied_writes.saturating_add(1);
                return Err(Error::MutableLimit {
                    resource,
                    limit,
                    observed: used.saturating_add(added),
                });
            }
        }
        state.accounted_bytes += charge.bytes;
        state.versions += charge.versions;
        state.generations += generations;
        state.peak_accounted_bytes = state.peak_accounted_bytes.max(state.accounted_bytes);
        state.peak_generations = state.peak_generations.max(state.generations);
        Ok(Reservation {
            pool: self.clone(),
            charge,
            generations,
        })
    }

    pub(crate) fn wrap(&self, table: Memtable) -> Result<MutableGeneration> {
        let reservation = self.reserve(table.charge(), usize::from(table.version_count() != 0))?;
        Ok(MutableGeneration { table, reservation })
    }

    pub(crate) fn empty(&self, sequence: u64) -> Arc<MutableGeneration> {
        Arc::new(MutableGeneration {
            table: Memtable::at_sequence(sequence),
            reservation: Reservation {
                pool: self.clone(),
                charge: MutableCharge::default(),
                generations: 0,
            },
        })
    }

    /// Reserve every prospective copy and incoming version before WAL effects.
    pub(crate) fn prepare(
        &self,
        current: &Arc<MutableGeneration>,
        incoming: MutableCharge,
    ) -> Result<PreparedMutable> {
        if Arc::strong_count(current) > 1 || Arc::weak_count(current) > 0 {
            let reservation = self.reserve(current.charge().checked_add(incoming)?, 1)?;
            Ok(PreparedMutable::Replacement(MutableGeneration {
                table: current.table.clone(),
                reservation,
            }))
        } else {
            Ok(PreparedMutable::Unique(self.reserve(
                incoming,
                usize::from(current.version_count() == 0),
            )?))
        }
    }
}

#[derive(Debug)]
pub(crate) struct Reservation {
    pool: MutableMemory,
    charge: MutableCharge,
    generations: usize,
}

impl Reservation {
    fn absorb(&mut self, mut delta: Self) {
        debug_assert!(Arc::ptr_eq(&self.pool.0, &delta.pool.0));
        // Both charges were admitted together by the shared pool before effects.
        self.charge.bytes += delta.charge.bytes;
        self.charge.versions += delta.charge.versions;
        self.generations += delta.generations;
        delta.charge = MutableCharge::default();
        delta.generations = 0;
    }
}

impl Drop for Reservation {
    fn drop(&mut self) {
        let mut state = self
            .pool
            .0
            .stats
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        state.accounted_bytes -= self.charge.bytes;
        state.versions -= self.charge.versions;
        state.generations -= self.generations;
    }
}

#[derive(Debug)]
pub(crate) struct MutableGeneration {
    // Drop payloads before releasing admission, including after database close.
    table: Memtable,
    reservation: Reservation,
}

impl Deref for MutableGeneration {
    type Target = Memtable;
    fn deref(&self) -> &Self::Target {
        &self.table
    }
}

pub(crate) enum PreparedMutable {
    Unique(Reservation),
    Replacement(MutableGeneration),
}

impl PreparedMutable {
    pub(crate) fn apply(
        self,
        current: &mut Arc<MutableGeneration>,
        batch: &WriteBatch,
        first: u64,
        last: u64,
    ) -> Result<()> {
        self.apply_with(current, |table| table.apply_write_batch(batch, first, last))
    }

    pub(crate) fn apply_owned(
        self,
        current: &mut Arc<MutableGeneration>,
        batch: WriteBatch,
        first: u64,
        last: u64,
    ) -> Result<()> {
        self.apply_with(current, |table| {
            table.apply_owned_write_batch(batch, first, last)
        })
    }

    fn apply_with(
        self,
        current: &mut Arc<MutableGeneration>,
        apply: impl FnOnce(&mut Memtable) -> Result<()>,
    ) -> Result<()> {
        match self {
            Self::Replacement(mut generation) => {
                apply(&mut generation.table)?;
                *current = Arc::new(generation);
            }
            Self::Unique(delta) => {
                // The caller retains exclusive database access from preparation
                // through publication; no read can acquire another source owner.
                let generation = Arc::get_mut(current).ok_or_else(|| {
                    Error::InvalidBatch("mutable ownership changed after admission".into())
                })?;
                apply(&mut generation.table)?;
                generation.reservation.absorb(delta);
            }
        }
        Ok(())
    }
}
