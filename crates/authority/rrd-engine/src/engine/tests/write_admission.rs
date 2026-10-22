use super::*;

fn domain_evidence(engine: &RrdEngine, prior_commit: &str, rejected_commit: &str) -> Value {
    let storage = &engine.storage;
    let sequence = storage.claims().sequence().unwrap();
    let scope = rrd_core::ScopeId::new(format!("instance:{}", instance())).unwrap();
    let read = storage.runtime().read_stamp(&scope).unwrap();
    let versions = storage
        .runtime()
        .read_versioned(
            &read,
            &[rrd_store::RuntimeVersionedSource::Claims { predicate: None }],
            rrd_store::RuntimeReadBudget::new(65_536).unwrap(),
        )
        .unwrap();
    serde_json::json!({
        "claim_sequence": sequence,
        "claims": storage.claims().claims_in_range(0, sequence).unwrap(),
        "read": read,
        "versions": versions.changes,
        "deltas": storage.runtime().projection_deltas_since(0, 8192).unwrap(),
        "outbox": storage.runtime().outbox_since(0, 8192).unwrap(),
        "prior_audit": storage.runtime().audit(prior_commit).unwrap(),
        "prior_outcome": storage.runtime().commit_outcome(prior_commit).unwrap(),
        "rejected_audit": storage.runtime().audit(rejected_commit).unwrap(),
        "rejected_outcome": storage.runtime().commit_outcome(rejected_commit).unwrap(),
    })
}

#[test]
fn oversized_semantic_commit_is_rejected_abortable_and_reopens_without_effects() {
    let (root, engine) = isolated_engine();
    let lease = engine
        .create_session(
            &session_request(5_000, 1),
            &id("admission-session"),
            1_000,
            "admission-session",
            "admission-session",
        )
        .unwrap();
    let begin = |engine: &RrdEngine, name: &str, at: u64| {
        engine
            .begin_transaction(
                &lease.session_id,
                &lease.token,
                &begin_request(),
                &mutation_context(&id(name), name, name),
                at,
            )
            .unwrap()
    };
    let prior = begin(&engine, "prior", 1_100);
    let prior_request = commit_request("prior-acknowledged");
    let prior_receipt = engine
        .commit_transaction(
            &lease.session_id,
            &lease.token,
            &prior.transaction_id,
            &id("prior-commit"),
            &prior_request,
            1_200,
            "prior-commit",
            "prior-commit",
        )
        .unwrap();
    let rejected = begin(&engine, "oversized", 1_300);
    let mutations = (0..4096)
        .map(|index| TransactionMutation::AssertClaim {
            subject: CanonicalId::new(format!("oversized-{index}")).unwrap(),
            predicate: CanonicalId::new("contains").unwrap(),
            object: "x".repeat(4096),
            valid_from: 1_000,
            tx_time: 1_000,
            producer: CanonicalId::new("admission-oracle").unwrap(),
            confidence: None,
        })
        .collect::<Vec<_>>();
    let request = CommitTransaction {
        operation_sha256: rrd_contract::transaction_operation_sha256(&mutations),
        mutations,
    };
    request.validate().unwrap();
    let before = engine.storage.runtime().cursor().unwrap();
    let prior_commit =
        public_runtime_commit(&prior_request, &lease.session_id, &instance(), 0, 1_200)
            .unwrap()
            .digest();
    let rejected_commit =
        public_runtime_commit(&request, &lease.session_id, &instance(), before, 1_400)
            .unwrap()
            .digest();
    let evidence_before = domain_evidence(&engine, &prior_commit, &rejected_commit);
    assert_eq!(evidence_before["claim_sequence"], 1);
    assert_eq!(evidence_before["claims"][0]["object"], "prior-acknowledged");
    assert!(!evidence_before["prior_outcome"].is_null());
    assert!(!evidence_before["prior_audit"].is_null());
    assert!(evidence_before["rejected_outcome"].is_null());
    assert!(evidence_before["rejected_audit"].is_null());
    let error = engine
        .commit_transaction(
            &lease.session_id,
            &lease.token,
            &rejected.transaction_id,
            &id("oversized-commit"),
            &request,
            1_400,
            "oversized-commit",
            "oversized-commit",
        )
        .unwrap_err();
    assert_eq!(
        error.kind(),
        crate::ServiceErrorKind::ResourceExhausted,
        "known batch admission is not an uncertain internal error: {error}"
    );
    assert!(!error.retryable());
    assert_eq!(engine.storage.runtime().cursor().unwrap(), before);
    assert_eq!(
        domain_evidence(&engine, &prior_commit, &rejected_commit),
        evidence_before
    );
    let (_, state) = engine
        .load_authenticated(&lease.session_id, &lease.token)
        .unwrap();
    assert!(state.transactions[&rejected.transaction_id]
        .commit_intent
        .is_none());
    drop(engine);
    let reopened = RrdEngine::open(root.path(), instance(), TOKEN_KEY).unwrap();
    assert_eq!(reopened.storage.runtime().cursor().unwrap(), before);
    assert_eq!(
        domain_evidence(&reopened, &prior_commit, &rejected_commit),
        evidence_before
    );
    let replay = reopened
        .commit_transaction(
            &lease.session_id,
            &lease.token,
            &prior.transaction_id,
            &id("prior-commit"),
            &prior_request,
            1_500,
            "prior-replay",
            "prior-replay",
        )
        .unwrap();
    let mut stable = replay.clone();
    stable.idempotent_replay = false;
    assert_eq!(stable, prior_receipt);
    assert!(replay.idempotent_replay);
    let aborted = reopened
        .abort_transaction(
            &lease.session_id,
            &lease.token,
            &rejected.transaction_id,
            &AbortTransaction {},
            &mutation_context(&id("abort-rejected"), "abort-rejected", "abort-rejected"),
            1_600,
        )
        .unwrap();
    assert_eq!(aborted.state, TransactionState::Aborted);
    let next = begin(&reopened, "valid-after-rejection", 1_700);
    // Use a distinct claim identity: the shared helper deliberately names the
    // same subject/predicate/valid-time/transaction-time on every call.
    let mut next_mutation = mutation("valid-after-rejection");
    if let TransactionMutation::AssertClaim { subject, .. } = &mut next_mutation {
        *subject = CanonicalId::new("document-after-rejection").unwrap();
    }
    let next_mutations = vec![next_mutation];
    let next_request = CommitTransaction {
        operation_sha256: rrd_contract::transaction_operation_sha256(&next_mutations),
        mutations: next_mutations,
    };
    let receipt = reopened
        .commit_transaction(
            &lease.session_id,
            &lease.token,
            &next.transaction_id,
            &id("valid-commit"),
            &next_request,
            1_800,
            "valid-commit",
            "valid-commit",
        )
        .unwrap();
    assert!(!receipt.idempotent_replay);
    assert_eq!(reopened.storage.runtime().cursor().unwrap(), before + 1);
    drop(reopened);
    let reopened = RrdEngine::open(root.path(), instance(), TOKEN_KEY).unwrap();
    let claims = reopened.storage.claims().claims_in_range(0, 8192).unwrap();
    assert_eq!(claims.len(), 2);
    assert_eq!(claims[0].object, "prior-acknowledged");
    assert_eq!(claims[1].object, "valid-after-rejection");
    assert_eq!(reopened.storage.runtime().cursor().unwrap(), before + 1);
    assert!(reopened
        .storage
        .runtime()
        .commit_outcome(&rejected_commit)
        .unwrap()
        .is_none());
    assert!(reopened
        .storage
        .runtime()
        .audit(&rejected_commit)
        .unwrap()
        .is_none());
}
