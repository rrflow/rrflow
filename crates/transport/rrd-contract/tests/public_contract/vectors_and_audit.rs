//! Vector collections, changefeeds, managed backups, audit and diagnostic reads.

use super::*;

#[test]
pub(super) fn vector_collection_contract_is_named_bounded_and_search_addressable() {
    let vector = NamedVectorDefinition {
        name: CanonicalId::new("title").unwrap(),
        field: CanonicalId::new("title_embedding").unwrap(),
        kind: VectorValueKind::Dense,
        dimensions: 2,
        metric: VectorSearchMetric::Cosine,
        embedding_model: None,
        memory_tier: VectorMemoryTier::Cached,
    };
    EnsureVectorCollection {
        scope: "instance:project-alpha".into(),
        collection_id: CanonicalId::new("documents").unwrap(),
        vectors: vec![vector.clone()],
    }
    .validate()
    .unwrap();
    ListVectorCollections {
        scope: "instance:project-alpha".into(),
    }
    .validate()
    .unwrap();
    EnsureVectorPayloadIndex {
        scope: "instance:project-alpha".into(),
        collection_id: CanonicalId::new("documents").unwrap(),
        field: CanonicalId::new("tenant").unwrap(),
        kind: VectorPayloadIndexKind::Keyword,
    }
    .validate()
    .unwrap();
    ListVectorPayloadIndexes {
        scope: "instance:project-alpha".into(),
        collection_id: CanonicalId::new("documents").unwrap(),
    }
    .validate()
    .unwrap();
    DeleteVectorPayloadIndex {
        scope: "instance:project-alpha".into(),
        collection_id: CanonicalId::new("documents").unwrap(),
        field: CanonicalId::new("tenant").unwrap(),
    }
    .validate()
    .unwrap();
    DeleteVectorCollection {
        scope: "instance:project-alpha".into(),
        collection_id: CanonicalId::new("documents").unwrap(),
        valid_at: 42,
        max_storage_keys: 10_000,
    }
    .validate()
    .unwrap();
    assert!(DeleteVectorCollection {
        scope: "instance:project-alpha".into(),
        collection_id: CanonicalId::new("documents").unwrap(),
        valid_at: 0,
        max_storage_keys: 10_000,
    }
    .validate()
    .is_err());
    EnsureVectorIndex {
        scope: "instance:project-alpha".into(),
        collection_id: CanonicalId::new("documents").unwrap(),
        vector_name: CanonicalId::new("title").unwrap(),
        configuration: VectorIndexConfiguration::Hnsw {
            m: 16,
            ef_construction: 100,
            max_level: 8,
            seed: 7,
            filter_properties: vec![CanonicalId::new("tenant").unwrap()],
        },
        build_policy: VectorIndexBuildPolicy::Cpu,
        max_storage_keys: 10_000,
    }
    .validate()
    .unwrap();
    SearchVectors {
        scope: "instance:project-alpha".into(),
        valid_at: 42,
        collection_id: Some(CanonicalId::new("documents").unwrap()),
        vector_name: Some(vector.name),
        field: None,
        query: VectorSearchQuery::Dense {
            values: vec![0.6, 0.8],
        },
        filter: Some(VectorPayloadFilter::Condition {
            condition: VectorPayloadCondition {
                property: CanonicalId::new("tenant").unwrap(),
                operator: VectorPayloadOperator::Equals {
                    value: QueryValue::String("alpha".into()),
                },
            },
        }),
        metric: None,
        top_k: 10,
        mode: VectorSearchMode::Exact,
        max_storage_keys: 100,
    }
    .validate()
    .unwrap();
    let approximate = SearchVectors {
        scope: "instance:project-alpha".into(),
        valid_at: 42,
        collection_id: Some(CanonicalId::new("documents").unwrap()),
        vector_name: Some(CanonicalId::new("title").unwrap()),
        field: None,
        query: VectorSearchQuery::Dense {
            values: vec![0.6, 0.8],
        },
        filter: None,
        metric: None,
        top_k: 10,
        mode: VectorSearchMode::RequireApproximate {
            exact_rerank: 20,
            ef_search: 100,
        },
        max_storage_keys: 100,
    };
    approximate.validate().unwrap();
    SearchHybrid {
        scope: "instance:project-alpha".into(),
        valid_at: 42,
        document_kind: CanonicalId::new("document").unwrap(),
        text_field: CanonicalId::new("title").unwrap(),
        text_query: "reason ready flow".into(),
        collection_id: CanonicalId::new("documents").unwrap(),
        vector_name: CanonicalId::new("title").unwrap(),
        vector_query: VectorSearchQuery::Dense {
            values: vec![0.6, 0.8],
        },
        vector_filter: None,
        vector_mode: VectorSearchMode::RequireApproximate {
            exact_rerank: 20,
            ef_search: 100,
        },
        fusion: HybridFusion::ReciprocalRank {
            rank_constant: 60,
            text_weight_millionths: 1_000_000,
            vector_weight_millionths: 1_000_000,
        },
        top_k: 10,
        candidate_k: 20,
        max_storage_keys: 100,
    }
    .validate()
    .unwrap();
    let mut invalid_approximate = approximate;
    invalid_approximate.mode = VectorSearchMode::AllowApproximate {
        exact_rerank: 9,
        ef_search: 100,
    };
    assert!(invalid_approximate.validate().is_err());
    assert!(SearchVectors {
        scope: "instance:project-alpha".into(),
        valid_at: 42,
        collection_id: Some(CanonicalId::new("documents").unwrap()),
        vector_name: None,
        field: None,
        query: VectorSearchQuery::Dense {
            values: vec![0.6, 0.8],
        },
        filter: None,
        metric: None,
        top_k: 10,
        mode: VectorSearchMode::Exact,
        max_storage_keys: 100,
    }
    .validate()
    .is_err());
    let point: TransactionMutation = serde_json::from_value(serde_json::json!({
        "mutation": "put_vector",
        "reference": {"kind": "embedding", "id": "alpha-title"},
        "subject": {"kind": "document", "id": "alpha"},
        "collection_id": "documents",
        "vector_name": "title",
        "field": "title_embedding",
        "valid_from": 42,
        "value": {"kind": "dense", "values": [0.6, 0.8]},
        "properties": {"tenant": {"type": "string", "value": "alpha"}}
    }))
    .unwrap();
    point.validate().unwrap();
    let mut incomplete = serde_json::to_value(point).unwrap();
    incomplete.as_object_mut().unwrap().remove("vector_name");
    assert!(serde_json::from_value::<TransactionMutation>(incomplete).is_err());
}

