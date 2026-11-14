//! Artifact transfer manifests, receipts, RPCs, progress and observation.

use super::*;

/// Exact immutable-object closure required to make one project scope serveable
/// after the grounded canonical snapshot is installed. Artifact bytes remain
/// outside the Raft/RRD LSM snapshot, while this manifest binds their transfer
/// to that snapshot and the exact scoped runtime read that named them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactTransferManifest {
    pub contract_version: u16,
    pub plan: ReplicaTransferPlan,
    pub scope: ScopeId,
    pub read: ReadStamp,
    pub objects: Vec<ObjectReference>,
    pub manifest_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactReplicaObjectReceipt {
    pub reference: RuntimeRef,
    pub sha256: String,
    pub length: u64,
    pub target: ObjectReceipt,
    pub transferred: bool,
}

/// Target-local residency evidence. It never rewrites the source publication
/// receipt embedded in canonical runtime truth.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactTransferReceipt {
    pub contract_version: u16,
    pub manifest_digest: String,
    pub source: NodeId,
    pub target: NodeId,
    pub objects: Vec<ArtifactReplicaObjectReceipt>,
    pub transferred_objects: u64,
    pub transferred_bytes: u64,
    pub completed_at: u64,
    pub receipt_digest: String,
}

/// One authenticated, replayable request in the out-of-band immutable-object
/// channel. The surrounding TLS transport additionally binds cluster, shard,
/// numeric/canonical peers, and the serialized request digest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactTransferRpc {
    pub contract_version: u16,
    pub operation: ArtifactTransferOperation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum ArtifactTransferOperation {
    Begin {
        manifest: Box<ArtifactTransferManifest>,
    },
    Chunk {
        manifest_digest: String,
        sha256: String,
        offset: u64,
        bytes: Vec<u8>,
        chunk_digest: String,
    },
    Complete {
        manifest_digest: String,
        completed_at: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactObjectProgress {
    pub sha256: String,
    pub expected_length: u64,
    pub next_offset: u64,
    pub complete: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "result", rename_all = "snake_case", deny_unknown_fields)]
pub enum ArtifactTransferRpcResult {
    Progress {
        manifest_digest: String,
        objects: Vec<ArtifactObjectProgress>,
    },
    ChunkAccepted {
        manifest_digest: String,
        object: ArtifactObjectProgress,
    },
    Completed {
        receipt: ArtifactTransferReceipt,
    },
}

/// Bounded control-plane evidence emitted by the authenticated source. A
/// deployment may persist these observations through its normal project
/// runtime commit path, export them, or both; raw artifact bytes and errors
/// never enter the observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactTransferObservationPhase {
    Prepared,
    ChunkAccepted,
    Completed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactTransferObservation {
    pub contract_version: u16,
    pub phase: ArtifactTransferObservationPhase,
    pub at: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_micros: Option<u64>,
    pub attempt: u64,
    pub scope: ScopeId,
    pub manifest_digest: String,
    pub source: NodeId,
    pub target: NodeId,
    pub shard: ShardId,
    pub placement_epoch: u64,
    pub grounded_snapshot: ShardReadStamp,
    pub read: ReadStamp,
    pub object_references: u64,
    pub distinct_objects: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub object_digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_offset: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_length: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub receipt_digest: Option<String>,
    pub transferred_objects: u64,
    pub transferred_bytes: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_digest: Option<String>,
}

impl ArtifactTransferObservation {
    fn base(
        manifest: &ArtifactTransferManifest,
        phase: ArtifactTransferObservationPhase,
        attempt: u64,
        at: u64,
    ) -> Self {
        Self {
            contract_version: CLUSTER_CONTRACT_VERSION,
            phase,
            at,
            duration_micros: None,
            attempt,
            scope: manifest.scope.clone(),
            manifest_digest: manifest.manifest_digest.clone(),
            source: manifest.plan.source.clone(),
            target: manifest.plan.target.clone(),
            shard: manifest.plan.shard,
            placement_epoch: manifest.plan.placement_epoch,
            grounded_snapshot: manifest.plan.grounded_snapshot.clone(),
            read: manifest.read.clone(),
            object_references: manifest.objects.len() as u64,
            distinct_objects: manifest.plan.artifact_digests.len() as u64,
            object_digest: None,
            next_offset: None,
            expected_length: None,
            receipt_digest: None,
            transferred_objects: 0,
            transferred_bytes: 0,
            error_digest: None,
        }
    }

    pub fn prepared(manifest: &ArtifactTransferManifest, attempt: u64, at: u64) -> Result<Self> {
        manifest.validate()?;
        let observation = Self::base(
            manifest,
            ArtifactTransferObservationPhase::Prepared,
            attempt,
            at,
        );
        observation.validate()?;
        Ok(observation)
    }

    pub fn progress(
        manifest: &ArtifactTransferManifest,
        attempt: u64,
        at: u64,
        object: &ArtifactObjectProgress,
    ) -> Result<Self> {
        manifest.validate()?;
        let expected = manifest
            .objects
            .iter()
            .find(|candidate| candidate.sha256 == object.sha256)
            .ok_or_else(|| {
                ClusterError::Invalid("artifact progress object is absent from its manifest".into())
            })?;
        if expected.length != object.expected_length
            || object.next_offset > object.expected_length
            || object.complete != (object.next_offset == object.expected_length)
        {
            return Err(ClusterError::Invalid(
                "artifact progress differs from its manifest object".into(),
            ));
        }
        let mut observation = Self::base(
            manifest,
            ArtifactTransferObservationPhase::ChunkAccepted,
            attempt,
            at,
        );
        observation.object_digest = Some(object.sha256.clone());
        observation.next_offset = Some(object.next_offset);
        observation.expected_length = Some(object.expected_length);
        observation.validate()?;
        Ok(observation)
    }

    pub fn completed(
        manifest: &ArtifactTransferManifest,
        attempt: u64,
        at: u64,
        duration_micros: u64,
        receipt: &ArtifactTransferReceipt,
    ) -> Result<Self> {
        receipt.validate(manifest)?;
        let mut observation = Self::base(
            manifest,
            ArtifactTransferObservationPhase::Completed,
            attempt,
            at,
        );
        observation.receipt_digest = Some(receipt.receipt_digest.clone());
        observation.duration_micros = Some(duration_micros);
        observation.transferred_objects = receipt.transferred_objects;
        observation.transferred_bytes = receipt.transferred_bytes;
        observation.validate()?;
        Ok(observation)
    }

    pub fn failed(
        manifest: &ArtifactTransferManifest,
        attempt: u64,
        at: u64,
        duration_micros: u64,
        error: &str,
    ) -> Result<Self> {
        manifest.validate()?;
        let mut observation = Self::base(
            manifest,
            ArtifactTransferObservationPhase::Failed,
            attempt,
            at,
        );
        observation.error_digest = Some(sha256_hex(error.as_bytes()));
        observation.duration_micros = Some(duration_micros);
        observation.validate()?;
        Ok(observation)
    }

    pub fn validate(&self) -> Result<()> {
        self.read
            .validate()
            .map_err(|error| ClusterError::Invalid(error.to_string()))?;
        if self.contract_version != CLUSTER_CONTRACT_VERSION
            || self.attempt == 0
            || self.scope != self.read.scope
            || self.placement_epoch != self.grounded_snapshot.placement_epoch
            || self.distinct_objects > self.object_references
        {
            return Err(ClusterError::Invalid(
                "artifact observation identity or grounded coordinates are invalid".into(),
            ));
        }
        validate_sha256("artifact observation manifest", &self.manifest_digest)?;
        if self.grounded_snapshot.validate().is_err() {
            return Err(ClusterError::Invalid(
                "artifact observation snapshot is invalid".into(),
            ));
        }
        match self.phase {
            ArtifactTransferObservationPhase::Prepared => {
                if self.object_digest.is_some()
                    || self.next_offset.is_some()
                    || self.expected_length.is_some()
                    || self.receipt_digest.is_some()
                    || self.error_digest.is_some()
                    || self.transferred_objects != 0
                    || self.transferred_bytes != 0
                    || self.duration_micros.is_some()
                {
                    return Err(ClusterError::Invalid(
                        "prepared artifact observation contains terminal evidence".into(),
                    ));
                }
            }
            ArtifactTransferObservationPhase::ChunkAccepted => {
                let (Some(digest), Some(offset), Some(length)) = (
                    self.object_digest.as_deref(),
                    self.next_offset,
                    self.expected_length,
                ) else {
                    return Err(ClusterError::Invalid(
                        "artifact progress observation is incomplete".into(),
                    ));
                };
                validate_sha256("artifact observation object", digest)?;
                if offset > length
                    || self.receipt_digest.is_some()
                    || self.error_digest.is_some()
                    || self.transferred_objects != 0
                    || self.transferred_bytes != 0
                    || self.duration_micros.is_some()
                {
                    return Err(ClusterError::Invalid(
                        "artifact progress observation is inconsistent".into(),
                    ));
                }
            }
            ArtifactTransferObservationPhase::Completed => {
                let Some(receipt) = self.receipt_digest.as_deref() else {
                    return Err(ClusterError::Invalid(
                        "completed artifact observation has no receipt".into(),
                    ));
                };
                validate_sha256("artifact observation receipt", receipt)?;
                if self.object_digest.is_some()
                    || self.next_offset.is_some()
                    || self.expected_length.is_some()
                    || self.error_digest.is_some()
                    || self.duration_micros.is_none()
                {
                    return Err(ClusterError::Invalid(
                        "completed artifact observation contains progress or error evidence".into(),
                    ));
                }
            }
            ArtifactTransferObservationPhase::Failed => {
                let Some(error) = self.error_digest.as_deref() else {
                    return Err(ClusterError::Invalid(
                        "failed artifact observation has no error digest".into(),
                    ));
                };
                validate_sha256("artifact observation error", error)?;
                if self.object_digest.is_some()
                    || self.next_offset.is_some()
                    || self.expected_length.is_some()
                    || self.receipt_digest.is_some()
                    || self.transferred_objects != 0
                    || self.transferred_bytes != 0
                    || self.duration_micros.is_none()
                {
                    return Err(ClusterError::Invalid(
                        "failed artifact observation contains success evidence".into(),
                    ));
                }
            }
        }
        Ok(())
    }
}

/// Deployment-owned persistence/export boundary for transfer observations.
/// Implementations must be thread-safe because OpenRaft can replicate several
/// learners concurrently.
pub trait ArtifactTransferObserver: Send + Sync {
    fn observe(
        &self,
        observation: ArtifactTransferObservation,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>>;
}

impl ArtifactTransferRpc {
    pub fn begin(manifest: ArtifactTransferManifest) -> Result<Self> {
        let request = Self {
            contract_version: CLUSTER_CONTRACT_VERSION,
            operation: ArtifactTransferOperation::Begin {
                manifest: Box::new(manifest),
            },
        };
        request.validate()?;
        Ok(request)
    }

    pub fn chunk(
        manifest_digest: impl Into<String>,
        sha256: impl Into<String>,
        offset: u64,
        bytes: Vec<u8>,
    ) -> Result<Self> {
        let chunk_digest = sha256_hex(&bytes);
        let request = Self {
            contract_version: CLUSTER_CONTRACT_VERSION,
            operation: ArtifactTransferOperation::Chunk {
                manifest_digest: manifest_digest.into(),
                sha256: sha256.into(),
                offset,
                bytes,
                chunk_digest,
            },
        };
        request.validate()?;
        Ok(request)
    }

    pub fn complete(manifest_digest: impl Into<String>, completed_at: u64) -> Result<Self> {
        let request = Self {
            contract_version: CLUSTER_CONTRACT_VERSION,
            operation: ArtifactTransferOperation::Complete {
                manifest_digest: manifest_digest.into(),
                completed_at,
            },
        };
        request.validate()?;
        Ok(request)
    }

    pub fn validate(&self) -> Result<()> {
        if self.contract_version != CLUSTER_CONTRACT_VERSION {
            return Err(ClusterError::Invalid(
                "artifact RPC contract version is unsupported".into(),
            ));
        }
        match &self.operation {
            ArtifactTransferOperation::Begin { manifest } => manifest.validate(),
            ArtifactTransferOperation::Chunk {
                manifest_digest,
                sha256,
                bytes,
                chunk_digest,
                ..
            } => {
                validate_sha256("artifact RPC manifest", manifest_digest)?;
                validate_sha256("artifact RPC object", sha256)?;
                validate_sha256("artifact RPC chunk", chunk_digest)?;
                if bytes.is_empty() || bytes.len() > ARTIFACT_TRANSFER_CHUNK_MAX_BYTES {
                    return Err(ClusterError::Invalid(format!(
                        "artifact RPC chunks must contain 1..={ARTIFACT_TRANSFER_CHUNK_MAX_BYTES} bytes"
                    )));
                }
                if sha256_hex(bytes) != *chunk_digest {
                    return Err(ClusterError::Invalid(
                        "artifact RPC chunk digest differs from its bytes".into(),
                    ));
                }
                Ok(())
            }
            ArtifactTransferOperation::Complete {
                manifest_digest, ..
            } => validate_sha256("artifact RPC manifest", manifest_digest),
        }
    }
}

pub(super) fn validate_sha256(label: &str, value: &str) -> Result<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(ClusterError::Invalid(format!(
            "{label} digest must be 64 lowercase hexadecimal bytes"
        )));
    }
    Ok(())
}

