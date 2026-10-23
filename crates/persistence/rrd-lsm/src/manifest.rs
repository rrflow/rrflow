use crate::filesystem::{AnchoredPath, OpenOptions};
use crate::{Error, Result};
use rrd_core::digest;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

pub const MANIFEST_FORMAT_VERSION: u16 = 3;
const MANIFEST_CONTROL_FORMAT_VERSION: u16 = 1;
pub(crate) const CURRENT_FILE: &str = "CURRENT";
pub(crate) const MANIFEST_DIRECTORY: &str = "manifests";
pub(crate) const CHECKPOINT_DIRECTORY: &str = "checkpoints";
pub(crate) const MANIFEST_LOCK_FILE: &str = "MANIFEST.LOCK";
static POINTER_TEMPORARY_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SegmentDescriptor {
    pub id: String,
    pub level: u8,
    #[serde(default)]
    pub format_version: u16,
    #[serde(default)]
    pub schema_digest: String,
    #[serde(default)]
    pub key_codec_digest: String,
    #[serde(default)]
    pub page_format_digest: String,
    pub first_key: Vec<u8>,
    pub last_key: Vec<u8>,
    pub minimum_sequence: u64,
    pub maximum_sequence: u64,
    pub entries: u64,
    pub bytes: u64,
    pub checksum: String,
}

impl SegmentDescriptor {
    fn validate(&self) -> Result<()> {
        if !is_sha256(&self.id) || !is_sha256(&self.checksum) {
            return Err(Error::InvalidManifest(
                "segment identity and checksum must be content-addressed SHA-256 values".into(),
            ));
        }
        if self.format_version != crate::SEGMENT_FORMAT_VERSION {
            return Err(Error::InvalidManifest(format!(
                "segment {} uses unsupported physical format {}",
                self.id, self.format_version
            )));
        }
        for (name, actual, expected) in [
            (
                "schema",
                self.schema_digest.as_str(),
                crate::SEGMENT_SCHEMA_DIGEST,
            ),
            (
                "key codec",
                self.key_codec_digest.as_str(),
                crate::SEGMENT_KEY_CODEC_DIGEST,
            ),
            (
                "page format",
                self.page_format_digest.as_str(),
                crate::SEGMENT_PAGE_FORMAT_DIGEST,
            ),
        ] {
            if !is_sha256(actual) || actual != expected {
                return Err(Error::InvalidManifest(format!(
                    "segment {} has an unknown {name} digest",
                    self.id
                )));
            }
        }
        if self.first_key > self.last_key {
            return Err(Error::InvalidManifest(format!(
                "segment {} has an inverted key range",
                self.id
            )));
        }
        if self.minimum_sequence == 0 || self.minimum_sequence > self.maximum_sequence {
            return Err(Error::InvalidManifest(format!(
                "segment {} has an invalid sequence range",
                self.id
            )));
        }
        if self.entries == 0 || self.bytes == 0 {
            return Err(Error::InvalidManifest(format!(
                "segment {} must contain entries and bytes",
                self.id
            )));
        }
        Ok(())
    }
}

/// Immutable physical state published by a single compare-and-swap update of
/// the `CURRENT` pointer. The digest excludes itself and is the manifest id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    pub format_version: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub application_format: Option<u64>,
    pub generation: u64,
    pub parent: Option<String>,
    pub created_at: u64,
    pub durable_sequence: u64,
    pub wal_start_sequence: u64,
    pub segments: Vec<SegmentDescriptor>,
    pub digest: String,
}

impl Manifest {
    pub fn new(
        generation: u64,
        parent: Option<String>,
        created_at: u64,
        durable_sequence: u64,
        wal_start_sequence: u64,
        segments: Vec<SegmentDescriptor>,
    ) -> Result<Self> {
        Self::new_inner(
            generation,
            parent,
            created_at,
            durable_sequence,
            wal_start_sequence,
            segments,
            None,
        )
    }

    pub fn new_with_application_format(
        generation: u64,
        parent: Option<String>,
        created_at: u64,
        durable_sequence: u64,
        wal_start_sequence: u64,
        segments: Vec<SegmentDescriptor>,
        application_format: u64,
    ) -> Result<Self> {
        if application_format == 0 {
            return Err(Error::InvalidManifest(
                "application format identity must be non-zero".into(),
            ));
        }
        Self::new_inner(
            generation,
            parent,
            created_at,
            durable_sequence,
            wal_start_sequence,
            segments,
            Some(application_format),
        )
    }

