//! Real native capacity corpus. Only the owned-image runner supplies admission.
use super::*;
use serde::Deserialize;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};

#[path = "capacity_observation.rs"]
mod observation;

#[path = "capacity_metadata.rs"]
mod metadata;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FillBarrier {
    Interval,
    Capacity,
}

#[cfg(target_os = "macos")]
#[path = "capacity_apfs.rs"]
mod apfs;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Volume {
    version: u16,
    root: PathBuf,
    nonce: String,
    filesystem: String,
    max_bytes: u64,
}

impl Volume {
    fn admitted() -> Self {
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

struct Filler {
    file: Option<File>,
    path: PathBuf,
    bytes: u64,
    native_code: i32,
    data_refills: u32,
    metadata_payload_bytes: u64,
    metadata_paths: Vec<PathBuf>,
    metadata_directories: Vec<PathBuf>,
    metadata_native_code: Option<i32>,
    free_inodes: Option<u64>,
}

// Filler files only need OS allocation, not survival of a power failure.
// Rust's sync_all AND sync_data request F_FULLFSYNC on Apple targets: using
// either for every empty filler file flushes the device's hardware cache.
// Keep that stronger barrier on authoritative engine files, never here.
fn sync_filler_allocation(file: &File) -> std::io::Result<()> {
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
fn sync_filler_host(file: &File) -> std::io::Result<()> {
    use std::os::fd::AsRawFd;
    // SAFETY: the borrowed File owns this live descriptor for the whole call.
    // No retry/fallback can turn a native allocation or I/O error into success.
    if unsafe { libc::fsync(file.as_raw_fd()) } == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

fn finish_allocation_barrier(
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
fn fill_data<W: Write>(
    file: &mut W,
    max_bytes: u64,
    started: Instant,
    mut synchronize: impl FnMut(&W, FillBarrier) -> std::io::Result<()>,
) -> std::io::Result<(u64, i32)> {
    let buffer = [0xa5u8; 64 * 1024];
    let mut bytes = 0;
    let mut bytes_at_barrier = 0;
    let mut barrier_since_write = false;
    let mut write_size = buffer.len();
    let mut writes = 0u64;
    loop {
        assert!(
            started.elapsed() < Duration::from_secs(25),
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
                write_size = buffer.len();
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
fn fill_remaining_data<W: Write>(
    file: &mut W,
    allocated: &mut u64,
    max_bytes: u64,
    started: Instant,
    synchronize: impl FnMut(&W, FillBarrier) -> std::io::Result<()>,
) -> std::io::Result<i32> {
    let remaining = max_bytes
        .checked_sub(*allocated)
        .expect("filler byte bound");
    let (added, code) = fill_data(file, remaining, started, synchronize)?;
    *allocated += added;
    Ok(code)
}

// Charge before the native effect, including uncertain/error outcomes. Metadata
// and file contents share the admitted volume's existing payload allowance.
fn allocate_metadata_payload(
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
fn settle_pressure(
    started: Instant,
    mut previous: (u64, usize),
    mut round: impl FnMut() -> (u64, usize),
) {
    loop {
        assert!(
            started.elapsed() < Duration::from_secs(25),
            "bounded joint pressure"
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

impl Filler {
    fn fill(volume: &Volume, metadata_directory: Option<&std::path::Path>) -> Self {
        let directory = metadata_directory.map(|path| {
            assert!(!std::fs::symlink_metadata(path)
                .unwrap()
                .file_type()
                .is_symlink());
            let path = std::fs::canonicalize(path).unwrap();
            assert!(
                path.starts_with(&volume.root) && path != volume.root,
                "metadata fill must remain inside the owned estate volume"
            );
            path
        });
        let path = volume.root.join(format!("{}.filler", volume.nonce));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .unwrap();
        let mut filler = Self {
            file: None,
            path,
            bytes: 0,
            native_code: 0,
            data_refills: 0,
            metadata_payload_bytes: 0,
            metadata_paths: Vec::new(),
            metadata_directories: Vec::new(),
            metadata_native_code: None,
            free_inodes: None,
        };
        // Nonzero data prevents sparse/zero-block optimizations. Allocation and
        // sync failures are actual OS results, never injected StorageFull.
        let started = Instant::now();
        // Open only the already admitted, private image root. File fsync alone
        // does not wait for the volume's metadata transaction to settle.
        #[cfg(target_os = "macos")]
        let volume_root = File::open(&volume.root).expect("owned volume root");
        let interval_barriers = std::cell::Cell::new(0_u64);
        let capacity_barriers = std::cell::Cell::new(0_u64);
        let barrier_us = std::cell::Cell::new(0_u128);
        let allocation_sync = |file: &File, reason: FillBarrier| {
            let began = Instant::now();
            let file_result = sync_filler_allocation(file);
            let result = match reason {
                FillBarrier::Interval => {
                    interval_barriers.set(interval_barriers.get() + 1);
                    file_result
                }
                FillBarrier::Capacity => {
                    capacity_barriers.set(capacity_barriers.get() + 1);
                    finish_allocation_barrier(file_result, || {
                        #[cfg(target_os = "macos")]
                        apfs::sync_volume(&volume_root)?;
                        Ok(())
                    })
                }
            };
            barrier_us.set(barrier_us.get() + began.elapsed().as_micros());
            result
        };
        // A full NTFS directory index does not prove that free MFT records are
        // exhausted: a small CURRENT file may still be resident. Reserve a
        // fixed set of empty, fixture-owned directory indexes before filling
        // data, then consume their spare records without touching engine files.
        #[cfg(windows)]
        if directory.is_some() {
            for index in 0..16 {
                let path = volume.root.join(format!("{}-m{index:02}", volume.nonce));
                std::fs::create_dir(&path).unwrap();
                filler.metadata_directories.push(path);
            }
        }
        // Ext4 directory blocks may have filename-dependent spare slots. Exhaust
        // inodes while data space remains, then fill data through the already
        // open filler. A free-inode observation of zero proves create denial
        // independent of the selected publication filename.
        #[cfg(target_os = "linux")]
        if let Some(directory) = &directory {
            filler.fill_metadata(volume, directory, started, &allocation_sync);
            use std::os::fd::AsRawFd;
            let handle = File::open(directory).unwrap();
            let mut state = std::mem::MaybeUninit::<libc::statvfs>::uninit();
            // SAFETY: valid owned directory fd and writable statvfs storage.
            assert_eq!(
                unsafe { libc::fstatvfs(handle.as_raw_fd(), state.as_mut_ptr()) },
                0
            );
            // SAFETY: successful fstatvfs initialized the value.
            let state = unsafe { state.assume_init() };
            assert_eq!(
                state.f_ffree, 0,
                "metadata error did not exhaust ext4 inodes"
            );
            filler.free_inodes = Some(state.f_ffree);
        }
        let data_started = Instant::now();
        (filler.bytes, filler.native_code) =
            fill_data(&mut file, volume.max_bytes, started, &allocation_sync)
                .expect("native filler allocation");
        eprintln!("native_capacity_data_phase={{\"bytes\":{},\"elapsed_ms\":{},\"interval_barriers\":{},\"capacity_barriers\":{},\"barrier_us\":{}}}", filler.bytes, data_started.elapsed().as_millis(), interval_barriers.get(), capacity_barriers.get(), barrier_us.get());
        filler.file = Some(file);
        if cfg!(target_os = "macos") {
            let directory = directory
                .as_ref()
                .expect("APFS metadata pressure directory");
            // Either category can release allocation usable by the other. A
            // single final drain is insufficient: finish a whole round with no
            // additional files or data before resuming the selected operation.
            settle_pressure(started, (filler.bytes, filler.metadata_paths.len()), || {
                filler.fill_metadata(volume, directory, started, &allocation_sync);
                filler.native_code = fill_remaining_data(
                    filler.file.as_mut().expect("data filler remains owned"),
                    &mut filler.bytes,
                    volume
                        .max_bytes
                        .checked_sub(filler.metadata_payload_bytes)
                        .expect("metadata payload byte bound"),
                    started,
                    &allocation_sync,
                )
                .expect("native joint metadata-phase data allocation");
                filler.data_refills += 1;
                (filler.bytes, filler.metadata_paths.len())
            });
        } else if !cfg!(target_os = "linux") {
            if let Some(directory) = &directory {
                filler.fill_metadata(volume, directory, started, &allocation_sync);
            }
        }
        // Each empty index can consume otherwise reusable MFT records. A
        // failure in one directory must not end the global metadata fill.
        for directory in filler.metadata_directories.clone() {
            filler.fill_metadata(volume, &directory, started, &allocation_sync);
        }
        filler
    }
    fn fill_metadata(
        &mut self,
        volume: &Volume,
        directory: &std::path::Path,
        started: Instant,
        allocation_sync: &impl Fn(&File, FillBarrier) -> std::io::Result<()>,
    ) {
        self.metadata_native_code = None;
        let code = metadata::fill(
            self,
            started,
            |filler| filler.metadata_paths.len(),
            |filler, index| {
                let path = directory.join(format!("c{}-{index:04x}", &volume.nonce[..8]));
                let file = OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&path)?;
                // Register before attribute allocation so every created entry
                // remains owned and is cleaned even if the attribute fails.
                filler.metadata_paths.push(path);
                #[cfg(target_os = "macos")]
                if let Err(error) = allocate_metadata_payload(
                    &mut filler.metadata_payload_bytes,
                    filler.bytes,
                    volume.max_bytes,
                    apfs::METADATA_PAYLOAD_BYTES as u64,
                    || apfs::allocate_metadata(&file),
                ) {
                    // Attribute exhaustion cannot replace native create denial.
                    if !error.raw_os_error().is_some_and(capacity_code) {
                        return Err(error);
                    }
                }
                drop(file);
                Ok(())
            },
            |filler| {
                if cfg!(target_os = "macos") {
                    // Final data pressure waits for the admitted volume's dirty
                    // metadata too. Batch filler creates; never fsync each one.
                    filler.native_code = fill_remaining_data(
                        filler.file.as_mut().expect("data filler remains owned"),
                        &mut filler.bytes,
                        volume
                            .max_bytes
                            .checked_sub(filler.metadata_payload_bytes)
                            .expect("metadata payload byte bound"),
                        started,
                        allocation_sync,
                    )?;
                    filler.data_refills += 1;
                }
                Ok(())
            },
        )
        .expect("native metadata allocation and settlement");
        self.metadata_native_code = Some(code);
    }
}

#[test]
fn filler_allocation_sync_preserves_regular_file_contents() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("allocation");
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .unwrap();
    let payload = [0xa5; 8192];
    file.write_all(&payload).unwrap();
    sync_filler_allocation(&file).unwrap();
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    sync_filler_host(&file).unwrap();
    drop(file);
    assert_eq!(std::fs::read(path).unwrap(), payload);
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn filler_host_sync_preserves_native_noncapacity_errors() {
    // A live socket descriptor is valid but cannot synchronize file
    // allocation. Exercise the actual syscall, not an injected capacity error.
    let (socket, _peer) = std::os::unix::net::UnixStream::pair().unwrap();
    let descriptor: std::os::fd::OwnedFd = socket.into();
    let file = File::from(descriptor);
    let error = sync_filler_host(&file).unwrap_err();
    assert_eq!(error.raw_os_error(), Some(libc::EINVAL));
    assert!(!capacity_code(error.raw_os_error().unwrap()));
}

mod filler_tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn successful_small_write_recovers_bulk_allocation_without_bytewise_drain() {
        struct ReopenedAllocation {
            written: usize,
            calls: usize,
        }
        const AVAILABLE: usize = 1024 * 1024 + 1;
        impl Write for ReopenedAllocation {
            fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                self.calls += 1;
                assert!(
                    self.calls < 128,
                    "available allocation drained byte by byte"
                );
                if self.written == AVAILABLE || (self.written == 0 && bytes.len() > 1) {
                    return Err(std::io::Error::from_raw_os_error(if cfg!(windows) {
                        112
                    } else {
                        28
                    }));
                }
                let count = bytes.len().min(AVAILABLE - self.written);
                self.written += count;
                Ok(count)
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let mut file = ReopenedAllocation {
            written: 0,
            calls: 0,
        };
        let mut syncs = 0;
        let (bytes, code) = fill_data(&mut file, 2 * 1024 * 1024, Instant::now(), |_, _| {
            syncs += 1;
            Ok(())
        })
        .unwrap();
        assert_eq!(bytes, AVAILABLE as u64);
        assert_eq!(syncs, 1);
        assert!(capacity_code(code));
    }

    #[test]
    fn settle_pressure_repeats_after_either_category_allocates() {
        let mut calls = 0;
        let states = [(8, 1), (8, 2), (12, 2), (12, 2)];
        settle_pressure(Instant::now(), (0, 0), || {
            let state = states[calls];
            calls += 1;
            state
        });
        assert_eq!(
            calls,
            states.len(),
            "stopped before both categories settled"
        );
    }

    #[test]
    fn settle_pressure_stable_state_stops_and_expired_clock_has_no_effect() {
        let calls = Cell::new(0);
        settle_pressure(Instant::now(), (8, 2), || {
            calls.set(calls.get() + 1);
            (8, 2)
        });
        assert_eq!(calls.get(), 1);
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            settle_pressure(Instant::now() - Duration::from_secs(25), (8, 2), || {
                calls.set(calls.get() + 1);
                (8, 2)
            });
        }))
        .is_err());
        assert_eq!(
            calls.get(),
            1,
            "expired original clock allowed a new effect"
        );
    }

    #[test]
    fn metadata_payload_is_charged_before_effect_and_reduces_data_allowance() {
        let mut charged = 0;
        let calls = Cell::new(0);
        for code in [28, 5] {
            let error = allocate_metadata_payload(&mut charged, 8, 16, 3, || {
                calls.set(calls.get() + 1);
                Err(std::io::Error::from_raw_os_error(code))
            })
            .unwrap_err();
            assert_eq!(error.raw_os_error(), Some(code));
        }
        assert_eq!(charged, 6, "failed native effects retain their allowance");
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = allocate_metadata_payload(&mut charged, 8, 16, 3, || {
                calls.set(calls.get() + 1);
                Ok(())
            });
        }))
        .is_err());
        assert_eq!(calls.get(), 2, "over-budget metadata must not reach the OS");
        assert_eq!(charged, 6);
        let mut file = DelayedAllocation::new();
        let mut data_bytes = 8;
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = fill_remaining_data(
                &mut file,
                &mut data_bytes,
                16 - charged,
                Instant::now(),
                |_, _| Ok(()),
            );
        }))
        .is_err());
        assert_eq!(
            file.remaining.get(),
            6,
            "data refill ignored metadata charge"
        );
    }

    #[test]
    fn capacity_file_error_still_settles_volume_and_preserves_errors() {
        let capacity = if cfg!(windows) { 112 } else { 28 };
        for file_code in [None, Some(capacity), Some(5)] {
            for volume_code in [None, Some(capacity), Some(5)] {
                let calls = Cell::new(0);
                let result = finish_allocation_barrier(
                    file_code.map_or(Ok(()), |code| Err(std::io::Error::from_raw_os_error(code))),
                    || {
                        calls.set(calls.get() + 1);
                        volume_code
                            .map_or(Ok(()), |code| Err(std::io::Error::from_raw_os_error(code)))
                    },
                );
                assert_eq!(calls.get(), u32::from(file_code != Some(5)));
                let expected = if file_code == Some(5) {
                    file_code
                } else {
                    volume_code.or(file_code)
                };
                assert_eq!(
                    result.err().and_then(|error| error.raw_os_error()),
                    expected
                );
            }
        }
    }

    struct DelayedAllocation {
        remaining: Cell<usize>,
        syncs: Cell<usize>,
        write_error: i32,
    }
    impl DelayedAllocation {
        fn new() -> Self {
            Self {
                remaining: Cell::new(8),
                syncs: Cell::new(0),
                write_error: if cfg!(windows) { 112 } else { 28 },
            }
        }
    }
    impl Write for DelayedAllocation {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            let n = bytes.len().min(self.remaining.get());
            if n == 0 {
                return Err(std::io::Error::from_raw_os_error(self.write_error));
            }
            self.remaining.set(self.remaining.get() - n);
            Ok(n)
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn metadata_refills_share_the_original_byte_and_time_bounds() {
        let mut file = DelayedAllocation::new();
        let mut allocated = 0;
        let started = Instant::now();
        let code =
            fill_remaining_data(&mut file, &mut allocated, 20, started, |_, _| Ok(())).unwrap();
        assert!(capacity_code(code));
        assert_eq!(allocated, 8);
        file.remaining.set(8); // Allocation released by a later metadata transaction.
        fill_remaining_data(&mut file, &mut allocated, 20, started, |_, _| Ok(())).unwrap();
        assert_eq!(allocated, 16);
        file.remaining.set(8);
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = fill_remaining_data(&mut file, &mut allocated, 20, started, |_, _| Ok(()));
        }))
        .is_err());
        assert_eq!(
            file.remaining.get(),
            4,
            "refill exceeded cumulative allowance"
        );
        let expired = Instant::now() - Duration::from_secs(25);
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = fill_remaining_data(&mut file, &mut allocated, 20, expired, |_, _| {
                panic!("expired refill synchronized")
            });
        }))
        .is_err());
        assert_eq!(file.remaining.get(), 4, "refill restarted the clock");
    }

    #[test]
    fn exact_interval_still_requires_capacity_settlement_and_fresh_denial() {
        const INTERVAL: usize = 4 * 1024 * 1024;
        let mut file = DelayedAllocation::new();
        file.remaining.set(INTERVAL);
        let mut reasons = Vec::new();
        let (bytes, code) = fill_data(
            &mut file,
            (INTERVAL + 1024) as u64,
            Instant::now(),
            |file, reason| {
                reasons.push(reason);
                if reason == FillBarrier::Capacity && file.syncs.get() == 0 {
                    file.remaining.set(16);
                    file.syncs.set(1);
                }
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(bytes, (INTERVAL + 16) as u64);
        assert_eq!(file.remaining.get(), 0);
        assert_eq!(
            reasons,
            [
                FillBarrier::Interval,
                FillBarrier::Capacity,
                FillBarrier::Capacity
            ]
        );
        assert!(capacity_code(code));
    }

    #[test]
    fn settles_pending_allocation_before_declaring_full() {
        // Both a successful barrier and delayed allocation failure can change
        // what the next write observes. Neither alone proves settled capacity.
        for delayed_error in [false, true] {
            let mut file = DelayedAllocation::new();
            let (bytes, code) = fill_data(&mut file, 1024, Instant::now(), |file, _| {
                let first = file.syncs.get() == 0;
                if first {
                    file.remaining.set(16);
                }
                file.syncs.set(file.syncs.get() + 1);
                if first && delayed_error {
                    Err(std::io::Error::from_raw_os_error(file.write_error))
                } else {
                    Ok(())
                }
            })
            .unwrap();
            assert_eq!(
                bytes, 24,
                "must consume allocation made available by the final sync"
            );
            assert_eq!(file.remaining.get(), 0);
            assert_eq!(file.syncs.get(), 2);
            assert!(capacity_code(code));
        }
    }

    #[test]
    fn preserves_noncapacity_write_and_sync_errors() {
        for fail_write in [false, true] {
            let mut file = DelayedAllocation::new();
            if fail_write {
                file.write_error = 5;
            }
            let error = fill_data(&mut file, 1024, Instant::now(), |_, _| {
                assert!(
                    !fail_write,
                    "unexpected write error must not trigger sync/retry"
                );
                Err(std::io::Error::from_raw_os_error(5))
            })
            .unwrap_err();
            assert_eq!(error.raw_os_error(), Some(5));
            assert!(!capacity_code(5));
        }
    }

    #[test]
    fn cannot_exceed_byte_cap_or_restart_expired_deadline() {
        let mut sink = Vec::new();
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = fill_data(&mut sink, 13, Instant::now(), |_, _| Ok(()));
        }))
        .is_err());
        assert_eq!(sink.len(), 13);
        let expired = Instant::now() - Duration::from_secs(25);
        let mut file = DelayedAllocation::new();
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = fill_data(&mut file, 1024, expired, |_, _| {
                panic!("expired fill synchronized")
            });
        }))
        .is_err());
        assert_eq!(file.remaining.get(), 8, "expired fill wrote data");
    }
}

