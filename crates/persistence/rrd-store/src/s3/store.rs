//! The S3-compatible immutable object store, range reads and upload validation.

use super::*;

pub struct S3CompatibleObjectStore<C> {
    pub(super) client: C,
    pub(super) backend_name: String,
    pub(super) policy: S3TransferPolicy,
}

impl<C: S3ObjectClient> S3CompatibleObjectStore<C> {
    pub fn new(client: C, backend_name: impl Into<String>) -> Result<Self> {
        Self::with_policy(client, backend_name, S3TransferPolicy::default())
    }

    pub fn with_policy(
        client: C,
        backend_name: impl Into<String>,
        policy: S3TransferPolicy,
    ) -> Result<Self> {
        client.capabilities().validate()?;
        let backend_name = backend_name.into();
        if backend_name.trim().is_empty() || backend_name.as_bytes().contains(&0) {
            return Err(Error::Object(
                "S3-compatible backend name must be non-empty and contain no NUL bytes".into(),
            ));
        }
        Ok(Self {
            client,
            backend_name,
            policy: policy.validate()?,
        })
    }

    pub fn put(&self, bytes: &[u8]) -> Result<VerifiedObject> {
        let sha256 = digest::sha256_hex(bytes);
        let key = ObjectReference::canonical_key(&sha256).map_err(Error::from)?;
        let metadata = match self.retry(|| self.client.put_if_absent(&key, bytes))? {
            ConditionalPut::Created(metadata) | ConditionalPut::AlreadyExists(metadata) => metadata,
        };
        validate_metadata(&key, bytes.len() as u64, &sha256, &metadata)?;
        self.verify_expected(&sha256, Some(bytes.len() as u64))
    }

