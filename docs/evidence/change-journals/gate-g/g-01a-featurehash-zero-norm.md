# G-01a: reject degenerate FeatureHash normalization

**Status:** active local repair evidence; candidate CI pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/g-01a-featurehash-zero-norm`
**Owner:** OBJ-06, G-01a and POAM-009
**Pull request:** [105](https://github.com/rrflow/rrflow-development/pull/105)

Baseline `1b6188709f907ca75ea880e8d006889e69380611`, tree
`9dc37d713440780f62bed1c3899aca9ae198fe15`, combines fetched development/main
with exact PR104 `116263723a28cb7f9d2c24fda8ed04f07e358eb7`. The pre-edit
draft PR records the change brief, source map, first oracle, commands and stop
conditions. The defect was exposed by the preceding repetition-capacity test
and recorded in POAM-009 rather than concealed by changing its input fixture.

With dimensions 32 and seed 44, the `source` and `1` tokens in `source-1`
map to bucket 20 with opposite signs. The old backend normalized that zero
vector by dividing by zero and returned 32 non-finite values to direct callers.
The shared dispatch validator rejected them afterward. The repair checks for
a finite positive norm before division and returns an explicit error for the
degenerate case. It does not fabricate a vector, change accepted embeddings,
alter the model digest or bypass downstream validation.

Source review covered README/AGENTS and G-01a/POAM-009 ownership, prior
repetition evidence, `FeatureHashBackend` construction/tokenization/hashing/
normalization in `crates/compute/rrd-inference/src/lib.rs`, model/descriptor
validation, shared output validation and the existing registry/controlled
execution tests. Changed code is the three-line norm guard and one regression
in `tests/pipeline.rs`. That test file and the complete changed function/diff
were reread. No code is moved or deleted. Existing pipeline/cancellation tests
characterize accepted-output and admission behavior.

The regression first calls the backend directly, then a mixed ordinary batch,
then controlled registry execution. Every degenerate operation must fail;
the next ordinary input must acquire the same backend and return a finite
32-dimensional unit vector. This binds the correction to the existing backend
and executor, without adding a fallback or separate recovery path.

| Command | Actual result |
|---|---|
| `cargo test --locked -p rrd-inference --test pipeline feature_hash_rejects_zero_norm -- --nocapture` before the repair | Failed as intended: `unwrap_err()` received a dense vector of 32 `NaN` values; 9.77 s including compilation. |
| Same focused command after the repair | Passed, 2.88 s. |
| `cargo test --locked -p rrd-inference` | 26 passed, no ignored/failures, 2.70 s. |
| `cargo clippy --locked -p rrd-inference --all-targets -- -D warnings` | Passed, 17.00 s. |

Commands use the existing Cargo target cache; local logs are under
`target/g01a-zero-*.log`. The defect is fully explained by the local signed-hash
and normalization implementation, so no external research or source adaptation
was needed. Existing bounded outcomes, cancellation checkpoints and resource
admission remain intact. No prompt content logging, diagnostic stream, schema,
dependency or measured performance claim is added.

Updated POAM-009 links this remediation while retaining the broader gap.
Generated journal navigation and execution inventory include this record.
Formatting, documentation, navigation, inventory and diff checks accompany the
candidate. No product version, release status, public schema, UI pin or
official promotion changes. Exact candidate CI is still required before merge.

No installed real-model, full-workspace, resource/capacity or soak qualification
was run for this localized arithmetic repair. The prior 171-pass engine and
147-pass contract/client/server evidence belongs to PR104 and is not presented
as a rerun for this commit. Gate G, C-07, POAM-009 and alpha remain open.

Checklist: pre-edit PR/baseline, canonical source/behavior map, reproduced
failure, minimal correction, focused/owning validation, trace/resource/research
decisions, source/diff review and linked evidence are recorded. Candidate CI
and any eventual merge are bound by the pull request, not by this local result.