impl Drop for Filler {
    fn drop(&mut self) {
        self.file.take();
        for path in &self.metadata_paths {
            if let Err(error) = std::fs::remove_file(path) {
                if !std::thread::panicking() {
                    panic!("owned metadata filler cleanup failed: {error}");
                }
            }
        }
        for path in &self.metadata_directories {
            if let Err(error) = std::fs::remove_dir(path) {
                if !std::thread::panicking() {
                    panic!("owned empty metadata directory cleanup failed: {error}");
                }
            }
        }
        // Exact create_new-owned file only; never recurse or remove estate data.
        if let Err(error) = std::fs::remove_file(&self.path) {
            if !std::thread::panicking() {
                panic!("owned filler cleanup failed: {error}");
            }
        }
    }
}
fn capacity_code(code: i32) -> bool {
    if cfg!(windows) {
        matches!(code, 39 | 112)
    } else if cfg!(target_os = "macos") {
        matches!(code, 28 | 69)
    } else {
        matches!(code, 28 | 122)
    }
}

fn resume_after_fill(
    mut child: Child,
    control: Value,
    volume: Volume,
    metadata_directory: Option<PathBuf>,
    wal_root: PathBuf,
) -> (Child, Filler, Value) {
    let line = child.line();
    let hit: Value = serde_json::from_str(
        std::str::from_utf8(&line)
            .unwrap()
            .trim()
            .strip_prefix(PREFIX)
            .unwrap(),
    )
    .unwrap();
    assert_eq!(hit["pid"], child.child.id());
    for key in ["nonce", "id", "point"] {
        assert_eq!(hit[key], control[key]);
    }
    println!("native_capacity_pause={hit}");
    println!(
        "native_capacity_wal_before_fill={}",
        observation::wal_state(&wal_root)
    );
    let filler = Filler::fill(&volume, metadata_directory.as_deref());
    println!(
        "native_capacity_fill={}",
        json!({"point":control["point"],
        "bytes":filler.bytes,"metadata_payload_bytes":filler.metadata_payload_bytes,"native_code":filler.native_code,"data_refills":filler.data_refills,
        "metadata_files":filler.metadata_paths.len(),
        "metadata_native_code":filler.metadata_native_code,"free_inodes":filler.free_inodes})
    );
    println!(
        "native_capacity_wal_after_fill={}",
        observation::wal_state(&wal_root)
    );
    child
        .child
        .resume_controlled(
            &serde_json::to_vec(&json!({"version":1,
        "nonce":control["nonce"],"id":control["id"]}))
            .unwrap(),
        )
        .unwrap();
    let line = match child.lines.recv_timeout(Duration::from_secs(60)) {
        Ok(Ok(line)) => line,
        failure => {
            println!(
                "native_capacity_wal_missing_event={}",
                observation::wal_state(&wal_root)
            );
            child.child.terminate();
            let output = child.child.finish().expect("reap failed capacity engine");
            let tail = |bytes: &[u8]| {
                String::from_utf8_lossy(&bytes[bytes.len().saturating_sub(2048)..]).into_owned()
            };
            panic!("selected native capacity event missing: {failure:?}; exit={}; stdout_tail={}; stderr_tail={}",
                output.status, tail(&output.stdout), tail(&output.stderr));
        }
    };
    let error: Value = serde_json::from_str(
        std::str::from_utf8(&line)
            .unwrap()
            .trim()
            .strip_prefix("C07F_NATIVE_IO_V1=")
            .expect("selected native I/O must actually fail"),
    )
    .unwrap();
    for key in ["pid", "nonce", "id", "point"] {
        assert_eq!(error[key], hit[key]);
    }
    assert!(
        capacity_code(error["native_code"].as_i64().unwrap() as i32),
        "wrong native error: {error}"
    );
    let expected_role = match control["point"].as_str().unwrap() {
        "wal_before_write" => "wal",
        "segment_before_create" => "segment",
        "manifest_before_create" => "manifest",
        _ => "publication",
    };
    assert_eq!(error["role"], expected_role);
    (child, filler, json!({"hit":hit,"error":error}))
}