    pub fn open_verified(&self, reference: &ObjectReference) -> Result<Box<dyn Read + Send + '_>> {
        reference.validate().map_err(Error::from)?;
        self.verify_expected(&reference.sha256, Some(reference.length))?;
        Ok(Box::new(S3RangeReader {
            store: self,
            key: reference.receipt.key.clone(),
            version: reference.receipt.version.clone(),
            length: reference.length,
            offset: 0,
            buffered: Cursor::new(Vec::new()),
        }))
    }

    /// Uploads one known content address with bounded memory. A retry discovers
    /// the prior multipart upload and reuses only parts whose exact bytes match
    /// the restarted source stream.
    pub fn put_verified_stream(
        &self,
        expected_sha256: &str,
        expected_length: u64,
        reader: &mut dyn Read,
    ) -> Result<VerifiedObject> {
        let key = ObjectReference::canonical_key(expected_sha256).map_err(Error::from)?;
        if expected_length == 0 {
            ensure_empty(reader)?;
            let actual = digest::sha256_hex(b"");
            if expected_sha256 != actual {
                return Err(Error::ObjectCorrupt {
                    expected: expected_sha256.to_owned(),
                    actual,
                });
            }
            return self.put(b"");
        }
        let part_count = expected_length.div_ceil(self.policy.part_bytes as u64);
        if part_count > u64::from(S3_MAX_PARTS) {
            return Err(Error::Object(format!(
                "object requires {part_count} parts beyond the S3 limit {S3_MAX_PARTS}"
            )));
        }

        let upload = match self.retry(|| {
            self.client
                .find_multipart(&key, expected_sha256, expected_length)
        })? {
            Some(upload) => upload,
            None => self.retry(|| {
                self.client
                    .create_multipart(&key, expected_sha256, expected_length)
            })?,
        };
        validate_upload(&key, expected_sha256, expected_length, &upload)?;
        let existing = indexed_parts(self.retry(|| self.client.list_parts(&upload))?)?;
        if existing.keys().next_back().copied().unwrap_or(0) > part_count as u32 {
            return Err(Error::Object(
                "remote multipart upload contains a part beyond the expected object".into(),
            ));
        }

        let mut full_digest = digest::Sha256::new();
        let mut completed = Vec::with_capacity(part_count as usize);
        let mut remaining = expected_length;
        for part_number in 1..=part_count as u32 {
            let expected_part_length = remaining.min(self.policy.part_bytes as u64) as usize;
            let bytes = read_exact_part(reader, expected_part_length)?;
            remaining -= bytes.len() as u64;
            full_digest.update(&bytes);
            let sha256 = digest::sha256_hex(&bytes);
            let part = match existing.get(&part_number) {
                Some(part) if part.length == bytes.len() as u64 && part.sha256 == sha256 => {
                    part.clone()
                }
                _ => self.retry(|| {
                    self.client
                        .upload_part(&upload, part_number, &bytes, &sha256)
                })?,
            };
            validate_part(part_number, bytes.len() as u64, &sha256, &part)?;
            completed.push(part);
        }
        ensure_empty(reader)?;
        let actual = full_digest.finalize_hex();
        if actual != expected_sha256 {
            return Err(Error::ObjectCorrupt {
                expected: expected_sha256.to_owned(),
                actual,
            });
        }
        validate_completed_parts(&completed, expected_length)?;
        let metadata = match self.retry(|| {
            self.client
                .complete_multipart_if_absent(&upload, &completed)
        })? {
            ConditionalPut::Created(metadata) | ConditionalPut::AlreadyExists(metadata) => metadata,
        };
        validate_metadata(&key, expected_length, expected_sha256, &metadata)?;
        self.verify_expected(expected_sha256, Some(expected_length))
    }

    pub fn verify(&self, sha256: &str) -> Result<VerifiedObject> {
        self.verify_expected(sha256, None)
    }

    pub fn get(&self, reference: &ObjectReference) -> Result<Vec<u8>> {
        reference.validate().map_err(Error::from)?;
        self.verify_expected(&reference.sha256, Some(reference.length))?;
        let bytes = self
            .retry(|| {
                self.client
                    .get(&reference.receipt.key, reference.receipt.version.as_deref())
            })?
            .ok_or_else(|| Error::ObjectMissing(reference.sha256.clone()))?;
        if bytes.len() as u64 != reference.length {
            return Err(Error::ObjectLengthMismatch {
                expected: reference.length,
                actual: bytes.len() as u64,
            });
        }
        let actual = digest::sha256_hex(&bytes);
        if actual != reference.sha256 {
            return Err(Error::ObjectCorrupt {
                expected: reference.sha256.clone(),
                actual,
            });
        }
        Ok(bytes)
    }

    pub fn inventory(&self, reachable: &BTreeSet<String>) -> Result<ObjectInventory> {
        let mut entries = self
            .retry(|| self.client.list("objects/sha256/"))?
            .into_iter()
            .map(|metadata| {
                let sha256 = metadata
                    .key
                    .rsplit('/')
                    .next()
                    .unwrap_or_default()
                    .to_owned();
                let (actual, length) = self.digest_remote(&metadata)?;
                let state = if actual != sha256 {
                    ObjectInventoryState::Corrupt {
                        actual_sha256: actual,
                    }
                } else if reachable.contains(&sha256) {
                    ObjectInventoryState::Reachable
                } else {
                    ObjectInventoryState::Orphan
                };
                Ok(ObjectInventoryEntry {
                    sha256,
                    length,
                    state,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        entries.sort_by(|left, right| left.sha256.cmp(&right.sha256));
        Ok(ObjectInventory {
            entries,
            staging_files: Vec::new(),
            quarantined_files: Vec::new(),
        })
    }

    pub fn reclaim_orphans(&self, unreachable: &BTreeSet<String>) -> Result<Vec<String>> {
        let mut removed = Vec::new();
        for sha256 in unreachable {
            let key = ObjectReference::canonical_key(sha256).map_err(Error::from)?;
            if self.retry(|| self.client.head(&key))?.is_some() {
                self.retry(|| self.client.delete(&key))?;
                removed.push(sha256.clone());
            }
        }
        removed.sort();
        Ok(removed)
    }

    pub fn into_client(self) -> C {
        self.client
    }

    fn verify_expected(&self, sha256: &str, length: Option<u64>) -> Result<VerifiedObject> {
        let key = ObjectReference::canonical_key(sha256).map_err(Error::from)?;
        let metadata = self
            .retry(|| self.client.head(&key))?
            .ok_or_else(|| Error::ObjectMissing(sha256.to_owned()))?;
        let expected_length = length.unwrap_or(metadata.length);
        validate_metadata(&key, expected_length, sha256, &metadata)?;
        let (actual, actual_length) = self.digest_remote(&metadata)?;
        if actual_length != expected_length {
            return Err(Error::ObjectLengthMismatch {
                expected: expected_length,
                actual: actual_length,
            });
        }
        if actual != sha256 {
            return Err(Error::ObjectCorrupt {
                expected: sha256.to_owned(),
                actual,
            });
        }
        Ok(VerifiedObject {
            sha256: sha256.to_owned(),
            length: actual_length,
            receipt: ObjectReceipt {
                backend: self.backend_name.clone(),
                key,
                version: metadata.version,
                etag: metadata.etag,
            },
        })
    }

    fn digest_remote(&self, metadata: &S3ObjectMetadata) -> Result<(String, u64)> {
        let mut hasher = digest::Sha256::new();
        let mut offset = 0u64;
        while offset < metadata.length {
            let take =
                usize::try_from((metadata.length - offset).min(self.policy.range_bytes as u64))
                    .expect("range is bounded by usize policy");
            let bytes = self
                .retry(|| {
                    self.client
                        .get_range(&metadata.key, metadata.version.as_deref(), offset, take)
                })?
                .ok_or_else(|| Error::ObjectMissing(metadata.key.clone()))?;
            if bytes.len() != take {
                return Err(Error::ObjectLengthMismatch {
                    expected: take as u64,
                    actual: bytes.len() as u64,
                });
            }
            hasher.update(&bytes);
            offset += bytes.len() as u64;
        }
        Ok((hasher.finalize_hex(), offset))
    }

    fn retry<T>(&self, mut operation: impl FnMut() -> Result<T>) -> Result<T> {
        for attempt in 1..=self.policy.max_attempts {
            match operation() {
                Err(Error::RemoteObjectTransient(_)) if attempt < self.policy.max_attempts => {}
                result => return result,
            }
        }
        unreachable!("positive bounded attempt count returns from its final attempt")
    }
}

impl<C: S3ObjectClient> ImmutableObjectStore for S3CompatibleObjectStore<C> {
    fn put(&self, bytes: &[u8]) -> Result<VerifiedObject> {
        S3CompatibleObjectStore::put(self, bytes)
    }

    fn open_verified(&self, reference: &ObjectReference) -> Result<Box<dyn Read + Send + '_>> {
        S3CompatibleObjectStore::open_verified(self, reference)
    }

    fn put_verified_stream(
        &self,
        expected_sha256: &str,
        expected_length: u64,
        reader: &mut dyn Read,
    ) -> Result<VerifiedObject> {
        S3CompatibleObjectStore::put_verified_stream(self, expected_sha256, expected_length, reader)
    }

    fn verify(&self, sha256: &str) -> Result<VerifiedObject> {
        S3CompatibleObjectStore::verify(self, sha256)
    }

    fn get(&self, reference: &ObjectReference) -> Result<Vec<u8>> {
        S3CompatibleObjectStore::get(self, reference)
    }

    fn inventory(&self, reachable: &BTreeSet<String>) -> Result<ObjectInventory> {
        S3CompatibleObjectStore::inventory(self, reachable)
    }

    fn reclaim_orphans(&self, unreachable: &BTreeSet<String>) -> Result<Vec<String>> {
        S3CompatibleObjectStore::reclaim_orphans(self, unreachable)
    }
}

pub(super) struct S3RangeReader<'a, C> {
    store: &'a S3CompatibleObjectStore<C>,
    key: String,
    version: Option<String>,
    length: u64,
    offset: u64,
    buffered: Cursor<Vec<u8>>,
}

impl<C: S3ObjectClient> Read for S3RangeReader<'_, C> {
    fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
        if output.is_empty() {
            return Ok(0);
        }
        let buffered = self.buffered.read(output)?;
        if buffered != 0 {
            return Ok(buffered);
        }
        if self.offset >= self.length {
            return Ok(0);
        }
        let take =
            usize::try_from((self.length - self.offset).min(self.store.policy.range_bytes as u64))
                .expect("range is bounded by usize policy");
        let bytes = self
            .store
            .retry(|| {
                self.store
                    .client
                    .get_range(&self.key, self.version.as_deref(), self.offset, take)
            })
            .map_err(std::io::Error::other)?
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "object missing"))?;
        if bytes.len() != take {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                format!("range returned {} of {take} bytes", bytes.len()),
            ));
        }
        self.offset += bytes.len() as u64;
        self.buffered = Cursor::new(bytes);
        self.buffered.read(output)
    }
}

