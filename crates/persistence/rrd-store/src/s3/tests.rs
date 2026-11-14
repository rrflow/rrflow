use super::*;
use crate::{export_logical_archive, LocalObjectStore, RrflowMxStore};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use tempfile::tempdir;

#[derive(Default)]
struct PendingUpload {
    upload: Option<S3MultipartUpload>,
    parts: BTreeMap<u32, (S3UploadedPart, Vec<u8>)>,
}

struct MemoryS3Client {
    capabilities: S3TransportCapabilities,
    objects: Mutex<BTreeMap<String, Vec<u8>>>,
    pending: Mutex<PendingUpload>,
    transient_part_failures: Mutex<BTreeMap<u32, u32>>,
    transient_range_failures: Mutex<u32>,
    part_uploads: Mutex<BTreeMap<u32, u32>>,
    max_range_bytes: AtomicU64,
    upload_id: AtomicU64,
}

impl Default for MemoryS3Client {
    fn default() -> Self {
        Self {
            capabilities: S3TransportCapabilities::authenticated_v4(),
            objects: Mutex::new(BTreeMap::new()),
            pending: Mutex::new(PendingUpload::default()),
            transient_part_failures: Mutex::new(BTreeMap::new()),
            transient_range_failures: Mutex::new(0),
            part_uploads: Mutex::new(BTreeMap::new()),
            max_range_bytes: AtomicU64::new(0),
            upload_id: AtomicU64::new(1),
        }
    }
}

impl MemoryS3Client {
    fn metadata(key: &str, bytes: &[u8]) -> S3ObjectMetadata {
        S3ObjectMetadata {
            key: key.to_owned(),
            length: bytes.len() as u64,
            etag: Some(format!("etag-{}", digest::sha256_hex(bytes))),
            version: Some("1".into()),
            checksum_sha256: Some(digest::sha256_hex(bytes)),
        }
    }

    fn fail_part(&self, part_number: u32, failures: u32) {
        self.transient_part_failures
            .lock()
            .unwrap()
            .insert(part_number, failures);
    }

    fn fail_ranges(&self, failures: u32) {
        *self.transient_range_failures.lock().unwrap() = failures;
    }
}

impl S3ObjectClient for MemoryS3Client {
    fn capabilities(&self) -> S3TransportCapabilities {
        self.capabilities
    }

    fn put_if_absent(&self, key: &str, bytes: &[u8]) -> Result<ConditionalPut> {
        let mut objects = self.objects.lock().unwrap();
        let created = !objects.contains_key(key);
        let stored = objects.entry(key.into()).or_insert_with(|| bytes.to_vec());
        let metadata = Self::metadata(key, stored);
        Ok(if created {
            ConditionalPut::Created(metadata)
        } else {
            ConditionalPut::AlreadyExists(metadata)
        })
    }

    fn head(&self, key: &str) -> Result<Option<S3ObjectMetadata>> {
        Ok(self
            .objects
            .lock()
            .unwrap()
            .get(key)
            .map(|bytes| Self::metadata(key, bytes)))
    }

    fn get(&self, key: &str, version: Option<&str>) -> Result<Option<Vec<u8>>> {
        if version.is_some_and(|version| version != "1") {
            return Ok(None);
        }
        Ok(self.objects.lock().unwrap().get(key).cloned())
    }

    fn get_range(
        &self,
        key: &str,
        version: Option<&str>,
        offset: u64,
        length: usize,
    ) -> Result<Option<Vec<u8>>> {
        let mut failures = self.transient_range_failures.lock().unwrap();
        if *failures != 0 {
            *failures -= 1;
            return Err(Error::RemoteObjectTransient(
                "ranged read unavailable".into(),
            ));
        }
        drop(failures);
        self.max_range_bytes
            .fetch_max(length as u64, Ordering::Relaxed);
        if version.is_some_and(|version| version != "1") {
            return Ok(None);
        }
        let objects = self.objects.lock().unwrap();
        let Some(bytes) = objects.get(key) else {
            return Ok(None);
        };
        let start = usize::try_from(offset).map_err(|_| Error::Object("offset overflow".into()))?;
        let end = start.saturating_add(length).min(bytes.len());
        Ok(Some(bytes.get(start..end).unwrap_or_default().to_vec()))
    }

    fn list(&self, prefix: &str) -> Result<Vec<S3ObjectMetadata>> {
        Ok(self
            .objects
            .lock()
            .unwrap()
            .iter()
            .filter(|(key, _)| key.starts_with(prefix))
            .map(|(key, bytes)| Self::metadata(key, bytes))
            .collect())
    }

    fn delete(&self, key: &str) -> Result<()> {
        self.objects.lock().unwrap().remove(key);
        Ok(())
    }

