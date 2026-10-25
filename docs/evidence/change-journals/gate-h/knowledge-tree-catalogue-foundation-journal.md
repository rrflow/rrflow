# Knowledge-tree catalogue and provider-path foundation

**Status:** active immutable change-journal evidence
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-h/knowledge-tree-catalogue-foundation`
**Owner:** OBJ-07 / H-06 client prerequisite and D-06 explicit external-source configuration

## Change brief and source boundary

Baseline 470594178d25f1bf12d3ec6d9041488817a9e931. The owner requested work in
D:\localdev\rrflow while C: is copied, preserving the authored Connectome UI,
original estate and all unrelated changes. The local pre-effect brief is
.tools/knowledge-path-change-brief.md. The development branch and later draft
PR provide review; this record is not a merge, candidate-CI or release receipt.

Current behavior had real native vector storage and an internal optional
PostgreSQL/pgvector adapter, but no persisted user-managed logical knowledge
trees, public management operations or authored UI. Target behavior is typed
project/scope-bound tree/path configuration under one RrdEngine, with native
collection bindings or explicit external EndpointBinding configuration. A
saved external path does not establish provider availability or execution.
No engine version change, alternate store, compatibility alias, hidden vendor
engine, mandatory Connectome dependency or implicit network capability is added.

Reviewed README/AGENTS/change-authoring and traceability owners; Gate D/H and
POA&M; knowledge-package and endpoint contracts; complete new tree contract;
engine core/control/error/session/security/installation and function/vector
catalogue patterns; complete changed engine module/test registration files;
store control/repository implementation and control_journal tests; public
contract operation/security/schema catalogue, invocation mapping, HTTP routing,
capabilities/handlers, Rust client operations and transport tests; generation
scripts, SDK fixtures and reviewed count/digest oracles. Frontend owners read
the authored RuntimeConnection/session/vector/model state and full changed
source; their complete diff was reread. The native bridge is a subsequent
qualified SDK integration, not part of this completed engine proof.

## Implementation and traceability

- New rrd-contract knowledge_tree.rs owns group/native/external bindings,
  project/scope catalogue, graph validation, immutable identity, bounded Unicode
  labels, canonical digest, ListKnowledgeTrees and revision-checked replacement.
  It reuses EndpointBinding/CredentialReference; no secret value enters config.
- New engine/knowledge.rs owns installed identity and session authorization,
  instance scope, current native collection/vector existence checks, bounded
  state decoding, integrity, durable CAS and exact accepted-response replay.
  Catalogue and receipt commit through existing ControlRepository::commit_batch.
  Neither a stale write nor replay can overwrite later changes. Native source
  availability is checked at save and must be revalidated for later execution.
- Existing executable public catalogue owns knowledge-tree-list and
  knowledge-tree-replace, their security actions, schema and HTTP dispatch.
  Rust client methods validate requests/responses. No independent operation
  registry or dynamic JSON dispatch was created.
- Public capability knowledge-tree-catalogue v1 advertises experimental
  installed configuration and explicit unavailability for unbound engines.
  The two operations and schemas regenerate into all existing SDK projections.
- Authored Connectome source adds capability-gated editing and native session
  requests, with immutable identities, retained drafts, 3-way conflict handling
  and structural repair. Native SDK integration/live UI CRUD remains separate
  evidence. Older4705941 estates issue no new tree route calls.
- Knowledge/provider and inference-extension references record the owner's
  HydraDB-first/Nexus-next selection, SWAG System One role, LFG classifier and
  governed recursive-learning track. Gate G wording and G-07/G-08 now name
  their configuration/promotion/rollback acceptance; none is marked complete.
  POAM-032 records verified provider/cloud/sync gaps. This design clarification
  does not implement those extensions.

Existing implementation paths are extended rather than removed. Knowledge
package export semantics, all previous operations, engine storage/security
composition and authored Records/Vectors/Storage lenses remain intact. Trace
and control journal receipts are retained; raw knowledge payload/secret logging
is not added. Resource budgets cover trees/paths/depth/encoded state and replay
receipts. Source adaptation is limited to official API research, not imported
vendor implementation. No external provider or cloud account was invoked.

## Commands, failures and evidence

Compact Rust builds used the canonical D: target, four jobs, no compiler wrapper,
no incremental objects and debug=0; large parent dependencies were built fully
on D: rather than claiming a cache/source-only substitute.

- Contract: cargo test --locked -p rrd-contract —110 passed, none failed/ignored;
  includes13 new tree contract tests. First run exposed a stale33-operation
  SDK conformance count; it was updated to the reviewed35 catalogue, preserving
  strict parity rather than suppressing the check.
- Engine: cargo test --locked -p rrd-engine knowledge --lib —13 passed;
  full cargo test --locked -p rrd-engine -- --test-threads=4 —170 passed
  (108 unit+62 integration), zero failed/ignored, doc0. First focused run
  had12 pass/1 fixture failure due to reused request/operation coordinates
  across principals; corrected only those test coordinates before the pass.
- Store: cargo test --locked -p rrd-store --test control_journal —3 passed,
  proving existing KV/MX batch-CAS atomicity and journal recovery.
- Public integration: cargo test --locked -p rrd-client --test knowledge_trees
  —1 passed, zero failed/ignored. Real authenticated HTTP against an explicitly
  disposable installed engine proves descriptor/product rows, exact project,
  read/write denials, CAS, exact replay, idempotency conflict and engine/HTTP
  restart persistence. It does not seed or use the owner's original9477 estate
  and does not claim a launched release binary or external provider test.
- Existing server: --lib --test http_process —19 passed. Existing client:
  --lib --test real_server --test transport_faults —7 passed; includes real
  loopback/mTLS and correlation, size, sequence and bearer-redaction denials.
- All five SDK generators passed byte checks; canonical generated-surface
  parity passed with35 HTTP operations and unchanged kernel signal fingerprint.
  New OpenAPI digest:9ff05ee56d8b17408eea2a5491aeb63140bc7852e37473b556d4be0df3c53c6d.
  Reviewed count/digest goldens and six existing authored citations were updated.
- TypeScript SDK standard check passed on officially checksum-verified
  Node24.22.0, pinned pnpm12: generation, Biome, tsc and5 tests. Go1.27.2
  generation/check and go test ./... passed. Modified Python lint/format passed
  using pinned Ruff0.17.1. Earlier ambient pnpm/Node/Ruff were too old; exact
  declared D: tool runtimes corrected the environment without source/lock downgrade.
- Authored frontend: pnpm check passed600 files, identity/dependency gates and
  tsc; pnpm test:rrflow passed28 tests including9 knowledge editing/guard cases;
  pnpm build passed3398 modules. Existing large-chunk advisory remains.
- Parent cargo fmt --all -- --check passed. Frozen version1.0.0, workflow policy,
  six inventory regression tests, regenerated navigation and documentation
  ownership checks passed. Final source inventory is regenerated after staging
  exact meaningful source paths; pnpm caches are excluded, not source inputs.

## Limits and follow-through

The original9477 recovery has its own [receipt](h06-existing-profile-recovery-journal.md)
and real271-record/986-relation evidence. It remained untouched by these tests.
Live native knowledge CRUD, the normally published/pinned SDK revision and the
final optional-child gitlink are subsequent integration evidence; they are not
inferred from frontend compilation or HTTP package tests.

HydraDB, Atlas and Nexus execution; document ingestion; general secret resolution;
bidirectional synchronization; turnkey cloud provisioning/auth; recursive
learning/LFG/SWAG execution; complete all-surface conformance; candidate CI and
release/distribution gates remain unqualified. Official Hydra v2 and Nexus
research requires observed external provenance rather than fabricated native
snapshot freshness, and exact source/ACL enforcement before account calls.
No roadmap or POA&M row was closed. The new POAM-032 and Gate G planning rows
make the remaining owner-directed work explicit rather than claim parity.
