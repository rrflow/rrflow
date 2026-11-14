//! Transaction binding effects, retries, prepared receipts and lost acknowledgements.

use super::*;

#[test]
pub(super) fn transaction_binding_effects_commit_atomically_and_failures_leave_data_unchanged() {
    let (_root, engine) = isolated_engine();
    let lease = engine
        .create_session(
            &session_request(10_000, 4),
            &id("function-binding-session"),
            1_000,
            "request-function-binding-session",
            "operation-function-binding-session",
        )
        .unwrap();
    let mut event_function = javascript(
        "emit-person-event",
        "(input) => ({ mutation_index: input.mutation_index })",
    );
    event_function
        .capabilities
        .insert(FunctionCapability::EmitEvent);
    let event_binding = binding(
        "person-event",
        &event_function,
        TransactionMutationKind::PutRecord,
        Some(CanonicalId::new("person").unwrap()),
        TransactionFunctionEffect::AppendEvent {
            kind: CanonicalId::new("person-changed").unwrap(),
        },
    );
    install(
        &engine,
        &lease,
        0,
        catalogue(1, vec![event_function], vec![event_binding]),
        "binding-one",
    );

    let transaction = engine
        .begin_transaction(
            &lease.session_id,
            &lease.token,
            &BeginTransaction {
                scope: CanonicalId::new("data").unwrap(),
                timeout_ms: 5_000,
            },
            &mutation_context(
                &id("begin-binding-one"),
                "request-begin-binding-one",
                "operation-begin-binding-one",
            ),
            1_100,
        )
        .unwrap();
    let person = CanonicalId::new("person").unwrap();
    let changed = CanonicalId::new("person-changed").unwrap();
    let registry = DataSchemaRegistry {
        revision: 1,
        migration: "install transaction function binding fixture".into(),
        organisation: DataEstateOrganisation::default(),
        tables: BTreeMap::from([
            (
                person.clone(),
                DataTableSchema {
                    model: DataLogicalModel::Document,
                    mode: DataSchemaMode::Strict,
                    properties: BTreeMap::new(),
                    allow_additional_properties: false,
                },
            ),
            (
                changed.clone(),
                DataTableSchema {
                    model: DataLogicalModel::Event,
                    mode: DataSchemaMode::Strict,
                    properties: BTreeMap::new(),
                    allow_additional_properties: false,
                },
            ),
        ]),
        records: BTreeMap::from([(
            person.clone(),
            DataRecordSchema {
                properties: BTreeMap::new(),
                allow_additional_properties: true,
                unique_properties: BTreeSet::new(),
            },
        )]),
        relations: BTreeMap::new(),
        events: BTreeMap::from([(
            changed.clone(),
            DataEventSchema {
                subject_required: false,
                subject_types: BTreeSet::new(),
                properties: BTreeMap::new(),
                allow_additional_properties: true,
            },
        )]),
    };
    let mutations = vec![
        TransactionMutation::PutSchema { registry },
        TransactionMutation::PutRecord {
            reference: DataReference {
                kind: person.clone(),
                id: CanonicalId::new("alice").unwrap(),
            },
            valid_from: 1_200,
            valid_to: None,
            properties: BTreeMap::new(),
        },
    ];
    let commit = CommitTransaction {
        operation_sha256: rrd_contract::transaction_operation_sha256(&mutations),
        mutations,
    };
    engine
        .commit_transaction(
            &lease.session_id,
            &lease.token,
            &transaction.transaction_id,
            &id("commit-binding-one"),
            &commit,
            1_200,
            "request-commit-binding-one",
            "operation-commit-binding-one",
        )
        .unwrap();
    let snapshot = engine
        .read_data_snapshot(
            &lease.session_id,
            &lease.token,
            &ReadDataSnapshot {
                valid_at: 1_200,
                max_storage_keys: 100,
            },
            1_201,
            "request-binding-snapshot",
            "operation-binding-snapshot",
        )
        .unwrap();
    assert!(snapshot.entries.iter().any(|entry| {
        matches!(
            &entry.value,
            TransactionMutation::AppendEvent { kind, .. } if kind == &changed
        )
    }));
    let binding_audit = SecurityRepository::new(&engine.storage, instance())
        .audit_since(0, 64)
        .unwrap();
    assert!(!binding_audit.records.iter().any(|(_, record)| {
        record.action == SecurityAction::FunctionExecute
            && record.phase == rrd_contract::AuditPhase::Completed
            && record.decision == AuditDecision::Allowed
    }), "successful transaction functions are evidenced by atomic receipts, not a pre-commit allowed audit");
    let (_, state) = engine
        .load_authenticated(&lease.session_id, &lease.token)
        .unwrap();
    let record = engine
        .transaction_record(&state, &transaction.transaction_id)
        .unwrap()
        .unwrap();
    let function_receipt: FunctionInvocationReceipt =
        record.commit_intent.unwrap().function_receipts[0].clone();
    let stored_receipt = engine
        .storage
        .function_catalogue()
        .invocation_receipt(instance().as_str(), function_receipt.invocation_id.as_str())
        .unwrap()
        .unwrap();
    assert_eq!(
        stored_receipt.receipt_sha256,
        function_receipt.receipt_sha256
    );
    assert_eq!(
        stored_receipt.runtime_commit_sha256,
        function_receipt.runtime_commit_sha256
    );
    assert!(engine
        .storage
        .runtime()
        .audit(function_receipt.runtime_commit_sha256.as_deref().unwrap())
        .unwrap()
        .is_some());
    engine
        .require_committed_transaction_function_receipts(std::slice::from_ref(&function_receipt))
        .unwrap();
    let mut absent_receipt = function_receipt.clone();
    absent_receipt.invocation_id = CanonicalId::new("function-absent-receipt").unwrap();
    absent_receipt.receipt_sha256.clear();
    let absent_receipt = absent_receipt.seal().unwrap();
    assert!(matches!(
        engine
            .require_committed_transaction_function_receipts(std::slice::from_ref(&absent_receipt)),
        Err(ServiceError::OperationDigestMismatch)
    ));

    let reject = javascript("reject-person", "() => false");
    let reject_binding = binding(
        "reject-person",
        &reject,
        TransactionMutationKind::PutRecord,
        Some(person.clone()),
        TransactionFunctionEffect::RequireTrue,
    );
    install(
        &engine,
        &lease,
        1,
        catalogue(2, vec![reject], vec![reject_binding]),
        "binding-two",
    );
    let transaction = engine
        .begin_transaction(
            &lease.session_id,
            &lease.token,
            &BeginTransaction {
                scope: CanonicalId::new("data").unwrap(),
                timeout_ms: 5_000,
            },
            &mutation_context(
                &id("begin-binding-two"),
                "request-begin-binding-two",
                "operation-begin-binding-two",
            ),
            1_300,
        )
        .unwrap();
    let before = engine.storage.runtime().cursor().unwrap();
    let mutations = vec![TransactionMutation::PutRecord {
        reference: DataReference {
            kind: person,
            id: CanonicalId::new("bob").unwrap(),
        },
        valid_from: 1_300,
        valid_to: None,
        properties: BTreeMap::new(),
    }];
    let rejected = engine.commit_transaction(
        &lease.session_id,
        &lease.token,
        &transaction.transaction_id,
        &id("commit-binding-two"),
        &CommitTransaction {
            operation_sha256: rrd_contract::transaction_operation_sha256(&mutations),
            mutations,
        },
        1_301,
        "request-commit-binding-two",
        "operation-commit-binding-two",
    );
    assert!(matches!(rejected, Err(ServiceError::Function(_))));
    assert_eq!(engine.storage.runtime().cursor().unwrap(), before);
}

