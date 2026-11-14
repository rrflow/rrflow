//! The segment page cache: policy, regions, admission, eviction and statistics.

use super::*;

/// Process-local immutable-page replacement policy. This policy is
/// configuration, not durable segment or manifest identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum PageCachePolicy {
    ExactLru,
    ScanResistantLru {
        protected_capacity_basis_points: u16,
    },
}

impl Default for PageCachePolicy {
    fn default() -> Self {
        Self::ScanResistantLru {
            protected_capacity_basis_points: DEFAULT_PAGE_CACHE_PROTECTED_CAPACITY_BASIS_POINTS,
        }
    }
}

impl PageCachePolicy {
    pub fn validate(self) -> Result<Self> {
        if let Self::ScanResistantLru {
            protected_capacity_basis_points,
        } = self
        {
            if !(1..10_000).contains(&protected_capacity_basis_points) {
                return Err(Error::InvalidConfiguration(
                    "scan-resistant page cache protected capacity must be between 1 and 9,999 basis points"
                        .into(),
                ));
            }
        }
        Ok(self)
    }

    pub fn kind(self) -> &'static str {
        match self {
            Self::ExactLru => "exact-lru",
            Self::ScanResistantLru { .. } => "scan-resistant-lru",
        }
    }

    pub fn protected_capacity_basis_points(self) -> u16 {
        match self {
            Self::ExactLru => 0,
            Self::ScanResistantLru {
                protected_capacity_basis_points,
            } => protected_capacity_basis_points,
        }
    }

    pub(crate) fn protected_capacity_bytes(self, capacity_bytes: usize) -> Result<usize> {
        self.validate()?;
        capacity_bytes
            .checked_mul(usize::from(self.protected_capacity_basis_points()))
            .map(|bytes| bytes / PAGE_CACHE_CAPACITY_BASIS_POINTS)
            .ok_or_else(|| {
                Error::InvalidConfiguration(
                    "page-cache protected-capacity calculation overflow".into(),
                )
            })
    }
}

/// Process-local immutable-page cache evidence. Counters distinguish physical
/// page reads from bytes borrowed, allocated, copied, or decompressed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PageCacheStats {
    pub policy: PageCachePolicy,
    pub capacity_bytes: usize,
    pub resident_bytes: usize,
    pub entries: usize,
    pub probationary_resident_bytes: usize,
    pub protected_resident_bytes: usize,
    pub probationary_entries: usize,
    pub protected_entries: usize,
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub admissions: u64,
    pub admission_rejections: u64,
    pub promotions: u64,
    pub demotions: u64,
    pub same_scope_hits: u64,
    pub duplicate_loads: u64,
    pub loads: u64,
    pub bytes_read: u64,
    pub bytes_decoded: u64,
    pub bytes_borrowed: u64,
    pub bytes_allocated: u64,
    pub bytes_copied: u64,
    pub bytes_decompressed: u64,
    pub filter_checks: u64,
    pub filter_negatives: u64,
}

#[derive(Debug)]
pub(crate) struct PageCache {
    pub(super) source_memory: SourcePageMemory,
    pub(super) policy: PageCachePolicy,
    pub(super) capacity_bytes: usize,
    pub(super) protected_capacity_bytes: usize,
    pub(super) resident_bytes: usize,
    pub(super) probationary_resident_bytes: usize,
    pub(super) protected_resident_bytes: usize,
    pub(super) values: HashMap<(u64, usize), CacheEntry>,
    pub(super) exact_order: BinaryHeap<Reverse<(u64, (u64, usize))>>,
    pub(super) probationary_order: BinaryHeap<Reverse<(u64, (u64, usize))>>,
    pub(super) protected_order: BinaryHeap<Reverse<(u64, (u64, usize))>>,
    pub(super) clock: u64,
    pub(super) hits: u64,
    pub(super) misses: u64,
    pub(super) evictions: u64,
    pub(super) admissions: u64,
    pub(super) admission_rejections: u64,
    pub(super) promotions: u64,
    pub(super) demotions: u64,
    pub(super) same_scope_hits: u64,
    pub(super) duplicate_loads: u64,
    pub(super) loads: u64,
    pub(super) bytes_read: u64,
    pub(super) bytes_decoded: u64,
    pub(super) bytes_borrowed: u64,
    pub(super) bytes_allocated: u64,
    pub(super) bytes_copied: u64,
    pub(super) bytes_decompressed: u64,
    pub(super) filter_checks: u64,
    pub(super) filter_negatives: u64,
}

