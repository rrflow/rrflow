# C-07: verify the installed database and relationship flow

**Status:** active implementation evidence; C-07 qualification remains open
**Coordinate:** `rrflow://rrflow-instance/data/evidence/c-07-integrated-readback`
**Owner:** OBJ-02 / C-07 public-client readback prerequisite / POAM-035 / PR127

## Baseline, brief and traceability

Public main `94d0567fb865fb49a90aaeeef4b5861707c9f8a7` is the starting
revision. Empty opening commit `8351b983eb8952318c41f2e1f62ba86257f3d3bd`
has baseline tree `eacaa14078fd881e9dfd46aa32543a54971a78d5` and
enabled [PR127](https://github.com/rrflow/rrflow-development/pull/127)'s scoped
brief before implementation. It creates no planning authority. PR126 is the
integration prerequisite; its frozen capacity candidate and receipts remain
unchanged and are not rebound to this example change.

The existing example combined independent task and relationship HEAD queries.
An intervening commit could make its output describe two committed views.
It also printed results without checking an independently expected model.
The replacement stays in `rrd-client/examples/support/rrflowdb.rs`: capture
the task read's known cursor, query the relationship at that cursor and the
same valid time, and reject incomplete, foreign-scope or mismatched stamps.
`verify open|done` checks the exact example task and owning relationship.
The existing public authentication, session, bounded query, transaction,
rollback, temporal and reopen behavior stays at its canonical boundary.

Traceability: `examples/rrflowdb.rs` dispatch and independent HEAD output,
`support/rrflowdb.rs::{tasks,relationships}` and the real-server regression
map to `DatabaseView`, `read_view`, fixed-cursor `relationships`, and verifier
assertions in the same files. No engine source is replaced, moved or deleted.
`read` keeps its task/relationship/cloud fields and strengthens consistency;
verification adds `verified_expected_status` only after all checks pass.

Scope additionally corrects the existing scale rationale's stale next-cut
paragraph, refreshes its primary research mapping, updates the local SDK
guide and links the demonstrated gap from POAM-035. No storage, provider,
UI, configuration, public protocol, dependency or product-version change.
Stop if this requires engine semantics or another authority; preserve the
running qualification rather than changing its source.

## Research and review

Full implementation/reference reads: the example, shared support, complete
`rrflowdb_local.rs`, `rrd-client/Cargo.toml`, local rrflowDB guide, scale and
contract rationale, change-authoring routine and PR template. Owner sections
reviewed: README/AGENTS, alpha objective, dependency spine, C-07 implementation
and execution reset, Gate J profile matrix, D-01/H-05a acceptance, CI procedure
and POAM-035. QueryResult/QueryExecutionSnapshot and syntax tests were read at
the changed type/grammar boundary. These sectional reads are not a claim to
have audited every engine subsystem.

The [scale rationale](../../../architecture/engine-scale-and-contracts.md#graph-vector-and-knowledge-research-mapped-to-rrflow)
records the 2026-10-24 primary-source review: FoundationDB fault testing,
kernel cgroup-v2 enforcement, Qdrant hybrid retrieval, Hydra hosted-memory
research, Pinecone Nexus and SurrealDB transactions. It maps each technique
and failure oracle to existing C/D/E/F/G/J owners. Vendor scores are not RRFlow
results; the hosted Hydra report and open-source graph implementation remain
distinct. No source code was adapted or dependency upgraded.

Trace/resource decision: preserve the existing bounded public query evidence
and operation identities; add no runtime logging or telemetry. Validation
does not allocate an unbounded result, loosen budgets or retry effects. The
local example is a small integration demonstration, not a capacity benchmark.

## Commands, failures and evidence

- `cargo test --locked -p rrd-client --test rrflowdb_local`: 2 passed,
  0 failed; initial build 72 seconds and tests 0.74 seconds. This includes the
  real installed Rust engine/server, public client, native store and reopen.
- `cargo test --locked -p rrd-client --all-targets`: 10 passed, 0 failed,
  0 ignored across the client targets; command 30.437 seconds.
- `cargo clippy --locked -p rrd-client --all-targets -- -D warnings`: passed,
  80.344 seconds, no warnings.
- `cargo build --locked -p rrflow-cli --bin rrflow -p rrd-client --example rrflowdb`:
  passed, 69.562 seconds. This is an unqualified Windows x86_64 debug build
  from the modified worktree, not a signed release or the Linux soak binary.
- `cargo fmt --all -- --check` and `git diff --check`: passed.
- `python scripts/knowledge/render_navigation.py` and
  `python scripts/ci/build_execution_inventory.py`: regenerated the nearest
  journal index and mechanical file plan. Both `--check` modes passed.
- `python scripts/ci/check_documentation.py`: passed ownership, statuses,
  coordinates, parent indexes and local links.

### Native executable flow

The operator script at `target/run_c07_native_readback.py` ran the documented
public CLI/example commands against an isolated fresh project. Evidence is
retained under `target/c07-integrated-readback-evidence/native-json/`:

1. `rrflow version --json`, `install identity --json`, `install plan --json`
   and exact-digest `install apply --json` succeeded.
2. `serve` bound loopback; authenticated `ready --json` succeeded.
3. The example's `init`, `verify open`, `update` and `verify done` succeeded.
4. `verify open` after update exited **1**, as required; it did not print a
   successful result. `abort` and `verify done` then succeeded with unchanged
   task rows and known cursor.
5. The owned process was terminated. Offline `verify --level quick --json`
   succeeded. A new process reopened the same estate and passed authenticated
   readiness and `verify done`.
6. Independent script assertions compared task and relationship rows, cursor,
   schema revision and read-manifest digests before and after restart. All
   matched. The task changed at cursor **6 -> 7** and reopened at **7**.

All 14 expected-success commands exited zero; the one negative command exited
one. Both owned server processes were stopped, and project state retained.
No provider, cloud account, user project or running capacity estate was used.
The private installed credential is not committed or included in this journal.

| Retained identity | SHA-256 |
|---|---|
| Native `rrflow.exe` | `bb37dd8e34a8e87d79c9b37b75c0161912127b7b4367311679ec04d1f8336ddd` |
| Native `rrflowdb.exe` example | `2af4801dd0ca0a949a06e34d887dd9fc806a562083acc181df065b5531b43d14` |
| Native `receipt.json` | `9d73204eddaf2b00aeb82a8e0790af99ecfb9bb3e6e578a6fd907f45c9c8fdb7` |
| Native command/output-digest receipt | `38139d0a256397c084b6c5f3665ee719436cf4c9e5476d3b2e35c3ce7bd3ad5e` |
| Client suite/lint/build command receipt | `cf1973725469772b4e420213132eb6412b72aff83b67d90da19f427207d4261d` |

The fixed-cursor oracle reads a task, commits a new version, then reads the
relationship at the earlier cursor and verifies the earlier model. Combining
the old task with the new HEAD relationship stamp is rejected. Negative
cases cover wrong expected state, foreign scope, truncated task/relationship
output, mismatched cursor/schema, missing/extra rows, wrong relationship
endpoint and inconsistent row count. The same test verifies state after
rejected multi-mutation writes, abort, successful update and disk reopen.

No Rust test failure surfaced in this package. The first native operator
attempt parsed human-readable `version` output as JSON and failed before
installation. Its output remains under the separate `native/` directory;
the corrected attempt explicitly used `--json` and a new estate. Exploratory shell
lookups used one nonexistent query directory and a Windows-incompatible path
glob; corrected lookups found `crates/compute/rrd-query`. One GitHub jq command
lost shell quotes and was replaced by PowerShell JSON selection. These were
inspection failures, not engine results. No prior failed capacity evidence
was deleted or relabeled.

## Completion checklist and limits

- Brief-first PR, one bounded owner, explicit dependency and isolated worktree.
- Existing behavior and characterization mapped to the same client boundary.
- Primary research, trace/resource decisions and stop conditions recorded.
- Focused then owning-package tests, native example proof, strict lint and
  generated checks passed. Changed source/reference files and the complete
  diff were reviewed against the full baseline reads; generated inventory
  changes contain only mechanical path/line-count projections.
- Generated journal navigation and execution inventory regenerated before commit.
- Roadmap/objective gate status: **no change**. POAM-035 gains the demonstrated
  example gap and its repair link; C-07 remains open.

This package does not prove ANN recall, complete reasoning execution,
PostgreSQL compatibility, cloud synchronization or Connectome behavior. The
full capacity matrix, native retained-lifetime acceptance and actual 24-hour
soak remain C-07 requirements. PR126 candidate CI and long-run results stay
under their original identity; PR127 needs its own candidate pipeline after
dependency integration. No merge, signed release or completed alpha is claimed.
The full workspace and native Linux/macOS example runs were not repeated for
this client-only package; its public-server tests and full candidate pipeline
remain the applicable cross-platform checks. No 24-hour result is claimed.