    fn new_inner(
        generation: u64,
        parent: Option<String>,
        created_at: u64,
        durable_sequence: u64,
        wal_start_sequence: u64,
        mut segments: Vec<SegmentDescriptor>,
        application_format: Option<u64>,
    ) -> Result<Self> {
        segments.sort_by(|left, right| {
            left.level
                .cmp(&right.level)
                .then_with(|| left.first_key.cmp(&right.first_key))
                .then_with(|| left.id.cmp(&right.id))
        });
        let mut manifest = Self {
            format_version: MANIFEST_FORMAT_VERSION,
            application_format,
            generation,
            parent,
            created_at,
            durable_sequence,
            wal_start_sequence,
            segments,
            digest: String::new(),
        };
        manifest.validate_components()?;
        manifest.digest = digest::sha256_hex(&manifest.bytes_without_digest()?);
        Ok(manifest)
    }

    pub fn validate(&self) -> Result<()> {
        if self.format_version != MANIFEST_FORMAT_VERSION {
            return Err(Error::UnsupportedVersion {
                object: "manifest",
                version: self.format_version,
            });
        }
        if self.application_format == Some(0) {
            return Err(Error::InvalidManifest(
                "application format identity must be non-zero".into(),
            ));
        }
        self.validate_components()?;
        let expected = digest::sha256_hex(&self.bytes_without_digest()?);
        if self.digest != expected {
            return Err(Error::InvalidManifest(
                "manifest digest does not match its fields".into(),
            ));
        }
        Ok(())
    }

    fn validate_components(&self) -> Result<()> {
        if self.generation == 0 {
            return Err(Error::InvalidManifest(
                "manifest generation must be greater than zero".into(),
            ));
        }
        if self.wal_start_sequence > self.durable_sequence.saturating_add(1) {
            return Err(Error::InvalidManifest(
                "WAL start cannot skip beyond the durable sequence".into(),
            ));
        }
        if self.generation == 1 && self.parent.is_some() {
            return Err(Error::InvalidManifest(
                "the first manifest cannot name a parent".into(),
            ));
        }
        if self.generation > 1 && self.parent.as_deref().is_none_or(str::is_empty) {
            return Err(Error::InvalidManifest(
                "later manifests require a parent digest".into(),
            ));
        }
        if self
            .parent
            .as_deref()
            .is_some_and(|parent| !is_sha256(parent))
        {
            return Err(Error::InvalidManifest(
                "manifest parent must be a SHA-256 digest".into(),
            ));
        }
        let mut identities = BTreeSet::new();
        let mut level_last_keys = BTreeMap::<u8, Vec<u8>>::new();
        for segment in &self.segments {
            segment.validate()?;
            if !identities.insert(&segment.id) {
                return Err(Error::InvalidManifest(format!(
                    "segment {} appears more than once",
                    segment.id
                )));
            }
            if segment.maximum_sequence > self.durable_sequence {
                return Err(Error::InvalidManifest(format!(
                    "segment {} extends beyond the durable sequence",
                    segment.id
                )));
            }
            if segment.level != 0 {
                if level_last_keys
                    .get(&segment.level)
                    .is_some_and(|last_key| last_key >= &segment.first_key)
                {
                    return Err(Error::InvalidManifest(format!(
                        "level {} contains overlapping or unordered segment ranges",
                        segment.level
                    )));
                }
                level_last_keys.insert(segment.level, segment.last_key.clone());
            }
        }
        Ok(())
    }