impl ArtifactTransferManifest {
    pub fn new(
        plan: ReplicaTransferPlan,
        scope: ScopeId,
        read: ReadStamp,
        mut objects: Vec<ObjectReference>,
    ) -> Result<Self> {
        objects.sort_by(|left, right| left.reference.cmp(&right.reference));
        let mut manifest = Self {
            contract_version: CLUSTER_CONTRACT_VERSION,
            plan,
            scope,
            read,
            objects,
            manifest_digest: String::new(),
        };
        manifest.validate_components()?;
        manifest.manifest_digest = sha256_hex(&manifest.identity_bytes()?);
        Ok(manifest)
    }

    pub fn validate(&self) -> Result<()> {
        self.validate_components()?;
        if self.manifest_digest != sha256_hex(&self.identity_bytes()?) {
            return Err(ClusterError::Invalid(
                "artifact transfer manifest digest differs from its fields".into(),
            ));
        }
        Ok(())
    }

    fn validate_components(&self) -> Result<()> {
        self.plan.validate()?;
        self.read
            .validate()
            .map_err(|error| ClusterError::Invalid(error.to_string()))?;
        if self.contract_version != CLUSTER_CONTRACT_VERSION || self.read.scope != self.scope {
            return Err(ClusterError::Invalid(
                "artifact manifest version, scope, or read binding is invalid".into(),
            ));
        }
        if self.objects.len() > 1_000_000 {
            return Err(ClusterError::Invalid(
                "artifact manifest object limit exceeded".into(),
            ));
        }
        let mut references = BTreeSet::new();
        let mut digests = BTreeSet::new();
        let mut previous = None;
        for object in &self.objects {
            object
                .validate()
                .map_err(|error| ClusterError::Invalid(error.to_string()))?;
            if previous
                .as_ref()
                .is_some_and(|value| value >= &object.reference)
                || !references.insert(object.reference.clone())
            {
                return Err(ClusterError::Invalid(
                    "artifact manifest objects must have unique canonical ordering".into(),
                ));
            }
            previous = Some(object.reference.clone());
            digests.insert(object.sha256.clone());
        }
        if digests != self.plan.artifact_digests {
            return Err(ClusterError::Invalid(
                "artifact manifest objects differ from the replica transfer plan".into(),
            ));
        }
        Ok(())
    }

