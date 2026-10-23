//! Linux owned-file cold-start observation. Never changes the host cache policy.
use super::*;

#[cfg(not(target_os = "linux"))]
pub(super) async fn run(
    _estate: &Estate,
    _child: &mut Option<super::super::process::Process>,
    _work: &mut Workload,
    _campaign: &StorageCampaign,
    _clock: Instant,
    _lifetime: Duration,
    _progress: &ProgressContext<'_>,
) -> DriverResult<ColdStartEvidence> {
    Err("verified owned-file cold start is available only on Linux".into())
}

#[cfg(target_os = "linux")]
pub(super) async fn run(
    estate: &Estate,
    child: &mut Option<super::super::process::Process>,
    work: &mut Workload,
    campaign: &StorageCampaign,
    clock: Instant,
    lifetime: Duration,
    progress: &ProgressContext<'_>,
) -> DriverResult<ColdStartEvidence> {
    let started_ns = ns(clock);
    let deadline = Instant::now() + Duration::from_millis(campaign.schedule.lifecycle_ms.get());
    super::super::kill(child.take().ok_or("missing cold-start server")?, clock);
    let locator =
        rrd_engine::RrdEngine::read_project_locator(&estate.project).map_err(|e| e.to_string())?;
    let project = std::fs::canonicalize(&estate.project).map_err(|e| e.to_string())?;
    let path = project.join(locator.storage_root);
    if std::fs::symlink_metadata(&path)
        .map_err(|e| e.to_string())?
        .file_type()
        .is_symlink()
    {
        return Err("cold native root must not be a symlink".into());
    }
    let root = std::fs::canonicalize(path).map_err(|e| e.to_string())?;
    if !root.starts_with(&project) || root == project {
        return Err("cold root escaped its owned installed project".into());
    }
    let policy = &estate.plan["configuration"]["storage"];
    let limit = |name: &str| {
        policy[name]
            .as_u64()
            .ok_or_else(|| format!("missing cold admission {name}"))
    };
    let mut observed = linux::evict(
        &root,
        limit("recovery_files")?,
        limit("recovery_metadata_bytes")?,
        limit("recovery_read_bytes")?,
        deadline,
        progress,
        work.seed,
    )?;
    observed.started_ns = started_ns;
    observed.evicted_ns = ns(clock);
    let (reopened, client, _) = estate.start_for(clock, lifetime);
    *child = Some(reopened);
    work.client = client;
    observed.ready_ns = ns(clock);
    let request = work.seed_record(work.seed % work.records)?;
    tokio::time::timeout_at(deadline.into(), work.verify(&request))
        .await
        .map_err(|_| "cold first read exceeded lifecycle deadline")??;
    observed.finished_ns = ns(clock);
    observed.readback_sha256 = request.effect_sha256;
    if Instant::now() > deadline {
        return Err("cold startup exceeded lifecycle deadline".into());
    }
    progress.writer.record_event(&ProgressEvent::ColdStart {
        point: progress.point.id.clone(),
        seed: work.seed,
        observation: observed.clone(),
    })?;
    progress.writer.checkpoint_progress()?;
    Ok(observed)
}

#[cfg(target_os = "linux")]
mod linux {
    use super::*;
    use std::{
        fs::File,
        os::unix::{
            fs::{MetadataExt, OpenOptionsExt},
            io::AsRawFd,
        },
        path::Path,
    };

    fn check_deadline(deadline: Instant) -> DriverResult<()> {
        if Instant::now() >= deadline {
            Err("cold file observation deadline".into())
        } else {
            Ok(())
        }
    }

    // Sorting makes the metadata identity reproducible. Every retained path is
    // charged before push; depth/count/bytes are independently bounded.
    fn inventory(
        root: &Path,
        count: u64,
        metadata: u64,
        deadline: Instant,
    ) -> DriverResult<Vec<PathBuf>> {
        let mut files = Vec::new();
        let mut pending = vec![(root.to_owned(), 0u8)];
        let mut entries = 0u64;
        let mut charged = 0u64;
        let device = std::fs::metadata(root).map_err(|e| e.to_string())?.dev();
        while let Some((directory, depth)) = pending.pop() {
            check_deadline(deadline)?;
            for entry in std::fs::read_dir(directory).map_err(|e| e.to_string())? {
                let entry = entry.map_err(|e| e.to_string())?;
                let path = entry.path();
                entries = entries
                    .checked_add(1)
                    .ok_or("cold inventory count overflow")?;
                charged = charged
                    .checked_add((path.as_os_str().len() as u64) * 4 + 512)
                    .ok_or("cold inventory byte overflow")?;
                if entries > count || charged > metadata {
                    return Err("cold inventory admission exhausted".into());
                }
                let entry = std::fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
                if entry.file_type().is_symlink() || entry.dev() != device {
                    return Err("cold inventory cannot cross aliases or mounts".into());
                }
                if entry.is_dir() {
                    if depth >= 4 {
                        return Err("cold inventory directory depth exceeded".into());
                    }
                    pending.push((path, depth + 1));
                } else if entry.is_file() {
                    files.push(path);
                } else {
                    return Err("cold inventory contains a non-regular entry".into());
                }
            }
        }
        files.sort();
        Ok(files)
    }

    struct Mapping {
        address: *mut libc::c_void,
        length: usize,
    }
    impl Drop for Mapping {
        fn drop(&mut self) {
            // SAFETY: owns the exact successful mmap range, never dereferenced.
            unsafe {
                libc::munmap(self.address, self.length);
            }
        }
    }