pub(super) fn validate_metadata(
    key: &str,
    expected_length: u64,
    expected_sha256: &str,
    metadata: &S3ObjectMetadata,
) -> Result<()> {
    if metadata.key != key {
        return Err(Error::Object(format!(
            "S3 response key {:?} differs from requested key {key:?}",
            metadata.key
        )));
    }
    if metadata.length != expected_length {
        return Err(Error::ObjectLengthMismatch {
            expected: expected_length,
            actual: metadata.length,
        });
    }
    if metadata.checksum_sha256.as_deref() != Some(expected_sha256) {
        return Err(Error::Object(format!(
            "S3 response for {key} lacks the exact expected SHA-256 checksum"
        )));
    }
    Ok(())
}

pub(super) fn validate_upload(
    key: &str,
    expected_sha256: &str,
    expected_length: u64,
    upload: &S3MultipartUpload,
) -> Result<()> {
    if upload.key != key
        || upload.expected_sha256 != expected_sha256
        || upload.expected_length != expected_length
        || upload.upload_id.trim().is_empty()
    {
        return Err(Error::Object(
            "S3 multipart identity differs from the requested content address".into(),
        ));
    }
    Ok(())
}

pub(super) fn indexed_parts(parts: Vec<S3UploadedPart>) -> Result<BTreeMap<u32, S3UploadedPart>> {
    let mut indexed = BTreeMap::new();
    for part in parts {
        if part.part_number == 0 || part.part_number > S3_MAX_PARTS {
            return Err(Error::Object("S3 multipart part number is invalid".into()));
        }
        let number = part.part_number;
        if indexed.insert(number, part).is_some() {
            return Err(Error::Object(
                "S3 multipart listing returned a duplicate part".into(),
            ));
        }
    }
    Ok(indexed)
}

