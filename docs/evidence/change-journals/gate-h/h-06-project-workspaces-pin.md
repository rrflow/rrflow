# H-06: pin the merged project workspaces UI

**Status:** active source integration evidence; parent candidate pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/h-06-project-workspaces-pin`
**Owner:** H-06 project UI integration prerequisite; POAM-011

Baseline: `335c9b67d8913f548de119378929a7aba8062a5a`. The change brief was
written before this source-integration evidence. A differing commit permits
opening the draft PR before advancing the gitlink.

[Connectome PR13](https://github.com/rrflow/connectome-development/pull/13)
merged at 2026-10-24 03:30:13 UTC as
`38142b77110a8f1db77e306e7d1ad375fd918267`. Reviewed head
`0aca893cd5e52ca596483645de4972a61bb17ca0` passed all frontend, real RRFlow
SDK/process and native Windows/Linux/macOS checks in
[candidate CI](https://github.com/rrflow/connectome-development/actions/runs/40055042635)
and [push CI](https://github.com/rrflow/connectome-development/actions/runs/40055038653).
The merged source tree equals the tested head's tree.

The UI's [sprint012 evidence](https://github.com/rrflow/connectome-development/blob/38142b77110a8f1db77e306e7d1ad375fd918267/.agents/sprints/012-project-workspaces.md)
records per-project routes, basic/advanced drilldown, bounded connection/owned
process registries, production browser checks and native two-project operation.
Knowledge-path activation/ingestion, replay, training and cloud sync remain
unavailable. Its engine/SDK stays pinned to
`4860a253663c9c757800aafeb40890b76114c4f0`; no latest engine contract migration
or full H-06 qualification is claimed.

This bounded parent package advances the optional source input only, preserves
`.gitmodules` and default engine independence, and updates the single Connectome
client reference. Existing UI source was reviewed/tested in its own PR; no
engine code is copied into the UI or sourced from it. No new upstream research,
source adaptation, tracing or performance measurement is needed for this pin.

## Parent integration evidence

[PR114](https://github.com/rrflow/rrflow-development/pull/114) was opened before
the gitlink mutation. Baseline tree:
`8c1289def013f36b2732ef7a6103deff711fe41f`.
The parent gitlink changed from `03b716b4dfe5623e6c82408c6f4df367c9795388`
to the reviewed merge above. `git ls-files --stage connectome` confirms mode
160000 and the exact target. Both tested head and merge have tree
`33452248add3f12c4144095330231d78289d05ab`.

`GIT_TERMINAL_PROMPT=0 git -c credential.helper= -c core.askPass= submodule
update --init -- connectome` acquired the public source successfully. The child
checkout reports the exact target revision/tree and credential-free HTTPS
origin. No escaping checkout or install-time fetch was introduced. The changed
client reference documents the actual project/UI bounds and remaining features.

Read the root README planning/checkout boundary, complete Connectome checkout
owner sections, child README/project workspace implementation, prior sprint012
validation and merged review/check results. No engine implementation was
rewritten or moved. The exact changes are the gitlink, client owner, this journal,
its generated nearest index and execution inventory. Existing child code and
its pinned engine remain untouched. The entire authored diff and gitlink were
reviewed; generated inventory is a projection, not another lifecycle authority.

Validation: exact mode/revision/tree and public acquisition checks passed.
Documentation, generated navigation, execution inventory and whitespace checks
passed before publication. No unchanged Rust/build/browser suite was rerun for
this pin; the child candidate already passed all its owning lanes and the parent
candidate must pass its source policy/CI before merge. No surfaced integration
error was suppressed. The first journal commit only enables a reviewable PR;
the final source pin is the implementation effect. No gate/POA&M completion,
latest-engine bundle qualification, training or cloud service is claimed.

Before landing, integrated public main
`2255c2bee9c2e4c2f7b44f053dc73a92feebc7b6` after PR109/110 passed all 23
checks and merged. The source merge was clean. Regenerate the inventory to
include both prerequisite histories and this pin, then require fresh candidate
CI. The child gitlink and its tested tree remain unchanged.
