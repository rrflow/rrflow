//! Allocation settlement for the admitted, disposable APFS image only.
use std::{fs::File, io, os::fd::AsRawFd};

// Apple SDK <sys/fsctl.h>: _IOW('A', 1, uint32_t), FSCTL_SYNC_WAIT.
// See apple-oss-distributions/xnu/bsd/sys/fsctl.h and
// bsd/vfs/vfs_syscalls.c::handle_sync_volume. Wait for this volume,
// without FSCTL_SYNC_FULLSYNC (a hardware durability barrier).
const FSIOC_SYNC_VOLUME: libc::c_ulong = 0x8004_4101;
const FSCTL_SYNC_WAIT: u32 = 1 << 1;

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
