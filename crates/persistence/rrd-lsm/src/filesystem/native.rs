use cap_primitives::fs as cap;
use std::ffi::OsStr;
use std::fs::File;
use std::io;
use std::path::Path;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct MountIdentity(Vec<u8>);

/// Native object identity plus immutable-file metadata. Page digests remain
/// the content authority; this prevents reopening a substituted directory entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FileIdentity {
    object: (u64, u64),
    bytes: u64,
    modified: std::time::SystemTime,
}

impl FileIdentity {
    pub(crate) fn capture(file: &File) -> io::Result<Self> {
        let metadata = file.metadata()?;
        #[cfg(unix)]
        let object = {
            use std::os::unix::fs::MetadataExt;
            (metadata.dev(), metadata.ino())
        };
        #[cfg(windows)]
        let object = {
            let info = information(file)?;
            (
                u64::from(info.dwVolumeSerialNumber),
                (u64::from(info.nFileIndexHigh) << 32) | u64::from(info.nFileIndexLow),
            )
        };
        #[cfg(not(any(unix, windows)))]
        let object = return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "native immutable file identity is unavailable",
        ));
        Ok(Self {
            object,
            bytes: metadata.len(),
            modified: metadata.modified()?,
        })
    }
}

fn denied(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::PermissionDenied, message)
}

pub(super) fn valid_component(name: &OsStr) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        let units: Vec<_> = name.encode_wide().collect();
        !units.is_empty()
            && !units.iter().any(|unit| matches!(*unit, 0 | 58 | 47 | 92))
            && !matches!(units.last(), Some(32 | 46))
    }
    #[cfg(not(windows))]
    {
        !name.is_empty()
    }
}

pub(super) fn validate_directory(file: &File) -> io::Result<()> {
    validate_object(file)?;
    if !file.metadata()?.is_dir() {
        return Err(denied("ordinary directory required"));
    }
    Ok(())
}

pub(super) fn validate_file(file: &File) -> io::Result<()> {
    validate_object(file)?;
    if !file.metadata()?.is_file() {
        return Err(denied("ordinary file required"));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let links = file.metadata()?.nlink();
        if links != 1 {
            return Err(invalid_links(links));
        }
    }
    #[cfg(windows)]
    {
        let links = u64::from(information(file)?.nNumberOfLinks);
        if links != 1 {
            return Err(invalid_links(links));
        }
    }
    Ok(())
}

#[cfg(any(unix, windows))]
fn invalid_links(links: u64) -> io::Error {
    io::Error::new(
        io::ErrorKind::PermissionDenied,
        format!("file link count denied: expected=1 observed={links}"),
    )
}

pub(super) fn validate_object(file: &File) -> io::Result<()> {
    let metadata = file.metadata()?;
    if !metadata.is_file() && !metadata.is_dir() {
        return Err(denied("nonregular filesystem object denied"));
    }
    #[cfg(windows)]
    {
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::Storage::FileSystem::{
            GetFileType, FILE_ATTRIBUTE_REPARSE_POINT, FILE_TYPE_DISK,
        };
        if unsafe { GetFileType(file.as_raw_handle()) } != FILE_TYPE_DISK
            || information(file)?.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0
        {
            return Err(denied("reparse or nondisk object denied"));
        }
    }
    Ok(())
}

#[cfg(target_os = "linux")]
pub(super) fn mount_identity(file: &File) -> io::Result<MountIdentity> {
    use std::os::fd::AsRawFd;
    let mut stat: libc::statx = unsafe { std::mem::zeroed() };
    let result = unsafe {
        libc::statx(
            file.as_raw_fd(),
            c"".as_ptr(),
            libc::AT_EMPTY_PATH | libc::AT_SYMLINK_NOFOLLOW,
            libc::STATX_MNT_ID,
            &mut stat,
        )
    };
    if result != 0 {
        return Err(io::Error::last_os_error());
    }
    if stat.stx_mask & libc::STATX_MNT_ID == 0 {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "native mount identity unavailable",
        ));
    }
    Ok(MountIdentity(stat.stx_mnt_id.to_ne_bytes().to_vec()))
}

#[cfg(target_os = "macos")]
pub(super) fn mount_identity(file: &File) -> io::Result<MountIdentity> {
    use std::os::fd::AsRawFd;
    let mut stat: libc::statfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::fstatfs(file.as_raw_fd(), &mut stat) } != 0 {
        return Err(io::Error::last_os_error());
    }
    let mut identity = Vec::new();
    // libc keeps fsid_t's two native integers private. Treat this initialized
    // native identifier as opaque bytes; it never enters a persistent format.
    identity.extend_from_slice(unsafe {
        std::slice::from_raw_parts(
            std::ptr::addr_of!(stat.f_fsid).cast::<u8>(),
            std::mem::size_of_val(&stat.f_fsid),
        )
    });
    identity.extend(stat.f_mntonname.iter().map(|value| *value as u8));
    Ok(MountIdentity(identity))
}

#[cfg(windows)]
pub(super) fn mount_identity(file: &File) -> io::Result<MountIdentity> {
    Ok(MountIdentity(
        information(file)?
            .dwVolumeSerialNumber
            .to_ne_bytes()
            .to_vec(),
    ))
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
pub(super) fn mount_identity(_file: &File) -> io::Result<MountIdentity> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "native filesystem containment unavailable",
    ))
}

