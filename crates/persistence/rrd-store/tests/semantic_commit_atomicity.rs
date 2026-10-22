use rrd_core::{
    digest, DataTransaction, RuntimeCommit, RuntimeLogicalModel, RuntimeMutation,
    RuntimeProperties, RuntimeRecord, RuntimeRecordSchema, RuntimeRef, RuntimeRelation,
    RuntimeRelationSchema, RuntimeSchemaRegistry, RuntimeType, ScopeId,
};
use rrd_store::{
    Error, FunctionInvocationReceiptRecord, RrflowKvStore, RrflowMxStore, StorageEngine,
    FUNCTION_INVOCATION_RECEIPT_FORMAT_VERSION,
};
use std::collections::BTreeSet;

#[derive(Debug, PartialEq, Eq)]
struct PublicEvidence {
    commit_id: String,
    cursor: u64,
    changes: usize,
    records: usize,
    relations: usize,
    delta_cursors: Vec<u64>,
    audit_cursor: u64,
    function_invocation_id: String,
    function_receipt_sha256: String,
}

// This fixture has only canonical typed keys (all below 0xff). Capture every
// physical row, including derived indexes and receipts, rather than accepting
// an unchanged cursor as proof that a rejected plan published nothing.
fn physical_rows(storage: &dyn StorageEngine) -> Vec<(Vec<u8>, Vec<u8>)> {
    let transaction = storage.begin_transaction().unwrap();
    let rows = transaction.scan(&[0], &[0xff], 65_536).unwrap();
    assert!(
        rows.len() < 65_536,
        "oracle must cover the complete fixture"
    );
    rows
}

fn reject_oversized_plan(storage: &dyn StorageEngine) -> Vec<(Vec<u8>, Vec<u8>)> {
    let seeded = commit(storage);
    assert_eq!((seeded.records, seeded.relations), (2, 1));
    let before = physical_rows(storage);
    let claim = |index: usize, object: String| {
        rrd_core::Claim::new(
            rrd_core::Subject::new(format!("admission-{index}")).unwrap(),
            rrd_core::Predicate::new("contains").unwrap(),
            object,
            200,
            200,
            rrd_core::Producer {
                actor: "admission-oracle".into(),
                on_behalf_of: None,
                session: None,
            },
        )
    };
    let mut request = RuntimeCommit {
        scope: ScopeId::new("project:semantic-commit-public").unwrap(),
        at: 200,
        actor: "admission-oracle".into(),
        expected_cursor: seeded.cursor,
        mutations: (0..4096)
            .map(|index| RuntimeMutation::Claim {
                claim: claim(index, "x".repeat(4096)),
            })
            .collect(),
    };
    request.validate().unwrap();
    let rejected_id = request.digest();
    assert!(matches!(storage.runtime().commit(&request),
        Err(Error::WriteBatchLimit { resource: rrd_lsm::WriteBatchResource::EncodedBytes, limit, observed })
            if limit == 16 * 1024 * 1024 && observed > limit));
    assert_eq!(
        physical_rows(storage),
        before,
        "rejection changed physical domain/index/audit/receipt state"
    );
    assert!(storage
        .runtime()
        .commit_outcome(&rejected_id)
        .unwrap()
        .is_none());
    assert!(storage.runtime().audit(&rejected_id).unwrap().is_none());
    // A fresh, valid semantic commit uses the same authority and remaining
    // capacity after rejection; its graph and function receipt stay intact.
    request.mutations = vec![RuntimeMutation::Claim {
        claim: claim(0, "accepted-after-rejection".into()),
    }];
    let accepted = storage.runtime().commit(&request).unwrap();
    assert_eq!(accepted.last_cursor, seeded.cursor + 1);
    let snapshot = storage
        .runtime()
        .data_snapshot(&request.scope, 200, 8192)
        .unwrap()
        .snapshot;
    assert_eq!((snapshot.records.len(), snapshot.relations.len()), (2, 1));
    let claims = storage.claims().claims_in_range(0, 8192).unwrap();
    assert_eq!(claims.len(), 1);
    assert_eq!(claims[0].object, "accepted-after-rejection");
    assert_eq!(
        storage
            .function_catalogue()
            .invocation_receipt("semantic-commit-instance", &seeded.function_invocation_id)
            .unwrap()
            .unwrap()
            .receipt_sha256,
        seeded.function_receipt_sha256
    );
    physical_rows(storage)
}

#[test]
fn oversized_semantic_plan_preserves_all_physical_effects_on_mx() {
    reject_oversized_plan(&RrflowMxStore::new());
}

