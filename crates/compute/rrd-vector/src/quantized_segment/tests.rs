use super::*;
use crate::SearchMode;
use rrd_core::{ReadStamp, RuntimeVector};
use std::io::Write;

fn candidate(scope: &ScopeId, id: usize, values: Vec<f32>) -> VectorCandidate {
    VectorCandidate {
        scope: scope.clone(),
        source_cursor: id as u64 + 1,
        vector: RuntimeVector {
            reference: RuntimeRef::new("embedding", format!("v{id}")).unwrap(),
            subject: RuntimeRef::new("document", format!("v{id}")).unwrap(),
            collection: VectorCollectionAddress {
                collection_id: "documents".into(),
                vector_name: "body".into(),
            },
            field: "body".into(),
            valid_from: 1,
            valid_to: None,
            value: VectorValue::Dense { values },
            provenance: None,
            properties: RuntimeProperties::new(),
        },
    }
}

fn segment(method: QuantizationMethod) -> QuantizedSegment {
    let scope = ScopeId::new("instance:quantized-segment").unwrap();
    let candidates = (0..12)
        .map(|row| {
            candidate(
                &scope,
                row,
                (0..64)
                    .map(|dimension| ((row * 17 + dimension * 13) % 101) as f32 / 50.0 - 1.0)
                    .collect(),
            )
        })
        .collect::<Vec<_>>();
    QuantizedSegment::build(
        QuantizedSegmentConfig {
            id: ProjectionId::new(format!("quant-{}", method.as_str())).unwrap(),
            scope,
            field: "body".into(),
            dimensions: 64,
            metric: ScoreMetric::Cosine,
            method,
            embedding_model: None,
            filter_properties: BTreeSet::new(),
        },
        1,
        12,
        candidates,
    )
    .unwrap()
}

#[test]
fn artifact_reader_rejects_candidate_without_collection_address() {
    let segment = segment(QuantizationMethod::Scalar);
    let old_header = read_header(segment.as_bytes()).unwrap();
    let mut metadata: serde_json::Value = serde_json::from_slice(
        &segment.as_bytes()[HEADER_BYTES..HEADER_BYTES + old_header.metadata_len],
    )
    .unwrap();
    metadata["candidates"][0]
        .as_object_mut()
        .unwrap()
        .remove("collection");
    let metadata = serde_json::to_vec(&metadata).unwrap();
    let payload = &segment.as_bytes()[old_header.payload_offset..];
    let payload_offset = align_up(HEADER_BYTES + metadata.len(), ALIGNMENT).unwrap();
    let mut bytes = vec![0_u8; payload_offset + payload.len()];
    write_header(
        &mut bytes,
        metadata.len(),
        payload_offset,
        payload.len(),
        old_header.row_bytes,
        old_header.rows,
        old_header.dimensions,
    )
    .unwrap();
    bytes[HEADER_BYTES..HEADER_BYTES + metadata.len()].copy_from_slice(&metadata);
    bytes[payload_offset..].copy_from_slice(payload);
    let checksum = artifact_digest(&bytes).unwrap();
    bytes[DIGEST_OFFSET..DIGEST_OFFSET + DIGEST_BYTES].copy_from_slice(&checksum);

    let error = QuantizedSegment::from_bytes(&bytes).unwrap_err();
    assert!(error.to_string().contains("missing field `collection`"));
}

#[test]
fn scalar_product_binary_round_trip_corruption_mmap_and_kernel_matrix() {
    for method in [
        QuantizationMethod::Scalar,
        QuantizationMethod::Product {
            compression: ProductCompression::X64,
        },
        QuantizationMethod::Binary,
    ] {
        let segment = segment(method);
        let reopened = QuantizedSegment::from_bytes(segment.as_bytes()).unwrap();
        assert_eq!(segment, reopened);
        assert!(
            reopened.descriptor().full_precision_vector_bytes
                / reopened.descriptor().packed_vector_bytes
                <= method.maximum_compression_ratio()
        );
        let mut corrupt = segment.as_bytes().to_vec();
        let last = corrupt.len() - 1;
        corrupt[last] ^= 1;
        assert!(QuantizedSegment::from_bytes(&corrupt).is_err());

        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(segment.as_bytes()).unwrap();
        file.flush().unwrap();
        let mapped = QuantizedSegment::open_mmap(file.path()).unwrap();
        assert_eq!(mapped.memory_placement(), QuantizedMemoryPlacement::Mapped);

        let scope = ScopeId::new("instance:quantized-segment").unwrap();
        let query = SearchRequest {
            read: ReadStamp::new(scope.clone(), None, 0, 12, Some("11".repeat(32))).unwrap(),
            scope,
            valid_at: 10,
            field: "body".into(),
            query: VectorQuery::Dense {
                values: (0..64)
                    .map(|dimension| ((3 * 17 + dimension * 13) % 101) as f32 / 50.0 - 1.0)
                    .collect(),
            },
            metric: ScoreMetric::Cosine,
            embedding_model: None,
            top_k: 3,
            mode: SearchMode::RequireApproximate { exact_rerank: 8 },
            filter: None,
        };
        let scalar = mapped
            .search_candidates_at(&query, 8, 12, QuantizedKernel::Scalar)
            .unwrap();
        let auto = mapped
            .search_candidates_at(&query, 8, 12, QuantizedKernel::Auto)
            .unwrap();
        assert_eq!(
            scalar.iter().map(|hit| &hit.reference).collect::<Vec<_>>(),
            auto.iter().map(|hit| &hit.reference).collect::<Vec<_>>()
        );
        for (scalar, auto) in scalar.iter().zip(&auto) {
            assert!((scalar.score - auto.score).abs() < 1.0e-5);
        }
        assert!(scalar.iter().any(|hit| hit.reference.id.as_str() == "v3"));
    }
}