async fn read_values(
    client: &RrdClient,
    session: &rrd_client::Session,
    at: u64,
) -> BTreeMap<String, String> {
    // Enumerate the complete identity set, then fetch bounded object windows at
    // that exact read cursor. Native capacity payloads exceed a single public
    // response; raising the contract's response cap would hide the boundary.
    let query = |text| ExecuteQuery {
        scope: format!("instance:{}", client.instance_id()),
        query: text,
        parameters: BTreeMap::new(),
        budget: QueryBudget {
            max_storage_keys: 100_000,
            max_rows: 1024,
            max_output_bytes: 512 * 1024,
            max_batch_rows: 64,
            max_memory_bytes: 64 * 1024 * 1024,
            max_spill_bytes: 64 * 1024 * 1024,
            max_elapsed_ms: 30_000,
        },
    };
    let index = client
        .execute_query(
            session,
            query(format!(
                "FROM claim:crash AT VALID {at} KNOWN HEAD PROJECT subject EXPLAIN CONTRACT"
            )),
            RequestOptions::read("capacity-index", "capacity-index").unwrap(),
        )
        .await
        .unwrap();
    assert!(!index.execution.truncated);
    let cursor = index.known_at_cursor;
    let mut subjects = index
        .rows
        .iter()
        .map(|row| {
            let QueryValue::String(subject) = &row.values["subject"] else {
                panic!("non-string subject")
            };
            subject.clone()
        })
        .collect::<Vec<_>>();
    subjects.sort();
    subjects.dedup();
    assert_eq!(subjects.len(), index.rows.len(), "duplicate claim subject");
    let mut values = BTreeMap::new();
    for window in subjects.chunks(64) {
        let first = serde_json::to_string(window.first().unwrap()).unwrap();
        let last = serde_json::to_string(window.last().unwrap()).unwrap();
        let result = client.execute_query(session, query(format!(
            "FROM claim:crash AT VALID {at} KNOWN {cursor} WHERE subject >= {first} AND subject <= {last} PROJECT subject, object EXPLAIN CONTRACT")),
            RequestOptions::read("capacity-window","capacity-window").unwrap()).await.unwrap();
        assert!(!result.execution.truncated);
        assert_eq!(result.known_at_cursor, cursor);
        assert_eq!(result.rows.len(), window.len());
        for row in result.rows {
            let QueryValue::String(value) = &row.values["object"] else {
                panic!("non-string claim")
            };
            assert!(values.insert(row.identity, value.clone()).is_none());
        }
    }
    assert_eq!(cursor, values.len() as u64 + 2);
    values
}

