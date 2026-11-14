//! Catalogue identity lineage and exact security actions.

use super::*;

#[test]
pub(super) fn catalogue_identity_lineage_cannot_reset_after_an_inactive_membership() {
    let (root, engine) = isolated_engine();
    let lease = engine
        .create_session(
            &session_request(5_000, 2),
            &id("function-lineage-session"),
            1_000,
            "request-function-lineage-session",
            "operation-function-lineage-session",
        )
        .unwrap();
    let original = javascript("lineage-policy", "() => true");
    let original_definition_sha256 = original.sha256();
    let original_binding = binding(
        "lineage-binding",
        &original,
        TransactionMutationKind::AssertClaim,
        None,
        TransactionFunctionEffect::RequireTrue,
    );
    let original_binding_sha256 = original_binding.sha256();
    install(
        &engine,
        &lease,
        0,
        catalogue(1, vec![original.clone()], vec![original_binding.clone()]),
        "lineage-one",
    );
    install(
        &engine,
        &lease,
        1,
        catalogue(2, vec![], vec![]),
        "lineage-inactive",
    );

    let reset_definition = javascript("lineage-policy", "() => false");
    let reset_definition_result = engine.replace_function_catalogue(
        &lease.session_id,
        &lease.token,
        &ReplaceFunctionCatalogue {
            expected_revision: 2,
            catalogue: catalogue(3, vec![reset_definition], vec![]),
        },
        1_200,
        "request-lineage-definition-reset",
        "operation-lineage-definition-reset",
    );
    assert!(matches!(
        reset_definition_result,
        Err(ServiceError::Contract(_))
    ));

    let reset_binding = binding(
        "lineage-binding",
        &original,
        TransactionMutationKind::PutSchema,
        None,
        TransactionFunctionEffect::RequireTrue,
    );
    let reset_binding_result = engine.replace_function_catalogue(
        &lease.session_id,
        &lease.token,
        &ReplaceFunctionCatalogue {
            expected_revision: 2,
            catalogue: catalogue(3, vec![original.clone()], vec![reset_binding]),
        },
        1_201,
        "request-lineage-binding-reset",
        "operation-lineage-binding-reset",
    );
    assert!(matches!(
        reset_binding_result,
        Err(ServiceError::Contract(_))
    ));
    assert_eq!(
        engine.load_current_function_catalogue().unwrap().revision,
        2
    );

    let mut successor = javascript("lineage-policy", "() => false");
    successor.revision = 2;
    successor.predecessor_sha256 = Some(original_definition_sha256);
    let mut successor_binding = binding(
        "lineage-binding",
        &successor,
        TransactionMutationKind::AssertClaim,
        None,
        TransactionFunctionEffect::RequireTrue,
    );
    successor_binding.revision = 2;
    successor_binding.predecessor_sha256 = Some(original_binding_sha256);
    install(
        &engine,
        &lease,
        2,
        catalogue(3, vec![successor], vec![successor_binding]),
        "lineage-three",
    );
    drop(engine);

    let reopened = RrdEngine::open(root.path(), instance(), TOKEN_KEY).unwrap();
    let restored = reopened.load_current_function_catalogue().unwrap();
    assert_eq!(restored.revision, 3);
    assert_eq!(
        restored.functions[&CanonicalId::new("lineage-policy").unwrap()].revision,
        2
    );
    assert_eq!(
        restored.transaction_bindings[&CanonicalId::new("lineage-binding").unwrap()].revision,
        2
    );
}

