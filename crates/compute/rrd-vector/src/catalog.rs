use crate::contract::invalid;

use crate::{
    CandidatePath, HnswBuildEvidence, HnswDescriptor, QuantizedDescriptor, SegmentDescriptor,
    TurboQuantDescriptor, VectorArtifact, VectorArtifactKind, EXACT_SCAN_PROJECTION_ID,
};

use rrd_core::{
    digest, ObjectReference, ProjectionId, ProjectionStamp, ProjectionState, Result, RuntimeRef,
    ScopeId, VectorCollectionAddress,
};

use serde::{Deserialize, Serialize};

use std::collections::{BTreeMap, BTreeSet};

pub const VECTOR_ARTIFACT_CATALOG_VERSION: u16 = 3;

pub const VECTOR_ARTIFACT_RECORD_TYPE: &str = "vector_artifact";

/// Stable, domain-separated identity for a structured named-vector source.
/// The stored address remains authoritative if a digest ever collides.
pub fn vector_source_digest(source: &VectorCollectionAddress) -> Result<String> {
    source.validate()?;
    let mut identity = b"rrflow-hnsw-source-v1\0".to_vec();
    // Freeze the v1 identity to these two ordered strings instead of the
    // evolving public struct's serialized fields.
    identity.extend(
        serde_json::to_vec(&(&source.collection_id, &source.vector_name)).map_err(|error| {
            rrd_core::Error::InvalidRuntime {
                reason: format!("vector source cannot be encoded: {error}"),
            }
        })?,
    );
    Ok(digest::sha256_hex(&identity))
}

/// The idempotent operation that published a generation: its key and the
/// SHA-256 of the request, recorded in the same commit as the generation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VectorArtifactOperationReceipt {
    pub idempotency_key: String,
    pub operation_sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<VectorArtifactSource>,
}

/// Versioned source identity, committed as part of the operation receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VectorArtifactSource {
    pub contract_version: u16,
    pub address: VectorCollectionAddress,
}

/// One immutable, reconstructable vector projection publication.
///
/// The descriptor says what may be served. The object reference says exactly
/// which verified bytes implement it. Both enter one authoritative runtime
/// transaction, so a catalog entry can never point at a partially published
/// file or silently change representation after restart.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VectorArtifactCatalogEntry {
    pub contract_version: u16,
    pub catalog_revision: u64,
    pub kind: VectorArtifactKind,
    pub descriptor: VectorProjectionDescriptor,
    pub object: ObjectReference,
    pub published_at: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub build_evidence: Option<HnswBuildEvidence>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation: Option<VectorArtifactOperationReceipt>,
    pub entry_digest: String,
}

impl VectorArtifactCatalogEntry {
    /// `{projection}.{generation}`: a public canonical id whenever the
    /// projection id is one, so the public changefeed can project the record.
    /// The generation has no `.`, so the identity stays unique.
    pub fn record_reference(descriptor: &VectorProjectionDescriptor) -> Result<RuntimeRef> {
        RuntimeRef::new(
            VECTOR_ARTIFACT_RECORD_TYPE,
            format!(
                "{}.{}",
                descriptor.stamp().id,
                descriptor.stamp().generation
            ),
        )
    }

    pub fn new(
        catalog_revision: u64,
        kind: VectorArtifactKind,
        descriptor: VectorProjectionDescriptor,
        object: ObjectReference,
        published_at: u64,
    ) -> Result<Self> {
        Self::new_with_build_evidence(
            catalog_revision,
            kind,
            descriptor,
            object,
            published_at,
            None,
            None,
        )
    }

