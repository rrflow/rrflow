//! S3 limits, authentication, transport capabilities, transfer policy, metadata, multipart uploads and the client trait.

use super::*;

pub const S3_MIN_PART_BYTES: usize = 5 * 1024 * 1024;
pub const S3_MAX_PART_BYTES: u64 = 5 * 1024 * 1024 * 1024;
pub const S3_MAX_PARTS: u32 = 10_000;
pub const DEFAULT_S3_RANGE_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum S3Authentication {
    Anonymous,
    AwsSignatureV4,
    MutualTls,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct S3TransportCapabilities {
    pub authentication: S3Authentication,
    pub signed_payloads: bool,
    pub conditional_writes: bool,
    pub resumable_multipart: bool,
    pub ranged_reads: bool,
    pub sha256_checksums: bool,
    pub paginated_listing: bool,
}

impl S3TransportCapabilities {
    pub fn authenticated_v4() -> Self {
        Self {
            authentication: S3Authentication::AwsSignatureV4,
            signed_payloads: true,
            conditional_writes: true,
            resumable_multipart: true,
            ranged_reads: true,
            sha256_checksums: true,
            paginated_listing: true,
        }
    }

    pub(super) fn validate(self) -> Result<()> {
        if self.authentication == S3Authentication::Anonymous {
            return Err(Error::Object(
                "S3 transport must authenticate every object operation".into(),
            ));
        }
        if !self.signed_payloads
            || !self.conditional_writes
            || !self.resumable_multipart
            || !self.ranged_reads
            || !self.sha256_checksums
            || !self.paginated_listing
        {
            return Err(Error::Object(
                "S3 transport lacks signed payload, conditional write, resumable multipart, ranged read, SHA-256 checksum, or paginated listing capability".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct S3TransferPolicy {
    pub part_bytes: usize,
    pub range_bytes: usize,
    pub max_attempts: u8,
}

impl Default for S3TransferPolicy {
    fn default() -> Self {
        Self {
            part_bytes: S3_MIN_PART_BYTES,
            range_bytes: DEFAULT_S3_RANGE_BYTES,
            max_attempts: 3,
        }
    }
}

impl S3TransferPolicy {
    pub(super) fn validate(self) -> Result<Self> {
        let part_bytes = u64::try_from(self.part_bytes)
            .map_err(|_| Error::Object("S3 part size exceeds u64".into()))?;
        if !(S3_MIN_PART_BYTES as u64..=S3_MAX_PART_BYTES).contains(&part_bytes) {
            return Err(Error::Object(format!(
                "S3 part size must be in {S3_MIN_PART_BYTES}..={S3_MAX_PART_BYTES} bytes"
            )));
        }
        if self.range_bytes == 0 || self.range_bytes > self.part_bytes {
            return Err(Error::Object(
                "S3 range size must be positive and no larger than one part".into(),
            ));
        }
        if self.max_attempts == 0 || self.max_attempts > 10 {
            return Err(Error::Object("S3 remote attempts must be in 1..=10".into()));
        }
        Ok(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct S3ObjectMetadata {
    pub key: String,
    pub length: u64,
    pub etag: Option<String>,
    pub version: Option<String>,
    pub checksum_sha256: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConditionalPut {
    Created(S3ObjectMetadata),
    AlreadyExists(S3ObjectMetadata),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct S3MultipartUpload {
    pub key: String,
    pub upload_id: String,
    pub expected_sha256: String,
    pub expected_length: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct S3UploadedPart {
    pub part_number: u32,
    pub length: u64,
    pub sha256: String,
    pub etag: String,
}

/// Synchronous S3 transport port. Implementations map these methods to signed,
/// bounded-time S3 requests. `put_if_absent` and multipart completion must use
/// real conditional requests, never HEAD-then-PUT emulation. `find_multipart`
/// and `list_parts` must consume all service pages before returning.
pub trait S3ObjectClient: Send + Sync {
    fn capabilities(&self) -> S3TransportCapabilities;
    fn put_if_absent(&self, key: &str, bytes: &[u8]) -> Result<ConditionalPut>;
    fn head(&self, key: &str) -> Result<Option<S3ObjectMetadata>>;
    fn get(&self, key: &str, version: Option<&str>) -> Result<Option<Vec<u8>>>;
    fn get_range(
        &self,
        key: &str,
        version: Option<&str>,
        offset: u64,
        length: usize,
    ) -> Result<Option<Vec<u8>>>;
    fn list(&self, prefix: &str) -> Result<Vec<S3ObjectMetadata>>;
    fn delete(&self, key: &str) -> Result<()>;
    fn find_multipart(
        &self,
        key: &str,
        expected_sha256: &str,
        expected_length: u64,
    ) -> Result<Option<S3MultipartUpload>>;
    fn create_multipart(
        &self,
        key: &str,
        expected_sha256: &str,
        expected_length: u64,
    ) -> Result<S3MultipartUpload>;
    fn list_parts(&self, upload: &S3MultipartUpload) -> Result<Vec<S3UploadedPart>>;
    fn upload_part(
        &self,
        upload: &S3MultipartUpload,
        part_number: u32,
        bytes: &[u8],
        sha256: &str,
    ) -> Result<S3UploadedPart>;
    fn complete_multipart_if_absent(
        &self,
        upload: &S3MultipartUpload,
        parts: &[S3UploadedPart],
    ) -> Result<ConditionalPut>;
    fn abort_multipart(&self, upload: &S3MultipartUpload) -> Result<()>;
}