#[test]
pub(super) fn transaction_retry_reuses_prepared_receipts_pinned_before_the_crash_window() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("function-retry");
    let engine = RrdEngine::open(&path, instance(), TOKEN_KEY).unwrap();
    let lease = engine
        .create_session(
            &session_request(5_000, 2),
            &id("function-retry-session"),
            1_000,
            "request-function-retry-session",
            "operation-function-retry-session",
        )
        .unwrap();
    let binding_id = CanonicalId::new("claim-validator").unwrap();
    let accepted = javascript("claim-policy", "() => true");
    let accepted_binding = binding(
        binding_id.as_str(),
        &accepted,
        TransactionMutationKind::AssertClaim,
        None,
        TransactionFunctionEffect::RequireTrue,
    );
    let accepted_definition_sha256 = accepted.sha256();
    let accepted_binding_sha256 = accepted_binding.sha256();
    install(
        &engine,
        &lease,
        0,
        catalogue(1, vec![accepted], vec![accepted_binding]),
        "retry-one",
    );
    let transaction = engine
        .begin_transaction(
            &lease.session_id,
            &lease.token,
            &begin_request(),
            &mutation_context(
                &id("function-retry-begin"),
                "request-function-retry-begin",
                "operation-function-retry-begin",
            ),
            1_100,
        )
        .unwrap();
    let request = commit_request("revision-one-must-win");
    let runtime_at = 1_150;
    let base = public_runtime_commit(
        &request,
        &lease.session_id,
        &instance(),
        transaction.read_cursor,
        runtime_at,
    )
    .unwrap();
    let revision_one = engine.load_current_function_catalogue().unwrap();
    let prepared_functions = engine
        .prepare_transaction_function_bindings(
            &revision_one,
            &request,
            &transaction.transaction_id,
            base,
            None,
            runtime_at,
            "request-function-retry-prepare",
            "operation-function-retry-prepare",
        )
        .unwrap();
    let prepared_receipt = prepared_functions.receipts[0].clone();
    persist_function_commit_intent(
        &engine,
        &lease,
        &transaction.transaction_id,
        "function-retry-commit",
        &request,
        runtime_at,
        &prepared_functions.commit.digest(),
        1,
        &prepared_functions.receipts,
        "function-retry-prepare",
    );

    let mut rejected = javascript("claim-policy", "() => false");
    rejected.revision = 2;
    rejected.predecessor_sha256 = Some(accepted_definition_sha256);
    let mut rejected_binding = binding(
        binding_id.as_str(),
        &rejected,
        TransactionMutationKind::AssertClaim,
        None,
        TransactionFunctionEffect::RequireTrue,
    );
    rejected_binding.revision = 2;
    rejected_binding.predecessor_sha256 = Some(accepted_binding_sha256);
    install(
        &engine,
        &lease,
        1,
        catalogue(2, vec![rejected], vec![rejected_binding]),
        "retry-two",
    );
    drop(engine);

    let reopened = RrdEngine::open(&path, instance(), TOKEN_KEY).unwrap();
    let receipt = reopened
        .commit_transaction(
            &lease.session_id,
            &lease.token,
            &transaction.transaction_id,
            &id("function-retry-commit"),
            &request,
            1_300,
            "request-function-retry-commit",
            "operation-function-retry-commit",
        )
        .unwrap();
    assert_eq!(receipt.last_runtime_cursor, Some(1));
    let stored_receipt = reopened
        .storage
        .function_catalogue()
        .invocation_receipt(instance().as_str(), prepared_receipt.invocation_id.as_str())
        .unwrap()
        .unwrap();
    assert_eq!(
        stored_receipt.receipt_sha256,
        prepared_receipt.receipt_sha256
    );
    assert_eq!(
        stored_receipt.runtime_commit_sha256,
        prepared_receipt.runtime_commit_sha256
    );
    assert_eq!(
        reopened.load_current_function_catalogue().unwrap().revision,
        2,
        "recovery must retain the newer head while executing pinned revision one",
    );
}

