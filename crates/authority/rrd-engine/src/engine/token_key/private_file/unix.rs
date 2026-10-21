#[cfg(test)]
use super::Path;
use super::{denied, File, Metadata};
use std::io;
use std::os::unix::fs::{MetadataExt, PermissionsExt};

pub(super) fn open_relative(
    parent: &File,
    name: &std::ffi::OsStr,
    create: bool,
    directory: bool,
) -> io::Result<File> {
    use std::os::fd::{AsRawFd, FromRawFd};
    use std::os::unix::ffi::OsStrExt;
    let name =
        std::ffi::CString::new(name.as_bytes()).map_err(|_| denied("private name contains NUL"))?;
    if create
        && directory
        && unsafe { libc::mkdirat(parent.as_raw_fd(), name.as_ptr(), 0o700) } != 0
    {
        return Err(io::Error::last_os_error());
    }
    let mut flags = libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC;
    if directory {
        flags |= libc::O_DIRECTORY | libc::O_RDONLY;
    } else if create {
        flags |= libc::O_RDWR | libc::O_CREAT | libc::O_EXCL;
    } else {
        flags |= libc::O_RDONLY;
    }
    let descriptor = unsafe { libc::openat(parent.as_raw_fd(), name.as_ptr(), flags, 0o600) };
    if descriptor < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: successful openat transfers this new descriptor to the File.
    Ok(unsafe { File::from_raw_fd(descriptor) })
}

#[cfg(test)]
pub(super) fn open_directory(path: &Path, create: bool) -> io::Result<File> {
    let path = rrd_store::filesystem::AnchoredPath::from_path(path)?;
    path.with_parent(|parent, name| open_relative(parent, name, create, true))
}

pub(super) fn validate_directory(file: &File) -> io::Result<()> {
    let metadata = file.metadata()?;
    // SAFETY: geteuid has no pointer arguments or preconditions.
    if !metadata.is_dir()
        || metadata.uid() != unsafe { libc::geteuid() }
        || metadata.permissions().mode() & 0o7777 != 0o700
    {
        return Err(denied(
            "estate directory requires effective-user ownership and mode 0700",
        ));
    }
    Ok(())
}

#[cfg(test)]
pub(super) fn open(path: &Path, create: bool) -> io::Result<File> {
    let path = rrd_store::filesystem::AnchoredPath::from_path(path)?;
    path.open_file_with(|parent, name| open_relative(parent, name, create, false))
}

pub(super) fn validate(file: &File) -> io::Result<Metadata> {
    let metadata = file.metadata()?;
    // SAFETY: geteuid has no pointer arguments or preconditions.
    let owner = unsafe { libc::geteuid() };
    if !metadata.is_file() || metadata.nlink() != 1 {
        return Err(denied(
            "secret must be a regular file with exactly one link",
        ));
    }
    if metadata.uid() != owner || metadata.permissions().mode() & 0o077 != 0 {
        return Err(denied(
            "secret must belong exclusively to the effective user",
        ));
    }
    Ok(metadata)
}

#[cfg(test)]
pub(super) fn make_directory_permissive_for_test(path: &Path) {
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

#[cfg(test)]
pub(super) fn restore_private_directory_for_test(path: &Path) {
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).unwrap();
}

#[cfg(test)]
mod tests {
    use super::super::{read_private_bounded, write_private_new};
    use super::*;

    #[test]
    fn directory_modes_and_symlink_leaves_are_denied_without_repair() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("directory");
        let directory = open_directory(&path, true).unwrap();
        validate_directory(&directory).unwrap();
        for mode in [0o750, 0o755, 0o777, 0o1700, 0o2700] {
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode)).unwrap();
            assert!(validate_directory(&directory).is_err());
            assert_eq!(std::fs::metadata(&path).unwrap().mode() & 0o7777, mode);
        }
        restore_private_directory_for_test(&path);
        let alias = root.path().join("alias");
        std::os::unix::fs::symlink(&path, &alias).unwrap();
        assert!(open_directory(&alias, false).is_err());
        validate_directory(&directory).unwrap();
    }

    #[test]
    fn permissive_modes_and_symlinks_are_denied_without_repair() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("secret");
        write_private_new(&path, b"secret").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o640)).unwrap();
        assert!(read_private_bounded(&path, 6).is_err());
        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o640
        );
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        let alias = root.path().join("alias");
        std::os::unix::fs::symlink(&path, &alias).unwrap();
        assert!(read_private_bounded(&alias, 6).is_err());
        assert_eq!(read_private_bounded(&path, 6).unwrap(), b"secret");
    }

    #[test]
    fn fifo_is_rejected_without_waiting_for_a_writer() {
        use std::os::unix::ffi::OsStrExt;
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("fifo");
        let name = std::ffi::CString::new(path.as_os_str().as_bytes()).unwrap();
        // SAFETY: name is a live NUL-terminated path in a test-owned directory.
        assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
        assert!(read_private_bounded(&path, 32).is_err());
    }
}
