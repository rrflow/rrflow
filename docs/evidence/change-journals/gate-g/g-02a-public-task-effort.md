# G-02a public task effort, durable Stop and result replay

**Status:** active implementation evidence; local component validation complete, candidate CI pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/g-02a-public-task-effort`
**Owner:** OBJ-06/OBJ-09, D-06/G-02a/H-06 prerequisite; POAM-009 remains open
**Pull request:** [103](https://github.com/rrflow/rrflow-development/pull/103)

## Brief, baseline and traceability

Starting revision `842731beb1a9c648f87dbb6dae3b5fe17e623646`, tree
`c26c631048c80ff0d401c7609ba5ee8ce1f8ef78`, combines fetched development/main
with exact PR102 `b586cbca31ae593ab313f791697a29368036fe5f`. The draft PR
description recorded the scope, source map, oracles and stop conditions before
implementation. The original worktree and unrelated DevPulse processes were
preserved. Prerequisite and candidate CI remain required before merge.

Previously, task embedding effort was embedded-only, always adopted estate
caps, returned only replay digests, and had no public durable Stop. This package
exposes one authenticated flow through the existing operation catalogue,
HTTP dispatcher and Rust SDK: explicitly activate the bundled CPU reference
adapter; configure immutable task limits at or below estate caps; execute a
bounded attempt; read budget/output; Stop using the observed budget digest;
reopen and replay without redispatch. Configuration starts the original
deadline immediately. Repeating configuration cannot reset it, even after Stop.

The canonical budget/stop types move from the engine module to
`rrd-contract/src/reasoning_budget.rs` and remain re-exported from the existing
engine API. The existing task identity, control CAS, runtime evidence event and
shared inference executor retain authority. Attempt records now retain bounded
optional output; prior digest-only records do not fabricate it. Completion
reloads control state before its bounded CAS retries so a concurrent Stop
cannot be overwritten. A process-local cancellation map delivers signals;
attempt-specific guard removal prevents an older completion from unregistering
a newer attempt. No source implementation or dependency is deleted, replaced
by an upstream crate, moved outside this repository or hidden behind a fallback.

The existing budget flow/concurrency/reopen corpus characterizes the moved
types and execution owner. The new controlled in-flight Stop history and real
authenticated socket/reopen history are the replacement evidence for exposing
and extending this boundary. Ordinary standalone embedding remains per-call;
task lifecycle, router/tree semantics and project-owned data are not replaced.

## Source and change review

Owner review: README, AGENTS, alpha outcomes, Gate G, POAM-009, the execution
map/change-authoring procedure, LFG/SWAG effort contract, Connectome boundary,
public protocol and Rust SDK reference. Full source review of the new contract,
public budget implementation, server handler and new test files, and the
existing budget/reservation/completion/persistence implementation, accompanies
the changed-symbol review below. Generated projections were reviewed through
their canonical export and parity checks rather than authored independently.

| Boundary | Source reviewed and implementation changes |
|---|---|
| Engine | `engine/reasoning_budget.rs`, `reasoning_budget/persistence.rs`, new `reasoning_budget/public.rs`, `engine/tests/reasoning_budget.rs`; `core.rs` both constructors and cancellation composition; `inference.rs` shared executor and backend projection. Existing task admission, clock/control helpers, security and atomic runtime/control commit were traced. |
| Contract | New `src/reasoning_budget.rs` and `tests/task_effort_contract.rs`; `src/inference.rs` response validation; `src/lib.rs` module export, six catalogue rows, envelope schema mapping and reviewed OpenAPI digest; `tests/public_contract.rs` explicit count goldens. Existing capability projection derives the new public rows automatically and keeps Connectome planned. |
| Transport | New `rrd-server/src/http/handlers/reasoning_budget.rs`, handler module and `http/router.rs`; existing envelope authentication/invocation and task handlers were traced. `rrd-client/src/operation.rs` adds six typed methods; new `tests/task_effort.rs` follows real socket/security/reopen fixtures. |
| Projections | Existing TypeScript/Python/Go/Java/.NET generators produce endpoint maps, TypeScript wire schema and signal projections. Reviewed count expectations in five SDK test files and `fixtures/rrd-sdk-conformance-v1.json` advance 42 to 48. Protocol/SDK digest citations and Rust operation coverage advance with them. |
| Planning/evidence | Existing Gate G delivery sequence, LFG/SWAG implementation reference, Connectome handoff and POAM-009 progress; this journal and generated nearest index/execution inventory. No new planning authority or Connectome pin change. |

## Research, observability and resource decisions

Primary-source review revisited [Temporal aggregate versus attempt timeouts](https://docs.temporal.io/encyclopedia/detecting-activity-failures)
and [Tokio blocking task termination limits](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html).
The design inference is to preserve the original durable deadline, narrow
child execution, and retain uncertainty/admission while non-cooperative work
continues. No external framework code or runtime service is imported.

The existing engine invocation audit and runtime budget event provide task,
attempt, configuration/result digests, usage and stop observations. No raw
prompt or private reasoning transcript is added. Public requests are capped at
16 inputs/64 KiB; output admission is 4096 values and retained task output is
at most 512 KiB. Existing storage transaction limits still apply. These are
admission bounds, not measured heap/latency/throughput or model-quality claims.
The bundled FeatureHash backend is explicitly a deterministic CPU reference
adapter, not an LLM. Activation is explicit and process-local; replay does not
activate or invoke it after reopen.

## Validation and surfaced failures

Rust commands used `CARGO_TARGET_DIR` at the existing local build cache.
Workspace `target/g02b-*.log` files contain local command output and are not
runtime assets. Durations include build or build-lock waits where applicable.

| Command / history | Actual result |
|---|---|
| `cargo check --locked -p rrd-engine -p rrd-contract -p rrd-client -p rrd-server` | Passed; removed the reported unused import. |
| `cargo test --locked -p rrd-engine public_budget_is_immutable -- --nocapture` | 1 focused test passed, 117.05 s. The invocation also compiled unrelated engine integration targets but ran no filtered tests in them. |
| `cargo test --locked -p rrd-client --test task_effort -- --nocapture` | Real authenticated socket flow passed, 161.78 s including compilation; test itself 0.35 s. |
| Initial `cargo test --locked -p rrd-contract -p rrd-client -p rrd-server` | Failed on three intentional public-contract goldens: operation count, shared corpus count and OpenAPI digest. Updated the exact reviewed expectations, preserving assertions. |
| Corrected same owning command | 146 tests passed across 30 reported targets, no failures/ignored, 60.97 s. Includes tightened exact permission-denied assertions in the new socket test. |
| Final `cargo test --locked -p rrd-contract` after activation-bound correction | 108 passed across 18 targets, 32.47 s. |
| Final `cargo clippy --locked -p rrd-contract --all-targets -- -D warnings` | Passed, 21.05 s. |
| Initial `cargo test --locked -p rrd-engine --lib -- --test-threads=4` | 167 passed, 1 shared-corpus expectation failed, 2 existing ignored. The same fixture count correction resolves the failure. |
| Corrected same engine command | 168 passed, 2 existing ignored, 59.97 s. |
| `cargo clippy --locked -p rrd-engine -p rrd-contract -p rrd-client -p rrd-server --lib --tests -- -D warnings` | Passed, 35.70 s. |
| TypeScript `lint`, `typecheck`, `test` | Lint/typecheck passed. Initial test exposed the old 42-operation assertion; corrected run passed all 6 tests. |
| Go `go test ./...` | Passed; generator/conformance command packages have no unit tests. |
| Existing five language generators and `python scripts/ci/check_generated_surfaces.py --write` | Passed; 48 HTTP operations and OpenAPI `28fdf404064be9b507ed2bc2780242ce15237eabe86cee5a7297af4470d4da3c`. Signal catalogue content digest remains unchanged. |
| `cargo fmt -p rrd-engine -p rrd-contract -p rrd-client -p rrd-server -- --check`, `git diff --check`, `python scripts/ci/check_generated_surfaces.py` | Passed after restoring generated line endings. |
| Initial documentation validation | Rejected a journal status without the required active/historical prefix and a missing Gate C dependency link in POAM-009. Both documentation declarations corrected. |
| `python scripts/knowledge/render_navigation.py`, `python scripts/ci/build_execution_inventory.py`, `python -X utf8 scripts/ci/check_documentation.py` | Corrected run passed: 17 indexes, 1465 inventory records, 438 document statuses and 436 classified coordinates. |

The in-flight test configures narrower limits, denies mutation/estate-cap
excess and stale Stop, blocks actual provider execution, durably stops it,
receives cancellation before releasing the non-cooperative provider, preserves
uncertainty, and reopens the same stopped budget/attempt. The real HTTP test
checks explicit activation, real FeatureHash computation, retained output,
non-redispatch of the same attempt, exact missing-grant denial (including the
separate EmbeddingGenerate grant), foreign scope denial, Stop replay and reopen
without any backend activation. Existing budget tests still prove final-step
contention, clock failure, exhaustion, configuration changes and unresolved work.

Tooling issues were not hidden: the host pnpm launcher referred to a missing
path; invoking the installed locked pnpm 12.14.0 module directly completed the
frozen install without downloading packages. Python's locked project command
could not find Python 3.15; the unchanged generator ran with the host Python
using `uv run --no-project --with ruff==0.17.1 python sdks/python/scripts/generate.py`.
This is generation evidence, not qualification of the Python 3.15 SDK runtime.
Generated Python/Java/.NET CRLF output caused a diff-whitespace check failure;
line endings were restored to the committed convention without changing
generated content or the generator authority.

Final source review found that the public activation validator permitted
dimensions 1 through 7 although FeatureHash requires at least 8. Restricted
the contract to 8..=1024 and added invalid/boundary cases. This changes input
validation, not the exported wire shape or the previously tested valid flow.

## Remaining boundaries and checklist

Full Python 3.15/Java 25/.NET 10.0.402 runtime suites were not run: the required
Python interpreter and Maven were unavailable, and the installed .NET SDK is
10.0.401. Candidate CI owns those pinned lanes. No all-language shared daemon
campaign, real LLM/router, Connectome UI, installed process-kill campaign,
C-07 disk/capacity qualification, 24-hour soak or full-workspace suite was run.
The two existing ignored engine tests remain excluded from this evidence.

Router/context/fallback aggregate admission, no-progress tracking, preset
resolution, token/cost accounting, authorized extension, reconciliation of
uncertain work and Connectome wiring remain open under Gate G/D-06/H-06.
Publication cannot forcibly interrupt storage; a delivery deadline can pass
after an accepted receipt commits. Host time remains unverified. New authorized
tasks are separate work, not proof of a global spending cap. This package is
not a complete reasoning runner, C-07 closure or alpha release.

Checklist: pre-edit PR and baseline, source/behavior traceability, smallest
oracles, bounded implementation, primary-source rationale, resource/trace
decisions, surfaced failures and scoped acceptance are recorded above.
Changed source/diff review and final formatting, generated-surface,
documentation, navigation, inventory and diff checks are recorded above. No roadmap checkbox,
POA&M closure, product version or official promotion changes. PR103's exact
candidate pipeline must pass before merge; local passes cannot replace it.
