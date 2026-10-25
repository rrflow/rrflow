//! Keep an already-owned failed estate without copying or selecting new paths.
use std::{
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

pub(super) struct OwnedDirectory {
    directory: Option<tempfile::TempDir>,
    retained: AtomicBool,
}

impl OwnedDirectory {
    pub(super) fn new(directory: tempfile::TempDir) -> Self {
        Self {
            directory: Some(directory),
            retained: AtomicBool::new(false),
        }
    }

    pub(super) fn path(&self) -> &Path {
        self.directory
            .as_ref()
            .expect("live owned directory")
            .path()
    }

    pub(super) fn retain(&self) -> &Path {
        self.retained.store(true, Ordering::Relaxed);
        self.path()
    }
}

impl Drop for OwnedDirectory {
    fn drop(&mut self) {
        if self.retained.load(Ordering::Relaxed) {
            if let Some(directory) = self.directory.take() {
                let _ = directory.keep();
            }
        }
        // Otherwise TempDir performs its ordinary owned cleanup.
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinary_completion_removes_only_its_owned_directory() {
        let parent = tempfile::tempdir().unwrap();
        let sentinel = parent.path().join("unrelated");
        std::fs::write(&sentinel, b"unchanged").unwrap();
        let owned = OwnedDirectory::new(tempfile::tempdir_in(parent.path()).unwrap());
        let path = owned.path().to_owned();
        std::fs::write(path.join("state"), b"owned").unwrap();
        drop(owned);
        assert!(!path.exists());
        assert_eq!(std::fs::read(sentinel).unwrap(), b"unchanged");
    }

    #[test]
    fn explicit_failure_retention_survives_unwind_without_copying_state() {
        let parent = tempfile::tempdir().unwrap();
        let owned = OwnedDirectory::new(tempfile::tempdir_in(parent.path()).unwrap());
        let path = owned.path().to_owned();
        std::fs::write(path.join("state"), b"original failure bytes").unwrap();
        let result = std::panic::catch_unwind(move || {
            assert_eq!(owned.retain(), path);
            assert_eq!(owned.retain(), path);
            panic!("characterized startup failure");
        });
        assert!(result.is_err());
        let entries: Vec<_> = std::fs::read_dir(parent.path()).unwrap().collect();
        assert_eq!(entries.len(), 1, "retention must not create a second copy");
        let path = entries.into_iter().next().unwrap().unwrap().path();
        assert_eq!(
            std::fs::read(path.join("state")).unwrap(),
            b"original failure bytes"
        );
        // The outer test-owned TempDir cleans up after inspection.
    }
}
