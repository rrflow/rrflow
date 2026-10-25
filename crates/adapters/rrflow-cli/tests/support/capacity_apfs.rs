//! Allocation settlement for the admitted, disposable APFS image only.
use std::{fs::File, io, os::fd::AsRawFd};

// Apple SDK <sys/fsctl.h>: _IOW('A', 1, uint32_t), FSCTL_SYNC_WAIT.
// See apple-oss-distributions/xnu/bsd/sys/fsctl.h and
// bsd/vfs/vfs_syscalls.c::handle_sync_volume. Wait for this volume,
// without FSCTL_SYNC_FULLSYNC (a hardware durability barrier).
const FSIOC_SYNC_VOLUME: libc::c_ulong = 0x8004_4101;
const FSCTL_SYNC_WAIT: u32 = 1 << 1;

// Apple File System Reference: embedded attributes fit below 3804 bytes.
// Only disposable filler files receive this private, create-only attribute.
pub(super) const METADATA_PAYLOAD_BYTES: usize = 3072;
const METADATA_NAME: &std::ffi::CStr = c"org.rrflow.capacity-fixture";

pub(super) fn allocate_metadata(file: &File) -> io::Result<()> {
    let payload = [0xa5u8; METADATA_PAYLOAD_BYTES];
    // SAFETY: the live descriptor, NUL-terminated name and initialized payload
    // remain valid throughout the synchronous call. Position zero is required
    // for ordinary attributes. XATTR_CREATE prevents replacing existing data.
    let result = unsafe {
        libc::fsetxattr(
            file.as_raw_fd(),
            METADATA_NAME.as_ptr(),
            payload.as_ptr().cast(),
            payload.len(),
            0,
            libc::XATTR_CREATE,
        )
    };
    if result == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

#[test]
fn metadata_payload_is_native_create_only_and_preserves_file_data() {
    use std::io::Write;
    let mut file = tempfile::tempfile().unwrap();
    file.write_all(b"fixture data").unwrap();
    allocate_metadata(&file).unwrap();
    let mut payload = [0u8; METADATA_PAYLOAD_BYTES];
    // SAFETY: writable buffer and live descriptor/name for the entire call.
    let count = unsafe {
        libc::fgetxattr(
            file.as_raw_fd(),
            METADATA_NAME.as_ptr(),
            payload.as_mut_ptr().cast(),
            payload.len(),
            0,
            0,
        )
    };
    assert_eq!(count, METADATA_PAYLOAD_BYTES as isize);
    assert_eq!(payload, [0xa5; METADATA_PAYLOAD_BYTES]);
    assert_eq!(file.metadata().unwrap().len(), 12);
    assert_eq!(
        allocate_metadata(&file).unwrap_err().raw_os_error(),
        Some(libc::EEXIST)
    );
}

unsafe extern "C" {
    // Apple Libc/include/unistd.h; libc does not expose this declaration.
    fn ffsctl(
        fd: libc::c_int,
        request: libc::c_ulong,
        data: *mut libc::c_void,
        options: libc::c_uint,
    ) -> libc::c_int;
}

pub(super) fn sync_volume(root: &File) -> io::Result<()> {
    let mut flags = FSCTL_SYNC_WAIT;
    // SAFETY: root owns the live descriptor throughout the call; flags is
    // writable u32 storage matching the request's encoded argument size.
    let result = unsafe {
        ffsctl(
            root.as_raw_fd(),
            FSIOC_SYNC_VOLUME,
            (&mut flags as *mut u32).cast(),
            0,
        )
    };
    if result == 0 {
        Ok(())
    } else {
        // Never translate an unsupported call or another native error into
        // successful settlement. The filler recognizes capacity codes only.
        Err(io::Error::last_os_error())
    }
}

#[test]
fn volume_barrier_preserves_nonfilesystem_descriptor_error() {
    // A socket has no backing filesystem; this cannot sync a developer volume.
    let (socket, _peer) = std::os::unix::net::UnixStream::pair().unwrap();
    let descriptor: std::os::fd::OwnedFd = socket.into();
    let error = sync_volume(&File::from(descriptor)).unwrap_err();
    assert!(error.raw_os_error().is_some());
    assert!(!super::capacity_code(error.raw_os_error().unwrap()));
}
