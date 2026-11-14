use super::*;
use rrd_contract::SearchVectors;
use rrd_contract::{
    CanonicalId, EnsureVectorIndex, EnsureVectorPayloadIndex, VectorIndexConfiguration,
    VectorPayloadIndexKind,
};

#[test]
fn a_replaced_session_outlasts_the_slowest_request_still_using_it() {
    const SLOWEST_REQUEST: Duration = Duration::from_secs(300);
    assert!(SESSION_REPLACE_AFTER + SLOWEST_REQUEST < Duration::from_millis(SESSION_LEASE_MS));
}

#[test]
fn point_mutations_decode_as_public_transaction_mutations() {
    let batch = PointBatch {
        ids: &[7],
        vectors: &[0.6, 0.8],
        categories: &[3],
        dimensions: 2,
    };
    let value = point_mutations(("lab_point", "lab_vector", "lab"), &batch);
    let mutations: Vec<TransactionMutation> = decode(value).unwrap();
    assert_eq!(mutations.len(), MUTATIONS_PER_POINT);
    let TransactionMutation::PutVector { properties, .. } = &mutations[1] else {
        panic!("second mutation should be the vector");
    };
    assert_eq!(
        properties.get("category"),
        Some(&rrd_contract::QueryValue::Unsigned(3))
    );
}

#[test]
fn point_references_round_trip_and_reject_anything_else() {
    assert_eq!(parse_point("p42"), Ok(42));
    assert!(parse_point("q42").is_err());
    assert!(parse_point("p").is_err());
}

#[test]
fn exact_and_quantized_search_modes_decode_as_public_requests() {
    for (index, expected) in [
        (IndexSpec::Exact, "exact"),
        (
            IndexSpec::TurboQuant {
                bits: "bits4".into(),
                seed: 7,
            },
            "require_approximate",
        ),
        (
            IndexSpec::Hnsw {
                m: 16,
                ef_construction: 100,
            },
            "require_approximate",
        ),
    ] {
        let mode = search_mode(&index, Some(60), Some(40), 10);
        assert_eq!(mode["mode"], expected);
        let request: SearchVectors = decode(json!({
            "scope": "instance:lab",
            "valid_at": VALID_AT,
            "collection_id": "lab",
            "vector_name": VECTOR_NAME,
            "query": {"kind": "dense", "values": [1.0, 0.0]},
            "top_k": 10,
            "mode": mode,
            "filter": {
                "kind": "condition",
                "condition": {
                    "property": "category",
                    "operator": {"operator": "equals", "value": {"type": "unsigned", "value": 3}}
                }
            },
            "max_storage_keys": MAX_VECTOR_STORAGE_KEYS
        }))
        .unwrap();
        request.validate().unwrap();
    }
}

#[test]
fn an_hnsw_build_and_its_payload_index_decode_as_public_requests() {
    let declared: EnsureVectorPayloadIndex =
        decode(payload_index_request("instance:lab", "lab")).unwrap();
    declared.validate().unwrap();
    assert_eq!(declared.field.as_str(), "category");
    assert_eq!(declared.kind, VectorPayloadIndexKind::Unsigned);

    let built: EnsureVectorIndex =
        decode(hnsw_index_request("instance:lab", "lab", 16, 100)).unwrap();
    built.validate().unwrap();
    assert_eq!(built.vector_name.as_str(), VECTOR_NAME);
    assert_eq!(built.max_storage_keys, MAX_VECTOR_STORAGE_KEYS);
    let VectorIndexConfiguration::Hnsw {
        m,
        ef_construction,
        filter_properties,
        ..
    } = built.configuration;
    assert_eq!((m, ef_construction), (16, 100));
    assert_eq!(
        filter_properties
            .iter()
            .map(CanonicalId::as_str)
            .collect::<Vec<_>>(),
        vec!["category"],
        "the graph must cover the filter the payload index declares"
    );
}

#[test]
fn only_a_graph_is_pinned() {
    assert_eq!(
        memory_tier(&IndexSpec::Hnsw {
            m: 16,
            ef_construction: 100
        }),
        "pinned"
    );
    assert_eq!(memory_tier(&IndexSpec::Exact), "cached");
    assert_eq!(
        memory_tier(&IndexSpec::TurboQuant {
            bits: "bits4".into(),
            seed: 7
        }),
        "cached"
    );
}