pub(super) fn file_options(options: &mut cap::OpenOptions) {
    #[cfg(unix)]
    {
        use cap::OpenOptionsExt;
        options
            .mode(0o600)
            .custom_flags(libc::O_NONBLOCK | libc::O_CLOEXEC);
    }
    #[cfg(windows)]
    {
        use cap::OpenOptionsExt;
        use windows_sys::Win32::Storage::FileSystem::FILE_FLAG_OPEN_REPARSE_POINT;
        options.custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    }
}

pub(super) fn metadata_options(options: &mut cap::OpenOptions) {
    file_options(options);
    #[cfg(windows)]
    {
        use cap::OpenOptionsExt;
        use windows_sys::Win32::Storage::FileSystem::{
            FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
        };
        options.custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT);
    }
}

pub(super) fn sync_directory(file: &File) -> io::Result<()> {
    #[cfg(unix)]
    {
        let mut options = cap::OpenOptions::new();
        options.read(true);
        cap::open(file, Path::new("."), &options)?.sync_all()
    }
    #[cfg(not(unix))]
    {
        let _ = file;
        Ok(())
    }
}

#[cfg(not(windows))]
pub(super) fn rename(from: &File, from_name: &Path, to: &File, to_name: &Path) -> io::Result<()> {
    cap::rename(from, from_name, to, to_name)
}

#[cfg(windows)]
pub(super) fn rename(from: &File, from_name: &Path, to: &File, to_name: &Path) -> io::Result<()> {
    rename_windows(from, from_name, to, to_name, true)
}

#[cfg(windows)]
fn rename_windows(
    from: &File,
    from_name: &Path,
    to: &File,
    to_name: &Path,
    replace: bool,
) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };
    // cap-primitives' directory handles omit FILE_SHARE_DELETE, retaining both
    // parents against replacement while preserving write-through publication.
    let from: Vec<_> = final_path(from)?
        .join(from_name)
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    let to: Vec<_> = final_path(to)?
        .join(to_name)
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    if unsafe {
        MoveFileExW(
            from.as_ptr(),
            to.as_ptr(),
            MOVEFILE_WRITE_THROUGH
                | if replace {
                    MOVEFILE_REPLACE_EXISTING
                } else {
                    0
                },
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(windows)]
fn information(
    file: &File,
) -> io::Result<windows_sys::Win32::Storage::FileSystem::BY_HANDLE_FILE_INFORMATION> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::GetFileInformationByHandle;
    let mut information = unsafe { std::mem::zeroed() };
    if unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut information) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(information)
}

#[cfg(windows)]
fn final_path(file: &File) -> io::Result<std::path::PathBuf> {
    use std::os::windows::{ffi::OsStringExt, io::AsRawHandle};
    use windows_sys::Win32::Storage::FileSystem::GetFinalPathNameByHandleW;
    let mut buffer = vec![0u16; 32_768];
    let length = unsafe {
        GetFinalPathNameByHandleW(
            file.as_raw_handle(),
            buffer.as_mut_ptr(),
            buffer.len() as u32,
            0,
        )
    };
    if length == 0 {
        return Err(io::Error::last_os_error());
    }
    if length as usize >= buffer.len() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "native pathname exceeds bound",
        ));
    }
    buffer.truncate(length as usize);
    Ok(std::ffi::OsString::from_wide(&buffer).into())
}

/// Atomic publication that cannot overwrite a concurrently published winner.
pub(super) fn rename_new(
    from: &File,
    from_name: &Path,
    to: &File,
    to_name: &Path,
) -> io::Result<()> {
    #[cfg(windows)]
    {
        rename_windows(from, from_name, to, to_name, false)
    }
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        use std::os::{fd::AsRawFd, unix::ffi::OsStrExt};
        let source = std::ffi::CString::new(from_name.as_os_str().as_bytes())
            .map_err(|_| denied("invalid source name"))?;
        let target = std::ffi::CString::new(to_name.as_os_str().as_bytes())
            .map_err(|_| denied("invalid target name"))?;
        #[cfg(target_os = "linux")]
        let result = unsafe {
            libc::renameat2(
                from.as_raw_fd(),
                source.as_ptr(),
                to.as_raw_fd(),
                target.as_ptr(),
                libc::RENAME_NOREPLACE,
            )
        };
        #[cfg(target_os = "macos")]
        let result = unsafe {
            libc::renameatx_np(
                from.as_raw_fd(),
                source.as_ptr(),
                to.as_raw_fd(),
                target.as_ptr(),
                libc::RENAME_EXCL,
            )
        };
        if result != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
    #[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
    {
        let _ = (from, from_name, to, to_name);
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "atomic exclusive publication unavailable",
        ))
    }
}

/// Refuse a filesystem that cannot enforce private objects before creation.
pub(super) fn validate_root(file: &File) -> io::Result<()> {
    #[cfg(windows)]
    {
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::Storage::FileSystem::GetVolumeInformationByHandleW;
        let mut flags = 0u32;
        if unsafe {
            GetVolumeInformationByHandleW(
                file.as_raw_handle(),
                std::ptr::null_mut(),
                0,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut flags,
                std::ptr::null_mut(),
                0,
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        // FILE_PERSISTENT_ACLS: the volume preserves and enforces ACLs.
        if flags & 0x0000_0008 == 0 {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "filesystem does not enforce persistent ACLs",
            ));
        }
    }
    #[cfg(not(windows))]
    {
        let _ = file;
    }
    Ok(())
}
