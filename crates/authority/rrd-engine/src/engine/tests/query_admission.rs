use super::*;
use rrd_contract::{
    DataLogicalModel, DataPropertySchema, DataRecordSchema, DataReference, DataSchemaMode,
    DataSchemaRegistry, DataTableSchema, DataValueType, ExecuteQuery, QueryBudget, QueryValue,
};
use std::collections::BTreeMap;

#[test]
fn public_query_uses_one_key_budget_after_unrelated_growth_and_native_reopen() {
    let (root, engine) = isolated_engine();
    let lease = engine
        .create_session(
            &session_request(8000, 2),
            &id("query-admission-session"),
            1000,
            "query-admission-session",
            "query-admission-session",
        )
        .unwrap();
    let transaction = engine
        .begin_transaction(
            &lease.session_id,
            &lease.token,
            &BeginTransaction {
                scope: CanonicalId::new("data").unwrap(),
                timeout_ms: 1000,
            },
            &mutation_context(
                &id("query-admission-begin"),
                "query-admission-begin",
                "query-admission-begin",
            ),
            1100,
        )
        .unwrap();
    let kind = CanonicalId::new("document").unwrap();
    let mut mutations = vec![TransactionMutation::PutSchema {
        registry: DataSchemaRegistry {
            revision: 1,
            migration: "query admission".into(),
            catalogue: DataCatalogueIdentity::default(),
            tables: BTreeMap::from([(
                kind.clone(),
                DataTableSchema {
                    model: DataLogicalModel::Document,
                    mode: DataSchemaMode::Strict,
                    properties: BTreeMap::new(),
                    allow_additional_properties: false,
                },
            )]),
            records: BTreeMap::from([(
                kind.clone(),
                DataRecordSchema {
                    properties: BTreeMap::from([(
                        "ordinal".into(),
                        DataPropertySchema {
                            value_type: DataValueType::Unsigned,
                            required: true,
                        },
                    )]),
                    allow_additional_properties: false,
                    unique_properties: Default::default(),
                },
            )]),
            relations: BTreeMap::new(),
            events: BTreeMap::new(),
        },
    }];
    for n in 0..100 {
        mutations.push(TransactionMutation::PutRecord {
            reference: DataReference {
                kind: kind.clone(),
                id: CanonicalId::new(format!("item-{n:03}")).unwrap(),
            },
            valid_from: 1100,
            valid_to: None,
            properties: BTreeMap::from([("ordinal".into(), QueryValue::Unsigned(n))]),
        });
    }
    engine
        .commit_transaction(
            &lease.session_id,
            &lease.token,
            &transaction.transaction_id,
            &id("query-admission-commit"),
            &CommitTransaction {
                operation_sha256: digest::sha256_hex(&serde_json::to_vec(&mutations).unwrap()),
                mutations,
            },
            1200,
            "query-admission-commit",
            "query-admission-commit",
        )
        .unwrap();
    let check = |engine: &RrdEngine| {
        for (filter, expected) in [
            ("id = \"item-000\"", 1),
            ("id >= \"item-000\" AND id < \"item-002\"", 2),
        ] {
            let mut request = ExecuteQuery { scope: format!("instance:{}", instance()),
                query: format!("FROM record:document AT VALID 1200 KNOWN HEAD WHERE {filter} PROJECT id, ordinal"),
                parameters: BTreeMap::new(), budget: QueryBudget { max_storage_keys: 128, ..QueryBudget::default() } };
            let result = engine
                .execute_query(
                    &lease.session_id,
                    &lease.token,
                    &request,
                    1300,
                    "query-admission-read",
                    "query-admission-read",
                )
                .unwrap();
            assert_eq!(result.rows.len(), expected);
            let used = result.execution.read_evidence.keys_examined;
            assert!(used < 128);
            request.budget.max_storage_keys = used;
            assert!(engine
                .execute_query(
                    &lease.session_id,
                    &lease.token,
                    &request,
                    1300,
                    "query-admission-exact",
                    "query-admission-exact"
                )
                .is_ok());
            request.budget.max_storage_keys -= 1;
            let error = engine
                .execute_query(
                    &lease.session_id,
                    &lease.token,
                    &request,
                    1300,
                    "query-admission-deny",
                    "query-admission-deny",
                )
                .unwrap_err();
            assert_eq!(error.kind(), crate::ServiceErrorKind::ResourceExhausted);
            request.budget.max_storage_keys = 128;
            request.query =
                "FROM record:document AT VALID 1200 KNOWN HEAD WHERE unknown_field = 1 PROJECT *"
                    .into();
            assert_eq!(
                engine
                    .execute_query(
                        &lease.session_id,
                        &lease.token,
                        &request,
                        1300,
                        "query-admission-invalid",
                        "query-admission-invalid"
                    )
                    .unwrap_err()
                    .kind(),
                crate::ServiceErrorKind::InvalidArgument
            );
        }
    };
    check(&engine);
    drop(engine);
    check(&RrdEngine::open(root.path(), instance(), TOKEN_KEY).unwrap());
}