    fn identity_bytes(&self) -> Result<Vec<u8>> {
        serde_json::to_vec(&(
            self.contract_version,
            &self.plan,
            &self.scope,
            &self.read,
            &self.objects,
        ))
        .map_err(|error| ClusterError::Invalid(format!("artifact manifest encode failed: {error}")))
    }
}

impl ArtifactTransferReceipt {
    pub fn new(
        manifest: &ArtifactTransferManifest,
        objects: Vec<ArtifactReplicaObjectReceipt>,
        completed_at: u64,
    ) -> Result<Self> {
        let transferred_objects = objects.iter().filter(|object| object.transferred).count() as u64;
        let transferred_bytes = objects
            .iter()
            .filter(|object| object.transferred)
            .try_fold(0u64, |total, object| total.checked_add(object.length))
            .ok_or_else(|| {
                ClusterError::Invalid("artifact transfer byte count overflowed".into())
            })?;
        let mut receipt = Self {
            contract_version: CLUSTER_CONTRACT_VERSION,
            manifest_digest: manifest.manifest_digest.clone(),
            source: manifest.plan.source.clone(),
            target: manifest.plan.target.clone(),
            objects,
            transferred_objects,
            transferred_bytes,
            completed_at,
            receipt_digest: String::new(),
        };
        receipt.validate_components(manifest)?;
        receipt.receipt_digest = sha256_hex(&receipt.identity_bytes()?);
        Ok(receipt)
    }

