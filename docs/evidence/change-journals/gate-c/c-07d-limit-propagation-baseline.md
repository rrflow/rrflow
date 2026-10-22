# C-07d typed mutable admission and inspection baseline

**Status:** active bounded dependency baseline; implementation and CI pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-c/c-07d-limit-propagation-baseline`
**Owner:** OBJ-02 prerequisite, [C-07d](../../../roadmap/rrflow-1.0/gate-c-07.md), [POAM-035](../../../poam/rrflow-1.0-alpha/poam-035.md)

Accepted main is `dde6ef86d278b8c59cf1f61ebc2d3923186396d5`, tree
`bc76b62d5a500395fb8e1193dcd4d94eeac79ada`. This package depends on PR 65's
native mutable admission and options-aware inspection, currently locally
qualified at `948f7170aae47b17a39cdfa08a1e062dd41f6cb7`. Dependency code may
be combined locally, but no acceptance is inferred before its own CI/merge.
Reconcile accepted PR 64/65/66 before this candidate lands.

Inspection confirms that store error conversion and contextual native open
wrappers erase `MutableLimit` into generic substrate errors. Engine prepared
intent cleanup recognizes the existing encoded limit but not mutable limits.
Store inspection uses default options despite PR 65's native options-aware
reader. The target retains typed byte/version rejection through adapters and
the existing absent-outcome proof; unknown I/O remains uncertain. Store
read-only inspection must enforce requested replay policy and preserve bytes.

The preimplementation PR binds store errors/open/inspector/export boundaries,
engine error/transaction handling, parent visibility of the actual persistent
composition helper, focused store and existing semantic/engine history tests,
native CI and supporting records. Reexport native option types rather than add
a parallel configuration representation. No installed setting, format/version,
dependency, automatic repair or semantic authority changes are in scope.

Reviewed owner chain and C-07/POAM; native policy/inspection/prepare/replay;
store and service error files, engine write-admission history, core composition,
prepared intent and durable outcome reconciliation; store constructor/inspection
and semantic atomicity fixtures. Existing evidence helpers remain canonical.
First oracle must reproduce actual generic classification/retained intent using
native caps, followed by corrected complete rejection/reopen/reuse histories.
Read-only low-budget denial must leave file bytes unchanged and allow later
adequate-budget inspection. No new runtime command has run for this seed.

No external research/adaptation is needed for typed propagation of existing
semantics. Preserve traces and use independent before/after values and file
inventories. Record actual commands, failures, complete changed-file reread,
checks not run and generated projections in the successor immutable journal.
Stop on post-effect misclassification, unproven intent clearing, partial domain
effects, inspection mutation, scope expansion or failed CI. C-07 and alpha
remain open; this package does not qualify installed policy, RSS, real capacity
exhaustion or the 24-hour soak.