#[derive(Debug)]
pub(super) struct CacheEntry {
    pub(super) value: Arc<LoadedPage>,
    pub(super) last_used: u64,
    pub(super) region: CacheRegion,
    pub(super) last_access: PageCacheAccess,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CacheRegion {
    Exact,
    Probationary,
    Protected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Engine-owned logical access provenance used only by the process-local page
/// replacement policy. It is neither serialized nor accepted from callers.
pub(crate) enum PageCacheAccess {
    OrdinaryOperation,
    ProjectedScope(u64),
}

impl PageCacheAccess {
    fn suppresses_promotion_from(self, previous: Self) -> bool {
        matches!(
            (previous, self),
            (Self::ProjectedScope(previous), Self::ProjectedScope(current))
                if previous == current
        )
    }
}

pub(crate) fn next_projected_cache_access() -> Result<PageCacheAccess> {
    CACHE_ACCESS_SCOPE_ID
        .try_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
            current.checked_add(1)
        })
        .map(PageCacheAccess::ProjectedScope)
        .map_err(|_| Error::InvalidProjectedRead("page-cache reuse scope exhausted".into()))
}

pub(crate) type SharedPageCache = Arc<Mutex<PageCache>>;

pub(crate) fn new_page_cache(capacity_bytes: usize) -> SharedPageCache {
    new_page_cache_with_policy(capacity_bytes, PageCachePolicy::default())
        .expect("default page-cache policy is valid")
}

pub(crate) fn new_page_cache_with_policy(
    capacity_bytes: usize,
    policy: PageCachePolicy,
) -> Result<SharedPageCache> {
    new_page_cache_with_limits(capacity_bytes, policy, DEFAULT_SOURCE_PAGE_BYTES)
}

pub(crate) fn new_page_cache_with_limits(
    capacity_bytes: usize,
    policy: PageCachePolicy,
    source_page_bytes: usize,
) -> Result<SharedPageCache> {
    let protected_capacity_bytes = policy.protected_capacity_bytes(capacity_bytes)?;
    Ok(Arc::new(Mutex::new(PageCache {
        source_memory: SourcePageMemory::new(source_page_bytes),
        policy,
        capacity_bytes,
        protected_capacity_bytes,
        resident_bytes: 0,
        probationary_resident_bytes: 0,
        protected_resident_bytes: 0,
        values: HashMap::new(),
        exact_order: BinaryHeap::new(),
        probationary_order: BinaryHeap::new(),
        protected_order: BinaryHeap::new(),
        clock: 0,
        hits: 0,
        misses: 0,
        evictions: 0,
        admissions: 0,
        admission_rejections: 0,
        promotions: 0,
        demotions: 0,
        same_scope_hits: 0,
        duplicate_loads: 0,
        loads: 0,
        bytes_read: 0,
        bytes_decoded: 0,
        bytes_borrowed: 0,
        bytes_allocated: 0,
        bytes_copied: 0,
        bytes_decompressed: 0,
        filter_checks: 0,
        filter_negatives: 0,
    })))
}

