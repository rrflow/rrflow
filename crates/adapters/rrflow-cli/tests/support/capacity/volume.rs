//! Volumes, fillers and the allocation barriers that fill them.

use super::*;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Volume {
    pub(super) version: u16,
    pub(super) root: PathBuf,
    pub(super) nonce: String,
    pub(super) filesystem: String,
    pub(super) max_bytes: u64,
}

impl Volume {
    pub(super) fn admitted() -> Self {
        let descriptor = std::env::var_os("RRFLOW_CAPACITY_DESCRIPTOR").expect(
            "run through scripts/ci/run_native_capacity.py; arbitrary volume roots are forbidden",
        );
        let mut bytes = Vec::new();
        File::open(descriptor)
            .unwrap()
            .take(4097)
            .read_to_end(&mut bytes)
            .unwrap();
        assert!(bytes.len() <= 4096);
        let mut volume: Self = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(volume.version, 1);
        assert_eq!(volume.nonce.len(), 64);
        assert!(volume.nonce.bytes().all(|b| b.is_ascii_hexdigit()));
        assert!((128 * 1024 * 1024..=2 * 1024 * 1024 * 1024).contains(&volume.max_bytes));
        assert!(volume.root.is_absolute());
        assert!(!std::fs::symlink_metadata(&volume.root)
            .unwrap()
            .file_type()
            .is_symlink());
        volume.root = std::fs::canonicalize(&volume.root).unwrap();
        assert_ne!(
            volume.root,
            std::fs::canonicalize(std::env::temp_dir()).unwrap()
        );
        volume.verify_native_mount();
        assert!(matches!(
            volume.filesystem.as_str(),
            "ext4" | "apfs" | "ntfs"
        ));
        let marker = volume.root.join("RRFLOW-CAPACITY-OWNER");
        assert!(std::fs::symlink_metadata(&marker).unwrap().is_file());
        let mut owner = Vec::new();
        File::open(marker)
            .unwrap()
            .take(65)
            .read_to_end(&mut owner)
            .unwrap();
        assert_eq!(owner, volume.nonce.as_bytes());
        volume
    }

    #[cfg(not(target_os = "macos"))]
    fn verify_native_mount(&self) {
        let disks = sysinfo::Disks::new_with_refreshed_list();
        let matching: Vec<_> = disks
            .iter()
            .filter(|disk| {
                std::fs::canonicalize(disk.mount_point()).ok().as_ref() == Some(&self.root)
            })
            .collect();
        assert_eq!(
            matching.len(),
            1,
            "fixture must be a separately enumerated native volume"
        );
        let disk = matching[0];
        assert_eq!(
            disk.file_system().to_str().unwrap().to_ascii_lowercase(),
            self.filesystem
        );
        assert!(disk.total_space() > 32 * 1024 * 1024 && disk.total_space() <= self.max_bytes);
    }

    #[cfg(target_os = "macos")]
    fn verify_native_mount(&self) {
        use std::os::unix::{ffi::OsStrExt, fs::MetadataExt};
        // sysinfo intentionally hides non-browsable APFS volumes. Query the
        // kernel mount itself; visibility in Finder is not volume admission.
        let path = std::ffi::CString::new(self.root.as_os_str().as_bytes()).unwrap();
        let mut native = std::mem::MaybeUninit::<libc::statfs>::uninit();
        // SAFETY: NUL-terminated path and writable, correctly sized output.
        let result = unsafe { libc::statfs(path.as_ptr(), native.as_mut_ptr()) };
        assert_eq!(
            result,
            0,
            "statfs failed: {}",
            std::io::Error::last_os_error()
        );
        // SAFETY: successful statfs initialized every output field.
        let native = unsafe { native.assume_init() };
        let text = |field: &[libc::c_char]| {
            let end = field
                .iter()
                .position(|c| *c == 0)
                .expect("terminated statfs field");
            field[..end].iter().map(|c| *c as u8).collect::<Vec<_>>()
        };
        let mount = text(&native.f_mntonname);
        assert_eq!(
            std::fs::canonicalize(std::ffi::OsStr::from_bytes(&mount)).unwrap(),
            self.root,
            "fixture must be an exact native mount root"
        );
        assert_eq!(text(&native.f_fstypename), self.filesystem.as_bytes());
        assert_eq!(self.filesystem, "apfs");
        assert_ne!(
            std::fs::metadata(&self.root).unwrap().dev(),
            std::fs::metadata(self.root.parent().unwrap())
                .unwrap()
                .dev(),
            "fixture must be a separate filesystem"
        );
        let total = native
            .f_blocks
            .checked_mul(u64::from(native.f_bsize))
            .expect("capacity overflow");
        assert!(total > 32 * 1024 * 1024 && total <= self.max_bytes);
    }
}