    fn bytes_without_digest(&self) -> Result<Vec<u8>> {
        #[derive(Serialize)]
        struct Content<'a> {
            format_version: u16,
            application_format: &'a Option<u64>,
            generation: u64,
            parent: &'a Option<String>,
            created_at: u64,
            durable_sequence: u64,
            wal_start_sequence: u64,
            segments: &'a [SegmentDescriptor],
        }
        Ok(serde_json::to_vec(&Content {
            format_version: self.format_version,
            application_format: &self.application_format,
            generation: self.generation,
            parent: &self.parent,
            created_at: self.created_at,
            durable_sequence: self.durable_sequence,
            wal_start_sequence: self.wal_start_sequence,
            segments: &self.segments,
        })?)
    }
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CurrentPointer {
    pub format_version: u16,
    pub generation: u64,
    pub manifest: String,
    pub checksum: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Checkpoint {
    pub format_version: u16,
    pub name: String,
    pub manifest: String,
    pub generation: u64,
    pub created_at: u64,
    pub checksum: String,
}

impl Checkpoint {
    fn new(name: &str, manifest: &Manifest, created_at: u64) -> Result<Self> {
        validate_checkpoint_name(name)?;
        let mut checkpoint = Self {
            format_version: MANIFEST_CONTROL_FORMAT_VERSION,
            name: name.to_owned(),
            manifest: manifest.digest.clone(),
            generation: manifest.generation,
            created_at,
            checksum: String::new(),
        };
        checkpoint.checksum = checkpoint.expected_checksum();
        Ok(checkpoint)
    }

    fn validate(&self) -> Result<()> {
        if self.format_version != MANIFEST_CONTROL_FORMAT_VERSION {
            return Err(Error::UnsupportedVersion {
                object: "checkpoint",
                version: self.format_version,
            });
        }
        validate_checkpoint_name(&self.name)?;
        if !is_sha256(&self.manifest)
            || !is_sha256(&self.checksum)
            || self.generation == 0
            || self.checksum != self.expected_checksum()
        {
            return Err(Error::InvalidManifest(
                "invalid checkpoint identity or checksum".into(),
            ));
        }
        Ok(())
    }

    fn expected_checksum(&self) -> String {
        let mut bytes = b"rrflow-checkpoint-v1\0".to_vec();
        bytes.extend_from_slice(&self.format_version.to_be_bytes());
        bytes.extend_from_slice(self.name.as_bytes());
        bytes.push(0);
        bytes.extend_from_slice(self.manifest.as_bytes());
        bytes.extend_from_slice(&self.generation.to_be_bytes());
        bytes.extend_from_slice(&self.created_at.to_be_bytes());
        digest::sha256_hex(&bytes)
    }
}

impl CurrentPointer {
    fn new(manifest: &Manifest) -> Self {
        let mut pointer = Self {
            format_version: MANIFEST_CONTROL_FORMAT_VERSION,
            generation: manifest.generation,
            manifest: manifest.digest.clone(),
            checksum: String::new(),
        };
        pointer.checksum = pointer.expected_checksum();
        pointer
    }

    pub(crate) fn validate(&self) -> Result<()> {
        if self.format_version != MANIFEST_CONTROL_FORMAT_VERSION {
            return Err(Error::UnsupportedVersion {
                object: "CURRENT pointer",
                version: self.format_version,
            });
        }
        if self.generation == 0 || !is_sha256(&self.manifest) || !is_sha256(&self.checksum) {
            return Err(Error::InvalidManifest(
                "invalid CURRENT pointer fields".into(),
            ));
        }
        if self.checksum != self.expected_checksum() {
            return Err(Error::InvalidManifest(
                "CURRENT pointer checksum mismatch".into(),
            ));
        }
        Ok(())
    }

    fn expected_checksum(&self) -> String {
        let mut bytes = b"rrflow-current-v1\0".to_vec();
        bytes.extend_from_slice(&self.format_version.to_be_bytes());
        bytes.extend_from_slice(&self.generation.to_be_bytes());
        bytes.extend_from_slice(self.manifest.as_bytes());
        digest::sha256_hex(&bytes)
    }
}

/// Holds the process lock for the full publication session. Manifest bytes are
/// synced before `CURRENT`, then the parent directory is synced after rename.
pub struct ManifestStore {
    root: AnchoredPath,
    _lock: File,
    publication_uncertain: AtomicBool,
}

impl Drop for ManifestStore {
    fn drop(&mut self) {
        // Release explicitly before Windows closes the handle. Relying only on
        // handle teardown can leave an immediate same-process reopen racing the
        // OS lock release.
        let _ = File::unlock(&self._lock);
    }
}

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

    fn acquire_writer(
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
            publication_uncertain: AtomicBool::new(false),
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

    fn publication_result<T>(&self, result: Result<T>) -> Result<T> {
        if result.is_err() && !matches!(&result, Err(Error::DiskAdmission { .. })) {
            // A failed rename or directory barrier may already have selected a
            // different root. No dependent effect may use the old in-memory root.
            self.publication_uncertain.store(true, Ordering::Release);
        }
        result
    }

    pub fn current(&self) -> Result<Option<(CurrentPointer, Manifest)>> {
        self.current_using(|path| Ok(path.read()?))
    }

    pub(crate) fn current_bounded(
        &self,
        budget: &mut crate::recovery_budget::RecoveryBudget,
    ) -> Result<Option<(CurrentPointer, Manifest)>> {
        self.current_using(|path| budget.metadata(path))
    }

    fn current_using(
        &self,
        mut read: impl FnMut(&AnchoredPath) -> Result<Vec<u8>>,
    ) -> Result<Option<(CurrentPointer, Manifest)>> {
        let path = self.root.join(CURRENT_FILE);
        match path.metadata() {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
                return Err(Error::InvalidManifest(
                    "CURRENT is not a regular non-symlink file".into(),
                ));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => {
                return Err(Error::io(
                    "validating CURRENT",
                    path.diagnostic_path(),
                    error,
                ))
            }
        }
        let bytes = read(&path)?;
        let pointer: CurrentPointer = serde_json::from_slice(&bytes)?;
        pointer.validate()?;
        let manifest = self.load_using(&pointer.manifest, read)?;
        if manifest.generation != pointer.generation {
            return Err(Error::InvalidManifest(
                "CURRENT generation differs from its manifest".into(),
            ));
        }
        Ok(Some((pointer, manifest)))
    }

    pub fn load(&self, digest: &str) -> Result<Manifest> {
        self.load_using(digest, |path| Ok(path.read()?))
    }

    pub(crate) fn load_bounded(
        &self,
        digest: &str,
        budget: &mut crate::reclamation::ReclamationBudget,
    ) -> Result<Manifest> {
        self.load_using(digest, |path| budget.read_metadata(path))
    }

    fn load_using(
        &self,
        digest: &str,
        mut read: impl FnMut(&AnchoredPath) -> Result<Vec<u8>>,
    ) -> Result<Manifest> {
        if !is_sha256(digest) {
            return Err(Error::InvalidManifest(
                "manifest lookup requires a SHA-256 identity".into(),
            ));
        }
        let path = self
            .root
            .join(MANIFEST_DIRECTORY)
            .join(format!("{digest}.json"));
        let metadata = path.metadata().map_err(|error| {
            Error::io("validating manifest file", path.diagnostic_path(), error)
        })?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(Error::InvalidManifest(
                "manifest is not a regular non-symlink file".into(),
            ));
        }
        let bytes = read(&path)?;
        let manifest: Manifest = serde_json::from_slice(&bytes)?;
        manifest.validate()?;
        if manifest.digest != digest {
            return Err(Error::InvalidManifest(
                "manifest filename differs from its content identity".into(),
            ));
        }
        Ok(manifest)
    }

    pub fn publish(&self, manifest: &Manifest, expected: Option<&str>) -> Result<CurrentPointer> {
        self.ensure_publication_ready()?;
        manifest.validate()?;
        let current = self.current()?;
        let actual = current
            .as_ref()
            .map(|(pointer, _)| pointer.manifest.clone());
        if actual.as_deref() != expected {
            return Err(Error::ManifestConflict {
                expected: expected.map(str::to_owned),
                actual,
            });
        }
        match current {
            None if manifest.generation == 1 && manifest.parent.is_none() => {}
            Some((pointer, _))
                if manifest.generation == pointer.generation + 1
                    && manifest.parent.as_deref() == Some(pointer.manifest.as_str()) => {}
            _ => {
                return Err(Error::InvalidManifest(
                    "manifest generation/parent does not extend CURRENT".into(),
                ))
            }
        }
        self.publication_result(self.persist_manifest(manifest))?;
        let pointer = CurrentPointer::new(manifest);
        self.publication_result(self.publish_pointer(&pointer))?;
        Ok(pointer)
    }

    pub fn checkpoint(
        &self,
        name: &str,
        manifest_digest: &str,
        created_at: u64,
    ) -> Result<Checkpoint> {
        self.ensure_publication_ready()?;
        let manifest = self.load(manifest_digest)?;
        let checkpoint = Checkpoint::new(name, &manifest, created_at)?;
        let directory = self.root.join(CHECKPOINT_DIRECTORY);
        let path = directory.join(format!("{name}.json"));
        let bytes = serde_json::to_vec(&checkpoint)?;
        if path.try_exists()? {
            let existing: Checkpoint = serde_json::from_slice(&path.read()?)?;
            existing.validate()?;
            if existing == checkpoint {
                self.publication_result(directory.sync_directory().map_err(Error::Io))?;
                return Ok(existing);
            }
            return Err(Error::InvalidManifest(format!(
                "checkpoint {name:?} already pins another manifest or instant"
            )));
        }
        let temporary = directory.join(format!(
            ".{name}.{}.tmp",
            POINTER_TEMPORARY_ID.fetch_add(1, Ordering::Relaxed)
        ));
        self.publication_result(persist_rename(&directory, &temporary, &path, &bytes))?;
        Ok(checkpoint)
    }

    pub fn checkpoints(&self) -> Result<Vec<Checkpoint>> {
        self.checkpoints_inner(None)
    }

    pub(crate) fn checkpoints_bounded(
        &self,
        budget: &mut crate::reclamation::ReclamationBudget,
    ) -> Result<Vec<Checkpoint>> {
        self.checkpoints_inner(Some(budget))
    }

    fn checkpoints_inner(
        &self,
        mut budget: Option<&mut crate::reclamation::ReclamationBudget>,
    ) -> Result<Vec<Checkpoint>> {
        let directory = self.root.join(CHECKPOINT_DIRECTORY);
        let mut checkpoints = Vec::new();
        let mut entries = Vec::new();
        for entry in directory.entries()? {
            if let Some(budget) = budget.as_deref_mut() {
                budget.entry()?;
            }
            entries.push(entry?);
        }
        entries.sort_by(|left, right| left.relative_path().cmp(right.relative_path()));
        for entry in entries {
            if entry
                .relative_path()
                .extension()
                .and_then(|value| value.to_str())
                != Some("json")
            {
                continue;
            }
            let bytes = match budget.as_deref_mut() {
                Some(budget) => budget.read_metadata(&entry)?,
                None => entry.read()?,
            };
            let checkpoint: Checkpoint = serde_json::from_slice(&bytes)?;
            checkpoint.validate()?;
            let manifest = match budget.as_deref_mut() {
                Some(budget) => self.load_bounded(&checkpoint.manifest, budget)?,
                None => self.load(&checkpoint.manifest)?,
            };
            if manifest.generation != checkpoint.generation {
                return Err(Error::InvalidManifest(format!(
                    "checkpoint {:?} generation differs from its manifest",
                    checkpoint.name
                )));
            }
            checkpoints.push(checkpoint);
        }
        checkpoints.sort_by(|left, right| left.name.cmp(&right.name));
        Ok(checkpoints)
    }

    pub fn release_checkpoint(&self, name: &str) -> Result<bool> {
        self.ensure_publication_ready()?;
        validate_checkpoint_name(name)?;
        let directory = self.root.join(CHECKPOINT_DIRECTORY);
        let path = directory.join(format!("{name}.json"));
        match path.remove_file() {
            Ok(()) => {
                self.publication_result(directory.sync_directory().map_err(Error::Io))?;
                Ok(true)
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(error) => self.publication_result(Err(error.into())),
        }
    }

    fn persist_manifest(&self, manifest: &Manifest) -> Result<()> {
        let directory = self.root.join(MANIFEST_DIRECTORY);
        let path = directory.join(format!("{}.json", manifest.digest));
        let bytes = serde_json::to_vec(manifest)?;
        match path.metadata() {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() || !metadata.is_file() {
                    return Err(Error::InvalidManifest(
                        "existing manifest is not a regular non-symlink file".into(),
                    ));
                }
                if path.read()? != bytes {
                    return Err(Error::InvalidManifest(
                        "existing manifest identity has different bytes".into(),
                    ));
                }
                directory.sync_directory()?;
                return Ok(());
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(Error::io(
                    "validating manifest destination",
                    path.diagnostic_path(),
                    error,
                ))
            }
        }
        let temporary = directory.join(format!(
            ".{}.{}.tmp",
            manifest.digest,
            POINTER_TEMPORARY_ID.fetch_add(1, Ordering::Relaxed)
        ));
        persist_rename(&directory, &temporary, &path, &bytes)
    }

    fn publish_pointer(&self, pointer: &CurrentPointer) -> Result<()> {
        let path = self.root.join(CURRENT_FILE);
        let temporary = self.root.join(format!(
            ".CURRENT.{}.tmp",
            POINTER_TEMPORARY_ID.fetch_add(1, Ordering::Relaxed)
        ));
        persist_rename(&self.root, &temporary, &path, &serde_json::to_vec(pointer)?)
    }
}

fn validate_checkpoint_name(name: &str) -> Result<()> {
    if name.is_empty()
        || name.len() > 128
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return Err(Error::InvalidManifest(format!(
            "invalid checkpoint name {name:?}"
        )));
    }
    Ok(())
}

fn persist_rename(
    _directory: &AnchoredPath,
    temporary: &AnchoredPath,
    target: &AnchoredPath,
    bytes: &[u8],
) -> Result<()> {
    temporary.admit_disk(bytes.len() as u64, true)?;
    crate::filesystem::persist_rename(temporary, target, bytes).map_err(Error::Io)
}
