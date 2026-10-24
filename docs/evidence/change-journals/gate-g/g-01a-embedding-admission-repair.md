# G-01a prerequisite ? bounded embedding admission repair

**Status:** active evidence for a bounded existing-path repair; G-01a and release qualification remain open
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journals/gate-g/g-01a-embedding-admission-repair`
**Owner:** [Gate G](../../../roadmap/rrflow-1.0/gate-g.md), OBJ-06 prerequisite and [POAM-009](../../../poam/rrflow-1.0-alpha/poam-009.md)
**Pull request:** [41](https://github.com/rrflow/rrflow-development/pull/41)

## Baseline and change brief

Started from clean fetched development/main
`0f424916bfb4468935e913ec0adf95235dc9117d`, tree
`9e070498301befa302199a794f4da40ecb961199`. The empty PR anchor
`f538ab3fdbafb0152d6fb7c6d15a0acb81163a74` opened the review description
before implementation. The official promotion repository was not a push target.

The existing engine held the embedding catalogue mutex through synchronous
backend execution. A blocked model consequently blocked model listing and
registration. Replace that coupled path with short catalogue selection and an
owned execution lease. Preserve the prior maximum of one executing batch per
engine; excess calls fail immediately instead of waiting behind backend work.
This is a repair of an existing POAM-009 defect, not an early SWAG executor,
G-01a completion or a change to D-01's active dependency order.

Stop conditions were changed authorization/model/network/shape semantics,
increased concurrency, an unbounded waiting queue, leaked admission, accepted
descriptor drift, schema or durable-format drift, and a second lifecycle owner.
No dependency, provider activation, executable fetch or runtime worker was added.

## Implementation traceability

| Previous source and behavior | Canonical replacement | Replacement evidence |
|---|---|---|
| `rrd-inference/src/lib.rs::EmbeddingBackendRegistry` owned mutable adapters and read their live descriptors; `embed_batch` executed inside its caller's lock | Private `src/registry.rs`, re-exported through the same inference crate: immutable admitted descriptors, revision, private adapter handles, one shared permit and `try_acquire` | Existing pipeline corpus plus selection/revision, contention, abandonment, error, panic and descriptor-drift tests |
| `rrd-engine/src/engine/inference.rs::generate_embeddings_at` looked up a descriptor then reacquired and held the registry for dispatch | One execution lease pins descriptor and selection revision; the registry guard ends before request copies and backend execution | Controlled blocked-backend engine regression, real authenticated listing/installation, same/other-backend busy rejection and successful post-release generation |
| Registry lock contention implicitly waited with no overload signal | `InferenceBusy` maps to existing `ResourceExhausted` and retryable status | Public engine rejection assertions; existing HTTP/WebSocket mapping consumes the same error kind |
| Backend panic could unwind through the registry guard | Backend mutex is separate; an owned permit releases during unwinding, known poison reports Internal through the engine conversion | Controlled panicking adapter, catalogue still readable, poisoned adapter denied, healthy backend still executable |

The existing `dispatch_embedding_batch` validator remains in the inference
owner. Network, exact model, source digest, batch count/bytes, output count,
finite dimensions and normalization checks remain in force. The old registry
`embed_batch` entry was removed and all repository callers were updated;
there is no compatibility/fallback dispatch lane. The standalone embedding
coordinator keeps its existing source-before/source-after and transaction CAS
characterization; it is not a new scheduling authority.

The execution lease is not cloneable, owns one atomic admission permit and
consumes itself when executing. Every error or normal return releases that
permit. A running synchronous backend retains the slot until it actually
returns or unwinds. The admitted descriptor is checked against the backend
before and after dispatch; drift cannot silently rewrite the catalogue or
produce accepted provenance. A registry installation during execution advances
the catalogue, but the in-flight result retains the selection-time revision.

## Research and resource decisions

- [Turso reliability research](https://turso.tech/blog/the-final-boss-of-reliability)
  supports testing controlled failure boundaries and invariants in addition to
  successful execution. This package uses deterministic channel/barrier
  ordering; it does not claim formal verification or a complete simulator.
- [Tokio blocking execution](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html)
  documents that already-started blocking work is not aborted by cancelling
  its task handle. Consequently this repair adds no pretend timeout or
  detached task that would release admission while work continues.
- [Rust mutex documentation](https://doc.rust-lang.org/std/sync/struct.Mutex.html)
  defines lock ownership and advisory panic poisoning. Separate catalogue and
  backend locks confine recognized poisoning; descriptor/result validation is
  retained independently. No upstream source or crate topology was copied.

One batch remains the ceiling. No queue, spawned worker, extra input copy on a
busy path, or performance claim is introduced. Existing source/read/model and
registry provenance remains the evidence vocabulary; tests observe adapter
entry and admission rather than logging request payloads. Higher concurrency,
configurable budgets, cancellation, deadlines, revocation and late output are
still the existing G-01a package and acceptance requirements.

## Files reviewed and authored

Owner/procedure review: AGENTS.md, README identity/status and relevant owner
links, OBJ-06, dependency spine, Gate G, POAM-009, architecture scale rationale,
change-authoring procedure, CI contract and independent-engine/source-adaptation
bootstrap section. No new planning authority was created.

Complete affected implementation/test files reviewed: inference `src/lib.rs`,
`src/registry.rs`, `tests/pipeline.rs`, `tests/admission.rs`; engine
`src/engine/inference.rs`, `error.rs`, `tests/native_inference.rs` and its new
`admission.rs` child. Supporting call-site inspection covered engine registry
construction in `core.rs`, catalogue access in `context.rs`, and HTTP/WebSocket
error-kind projection. Manifests were inspected; no dependencies changed.

Created: `crates/compute/rrd-inference/src/registry.rs`,
`crates/compute/rrd-inference/tests/admission.rs`,
`crates/authority/rrd-engine/src/engine/tests/native_inference/admission.rs`,
and this record. Changed: the five existing Rust owner/caller files above,
POAM-009, the scale rationale, generated gate-g journal index and generated
execution file inventory. No source file was deleted; registry definitions
moved directly to their owning crate's private module. Full-file reread and
complete-diff review cover the final source and test changes.

## Failure oracle and acceptance

The initial controlled engine oracle ran against the old implementation and
failed: `model listing was blocked by embedding execution`. The test releases
the controlled adapter before asserting, so failure does not leave a hanging
thread. The repaired oracle passes, then extends the observation to install,
list, typed busy rejection, selection revision and generation after release.
Timeouts are test hang bounds, not a latency target or model deadline.

| Command | Result |
|---|---|
| `cargo test --locked -p rrd-engine --lib blocked_embedding_does_not_lock_model_catalogue -- --nocapture` against old code | Failed as intended, one failure: catalogue blocked until backend release |
| `cargo test --locked -p rrd-engine --lib blocked_embedding_does_not_lock_model_catalogue` after repair and extension | 1 passed; authenticated catalogue/install and busy/provenance/post-release checks |
| `cargo test --locked -p rrd-inference --test admission` | 6 passed after fixture-ID correction |
| `cargo test --locked -p rrd-inference --all-targets --no-fail-fast` | 16 passed, zero failed/ignored: 6 admission and 10 preserved pipeline tests |
| `cargo test --locked -p rrd-engine --all-targets --no-fail-fast` | 178 passed, one architecture checkout-closure failure because the new test target was untracked; all 117 engine unit tests passed |
| `cargo test --locked -p rrd-engine --test workspace_architecture` after staging intended files | All 31 passed; the 179 unique engine cases are covered by the full run plus this corrected target, not 210 distinct tests |
| `cargo clippy --locked -p rrd-inference -p rrd-engine --all-targets -- -D warnings` | Passed |
| `cargo fmt --all -- --check` | Passed |
| `python scripts/ci/check_documentation.py` | Passed after status-header correction |
| `python scripts/ci/check_workflow.py` | Passed |
| `python scripts/knowledge/render_navigation.py --check` | Passed: 17 indexes, 347 nodes, 1,524 edges |
| `python scripts/ci/build_execution_inventory.py --check` | Passed: 1,282 path records |
| `python scripts/ci/test_execution_inventory.py` | 4 passed |
| `python -m unittest discover -s scripts/knowledge -p 'test_*.py'` | 17 passed |
| `git diff --cached --check` | Passed |

The journal/index and file inventory were regenerated with
`python scripts/knowledge/render_navigation.py` and
`python scripts/ci/build_execution_inventory.py`. Local logs retain initial
failures and final exits; no failure was suppressed. The wider candidate
pipeline is still required at the exact committed head before merge.

## Surfaced failures and limits

The first regression fixture had an explicit scoped-thread capture lifetime
error; its sender was moved with borrowed engine/session references. One early
shell redirection could not open the still-active prior log, so that invocation
did not run a test; subsequent subprocess wrappers record exact exit codes.
The multi-backend fixture initially reused feature-hash's stable backend ID
across seeds and correctly hit duplicate-ID rejection; controlled fixtures now
declare distinct IDs. The extended engine fixture also needed its explicit
service-error type/import. These were test construction corrections, not
weakened production checks. The first full engine run also exposed the
repository requirement that new Cargo test targets be tracked: the new
`tests/admission.rs` had not yet been staged. Staging the intended files fixes
that checkout-closure condition. The first documentation check required the
journal status to declare active/historical explicitly; its header was corrected. Initial exploratory path guesses were corrected
through repository file discovery and are not test evidence.

A blocked backend still prevents another model batch from executing. There is
no cancellation, hard interruption, deadline, configurable parallel dispatch,
revocation fence or durable reasoning lease here. Recognized Rust mutex poison
is an error signal, not complete panic containment; panic=abort/process-kill
recovery is not proven. The standalone coordinator's separate descriptor-drift
qualification is not supplied by these registry tests. No real model/provider,
accelerator, cloud service, training quality, large-codebase benchmark or SQL
compatibility was qualified. Full native/optional-feature/SDK evidence belongs
to this candidate's CI; neither green component tests nor green CI closes Gate G.

## Auditable checklist and status

- [x] Bound one existing defect, owner, baseline and stop conditions before implementation in the PR.
- [x] Map replaced registry/dispatch behavior to one canonical destination and existing/new tests.
- [x] Record primary research, no-source-adaptation decision and honest resource/trace limits.
- [x] Run the first failing behavioral oracle before replacement.
- [x] Preserve authorization, validation, provenance and one-batch execution capacity.
- [x] Record final package and repository-policy results before commit.
- [x] Regenerate navigation/inventory and verify final diff before commit.
- [x] Bind merge to the exact candidate pipeline; its completed run link is attached to the PR before merge, not predicted in this record.

POAM-009 remains open. No objective, roadmap checkbox, version or release
status changes. Candidate CI and merge evidence are recorded on PR 41 after
this immutable local evidence record is committed.
