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

pub(super) fn filler_allocation_sync_preserves_regular_file_contents() {
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
pub(super) fn filler_host_sync_preserves_native_noncapacity_errors() {
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
        // Exact create_new-owned file only; never recurse or remove estate data.
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
