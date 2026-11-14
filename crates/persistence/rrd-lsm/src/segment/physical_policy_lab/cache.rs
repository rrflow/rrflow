//! Exact and segmented LRU caches.

use super::*;

impl ExactLru {
    pub(super) fn new(capacity: usize) -> Self {
        Self {
            capacity,
            resident: 0,
            peak: 0,
            clock: 0,
            entries: BTreeMap::new(),
            observation: CachePolicyObservation {
                policy: "exact-byte-lru".into(),
                capacity_bytes: capacity as u64,
                peak_resident_bytes: 0,
                final_resident_bytes: 0,
                hits: 0,
                misses: 0,
                admissions: 0,
                evictions: 0,
                post_scan_hot_hits: 0,
                post_scan_hot_requests: 0,
                semantic_identity_exact: true,
                exact_capacity_respected: true,
            },
        }
    }

    pub(super) fn access(&mut self, access: CacheAccess) -> Result<()> {
        self.clock = self.clock.saturating_add(1);
        if access.post_scan_hot {
            self.observation.post_scan_hot_requests += 1;
        }
        if let Some(entry) = self.entries.get_mut(&access.page_id) {
            if entry.bytes != access.bytes {
                return Err(Error::InvalidSegment(
                    "cache trace changed one page's physical identity".into(),
                ));
            }
            entry.stamp = self.clock;
            self.observation.hits += 1;
            if access.post_scan_hot {
                self.observation.post_scan_hot_hits += 1;
            }
            return Ok(());
        }
        self.observation.misses += 1;
        if access.bytes > self.capacity {
            return Ok(());
        }
        while self.resident.saturating_add(access.bytes) > self.capacity {
            let (&victim, entry) = self
                .entries
                .iter()
                .min_by_key(|(_, entry)| entry.stamp)
                .ok_or_else(|| Error::InvalidSegment("LRU cannot select an eviction".into()))?;
            let bytes = entry.bytes;
            self.entries.remove(&victim);
            self.resident -= bytes;
            self.observation.evictions += 1;
        }
        self.entries.insert(
            access.page_id,
            CacheEntry {
                bytes: access.bytes,
                stamp: self.clock,
            },
        );
        self.resident += access.bytes;
        self.peak = self.peak.max(self.resident);
        self.observation.admissions += 1;
        if self.resident > self.capacity {
            return Err(Error::InvalidSegment("LRU exceeded exact capacity".into()));
        }
        Ok(())
    }

    pub(super) fn finish(mut self) -> CachePolicyObservation {
        self.observation.peak_resident_bytes = self.peak as u64;
        self.observation.final_resident_bytes = self.resident as u64;
        self.observation
    }
}

impl SegmentedLru {
    pub(super) fn new(capacity: usize) -> Self {
        let probation_capacity = (capacity / 5).max(1).min(capacity);
        let protected_capacity = capacity.saturating_sub(probation_capacity);
        Self {
            capacity,
            protected_capacity,
            probation_capacity,
            protected_resident: 0,
            probation_resident: 0,
            peak: 0,
            clock: 0,
            protected: BTreeMap::new(),
            probation: BTreeMap::new(),
            observation: CachePolicyObservation {
                policy: "exact-byte-segmented-lru-candidate".into(),
                capacity_bytes: capacity as u64,
                peak_resident_bytes: 0,
                final_resident_bytes: 0,
                hits: 0,
                misses: 0,
                admissions: 0,
                evictions: 0,
                post_scan_hot_hits: 0,
                post_scan_hot_requests: 0,
                semantic_identity_exact: true,
                exact_capacity_respected: true,
            },
        }
    }

    pub(super) fn access(&mut self, access: CacheAccess) -> Result<()> {
        self.clock = self.clock.saturating_add(1);
        if access.post_scan_hot {
            self.observation.post_scan_hot_requests += 1;
        }
        if let Some(entry) = self.protected.get_mut(&access.page_id) {
            validate_cache_identity(*entry, access)?;
            entry.stamp = self.clock;
            self.hit(access);
            return Ok(());
        }
        if let Some(mut entry) = self.probation.remove(&access.page_id) {
            validate_cache_identity(entry, access)?;
            self.probation_resident -= entry.bytes;
            entry.stamp = self.clock;
            self.protected_resident += entry.bytes;
            self.protected.insert(access.page_id, entry);
            self.hit(access);
            self.rebalance_protected()?;
            self.check_capacity()?;
            return Ok(());
        }
        self.observation.misses += 1;
        if access.bytes > self.probation_capacity {
            return Ok(());
        }
        while self.probation_resident.saturating_add(access.bytes) > self.probation_capacity {
            self.evict_probation()?;
        }
        self.probation.insert(
            access.page_id,
            CacheEntry {
                bytes: access.bytes,
                stamp: self.clock,
            },
        );
        self.probation_resident += access.bytes;
        self.observation.admissions += 1;
        self.update_peak();
        self.check_capacity()
    }

    fn hit(&mut self, access: CacheAccess) {
        self.observation.hits += 1;
        if access.post_scan_hot {
            self.observation.post_scan_hot_hits += 1;
        }
    }

    fn rebalance_protected(&mut self) -> Result<()> {
        while self.protected_resident > self.protected_capacity {
            let (&victim, entry) = self
                .protected
                .iter()
                .min_by_key(|(_, entry)| entry.stamp)
                .ok_or_else(|| {
                    Error::InvalidSegment("segmented LRU has no protected victim".into())
                })?;
            let entry = *entry;
            self.protected.remove(&victim);
            self.protected_resident -= entry.bytes;
            while self.probation_resident.saturating_add(entry.bytes) > self.probation_capacity {
                self.evict_probation()?;
            }
            if entry.bytes <= self.probation_capacity {
                self.probation.insert(victim, entry);
                self.probation_resident += entry.bytes;
            } else {
                self.observation.evictions += 1;
            }
        }
        self.update_peak();
        Ok(())
    }

    fn evict_probation(&mut self) -> Result<()> {
        let (&victim, entry) = self
            .probation
            .iter()
            .min_by_key(|(_, entry)| entry.stamp)
            .ok_or_else(|| Error::InvalidSegment("segmented LRU has no probation victim".into()))?;
        let bytes = entry.bytes;
        self.probation.remove(&victim);
        self.probation_resident -= bytes;
        self.observation.evictions += 1;
        Ok(())
    }

    fn update_peak(&mut self) {
        self.peak = self.peak.max(
            self.protected_resident
                .saturating_add(self.probation_resident),
        );
    }

    fn check_capacity(&self) -> Result<()> {
        let resident = self
            .protected_resident
            .saturating_add(self.probation_resident);
        if resident > self.capacity
            || self.protected_resident > self.protected_capacity
            || self.probation_resident > self.probation_capacity
        {
            return Err(Error::InvalidSegment(
                "segmented LRU exceeded exact byte partition".into(),
            ));
        }
        Ok(())
    }

    pub(super) fn finish(mut self) -> Result<CachePolicyObservation> {
        self.check_capacity()?;
        self.observation.peak_resident_bytes = self.peak as u64;
        self.observation.final_resident_bytes =
            self.protected_resident
                .saturating_add(self.probation_resident) as u64;
        Ok(self.observation)
    }
}

pub(super) fn validate_cache_identity(entry: CacheEntry, access: CacheAccess) -> Result<()> {
    if entry.bytes != access.bytes {
        return Err(Error::InvalidSegment(
            "cache trace changed one page's physical identity".into(),
        ));
    }
    Ok(())
}