pub(crate) fn page_cache_stats(cache: &SharedPageCache) -> PageCacheStats {
    let cache = cache
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let probationary_entries = cache
        .values
        .values()
        .filter(|entry| entry.region == CacheRegion::Probationary)
        .count();
    let protected_entries = cache
        .values
        .values()
        .filter(|entry| entry.region == CacheRegion::Protected)
        .count();
    PageCacheStats {
        policy: cache.policy,
        capacity_bytes: cache.capacity_bytes,
        resident_bytes: cache.resident_bytes,
        entries: cache.values.len(),
        probationary_resident_bytes: cache.probationary_resident_bytes,
        protected_resident_bytes: cache.protected_resident_bytes,
        probationary_entries,
        protected_entries,
        hits: cache.hits,
        misses: cache.misses,
        evictions: cache.evictions,
        admissions: cache.admissions,
        admission_rejections: cache.admission_rejections,
        promotions: cache.promotions,
        demotions: cache.demotions,
        same_scope_hits: cache.same_scope_hits,
        duplicate_loads: cache.duplicate_loads,
        loads: cache.loads,
        bytes_read: cache.bytes_read,
        bytes_decoded: cache.bytes_decoded,
        bytes_borrowed: cache.bytes_borrowed,
        bytes_allocated: cache.bytes_allocated,
        bytes_copied: cache.bytes_copied,
        bytes_decompressed: cache.bytes_decompressed,
        filter_checks: cache.filter_checks,
        filter_negatives: cache.filter_negatives,
    }
}

pub(super) fn record_filter_probe(cache: &SharedPageCache, negative: bool) -> Result<()> {
    let mut cache = cache
        .lock()
        .map_err(|_| Error::InvalidSegment("immutable page cache lock poisoned".into()))?;
    cache.filter_checks = cache.filter_checks.saturating_add(1);
    if negative {
        cache.filter_negatives = cache.filter_negatives.saturating_add(1);
    }
    Ok(())
}

impl PageCache {
    pub(super) fn lookup(
        &mut self,
        key: (u64, usize),
        access: PageCacheAccess,
    ) -> Result<Option<Arc<LoadedPage>>> {
        let Some((region, previous_access)) = self
            .values
            .get(&key)
            .map(|entry| (entry.region, entry.last_access))
        else {
            self.misses = self.misses.saturating_add(1);
            return Ok(None);
        };

        self.hits = self.hits.saturating_add(1);
        match (self.policy, region) {
            (PageCachePolicy::ExactLru, CacheRegion::Exact)
            | (PageCachePolicy::ScanResistantLru { .. }, CacheRegion::Protected) => {
                self.refresh(key, region, access);
            }
            (PageCachePolicy::ScanResistantLru { .. }, CacheRegion::Probationary) => {
                if access.suppresses_promotion_from(previous_access) {
                    self.same_scope_hits = self.same_scope_hits.saturating_add(1);
                    self.refresh(key, region, access);
                } else {
                    self.promote(key, access)?;
                    self.demote_to_protected_target()?;
                }
            }
            _ => {
                return Err(Error::InvalidSegment(
                    "page-cache entry region conflicts with replacement policy".into(),
                ));
            }
        }
        self.maybe_rebuild_orders();
        self.validate_residency()?;
        Ok(Some(Arc::clone(
            &self
                .values
                .get(&key)
                .expect("cache hit remains resident")
                .value,
        )))
    }

    pub(super) fn complete_load(
        &mut self,
        key: (u64, usize),
        loaded: Arc<LoadedPage>,
        resident_bytes: usize,
        access: PageCacheAccess,
    ) -> Result<Arc<LoadedPage>> {
        if let Some(entry) = self.values.get(&key) {
            let region = entry.region;
            let value = Arc::clone(&entry.value);
            self.duplicate_loads = self.duplicate_loads.saturating_add(1);
            self.refresh(key, region, access);
            self.maybe_rebuild_orders();
            self.validate_residency()?;
            return Ok(value);
        }
        if self.capacity_bytes == 0 || resident_bytes > self.capacity_bytes {
            self.admission_rejections = self.admission_rejections.saturating_add(1);
            return Ok(loaded);
        }

        while resident_bytes > self.capacity_bytes.saturating_sub(self.resident_bytes) {
            let removed = match self.policy {
                PageCachePolicy::ExactLru => self.evict_oldest(CacheRegion::Exact)?,
                PageCachePolicy::ScanResistantLru { .. } => {
                    if self.evict_oldest(CacheRegion::Probationary)? {
                        true
                    } else {
                        self.evict_oldest(CacheRegion::Protected)?
                    }
                }
            };
            if !removed {
                return Err(Error::InvalidSegment(
                    "page-cache capacity pressure found no resident eviction victim".into(),
                ));
            }
        }

        let region = match self.policy {
            PageCachePolicy::ExactLru => CacheRegion::Exact,
            PageCachePolicy::ScanResistantLru { .. } => CacheRegion::Probationary,
        };
        let stamp = self.next_stamp();
        self.resident_bytes = self
            .resident_bytes
            .checked_add(resident_bytes)
            .ok_or_else(|| Error::InvalidSegment("page-cache residency overflow".into()))?;
        self.add_region_bytes(region, resident_bytes)?;
        self.values.insert(
            key,
            CacheEntry {
                value: Arc::clone(&loaded),
                last_used: stamp,
                region,
                last_access: access,
            },
        );
        self.push_order(region, stamp, key);
        self.admissions = self.admissions.saturating_add(1);
        self.maybe_rebuild_orders();
        self.validate_residency()?;
        Ok(loaded)
    }