#[test]
pub(super) fn substituted_runtime_build_in_a_prepared_receipt_fails_closed_after_reopen() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("function-runtime-substitution");
    let engine = RrdEngine::open(&path, instance(), TOKEN_KEY).unwrap();
    let lease = engine
        .create_session(
            &session_request(5_000, 2),
            &id("function-runtime-substitution-session"),
            1_000,
            "request-function-runtime-substitution-session",
            "operation-function-runtime-substitution-session",
        )
        .unwrap();
    let accepted = javascript("runtime-bound-policy", "() => true");
    let accepted_binding = binding(
        "runtime-bound-validator",
        &accepted,
        TransactionMutationKind::AssertClaim,
        None,
        TransactionFunctionEffect::RequireTrue,
    );
    install(
        &engine,
        &lease,
        0,
        catalogue(1, vec![accepted], vec![accepted_binding]),
        "runtime-substitution",
    );
    let transaction = engine
        .begin_transaction(
            &lease.session_id,
            &lease.token,
            &begin_request(),
            &mutation_context(
                &id("function-runtime-substitution-begin"),
                "request-function-runtime-substitution-begin",
                "operation-function-runtime-substitution-begin",
            ),
            1_100,
        )
        .unwrap();
    let request = commit_request("substituted-runtime-build-must-not-commit");
    let runtime_at = 1_150;
    let base = public_runtime_commit(
        &request,
        &lease.session_id,
        &instance(),
        transaction.read_cursor,
        runtime_at,
    )
    .unwrap();
    let prepared = engine
        .prepare_transaction_function_bindings(
            &engine.load_current_function_catalogue().unwrap(),
            &request,
            &transaction.transaction_id,
            base,
            None,
            runtime_at,
            "request-function-runtime-substitution-prepare",
            "operation-function-runtime-substitution-prepare",
        )
        .unwrap();
    let expected_commit_sha256 = prepared.commit.digest();
    let invocation_id = prepared.receipts[0].invocation_id.clone();
    let mut substituted_receipt = prepared.receipts[0].clone();
    substituted_receipt.runtime_build_sha256 =
        digest::sha256_hex(b"unavailable-substituted-runtime-build");
    let substituted_receipt = substituted_receipt.seal().unwrap();
    persist_function_commit_intent(
        &engine,
        &lease,
        &transaction.transaction_id,
        "function-runtime-substitution-commit",
        &request,
        runtime_at,
        &expected_commit_sha256,
        1,
        &[substituted_receipt],
        "function-runtime-substitution-prepare",
    );
    drop(engine);

    let reopened = RrdEngine::open(&path, instance(), TOKEN_KEY).unwrap();
    let before_cursor = reopened.storage.runtime().cursor().unwrap();
    let rejected = reopened.commit_transaction(
        &lease.session_id,
        &lease.token,
        &transaction.transaction_id,
        &id("function-runtime-substitution-commit"),
        &request,
        1_300,
        "request-function-runtime-substitution-commit",
        "operation-function-runtime-substitution-commit",
    );
    assert!(matches!(
        rejected,
        Err(ServiceError::OperationDigestMismatch)
    ));
    assert_eq!(reopened.storage.runtime().cursor().unwrap(), before_cursor);
    assert_eq!(reopened.storage.claims().sequence().unwrap(), 0);
    assert!(reopened
        .storage
        .runtime()
        .commit_outcome(&expected_commit_sha256)
        .unwrap()
        .is_none());
    assert!(reopened
        .storage
        .function_catalogue()
        .invocation_receipt(instance().as_str(), invocation_id.as_str())
        .unwrap()
        .is_none());
}

