# PR16: integrate the shared APFS allocation correction

**Status:** active integration evidence; native candidate qualification pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/knowledge-trees-apfs-settlement`
**Owner:** PR16's existing alpha/roadmap outcome; shared C-07f / POAM-035 prerequisite

## Brief and source preservation

[PR16](https://github.com/rrflow/rrflow-development/pull/16) received its
integration brief before effects. Starting revision `007f8240b710ce7de349a8124610d2e75046099d`.
Ordinary-merge source `b70db5832c06aad404409df50889caaad2dabba3` carries PR129's exact fixture correction
`d04a0dc5b3379d52fa893b12cd71bc30669fee16`. No history rewrite or feature source deletion occurs.
The task-control, endpoint and knowledge-tree dependency order remains
PR105, then PR113, then PR16. Other package owners and boundaries are unchanged.

The [native failure and correction record](../gate-c/c-07f-volume-allocation-settlement.md)
owns the Apple ABI research, failure log, admitted-volume barrier and native
qualification. It records the actual PR118 metadata count failure, not a
successful APFS repair. This package inherits those changes verbatim. The four
shared Rust source/test files were byte-compared to the owning revision after
merge; package-specific implementation and tests are preserved.

Conflicts: none.
Only replaceable generated projections were resolved by regeneration. The
complete baseline diff was constrained to those shared source/tests, the native
workflow and supporting/generated documentation; no other implementation
changed. Existing package source characterizations remain bound to their
original revisions rather than relabeled as new tests.

## Research, trace and validation

Read: README and package brief/journals, the exact incoming commit/diff, shared
capacity.rs/capacity_apfs.rs, acceptance validator/native-step tests, workflow
native cases, original retained failure and POAM035. New external research or
profiling is unnecessary for verbatim integration; the owning repair's primary
Apple references and bounded diagnostics remain canonical. Stop for any
non-generated source conflict, lost source/test, widened resource bound,
external activation, or unsupported acceptance claim.

No provider, public API, Connectome gitlink, runtime budget or timeout change
is introduced by this merge. New paths are the inherited helper/journal and
this package journal; generated navigation/inventory follow their owners.
The previous package journals remain immutable.

Validation results are recorded below before commit. Native candidate CI must
run against this final package tree after shared qualification; PR129's result
cannot qualify a different head. No actual APFS call, native capacity matrix,
full profile or 24-hour soak ran locally for this integration.

C-07 and POAM035 remain open; no roadmap or alpha maturity status changes.

Passed commands:

- `python scripts/knowledge/render_navigation.py`: passed.
- `python scripts/ci/build_execution_inventory.py`: passed.
- `python scripts/ci/check_documentation.py`: passed.
- `python scripts/ci/check_workflow.py`: passed.
- `python scripts/knowledge/render_navigation.py --check`: passed.
- `python scripts/ci/build_execution_inventory.py --check`: passed.
- `rustfmt --edition 2021 --check crates/adapters/rrflow-cli/tests/support/capacity.rs crates/adapters/rrflow-cli/tests/support/capacity_apfs.rs crates/evaluation/rrflow-eval/src/storage/acceptance.rs crates/evaluation/rrflow-eval/src/storage/acceptance_tests.rs`: passed.
- `git diff --check`: passed.

Shared source identity checks passed; changed source and generated projections
were inspected. Checklist: brief before effects; source/failure ownership;
canonical dependency order; unchanged resource/recovery oracles; focused
integration checks; native omissions explicit; immutable linked journal.
Full candidate and normal merge-group CI remain mandatory before landing.
