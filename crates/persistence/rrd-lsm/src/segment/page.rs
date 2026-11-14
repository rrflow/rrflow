//! Page sources, page reads, verification and decompression.

use super::*;

#[derive(Debug, Clone)]
pub(super) enum PageSource {
    File {
        file: Arc<crate::io::SegmentFile>,
        selected: SelectedIo,
        io: SharedIoContext,
    },
    Mapped {
        bytes: Arc<Mmap>,
        io: SharedIoContext,
    },
    Bytes(Arc<Vec<u8>>),
}

#[derive(Debug)]
pub(super) struct LoadedPage {
    pub(super) buffer: Buffer,
    pub(super) borrowed: bool,
    pub(super) allocated_bytes: usize,
    pub(super) copied_bytes: usize,
    pub(super) decompressed_bytes: usize,
    pub(super) actual_io: Option<SelectedIo>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct PageReadContext {
    raw_borrowed: bool,
    source_allocated_bytes: usize,
    raw_copied_bytes: usize,
    actual_io: Option<SelectedIo>,
}

impl LoadedPage {
    pub(super) fn resident_bytes(&self) -> usize {
        self.buffer.capacity().max(self.buffer.len()).max(1)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(super) struct PageLoadEvidence {
    pub cache_hit: bool,
    pub cache_miss: bool,
    pub cache_load: bool,
    pub actual_io: Option<SelectedIo>,
    pub physical_bytes: u64,
    pub borrowed_bytes: u64,
    pub decoded_bytes: u64,
    pub decompressed_bytes: u64,
    pub allocated_bytes: u64,
    pub copied_bytes: u64,
}

pub(super) fn open_page_source(
    file: crate::io::SegmentFile,
    io: SharedIoContext,
) -> Result<PageSource> {
    let selected = io.file_backend()?;
    if selected == SelectedIo::Mmap {
        let lease = file.lease()?;
        let mapped = unsafe { MmapOptions::new().map(&*lease) };
        match mapped {
            Ok(bytes) => {
                io.record_segment(SelectedIo::Mmap);
                return Ok(PageSource::Mapped {
                    bytes: Arc::new(bytes),
                    io,
                });
            }
            Err(error) if io.policy().allow_fallback => {
                io.record_fallback(&format!("mmap failed: {error}"));
            }
            Err(error) => return Err(Error::Io(error)),
        }
    }
    let selected = if selected == SelectedIo::Mmap {
        SelectedIo::Bounded
    } else {
        selected
    };
    io.record_segment(selected);
    Ok(PageSource::File {
        file: Arc::new(file),
        selected,
        io,
    })
}

pub(super) fn reserve_source(
    cache: &SharedPageCache,
    bytes: usize,
) -> Result<source_memory::SourceReservation> {
    let mut cache = cache
        .lock()
        .map_err(|_| Error::InvalidSegment("immutable page cache lock poisoned".into()))?;
    loop {
        let stats = cache.source_memory.stats();
        if bytes <= stats.limit_bytes - stats.reserved_bytes {
            break;
        }
        let removed = match cache.policy {
            PageCachePolicy::ExactLru => cache.evict_oldest(CacheRegion::Exact)?,
            PageCachePolicy::ScanResistantLru { .. } => {
                cache.evict_oldest(CacheRegion::Probationary)?
                    || cache.evict_oldest(CacheRegion::Protected)?
            }
        };
        if !removed {
            break;
        }
    }
    cache.source_memory.reserve(bytes)
}

pub(crate) fn source_page_memory(cache: &SharedPageCache) -> SourcePageMemory {
    cache
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .source_memory
        .clone()
}

pub(super) fn read_page(
    source: &PageSource,
    descriptor: &PageDescriptor,
    cache: &SharedPageCache,
) -> Result<LoadedPage> {
    let start = usize::try_from(descriptor.offset)
        .map_err(|_| Error::InvalidSegment("v6 page offset exceeds usize".into()))?;
    let end = start
        .checked_add(descriptor.physical_bytes)
        .ok_or_else(|| Error::InvalidSegment("v6 page range overflow".into()))?;
    match source {
        PageSource::File { file, selected, io } => {
            let lease = file.lease()?;
            let file = &*lease;
            let reservation = reserve_source(cache, descriptor.physical_bytes)?;
            let mut bytes = MutableBuffer::from_len_zeroed(descriptor.physical_bytes);
            // A row group retains a complete key/version chain and can exceed
            // the soft page target. The I/O policy bounds each native request,
            // not that authenticated page. Do not expose any bytes until every
            // chunk has been acquired and the full stored-page digest matches.
            let mut actual_io = *selected;
            let mut offset = descriptor.offset;
            if descriptor.physical_bytes == 0 {
                actual_io = io.read_exact_at(actual_io, file, offset, bytes.as_slice_mut())?;
            } else {
                for chunk in bytes
                    .as_slice_mut()
                    .chunks_mut(io.policy().max_request_bytes)
                {
                    actual_io = io.read_exact_at(actual_io, file, offset, chunk)?;
                    offset = offset.checked_add(chunk.len() as u64).ok_or_else(|| {
                        Error::InvalidSegment("v6 page read offset overflow".into())
                    })?;
                }
            }
            let allocated = bytes.capacity();
            let buffer = reservation.wrap(bytes)?;
            verify_stored_page(buffer.as_slice(), descriptor)?;
            finish_page(
                cache,
                buffer.as_slice(),
                descriptor,
                Some(buffer.clone()),
                PageReadContext {
                    source_allocated_bytes: allocated,
                    actual_io: Some(actual_io),
                    ..PageReadContext::default()
                },
            )
        }
        PageSource::Mapped { bytes, io } => {
            let slice = bytes
                .get(start..end)
                .ok_or_else(|| Error::InvalidSegment("v6 mapped page range is absent".into()))?;
            io.record_read(SelectedIo::Mmap, slice.len());
            verify_stored_page(slice, descriptor)?;
            let raw = if descriptor.compression == format::PageCompression::None {
                Some(borrow_mmap(bytes, start, slice.len())?)
            } else {
                None
            };
            finish_page(
                cache,
                slice,
                descriptor,
                raw,
                PageReadContext {
                    raw_borrowed: true,
                    actual_io: Some(SelectedIo::Mmap),
                    ..PageReadContext::default()
                },
            )
        }
        PageSource::Bytes(bytes) => {
            let slice = bytes
                .get(start..end)
                .ok_or_else(|| Error::InvalidSegment("v6 snapshot page range is absent".into()))?;
            verify_stored_page(slice, descriptor)?;
            if descriptor.compression == format::PageCompression::None {
                let reservation = reserve_source(cache, slice.len())?;
                let mut bytes = MutableBuffer::from_len_zeroed(slice.len());
                bytes.as_slice_mut().copy_from_slice(slice);
                let allocated = bytes.capacity();
                let buffer = reservation.wrap(bytes)?;
                finish_page(
                    cache,
                    buffer.as_slice(),
                    descriptor,
                    Some(buffer.clone()),
                    PageReadContext {
                        source_allocated_bytes: allocated,
                        raw_copied_bytes: slice.len(),
                        ..PageReadContext::default()
                    },
                )
            } else {
                finish_page(cache, slice, descriptor, None, PageReadContext::default())
            }
        }
    }
}

pub(super) fn verify_stored_page(stored: &[u8], descriptor: &PageDescriptor) -> Result<()> {
    if ring::digest::digest(&ring::digest::SHA256, stored).as_ref() != descriptor.digest {
        return invalid("v6 stored-page checksum does not match");
    }
    Ok(())
}

pub(super) fn finish_page(
    cache: &SharedPageCache,
    stored: &[u8],
    descriptor: &PageDescriptor,
    raw_buffer: Option<Buffer>,
    context: PageReadContext,
) -> Result<LoadedPage> {
    match descriptor.compression {
        format::PageCompression::None => {
            let buffer = raw_buffer
                .ok_or_else(|| Error::InvalidSegment("v6 raw page has no source buffer".into()))?;
            Ok(LoadedPage {
                buffer,
                borrowed: context.raw_borrowed,
                allocated_bytes: context.source_allocated_bytes,
                copied_bytes: context.raw_copied_bytes,
                decompressed_bytes: 0,
                actual_io: context.actual_io,
            })
        }
        format::PageCompression::Lz4Block => {
            let reservation = reserve_source(cache, descriptor.logical_bytes)?;
            let mut decoded = MutableBuffer::from_len_zeroed(descriptor.logical_bytes);
            let decoded_bytes = lz4_flex::block::decompress_into(stored, decoded.as_slice_mut())
                .map_err(|error| {
                    Error::InvalidSegment(format!("v6 LZ4 page decode failed: {error}"))
                })?;
            if decoded_bytes != descriptor.logical_bytes {
                return invalid(format!(
                    "v6 LZ4 page decoded {decoded_bytes} bytes, expected {}",
                    descriptor.logical_bytes
                ));
            }
            let decoded_capacity = decoded.capacity();
            let buffer = reservation.wrap(decoded)?;
            let allocated_bytes = context
                .source_allocated_bytes
                .checked_add(decoded_capacity)
                .ok_or_else(|| {
                    Error::InvalidSegment("v6 page allocation evidence overflow".into())
                })?;
            Ok(LoadedPage {
                buffer,
                borrowed: false,
                allocated_bytes,
                copied_bytes: 0,
                decompressed_bytes: descriptor.logical_bytes,
                actual_io: context.actual_io,
            })
        }
    }
}

pub(super) fn borrow_mmap(bytes: &Arc<Mmap>, start: usize, length: usize) -> Result<Buffer> {
    if length == 0 {
        return Ok(Buffer::default());
    }
    let pointer = unsafe { bytes.as_ptr().add(start) as *mut u8 };
    let pointer = NonNull::new(pointer)
        .ok_or_else(|| Error::InvalidSegment("v6 mapped page has a null pointer".into()))?;
    let owner: Arc<dyn Allocation> = Arc::clone(bytes) as Arc<dyn Allocation>;
    Ok(unsafe { Buffer::from_custom_allocation(pointer, length, owner) })
}
