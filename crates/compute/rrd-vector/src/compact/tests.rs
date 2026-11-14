use super::*;
use rrd_core::{ProjectionId, ScopeId};
use std::collections::BTreeSet;

#[test]
fn artifact_reader_rejects_candidate_without_collection_address() {
    let scope = ScopeId::new("instance:compact-required-address").unwrap();
    let segment = CompactDenseSegment::build(
        VectorSegmentConfig {
            id: ProjectionId::new("vector-compact-required-address").unwrap(),
            scope: scope.clone(),
            field: "body".into(),
            dimensions: 2,
            metric: ScoreMetric::Dot,
            embedding_model: None,
            filter_properties: BTreeSet::new(),
        },
        1,
        1,
        [VectorCandidate {
            scope,
            source_cursor: 1,
            vector: RuntimeVector {
                reference: RuntimeRef::new("embedding", "a").unwrap(),
                subject: RuntimeRef::new("document", "a").unwrap(),
                collection: VectorCollectionAddress {
                    collection_id: "documents".into(),
                    vector_name: "body".into(),
                },
                field: "body".into(),
                valid_from: 1,
                valid_to: None,
                value: VectorValue::Dense {
                    values: vec![1.0, 0.0],
                },
                provenance: None,
                properties: RuntimeProperties::new(),
            },
        }],
    )
    .unwrap();
    let old_header = parse_header(segment.as_bytes()).unwrap();
    let mut metadata: serde_json::Value = serde_json::from_slice(
        &segment.as_bytes()[HEADER_BYTES..HEADER_BYTES + old_header.metadata_len],
    )
    .unwrap();
    metadata["candidates"][0]
        .as_object_mut()
        .unwrap()
        .remove("collection");
    let metadata = serde_json::to_vec(&metadata).unwrap();
    let vector_offset = align_up(HEADER_BYTES + metadata.len(), ALIGNMENT).unwrap();
    let payload = &segment.as_bytes()[old_header.vector_offset..];
    let mut bytes = vec![0_u8; vector_offset + payload.len()];
    write_header(
        &mut bytes,
        metadata.len(),
        vector_offset,
        payload.len(),
        old_header.row_stride,
        old_header.rows,
        old_header.dimensions,
    )
    .unwrap();
    bytes[HEADER_BYTES..HEADER_BYTES + metadata.len()].copy_from_slice(&metadata);
    bytes[vector_offset..].copy_from_slice(payload);
    let checksum = artifact_digest(&bytes).unwrap();
    bytes[DIGEST_OFFSET..DIGEST_OFFSET + DIGEST_BYTES].copy_from_slice(&checksum);

    let error = CompactDenseSegment::from_bytes(&bytes).unwrap_err();
    assert!(error.to_string().contains("missing field `collection`"));
}
