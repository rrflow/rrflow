//! Storage observation scope and bounded storage cursors.

#[test]
pub(super) fn storage_observation_has_bounded_scope_and_a_read_only_authenticated_route() {
    use rrd_contract::ReadStorageDiagnostics;
    for scope in [
        String::new(),
        "x".repeat(rrd_contract::MAX_ID_BYTES + 1),
        "instance:a\0".into(),
    ] {
        assert!(ReadStorageDiagnostics { scope }.validate().is_err());
    }
    assert!(
        serde_json::from_value::<ReadStorageDiagnostics>(serde_json::json!({
            "scope":"instance:alpha", "physical_store":"/outside"
        }))
        .is_err()
    );
    let endpoint = rrd_contract::endpoint_catalogue()
        .endpoints
        .into_iter()
        .find(|e| e.operation.as_str() == "diagnostics-storage-read")
        .unwrap();
    assert_eq!(endpoint.path, "/v1/diagnostics/storage/read");
    assert_eq!(
        endpoint.authentication,
        rrd_contract::EndpointAuthentication::SessionBearer
    );
    assert_eq!(
        endpoint.action.fixed_action(),
        Some(rrd_contract::SecurityAction::DiagnosticsRead)
    );
    assert!(!endpoint.mutation);
}

#[test]
pub(super) fn source_page_capacity_is_explicit_finite_and_part_of_sealed_configuration() {
    let mut limits = rrd_contract::StorageLimits::default();
    assert_eq!(limits.source_page_bytes, 256 * 1024 * 1024);
    limits.source_page_bytes = 0;
    assert!(limits.validate().is_err());
    limits.source_page_bytes = (1_u64 << 40) + 1;
    assert!(limits.validate().is_err());
    let mut wire = serde_json::to_value(rrd_contract::StorageLimits::default()).unwrap();
    wire.as_object_mut().unwrap().remove("source_page_bytes");
    assert!(serde_json::from_value::<rrd_contract::StorageLimits>(wire).is_err());
}

#[test]
pub(super) fn storage_cursor_rejects_unknown_actions_fields_and_unbounded_budgets() {
    use rrd_contract::{CorrelationId, InspectStorageCursor, StorageCursorAction};
    for action in [
        serde_json::json!({"kind":"observe","extra":true}),
        serde_json::json!({"kind":"release","extra":true}),
        serde_json::json!({"kind":"unknown"}),
    ] {
        assert!(serde_json::from_value::<StorageCursorAction>(action).is_err());
    }
    for (rows, bytes, lifetime) in [
        (0, 4096, 1000),
        (4097, 4096, 1000),
        (1, 0, 1000),
        (1, 4194305, 1000),
        (1, 4096, 0),
        (1, 4096, 60001),
    ] {
        assert!(InspectStorageCursor {
            scope: "instance:test".into(),
            cursor_id: CorrelationId::new("reader").unwrap(),
            action: StorageCursorAction::Open {
                batch_rows: rows,
                batch_bytes: bytes,
                lifetime_ms: lifetime
            }
        }
        .validate()
        .is_err());
    }
}