#[test]
fn oversized_semantic_plan_preserves_all_physical_effects_on_kv_and_reopen() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("admission");
    let expected = {
        let storage = RrflowKvStore::open(&root).unwrap();
        reject_oversized_plan(&storage)
    };
    assert_eq!(
        physical_rows(&RrflowKvStore::open(&root).unwrap()),
        expected
    );
}

fn commit(storage: &dyn StorageEngine) -> PublicEvidence {
    let scope = ScopeId::new("project:semantic-commit-public").unwrap();
    let file = RuntimeType::new("file").unwrap();
    let imports = RuntimeType::new("imports").unwrap();
    let mut schema = RuntimeSchemaRegistry::empty(1, "public semantic commit corpus");
    schema
        .define_record_table(
            file.clone(),
            RuntimeLogicalModel::Relational,
            RuntimeRecordSchema::default(),
        )
        .unwrap();
    schema
        .define_relation_table(
            imports,
            RuntimeRelationSchema {
                from: BTreeSet::from([file.clone()]),
                to: BTreeSet::from([file]),
                ..RuntimeRelationSchema::default()
            },
        )
        .unwrap();
    let source = RuntimeRef::new("file", "src/lib.rs").unwrap();
    let target = RuntimeRef::new("file", "src/store.rs").unwrap();
    let commit = RuntimeCommit {
        scope: scope.clone(),
        at: 100,
        actor: "agent:semantic-commit-public".into(),
        expected_cursor: 0,
        mutations: vec![
            RuntimeMutation::Schema { registry: schema },
            RuntimeMutation::Record {
                record: RuntimeRecord {
                    reference: source.clone(),
                    valid_from: 100,
                    valid_to: None,
                    properties: RuntimeProperties::new(),
                },
            },
            RuntimeMutation::Record {
                record: RuntimeRecord {
                    reference: target.clone(),
                    valid_from: 100,
                    valid_to: None,
                    properties: RuntimeProperties::new(),
                },
            },
            RuntimeMutation::Relation {
                relation: RuntimeRelation {
                    reference: RuntimeRef::new("imports", "lib-store").unwrap(),
                    from: source,
                    to: target,
                    valid_from: 100,
                    valid_to: None,
                    properties: RuntimeProperties::new(),
                },
            },
        ],
    };
    let runtime_commit_sha256 = commit.digest();
    let function_receipt = function_receipt(&runtime_commit_sha256);
    let transaction =
        DataTransaction::new(storage.runtime().read_stamp(&scope).unwrap(), commit).unwrap();
    let outcome = storage
        .runtime()
        .commit_data_transaction_with_function_receipts(
            &transaction,
            "semantic-commit-instance",
            std::slice::from_ref(&function_receipt),
        )
        .unwrap();
    let page = storage
        .runtime()
        .changes_since(0, 16, Some(&scope))
        .unwrap();
    let snapshot = storage
        .runtime()
        .data_snapshot(&scope, 100, 256)
        .unwrap()
        .snapshot;
    let deltas = storage.runtime().projection_deltas_since(0, 16).unwrap();
    let outbox = storage.runtime().outbox_since(0, 16).unwrap();
    assert_eq!(deltas, outbox);
    assert_eq!(outcome.outbox_count, 3);
    assert_eq!(
        storage
            .runtime()
            .commit_outcome(&outcome.commit_id)
            .unwrap(),
        Some(outcome.clone())
    );
    let audit = storage
        .runtime()
        .audit(&outcome.commit_id)
        .unwrap()
        .unwrap();
    audit.validate().unwrap();
    let stored_receipt = storage
        .function_catalogue()
        .invocation_receipt("semantic-commit-instance", &function_receipt.invocation_id)
        .unwrap()
        .unwrap();
    assert_eq!(stored_receipt, function_receipt);

    PublicEvidence {
        commit_id: outcome.commit_id,
        cursor: outcome.last_cursor,
        changes: page.changes.len(),
        records: snapshot.records.len(),
        relations: snapshot.relations.len(),
        delta_cursors: deltas
            .into_iter()
            .map(|delta| delta.source_cursor)
            .collect(),
        audit_cursor: audit.outcome_cursor.unwrap(),
        function_invocation_id: stored_receipt.invocation_id,
        function_receipt_sha256: stored_receipt.receipt_sha256,
    }
}