fn requested_values(request: &CommitTransaction) -> BTreeMap<String, String> {
    request
        .mutations
        .iter()
        .map(|m| {
            let TransactionMutation::AssertClaim {
                subject,
                predicate,
                object,
                ..
            } = m
            else {
                panic!("claim corpus")
            };
            (format!("claim:{subject}:{predicate}"), object.clone())
        })
        .collect()
}

async fn begin(
    client: &RrdClient,
    session: &rrd_client::Session,
    name: &str,
) -> rrd_contract::TransactionLease {
    client
        .begin_transaction(
            session,
            BeginTransaction {
                scope: CanonicalId::new("claims").unwrap(),
                timeout_ms: 600_000,
            },
            options(&format!("{name}-begin")),
        )
        .await
        .unwrap()
}

fn wal_bytes(estate: &Estate) -> u64 {
    let locator = RrdEngine::read_project_locator(&estate.project).unwrap();
    let root = estate.project.join(locator.storage_root).join("wal");
    std::fs::read_dir(root)
        .unwrap()
        .map(|entry| entry.unwrap().metadata().unwrap().len())
        .max()
        .unwrap()
}

fn metadata_pressure_directory(
    point: &str,
    native_root: &std::path::Path,
    filesystem: &str,
) -> Option<PathBuf> {
    match point {
        // APFS can deny growth of the data filler while another existing file
        // still allocates. Consume metadata pressure outside the WAL directory,
        // where disposable entries must never be mistaken for authoritative WAL.
        "wal_before_write" if filesystem == "apfs" => Some(native_root.to_path_buf()),
        "wal_before_write" => None,
        "segment_before_create" => Some(native_root.join("segments")),
        "manifest_before_create" => Some(native_root.join("manifests")),
        "current_before_create" => Some(native_root.to_path_buf()),
        _ => panic!("unsupported native capacity point"),
    }
}

