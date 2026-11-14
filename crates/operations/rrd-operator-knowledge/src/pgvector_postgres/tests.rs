use super::*;

#[test]
fn deployment_rejects_ambiguous_columns_and_old_extensions() {
    let deployment = PgvectorDeployment {
        contract_version: OPERATOR_KNOWLEDGE_CONTRACT_VERSION,
        relation: PgvectorRelation {
            schema: "public".into(),
            relation: "knowledge".into(),
            id_column: "id".into(),
            subject_column: "subject".into(),
            vector_column: "embedding".into(),
            tenant_column: "tenant".into(),
        },
        model_digest_column: "embedding".into(),
        source_cursor_column: "source_cursor".into(),
        control_schema: "rrflow".into(),
        metadata_table: "metadata".into(),
        revision_table: "revision".into(),
        applied_work_table: "applied".into(),
    };
    assert!(deployment.validate().is_err());
    assert!(require_pgvector_08("0.7.4").is_err());
    assert!(require_pgvector_08("0.8.0").is_ok());
    assert!(require_pgvector_08("1.0.0").is_ok());
}

#[test]
fn explain_path_detection_ignores_non_vector_indexes() {
    let explain =
        serde_json::json!([{"Plan": {"Node Type": "Index Scan", "Index Name": "tenant_idx"}}]);
    let catalog = CatalogEvidence {
        extension_version: "0.8.1".into(),
        relation_oid: 42,
        columns: Vec::new(),
        indexes: vec![
            IndexEvidence {
                name: "tenant_idx".into(),
                access_method: "btree".into(),
                definition: "CREATE INDEX tenant_idx".into(),
            },
            IndexEvidence {
                name: "knowledge_hnsw".into(),
                access_method: "hnsw".into(),
                definition: "CREATE INDEX knowledge_hnsw".into(),
            },
        ],
    };
    assert_eq!(
        selected_path(&explain, &catalog).0,
        OperatorAccessPath::Exact
    );
}

#[test]
fn tls_connection_contract_rejects_downgrade_modes_before_connecting() {
    assert!(require_tls_config("host=localhost sslmode=disable").is_err());
    assert!(require_tls_config("host=localhost sslmode=prefer").is_err());
    assert_eq!(
        require_tls_config("host=localhost sslmode=require")
            .unwrap()
            .get_ssl_mode(),
        SslMode::Require
    );
}
