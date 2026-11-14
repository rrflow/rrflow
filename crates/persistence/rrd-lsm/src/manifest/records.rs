//! Segment descriptors, manifests, checkpoints and the current pointer, with their validation.

use super::*;

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

pub(super) fn is_sha256(value: &str) -> bool {
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
    pub(super) fn new(name: &str, manifest: &Manifest, created_at: u64) -> Result<Self> {
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

    pub(super) fn validate(&self) -> Result<()> {
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
    pub(super) fn new(manifest: &Manifest) -> Self {
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
