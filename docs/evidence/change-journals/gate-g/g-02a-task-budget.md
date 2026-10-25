# G-02a prerequisite: durable task budgets for embedded inference

**Status:** active implementation evidence; installed qualification and candidate CI pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/g-02a-task-budget`
**Owner:** OBJ-06, D-06/G-02a and POAM-009
**Pull request:** [102](https://github.com/rrflow/rrflow-development/pull/102)

## Scope and baseline

Starting revision `920d9c96cb06e4607cb2ba98760d9fe5c754ec1c`, tree
`9064f8ae497e796bac6aabe4795a8b187d28af77`, combines fetched development/main
with PR101 prerequisite `dfe3cba2696e49b3376f905835ccdfc9f7c03a69`. Their
content diff is empty. The existing 157-pass engine/24-pass inference evidence
is recorded in the prerequisite journal; it was not rerun as a separate
pre-edit baseline. The pre-edit draft PR names the source map, first oracle,
scope, research, acceptance and stop conditions. No prerequisite CI is bypassed.

Existing embedding calls had a step deadline and process-local cancellation,
but no aggregate task allowance. The new embedded `generate_task_embeddings`
operation binds the existing task to one immutable estate configuration,
original deadline and finite step limit. It requires ReasoningWrite and
EmbeddingGenerate; inspection requires ReasoningRead. Ordinary standalone
embedding and task open/close semantics remain unchanged.

Reservation and completion use existing engine control state and its journal,
paired atomically with a task-subject runtime evidence event through
`commit_with_control_transitions`. The checked runtime cursor fences task
closure at reservation. The two control point reads do not scan attempt or
task history; state is bounded and each attempt has its own stable key. Both
control payloads count toward existing transaction admission. No key family,
storage authority, scheduler, provider loop or transport operation is added.

The reserved step is never refunded. A repeated identity cannot re-execute;
changed input under the same identity conflicts. A completed identity exposes
its result digest; full output is not cached. Interrupted reservation or failed
completion publication retains uncertainty. Further attempts cannot bypass it.
Three bounded publication attempts may retry transaction conflicts, never
provider execution. Completion must be durable before success returns; output
crossing the inherited deadline is withheld. Cancellation, failure, expiry,
rollback, configuration change and exhaustion cannot reset the allowance.

## Traceability, research and review

Reviewed README/AGENTS, OBJ-06, Gate G, POAM-009, LFG/SWAG budget reference,
change-authoring and implementation-navigation procedures. Relevant source
review includes task open/close/identity reads and commit/schema helpers,
inference preparation/dispatch/output validation, clock source and assessment,
control transition helpers and storage CAS, runtime/control atomic composition,
transaction admission, engine errors/exports/test composition and existing
native inference cancellation tests. The task schema and store implementation
are reused without rewriting them. Reasoning-tree contracts remain separate
inventory; this package does not claim to execute a tree.

Created `engine/reasoning_budget.rs`, its `persistence.rs` child and
`engine/tests/reasoning_budget.rs` under `crates/authority/rrd-engine/src`.
Changed engine `inference.rs`, `error.rs`, `mod.rs`, `tests.rs` and crate
`lib.rs`; added the narrowing-only helper and regression to
`crates/compute/rrd-inference/src/execution_control.rs` and
`tests/cancellation.rs`. Existing inference code remains the one executor;
the helper preserves cancellation and cannot extend its parent's deadline.
Changed the existing LFG/SWAG, Gate G and POAM009 owners; generated navigation
and inventory link this journal. No implementation was removed or moved.

Primary sources: Rust's [clock distinction](https://doc.rust-lang.org/std/time/)
and [Instant](https://doc.rust-lang.org/stable/std/time/struct.Instant.html),
and Temporal's [attempt versus aggregate timeouts](https://docs.temporal.io/encyclopedia/detecting-activity-failures).
The design inference is a persisted original wall deadline plus narrower
monotonic child ceilings and charged execution time. It does not import
Temporal code or introduce a dependency. Host time remains unverified; any
observed rollback stops this budget, irrespective of tolerance on unrelated
estate operations. No trusted cross-host or suspension-time guarantee is made.

Trace/resource decisions: public task linkage, configuration/request/result
digests, counts, terminal reasons and outstanding-work flags; no prompt or
private thought logging. Point reads and finite attempt counts bound state
access; no throughput, memory plateau or model-quality claim is made. Existing
single-worker admission remains intact during a timed-out non-cooperative call.

## Validation and surfaced failures

Commands run with `CARGO_TARGET_DIR` set to the existing local build cache;
timings include compilation where applicable. Logs are under the workspace
`target/g02a-budget-*.log` and are not committed as product assets.

| Command / history | Result |
|---|---|
| Initial focused `cargo test --locked -p rrd-engine --lib reasoning_budget -- --nocapture` | Compile failed: CorrelationId needs `as_str()` for actor formatting; scoped test closures needed owned captures. Fixed directly. |
| Focused retry | 6 passed, 1 failed: the test installed a second FeatureHash backend with the same catalogue ID. Assigned the controlled backend its own exact descriptor ID; production admission was unchanged. |
| Focused corrected history | 7 passed, including real computation/reopen, last-step contention, unresolved reservation, clock failure and blocked-provider deadline. |
| Expanded focused history | 9 passed, 22.12 s including build; added running cancellation and closed-task/scope denial. |
| `cargo test --locked -p rrd-engine --lib reasoning_budget` | 10 passed, 19.52 s including build. |
| `cargo test --locked -p rrd-engine --lib -- --test-threads=4` | 167 passed, 2 existing ignored, 31.28 s. |
| `cargo test --locked -p rrd-inference` | 25 passed, 5.92 s. |
| `cargo clippy --locked -p rrd-engine --lib --tests -p rrd-inference --all-targets -- -D warnings` | Initial failure: one needless reference in the blocked test. Removed it; corrected strict lint passed, 5.16 s. No production change or suppressed lint. |
| `cargo fmt -p rrd-engine -p rrd-inference -- --check` | Passed. |
| `python scripts/knowledge/render_navigation.py`, `python scripts/ci/build_execution_inventory.py`, `python -X utf8 scripts/ci/check_documentation.py`, `git diff --check` | Passed; generated journal index and inventory updated. |

The real computation test performs two successful FeatureHash calls across
rrflowKV close/reopen, verifies the original deadline/count, rejects a third
attempt and checks result-digest replay/input conflict without another call.
Other histories distinguish pre-dispatch interruption from completion-clock
failure after actual computation. The two-worker test permits exactly one
backend call. Blocked-provider histories retain its worker permit after timeout
or cancellation and persist the uncertainty rather than claiming termination.
The narrowing helper test prevents deadline extension and cancellation loss.

## Remaining boundaries and checklist

Native component reopen is not an installed process-kill campaign. Installed
crashes before dispatch/after response/after commit, real LLM cancellation and
clock manipulation/suspension remain unqualified. The two pre-existing ignored
engine tests remain outside this evidence. No 24-hour soak, C-07 capacity,
real-provider performance, SDK/daemon/UI or full-workspace qualification was
run for this bounded package. Candidate CI is still required.

There is no complete governed reasoning runner, router/context/fallback budget
integration, per-run preset resolution, token/cost accounting, no-progress
controller, durable Stop operation, authorized extension, full output replay
or uncertainty reconciliation. An execution failure conservatively stops the
task budget. Starting a separate authorized task is distinct work, not proof of
a global estate spending limit. Ordinary standalone embedding remains governed
by its per-call ceiling. No Gate G, C-07, alpha or POAM closure is claimed.

Checklist: pre-edit PR, exact baseline/source mapping, first oracle, research,
trace/resource decisions, implementation in the existing authority, surfaced
failures, focused/owning validation, final review, linked journal and generated
projections are recorded. Changed source files and the complete diff were
reread; formatting, documentation, navigation, inventory and diff checks passed.
Merge requires the exact candidate pipeline; PR100/101 prerequisites are not
reported as merged by this journal.
