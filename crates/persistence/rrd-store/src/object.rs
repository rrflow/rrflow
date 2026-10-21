//! Immutable, content-addressed object tier.
//!
//! Bytes are durable and verified before their canonical reference is handed
//! to the data transaction. A failed data transaction can therefore leave an
//! unreachable object, but never a reachable partial object; inventory and
//! reclamation make that asymmetry explicit.

use crate::{Error, Result};
use rrd_core::{digest, ObjectReceipt, ObjectReference};
use rrd_lsm::filesystem::{AnchoredPath, OpenOptions};
use std::collections::{BTreeMap, BTreeSet};
#[cfg(test)]
use std::fs;
use std::fs::File;
use std::io::{Cursor, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

static STAGE_ORDINAL: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectStep {
    BeforeStageWrite,
    AfterStageSync,
    AfterPublish,
    BeforeVerify,
    AfterVerify,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedObject {
    pub sha256: String,
    pub length: u64,
    pub receipt: ObjectReceipt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObjectInventoryState {
    Reachable,
    Orphan,
    Corrupt { actual_sha256: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectInventoryEntry {
    pub sha256: String,
    pub length: u64,
    pub state: ObjectInventoryState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectInventory {
    pub entries: Vec<ObjectInventoryEntry>,
    pub staging_files: Vec<PathBuf>,
    pub quarantined_files: Vec<PathBuf>,
}

pub trait ImmutableObjectStore: Send + Sync {
    fn put(&self, bytes: &[u8]) -> Result<VerifiedObject>;
    /// Opens a verified object for bounded-memory transfer. Implementations
    /// must authenticate digest and length before returning the reader.
    fn open_verified(&self, reference: &ObjectReference) -> Result<Box<dyn Read + Send + '_>>;
    /// Consumes exactly one declared content address. Extra, truncated, or
    /// substituted bytes fail before the object becomes addressable.
    fn put_verified_stream(
        &self,
        expected_sha256: &str,
        expected_length: u64,
        reader: &mut dyn Read,
    ) -> Result<VerifiedObject>;
    fn verify(&self, sha256: &str) -> Result<VerifiedObject>;
    fn get(&self, reference: &ObjectReference) -> Result<Vec<u8>>;
    /// Returns the same file handle whose immutable bytes were verified.
    /// Callers may use it for read-only mmap; remote and in-memory
    /// authorities return `None` and retain the verified streaming boundary.
    fn verified_file(&self, _reference: &ObjectReference) -> Result<Option<File>> {
        Ok(None)
    }
    fn inventory(&self, reachable: &BTreeSet<String>) -> Result<ObjectInventory>;
    fn reclaim_orphans(&self, unreachable: &BTreeSet<String>) -> Result<Vec<String>>;
}

/// Process-local immutable bytes for the true in-memory RRD composition.
/// Content addressing and verification remain identical to durable object
/// stores; only persistence and physical recovery are intentionally absent.
#[derive(Default)]
pub struct MemoryObjectStore {
    objects: Mutex<BTreeMap<String, Vec<u8>>>,
}

impl MemoryObjectStore {
    pub fn new() -> Self {
        Self::default()
    }

    fn verified(&self, sha256: &str, bytes: &[u8]) -> Result<VerifiedObject> {
        let actual = digest::sha256_hex(bytes);
        if actual != sha256 {
            return Err(Error::ObjectCorrupt {
                expected: sha256.to_owned(),
                actual,
            });
        }
        let key = ObjectReference::canonical_key(sha256).map_err(Error::from)?;
        Ok(VerifiedObject {
            sha256: sha256.to_owned(),
            length: bytes.len() as u64,
            receipt: ObjectReceipt {
                backend: "memory".into(),
                key,
                version: None,
                etag: Some(sha256.to_owned()),
            },
        })
    }
}

impl ImmutableObjectStore for MemoryObjectStore {
    fn put(&self, bytes: &[u8]) -> Result<VerifiedObject> {
        let sha256 = digest::sha256_hex(bytes);
        let verified = self.verified(&sha256, bytes)?;
        self.objects
            .lock()
            .expect("memory object mutex")
            .entry(sha256)
            .or_insert_with(|| bytes.to_vec());
        Ok(verified)
    }

    fn open_verified(&self, reference: &ObjectReference) -> Result<Box<dyn Read + Send + '_>> {
        Ok(Box::new(Cursor::new(self.get(reference)?)))
    }

    fn put_verified_stream(
        &self,
        expected_sha256: &str,
        expected_length: u64,
        reader: &mut dyn Read,
    ) -> Result<VerifiedObject> {
        let capacity = usize::try_from(expected_length)
            .map_err(|_| Error::Object("object length exceeds process address space".into()))?;
        let mut bytes = Vec::new();
        bytes.try_reserve_exact(capacity).map_err(|_| {
            Error::Object("memory object capacity cannot be reserved within process limits".into())
        })?;
        reader
            .take(expected_length.saturating_add(1))
            .read_to_end(&mut bytes)?;
        let actual_length = bytes.len() as u64;
        if actual_length != expected_length {
            return Err(Error::ObjectLengthMismatch {
                expected: expected_length,
                actual: actual_length,
            });
        }
        let verified = self.verified(expected_sha256, &bytes)?;
        self.objects
            .lock()
            .expect("memory object mutex")
            .entry(expected_sha256.to_owned())
            .or_insert(bytes);
        Ok(verified)
    }

    fn verify(&self, sha256: &str) -> Result<VerifiedObject> {
        let objects = self.objects.lock().expect("memory object mutex");
        let bytes = objects
            .get(sha256)
            .ok_or_else(|| Error::ObjectMissing(sha256.to_owned()))?;
        self.verified(sha256, bytes)
    }

    fn get(&self, reference: &ObjectReference) -> Result<Vec<u8>> {
        reference.validate().map_err(Error::from)?;
        let canonical_key =
            ObjectReference::canonical_key(&reference.sha256).map_err(Error::from)?;
        if reference.receipt.backend != "memory" || reference.receipt.key != canonical_key {
            return Err(Error::Object(
                "object receipt does not belong to the memory object authority".into(),
            ));
        }
        let objects = self.objects.lock().expect("memory object mutex");
        let bytes = objects
            .get(&reference.sha256)
            .ok_or_else(|| Error::ObjectMissing(reference.sha256.clone()))?;
        let verified = self.verified(&reference.sha256, bytes)?;
        if verified.length != reference.length {
            return Err(Error::ObjectLengthMismatch {
                expected: reference.length,
                actual: verified.length,
            });
        }
        Ok(bytes.clone())
    }

    fn inventory(&self, reachable: &BTreeSet<String>) -> Result<ObjectInventory> {
        let objects = self.objects.lock().expect("memory object mutex");
        let entries = objects
            .iter()
            .map(|(sha256, bytes)| {
                let verified = self.verified(sha256, bytes)?;
                Ok(ObjectInventoryEntry {
                    sha256: sha256.clone(),
                    length: verified.length,
                    state: if reachable.contains(sha256) {
                        ObjectInventoryState::Reachable
                    } else {
                        ObjectInventoryState::Orphan
                    },
                })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(ObjectInventory {
            entries,
            staging_files: Vec::new(),
            quarantined_files: Vec::new(),
        })
    }

    fn reclaim_orphans(&self, unreachable: &BTreeSet<String>) -> Result<Vec<String>> {
        let mut objects = self.objects.lock().expect("memory object mutex");
        let removed = unreachable
            .iter()
            .filter_map(|sha256| objects.remove(sha256).map(|_| sha256.clone()))
            .collect();
        Ok(removed)
    }
}

/// Type-erased immutable-object authority paired with [`crate::StorageProfile`].
pub struct ObjectStoreBox(Box<dyn ImmutableObjectStore>);

impl ObjectStoreBox {
    pub fn new(store: impl ImmutableObjectStore + 'static) -> Self {
        Self(Box::new(store))
    }
}

impl ImmutableObjectStore for ObjectStoreBox {
    fn put(&self, bytes: &[u8]) -> Result<VerifiedObject> {
        self.0.put(bytes)
    }

    fn open_verified(&self, reference: &ObjectReference) -> Result<Box<dyn Read + Send + '_>> {
        self.0.open_verified(reference)
    }

    fn put_verified_stream(
        &self,
        expected_sha256: &str,
        expected_length: u64,
        reader: &mut dyn Read,
    ) -> Result<VerifiedObject> {
        self.0
            .put_verified_stream(expected_sha256, expected_length, reader)
    }

    fn verify(&self, sha256: &str) -> Result<VerifiedObject> {
        self.0.verify(sha256)
    }

    fn get(&self, reference: &ObjectReference) -> Result<Vec<u8>> {
        self.0.get(reference)
    }

    fn verified_file(&self, reference: &ObjectReference) -> Result<Option<File>> {
        self.0.verified_file(reference)
    }

    fn inventory(&self, reachable: &BTreeSet<String>) -> Result<ObjectInventory> {
        self.0.inventory(reachable)
    }

    fn reclaim_orphans(&self, unreachable: &BTreeSet<String>) -> Result<Vec<String>> {
        self.0.reclaim_orphans(unreachable)
    }
}

#[derive(Debug, Clone)]
pub struct LocalObjectStore {
    root: AnchoredPath,
}

impl LocalObjectStore {
    /// Opens an object store, creating its directories when absent. Writers
    /// and restore targets use this; verification uses `open_existing` so a
    /// read-only check never creates bytes.
    pub fn open(root: impl AsRef<Path>) -> Result<Self> {
        Self::open_anchored(&AnchoredPath::from_path(root.as_ref())?)
    }

    pub fn open_anchored(root: &AnchoredPath) -> Result<Self> {
        root.create_dir_all()?;
        let root = root.pin_directory()?;
        root.join("objects/sha256").create_dir_all()?;
        root.join("staging").create_dir_all()?;
        root.join("quarantine").create_dir_all()?;
        root.sync_directory()?;
        Ok(Self { root })
    }

    /// Opens an existing store without creating any path.
    pub fn open_existing(root: impl AsRef<Path>) -> Result<Self> {
        Self::open_existing_anchored(&AnchoredPath::open_root(root.as_ref())?)
    }

    pub fn open_existing_anchored(root: &AnchoredPath) -> Result<Self> {
        let root = root.pin_directory()?;
        root.join("objects/sha256").pin_directory()?;
        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        self.root.diagnostic_path()
    }

    pub fn put(&self, bytes: &[u8]) -> Result<VerifiedObject> {
        self.put_with_hook(bytes, |_| Ok(()))
    }

    pub fn open_verified(&self, reference: &ObjectReference) -> Result<Box<dyn Read + Send + '_>> {
        Ok(Box::new(self.verified_file(reference)?))
    }

    /// Streams a known content address through the same stage/sync/publish
    /// boundary as ordinary local objects without materializing the payload.
    pub fn put_verified_stream(
        &self,
        expected_sha256: &str,
        expected_length: u64,
        reader: &mut dyn Read,
    ) -> Result<VerifiedObject> {
        let key = ObjectReference::canonical_key(expected_sha256).map_err(Error::from)?;
        let stage_name = format!(
            "{}-{}-transfer",
            std::process::id(),
            STAGE_ORDINAL.fetch_add(1, Ordering::Relaxed)
        );
        let stage_path = self.root.join("staging").join(stage_name);
        let mut stage = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&stage_path)?;
        let staged = (|| -> Result<()> {
            let mut hasher = digest::Sha256::new();
            let mut length = 0u64;
            let mut buffer = [0u8; 64 * 1024];
            loop {
                let read = reader.read(&mut buffer)?;
                if read == 0 {
                    break;
                }
                length = length
                    .checked_add(read as u64)
                    .ok_or_else(|| Error::Object("object length overflowed u64".into()))?;
                if length > expected_length {
                    return Err(Error::ObjectLengthMismatch {
                        expected: expected_length,
                        actual: length,
                    });
                }
                hasher.update(&buffer[..read]);
                stage.write_all(&buffer[..read])?;
            }
            if length != expected_length {
                return Err(Error::ObjectLengthMismatch {
                    expected: expected_length,
                    actual: length,
                });
            }
            let actual = hasher.finalize_hex();
            if actual != expected_sha256 {
                return Err(Error::ObjectCorrupt {
                    expected: expected_sha256.to_owned(),
                    actual,
                });
            }
            stage.sync_all()?;
            self.root.join("staging").sync_directory()?;
            Ok(())
        })();
        if let Err(error) = staged {
            drop(stage);
            let _ = stage_path.remove_file();
            return Err(error);
        }
        drop(stage);
        let final_path = self.root.join(key);
        self.publish_stage(&stage_path, &final_path, expected_sha256)?;
        let verified = self.verify(expected_sha256)?;
        if verified.length != expected_length {
            return Err(Error::ObjectLengthMismatch {
                expected: expected_length,
                actual: verified.length,
            });
        }
        Ok(verified)
    }

    /// Publishes one immutable file without materializing it as a `Vec`.
    pub fn put_file(&self, source: impl AsRef<Path>) -> Result<VerifiedObject> {
        self.put_file_with_hook(source, |_| Ok(()))
    }

    pub fn put_file_with_hook(
        &self,
        source: impl AsRef<Path>,
        hook: impl FnMut(ObjectStep) -> Result<()>,
    ) -> Result<VerifiedObject> {
        let mut source = AnchoredPath::from_path(source.as_ref())?.open_read()?;
        self.put_reader_with_hook(&mut source, hook)
    }

    pub fn put_reader(&self, source: &mut dyn Read) -> Result<VerifiedObject> {
        self.put_reader_with_hook(source, |_| Ok(()))
    }

    fn put_reader_with_hook(
        &self,
        source: &mut dyn Read,
        mut hook: impl FnMut(ObjectStep) -> Result<()>,
    ) -> Result<VerifiedObject> {
        hook(ObjectStep::BeforeStageWrite)?;
        let stage_name = format!(
            "{}-{}-stream",
            std::process::id(),
            STAGE_ORDINAL.fetch_add(1, Ordering::Relaxed)
        );
        let stage_path = self.root.join("staging").join(stage_name);
        let mut stage = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&stage_path)?;
        let mut digest = digest::Sha256::new();
        let mut length = 0u64;
        let mut buffer = [0u8; 64 * 1024];
        let staged = (|| -> Result<(String, u64)> {
            loop {
                let read = source.read(&mut buffer)?;
                if read == 0 {
                    break;
                }
                length = length
                    .checked_add(read as u64)
                    .ok_or_else(|| Error::Object("object length overflowed u64".into()))?;
                digest.update(&buffer[..read]);
                stage.write_all(&buffer[..read])?;
            }
            stage.sync_all()?;
            self.root.join("staging").sync_directory()?;
            Ok((digest.finalize_hex(), length))
        })();
        let (sha256, length) = match staged {
            Ok(value) => value,
            Err(error) => {
                drop(stage);
                let _ = stage_path.remove_file();
                return Err(error);
            }
        };
        drop(stage);
        hook(ObjectStep::AfterStageSync)?;
        let key = ObjectReference::canonical_key(&sha256).map_err(Error::from)?;
        let final_path = self.root.join(&key);
        self.publish_stage(&stage_path, &final_path, &sha256)?;
        hook(ObjectStep::AfterPublish)?;
        hook(ObjectStep::BeforeVerify)?;
        let verified = self.verify(&sha256)?;
        if verified.length != length {
            return Err(Error::ObjectLengthMismatch {
                expected: length,
                actual: verified.length,
            });
        }
        hook(ObjectStep::AfterVerify)?;
        Ok(verified)
    }

    /// Stages, syncs, atomically publishes, and verifies one immutable object.
    /// The hook exists so crash/failure tests can stop at every boundary.
    pub fn put_with_hook(
        &self,
        bytes: &[u8],
        mut hook: impl FnMut(ObjectStep) -> Result<()>,
    ) -> Result<VerifiedObject> {
        hook(ObjectStep::BeforeStageWrite)?;
        let sha256 = digest::sha256_hex(bytes);
        let key = ObjectReference::canonical_key(&sha256).map_err(Error::from)?;
        let final_path = self.root.join(&key);
        let stage_name = format!(
            "{}-{}-{}",
            std::process::id(),
            STAGE_ORDINAL.fetch_add(1, Ordering::Relaxed),
            sha256
        );
        let stage_path = self.root.join("staging").join(stage_name);
        let mut stage = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&stage_path)?;
        stage.write_all(bytes)?;
        stage.sync_all()?;
        drop(stage);
        self.root.join("staging").sync_directory()?;
        hook(ObjectStep::AfterStageSync)?;

        self.publish_stage(&stage_path, &final_path, &sha256)?;
        hook(ObjectStep::AfterPublish)?;
        hook(ObjectStep::BeforeVerify)?;
        let verified = self.verify(&sha256)?;
        hook(ObjectStep::AfterVerify)?;
        Ok(verified)
    }

    pub fn verify(&self, sha256: &str) -> Result<VerifiedObject> {
        self.verify_file(sha256).map(|(verified, _)| verified)
    }

    fn verify_file(&self, sha256: &str) -> Result<(VerifiedObject, File)> {
        let key = ObjectReference::canonical_key(sha256).map_err(Error::from)?;
        let path = self.root.join(&key);
        let mut file = path.open_read().map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                Error::ObjectMissing(sha256.to_owned())
            } else {
                Error::from(error)
            }
        })?;
        let (actual, length) = digest_reader(&mut file)?;
        if actual != sha256 {
            return Err(Error::ObjectCorrupt {
                expected: sha256.to_owned(),
                actual,
            });
        }
        file.seek(SeekFrom::Start(0))?;
        Ok((
            VerifiedObject {
                sha256: sha256.to_owned(),
                length,
                receipt: ObjectReceipt {
                    backend: "local".into(),
                    key,
                    version: None,
                    etag: Some(sha256.to_owned()),
                },
            },
            file,
        ))
    }

    pub fn get(&self, reference: &ObjectReference) -> Result<Vec<u8>> {
        let mut file = self.verified_file(reference)?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        Ok(bytes)
    }

    /// Returns the authenticated file, rewound for bounded reads or mapping.
    pub fn verified_file(&self, reference: &ObjectReference) -> Result<File> {
        reference.validate().map_err(Error::from)?;
        let (verified, file) = self.verify_file(&reference.sha256)?;
        if verified.length != reference.length {
            return Err(Error::ObjectLengthMismatch {
                expected: reference.length,
                actual: verified.length,
            });
        }
        Ok(file)
    }

    pub fn inventory(&self, reachable: &BTreeSet<String>) -> Result<ObjectInventory> {
        let mut entries = Vec::new();
        let sha_root = self.root.join("objects/sha256");
        for bucket in sorted_paths(&sha_root)? {
            if !bucket.metadata()?.is_dir() {
                continue;
            }
            for path in sorted_paths(&bucket)? {
                if !path.metadata()?.is_file() {
                    continue;
                }
                let Some(name) = path
                    .relative_path()
                    .file_name()
                    .and_then(|name| name.to_str())
                else {
                    continue;
                };
                let mut file = path.open_read()?;
                let (actual, length) = digest_reader(&mut file)?;
                let state = if actual != name {
                    ObjectInventoryState::Corrupt {
                        actual_sha256: actual,
                    }
                } else if reachable.contains(name) {
                    ObjectInventoryState::Reachable
                } else {
                    ObjectInventoryState::Orphan
                };
                entries.push(ObjectInventoryEntry {
                    sha256: name.to_owned(),
                    length,
                    state,
                });
            }
        }
        Ok(ObjectInventory {
            entries,
            staging_files: sorted_paths(&self.root.join("staging"))?
                .iter()
                .map(|path| path.diagnostic_path().to_owned())
                .collect(),
            quarantined_files: sorted_paths(&self.root.join("quarantine"))?
                .iter()
                .map(|path| path.diagnostic_path().to_owned())
                .collect(),
        })
    }

    /// Moves corrupt objects out of the addressable namespace. The returned
    /// path is operator evidence; no reachable reference is silently rewritten.
    pub fn quarantine(&self, sha256: &str) -> Result<PathBuf> {
        let key = ObjectReference::canonical_key(sha256).map_err(Error::from)?;
        let source = self.root.join(key);
        if !source.try_exists()? {
            return Err(Error::ObjectMissing(sha256.to_owned()));
        }
        let target = self.root.join("quarantine").join(format!(
            "{}-{}",
            sha256,
            STAGE_ORDINAL.fetch_add(1, Ordering::Relaxed)
        ));
        publish_object_path(&source, &target)?;
        Ok(target.diagnostic_path().to_owned())
    }

    /// Reclaims only digests the caller proved unreachable. A later M4
    /// coordinator supplies that set from current references and retention pins.
    pub fn reclaim_orphans(&self, unreachable: &BTreeSet<String>) -> Result<Vec<String>> {
        let mut removed = Vec::new();
        for sha256 in unreachable {
            let key = ObjectReference::canonical_key(sha256).map_err(Error::from)?;
            let path = self.root.join(key);
            if path.try_exists()? {
                path.remove_file()?;
                removed.push(sha256.clone());
            }
        }
        removed.sort();
        Ok(removed)
    }

    fn publish_stage(
        &self,
        stage_path: &AnchoredPath,
        final_path: &AnchoredPath,
        sha256: &str,
    ) -> Result<()> {
        let parent = final_path
            .parent()
            .ok_or_else(|| Error::Object("object publication target has no parent".into()))?;
        parent.create_dir_all()?;
        if final_path.try_exists()? {
            match self.verify(sha256) {
                Ok(_) => {
                    // Content addressing makes an existing verified value equivalent.
                    stage_path.remove_file()?;
                }
                Err(Error::ObjectCorrupt { .. }) => {
                    self.quarantine(sha256)?;
                    publish_object_path(stage_path, final_path)?;
                }
                Err(error) => return Err(error),
            }
        } else {
            publish_object_path(stage_path, final_path)?;
        }
        Ok(())
    }
}

