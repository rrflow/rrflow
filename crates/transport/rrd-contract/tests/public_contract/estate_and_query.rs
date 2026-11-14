//! Estate reads, backup jobs, recovery posture, query, live query and index contracts.

use super::*;

#[test]
pub(super) fn estate_read_contract_has_frozen_field_names_and_strict_empty_payload() {
    let snapshot = EstateSnapshot {
        format_version: 1,
        id: CanonicalId::new("estate-a").unwrap(),
        revision: 7,
        created_at_unix_ms: 10,
        updated_at_unix_ms: 20,
        activity_policy: EstateActivityPolicySnapshot {
            idle_after_ms: 300_000,
            stale_after_ms: 1_800_000,
            neglected_after_ms: 604_800_000,
        },
        authority: EstateAuthoritySnapshot::empty(),
        instances: Vec::new(),
        operations: Vec::new(),
        idempotency_binding_count: 0,
    };
    let expected = serde_json::json!({
        "format_version": 1,
        "id": "estate-a",
        "revision": 7,
        "created_at_unix_ms": 10,
        "updated_at_unix_ms": 20,
        "activity_policy": {
            "idle_after_ms": 300000,
            "stale_after_ms": 1800000,
            "neglected_after_ms": 604800000
        },
        "authority": serde_json::to_value(EstateAuthoritySnapshot::empty()).unwrap(),
        "instances": [],
        "operations": [],
        "idempotency_binding_count": 0
    });
    assert_eq!(serde_json::to_value(&snapshot).unwrap(), expected);
    assert_eq!(
        serde_json::from_value::<EstateSnapshot>(expected).unwrap(),
        snapshot
    );
    let mutation = EstateMutationResult {
        estate: snapshot,
        idempotent_replay: true,
    };
    let mutation_json = serde_json::to_value(&mutation).unwrap();
    assert_eq!(mutation_json["estate"]["id"], "estate-a");
    assert_eq!(mutation_json["idempotent_replay"], true);
    let mut unknown = mutation_json;
    unknown
        .as_object_mut()
        .unwrap()
        .insert("unknown".into(), serde_json::json!(true));
    assert!(serde_json::from_value::<EstateMutationResult>(unknown).is_err());
    assert_eq!(
        serde_json::from_str::<ReadEstate>("{}").unwrap(),
        ReadEstate {}
    );
    assert!(serde_json::from_str::<ReadEstate>(r#"{"unknown":true}"#).is_err());
}

#[test]
pub(super) fn estate_backup_jobs_are_a_separate_strict_public_resource() {
    let snapshot = EstateBackupJobsSnapshot {
        estate_id: CanonicalId::new("estate-a").unwrap(),
        estate_revision: 8,
        jobs: vec![EstateBackupJobSnapshot {
            id: CanonicalId::new("backup-daily").unwrap(),
            instance_id: CanonicalId::new("instance-a").unwrap(),
            source_generation: 1,
            label: "daily.0001".into(),
            request_sha256: "a".repeat(64),
            state: EstateBackupJobState::Pending,
            attempts: 0,
            created_at_unix_ms: 80,
            updated_at_unix_ms: 80,
            recovery_policy: EstateBackupRecoveryPolicySnapshot {
                revision: 1,
                max_rpo_ms: 86_400_000,
                max_rto_ms: 3_600_000,
                minimum_recovery_points: 1,
                retention_ms: 604_800_000,
            },
            lease: None,
            receipts: Vec::new(),
            backup_id: None,
            archive_sha256: None,
            catalogue_sha256: None,
            error: None,
        }],
    };
    let expected = serde_json::json!({
        "estate_id": "estate-a",
        "estate_revision": 8,
        "jobs": [{
            "id": "backup-daily",
            "instance_id": "instance-a",
            "source_generation": 1,
            "label": "daily.0001",
            "request_sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "state": "pending",
            "attempts": 0,
            "created_at_unix_ms": 80,
            "updated_at_unix_ms": 80,
            "recovery_policy": {
                "revision": 1,
                "max_rpo_ms": 86400000,
                "max_rto_ms": 3600000,
                "minimum_recovery_points": 1,
                "retention_ms": 604800000
            }
        }]
    });
    assert_eq!(serde_json::to_value(&snapshot).unwrap(), expected);
    assert_eq!(
        serde_json::from_value::<EstateBackupJobsSnapshot>(expected.clone()).unwrap(),
        snapshot
    );
    let mut missing_policy = expected.clone();
    missing_policy["jobs"][0]
        .as_object_mut()
        .unwrap()
        .remove("recovery_policy");
    assert!(serde_json::from_value::<EstateBackupJobsSnapshot>(missing_policy).is_err());
    let mut unknown = expected;
    unknown["jobs"][0]["unknown"] = serde_json::json!(true);
    assert!(serde_json::from_value::<EstateBackupJobsSnapshot>(unknown).is_err());
}

#[test]
pub(super) fn estate_recovery_posture_is_strict_bounded_by_identity_and_path_free() {
    let recovery = serde_json::json!({
        "estate_id": "estate-a",
        "estate_revision": 12,
        "policies": [{
            "instance_id": "instance-a",
            "revision": 1,
            "max_rpo_ms": 86400000,
            "max_rto_ms": 3600000,
            "minimum_recovery_points": 2,
            "retention_ms": 604800000,
            "updated_at_unix_ms": 80
        }],
        "recovery_points": [],
        "retention_pins": [],
        "restore_evidence": [],
        "prune_intents": []
    });
    let decoded: rrd_contract::EstateRecoverySnapshot =
        serde_json::from_value(recovery.clone()).unwrap();
    assert_eq!(decoded.policies[0].revision, 1);
    assert!(!serde_json::to_string(&decoded).unwrap().contains("path"));
    let mut unknown = recovery;
    unknown["policies"][0]["target_path"] = serde_json::json!("/tmp/escape");
    assert!(serde_json::from_value::<rrd_contract::EstateRecoverySnapshot>(unknown).is_err());
}

#[test]
pub(super) fn query_contract_is_transport_neutral_bounded_and_strict() {
    let request = ExecuteQuery {
        scope: "instance:project-alpha".into(),
        query: "FROM record:document AT VALID 42 KNOWN HEAD PROJECT title".into(),
        parameters: BTreeMap::from([("title".into(), QueryValue::String("alpha".into()))]),
        budget: QueryBudget::default(),
    };
    request.validate().unwrap();
    let encoded = serde_json::to_value(&request).unwrap();
    assert_eq!(encoded["scope"], "instance:project-alpha");
    let mut unknown = encoded.clone();
    unknown["unknown"] = serde_json::json!(true);
    assert!(serde_json::from_value::<ExecuteQuery>(unknown).is_err());

    let result = QueryResult {
        canonical_query: request.query.clone(),
        scope: request.scope.clone(),
        read_manifest_sha256: "a".repeat(64),
        known_at_cursor: 7,
        result_identity_version: rrd_core::result_identity::RESULT_IDENTITY_VERSION,
        schema_revision: 1,
        plan: QueryPlanSnapshot {
            plan_sha256: "b".repeat(64),
            security_policy_revision: 4,
            authorization_sha256: "c".repeat(64),
            exact: true,
            deterministic_order: "identity_ascending".into(),
            authorization_boundary: "scope:instance:project-alpha".into(),
            candidates: vec![QueryPlanCandidate {
                name: "versioned_source_read".into(),
                selected: true,
                exact: true,
                reason: "direct semantic-version fixture".into(),
            }],
        },
        execution: QueryExecutionSnapshot {
            selected_versions: 7,
            read_evidence: read_evidence(),
            returned_rows: 1,
            output_bytes: 32,
            truncated: false,
            analysis: Some(QueryExecutionAnalysisSnapshot {
                engine: "datafusion-55".into(),
                provider_scans: 1,
                input_rows: 2,
                input_batches: 1,
                input_memory_bytes: 512,
                output_batches: 1,
                projection_pushdown: "exact".into(),
                filter_pushdown: "unsupported_exact_post_scan".into(),
                limit_pushdown: "retained_above_scan".into(),
                physical_operators: 5,
                peak_memory_bytes: 1_024,
                spill_count: 0,
                spilled_bytes: 0,
                spilled_rows: 0,
                elapsed_micros: 50,
            }),
        },
        rows: vec![QueryRowSnapshot {
            identity: "document:alpha".into(),
            values: BTreeMap::from([("title".into(), QueryValue::String("Alpha".into()))]),
        }],
    };
    assert_eq!(
        serde_json::from_value::<QueryResult>(serde_json::to_value(&result).unwrap()).unwrap(),
        result
    );

    let mut invalid_budget = request_budget_fixture();
    invalid_budget.max_rows = 0;
    assert!(invalid_budget.validate().is_err());
    let minimal_budget: QueryBudget = serde_json::from_value(serde_json::json!({
        "max_storage_keys": 100,
        "max_rows": 10,
        "max_output_bytes": 4096,
        "max_batch_rows": 10
    }))
    .unwrap();
    assert_eq!(minimal_budget.max_memory_bytes, 64 * 1024 * 1024);
    assert_eq!(minimal_budget.max_spill_bytes, 256 * 1024 * 1024);
    assert_eq!(minimal_budget.max_elapsed_ms, 30_000);
    minimal_budget.validate().unwrap();
    let mut invalid_memory = minimal_budget;
    invalid_memory.max_memory_bytes = rrd_contract::MAX_QUERY_MEMORY_BYTES + 1;
    assert!(invalid_memory.validate().is_err());
    let mut invalid_parameter = request.clone();
    invalid_parameter
        .parameters
        .insert("nested".into(), QueryValue::List(Vec::new()));
    assert!(invalid_parameter.validate().is_err());
}

#[test]
pub(super) fn query_transaction_contract_reuses_typed_mutations_and_is_bounded() {
    let request = ExecuteQueryTransaction {
        scope: "instance:project-alpha".into(),
        program: "BEGIN; MUTATE $document; COMMIT;".into(),
        mutation_bindings: BTreeMap::from([(
            "document".into(),
            TransactionMutation::PutRecord {
                reference: DataReference {
                    kind: CanonicalId::new("document").unwrap(),
                    id: CanonicalId::new("alpha").unwrap(),
                },
                valid_from: 100,
                valid_to: None,
                properties: BTreeMap::from([("title".into(), QueryValue::String("Alpha".into()))]),
            },
        )]),
        timeout_ms: 1_000,
    };
    request.validate().unwrap();
    let mut unknown = serde_json::to_value(&request).unwrap();
    unknown["autocommit"] = serde_json::json!(true);
    assert!(serde_json::from_value::<ExecuteQueryTransaction>(unknown).is_err());

    let mut invalid = request.clone();
    invalid.timeout_ms = 0;
    assert!(invalid.validate().is_err());
    let mut invalid = request;
    invalid.mutation_bindings.clear();
    assert!(invalid.validate().is_err());
}

pub(super) fn request_budget_fixture() -> QueryBudget {
    QueryBudget::default()
}

pub(super) fn read_evidence() -> ReadEvidence {
    ReadEvidence {
        contract_version: 1,
        key_budget: 64,
        point_reads: 1,
        range_scans: 1,
        keys_examined: 2,
        values_decoded: 1,
        decoded_bytes: 64,
        stamp_validation: ReadStampValidationEvidence {
            method: ReadStampValidationMethod::AuthenticatedCurrentHead,
            change_reads: 0,
            proof_nodes: 0,
        },
        paths: vec![
            ReadPathEvidence {
                path: ReadAccessPath::ReadStamp,
                point_reads: 1,
                range_scans: 0,
                keys_examined: 1,
                values_decoded: 0,
                decoded_bytes: 0,
            },
            ReadPathEvidence {
                path: ReadAccessPath::RecordVersions,
                point_reads: 0,
                range_scans: 1,
                keys_examined: 1,
                values_decoded: 1,
                decoded_bytes: 64,
            },
        ],
    }
}

#[test]
pub(super) fn live_query_contract_is_resumable_bounded_and_strict() {
    let request = PollLiveQuery {
        scope: "instance:project-alpha".into(),
        query: "FROM record:document AT VALID 42 KNOWN HEAD PROJECT title".into(),
        parameters: BTreeMap::new(),
        after_cursor: 41,
        result_identity_version: Some(rrd_core::result_identity::RESULT_IDENTITY_VERSION),
        budget: QueryBudget::default(),
        max_delta_rows: 256,
        wait_timeout_ms: 1_000,
    };
    request.validate().unwrap();
    let mut encoded = serde_json::to_value(&request).unwrap();
    encoded["stream"] = serde_json::json!(true);
    assert!(serde_json::from_value::<PollLiveQuery>(encoded).is_err());
    let mut invalid_wait = request.clone();
    invalid_wait.wait_timeout_ms = 5_001;
    assert!(invalid_wait.validate().is_err());
    let mut invalid = request;
    invalid.max_delta_rows = 0;
    assert!(invalid.validate().is_err());
}

#[test]
pub(super) fn query_index_administration_contract_is_bounded_and_strict() {
    EnsureQueryIndex {
        scope: "instance:project-alpha".into(),
        index_id: CanonicalId::new("documents-by-status").unwrap(),
        definition_query: "FROM record:document AT VALID 42 KNOWN HEAD PROJECT status, title"
            .into(),
        unique: false,
        kind: QueryIndexKind::Scalar,
        full_text: None,
        budget: QueryBudget::default(),
    }
    .validate()
    .unwrap();
    assert!(
        serde_json::from_value::<EnsureVectorIndex>(serde_json::json!({
            "scope": "instance:project-alpha",
            "collection_id": "documents",
            "vector_name": "title",
            "configuration": {
                "kind": "turbo_quant",
                "bits": "bits2",
                "seed": 11,
                "filter_properties": ["tenant"]
            },
            "build_policy": {"kind": "cpu"},
            "max_storage_keys": 10000
        }))
        .is_err()
    );
    ListQueryIndexes {
        scope: "instance:project-alpha".into(),
    }
    .validate()
    .unwrap();
    assert!(ListQueryIndexes {
        scope: String::new()
    }
    .validate()
    .is_err());
}

#[test]
pub(super) fn vector_quantization_lifecycle_contract_is_bounded_and_strict() {
    let build = BuildVectorQuantizationArtifact {
        scope: "instance:project-alpha".into(),
        collection_id: CanonicalId::new("documents").unwrap(),
        vector_name: CanonicalId::new("title").unwrap(),
        method: VectorQuantizationMethod::Product {
            compression: VectorProductCompression::X64,
        },
        filter_properties: vec![CanonicalId::new("tenant").unwrap()],
        max_storage_keys: 10_000,
    };
    build.validate().unwrap();
    assert_eq!(build.method.maximum_compression_ratio(), 64);

    let mut duplicate_filter = build.clone();
    duplicate_filter
        .filter_properties
        .push(CanonicalId::new("tenant").unwrap());
    assert!(duplicate_filter.validate().is_err());
    let mut unbounded = build.clone();
    unbounded.max_storage_keys = 0;
    assert!(unbounded.validate().is_err());
    let mut unknown = serde_json::to_value(&build).unwrap();
    unknown["always_ram"] = serde_json::json!(true);
    assert!(serde_json::from_value::<BuildVectorQuantizationArtifact>(unknown).is_err());

    ListVectorQuantizationArtifacts {
        scope: build.scope.clone(),
        collection_id: Some(build.collection_id.clone()),
        vector_name: None,
        max_artifacts: 100,
    }
    .validate()
    .unwrap();
    assert!(ListVectorQuantizationArtifacts {
        scope: build.scope.clone(),
        collection_id: None,
        vector_name: None,
        max_artifacts: 0,
    }
    .validate()
    .is_err());

    let artifact_id = CanonicalId::new("quant-product-documents-title").unwrap();
    ActivateVectorQuantizationArtifact {
        scope: build.scope.clone(),
        artifact_id: artifact_id.clone(),
        generation: 1,
    }
    .validate()
    .unwrap();
    assert!(RetireVectorQuantizationArtifact {
        scope: build.scope,
        artifact_id,
        generation: 0,
    }
    .validate()
    .is_err());
}
