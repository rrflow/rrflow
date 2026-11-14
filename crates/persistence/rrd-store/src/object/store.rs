//! The immutable object store trait, verified objects, inventories and the in-memory and boxed stores.

use super::*;

pub(super) static STAGE_ORDINAL: AtomicU64 = AtomicU64::new(1);

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
