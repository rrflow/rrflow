# G-05: user-selected effort and aggregate budgets

**Status:** active contract-preparation evidence; runtime implementation remains open
**Coordinate:** `rrflow://rrflow-instance/data/evidence/g-05-user-selected-effort-contract`
**Owner:** [Gate G](../../../roadmap/rrflow-1.0/gate-g.md), OBJ-04 and POAM-009
**Pull request:** [90](https://github.com/rrflow/rrflow-development/pull/90)

Starting revision `ff11edf6ec8da7461955cb4eb349fb81828a2f09` integrates the
current C-07 candidate from fetched main. The owner's additional requirement
is configurable effort that prevents unwanted hours-long reasoning loops.
The pre-edit PR description binds this documentation-only preparation; it
does not replace the ongoing C-07 task or skip Gate G prerequisites.

## Change brief and review

`ReasoningLimits` currently validates three ceilings, seals them into installed
configuration and exposes them through capabilities. Code inspection found
no end-to-end governed-run consumer. The capability is unavailable. The old
reference required a remaining retry budget but did not fully specify user
effort, aggregate attempts, stop conditions or whether resume resets limits.

The LFG/SWAG reference now owns finite presets/custom choices, one aggregate
elapsed/steps/attempts/token/cost budget, repetition detection, partial stop
receipts and explicit extensions. Gate G binds adapter, durable-run and context
acceptance to that owner. Connectome links its controls and unavailable states;
the installation reference shows the existing complete-TOML input path and a
two-minute configuration example while explicitly denying runtime enforcement.
POAM-009 records the observed gap. No shipped default, runtime type, API, SDK,
implementation, external dependency or lifecycle authority changed.

Read README, alpha objective, Gate G/H, POAM-009, authoring procedure, complete
LFG/SWAG reference; relevant installed-lifecycle and Connectome configuration
sections; `rrd-contract/src/deployment.rs` reasoning declarations/validation/
defaults, all reasoning-limit usages, bundled format-3 TOML, installation
custom-configuration tests and server capability projection. Reviewed complete
changed sections and diff. New journal and generated navigation/inventory are
the only additional paths. No implementation was moved/deleted/rewritten, so
replacement traceability is not applicable.

Research: user requirement and actual source ownership suffice; no external
research or adaptation claim. Trace/resource/debug: preserve the observed
metadata-versus-enforcement distinction. Future acceptance requires actual
installed histories and real-adapter limits, not only mock timers. Stop on
new authority, changed defaults, dependency-order bypass or unsupported claims.

## Commands and evidence

- Repository search for all three reasoning limit fields found declaration,
  defaults, validation, capability projection and installation tests, not
  aggregate runner enforcement. This is implementation inventory evidence.
- A Python `tomllib` check parsed the documented overlay into the complete
  bundled configuration, retained required sections, confirmed 120000/16/15000
  and checked them against current declared Rust ceilings. The defaults stayed
  900000/256/60000. This checks the example, not engine execution.
- `python scripts/knowledge/render_navigation.py` and
  `python scripts/ci/build_execution_inventory.py` regenerate discovery.
- `python scripts/ci/check_documentation.py`, both generation `--check`
  commands and `git diff --check` all passed.

The first documentation check caught an observation paragraph accidentally
placed inside POAM-009's owning-gates section, where the gate parser treated
its capital C as another owner. The observation moved to the deficiency
discussion; assigned gates did not change. No runtime test was added or rerun
for these prose-only changes. C-07 native campaigns continue on their frozen
source and are not evidence for this target feature.

Checklist: pre-edit brief, source/owner inspection, explicit research and
trace decisions, example check, changed-section/diff reread, linked journal
and generated projections are recorded. Runtime budgets, presets, governed
configuration changes, Connectome controls and adversarial installed/real-model
histories remain unimplemented/unqualified at their existing gates. No gate,
objective, alpha or POA&M status changed. Exact candidate CI is required before
merge; no successful CI or merge is implied here.
