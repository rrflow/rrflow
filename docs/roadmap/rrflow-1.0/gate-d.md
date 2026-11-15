# Gate D — install, configure, and attune one real engram

**Status:** active canonical RRFlow 1.0 roadmap chapter
**Coordinate:** `rrflow://rrflow-instance/data/roadmap/rrflow-1.0/gate-d`
**Owner:** [RRFlow 1.0 release roadmap](../rrflow-1.0.md)

This chapter is part of the canonical RRFlow 1.0 roadmap. The parent
record owns release order and completion status; this file cannot change
either independently.

The [RRFlow kernel contract](../kernel-contract.md) carries Gate D's project-workflow
work as packages K0, K5 and K6. The historical
[adaptive developer-build design](adaptive-developer-build-design.md)
maps the fixed default sequence to a proposed capability-selected replacement
and a real-project acceptance oracle. It is supporting design, not accepted
implementation or a change to the release gates below.

| Done | ID | Required change | Owning boundary | Acceptance evidence |
|---|---|---|---|---|
| [x] | D-01 | Implement the [installed lifecycle](../../reference/deployment/installed-lifecycle.md) foundation and first walking product spine. The default release exposes one primary `rrflow`/`rrflow.exe` command tree. `rrflow install plan` resolves only bundle-resident, versioned project-bootstrap templates, provider-neutral specialization manifests, and attunement profiles; is byte-deterministic and side-effect free; and covers every file, record, process, adapter candidate, principal/role/grant, credential action, permission, budget, digest, and removal rule. `install apply` accepts the exact digest, explicit fresh/existing mode, and a minimal `.rrflow/config.toml` locator with no canonical mutable state or plaintext secret. Split rrflowKV create-new, open-existing, and read-only inspection so normal open never creates or repairs an engram. `initialize_instance` is the only cold-start path and atomically commits the installed binding, initial seat/security authority, checkpoint, audit, and commit evidence before readiness. Compose `serve`, authenticated `ready`, `version`, and baseline read-only `verify` through the same executable and `RrdEngine`; reusable server/MCP code is linked as libraries rather than checkout-built readiness prerequisites. Remove the old supervisor, standalone bootstrap, server initializer, create-on-open/startup binding, private marker/JSON state, and their successful fixtures in the same direct-convergence package after replacement evidence passes. | `rrflow-cli`, `rrd-engine`, `rrd-store`, `rrd-server`, `rrflow-mcp` | Native real-process tests for Linux and the available platform development runners prove `version`; deterministic plan; no-write preview; exact apply; one high-entropy credential through the previewed sink; authenticated readiness; one commit/query; close/reopen; and baseline read-only verification through the primary executable, with outbound network denied and sibling directories absent. Fresh and existing projects preserve user content and any managed instruction edit is ownership/digest bound. Interruption tests cover staging, credential prepare/delivery, engine commit, locator publication, acknowledgement, and cleanup with no duplicate credential/policy/audit outcome. Drift, path/link/mount replacement, ACL/mode, provider revision, partial state, stale lease, unsupported format, missing installed binding, and already-installed cold start fail closed. `rrflow dev doctor`, a PID/port/marker, source topology, and separate binaries cannot satisfy or substitute for this proof. |
| [ ] | D-02 | Persist attunement jobs and checkpoints through `RrdEngine`; implement status, resume, cancel, leases, idempotency, and phase input/output digests. | `rrd-engine` | Kill/restart tests at each transition resume committed work once and never infer completion from emitted events. |
| [ ] | D-03 | Implement only deterministic project-tree inventory: engine-authorized metadata/content reads, pure proposal building, Git-correct ignore rules, secret/generated/vendor/cache exclusions, stable root-relative identities, Merkle tree digest, containment edges, incremental change set, explicit errors, and bounded work accounting. | `rrd-attunement` through `rrd-engine` | This repository produces the same committed `SourceTreeSnapshot` digest under different traversal schedules; excludes `.git`, `target`, `node_modules`, rrflowDB files, generated bulk data, and secret payloads; handles tracked ignored files, non-Git roots, hidden source, symlink/mount escape, file races, cancellation/restart, and close/reopen; an unchanged rerun reads zero content bytes and commits a no-work checkpoint. |
| [ ] | D-04 | Add incremental Tree-sitter parsing pinned to one committed source-tree snapshot/change-set digest and parser/language revisions; preserve incomplete syntax and errors as evidence. | `rrd-attunement` through `rrd-engine` | Parse is denied before the D-03 snapshot/checkpoint commit; edit-one-file reparses only the changed source, preserves unaffected identities, records `ERROR`/`MISSING` nodes, and resumes after process restart. |
| [ ] | D-05 | Implement configurable source normalization, entity linking, lexical and vector indexing, relationship/graph derivation, grounding and verification as engine capabilities over the accepted persistent and analytical paths. Select only work required by the project, prompt, configuration, available capabilities, permissions and budget; only actual data/effect dependencies constrain order. Add no temporary snapshot index or parallel attunement store. | `rrd-engine` plus owning subsystem | Different projects and policies select different valid capability sets, including no optional model; unavailable capabilities remain explicit. Each selected durable effect has a bounded input, checkpoint, failure/retry outcome and independently checked output. Source-derived results are readable through the same authorized paths after reopen. |
| [ ] | D-06 | Classify project generators, package scripts, build/test/evaluation harnesses, CI/deployment tools, SQL, PostgreSQL, Turso, Dragonfly, object stores, models, meshes, and other application/operator systems as optional external capabilities or sources. Discovery creates an inactive typed candidate; only an exact previewed immutable binding plus explicit configuration, policy, and authorization may activate it. A command binding captures either one authenticated direct-process argv or the complete package-script/toolchain/lockfile/wrapper/lifecycle closure; literal package-manager argv alone is insufficient. Never install, start, execute, or select an external system as RRFlow persistence or lifecycle authority implicitly. | `rrd-contract`, `rrd-attunement`, `rrd-engine` | Fresh/existing fixtures prove RRFlow reaches default readiness without an external capability; discovery executes nothing; preview performs no writes or effects; exact apply/retirement is durable and idempotent; a generator or harness binding is source- and digest-bound and declares its permissions, sandbox, budgets, cancellation, uncertainty, verification, and output policy without invoking it; mutable/incomplete closures and unavailable enforcement fail closed; credentials are never copied; and clean uninstall removes only RRFlow-owned integration state. Project-command execution and output re-inventory are accepted later by I-03/I-07. |
| [ ] | D-07 | Bind installation to the DevForge CoW placement contract: immutable tools/models may live in the shared lower layer; workspace changes and all rrflowKV WAL, manifest, segment, catalogue, and graph state live in the writable upper layer. | `rrflow-cli`, DevForge adapter | Two clones share the same lower digest while independent writes, crash recovery, and deletion in one upper layer cannot affect the other. |
| [ ] | D-08 | Recognize content-addressed dependency mounts as shared immutable inputs rather than copying or attuning dependency caches into each engram. | `rrd-attunement`, DevForge adapter | Rust, Go, Node, and model-cache fixture proves stable mount digests, zero duplicate ingestion, and correct invalidation when a mounted digest changes. |
| [ ] | D-09 | Measure logical size, allocated blocks, compression, WAL growth, and snapshot size separately; never infer zero-byte or sparse-allocation savings from logical file size. | `rrd-lsm`, release harness | Fresh clone and sustained-write reports account for lower, upper, cache, WAL, segment, and snapshot bytes with reproducible filesystem commands. |
| [ ] | D-10 | Add a hibernation preparation/restore contract that quiesces writes, captures a verified rrflowKV snapshot boundary, exports to the configured cold tier, and resumes without changing engram identity. | `rrd-engine`, DevForge adapter | Interrupted export, corrupt object, restore, rollback, and hot-to-cold-to-hot tests prove no acknowledged-write loss and no split authority. |
| [ ] | D-11 | Complete the installed operational lifecycle: offline read-only quick/full verification; digest-bound offline repair plan/apply; authenticated backup verification; restore into an absent staging target plus explicit publication; explicitly non-authoritative salvage; and ownership-aware uninstall that retains rrflowDB/backups and reports `RETAINED`. Repair may truncate only a proven incomplete final WAL frame, delete only unreachable physical objects, rebuild derived graph/BM25/vector/TurboQuant/analytical projections from canonical semantic state and exact source cursors, or reconcile durable install/attunement receipts. It never guesses manifest, sequence, transaction, authorization, or acknowledged semantic truth, and it never edits the live root in place. | `rrd-engine`, `rrd-store`, `rrd-lsm`, `rrflow-cli` | On the D-01-installed binary, corruption/fault fixtures prove verify changes no bytes; every plan is deterministic and stale-plan rejection occurs before mutation; apply requires an exclusive offline lease, authenticated snapshot, exact preconditions, an absent reflink/clone or fully accounted same-filesystem candidate, audit receipt, full candidate verification, one durable atomic locator replacement, and retention/quarantine of the prior root. Complete-frame/canonical corruption routes to absent-target restore or labeled salvage rather than reset; projection rebuild matches exact oracles; backup restore never overlays live state; and uninstall removes only unchanged RRFlow-owned integration while preserving project state and retained data. |

