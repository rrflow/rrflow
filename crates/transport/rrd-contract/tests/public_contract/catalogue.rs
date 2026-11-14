//! Storage observations, the endpoint catalogue, OpenAPI, subscriptions and transactions.

use super::*;

#[test]
pub(super) fn diagnostic_storage_observation_matches_its_profile_and_evidence_level() {
    use rrd_contract::{DiagnosticStorageEvidenceLevel as Level, StorageProfileKind as Profile};
    let storage = |profile, evidence_level, rrflow_kv| rrd_contract::DiagnosticStorageSnapshot {
        admission: None,
        observed_at_unix_ms: 1_000,
        profile,
        evidence_level,
        rrflow_kv,
    };

    storage(
        Profile::RrflowKv,
        Level::RrflowKvCounters,
        Some(rrflow_kv_storage_counters()),
    )
    .validate()
    .unwrap();
    storage(Profile::RrflowMx, Level::LogicalOnly, None)
        .validate()
        .unwrap();
    assert!(storage(Profile::RrflowKv, Level::RrflowKvCounters, None)
        .validate()
        .is_err());
    assert!(storage(
        Profile::RrflowMx,
        Level::RrflowKvCounters,
        Some(rrflow_kv_storage_counters())
    )
    .validate()
    .is_err());
    assert!(storage(
        Profile::RrflowKv,
        Level::LogicalOnly,
        Some(rrflow_kv_storage_counters())
    )
    .validate()
    .is_err());
    let mut empty_reason = rrflow_kv_storage_counters();
    empty_reason.segment_io.last_fallback_reason = Some(String::new());
    assert!(storage(
        Profile::RrflowKv,
        Level::RrflowKvCounters,
        Some(empty_reason)
    )
    .validate()
    .is_err());
    let mut observed_at_zero = storage(Profile::RrflowMx, Level::LogicalOnly, None);
    observed_at_zero.observed_at_unix_ms = 0;
    assert!(observed_at_zero.validate().is_err());

    let encoded = serde_json::to_value(storage(
        Profile::RrflowKv,
        Level::RrflowKvCounters,
        Some(rrflow_kv_storage_counters()),
    ))
    .unwrap();
    assert_eq!(encoded["profile"], "rrflow_kv");
    assert_eq!(encoded["evidence_level"], "rrflow_kv_counters");
    assert_eq!(encoded["rrflow_kv"]["segment_io"]["requested_mode"], "auto");
    assert!(encoded["rrflow_kv"]["segment_io"]
        .get("last_fallback_reason")
        .is_none());
    let mut unknown = encoded;
    unknown["rrflow_kv"]["page_cache"]["physical_path"] = "/tmp/escape".into();
    assert!(serde_json::from_value::<rrd_contract::DiagnosticStorageSnapshot>(unknown).is_err());
    assert_eq!(
        serde_json::to_value(rrd_contract::DiagnosticAuthority::Observation).unwrap(),
        "observation"
    );
}