#[test]
fn apfs_wal_consumes_metadata_pressure_outside_authoritative_wal_directory() {
    let estate = tempfile::tempdir().unwrap();
    let root = estate.path();
    // A failed write to one APFS filler file did not prevent another existing
    // file from allocating. WAL pressure must include the metadata phase too.
    assert_eq!(
        metadata_pressure_directory("wal_before_write", root, "apfs"),
        Some(root.to_path_buf())
    );
    for filesystem in ["ext4", "ntfs"] {
        assert_eq!(
            metadata_pressure_directory("wal_before_write", root, filesystem),
            None
        );
    }
    for filesystem in ["apfs", "ext4", "ntfs"] {
        for (point, directory) in [
            ("segment_before_create", root.join("segments")),
            ("manifest_before_create", root.join("manifests")),
            ("current_before_create", root.to_path_buf()),
        ] {
            assert_eq!(
                metadata_pressure_directory(point, root, filesystem),
                Some(directory)
            );
        }
    }
}

fn capacity_case(point: &str, mode: &str) {
    let volume = Volume::admitted();
    let estate = Estate::install_at(mode, Some(&volume.root), point != "wal_before_write");
    let locator = RrdEngine::read_project_locator(&estate.project).unwrap();
    let native_root = estate.project.join(locator.storage_root);
    let metadata_directory = metadata_pressure_directory(point, &native_root, &volume.filesystem);
    let credential = RrdEngine::read_installed_api_key(&estate.project).unwrap();
    let mut ledger = AcknowledgementLedger::default();
    let clock = Instant::now();
    tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
        let (child, client) = estate.serve(&Value::Null);
        let session = client.create_session(credential.principal_id.clone(),credential.credential(),
            CreateSession {limits:SessionLimits {idle_timeout_ms:600_000,absolute_timeout_ms:3_600_000,max_open_transactions:2}},
            options("capacity-session")).await.unwrap();
        let mut values = BTreeMap::new();
        let mut acknowledged = Vec::new();
        // Consume the native WAL's initial preallocation with real acknowledged
        // operations. No direct file growth or sparse-file simulation is allowed.
        for index in 0..256 {
            let name = format!("prior-{index}");
            let mut request = request(&name, estate.at);
            for mutation in &mut request.mutations {
                if let TransactionMutation::AssertClaim {object,..} = mutation { *object = format!("{name}:").repeat(400); object.truncate(4096); }
            }
            request.operation_sha256 = transaction_operation_sha256(&request.mutations);
            let expected = requested_values(&request);
            let lease = begin(&client,&session,&name).await;
            let started = clock.elapsed().as_nanos() as u64;
            let receipt = client.commit_transaction(&session,&lease.transaction_id,request.clone(),options(&name)).await.unwrap();
            record(&mut ledger, &name, &request, &expected, &receipt, started, &clock);
            values.extend(expected);
            acknowledged.push((name,request,lease,receipt_hash(&receipt)));
            if point != "wal_before_write" || wal_bytes(&estate) > 2 * 1024 * 1024 { break; }
            assert!(index < 255,"failed to consume initial WAL allocation within corpus bound");
        }
        assert_eq!(read_values(&client,&session,estate.at).await,values);
        child.kill();
        let mutations = (0..180).map(|index| TransactionMutation::AssertClaim {
            subject:CanonicalId::new(format!("crash-uncertain-{index}")).unwrap(),
            predicate:CanonicalId::new("crash").unwrap(), object:format!("{index:04}").repeat(1024),
            valid_from:estate.at,tx_time:estate.at,producer:CanonicalId::new("independent-oracle").unwrap(),confidence:None,
        }).collect::<Vec<_>>();
        let target = CommitTransaction {operation_sha256:transaction_operation_sha256(&mutations),mutations};
        assert!(serde_json::to_vec(&target).unwrap().len() < 1024 * 1024);
        let target_values = requested_values(&target);
        ledger.expected.push(ExpectedOperation {logical_id:"uncertain".to_owned().try_into().unwrap(),
            request_sha256:digest("c07f.request",&target).unwrap(),effect_sha256:digest("c07f.effect",&target_values).unwrap(),
            resolution:ExpectedResolution::Present});
        let mut control = estate.control(point,1,Some(&target.operation_sha256));
        control["resume"] = json!(true);
        let (child,client) = estate.serve(&control);
        let lease = begin(&client,&session,"uncertain").await;
        let started = clock.elapsed().as_nanos() as u64;
        let controller = tokio::task::spawn_blocking(move ||resume_after_fill(child,control,volume,metadata_directory,native_root.join("wal")));
        let result = client.commit_transaction(&session,&lease.transaction_id,target.clone(),options("uncertain")).await;
        let result_class = match &result {
            Ok(_) => "acknowledged".to_owned(),
            Err(rrd_client::Error::Api { error, .. }) => format!("api-{:?}",error.code),
            Err(rrd_client::Error::Timeout) => "client-timeout".to_owned(),
            Err(_) => "client-failure".to_owned(),
        };
        let message = match &result {
            Err(rrd_client::Error::Api {error,..}) => Some(error.message.chars().take(512).collect::<String>()),
            _ => None,
        };
        println!("native_capacity_client={}",json!({"point":point,"mode":mode,"class":result_class,"message":message}));
        let (child,filler,native) = controller.await.unwrap();
        assert!(result.is_err(),"capacity operation unexpectedly acknowledged");
        child.kill();
        let fill = json!({"bytes":filler.bytes,"metadata_payload_bytes":filler.metadata_payload_bytes,"native_code":filler.native_code,"data_refills":filler.data_refills,
            "metadata_files":filler.metadata_paths.len(),"metadata_native_code":filler.metadata_native_code,"free_inodes":filler.free_inodes});
        drop(filler);
        ledger.attempts.push(Attempt {attempt_id:"capacity-uncertain".to_owned().try_into().unwrap(),
            logical_id:"uncertain".to_owned().try_into().unwrap(),request_sha256:digest("c07f.request",&target).unwrap(),
            started_ns:started,finished_ns:clock.elapsed().as_nanos() as u64,
            outcome:AttemptOutcome::Uncertain {reason:"native-capacity-error".to_owned().try_into().unwrap()}});
        // Ordinary verify cannot silently repair. Only a positively classified
        // tail yields a plan; any other failure remains fatal on serve/re-read.
        let mut command = estate.command();
        command.args(["repair","plan","--project"]).arg(&estate.project);
        let output = installed_command(&mut command,CommandPhase::RepairPlan,Duration::from_secs(120),8*LIMIT)
            .unwrap_or_else(|error| panic!("{error}"));
        let repair_receipt = if output.status.success() {
            let plan:Value = serde_json::from_slice(&output.stdout).unwrap();
            let path = estate.temp.path().join("repair.json");
            std::fs::write(&path,&output.stdout).unwrap();
            Some(estate.invoke(&["repair","apply","--project",estate.project.to_str().unwrap(),
                "--plan",path.to_str().unwrap(),"--expect",plan["plan_sha256"].as_str().unwrap()]))
        } else { None };
        let (child,client) = estate.serve(&Value::Null);
        let after = read_values(&client,&session,estate.at).await;
        let mut with_target = values.clone(); with_target.extend(target_values.clone());
        assert!(after == values || after == with_target,"prior ACK lost or non-atomic target after capacity recovery");
        // Resolve the unknown attempt via the original request identity, then
        // prove receipt stability for every prior acknowledged logical call.
        let retry_started = clock.elapsed().as_nanos() as u64;
        let receipt = client.commit_transaction(&session,&lease.transaction_id,target.clone(),options("uncertain")).await.unwrap();
        record(&mut ledger,"uncertain",&target,&target_values,&receipt,retry_started,&clock);
        values = with_target;
        for (name,request,lease,hash) in acknowledged {
            let receipt = client.commit_transaction(&session,&lease.transaction_id,request,options(&name)).await.unwrap();
            assert!(receipt.idempotent_replay); assert_eq!(receipt_hash(&receipt),hash);
        }
        let last = request("after",estate.at);
        let lease = begin(&client,&session,"after").await;
        let started = clock.elapsed().as_nanos() as u64;
        let receipt = client.commit_transaction(&session,&lease.transaction_id,last.clone(),options("after")).await.unwrap();
        record(&mut ledger,"after",&last,&expected("after"),&receipt,started,&clock);
        values.extend(expected("after"));
        assert_eq!(read_values(&client,&session,estate.at).await,values);
        child.kill();
        let (child,client) = estate.serve(&Value::Null);
        assert_eq!(read_values(&client,&session,estate.at).await,values);
        child.kill();
        let report = ledger.reconcile(1024, 1_800_000_000_000).unwrap();
        assert!(report.violations.is_empty());
        println!("\nnative_capacity_receipt={}",json!({"version":1,"mode":mode,"point":point,
            "build":estate.build,"native":native,"filler":fill,"repair":repair_receipt,
            "expected_sha256":digest("c07f.values",&values).unwrap(),"ledger":ledger,"report":report}));
    });
}

