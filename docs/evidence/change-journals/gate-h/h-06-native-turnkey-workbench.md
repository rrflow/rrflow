# H-06 native turnkey workbench integration

**Status:** active immutable change-journal evidence
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-h/h-06-native-turnkey-workbench`
**Owner:** OBJ-09 / H-06 optional client integration prerequisite

## Baseline and change brief

Starting RRFlow revision: `4860a253663c9c757800aafeb40890b76114c4f0`.
The prior optional gitlink is `ce08e2f6e1efd60ea7762598c23dc4659d0df1a3`.
The owner requested a native binary and an understandable, navigable Connectome
experience. The local brief preceded parent edits; GitHub needs a nonempty
source difference before a PR exists, so the bounded integration commit creates
its PR before any subsequent implementation. No parent runtime code changes.

Current to target: source registration and a historical client audit become a
pin to the reviewed native workbench, with current use instructions and exact
bounded evidence. The [client contract](../../../reference/client/connectome.md#native-local-workbench)
owns the behavior boundary. README links it; POAM-011 records a verified
schemaless query gap. Engine independence, version 1.0.0 and all objective/gate
statuses remain unchanged. Stop on a failed child candidate, mismatched pin or
sidecar, second authority, undeclared dependency or unsupported completion claim.

Read/reviewed at this boundary: README.md, AGENTS.md, the alpha objective,
Gate H, POAM-011, client contract, CI operations, execution map and its complete
change-authoring procedure/traceability sections; child README, governance,
native-workbench guide, PR12 implementation diff and immutable sprint journal;
existing public-association receipt; native query binder fields_for_source and
series execution source matching for the observed limitations. Git and the
inventory generator derive baseline paths/digests rather than a copied audit.

Changed paths: `connectome` gitlink, `README.md`,
`docs/reference/client/connectome.md`,
`docs/poam/rrflow-1.0-alpha/poam-011.md`, this receipt, nearest generated journal
index and `docs/roadmap/rrflow-1.0-file-plan.jsonl`. No source is deleted, moved
or rewritten. Traceability: old optional UI pin -> reviewed child PR12 revision;
existing native SDK/session, process lifecycle and public engine authority are
retained. The old audit is labelled historical, not overwritten as current proof.

## Child behavior and evidence

[Connectome PR12](https://github.com/rrflow/connectome-development/pull/12)
and its [immutable journal](https://github.com/rrflow/connectome-development/blob/050c6f6687dd5ef67b6ae393f3ef1575e6c14dac/.agents/sprints/011-native-turnkey-workbench.md)
own source-level evidence. Implementation head:
`050c6f6687dd5ef67b6ae393f3ef1575e6c14dac`. SDK/engine input:
`4860a253663c9c757800aafeb40890b76114c4f0`.

The first oracle rejects installation without exact preview acceptance and
proves accepted installation/reopen through the real engine. The owning
process suite passed all 4 tests, including independent sessions, verified
sidecar, schema, graph, vector/storage and key/value/event/series/geo queries.
Native unit tests passed 27 with those 4 deliberately ignored in the ordinary
suite; strict clippy, check and format passed. Frontend check/build and 3
browser rendering smoke tests passed. Browser tests are not native proof.

The Windows desktop/NSIS build passed. Actual WebView2 interaction cancelled
and accepted the native preview, read the installed catalogue, queried seeded
native data, closed, reopened, queried again and rendered relationships.
Post-shutdown `rrflow --json verify --project <estate> --level quick` passed.
No mock or raw filesystem read supplied the UI results. Existing local data
and the original user checkout were preserved.

Local artifact SHA-256 values (unsigned Windows development builds):

| Artifact | SHA-256 |
|---|---|
| rrflow.exe | `906147b144542cc5a907031045efe55c2533113be6ada5680e72aef4e68f372c` |
| connectome.exe | `7780a9ecec858c4b5bbbe9dfd2db492e0f75891d2935fc1a4de50c7f39c64107` |
| Connectome_0.0.5_x64-setup.exe | `c2f2f207e63119d4b125494cd2a42c8281cefb2b2ed4b0015b4972a2bd02e68c` |

Research/source adaptation: local canonical operations and actual executable
query behavior; no upstream code import. Debug/trace choice: existing stamped
responses, native stage events, real process logs and desktop inspection.
Resource choice: keep existing native budgets and bounded catalogue/row output;
no new performance claim or duplicate trace authority.

Surfaced failures: wrong strict/schemaless fixture mode, unregistered
schemaless record query, and sample-kind versus series-record-kind confusion
were exposed by real process tests. Fixtures and UI labels/selector were
corrected; the schemaless query gap remains open in POAM-011. Windows path/EOL,
UTF-8 editing and optional installer-banner issues were corrected before the
final build. Child journal retains details and inherited frontend warnings.

## Parent acceptance and disposition

Parent checks passed: `python scripts/knowledge/render_navigation.py` and
`--check` (14 indexes, 305 nodes, 1329 edges); `python
scripts/ci/build_execution_inventory.py` and `--check` (1209 records);
`python scripts/ci/check_documentation.py` (306 statuses, 304 coordinates);
`python scripts/ci/check_workflow.py` (3 workflows, 7 substantive jobs,
5 engine suites, 20 default and 6 optional packages); `cargo test -p rrd-engine
--test workspace_architecture --locked` (31 passed); `git diff --check`
passed. No parent runtime failure surfaced. Full
parent engine suites are delegated to candidate CI because this package changes
only a UI gitlink and documentation. Child real-process results do not replace
parent candidate checks. Complete H-06, signed/offline distribution,
clean-machine installer execution, remote/live fault/resource/accessibility,
PostgreSQL, media inference, DevPulse packaging and cloud synchronization are
not qualified by this package.

Change checklist: owning outcome and scope, baseline/diff inspection,
replacement traceability, first oracle, research/source provenance,
trace/resource/debug decisions, scoped implementation and failures, child
acceptance, full changed-file/diff reread and linked evidence are recorded.
The PR carries destination URL/ref/revision and candidate/merge receipts.
No objective or roadmap status changes; POAM-011 gains a verified deficiency.

The gitlink pins child candidate `03b716b4dfe5623e6c82408c6f4df367c9795388`,
which adds only a [README correction receipt](https://github.com/rrflow/connectome-development/blob/03b716b4dfe5623e6c82408c6f4df367c9795388/.agents/sprints/011a-workbench-readme-correction.md)
after the built implementation head above. Parent review/CI may run in parallel;
parent merge is conditional on that exact child passing CI and being merged
first. This immutable record does not claim a pending CI result; exact run and
merge evidence is attached to the respective PRs and local delivery receipt.