#[test]
pub(super) fn endpoint_catalogue_is_complete_sorted_and_transport_neutral() {
    let catalogue = rrd_contract::endpoint_catalogue();
    catalogue.validate().unwrap();
    assert!(catalogue
        .endpoints
        .iter()
        .any(|endpoint| endpoint.operation.as_str() == "attunement-replan-accept"));
    assert_eq!(
        catalogue.endpoints[0].operation.as_str(),
        "attunement-cancel"
    );
    let create = catalogue
        .endpoints
        .iter()
        .find(|endpoint| endpoint.operation.as_str() == "session-create")
        .unwrap();
    assert_eq!(create.path, "/v1/sessions");
    assert!(create.mutation);
    assert_eq!(
        create.authentication,
        rrd_contract::EndpointAuthentication::ApiKey
    );
    assert!(catalogue.endpoints.iter().all(|endpoint| {
        !endpoint.request_type.contains("::") && !endpoint.response_type.contains("::")
    }));
    let context = catalogue
        .endpoints
        .iter()
        .find(|endpoint| endpoint.operation.as_str() == "context-assemble")
        .unwrap();
    assert_eq!(
        context.action.fixed_action(),
        Some(rrd_contract::SecurityAction::MemoryContextRead)
    );
    assert_eq!(context.path, "/v1/context/assemble");
    for (operation, path, action, mutation, request, response) in [
        (
            "knowledge-tree-list",
            "/v1/knowledge/trees/list",
            rrd_contract::SecurityAction::KnowledgeTreeRead,
            false,
            "ListKnowledgeTrees",
            "KnowledgeTreeCatalogue",
        ),
        (
            "knowledge-tree-replace",
            "/v1/knowledge/trees/replace",
            rrd_contract::SecurityAction::KnowledgeTreeWrite,
            true,
            "ReplaceKnowledgeTrees",
            "ReplaceKnowledgeTreesResult",
        ),
    ] {
        let descriptor = catalogue
            .endpoints
            .iter()
            .find(|endpoint| endpoint.operation.as_str() == operation)
            .unwrap();
        assert_eq!(descriptor.path, path);
        assert_eq!(descriptor.method, rrd_contract::HttpMethod::Post);
        assert_eq!(
            descriptor.authentication,
            rrd_contract::EndpointAuthentication::SessionBearer
        );
        assert_eq!(descriptor.action.fixed_action(), Some(action));
        assert_eq!(descriptor.mutation, mutation);
        assert_eq!(descriptor.request_type, request);
        assert_eq!(descriptor.response_type, response);
    }
    let encoded = serde_json::to_value(&catalogue).unwrap();
    assert_eq!(
        serde_json::from_value::<rrd_contract::EndpointCatalogue>(encoded).unwrap(),
        catalogue
    );
}

#[test]
pub(super) fn openapi_is_derived_from_every_catalogue_operation_and_wire_type() {
    let catalogue = rrd_contract::endpoint_catalogue();
    let document = rrd_contract::openapi_document().unwrap();
    assert_eq!(document["openapi"], "3.1.0");
    assert_eq!(document["x-rrd-protocol"], PROTOCOL);
    assert_eq!(document["x-rrd-protocol-version"], PROTOCOL_VERSION);
    assert_eq!(document["x-rrd-endpoint-count"], catalogue.endpoints.len());
    assert_eq!(
        document["x-rrd-websocket-endpoint-count"],
        catalogue.websocket_endpoints.len()
    );
    assert_eq!(
        document["x-rrd-websocket-endpoints"],
        serde_json::to_value(&catalogue.websocket_endpoints).unwrap()
    );
    assert!(document["components"]["schemas"]["WebSocketFrame"].is_object());
    for endpoint in catalogue.endpoints {
        let method = match endpoint.method {
            rrd_contract::HttpMethod::Get => "get",
            rrd_contract::HttpMethod::Post => "post",
            rrd_contract::HttpMethod::Delete => "delete",
        };
        let operation = &document["paths"][&endpoint.path][method];
        assert_eq!(operation["operationId"], endpoint.operation.as_str());
        assert_eq!(operation["summary"], endpoint.summary.as_str());
        assert_eq!(operation["description"], endpoint.description.as_str());
        assert_eq!(
            operation["tags"],
            serde_json::json!([endpoint.family.as_str()])
        );
        assert!(operation["responses"]["200"]["content"]["application/json"]["schema"].is_object());
        if endpoint.method != rrd_contract::HttpMethod::Get {
            assert!(operation["requestBody"]["content"]["application/json"]["schema"].is_object());
        }
    }
    let encoded = serde_json::to_string(&document).unwrap();
    assert!(!encoded.contains("rrd_store"));
    assert!(!encoded.contains("rrd_server"));
    let mut pretty = serde_json::to_vec_pretty(&document).unwrap();
    pretty.push(b'\n');
    assert_eq!(
        sha256_for_test(&pretty),
        rrd_contract::OPENAPI_DOCUMENT_SHA256,
        "OpenAPI drift requires an intentional protocol/schema review"
    );
}

