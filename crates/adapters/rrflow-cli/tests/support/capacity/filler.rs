//! Filler operations, their synchronisation tests and capacity codes.

use super::*;

impl Filler {
    pub(super) fn fill(volume: &Volume, metadata_directory: Option<&std::path::Path>) -> Self {
        let directory = metadata_directory.map(|path| {
            assert!(!std::fs::symlink_metadata(path)
                .unwrap()
                .file_type()
                .is_symlink());
            let path = std::fs::canonicalize(path).unwrap();
            assert!(
                path.starts_with(&volume.root) && path != volume.root,
                "metadata fill must remain inside the owned engram volume"
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
            let result = synchronize_allocation(
                reason,
                !cfg!(target_os = "macos"),
                || {
                    match reason {
                        FillBarrier::Interval => interval_barriers.set(interval_barriers.get() + 1),
                        FillBarrier::Capacity => capacity_barriers.set(capacity_barriers.get() + 1),
                    }
                    sync_filler_allocation(file)
                },
                || {
                    #[cfg(target_os = "macos")]
                    apfs::sync_volume(&volume_root)?;
                    Ok(())
                },
            );
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
        // Exact create_new-owned file only; never recurse or remove engram data.
        if let Err(error) = std::fs::remove_file(&self.path) {
            if !std::thread::panicking() {
                panic!("owned filler cleanup failed: {error}");
            }
        }
    }
}
pub(super) fn capacity_code(code: i32) -> bool {
    if cfg!(windows) {
        matches!(code, 39 | 112)
    } else if cfg!(target_os = "macos") {
        matches!(code, 28 | 69)
    } else {
        matches!(code, 28 | 122)
    }
}

mod tests {
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
            settle_pressure(expired_fill_clock(), (8, 2), || {
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
    fn stable_full_refill_uses_one_byte_before_and_after_settlement() {
        struct Full {
            attempts: Vec<usize>,
        }
        impl Write for Full {
            fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                self.attempts.push(bytes.len());
                Err(std::io::Error::from_raw_os_error(if cfg!(windows) {
                    112
                } else {
                    28
                }))
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let mut file = Full {
            attempts: Vec::new(),
        };
        let mut allocated = 128;
        let mut barriers = Vec::new();
        let code = fill_remaining_data(
            &mut file,
            &mut allocated,
            1024,
            Instant::now(),
            |_, reason| {
                barriers.push(reason);
                Ok(())
            },
        )
        .unwrap();
        assert!(capacity_code(code));
        assert_eq!(allocated, 128);
        assert_eq!(barriers, [FillBarrier::Capacity]);
        assert_eq!(
            file.attempts,
            [1, 1],
            "a stable refill must not repeat bulk-sized failed probes"
        );
    }

    #[test]
    fn refill_consumes_space_released_by_settlement_before_final_denial() {
        struct Refill {
            remaining: Cell<usize>,
            attempts: Vec<usize>,
        }
        impl Write for Refill {
            fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                self.attempts.push(bytes.len());
                let count = bytes.len().min(self.remaining.get());
                if count == 0 {
                    return Err(std::io::Error::from_raw_os_error(if cfg!(windows) {
                        112
                    } else {
                        28
                    }));
                }
                self.remaining.set(self.remaining.get() - count);
                Ok(count)
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let mut file = Refill {
            remaining: Cell::new(0),
            attempts: Vec::new(),
        };
        let mut allocated = 0;
        let mut barriers = 0;
        let code = fill_remaining_data(
            &mut file,
            &mut allocated,
            128,
            Instant::now(),
            |file, reason| {
                assert_eq!(reason, FillBarrier::Capacity);
                if barriers == 0 {
                    file.remaining.set(17);
                }
                barriers += 1;
                Ok(())
            },
        )
        .unwrap();
        assert!(capacity_code(code));
        assert_eq!(allocated, 17);
        assert_eq!(file.remaining.get(), 0);
        assert_eq!(barriers, 2);
        assert_eq!(&file.attempts[..2], &[1, 1]);
        assert!(
            file.attempts[2] > 1,
            "successful probe must resume bulk writes"
        );
        assert_eq!(file.attempts.last(), Some(&1));
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
        let expired = expired_fill_clock();
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = fill_remaining_data(&mut file, &mut allocated, 20, expired, |_, _| {
                panic!("expired refill synchronized")
            });
        }))
        .is_err());
        assert_eq!(file.remaining.get(), 4, "refill restarted the clock");
    }

    #[test]
    fn disposable_apfs_fill_defers_intervals_but_settles_and_drains_delayed_allocation() {
        const INITIAL: usize = 60 * 4 * 1024 * 1024;
        for periodic_file_sync in [false, true] {
            let mut file = DelayedAllocation::new();
            file.remaining.set(INITIAL);
            let file_syncs = Cell::new(0);
            let volume_syncs = Cell::new(0);
            let (bytes, code) = fill_data(
                &mut file,
                (INITIAL + 1024) as u64,
                Instant::now(),
                |file, reason| {
                    synchronize_allocation(
                        reason,
                        periodic_file_sync,
                        || {
                            file_syncs.set(file_syncs.get() + 1);
                            Ok(())
                        },
                        || {
                            if volume_syncs.get() == 0 {
                                file.remaining.set(16);
                            }
                            volume_syncs.set(volume_syncs.get() + 1);
                            Ok(())
                        },
                    )
                },
            )
            .unwrap();
            assert_eq!(bytes, (INITIAL + 16) as u64);
            assert_eq!(
                file.remaining.get(),
                0,
                "settled allocation escaped pressure"
            );
            assert!(capacity_code(code));
            assert_eq!(volume_syncs.get(), 2, "final settlement was skipped");
            assert_eq!(
                file_syncs.get(),
                if periodic_file_sync { 62 } else { 2 },
                "disposable APFS intervals consumed unnecessary barriers"
            );
        }
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
        let expired = expired_fill_clock();
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
