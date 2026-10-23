# G-03 prerequisite: configurable repeated task inference

**Status:** active implementation evidence; candidate CI and full runner qualification pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/g-03-task-repetition`
**Owner:** OBJ-06, D-06/G-03 prerequisite; POAM-009 remains open
**Pull request:** [104](https://github.com/rrflow/rrflow-development/pull/104)

## Brief, baseline and source map

Baseline `44ec25d8c9e572de55698da05322ae747591c16e`, tree
`a27a14852d0e2272bbfa162407ad4a2e1c7a35ad`, is fetched development/main plus
exact PR103 `1084ff97108292f1419293c8b2d0ed656abd1ca8`. The draft PR carried
the change brief, source map, failure oracle, research/resource decisions and
stop conditions before implementation. Existing passing task-budget evidence
is in the predecessor journal. Candidate CI and prerequisite gates remain
required; the original worktree and unrelated processes are untouched.

Previously, task attempts shared finite time/step limits but could resubmit
identical embedding batches under different attempt/input labels until those
limits expired. The optional immutable `TaskRepetitionPolicy` now limits
equivalent attempts across the whole task. Reservation records counts in the
existing `TaskReasoningBudget` and its atomic control/runtime event transaction.
No separate loop, scheduler, cache, lifecycle or storage authority is added.

Equivalence hashes a versioned tuple of model digest, modality, dimensions,
normalization and sorted media-type/content-digest pairs. Multiplicity matters;
correlation/input IDs, backend aliases, batch order and budget-event cursors
do not. Changed bytes/model processing identity remain distinct. Accepted
receipt replay does not reserve another count. A repeated-work denial persists
before dispatch; policy cannot be raised or disabled by reconfiguration,
reconnect or reopen. This is exact embedding-batch equivalence, not a judgement
of semantic progress or model quality.

Tracking retains at most 128 distinct identities. Existing entries may spend
their remainder; a new identity beyond the bound durably stops the budget.
No eviction resets old counts. Time/step limits, uncertain attempts and Stop
remain admission fences. Missing optional fields preserve the previous
time/step-only budget serialization; no migration lane is introduced.

Owner review followed README/AGENTS, alpha outcomes, Gate G, POAM-009,
LFG/SWAG, the public-effort journal and change-authoring procedure. Complete
budget contract, reserve/finish, persistence and public configuration code,
the new repetition helper, affected tests and their final diffs were reviewed.
Model/descriptor validation, FeatureHash computation and shared output checks
were traced at the existing inference owner.

| Boundary | Reviewed/changed implementation |
|---|---|
| Contract | `rrd-contract/src/reasoning_budget.rs`: policy, bounded persisted counts, terminal reasons and validation; `tests/task_effort_contract.rs`: invalid policy/count/history and prior serialization. `src/lib.rs` pins the reviewed new OpenAPI digest. |
| Engine | `engine/reasoning_budget.rs`: admission and reservation; new `reasoning_budget/repetition.rs`: equivalence/check; `persistence.rs`: validate loaded counts; `public.rs`: immutable policy binding. Existing CAS, attempt receipts, deadlines and execution remain canonical. |
| Engine tests | `engine/tests/reasoning_budget.rs`: real computation/cycles, renamed/reordered batches and backend alias, changed model/content, new session/reopen, policy immutability, competing workers and bounded 128-identity history. |
| Client | Existing real authenticated `rrd-client/tests/task_effort.rs`: A/B/A denial through HTTP and same stopped policy/counts after server/store reopen. No new transport operation. |
| Generated/reference | Five existing SDK generators, TypeScript wire schema, seven signal projections, OpenAPI constant and protocol/SDK digest citations; existing Gate G/LFG-SWAG/POAM-009 progress, this linked journal, nearest index and execution inventory. |

No implementation is deleted or moved. Characterization retains the previous
task budget/Stop/replay corpus; the new flow tests extend that same owner.

## Research, observability and bounds

Local canonical model identity, source bytes and transaction semantics determine
equivalence; no new upstream algorithm or external dependency is imported.
The prior [aggregate-timeout rationale](https://docs.temporal.io/encyclopedia/detecting-activity-failures)
and [non-abortable blocking-work limit](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html)
remain relevant to unchanged deadline/cancellation behavior. No fresh research
claim or broad semantic no-progress claim is made.

Public observation adds bounded digests/counts and two terminal reason values,
not prompt bytes or private thought. Each count is positive, within its policy,
and all counts sum to reserved steps. Existing storage/control input bounds
still apply. The explicit 128-entry admission boundary is not a measured heap,
throughput, installed resource or 24-hour soak qualification.

## Validation and exposed failures

Commands use the existing local Cargo cache. Logs are under workspace
`target/g03-*.log`, not distributed assets.

- Initial `cargo check --locked -p rrd-engine -p rrd-client` passed, 50.08 s.
- The first focused engine test compile failed because scoped worker closures
  borrowed a local closure. Changed captures to `move`; no production behavior
  was altered to address that compile failure.
- The next focused run passed cycle/reopen and contention, but the capacity
  fixture failed on `source-1` with FeatureHash seed 44/dimensions 32. The
  `source` and `1` token contributions occupy bucket 20 with opposite signs;
  normalization divided by zero. Existing shared output validation rejected
  the non-finite result before acceptance. Recorded the adapter deficiency in
  POAM-009. The repetition-capacity oracle now uses single-token inputs to
  guarantee nonzero vectors while still executing every bounded reservation.
- `cargo test --locked -p rrd-contract --test task_effort_contract -p rrd-client --test task_effort -- --nocapture`
  passed: 2 contract tests and the real HTTP flow, 77.94 s including compilation.
- TypeScript typecheck passed. Its initial test ran before signal regeneration
  and caught the old signal projection's OpenAPI digest. Regenerated from the
  canonical export; no assertion was removed or relaxed.

Final validation:

| Command | Result |
|---|---|
| `cargo test --locked -p rrd-engine --lib repetition_ -- --nocapture` | Corrected history: 3 passed, 47.17 s including build; actual tests 1.87 s. |
| `cargo test --locked -p rrd-engine --lib -- --test-threads=4` | 171 passed, 2 existing ignored, 28.89 s. |
| `cargo test --locked -p rrd-contract -p rrd-client -p rrd-server` | 147 passed, no failures/ignored, 123.53 s including build/lock waits. |
| `cargo clippy --locked -p rrd-engine -p rrd-contract -p rrd-client --lib --tests -- -D warnings` | Passed. |
| TypeScript typecheck/lint and corrected tests; Go `go test ./...` | Passed; 6 TypeScript tests. |
| Five canonical SDK generators and `python scripts/ci/check_generated_surfaces.py --write` | Passed, 48 operations and seven signal projections. |
| `cargo fmt -p rrd-engine -p rrd-contract -p rrd-client -- --check`, `git diff --check`, generated-surface verification, navigation/inventory generation and corrected documentation validation | Passed; 1467 inventory records and 439 document statuses. |

Documentation validation initially required explicit Gate F/H dependency links
in POAM-009; added those owner links. No gate status changed. The reviewed OpenAPI shape is
`13435dd98b812a64f630d4ae91cb065501e4abab934aa07bd8e53f37b6773814`;
the operation count remains 48 and signal catalogue content remains unchanged.

## Limits and checklist

This package does not implement route/context/fallback execution, arbitrary
semantic progress, token/cost accounting, an LLM, Connectome controls,
extensions or uncertain-work reconciliation. Once stopped, a different input
does not reopen this task's budget. Existing standalone embedding remains
per-call. Model descriptors are trusted admitted adapter declarations; this
does not verify a remote provider's claimed weights.

No installed process-kill/resource/capacity campaign, 24-hour soak or full
workspace suite is claimed. Python/Java/.NET runtime lanes remain candidate
CI responsibilities; generation parity is not execution conformance. The
FeatureHash zero-norm defect is separately tracked, not hidden by the fixture.

Checklist: pre-edit PR, baseline, canonical source map, first oracle,
research/trace/resource decisions, implementation, exposed failures and linked
evidence are recorded. Final diff/source review and generated indexes accompany
the candidate. No version, roadmap checkbox, POA&M closure, Connectome pin or
official promotion changes. Exact candidate CI must pass before merge.
