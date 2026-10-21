//! Secret leaf-file I/O. Validate and consume one handle, never check then reopen.
//! Ancestor traversal and mount confinement remain the installation boundary's
//! responsibility; this module does not claim an atomic filesystem snapshot.

use std::fs::{File, Metadata};
use std::io::{self, Read, Write};
use std::path::Path;

#[cfg(unix)]
#[path = "private_file/unix.rs"]
mod platform;
#[cfg(windows)]
#[path = "private_file/windows.rs"]
mod platform;
#[cfg(not(any(unix, windows)))]
mod platform {
    use super::*;
    pub(super) fn open(_: &Path, _: bool) -> io::Result<File> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "native secret-file enforcement unavailable",
        ))
    }
    pub(super) fn validate(_: &File) -> io::Result<Metadata> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "native secret-file enforcement unavailable",
        ))
    }
}

fn denied(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::PermissionDenied, message)
}

pub(in crate::engine) fn read_private_bounded(path: &Path, maximum: u64) -> io::Result<Vec<u8>> {
    read_opened(platform::open(path, false)?, maximum)
}

fn read_opened(mut file: File, maximum: u64) -> io::Result<Vec<u8>> {
    let limit = maximum.checked_add(1).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "secret-file byte bound overflow",
        )
    })?;
    let before = platform::validate(&file)?;
    if before.len() > maximum {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "secret file exceeds its byte bound",
        ));
    }
    let mut bytes = Vec::new();
    (&mut file).take(limit).read_to_end(&mut bytes)?;
    let after = platform::validate(&file)?;
    if bytes.len() as u64 > maximum
        || bytes.len() as u64 != before.len()
        || after.len() != before.len()
        || after.modified()? != before.modified()?
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "secret file changed during bounded read",
        ));
    }
    Ok(bytes)
}

pub(in crate::engine) fn write_private_new(path: &Path, bytes: &[u8]) -> io::Result<()> {
    // Exclusive creation establishes confidentiality before the first byte.
    // A failed validation leaves only an empty file; never repair an existing ACL.
    let mut file = platform::open(path, true)?;
    platform::validate(&file)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    platform::validate(&file)?;
    #[cfg(unix)]
    File::open(
        path.parent()
            .ok_or_else(|| denied("secret file has no parent"))?,
    )?
    .sync_all()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_private_reads_and_exclusive_creation_preserve_bytes() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("secret");
        write_private_new(&path, b"secret").unwrap();
        assert_eq!(read_private_bounded(&path, 6).unwrap(), b"secret");
        assert!(read_private_bounded(&path, 5).is_err());
        assert!(read_private_bounded(&path, u64::MAX).is_err());
        assert_eq!(
            write_private_new(&path, b"replace").unwrap_err().kind(),
            io::ErrorKind::AlreadyExists
        );
        assert_eq!(std::fs::read(&path).unwrap(), b"secret");
        assert!(read_private_bounded(root.path(), 64).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn path_replacement_cannot_redirect_an_open_secret_read() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("secret");
        write_private_new(&path, b"original").unwrap();
        let file = platform::open(&path, false).unwrap();
        std::fs::rename(&path, root.path().join("retained")).unwrap();
        write_private_new(&path, b"replaced").unwrap();
        assert_eq!(read_opened(file, 8).unwrap(), b"original");
        assert_eq!(read_private_bounded(&path, 8).unwrap(), b"replaced");
    }

    #[cfg(windows)]
    #[test]
    fn open_secret_handle_excludes_replacement_and_concurrent_writers() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("secret");
        write_private_new(&path, b"original").unwrap();
        let file = platform::open(&path, false).unwrap();
        assert!(std::fs::rename(&path, root.path().join("moved")).is_err());
        assert!(std::fs::OpenOptions::new().write(true).open(&path).is_err());
        assert_eq!(read_opened(file, 8).unwrap(), b"original");
        std::fs::rename(&path, root.path().join("moved")).unwrap();
    }
}
