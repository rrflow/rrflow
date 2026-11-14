//! The local filesystem object store and durable publication.

use super::*;

#[derive(Debug, Clone)]
pub struct LocalObjectStore {
    pub(super) root: AnchoredPath,
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
        source.rename_to(&target)?;
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
                    publish_object_path(stage_path, final_path, sha256)?;
                }
                Err(error) => return Err(error),
            }
        } else {
            publish_object_path(stage_path, final_path, sha256)?;
        }
        Ok(())
    }
}

pub(super) fn digest_reader(reader: &mut impl Read) -> Result<(String, u64)> {
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

pub(super) fn sorted_paths(path: &AnchoredPath) -> Result<Vec<AnchoredPath>> {
    if !path.try_exists()? {
        return Ok(Vec::new());
    }
    path.read_dir().map_err(Error::from)
}

pub(super) fn publish_object_path(
    source: &AnchoredPath,
    target: &AnchoredPath,
    expected_sha256: &str,
) -> Result<()> {
    match source.rename_new_to(target) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            // Another publisher can win after the caller observed absence.
            // Reuse only verified content, preserving its file identity and readers.
            let mut winner = target.open_read()?;
            let (actual, _) = digest_reader(&mut winner)?;
            if actual != expected_sha256 {
                return Err(Error::ObjectCorrupt {
                    expected: expected_sha256.to_owned(),
                    actual,
                });
            }
            target
                .parent()
                .ok_or_else(|| Error::Object("object publication target has no parent".into()))?
                .sync_directory()?;
            source.remove_file()?;
            Ok(())
        }
        Err(error) => Err(Error::from(error)),
    }
}
