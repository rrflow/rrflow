//! Mutation-free physical inspection for installed-instance verification.
//!
//! This reader deliberately does not implement the storage writer traits. It
//! takes a shared nonblocking manifest lease, authenticates the complete
//! CURRENT physical closure plus any retained parent manifests, and replays
//! the active WAL into private memory. Garbage collection may deliberately
//! remove unpinned parents and their segments; that clean missing-parent
//! boundary is not corruption. No create, truncate, reconcile, or repair path
//! is reachable through the public reader. The explicit repair owner alone may
//! request an exclusive lease and a validated incomplete-tail prefix.

use crate::database::{wal_path, SEGMENT_DIRECTORY, WAL_DIRECTORY};
use crate::filesystem::AnchoredPath;
use crate::io::IoContext;
use crate::manifest::{
    CurrentPointer, Manifest, CHECKPOINT_DIRECTORY, CURRENT_FILE, MANIFEST_DIRECTORY,
    MANIFEST_LOCK_FILE,
};
use crate::recovery_budget::RecoveryBudget;
use crate::segment::{new_page_cache_with_policy, Segment};
use crate::wal::replay_file_bounded;
use crate::{DatabaseOptions, Error, Memtable, Result, VersionedValue};
use rrd_core::digest::Sha256;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InspectionFile {
    pub relative_path: String,
    pub bytes: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DatabaseInspection {
    pub application_format: Option<u64>,
    pub current_manifest_sha256: String,
    pub current_generation: u64,
    pub durable_sequence: u64,
    pub visible_sequence: u64,
    pub wal_valid_bytes: u64,
    /// Newest manifest first, ending at generation one or the first absent
    /// parent. The current garbage collector deliberately creates that latter
    /// boundary for unpinned history; a quick inspection does not claim why a
    /// particular absent ancestor is missing.
    pub manifest_lineage: Vec<String>,
    pub reachable_segments: Vec<String>,
    pub files: Vec<InspectionFile>,
    pub total_bytes: u64,
}

/// A bounded read view backed only by authenticated existing bytes.
pub struct ReadOnlyDatabase {
    _root: AnchoredPath,
    _lock: File,
    manifest: Manifest,
    memtable: Memtable,
    segments: Vec<Segment>,
    inspection: DatabaseInspection,
}

impl Drop for ReadOnlyDatabase {
    fn drop(&mut self) {
        let _ = File::unlock(&self._lock);
    }
}

impl ReadOnlyDatabase {
    pub fn open_with_options(root: &Path, options: DatabaseOptions) -> Result<Self> {
        Self::open_anchored_with_options(&AnchoredPath::open_root(root)?, options)
    }

    pub fn open(root: &Path) -> Result<Self> {
        Self::open_anchored(&AnchoredPath::open_root(root)?)
    }

    pub fn open_anchored(root: &AnchoredPath) -> Result<Self> {
        Self::open_anchored_with_options(root, DatabaseOptions::default())
    }

    pub fn open_anchored_with_options(
        root: &AnchoredPath,
        options: DatabaseOptions,
    ) -> Result<Self> {
        Self::open_inner(root, options, false).map(|(reader, _, _)| reader)
    }

    pub(crate) fn open_inner(
        root: &AnchoredPath,
        options: DatabaseOptions,
        repair: bool,
    ) -> Result<(Self, Option<u64>, RecoveryBudget)> {
        let options = options.validate()?;
        let mut budget = RecoveryBudget::new(options.recovery);
        let retained_root = root.pin_directory()?;
        let root = &retained_root;
        let metadata = root.metadata().map_err(|error| {
            Error::io(
                "opening database root for inspection",
                root.diagnostic_path(),
                error,
            )
        })?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(Error::InvalidManifest(
                "inspection target is not a non-symlink database directory".into(),
            ));
        }
        for relative in [
            MANIFEST_DIRECTORY,
            CHECKPOINT_DIRECTORY,
            SEGMENT_DIRECTORY,
            WAL_DIRECTORY,
        ] {
            require_directory(root, Path::new(relative))?;
        }

        let lock_path = root.join(MANIFEST_LOCK_FILE);
        require_regular_file(root, Path::new(MANIFEST_LOCK_FILE))?;
        let lock = lock_path.open_read().map_err(|error| {
            Error::io(
                "opening manifest lock for inspection",
                lock_path.diagnostic_path(),
                error,
            )
        })?;
        let lease = if repair {
            lock.try_lock()
        } else {
            lock.try_lock_shared()
        };
        match lease {
            Ok(()) => {}
            Err(std::fs::TryLockError::WouldBlock) => {
                return Err(Error::DatabaseWriterLock {
                    path: lock_path.diagnostic_path().to_owned(),
                });
            }
            Err(std::fs::TryLockError::Error(error)) => {
                return Err(Error::io(
                    "acquiring shared manifest inspection lock",
                    lock_path.diagnostic_path(),
                    error,
                ));
            }
        }

        let current_path = root.join(CURRENT_FILE);
        require_regular_file(root, Path::new(CURRENT_FILE))?;
        let current_bytes = budget.metadata(&current_path)?;
        let pointer: CurrentPointer = serde_json::from_slice(&current_bytes)?;
        pointer.validate()?;

        let mut lineage = Vec::new();
        let mut next_digest = Some(pointer.manifest.clone());
        let mut expected_generation = pointer.generation;
        let mut expected_application_format = None;
        let mut ended_at_gc_boundary = false;
        while let Some(digest) = next_digest {
            budget.tick()?;
            if lineage
                .iter()
                .any(|manifest: &Manifest| manifest.digest == digest)
            {
                return Err(Error::InvalidManifest(
                    "manifest ancestry contains a cycle".into(),
                ));
            }
            let relative = PathBuf::from(MANIFEST_DIRECTORY).join(format!("{digest}.json"));
            match root.join(&relative).metadata() {
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    ended_at_gc_boundary = true;
                    break;
                }
                Err(error) => {
                    return Err(Error::io(
                        "validating retained manifest for inspection",
                        root.join(&relative).diagnostic_path(),
                        error,
                    ));
                }
                Ok(_) => {}
            }
            let manifest = load_manifest(root, &digest, &mut budget)?;
            if manifest.generation != expected_generation {
                return Err(Error::InvalidManifest(
                    "manifest ancestry does not decrement one generation at a time".into(),
                ));
            }
            match expected_application_format {
                None => expected_application_format = Some(manifest.application_format),
                Some(expected) if expected != manifest.application_format => {
                    return Err(Error::InvalidManifest(
                        "application format changes within manifest ancestry".into(),
                    ));
                }
                Some(_) => {}
            }
            next_digest = manifest.parent.clone();
            expected_generation = expected_generation.saturating_sub(1);
            lineage.push(manifest);
        }
        if !ended_at_gc_boundary && expected_generation != 0 {
            return Err(Error::InvalidManifest(
                "manifest ancestry does not terminate at generation one".into(),
            ));
        }
        let manifest = lineage
            .first()
            .cloned()
            .ok_or_else(|| Error::InvalidManifest("CURRENT has no manifest".into()))?;
        for descriptor in &manifest.segments {
            budget.file(descriptor.bytes)?;
        }
        let descriptors = manifest
            .segments
            .iter()
            .map(|descriptor| {
                (
                    (descriptor.id.clone(), descriptor.level),
                    descriptor.clone(),
                )
            })
            .collect::<BTreeMap<_, _>>();
        if descriptors.len() != manifest.segments.len() {
            return Err(Error::InvalidManifest(
                "current manifest repeats a segment identity".into(),
            ));
        }

        let page_cache =
            new_page_cache_with_policy(options.page_cache_bytes, options.page_cache_policy)?;
        let io = IoContext::new(options.segment_io)?;
        let mut segments = Vec::with_capacity(manifest.segments.len());
        for descriptor in &manifest.segments {
            budget.tick()?;
            let relative = PathBuf::from(SEGMENT_DIRECTORY).join(format!("{}.seg", descriptor.id));
            require_regular_file(root, &relative)?;
            segments.push(Segment::open_expected_with_cache_and_io(
                &root.join(relative),
                descriptor,
                Arc::clone(&page_cache),
                Arc::clone(&io),
            )?);
        }

        let mut memtable = Memtable::at_sequence(manifest.durable_sequence);
        let active_wal = wal_path(root, manifest.wal_start_sequence);
        let active_wal_relative = active_wal.relative_path();
        require_regular_file(root, active_wal_relative)?;
        let mut wal_file = active_wal.open_read()?;
        let recovery = replay_file_bounded(
            &mut wal_file,
            manifest.wal_start_sequence,
            &mut budget,
            |batch| memtable.apply_owned_recovered(batch, options.mutable_limits),
        )?;
        if !repair {
            if let Some(offset) = recovery.torn_tail {
                return Err(Error::TornTail { offset });
            }
        }

        let mut relative_files = BTreeSet::<PathBuf>::new();
        relative_files.insert(PathBuf::from(CURRENT_FILE));
        relative_files.insert(PathBuf::from(MANIFEST_LOCK_FILE));
        relative_files.insert(active_wal_relative.to_owned());
        for ancestor in &lineage {
            relative_files.insert(
                PathBuf::from(MANIFEST_DIRECTORY).join(format!("{}.json", ancestor.digest)),
            );
        }
        for descriptor in descriptors.values() {
            relative_files
                .insert(PathBuf::from(SEGMENT_DIRECTORY).join(format!("{}.seg", descriptor.id)));
        }
        let mut files = Vec::with_capacity(relative_files.len());
        let mut total_bytes = 0u64;
        for relative in relative_files {
            let inspected = inspect_file(root, &relative, &mut budget)?;
            total_bytes = total_bytes.checked_add(inspected.bytes).ok_or_else(|| {
                Error::InvalidManifest("inspection file byte count overflow".into())
            })?;
            files.push(inspected);
        }

        let inspection = DatabaseInspection {
            application_format: manifest.application_format,
            current_manifest_sha256: manifest.digest.clone(),
            current_generation: manifest.generation,
            durable_sequence: manifest.durable_sequence,
            visible_sequence: memtable.maximum_sequence(),
            wal_valid_bytes: recovery.valid_bytes,
            manifest_lineage: lineage
                .iter()
                .map(|ancestor| ancestor.digest.clone())
                .collect(),
            reachable_segments: descriptors
                .values()
                .map(|descriptor| descriptor.id.clone())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect(),
            files,
            total_bytes,
        };
        Ok((
            Self {
                _root: retained_root,
                _lock: lock,
                manifest,
                memtable,
                segments,
                inspection,
            },
            recovery.torn_tail,
            budget,
        ))
    }

    pub fn inspection(&self) -> &DatabaseInspection {
        &self.inspection
    }

    pub fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        let sequence = self.inspection.visible_sequence;
        if let Some(version) = self.memtable.get_version(key, sequence) {
            return Ok(version.value.as_deref().map(<[u8]>::to_vec));
        }
        let mut best_sequence = 0;
        let mut best_value = None;
        for segment in &self.segments {
            if let Some(version) = segment.get_version(key, sequence)? {
                if version.sequence > best_sequence {
                    best_sequence = version.sequence;
                    best_value = version.value;
                }
            }
        }
        Ok(best_value.map(|value| value.into_vec()))
    }

    pub fn scan(&self, start: &[u8], end: Option<&[u8]>) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        let sequence = self.inspection.visible_sequence;
        let mut visible = BTreeMap::<Vec<u8>, VersionedValue>::new();
        for segment in &self.segments {
            for (key, version) in segment.visible_from(start, end, sequence)? {
                if visible
                    .get(&key)
                    .is_none_or(|current| version.sequence > current.sequence)
                {
                    visible.insert(key, version);
                }
            }
        }
        for (key, version) in self.memtable.visible_from(start, end, sequence) {
            if visible
                .get(&key)
                .is_none_or(|current| version.sequence > current.sequence)
            {
                visible.insert(key, version);
            }
        }
        Ok(visible
            .into_iter()
            .filter_map(|(key, version)| version.value.map(|value| (key, value.into_vec())))
            .collect())
    }

    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }
}

