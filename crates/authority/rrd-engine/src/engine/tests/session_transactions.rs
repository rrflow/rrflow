use super::*;

#[test]
fn inline_session_retention_persists_a_reader_fence_and_rejects_unknown_formats() {
    let (root, engine) = isolated_engine();
    let session = engine
        .create_session(
            &session_request(9_000, 1),
            &id("fence"),
            1_000,
            "fence",
            "fence",
        )
        .unwrap();
    let key = format!(
        "server/state/{}/session/{}",
        instance(),
        session.session_id.as_str()
    );
    let before = engine.storage.control().get(&key).unwrap().unwrap();
    let mut inline: Value = serde_json::from_slice(&before).unwrap();
    // The pre-retention encoding has all records inline and no terminal marker.
    inline
        .as_object_mut()
        .unwrap()
        .remove("terminal_transaction_format");
    engine
        .storage
        .control()
        .commit(&ControlTransition {
            key: key.clone(),
            expected: Some(before),
            replacement: Some(serde_json::to_vec(&inline).unwrap()),
            at: 1_050,
            actor: "test".into(),
            action: "test.inline".into(),
            request_id: "fence".into(),
            operation_id: "fence".into(),
        })
        .unwrap();
    let context = mutation_context(&id("fence-begin"), "fence", "fence");
    let transaction = engine
        .begin_transaction(
            &session.session_id,
            &session.token,
            &begin_request(),
            &context,
            1_100,
        )
        .unwrap();
    let request = commit_request("fenced");
    let first = engine
        .commit_transaction(
            &session.session_id,
            &session.token,
            &transaction.transaction_id,
            &id("fence-commit"),
            &request,
            1_200,
            "fence",
            "fence",
        )
        .unwrap();
    let retained = engine.storage.control().get(&key).unwrap().unwrap();
    let mut state: Value = serde_json::from_slice(&retained).unwrap();
    assert_eq!(state["terminal_transaction_format"], 1);
    assert!(state["transactions"].as_object().unwrap().is_empty());
    drop(engine);
    let engine = RrdEngine::open(root.path(), instance(), TOKEN_KEY).unwrap();
    let mut replay = engine
        .commit_transaction(
            &session.session_id,
            &session.token,
            &transaction.transaction_id,
            &id("fence-commit"),
            &request,
            1_300,
            "fence",
            "fence",
        )
        .unwrap();
    assert!(replay.idempotent_replay);
    replay.idempotent_replay = false;
    assert_eq!(replay, first);
    state["terminal_transaction_format"] = 99.into();
    engine
        .storage
        .control()
        .commit(&ControlTransition {
            key,
            expected: Some(retained),
            replacement: Some(serde_json::to_vec(&state).unwrap()),
            at: 1_400,
            actor: "test".into(),
            action: "test.future-format".into(),
            request_id: "fence".into(),
            operation_id: "fence".into(),
        })
        .unwrap();
    let sequence = engine.storage.control().sequence().unwrap();
    assert!(matches!(
        engine.begin_transaction(
            &session.session_id,
            &session.token,
            &begin_request(),
            &context,
            1_500
        ),
        Err(ServiceError::Contract(_))
    ));
    assert_eq!(engine.storage.control().sequence().unwrap(), sequence);
    assert_eq!(engine.storage.runtime().cursor().unwrap(), 1);
}

fn bounded_history(engine: &RrdEngine, count: usize) -> rrd_contract::SessionLease {
    let session = engine
        .create_session(
            &session_request(9_000, 1),
            &id("history"),
            1_000,
            "history",
            "history",
        )
        .unwrap();
    for ordinal in 0..count {
        let key = id(&format!("history-{ordinal}"));
        let context = mutation_context(&key, "history", "history");
        let transaction = engine
            .begin_transaction(
                &session.session_id,
                &session.token,
                &begin_request(),
                &context,
                1_100,
            )
            .unwrap();
        let receipt = engine
            .commit_transaction(
                &session.session_id,
                &session.token,
                &transaction.transaction_id,
                &key,
                &commit_request(key.as_str()),
                1_200,
                "history",
                "history",
            )
            .unwrap();
        assert_eq!(receipt.last_runtime_cursor, Some(ordinal as u64 + 1));
        if ordinal % 32 == 31 {
            let (bytes, _) = engine
                .load_authenticated(&session.session_id, &session.token)
                .unwrap();
            assert!(
                bytes.len() <= 16 * 1024,
                "session history grew to {} bytes after {} completed transactions",
                bytes.len(),
                ordinal + 1
            );
        }
    }
    session
}

