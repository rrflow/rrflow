# Installed project indexing: implementation and evidence plan

**Status:** historical; superseded on 2026-11-13 by the [kernel contract](../kernel-contract.md), which carries every package listed here; kept as the record of the work and decisions it describes
**Superseded by:** [RRFlow kernel contract](../kernel-contract.md)
**Refreshed:** the [D-02 evidence-driven attunement plan](../d02-evidence-driven-attunement-engineering-plan.md) of 2026-11-11 rebinds these packages to main `ab32ec16` after the drafts it reviewed were absorbed; where the two differ on order, the newer plan's acceptance ledger governs
**Coordinate:** `rrflow://rrflow-instance/data/design/installed-project-indexing-plan`
**Owner:** [Gate D project workflow](gate-d.md#project-workflow-build-packages); [alpha objective](../../objectives/rrflow-1.0-alpha.md#observable-engine-objective); [POAM-006](../../poam/rrflow-1.0-alpha/poam-006.md)
**Reviewed:** public main `3a72a6b4adc43c842f67a59f4fc91d725eac7580`; parser draft [#241](https://github.com/rrflow/rrflow-development/pull/241) `ba11fdb35d5a080ab3e66361b89f942064d75d39`; interrupted-refresh regression [#252](https://github.com/rrflow/rrflow-development/pull/252) `28afd7a047100c7eb8cf36d5d27f268551384bc1`; inherited-directory draft [#247](https://github.com/rrflow/rrflow-development/pull/247) remains separate and incomplete.

This is an implementing-agent handoff for the first real project-to-query cycle. It refines the [adaptive developer design](adaptive-developer-build-design.md), not the release order, alpha objective, POA&M status, or a fixed runtime sequence. Refresh the revisions, PRs, their CI and source references before implementing each package. A local test or a proposed symbol below is not evidence that an installed workflow works.

## Exact result to build

A person selects an existing project, sees the proposed engram path, exclusions, inherited access, work limits, optional capabilities and exact effects, and accepts the preview. The installed engine creates or reopens its own project identity and map, reads permitted source without moving files or changing inherited ACLs, and reports background progress. It durably publishes an inventory and indexed source spans with provenance. After a restart, an authenticated rrflowQL/context request returns the same permitted source evidence and read stamp; Connectome displays it using the public engine contract. An edited or deleted file invalidates only dependent current results and can be reprocessed without losing unaffected committed work. A second project cannot see the first project's records.

The project map is a versioned, replaceable discovery projection of the
engine-owned project identity, source bindings and committed inventory, not a
second planning or lifecycle owner. The installation preview must name its
exact path and replacement/removal policy before an authorized write; a missing
map can be regenerated from the engine, and a modified or foreign file cannot
silently change engine authority. The implementing PR must first identify
whether an existing template or public operation already owns this projection;
the inspected installation paths did not establish a current automatic map.
Minimize upfront questions to the project boundary and decisions genuinely
needed for the selected effect. Continue permitted scanning and indexing in the
background with visible progress; an unanswered optional provider question
must not stop local inventory. Idle/soak work uses configurable foreground
resource ceilings and must pause or yield when those ceilings require it.

The first joined proof may run without a model or vector provider: lexical and graph retrieval with source citations must work, and unavailable vector/inference capability is reported. The complete alpha objective additionally requires vector access, configurable reasoning and other named gates; a lexical slice cannot close it. Knowledge-path connectors, LFG, SWAG and provider capture are explicitly configured integrations; the default installed engram cannot depend on them. Parquet supports analytical exchange and the requested future cold-storage lifecycle, not the WAL or a prerequisite for a small local project.

The existing installed-process driver is
`crates/adapters/rrflow-cli/tests/installed_lifecycle.rs` test
`installed_engine_plans_applies_serves_discovers_authenticates_and_reopens`.
Its recorded run on parser draft `77fd7e8fde316cfd22ec1c367aaf75de50db5539`
reached Parse and failed `phase-unavailable`; that result is not a test run of
the later draft. The dispatcher still has no Parse executor at the reviewed
candidate `fa2409867fd906a85f5b98b2e512107dd2044c08`. Preserve this driver and
extend it to check actual persisted span/query readback, edits and restart. The
native supervisor components are prerequisites; rerun the joined driver after
its execution boundary changes, and retain the failing result until it passes.

RRFlow's intended fast prompt cycles and long-running multi-day work both use
the same authority, persistent state and configurable effort/resource policies.
Long jobs need durable checkpoints, bounded leases and heartbeat/recovery,
interruptible execution, provider timeout/uncertainty reconciliation,
accumulated budgets and evidence readback; merely leaving a model call running
for days is not a durable task. LFG and SWAG connect through explicit engine
contracts when available. Their planned presence does not demonstrate reasoning
quality, latency or parity with SurrealDB; measure those separately on defined
workloads after the installed project loop works.

## Evidence map: what exists and what is missing

| Boundary | Inspected implementation and limitation | Change destination |
|---|---|---|
| Project admission | `crates/authority/rrd-engine/src/engine/installation.rs` and `installation/inventory.rs` scan under a compiled policy. `AnchoredPath` and observations constrain source access. The current scan retains entries and is not an atomic filesystem snapshot. The existing-project branch #247 still has unresolved interrupted staging and ACL re-admission. | Keep preview/apply, source anchoring and installed identity in `RrdEngine`; reconcile retained directories without altering project ACLs, moving existing entries or replacing user data. |
| Default plan | `assets/install/default-profile-v3.json` and `clyffy-profile-v3.json` contain the same eleven phases; `installation.rs` copies the profile absent a supplied proposal. `rrd-contract/src/attunement.rs` validates a linear phase chain ending in Verify. | Replace fixed default selection with available, configured engine capability/effect selection and versioned dependency-bound records. Preserve traceability of existing formats and characterize them before removal. |
| Inventory | `attunement/inventory.rs`, `inventory/records.rs`, `publication.rs`, and `reuse.rs` bind proposal, source recheck, publication groups, lease and head. The final transaction links job checkpoint, receipt and head to the runtime read stamp. | Reuse the same canonical runtime and transaction authority; prove partial groups are not presented as a complete head. Make large-snapshot reads bounded rather than reconstructing the entire proposal. |
| Parse | Main's `engine/attunement.rs` executes Connect and Inventory and returns `phase-unavailable` for other phases. Draft #241 adds `rrd-attunement/src/parse.rs` and `worker.rs`, a selected policy, bound source reading, response validation, Windows/Linux child-containment foundations and a Windows pipe experiment. The Windows pipe experiment has an unresolved concurrent blanket-inheritance risk and no integrated deadline or executable admission. It supplies neither committed span publication nor installed Parse execution. `records::load_for_parse` currently reconstructs the whole Inventory proposal with a size bound. | Complete #241 or carry its characterized behavior into the adaptive capability contract. Do not activate until native worker admission, source/lease recheck, transaction publication, replay and the real installed test pass. |
| Query and prompt context | `engine/query.rs`, `engine/context.rs`, `engine/retrieval.rs`, `engine/vector/search.rs`, and `rrd-query` contain real query/retrieval components. The draft `parse_source_is_bound_to_the_committed_inventory_after_reopen` traverses the committed Inventory relation after reopening; it does not publish parsed spans. `context.rs` currently fixes RRF constant and seed/text/vector/graph weights in code. | Read the same persisted source/span identities through scoped rrflowQL and context, then make ranking/graph selection a versioned bounded policy with effective values and origins visible. Check actual D-05/E/F prerequisite evidence before claiming access paths qualify. |
| Parquet | `engine/query_export.rs` rejects truncated results and currently bounds the export at 1,024 rows. `rrd-query/tests/parquet_export.rs` characterizes typed bounded files. | D-06 owns exchange/import; F-06 owns the missing background cold-storage lifecycle. Neither bounded export nor a file alone proves WAL durability or project indexing. |
| UX and integrations | The pinned Connectome's `src-tauri/src/rrd/client.rs` and `engram.rs` contain client/preview paths; static phase declarations in `src/rrflow/attunement.ts` do not prove a live screen. The pinned UI has not been fully qualified for this journey. | Audit the exact pin and wire existing/public generated operations to engine state. Add client-facing status only where the authoritative operation exists; no UI-owned lifecycle or plan. |

## Source and effect model

The engine owns a versioned project identity and installation record, an admitted root/knowledge-path binding, a policy revision, a source snapshot/read stamp and durable job/receipt records. Record these distinct identities in every derivation: `engram_id`, `project_id`, `source_binding_id` and revision, root-relative path and stable entry ID, content digest, snapshot/change-set digest, parser/grammar revision, effective configuration digest, authorization revision, plan/effect ID and read/commit coordinates. Do not use filesystem path text alone as a durable authorization token.

Source spans use bounded byte offsets and the content digest of the exact source observed. Syntax ERROR/MISSING nodes remain evidence, not silently repaired source. Extracted entities and relationships must distinguish deterministic syntax facts from model-derived proposals, carry provenance and model/configuration identity, and avoid representing guesses as verified facts. New source-derived rows, relations, index entries and invalidations go through the existing engine mutation/transaction owner. An index is derived from the canonical records and rebuildable; neither a local cache nor a vector service is a second semantic authority.

Treat an effect's dependency set as more than file hashes when it depends on directory membership, a negative lookup, policy, access grants, tool version, configuration, a question answer or a query range. Re-observe those inputs and the lease immediately before each publication or external mutation. A mismatch refuses that effect without publishing it; record why, propose a successor linked to the old plan, and retain committed receipts. An unrelated edit must not invalidate an effect whose complete declared dependencies are unchanged. If dependency completeness cannot be established, refresh the relevant boundary conservatively and report the weaker guarantee. Do not claim filesystem check-and-write is atomic when the OS boundary does not provide it: use anchored handles/identity checks and test the residual race.

Groups may be committed incrementally only with a durable job-bound cursor, idempotent group identity and bounded aggregate input/output/resource accounting across retries. Publish the new complete head only after all groups and their dependencies are verified. Intermediate groups must be unreachable as a complete current index. On restart, reconcile committed groups and the existing head before resuming; stale workers fail their fence. For delete or changed permissions, retire/tombstone dependent current results at an authorized transaction boundary, retain historical read-stamp behavior as supported, and deny newly unauthorized reads. No unaudited fallback to old current results.

## Handoff shape and enforcement

The following is a **semantic example**, not an implemented wire schema. Derive the eventual versioned schema and SDKs from the executable contract owner. A schema alone cannot force an external agent to obey a plan; only engine-mediated effects can be rejected at admission, and an independent checker must compare actual external changes and receipts.

```json
{
  "version": 1,
  "engram_id": "example-engram",
  "project_id": "example-project",
  "plan_id": "example-plan",
  "plan_digest": "<canonical digest>",
  "predecessor_digest": null,
  "configuration_revision": "<digest>",
  "read_stamp": "<engine-issued coordinates>",
  "questions": [{"id": "q1", "state": "answered", "answer_digest": "<digest>"}],
  "effects": [{
    "id": "effect-a", "capability": "<registered name and version>",
    "inputs": [{"kind": "source", "identity": "<entry id>", "digest": "<content digest>"}],
    "depends_on": [], "required_questions": ["q1"],
    "authorization": "<grant revision>", "budgets": {"input_bytes": 1, "output_bytes": 1, "elapsed_ms": 1},
    "idempotency_key": "<unique key>", "verification": "<observable predicate>"
  }]
}
```

The numbers are placeholders, not recommended budgets. Validate structural bounds, canonical encoding/digest, supported capability and typed parameters, acyclic dependencies, scope, authorization, relevant source observations, questions, aggregate budgets and expected revision at the actual effect boundary. The list contains only effects selected for that request; empty and differing selections are valid when no effect is necessary. Exploratory reading and private reasoning are not forced into this schema. Unknown effects fail closed, while unfamiliar non-effect ideas remain representable. Durable result records bind the actual outcome, source/read/commit coordinates, actor, memory changes, errors and successor lineage; a sampled trace is not an audit ledger. A lost external acknowledgement is uncertain until idempotency or readback establishes the result.

For implementing-agent handoff, each PR description and linked journal must
also identify: owning alpha outcome and roadmap package; baseline revision and
current behavior; intended public behavior; exact affected source, existing
characterization tests and canonical destination; input/effect and provenance
model; failing oracle and exact commands; trace and resource decisions; test
results and unrun checks; explicit stop conditions; and per-effect evidence or
reason for a successor plan. This is a repository authoring record, not a new
runtime lifecycle. An agent must compare each proposed change with that record
and update it before changing scope. The reviewer independently compares the
actual diff, command output and persisted receipts with the record. A prose or
JSON document cannot itself prevent an actor with direct filesystem access
from bypassing the engine; do not describe it as such.

## Implementation packages and first failing oracles

These packages are review/merge units following the existing D-02/D-03/D-04/D-05 and H-06 owners, not required runtime phases. Before code, write each PR's brief, exact source/test map, format-compatibility decision, smallest red test, trace/resource decision, stop condition and linked journal; characterize existing behavior and retain it at the canonical boundary. Reconcile overlapping branches rather than starting competing edits to generated surfaces.

1. **Admission and adaptive work selection (D-02/D-03).** Characterize current default profiles, jobs, serialized digests and actual installed preview/apply. Map and replace the closed phase assumptions in `rrd-contract/src/attunement.rs`, `rrd-engine/src/engine/installation.rs`, `engine/attunement.rs`, their tests and generated public projections. Use one engine-owned versioned capability owner, selected dependencies and per-effect source/read identity. Define what happens to an existing pre-release engram/job before changing its format; do not invent a second lifecycle. First red oracle: two different projects or policies select different available work, unknown effect is rejected, no provider is activated, and an unrelated edit does not trap a completed independent effect. Preserve exact-preview authorization and do not silently change ACLs. Coordinate #247's staging/ACL recovery before relying on retained directories.
2. **Bounded snapshot access and incremental freshness (D-03/D-04).** Characterize `installation/inventory.rs`, `attunement/inventory/{records,publication,reuse}.rs` and `rrd-attunement/src/inventory.rs`. Design a bounded, authenticated, ordered/paged membership traversal at the committed stamp; authenticate each page/head, charge keys/bytes cumulatively, and preserve snapshot completeness and existing receipt/head atomicity. Avoid materializing every record or assuming the whole snapshot fits one write transaction. First red oracle: a deliberately small per-group budget processes a multi-group fixture, restarts between groups without duplicate publication, and refuses head substitution or changed relevant input. Then measure the roadmap's 100,000-record profile on an actual installed engine; no large-scale claim follows from a small fixture.
3. **Isolated parser and source publication (D-04).** Build on reviewed #241 bindings and framing; `engine/attunement.rs` remains the authority. Pin the shipped worker executable and digest, admit only explicit inherited handles, apply an OS-enforced memory limit before worker code runs, enforce time/cancel by terminating and reaping the exact child, and reject truncated/late/forged/out-of-budget responses. Windows needs suspended creation plus job assignment before resuming and a tested parent-death cleanup strategy; Linux needs a pre-exec limit and separately proven parent-death cleanup. Do not claim `RLIMIT_AS` is resident-memory accounting. Unsupported platforms fail closed until qualified. Re-read the authorized source/checkpoint and fence before publishing bounded span/relationship groups. First red oracle: the real primary executable indexes one source, kills a worker mid-request and between publication groups, restarts, and returns exactly one final source/span head; substitution, revoked access and source edits publish no stale result. An orphan between process creation and containment is a stop condition, not a tolerated race.
4. **Source-derived retrieval and configuration (D-05/E/F/H-09).** Map actual `rrd-query` graph/BM25/vector paths and `engine/{query,context,retrieval}.rs` before changing them. Publish versioned source chunks/spans and deterministic structural relations under the same rrflowKV transaction authority; link lexical and graph indexes to those identities and the committed read stamp. Make retrieval avenues, fusion weights, graph depth, limits and optional vector/model selection explicit versioned effective configuration. Preserve authorization before candidate generation. First red oracle: an installed engine indexes two files, a distinctive query returns the correct source identity and byte span after reopen, a deleted or forbidden file no longer appears at current scope, and two valid policies expose different effective selection without inventing an unavailable vector result. Qualify native vector/embedding and TurboQuant separately against exact filtered search and held-out tasks; do not equate a vector library test with source-ingestion proof.
5. **Job, question and evidence surfaces (D-02a/H-04/H-05).** Extend the existing operation catalogue and engine job/receipt stores with the authorized list/read/status/resume/cancel, question and receipt capabilities required by OBJ-14. A question blocks only effects that name it; background progress exposes counts, budgets, exclusions, error/paused state and the last committed head. Record memory reads/writes and uncertainty in durable readback, correlated by plan/effect/job/source IDs. Traces can be sampled and contain no source or credentials by default. First red oracle: kill the installed process before/after a committed job transition, replay an identical request, and compare visible job state and receipts with actual mutations; no duplicate effect or missing acknowledged memory receipt.
6. **Installed joined proof and Connectome (H-06/J).** Add an executable integration driver to `crates/adapters/rrflow-cli/tests/installed_lifecycle.rs` or its existing support modules; the precise new test name/command is a deliverable, not an existing command. It must launch the built `rrflow` binary, use public authenticated operations, and retain exact binary/UI/source/configuration digests, platform, request/receipt IDs, read stamps and raw results. Run the fixture, ordinary edit, deletion, crash/reopen and independent checker; repeat with a separately selected real project and two-engram isolation. Audit and test the exact Connectome pin using only generated/public client operations. Until an actual desktop is launched and compared with engine readback, UI acceptance remains open.

The first executable target is package 3's real source/span result joined to packages 1–2, then package 4's retrieval and package 6's installed proof. Preparation for the latter starts with its failing driver, not after a collection of isolated green unit tests. This dependency order does not require eleven runtime operations or prevent independent read-only exploration. The repository's existing D-05 sequence and gate prerequisites still govern acceptance.

## Execution reset: one integrated developer build

This section is an implementation handoff, not a new roadmap or a sequence
of runtime reasoning stages. One named integration driver owns the installed
project journey through the PR dependency chain. Refresh all heads before
work; the following are observations, not merge claims:

| Candidate | Actual state at this review | Integration action |
|---|---|---|
| Public main `3a72a6b4` | Fixed eleven-phase defaults; installed Parse unavailable; C-07 open. | Use as the source baseline, not the old root checkout. |
| [#252](https://github.com/rrflow/rrflow-development/pull/252) `28afd7a0` | Red Windows real-engine regression: an interrupted refresh leaves the old completed head but exposes a newer root record at `KNOWN HEAD`; the checkpoint cursor still reads the old record, including after reopen. No repair. | Complete the generation-visibility contract below and its tests; land through exact candidate and merge-group checks. |
| [#241](https://github.com/rrflow/rrflow-development/pull/241) `ba11fdb3` | Parser and private native foundations exist. A source-built Windows primary binary returned a syntax span through a job and pipe. Installed project test still fails `phase-unavailable`; the Windows inherited-handle experiment has an unresolved concurrent-launch risk. | Integrate #252 by ordinary merge, resolve the shared source/test files and regenerate projections. Complete worker admission and durable source publication before readiness. |
| [#247](https://github.com/rrflow/rrflow-development/pull/247) | Existing-directory work is a separate incomplete draft; earlier review found staging and ACL re-admission gaps. | Characterize and complete in dependency order when the chosen real project needs it. Never move user entries or change inherited project permissions to make admission succeed. |
| This design #243 | Supporting design and workflow changes only; draft CI is skipped. | Reconcile it with the final implementations; documentation is not an alpha or merge prerequisite unless the owning PR requires it. |

The installed driver is
`crates/adapters/rrflow-cli/tests/installed_lifecycle.rs::installed_engine_plans_applies_serves_discovers_authenticates_and_reopens`.
At #241 `b5fa4499`, Windows reached and queried the committed Inventory
identities after forced restart, then failed on Parse. At #241 `ba11fdb3` the
same failure remained. The controlling outcome is a **new passing installed
source-to-query assertion** in this driver, including actual parsed spans,
restart, edit/delete and independent expected source bytes. A pass through
Inventory alone is not that outcome. Keep the failing test visible; do not
mark an incomplete draft ready solely to obtain an expensive candidate run.

### Resolve completed-generation visibility at the storage and query owners

The current `inventory/records.rs::mutations` writes records under stable
logical IDs, `publication.rs::Publication::publish` commits groups before
`finish` changes the head, and `records.rs::load_prior` reads the stored
checkpoint stamp. The pointer alone does not isolate head-time reads of those
same IDs. A current snapshot and its entries must be selected together; the
root digest, content identity, membership, graph and index cannot come from
different generations.

Use **generation indirection in the existing rrflowKV/rrd-store authority** as
the implementation target: bounded private generation data is durably written
without becoming current semantic records; an authenticated, bounded manifest
names its ordered membership and source/configuration identities; the existing
runtime/control transaction atomically selects the complete generation with
the job checkpoint and receipt. A reader captures one authenticated selection
with its read stamp and resolves source records, containment and derived
indexes through it. Distinguish stable project/source identity from immutable
physical fact/version identity. Reuse unchanged content-addressed facts where
the complete dependency set is equal. Intermediate, abandoned and superseded
generations are not current merely because their bytes were committed.

This is a design to implement and prove, not an existing substrate feature. It
must not create a second database, a client-owned head, or an untracked source
of semantic truth. Implement selection as a versioned engine-owned semantic
event committed in the **same** transaction as the complete head, checkpoint
and receipt. Its bounded payload binds the scope, generation/root digest,
manifest/page count, source/configuration/plan identity and predecessor. The
existing `ReadStamp` cursor and authenticated runtime accumulator must be
shown to cover that event; do not change its wire format merely to add another
head. At an arbitrary requested cursor, `rrd-store` resolves the latest authorized
selection at or before that cursor through a bounded typed index and verifies
its inclusion, then authenticates selected generation pages against the
event's root. A missing or corrupt proof fails the read. Restrict publication
of the selection event and its reserved data types to the existing engine
authority; a caller's arbitrary event cannot select a source generation.

Implement that resolver and bounded physical reads in
`rrd-store/src/repository/runtime.rs`, `access/versioned_read.rs` and
`access/semantic_commit/`, with any required schema/index definitions at their
existing owner. The current `ReadStamp` and accumulator do **not** yet prove
this proposed relationship. Query/export evidence must report the actual read
stamp and selected generation; a selected source generation is part of the
logical state at that stamp, not a silent rewind of every other record. A
design that requires the final transaction to contain every source entry,
edge, deletion or member ID at arbitrary admitted scale is not bounded
publication. Prove atomic selection, inclusion and recovery before enabling
the resolver; if the existing event/index mechanism cannot establish those
properties, revise this PR brief and retain the red oracle.

Make the change visible consistently in `engine/query.rs::execute_query_scoped`,
`poll_live_query_page`, `ensure_query_index`, `engine/query_export.rs`,
`engine/retrieval.rs`, `engine/context.rs`, and the corresponding `rrd-query`
pipeline, GraphQL/client and index paths that can consume these records. Map
each actual call before editing: source-only, traversal, joins with unrelated
records, literal/parameter/HEAD cursors, live queries, exports and direct
diagnostic reads need explicit semantics. An unqualified scan must not claim
or leak an abandoned generation as current. A mixed query must preserve one
honestly reported view; it may return an explicit unsupported/error result
until that combination is implemented, but must not silently rewind unrelated
data or pretend a second cursor is the caller's requested HEAD. Preserve
authorization before candidate generation and recheck newly revoked access.

Replace the monolithic `entry_ids` reconstruction in `inventory/records.rs`
with ordered bounded pages and authenticated membership; charge aggregate
keys, bytes, elapsed time and output across pages and retries. Store only the
small root/count/page commitments in the final atomic publication. Deleted
entries and obsolete indexes leave the selected generation without requiring
one oversized final retirement batch; old retained read stamps remain valid
until their existing retention policy permits reclamation. A live reader pins
the generations it needs; cancellation cannot publish a partial head, and
reclamation cannot remove pinned data. Measure write amplification and retained
bytes rather than assuming zero-cost reuse.

Before replacing the current record/link representation, characterize the
existing stable logical IDs, serialized proposal/head digests, deletion and
reopen tests, and the public queries used by #241 and the installed client.
The same canonical reader must authenticate already committed pre-release
snapshots and their old read stamps; new writes use the new representation.
For an existing engram, admit an explicit engine-owned initial selection from
the previously completed, authenticated head, and atomically record it before
selecting any new generation; do not treat an incomplete progress group as a
completed snapshot. The old checkpoint continues to use its original retained
read stamp. Test the initial selection, interrupted upgrade/retry and an engram
with no completed head; a purely implicit conversion is not evidence. Do not
silently rewrite an engram, require reinstall after an ordinary edit, or
introduce a separate legacy engine. If this cannot be achieved at the existing
authority, retain the failing test and resolve the contract before declaring a
repair.

Extend #252's real-engine oracle to require: old snapshot/root and unrelated
HEAD data remain correct after each interrupted group; the selected generation
changes exactly once after completion; a different edit produces a successor
without discarding unaffected receipts; deletes and permission revocation do
not appear current; old pinned readers retain their old results; restart and
same-key replay do not duplicate publication; competing jobs/changed source
and lease reject before selection; and raw scans, joins, export, live and index
reads cannot bypass the selection. Include a deliberately small page/batch
fixture, then the owning 100,000-record profile. The existing 2,048-file
regression is real Windows evidence, not a 100,000-record pass.

### Join the parser, adaptive plan and visible prompt cycle

Use the installed plan and committed source selection, not a worker-supplied
path or a second source inventory. In #241, bind the exact shipped executable
to the installed distribution and close the check/use replacement window;
then admit only the declared source and policy, stream bounded bytes, validate
the returned grammar, digest and byte ranges, and recheck authorization, lease,
source dependencies and expected generation at publication. The existing
Windows anonymous-pipe experiment cannot be enabled while another concurrently
launched child could inherit its handles. Evaluate a private, authenticated,
deadline-controlled local IPC channel with peer-process identity and explicit
ACL/remote denial; Microsoft's documented default named-pipe ACL is not a
private channel. On Linux prove actual child identity, descriptor containment,
parent-death cleanup, timeout and reaping. Unsupported environments report
unavailable; they do not silently run an unbounded parser. Windows project
files retain their inherited permissions.

The owner of a parser request must bound **both** pipe directions, source and
output bytes, process memory, total elapsed time, concurrent workers and
cleanup. Handle a child that never reads, writes an oversized or truncated
frame, sends a forged identity, exits early, or is killed while writing. Kill
and reap the owned process, discard partial output, and leave no accepted
checkpoint. A successful worker result is a proposal until the engine commits
source spans, graph relationships, job progress and receipt together under the
selected generation. Test real primary-binary execution, not just the pure
parser or test executable. The stored source's content digest must match the
bytes independently checked by the test after reopen.

For default planning, replace both `attunement_phases` arrays and the linear
terminal-Verify assumptions in `rrd-contract/src/attunement.rs`,
`installation.rs` and `engine/attunement.rs` with a versioned engine-owned
capability/effect catalogue and per-request selection. Available/configured
capabilities, project facts, user scope, permission, budget and effort select
work; an optional provider is not activated by discovery. Empty, short and
different selections are valid. Fixed wire-format versions, digest algorithms,
security bounds and OS invariants may be constants when documented; a fixed
number of reasoning steps or a preselected provider is not the runtime plan.
Characterize existing v2 plans/checkpoints and installed digests, then preserve
their accepted effects through the same canonical reader and authorized
successor mechanism. No schema of private thought and no second lifecycle.

The generated effect/handoff schema must be derived from the executable
contract owner and include: plan/version/digest and predecessor; project,
source binding and effective-configuration revisions; capability/version;
declared read dependencies including membership and negative lookups;
authorization and question dependencies; budgets, idempotency and verification
predicate; and effect receipt, outcome and resulting read/commit coordinates.
Before **each mediated durable or external effect**, validate those fields,
the relevant source and lease, and the allowed capability. A changed relevant
input produces a fenced successor; an unrelated edit preserves independently
valid work. A question blocks only effects that depend on it; otherwise the
engine continues authorized background work with visible progress. Persist
memory and effect receipts independently of sampled traces. A schema cannot
force an agent with unrestricted filesystem access to follow a plan: the
engine enforces mediated effects, while CI/review compares actual repository
changes and receipts against the PR brief.

The first project prompt uses persisted source-derived lexical/graph evidence
without requiring a model or external vector service. Publish the selected
avenues, effective graph/retrieval limits and reasons for unavailable avenues;
then add configured vector/embedding and LFG/SWAG through the same engine
contract. rrflowKV durable cache and prompt-local context are distinct
lifetimes under one authority. Connectome reads those public operations and
displays source, progress, configuration and audit records; it does not create
the job or plan. Current Parquet behavior remains bounded analytical export;
F-06 separately requires background cold storage and D-06 exchange/import.
Knowledge paths to local data,
Qdrant, SurrealDB and services are explicitly configured adapters, not implicit
storage backends or proof of competitive performance.

For onboarding, ask for the project boundary and fresh/existing choice only
when they are not already determined; ask for a nested-repository boundary or
an external capability only when the selected action genuinely needs that
decision. Show the exact proposed RRFlow-owned project map, files, permitted
source roots, exclusions, permissions, budget and configuration digest before
the authorized write. Use inherited Windows access without changing the
project's ACL or moving its files. Create and discover the map automatically
through the engine after apply, and make it a replaceable projection of the
engine record. Safe discovery and explicitly authorized local background work
do not wait for an optional model or connector answer. Missing permission,
excluded paths, progress, pause/recovery and errors must be understandable in
the public status operation and Connectome; the existing map owner's actual
contract and collision/removal behavior must be proved before writing.

The project journey after the first source-to-query proof is still required:

| Result | Canonical change and independent evidence |
|---|---|
| Configure KV, QL and DB | Expose effective engine-owned settings, origins, limits and digest via the executable operation catalogue; preview authorized changes and show them in Connectome. Restart/replay must retain the accepted setting and deny an unapproved expansion. |
| Use any supported AI client | Generate operations/models from `rrd-contract`'s executable catalogue through `rrd-client`, language SDKs and `scripts/ci/run_sdk_conformance.py`. Exercise an actual project session, prompt, receipt and reopen through the public interface. Per-project/boundary isolation is the default; any global integration is explicit and separately authorized. |
| Connect LFG and SWAG | Apply `docs/reference/inference/lfg-swag.md` at engine contracts, exposing availability, chosen policy, budgets and evidence. A mock or UI toggle is not a connected model; unavailable integrations stay visible as such. |
| Attach knowledge paths | Local folders and service connectors start as discovered, inactive candidates. Preview source boundary, permissions, data movement and removal behavior; index only after authorization. Prove source-specific identity, revoke/delete, restart and no implicit replacement of rrflowDB. Qdrant/SurrealDB ingestion or migration is separately qualified. |
| Observe and optimize | Record durable job/effect/memory receipts plus bounded metrics for inventory, source generation, retrieval and worker failure. Freeze corpus, judgments, hardware, baseline, thresholds and seeds before comparing recall, freshness, latency, memory or competitors. Sampled traces aid diagnosis and cannot stand in for audit. |
| Extend native rrflowDB | Keep transaction/WAL and semantic authority in RRFlow; map researched DuckDB/ClickHouse techniques to Rust query/storage owners with provenance and tests, not a renamed external runtime. PostgreSQL and SQLite compatibility, Parquet cold storage and streaming/import, ANN/TurboQuant accuracy and platform qualification keep their own gates. |

### Embedded database, columnar compaction and fast routing direction

The owner requests an embedded Rust library mode and a durable write path that
uses rrflowKV for hot state and writes compressed Parquet row groups during
background compaction. This is a target for the existing rrflowDB/rrflowKV
authority, not an implemented storage format. Embedded callers must use the
same `RrdEngine`, engram authorization, transaction, schema and recovery
semantics as the daemon; an in-memory profile is explicitly volatile. The
existing engine libraries are implementation inventory, not a qualified,
stable embedded SDK. PostgreSQL SQL/wire behavior remains H-08 work.

In persistent mode, acknowledge a durable transaction only after its WAL
record has reached the configured durability boundary; an in-memory buffer
alone cannot survive a crash. Apply committed versions to a bounded memtable,
then publish immutable compaction output with authenticated manifests,
recovery ordering and safe reader retention. The requested Parquet design must
carry keys, versions, tombstones, schema, ordering and source/read identity;
prove atomic publication, restart, interrupted compaction and exact visibility
before it replaces the existing native segment representation. A single
transaction or foreground point lookup must not wait for a whole analytical
rewrite, and WAL reclamation must wait until the selected durable state permits
it. Preserve existing native LSM behavior and tests until the replacement
proves these requirements. [PostgreSQL's WAL documentation](https://www.postgresql.org/docs/18/wal-intro.html)
explains the durable-log ordering; it does not prove RRFlow's implementation.

Keep point lookups, transactional mutations and graph adjacency on their
native indexed paths; expose an authorized, stamped Arrow/DataFusion analytical
stream for scans and eligible SWAG/LFG feature batches. Parquet is a columnar
file format, not the transaction manager or an automatically zero-copy memory
layout. Column/page projection and predicate pruning may skip work only when
the actual metadata and predicate support permit it; decode, allocation and
decompression must be accounted for. The [Arrow Parquet reader](https://docs.rs/parquet/59.2.0/parquet/arrow/arrow_reader/struct.ArrowReaderBuilder.html)
exposes projection, row-group and row-filter primitives, not automatic
application-specific predicate evaluation. Current rrflowQL Parquet export is
bounded to 1,024 untruncated rows; it is not compaction or a streaming scan.

SWAG and LFG are configured, authorized capabilities at the existing engine
edge. A local fast path must admit its exact model/configuration and resource
budgets, recheck its source identity, report selected or skipped avenues, and
fail closed on unavailable or stale inputs. No external model, process spawn or
unbounded worker is implicit. Compare an independently frozen corpus and
correctness rubric against configured LLM-agent baselines and against RRFlow's
own no-pushdown baseline, including no-answer and stale/denied cases. Measure
end-to-end decision latency and p50/p95/p99, throughput, CPU/GPU, memory,
allocation/decoded/scanned bytes, cache warmth and maintenance, write/space
amplification, recovery, model usage and answer quality. A sub-10 ms decision,
zero synchronization cost, zero-copy or competitive superiority is a
hypothesis until those matched tests pass on named hardware.

[pgrust](https://github.com/malisper/pgrust/tree/38634b408dea373f3944006dfbf74d19cbfc409f)
is a PostgreSQL-in-Rust reference with documented vectorized execution and
columnar/WAL ideas. At that revision its project describes an embedded variant
as future work, notes `unsafe` code, recommends against production use and
licenses its code under AGPL-3.0. Study specific behavior and failure semantics
at the owning H-08/F/C boundaries; do not import or copy its code without a
separate provenance/license and architecture decision. Its benchmarks and
compatibility claims are not RRFlow evidence.

For the agent handoff, the eventual generated contract's minimal semantic
shape is the following; the fields and validators are **proposed**, not a
currently enforced API. The catalogue owns the final wire schema and SDKs.

```json
{
  "contract_version": 1,
  "plan_digest": "<canonical-plan-digest>",
  "predecessor_digest": null,
  "scope": {"engram": "<id>", "project": "<id>"},
  "source": {"binding_revision": "<digest>", "read_stamp": "<engine-stamp>"},
  "configuration_digest": "<effective-config-digest>",
  "effects": [{
    "id": "<id>", "capability": "<catalogue-id-and-version>",
    "reads": ["<typed-complete-dependency>"], "questions": [],
    "authorization_revision": "<revision>", "budget": "<bounded-policy>",
    "idempotency_key": "<key>", "verify": "<observable-predicate>"
  }],
  "receipts": [{"effect_id": "<id>", "outcome": "<state>", "evidence_digest": "<digest>"}]
}
```

Require canonical serialization and digest verification, supported capability
and typed inputs, scoped read-set/source identity, policy and authorization,
dependency acyclicity, resource admission and idempotency at execution. A
receipt must bind the *actual* effect and verification, including denied,
interrupted and uncertain states; it must not be populated in advance as proof.
For an unmediated repository edit, the PR description and its linked journal
are the reviewable change brief; compare the actual diff and test artifacts to
that brief. Neither this example nor an instruction file can technically stop
an agent with independent filesystem access. Enforce whatever can be enforced
at the engine effect boundary and report the remaining external-review limit.

### Delivery, CI and measurement

The integration driver owns one current candidate tree and a ledger inside the
existing PR/journals: outcome; exact main/branch/tree/binary identities; last
installed command and failure; completed prerequisites; next failing boundary;
overlapping files and integration order; and candidate/merge-group run links.
The journal records actual commands, failures and unrun checks. It is evidence,
not a new runtime plan or status authority. Do not open a competing PR for an
overlapping file; if repository policy requires a separate bounded PR, land
its dependency first, merge it normally into the dependent draft, and rerun
the joined driver. A focused component pass leads directly to its next
dependent change or installed test in the same working session. When a
documented stop condition or an external blocker prevents continuation, record
the exact next action; do not leave a component green as the session's implied outcome.

Run focused red/green checks locally, then the affected owning suite and the
integrated driver. Run generated-surface checks when those surfaces change.
Request full candidate CI when the coherent candidate is ready; preserve all
required native, crash, reopen, SDK and merge-group gates. A draft skip is not
acceptance. Inspect a failed run's exact revision/log before rerunning; group
compatible repairs on the same candidate rather than restarting the whole
pipeline for every isolated edit. After a ready PR has its required checks and
review, use the existing merge queue/auto-merge policy and verify the merge and
branch deletion; never force or bypass the queue.

The repository reports auto-merge and branch deletion enabled. At this review
its repository-level runner list and repository Actions variables were empty;
the organization runner endpoint returned HTTP 403, so organization runners
and server availability are **unknown**. `ci.yml` selects self-hosted Linux
only with the configured repository variables and permitted event source.
Verify the owner's runner group, isolation, labels, local cache and trust
boundary before changing routing; do not move untrusted public-fork code onto a
privileged server. GitHub's Actions cache is cloud-hosted even for self-hosted
runners; a deliberately managed local cache is a separate operator setup.
Preserve the hosted Windows/macOS lanes and exact-candidate requirements.

C-07 remains its own storage exit. The retained 100,000-operation candidate
acknowledged all writes but verified only 44,556 before the original two-hour
setup limit; other runs failed differently, and no single cause is proven.
Use the existing sampled diagnostics and exact workload to attribute the
specific remaining limit, fix at its owner, then run the frozen native capacity
matrix, independent full readback, failure/recovery histories and actual
24-hour soak from `gate-c-07.md`. A short preflight, this source-generation
repair or a green component suite is not C-07 closure.

Primary references: [FoundationDB generation indirection](https://apple.github.io/foundationdb/developer-guide.html),
[Windows pipe security](https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipe-security-and-access-rights),
[GitHub ephemeral runners](https://docs.github.com/en/actions/reference/runners/self-hosted-runners),
and [GitHub cache storage](https://docs.github.com/en/actions/concepts/workflows-and-actions/dependency-caching).
These inform design choices; they do not establish RRFlow correctness or
authorize copying upstream code.

## Tests, measurements and stop conditions

Existing characterization commands, not claims about this new implementation:

```text
cargo test --locked -p rrd-engine --lib interrupted_refresh_does_not_expose_partial_facts_as_completed_snapshot -- --nocapture
cargo test --locked -p rrd-engine --lib inventory_delta::
cargo test --locked -p rrd-engine --lib inventory_interruption_preserves_incomplete_groups_and_resumes_after_reopen
cargo test --locked -p rrd-engine --lib inventory_edit_during_publication_rejects_stale_completion_and_allows_replan
cargo test --locked -p rrd-engine --lib windows_supervisor::tests
cargo test --locked -p rrd-contract --test attunement_contract
cargo test --locked -p rrd-engine --lib inventory_checkpoint_and_source_records_commit_reopen_and_replan_together
cargo test --locked -p rrd-engine --lib git_selected_plan_ignores_irrelevant_edits_replans_tracked_edits_and_reopens
cargo test --locked -p rrflow-cli --test installed_lifecycle -- --nocapture
```

The interrupted-refresh test exists only on #252; `windows_supervisor::tests`
exists only on #241 and selects tests only on Windows. On #241,
`cargo test --locked -p rrd-attunement` and its actual `parser_worker` process
test characterize the draft; they do not prove native supervision or engine
publication. Confirm every filter selects a nonzero test count. Extend the named
installed-flow driver and run the smallest red case before each package's code,
then its owning suites, generated contract/SDK checks, candidate CI and
protected merge-group CI. Preserve existing crash and reopen cases rather than
replacing them with mocks.

The independent expected ledger is derived from the fixture files and authorized plan, not from the engine's claimed totals. Compare source bytes/digests, span bounds, current graph/lexical results, actual file effects, job receipts, and restored state. Required cases include excluded/secret paths, symlink and mount escape, permissions revoked mid-run, file replacement, malformed syntax, unsupported language, changed and unrelated files, deletes, size/node/time limits, slow or killed worker, stale lease, repeated idempotency key with same and different content, cancellation, storage-full, process death before and after publication, and two engrams. Distinguish acknowledged success, denied, retryable, incomplete and uncertain outcomes.

Measure initial discovery/read/parse/index time, bytes read/written, records, peak RSS and storage, queue time, restart work, per-query latency, candidate counts, truncation and source freshness; record cache state, workload and hardware. Compare warm/cold and edit-one-file against full rebuild, and monitor foreground/idle resource budgets. A watcher is an optimization, not the source of truth: overflow, restart and missed events require rescan/reconciliation. Do not promise zero content reads based solely on metadata. For retrieval, freeze corpus, exclusions, judgments, exact filtered oracle, baselines, seeds, metrics, thresholds and confidence procedure before comparing. Report recall/precision or nDCG, freshness and citation validity with latency/resource distributions; include vector-disabled and stale/denied cases. Freeze 100,000-record and native capacity/soak profiles from the owning gates rather than inventing new passing thresholds.

Stop implementation or keep the affected capability unavailable if source containment or authorization cannot be rechecked, native memory/deadline/child cleanup cannot be enforced, output/transaction limits are exceeded, durable progress cannot be replayed without duplication, an old snapshot can appear current after an edit, or the independent oracle disagrees. Record failures and missing native platforms; do not increase budgets, fake storage errors, weaken assertions, or call this C-07 qualification. Exact-candidate C-07 native capacity, recovery and soak remain separately required.

## Research applied and open decisions

- [Tree-sitter's parser API](https://docs.rs/tree-sitter/latest/tree_sitter/struct.Parser.html) supports cancellation-related options and requires an old tree to be edited to match changed text before incremental reuse. The draft pins 0.24.7; confirm behavior against that exact version, not only latest documentation. A callback is not OS memory isolation.
- Microsoft's [process creation flags](https://learn.microsoft.com/en-us/windows/win32/procthread/process-creation-flags) define suspended startup; the chosen job/handle association and parent-death behavior still require real native tests. Linux [`getrlimit`](https://man7.org/linux/man-pages/man2/getrlimit.2.html) documents inherited limits and `RLIMIT_AS` virtual address-space behavior; it does not establish aggregate process-tree isolation.
- [Qdrant's hybrid-search tuning](https://qdrant.tech/documentation/search-tuning/how-to-tune-hybrid-search/) describes rank-fusion tradeoffs and calls for evaluation on one's own labels. Its [quantization documentation](https://qdrant.tech/documentation/quantization/) discusses accuracy, oversampling and rescoring tradeoffs. These motivate configurable experiments, not an RRFlow performance claim or automatic choice of Qdrant as authority.
- [Apache Parquet](https://parquet.apache.org/) defines a column-oriented analytical file format. Its presence does not establish transactional WAL, native indexing, full streaming or import. [OpenTelemetry sampling](https://opentelemetry.io/docs/concepts/sampling/) can omit spans; complete durable effect receipts require their own retained record.
- The owner withdrew the prior DuckDB adapter and worker direction. [Native analytical database research](../../reference/data/duckdb.md) maps DuckDB vector execution and ClickHouse sparse indexing to RRFlow query/storage boundaries and records upstream licenses. Both have substantial C++ implementations; adapting behavior or porting eligible code to Rust is a real engineering and testing task, not a rename or automatic transfer of performance.

Before the relevant package is ready, decide and document: (a) canonical versioned capability/plan compatibility for already installed engrams; (b) exact bounded snapshot paging/proof and safe source invalidation with concurrent edits; (c) native child creation/parent-death containment per supported platform; (d) source/chunk schema and lexical/vector publication dependencies; (e) the actual public configuration mutation and question UX where the existing owner does not yet authorize one; (f) the held-out dataset and pass/fail thresholds. These are engineering decisions with named owners and tests, not reasons to block unrelated implementation or manufacture defaults.