pub(super) struct Filler {
    pub(super) file: Option<File>,
    pub(super) path: PathBuf,
    pub(super) bytes: u64,
    pub(super) native_code: i32,
    pub(super) data_refills: u32,
    pub(super) metadata_payload_bytes: u64,
    pub(super) metadata_paths: Vec<PathBuf>,
    pub(super) metadata_directories: Vec<PathBuf>,
    pub(super) metadata_native_code: Option<i32>,
    pub(super) free_inodes: Option<u64>,
}

// Filler files only need OS allocation, not survival of a power failure.
// Rust's sync_all AND sync_data request F_FULLFSYNC on Apple targets: using
// either for every empty filler file flushes the device's hardware cache.
// Keep that stronger barrier on authoritative engine files, never here.
pub(super) fn sync_filler_allocation(file: &File) -> std::io::Result<()> {
    #[cfg(target_os = "macos")]
    {
        sync_filler_host(file)
    }
    #[cfg(not(target_os = "macos"))]
    {
        file.sync_data()
    }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
pub(super) fn sync_filler_host(file: &File) -> std::io::Result<()> {
    use std::os::fd::AsRawFd;
    // SAFETY: the borrowed File owns this live descriptor for the whole call.
    // No retry/fallback can turn a native allocation or I/O error into success.
    if unsafe { libc::fsync(file.as_raw_fd()) } == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

// Only disposable pressure files use this schedule. Engine durability is separate.
pub(super) fn synchronize_allocation(
    reason: FillBarrier,
    periodic_file_sync: bool,
    sync_file: impl FnOnce() -> std::io::Result<()>,
    sync_volume: impl FnOnce() -> std::io::Result<()>,
) -> std::io::Result<()> {
    // APFS pressure is settled at the final file + waited-volume barrier.
    // Disposable filler has no intermediate durability requirement. An interval
    // is never a capacity proof; fill_data still demands fresh denial afterward.
    if reason == FillBarrier::Interval && !periodic_file_sync {
        return Ok(());
    }
    let file_result = sync_file();
    match reason {
        FillBarrier::Interval => file_result,
        FillBarrier::Capacity => finish_allocation_barrier(file_result, sync_volume),
    }
}

pub(super) fn finish_allocation_barrier(
    file_result: std::io::Result<()>,
    settle_volume: impl FnOnce() -> std::io::Result<()>,
) -> std::io::Result<()> {
    if let Err(error) = &file_result {
        if !error.raw_os_error().is_some_and(capacity_code) {
            return file_result;
        }
    }
    // ENOSPC from the file barrier must not skip volume settlement. Retain
    // that native error after settlement; a later write remains the oracle.
    settle_volume()?;
    file_result
}

// A capacity error before synchronization can leave pending allocation. Probe
// again after the barrier; only a filler unchanged since that attempt followed by
// a native one-byte failure is ready for the selected engine operation.
pub(super) fn fill_data<W: Write>(
    file: &mut W,
    max_bytes: u64,
    started: Instant,
    synchronize: impl FnMut(&W, FillBarrier) -> std::io::Result<()>,
) -> std::io::Result<(u64, i32)> {
    fill_data_with_probe(file, max_bytes, started, false, synchronize)
}

pub(super) fn fill_data_with_probe<W: Write>(
    file: &mut W,
    max_bytes: u64,
    started: Instant,
    refill: bool,
    mut synchronize: impl FnMut(&W, FillBarrier) -> std::io::Result<()>,
) -> std::io::Result<(u64, i32)> {
    let buffer = [0xa5u8; 64 * 1024];
    let mut bytes = 0;
    let mut bytes_at_barrier = 0;
    let mut barrier_since_write = false;
    // A refill follows a previous capacity denial. Probe the smallest native
    // allocation first; any successful byte immediately restores bulk writes.
    let mut write_size = if refill { 1 } else { buffer.len() };
    let mut writes = 0u64;
    loop {
        assert!(
            started.elapsed() < FILL_BOUND,
            "bounded owned-volume fill: bytes={bytes} writes={writes} request_bytes={write_size} elapsed_ms={}",
            started.elapsed().as_millis()
        );
        assert!(
            bytes < max_bytes,
            "owned filesystem did not enforce its capacity"
        );
        let attempt = write_size.min((max_bytes - bytes) as usize);
        writes += 1;
        match file.write(&buffer[..attempt]) {
            Ok(0) => panic!("zero-length filler write"),
            Ok(n) => {
                bytes += n as u64;
                barrier_since_write = false;
                // A reduced probe can succeed as allocation becomes available.
                // Resume bulk writes instead of draining that space bytewise.
                // Subsequent capacity errors still reduce to the one-byte oracle.
                write_size = buffer.len();
            }
            Err(error) => {
                let Some(code) = error.raw_os_error().filter(|code| capacity_code(*code)) else {
                    return Err(error);
                };
                if attempt > 1 {
                    write_size = (attempt / 16).max(1);
                    continue;
                }
                if barrier_since_write {
                    return Ok((bytes, code));
                }
                // Settle the final partial synchronization interval, then fill
                // any allocation made available by that barrier. No new clock.
                if let Err(error) = synchronize(file, FillBarrier::Capacity) {
                    if !error.raw_os_error().is_some_and(capacity_code) {
                        return Err(error);
                    }
                }
                bytes_at_barrier = bytes;
                barrier_since_write = true;
                write_size = if refill { 1 } else { buffer.len() };
                continue;
            }
        }
        // Partial writes need not land on an exact multiple of the interval.
        if bytes - bytes_at_barrier >= 4 * 1024 * 1024 {
            if let Err(error) = synchronize(file, FillBarrier::Interval) {
                if !error.raw_os_error().is_some_and(capacity_code) {
                    return Err(error);
                }
            }
            bytes_at_barrier = bytes;
            // An interval file barrier is not final waited volume settlement.
            barrier_since_write = false;
        }
    }
}

// Metadata transactions can make allocation available after the initial fill.
// Every subsequent drain shares one aggregate byte allowance and start time.
pub(super) fn fill_remaining_data<W: Write>(
    file: &mut W,
    allocated: &mut u64,
    max_bytes: u64,
    started: Instant,
    synchronize: impl FnMut(&W, FillBarrier) -> std::io::Result<()>,
) -> std::io::Result<i32> {
    let remaining = max_bytes
        .checked_sub(*allocated)
        .expect("filler byte bound");
    let (added, code) = fill_data_with_probe(file, remaining, started, true, synchronize)?;
    *allocated += added;
    Ok(code)
}

// Charge before the native effect, including uncertain/error outcomes. Metadata
// and file contents share the admitted volume's existing payload allowance.
pub(super) fn allocate_metadata_payload(
    charged: &mut u64,
    data_bytes: u64,
    max_bytes: u64,
    payload_bytes: u64,
    allocate: impl FnOnce() -> std::io::Result<()>,
) -> std::io::Result<()> {
    let next = charged
        .checked_add(payload_bytes)
        .expect("metadata byte overflow");
    assert!(
        data_bytes
            .checked_add(next)
            .is_some_and(|total| total <= max_bytes),
        "metadata payload exceeds aggregate filler byte bound"
    );
    *charged = next;
    allocate()
}

// One category can release allocation usable by the other. Admission to the
// selected engine operation requires a whole pressure round without progress.
pub(super) fn settle_pressure(
    started: Instant,
    mut previous: (u64, usize),
    mut round: impl FnMut() -> (u64, usize),
) {
    loop {
        assert!(
            started.elapsed() < FILL_BOUND,
            "bounded joint pressure: bytes={} files={} elapsed_ms={}",
            previous.0,
            previous.1,
            started.elapsed().as_millis()
        );
        let current = round();
        assert!(
            current.0 >= previous.0 && current.1 >= previous.1,
            "owned pressure accounting must be monotonic"
        );
        if current == previous {
            return;
        }
        previous = current;
    }
}
