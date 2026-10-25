# H-06 native knowledge bridge and Connectome viewport

**Status:** active immutable change-journal evidence
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-h/h06-native-knowledge-viewport`
**Owner:** OBJ-07 / H-06 authored client integration prerequisite

## Change brief and traceability

Starting engine revision e5672ec1d953c736b2befa38b34a13623a23a5d6; authored
client baseline abc549cf7bb61f2ed4a20353ccb2e3621da515c9. Work continued only
under D:/localdev/rrflow, preserving the owner's authored branch, independent
client history, original estate and unrelated changes. The pre-effect briefs
are recorded in draft [PR 16](https://github.com/rrflow/rrflow-development/pull/16)
and local .tools/knowledge-path-change-brief.md and
.tools/connectome-viewport-change-brief.md. No merge or promotion is claimed.
This follows the [foundation receipt](knowledge-tree-catalogue-foundation-journal.md)
and [existing-profile recovery receipt](h06-existing-profile-recovery-journal.md)
without rewriting their historical evidence.

Current behavior had engine-side tree contracts and authored editing source,
but the client needed the published SDK bridge and a usable bounded workspace.
Large status/lens panels scrolled with the working area; credential-path typing
persisted partial paths and could trigger native attempts; sign-out errors
were invisible. Target is an exact-instance native bridge and an understandable
workspace with persistent connection controls, accessible compact view buttons,
independent content scrolling and recoverable errors. Detailed identity remains
available in Engine details. The shared knowledge editor remains mounted across
view switches and preserves its CAS, merge, conflict and structural repair flow.

Reviewed owners: README, AGENTS, change-authoring/traceability procedure, H-06
client reference, foundation/recovery receipts. Complete changed new boundaries
were read: native rrd/client/knowledge.rs and command registration; rrflow/session
and knowledge models/tests; RuntimeConnection/index, session, style, knowledge
form; Section component; existing browser smoke suite. Assigned authors read
full native client/security/process-test and touched records/vectors/storage
boundaries. Full-file rereads and independent review followed the final edits.
The authored-source replacement map and earlier qualification are retained in
the child's docs/rrflow-native-rewrite.md; no alias or vendor wrapper replaces
an authored implementation.

Changed paths stay at their existing owners: child Cargo.toml/lock pin the real
published Git SDK; rrd/client/knowledge.rs implements typed SDK calls, rrd.rs/main
register them; rrflow/session supplies safe renderer views. RuntimeConnection
owns layout and lens selection, session.tsx owns native credential/session UX,
knowledge.tsx owns the existing editor presentation. Section adds an optional
compact setting with its old default unchanged; Records/Vectors/Storage use it.
Graph canvases size to the viewport; detail panels scroll rather than clip.
One shared openLens handler resets deep scroll without discarding knowledge drafts.
README bootstrap pin and client reference follow these actual source boundaries.

Credential text is now a local draft until explicit Sign in. Browse selection
cannot change the native target mid-request; attempts snapshot the exact profile,
endpoint and instance, and guard concurrent actions. Actual sign-in/sign-out and
file selection errors remain visible with recovery actions. Full profile identity
changes reset the draft. Existing native exact-instance/session guards remain.

Knowledge forms present names, source types and service names first; technical
identities, placement and limits remain accessible in accordions. A sticky Save
and feedback row shows unsaved/loading/conflict states. Provider labels retain
canonical IDs and explicitly describe setup-only, unverified configuration.
No secret values, provider calls, default new estate, engine restart, new cloud
API, fake progress or cancellation, or alternative authority is introduced.

The existing Vectors map is a bounded expensive read (up to 600 points with exact
neighbor searches). It continues after view changes and has no native cancel
operation. The UI now advertises background map building in its persistent
navigation, disables automatic whole-build retries/remount/focus/network retries,
and deduplicates explicit refresh with cancelRefetch:false. Its existing bounded
native behavior remains; no cancellation or performance qualification is claimed.

Research/source adaptation: not required for this local integration and layout;
existing official provider research remains a separate implementation brief.
Tracing/resource/debug: preserve native errors and existing engine diagnostics;
no raw knowledge/credential logging. Use compact D: Rust builds and existing
frontend budgets. Stop at any required new engine or provider authority.

## Verification and surfaced failures

The published A SDK was fetched through the normal locked Git dependency,
with no local path override or injected Git cache source. Native cargo check
passed. Native suite: 40 passed, one installed-process case explicitly ignored.
Separately invoking scripts/test-native-client.ps1 against the primary CLI
built from A passed 1, ignored 0: real plan/apply/authenticated readiness,
knowledge list, actual native vector collection/path save/read, stale-write
refusal, wrong-instance refusal, exact query and existing security denials.
The retained isolated fixture is 825daf1883ab4d48974cc11339b88849. It is not the
original estate and was stopped by the test finally block.

Primary CLI artifact rrflow-e5672ec1.exe SHA256:
8423528fc0cec016578a47ee0ed6499c3081e4a7e07c0295ecd227b2a4d7d044.
Its source tree is 3a95fe9558f747a39e303cba24dbf0e6ede79dd1. Build and process
evidence are under ignored .tools/rrflow-bin and .tools/knowledge-native-process-tests.
No binary is committed or promoted.

The D: desktop app with A SDK separately authenticated to the original
instance-07d784c6047ec8be023891a2 using the same credential copied to D: with
preserved restrictive file ACL; equality was checked without printing contents.
The original profile/query/engine were preserved. Live reads show local-operator,
271 records, 986 relations, cursor 1529, one collection/one vector and live storage.
Native interaction was deferred during the owner's active screen recording;
this is not proof of new GUI tree CRUD on that older engine.

Frontend pnpm check passes 600 files, full identity/dependency gates and tsc;
pnpm test:rrflow passes 28, zero failures/skips, including 9 knowledge editing
and capability/instance guards. Production build passes 3398 modules; the
existing large-chunk advisory remains. Existing production browser smoke passes 2
with public-bootstrap fixtures, explicitly not native/session/provider proof.
Responsive checks cover 1366x768, 1024x768, 390x844, 1366x360 and 390x600.
Short-height cases assert positive actual content scroll, fixed recovery-control
position and no horizontal overflow. The local HTML report retains screenshots;
the phone and short-laptop images were visually inspected.

Intermediate failures were corrected rather than hidden: the native CanonicalId
error conversion mismatch; an incomplete in-progress Accordion.Item close;
Windows default Python source decoding (edits rerun as UTF-8); live scroll
escaping the intended flex viewport; graph-side content clipping; a browser
strict locator matching both main and inspector checks (scoped to its dialog);
a PowerShell comma argument that prevented the HTML reporter from launching;
and an unnecessary-effect dependency lint (scroll reset moved to the explicit
navigation handler). Independent review found no further confirmed target,
credential, draft or data-loss regression after these corrections.

## Acceptance boundary and checklist

All source edits are within the existing optional client boundary and the
published engine contract. Owner/current/target/scope/stop decisions, source
reads and traceability, first failure oracles, source research decision,
resource/debug choices, actual checks and failures are recorded above.
Final pnpm check and production build pass on the settled source. Final browser
smoke passes two tests with all five sizes. Version policy and CI workflow policy
pass; navigation regenerated 14 indexes with 294 nodes/1265 edges. Parent inventory
and linked child revision are checked after the published child commit is pinned.

Not run: native knowledge-editor GUI CRUD, complete authenticated narrow-window
and keyboard qualification, external provider endpoints, live cloud/sync,
LFG/SWAG execution and recursive-learning promotion/rollback. They remain open
under H-06, Gate D/G and POAM-032. These local source/build/UI results do not
advance roadmap acceptance or constitute candidate CI, merge or release.


## Review finalization

The client source is committed as 8a9eb5e52b8df91b888b75f6c98edabeb8d3199d and pushed only to the recorded
Connectome development branch; [draft client PR 10](https://github.com/rrflow/connectome-development/pull/10)
carries the concrete implementation and qualification record. The parent pins
this exact reviewed authored descendant. Git destination/ref/revision were
recorded before each development push. Staging excluded build outputs, logs and
copied line-ending-only changes; normalized staged diff checks passed.

Parent navigation, documentation ownership and Git inventory checks pass
against the staged gitlink: 14 indexes, 295 document statuses/293 coordinates,
and 1196 execution records. Final normalized staged diff checks pass.

[Client CI run 39179914833](https://github.com/rrflow/connectome-development/actions/runs/39179914833)
reported four failures before any job step started. Its GitHub annotation says
recent account payments failed or the spending limit needs increasing. No job
logs exist because the jobs never ran. This is an account runner-start block,
not a passing candidate or a source-test result; no workflow was weakened or
billing setting changed. Parent draft PR 16 currently reports no checks.
Candidate CI, merge and release remain unclaimed. No roadmap/POA&M gate status is advanced.