fn function_receipt(runtime_commit_sha256: &str) -> FunctionInvocationReceiptRecord {
    let canonical_receipt_json = format!(
        "{{\"invocation_id\":\"semantic-function-one\",\"runtime_commit_sha256\":\"{runtime_commit_sha256}\"}}"
    );
    FunctionInvocationReceiptRecord {
        format_version: FUNCTION_INVOCATION_RECEIPT_FORMAT_VERSION,
        invocation_id: "semantic-function-one".into(),
        runtime_commit_sha256: Some(runtime_commit_sha256.into()),
        receipt_sha256: digest::sha256_hex(
            format!("accepted-function-receipt:{runtime_commit_sha256}").as_bytes(),
        ),
        canonical_receipt_sha256: digest::sha256_hex(canonical_receipt_json.as_bytes()),
        canonical_receipt_json,
    }
}

#[test]
fn semantic_commit_and_function_receipt_have_identical_evidence_on_mx_kv_and_kv_reopen() {
    let mx = RrflowMxStore::new();
    let mx_evidence = commit(&mx);

    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("rrflow-kv");
    let kv_evidence = {
        let kv = RrflowKvStore::open(&path).unwrap();
        commit(&kv)
    };
    assert_eq!(mx_evidence, kv_evidence);

    let reopened = RrflowKvStore::open(&path).unwrap();
    assert_eq!(reopened.runtime().cursor().unwrap(), kv_evidence.cursor);
    assert_eq!(
        reopened
            .runtime()
            .projection_deltas_since(0, 16)
            .unwrap()
            .into_iter()
            .map(|delta| delta.source_cursor)
            .collect::<Vec<_>>(),
        kv_evidence.delta_cursors
    );
    assert_eq!(
        reopened
            .runtime()
            .audit(&kv_evidence.commit_id)
            .unwrap()
            .unwrap()
            .outcome_cursor,
        Some(kv_evidence.audit_cursor)
    );
    let receipt = reopened
        .function_catalogue()
        .invocation_receipt(
            "semantic-commit-instance",
            &kv_evidence.function_invocation_id,
        )
        .unwrap()
        .unwrap();
    assert_eq!(receipt.receipt_sha256, kv_evidence.function_receipt_sha256);
    assert_eq!(receipt.runtime_commit_sha256, Some(kv_evidence.commit_id));
}

fn assert_mismatched_receipt_leaves_no_semantic_effect(storage: &dyn StorageEngine) {
    let scope = ScopeId::new("project:mismatched-function-receipt").unwrap();
    let mut schema = RuntimeSchemaRegistry::empty(1, "mismatched receipt must not commit");
    schema
        .define_record_table(
            RuntimeType::new("receipt-probe").unwrap(),
            RuntimeLogicalModel::Relational,
            RuntimeRecordSchema::default(),
        )
        .unwrap();
    let commit = RuntimeCommit {
        scope: scope.clone(),
        at: 200,
        actor: "agent:mismatched-function-receipt".into(),
        expected_cursor: 0,
        mutations: vec![RuntimeMutation::Schema { registry: schema }],
    };
    let commit_id = commit.digest();
    let transaction =
        DataTransaction::new(storage.runtime().read_stamp(&scope).unwrap(), commit).unwrap();
    let mut receipt = function_receipt(&commit_id);
    receipt.invocation_id = "mismatched-function-receipt".into();
    receipt.runtime_commit_sha256 = Some(digest::sha256_hex(b"another-runtime-commit"));
    let rejected = storage
        .runtime()
        .commit_data_transaction_with_function_receipts(
            &transaction,
            "mismatched-receipt-instance",
            std::slice::from_ref(&receipt),
        );
    assert!(matches!(rejected, Err(Error::FunctionConstraint(_))));
    assert_eq!(storage.runtime().cursor().unwrap(), 0);
    assert!(storage
        .runtime()
        .commit_outcome(&commit_id)
        .unwrap()
        .is_none());
    assert!(storage
        .function_catalogue()
        .invocation_receipt("mismatched-receipt-instance", &receipt.invocation_id)
        .unwrap()
        .is_none());
}

#[test]
fn mismatched_function_receipt_cannot_escape_a_rejected_semantic_commit() {
    assert_mismatched_receipt_leaves_no_semantic_effect(&RrflowMxStore::new());

    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("rrflow-kv-rejected-receipt");
    {
        let kv = RrflowKvStore::open(&path).unwrap();
        assert_mismatched_receipt_leaves_no_semantic_effect(&kv);
    }
    let reopened = RrflowKvStore::open(&path).unwrap();
    assert_eq!(reopened.runtime().cursor().unwrap(), 0);
    assert!(reopened
        .function_catalogue()
        .invocation_receipt("mismatched-receipt-instance", "mismatched-function-receipt")
        .unwrap()
        .is_none());
}
