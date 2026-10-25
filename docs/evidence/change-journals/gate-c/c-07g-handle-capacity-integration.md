# C-07g bounded handles and capacity fixture integration

**Status:** active immutable change-journal evidence
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-c/c-07g-handle-capacity-integration`
**Owner:** OBJ-02 / C-07f and C-07g / POAM-035 integrated candidate

[PR130](https://github.com/rrflow/rrflow-development/pull/130) published the
integration brief before merging implementation. Starting head was
`8a6752b255cbd9c42636c506de54de06474776f9`, tree
`d6f05e3db40122517a653eaf9a7f53c838f0e5ec`. The incoming ordinary merge is
PR131 candidate `6911f397b33b9282acad2c5be437cb811ba6740c`.

This combines the [bounded handle repair](c-07g-bounded-segment-handles.md)
and [batched capacity settlement](c-07f-batched-allocation-settlement.md).
The current engine can bound cached immutable segment handles and expose its
installed limit and counters, but its candidate hit the retained macOS fixture
timeout. The incoming fixture preserves real native exhaustion and readback
while changing only disposable allocation-barrier scheduling. Their canonical
engine and test owners remain distinct. No timeout, cap, durability rule,
configuration default or native failure oracle is weakened in integration.
Stop for a new source boundary, changed storage semantics or evidence identity.

Traceability: preserve every handle-pool/configuration/diagnostic change and
characterization test in PR130; preserve PR131's capacity.rs helper scheduling,
capacity_metadata.rs regression tests and immutable journal in the existing
installed crash owner. Both POAM-035 records and journal links survive. Only the
generated file inventory conflicted; restore its current projection, resolve
the Git index and regenerate from the merged source. No manual inventory
meaning, historical-code lane or copied engine implementation is introduced.
This journal is the only new semantic document; navigation and file inventory
are generated. The old journals remain unchanged.

Reviewed the two PR briefs and journals, full capacity.rs/metadata helper/APFS
binding, handle-pool implementation/tests, the conflict and complete merge diff.
Research and debug decisions carry their owning packages' primary-source
records; integration introduces no new algorithm or external dependency.
Capacity diagnostics and native receipts retain their bounded originals.
The initial inventory generator refused the still-unmerged Git index; staging
the resolved generated path before regeneration corrects that ordering.

Validation: `cargo test -p rrflow-cli --test installed_crash --features
crash-test --locked` passed on Windows: 24 passed, two dedicated owned-image
tests ignored, 17.43 seconds. Full command output and strict-lint output are
retained at `target/c07-handles-fixture-integrated.log`. `cargo clippy -p rrflow-cli --bin rrflow --test installed_crash --test
lifecycle_process --features crash-test --locked -- -D warnings` passed.
`cargo fmt --all -- --check`, staged/unstaged whitespace, documentation (611
statuses, 609 coordinates), workflow, navigation (17 indexes, 610 nodes,
2418 edges) and regenerated inventory checks (1737 records) passed.

PR131 run 40201872215 attempt 2 passed all three native capacity jobs,
including repeated macOS exhaustion, before this integration commit; complete
candidate acceptance was still pending on the three 100-cycle reopen jobs. No partial CI result qualifies this merged
candidate. Keep PR130 draft while the dependency is pending and require its
own complete candidate and protected merge queue checks.

The full-size Linux preflight is running under frozen revision 8a6752b2 and its
own executable/configuration/corpus/hardware identities. It remains
characterization and cannot be relabeled as qualification for this merged
source/harness. No actual 24-hour soak has started. Final installed capacity,
profile repetitions and 24-hour acceptance remain C-07 prerequisites; no alpha,
roadmap or POAM closure is claimed.

Change checklist: owning outcome and exact parents bound before effects;
replacement paths/tests mapped; generated conflict resolved mechanically;
research/trace/resource decisions retained; combined focused crash suite run;
initial projection ordering failure recorded; changed source/diff reviewed;
linked journal and generated navigation supplied; dependency and final candidate
CI remain explicit stop conditions for merge.
