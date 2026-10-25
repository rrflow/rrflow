use super::*;
use rrd_store::RrflowKvStore;

#[test]
fn projected_read_denial_preserves_durable_uncertainty_and_read_pressure() {
    let denial = || rrd_store::Error::ProjectedReadLimit {
        resource: rrd_store::ProjectedReadResource::OutputBufferBytes,
        limit: 8192,
        observed: 8480,
    };
    let durable = ServiceError::from(denial());
    assert_eq!(durable.kind(), crate::ServiceErrorKind::Internal);
    assert!(!durable.retryable());
    for read in [
        crate::engine::error::storage_read_error(denial()),
        crate::engine::error::query_storage_error(denial()),
        ServiceError::from(rrd_query::Error::from(denial())),
    ] {
        assert_eq!(read.kind(), crate::ServiceErrorKind::ResourceExhausted);
        assert!(!read.retryable());
        assert!(read.to_string().contains("8192"));
        assert!(read.to_string().contains("8480"));
    }
    let corruption = rrd_store::Error::Substrate("corrupt journal".into());
    assert_eq!(
        crate::engine::error::storage_read_error(corruption).kind(),
        crate::ServiceErrorKind::Internal
    );
}

#[test]
fn installed_disk_reserve_lowers_to_native_policy_and_rejection_class() {
    let policy = rrd_contract::StorageLimits {
        checkpoint_entries: 7,
        recovery_reserve_bytes: 32 * 1024 * 1024,
        native_transactions: 5,
        native_transaction_bytes: 16384,
        active_read_views: 7,
        pinned_read_bytes: 8192,
        source_page_bytes: 32768,
        ..Default::default()
    };
    let native = crate::engine::core::installed_storage_options(&policy).unwrap();
    assert_eq!(native.recovery_reserve_bytes, policy.recovery_reserve_bytes);
    assert_eq!(native.transaction_limits.active, 5);
    assert_eq!(native.transaction_limits.buffered_bytes, 16384);
    assert_eq!(native.active_read_views, 7);
    assert_eq!(native.pinned_read_bytes, 8192);
    assert_eq!(native.source_page_bytes, 32768);
    let source_error = rrd_store::Error::SourcePageLimit {
        limit: 32768,
        observed: 32769,
    };
    assert!(matches!(
        ServiceError::from(source_error),
        ServiceError::Storage(_)
    ));
    let query_error = rrd_query::Error::from(rrd_store::Error::SourcePageLimit {
        limit: 32768,
        observed: 32769,
    });
    assert!(matches!(
        ServiceError::from(query_error),
        ServiceError::StorageAdmission(_)
    ));

    assert_eq!(native.checkpoint_entries, 7);
    let checkpoint = ServiceError::from(rrd_store::Error::CheckpointLimit {
        limit: 7,
        observed: 8,
    });
    assert!(matches!(checkpoint, ServiceError::StorageAdmission(_)));
    let error = rrd_store::Error::DiskAdmission {
        available: 100,
        required: 200,
        reserve: 50,
    };
    assert!(matches!(
        ServiceError::from(error),
        ServiceError::StorageAdmission(_)
    ));
}

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
    rejection_history(None, None, 4096, 4096, "write batch encoded_bytes");
}

#[test]
fn mutable_byte_rejection_preserves_domain_and_clears_only_absent_intent() {
    rejection_history(
        Some(rrd_store::MutableLimits {
            accounted_bytes: 128 * 1024,
            versions: 8192,
        }),
        None,
        256,
        1024,
        "mutable accounted bytes",
    );
}

#[test]
fn mutable_version_rejection_preserves_domain_and_clears_only_absent_intent() {
    rejection_history(
        Some(rrd_store::MutableLimits {
            accounted_bytes: 8 * 1024 * 1024,
            versions: 256,
        }),
        None,
        256,
        16,
        "mutable versions",
    );
}

#[test]
fn native_buffer_rejection_preserves_domain_and_resolves_absent_intent() {
    rejection_history(
        None,
        Some(rrd_store::TransactionLimits {
            active: 64,
            buffered_bytes: 128 * 1024,
        }),
        256,
        1024,
        "transaction owned buffer bytes",
    );
}

