# SWAG learning boundary alignment journal

**Status:** active immutable change-journal evidence
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-g/swag-learning-boundary-alignment`
**Owner:** receipt for learning-plan alignment with existing inference and engine boundaries

## Package and baseline

OBJ-06/OBJ-08 prerequisites; G-06 offline controller qualification and I-03/I-07
governed improvement. This is a documentation and organization package, not
runtime implementation or a trained-model result.

Fetched `development` before edits. Baseline:
`8e07399843d4dd55e847dc191e7bda59bc05614a`; tree:
`ba00c4f4c13de3c1d5404f6807fedc4bf202afbf`. Work is isolated on
`agent/swag-training-boundary`, based on fetched `development/main`. The
original checkout has an unrelated unresolved merge; its index and files were
not modified. No private official-repository promotion is authorized here.

The change brief was written outside the source tree before editing and is
attached to the source-bearing pull request. Its scope is the learning folder,
existing reference/architecture links, G/I package refinements, POAM-009,
profile terminology, this journal and generated navigation/inventory. Stop
conditions prohibit runtime changes, a second scheduler/store, automatic
activation, default Python/GPU dependencies or unrelated merge resolution.

## Inspection and traceability

Complete relevant owner reads include the root README, AGENTS, alpha
objectives, system overview, LFG/SWAG reference, inference index, DevPulse
contract, Gate G, Gate I, POAM-009, change-authoring and
implementation-navigation records. The CI topology/presubmit sections and
agent-bootstrap independent-engine/project-integration sections were inspected.

Implementation inspection was targeted, not a complete Rust source audit:

| Existing behavior and inspected symbols | Disposition and owning package |
|---|---|
| `rrd-contract/src/router.rs::RouteStepDecision`, its `validate_for`, and `tests/router_contract.rs` test inventory | Preserve the three decisions and candidate/budget binding; endpoint selection is a separate G-01b capability. |
| `rrd-contract/src/model_manifest.rs::RouterModelManifest` and validation; `model_manifest_contract` test inventory | Preserve schema/grammar/resource identity. G-06b cannot load arbitrary endpoint-selector outputs or unbound LoRA sidecars through this manifest. |
| `rrd-inference/src/lib.rs::RouterModelArtifacts`, `verify_router_model_before_load`, `load_router_model_after_handshake` | Retain the existing compute boundary and byte admission. No replacement or relocation. |
| `rrd-inference/tests/pipeline.rs` router admission and corruption/drift rejection tests | Preserve rejection before the loader. These tests were read, not run locally for a documentation-only change. |
| `rrd-engine/src/engine/inference.rs` admission/catalogue/generation excerpts | Preserve engine authority; G-01a already owns registry-lock correction. Training must not bypass it with another executor. |
| Existing `EstateConfiguration` and `EndpointProfile` declarations and endpoint/hardware validation excerpts | Extend existing configuration owners in future D-06/G-01b work; global preferences are explicitly accepted per estate. |

No source behavior, caller, wire schema, persistence format or test is removed
or rewritten. Therefore no replacement-equivalence evidence is claimed.
Exact future implementation envelopes remain the responsibility of their
bounded PRs; this journal is not a new global implementation inventory.

## Changes

Created `docs/reference/inference/learning/` containing its generated index,
`controller-training.md` and `spark.md`. The existing inference index,
LFG/SWAG reference and root README link to that one boundary.

Changed the RROFLOW terminology row from unowned to explicitly unimplemented
G-06/I-03/I-07 work. Gate G now separates offline dataset/baseline, artifact
qualification and installed comparison; Gate I separates learning candidates
and governed promotion using existing routines. POAM-009 records the missing
pipeline/evaluation/promotion and manifest limitations. The existing DevPulse
preferences section defines AI Development Profile/AIDevSpec as product terms
and proposed structured representation, not an implemented extra schema.

No Rust, SDK, dependency, version, Connectome source, training asset, installed
machine or runtime configuration changed. Implementation folders will accompany
their first tested package rather than empty scaffolding.

## Research and diagnostics

Primary sources were reviewed in the preceding research turn on 2026-10-20:
NVIDIA Spark/NeMo/Unsloth guides; RouteLLM, Arch-Router, BaRP, MetaRouter and
LLMRouterBench papers; Phoenix and Raindrop Workshop public descriptions;
AGENTS.md and Claude memory. The learning records link the sources and state
the adaptation/uncertainty decisions. No source code was copied.

No runtime tracing or resource benchmark was needed because no executable
behavior changed. The contracts require future measured resource peaks,
latency breakdown, causal evidence, held-out quality, checkpoint reload,
quantization evaluation and promotion/replay failure evidence. None is claimed
as an observed result.

## Verification

First characterization: the baseline RROFLOW row explicitly had no roadmap
owner; the current router manifest binds the three-proposal schema, and the
repository search found no first-party training pipeline. This package gives
the missing responsibilities canonical owners without changing that schema.

Local commands:

- `python scripts/knowledge/render_navigation.py`: passed; initial render
  discovered 17 indexes, 334 nodes and 1,482 edges and updated the new index.
- `python scripts/ci/check_documentation.py`: passed before the journal;
  335 statuses and 333 classified coordinates.
- `python scripts/knowledge/test_navigation.py`: 6 tests passed.
- `python scripts/knowledge/test_export.py`: 11 tests passed.
- `python scripts/ci/test_execution_inventory.py`: 4 tests passed.
- `git diff --check`: passed.

Final journal-inclusive checks also passed:

- `python scripts/ci/check_documentation.py`: 336 statuses, 334 classified
  coordinates and valid ownership/links.
- `python scripts/knowledge/render_navigation.py --check`: 17 indexes,
  335 nodes, 1,483 edges and no drift.
- `python scripts/ci/build_execution_inventory.py --check`: 1,255 records.
- `python scripts/ci/check_workflow.py`: 3 workflows, 7 substantive jobs,
  5 engine suites, 20 default-feature and 6 optional-feature packages.

Candidate CI must pass on the exact PR head before merge; this journal does
not predeclare its result. The PR records final exact-head validation.

Surfaced exploration errors: a guessed `source-layout.md` path did not exist,
and searches of absent `training`, `models` and `evals` directories returned
missing-path errors. File discovery located the canonical
`implementation-navigation.md`; these failures produced no repository change.
Large combined reads were truncated and relevant portions were reread.

Not run locally: Rust/model/Spark tests, training, deployment, UI tests and
performance measurements, because this package changes only documentation and
no Spark host has been identified or accessed. Existing CI remains mandatory.
The original checkout's unrelated merge conflicts remain unresolved.

## Review and status

Changed authored records were reread and the complete authored diff reviewed;
generated navigation and inventory are checked mechanically. The package
checklist covers baseline, owner, exact scope, first oracle, research,
trace/resource decision, tests, errors and stop conditions. No implementation
was deleted or moved. Product version remains `1.0.0`.

No objective, roadmap checkbox or POA&M closure changed. Ownership and required
acceptance were refined; execution, training and promotion remain open.
The development-repository pull request, its exact-head CI and merge record
are the final effect evidence for this package.
