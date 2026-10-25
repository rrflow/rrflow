//! One native, nonblocking admission for installation and recovery of a project.
//! The empty file is retained forever; unlinking it on release would permit two
//! independently locked inodes. Presence, PID and age never establish ownership.
use super::*;
use crate::engine::token_key::open_private_empty_anchored;
use std::fs::{File, TryLockError};

pub(super) const RELATIVE_PATH: &str = rrd_contract::INSTALLATION_LEASE_PATH;

pub(super) struct InstallationLease {
    _file: File,
}

impl Drop for InstallationLease {
    fn drop(&mut self) {
        // A duplicate open-file description (including a concurrent fork)
        // can outlive this handle. End this owner's lock explicitly, as the
        // manifest writer does, before closing the retained native handle.
        let _ = File::unlock(&self._file);
    }
}

pub(super) fn admit_existing(project: &AnchoredPath) -> Result<Option<File>> {
    match open_private_empty_anchored(&project.join(RELATIVE_PATH), false) {
        Ok(file) => Ok(Some(file)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(map_io(error)),
    }
}

impl InstallationLease {
    pub(super) fn existing(project: &AnchoredPath) -> Result<Option<Self>> {
        admit_existing(project)?.map(Self::lock).transpose()
    }

    pub(super) fn create_or_acquire(project: &AnchoredPath) -> Result<Self> {
        match open_private_empty_anchored(&project.join(RELATIVE_PATH), true) {
            Ok(file) => Self::lock(file),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                Self::existing(project)?.ok_or_else(conflict)
            }
            Err(error) => Err(map_io(error)),
        }
    }

    fn lock(file: File) -> Result<Self> {
        match file.try_lock() {
            Ok(()) => Ok(Self { _file: file }),
            Err(TryLockError::WouldBlock) => Err(conflict()),
            Err(TryLockError::Error(error)) => Err(map_io(error)),
        }
    }
}

fn conflict() -> ServiceError {
    ServiceError::StorageConflict("another installer owns this project's installation lease".into())
}

fn map_io(error: std::io::Error) -> ServiceError {
    // A Windows handle opened by an incompatible writer also denies admission.
    if error.kind() == std::io::ErrorKind::WouldBlock
        || cfg!(windows) && error.raw_os_error() == Some(32)
    {
        conflict()
    } else {
        storage_error(error)
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn lease_drop_releases_a_duplicated_description_without_releasing_its_successor() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("lease");
        let lease = InstallationLease::lock(File::create(&path).unwrap()).unwrap();
        // dup/fork keep the same open-file description alive after one close.
        let inherited = lease._file.try_clone().unwrap();
        let contender = File::options().read(true).write(true).open(&path).unwrap();
        assert!(matches!(
            contender.try_lock(),
            Err(TryLockError::WouldBlock)
        ));
        drop(lease);
        let successor = InstallationLease::lock(contender).unwrap();
        drop(inherited);
        let third = File::options().read(true).write(true).open(&path).unwrap();
        assert!(matches!(third.try_lock(), Err(TryLockError::WouldBlock)));
        drop(successor);
        let _next = InstallationLease::lock(third).unwrap();
        assert_eq!(std::fs::metadata(path).unwrap().len(), 0);
    }
}