#[test]
pub(super) fn changefeed_request_is_cursor_addressed_bounded_and_strict() {
    let request = ReadChangefeed {
        scope: "instance:project-alpha".into(),
        after_cursor: 41,
        limit: 256,
    };
    request.validate().unwrap();
    let mut encoded = serde_json::to_value(&request).unwrap();
    encoded["unknown"] = serde_json::json!(true);
    assert!(serde_json::from_value::<ReadChangefeed>(encoded).is_err());

    let mut invalid = request.clone();
    invalid.limit = 0;
    assert!(invalid.validate().is_err());
    invalid.limit = rrd_contract::MAX_CHANGEFEED_PAGE + 1;
    assert!(invalid.validate().is_err());

    let mut follow = FollowChangefeed {
        read: request,
        wait_timeout_ms: 1_000,
    };
    follow.validate().unwrap();
    follow.wait_timeout_ms = rrd_contract::MAX_CHANGEFEED_WAIT_MS + 1;
    assert!(follow.validate().is_err());
}

#[test]
pub(super) fn managed_backup_contract_accepts_no_filesystem_paths() {
    CreateInstanceBackup {
        label: "before-upgrade".into(),
        created_at_unix_ms: 500,
    }
    .validate()
    .unwrap();

    RestoreInstanceBackup {
        backup_sha256: "a".repeat(64),
        restore_id: CanonicalId::new("restore-a").unwrap(),
        restored_at_unix_ms: 600,
    }
    .validate()
    .unwrap();

    let with_path = serde_json::json!({
        "label": "before-upgrade",
        "created_at_unix_ms": 500,
        "target": "/tmp/escape"
    });
    assert!(serde_json::from_value::<CreateInstanceBackup>(with_path).is_err());

    let invalid = RestoreInstanceBackup {
        backup_sha256: "not-a-digest".into(),
        restore_id: CanonicalId::new("restore-a").unwrap(),
        restored_at_unix_ms: 600,
    };
    assert!(invalid.validate().is_err());
}