    pub fn validate(&self, manifest: &ArtifactTransferManifest) -> Result<()> {
        self.validate_components(manifest)?;
        if self.receipt_digest != sha256_hex(&self.identity_bytes()?) {
            return Err(ClusterError::Invalid(
                "artifact transfer receipt digest differs from its fields".into(),
            ));
        }
        Ok(())
    }

    fn validate_components(&self, manifest: &ArtifactTransferManifest) -> Result<()> {
        manifest.validate()?;
        if self.contract_version != CLUSTER_CONTRACT_VERSION
            || self.manifest_digest != manifest.manifest_digest
            || self.source != manifest.plan.source
            || self.target != manifest.plan.target
            || self.objects.len() != manifest.objects.len()
        {
            return Err(ClusterError::Invalid(
                "artifact transfer receipt identity differs from its manifest".into(),
            ));
        }
        let mut transferred_objects = 0u64;
        let mut transferred_bytes = 0u64;
        for (receipt, object) in self.objects.iter().zip(&manifest.objects) {
            let target_reference = ObjectReference {
                reference: receipt.reference.clone(),
                subject: object.subject.clone(),
                sha256: receipt.sha256.clone(),
                length: receipt.length,
                media_type: object.media_type.clone(),
                receipt: receipt.target.clone(),
                properties: object.properties.clone(),
            };
            target_reference
                .validate()
                .map_err(|error| ClusterError::Invalid(error.to_string()))?;
            if receipt.reference != object.reference
                || receipt.sha256 != object.sha256
                || receipt.length != object.length
            {
                return Err(ClusterError::Invalid(
                    "artifact object receipt differs from its manifest object".into(),
                ));
            }
            if receipt.transferred {
                transferred_objects += 1;
                transferred_bytes =
                    transferred_bytes
                        .checked_add(receipt.length)
                        .ok_or_else(|| {
                            ClusterError::Invalid("artifact transfer byte count overflowed".into())
                        })?;
            }
        }
        if transferred_objects != self.transferred_objects
            || transferred_bytes != self.transferred_bytes
        {
            return Err(ClusterError::Invalid(
                "artifact transfer receipt counters differ from its objects".into(),
            ));
        }
        Ok(())
    }

    fn identity_bytes(&self) -> Result<Vec<u8>> {
        serde_json::to_vec(&(
            self.contract_version,
            &self.manifest_digest,
            &self.source,
            &self.target,
            &self.objects,
            self.transferred_objects,
            self.transferred_bytes,
            self.completed_at,
        ))
        .map_err(|error| ClusterError::Invalid(format!("artifact receipt encode failed: {error}")))
    }
}