#[test]
pub(super) fn durable_function_receipt_closes_a_lost_ack_without_reexecution() {
    super::super::super::function::reset_test_execution_count();
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("function-lost-ack");
    let engine = RrdEngine::open(&path, instance(), TOKEN_KEY).unwrap();
    let lease = engine
        .create_session(
            &session_request(5_000, 2),
            &id("function-lost-ack-session"),
            1_000,
            "request-function-lost-ack-session",
            "operation-function-lost-ack-session",
        )
        .unwrap();
    let binding_id = CanonicalId::new("lost-ack-validator").unwrap();
    let accepted = javascript("lost-ack-policy", "() => true");
    let accepted_definition_sha256 = accepted.sha256();
    let accepted_binding = binding(
        binding_id.as_str(),
        &accepted,
        TransactionMutationKind::AssertClaim,
        None,
        TransactionFunctionEffect::RequireTrue,
    );
    let accepted_binding_sha256 = accepted_binding.sha256();
    install(
        &engine,
        &lease,
        0,
        catalogue(1, vec![accepted], vec![accepted_binding]),
        "lost-ack-one",
    );
    let transaction = engine
        .begin_transaction(
            &lease.session_id,
            &lease.token,
            &begin_request(),
            &mutation_context(
                &id("function-lost-ack-begin"),
                "request-function-lost-ack-begin",
                "operation-function-lost-ack-begin",
            ),
            1_100,
        )
        .unwrap();
    let request = commit_request("durable-receipt-must-win");
    let runtime_at = 1_150;
    let base = public_runtime_commit(
        &request,
        &lease.session_id,
        &instance(),
        transaction.read_cursor,
        runtime_at,
    )
    .unwrap();
    let prepared = engine
        .prepare_transaction_function_bindings(
            &engine.load_current_function_catalogue().unwrap(),
            &request,
            &transaction.transaction_id,
            base,
            None,
            runtime_at,
            "request-function-lost-ack-prepare",
            "operation-function-lost-ack-prepare",
        )
        .unwrap();
    assert_eq!(super::super::super::function::test_execution_count(), 1);
    let expected_commit_sha256 = prepared.commit.digest();
    let prepared_receipt = prepared.receipts[0].clone();
    persist_function_commit_intent(
        &engine,
        &lease,
        &transaction.transaction_id,
        "function-lost-ack-commit",
        &request,
        runtime_at,
        &expected_commit_sha256,
        1,
        &prepared.receipts,
        "function-lost-ack-prepare",
    );

    let scope = ScopeId::new(format!("instance:{}", instance())).unwrap();
    let read = engine.storage.runtime().read_stamp(&scope).unwrap();
    assert_eq!(read.commit_cursor, transaction.read_cursor);
    let data_transaction = DataTransaction::new(read, prepared.commit).unwrap();
    let receipt_records = prepared
        .receipts
        .iter()
        .map(super::super::super::function::encode_receipt_record)
        .collect::<std::result::Result<Vec<_>, _>>()
        .unwrap();
    let committed = engine
        .storage
        .runtime()
        .commit_data_transaction_with_function_receipts(
            &data_transaction,
            instance().as_str(),
            &receipt_records,
        )
        .unwrap();
    assert_eq!(committed.last_cursor, 1);
    assert_eq!(super::super::super::function::test_execution_count(), 1);

    let mut rejected = javascript("lost-ack-policy", "() => false");
    rejected.revision = 2;
    rejected.predecessor_sha256 = Some(accepted_definition_sha256);
    let mut rejected_binding = binding(
        binding_id.as_str(),
        &rejected,
        TransactionMutationKind::AssertClaim,
        None,
        TransactionFunctionEffect::RequireTrue,
    );
    rejected_binding.revision = 2;
    rejected_binding.predecessor_sha256 = Some(accepted_binding_sha256);
    install(
        &engine,
        &lease,
        1,
        catalogue(2, vec![rejected], vec![rejected_binding]),
        "lost-ack-two",
    );
    drop(engine);

    let reopened = RrdEngine::open(&path, instance(), TOKEN_KEY).unwrap();
    assert_eq!(
        reopened.load_current_function_catalogue().unwrap().revision,
        2
    );
    let before_retry_cursor = reopened.storage.runtime().cursor().unwrap();
    let recovered = reopened
        .commit_transaction(
            &lease.session_id,
            &lease.token,
            &transaction.transaction_id,
            &id("function-lost-ack-commit"),
            &request,
            1_300,
            "request-function-lost-ack-commit",
            "operation-function-lost-ack-commit",
        )
        .unwrap();
    assert!(recovered.idempotent_replay);
    assert_eq!(recovered.last_runtime_cursor, Some(1));
    assert_eq!(
        reopened.storage.runtime().cursor().unwrap(),
        before_retry_cursor
    );
    assert_eq!(reopened.storage.claims().sequence().unwrap(), 1);
    assert_eq!(
        super::super::super::function::test_execution_count(),
        1,
        "recovery must use the durable build-bound receipt instead of executing a function again",
    );
    let stored_receipt = reopened
        .storage
        .function_catalogue()
        .invocation_receipt(instance().as_str(), prepared_receipt.invocation_id.as_str())
        .unwrap()
        .unwrap();
    assert_eq!(
        stored_receipt.receipt_sha256,
        prepared_receipt.receipt_sha256
    );
    assert_eq!(
        stored_receipt.runtime_commit_sha256,
        Some(expected_commit_sha256)
    );
}
