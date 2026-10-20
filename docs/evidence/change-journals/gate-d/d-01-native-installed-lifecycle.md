# D-01 native installed lifecycle qualification

**Status:** active bounded qualification evidence
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-d/d-01-native-installed-lifecycle`
**Owner:** OBJ-07, D-01, POAM-006

Starting revision: `b6ec9a6f9842d0741bd5bee39929067852e25235`.
The worktree was created from public development main; unrelated worktrees
remain unchanged. The local PR brief preceded edits. GitHub needs a nonempty
diff, so the test/evidence commit opens the PR before any production fix.
This package changes qualification code and CI, not the engine runtime.

## Gap, scope and replacement evidence

The baseline `cargo test --locked -p rrflow-cli --test installed_lifecycle --
--list` listed only three argument tests on Windows and emitted ten unused
import/helper warnings. The process case was `cfg(unix)` and used
`Cargo.toml` as a surrogate executable identity. Platform CI did not invoke
the primary installed lifecycle test.

`crates/adapters/rrflow-cli/tests/installed_lifecycle.rs` now tests fresh and
existing projects on native platforms, binds the actual executable without
the debug identity override, and restarts a served engine for authenticated
query readback after termination and after restore publication. The existing
no-write/deterministic preview, wrong-digest rejection, exact/idempotent
apply, configured budgets, discovery, backup, writer exclusion, quick verify
and secret-output checks remain. Original project files are compared after
the lifecycle. The child guard captures output concurrently, bounds startup
to 60 seconds and reaps only owned children even after assertion failures.
Unix retains bounded graceful SIGINT; Windows terminates its exact child
handle after acknowledged operations. This does not prove graceful Windows
shutdown or every crash boundary.

`.github/workflows/ci-reusable.yml` adds the primary test and strict focused
Clippy to Windows/macOS lanes. `scripts/ci/check_workflow.py` requires the
primary lifecycle in both Linux topology and platform lanes and requires
platform warning denial. `docs/operations/ci.md` describes those exact
assertions; POAM-006 records the coverage gap without closing the broader
item. The nearest journal index and execution inventory are regenerated.

## Inspection and engineering decisions

Read: README and AGENTS, OBJ-07 and the objective status table, complete
dependency spine, Gate D, POAM-006/-009/-010, ADR-0002, installed-lifecycle
product boundary, change-authoring and implementation-navigation procedures,
the prior D-01 walking-product receipt, complete installed lifecycle test,
CLI argument/serve composition, platform/topology workflow and relevant
workflow validator. Exploratory router/inference contracts confirmed the
dependency order; no model adapter was added ahead of its prerequisites.
Initially guessed source paths were absent and were resolved with the
repository file list before editing.

No upstream adaptation, external research, performance claim or new runtime
trace is needed. Existing engine commits, query results and control receipts
are the runtime oracle; harness cleanup is not product lifecycle authority.
Stop on unrelated runtime scope, weakened checks, changed executable binding,
lost project files or failed candidate CI. No source module, public API,
storage format, product version, objective or roadmap status changes.

## Executed evidence

Windows with pinned Rust 1.99.1 and shared
`CARGO_TARGET_DIR=D:/localdev/devlabs/rrflow/target`.
The first filtered run after editing reused the old cached test binary and
ran zero tests with the baseline warnings. That is not a pass. After
`cargo clean -p rrflow-cli`, the same focused command
`cargo test --locked -p rrflow-cli --test installed_lifecycle
installed_engine_plans_applies_serves_discovers_authenticates_and_reopens --
--nocapture` rebuilt and passed one real process test, four filtered out,
in 152.40 seconds. It completed commit, termination, restarted readback,
backup publication and restored readback without warnings.

Workflow policy passed. An in-memory adversarial check removed each newly
required platform command independently; policy rejected both missing-test
and missing-warning-denial cases, then accepted the unchanged workflow.
No workflow file was mutated by that adversarial check.

`cargo test --locked -p rrflow-cli` passed all 13 tests across eight result
groups, with zero failures, ignored tests or warnings. The installed target
ran five tests, including fresh and existing processes, in 150.98 seconds.
Its raw local log is `target/rrflow-native-lifecycle-suite.log`.
`cargo clippy --locked -p rrflow-cli --test installed_lifecycle -- -D warnings`
passed. Navigation passed with 322 nodes/1394 edges, inventory generation
with 1236 records and documentation policy with 323 statuses/321 coordinates.
The complete test diff, CI commands, policy and supporting records were
reviewed; the existing process assertions have stronger replacement evidence.

Final candidate CI/merge evidence is attached to the package PR. This receipt is
not model-routing, durable reasoning, PostgreSQL, cloud synchronization,
clean-machine release or complete D-01/C-07 qualification. All such owning
gates remain open. Scope, baseline, traceability, oracles, failures, limits
and stop decisions form this package's auditable change checklist.