    fn find_multipart(
        &self,
        key: &str,
        expected_sha256: &str,
        expected_length: u64,
    ) -> Result<Option<S3MultipartUpload>> {
        Ok(self
            .pending
            .lock()
            .unwrap()
            .upload
            .clone()
            .filter(|upload| {
                upload.key == key
                    && upload.expected_sha256 == expected_sha256
                    && upload.expected_length == expected_length
            }))
    }

    fn create_multipart(
        &self,
        key: &str,
        expected_sha256: &str,
        expected_length: u64,
    ) -> Result<S3MultipartUpload> {
        let upload = S3MultipartUpload {
            key: key.into(),
            upload_id: format!("upload-{}", self.upload_id.fetch_add(1, Ordering::Relaxed)),
            expected_sha256: expected_sha256.into(),
            expected_length,
        };
        let mut pending = self.pending.lock().unwrap();
        pending.upload = Some(upload.clone());
        pending.parts.clear();
        Ok(upload)
    }

    fn list_parts(&self, upload: &S3MultipartUpload) -> Result<Vec<S3UploadedPart>> {
        let pending = self.pending.lock().unwrap();
        if pending.upload.as_ref() != Some(upload) {
            return Err(Error::Object("unknown multipart upload".into()));
        }
        Ok(pending
            .parts
            .values()
            .map(|(part, _)| part.clone())
            .collect())
    }

    fn upload_part(
        &self,
        upload: &S3MultipartUpload,
        part_number: u32,
        bytes: &[u8],
        sha256: &str,
    ) -> Result<S3UploadedPart> {
        let mut failures = self.transient_part_failures.lock().unwrap();
        if failures.get(&part_number).copied().unwrap_or(0) > 0 {
            *failures.get_mut(&part_number).unwrap() -= 1;
            return Err(Error::RemoteObjectTransient(format!(
                "part {part_number} unavailable"
            )));
        }
        drop(failures);
        *self
            .part_uploads
            .lock()
            .unwrap()
            .entry(part_number)
            .or_default() += 1;
        let mut pending = self.pending.lock().unwrap();
        if pending.upload.as_ref() != Some(upload) {
            return Err(Error::Object("unknown multipart upload".into()));
        }
        if digest::sha256_hex(bytes) != sha256 {
            return Err(Error::Object("part checksum is invalid".into()));
        }
        let part = S3UploadedPart {
            part_number,
            length: bytes.len() as u64,
            sha256: sha256.into(),
            etag: format!("part-{part_number}-{sha256}"),
        };
        pending
            .parts
            .insert(part_number, (part.clone(), bytes.to_vec()));
        Ok(part)
    }

    fn complete_multipart_if_absent(
        &self,
        upload: &S3MultipartUpload,
        parts: &[S3UploadedPart],
    ) -> Result<ConditionalPut> {
        let mut pending = self.pending.lock().unwrap();
        if pending.upload.as_ref() != Some(upload) {
            return Err(Error::Object("unknown multipart upload".into()));
        }
        let mut bytes = Vec::new();
        for part in parts {
            let (stored, value) = pending
                .parts
                .get(&part.part_number)
                .ok_or_else(|| Error::Object("completion part is absent".into()))?;
            if stored != part {
                return Err(Error::Object("completion part diverged".into()));
            }
            bytes.extend_from_slice(value);
        }
        if bytes.len() as u64 != upload.expected_length
            || digest::sha256_hex(&bytes) != upload.expected_sha256
        {
            return Err(Error::Object("full multipart checksum diverged".into()));
        }
        let mut objects = self.objects.lock().unwrap();
        let created = !objects.contains_key(&upload.key);
        let stored = objects.entry(upload.key.clone()).or_insert(bytes);
        let metadata = Self::metadata(&upload.key, stored);
        pending.upload = None;
        pending.parts.clear();
        Ok(if created {
            ConditionalPut::Created(metadata)
        } else {
            ConditionalPut::AlreadyExists(metadata)
        })
    }

    fn abort_multipart(&self, upload: &S3MultipartUpload) -> Result<()> {
        let mut pending = self.pending.lock().unwrap();
        if pending.upload.as_ref() == Some(upload) {
            pending.upload = None;
            pending.parts.clear();
        }
        Ok(())
    }
}

fn object_reference(verified: &VerifiedObject) -> ObjectReference {
    ObjectReference::for_verified(
        "archive",
        None,
        "application/vnd.rrflow.archive",
        verified.sha256.clone(),
        verified.length,
        verified.receipt.clone(),
    )
    .unwrap()
}

#[test]
fn unauthenticated_or_incomplete_transport_is_refused_before_io() {
    let anonymous = MemoryS3Client {
        capabilities: S3TransportCapabilities {
            authentication: S3Authentication::Anonymous,
            ..S3TransportCapabilities::authenticated_v4()
        },
        ..MemoryS3Client::default()
    };
    assert!(S3CompatibleObjectStore::new(anonymous, "s3:test").is_err());

    let unsigned = MemoryS3Client {
        capabilities: S3TransportCapabilities {
            signed_payloads: false,
            ..S3TransportCapabilities::authenticated_v4()
        },
        ..MemoryS3Client::default()
    };
    assert!(S3CompatibleObjectStore::new(unsigned, "s3:test").is_err());
}