## Real-project implementation and acceptance

D-02 is the next runtime outcome, subject to D-02a admission. Installation currently writes a pending
`AttunementJob`; `installation/inventory.rs` explicitly supplies installation
preconditions, not a committed D-03 source snapshot. Job types and Connectome's
phase descriptions do not execute ingestion. Complete the existing job state
machine at `RrdEngine`, then implement D-03/D-04 and the D-05 capabilities
their selected inputs require at their canonical owners. No fixed number of
runtime reasoning or attunement steps is implied. No client-side ingestion database,
fixture-loaded demo or temporary snapshot index substitutes for these packages.

The configured project must resolve global defaults plus project overrides to
one versioned, engine-owned effective configuration, with field origins and a
digest. Hard authorization/resource policy cannot be loosened by preferences.
A changed global default cannot silently rewrite an installed project's sealed
configuration, activate a provider or grant context sharing. Changes use the
existing preview/authorization/effect boundary; an inexpressible public action
is an explicit blocker, not permission to invent a route or change the API.

The joined acceptance follows one real project through authorized install,
job execution, committed inventory, source chunks, lexical and graph relations,
configured real embeddings and vector retrieval. Retain source/model/configuration
identities, exact spans and a known-answer query. Edit and delete source, reindex,
prove stale evidence is excluded, restart and repeat readback. Repeat with a
second project concurrently and prove isolated state and defaults/overrides.
Connectome progressively exposes supported operations, per-project data and
phase failures through the existing public contract, with unfinished phases
clearly unavailable. Its optional presence cannot become engine readiness.

