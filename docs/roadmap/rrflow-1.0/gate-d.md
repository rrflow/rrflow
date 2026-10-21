# Gate D — install, configure, and attune one real estate

**Status:** active canonical RRFlow 1.0 roadmap chapter
**Coordinate:** `rrflow://rrflow-instance/data/roadmap/rrflow-1.0/gate-d`
**Owner:** [RRFlow 1.0 release roadmap](../rrflow-1.0.md)

This chapter is part of the canonical RRFlow 1.0 roadmap. The parent
record owns release order and completion status; this file cannot change
either independently.

| Done | ID | Required change | Owning boundary | Acceptance evidence |
|---|---|---|---|---|
| [ ] | D-01 | Implement the [installed lifecycle](../../reference/deployment/installed-lifecycle.md) foundation and first walking product spine. The default release exposes one primary `rrflow`/`rrflow.exe` command tree. `rrflow install plan` resolves only bundle-resident, versioned project-bootstrap templates, provider-neutral specialization manifests, and attunement profiles; is byte-deterministic and side-effect free; and covers every file, record, process, adapter candidate, principal/role/grant, credential action, permission, budget, digest, and removal rule. `install apply` accepts the exact digest, explicit fresh/existing mode, and a minimal `.rrflow/config.toml` locator with no canonical mutable state or plaintext secret. Split rrflowKV create-new, open-existing, and read-only inspection so normal open never creates or repairs an estate. `initialize_instance` is the only cold-start path and atomically commits the installed binding, initial seat/security authority, checkpoint, audit, and commit evidence before readiness. Compose `serve`, authenticated `ready`, `version`, and baseline read-only `verify` through the same executable and `RrdEngine`; reusable server/MCP code is linked as libraries rather than checkout-built readiness prerequisites. Remove the old supervisor, standalone bootstrap, server initializer, create-on-open/startup binding, private marker/JSON state, and their successful fixtures in the same direct-convergence package after replacement evidence passes. | `rrflow-cli`, `rrd-engine`, `rrd-store`, `rrd-server`, `rrflow-mcp` | Native real-process tests for Linux and the available platform development runners prove `version`; deterministic plan; no-write preview; exact apply; one high-entropy credential through the previewed sink; authenticated readiness; one commit/query; close/reopen; and baseline read-only verification through the primary executable, with outbound network denied and sibling directories absent. Fresh and existing projects preserve user content and any managed instruction edit is ownership/digest bound. Interruption tests cover staging, credential prepare/delivery, engine commit, locator publication, acknowledgement, and cleanup with no duplicate credential/policy/audit outcome. Drift, path/link/mount replacement, ACL/mode, provider revision, partial state, stale lease, unsupported format, missing installed binding, and already-installed cold start fail closed. `rrflow dev doctor`, a PID/port/marker, source topology, and separate binaries cannot satisfy or substitute for this proof. |
| [ ] | D-02 | Persist attunement jobs and checkpoints through `RrdEngine`; implement status, resume, cancel, leases, idempotency, and phase input/output digests. | `rrd-engine` | Kill/restart tests at each transition resume committed work once and never infer completion from emitted events. |
| [ ] | D-03 | Implement only deterministic project-tree inventory: engine-authorized metadata/content reads, pure proposal building, Git-correct ignore rules, secret/generated/vendor/cache exclusions, stable root-relative identities, Merkle tree digest, containment edges, incremental change set, explicit errors, and bounded work accounting. | `rrd-attunement` through `rrd-engine` | This repository produces the same committed `SourceTreeSnapshot` digest under different traversal schedules; excludes `.git`, `target`, `node_modules`, rrflowDB files, generated bulk data, and secret payloads; handles tracked ignored files, non-Git roots, hidden source, symlink/mount escape, file races, cancellation/restart, and close/reopen; an unchanged rerun reads zero content bytes and commits a no-work checkpoint. |
| [ ] | D-04 | Add incremental Tree-sitter parsing pinned to one committed source-tree snapshot/change-set digest and parser/language revisions; preserve incomplete syntax and errors as evidence. | `rrd-attunement` through `rrd-engine` | Parse is denied before the D-03 snapshot/checkpoint commit; edit-one-file reparses only the changed source, preserves unaffected identities, records `ERROR`/`MISSING` nodes, and resumes after process restart. |
| [ ] | D-05 | After Gates E and F pass, implement normalize, entity-link, lexical-index, embed, vector-index, graph, ground, and verify one at a time against the accepted persistent/index/Arrow paths; add no temporary snapshot index or parallel attunement store. | `rrd-engine` plus owning subsystem | Every phase has an exact fixture, durable checkpoint, failure/retry case, output digest, physical-plan/trace evidence, and independent acceptance test before the next phase begins; lexical/vector/graph output is readable through the same native and analytical paths used outside attunement. |
| [ ] | D-06 | Classify project generators, package scripts, build/test/evaluation harnesses, CI/deployment tools, SQL, PostgreSQL, Turso, Dragonfly, object stores, models, meshes, and other application/operator systems as optional external capabilities or sources. Discovery creates an inactive typed candidate; only an exact previewed immutable binding plus explicit configuration, policy, and authorization may activate it. A command binding captures either one authenticated direct-process argv or the complete package-script/toolchain/lockfile/wrapper/lifecycle closure; literal package-manager argv alone is insufficient. Never install, start, execute, or select an external system as RRFlow persistence or lifecycle authority implicitly. | `rrd-contract`, `rrd-attunement`, `rrd-engine` | Fresh/existing fixtures prove RRFlow reaches default readiness without an external capability; discovery executes nothing; preview performs no writes or effects; exact apply/retirement is durable and idempotent; a generator or harness binding is source- and digest-bound and declares its permissions, sandbox, budgets, cancellation, uncertainty, verification, and output policy without invoking it; mutable/incomplete closures and unavailable enforcement fail closed; credentials are never copied; and clean uninstall removes only RRFlow-owned integration state. Project-command execution and output re-inventory are accepted later by I-03/I-07. |
| [ ] | D-07 | Bind installation to the DevForge CoW placement contract: immutable tools/models may live in the shared lower layer; workspace changes and all rrflowKV WAL, manifest, segment, catalogue, and graph state live in the writable upper layer. | `rrflow-cli`, DevForge adapter | Two clones share the same lower digest while independent writes, crash recovery, and deletion in one upper layer cannot affect the other. |
| [ ] | D-08 | Recognize content-addressed dependency mounts as shared immutable inputs rather than copying or attuning dependency caches into each estate. | `rrd-attunement`, DevForge adapter | Rust, Go, Node, and model-cache fixture proves stable mount digests, zero duplicate ingestion, and correct invalidation when a mounted digest changes. |
| [ ] | D-09 | Measure logical size, allocated blocks, compression, WAL growth, and snapshot size separately; never infer zero-byte or sparse-allocation savings from logical file size. | `rrd-lsm`, release harness | Fresh clone and sustained-write reports account for lower, upper, cache, WAL, segment, and snapshot bytes with reproducible filesystem commands. |
| [ ] | D-10 | Add a hibernation preparation/restore contract that quiesces writes, captures a verified rrflowKV snapshot boundary, exports to the configured cold tier, and resumes without changing estate identity. | `rrd-engine`, DevForge adapter | Interrupted export, corrupt object, restore, rollback, and hot-to-cold-to-hot tests prove no acknowledged-write loss and no split authority. |
| [ ] | D-11 | Complete the installed operational lifecycle: offline read-only quick/full verification; digest-bound offline repair plan/apply; authenticated backup verification; restore into an absent staging target plus explicit publication; explicitly non-authoritative salvage; and ownership-aware uninstall that retains rrflowDB/backups and reports `RETAINED`. Repair may truncate only a proven incomplete final WAL frame, delete only unreachable physical objects, rebuild derived graph/BM25/vector/TurboQuant/analytical projections from canonical semantic state and exact source cursors, or reconcile durable install/attunement receipts. It never guesses manifest, sequence, transaction, authorization, or acknowledged semantic truth, and it never edits the live root in place. | `rrd-engine`, `rrd-store`, `rrd-lsm`, `rrflow-cli` | On the D-01-installed binary, corruption/fault fixtures prove verify changes no bytes; every plan is deterministic and stale-plan rejection occurs before mutation; apply requires an exclusive offline lease, authenticated snapshot, exact preconditions, an absent reflink/clone or fully accounted same-filesystem candidate, audit receipt, full candidate verification, one durable atomic locator replacement, and retention/quarantine of the prior root. Complete-frame/canonical corruption routes to absent-target restore or labeled salvage rather than reset; projection rebuild matches exact oracles; backup restore never overlays live state; and uninstall removes only unchanged RRFlow-owned integration while preserving project state and retained data. |

