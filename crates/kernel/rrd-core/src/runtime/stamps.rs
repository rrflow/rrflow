//! Read stamps, snapshot handles and retention pins.

use super::*;

/// Exact logical state observed by a reader or transaction author.
///
/// `manifest_id` is a content digest over the other fields. The compatibility
/// engines do not pretend to expose a physical LSM manifest; this semantic
/// manifest identity is stable across backends and becomes the parent identity
/// of a native `RRD LSM` physical manifest later.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadStamp {
    pub contract_version: u16,
    pub scope: ScopeId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema_revision: Option<u64>,
    pub catalog_revision: u64,
    pub commit_cursor: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub head_digest: Option<String>,
    /// RFC 9162 Merkle root for the exact global log prefix. `None` denotes a
    /// legacy stamp whose validation must replay the hash chain.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accumulator_root: Option<String>,
    pub manifest_id: String,
}

impl ReadStamp {
    pub fn new(
        scope: ScopeId,
        schema_revision: Option<u64>,
        catalog_revision: u64,
        commit_cursor: u64,
        head_digest: Option<String>,
    ) -> Result<Self> {
        let mut stamp = Self {
            contract_version: DATA_RUNTIME_CONTRACT_VERSION,
            scope,
            schema_revision,
            catalog_revision,
            commit_cursor,
            head_digest,
            accumulator_root: None,
            manifest_id: String::new(),
        };
        stamp.validate_components()?;
        stamp.manifest_id = digest::sha256_hex(&stamp.manifest_bytes());
        Ok(stamp)
    }

    pub fn authenticated(
        scope: ScopeId,
        schema_revision: Option<u64>,
        catalog_revision: u64,
        commit_cursor: u64,
        head_digest: Option<String>,
        accumulator_root: String,
    ) -> Result<Self> {
        let mut stamp = Self {
            contract_version: DATA_RUNTIME_CONTRACT_VERSION,
            scope,
            schema_revision,
            catalog_revision,
            commit_cursor,
            head_digest,
            accumulator_root: Some(accumulator_root),
            manifest_id: String::new(),
        };
        stamp.validate_components()?;
        stamp.manifest_id = digest::sha256_hex(&stamp.manifest_bytes());
        Ok(stamp)
    }

    pub fn validate(&self) -> Result<()> {
        self.validate_components()?;
        validate_digest("read stamp manifest", &self.manifest_id)?;
        let expected = digest::sha256_hex(&self.manifest_bytes());
        if self.manifest_id != expected {
            return Err(Error::InvalidRuntime {
                reason: "read stamp manifest digest does not match its fields".into(),
            });
        }
        Ok(())
    }

    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut out = self.manifest_bytes();
        text(&mut out, &self.manifest_id);
        out
    }

    fn validate_components(&self) -> Result<()> {
        if self.contract_version != DATA_RUNTIME_CONTRACT_VERSION {
            return Err(Error::InvalidRuntime {
                reason: format!(
                    "unsupported data-runtime contract version {}",
                    self.contract_version
                ),
            });
        }
        if self.schema_revision == Some(0) {
            return Err(Error::InvalidRuntime {
                reason: "schema revision zero is not a valid installed revision".into(),
            });
        }
        match (self.commit_cursor, self.head_digest.as_deref()) {
            (0, None) => {}
            (0, Some(_)) => {
                return Err(Error::InvalidRuntime {
                    reason: "an empty runtime cannot have a hash-chain head".into(),
                });
            }
            (_, Some(value)) => validate_digest("read stamp hash-chain head", value)?,
            (_, None) => {
                return Err(Error::InvalidRuntime {
                    reason: "a non-empty runtime must carry its hash-chain head".into(),
                });
            }
        }
        if let Some(root) = &self.accumulator_root {
            validate_digest("read stamp accumulator root", root)?;
        }
        Ok(())
    }

    fn manifest_bytes(&self) -> Vec<u8> {
        let mut out = b"rrflow-read-stamp-v1\0".to_vec();
        out.extend_from_slice(&self.contract_version.to_be_bytes());
        text(&mut out, self.scope.as_str());
        encode_optional_u64(&mut out, self.schema_revision);
        out.extend_from_slice(&self.catalog_revision.to_be_bytes());
        out.extend_from_slice(&self.commit_cursor.to_be_bytes());
        optional_text(&mut out, self.head_digest.as_deref());
        if let Some(root) = &self.accumulator_root {
            out.extend_from_slice(b"rrflow-read-stamp-accumulator-v1\0");
            text(&mut out, root);
        }
        out
    }
}

/// Persisted lease over one transaction-consistent read stamp.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotHandle {
    pub contract_version: u16,
    pub id: SnapshotId,
    pub read: ReadStamp,
    pub owner: String,
    pub created_at: Millis,
    pub expires_at: Millis,
}

