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
Parent integration checks and exact changed paths follow before publication.