    fn next_stamp(&mut self) -> u64 {
        self.clock = self.clock.wrapping_add(1);
        if self.clock == 0 {
            self.renumber();
        }
        self.clock
    }

    fn refresh(&mut self, key: (u64, usize), region: CacheRegion, access: PageCacheAccess) {
        let stamp = self.next_stamp();
        let entry = self.values.get_mut(&key).expect("cache key was checked");
        debug_assert_eq!(entry.region, region);
        entry.last_used = stamp;
        entry.last_access = access;
        self.push_order(region, stamp, key);
    }

    fn promote(&mut self, key: (u64, usize), access: PageCacheAccess) -> Result<()> {
        let resident_bytes = self
            .values
            .get(&key)
            .filter(|entry| entry.region == CacheRegion::Probationary)
            .map(|entry| entry.value.resident_bytes())
            .ok_or_else(|| {
                Error::InvalidSegment("page-cache promotion source is not probationary".into())
            })?;
        self.subtract_region_bytes(CacheRegion::Probationary, resident_bytes)?;
        self.add_region_bytes(CacheRegion::Protected, resident_bytes)?;
        let stamp = self.next_stamp();
        let entry = self.values.get_mut(&key).expect("cache key was checked");
        entry.region = CacheRegion::Protected;
        entry.last_used = stamp;
        entry.last_access = access;
        self.push_order(CacheRegion::Protected, stamp, key);
        self.promotions = self.promotions.saturating_add(1);
        Ok(())
    }

    fn demote_to_protected_target(&mut self) -> Result<()> {
        while self.protected_resident_bytes > self.protected_capacity_bytes {
            let Some((key, stamp)) = self.pop_oldest_valid(CacheRegion::Protected) else {
                return Err(Error::InvalidSegment(
                    "page-cache protected bytes have no demotion victim".into(),
                ));
            };
            let resident_bytes = self
                .values
                .get(&key)
                .expect("valid cache victim remains resident")
                .value
                .resident_bytes();
            self.subtract_region_bytes(CacheRegion::Protected, resident_bytes)?;
            self.add_region_bytes(CacheRegion::Probationary, resident_bytes)?;
            self.values
                .get_mut(&key)
                .expect("valid cache victim remains resident")
                .region = CacheRegion::Probationary;
            self.probationary_order.push(Reverse((stamp, key)));
            self.demotions = self.demotions.saturating_add(1);
        }
        Ok(())
    }

    pub(super) fn evict_oldest(&mut self, region: CacheRegion) -> Result<bool> {
        let Some((key, _)) = self.pop_oldest_valid(region) else {
            return Ok(false);
        };
        let removed = self
            .values
            .remove(&key)
            .expect("valid cache victim remains resident");
        let resident_bytes = removed.value.resident_bytes();
        self.resident_bytes = self
            .resident_bytes
            .checked_sub(resident_bytes)
            .ok_or_else(|| Error::InvalidSegment("page-cache residency underflow".into()))?;
        self.subtract_region_bytes(region, resident_bytes)?;
        self.evictions = self.evictions.saturating_add(1);
        Ok(true)
    }