fn replay_history(engine: &RrdEngine, session: &rrd_contract::SessionLease, count: usize) {
    let sequence = engine.storage.control().sequence().unwrap();
    for ordinal in 0..count {
        let key = id(&format!("history-{ordinal}"));
        let context = mutation_context(&key, "replay", "replay");
        // Stable identities and receipts remain accessible after absolute expiry.
        let transaction = engine
            .begin_transaction(
                &session.session_id,
                &session.token,
                &begin_request(),
                &context,
                20_000,
            )
            .unwrap();
        assert_eq!(transaction.state, TransactionState::Committed);
        let receipt = engine
            .commit_transaction(
                &session.session_id,
                &session.token,
                &transaction.transaction_id,
                &key,
                &commit_request(key.as_str()),
                20_000,
                "replay",
                "replay",
            )
            .unwrap();
        assert!(receipt.idempotent_replay);
        assert_eq!(receipt.last_runtime_cursor, Some(ordinal as u64 + 1));
        assert!(matches!(
            engine.commit_transaction(
                &session.session_id,
                &session.token,
                &transaction.transaction_id,
                &id("wrong-key"),
                &commit_request(key.as_str()),
                20_000,
                "replay",
                "replay",
            ),
            Err(ServiceError::IdempotencyConflict)
        ));
    }
    assert_eq!(engine.storage.control().sequence().unwrap(), sequence);
    assert_eq!(engine.storage.runtime().cursor().unwrap(), count as u64);
    assert_eq!(engine.storage.claims().sequence().unwrap(), count as u64);
}

#[test]
fn native_session_transaction_history_stays_bounded_and_replays_after_reopen() {
    let (root, engine) = isolated_engine();
    let session = bounded_history(&engine, 768);
    drop(engine);
    let reopened = RrdEngine::open(root.path(), instance(), TOKEN_KEY).unwrap();
    replay_history(&reopened, &session, 768);
}

#[test]
fn volatile_session_transaction_history_has_the_same_bounded_replay() {
    let engine = RrdEngine::rrflow_mx(instance(), TOKEN_KEY);
    let session = bounded_history(&engine, 96);
    replay_history(&engine, &session, 96);
}

#[test]
fn terminal_retention_and_session_cas_publish_together_or_neither() {
    let (root, engine) = isolated_engine();
    let session = engine
        .create_session(
            &session_request(9_000, 1),
            &id("atomic"),
            1_000,
            "atomic",
            "atomic",
        )
        .unwrap();
    let context = mutation_context(&id("atomic-begin"), "atomic", "atomic");
    let transaction = engine
        .begin_transaction(
            &session.session_id,
            &session.token,
            &begin_request(),
            &context,
            1_100,
        )
        .unwrap();
    let (expected, mut state) = engine
        .load_authenticated(&session.session_id, &session.token)
        .unwrap();
    state
        .transactions
        .get_mut(&transaction.transaction_id)
        .unwrap()
        .lease
        .state = TransactionState::Aborted;
    let intended = state.clone();
    let key = super::super::session_transactions::terminal_transaction_key(
        &instance(),
        &session.session_id,
        &transaction.transaction_id,
    );
    let sequence = engine.storage.control().sequence().unwrap();
    let mut stale = expected.clone();
    stale.push(b' ');
    assert!(matches!(
        engine.replace_session(
            &session.session_id,
            stale,
            &mut state,
            1_200,
            "test.retention",
            "atomic",
            "atomic"
        ),
        Err(ServiceError::StorageConflict(_))
    ));
    assert_eq!(
        state, intended,
        "failed CAS must not remove the caller's record"
    );
    assert_eq!(engine.storage.control().get(&key).unwrap(), None);
    assert_eq!(
        engine
            .load_authenticated(&session.session_id, &session.token)
            .unwrap()
            .0,
        expected
    );
    assert_eq!(engine.storage.control().sequence().unwrap(), sequence);
    let replacement = engine
        .replace_session(
            &session.session_id,
            expected,
            &mut state,
            1_200,
            "test.retention",
            "atomic",
            "atomic",
        )
        .unwrap();
    assert!(state.transactions.is_empty());
    let retained = engine.storage.control().get(&key).unwrap().unwrap();
    let entries = engine.storage.control().journal_since(sequence, 3).unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].replacement.as_ref(), Some(&retained));
    assert_eq!(entries[1].replacement.as_ref(), Some(&replacement));
    assert_eq!(
        entries[1].previous_digest.as_deref(),
        Some(entries[0].digest.as_str())
    );
    assert!(entries.iter().all(|entry| entry.verify()));
    // A colliding terminal key cannot overwrite its receipt or publish a
    // different session, even if the session CAS itself is current.
    let mut collision = intended;
    assert!(matches!(
        engine.replace_session(
            &session.session_id,
            replacement.clone(),
            &mut collision,
            1_300,
            "test.collision",
            "atomic",
            "atomic"
        ),
        Err(ServiceError::StorageConflict(_))
    ));
    assert_eq!(engine.storage.control().get(&key).unwrap(), Some(retained));
    assert_eq!(engine.storage.control().sequence().unwrap(), sequence + 2);
    drop(engine);
    let engine = RrdEngine::open(root.path(), instance(), TOKEN_KEY).unwrap();
    assert_eq!(
        engine
            .load_authenticated(&session.session_id, &session.token)
            .unwrap()
            .0,
        replacement
    );
    assert_eq!(
        engine
            .begin_transaction(
                &session.session_id,
                &session.token,
                &begin_request(),
                &context,
                20_000
            )
            .unwrap()
            .state,
        TransactionState::Aborted
    );
}