fn digest_reader(reader: &mut impl Read) -> Result<(String, u64)> {
    let mut digest = digest::Sha256::new();
    let mut length = 0u64;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        length = length
            .checked_add(read as u64)
            .ok_or_else(|| Error::Object("object length overflowed u64".into()))?;
        digest.update(&buffer[..read]);
    }
    Ok((digest.finalize_hex(), length))
}

impl ImmutableObjectStore for LocalObjectStore {
    fn put(&self, bytes: &[u8]) -> Result<VerifiedObject> {
        LocalObjectStore::put(self, bytes)
    }

    fn open_verified(&self, reference: &ObjectReference) -> Result<Box<dyn Read + Send + '_>> {
        LocalObjectStore::open_verified(self, reference)
    }

    fn put_verified_stream(
        &self,
        expected_sha256: &str,
        expected_length: u64,
        reader: &mut dyn Read,
    ) -> Result<VerifiedObject> {
        LocalObjectStore::put_verified_stream(self, expected_sha256, expected_length, reader)
    }

    fn verify(&self, sha256: &str) -> Result<VerifiedObject> {
        LocalObjectStore::verify(self, sha256)
    }

    fn get(&self, reference: &ObjectReference) -> Result<Vec<u8>> {
        LocalObjectStore::get(self, reference)
    }

    fn verified_file(&self, reference: &ObjectReference) -> Result<Option<File>> {
        LocalObjectStore::verified_file(self, reference).map(Some)
    }

    fn inventory(&self, reachable: &BTreeSet<String>) -> Result<ObjectInventory> {
        LocalObjectStore::inventory(self, reachable)
    }

    fn reclaim_orphans(&self, unreachable: &BTreeSet<String>) -> Result<Vec<String>> {
        LocalObjectStore::reclaim_orphans(self, unreachable)
    }
}

