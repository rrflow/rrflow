//! Active read views, view leases and the projected read stream type.

use super::*;

#[derive(Debug, Default)]
pub(crate) struct ActiveReadViews {
    pub(super) active: usize,
    pub(super) pinned_bytes: u64,
    pub(super) manifests: BTreeMap<String, ActiveManifest>,
    pub(super) segments: BTreeMap<String, ActiveSegment>,
    pub(super) memtables: BTreeMap<usize, ActiveMemtable>,
}

#[derive(Debug)]
pub(super) struct ActiveManifest {
    pub(super) manifest: Arc<Manifest>,
    pub(super) references: usize,
}

#[derive(Debug)]
pub(super) struct ActiveSegment {
    descriptor: crate::SegmentDescriptor,
    references: usize,
}

// Level is manifest placement, not immutable segment-file identity.
pub(super) fn same_segment_file(
    first: &crate::SegmentDescriptor,
    second: &crate::SegmentDescriptor,
) -> bool {
    if first == second {
        return true;
    }
    let mut normalized = second.clone();
    normalized.level = first.level;
    *first == normalized
}

#[derive(Debug)]
pub(super) struct ActiveMemtable {
    // Retaining the allocation prevents address reuse until the last lease.
    generation: Arc<crate::mutable_memory::MutableGeneration>,
    references: usize,
    bytes: u64,
}

impl ActiveReadViews {
    pub(crate) fn ownership(registry: &Arc<Mutex<Self>>) -> Result<(u64, u64)> {
        let views = registry
            .lock()
            .map_err(|_| Error::PoisonedReadViewRegistry)?;
        Ok((views.active as u64, views.pinned_bytes))
    }

    pub(crate) fn acquire(
        registry: &Arc<Mutex<Self>>,
        manifest: Arc<Manifest>,
        memtable: Option<&Arc<crate::mutable_memory::MutableGeneration>>,
        memtable_bytes: u64,
        segments: &[Arc<Segment>],
        budget: ProjectedReadBudget,
    ) -> Result<ReadViewLease> {
        let mut views = registry
            .lock()
            .map_err(|_| Error::PoisonedReadViewRegistry)?;
        let next_active = views
            .active
            .checked_add(1)
            .ok_or(Error::PoisonedReadViewRegistry)?;
        check_limit(
            ProjectedReadResource::ActiveViews,
            budget.max_active_views as u64,
            next_active as u64,
        )?;
        let digest = manifest.digest.clone();
        let next_manifest_refs = match views.manifests.get(&digest) {
            Some(entry) => {
                if !Arc::ptr_eq(&entry.manifest, &manifest) && entry.manifest != manifest {
                    return Err(Error::PoisonedReadViewRegistry);
                }
                entry
                    .references
                    .checked_add(1)
                    .ok_or(Error::PoisonedReadViewRegistry)?
            }
            None => 1,
        };
        let mut next_segments = Vec::with_capacity(segments.len());
        let mut identities = std::collections::BTreeSet::new();
        let mut new_segment_charge = 0_u64;
        for segment in segments {
            let descriptor = &segment.descriptor;
            if !identities.insert(&descriptor.id) {
                return Err(Error::PoisonedReadViewRegistry);
            }
            let references = match views.segments.get(&descriptor.id) {
                Some(entry) => {
                    if !same_segment_file(&entry.descriptor, descriptor) {
                        return Err(Error::PoisonedReadViewRegistry);
                    }
                    entry
                        .references
                        .checked_add(1)
                        .ok_or(Error::PoisonedReadViewRegistry)?
                }
                None => {
                    new_segment_charge = new_segment_charge
                        .checked_add(descriptor.bytes)
                        .ok_or(Error::PoisonedReadViewRegistry)?;
                    1
                }
            };
            next_segments.push((descriptor, references));
        }
        // Empty views do not retain a mutable generation or cause copy-on-write.
        let mutable = if memtable_bytes == 0 {
            None
        } else {
            let generation = memtable.ok_or(Error::PoisonedReadViewRegistry)?;
            if u64::try_from(generation.approximate_bytes()).ok() != Some(memtable_bytes) {
                return Err(Error::PoisonedReadViewRegistry);
            }
            let key = Arc::as_ptr(generation) as usize;
            let (references, charge) = match views.memtables.get(&key) {
                Some(entry) => {
                    if !Arc::ptr_eq(&entry.generation, generation) || entry.bytes != memtable_bytes
                    {
                        return Err(Error::PoisonedReadViewRegistry);
                    }
                    (
                        entry
                            .references
                            .checked_add(1)
                            .ok_or(Error::PoisonedReadViewRegistry)?,
                        0,
                    )
                }
                None => (1, memtable_bytes),
            };
            Some((key, generation, references, charge))
        };
        let new_memtable_charge = mutable.as_ref().map_or(0, |entry| entry.3);
        let next_pinned = views
            .pinned_bytes
            .checked_add(new_memtable_charge)
            .and_then(|bytes| bytes.checked_add(new_segment_charge))
            .ok_or(Error::PoisonedReadViewRegistry)?;
        check_limit(
            ProjectedReadResource::PinnedBytes,
            budget.max_pinned_bytes,
            next_pinned,
        )?;
        match views.manifests.get_mut(&digest) {
            Some(entry) => entry.references = next_manifest_refs,
            None => {
                views.manifests.insert(
                    digest.clone(),
                    ActiveManifest {
                        manifest,
                        references: next_manifest_refs,
                    },
                );
            }
        }
        let mut segment_ids = Vec::with_capacity(next_segments.len());
        for (descriptor, references) in next_segments {
            match views.segments.get_mut(&descriptor.id) {
                Some(entry) => entry.references = references,
                None => {
                    views.segments.insert(
                        descriptor.id.clone(),
                        ActiveSegment {
                            descriptor: descriptor.clone(),
                            references,
                        },
                    );
                }
            }
            segment_ids.push(descriptor.id.clone());
        }
        let memtable_key = mutable.map(|(key, generation, references, _)| {
            match views.memtables.get_mut(&key) {
                Some(entry) => entry.references = references,
                None => {
                    views.memtables.insert(
                        key,
                        ActiveMemtable {
                            generation: Arc::clone(generation),
                            references,
                            bytes: memtable_bytes,
                        },
                    );
                }
            }
            key
        });
        views.active = next_active;
        views.pinned_bytes = next_pinned;
        Ok(ReadViewLease {
            registry: Arc::clone(registry),
            manifest: digest,
            segment_ids,
            memtable_key,
            released: false,
        })
    }