/// `rrd-contract` has no workspace dependencies, so `CanonicalId` keeps its own
/// copy of the kernel's grammar. This proves they cannot drift apart.
#[test]
pub(super) fn canonical_id_agrees_with_the_kernel_grammar() {
    let mut values = vec![
        String::new(),
        "a".repeat(rrd_core::ident::MAX_CANONICAL_ID_BYTES),
        "a".repeat(rrd_core::ident::MAX_CANONICAL_ID_BYTES + 1),
    ];
    for byte in 0..=u8::MAX {
        let character = char::from(byte);
        values.push(character.to_string());
        values.push(format!("a{character}"));
        values.push(format!("{character}a"));
    }
    assert_eq!(
        rrd_contract::MAX_ID_BYTES,
        rrd_core::ident::MAX_CANONICAL_ID_BYTES
    );
    for value in values {
        assert_eq!(
            rrd_contract::CanonicalId::new(value.clone()).is_ok(),
            rrd_core::ident::canonical_id_problem(&value).is_none(),
            "the two grammars disagree on {value:?}"
        );
    }
}

#[test]
pub(super) fn every_catalogue_operation_carries_its_own_reference_text() {
    let catalogue = rrd_contract::endpoint_catalogue();
    catalogue.validate().unwrap();
    let mut operations = catalogue
        .endpoints
        .iter()
        .map(|endpoint| endpoint.operation.as_str().to_owned())
        .chain(
            catalogue
                .websocket_endpoints
                .iter()
                .map(|endpoint| endpoint.operation.as_str().to_owned()),
        )
        .collect::<Vec<_>>();
    operations.sort();
    // No operation without text, and no text left behind by a removed one.
    assert_eq!(
        operations,
        rrd_contract::referenced_operations()
            .map(str::to_owned)
            .collect::<Vec<_>>()
    );
    assert_eq!(
        catalogue
            .families
            .iter()
            .map(|family| family.family)
            .collect::<Vec<_>>(),
        rrd_contract::OperationFamily::ALL.to_vec()
    );
    for family in rrd_contract::OperationFamily::ALL {
        assert!(
            catalogue
                .endpoints
                .iter()
                .map(|endpoint| endpoint.family)
                .chain(
                    catalogue
                        .websocket_endpoints
                        .iter()
                        .map(|endpoint| endpoint.family)
                )
                .any(|used| used == family),
            "family {} has no operation",
            family.as_str()
        );
    }
    let summaries = catalogue
        .endpoints
        .iter()
        .map(|endpoint| endpoint.summary.as_str())
        .chain(
            catalogue
                .websocket_endpoints
                .iter()
                .map(|endpoint| endpoint.summary.as_str()),
        )
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        summaries.len(),
        operations.len(),
        "each operation needs its own summary"
    );

    let document = rrd_contract::openapi_document().unwrap();
    assert_eq!(
        document["tags"]
            .as_array()
            .unwrap()
            .iter()
            .map(|tag| tag["name"].as_str().unwrap())
            .collect::<Vec<_>>(),
        rrd_contract::OperationFamily::ALL
            .iter()
            .map(|family| family.as_str())
            .collect::<Vec<_>>()
    );
}

#[test]
pub(super) fn catalogue_validation_refuses_missing_or_malformed_reference_text() {
    let catalogue = rrd_contract::endpoint_catalogue();
    let reject = |change: &dyn Fn(&mut rrd_contract::EndpointCatalogue)| {
        let mut changed = catalogue.clone();
        change(&mut changed);
        assert!(changed.validate().is_err());
    };
    reject(&|catalogue| catalogue.endpoints[0].summary.clear());
    reject(&|catalogue| catalogue.endpoints[0].summary.push('.'));
    reject(&|catalogue| catalogue.endpoints[0].summary.push_str(&"x".repeat(80)));
    reject(&|catalogue| catalogue.endpoints[0].description.clear());
    reject(&|catalogue| catalogue.endpoints[0].description.push_str(" More"));
    reject(&|catalogue| catalogue.endpoints[0].description.push('\n'));
    reject(&|catalogue| catalogue.endpoints[0].description.insert(0, ' '));
    reject(&|catalogue| catalogue.websocket_endpoints[0].summary.clear());
    reject(&|catalogue| catalogue.families.clear());
    reject(&|catalogue| {
        let family = catalogue.endpoints[0].family;
        catalogue
            .families
            .retain(|described| described.family != family);
    });
    reject(&|catalogue| {
        let duplicate = catalogue.families[0].clone();
        catalogue.families.push(duplicate);
    });
    reject(&|catalogue| catalogue.families[0].title.push('.'));
    let encoded = serde_json::to_value(&catalogue).unwrap();
    let mut without_families = encoded.clone();
    without_families.as_object_mut().unwrap().remove("families");
    assert!(serde_json::from_value::<rrd_contract::EndpointCatalogue>(without_families).is_err());
    let mut without_summary = encoded;
    without_summary["endpoints"][0]
        .as_object_mut()
        .unwrap()
        .remove("summary");
    assert!(serde_json::from_value::<rrd_contract::EndpointCatalogue>(without_summary).is_err());
}

