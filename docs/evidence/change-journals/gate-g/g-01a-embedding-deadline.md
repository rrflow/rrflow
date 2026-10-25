# G-01a: configured embedding deadlines

**Status:** active implementation evidence; candidate CI and full reasoning qualification pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/g-01a-embedding-deadline`
**Owner:** OBJ-06, [Gate G](../../../roadmap/rrflow-1.0/gate-g.md) and POAM-009
**Pull request:** [100](https://github.com/rrflow/rrflow-development/pull/100)

## Baseline and change brief

Branched from fetched development/main
`c148465068f675ed70068b4b0aea60725e3718dc`. Carried only the already authored
PR90 effort contract, not the unrelated C-07 implementation stack. Source
implementation began at `4531db68515cfe601511028348691aecbb40b08a`, tree
`089789ea87a7c5f3b0d2cc305d85cf1aadfdbb11`, after the draft PR brief.

Previously, one admitted synchronous embedding batch could block its engine
caller indefinitely. Now `generate_embeddings_at` consumes the sealed estate's
`reasoning.max_step_elapsed_ms`. One monotonic deadline spans input preparation,
adapter wait and output acceptance. `EmbeddingExecution::embed_batch_until`
uses an owned worker and capacity-one result channel. Timeout rejects output
without releasing the adapter's admission prematurely. A late result cannot
become a successful engine response. Expired dispatch never calls the backend.

`InferenceDeadlineExceeded` maps to the existing public DeadlineExceeded kind,
is non-retryable and carries `execution_may_continue`. Worker creation/panic
failure is explicit. The implementation never retries and does not increase
the existing single executing batch ceiling. Listing retains its existing
independence from execution.

This is a bounded caller wait and result-acceptance boundary, not forced
termination, aggregate run accounting, durable cancellation or a new scheduler.
The direct synchronous library method remains available without a wait bound;
the engine uses the deadline-aware method. No provider or UI lifecycle is added.

## Scope, research and review

Changed source: `rrd-inference/src/registry.rs` and shared input/output validation
in `src/lib.rs`; `rrd-engine/src/engine/inference.rs` and `error.rs`. Added
`rrd-inference/tests/deadline.rs` and an engine regression in
`tests/native_inference/admission.rs`. Reviewed descriptor/model/network/digest,
batch shape, job/source provenance, admission lease, panic poisoning, common
error mapping, sealed configuration, generation/search/context call sites and
existing pipeline/admission tests. The shared validation refactor preserves the
synchronous job coordinator's validation and native batching behavior; the
existing ten pipeline and six admission tests remain its replacement evidence.

Owner records read: README, AGENTS, alpha objective, Gate G, POAM-009, installed
configuration, LFG/SWAG contract, authoring routine and CI operations. Source
review covers the changed files, their exact edited symbols and full diff.
Supporting changes update the existing inference/installation references,
Gate G sequence, POAM-009, this journal and generated navigation/inventory.

The [research decisions](../../../reference/inference/lfg-swag.md#research-decisions-for-budget-enforcement)
cite primary Tokio, AWS, Temporal and LangGraph documentation. Timeout is not
proof of worker termination; retries need one total allowance; a step cap alone
cannot bound a blocked call. These behaviors map to RRFlow's existing engine
and admission owners. No upstream code or dependency was copied. Trace/debug
decision: closed typed errors, retained public outcome, no raw input logging.
Stop conditions exclude early permit release, extra workers behind a timed-out
adapter, automatic retries, late successful output and false cancellation claims.

## Validation

Windows native commands used the existing local build cache. Raw logs are
`target/g01a-*.log`; times include compilation where applicable.

| Command | Result |
|---|---|
| `cargo test --locked -p rrd-inference --test deadline` | 4 passed, 36.16 s including build |
| `cargo test --locked -p rrd-inference` | 20 passed, 2.80 s; includes all admission/deadline/pipeline tests |
| `cargo test --locked -p rrd-engine --lib configured_embedding_deadline` | First compile failed because the fixture assumed `EstateConfigurationInput::default`; corrected to an actual sealed/validated configuration |
| Same focused engine command after correction | 1 passed, 35.64 s including build |
| `cargo test --locked -p rrd-engine --lib native_inference` | 3 passed, 3.03 s |
| `cargo test --locked -p rrd-engine --lib -- --test-threads=4` | 156 passed, 2 existing ignored, 32.00 s |
| `cargo clippy --locked -p rrd-inference --all-targets -p rrd-engine --lib -- -D warnings` | Passed, 27.64 s |

The engine test seals a 250 ms step limit, blocks an installed test adapter,
observes a non-retryable deadline error before releasing that adapter, proves
another call is denied while it runs, then proves admission is recoverable.
The library tests separately prove expired admission invokes no adapter,
worker panic does not leak the slot, and the real dependency-free FeatureHash
backend returns identical vectors and provenance through both execution paths.
This is embedded engine evidence, not an installed daemon or real frontier
provider cancellation claim. Existing two ignored engine tests were not enabled.

During prerequisite cherry-pick, the generated inventory conflicted. The first
generator invocation rejected the unmerged index. The conflict markers were
then accidentally staged with the carried commit; the subsequent mechanical
regeneration removed them in a separate commit before the branch was pushed.
History was not rewritten. No conflict marker remains in the final diff.

Formatting, navigation, inventory, documentation and diff checks passed
before the implementation commit. Candidate CI must pass on the final exact
revision before merge. No deployment, promotion or merge is claimed here.

## Remaining acceptance

The [budget-control delivery sequence](../../../roadmap/rrflow-1.0/gate-g.md#budget-control-delivery-sequence)
binds the remaining code and falsifying histories: qualified cancellation and
router port; durable shared budget/reservations; no-progress and fallback
accounting; finite editable profiles, Stop and authorized extensions through
Connectome; installed real-adapter replay/crash/clock/concurrency qualification.
Those are not implemented by this per-batch deadline.

Checklist: pre-edit PR scope, owner/baseline, canonical source boundary,
research/trace decisions, validation-preserving replacement map, first oracle,
surfaced failure/correction, focused and owning suites, diff review, journal
and generated navigation are recorded. No alpha/objective/roadmap/POAM closure.
C-07, aggregate run enforcement and full Gate G remain open.