fn load_manifest(
    root: &AnchoredPath,
    digest: &str,
    budget: &mut RecoveryBudget,
) -> Result<Manifest> {
    if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(Error::InvalidManifest(
            "manifest lookup requires a SHA-256 identity".into(),
        ));
    }
    let relative = PathBuf::from(MANIFEST_DIRECTORY).join(format!("{digest}.json"));
    require_regular_file(root, &relative)?;
    let path = root.join(relative);
    let bytes = budget.metadata(&path)?;
    let manifest: Manifest = serde_json::from_slice(&bytes)?;
    manifest.validate()?;
    if manifest.digest != digest {
        return Err(Error::InvalidManifest(
            "manifest filename differs from its content identity".into(),
        ));
    }
    Ok(manifest)
}

fn require_directory(root: &AnchoredPath, relative: &Path) -> Result<()> {
    let path = root.join(relative);
    let metadata = path.metadata().map_err(|error| {
        Error::io(
            "validating inspection directory",
            path.diagnostic_path(),
            error,
        )
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(Error::InvalidManifest(format!(
            "inspection directory {} is symbolic or invalid",
            path.display()
        )));
    }
    Ok(())
}

fn require_regular_file(root: &AnchoredPath, relative: &Path) -> Result<()> {
    let path = root.join(relative);
    let metadata = path
        .metadata()
        .map_err(|error| Error::io("validating inspection file", path.diagnostic_path(), error))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(Error::InvalidManifest(format!(
            "inspection file {} is symbolic or invalid",
            path.display()
        )));
    }
    Ok(())
}