## [Accepted evidence](gate-d-evidence.md)

## D-01 remaining closure packages

D-01 remains the active dependency-spine item and an OBJ-07 prerequisite.
This is its implementation order, reconciled on 2026-10-21 against
`1815af32a53e161bc5f3d7f0f7b4365462cd9f7a`. It refines the D-01 row above;
it neither changes that row's acceptance nor creates another status owner.
The [bounded evidence baseline](gate-d-evidence.md#d-01-bounded-baseline-2026-10-21)
records PRs 43, 44 and 45. Preserve their passing offline Linux, native staged
process, distinct-identity and private secret-file fixtures. Do not rebuild
those capabilities merely because the parent item is open.

The sequence is **directory security -> filesystem containment -> installation
exclusivity -> complete bootstrap -> native qualification -> closure**. The
first implementation PR is D-01.1 below. Each package requires a fresh source
reconciliation, pre-edit PR brief, linked journal and successful candidate CI.
Package numbers are subdivisions of D-01, not additional release gates.

| Package / state | Current behavior and canonical implementation scope | First failure oracle and exit evidence |
|---|---|---|
| **D-01.1 Private estate directories / next** | `engine/estate_layout.rs::create_estate_directories` requests 0700 on Unix but uses default Windows permissions; recovery entry validation checks names/types without a complete directory ownership policy. Extend that boundary and the existing `engine/token_key/private_file` native helpers as appropriate. Keep project-owned parents unchanged. | Create an estate under permissive/inherited ACLs and retry with substituted ownership/mode/DACL. Newly created RRFlow directories must have the declared private policy from creation; incompatible existing directories fail before secret delivery or store mutation, without silently repairing permissions. Prove Windows and Unix behavior, foreign-owner denial where the runner can exercise it, byte stability on denial, and the retained eight-stage recovery corpus. This alone does not prove ancestor containment. |
| **D-01.2 Anchored filesystem operations / planned after .1** | `safe_join` and `reject_existing_symlink_path` check names then return paths; installation reads, publication and cleanup reopen or unlink by name. Replace this check/use gap at `estate_layout.rs`, `installation.rs`, `installation/recovery.rs` and their storage-opening handoff, preserving `RrdEngine` and existing rrflowKV create/open/inspection authority. Map every affected store call before editing its owner. | Deterministically replace a project/managed ancestor between validation and read/create/publish/cleanup, including Windows junction/reparse and Unix symlink/mount cases. No outside sentinel is read, written, disclosed or deleted. Hold and validate native directory/object identities for each effect; detect unsupported enforcement before effects. Prove locator/pending-file publication and intent acknowledgement target only the authorized objects. Carry the protection through storage open, inspection and retained handles; validating a parent and then handing an unchecked `PathBuf` to storage is not an exit. |
| **D-01.3 Exclusive install and recovery / planned after .2** | `apply_installation_with_observer` publishes a create-new intent before opening the store; `acknowledge_intent` cleans up after publication. Store writer exclusion does not by itself prove exclusivity over this entire interval. Qualify and, where the oracle fails, implement the local privileged fresh-target lease required by the bootstrap contract at this existing authority. | Two real processes apply the same plan, competing plans, and recovery versus apply at controlled barriers before intent, store, locator and cleanup. Exactly one admitted writer may produce effects; the loser returns a bounded conflict. Kill the owner and retry: one credential, policy, checkpoint, audit outcome and locator remain. A stale owner/lease cannot clear a newer admission. Neither elapsed time nor a PID alone permits takeover. Reuse existing lock semantics where proven; add no sidecar lifecycle database or client lock authority. |
| **D-01.4 Complete bundled bootstrap / planned after .3** | `rrd-contract/src/attunement.rs::InstallationPlan` validates eight fixed managed paths; `DefaultInstallProfile` embeds a default configuration and phase list. Extend those canonical types, engine `assets/install/`, `plan_installation`, `validate_sealed_preview`, `installation_actions`, apply/recovery and CLI projection together. Supply actual versioned project templates and specialization manifests, not only IDs or another documentation catalogue. | An independently enumerated expected-effect fixture detects any unpreviewed file, transient path, canonical record or invocation. Repeated preview of identical explicit inputs is byte-identical and performs no mutation, credential generation/read, process start or provider call. Exact apply materializes only the declared closure; template/specialization/adapter revision, source or permission drift fails before effects. Fresh and existing projects preserve owned content; interruption at each newly introduced effect resumes without duplicates. Contract, schema and client projections are generated from their existing owner and qualified together. |
| **D-01.5 Native adversarial and offline qualification / planned after .4** | Extend `engine/tests/installation.rs`, `installation_recovery.rs`, native private-file tests, `rrflow-cli/tests/installed_lifecycle.rs`, `scripts/ci/run_offline_lifecycle.py` and the existing native CI lanes. Current Linux offline proof and Windows/macOS staged runs are different evidence; preserve that distinction. | Run the matrix below on native Linux, Windows and macOS using actual primary executables. Add asserted outbound denial and checkout/sibling absence for each claimed offline platform, plus deterministic native path/ACL/lease failures. A missing required privilege, unavailable enforcement or skipped adversarial case is an open qualification gap, never a pass. Keep finite deadlines, output ceilings and retained failure artifacts. Do not replace native evidence with cross-compilation or mocks. |
| **D-01.6 Acceptance reconciliation / planned after .5** | Join exact-candidate results in this chapter's accepted-evidence record and reconcile POAM-006. Existing test names are an inventory, not proof that every requirement passed. | At one candidate revision, account for every D-01 requirement, OS/filesystem profile, binary digest, command, case count, denial and retained receipt. No required row may be missing or skipped. Only then mark D-01 complete and advance the dependency spine to C-07; parent OBJ-07 and alpha remain open. |

### Bootstrap scope and durable-effect boundary

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

All rows are required for closure. The baseline column identifies retained
evidence, not a claim that the full row has already passed.

| Requirement | Retained baseline and remaining package | Required observable result |
|---|---|---|
| Primary executable and offline independence | PR 43; .5/.6 extend and reconcile | `version`, fresh/existing lifecycle, authenticated ready and commit/query succeed from the staged actual binary with denied outbound access and absent checkout/siblings. Record OS library assumptions; signed bundle/SBOM qualification stays J. |
| Pure and complete preview | Existing deterministic/no-write tests; .4 | Exact expected file/record/effect set and bounded inputs; no secret reads/generation, writes or external execution. User content hashes and canonical cursors do not change. |
| Identity, mode and exact authorization | PR 44; .3/.4 | Wrong digest, fresh/existing mode, root, executable, template/profile/specialization or configured provider revision deny before effects. Independent identical projects remain distinct; authorized post-install relocation preserves identity. |
| Native filesystem and credential boundary | PR 45 leaf-file proof; .1/.2/.5 | Private directory/file ownership, mode and DACL; no symlink/reparse/mount escape, hard-linked secret acceptance, external-path effects or secret disclosure. Changed/unsupported enforcement denies, with no silent repair. |
| Interrupted and concurrent apply | Eight typed stage tests; .3/.4/.5 | Before/after each credential preparation/delivery, engine commit, locator publication, acknowledgement and cleanup, plus new bootstrap effects: failure/kill/retry preserves a single authorized outcome. Competing/stale writers cannot mutate or acknowledge another admission. |
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

The last command currently requires Linux namespaces; it is not a Windows or
macOS offline command. D-01.5 must publish the exact native enforcement commands
with its runner changes. A contract-changing .4 PR additionally runs the
`rrd-contract` suite, generated-surface and affected SDK conformance checks.
Every package runs documentation/navigation/inventory checks and the full
candidate pipeline before merge. These are prospective acceptance commands,
not tests executed by this planning update.

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
| D-06 knowledge adapter | Same binding/admission boundary; qualified configuration/discovery and explicit source credentials/egress | First read-only provider profile proves actual API/version, source identity/ACL, pagination, resource limits, cancellation and uncertainty. Foreign account, incomplete provenance, expired freshness and unsupported operation fail explicitly. Mock results cannot qualify HydraDB, Pinecone Nexus or Qdrant. |
| D-05 source and chunk provenance | Existing source/object/record/relation/schema, attunement job and semantic commit boundaries; D-02/D-03, qualified adapter and E/F access paths | Stable source/chunk identities, bounded decode, source-span readback, duplicate/restart and source-change/delete/ACL invalidation pass on installed rrflowKV. Publish only complete bounded groups; source snapshots and derived outputs remain distinct. |
| D-05 knowledge derivation | Qualified chunk provenance; existing inference/embedding and native graph/lexical/vector paths, H-09 family definitions | Extraction proposals retain source/model/configuration and verification; poisoned summary, conflicting entity, unsupported modality and incompatible embedding space cannot silently publish. Measure update/ingest cost and retrieval quality at equal token budgets before selecting a strategy. |
| D-06 bounded Parquet export prerequisite | Existing authenticated query pipeline and typed Arrow conversion; brought forward at the owner's request to bundle a callable export in the installed binary | Complete bounded query-result files preserve typed values and read/authorization identity; reject truncation, wrong scope/token, denied fields and exhausted budgets; actual installed binary decodes export before/after restart. This bounded prerequisite does not qualify import or Gate F streaming. |
| D-06 full Parquet exchange | Qualified source binding; existing `rrd-query` Arrow/DataFusion and engine mutation/export boundaries, F streaming/budget evidence | Pin reader/type/encoding profile. Malformed/oversized metadata, decompression expansion, lossy coercion, wrong stamp/ACL and interrupted publication fail without partial canonical writes or visible incomplete exports. Import/read/export match canonical values and reopened state; memory/scan/spill remain bounded. |

Each row lands as its own bounded implementation PR with exact symbols,
characterization tests and research provenance resolved from that candidate.
H-09 consumes accepted source/projection output; project links remain H-10,
and bidirectional synchronization remains H-07.

## Exit condition

Gate D exits only when a fresh and an existing project can be installed from
bundle-resident inputs through the single primary executable, attuned,
interrupted, resumed, served, authenticated, verified, repaired/restored under
the explicit safety contract, reopened, and uninstalled without damaging
project-owned state, with the network denied and no external database service.