#[test]
fn local_and_s3_compatible_adapters_have_identical_content_semantics() {
    let directory = tempdir().unwrap();
    let local = LocalObjectStore::open(directory.path()).unwrap();
    let s3 = S3CompatibleObjectStore::new(MemoryS3Client::default(), "s3:test").unwrap();
    for bytes in [
        b"".as_slice(),
        b"one".as_slice(),
        b"two-two".as_slice(),
        b"one".as_slice(),
    ] {
        let local_value = local.put(bytes).unwrap();
        let s3_value = s3.put(bytes).unwrap();
        assert_eq!(local_value.sha256, s3_value.sha256);
        assert_eq!(local_value.length, s3_value.length);
    }
    assert_eq!(
        local.inventory(&BTreeSet::new()).unwrap().entries,
        s3.inventory(&BTreeSet::new()).unwrap().entries
    );
}

#[test]
fn logical_archive_uses_the_same_authenticated_resumable_object_path() {
    let temporary = tempdir().unwrap();
    let archive = temporary.path().join("empty.rrd-archive");
    export_logical_archive(&RrflowMxStore::new(), &archive).unwrap();
    let bytes = std::fs::read(&archive).unwrap();
    let sha256 = digest::sha256_hex(&bytes);
    let remote = S3CompatibleObjectStore::new(MemoryS3Client::default(), "s3:archive").unwrap();
    let mut source = std::fs::File::open(archive).unwrap();
    let verified = remote
        .put_verified_stream(&sha256, bytes.len() as u64, &mut source)
        .unwrap();
    let reference = object_reference(&verified);
    remote.client.fail_ranges(2);
    let mut restored = Vec::new();
    remote
        .open_verified(&reference)
        .unwrap()
        .read_to_end(&mut restored)
        .unwrap();
    assert_eq!(restored, bytes);
}

#[test]
fn interrupted_upload_resumes_parts_and_transient_errors_are_bounded() {
    let bytes = (0..(S3_MIN_PART_BYTES * 2 + 31))
        .map(|index| (index % 251) as u8)
        .collect::<Vec<_>>();
    let sha256 = digest::sha256_hex(&bytes);
    let client = MemoryS3Client::default();
    client.fail_part(2, 1);
    let store = S3CompatibleObjectStore::with_policy(
        client,
        "s3:test",
        S3TransferPolicy {
            max_attempts: 1,
            ..S3TransferPolicy::default()
        },
    )
    .unwrap();
    assert!(matches!(
        store.put_verified_stream(&sha256, bytes.len() as u64, &mut Cursor::new(&bytes)),
        Err(Error::RemoteObjectTransient(_))
    ));
    let verified = store
        .put_verified_stream(&sha256, bytes.len() as u64, &mut Cursor::new(&bytes))
        .unwrap();
    assert_eq!(verified.sha256, sha256);
    let uploads = store.client.part_uploads.lock().unwrap();
    assert_eq!(uploads.get(&1), Some(&1));
    assert_eq!(uploads.get(&2), Some(&1));
    assert_eq!(uploads.get(&3), Some(&1));
}

#[test]
fn ranged_remote_recovery_restores_local_loss_and_corruption_fails_closed() {
    let bytes = vec![0x5a; S3_MIN_PART_BYTES + 19];
    let sha256 = digest::sha256_hex(&bytes);
    let client = MemoryS3Client::default();
    client.fail_part(1, 2);
    let remote = S3CompatibleObjectStore::new(client, "s3:test").unwrap();
    let verified = remote
        .put_verified_stream(&sha256, bytes.len() as u64, &mut Cursor::new(&bytes))
        .unwrap();
    let reference = object_reference(&verified);

    let temporary = tempdir().unwrap();
    let local = LocalObjectStore::open(temporary.path()).unwrap();
    local.put(&bytes).unwrap();
    std::fs::remove_file(local.root().join(&reference.receipt.key)).unwrap();
    assert!(matches!(
        local.verify(&sha256),
        Err(Error::ObjectMissing(_))
    ));
    let mut reader = remote.open_verified(&reference).unwrap();
    local
        .put_verified_stream(&sha256, bytes.len() as u64, reader.as_mut())
        .unwrap();
    assert_eq!(local.get(&reference).unwrap(), bytes);
    assert!(remote.client.max_range_bytes.load(Ordering::Relaxed) <= DEFAULT_S3_RANGE_BYTES as u64);

    let mut objects = remote.client.objects.lock().unwrap();
    objects.get_mut(&reference.receipt.key).unwrap()[17] ^= 0x40;
    drop(objects);
    assert!(matches!(
        remote.verify(&sha256),
        Err(Error::Object(_)) | Err(Error::ObjectCorrupt { .. })
    ));
}