pub(super) fn validate_part(
    part_number: u32,
    expected_length: u64,
    expected_sha256: &str,
    part: &S3UploadedPart,
) -> Result<()> {
    if part.part_number != part_number
        || part.length != expected_length
        || part.sha256 != expected_sha256
        || part.etag.trim().is_empty()
    {
        return Err(Error::Object(format!(
            "S3 multipart part {part_number} returned divergent checksum, length, number, or ETag"
        )));
    }
    Ok(())
}

pub(super) fn validate_completed_parts(
    parts: &[S3UploadedPart],
    expected_length: u64,
) -> Result<()> {
    let mut length = 0u64;
    for (index, part) in parts.iter().enumerate() {
        if part.part_number != u32::try_from(index + 1).expect("S3 part count fits u32") {
            return Err(Error::Object(
                "S3 multipart completion requires consecutive parts beginning at one".into(),
            ));
        }
        length = length
            .checked_add(part.length)
            .ok_or_else(|| Error::Object("S3 multipart length overflowed".into()))?;
    }
    if length != expected_length {
        return Err(Error::ObjectLengthMismatch {
            expected: expected_length,
            actual: length,
        });
    }
    Ok(())
}

pub(super) fn read_exact_part(reader: &mut dyn Read, length: usize) -> Result<Vec<u8>> {
    let mut bytes = vec![0; length];
    let mut offset = 0;
    while offset < length {
        let read = reader.read(&mut bytes[offset..])?;
        if read == 0 {
            return Err(Error::ObjectLengthMismatch {
                expected: length as u64,
                actual: offset as u64,
            });
        }
        offset += read;
    }
    Ok(bytes)
}

pub(super) fn ensure_empty(reader: &mut dyn Read) -> Result<()> {
    let mut extra = [0u8; 1];
    if reader.read(&mut extra)? != 0 {
        return Err(Error::Object(
            "object stream contains bytes beyond its declared length".into(),
        ));
    }
    Ok(())
}