    fn pop_oldest_valid(&mut self, region: CacheRegion) -> Option<((u64, usize), u64)> {
        loop {
            let candidate = match region {
                CacheRegion::Exact => self.exact_order.pop(),
                CacheRegion::Probationary => self.probationary_order.pop(),
                CacheRegion::Protected => self.protected_order.pop(),
            };
            let Reverse((stamp, key)) = candidate?;
            if self
                .values
                .get(&key)
                .is_some_and(|entry| entry.region == region && entry.last_used == stamp)
            {
                return Some((key, stamp));
            }
        }
    }

    fn push_order(&mut self, region: CacheRegion, stamp: u64, key: (u64, usize)) {
        let entry = Reverse((stamp, key));
        match region {
            CacheRegion::Exact => self.exact_order.push(entry),
            CacheRegion::Probationary => self.probationary_order.push(entry),
            CacheRegion::Protected => self.protected_order.push(entry),
        }
    }

    fn add_region_bytes(&mut self, region: CacheRegion, resident_bytes: usize) -> Result<()> {
        let field = match region {
            CacheRegion::Exact => return Ok(()),
            CacheRegion::Probationary => &mut self.probationary_resident_bytes,
            CacheRegion::Protected => &mut self.protected_resident_bytes,
        };
        *field = field
            .checked_add(resident_bytes)
            .ok_or_else(|| Error::InvalidSegment("page-cache region residency overflow".into()))?;
        Ok(())
    }

    fn subtract_region_bytes(&mut self, region: CacheRegion, resident_bytes: usize) -> Result<()> {
        let field = match region {
            CacheRegion::Exact => return Ok(()),
            CacheRegion::Probationary => &mut self.probationary_resident_bytes,
            CacheRegion::Protected => &mut self.protected_resident_bytes,
        };
        *field = field
            .checked_sub(resident_bytes)
            .ok_or_else(|| Error::InvalidSegment("page-cache region residency underflow".into()))?;
        Ok(())
    }

    pub(super) fn validate_residency(&self) -> Result<()> {
        if self.resident_bytes > self.capacity_bytes {
            return Err(Error::InvalidSegment(
                "page-cache residency exceeds configured capacity".into(),
            ));
        }
        let classified = self
            .probationary_resident_bytes
            .checked_add(self.protected_resident_bytes)
            .ok_or_else(|| {
                Error::InvalidSegment("page-cache classified residency overflow".into())
            })?;
        match self.policy {
            PageCachePolicy::ExactLru if classified != 0 => Err(Error::InvalidSegment(
                "exact-LRU cache contains scan-resistant region bytes".into(),
            )),
            PageCachePolicy::ScanResistantLru { .. } if classified != self.resident_bytes => {
                Err(Error::InvalidSegment(
                    "scan-resistant page-cache region bytes disagree with total residency".into(),
                ))
            }
            _ => Ok(()),
        }
    }

    fn maybe_rebuild_orders(&mut self) {
        let order_entries = self
            .exact_order
            .len()
            .saturating_add(self.probationary_order.len())
            .saturating_add(self.protected_order.len());
        if order_entries > self.values.len().saturating_mul(2).max(32) {
            self.rebuild_orders();
        }
    }

    fn rebuild_orders(&mut self) {
        self.exact_order.clear();
        self.probationary_order.clear();
        self.protected_order.clear();
        let entries = self
            .values
            .iter()
            .map(|(key, entry)| (entry.region, entry.last_used, *key))
            .collect::<Vec<_>>();
        for (region, stamp, key) in entries {
            self.push_order(region, stamp, key);
        }
    }

    fn renumber(&mut self) {
        let mut ordered = self
            .values
            .iter()
            .map(|(key, entry)| (entry.last_used, *key))
            .collect::<Vec<_>>();
        ordered.sort_unstable();
        for (index, (_, key)) in ordered.into_iter().enumerate() {
            self.values
                .get_mut(&key)
                .expect("cache key exists")
                .last_used = u64::try_from(index + 1).expect("cache size fits u64");
        }
        self.clock = u64::try_from(self.values.len()).expect("cache size fits u64");
        self.rebuild_orders();
    }
}
