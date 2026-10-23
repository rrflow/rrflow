# G-01a: cooperative embedding cancellation

**Status:** active implementation evidence; candidate CI and durable cancellation pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/g-01a-cooperative-cancellation`
**Owner:** OBJ-06, [Gate G](../../../roadmap/rrflow-1.0/gate-g.md) and POAM-009
**Pull request:** [101](https://github.com/rrflow/rrflow-development/pull/101)

## Scope and baseline

Starting revision `e4fabca17587c6a3f4d6f3e8dd2fa8e3f51e6150`, tree
`03bbb69f7a430f879b406fa87a81d7a4ee0088bb`, combines fetched development/main
with exact PR100 prerequisite `dd436ce9e0ab52fdd326ec64346515b89de12998`.
The pre-edit draft PR binds one current execution-boundary repair. PR100's
candidate jobs remained queued; neither prerequisite nor candidate gate is
bypassed by local test results.

Previously the configured deadline bounded waiting but supplied no stop check
to adapter code. `EmbeddingCancellation` now provides a one-way shared signal;
`EmbeddingExecutionControl` binds it to an immutable monotonic deadline.
`EmbeddingExecution` passes that control into the adapter, checks it before
dispatch and acceptance, and waits in at most ten-millisecond blocking quanta
to observe cancellation. Scheduling can add latency. The actual worker keeps
the existing permit until it returns, including non-cooperative work.

`EmbeddingBackend::embed_batch_controlled` defaults to pre/post checks around
the existing native batch method. FeatureHash additionally checks during token
processing/hashing, between inputs and around normalization. Individual bounded
string/vector operations are not preempted. Cancellation produces no successful
late result, no automatic retry and no replacement worker behind active work.
The immutable control is process-local, not a new durable lifecycle owner.

The engine's embedded `generate_embeddings_with_cancellation` entry point shares
ordinary generation's authorization, read stamp, configured ceiling, validation
and implementation. It maps cancellation to a non-retryable closed error with
an explicit outstanding-work flag. No wire operation, schema, descriptor claim
of hard termination, daemon Stop route or Connectome control is added.

## Traceability and research

Read README, AGENTS, alpha objective, Gate G, POAM-009, authoring procedure,
existing deadline journal/contract and CI state. Reviewed the complete current
registry, adapter trait/dispatch, FeatureHash implementation, deadline and
admission tests, engine authorization/generation/error path, and optional
FastEmbed native batch implementation. Reviewed every changed block and diff.

Created `rrd-inference/src/execution_control.rs` and `tests/cancellation.rs`.
Changed `src/registry.rs`, `src/lib.rs`, engine `inference.rs`, `error.rs` and
`tests/native_inference/admission.rs`. Existing deadline checks converge onto
the shared control; normal generation delegates to the same authorized path.
FeatureHash's private computation accepts a checkpoint callback so the actual
algorithm can be interrupted deterministically in a test without production
global hooks. The deadline regression is retained in a shared fixture with
cancellation; both original deadline and new cancellation assertions pass.

The [primary-source research](../../../reference/inference/lfg-swag.md#research-decisions-for-budget-enforcement)
from PR100 remains applicable: caller timeout is not provider termination.
No new upstream code, runtime or dependency is introduced. Debug decisions:
closed outcomes, no input logging, actual worker ownership and no retry loop.
Stop conditions exclude premature permit release, accepted late output,
increased execution capacity, a second authority or false hard-stop claims.
Supporting changes are the owning references, POAM-009, this linked record and
generated navigation/inventory. No storage or release changes.

## Validation

Windows native logs are `target/g01a-cancel*.log` and
`target/g01a-cancellation-*.log`. Times include builds/cache waits where present.

| Command | Result |
|---|---|
| `cargo test --locked -p rrd-inference --test cancellation` | 3 passed, 1.27 s |
| `cargo test --locked -p rrd-inference --lib cooperative_feature_hash` | 1 passed, 13.22 s |
| `cargo test --locked -p rrd-inference` after adding the real-kernel test | 24 passed, 5.11 s |
| `cargo test --locked -p rrd-engine --lib native_inference` | 4 passed, 68.31 s including build |
| `cargo test --locked -p rrd-engine --lib -- --test-threads=4` | 157 passed, 2 existing ignored, 52.69 s |
| `cargo check --locked -p rrd-inference --all-targets --features fastembed-local` | Passed, 30.70 s; compile evidence only, no ONNX model execution |
| `cargo clippy --locked -p rrd-inference --all-targets -p rrd-engine --lib -- -D warnings` | Initial failure: test module preceded production items. Moved the unchanged test module to the file end; corrected strict lint passed, 7.86 s |

The running-work history distinguishes cooperative and non-cooperative adapters,
observes cancellation before releasing blocked work, verifies retained capacity
when required and eventual admission recovery. The actual FeatureHash kernel
test cancels at its twentieth checkpoint, proves no output and no later
checkpoint, then reuses the model successfully. Ordinary controlled output
equals the existing algorithm. The engine test retains authentication and the
sealed ceiling and observes a non-retryable cancellation while work is blocked.

The first patch attempt was rejected for targeting a new file twice; it made
no changes and was reapplied as valid separate patches. All test assertions
remain intact. The test-module-only lint relocation required no repeated full
runtime suite; strict lint recompiled the test target. Formatting, navigation,
inventory, documentation and diff checks passed before commit.

## Remaining work and checklist

The two existing ignored engine tests remain ignored. Installed daemon Stop,
durable cancellation/reservations, router execution, run-wide effort/cost/attempt
limits, no-progress tracking, UI controls and real-provider cancellation are
unqualified. Process-local cancellation does not survive restart. FastEmbed
retains its blocking-call limitation. No hard real-time stop bound is claimed.

Checklist: pre-edit PR, owner/baseline, canonical source replacement map,
research/trace choices, first oracle, affected and owning tests, failure and
correction, review, linked journal and generated projections are recorded.
No roadmap, objective or POAM closure. Candidate CI must pass at the final
revision before merge; neither PR100 nor PR101 is reported as merged here.
