# D-01 combined identity and standalone qualification

**Status:** active bounded integration evidence; alpha remains open
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-d/d-01-combined-identity-qualification`
**Owner:** OBJ-07, D-01, POAM-006 and POAM-020

## Brief, baseline and scope

[PR 44](https://github.com/rrflow/rrflow-development/pull/44) records this
qualification dependency before integration. Identity implementation
`45ccfd9f` incorporates standalone qualification candidate
`f0312a59a1403379a4e94b7ecacc994fc3c3901a` from PR 43 by ordinary merge.
PR 43 must pass and merge before this package lands. No main push or history
rewrite is used. Both original package journals remain immutable.

The lifecycle test merges without a source conflict: explicit identity,
version advertisement and foreign-root denial now execute through the staged
binary and clean environment. Both fresh/existing process cases, restart,
query, Parquet, backup/restore and authentication assertions remain. The only
Git conflict is the generated file inventory; its owning tool regenerates it.
POAM-020 labels the recorded collision as a baseline observation and points
to the bounded repair without claiming full topology closure.

Read the merged lifecycle fixture and both assertion sets, Gate D, POAM-020,
the two package journals, runner scheduling correction and merge diff.
No new upstream research, runtime source replacement, trace authority or
resource model is introduced. Existing receipts and process readback remain
the oracle. Scope is this integration journal, generated projections and
the POA&M wording. Stop on lost assertions, generated drift, unexpected
project effects or failed candidate CI.

## Acceptance and checklist

Run the actual staged CLI lifecycle, strict focused lint, runner policy,
documentation/workflow, navigation/inventory and formatting checks before
commit. Retain the prior engine/contract/Rust SDK evidence from the identity
package; unchanged runtime code does not warrant relabeling those as new runs.
All 17 exact candidate checks, including Linux offline execution and shared
language conformance, remain mandatory for merge. Results attach below and
candidate/merge receipts attach to PR 44.

Owning outcome, brief, starting revisions, changed scope, replacement map,
research/trace decisions, acceptance oracle and stop conditions are recorded.
This integration closes no objective, roadmap item or POA&M, changes no
product version, and claims no true alpha or signed-release completion.

Local result: the staged actual-primary lifecycle passed all five tests in
214.24 seconds, with a 155,411,456-byte executable. Both identity allocation
and plan-version assertions executed alongside fresh/existing install, denied
foreign apply, query, restart and restore readback. Strict lifecycle Clippy,
six runner-policy tests, four execution-inventory tests, 17 knowledge tests,
documentation/workflow policy, formatting and diff checks passed. The first
inventory regeneration correctly refused the unresolved Git index; resolving
the generated entry from its baseline, then regenerating from the combined
source, succeeded. This tooling error did not execute product code.
