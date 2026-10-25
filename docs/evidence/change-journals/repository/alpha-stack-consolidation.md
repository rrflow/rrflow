# Existing alpha stack: one preserved integration candidate

**Status:** active source-preservation evidence; final candidate and merge qualification pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/alpha-stack-consolidation`
**Owner:** OBJ-07 / H-06 integration prerequisite; existing C-07, G-01a/G-02a and D-06 package owners remain authoritative

[PR114](https://github.com/rrflow/rrflow-development/pull/114) is the existing
combined implementation destination. Its consolidation brief and source/test
map were published before this record was created. Driver: current repository
integration session. Starting revision
`56920c565c0cec7be9164eaf49acab6a1f1a0614`, tree
`44223e777d74903362d1c4ae18edb790a4b90142`; fetched accepted development/main
remains `90373e90e6d04e4093e8e813a74d396bacb96fcf`.

## Change brief and traceability result

The implementation was already combined through ordinary merges in PR114.
This correction changes the integration/acceptance path, not the implemented
engine or UI scope. Ten overlapping candidate runs occupied all 20 observed
runner slots while PR129's 22 successful substantive jobs waited for the
aggregate gate. One final full candidate and its protected merge group will
qualify the combined tree. Earlier partial runs retain their own identities;
they are not substitutes for that acceptance. No required check, test, budget,
branch protection or workflow is reduced.

The repository's [team integration procedure](../../../roadmap/rrflow-1.0-execution/change-authoring.md#team-ownership-and-integration)
permits consolidation with an implementation-requirements map. The PR brief
maps every source package, canonical destination, characterization tests and
owning gate. The existing implementation remains at those same destinations:

| Source | Retained behavior and tests | Detailed implementation evidence |
|---|---|---|
| PR129 / PR128 | Installed capacity/recovery, bounded APFS pressure and teardown, encoder byte equivalence and working-set reduction, relationship readback | [Joint pressure](../gate-c/c-07f-apfs-joint-pressure.md), [encoder](../gate-c/c-07-encoder-working-set.md), [native integration](../gate-c/c-07-native-wal-candidate.md) |
| PR105 / PR100–104 | Inference deadlines/cancellation; durable budgets, Stop, repetition and replay; actual authenticated HTTP task flow and reopen | [Task integration](../gate-g/task-effort-current-main-integration.md) |
| PR113 | Durable endpoint preview/configuration/revocation and restart; canonical schemas/clients and exact evaluation goldens | [Combined endpoint/task integration](../gate-d/endpoint-profile-task-control-integration.md) |
| PR16 | Governed knowledge-tree catalogue, canonical transaction admission, CAS/replay/reopen and actual HTTP flow; combined public catalogue | [Knowledge integration](../gate-h/knowledge-trees-public-api-integration.md) |
| PR118 / PR30 / PR23 | Owner-directed requirements, devcontainer inputs and bootstrap, current ruleset evidence; no new runtime or repository-setting effect | [Owner direction](owner-direction-joint-pressure-integration.md), [tooling](devcontainer-joint-pressure-integration.md), [ruleset](ruleset-joint-pressure-integration.md) |
| PR76 / PR46 | Canonical successful-run receipt validation and bounded offline lifecycle orchestration/tests | [Acceptance](../gate-c/c-07h-joint-pressure-integration.md), [offline lifecycle](../gate-c/offline-lifecycle-joint-pressure-integration.md) |
| PR114 | Optional Connectome gitlink `38142b77110a8f1db77e306e7d1ad375fd918267`; no current-engine UI qualification claim | [Existing pin integration](../gate-h/h-06-joint-pressure-integration.md) |

The current source heads remain reachable in the existing integration, but
ancestry alone is not preservation evidence. The first oracle compares the
entire Git index/tree to the stated baseline: every pre-existing implementation,
test, schema, configuration, workflow and gitlink must remain byte-identical.
Only this new journal, generated journal navigation and the generated file
inventory may differ. No implementation, test, fixture, historical journal or
branch is deleted or rewritten.

## Files reviewed and effect envelope

This consolidation read the change-authoring and implementation-navigation
procedures, all nine joint-pressure integration journals, the task-current-main,
endpoint/task and knowledge-public-API integration records, and the PR114 brief
in full. It reviewed README's product/bootstrap boundaries and the CI operational
contract's candidate-chain section alongside the retained component PR briefs.
The component packages' full source reviews and exact test commands/results
remain in those linked records rather than being claimed as new executions.
The complete new file and final staged diff are reviewed before commit.

Created: this journal. Generated changes: journal navigation and
`docs/roadmap/rrflow-1.0-file-plan.jsonl`. The first-party source tree, public
catalogue, dependency locks, workflow, default configuration, tests, assets and
Connectome pin are outside the permitted edit set. Stop for any drift in those
paths, missing behavior or tests, a relaxed oracle or unsupported status claim.

## Retained observations and research

Before withdrawing redundant candidates, raw run and job API observations were
retained under `target/alpha-stack-consolidation-evidence/`. At capture, PR129
had 22 successful jobs; PR105 18; PR113 six; PR16 six; PR118 eleven; PR30 eight;
PR23 four; PR76 five; PR46 zero; and PR114 one. No captured job had failed or
timed out. PR114's obsolete run was cancelled by its temporary draft transition.
The others will be withdrawn only after the evidence-only candidate is ready.
These observations describe partial runs, not completed candidate acceptance.

Run 40163426522's native jobs passed 32 macOS, eight Windows and eight Linux
cases. Integrated PR118 run 40165581353 / job 121085826967 independently passed
32 macOS cases, actually exercising metadata pressure at all four selected
WAL/segment/manifest/CURRENT boundaries. All 32 receipts had zero unresolved
operations and history violations. Raw log SHA-256:
`09097e451f34b58d74c42198f501dae524a6ffb699488f2e79147eda8ee30ca0`
(637,869 bytes). Additional successful native logs remain at their original
candidate paths. Original time, file-count and aggregate byte bounds remain.

The temporary read-only log collector attempted to retrieve a cancelled,
never-run PR114 native job and received HTTP 404. Its selection was corrected
to retain completed success/failure/timeout jobs; cancelled-job metadata stays
in the raw API records. This was an evidence-collection selection error, not
an engine test failure. No native receipt was manufactured or replaced.

Primary research: [GitHub indirect merges](https://docs.github.com/en/pull-requests/reference/pull-request-merges#indirect-merges)
explains that reachable source heads can be marked merged independently of
their own checks; that label is therefore not our oracle. The final combined
candidate and required merge-group pipeline must pass.
[Managed stack requirements](https://docs.github.com/en/pull-requests/reference/stacked-pull-requests#merge-requirements)
require linear history; no stack/rebase tool or history rewrite is introduced.
Existing native resource/debug traces remain sufficient for unchanged source;
no runtime instrumentation or new upstream code is needed.

## Verification and remaining qualification

The first whole-tree oracle passed: all 1,506 baseline paths outside the two
generated indexes retain their exact Git blob/mode identities, including every
implementation/test file and the Connectome gitlink. The preserved identity
map SHA-256 is
`9ed67a9bf430733278d27390011a34a375e0f8d6b439c06bf5fffbee566953cc`.
The ten recorded source heads are retained. The only changed paths are the
three declared evidence/projection files.

Local commands and results:

- `python target/verify_alpha_stack_preservation.py` from the primary checkout:
  passed the full index/tree comparison and source-head checks.
- `python scripts/knowledge/render_navigation.py`: passed; one index updated.
- `python scripts/ci/build_execution_inventory.py`: passed; 1,728 records.
- `python scripts/ci/check_workflow.py`: passed.
- `python scripts/knowledge/render_navigation.py --check`: passed.
- `python scripts/ci/build_execution_inventory.py --check`: passed.
- `git diff --cached --check`: passed.
- Initial `python scripts/ci/check_documentation.py` rejected this record's
  missing explicit active/historical status prefix. The status is corrected to
  active. The next check exposed the stale generated navigation after that
  correction; regeneration resolved it. The documentation check then passed:
  606 document statuses, 604 classified coordinates and all local links.

Runtime suites are not repeated locally for an evidence-only delta. The final exact head must pass
every existing candidate partition, and the normal protected merge group must
also pass. Collect both canonical successful-run receipts before claiming
landing. Keep superseded PRs until their implementations and tests reach main;
then inspect actual GitHub state and verify final source identity.

The separate segment-descriptor exhaustion defect remains open in
[its native failure record](../gate-c/c-07g-native-descriptor-failure.md).
Capacity profiles and the 24-hour soak have not been qualified. Complete model
routing, provider ingestion, PostgreSQL compatibility, cloud sync and current
Connectome client qualification remain with their existing owners. No objective,
roadmap, POA&M completion or product version is changed.

Checklist: pre-effect brief; existing owner boundaries and source/test map;
exact baseline and whole-tree preservation oracle; immutable original evidence;
explicit research/trace decisions; generated navigation/inventory; complete
diff review; candid omitted checks; final candidate and merge receipts required.