#[test]
pub(super) fn audit_read_contract_is_bounded_and_strict() {
    ReadAudit {
        after_sequence: 41,
        limit: 128,
    }
    .validate()
    .unwrap();
    assert!(ReadAudit {
        after_sequence: 0,
        limit: 0,
    }
    .validate()
    .is_err());
    assert!(serde_json::from_value::<ReadAudit>(serde_json::json!({
        "after_sequence": 0,
        "limit": 1,
        "include_bodies": true
    }))
    .is_err());
    rrd_contract::ExportAudit {
        after_sequence: 41,
        limit: 128,
    }
    .validate()
    .unwrap();
    assert!(
        serde_json::from_value::<rrd_contract::ExportAudit>(serde_json::json!({
            "after_sequence": 0,
            "limit": 1,
            "target_path": "/tmp/audit"
        }))
        .is_err()
    );
}

#[test]
pub(super) fn audit_export_rejects_content_record_and_chain_substitution() {
    let mut record = rrd_contract::AuditRecordSnapshot {
        sequence: 7,
        audit_id: CanonicalId::new("audit-contract-record").unwrap(),
        at_unix_ms: 100,
        principal_id: Some(CanonicalId::new("contract-auditor").unwrap()),
        action: rrd_contract::SecurityAction::AuditRead,
        resource: ResourcePath {
            segments: vec![ResourceId::new(ResourceKind::Instance, "contract-instance").unwrap()],
        },
        request_id: "request-audit-contract".into(),
        operation_id: "operation-audit-contract".into(),
        phase: rrd_contract::AuditPhase::Completed,
        decision: rrd_contract::AuditDecision::Allowed,
        status_code: 200,
        request_sha256: "a".repeat(64),
        response_sha256: "b".repeat(64),
        previous_audit_sha256: Some("c".repeat(64)),
        audit_sha256: String::new(),
    };
    record.audit_sha256 = sha256_for_test(
        &serde_json::to_vec(&(
            &record.audit_id,
            record.at_unix_ms,
            &record.principal_id,
            record.action,
            &record.resource,
            &record.request_id,
            &record.operation_id,
            record.phase,
            record.decision,
            record.status_code,
            &record.request_sha256,
            &record.response_sha256,
            &record.previous_audit_sha256,
        ))
        .unwrap(),
    );
    let json_lines = format!("{}\n", serde_json::to_string(&record).unwrap());
    let export = rrd_contract::AuditExport {
        requested_after_sequence: 0,
        through_sequence: 8,
        chain_anchor_sha256: record.previous_audit_sha256.clone(),
        chain_head_sha256: Some(record.audit_sha256.clone()),
        record_count: 1,
        media_type: "application/x-ndjson; profile=rrd-audit-v1".into(),
        content_sha256: sha256_for_test(json_lines.as_bytes()),
        json_lines,
    };
    export.validate().unwrap();

    let mut content_tampered = export.clone();
    content_tampered.json_lines.push(' ');
    assert!(content_tampered.validate().is_err());

    let mut record_tampered = record;
    record_tampered.response_sha256 = "d".repeat(64);
    let substituted = format!("{}\n", serde_json::to_string(&record_tampered).unwrap());
    let mut sealed_substitution = export;
    sealed_substitution.content_sha256 = sha256_for_test(substituted.as_bytes());
    sealed_substitution.json_lines = substituted;
    assert!(sealed_substitution.validate().is_err());
}