    pub fn new_with_build_evidence(
        catalog_revision: u64,
        kind: VectorArtifactKind,
        descriptor: VectorProjectionDescriptor,
        object: ObjectReference,
        published_at: u64,
        build_evidence: Option<HnswBuildEvidence>,
        operation: Option<VectorArtifactOperationReceipt>,
    ) -> Result<Self> {
        Self::new_with_source(
            catalog_revision,
            kind,
            descriptor,
            object,
            published_at,
            build_evidence,
            operation,
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_with_source(
        catalog_revision: u64,
        kind: VectorArtifactKind,
        descriptor: VectorProjectionDescriptor,
        object: ObjectReference,
        published_at: u64,
        build_evidence: Option<HnswBuildEvidence>,
        mut operation: Option<VectorArtifactOperationReceipt>,
        source: Option<VectorCollectionAddress>,
    ) -> Result<Self> {
        if operation
            .as_ref()
            .is_some_and(|receipt| receipt.source.is_some())
        {
            return invalid("source must be bound by the catalogue constructor");
        }
        if let Some(address) = source {
            let receipt = operation
                .as_mut()
                .ok_or_else(|| rrd_core::Error::InvalidRuntime {
                    reason: "named source requires an operation receipt".into(),
                })?;
            receipt.source = Some(VectorArtifactSource {
                contract_version: 1,
                address,
            });
        }
        let mut entry = Self {
            contract_version: VECTOR_ARTIFACT_CATALOG_VERSION,
            catalog_revision,
            kind,
            descriptor,
            object,
            published_at,
            build_evidence,
            operation,
            entry_digest: String::new(),
        };
        entry.validate_components()?;
        entry.entry_digest = digest::sha256_hex(&entry.identity_bytes()?);
        Ok(entry)
    }

    pub fn validate(&self) -> Result<()> {
        self.validate_components()?;
        let expected = digest::sha256_hex(&self.identity_bytes()?);
        if self.entry_digest != expected {
            return invalid("vector artifact catalog entry digest does not match its fields");
        }
        Ok(())
    }

    pub fn scope(&self) -> &ScopeId {
        self.descriptor.scope()
    }

    pub fn source(&self) -> Option<&VectorCollectionAddress> {
        self.operation
            .as_ref()?
            .source
            .as_ref()
            .map(|source| &source.address)
    }

    pub fn decode_artifact(&self, bytes: &[u8]) -> Result<VectorArtifact> {
        self.validate()?;
        if bytes.len() as u64 != self.object.length
            || digest::sha256_hex(bytes) != self.object.sha256
        {
            return invalid("vector artifact object bytes differ from the catalog reference");
        }
        let artifact = VectorArtifact::from_bytes(self.kind, bytes)?;
        if artifact.descriptor() != self.descriptor {
            return invalid("decoded vector artifact differs from its catalog descriptor");
        }
        if let (Some(source), VectorArtifact::Hnsw(index)) = (self.source(), &artifact) {
            if !index.contains_only_source(source) {
                return invalid("HNSW artifact contains a foreign source");
            }
        }
        Ok(artifact)
    }

    fn validate_components(&self) -> Result<()> {
        if !matches!(self.contract_version, 2 | VECTOR_ARTIFACT_CATALOG_VERSION)
            || self.catalog_revision == 0
            || (self.contract_version == 2 && self.source().is_some())
        {
            return invalid("vector artifact catalog version and revision must be valid");
        }
        if let Some(source) = self
            .operation
            .as_ref()
            .and_then(|receipt| receipt.source.as_ref())
        {
            if source.contract_version != 1 {
                return invalid("vector source binding version is unsupported");
            }
            source.address.validate()?;
            if self.kind != VectorArtifactKind::Hnsw {
                return invalid("named source binding requires an HNSW artifact");
            }
        }
        self.descriptor.validate()?;
        if self.descriptor.stamp().state != ProjectionState::Ready {
            return invalid("cataloged vector artifact must be ready");
        }
        if matches!(
            self.descriptor,
            VectorProjectionDescriptor::Quantized { .. }
                | VectorProjectionDescriptor::TurboQuant { .. }
        ) || matches!(
            self.kind,
            VectorArtifactKind::ScalarQuantized
                | VectorArtifactKind::ProductQuantized
                | VectorArtifactKind::BinaryQuantized
                | VectorArtifactKind::TurboQuant
        ) {
            return invalid("quantized vector artifacts must use the quantization lifecycle");
        }
        let kind_matches = matches!(
            (&self.descriptor, self.kind),
            (
                VectorProjectionDescriptor::ExactSegment { .. },
                VectorArtifactKind::ExactSegment | VectorArtifactKind::CompactDense,
            ) | (
                VectorProjectionDescriptor::Hnsw { .. },
                VectorArtifactKind::Hnsw
            )
        );
        if !kind_matches {
            return invalid("vector artifact codec kind differs from its projection descriptor");
        }
        match (&self.descriptor, &self.build_evidence) {
            (VectorProjectionDescriptor::Hnsw { descriptor }, Some(build_evidence)) => {
                build_evidence.validate()?;
                let nodes = u64::try_from(descriptor.nodes).map_err(|_| {
                    rrd_core::Error::InvalidRuntime {
                        reason: "HNSW descriptor node count exceeds u64".into(),
                    }
                })?;
                let dimensions = u64::try_from(descriptor.dimensions).map_err(|_| {
                    rrd_core::Error::InvalidRuntime {
                        reason: "HNSW descriptor dimensions exceed u64".into(),
                    }
                })?;
                if build_evidence.resources.input_vectors != nodes
                    || build_evidence.resources.dimensions != dimensions
                {
                    return invalid(
                        "HNSW build evidence resources differ from the artifact descriptor",
                    );
                }
            }
            (VectorProjectionDescriptor::Hnsw { .. }, None) => {}
            (_, Some(_)) => {
                return invalid("HNSW build evidence cannot describe another artifact kind");
            }
            (_, None) => {}
        }
        self.object.validate()?;
        if self.object.media_type != self.kind.media_type() {
            return invalid("vector artifact object media type differs from its codec kind");
        }
        if self.object.subject.as_ref() != Some(&Self::record_reference(&self.descriptor)?) {
            return invalid("vector artifact object is not attached to its catalog record");
        }
        Ok(())
    }

    fn identity_bytes(&self) -> Result<Vec<u8>> {
        let encoded = serde_json::to_vec(&(
            self.contract_version,
            self.catalog_revision,
            self.kind,
            &self.descriptor,
            &self.object,
            self.published_at,
            &self.build_evidence,
            &self.operation,
        ));
        encoded.map_err(|error| rrd_core::Error::InvalidRuntime {
            reason: format!("vector artifact catalog identity cannot be encoded: {error}"),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum VectorProjectionDescriptor {
    ExactSegment { descriptor: SegmentDescriptor },
    Hnsw { descriptor: HnswDescriptor },
    Quantized { descriptor: QuantizedDescriptor },
    TurboQuant { descriptor: TurboQuantDescriptor },
}

impl VectorProjectionDescriptor {
    pub fn stamp(&self) -> &ProjectionStamp {
        match self {
            Self::ExactSegment { descriptor } => &descriptor.stamp,
            Self::Hnsw { descriptor } => &descriptor.stamp,
            Self::Quantized { descriptor } => &descriptor.stamp,
            Self::TurboQuant { descriptor } => &descriptor.stamp,
        }
    }

    pub fn scope(&self) -> &rrd_core::ScopeId {
        match self {
            Self::ExactSegment { descriptor } => &descriptor.scope,
            Self::Hnsw { descriptor } => &descriptor.scope,
            Self::Quantized { descriptor } => &descriptor.scope,
            Self::TurboQuant { descriptor } => &descriptor.scope,
        }
    }

    fn stamp_mut(&mut self) -> &mut ProjectionStamp {
        match self {
            Self::ExactSegment { descriptor } => &mut descriptor.stamp,
            Self::Hnsw { descriptor } => &mut descriptor.stamp,
            Self::Quantized { descriptor } => &mut descriptor.stamp,
            Self::TurboQuant { descriptor } => &mut descriptor.stamp,
        }
    }

    pub fn validate(&self) -> Result<()> {
        match self {
            Self::ExactSegment { descriptor } => descriptor.validate(),
            Self::Hnsw { descriptor } => descriptor.validate(),
            Self::Quantized { descriptor } => descriptor.validate(),
            Self::TurboQuant { descriptor } => descriptor.validate(),
        }
    }

    pub fn candidate_path(&self, estimated_cost: u64) -> CandidatePath {
        match self {
            Self::ExactSegment { descriptor } => descriptor.candidate_path(estimated_cost),
            Self::Hnsw { descriptor } => descriptor.candidate_path(estimated_cost),
            Self::Quantized { descriptor } => descriptor.candidate_path(estimated_cost),
            Self::TurboQuant { descriptor } => descriptor.candidate_path(estimated_cost),
        }
    }

    fn same_kind(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::ExactSegment { .. }, Self::ExactSegment { .. })
            | (Self::Hnsw { .. }, Self::Hnsw { .. })
            | (Self::TurboQuant { .. }, Self::TurboQuant { .. }) => true,
            (Self::Quantized { descriptor: left }, Self::Quantized { descriptor: right }) => {
                left.method == right.method
            }
            _ => false,
        }
    }
}

impl From<SegmentDescriptor> for VectorProjectionDescriptor {
    fn from(descriptor: SegmentDescriptor) -> Self {
        Self::ExactSegment { descriptor }
    }
}

impl From<HnswDescriptor> for VectorProjectionDescriptor {
    fn from(descriptor: HnswDescriptor) -> Self {
        Self::Hnsw { descriptor }
    }
}

impl From<QuantizedDescriptor> for VectorProjectionDescriptor {
    fn from(descriptor: QuantizedDescriptor) -> Self {
        Self::Quantized { descriptor }
    }
}

impl From<TurboQuantDescriptor> for VectorProjectionDescriptor {
    fn from(descriptor: TurboQuantDescriptor) -> Self {
        Self::TurboQuant { descriptor }
    }
}

/// Compare-and-swap catalog for every rebuildable vector projection.
///
/// Publication moves the previous generation to `retired`. Callers may only
/// reclaim its artifact after proving no leased snapshot protects that exact
/// `(projection id, generation)` pair.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VectorCatalog {
    pub revision: u64,
    pub entries: BTreeMap<ProjectionId, VectorProjectionDescriptor>,
    #[serde(default)]
    pub retired: Vec<VectorProjectionDescriptor>,
}

impl VectorCatalog {
    /// Restores one quantization-lifecycle-selected active artifact into a
    /// fresh serving view. Unlike `publish`, the durable lifecycle has already
    /// validated the generation chain, so restart may legitimately restore
    /// generation N without replaying retired artifact bodies 1..N-1.
    /// Restoration is revision-neutral: `revision` belongs exclusively to the
    /// generic vector projection catalogue and cannot be advanced by another
    /// durable authority.
    pub fn restore_quantization_lifecycle_active(
        &mut self,
        descriptor: impl Into<VectorProjectionDescriptor>,
    ) -> Result<u64> {
        let descriptor = descriptor.into();
        descriptor.validate()?;
        if !matches!(
            descriptor,
            VectorProjectionDescriptor::Quantized { .. }
                | VectorProjectionDescriptor::TurboQuant { .. }
        ) {
            return invalid("quantization lifecycle restoration accepts only quantized artifacts");
        }
        if descriptor.stamp().state != ProjectionState::Ready
            || descriptor.stamp().id.as_str() == EXACT_SCAN_PROJECTION_ID
            || self.entries.contains_key(&descriptor.stamp().id)
        {
            return invalid("restored vector artifact is not one unique ready generation");
        }
        self.entries
            .insert(descriptor.stamp().id.clone(), descriptor);
        Ok(self.revision)
    }

    pub fn publish(
        &mut self,
        expected_revision: u64,
        descriptor: impl Into<VectorProjectionDescriptor>,
    ) -> Result<u64> {
        self.expect_revision(expected_revision)?;
        let descriptor = descriptor.into();
        descriptor.validate()?;
        if matches!(
            descriptor,
            VectorProjectionDescriptor::Quantized { .. }
                | VectorProjectionDescriptor::TurboQuant { .. }
        ) {
            return invalid("quantized vector artifacts must use the quantization lifecycle");
        }
        if descriptor.stamp().id.as_str() == EXACT_SCAN_PROJECTION_ID {
            return invalid("vector projection uses the reserved exact-scan identity");
        }
        if descriptor.stamp().state != ProjectionState::Ready {
            return invalid("only a ready vector projection can be published");
        }
        let id = descriptor.stamp().id.clone();
        match self.entries.get(&id) {
            Some(previous) => {
                if !previous.same_kind(&descriptor) {
                    return invalid("vector projection kind cannot change within one identity");
                }
                if descriptor.stamp().generation != previous.stamp().generation.saturating_add(1) {
                    return invalid("vector projection generation must advance exactly once");
                }
                if descriptor.stamp().source_cursor < previous.stamp().source_cursor {
                    return invalid("vector projection source coverage cannot move backward");
                }
            }
            None if descriptor.stamp().generation != 1 => {
                return invalid("first vector projection generation must be one");
            }
            None => {}
        }
        if let Some(mut previous) = self.entries.remove(&id) {
            previous.stamp_mut().state = ProjectionState::Retiring;
            self.retired.push(previous);
            self.retired.sort_by(|left, right| {
                left.stamp()
                    .id
                    .cmp(&right.stamp().id)
                    .then_with(|| left.stamp().generation.cmp(&right.stamp().generation))
            });
        }
        self.entries.insert(id, descriptor);
        self.bump_revision()
    }

    pub fn quarantine(
        &mut self,
        expected_revision: u64,
        id: &ProjectionId,
        generation: u64,
    ) -> Result<u64> {
        self.expect_revision(expected_revision)?;
        let descriptor =
            self.entries
                .get_mut(id)
                .ok_or_else(|| rrd_core::Error::InvalidRuntime {
                    reason: format!("vector projection {id} is absent"),
                })?;
        if descriptor.stamp().generation != generation {
            return invalid("vector quarantine generation differs from catalog");
        }
        if descriptor.stamp().state != ProjectionState::Ready {
            return invalid("only a ready vector generation can be quarantined");
        }
        descriptor.stamp_mut().state = ProjectionState::Quarantined;
        self.bump_revision()
    }

    pub fn reclaim_retired(&mut self, protected: &BTreeSet<(ProjectionId, u64)>) -> Vec<String> {
        let mut reclaimed = Vec::new();
        self.retired.retain(|descriptor| {
            if protected.contains(&(descriptor.stamp().id.clone(), descriptor.stamp().generation)) {
                true
            } else {
                reclaimed.push(descriptor.stamp().artifact_digest.clone());
                false
            }
        });
        reclaimed.sort();
        reclaimed
    }

    fn expect_revision(&self, expected_revision: u64) -> Result<()> {
        if self.revision != expected_revision {
            return invalid(format!(
                "vector catalog conflict: expected revision {expected_revision}, actual {}",
                self.revision
            ));
        }
        Ok(())
    }

    fn bump_revision(&mut self) -> Result<u64> {
        self.revision =
            self.revision
                .checked_add(1)
                .ok_or_else(|| rrd_core::Error::InvalidRuntime {
                    reason: "vector catalog revision overflowed".into(),
                })?;
        Ok(self.revision)
    }
}

#[cfg(test)]
mod tests;