fn sorted_paths(path: &AnchoredPath) -> Result<Vec<AnchoredPath>> {
    if !path.try_exists()? {
        return Ok(Vec::new());
    }
    path.read_dir().map_err(Error::from)
}

fn publish_object_path(source: &AnchoredPath, target: &AnchoredPath) -> Result<()> {
    source.rename_to(target).map_err(Error::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn memory_objects_keep_the_same_content_address_and_receipt_boundary() {
        let store = MemoryObjectStore::new();
        let verified = store.put(b"memory canonical bytes").unwrap();
        assert_eq!(verified.receipt.backend, "memory");
        let reference = ObjectReference::for_bytes(
            "memory-object",
            None,
            "application/octet-stream",
            b"memory canonical bytes",
            verified.receipt.clone(),
        )
        .unwrap();
        assert_eq!(store.get(&reference).unwrap(), b"memory canonical bytes");
        assert_eq!(store.verify(&verified.sha256).unwrap(), verified);

        let removed = store
            .reclaim_orphans(&BTreeSet::from([reference.sha256.clone()]))
            .unwrap();
        assert_eq!(removed, vec![reference.sha256.clone()]);
        assert!(matches!(
            store.verify(&reference.sha256),
            Err(Error::ObjectMissing(_))
        ));
    }

    #[test]
    fn local_put_is_content_addressed_idempotent_and_verified() {
        let directory = tempdir().unwrap();
        let store = LocalObjectStore::open(directory.path()).unwrap();
        let first = store.put(b"canonical bytes").unwrap();
        let second = store.put(b"canonical bytes").unwrap();
        assert_eq!(first, second);
        assert_eq!(store.verify(&first.sha256).unwrap(), first);
        assert!(
            store.inventory(&BTreeSet::new()).unwrap().entries[0].state
                == ObjectInventoryState::Orphan
        );
    }

    #[test]
    fn local_file_put_streams_to_the_same_content_address() {
        let directory = tempdir().unwrap();
        let source = directory.path().join("source.bin");
        let bytes = vec![0x5a; 256 * 1024 + 17];
        fs::write(&source, &bytes).unwrap();
        let store = LocalObjectStore::open(directory.path().join("objects")).unwrap();
        let streamed = store.put_file(&source).unwrap();
        let in_memory = store.put(&bytes).unwrap();
        assert_eq!(streamed, in_memory);

        let reference = ObjectReference::for_verified(
            "streamed",
            None,
            "application/octet-stream",
            streamed.sha256.clone(),
            streamed.length,
            streamed.receipt.clone(),
        )
        .unwrap();
        let mut verified = store.verified_file(&reference).unwrap();
        let mut actual = Vec::new();
        verified.read_to_end(&mut actual).unwrap();
        assert_eq!(actual, bytes);
    }

    #[test]
    fn verified_handle_keeps_authenticated_bytes_after_leaf_replacement() {
        let directory = tempdir().unwrap();
        let store = LocalObjectStore::open(directory.path()).unwrap();
        let object = store.put(b"authenticated original").unwrap();
        let reference = ObjectReference::for_verified(
            "retained",
            None,
            "application/octet-stream",
            object.sha256,
            object.length,
            object.receipt,
        )
        .unwrap();
        let mut file = store.verified_file(&reference).unwrap();
        let path = store.root().join(&reference.receipt.key);
        fs::rename(&path, path.with_extension("retained")).unwrap();
        fs::write(&path, b"substituted contents").unwrap();
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).unwrap();
        assert_eq!(bytes, b"authenticated original");
        assert!(store.verified_file(&reference).is_err());
    }

    #[test]
    fn verified_stream_is_bounded_content_addressed_and_fail_closed() {
        let directory = tempdir().unwrap();
        let store = LocalObjectStore::open(directory.path()).unwrap();
        let bytes = vec![0x3c; 192 * 1024 + 9];
        let sha256 = digest::sha256_hex(&bytes);
        let mut reader = std::io::Cursor::new(bytes.clone());
        let streamed = store
            .put_verified_stream(&sha256, bytes.len() as u64, &mut reader)
            .unwrap();
        assert_eq!(streamed, store.put(&bytes).unwrap());

        let wrong = "11".repeat(32);
        let mut substituted = std::io::Cursor::new(bytes.clone());
        assert!(matches!(
            store.put_verified_stream(&wrong, bytes.len() as u64, &mut substituted),
            Err(Error::ObjectCorrupt { .. })
        ));
        assert!(matches!(store.verify(&wrong), Err(Error::ObjectMissing(_))));

        let mut truncated = std::io::Cursor::new(&bytes[..bytes.len() - 1]);
        assert!(matches!(
            store.put_verified_stream(&sha256, bytes.len() as u64, &mut truncated),
            Err(Error::ObjectLengthMismatch { .. })
        ));
        assert!(store
            .inventory(&BTreeSet::new())
            .unwrap()
            .staging_files
            .is_empty());
    }

    #[test]
    fn failed_after_sync_remains_staged_and_never_visible() {
        let directory = tempdir().unwrap();
        let store = LocalObjectStore::open(directory.path()).unwrap();
        let result = store.put_with_hook(b"not published", |step| {
            if step == ObjectStep::AfterStageSync {
                Err(Error::FaultInjected("after_stage_sync"))
            } else {
                Ok(())
            }
        });
        assert!(matches!(result, Err(Error::FaultInjected(_))));
        let inventory = store.inventory(&BTreeSet::new()).unwrap();
        assert_eq!(inventory.entries.len(), 0);
        assert_eq!(inventory.staging_files.len(), 1);
    }

    #[test]
    fn corrupt_object_is_reported_and_quarantined() {
        let directory = tempdir().unwrap();
        let store = LocalObjectStore::open(directory.path()).unwrap();
        let object = store.put(b"valid").unwrap();
        fs::write(store.root().join(&object.receipt.key), b"corrupt").unwrap();
        assert!(matches!(
            store.verify(&object.sha256),
            Err(Error::ObjectCorrupt { .. })
        ));
        let target = store.quarantine(&object.sha256).unwrap();
        assert!(target.exists());
        assert!(matches!(
            store.verify(&object.sha256),
            Err(Error::ObjectMissing(_))
        ));
    }

    #[test]
    fn every_publication_boundary_has_explicit_recovery_state() {
        for point in [
            ObjectStep::BeforeStageWrite,
            ObjectStep::AfterStageSync,
            ObjectStep::AfterPublish,
            ObjectStep::BeforeVerify,
            ObjectStep::AfterVerify,
        ] {
            let directory = tempdir().unwrap();
            let store = LocalObjectStore::open(directory.path()).unwrap();
            let result = store.put_with_hook(b"boundary", |step| {
                if step == point {
                    Err(Error::FaultInjected("object_boundary"))
                } else {
                    Ok(())
                }
            });
            assert!(matches!(result, Err(Error::FaultInjected(_))));
            let inventory = store.inventory(&BTreeSet::new()).unwrap();
            match point {
                ObjectStep::BeforeStageWrite => {
                    assert!(inventory.entries.is_empty());
                    assert!(inventory.staging_files.is_empty());
                }
                ObjectStep::AfterStageSync => {
                    assert!(inventory.entries.is_empty());
                    assert_eq!(inventory.staging_files.len(), 1);
                }
                ObjectStep::AfterPublish | ObjectStep::BeforeVerify | ObjectStep::AfterVerify => {
                    assert_eq!(inventory.entries.len(), 1);
                    assert_eq!(inventory.entries[0].state, ObjectInventoryState::Orphan);
                }
            }
        }
    }

    #[test]
    fn explicit_orphan_reclamation_removes_only_proven_candidates() {
        let directory = tempdir().unwrap();
        let store = LocalObjectStore::open(directory.path()).unwrap();
        let retained = store.put(b"retained").unwrap();
        let orphan = store.put(b"orphan").unwrap();
        let removed = store
            .reclaim_orphans(&BTreeSet::from([orphan.sha256.clone()]))
            .unwrap();
        assert_eq!(removed, vec![orphan.sha256]);
        assert!(store.verify(&retained.sha256).is_ok());
    }
}