#[test]
pub(super) fn diagnostic_read_contract_is_bounded_strict_and_separately_authorized() {
    let bounded = |max| rrd_contract::ReadDiagnosticSnapshot {
        scope: "instance:alpha".into(),
        graph_valid_at_unix_ms: 1_000,
        graph_known_at_cursor: None,
        graph_compare_cursor: 0,
        graph_max_records: Some(max),
        runtime_max_scanned_changes: 1_024,
        changes_after_cursor: 0,
        change_limit: 1,
        audit_after_sequence: 0,
        audit_limit: 1,
    };
    bounded(100).validate().unwrap();
    assert!(bounded(0).validate().is_err());
    assert!(bounded(rrd_contract::MAX_DIAGNOSTIC_GRAPH_RECORDS + 1)
        .validate()
        .is_err());
    rrd_contract::ReadDiagnosticSnapshot {
        scope: "instance:alpha".into(),
        graph_valid_at_unix_ms: 1_000,
        graph_known_at_cursor: Some(30),
        graph_compare_cursor: 10,
        graph_max_records: None,
        runtime_max_scanned_changes: 1_024,
        changes_after_cursor: 10,
        change_limit: 128,
        audit_after_sequence: 20,
        audit_limit: 128,
    }
    .validate()
    .unwrap();
    assert!(rrd_contract::ReadDiagnosticSnapshot {
        scope: "instance:alpha".into(),
        graph_valid_at_unix_ms: 1_000,
        graph_known_at_cursor: Some(30),
        graph_compare_cursor: 10,
        graph_max_records: None,
        runtime_max_scanned_changes: 1_024,
        changes_after_cursor: 0,
        change_limit: 0,
        audit_after_sequence: 0,
        audit_limit: 1,
    }
    .validate()
    .is_err());
    assert!(
        serde_json::from_value::<rrd_contract::ReadDiagnosticSnapshot>(serde_json::json!({
            "scope": "instance:alpha",
            "graph_valid_at_unix_ms": 1000,
            "graph_known_at_cursor": 30,
            "graph_compare_cursor": 10,
            "runtime_max_scanned_changes": 1024,
            "changes_after_cursor": 0,
            "change_limit": 1,
            "audit_after_sequence": 0,
            "audit_limit": 1,
            "physical_store": "/tmp/escape"
        }))
        .is_err()
    );

    let descriptor = rrd_contract::endpoint_catalogue()
        .endpoints
        .into_iter()
        .find(|endpoint| endpoint.operation.as_str() == "diagnostics-read")
        .unwrap();
    assert_eq!(descriptor.path, "/v1/diagnostics/read");
    assert_eq!(
        descriptor.action.fixed_action(),
        Some(rrd_contract::SecurityAction::DiagnosticsRead)
    );
    assert!(!descriptor.mutation);
}

pub(super) fn rrflow_kv_storage_counters() -> rrd_contract::DiagnosticRrflowKvSnapshot {
    rrd_contract::DiagnosticRrflowKvSnapshot {
        rrflow_cld: None,
        physical_sequence: 12,
        manifest_generation: 2,
        durable_sequence: 10,
        memtable: rrd_contract::DiagnosticMemtableSnapshot {
            versions: 4,
            max_versions: 64,
            approximate_bytes: 512,
            keys: 4,
            key_payload_bytes: 96,
            value_payload_bytes: 256,
            tombstones: 0,
            spilled_chains: 0,
            spilled_version_capacity: 0,
            version_record_bytes: 128,
            owned_bytes_lower_bound: 480,
        },
        write_ahead_log: rrd_contract::DiagnosticWriteAheadLogSnapshot {
            payload_bytes: 2_048,
            payload_max_bytes: 1 << 20,
        },
        maintenance: rrd_contract::DiagnosticMaintenanceSnapshot {
            automatic_flushes: 1,
            failed_flushes: 0,
            write_stalls: 0,
            oversized_batches: 0,
            automatic_compactions: 0,
            failed_compactions: 0,
            compaction_input_bytes: 0,
            compaction_output_bytes: 0,
            peak_compaction_buffer_bytes: 0,
        },
        segment_set: rrd_contract::DiagnosticSegmentSetSnapshot {
            segments: 1,
            segment_bytes: 4_096,
            l0_segments: 1,
            l0_compaction_trigger: 4,
            compaction_debt_segments: 0,
            compaction_target_segment_bytes: 1 << 26,
        },
        page_cache: rrd_contract::DiagnosticPageCacheSnapshot {
            capacity_bytes: 1 << 24,
            resident_bytes: 4_096,
            entries: 1,
            hits: 9,
            misses: 1,
            evictions: 0,
            page_loads: 1,
            bytes_read: 4_096,
            bytes_decoded: 4_096,
            bytes_borrowed: 0,
            bytes_allocated: 4_096,
            bytes_copied: 0,
            bytes_decompressed: 0,
            filter_checks: 3,
            filter_negatives: 2,
        },
        segment_io: rrd_contract::DiagnosticSegmentIoSnapshot {
            max_open_files: 128,
            open_files: 0,
            peak_open_files: 0,
            handle_evictions: 0,
            handle_reopens: 0,
            handle_denials: 0,

            requested_mode: rrd_contract::DiagnosticSegmentIoMode::Auto,
            mmap_segments: 0,
            io_uring_segments: 0,
            bounded_segments: 1,
            fallbacks: 0,
            last_fallback_reason: None,
            read_operations: 1,
            mmap_reads: 0,
            io_uring_reads: 0,
            bounded_reads: 1,
            bytes_read: 4_096,
            max_request_bytes: 1 << 20,
            peak_request_bytes: 4_096,
        },
    }
}
