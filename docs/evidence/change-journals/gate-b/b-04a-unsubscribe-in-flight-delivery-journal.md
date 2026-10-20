# B-04a unsubscribe in-flight delivery journal

**Status:** active immutable change-journal evidence
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-b/b-04a-unsubscribe-in-flight-delivery`
**Owner:** B-04a WebSocket unsubscribe ordering package receipt; not runtime or roadmap authority

This record reports a correction to the B-04 multiplexed WebSocket evidence.
The real-server Rust client test assumed that the frame after an `unsubscribe`
is always `unsubscribed`. The server's delivery poll can send the next page
first, so the candidate engine suite failed intermittently. The
[subscription reference](../../../reference/protocol/subscriptions.md) now
states the ordering, and the test accepts the in-flight page while asserting
that it continues the acknowledged stream.

## Evidence record

```text
gate/package and alpha outcome or prerequisite advanced: B-04 follow-up (B-04a) — the multiplexed WebSocket subscription evidence for OBJ-09 (provider-neutral client operation). The candidate engine suite (service-protocol) must not fail on a legal frame order.
starting revision/tree and pull request: main at 13ce3386248e87e9d3be1e94969ec8f408eb7317 (tree 7028e2ea75daa5a9a1c28633205918237bedf484); branch agent/ws-unsubscribe-in-flight-delivery; the pull request in rrflow/rrflow-development that carries this journal.
change brief (current -> target behavior, owner, exact scope, unchanged behavior, stop conditions): current — crates/transport/rrd-client/tests/real_server.rs::rust_client_negotiates_authenticates_queries_and_reads_audit acknowledges cursor 2 while later commits exist, sends unsubscribe, and panics unless the very next frame is unsubscribed; the server's socket loop can run a delivery poll and send the page for cursor 2..3 before it reads the unsubscribe, so the test fails on a legal order. Target — the test waits for unsubscribed while accepting at most max_in_flight (1) delivery for that subscription, which must carry the reconnected generation, a later delivery sequence, and from_cursor 2, and it checks that unsubscribed reports acknowledged cursor 2; docs/reference/protocol/subscriptions.md states that a delivery sent before the server reads an unsubscribe can precede unsubscribed and that none follows it. Unchanged: every server, client, contract, and SDK behavior, wire shape, and digest. Stop if the server can send a delivery for a subscription after unsubscribed, or if more than max_in_flight deliveries can precede it.
files and symbols read in full: AGENTS.md; docs/reference/protocol/subscriptions.md; crates/transport/rrd-server/src/http/websocket.rs (socket select loop, poll_subscriptions, the Unsubscribe handler, active_matches); crates/transport/rrd-client/tests/real_server.rs (the subscription, reconnect, replay, acknowledgement, and unsubscribe sequence). Read in part: rrd-contract lib.rs (SubscriptionResume and max_in_flight bounds); docs/operations/ci.md; the failing candidate job log.
implementation/evidence paths created, changed, moved, or deleted: changed crates/transport/rrd-client/tests/real_server.rs and docs/reference/protocol/subscriptions.md; created this journal; regenerated docs/evidence/change-journals/gate-b/README.md and docs/roadmap/rrflow-1.0-file-plan.jsonl. Nothing moved or deleted.
research decision and primary-source/adaptation record: no external source. The ordering follows from RRFlow's own server: one tokio::select! loop handles inbound frames and the delivery poll tick in sequence; poll_subscriptions sends a Delivery for a due subscription; the Unsubscribe handler removes the subscription from the active map before it sends Unsubscribed, so later polls never select it; outstanding deliveries are bounded by the subscription's max_in_flight through SubscriptionBackpressure.
trace/resource/debug decision (add/preserve/not applicable, with reason): not applicable — no runtime behavior changes; the test's existing frame-level assertions carry the diagnosis.
first-failure or characterization oracle and result: candidate run 39347696550 (pull request #19, head 39139a0bb6c556c752a3de5ea4fb8ebaf17aa562) failed in pipeline / engine suite (service-protocol) at real_server.rs:852 with "unexpected unsubscribe payload: Delivery(... connection_generation: 2, delivery_sequence: 4, from_cursor: 2, through_cursor: 3 ... head_cursor: 4, has_more: true ...)"; a re-run of the failed jobs on the same commit passed. Characterization on this host (2 cores): the unmodified test, run 60 times six at a time with two busy-loop CPU burners, failed 27 of 60 times on main's tree (all at the unsubscribe assertion, real_server.rs:850; 8 also logged the proxy task's secondary BrokenPipe at line 451) and 21 of 60 times on pull request #19's merged tree; run alone 40 times it passed 40 times.
acceptance commands and exact results: cargo fmt --all -- --check: clean. cargo test --locked -p rrd-client --test real_server: 3 passed. The same contention run as the characterization, 90 times with this change: 0 failed. cargo clippy --locked -p rrd-client --all-targets -- -D warnings: clean. cargo test --locked -p rrd-client --no-fail-fast: every target passed. python3 scripts/ci/check_documentation.py, scripts/knowledge/render_navigation.py --check, scripts/ci/build_execution_inventory.py --check, scripts/ci/check_workflow.py, and scripts/ci/test_execution_inventory.py: OK.
surfaced failures and corrections: the first wording of the assertion claimed at most one unacknowledged delivery in general; the server bounds it by each subscription's max_in_flight, so the test and the reference now say that, and the test's own subscription uses 1.
checks not run and reason: the remaining workspace suites, SDK conformance, installed lifecycle, and macOS and Windows jobs — no code they exercise changed; candidate CI runs them on the pull request.
remaining known errors and owner: delivery is still polled rather than woken by each commit (H-03 owns commit-impact delivery); no other ordering assumption about unsubscribe exists in the repository's tests.
full-file reread and diff review: complete for every created and changed authored file before commit.
change checklist: complete — owner chain, protocol reference, test oracle, journal index, and execution inventory reconciled.
roadmap/POA&M status change, or explicit none: none. The deficiency is a test oracle that rejected a legal frame order; this package closes it, so no open gap enters the POA&M.
pull request, CI run, and merge evidence: the pull request carries the candidate CI runs and the merge; this record does not restate them.
```

## Links

- [Gate B](../../../roadmap/rrflow-1.0/gate-b.md)
- [B-04 evidence journal](b-04-evidence-journal.md)
- [Durable subscriptions](../../../reference/protocol/subscriptions.md)