#[test]
fn exhausted_native_owners_do_not_reclassify_an_accepted_commit_as_rejected() {
    let root = tempfile::tempdir().unwrap();
    let engine = configured_engine(&root.path().join("engine"), None, None, true);
    let lease = engine
        .create_session(
            &session_request(5_000, 1),
            &id("owner-session"),
            1_000,
            "owner-session",
            "owner-session",
        )
        .unwrap();
    let transaction = engine
        .begin_transaction(
            &lease.session_id,
            &lease.token,
            &begin_request(),
            &mutation_context(&id("owner-begin"), "owner-begin", "owner-begin"),
            1_100,
        )
        .unwrap();
    let request = commit_request("accepted-before-pressure");
    let commit = |at| {
        engine.commit_transaction(
            &lease.session_id,
            &lease.token,
            &transaction.transaction_id,
            &id("owner-commit"),
            &request,
            at,
            "owner-commit",
            "owner-commit",
        )
    };
    let accepted = commit(1_200).unwrap();
    let held = (0..64)
        .map(|_| engine.storage.begin_transaction().unwrap())
        .collect::<Vec<_>>();
    let error = commit(1_300).unwrap_err();
    assert!(
        matches!(error, ServiceError::Storage(_)),
        "unresolved replay reported rejection: {error}"
    );
    drop(held);
    let mut replay = commit(1_400).unwrap();
    assert!(replay.idempotent_replay);
    replay.idempotent_replay = false;
    assert_eq!(replay, accepted);
    assert_eq!(engine.storage.claims().sequence().unwrap(), 1);
}

fn configured_engine(
    root: &std::path::Path,
    limits: Option<rrd_store::MutableLimits>,
    native: Option<rrd_store::TransactionLimits>,
    create: bool,
) -> RrdEngine {
    if limits.is_none() && native.is_none() {
        return RrdEngine::open(root, instance(), TOKEN_KEY).unwrap();
    }
    let options = rrd_store::DatabaseOptions {
        mutable_limits: limits.unwrap_or_default(),
        transaction_limits: native.unwrap_or_default(),
        ..Default::default()
    };
    let storage = if create {
        RrflowKvStore::create_new_with_options(root, options).unwrap()
    } else {
        RrflowKvStore::open_existing_with_options(root, options).unwrap()
    };
    let operation_parent = storage.anchored_path().parent();
    RrdEngine::compose_persistent(
        operation_parent,
        instance(),
        TOKEN_KEY,
        storage,
        create,
        Default::default(),
    )
    .unwrap()
}

fn rejection_history(
    limits: Option<rrd_store::MutableLimits>,
    native: Option<rrd_store::TransactionLimits>,
    count: usize,
    object_bytes: usize,
    expected_error: &str,
) {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("engine");
    let engine = configured_engine(&path, limits, native, true);
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
    let mutations = (0..count)
        .map(|index| TransactionMutation::AssertClaim {
            subject: CanonicalId::new(format!("oversized-{index}")).unwrap(),
            predicate: CanonicalId::new("contains").unwrap(),
            object: "x".repeat(object_bytes),
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
    let (_, rejected_state) = engine
        .load_authenticated(&lease.session_id, &lease.token)
        .unwrap();
    let intent_retained = rejected_state.transactions[&rejected.transaction_id]
        .commit_intent
        .is_some();
    assert_eq!(
        error.kind(),
        crate::ServiceErrorKind::ResourceExhausted,
        "known batch admission is not an uncertain internal error: {error}; intent_retained={intent_retained}"
    );
    assert!(!error.retryable());
    assert!(
        error.to_string().contains(expected_error),
        "wrong admission resource: {error}"
    );
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
    let reopened = configured_engine(&path, limits, native, false);
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
    let reopened = configured_engine(&path, limits, native, false);
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

#[test]
fn segment_handle_policy_and_pressure_preserve_durable_outcome_classification() {
    let mut policy = rrd_contract::StorageLimits::default();
    assert_eq!(
        crate::engine::core::installed_storage_options(&policy)
            .unwrap()
            .segment_io
            .max_open_files,
        128
    );
    let sealed = serde_json::to_value(&policy).unwrap();
    assert!(sealed.get("segment_open_files").is_none());
    assert_eq!(
        serde_json::from_value::<rrd_contract::StorageLimits>(sealed).unwrap(),
        policy
    );
    policy.segment_open_files = Some(3);
    assert_eq!(
        crate::engine::core::installed_storage_options(&policy)
            .unwrap()
            .segment_io
            .max_open_files,
        3
    );
    for invalid in [0, 4097] {
        policy.segment_open_files = Some(invalid);
        assert!(crate::engine::core::installed_storage_options(&policy).is_err());
    }
    let denial = || rrd_store::Error::SegmentHandleLimit {
        limit: 3,
        observed: 4,
    };
    assert_eq!(
        ServiceError::from(denial()).kind(),
        crate::ServiceErrorKind::Internal
    );
    for read in [
        crate::engine::error::storage_read_error(denial()),
        crate::engine::error::query_storage_error(denial()),
        ServiceError::from(rrd_query::Error::from(denial())),
    ] {
        assert_eq!(read.kind(), crate::ServiceErrorKind::ResourceExhausted);
    }
}