#[test]
fn closing_a_full_session_keeps_one_bounded_batch_and_all_replay_identities() {
    let engine = RrdEngine::rrflow_mx(instance(), TOKEN_KEY);
    let session = engine
        .create_session(
            &session_request(9_000, 256),
            &id("bulk"),
            1_000,
            "bulk",
            "bulk",
        )
        .unwrap();
    let mut contexts = Vec::new();
    for ordinal in 0..256 {
        let context = mutation_context(&id(&format!("bulk-{ordinal}")), "bulk", "bulk");
        engine
            .begin_transaction(
                &session.session_id,
                &session.token,
                &begin_request(),
                &context,
                1_100,
            )
            .unwrap();
        contexts.push(context);
    }
    let before = engine.storage.control().sequence().unwrap();
    let closed = engine
        .close_session(
            &session.session_id,
            &session.token,
            &CloseSession {},
            &id("close"),
            1_200,
            "close",
            "close",
        )
        .unwrap();
    assert_eq!(closed.affected_open_transactions, 256);
    assert_eq!(engine.storage.control().sequence().unwrap(), before + 2);
    for context in contexts {
        assert_eq!(
            engine
                .begin_transaction(
                    &session.session_id,
                    &session.token,
                    &begin_request(),
                    &context,
                    20_000
                )
                .unwrap()
                .state,
            TransactionState::Aborted
        );
    }
    assert_eq!(engine.storage.control().sequence().unwrap(), before + 2);
}

#[test]
fn terminal_records_are_bound_to_session_identity_and_reject_invalid_state() {
    let engine = RrdEngine::rrflow_mx(instance(), TOKEN_KEY);
    let session = bounded_history(&engine, 1);
    let other = engine
        .create_session(
            &session_request(9_000, 1),
            &id("other"),
            1_000,
            "other",
            "other",
        )
        .unwrap();
    let context = mutation_context(&id("history-0"), "replay", "replay");
    let transaction = engine
        .begin_transaction(
            &session.session_id,
            &session.token,
            &begin_request(),
            &context,
            1_300,
        )
        .unwrap();
    assert!(matches!(
        engine.commit_transaction(
            &other.session_id,
            &other.token,
            &transaction.transaction_id,
            &id("history-0"),
            &commit_request("history-0"),
            1_300,
            "other",
            "other"
        ),
        Err(ServiceError::TransactionNotFound)
    ));
    assert!(matches!(
        engine.begin_transaction(
            &session.session_id,
            &other.token,
            &begin_request(),
            &context,
            1_300
        ),
        Err(ServiceError::Unauthenticated)
    ));
    let mut changed_begin = begin_request();
    changed_begin.timeout_ms += 1;
    assert!(matches!(
        engine.begin_transaction(
            &session.session_id,
            &session.token,
            &changed_begin,
            &context,
            1_300
        ),
        Err(ServiceError::IdempotencyConflict)
    ));
    assert!(matches!(
        engine.commit_transaction(
            &session.session_id,
            &session.token,
            &transaction.transaction_id,
            &id("history-0"),
            &commit_request("changed"),
            1_300,
            "other",
            "other"
        ),
        Err(ServiceError::IdempotencyConflict)
    ));
    let key = super::super::session_transactions::terminal_transaction_key(
        &instance(),
        &session.session_id,
        &transaction.transaction_id,
    );
    let original = engine.storage.control().get(&key).unwrap().unwrap();
    let mut current = original.clone();
    for field in ["format", "session", "transaction", "state"] {
        let mut invalid: Value = serde_json::from_slice(&original).unwrap();
        match field {
            "format" => invalid["format_version"] = 99.into(),
            "session" => {
                invalid["record"]["lease"]["session_id"] = other.session_id.as_str().into()
            }
            "transaction" => invalid["record"]["lease"]["transaction_id"] = "wrong-id".into(),
            "state" => invalid["record"]["lease"]["state"] = "open".into(),
            _ => unreachable!(),
        }
        let replacement = serde_json::to_vec(&invalid).unwrap();
        engine
            .storage
            .control()
            .commit(&ControlTransition {
                key: key.clone(),
                expected: Some(current),
                replacement: Some(replacement.clone()),
                at: 1_300,
                actor: "test".into(),
                action: "test.corrupt".into(),
                request_id: "test".into(),
                operation_id: "test".into(),
            })
            .unwrap();
        current = replacement;
        let sequence = engine.storage.control().sequence().unwrap();
        assert!(matches!(
            engine.begin_transaction(
                &session.session_id,
                &session.token,
                &begin_request(),
                &context,
                1_400
            ),
            Err(ServiceError::Contract(_))
        ));
        assert_eq!(engine.storage.control().sequence().unwrap(), sequence);
    }
}