impl SnapshotHandle {
    pub fn new(
        read: ReadStamp,
        owner: impl Into<String>,
        created_at: Millis,
        ttl: Millis,
    ) -> Result<Self> {
        read.validate()?;
        let owner = owner.into();
        validate_text("snapshot owner", &owner)?;
        if ttl == 0 {
            return Err(Error::InvalidRuntime {
                reason: "snapshot lease duration must be greater than zero".into(),
            });
        }
        let expires_at = created_at
            .checked_add(ttl)
            .ok_or_else(|| Error::InvalidRuntime {
                reason: "snapshot lease expiration overflowed".into(),
            })?;
        let id = SnapshotId::new(Self::identity(&read, &owner, created_at, expires_at))?;
        Ok(Self {
            contract_version: DATA_RUNTIME_CONTRACT_VERSION,
            id,
            read,
            owner,
            created_at,
            expires_at,
        })
    }

    pub fn validate(&self) -> Result<()> {
        if self.contract_version != DATA_RUNTIME_CONTRACT_VERSION {
            return Err(Error::InvalidRuntime {
                reason: format!(
                    "unsupported snapshot contract version {}",
                    self.contract_version
                ),
            });
        }
        self.read.validate()?;
        validate_text("snapshot owner", &self.owner)?;
        if self.expires_at <= self.created_at {
            return Err(Error::InvalidRuntime {
                reason: "snapshot lease must expire after creation".into(),
            });
        }
        let expected = Self::identity(&self.read, &self.owner, self.created_at, self.expires_at);
        if self.id.as_str() != expected {
            return Err(Error::InvalidRuntime {
                reason: "snapshot identity does not match its lease and read stamp".into(),
            });
        }
        Ok(())
    }

    pub fn is_expired(&self, now: Millis) -> bool {
        now >= self.expires_at
    }

    fn identity(read: &ReadStamp, owner: &str, created_at: Millis, expires_at: Millis) -> String {
        let mut bytes = b"rrflow-snapshot-handle-v1\0".to_vec();
        bytes.extend_from_slice(&read.canonical_bytes());
        text(&mut bytes, owner);
        bytes.extend_from_slice(&created_at.to_be_bytes());
        bytes.extend_from_slice(&expires_at.to_be_bytes());
        digest::sha256_hex(&bytes)
    }
}

/// Logical retention root derived from a live snapshot lease.
///
/// Compatibility engines retain the append-only log, so this pin names the
/// semantic manifest and minimum cursor. Native `RRD LSM` must map the same pin
/// to every physical manifest, segment, and object required to serve it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetentionPin {
    pub contract_version: u16,
    pub id: RetentionPinId,
    pub snapshot_id: SnapshotId,
    pub scope: ScopeId,
    pub manifest_id: String,
    pub minimum_cursor: u64,
    pub expires_at: Millis,
}

impl RetentionPin {
    pub fn from_snapshot(snapshot: &SnapshotHandle) -> Result<Self> {
        snapshot.validate()?;
        let id = RetentionPinId::new(Self::identity(
            &snapshot.id,
            &snapshot.read.scope,
            &snapshot.read.manifest_id,
            snapshot.read.commit_cursor,
            snapshot.expires_at,
        ))?;
        let pin = Self {
            contract_version: DATA_RUNTIME_CONTRACT_VERSION,
            id,
            snapshot_id: snapshot.id.clone(),
            scope: snapshot.read.scope.clone(),
            manifest_id: snapshot.read.manifest_id.clone(),
            minimum_cursor: snapshot.read.commit_cursor,
            expires_at: snapshot.expires_at,
        };
        pin.validate()?;
        Ok(pin)
    }

    pub fn validate(&self) -> Result<()> {
        if self.contract_version != DATA_RUNTIME_CONTRACT_VERSION {
            return Err(Error::InvalidRuntime {
                reason: format!(
                    "unsupported retention-pin contract version {}",
                    self.contract_version
                ),
            });
        }
        validate_digest("retention-pin manifest", &self.manifest_id)?;
        validate_digest("retention pin identity", self.id.as_str())?;
        let expected = Self::identity(
            &self.snapshot_id,
            &self.scope,
            &self.manifest_id,
            self.minimum_cursor,
            self.expires_at,
        );
        if self.id.as_str() != expected {
            return Err(Error::InvalidRuntime {
                reason: "retention pin identity does not match its snapshot".into(),
            });
        }
        Ok(())
    }

    pub fn is_expired(&self, now: Millis) -> bool {
        now >= self.expires_at
    }

    fn identity(
        snapshot_id: &SnapshotId,
        scope: &ScopeId,
        manifest_id: &str,
        minimum_cursor: u64,
        expires_at: Millis,
    ) -> String {
        let mut bytes = b"rrflow-retention-pin-v1\0".to_vec();
        text(&mut bytes, snapshot_id.as_str());
        text(&mut bytes, scope.as_str());
        text(&mut bytes, manifest_id);
        bytes.extend_from_slice(&minimum_cursor.to_be_bytes());
        bytes.extend_from_slice(&expires_at.to_be_bytes());
        digest::sha256_hex(&bytes)
    }
}