fn record(
    ledger: &mut AcknowledgementLedger,
    name: &str,
    request: &CommitTransaction,
    values: &BTreeMap<String, String>,
    receipt: &CommitReceipt,
    started: u64,
    clock: &Instant,
) {
    let logical_id = name.to_owned().try_into().unwrap();
    let request_sha256 = digest("c07f.request", request).unwrap();
    let effect_sha256 = digest("c07f.effect", values).unwrap();
    if !ledger
        .expected
        .iter()
        .any(|item| item.logical_id == logical_id)
    {
        ledger.expected.push(ExpectedOperation {
            logical_id,
            request_sha256: request_sha256.clone(),
            effect_sha256: effect_sha256.clone(),
            resolution: ExpectedResolution::Present,
        });
    }
    ledger.attempts.push(Attempt {
        attempt_id: format!("ack-{name}").try_into().unwrap(),
        logical_id: name.to_owned().try_into().unwrap(),
        request_sha256: request_sha256.clone(),
        started_ns: started,
        finished_ns: clock.elapsed().as_nanos() as u64,
        outcome: AttemptOutcome::Acknowledged {
            receipt_sha256: receipt_hash(receipt),
        },
    });
    ledger.readback.push(Readback::Present {
        logical_id: name.to_owned().try_into().unwrap(),
        request_sha256,
        effect_sha256,
        receipt_sha256: receipt_hash(receipt),
        applications: 1.try_into().unwrap(),
    });
}