#[test]
pub(super) fn bounded_storage_cursor_requires_ordered_bounded_physical_keys() {
    use rrd_contract::{CorrelationId, InspectStorageCursor, StorageCursorAction};
    let request = |start: Vec<u8>, end: Vec<u8>| {
        let action = serde_json::from_value::<StorageCursorAction>(serde_json::json!({
            "kind": "open_range", "batch_rows": 1, "batch_bytes": 4096,
            "lifetime_ms": 1000, "start": start, "end": end
        }));
        action.map(|action| InspectStorageCursor {
            scope: "instance:test".into(),
            cursor_id: CorrelationId::new("reader").unwrap(),
            action,
        })
    };
    assert!(request(b"a".to_vec(), b"z".to_vec())
        .unwrap()
        .validate()
        .is_ok());
    for (start, end) in [
        (vec![], b"z".to_vec()),
        (b"a".to_vec(), vec![]),
        (b"z".to_vec(), b"a".to_vec()),
        (b"a".to_vec(), b"a".to_vec()),
        (vec![1; 4097], vec![2; 4097]),
        (b"a".to_vec(), vec![2; 4097]),
    ] {
        assert!(request(start, end).unwrap().validate().is_err());
    }
    let invalid_budget = serde_json::json!({
        "kind": "open_range", "batch_rows": 0, "batch_bytes": 4096,
        "lifetime_ms": 1000, "start": [97], "end": [98]
    });
    let action = serde_json::from_value::<StorageCursorAction>(invalid_budget).unwrap();
    assert!(InspectStorageCursor {
        scope: "instance:test".into(),
        cursor_id: CorrelationId::new("reader").unwrap(),
        action,
    }
    .validate()
    .is_err());
    let extra = serde_json::json!({
        "kind": "open_range", "batch_rows": 1, "batch_bytes": 4096,
        "lifetime_ms": 1000, "start": [97], "end": [98], "extra": true
    });
    assert!(serde_json::from_value::<StorageCursorAction>(extra).is_err());
}

#[test]
pub(super) fn bounded_storage_cursor_schema_exposes_the_runtime_limits() {
    let schema =
        serde_json::to_value(schemars::schema_for!(rrd_contract::StorageCursorAction)).unwrap();
    let variants = schema["oneOf"].as_array().unwrap();
    let range = variants
        .iter()
        .find(|v| v["properties"]["kind"]["const"] == "open_range")
        .unwrap();
    for field in ["start", "end"] {
        assert_eq!(range["properties"][field]["minItems"], 1);
        assert_eq!(range["properties"][field]["maxItems"], 4096);
    }
    for (field, max) in [
        ("batch_rows", 4096),
        ("batch_bytes", 4 * 1024 * 1024),
        ("lifetime_ms", 60000),
    ] {
        assert_eq!(range["properties"][field]["minimum"], 1);
        assert_eq!(range["properties"][field]["maximum"], max);
    }
    fn projected(value: &serde_json::Value) -> bool {
        if value["properties"]["kind"]["const"] == "open_range"
            && value["properties"]["start"]["minItems"] == 1
            && value["properties"]["start"]["maxItems"] == 4096
            && value["properties"]["end"]["minItems"] == 1
            && value["properties"]["end"]["maxItems"] == 4096
            && value["properties"]["lifetime_ms"]["maximum"] == 60000
        {
            return true;
        }
        match value {
            serde_json::Value::Array(items) => items.iter().any(projected),
            serde_json::Value::Object(fields) => fields.values().any(projected),
            _ => false,
        }
    }
    assert!(projected(&rrd_contract::openapi_document().unwrap()));
}

#[test]
pub(super) fn storage_cursor_record_kind_is_typed_and_preserves_diagnostic_budgets() {
    use rrd_contract::{CorrelationId, InspectStorageCursor, StorageCursorAction};
    let action = |kind: &str, rows: u16| {
        serde_json::from_value::<StorageCursorAction>(serde_json::json!({
            "kind": "open_record_kind", "record_kind": kind,
            "batch_rows": rows, "batch_bytes": 4096, "lifetime_ms": 1000
        }))
    };
    let request = InspectStorageCursor {
        scope: "instance:test".into(),
        cursor_id: CorrelationId::new("reader").unwrap(),
        action: action("capacity_document", 1).unwrap(),
    };
    assert!(request.validate().is_ok());
    assert!(action("", 1).is_err());
    assert!(action("NOT-A-CANONICAL-ID", 1).is_err());
    assert!(InspectStorageCursor {
        action: action("capacity_document", 0).unwrap(),
        ..request
    }
    .validate()
    .is_err());
    assert!(
        serde_json::from_value::<StorageCursorAction>(serde_json::json!({
            "kind": "open_record_kind", "record_kind": "capacity_document",
            "batch_rows": 1, "batch_bytes": 4096, "lifetime_ms": 1000,
            "start": [1]
        }))
        .is_err()
    );
}
