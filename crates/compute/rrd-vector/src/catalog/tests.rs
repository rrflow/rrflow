use super::*;
use crate::{ScoreMetric, VectorSegmentConfig};
use rrd_core::{ObjectReceipt, ScopeId, DATA_RUNTIME_CONTRACT_VERSION};
use std::collections::BTreeSet;

fn segment(id: &str, generation: u64, cursor: u64) -> SegmentDescriptor {
    let scope = ScopeId::new("instance:catalog").unwrap();
    let config = VectorSegmentConfig {
        id: ProjectionId::new(id).unwrap(),
        scope: scope.clone(),
        field: "body".into(),
        dimensions: 2,
        metric: ScoreMetric::Dot,
        embedding_model: None,
        filter_properties: BTreeSet::new(),
    };
    SegmentDescriptor {
        stamp: ProjectionStamp {
            contract_version: DATA_RUNTIME_CONTRACT_VERSION,
            id: config.id.clone(),
            generation,
            source_cursor: cursor,
            config_digest: config.digest().unwrap(),
            artifact_digest: format!("{generation:064x}"),
            state: ProjectionState::Ready,
        },
        scope,
        field: "body".into(),
        dimensions: 2,
        metric: ScoreMetric::Dot,
        embedding_model: None,
        filter_properties: BTreeSet::new(),
        minimum_cursor: 0,
        candidate_versions: cursor as usize,
    }
}

fn artifact_entry() -> VectorArtifactCatalogEntry {
    let descriptor = VectorProjectionDescriptor::from(segment("vector-body", 1, 1));
    let subject = VectorArtifactCatalogEntry::record_reference(&descriptor).unwrap();
    let bytes = b"current vector artifact";
    let object_key = ObjectReference::canonical_key(&digest::sha256_hex(bytes)).unwrap();
    let object = ObjectReference::for_bytes(
        "vector-artifact",
        Some(subject),
        VectorArtifactKind::ExactSegment.media_type(),
        bytes,
        ObjectReceipt {
            backend: "memory".into(),
            key: object_key,
            version: None,
            etag: None,
        },
    )
    .unwrap();
    VectorArtifactCatalogEntry::new(1, VectorArtifactKind::ExactSegment, descriptor, object, 1)
        .unwrap()
}

fn hnsw_entry(source: VectorCollectionAddress) -> VectorArtifactCatalogEntry {
    let scope = ScopeId::new("instance:catalog").unwrap();
    let index = crate::HnswIndex::build(
        crate::HnswConfig {
            id: ProjectionId::new("hnsw-bound").unwrap(),
            scope,
            field: "body".into(),
            dimensions: 2,
            metric: ScoreMetric::Dot,
            embedding_model: None,
            m: 4,
            ef_construction: 16,
            max_level: 4,
            seed: 7,
            filter_properties: BTreeSet::new(),
        },
        1,
        0,
        Vec::new(),
    )
    .unwrap();
    let descriptor = VectorProjectionDescriptor::from(index.descriptor().clone());
    let subject = VectorArtifactCatalogEntry::record_reference(&descriptor).unwrap();
    let bytes = index.as_bytes();
    let object = ObjectReference::for_bytes(
        "hnsw-artifact",
        Some(subject),
        VectorArtifactKind::Hnsw.media_type(),
        bytes,
        ObjectReceipt {
            backend: "memory".into(),
            key: ObjectReference::canonical_key(&digest::sha256_hex(bytes)).unwrap(),
            version: None,
            etag: None,
        },
    )
    .unwrap();
    VectorArtifactCatalogEntry::new_with_source(
        1,
        VectorArtifactKind::Hnsw,
        descriptor,
        object,
        1,
        None,
        Some(VectorArtifactOperationReceipt {
            idempotency_key: "source-test".into(),
            operation_sha256: "11".repeat(32),
            source: None,
        }),
        Some(source),
    )
    .unwrap()
}

#[test]
fn named_source_identity_v1_is_stable_and_delimiter_safe() {
    let first = VectorCollectionAddress {
        collection_id: "a-b".into(),
        vector_name: "c".into(),
    };
    let second = VectorCollectionAddress {
        collection_id: "a".into(),
        vector_name: "b-c".into(),
    };
    assert_eq!(
        vector_source_digest(&first).unwrap(),
        "8dd7f0b7ea2710de712692ad9aba5d6d4d8a0535986574f15a6ea5fac7dfd121"
    );
    assert_eq!(
        vector_source_digest(&second).unwrap(),
        "c61e1886c6a087daa1169ebab7be4f5e6d366a29fc702ae16ef000351b13d9a8"
    );
}