#[test]
pub(super) fn native_schema_discovery_is_bounded_strict_and_administratively_authorized() {
    let request = rrd_contract::ReadDataSchema {
        scope: "instance:local".into(),
        max_storage_keys: 4096,
    };
    request.validate().unwrap();
    for max_storage_keys in [0, u32::MAX] {
        assert!(rrd_contract::ReadDataSchema {
            max_storage_keys,
            ..request.clone()
        }
        .validate()
        .is_err());
    }
    assert!(
        serde_json::from_value::<rrd_contract::ReadDataSchema>(serde_json::json!({
            "scope": "instance:local", "max_storage_keys": 4096, "cloud_account": "implicit"
        }))
        .is_err()
    );
    let catalogue = rrd_contract::endpoint_catalogue();
    let descriptor = catalogue
        .endpoints
        .iter()
        .find(|entry| entry.operation.as_str() == "data-schema-read")
        .unwrap();
    assert_eq!(
        descriptor.action.fixed_action(),
        Some(rrd_contract::SecurityAction::DiagnosticsRead)
    );
    assert!(!descriptor.mutation);
}

#[test]
pub(super) fn durable_subscription_contract_bounds_retention_backpressure_and_stream_shape() {
    let mut request = rrd_contract::OpenSubscription {
        subscription_id: CorrelationId::new("subscription-contract").unwrap(),
        stream: rrd_contract::SubscriptionStream::Changefeed {
            scope: "instance:test-instance".into(),
        },
        after_cursor: 7,
        batch_size: 16,
        max_in_flight: 4,
        retention_cursor_window: 1_024,
        lease_ms: 10_000,
        heartbeat_interval_ms: 500,
    };
    request.validate().unwrap();
    let encoded = serde_json::to_value(&request).unwrap();
    assert_eq!(
        serde_json::from_value::<rrd_contract::OpenSubscription>(encoded).unwrap(),
        request
    );
    request.max_in_flight = 0;
    assert!(request.validate().is_err());
    request.max_in_flight = 1;
    request.retention_cursor_window = 15;
    assert!(request.validate().is_err());

    let live = rrd_contract::OpenSubscription {
        subscription_id: CorrelationId::new("subscription-live-contract").unwrap(),
        stream: rrd_contract::SubscriptionStream::LiveQuery {
            scope: "instance:test-instance".into(),
            query: "SELECT * FROM document AT VALID 1000 ORDER BY @id ASC".into(),
            parameters: std::collections::BTreeMap::new(),
            budget: rrd_contract::QueryBudget::default(),
            max_delta_rows: 128,
            result_identity_version: None,
        },
        after_cursor: 0,
        batch_size: 16,
        max_in_flight: 1,
        retention_cursor_window: 1_024,
        lease_ms: 10_000,
        heartbeat_interval_ms: 500,
    };
    live.validate().unwrap();

    let acknowledgement = rrd_contract::SubscriptionAcknowledgement {
        subscription_id: CorrelationId::new("subscription-contract").unwrap(),
        connection_generation: 3,
        delivery_sequence: 9,
        through_cursor: 42,
    };
    acknowledgement.validate().unwrap();
    let catalogue = rrd_contract::endpoint_catalogue();
    assert_eq!(catalogue.websocket_endpoints.len(), 1);
    assert_eq!(catalogue.websocket_endpoints[0].path, "/v1/ws");
    assert_eq!(
        catalogue.websocket_endpoints[0].operation.as_str(),
        "websocket-connect"
    );
    assert_eq!(
        catalogue.websocket_endpoints[0].frame_type,
        "WebSocketFrame"
    );
    assert_eq!(
        catalogue.websocket_endpoints[0].connect_action,
        rrd_contract::SecurityAction::WebSocketConnect
    );
    assert_eq!(
        catalogue
            .endpoints
            .iter()
            .find(|endpoint| endpoint.operation.as_str() == "subscription-open")
            .unwrap()
            .action
            .fixed_action(),
        Some(rrd_contract::SecurityAction::SubscriptionOpen)
    );
    assert_eq!(
        catalogue
            .endpoints
            .iter()
            .find(|endpoint| endpoint.operation.as_str() == "subscription-close")
            .unwrap()
            .action
            .fixed_action(),
        Some(rrd_contract::SecurityAction::SubscriptionClose)
    );
}