Each package proves its particular before/after transition. D-02 alone does not
claim parsing or retrieval; a configured embedding provider alone does not prove
vectors were generated. The complete journey requires all relevant package
receipts and does not erase remaining release qualification.

## Nested repository setup

The owner's 2026-10-31 direction makes a discovered nested repository a user
choice. Offer exclude, separate project/instance (recommended), and one project
instance containing both selected repositories. Explain that independent
instances retain separate stores and authentication and that any cross-project
read requires its own explicit link. The one-instance choice is one bounded
project with independently identified source repositories and one engram, not
a silent merge of pre-existing engrams. The
[topology owner](../../architecture/instance-topology.md#user-selected-repository-boundaries)
defines the semantics.

D-01/D-03 must expose source-bound repository candidates in a no-effect preview,
carry the selected membership and every admitted Git metadata root into the
exact plan, and revalidate them before apply and during inventory. An assistant
may explain the choices and ask the user, but cannot accept a choice or expand
its own filesystem authority. Gitfile, parent index, configuration, ignore and
back-reference inputs need explicit bounded admission and identity/race checks;
external metadata is not followed merely because an untrusted file names it.
The selected repositories remain source inputs, not engine dependencies or an
excuse to execute Git or fetch during installation.

Acceptance must use the real RRFlow repository and its pinned Connectome
submodule as well as adverse inputs. Verify each of exclude, separate and
shared choices; exact preview and stale-plan denial; no changes before apply;
independent authenticated instances and read isolation for separate projects;
one transaction/read-stamp authority and source-specific provenance for the
shared project; edit/delete, restart and readback; wrong Gitfile target,
changed parent index, symlink/mount escape, missing metadata and cancellation.
Run the installed primary binary and the public client path; report unsupported
choices explicitly until they work. The [real-checkout gap](../../poam/rrflow-1.0-alpha/poam-006.md#nested-repository-setup-gap)
remains open. This requirement does not by itself qualify later graph/vector
indexing, project links, UI or cloud behavior.

## Project-workflow build packages

These refine D-02 through D-05, not a second roadmap. At audit baseline
`337d136210ff59b2359f898e717166c087dd5bfc`, the file-plan generator marked
`engine/attunement.rs`, `tests/attunement_resume.rs` and the
`crates/compute/rrd-attunement/` modules as **planned**, not implemented.
The embedded package below now implements the engine control module; the
remaining phase/compute modules and complete qualification remain open.
The owner's subsequent instruction to implement authorizes the bounded D-02
embedded-engine package below. It does not reinterpret installation/readiness
as permission for source effects or change the existing transport API.

### Embedded D-02 control implementation

`RrdEngine::read_attunement` and `control_attunement` now use the installed job
and existing EngramRead/EngramAdmin permissions. Additive Rust commands start,
step, pause, recover an expired lease, resume and request/acknowledge cancellation;
no existing HTTP operation, wire model, installation action or readiness meaning
changes. There is no unattended worker or source/provider activation.

Each mutation binds revision, principal-scoped idempotency and plan/lease identity
as applicable. Job and exact response receipt commit in one control transaction.
The Connect worker verifies installed identity/configuration and records a real
runtime read stamp. Inventory currently produces `phase-unavailable` with no
checkpoint. Configurable finite leases, bounded encoding, a lifetime transition
limit and cancellation reserve bound this package; D-02c's complete scheduling
and project settings remain open. This is partial D-02 implementation, not
source indexing or C-07/D-02 completion. The [package journal](../../evidence/change-journals/gate-d/d-02-durable-attunement.md)
owns exact tests and remaining qualification.

| Package / dependency | Input, owning code and durable output | Failure oracle and acceptance |
|---|---|---|
| D-02a admission decision; before executor implementation | Existing `rrd-contract/src/attunement.rs` job/plan/resume/cancel validators, `src/lib.rs::endpoint_catalogue`, CLI installed command dispatch and installation preview. Map each public action to authorization and actual engine dispatch. | The initial audit found only contract types. The embedded package above adds authenticated engine dispatch; public HTTP/SDK dispatch remains absent. Installation previews `CreateAttunementJob`, not unlimited execution. Publish the exact operation/permission/activation decision before source effects. Preserve existing APIs. On 2026-10-26 the owner allowed adding the missing planned job list, read, status, resume, cancel, receipt-read and job-transition journal operations to the operation catalogue under existing grants ([OBJ-14](../../objectives/rrflow-1.0-alpha.md#required-outcomes), [POAM-040](../../poam/rrflow-1.0-alpha/poam-040.md)); global configuration operations remain undecided. Do not reinterpret task-open, readiness, install or arbitrary query as an ingestion command. |
| D-02b durable worker; after D-02a | Extend `RrdEngine` at `engine/attunement.rs`, using the existing installed job, `AttunementJob::validate_transition`, lease generation and canonical transaction authority. Input binds job revision, plan, principal, configuration and source identity; output is the job transition and checkpoint with effect evidence. | Two workers compete for one revision; only the fenced winner publishes. Crash before effect, after effect and before ACK; retry resolves the same receipt. Expired generation cannot commit. Cancel releases owned resources and never publishes a false success. An unimplemented phase stops explicitly, never gets a fabricated checkpoint. Planned `tests/attunement_resume.rs` plus real installed-process proof. |
| D-02c bounded scheduling and settings; required before unattended work | Existing engram configuration, reasoning/query budgets and phase estimates. Engine owns effective limits and field origins; pure preference resolution cannot grant effects. State the exact configuration apply operation and how changed settings affect pending/running jobs. | Estimated item/byte/duration fields are not enforcement. Admit memory/input/output/elapsed/retry/worker limits before work; one remaining budget is charged across retries. Blocked/paused/rejected states identify the exhausted limit. Global changes do not silently rewrite projects; concurrent changes use revision checks. Public settings controls remain blocked until their admission mapping is resolved. On 2026-10-31 the owner asked that unattended relationship and index maintenance run only inside operator-set idle windows and budgets, adjusted from Connectome ([first complete cycle](executable-dependency-spine.md#first-complete-cycle)). |
| D-03 source snapshot; after durable worker and bounded filesystem admission | Planned `rrd-attunement/src/inventory.rs` builds pure proposals from engine-authorized reads; existing `installation/inventory.rs` remains installation precondition logic. Commit stable root-relative source identities, content digests, containment, exclusions and a change set through engine transactions. | Ordered/shuffled traversal gives identical snapshot; unchanged rerun reads zero content bytes. Git tracked/ignored rules, non-Git roots, secrets, symlink/mount escape and concurrent source edits have explicit outcomes. Interrupt/restart never exposes a partial snapshot as complete. Planned inventory test target; real repository fixture after small authored adversarial fixtures. |
| D-04 parse and source spans; after D-03 | Planned `rrd-attunement/src/parse.rs`; pinned parser/language revisions consume one committed change set. Output is source-linked syntax/symbol/chunk proposals with byte ranges and parse errors. | One-file edit reparses only affected inputs; delete invalidates their spans. Unsupported languages remain explicit and incomplete syntax retains errors. Parser/model guesses cannot overwrite source facts. Planned incremental-parse target and installed readback prove digest/range identity after restart. |
| D-05a normalize and entity links; after D-04 and consumed E-01/E-02 paths | Planned `normalize.rs`/`entity_link.rs` plus canonical schema/data/relation commits. Deterministic source facts and inferred relationships carry distinct method/source/configuration identities. | Duplicate source or ambiguous symbol names cannot merge unrelated identities. Reprocessing is idempotent; source change/delete retires dependents at a new cursor, old-stamp reads remain valid. Semantic effects and checkpoint are atomically visible or explicitly staged then published. |
| D-05b lexical access; after D-05a and E-03 | Persistent dictionary/statistics/postings/tombstones at source cursor; use existing query/store boundaries. No per-request full rebuild as the completed scalable path. | Incremental insert/edit/delete/reopen matches full-rebuild oracle and returns exact current spans. Record visited/read work separately from overall engram size. Stale projections are reported or handled by the qualified canonical fallback, never silently presented as current. |
| D-05c embeddings and vector access; after source provenance and canonical E-04 exact path | Existing embedding registry/engine inference and vector collection APIs consume authorized chunks with pinned model/tokenizer/dimension/metric. Commit canonical vectors and derived-index deltas; approximate generations remain separate acceleration. | Real admitted embedding provider, not fabricated vectors, proves source-to-vector generation. Model mismatch, wrong dimensions, timeout, late output, revoke, duplicate and delete tests retain one accepted publication. Exact filtered search is the oracle; HNSW/quantization requires its own E-04 quality/build/recovery evidence. |
| D-05d graph, grounding and verification; after prior outputs | Planned `ground.rs`/`verify.rs`, temporal adjacency and stamped context/query paths consume committed source/model identities. Verify relationship/citation closure, freshness and evidence coverage; complete the job only when all required phase outputs exist. | A known-answer query returns actual spans and graph/vector evidence. Edit/delete source, rebuild, restart, and repeat with two projects concurrently; no stale or foreign source appears. Missing evidence causes explicit incompleteness/abstention. This joined proof is the first local-project checkpoint, not full alpha. |

The joined proof uses an authored, revision-pinned small project with a named
symbol, cross-file call, natural-language documentation, ambiguous names, an
ignored generated file and a secret sentinel. Its expected spans and relations
are independently enumerated, not generated by the implementation under test.
Run these cases in order: initial source ingestion; exact symbol query; lexical
and real-vector intent queries; multi-file relation query; change one source;
delete one source; rerun unchanged; cancel/restart midway; repeat after engine
restart; execute a second project with conflicting names. No-answer and denied
queries must not fabricate citations or disclose the sentinel. Every asserted
exact result must match; approximate quality uses the predeclared E-04/J-04
profile rather than an invented threshold. Then use a pinned real repository
revision for growth. The authored project is a correctness fixture, not evidence
of scale or competitive quality.

Package commands use the existing contract regression target
`cargo test -p rrd-contract --test attunement_contract --locked` for preserved
wire semantics. That target validates types; it cannot prove the worker.
New phase targets above must be created in their implementation package before
claiming execution. The owning installed-process suite and native platforms are
selected from the existing CI contract for the changed boundary. No parser
crate, model file or external service may become an undeclared runtime fetch.

All phase proposals flow through one transaction authority. A checkpoint may
reference an already committed bounded group only with durable idempotency and
publication state that distinguishes partial from complete output. Never write
rrflowDB, graph and vector state through unrelated commits and then assert the
job succeeded. The phase plan's digest chain must remain valid across resume;
changed source/configuration requires explicit replan or rejection, not editing
the accepted plan in place.

E/F are consumed at specific boundaries: semantic records/adjacency require
E-01/E-02; persisted lexical access E-03; exact and approximate vector stages
E-04; access selection E-05. Analytical streaming consumes F-01/F-02/F-04;
mixed analytical operators consume F-03 and cache qualification F-05. Native
point/graph/exact queries do not acquire a DataFusion dependency merely because
a later analytical path exists. Optional knowledge-source adapters follow D-06
and reuse this same provenance/publication pipeline.

## Turnkey provisioning acceptance

D-01/D-02/D-05 compose the [minimal-input Connectome flow](../../reference/client/connectome.md#turnkey-project-entry-and-progressive-detail).
The installed plan must bind generated project identity, canonical engram,
versioned bootstrap schemas, graph/source definitions, admitted model and vector
collection parameters. Users select a project and review effects; they do not
manually construct three DB/KV/QL databases or pre-seed records to make the demo
work. D-02a still owns explicit ingestion activation and missing public controls.
D-02c/G own [effort inheritance](../../reference/inference/lfg-swag.md#inheriting-effort-from-the-calling-ai);
engine-enforced limits cannot be replaced with UI presets or client timers.

Prove a no-account local installation, automatic source-derived data, first
source-citing query and changed/deleted-source readback through the same engram.
State which modalities and languages the admitted profile actually supports;
unsupported input cannot silently become indexed content. Default local model
assets follow J-05's bundle requirement. External models, knowledge paths and
rroflow.ai remain separate opt-in effects. Failure to qualify those optional
profiles must not silently change the meaning of local indexing readiness.

## [Accepted evidence](gate-d-evidence.md)

## D-01 remaining closure packages

D-01 is accepted as an OBJ-07 prerequisite; OBJ-07 remains partial.
The [final acceptance matrix](gate-d-evidence.md#d-01-accepted-installed-foundation-2026-10-21)
binds every required row to candidate `9b6c3571`, native receipts and successful
CI. The active dependency-spine item is now D-02 and the real-project workflow;
C-07 final qualification remains open. The historical D-01 implementation order below
was established on 2026-10-21 against `1815af32a53e161bc5f3d7f0f7b4365462cd9f7a`.
It refines the D-01 row above;
it neither changes that row's acceptance nor creates another status owner.
The [bounded evidence baseline](gate-d-evidence.md#d-01-bounded-baseline-2026-10-21)
records PRs 43, 44 and 45. Preserve their passing offline Linux, native staged
process, distinct-identity and private secret-file fixtures. Later gates extend
this accepted foundation rather than rebuilding it.

The sequence is **directory security -> filesystem containment -> installation
exclusivity -> complete bootstrap -> native qualification -> closure**. D-01.1 landed through PR 48; D-01.2 landed through PR 49; D-01.3 landed through PR 50; D-01.4 landed through PR 51; D-01.5 landed through PR 52; D-01.6 reconciles its accepted evidence here. Each package requires a fresh source
reconciliation, pre-edit PR brief, linked journal and successful candidate CI.
Package numbers are subdivisions of D-01, not additional release gates.

| Package / state | Current behavior and canonical implementation scope | First failure oracle and exit evidence |
|---|---|---|
| **D-01.1 Private engram directories / accepted bounded implementation** | `engine/engram_layout.rs` and `engine/token_key/private_file` now create and admit private directories through native owner/mode/DACL checks. Project-owned parents remain unchanged. [PR 48](https://github.com/rrflow/rrflow-development/pull/48) candidate `09627235` passed run `39621697700` attempt 2 and merged as `2d7167c9`; [the journal](../../evidence/change-journals/gate-d/d-01-private-estate-directories.md) records the bounded implementation. Final privileged/native qualification is accepted in .5 below. | Create an engram under permissive/inherited ACLs and retry with substituted ownership/mode/DACL. Newly created RRFlow directories must have the declared private policy from creation; incompatible existing directories fail before secret delivery or store mutation, without silently repairing permissions. Prove Windows and Unix behavior, foreign-owner denial where the runner can exercise it, byte stability on denial, and the retained eight-stage recovery corpus. This alone does not prove ancestor containment. |
| **D-01.2 Anchored filesystem operations / accepted bounded implementation** | [PR 49](https://github.com/rrflow/rrflow-development/pull/49) replaces pathname check/use with retained project/private-directory capabilities through installation, rrflowKV/LSM, object/vector and backup/snapshot handoffs. Native operations and verified-file consumers retain existing engine/storage authority. Candidate `92df59ef` passed full run `39635463758` and merged as `0a5078b5`; [the journal](../../evidence/change-journals/gate-d/d-01-anchored-filesystem.md) and [checkout correction](../../evidence/change-journals/gate-d/d-01-canonical-manifest-checkout.md) bind the implementation. Final bounded adversarial qualification is accepted in .5 below. | Deterministically replace a project/managed ancestor between validation and read/create/publish/cleanup, including Windows junction/reparse and Unix symlink/mount cases. No outside sentinel is read, written, disclosed or deleted. Hold and validate native directory/object identities for each effect; detect unsupported enforcement before effects. Prove locator/pending-file publication and intent acknowledgement target only the authorized objects. Carry the protection through storage open, inspection and retained handles; validating a parent and then handing an unchecked `PathBuf` to storage is not an exit. |
| **D-01.3 Exclusive install and recovery / accepted bounded implementation** | [PR 50](https://github.com/rrflow/rrflow-development/pull/50) adds a retained native project lease from admission through final acknowledgement at `apply_installation_with_observer`. Plan format 3 previews its exact path, precondition, acquisition and retained lifetime. Read-only admission is revalidated under the lease; existing filesystem capabilities and storage authority remain in place. The [repair journal](../../evidence/change-journals/gate-d/d-01-exclusive-installation.md) records the original failing process oracle and replacement corpus. Candidate `3817cb5a` passed all 17 jobs in full run `39647536747` and merged as `6ba9dc5a`. Final native qualification is accepted in .5 below. | Two real processes apply the same plan, competing plans, and recovery versus apply at controlled barriers before intent, store, locator and cleanup. Exactly one admitted writer may produce effects; the loser returns a bounded conflict. Kill the owner and retry: one credential, policy, checkpoint, audit outcome and locator remain. A stale owner/lease cannot clear a newer admission. Neither elapsed time nor a PID alone permits takeover. Reuse existing lock semantics where proven; add no sidecar lifecycle database or client lock authority. |
| **D-01.4 Complete bundled bootstrap / accepted bounded implementation** | [PR 51](https://github.com/rrflow/rrflow-development/pull/51) binds linked generic/Clyffy templates and specialization, all thirteen paths and ten ordered actions, bounded initial records and actual instruction publication/recovery in plan version 4. Unknown or changed bundle/installed bytes deny. Candidate `9cfeeca4` passed all 17 jobs in [run 39654924753](https://github.com/rrflow/rrflow-development/actions/runs/39654924753) and merged as `7aa10e23`. The [implementation journal](../../evidence/change-journals/gate-d/d-01-bundled-bootstrap.md) and [SDK identity correction](../../evidence/change-journals/gate-d/d-01-bootstrap-sdk-identity.md) preserve local and failing-candidate evidence; final native qualification is accepted in .5 below. | An independently enumerated expected-effect fixture detects any unpreviewed file, transient path, canonical record or invocation. Repeated preview of identical explicit inputs is byte-identical and performs no mutation, credential generation/read, process start or provider call. Exact apply materializes only the declared closure; template/specialization/adapter revision, source or permission drift fails before effects. Fresh and existing projects preserve owned content; interruption at each newly introduced effect resumes without duplicates. Contract, schema and client projections are generated from their existing owner and qualified together. |
| **D-01.5 Native adversarial and offline qualification / accepted** | [PR 52](https://github.com/rrflow/rrflow-development/pull/52) candidate `9b6c3571` passed all 17 required jobs in [run 39674145683](https://github.com/rrflow/rrflow-development/actions/runs/39674145683), with only macOS and its dependent gate retried after hosted-runner capacity cancellation. All three native isolated actual-binary receipts pass; owner/mount/ACL fixtures and eleven-boundary recovery pass. The [accepted evidence](gate-d-evidence.md#d-01-accepted-installed-foundation-2026-10-21) records exact hashes and limits. | Run the matrix below on native Linux, Windows and macOS using actual primary executables. Add asserted outbound denial and checkout/sibling absence for each claimed offline platform, plus deterministic native path/ACL/lease failures. A missing required privilege, unavailable enforcement or skipped adversarial case is an open qualification gap, never a pass. Keep finite deadlines, output ceilings and retained failure artifacts. Do not replace native evidence with cross-compilation or mocks. |
| **D-01.6 Acceptance reconciliation / reconciled** | The [accepted matrix](gate-d-evidence.md#d-01-accepted-installed-foundation-2026-10-21), retained receipts and [reconciliation journal](../../evidence/change-journals/gate-d/d-01-acceptance-reconciliation.md) bind all eight acceptance rows. D-01 is checked; the current dependency spine owns subsequent work. POAM-006 stays open and OBJ-07 stays partial. | At one candidate revision, account for every D-01 requirement, OS/filesystem profile, binary digest, command, case count, denial and retained receipt. No required row may be missing or skipped. Only then mark D-01 complete and advance according to the current dependency spine; parent OBJ-07 and alpha remain open. |

### Bootstrap scope and durable-effect boundary

The D-01.4 [implementation receipt](../../evidence/change-journals/gate-d/d-01-bundled-bootstrap.md)
binds the version-4 preview, linked bootstrap assets, independent effect corpus
and local results. PR 51 now supplies successful exact-candidate CI and merge
evidence above. D-01.5/.6 now close the parent D-01 prerequisite. The
[native qualification oracle](../../evidence/change-journals/gate-d/d-01-native-qualification-oracle.md)
recorded incomplete corpus admission; the accepted runner now requires exactly
five declared and passed cases, with no omissions or skips.

D-01.4 must bind the template, specialization, attunement profile and effective
configuration revisions/digests; the initial seat, principals, roles/grants and
credential generation/sink policy; and every planned path, record, command or
explicitly empty process list. Include transient intent/staging/lease paths,
ownership/preconditions, permissions, resource bounds and removal rules. Seal
secret descriptors and bounds rather than future secret values. Effects whose
bytes depend on generated secrets or engine time bind their allowed schema and
generation action; the durable result records actual digests without exposing
secret payloads. Byte determinism applies to the same explicit identity and
input set, not independently generated installation identities.

The [bootstrap owner](../../reference/agent-bootstrap.md#installed-specialization)
defines the generic provider-neutral seat and the separately selected Clyffy
specialization for this repository. Do not hardcode Clyffy, a provider or a
language into the generic template. Existing project instruction files are
preserved by default. Any selected managed inclusion requires exact byte/owner
preconditions, a previewed change and a retained ownership receipt; refuse an
ambiguous merge rather than rewriting user instructions. Removal metadata is
required now; the uninstall executor remains D-11.

Persist the selected bootstrap through the existing installed binding/control
transaction and recovery flow. The minimal locator remains a locator, not a
specialization or policy database. The attunement job stays visibly pending;
installation must not pretend that the precondition inventory is D-03's
committed Git-correct source snapshot or that inactive capability IDs are
verified discoveries. Bind configured candidate revisions and explicit absent
states now; dynamic discovery/activation and external adapter execution stay
D-06/I. No external invocation is needed for default readiness. Unknown
effect kinds, unavailable templates and incompatible plan formats fail closed;
contract revisions may change without changing product version `1.0.0` or
adding a pre-alpha migration/fallback lane.

### D-01 acceptance matrix

All rows are required. This table retains the implementation baseline and
required behavior; the [accepted result matrix](gate-d-evidence.md#d-01-accepted-installed-foundation-2026-10-21)
binds each row to final observed evidence and its limits.

| Requirement | Retained baseline and remaining package | Required observable result |
|---|---|---|
| Primary executable and offline independence | PR 43; .5/.6 extend and reconcile | `version`, fresh/existing lifecycle, authenticated ready and commit/query succeed from the staged actual binary with denied outbound access and absent checkout/siblings. Record OS library assumptions; signed bundle/SBOM qualification stays J. |
| Pure and complete preview | Existing deterministic/no-write tests; .4 | Exact expected file/record/effect set and bounded inputs; no secret reads/generation, writes or external execution. User content hashes and canonical cursors do not change. |
| Identity, mode and exact authorization | PR 44; .3/.4 | Wrong digest, fresh/existing mode, root, executable, template/profile/specialization or configured provider revision deny before effects. Independent identical projects remain distinct; authorized post-install relocation preserves identity. |
| Native filesystem and credential boundary | PR 45 leaf-file proof; .1/.2/.5 | Private directory/file ownership, mode and DACL; no symlink/reparse/mount escape, hard-linked secret acceptance, external-path effects or secret disclosure. Changed/unsupported enforcement denies, with no silent repair. |
| Interrupted and concurrent apply | Eleven boundaries (ten durable effects plus lease); .3/.4/.5 | Before/after each credential preparation/delivery, engine commit, locator publication, acknowledgement and cleanup, plus new bootstrap effects: failure/kill/retry preserves a single authorized outcome. Competing/stale writers cannot mutate or acknowledge another admission. |
| Installed authority and normal open | Existing installed binding and read-only inspector tests; .2/.4/.6 | Missing/damaged policy or binding, unsupported format, foreign/partial state and already-installed cold start deny. `open_existing` never creates or repairs; readiness follows committed installed/security authority. |
| Persistence and preservation | Existing native restart and backup readback; .4/.5 | Authenticated committed data, identities, bootstrap ownership receipts and pending-job state agree after termination/reopen. Baseline quick verify changes no bytes. Fresh/existing project content is preserved. |
| Evidence and resource limits | Existing inventory ceilings and redaction; every package | Record outcome/denial, stage, plan/build identity, bounded work and durable result without plaintext credentials. Exhausted bounds terminate predictably. Traces explain observed effects; they are not durable authority or private chain-of-thought proof. |

### Commands, research and stop conditions

Each implementation PR names its new failing test and runs that first. Existing
owning commands, to retain and extend rather than substitute for missing cases:

```text
cargo test --locked -p rrd-engine --lib engine::token_key
cargo test --locked -p rrd-engine --lib engine::tests::installation
cargo test --locked -p rrd-engine --all-targets
cargo test --locked -p rrflow-cli --test installed_lifecycle -- --nocapture
cargo clippy --locked -p rrd-engine -p rrflow-cli --all-targets -- -D warnings
cargo fmt --all --check
python3 scripts/ci/run_offline_lifecycle.py
```

The D-01.5 [qualification candidate](../../evidence/change-journals/gate-d/d-01-native-qualification-candidate.md)
extends the last command with exact native enforcement and receipt requirements.
Windows policy setup is restricted to ephemeral hosted CI; macOS requires its
inherited native profile. Their accepted bounded native results are linked above; later revisions
require their own exact-candidate qualification. A contract-changing .4 PR additionally runs the
`rrd-contract` suite, generated-surface and affected SDK conformance checks.
Every package runs documentation/navigation/inventory checks and the full
candidate pipeline before merge. Exact commands, candidate results and retained receipts are in the accepted
evidence and immutable journals; this reconciliation changes no runtime code.

Native API selection is a qualification obligation. The primary
[Linux openat2 reference](https://www.man7.org/linux/man-pages/man2/openat2.2.html)
documents directory-relative resolution and separate symlink/mount controls;
[Microsoft CreateFileW](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilew)
documents handle sharing, security and reparse behavior; Apple's archived
[open reference](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/open.2.html)
documents target-symlink refusal, not a complete ancestor guarantee. Verify
actual supported OS/kernel/SDK/filesystem semantics in the implementing PR.
Do not assume Linux flags exist on macOS, or that a final-component no-follow
flag, path canonicalization, same-device check or lock alone confines an entire
operation. No upstream database implementation is needed for this work.

Stop an implementation package on an unprotected storage handoff, an effect
outside its reviewed closure, unsupported native enforcement, a changed
transaction/lifecycle authority, an unbounded retry/read, a secret-bearing
diagnostic, or a failed acceptance case. Split any newly discovered cross-owner
implementation into a separately scoped PR before mutation; retain the unmet
requirement here. Required evidence cannot be waived to make a platform green.
No same-user attacker immunity, atomic project snapshot, scale claim or release
readiness follows merely from directory handles or these bounded tests.

The subsequent alpha work is already ordered by the
[dependency spine](executable-dependency-spine.md): installed storage recovery
and lifetime, durable jobs and project inventory, native retrieval/indexes,
bounded analytics, complete attunement and operations, executable governed
reasoning, client equivalence, automation and release qualification. Finishing
the table above does not complete those downstream outcomes or change alpha
status.

## Connector and knowledge ingestion packages

The [knowledge connector contract](../../reference/data/knowledge-connectors.md)
defines target requirements. These packages refine existing D-05/D-06 work;
they add no completed item or new global execution wave. Specification and
fixtures may be prepared now; runtime work obeys the dependency spine.

| Package | Existing boundary and prerequisites | Required failure oracle and exit |
|---|---|---|
| D-06 configuration/discovery | `rrd-contract` executable capability catalogue, `endpoint_binding.rs`, `deployment.rs`; installed D-01 authority. H-04 later consumes the generated projection; full client parity is not a prerequisite for this core contract. | Generate requirements and availability from one owner; missing secret, unsupported adapter, expired observation, unauthorized and quota states remain distinct. Discover/preview causes no external call or mutation. Exact apply/CAS, revoke/restart and stale revision tests pass; H-04 proves CLI/MCP/SDK/UI parity. |
| D-06 bounded endpoint-configuration prerequisite | Owner-requested implementation of the existing endpoint profile at installed D-01 authority; [candidate journal](../../evidence/change-journals/gate-d/endpoint-profile-configuration.md). This brings forward only configuration publication, without advancing the dependency spine or full D-06. | Authenticated read/pure preview/exact-digest CAS apply; revisioned removal; MX/KV, installed HTTP, concurrent writers, foreign identity, lost response and reopen tests. No external activation, availability qualification or ingestion is implied. |
| D-06 knowledge adapter | Same binding/admission boundary; qualified configuration/discovery and explicit source credentials/egress | First read-only provider profile proves actual API/version, source identity/ACL, pagination, resource limits, cancellation and uncertainty. Foreign account, incomplete provenance, expired freshness and unsupported operation fail explicitly. Mock results cannot qualify HydraDB, Pinecone Nexus or Qdrant. |
| D-05 source and chunk provenance | Existing source/object/record/relation/schema, attunement job and semantic commit boundaries; D-02/D-03, qualified adapter and E/F access paths | Stable source/chunk identities, bounded decode, source-span readback, duplicate/restart and source-change/delete/ACL invalidation pass on installed rrflowKV. Publish only complete bounded groups; source snapshots and derived outputs remain distinct. |
| D-05 knowledge derivation | Qualified chunk provenance; existing inference/embedding and native graph/lexical/vector paths, H-09 family definitions | Extraction proposals retain source/model/configuration and verification; poisoned summary, conflicting entity, unsupported modality and incompatible embedding space cannot silently publish. Measure update/ingest cost and retrieval quality at equal token budgets before selecting a strategy. |
| D-06 bounded Parquet export prerequisite | Existing authenticated query pipeline and typed Arrow conversion; brought forward at the owner's request to bundle a callable export in the installed binary | Complete bounded query-result files preserve typed values and read/authorization identity; reject truncation, wrong scope/token, denied fields and exhausted budgets; actual installed binary decodes export before/after restart. This bounded prerequisite does not qualify import or Gate F streaming. |
| D-06 full Parquet exchange | Qualified source binding; existing `rrd-query` Arrow/DataFusion and engine mutation/export boundaries, F streaming/budget evidence | Pin reader/type/encoding profile. Malformed/oversized metadata, decompression expansion, lossy coercion, wrong stamp/ACL and interrupted publication fail without partial canonical writes or visible incomplete exports. Import/read/export match canonical values and reopened state; memory/scan/spill remain bounded. |

Each row lands as its own bounded implementation PR with exact symbols,
characterization tests and research provenance resolved from that candidate.
H-09 consumes accepted source/projection output; project links remain H-10,
and bidirectional synchronization remains H-07.

## DuckDB interoperability packages

**Withdrawn by the owner:** the earlier DuckDB interoperability and optional-worker
packages. DuckDB and ClickHouse are implementation research inputs, not
RRFlow runtime dependencies or integration requirements. Adapt selected
behaviors or eligible source to the existing native rrflowDB/rrflowKV and
rrd-query boundaries with file-level provenance, compatible licensing,
semantic and resource tests, and exact-candidate evidence. The existing D-05,
D-06 Parquet, E/F query and C-07 durability requirements continue unchanged.
[Research and prior direction](../../reference/data/duckdb.md) and
[POAM-036](../../poam/rrflow-1.0-alpha/poam-036.md) preserve history.

## Exit condition

Gate D exits only when a fresh and an existing project can be installed from
bundle-resident inputs through the single primary executable, attuned,
interrupted, resumed, served, authenticated, verified, repaired/restored under
the explicit safety contract, reopened, and uninstalled without damaging
project-owned state, with the network denied and no external database service.