fn inspect_file(
    root: &AnchoredPath,
    relative: &Path,
    budget: &mut RecoveryBudget,
) -> Result<InspectionFile> {
    require_regular_file(root, relative)?;
    let path = root.join(relative);
    let mut file = path.open_read().map_err(|error| {
        Error::io(
            "opening file for inspection digest",
            path.diagnostic_path(),
            error,
        )
    })?;
    let expected_bytes = file.metadata()?.len();
    budget.file(expected_bytes)?;
    let mut digest = Sha256::new();
    let mut bytes = 0u64;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        budget.tick()?;
        let read = file.read(&mut buffer).map_err(|error| {
            Error::io(
                "reading file for inspection digest",
                path.diagnostic_path(),
                error,
            )
        })?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
        bytes = bytes
            .checked_add(read as u64)
            .ok_or_else(|| Error::InvalidManifest("inspection file size overflow".into()))?;
        if bytes > expected_bytes {
            return Err(Error::InvalidManifest(
                "inspection file grew while hashing".into(),
            ));
        }
    }
    if bytes != expected_bytes {
        return Err(Error::InvalidManifest(
            "inspection file length changed".into(),
        ));
    }
    budget.tick()?;
    Ok(InspectionFile {
        relative_path: relative.to_string_lossy().replace('\\', "/"),
        bytes,
        sha256: digest.finalize_hex(),
    })
}
