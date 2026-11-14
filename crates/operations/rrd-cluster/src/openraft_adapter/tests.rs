use super::*;

/// Callers build a peer's NodeId with `.expect` once this has passed, so a
/// bad id must stop here as a configuration error and not panic later.
#[test]
fn a_raft_node_id_outside_the_canonical_grammar_is_refused() {
    let node = |canonical_id: &str| RrdRaftNode {
        canonical_id: canonical_id.into(),
        zone: "zone-a".into(),
        endpoint: "127.0.0.1:1".into(),
    };
    node("node-a").validate().unwrap();
    assert!(node("node:a").validate().is_err());
}

#[test]
fn request_history_retains_exactly_the_declared_log_window() {
    let mut state = StateMachineData::default();
    for index in 1..=RRFLOW_RAFT_REQUEST_RETENTION_LOGS {
        state.requests.insert(
            format!("request-{index}"),
            AppliedRequest {
                first_applied_index: index,
                command_digest: format!("digest-{index}"),
                response: RrdRaftResponse {
                    accepted: true,
                    duplicate: false,
                    term: 1,
                    index,
                    state_digest: "a".repeat(64),
                    reason: "retention fixture".into(),
                    runtime_outcome: None,
                },
            },
        );
        prune_request_history(&mut state, index);
    }
    assert_eq!(
        state.requests.len() as u64,
        RRFLOW_RAFT_REQUEST_RETENTION_LOGS
    );
    assert!(state.requests.contains_key("request-1"));

    let next = RRFLOW_RAFT_REQUEST_RETENTION_LOGS + 1;
    state.requests.insert(
        format!("request-{next}"),
        AppliedRequest {
            first_applied_index: next,
            command_digest: format!("digest-{next}"),
            response: RrdRaftResponse {
                accepted: false,
                duplicate: false,
                term: 1,
                index: next,
                state_digest: "b".repeat(64),
                reason: "retention boundary".into(),
                runtime_outcome: None,
            },
        },
    );
    prune_request_history(&mut state, next);
    assert_eq!(
        state.requests.len() as u64,
        RRFLOW_RAFT_REQUEST_RETENTION_LOGS
    );
    assert!(!state.requests.contains_key("request-1"));
    assert!(state.requests.contains_key("request-2"));
    assert!(state.requests.contains_key(&format!("request-{next}")));

    let reopened: StateMachineData =
        serde_json::from_slice(&serde_json::to_vec(&state).unwrap()).unwrap();
    assert_eq!(reopened.requests, state.requests);
}