#[test]
#[ignore = "requires runner-owned ext4/APFS/NTFS image; never fill a developer volume"]
fn installed_native_capacity_recovers_acknowledged_history() {
    let point = std::env::var("RRFLOW_CAPACITY_POINT").unwrap();
    assert!([
        "wal_before_write",
        "segment_before_create",
        "manifest_before_create",
        "current_before_create"
    ]
    .contains(&point.as_str()));
    let mode = std::env::var("RRFLOW_CAPACITY_MODE").unwrap();
    assert!(matches!(mode.as_str(), "fresh" | "existing"));
    capacity_case(&point, &mode);
}

/// Characterize the actual installed domain's first WAL append without filling
/// a filesystem. Native exhaustion remains the separate owned-image oracle.
#[test]
fn installed_capacity_wal_payload_is_observed_before_domain_commit() {
    let estate = Estate::install("existing");
    let credential = RrdEngine::read_installed_api_key(&estate.project).unwrap();
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let mutations = (0..180)
                .map(|index| TransactionMutation::AssertClaim {
                    subject: CanonicalId::new(format!("crash-uncertain-{index}")).unwrap(),
                    predicate: CanonicalId::new("crash").unwrap(),
                    object: format!("{index:04}").repeat(1024),
                    valid_from: estate.at,
                    tx_time: estate.at,
                    producer: CanonicalId::new("independent-oracle").unwrap(),
                    confidence: None,
                })
                .collect::<Vec<_>>();
            let target = CommitTransaction {
                operation_sha256: transaction_operation_sha256(&mutations),
                mutations,
            };
            let mut control = estate.control("wal_before_write", 1, Some(&target.operation_sha256));
            control["resume"] = json!(true);
            let (mut child, client) = estate.serve(&control);
            let session = client
                .create_session(
                    credential.principal_id.clone(),
                    credential.credential(),
                    CreateSession {
                        limits: SessionLimits {
                            idle_timeout_ms: 600_000,
                            absolute_timeout_ms: 3_600_000,
                            max_open_transactions: 2,
                        },
                    },
                    options("capacity-session"),
                )
                .await
                .unwrap();
            let lease = begin(&client, &session, "selection").await;
            let controller = tokio::task::spawn_blocking(move || {
                let line = child.line();
                let hit: Value = serde_json::from_str(
                    std::str::from_utf8(&line)
                        .unwrap()
                        .trim()
                        .strip_prefix(PREFIX)
                        .unwrap(),
                )
                .unwrap();
                for key in ["nonce", "id", "point"] {
                    assert_eq!(hit[key], control[key]);
                }
                assert_eq!(hit["pid"], child.child.id());
                println!("installed_capacity_wal_selection={hit}");
                child
                    .child
                    .resume_controlled(
                        &serde_json::to_vec(
                            &json!({"version":1,"nonce":hit["nonce"],"id":hit["id"]}),
                        )
                        .unwrap(),
                    )
                    .unwrap();
                (child, hit)
            });
            let receipt = client
                .commit_transaction(
                    &session,
                    &lease.transaction_id,
                    target.clone(),
                    options("selection"),
                )
                .await
                .unwrap();
            let (child, hit) = controller.await.unwrap();
            assert!(hit["wal_payload_bytes"].as_u64().unwrap() > 0);
            let replay = client
                .commit_transaction(
                    &session,
                    &lease.transaction_id,
                    target.clone(),
                    options("selection"),
                )
                .await
                .unwrap();
            assert!(replay.idempotent_replay);
            assert_eq!(receipt_hash(&receipt), receipt_hash(&replay));
            assert_eq!(
                read_values(&client, &session, estate.at).await,
                requested_values(&target)
            );
            child.kill();
            let (child, client) = estate.serve(&Value::Null);
            assert_eq!(
                read_values(&client, &session, estate.at).await,
                requested_values(&target)
            );
            child.kill();
        });
}