#[test]
pub(super) fn exact_security_actions_gate_catalogue_execution_and_transaction_bindings() {
    let (_root, engine) = isolated_engine();
    let admin_id = CanonicalId::new("function-admin").unwrap();
    let limited_id = CanonicalId::new("function-reader").unwrap();
    let resource = ResourcePath {
        segments: vec![ResourceId::new(ResourceKind::Instance, instance().as_str()).unwrap()],
    };
    let principal =
        |principal_id: CanonicalId, credential: &[u8], actions: &[SecurityAction]| Principal {
            id: principal_id,
            kind: PrincipalKind::Service,
            credential_sha256: digest::sha256_hex(credential),
            credential_revision: 1,
            not_before_unix_ms: 1,
            expires_at_unix_ms: u64::MAX,
            disabled: false,
            role_ids: BTreeSet::new(),
            grants: actions
                .iter()
                .copied()
                .map(|action| ResourceGrant {
                    action,
                    resource_prefix: resource.clone(),
                    data_policy: None,
                })
                .collect(),
        };
    SecurityRepository::new(&engine.storage, instance())
        .initialize(
            SecurityState {
                format_version: rrd_security::SECURITY_FORMAT,
                revision: 1,
                principals: BTreeMap::from([
                    (
                        admin_id.clone(),
                        principal(
                            admin_id.clone(),
                            b"function-admin-key",
                            &[
                                SecurityAction::SessionCreate,
                                SecurityAction::FunctionCatalogueRead,
                                SecurityAction::FunctionCatalogueWrite,
                                SecurityAction::FunctionExecute,
                            ],
                        ),
                    ),
                    (
                        limited_id.clone(),
                        principal(
                            limited_id.clone(),
                            b"function-reader-key",
                            &[
                                SecurityAction::SessionCreate,
                                SecurityAction::FunctionCatalogueRead,
                                SecurityAction::TransactionBegin,
                                SecurityAction::TransactionCommit,
                            ],
                        ),
                    ),
                ]),
                roles: BTreeMap::new(),
                identity_bindings: BTreeMap::new(),
                jwt_issuers: BTreeMap::new(),
            },
            1,
            "function-security-test",
            "request-function-security",
            "operation-function-security",
        )
        .unwrap();
    let admin = engine
        .create_authenticated_session(
            &admin_id,
            b"function-admin-key",
            &session_request(5_000, 2),
            &id("function-admin-session"),
            1_000,
            "request-function-admin-session",
            "operation-function-admin-session",
        )
        .unwrap();
    let limited = engine
        .create_authenticated_session(
            &limited_id,
            b"function-reader-key",
            &session_request(5_000, 2),
            &id("function-reader-session"),
            1_000,
            "request-function-reader-session",
            "operation-function-reader-session",
        )
        .unwrap();
    let definition = javascript("secured-policy", "() => true");
    let binding = binding(
        "secured-binding",
        &definition,
        TransactionMutationKind::AssertClaim,
        None,
        TransactionFunctionEffect::RequireTrue,
    );
    install(
        &engine,
        &admin,
        0,
        catalogue(1, vec![definition], vec![binding]),
        "secured",
    );

    assert_eq!(
        engine
            .function_catalogue(
                &limited.session_id,
                &limited.token,
                1_100,
                "request-function-list-allowed",
                "operation-function-list-allowed",
            )
            .unwrap()
            .revision,
        1,
    );
    let denied_execute = engine.execute_function(
        &limited.session_id,
        &limited.token,
        &ExecuteFunction {
            function_id: CanonicalId::new("secured-policy").unwrap(),
            input: QueryValue::Null,
        },
        1_101,
        "request-function-execute-denied",
        "operation-function-execute-denied",
    );
    assert!(matches!(
        denied_execute,
        Err(ServiceError::PermissionDenied)
    ));
    let denied_write = engine.replace_function_catalogue(
        &limited.session_id,
        &limited.token,
        &ReplaceFunctionCatalogue {
            expected_revision: 1,
            catalogue: catalogue(
                2,
                vec![javascript("replacement-policy", "() => true")],
                vec![],
            ),
        },
        1_102,
        "request-function-write-denied",
        "operation-function-write-denied",
    );
    assert!(matches!(denied_write, Err(ServiceError::PermissionDenied)));

    let transaction = engine
        .begin_transaction(
            &limited.session_id,
            &limited.token,
            &begin_request(),
            &mutation_context(
                &id("function-binding-begin"),
                "request-function-binding-begin",
                "operation-function-binding-begin",
            ),
            1_110,
        )
        .unwrap();
    let before = engine.storage.runtime().cursor().unwrap();
    let request = commit_request("function-binding-must-be-denied");
    let denied_binding = engine.commit_transaction(
        &limited.session_id,
        &limited.token,
        &transaction.transaction_id,
        &id("function-binding-commit"),
        &request,
        1_120,
        "request-function-binding-denied",
        "operation-function-binding-denied",
    );
    assert!(matches!(
        denied_binding,
        Err(ServiceError::PermissionDenied)
    ));
    assert_eq!(engine.storage.runtime().cursor().unwrap(), before);

    let records = SecurityRepository::new(&engine.storage, instance())
        .audit_since(0, 64)
        .unwrap()
        .records;
    for (request_id, action) in [
        (
            "request-function-execute-denied",
            SecurityAction::FunctionExecute,
        ),
        (
            "request-function-write-denied",
            SecurityAction::FunctionCatalogueWrite,
        ),
        (
            "request-function-binding-denied",
            SecurityAction::FunctionExecute,
        ),
    ] {
        assert!(records.iter().any(|(_, record)| {
            record.request_id == request_id
                && record.action == action
                && record.phase == AuditPhase::Completed
                && record.decision == AuditDecision::Denied
        }));
    }
}
