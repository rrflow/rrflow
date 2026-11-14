use super::*;

#[test]
fn pgvector_sql_is_parameterized_quoted_and_path_explicit() {
    let relation = PgvectorRelation {
        schema: "operator data".into(),
        relation: "knowledge\"items".into(),
        id_column: "id".into(),
        subject_column: "subject".into(),
        vector_column: "embedding".into(),
        tenant_column: "project".into(),
    };
    let controls = OperatorSearchControls {
        requested_path: OperatorAccessPath::Hnsw,
        iterative_scan: IterativeScanMode::StrictOrder,
        hnsw_ef_search: Some(80),
        hnsw_max_scan_tuples: Some(20_000),
        ivfflat_probes: None,
        ivfflat_max_probes: None,
    };
    let plan = relation
        .build_search(ScoreMetric::Cosine, &controls)
        .unwrap();
    assert!(plan
        .search_sql
        .contains("\"operator data\".\"knowledge\"\"items\""));
    assert!(plan.search_sql.contains("$1::text::vector"));
    assert!(plan.search_sql.contains("$2"));
    assert!(plan.search_sql.contains("LIMIT $3"));
    assert!(!plan.search_sql.contains("strict_order"));
    assert_eq!(
        plan.settings,
        [
            ("enable_seqscan".into(), "off".into()),
            ("hnsw.ef_search".into(), "80".into()),
            ("hnsw.max_scan_tuples".into(), "20000".into()),
            ("hnsw.iterative_scan".into(), "strict_order".into()),
        ]
    );
}

#[test]
fn path_controls_fail_closed_instead_of_cross_applying() {
    let mut controls = OperatorSearchControls::exact();
    controls.hnsw_ef_search = Some(40);
    assert!(controls.validate().is_err());
    let controls = OperatorSearchControls {
        requested_path: OperatorAccessPath::IvfFlat,
        iterative_scan: IterativeScanMode::RelaxedOrder,
        hnsw_ef_search: None,
        hnsw_max_scan_tuples: None,
        ivfflat_probes: Some(100),
        ivfflat_max_probes: Some(10),
    };
    assert!(controls.validate().is_err());

    let relation = PgvectorRelation {
        schema: "public".into(),
        relation: "knowledge".into(),
        id_column: "id".into(),
        subject_column: "subject".into(),
        vector_column: "embedding".into(),
        tenant_column: "project".into(),
    };
    let mut controls = controls;
    controls.ivfflat_max_probes = Some(100);
    controls.iterative_scan = IterativeScanMode::StrictOrder;
    assert!(controls.validate().is_err());
    controls.iterative_scan = IterativeScanMode::RelaxedOrder;
    let error = relation
        .build_search(ScoreMetric::Manhattan, &controls)
        .unwrap_err();
    assert!(error.to_string().contains("does not support Manhattan"));
}
