# C-07 recovery and source-replan integration

**Status:** active draft evidence; C-07, D-02/D-03 and alpha remain open.
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-c/c-07-recovery-inventory-integration`
**Owner:** PR #214, OBJ-02 / C-07 / POAM-035; dependency PR #224.

Starting local head `a56eb62a6d37fa73562b7a6fa1308e06f073d08a`, tree
`d86107ed9b4b1ce20a6b051c54cf6e2cbb417ca9`; the PR brief
preceded the ordinary merge of #224
`ffe3af9f73177a0af3d594a637f78368b6b93c16`. Public main at the start was
`f256c8e1337d85fa01f2a0c61e895a4d213163fa`. The single textual conflict was
`rrd-server/src/http/response.rs`, where both sides inserted specific errors
before the generic mapping. The resolution preserves the proven-absent stale
transaction's nonretryable abort/reexecute action and the changed source's
nonretryable replan-preview action. A focused test requires their distinct HTTP
statuses, codes, reasons, actions and transaction cursor details. The existing
WebSocket stale-transaction mapping and installed HTTP replan oracle remain.
The tested staged source tree before this journal was
`37fb0117183cd2d9f857e684105428418c95c025`.

Reviewed the complete conflict owner, each incoming error mapping, the existing
stale classifier and HTTP test, selected-plan error test, PR214 and #224 briefs,
and C-07 retained evidence. The merge retains the dependency's source-bound
plan, Inventory, tests and journals, and PR214's workload, conflict analysis,
safe explicit abort/reexecute and CI policy. This record and generated index
are the integration-specific evidence paths. Full changed-file reread remains
required. No new database, hidden retry, upstream code, authorization bypass or
changed acceptance threshold is introduced.

Files reread in full for this integration include
`crates/transport/rrd-server/src/http/response.rs`,
`crates/adapters/rrflow-cli/tests/support/sustained/resource_window.rs`,
`scripts/ci/analyze_sustained_conflicts.py`,
`scripts/ci/test_analyze_sustained_conflicts.py`, and
`docs/roadmap/rrflow-1.0-execution/change-authoring.md`. Relevant portions of
the store's semantic commit, runtime repository and snapshot tests were read to
interpret the retained failure. The PR brief records the prior PostgreSQL and
SQLite isolation research. This integration adapts no upstream code; it retains
the bounded diagnostic traces and resource observations to avoid inferring
causation from client intervals. Full changed-file reread remains pending.

Independently rereading the retained first 4,000-record campaign's 12 measured
segments found 150 mixed-write attempts and one pair with overlapping client
intervals. In segment 9, rejected sequence 336 (slot 36, generation 5) and
acknowledged sequence 337 (slot 37, generation 5) overlapped by 41,657,100 ns
while the reader was held. Segment SHA-256
`0b27ad22f9db563ab79e2ef97ee06ae49d5ee2318345c1c19e69afe7e4888a20`;
the retained result SHA-256 is
`7c3998c35a3b3dbc126dc514163270d007785c88c535238473cbcb6ce14e4085`.
The result was already recorded as failed. These are client intervals, not
transaction-begin/commit ordering or proof of the error's internal variant,
winning write, reader causation or a fix. The later corrected-cadence failure
and its distinct source/trace remain recorded in PR214's earlier journals.
From the root checkout, the read-only correlation used Python's JSON parser on the ignored retained
`target/c07-semantic-4k-fa945bc7/evidence/segment-*.json` files:

```python
import json
from pathlib import Path
root = Path("target/c07-semantic-4k-fa945bc7/evidence")
segments = [json.loads(p.read_text()) for p in sorted(root.glob("segment-*.json"))]
writes = [(s["index"], a) for s in segments if s.get("phase") == "measured"
          for a in s.get("attempts", []) if a.get("kind") == "mixed_write"]
pairs = [(sa, a, sb, b) for i, (sa, a) in enumerate(writes)
         for sb, b in writes[i + 1:]
         if a["started_ns"] < b["finished_ns"] and b["started_ns"] < a["finished_ns"]]
```

The same client-interval comparison was also applied to the retained corrected-
cadence evidence in the root checkout at `target/c07-cadence-4k/evidence`.
Rejected synthetic writes `load-9-337` and `load-9-338` overlapped acknowledged
`load-9-336`; rejected `load-10-356` and `load-10-357` overlapped acknowledged
`load-10-358`. In each group, the acknowledged response finished before both
rejected responses. This still does not expose begin/commit ordering, prove the
advancing actor or distinguish the storage conflict branches.

Initial integrated checks: `python scripts/ci/test_analyze_sustained_conflicts.py`
passed eight; `python scripts/ci/test_native_sample_policy.py` passed seven;
`python scripts/ci/test_offline_lifecycle.py` passed 18; and
`python scripts/ci/check_workflow.py` and `cargo fmt --all --check` passed.
`cargo test --offline --locked -p rrd-server --lib
source_change_and_stale_transaction_keep_distinct_recovery_actions` passed one;
`cargo test --offline --locked -p rrd-server --test http_process
installed_attunement_replan_over_http_survives_restart_and_replay` passed one.
`cargo test --offline --locked -p rrd-engine --lib stale_snapshot_tests`
passed one.
`cargo test --offline --locked -p rrflow-cli --test installed_reopen --
sustained::installed_disjoint_writes_from_one_stamp_conflict_without_partial_effect
--exact --ignored --nocapture` passed one on Windows, including explicit
abort, fresh commit, readback, process restart and idempotent replay.
`cargo test --offline --locked -p rrd-server` passed its package suite,
including 21 HTTP process tests and four local estate driver tests.
`python scripts/knowledge/render_navigation.py` generated one index;
`python scripts/ci/check_documentation.py` and
`python scripts/ci/build_execution_inventory.py --check` passed. Engine
`cargo clippy --offline --locked -p rrd-server -p rrd-engine --all-targets --
-D warnings` passed. With `CARGO_NET_OFFLINE=true` and this worktree's own
target directory, `python scripts/ci/check_generated_surfaces.py --check`
passed for 70 HTTP operations and seven signal projections; OpenAPI SHA-256
`17370b53bc27e385e37eb049099846e35ece2ee22956a8ad94030100f60999c0`.
No candidate, merge-group, native capacity or soak success is inferred.

The #224 candidate must land with protected checks before this branch can be
integrated. Full review, its own candidate/merge-group CI, causal C-07 repair,
native matrix and 24-hour soak remain open. No objective, roadmap or POA&M
completion status changed.
