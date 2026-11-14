//! Creating and opening a manifest store and acquiring its writer.

use super::*;

impl ManifestStore {
    /// Creates the manifest namespace and its exclusive writer lease. The
    /// caller must have already established an empty database root.
    pub fn create(root: &Path) -> Result<Self> {
        Self::create_anchored(&AnchoredPath::open_root(root)?)
    }

    pub fn create_anchored(root: &AnchoredPath) -> Result<Self> {
        let manifest_directory = root.join(MANIFEST_DIRECTORY);
        let checkpoint_directory = root.join(CHECKPOINT_DIRECTORY);
        manifest_directory.create_dir().map_err(|error| {
            Error::io(
                "creating manifest directory",
                manifest_directory.diagnostic_path(),
                error,
            )
        })?;
        checkpoint_directory.create_dir().map_err(|error| {
            Error::io(
                "creating checkpoint directory",
                checkpoint_directory.diagnostic_path(),
                error,
            )
        })?;
        Self::acquire_writer(root, true, false)
    }

    /// Opens a complete manifest namespace without creating any path.
    pub fn open_existing(root: &Path) -> Result<Self> {
        Self::open_existing_anchored(&AnchoredPath::open_root(root)?)
    }

    pub fn open_existing_anchored(root: &AnchoredPath) -> Result<Self> {
        for (operation, path) in [
            ("opening manifest directory", root.join(MANIFEST_DIRECTORY)),
            (
                "opening checkpoint directory",
                root.join(CHECKPOINT_DIRECTORY),
            ),
        ] {
            let metadata = path
                .metadata()
                .map_err(|error| Error::io(operation, path.diagnostic_path(), error))?;
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err(Error::InvalidManifest(format!(
                    "{} is not a non-symlink directory",
                    path.display()
                )));
            }
        }
        Self::acquire_writer(root, false, false)
    }

    /// Legacy manifest utility entry point retained for direct manifest tests.
    /// Database lifecycle code uses `create` or `open_existing` explicitly.
    pub fn open(root: &Path) -> Result<Self> {
        Self::open_anchored(&AnchoredPath::open_root(root)?)
    }

    pub fn open_anchored(root: &AnchoredPath) -> Result<Self> {
        let manifest_directory = root.join(MANIFEST_DIRECTORY);
        let checkpoint_directory = root.join(CHECKPOINT_DIRECTORY);
        manifest_directory.create_dir_all().map_err(|error| {
            Error::io(
                "creating manifest directory",
                manifest_directory.diagnostic_path(),
                error,
            )
        })?;
        checkpoint_directory.create_dir_all().map_err(|error| {
            Error::io(
                "creating checkpoint directory",
                checkpoint_directory.diagnostic_path(),
                error,
            )
        })?;
        Self::acquire_writer(root, false, true)
    }

    pub(super) fn acquire_writer(
        root: &AnchoredPath,
        create_new: bool,
        create_if_missing: bool,
    ) -> Result<Self> {
        let lock_path = root.join(MANIFEST_LOCK_FILE);
        if !create_new && !create_if_missing {
            let metadata = lock_path.metadata().map_err(|error| {
                Error::io(
                    "validating existing manifest lock",
                    lock_path.diagnostic_path(),
                    error,
                )
            })?;
            if metadata.file_type().is_symlink() || !metadata.is_file() {
                return Err(Error::InvalidManifest(
                    "manifest lock is not a regular non-symlink file".into(),
                ));
            }
        }
        // A process-local counter restarts at one and can collide with bytes
        // left by a killed publisher. A new writer namespace preserves those
        // bytes while allowing subsequent create-new publications to proceed.
        use ring::rand::SecureRandom;
        let mut nonce = [0u8; 16];
        ring::rand::SystemRandom::new()
            .fill(&mut nonce)
            .map_err(|_| Error::Io(std::io::Error::other("manifest writer entropy unavailable")))?;
        let mut options = OpenOptions::new();
        options.read(true).write(true).truncate(false);
        if create_new {
            options.create_new(true);
        } else {
            options.create(create_if_missing);
        }
        let lock = options.open(&lock_path).map_err(|error| {
            Error::io("opening manifest lock", lock_path.diagnostic_path(), error)
        })?;
        match lock.try_lock() {
            Ok(()) => {}
            Err(std::fs::TryLockError::WouldBlock) => {
                return Err(Error::DatabaseWriterLock {
                    path: lock_path.diagnostic_path().to_owned(),
                });
            }
            Err(std::fs::TryLockError::Error(error)) => {
                return Err(Error::io(
                    "acquiring manifest lock",
                    lock_path.diagnostic_path(),
                    error,
                ));
            }
        }
        let store = Self {
            root: root.to_owned(),
            _lock: lock,
            temporary_nonce: nonce.iter().map(|byte| format!("{byte:02x}")).collect(),
            publication_uncertain: AtomicBool::new(false),
            checkpoint_entries: DEFAULT_CHECKPOINT_ENTRIES,
            checkpoint_budget: crate::ReclamationLimits::default(),
            checkpoint_gate: Mutex::new(()),
        };
        if !create_new {
            // Reopen may observe an earlier rename/unlink whose barrier failed.
            // Stabilize the selected root and checkpoint names before allowing
            // effects (especially GC) based on that observed reachability.
            root.sync_directory()?;
            root.join(CHECKPOINT_DIRECTORY).sync_directory()?;
        }
        Ok(store)
    }

    pub(crate) fn ensure_publication_ready(&self) -> Result<()> {
        if self.publication_uncertain.load(Ordering::Acquire) {
            return Err(Error::RecoveryRequired {
                boundary: "uncertain manifest publication",
            });
        }
        Ok(())
    }

    pub(super) fn publication_result<T>(&self, result: Result<T>) -> Result<T> {
        if result.is_err() && !matches!(&result, Err(Error::DiskAdmission { .. })) {
            // A failed rename or directory barrier may already have selected a
            // different root. No dependent effect may use the old in-memory root.
            self.publication_uncertain.store(true, Ordering::Release);
        }
        result
    }
}