pub(super) fn sha256_for_test(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    use std::fmt::Write;
    Sha256::digest(bytes)
        .iter()
        .fold(String::with_capacity(64), |mut output, byte| {
            write!(&mut output, "{byte:02x}").unwrap();
            output
        })
}

#[test]
pub(super) fn mutation_idempotency_protocol_and_capability_order_fail_closed() {
    let fixture = contract_fixture();
    let mut request = fixture.request;
    request.context.idempotency_key = None;
    assert!(request.validate(true).is_err());
    assert!(request.validate(false).is_ok());

    request.protocol_version += 1;
    assert!(request.validate(false).is_err());

    let mut service = fixture.service;
    service.capabilities.reverse();
    assert!(service.validate().is_err());
    service
        .capabilities
        .sort_by(|left, right| left.name.cmp(&right.name));
    service.capabilities.push(service.capabilities[0].clone());
    assert!(service.validate().is_err());
}

#[test]
pub(super) fn resource_paths_are_explicit_bounded_and_non_repeating() {
    let empty = ResourcePath { segments: vec![] };
    assert!(empty.validate().is_err());

    let repeated = ResourcePath {
        segments: vec![
            ResourceId::new(ResourceKind::Project, "alpha").unwrap(),
            ResourceId::new(ResourceKind::Project, "beta").unwrap(),
        ],
    };
    assert!(repeated.validate().is_err());
}

#[test]
pub(super) fn session_and_claim_transaction_contracts_are_bounded() {
    let limits = SessionLimits {
        idle_timeout_ms: 30_000,
        absolute_timeout_ms: 300_000,
        max_open_transactions: 4,
    };
    limits.validate().unwrap();
    SessionLease {
        session_id: CorrelationId::new("session-1").unwrap(),
        token: CorrelationId::new("token-1").unwrap(),
        issued_at_unix_ms: 100,
        idle_expires_at_unix_ms: 30_100,
        absolute_expires_at_unix_ms: 300_100,
        limits: limits.clone(),
    }
    .validate()
    .unwrap();
    BeginTransaction {
        scope: CanonicalId::new("instance-a").unwrap(),
        timeout_ms: 10_000,
    }
    .validate()
    .unwrap();
    let commit = CommitTransaction {
        operation_sha256: "3c94150b4ea4f9dcb27d3b602e9f190debe656533c047b99e11367bc6a28017f".into(),
        mutations: vec![TransactionMutation::AssertClaim {
            subject: CanonicalId::new("task-1").unwrap(),
            predicate: CanonicalId::new("status").unwrap(),
            object: "verified".into(),
            valid_from: 100,
            tx_time: 101,
            producer: CanonicalId::new("agent-test").unwrap(),
            confidence: Some(0.9),
        }],
    };
    commit.validate().unwrap();
    CommitReceipt {
        transaction_id: CorrelationId::new("transaction-1").unwrap(),
        operation_sha256: "3c94150b4ea4f9dcb27d3b602e9f190debe656533c047b99e11367bc6a28017f".into(),
        first_claim_sequence: 7,
        last_claim_sequence: 7,
        mutation_count: 1,
        runtime_commit_sha256: None,
        first_runtime_cursor: None,
        last_runtime_cursor: None,
        claim_mutation_count: None,
        idempotent_replay: false,
    }
    .validate()
    .unwrap();
    assert_eq!(TransactionState::Open, TransactionState::Open);

    let mut invalid = limits;
    invalid.idle_timeout_ms = invalid.absolute_timeout_ms + 1;
    assert!(invalid.validate().is_err());
    let mut invalid_commit = commit;
    let TransactionMutation::AssertClaim { confidence, .. } = &mut invalid_commit.mutations[0]
    else {
        unreachable!()
    };
    *confidence = Some(f32::NAN);
    assert!(invalid_commit.validate().is_err());
}