    fn resident(file: &File, length: u64, page: u64, deadline: Instant) -> DriverResult<u64> {
        let mut offset = 0u64;
        let mut resident = 0u64;
        // At most 64 MiB of virtual address space and 16 KiB residency bytes.
        while offset < length {
            check_deadline(deadline)?;
            let bytes = (length - offset).min(64 * 1024 * 1024) as usize;
            // SAFETY: valid owned fd; aligned offset and nonzero bounded length.
            // PROT_NONE ensures observation never reads the mapped file data.
            let address = unsafe {
                libc::mmap(
                    std::ptr::null_mut(),
                    bytes,
                    libc::PROT_NONE,
                    libc::MAP_SHARED,
                    file.as_raw_fd(),
                    offset as libc::off_t,
                )
            };
            if address == libc::MAP_FAILED {
                return Err(std::io::Error::last_os_error().to_string());
            }
            let mapping = Mapping {
                address,
                length: bytes,
            };
            let mut pages = vec![0u8; (bytes as u64).div_ceil(page) as usize];
            // SAFETY: successful mapping and one writable byte per mapped page.
            if unsafe { libc::mincore(mapping.address, mapping.length, pages.as_mut_ptr()) } != 0 {
                return Err(std::io::Error::last_os_error().to_string());
            }
            resident += pages.iter().filter(|byte| **byte & 1 != 0).count() as u64;
            offset += bytes as u64;
        }
        Ok(resident)
    }

    pub(super) fn evict(
        root: &Path,
        files: u64,
        metadata: u64,
        maximum_bytes: u64,
        deadline: Instant,
        progress: &ProgressContext<'_>,
        seed: u64,
    ) -> DriverResult<ColdStartEvidence> {
        // SAFETY: sysconf has no pointer arguments or memory effects.
        let page = u64::try_from(unsafe { libc::sysconf(libc::_SC_PAGESIZE) })
            .map_err(|_| "native page size unavailable")?;
        if !page.is_power_of_two() || !(4096..=65536).contains(&page) {
            return Err("unsupported native page size".into());
        }
        let paths = inventory(root, files, metadata, deadline)?;
        let mut observation = ColdStartEvidence {
            started_ns: 0,
            evicted_ns: 0,
            ready_ns: 0,
            finished_ns: 0,
            files: 0,
            file_bytes: 0,
            pages: 0,
            resident_pages: 0,
            page_bytes: page,
            inventory_sha256: digest("c07g.cold-inventory", &seed)?,
            readback_sha256: digest("c07g.cold-pending", &seed)?,
        };
        for path in paths {
            check_deadline(deadline)?;
            let file = std::fs::OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_NOFOLLOW)
                .open(&path)
                .map_err(|e| e.to_string())?;
            let before = file.metadata().map_err(|e| e.to_string())?;
            // mincore must be permitted to disclose real file residency. Reject
            // shared hardlinks and foreign owners rather than assuming a cache state.
            if !before.is_file()
                || before.nlink() != 1
                || before.uid() != unsafe { libc::geteuid() }
            {
                return Err("cold observation requires a singly linked owned regular file".into());
            }
            observation.file_bytes = observation
                .file_bytes
                .checked_add(before.len())
                .ok_or("cold byte overflow")?;
            if observation.file_bytes > maximum_bytes {
                return Err("cold file-byte admission exhausted".into());
            }
            let identity = digest(
                "c07g.cold-file",
                &(
                    path.strip_prefix(root).map_err(|e| e.to_string())?,
                    before.dev(),
                    before.ino(),
                    before.len(),
                    before.mtime(),
                    before.mtime_nsec(),
                ),
            )?;
            file.sync_all().map_err(|e| e.to_string())?;
            let pages = before.len().div_ceil(page);
            let length = pages
                .checked_mul(page)
                .filter(|v| *v <= i64::MAX as u64)
                .ok_or("cold aligned size overflow")?;
            if length > 0 {
                // SAFETY: valid owned file descriptor and nonnegative aligned range.
                let code = unsafe {
                    libc::posix_fadvise(
                        file.as_raw_fd(),
                        0,
                        length as libc::off_t,
                        libc::POSIX_FADV_DONTNEED,
                    )
                };
                if code != 0 {
                    return Err(std::io::Error::from_raw_os_error(code).to_string());
                }
            }
            let resident_pages = resident(&file, before.len(), page, deadline)?;
            progress.writer.record_event(&ProgressEvent::ColdFile {
                point: progress.point.id.clone(),
                seed,
                file_identity_sha256: identity.clone(),
                bytes: before.len(),
                page_bytes: page,
                pages,
                resident_pages,
            })?;
            progress.writer.checkpoint_progress()?;
            let after = file.metadata().map_err(|e| e.to_string())?;
            if after.len() != before.len()
                || after.mtime() != before.mtime()
                || after.mtime_nsec() != before.mtime_nsec()
            {
                return Err("cold file changed during observation".into());
            }
            if resident_pages != 0 {
                return Err("cache advice left resident pages; cold state unproven".into());
            }
            observation.files += 1;
            observation.pages += pages;
            observation.inventory_sha256 = digest(
                "c07g.cold-inventory",
                &(observation.inventory_sha256, identity),
            )?;
        }
        if observation.pages == 0 {
            return Err("cold inventory contains no data pages".into());
        }
        check_deadline(deadline)?;
        Ok(observation)
    }
}