    pub(crate) fn visit_segments(
        registry: &Arc<Mutex<Self>>,
        mut visit: impl FnMut(&crate::SegmentDescriptor) -> Result<()>,
    ) -> Result<()> {
        let views = registry
            .lock()
            .map_err(|_| Error::PoisonedReadViewRegistry)?;
        for entry in views.segments.values() {
            visit(&entry.descriptor)?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct ReadViewLease {
    pub(super) registry: Arc<Mutex<ActiveReadViews>>,
    pub(super) manifest: String,
    pub(super) segment_ids: Vec<String>,
    pub(super) memtable_key: Option<usize>,
    pub(super) released: bool,
}

impl ReadViewLease {
    pub(super) fn release(&mut self) -> Result<()> {
        if self.released {
            return Ok(());
        }
        let mut views = self
            .registry
            .lock()
            .map_err(|_| Error::PoisonedReadViewRegistry)?;
        let next_active = views
            .active
            .checked_sub(1)
            .ok_or(Error::PoisonedReadViewRegistry)?;
        let manifest = views
            .manifests
            .get(&self.manifest)
            .ok_or(Error::PoisonedReadViewRegistry)?;
        let next_manifest_refs = manifest
            .references
            .checked_sub(1)
            .ok_or(Error::PoisonedReadViewRegistry)?;
        let mut segment_charge = 0_u64;
        let mut next_segments = Vec::with_capacity(self.segment_ids.len());
        for id in &self.segment_ids {
            let entry = views
                .segments
                .get(id)
                .ok_or(Error::PoisonedReadViewRegistry)?;
            let refs = entry
                .references
                .checked_sub(1)
                .ok_or(Error::PoisonedReadViewRegistry)?;
            if refs == 0 {
                segment_charge = segment_charge
                    .checked_add(entry.descriptor.bytes)
                    .ok_or(Error::PoisonedReadViewRegistry)?;
            }
            next_segments.push((id, refs));
        }
        let mutable = match self.memtable_key {
            Some(key) => {
                let entry = views
                    .memtables
                    .get(&key)
                    .ok_or(Error::PoisonedReadViewRegistry)?;
                let refs = entry
                    .references
                    .checked_sub(1)
                    .ok_or(Error::PoisonedReadViewRegistry)?;
                Some((key, refs, if refs == 0 { entry.bytes } else { 0 }))
            }
            None => None,
        };
        let mutable_charge = mutable.as_ref().map_or(0, |entry| entry.2);
        let next_pinned = views
            .pinned_bytes
            .checked_sub(mutable_charge)
            .and_then(|bytes| bytes.checked_sub(segment_charge))
            .ok_or(Error::PoisonedReadViewRegistry)?;
        if next_manifest_refs == 0 {
            views.manifests.remove(&self.manifest);
        } else {
            views
                .manifests
                .get_mut(&self.manifest)
                .expect("validated manifest")
                .references = next_manifest_refs;
        }
        for (id, refs) in next_segments {
            if refs == 0 {
                views.segments.remove(id);
            } else {
                views
                    .segments
                    .get_mut(id)
                    .expect("validated segment")
                    .references = refs;
            }
        }
        if let Some((key, refs, _)) = mutable {
            if refs == 0 {
                views.memtables.remove(&key);
            } else {
                views
                    .memtables
                    .get_mut(&key)
                    .expect("validated mutable generation")
                    .references = refs;
            }
        }
        views.active = next_active;
        views.pinned_bytes = next_pinned;
        self.released = true;
        Ok(())
    }
}

impl Drop for ReadViewLease {
    fn drop(&mut self) {
        let _ = self.release();
    }
}

#[derive(Debug)]
pub(crate) struct ReadView {
    pub sequence: u64,
    pub manifest: String,
    pub memtable: Arc<crate::mutable_memory::MutableGeneration>,
    pub segments: Vec<Arc<Segment>>,
    pub pinned_bytes: u64,
}

#[derive(Debug)]
pub struct ProjectedReadStream {
    pub(super) projection: ProjectedReadProjection,
    pub(super) budget: ProjectedReadBudget,
    pub(super) meter: ProjectedReadMeter,
    pub(super) cursors: Vec<RunCursor>,
    pub(super) heads: Vec<Option<ProjectedEntry>>,
    pub(super) heap: BinaryHeap<HeapItem>,
    pub(super) unopened: BinaryHeap<HeapItem>,
    pub(super) pending: Option<PreparedRow>,
    pub(super) initialized: bool,
    pub(super) terminal: bool,
    pub(super) lease: Option<ReadViewLease>,
    pub(super) output_memory: ProjectedOutputMemory,
}
