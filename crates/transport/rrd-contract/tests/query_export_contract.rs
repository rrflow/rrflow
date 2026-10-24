use rrd_contract::{ExecuteQuery, ExportQuery, QueryBudget, MAX_PARQUET_EXPORT_BYTES};
use std::collections::BTreeMap;

#[test]
fn export_request_is_closed_and_bounded_by_both_profile_and_query_budget() {
    let mut request = ExportQuery {
        query: ExecuteQuery {
            scope: "instance:test".into(),
            query: "FROM claim:status AT VALID 1 KNOWN HEAD".into(),
            parameters: BTreeMap::new(),
            budget: QueryBudget::default(),
        },
        max_output_bytes: MAX_PARQUET_EXPORT_BYTES as u64,
    };
    request.validate().unwrap();
    request.max_output_bytes += 1;
    assert!(request.validate().is_err());
    request.max_output_bytes = 0;
    assert!(request.validate().is_err());
    request.max_output_bytes = 100;
    request.query.budget.max_output_bytes = 99;
    assert!(request.validate().is_err());
    let mut json = serde_json::to_value(&request).unwrap();
    json["destination_path"] = "/tmp/leak.parquet".into();
    assert!(serde_json::from_value::<ExportQuery>(json).is_err());
}

#[test]
fn export_catalogue_reuses_query_authorization_and_generates_schemas() {
    let catalogue = rrd_contract::endpoint_catalogue();
    let export = catalogue
        .endpoints
        .iter()
        .find(|e| e.operation.as_str() == "query-export")
        .unwrap();
    assert_eq!(export.path, "/v1/query/export");
    assert_eq!(
        export.authentication,
        rrd_contract::EndpointAuthentication::SessionBearer
    );
    assert_eq!(
        export.action.fixed_action(),
        Some(rrd_contract::SecurityAction::QueryExecute)
    );
    assert!(!export.mutation);
}
