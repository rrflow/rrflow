# H-07 DevPulse cloud and homelab delivery scope

**Status:** active immutable change-journal evidence
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-h/h-07-devpulse-cloud-homelab-scope`
**Owner:** OBJ-09/OBJ-10 delivery prerequisite and H-07/J-04 scope correction

## Baseline and bounded change

Starting public development revision: `f948abb108f849f8947ccc2b99ef6b9844a13915` (the fetched main revision).
The owner clarified that the target is a competitive native AI database/service
integrated with their DevPulse project VM/container application, with real
bidirectional synchronization to shippin.cloud or a homelab. The previously
built Windows development engine/Connectome bundle is useful local evidence;
it does not establish that complete product. No new runtime behavior is
implemented or qualified by this documentation change.

The local change brief preceded edits. GitHub needs a differing source commit
before a PR can exist, so the first bounded documentation commit creates its
PR. No runtime code is changed before or after that prerequisite exception.
Stop conditions: invented endpoint/provider availability, unsupported readiness
claim, competing owner, destructive migration or unqualified source adaptation.

Current to target: correct the cloud-client-only framing; identify DevPulse
project provisioning and managed/self-hosted destinations; separate native
database sync, selected source/asset transfer and environment reconstruction;
define the native receipt/deduplication dependency before client integration;
and bind competitive claims to existing J-04 evidence. The one-engine and
independent-install invariants remain intact.

## Inspection and decisions

Owner records reviewed: README, AGENTS, alpha objectives, instance topology,
system-overview delivery sections, the complete DevPulse integration contract,
rrflowDB capability/proof sections, Gate H, Gate J, dependency spine, POAM-011,
and the complete change-authoring procedure. Implementation inspected:
`rrd-engine/src/engine/transaction.rs` begin/idempotency boundary and repository
searches over public contracts/engine code for synchronization/import/binding.
The transaction identity binds session and idempotency key. It is not proof of
cross-instance source-origin deduplication or an admitted sync-import API.

Read-only application discovery found multiple DevPulse checkouts. The
`D:/localdev/platform_devpulse` checkout identifies the platform repository,
a Tauri/Svelte client and `systems/devpulse-engine` facade; its architecture
describes existing persistence/model services. Its current checkout had an
untracked `slots/` directory, which was left untouched. Other DevPulse checkouts
also exist; the authoritative VM/container app checkout was requested from the
owner and is not inferred from a folder name. Existing Shippin root README,
AGENTS and selected module references identify infrastructure/project-workspace
boundaries. No external application, nested repository, live service or
credential was changed. Searches in selected paths do not prove that a feature
is absent from every user repository.

The architecture skill was used to assess boundaries and consequences. Existing
owners already cover the decision, so no parallel ADR or product/status ledger
was added. Local account-independent operation and operator-controlled homelab
identity are required; managed account sign-in does not authorize transfers.
Native sync receipts belong to the engine, infrastructure receipts to the
configured provider, and source/environment transfer to authorized project
adapters. More qualification work is required in exchange for preserving those
boundaries and truthful partial-failure reporting.

Primary-source research reviewed SurrealDB's model/deployment overview and
Pinecone's architecture/production guidance; Gate J links the exact pages.
They inform test dimensions only. No upstream code, architecture or dependency
was imported. The meaning of the owner's additional term "Nexus" remains a
requested clarification; no product identity or capability was fabricated.

## Effects and verification

Changed authored paths: README; system-overview delivery definitions;
`docs/reference/client/devpulse-cloud.md`; rrflowDB capability benchmark list;
Gate H H-07a through H-07g prerequisites/oracles; Gate J delivery/competitive
qualification; POAM-011. This receipt and generated nearest index/inventory
complete the package. No source, test, binary, version or deployment changes.

First oracle: the owning contract must name local/managed/homelab destinations,
both sync directions, independent transfer concerns, native atomic origin
receipts, snapshot/changefeed handoff, conflict/delete/restart behavior and
actual-provider acceptance, without claiming they exist. Diff review confirms
those requirements and explicitly retains the unimplemented status.

Trace/resource/debug decision: preserve engine authority and require bounded
transfer/backpressure and correlated source/destination receipts in the target
contract. Runtime instrumentation and measurement are not added for a prose
change. Local acceptance passed:

- `python scripts/knowledge/render_navigation.py`: 15 generated indexes,
  314 nodes, 1376 edges; nearest journal index updated.
- `python scripts/ci/build_execution_inventory.py`: 1228 records generated.
- `python scripts/ci/check_documentation.py`: 315 status records and
  313 coordinated records passed.
- `python scripts/ci/check_workflow.py`: 3 workflows, 7 substantive jobs,
  5 engine suites, 20 default packages and 6 optional packages passed.
- `git diff --check`: passed.

Review corrected a circular package dependency: H-07d implements the service
artifact requirements which J-03/J-05 later qualify; final J qualification
cannot be its prerequisite. No runtime failure was repaired by this package.
Read-only discovery also encountered invalid Git command arguments and searches
for nonexistent split contract files; these were corrected by inspecting the
actual source layout and are not product failures. Runtime tests,
competitive benchmarks, real accounts/homelabs and DevPulse provisioning are
not run because no implementation or deployment changed; candidate CI remains
the source-landing gate, not product readiness evidence.

Checklist: owner/baseline/scope and stop conditions, source inspection,
characterization, research/provenance, trace/resource decision, exact effects,
limitations and linked evidence are recorded. Authored sections and the complete
diff were reread. No objective or roadmap item is completed. POAM-011 gains
specific missing delivery requirements; its lifecycle status is unchanged.
The PR records the destination URL/ref/revision and subsequent CI/merge result.