#[test]
fn named_source_is_authenticated_and_older_entries_remain_unbound() {
    let source = VectorCollectionAddress {
        collection_id: "documents".into(),
        vector_name: "body".into(),
    };
    let entry = hnsw_entry(source.clone());
    assert!(entry.validate().is_ok());
    let mut tampered = entry.clone();
    tampered
        .operation
        .as_mut()
        .unwrap()
        .source
        .as_mut()
        .unwrap()
        .address
        .collection_id = "other".into();
    assert!(tampered.validate().is_err());

    // Reconstruct the previous format's exact identity calculation rather
    // than deriving an expected digest from the new implementation.
    let mut old = entry;
    old.contract_version = 2;
    old.operation.as_mut().unwrap().source = None;
    let bytes = serde_json::to_vec(&(
        old.contract_version,
        old.catalog_revision,
        old.kind,
        &old.descriptor,
        &old.object,
        old.published_at,
        &old.build_evidence,
        &old.operation,
    ))
    .unwrap();
    old.entry_digest = digest::sha256_hex(&bytes);
    let restored: VectorArtifactCatalogEntry =
        serde_json::from_slice(&serde_json::to_vec(&old).unwrap()).unwrap();
    assert!(restored.validate().is_ok());
    assert_eq!(restored.source(), None);
    old.operation.as_mut().unwrap().source = Some(VectorArtifactSource {
        contract_version: 1,
        address: source,
    });
    assert!(old.validate().is_err());
}

fn turboquant_descriptor() -> VectorProjectionDescriptor {
    VectorProjectionDescriptor::TurboQuant {
        descriptor: TurboQuantDescriptor {
            stamp: ProjectionStamp {
                contract_version: DATA_RUNTIME_CONTRACT_VERSION,
                id: ProjectionId::new("quant-turbo-body").unwrap(),
                generation: 1,
                source_cursor: 1,
                config_digest: "11".repeat(32),
                artifact_digest: "22".repeat(32),
                state: ProjectionState::Ready,
            },
            scope: ScopeId::new("instance:catalog").unwrap(),
            field: "body".into(),
            dimensions: 2,
            metric: ScoreMetric::Dot,
            bits: crate::TurboQuantBits::Bits2,
            seed: 7,
            embedding_model: None,
            filter_properties: BTreeSet::new(),
            minimum_cursor: 0,
            candidate_versions: 1,
            packed_vector_bytes: 1,
            full_precision_vector_bytes: 8,
            estimator: crate::TurboQuantEstimator::Mse,
        },
    }
}

#[test]
fn generic_catalogue_rejects_turboquant_artifacts() {
    let descriptor = turboquant_descriptor();
    let subject = VectorArtifactCatalogEntry::record_reference(&descriptor).unwrap();
    let bytes = b"generic TurboQuant bytes must never become authoritative";
    let object = ObjectReference::for_bytes(
        "vector-artifact",
        Some(subject),
        VectorArtifactKind::TurboQuant.media_type(),
        bytes,
        ObjectReceipt {
            backend: "memory".into(),
            key: ObjectReference::canonical_key(&digest::sha256_hex(bytes)).unwrap(),
            version: None,
            etag: None,
        },
    )
    .unwrap();

    let error =
        VectorArtifactCatalogEntry::new(1, VectorArtifactKind::TurboQuant, descriptor, object, 1)
            .unwrap_err()
            .to_string();
    assert!(error.contains("quantization lifecycle"));
}

#[test]
fn pre_1_0_vector_artifact_catalogue_entries_are_rejected() {
    let mut entry = artifact_entry();
    entry.contract_version = 1;
    entry.entry_digest = digest::sha256_hex(&entry.identity_bytes().unwrap());

    assert!(entry
        .validate()
        .unwrap_err()
        .to_string()
        .contains("version"));
}

#[test]
fn catalog_enforces_cas_generation_retirement_and_quarantine() {
    let id = ProjectionId::new("vector-body").unwrap();
    let mut catalog = VectorCatalog::default();
    assert_eq!(catalog.publish(0, segment(id.as_str(), 1, 1)).unwrap(), 1);
    assert!(catalog.publish(0, segment(id.as_str(), 2, 2)).is_err());
    assert_eq!(catalog.publish(1, segment(id.as_str(), 2, 2)).unwrap(), 2);
    assert_eq!(catalog.retired.len(), 1);
    let protected = BTreeSet::from([(id.clone(), 1)]);
    assert!(catalog.reclaim_retired(&protected).is_empty());
    assert_eq!(catalog.reclaim_retired(&BTreeSet::new()).len(), 1);
    assert_eq!(catalog.quarantine(2, &id, 2).unwrap(), 3);
    assert_eq!(
        catalog.entries[&id].stamp().state,
        ProjectionState::Quarantined
    );
}