#[test]
pub(super) fn vector_mutation_requires_collection_and_named_vector_identity() {
    let collectionless = serde_json::json!({
        "mutation": "put_vector",
        "reference": {"kind": "embedding", "id": "document-a-body"},
        "subject": {"kind": "document", "id": "document-a"},
        "field": "body-embedding",
        "valid_from": 1,
        "value": {"kind": "dense", "values": [1.0, 0.0]},
        "properties": {}
    });
    assert!(serde_json::from_value::<TransactionMutation>(collectionless).is_err());
}

#[test]
pub(super) fn lifecycle_health_and_preview_payloads_are_bounded_and_strict() {
    let mutation = TransactionMutation::AssertClaim {
        subject: CanonicalId::new("task-1").unwrap(),
        predicate: CanonicalId::new("status").unwrap(),
        object: "verified".into(),
        valid_from: 100,
        tx_time: 101,
        producer: CanonicalId::new("agent-test").unwrap(),
        confidence: None,
    };
    let preview = PreviewTransaction {
        mutations: vec![mutation.clone()],
        valid_at: Some(100),
        max_storage_keys: 100,
    };
    preview.validate().unwrap();
    TransactionPreview {
        transaction_id: CorrelationId::new("transaction-1").unwrap(),
        read_cursor: 7,
        operation_sha256: "3c94150b4ea4f9dcb27d3b602e9f190debe656533c047b99e11367bc6a28017f".into(),
        mutations: preview.mutations,
        prospective: DataSnapshot {
            scope: "instance:test".into(),
            valid_at: 100,
            known_at_cursor: 8,
            schema_revision: 1,
            read_manifest_sha256: "0".repeat(64),
            selected_versions: 0,
            read_evidence: read_evidence(),
            entries: Vec::new(),
        },
        idempotent_replay: false,
    }
    .validate()
    .unwrap();
    SessionTermination {
        session_id: CorrelationId::new("session-1").unwrap(),
        state: SessionEndState::Closed,
        ended_at_unix_ms: 1,
        affected_open_transactions: 2,
        idempotent_replay: false,
    }
    .validate()
    .unwrap();
    Liveness {
        observed_at_unix_ms: 1,
    }
    .validate()
    .unwrap();
    Readiness {
        observed_at_unix_ms: 1,
        claim_sequence: 2,
        runtime_cursor: 3,
        backend: CanonicalId::new("rrd-lsm").unwrap(),
    }
    .validate()
    .unwrap();

    assert!(serde_json::from_str::<RenewSession>(r#"{"unknown":true}"#).is_err());
    assert!(serde_json::from_str::<CloseSession>(r#"{"unknown":true}"#).is_err());
    assert!(
        serde_json::from_str::<PreviewTransaction>(r#"{"mutations":[],"unknown":true}"#).is_err()
    );
}

#[test]
pub(super) fn transaction_digest_has_a_cross_language_golden_vector() {
    let first: Vec<TransactionMutation> = serde_json::from_str(
        r#"[{"mutation":"assert_claim","subject":"task-1","predicate":"status","object":"verified","valid_from":100,"tx_time":101,"producer":"agent-test"}]"#,
    )
    .unwrap();
    let reordered: Vec<TransactionMutation> = serde_json::from_str(
        r#"[{"producer":"agent-test","tx_time":101,"valid_from":100,"object":"verified","predicate":"status","subject":"task-1","mutation":"assert_claim"}]"#,
    )
    .unwrap();
    let expected = "d85547ca333304b54db4bc0249b6e11f83cf975fdc7cfebcf551ec6564bb71f8";
    assert_eq!(transaction_operation_sha256(&first), expected);
    assert_eq!(transaction_operation_sha256(&reordered), expected);
}
