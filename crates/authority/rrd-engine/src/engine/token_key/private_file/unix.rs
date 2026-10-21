use super::{denied, File, Metadata, Path};
use std::fs::OpenOptions;
use std::io;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};

pub(super) fn open(path: &Path, create: bool) -> io::Result<File> {
    let mut options = OpenOptions::new();
    options
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC);
    if create {
        options.write(true).create_new(true).mode(0o600);
    }
    options.open(path)
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
mod tests {
    use super::super::{read_private_bounded, write_private_new};
    use super::*;

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
