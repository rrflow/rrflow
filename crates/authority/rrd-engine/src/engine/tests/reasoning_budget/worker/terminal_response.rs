//! Controlled wire faults exercise durable engine state, not model quality.
use super::*;

#[test]
fn rejected_terminal_response_releases_capacity_but_lost_response_does_not() {
    for lost in [false, true] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let (root, engine, lease, selection) =
            admitted_fixture(&format!("http://{}/v1", listener.local_addr().unwrap()));
        let identity = engine.installed_engram.clone();
        let request = selected_request(&engine, &lease, &listener, &selection, 1_000, false);
        let before = budget(&engine, &lease, &request.task_id);
        let result = std::thread::scope(|scope| {
            let server = scope.spawn(|| {
                let (mut socket, body) = read_wire(&listener);
                if lost {
                    return;
                }
                let wire: serde_json::Value = serde_json::from_slice(&body).unwrap();
                let bytes = serde_json::to_vec(&serde_json::json!({
                    "model": wire["model"],
                    "choices": [{"finish_reason": "length", "message": {
                        "role": "assistant", "content": "Incomplete protocol fixture"
                    }}]
                }))
                .unwrap();
                write!(socket, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", bytes.len()).unwrap();
                socket.write_all(&bytes).unwrap();
            });
            let result = engine.execute_task_worker_with_clock(
                &lease.session_id,
                &lease.token,
                &request,
                "terminal-worker",
                "terminal-worker",
                &FixedClock::at(1_100),
            );
            server.join().unwrap();
            result
        });
        assert!(
            matches!(
                result,
                Err(ServiceError::EndpointCallFailed { execution_may_continue, .. })
                    if execution_may_continue == lost
            ),
            "lost={lost}: {result:?}"
        );
        let after = budget(&engine, &lease, &request.task_id);
        assert_eq!(after.reserved_steps, 2);
        assert_eq!(after.deadline_unix_ms, before.deadline_unix_ms);
        assert!(after.charged_elapsed_ms >= before.charged_elapsed_ms);
        assert_eq!(after.stopped, Some(ReasoningBudgetStop::AttemptFailed));
        assert_eq!(after.execution_may_continue, lost);
        assert!(after.active_attempt.is_none());
        let key = format!(
            "server/state/{}/endpoint-proposal/capacity",
            engine.instance
        );
        let capacity = engine.storage.control().get(&key).unwrap().unwrap();
        let state: serde_json::Value = serde_json::from_slice(&capacity).unwrap();
        assert_eq!(
            state
                .get("workers")
                .and_then(|v| v.as_object())
                .map_or(0, |v| v.len()),
            usize::from(lost)
        );
        let read = ReadTaskAttempt {
            scope: request.scope.clone(),
            task_id: request.task_id.clone(),
            attempt_id: request.attempt_id.clone(),
        };
        let retained = engine
            .read_task_attempt(
                &lease.session_id,
                &lease.token,
                &read,
                1_200,
                "read",
                "read",
            )
            .unwrap();
        let attempt = retained.attempt.as_ref().unwrap();
        assert_eq!(attempt.status, TaskAttemptStatus::Stopped);
        assert!(attempt.worker_output.is_none() && attempt.outcome_sha256.is_none());
        drop(engine);
        let mut engine = RrdEngine::open_existing(root.path(), instance(), TOKEN_KEY).unwrap();
        engine.installed_engram = identity;
        configure(&mut engine, 8);
        assert_eq!(budget(&engine, &lease, &request.task_id), after);
        assert_eq!(
            engine.storage.control().get(&key).unwrap().unwrap(),
            capacity
        );
        assert_eq!(
            engine
                .read_task_attempt(
                    &lease.session_id,
                    &lease.token,
                    &read,
                    1_300,
                    "reopen",
                    "reopen"
                )
                .unwrap(),
            retained
        );
        let resume = rrd_contract::ResumeTaskBudget {
            scope: request.scope.clone(),
            task_id: request.task_id.clone(),
            expected_budget_sha256: operation_digest(&after).unwrap(),
        };
        let resumed = engine.resume_task_budget_with_clock(
            &lease.session_id,
            &lease.token,
            &resume,
            "resume",
            "resume",
            &FixedClock::at(1_400),
        );
        if lost {
            assert!(matches!(
                resumed,
                Err(ServiceError::ReasoningBudgetStopped(
                    ReasoningBudgetStop::UnresolvedAttempt
                ))
            ));
            assert_eq!(budget(&engine, &lease, &request.task_id), after);
        } else {
            let mut expected = after;
            expected.stopped = None;
            expected.last_observed_at_unix_ms = 1_400;
            assert_eq!(resumed.unwrap().budget, Some(expected));
            assert!(matches!(
                engine.execute_task_worker_with_clock(
                    &lease.session_id,
                    &lease.token,
                    &request,
                    "old-worker",
                    "old-worker",
                    &FixedClock::at(1_500),
                ),
                Err(ServiceError::ReasoningAttemptRecorded { .. })
            ));
        }
        assert_eq!(
            engine.storage.control().get(&key).unwrap().unwrap(),
            capacity
        );
    }
}
