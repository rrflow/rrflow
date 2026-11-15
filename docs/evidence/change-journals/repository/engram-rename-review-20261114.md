# The estate-to-engram rename: what the archived attempt teaches

**Status:** active change evidence; the rename itself is open under POAM-078
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/repository/engram-rename-review-20261114`
**Owner:** [POAM-078](../../../poam/rrflow-1.0-alpha/poam-078.md)

**What this is.** A read-only review of the archived worktree `wip/engram-rename-diagnosis-20261106`, written on 2026-11-14 by a reviewing agent against `main` at `2b14c3ff1`. It is the plan input for the rename: the old-to-new map, the compatibility hazards, an inventory of what to rename, and a recommended order. **Its claims are leads, not verified facts.** The reviewer ran no builds. What was re-proven on `main` afterwards is listed in [the ledger](archive-absorption-20261114.md) and [the journal for the five dropped behaviours](engram-rename-dropped-behaviours-20261114.md); everything else needs its own check before it is relied on. File counts were taken at that revision and will have drifted.

Reviewed 2026-11-14 against `development/main` = `2b14c3ff1`. Nothing in the repository was changed by this review; no cargo, tests or builds were run. During the review `development/main` advanced to `a60255450` ("Absorption ledger for the archive; record POAM-081", five documentation files, +109 lines, no code) and `docs/poam/rrflow-1.0-alpha/poam-081.md` was being edited in the working tree by someone else; file counts may be off by one or two documentation files as a result. all `path:line` citations for main were taken with `git show`/`git grep development/main`, except `docs/roadmap/kernel-contract.md`, `docs/architecture/code-structure-and-conventions.md` and `scripts/ci/*.py`, which were read from the working tree. Every count below comes from a `git grep`/`git diff` run in this session; anything not read is marked "not read".

## Summary

| ref | verdict | VALUABLE items | one-line reason |
|---|---|---|---|
| `refs/archive/20261114/wip/engram-rename-diagnosis-20261106` (`d7ef0a13b`) | **PORT the method and five dropped behaviours; DROP the tree itself** | 7 (2 rename techniques, 5 behaviours main lost) | The tree is a repaired blanket substitution on an 8-day-old main and cannot be applied; but it holds the only worked old→new map, the only measured compatibility evidence, and five pieces of engine behaviour that a later merge silently dropped from main. |

Three facts that change how the rename must be planned:

1. **Main was never renamed, including the TypeScript SDK.** Commit `bf6dec69f` is titled "rename 'estate' to 'engram' in TypeScript SDK and tests" but is a 637-file, 4598±4598-line case-insensitive substitution of the whole repository. PR #290 then merged main back over it and "restore[d] current-main public and persisted identity" (`docs/evidence/change-journals/gate-d/d-01-pr290-current-main-installer-reconciliation.md`, change brief). Today `sdks/typescript/src/operation.ts:31,92-93` and `sdks/typescript/tests/sdk-conformance.ts:327-332` say `estate`.
2. **Main carries a half-renamed residue** from that episode: miscased aliases (`InstalledengramRecord`, `PersistMemoryengram`, `MemoryengramPlan`, `DataengramOrganisation`), two planners, and an unused `ResourceKind::Engram`. This is already recorded as POAM-078.
3. **The same merge dropped five behaviours from main** that the archive still has (section 2). Two of them leave main's documentation describing code that does not exist, and one looks like a live defect (routing input cannot narrow candidates).

---

## 0. What the ref actually is (history)

- `git log -1 --format=%P d7ef0a13b` → parents `bf6dec69f` and `7fb5e90a4` (an index commit). It is a stash-shaped save of the worktree `agent/engram-build-repair-20261107`. `git merge-base development/main <ref>` = `bf6dec69f`, which **is an ancestor of main**. So "commits main lacks" is zero real commits; everything is in `git diff bf6dec69f d7ef0a13b` (617 files, +12013 −2473).
- That diff is three things mixed together:
  1. a merge of then-current main `5f8a7e367` (2026-11-07, "Merge pull request #230 … task-budget-resume") into the candidate — this is where the 112 added files (task-budget-resume receipts, e-04 journals, POAM-050..052) come from, and all of it is on main already;
  2. the rename repair (file moves, casing repair, restoration of 244 historical records);
  3. six evidence/POA&M additions of its own.
- Isolating (2)+(3): comparing the ref with `5f8a7e367` after normalising every case-variant of estate/engram to one token gives **403 changed files = 352 rename-only + 2 digest-only + 43 with other content + 6 added**. The 43+6 are section 2.
- Against today's main, 238 of the 617 paths still differ (216 code). This is almost entirely main's later progress (module splits on 11-13/14) plus the rename, not unported work.

Timeline, from commit metadata:

| when | what | evidence |
|---|---|---|
| 2026-11-06 19:35 | Owner commits the blanket substitution | `bf6dec69f`, 637 files, no path renames (`git diff --name-status -M bf6dec69f^ bf6dec69f` shows only `M`) |
| 2026-11-07 | Local repair in this worktree (the archived ref) | journal `docs/evidence/change-journals/gate-h/engram-connectome-repair-2026-11-07.md` on the ref |
| 2026-11-08/09 | Copilot agent commits on the PR branch, then "Merge main into the draft PR" | `d5d6385d1`, `d54ed35f0`, `4c2f2a6a1` |
| 2026-11-09 | PR #290 merges; estate names restored on main | `a9f1b0fb8`; journal `gate-d/d-01-pr290-current-main-installer-reconciliation.md` |
| 2026-11-11 | Half-rename recorded | `docs/poam/rrflow-1.0-alpha/poam-078.md` |
| 2026-11-13 | Plan places the rename "inside K13, after K4" | `docs/roadmap/kernel-contract.md:327` |

---

## 1. What the attempt did — the old → new map

### 1.1 Two layers

**Layer A, the blanket commit `bf6dec69f`:** every case-insensitive `estate` became lowercase `engram`. Token pairs extracted from `git diff -U0 bf6dec69f^ bf6dec69f` (186 distinct pairs, 0 unpaired lines), for example:

| before | after (broken) | count |
|---|---|---|
| `EstateAdmin` | `engramAdmin` | 79 |
| `InstalledEstateIdentity` | `InstalledengramIdentity` | 58 |
| `ESTATE_DIRECTORY` | `engram_DIRECTORY` | 11 |
| `WebSocketReceiveState` | `WebSocketReceivengram` | 17 |
| `PlacementNodeState` | `PlacementNodengram` | 9 |
| `KnowledgeState` | `Knowledgengram` | 3 |
| `restated` | `rengramd` | 1 |
| `tracestate` (W3C header) | `tracengram` | docs |

**Layer B, the archived repair:** it fixed Rust identifiers to proper casing, moved files, and restored the wrongly hit `…eState` types. Token pairs from `git diff -U0 -M40 5f8a7e367 <ref>` over code (256 pairs):

### 1.2 The map, by kind

**Rule for Rust identifiers** (applied to all 70 public types and every function/const): `Estate`→`Engram`, `estate`→`engram`, `ESTATE`→`ENGRAM`, inside the identifier, nothing else changed.

| kind | old | new | where on the ref |
|---|---|---|---|
| Crate | `rrd-estate` / `rrd_estate` (111 uses) | `rrd-engram` / `rrd_engram` | `crates/authority/rrd-engram/Cargo.toml:2`; root `Cargo.toml` member |
| Crate dir | `crates/authority/rrd-estate/**` (18 files) | `crates/authority/rrd-engram/**` | rename status `R084`–`R100` |
| Module files | `engine/estate.rs`, `estate_control.rs`, `estate_layout.rs`, `memory_estate.rs` | `engine/engram.rs`, `engram_control.rs`, `engram_layout.rs`, `memory_engram.rs` | `crates/authority/rrd-engine/src/engine/` |
| Test modules | `engine/tests/estate_control.rs`, `estate_organisation.rs`, `memory_estate.rs` | `…/engram_control.rs`, `engram_organisation.rs`, `memory_engram.rs` | same |
| Contract module | `rrd-contract/src/memory_estate.rs` | `memory_engram.rs` | `R098` |
| Asset | `assets/install/default-estate-configuration-v3.toml` | `default-engram-configuration-v3.toml` | `R100`; referenced at `installation.rs:34,256` and both profile JSONs line 4 |
| Binary source | `src/bin/rrd-estate-admin.rs` | `src/bin/rrd-engram-admin.rs` | `R094` |
| Types (examples) | `EstateConfiguration`, `InstalledEstateIdentity`, `EstateSnapshot`, `ReadEstate`, `DataEstateOrganisation`, `RuntimeEstateOrganisation`, `PersistMemoryEstate`, `MemoryEstatePlan`, `LocalEstatePermission`, `EstateRepository`, `EstateDriver` | `EngramConfiguration`, `InstalledEngramIdentity`, `EngramSnapshot`, `ReadEngram`, `DataEngramOrganisation`, `RuntimeEngramOrganisation`, `PersistMemoryEngram`, `MemoryEngramPlan`, `LocalEngramPermission`, `EngramRepository`, `EngramDriver` | all 70; full list of today's names in Appendix B |
| Enum variants | `ResourceKind::Estate`, `SecurityAction::EstateRead/EstateAdmin`, `RrdOperation::EstateRead/EstateAdmin`, `ServiceError::EstateInUse`, `CreateEstateDirectories`, `EstateDirectory`, `EstateDirectoriesReady`, `OpenPublishedEstate` | `…Engram…` | `ResourceKind` on the ref has only `Engram` (`rrd-contract/src/lib.rs:5598`) |
| Constants | `ESTATE_DIRECTORY`, `PRIVATE_ESTATE_DIRECTORIES`, `ADOPTED_ESTATE_DIRECTORY`, `MAX_ADOPTED_ESTATE_ENTRIES`, `ESTATE_FORMAT`, `ESTATE_AUTHORITY_FORMAT`, `ESTATE_CONFIGURATION_FORMAT_VERSION`, `ESTATE_RECOVERY_OPERATION_FORMAT`, `MAX_ESTATE_GRANTS` | `ENGRAM_…` | values unchanged: `ENGRAM_DIRECTORY = ".rrflow"` (`engram_layout.rs:7`) |
| Operation id | `"estate-read"` | `"engram-read"` | `rrd-contract/src/lib.rs:3928`, `operation_reference.rs:271`, `rrd-server/src/http/router.rs:81` |
| HTTP path | `/v1/estates/{estate}/read` | `/v1/engrams/{engram}/read` | `rrd-contract/src/lib.rs:3930`, `rrd-client/src/operation.rs:768`, `rrd-server/src/http/capabilities.rs:31` |
| Capability name | `"estate-authority-read"` | `"engram-authority-read"` | `rrd-server/src/http/capabilities.rs:206` |
| Serialized fields | `estate_id`, `estate_revision`, `expected_estate_revision`, `based_on_estate_revision`, `installed_estate`, `adopted_estate_entries`, `estate`, `estates` | `engram_id`, `engram_revision`, … `engram`, `engrams` | 135 uses of `engram_id` |
| Serialized variants | `create_estate_directories`, `estate_directory`, `estate_directories_ready`, `estate_read`, `estate_admin`, resource kind `"estate"` | `create_engram_directories`, … `"engram"` | `installation/recovery.rs:32` |
| Storage keys | `server/state/estate/{id}/document`; `server/state/estate-recovery/{id}/{kind}/{identity}`; backup scope `server/state/estate` | `server/state/engram/…`; `server/state/engram-recovery/…` | `rrd-engram/src/lib.rs:753`; `engram_control.rs:1370`; `backup.rs:338`; `rrd-store/src/archive/control.rs:164-165,532-537` |
| Digest domain separators | `rrflow-estate-configuration-v3\0`; `rrflow-estate-organisation-v1\0`; `rrflow-installed-estate-v3`; `rrd-estate-backup-create-v1`; `rrd-local-estate-backup-v1`; `estate-reconcile-v1`; `estate-backup-reconcile-v1`; `estate-recovery-{prune,restore}(-audit)-v1`; `estate-recovery-closure-v1` | same with `engram` | `rrd-contract/src/deployment.rs:685`; `rrd-core/src/runtime.rs:2418`; `installation.rs:1618`; `rrd-engram/src/backup_job.rs:794`; `engram_control.rs:400,457,531,553,768,792,958,1234` |
| Generated id prefix | `canonical_prefixed("estate", …)` → ids like `estate-419b…` | `canonical_prefixed("engram", …)` → `engram-e944…` | `installation.rs:575,1702`; `fixtures/rrd-sdk-conformance-v1.json:23` |
| Journal action names | `estate.create`, `estate.desired.set`, `estate.operation.lease/receipt`, `estate.instance.observe/activity`, `estate.activity.refresh`, `estate.authority.apply`, `estate.backup.{schedule,lease,prepared,completed,failed}`, `estate.recovery.{policy.set,pin.created,pin.released,restore.verified,prune.prepared,prune.completed,…}` | `engram.…` | `rrd-engram/src/lib.rs:776-1184`, `backup_job.rs:265-437`, `recovery.rs:250-528` |
| Schema migration string | `"memory estate bootstrap"` | `"memory engram bootstrap"` | `memory_engram.rs:297`; also changed inside the frozen fixture, see 4.3 |
| CLI flag | `--estate` | `--engram` | `rrd-backup-controller.rs:119`, `rrd-engram-admin.rs:75`, `rrd-estate-controller.rs:116`, `rrd-recovery-controller.rs:118` |
| CLI defaults | `"estate-authority"`, `"estate-control"` | `"engram-authority"`, `"engram-control"` | `rrd-backup-controller.rs:184`, `rrd-recovery-controller.rs:192` |
| Error/usage strings | `"rrd-estate-admin: {error}"`, `"installed estate record is missing"` | `"rrd-engram-admin: …"`, `"installed engram record …"` | `rrd-engram-admin.rs:20,273` |
| Env vars (test hooks) | `RRD_ESTATE_TEST_HOLD_AFTER_{EFFECT,STEP}_FILE` | `RRD_ENGRAM_TEST_HOLD_AFTER_…` | `rrd-estate-controller.rs:10-11` |
| CI | job `estate-portability`; `-p rrd-estate`; `--bin rrd-estate-controller`; `--test local_estate_driver`; `installed_two_estate_isolation_under_concurrent_load` | `engram-portability`; `-p rrd-engram`; `--bin rrd-engram-controller`; `--test local_engram_driver`; `…two_engram_isolation…` | `.github/workflows/ci-reusable.yml:153-850`; `scripts/ci/check_workflow.py:568-1070` |
| SDK hand-written | resource segment `"estate"`, parameter `estate` | `"engram"`, `engram` | `sdks/go/operation.go:21,197-202`; `sdks/python/src/rrd_client/operation.py:18,74-75`; `sdks/typescript/src/operation.ts:31,92-93` |
| SDK fixture | `"estate"`, `"estate_revision"` | `"engram"`, `"engram_revision"` | `fixtures/rrd-sdk-conformance-v1.json:11,23,320,328` |
| Test names | ~45 `…_estate_…` test functions | `…_engram_…` | e.g. `a_served_engram_is_found_proved_and_not_served_twice` |
| Docs prose | "estate" in ~170 non-evidence docs | "engram" | not individually reviewed |

**Not changed by the attempt, correctly:** the on-disk layout. `.rrflow`, `.rrflow/rrd`, `.rrflow/rrd/roots`, `.rrflow/credentials`, `.rrflow/config.toml`, `previous` have no "estate" in them on main (`crates/authority/rrd-engine/src/engine/estate_layout.rs:10-23`; `crates/transport/rrd-contract/src/attunement/installation.rs:198`). Only the constant names carry the word.

---

## 2. What it changed beyond renaming

Method: normalised comparison with `5f8a7e367` (the main it merged), then each surviving hunk checked on today's main by identifier. For the five "dropped" items the loss point is proven by counting the identifier in `crates/` across four commits: candidate side `d54ed35f0`, main side `ae01ce328`, merge result `4c2f2a6a1`, PR #290 merge `a9f1b0fb8`, and today's main.

| identifier | candidate | main side | merge result | #290 | main today |
|---|---|---|---|---|---|
| `project_map_line` | 3 | 0 | 0 | 0 | 0 |
| `"project-map.md"` (adoption list) | 1 | 0 | 0 | 0 | 0 |
| `the_project_map_describes_the_project…` (test) | 1 | 0 | 0 | 0 | 0 |
| `validate_flight_identity` | 2 | 0 | 0 | 0 | 0 |
| `selector_policy_id` | 7 | 0 | 0 | 0 | 0 |
| `selected worker adapter is not implemented` | 1 | 0 | 0 | 0 | 0 |
| `routing: selection.routing.clone` | 1 | 0 | 0 | 0 | 0 |
| `worker_adapter_profiles_are_typed_and_extensible` (test) | 1 | 0 | 0 | 0 | 0 |
| `narrow_allowance` | 4 | 0 | 3 | 3 | 3 |

These were authored in commits that are ancestors of main (`dd27aa96a` 2026-10-31, `232171961`, `a4db01615`, `b134b8bf3` 2026-11-05) and removed by merge-conflict resolution in `4c2f2a6a1`, so no commit on main shows a deletion.

### Item 2.1 — Project map is never refreshed — `VALUABLE, MISSING`

- **Ref:** `crates/adapters/rrflow-cli/src/installed.rs` adds `fn project_map_line(project: &Path) -> String` calling `RrdEngine::refresh_project_map(project)`, used after `install apply` and in `serve`; output lines `"project map: {} ({} files{})"` / `"project map: not refreshed ({error})"`. `engram_layout.rs` adds `"project-map.md"` and `".project-map.pending"` to the list of `.rrflow` entries a reinstall recognises as RRFlow's own. Test `the_project_map_describes_the_project_and_keeps_the_owners_notes` in `engine/tests/installation.rs` (60 lines: installs, asserts `first.files == 6`, the `| \`src/\` | 2 | 400 B | TypeScript |` row, that edited notes survive, that an unchanged project is not rewritten, and that `.rrflow/.project-map.pending` is gone).
- **Main:** `refresh_project_map` is defined at `crates/authority/rrd-engine/src/engine/project_map.rs:36` and exported at `engine/mod.rs:185`, but `git grep refresh_project_map development/main -- crates scripts sdks` finds only the definition. No caller, no test. `docs/reference` has no "Project map" section (the ref adds one to `docs/reference/deployment/installed-lifecycle.md`, "### Project map", 19 lines, citing the owner decision of 2026-10-31).
- **Target today:** caller in `crates/adapters/rrflow-cli/src/installed.rs` (main has `estate_open_elsewhere` at :433 in the same file, so the serve path is there); adoption entries in `crates/authority/rrd-engine/src/engine/estate_layout.rs` near `existing_estate_entries` (:308) — **read main's function first**: on main it lists every entry except `previous` and has no allow-list, so the two-name addition may no longer have a place; test into `crates/authority/rrd-engine/src/engine/tests/installation/` (split folder).
- **Adapt:** main's `project_map.rs` already replaced the ref's hard-coded extension table with `super::source_language::source_language` (diff shows main −3/+33 the other way), so take only the caller, the test and the doc section, not the ref's `project_map.rs`.

### Item 2.2 — Selection proposals are not bound to their prompt flight — `VALUABLE, MISSING`

- **Ref:** `crates/authority/rrd-engine/src/engine/endpoint_proposal.rs` — when `request.flight` is present it calls `read_prompt_flight`, then `validate_flight_identity(request, flight, &observed_flight.record)`, which returns `TaskAttemptInvalidated(InputsChanged)` unless `observed.scope == request.scope`, `observed.task_id == Some(request.task_id)`, `observed.cohort_sha256 == expected.prompt_cohort_sha256` and the context-request digests match.
- **Main:** the contract type exists (`crates/transport/rrd-contract/src/endpoint_proposal.rs:225` `EndpointSelectionFlightIdentity`) and the engine copies it into the proposal (`crates/authority/rrd-engine/src/engine/endpoint_proposal.rs:509` `flight: request.flight.clone()`), but nothing reads the flight or compares it (`git grep 'read_prompt_flight\|cohort' …/endpoint_proposal.rs` → no match). `docs/reference/inference/lfg-swag.md` on the ref states the flight "must match the named task and the retained flight record exactly before the proposal is admitted"; whether main's copy of that doc carries the same sentence was not read.
- **Target:** `crates/authority/rrd-engine/src/engine/endpoint_proposal.rs` before the packet is built (~:178). No rename adaptation needed. No engine test for it exists on the ref either (the contract test `tests/endpoint_proposal.rs` is on main); one must be written.

### Item 2.3 — Routing input cannot narrow candidates in the engine — `VALUABLE, MISSING` (probable defect on main)

- **Ref:** same file computes `let allowance = policy.allowance(&configuration)?` then `routing.narrow_allowance(&allowance)?` when `request.routing` is present, and puts the narrowed allowance in the packet.
- **Main:** `crates/authority/rrd-engine/src/engine/endpoint_proposal.rs:187-191` builds the packet with the **un-narrowed** `policy.allowance(&configuration)` and `routing: request.routing.clone()`. The contract's own check, `crates/transport/rrd-contract/src/endpoint_proposal.rs:376-388`, expects `self.allowance == routing.narrow_allowance(&configured_allowance)` when routing is present and otherwise returns `"selection packet differs from the configured policy"`. The engine calls `packet.validate_for` at :396.
- **Consequence (inferred from the two reads above, not executed):** a request whose routing narrows to fewer candidates than configured should be rejected by main's own validation. `UNCLEAR` only in whether `validate_for` is reached on the non-LFG path; lines 396 sits inside the LFG advisory branch and lines 200-212 call `prepare_controller` — reading `prepare_controller` and line 521 (`.validate_for(request, &packet, &configuration)`) would settle it. A three-line test with `narrowed_candidate_count` below the configured count settles it faster.
- **Target:** same file, replace the `allowance:` expression at :187.

### Item 2.4 — Worker execution does not fail closed on an unimplemented adapter, and drops routing — `VALUABLE, MISSING`

- **Ref:** `crates/authority/rrd-engine/src/engine/endpoint_worker.rs` adds, after the worker binding lookup:
  `if !worker.adapter.is_native_chat_adapter() { return Err(ServiceError::EndpointCallFailed { reason: "selected worker adapter is not implemented by this engine build", execution_may_continue: false }); }`
  and sets `routing: selection.routing.clone()` on `TaskWorkerOutput`. Contract test `worker_adapter_profiles_are_typed_and_extensible` in `crates/transport/rrd-contract/tests/endpoint_worker.rs`.
- **Main:** `is_native_chat_adapter` is defined at `crates/transport/rrd-contract/src/endpoint_worker.rs:50` and called nowhere (`git grep` over `crates` returns only the definition). `crates/authority/rrd-engine/src/engine/endpoint_worker/execute.rs:618` sets `routing: None`. The test is absent. `docs/reference/inference/lfg-swag.md:259` on main says the engine "fails closed on unimplemented adapters" — the code does not.
- **Consequence:** a worker policy with `adapter: { kind: "other", … }` validates (`endpoint_worker.rs:382` calls only `adapter.validate()`) and would be executed as an OpenAI-style chat call.
- **Target:** `crates/authority/rrd-engine/src/engine/endpoint_worker/execute.rs` (split file; find the binding lookup that returns `"selected worker is not configured"`), and `crates/transport/rrd-contract/tests/endpoint_worker.rs`.

### Item 2.5 — Observation exports carry no selector identity — `VALUABLE, MISSING`

- **Ref:** `crates/operations/rrd-observation/src/lib.rs` adds three optional fields to the references struct — `selector_policy_id`, `selector_routing_sha256`, `selector_narrowed_candidate_count` — with validation (digest shape, id shape, count ≠ 0). `engine/observation_delivery/persistence.rs` fills them from `attempt.endpoint_proposal.routing` or `attempt.worker_output.routing` using `rrd_contract::endpoint_selection_routing_sha256`. Test additions in `engine/tests/reasoning_budget/worker/observation.rs` assert `references["selector_policy_id"] == "semantic-router-v1"`, the count and the digest.
- **Main:** `crates/operations/rrd-observation/src/references.rs:19` ends at `configuration_sha256`; no `selector_` identifier anywhere in `crates`. `docs/reference/inference/lfg-swag.md:88` on main documents all three fields as existing.
- **Target:** `crates/operations/rrd-observation/src/references.rs`, `crates/authority/rrd-engine/src/engine/observation_delivery/persistence.rs`, and the split test folder. Depends on 2.4 (worker output must carry routing).

### Item 2.6 — Historical-capture projection for tests — `VALUABLE, MISSING` (rename technique)

- **Ref:** new `crates/transport/rrd-contract/tests/support/observed_configuration.rs` (52 lines). `project(bytes, file_digest, captured_digest)` asserts the file's SHA-256 matches the manifest, parses it with a test-only `HistoricalConfiguration`/`HistoricalIdentity { estate_id }` (both `deny_unknown_fields`), asserts the canonical digest equals the receipt's, then builds a current `EndpointConfiguration` with `engram_id` and asserts its digest **differs** from the captured one. Used by `tests/endpoint_policy.rs`, `endpoint_selection.rs`, `endpoint_worker.rs`; those tests flip `assert_eq!` → `assert_ne!` on the receipt digest and derive a fresh allowance after asserting the captured one "cannot authorize the derived identity". The engine side (`engine/tests/reasoning_budget/proposal.rs:22-32`) does the same with `serde_json::Value`.
- **Why it matters:** the capture fixtures are immutable observations of a real provider run (section 4.3). This is the only approach on record that renames the field without rewriting captured bytes or their recorded digests.
- **Target:** same paths on main (the fixtures and the three tests are at the same paths; main also reads these fixtures from `crates/adapters/rrflow-cli/tests/installed_lifecycle/main.rs:46-50`, `crates/authority/rrd-engine/src/engine/tests/{endpoint_configuration.rs:130, reasoning_budget.rs:37, reasoning_budget/lfg_flow.rs:496, reasoning_budget/proposal.rs:15}`, `crates/compute/rrd-inference/tests/{endpoint_chat.rs:17,229, model_chat.rs:15}` — nine readers, the ref handled four).

### Item 2.7 — Measured compatibility evidence — `VALUABLE, MISSING` (as a record)

The ref's journal (`docs/evidence/change-journals/gate-h/engram-connectome-repair-2026-11-07.md`) and its `docs/poam/rrflow-1.0-alpha/poam-053.md` are the only place these measurements exist:

- OpenAPI before/after digests `e6bd6b60…` → `a444f482…`; "289 changed leaves across 80 groups, including renamed resource kinds, authorization actions, identity fields and paths".
- `install_and_attunement_contract_matches_golden_json` fails; the fixture needs exactly sixteen digest fields refreshed (confirmed here: `crates/transport/rrd-contract/fixtures/install-attunement-v2.json` is digest-only different, 16 lines) plus one in `public-contract-v1.json`.
- A real reopen probe: an engine built before the rename installed a project; the renamed engine "exited 1 … its TOML decoder rejected the persisted `estate_id` field as unknown and expected `engram_id`".
- "twelve operation descriptions" in `operation_reference.rs` had to be corrected to the renamed action symbols.
- The Rust-identifier pass: "79 diagnosed non-ambiguous Rust identifier names in 74 files (834 identifier occurrences)", then 73 files for casing.

The journal should be kept as history. **Its POAM number collides:** main's POAM-053 is "clients cannot build RRFlow's HNSW or payload indexes" (`docs/poam/rrflow-1.0-alpha.md:69`). Do not port the POA&M file; the finding belongs under POAM-078.

### Other differing code

| item | status | evidence |
|---|---|---|
| `EndpointWorkerAdapter` enum, `EndpointSelectionRoutingInput`, `EndpointSelectionFlightIdentity`, `narrow_allowance`, `endpoint_selection_routing_sha256` (contract) | `ON MAIN` | `crates/transport/rrd-contract/src/endpoint_worker.rs:14-60`, `endpoint_proposal.rs:123,134,173,225` |
| `project_entries` / `ProjectEntry` | `ON MAIN` | `crates/authority/rrd-engine/src/engine/installation/inventory/entries.rs:46` |
| `project_map.rs`, `ProjectMapSummary`, `PROJECT_MAP_RELATIVE_PATH` | `ON MAIN` (newer) | `crates/authority/rrd-engine/src/engine/project_map.rs:15,36` |
| `tests/endpoint_proposal.rs` | `ON MAIN` | `crates/transport/rrd-contract/tests/endpoint_proposal.rs` (87 lines; differs 87/87, not compared line by line — likely line endings; `UNCLEAR`) |
| Import reordering in 12 `rrd-engram` files, `engine/mod.rs`, `rrd-client/src/operation.rs`, `tests/public_contract.rs` | rename side-effect | rustfmt re-sorts `Error`/`Engram…` and `Ensure…`/`Engram…`; expect the same churn |
| Golden digest refresh (`install-attunement-v2.json` 16 digests, `public-contract-v1.json` 1) | rename consequence, must be regenerated on main, not copied | values are for an 8-day-old contract |
| Everything under gate-e / gate-g journals, POAM-050..052, `scripts/inference/installed_task_resume_proof.py`, `reasoning_budget/resume.rs`, `identifier_grammar.rs`, `versioned_read_batch_tests.rs` | `ON MAIN` | came from the merge of `5f8a7e367`; e.g. `docs/evidence/change-journals/gate-g/task-budget-resume.md`, `gate-h/h-04-kernel-identifier-grammar.md` are tracked on main |
| `connectome` gitlink → `fe97f5431` | `SUPERSEDED` | main pins `a7af4e0cf` |
| `docs/reference/client/connectome.md` Workbench paragraph | `SUPERSEDED` (not found on main; Connectome has moved on; not read further) | |

---

## 3. Why it was a "diagnosis" and never finished

From its own journal (quotes are from the ref's `engram-connectome-repair-2026-11-07.md`):

- "The requested target is a buildable engram identity … The current work does not establish that target".
- "Cargo metadata initially failed because the renamed authority crate path was absent" — the blanket commit changed references but moved no files.
- "The complete wide rename, all merged paths and installed-format consequences have not been reviewed."
- "The draft does not yet meet the change checklist: full changed-file reread and diff reconciliation, full owning suites, strict lint, generated surfaces and documentation, installed migration/reopen, cross-platform candidate CI, and merge-group evidence remain unrun or incomplete."
- "The rename must not be merged as a safe upgrade on this evidence." (after the reopen probe)
- The OpenAPI digest assertion was left failing on purpose: "The test is intentionally not bypassed." Confirmed: the ref's `OPENAPI_DOCUMENT_SHA256` (`rrd-contract/src/lib.rs:274`) is still the pre-rename `e6bd6b60…`.

What reached: `cargo check --workspace --all-targets` and four-package strict clippy passed on Windows; `rrd-contract` tests passed except the OpenAPI digest; full engine/server/CLI suites were not run.

Left inconsistent on the ref (verified by grep on the ref):

| inconsistency | evidence |
|---|---|
| Files not renamed although their contents were | `crates/adapters/rrflow-cli/src/bin/rrd-estate-controller.rs` (prints `"rrd-engram-controller: {error}"` at :26), `crates/adapters/rrflow-cli/tests/estate_admin.rs`, `crates/authority/rrd-engram/tests/estate_authority.rs`, `crates/kernel/rrd-core/fixtures/estate-organisation-v1.json`, `crates/kernel/rrd-core/tests/estate_organisation_golden.rs`, `crates/transport/rrd-server/tests/local_estate_driver.rs` |
| CI names targets that do not exist | workflow calls `--bin rrd-engram-controller` and `--test local_engram_driver` (`ci-reusable.yml:263,265,419`) but the files above keep the estate names and `rrflow-cli/Cargo.toml` declares no explicit bins — PR #290's journal records exactly this CI failure |
| Generated SDK files hand-substituted, not regenerated | `sdks/go/endpoints_gen.go:41` `OperationengramRead`; `sdks/typescript/src/generated/rrd-openapi.ts` has 171 "engram" and 2 "estate" lines; `check_generated_surfaces.py --check` "failed as expected" |
| Miscased names in docs and comments | 70+ hits such as `engramConfiguration` (`README.md:339`), `InstalledengramIdentity` (`docs/architecture/instance-topology.md:261`), `engramLayout` (`engine/tests/installation_bootstrap.rs:41,173`) |
| Third-party and unrelated words damaged | `tracengram` for W3C `tracestate` in `docs/architecture/engine-data-flow.md:661-784` and all six `docs/reference/sdk/*.md`; `rengramd` for "restated" in `crates/compute/rrd-query/tests/graph_traversal.rs:321` |
| Doc files not renamed | `docs/reference/operations/estate-control.md`, `docs/reference/data/estate-organisation.md`, `docs/reference/security/local-estate-authorization.md`, `docs/decisions/0003-configurable-estate-organisation.md`, `docs/research/rrflow-canonical-estate-configuration-and-runtime-controls.md` |
| Frozen fixture edited | `crates/kernel/rrd-core/fixtures/schema-registry-default-digest-v1.json` (see 4.3) |
| Connectome | pinned UI's native SDK "is still pinned to the older estate wire contract" |

---

## 4. Compatibility hazards

`docs/roadmap/kernel-contract.md:333`: "Pre-release installed v2 plans and jobs | reinstall for pre-release; no migration lane". `:327`: "When the estate-to-engram rename happens | inside K13, after K4". Both are listed under "Owner decisions still open" as defaults. The owner's 2026-11-14 instruction to finish now supersedes the timing default; whether it also confirms "reinstall" for this rename should be stated in the PR brief, because the reopen probe in 2.7 shows an installed project stops opening.

`#[serde(deny_unknown_fields)]`: 1052 occurrences in 216 files under `crates/`. Any renamed field is a hard decode failure for old bytes, not a silent default.

### 4.1 Removed by the reinstall default (persisted inside one installed project)

| hazard | defined on main |
|---|---|
| Locator and installed identity field `estate_id` | `crates/transport/rrd-contract/src/deployment/profile.rs:16-19` (`InstalledEstateIdentity`, `deny_unknown_fields`) |
| Generated id prefix `estate-…` | `crates/authority/rrd-engine/src/engine/installation/planning.rs:113`; `installation/validation.rs:98` |
| Identity seed domain `"rrflow-installed-estate-v3"` | `crates/authority/rrd-engine/src/engine/installation/validation.rs:14` |
| Configuration digest domain `rrflow-estate-configuration-v3\0` | `crates/transport/rrd-contract/src/deployment/configuration.rs:230` |
| Organisation digest domain `rrflow-estate-organisation-v1\0` | `crates/kernel/rrd-core/src/runtime/encoding.rs:346` |
| Storage keys `server/state/estate/{id}/document` | `crates/authority/rrd-estate/src/repository.rs:7` |
| Storage keys `server/state/estate-recovery/{id}/{kind}/{identity}` | `crates/authority/rrd-engine/src/engine/estate_control/local_recovery.rs:115` |
| Backup scope prefix `server/state/estate` and its prefix-match rule | `crates/authority/rrd-engine/src/engine/backup.rs:338`; `crates/persistence/rrd-store/src/archive/control.rs:164-165,532-537` — **existing backups made before the rename will not contain the new prefix and vice versa** |
| Authority-journal action names `estate.*` (23 strings) | `crates/authority/rrd-estate/src/repository.rs:30,50` and siblings (Appendix A) |
| Idempotency/operation digests `estate-reconcile-v1`, `estate-backup-reconcile-v1`, `estate-recovery-*-v1`, `rrd-estate-backup-create-v1`, `rrd-local-estate-backup-v1` | `engine/estate_control/admin.rs:238,295`; `recovery.rs:45,67,282,306,472`; `drivers.rs:207`; `rrd-estate/src/backup_job/validation.rs:303` |
| Recovery state `format_version` | `crates/authority/rrd-engine/src/engine/estate_control.rs:28` |
| Document fields `estate_id`, `estate_revision`, `expected_estate_revision`, `based_on_estate_revision`, `estates` | `crates/authority/rrd-estate/src/{backup_reconcile.rs:14, reconcile.rs:14, recovery/model.rs:134,187,210, local_authorization.rs:32,38}` |
| Local authorization file (`estates` map, `LocalEstatePermission`) | `crates/authority/rrd-estate/src/local_authorization.rs:14,32` — an operator-written file; reinstall does not regenerate it. Check how it is produced before assuming it is covered |
| Security actions `estate_read` / `estate_admin` in stored grants and audit records | `crates/transport/rrd-contract/src/audit.rs:47-48` (`rename_all = "snake_case"`) |
| Resource kind `"estate"` in stored resource paths and audit | `crates/transport/rrd-contract/src/identity.rs:77`; `platform.rs:47-49` |
| Install plan variants and fields `create_estate_directories`, `estate_directory`, `adopted_estate_entries` | `crates/transport/rrd-contract/src/attunement/installation.rs:52,96,191` |
| Recovery step `"estate_directories_ready"` | `crates/authority/rrd-engine/src/engine/installation/recovery.rs:32` |
| Profile asset name and the profile digest it feeds | `crates/authority/rrd-engine/assets/install/{default,clyffy}-profile-v3.json:4`; checked at `installation/profile.rs:261`; embedded at `installation.rs:61` |
| Exclusion rule id `"estate-state"` (pattern `.rrflow`) | same two profile JSONs, line 93 — changes `profile_sha256` |
| Schema migration strings | `crates/authority/rrd-engine/src/engine/memory_engram.rs:123,214` |

All of these change digests that are sealed into an install (`plan_sha256`, `configuration_sha256`, `profile_sha256`, `record_sha256`). Under "reinstall" that is acceptable; it also means **every golden that embeds one must be regenerated by its owning generator** (4.3).

### 4.2 Not removed by reinstall — on the wire, across repositories

| hazard | defined on main | who breaks |
|---|---|---|
| Operation id `estate-read` | `crates/transport/rrd-contract/src/catalogue/lifecycle.rs:111`; `operation_reference.rs:274`; `rrd-server/src/http/router.rs:82` | five SDKs, Connectome, any script calling by id |
| HTTP path `/v1/estates/{estate}/read` and path parameter `estate` | `catalogue/lifecycle.rs:112`; `rrd-client/src/operation/operations.rs:67`; `rrd-server/src/http/capabilities.rs:31`; negative tests `router.rs:501-502` | same |
| Response/request schema names (`EstateSnapshot`, `ReadEstate`, 47 more) | `crates/transport/rrd-contract/src/estate_authority.rs:9-505` | OpenAPI component names → generated TypeScript types |
| Handshake field `installed_estate` | `crates/transport/rrd-contract/src/conformance.rs:152` | Connectome reads `handshake.service.installed_estate.estate_id` (`connectome/src/features/runtime/instance/instance.tsx:120` at the pinned revision) |
| Diagnostic snapshot field `estate` and section name | `crates/transport/rrd-contract/src/diagnostic/snapshot.rs:104`; `crates/authority/rrd-engine/src/engine/diagnostic/sections.rs:254` | diagnostics consumers |
| Capability `estate-authority-read` | `crates/transport/rrd-server/src/http/capabilities.rs:206` | capability negotiation |
| Security action names in the OpenAPI enum | `audit.rs:47-48` | SDK types; the journal's "289 changed leaves" |
| OpenAPI digest | `crates/transport/rrd-contract/src/protocol.rs:5-6` (`0e688c6b…`); asserted at `tests/public_contract/catalogue.rs:197` | must be recomputed; six hand-cited copies are checked by `scripts/ci/check_generated_surfaces.py` (`DIGEST_CITATIONS`: `docs/reference/protocol/public-contract.md` and five `docs/reference/sdk/*.md`) |
| Generated SDK files | `sdks/typescript/src/generated/{rrd-openapi.ts,endpoints.ts}`, `sdks/python/src/rrd_client/generated/endpoints.py`, `sdks/go/endpoints_gen.go`, `sdks/java/…/OperationId.java`, `sdks/dotnet/…/Generated/OperationId.g.cs` | regenerate, never hand-edit (the ref's `OperationengramRead` shows why) |
| Hand-written SDK resource segment `"estate"` | `sdks/typescript/src/operation.ts:31,92-93`; `sdks/python/src/rrd_client/operation.py:18,74-75`; `sdks/go/operation.go:21,197-202`; `sdks/java/…/OperationBinding.java:18,102-104`; `sdks/dotnet/…/OperationBinding.cs:12,125-129` | not covered by the generator |
| SDK conformance corpus | `fixtures/rrd-sdk-conformance-v1.json:11,23,320,328`; model at `crates/transport/rrd-contract/src/sdk_conformance.rs:21,53,61,145`; server at `crates/transport/rrd-client/examples/sdk_conformance_server.rs:241`; five runners under `sdks/*/tests` | regenerate the corpus from its owner, then update the runners |
| CLI flag `--estate`, default instance names, binary names `rrd-estate-admin`, `rrd-estate-controller` | `crates/adapters/rrflow-cli/src/bin/*.rs` (lines in section 1.2) | CI (`ci-reusable.yml:330,506`), `scripts/ci/check_workflow.py:713,1414`, docs, operators |
| CI job id `estate-portability` | `.github/workflows/ci-reusable.yml:217,1124,1149,1179`; `scripts/ci/check_workflow.py:52,62,812,1217,1234,1303,1312,1398-1404` | the checker asserts the literal name. The required check is `ci-gate` (`:1114`) and the job's display name is already "engram process", so branch protection should be unaffected — **not verified against repository settings** |
| Connectome | pins `rrd-client`/`rrd-contract` by git revision `b099b5bc` (`connectome/src-tauri/Cargo.toml:47-48`); 87 files mention estate, e.g. `src-tauri/src/rrd/client/engram.rs:23` `installed.estate_id`, `src/rrflow/session.ts:44` | needs its own PR in `rrflow/connectome-development` after the engine lands |

### 4.3 Not removed by reinstall — frozen bytes inside this repository

| hazard | where | what to do |
|---|---|---|
| Captured provider observations with `"estate_id": "estate-2b47…"` whose file and canonical digests are recorded in `files.json`/`receipt.json` | `crates/transport/rrd-contract/tests/fixtures/endpoint-selection-observed/{available,withheld}/configuration.json:5`; `crates/compute/rrd-inference/tests/fixtures/endpoint-chat-native/configuration.json:5` | Do not edit. Use the projection in item 2.6 for all nine readers, or re-capture (needs a live provider run and the owner's approval) |
| "Never regenerated" golden | `crates/kernel/rrd-core/fixtures/schema-registry-default-digest-v1.json:2,7` (`"migration": "memory estate bootstrap"`), read by `crates/kernel/rrd-core/tests/schema_registry_default_digest.rs:90,152` | Leave both the fixture and the test's input string as they are. The ref changed the string in both without touching `commit_digest`; whether that test passed on the ref was **not determined** |
| Example URL | `crates/transport/rrd-contract/tests/fixtures/endpoint-profile-v1.json:289` `https://warden.estate.example` | Leave; the ref had to restore it byte-for-byte |
| Goldens that embed sealed digests | `crates/transport/rrd-contract/fixtures/install-attunement-v2.json` (5 estate lines, 16 dependent digests); `public-contract-v1.json` (3 lines, 1 digest); `crates/kernel/rrd-core/fixtures/estate-organisation-v1.json` | Regenerate through the owning test's opt-in generator; review that only names and digests moved |
| Storage qualification goldens and schemas (`estates`, `estate_process`, `requires_estate_interference`) | `crates/evaluation/qualification/rrflow-qualification-storage/tests/fixtures/storage/{manifest,report}.json`; `eval/schemas/storage-*.json`; `eval/capacity-profiles-v1.json` | Regenerate with `RRFLOW_UPDATE_STORAGE_GOLDENS=1` (named in PR #290's journal). Stored result files that conform to the old schema become unreadable — decide whether `eval/results/**` is history |
| Historical evidence | `docs/evidence/**` (285 files), `docs/roadmap/rrflow-1.0-active-change.json` ("stays unchanged as historical evidence", `AGENTS.md`), `docs/history/**` | Never rewrite. The blanket commit hit 245 of them and the ref spent a step restoring 244 |
| Recorded user prompts and results | `eval/inputs/staging/*.json`, `eval/results/real-project-workbench-20261108-101226/*.json` | Leave |

### 4.4 Words that contain "estate" and are not the product term

A case-insensitive match is wrong. `git grep -il estate` returns 824 files; the real term (`(?<![Rr])estate|Estate|ESTATE`, case-sensitive) is in 798. The difference and the remaining traps:

| token | count on main | meaning |
|---|---|---|
| `FileState` 23, `WebSocketReceiveState` 17, `PlacementNodeState` 9, `RrflowCldLifecycleState` 7, `useState` 6, `StoreState` 5, `KnowledgeState` 3 | — | `…e` + `State`; must not change |
| `EstateState` 3 | — | a real estate type; becomes `EngramState` |
| `restate`, `restated`, `restates`, `restatements` | 90 / 72 / 7 / 662 | English; mostly `eval/inputs/staging` |
| `tracestate` | 21, in 10 doc files | W3C Trace Context header |
| `warden.estate.example` | 1 | fixture URL |
| "one real estate" | `docs/roadmap/rrflow-1.0.md:130`, `rrflow-1.0/gate-d.md:1` | is the product term, but the heading anchor `#gate-d--install-configure-and-attune-one-real-estate` is linked from `docs/reference/automation/project-command-capabilities.md:19` and `README.md:247-251` |

---

## 5. Inventory of what must be renamed on today's main

### 5.1 Files (`git grep -il estate development/main`, then the real-term count)

| area | `-i` count | real term | notes |
|---|---|---|---|
| `crates/` | 345 | 327 | breakdown below |
| `docs/` | 426 | 420 | 285 are `docs/evidence` — do not touch |
| `eval/` | 20 | 18 | schemas and README rename; `inputs/staging` and `results` stay |
| `sdks/` | 17 | 17 | 6 generated, 5 bindings, 6 tests |
| `scripts/` | 10 | 10 | `ci/{build_execution_inventory,check_workflow,run_sdk_conformance,run_sustained_capacity,test_rrflowdb_standalone}.py`, `ci/code_shape_baseline.json`, `inference/{controller_trial_README.txt,installed_controller_proof.py}`, `knowledge/{render,test}_capacity_profiles.py` |
| root | 5 | 5 | `Cargo.toml:6`, `Cargo.lock`, `README.md`, `SECURITY.md`, `.github/workflows/ci-reusable.yml` |
| `fixtures/` | 1 | 1 | `rrd-sdk-conformance-v1.json` |
| **total** | **824** | **798** | 4938 matching lines |

Crates (real term): `authority/rrd-engine` 128 · `transport/rrd-contract` 51 · `adapters/rrflow-cli` 31 · `authority/rrd-estate` 30 · `evaluation/qualification` 17 · `transport/rrd-server` 14 · `transport/rrd-client` 12 · `persistence/rrd-lsm` 11 · `persistence/rrd-store` 10 · `kernel/rrd-core` 10 · `compute/rrd-query` 6 · `compute/rrd-inference` 2 · `compute/rrd-attunement` 2 · `evaluation/lab` 1 · `compute/rrd-vector` 1 · `authority/rrd-security` 1.

Docs outside evidence (135 files): `poam` 36 · `reference` 48 · `roadmap` 23 · `architecture` 11 · `research` 9 · `decisions` 5 · `operations`, `objectives`, `history` 1 each.

### 5.2 Paths with "estate" in the name (57 code paths, 13 doc paths)

Code: the whole `crates/authority/rrd-estate/` (30 files); `rrd-engine/src/engine/{estate.rs, estate_control.rs, estate_control/{admin,drivers,local_recovery,recovery}.rs, estate_layout.rs, memory_estate.rs, tests/{estate_control,estate_organisation,memory_estate}.rs}`; `rrd-engine/assets/install/default-estate-configuration-v3.toml`; `rrflow-cli/src/bin/{rrd-estate-admin,rrd-estate-controller}.rs`; `rrflow-cli/tests/{estate_admin.rs, support/sustained/estate.rs}`; `rrd-core/{fixtures/estate-organisation-v1.json, tests/estate_organisation_golden.rs}`; `rrd-contract/{src/estate_authority.rs, src/memory_estate.rs, tests/public_contract/estate_and_query.rs}`; `rrd-server/tests/local_estate_driver.rs`.

Docs to rename: `docs/decisions/0003-configurable-estate-organisation.md`, `docs/reference/data/estate-organisation.md`, `docs/reference/operations/estate-control.md`, `docs/reference/security/local-estate-authorization.md`, `docs/research/rrflow-canonical-estate-configuration-and-runtime-controls.md`. They are linked from 2–4 evidence journals each (`git grep -l` counts: estate-control 4, local-estate-authorization 3, estate-organisation 3, ADR-0003 2, research 2) and from `scripts/ci/build_execution_inventory.py:1690,1712`. Renaming them forces a link-only edit in historical journals, or the link checker fails. **Owner/engineer decision:** rename and fix links (a link is navigation, not evidence), or keep the five file names.

Docs that must keep their names (historical): the nine `docs/evidence/change-journals/**/*estate*.md` journals.

### 5.3 Half-rename residue on main to remove in the same change (POAM-078)

| residue | where |
|---|---|
| `pub struct InstalledengramRecord` + `pub type InstalledEstateRecord = InstalledengramRecord` | `crates/authority/rrd-engine/src/engine/installation/records.rs:105,118`; 15 uses across `attunement.rs:13`, `attunement/replan.rs:98-99`, `installation/{apply.rs:441, open.rs:52,225, storage.rs:176,196,298,357}` |
| `pub type PersistMemoryengram = crate::PersistMemoryEstate`, `MemoryengramPlan` | `crates/transport/rrd-contract/src/memory_engram.rs:7,10`; exported `lib.rs:199` |
| `pub type DataengramOrganisation = DataEstateOrganisation` | `crates/transport/rrd-contract/src/data.rs:250` |
| Two planner modules | `crates/authority/rrd-engine/src/engine/memory_engram.rs` and `memory_estate.rs` (`mod.rs:131-132`); `memory_estate.rs:216-220` delegates; two test files `engine/tests/memory_engram.rs`, `memory_estate.rs` |
| Two loaders | `load_engram_configuration` (`installation/profile.rs:336`) and `load_estate_configuration` (`:369`, "Retained source-level name") |
| `ResourceKind::Estate` and unused `ResourceKind::Engram` | `crates/transport/rrd-contract/src/identity.rs:77-78`; `Estate` has 7 uses, `Engram` none — **`Engram` is already in the OpenAPI enum**, so removing `Estate` is the only wire change here |
| `engram_configuration_key` | `installation/storage.rs:338` (already correct; key string has no "estate") |
| Mixed test/CI names | `crates/adapters/rrflow-cli/tests/installed_lifecycle/engrams_and_history.rs:9` and `scripts/ci/selected_tests.py:28,31` already say engram; `installed_two_estate_isolation_under_concurrent_load` does not |

### 5.4 Public items — see Appendix B (112 `pub`/`pub(…)` items; 89 fully `pub`; 70 public types = 49 structs + 16 enums + 3 type aliases + 2 traits)

### 5.5 All distinct identifiers and string literals — see Appendix A (203 identifiers, and the short string literals)

---

## 6. Recommended order of work (one change on main)

The ref shows what not to do: substitute text first and repair afterwards. Drive the rename from the type system outward and let the compiler and the owning generators produce everything derived.

**Step 0 — brief and decisions (no code).** PR description per `AGENTS.md`: owner K13 / R10 / POAM-078; state "reinstall, no migration lane" as the compatibility rule; list the frozen bytes of 4.3 and the false positives of 4.4 as exclusions; decide (a) the five doc file names (5.2), (b) whether digest domain separators and id prefix are renamed (recommended: yes, since every sealed digest changes anyway and the owner said "never estate"), (c) plural route `/v1/engrams/{engram}/read`. Record a journal under `docs/evidence/change-journals/`.

**Step 1 — characterise before touching names.** Capture on main: the OpenAPI export and its digest; `python scripts/ci/check_generated_surfaces.py --check` passing; the list of test names (`cargo test -- --list` per package) so the "no test lost" claim is checkable. Per the owner's rule (commit the failing test, then fix), this is also where the tests for items 2.3 and 2.4 go if those are fixed first — **do them as a separate change before the rename**, so the rename diff stays mechanical.

**Step 2 — remove the residue while names are still "estate"** (compiles, no wire change): delete the three miscased aliases and the two duplicate modules/tests/loaders by pointing callers at one name; keep every test (merge the two memory test files, do not drop either). Leave `ResourceKind::Engram` for step 4.

**Step 3 — crate and module moves, contents unchanged except paths** (`git mv`, so history follows): `crates/authority/rrd-estate` → `rrd-engram`; update root `Cargo.toml:6`, the four dependents (`rrflow-cli/Cargo.toml:36`, `rrd-engine/Cargo.toml:31,52`, `rrd-client/Cargo.toml:28`, `rrd-server/Cargo.toml:28`), `use rrd_estate` (111 sites) and `Cargo.lock` (let cargo rewrite it). Move the engine modules, the `estate_control/` folder, contract modules, test files, the two bin sources, `estate.rs` support files, the core fixture and the asset (update `installation.rs:61`, `profile.rs:261`, both profile JSONs line 4). Check each step with `cargo metadata --locked` then `cargo check --workspace --all-targets`. Update `scripts/ci/build_execution_inventory.py` path keys (`:75,393,404,534,634,635,666,673,698`) in the same step — it maps paths to gates.

**Step 4 — Rust identifiers, by the compiler, crate by crate in dependency order:** `rrd-core` → `rrd-contract` → `rrd-store`/`rrd-lsm` → `rrd-engram` → `rrd-engine` → `rrd-client` → `rrd-server` → `rrflow-cli` → evaluation crates. Use a word-boundary, case-preserving rename (`Estate`→`Engram`, `estate`→`engram`, `ESTATE`→`ENGRAM`) restricted to identifier tokens from Appendix A, never a raw substring replace. Collapse `ResourceKind` to `Engram` only. After each crate: `cargo check`, then `cargo fmt` (expect import re-sorting as in section 2), then strict clippy — the ref's journal notes case errors surface as `non_camel_case_types` warnings that candidate CI treats as errors.

**Step 5 — serialized names and literals, deliberately and separately** (so a reviewer sees every wire/persisted change in one commit): serde field names follow the identifiers automatically; review the list in 4.1/4.2 and the literals in Appendix A by hand; update `operation_reference.rs` descriptions (twelve needed it on the ref); rename CLI flags and usage strings; rename env-var test hooks in both the bin and its test together.

**Step 6 — frozen fixtures.** Add the historical projection (item 2.6) and route all nine fixture readers through it. Leave `schema-registry-default-digest-v1.json`, `endpoint-profile-v1.json:289` and the three capture files byte-identical; verify with `git diff --stat` that they are untouched.

**Step 7 — regenerate goldens through their owners,** then review each structured diff contains only renamed names and digests: `install-attunement-v2.json` and `public-contract-v1.json` (the owning test's opt-in generator; the ref's journal names `install_and_attunement_contract_matches_golden_json`), `engram-organisation-v1.json`, the SDK conformance corpus, storage goldens and `eval/schemas/storage-*.json` (`RRFLOW_UPDATE_STORAGE_GOLDENS=1`), `eval/capacity-profiles-v1.json` via `scripts/knowledge/render_capacity_profiles.py`. The exact generator invocations were not read here; take them from each test.

**Step 8 — OpenAPI and SDK projections.** Export the OpenAPI document, diff it structurally against the step-1 capture (expect the same shape of change the ref measured: resource kinds, actions, identity fields, paths, schema names — and nothing else), set `OPENAPI_DOCUMENT_SHA256` (`protocol.rs:5-6`), run `python scripts/ci/check_generated_surfaces.py` in render mode to rewrite the six endpoint outputs and signal outputs, update the six hand-cited digests it checks, then `--check`. Hand-edit the five SDK `OperationBinding`/`operation` files and five conformance runners (they are not generated). Run `scripts/ci/run_sdk_conformance.py`.

**Step 9 — CI and selectors.** `.github/workflows/ci-reusable.yml` (package, bin, test and job names at the lines in 4.2) together with `scripts/ci/check_workflow.py` (it asserts the literal strings); `scripts/ci/selected_tests.py` — already engram at :28,31; re-derive every case name from the renamed tests and confirm each selected name still resolves to exactly one test (a renamed test that no selector matches is silently not run); `scripts/ci/code_shape_baseline.json` (regenerate, it records paths); `run_sustained_capacity.py`, `test_rrflowdb_standalone.py`. Run `python scripts/ci/check_workflow.py` and `python scripts/check_version.py`.

**Step 10 — documentation.** Rename in `README.md`, `SECURITY.md`, `docs/{reference,architecture,decisions,poam,roadmap,research,operations,objectives}` with the 4.4 exclusions (`tracestate`, "restate…"). Exclude `docs/evidence/**`, `docs/history/**`, `docs/roadmap/rrflow-1.0-active-change.json`, `eval/inputs/**`, `eval/results/**`. Fix heading anchors and their inbound links. Close POAM-078 only with the evidence its owning record names; update `kernel-contract.md:327` and `code-structure-and-conventions.md` R10. Run `python3 scripts/knowledge/render_navigation.py` and `scripts/ci/check_documentation.py`.

**Step 11 — vocabulary ratchet.** Add the check R10 asks for ("a vocabulary check like the existing retired-name bans", `docs/architecture/code-structure-and-conventions.md:38`): fail on `(?<![Rr])estate|Estate|ESTATE` outside an allow-list (evidence, history, `tracestate`, the frozen fixtures, staging prompts) and on `[a-z]engram[A-Z]|[A-Z][a-z]+engram\b`. Without it the word returns.

**Step 12 — prove it on the real path** (owner rule: green is not proof): fresh install → serve → `engram-read` over HTTP → stop → reopen → backup → restore, on a scratch project; then confirm an old install is refused with a clear message rather than a serde error (the reinstall rule made visible). Full owning suites: `rrd-contract`, `rrd-engram`, `rrd-engine`, `rrd-server`, `rrflow-cli` including the ignored fleet case.

**Step 13 — Connectome, after the engine commit is on main.** In `rrflow/connectome-development`: bump `rrd-client`/`rrd-contract` `rev` (`src-tauri/Cargo.toml:47-48`) to the renamed revision, rename `estate_id`/`installed_estate`/`estate-read` uses (87 files, about half are sprint notes and visual-test text), land through its own CI. Then repin the `connectome` gitlink in RRFlow (currently `a7af4e0cf`) and update `docs/reference/client/connectome.md`. Until then the pinned UI builds against the old revision and cannot talk to a renamed engine; say so in the engine PR.

What a reviewer can check at each step: 2, 3, 4 compile and pass with **no** OpenAPI digest change (2, 3) or with a digest change explained entirely by step 5's list (4+5 together, if serde names are derived). If steps 4 and 5 cannot be separated because field names are identifiers, land them as one commit and attach the structured OpenAPI diff.

---

## 7. Risk if ported

- **Applying the ref's tree or diff:** not possible. It predates the 11-13/14 module splits (its `rrd-contract/src/lib.rs` is still ~5600 lines; main has `identity.rs`, `protocol.rs`, `catalogue/`), and it would revert a week of main.
- **Rename itself:** every installed project and every existing backup stops opening (4.1); every external client breaks until regenerated (4.2); Connectome is out of step until step 13.
- **Silent test loss:** renamed test functions that CI selects by exact name (`selected_tests.py`, `--exact` in the workflow at `:776`).
- **Substring damage:** section 4.4.
- **Items 2.2–2.5:** they change behaviour on the selection/worker path that main has since reworked (LFG advisory at `endpoint_proposal.rs:386-396`, source capture in `endpoint_worker/execute.rs`). Port by intent with new tests, not by patch. Must test: narrowing with LFG enabled and disabled; `adapter: other` refused before any network call; observation export with and without routing.

## Verdict

**PORT** — by hand, as separate changes in this order:

1. Items 2.3 and 2.4 (probable defects; failing tests first).
2. Items 2.2 and 2.5 (documented behaviour that is absent), and 2.1 (project map wiring, test, doc section).
3. The rename on main per section 6, reusing item 2.6 and the measurements in 2.7.

Keep the ref's journal `gate-h/engram-connectome-repair-2026-11-07.md` as a historical record (it is not on main). Drop the rest of the tree.

**Owner decisions needed:** (1) confirm reinstall-only for this rename, including existing backups; (2) the five doc file names in 5.2; (3) whether `eval/results/**` is history.

## Cross-cutting notes

**Main defects suggested by this review**

1. `refresh_project_map` has no caller and no test (`project_map.rs:36`); the owner's 2026-10-31 import behaviour ("auto-create/evolve a project map") is not running.
2. Routing input is passed through without narrowing (`endpoint_proposal.rs:187-191` vs contract `endpoint_proposal.rs:376-388`).
3. Unimplemented worker adapters are not refused (`is_native_chat_adapter` uncalled); `lfg-swag.md:259` says they are.
4. `lfg-swag.md:88` documents `selector_policy_id`, `selector_routing_sha256`, `selector_narrowed_candidate_count`; no code has them.
5. Flight identity on a selection request is stored but never verified.
6. The commit title of `bf6dec69f` misdescribes a 637-file change; anything reasoning from `git log` will conclude the TypeScript SDK is renamed. It is not.
7. Merge `4c2f2a6a1` resolved conflicts by taking main's side of whole files in at least five places. Other candidate-side work in that merge may have been lost the same way; only the identifiers in section 2 were checked.

**Duplicated work:** the ref and PR #290 are two repairs of the same commit with opposite outcomes (ref: commit to engram; #290: restore estate). #290's journal already contains the 18-file `rrd-engram` ↔ `rrd-estate` pairing table, which is the crate-move map for step 3.

**Not read / not verified:** full prose of the ~135 non-evidence docs; the generators' exact command lines; `prepare_controller` and the non-LFG `validate_for` path; branch-protection settings; whether `schema_registry_default_digest` passed on the ref; line-by-line equality of `tests/endpoint_proposal.rs`.


---

## Appendix A — every distinct estate identifier and literal on main

Source: `git grep -ohP "[A-Za-z0-9_]*(?:(?<![Rr])estate|Estate|ESTATE)[A-Za-z0-9_]*" development/main -- crates sdks scripts eval fixtures .github Cargo.toml` (203 tokens; number = occurrences). Includes prose words in comments and in `eval/inputs/staging`.


Bare words: `estate` 1728, `estates` 148, `Estate` 72, `_estate` 1.


**Types and variants (PascalCase)** (89): `EstateAdmin` 90 · `EstateRepository` 77 · `EstateConfiguration` 68 · `InstalledEstateIdentity` 65 · `EstateRead` 60 · `EstateDocument` 59 · `DataEstateOrganisation` 53 · `EstateFilesystem` 52 · `EstateLayout` 33 · `LocalEstatePermission` 29 · `EstateAdminAction` 28 · `EstateInUse` 19 · `EstateSnapshot` 18 · `EstateDesiredPhase` 16 · `ReadEstate` 15 · `EstateBackupJob` 15 · `EstateAuthorityResourceKind` 15 · `RuntimeEstateOrganisation` 14 · `EstateOperation` 14 · `EstateConfigurationInput` 14 · `EstateAdminResult` 13 · `EstateObservedPhase` 12 · `EstateDriver` 12 · `PersistMemoryEstate` 11 · `EstateRetentionPinKind` 10 · `EstateBackupDriver` 10 · `InstalledEstateRecord` 9 · `EstateOperationState` 9 · `EstateBackupJobState` 9 · `EstateAuthorityReceiptBoundary` 9 · `EstateRecoverySnapshot` 8 · `EstateRecoveryPolicy` 8 · `EstateOperationKind` 8 · `EstateDirectoriesReady` 8 · `EstateBackupJobsSnapshot` 8 · `EstateAuthorityStatus` 8 · `MemoryEstatePlan` 7 · `EstateRetentionPin` 7 · `EstateMutationResult` 7 · `EstateBackupJobSnapshot` 7 · `EstateAuthoritySnapshot` 7 · `EstateActivityClass` 7 · `EstateReceiptBoundary` 6 · `EstatePreview` 6 · `EstateAuthorityState` 6 · `InstalledEstate` 5 · `EstateRetentionDecisionSnapshot` 5 · `EstateRetentionDecision` 5 · `EstateRestoreEvidence` 5 · `EstateRecoveryPruneIntent` 5 · `EstateRecoveryPoint` 5 · `EstateRecoveryMutationResult` 5 · `EstateLeaseSnapshot` 5 · `EstateDirectory` 5 · `EstateBackupRecoveryPolicySnapshot` 5 · `EstateBackupReceiptBoundary` 5 · `EstateActivityPolicySnapshot` 5 · `OpenPublishedEstate` 4 · `EstateRetentionPinKindSnapshot` 4 · `EstateRecoveryRestoreResult` 4 · `EstateRecoveryPruneResult` 4 · `EstateReconcileOutcome` 4 · `EstateBackupReconcileOutcome` 4 · `CreateEstateDirectories` 4 · `OperationEstateRead` 3 · `EstateState` 3 · `EstateServeRequest` 3 · `EstateSelection` 3 · `EstateRetentionPinSnapshot` 3 · `EstateRestoreEvidenceSnapshot` 3 · `EstateRecoveryRestoreOperationState` 3 · `EstateRecoveryPruneOperationState` 3 · `EstateRecoveryPruneIntentSnapshot` 3 · `EstateRecoveryPolicySnapshot` 3 · `EstateRecoveryPointSnapshot` 3 · `EstateReceiptSnapshot` 3 · `EstateOperationSnapshot` 3 · `EstateObservedSnapshot` 3 · `EstateInstanceSnapshot` 3 · `EstateDesiredSnapshot` 3 · `EstateBackupReceiptSnapshot` 3 · `EstateBackupMutationResult` 3 · `EstateAuthorityResourceSnapshot` 3 · `EstateAuthorityReceiptSnapshot` 3 · `EstateAuthorityObservedSnapshot` 3 · `EstateAuthorityDesiredSnapshot` 3 · `EstateActivitySnapshot` 3 · `EstateProcess` 2 · `MemoryEstate` 1


**Constants and env vars (SCREAMING)** (12): `ESTATE_DIRECTORY` 15 · `PRIVATE_ESTATE_DIRECTORIES` 10 · `ADOPTED_ESTATE_DIRECTORY` 10 · `MAX_ADOPTED_ESTATE_ENTRIES` 7 · `ESTATE_CONFIGURATION_FORMAT_VERSION` 7 · `ESTATE_RECOVERY_OPERATION_FORMAT` 5 · `ESTATE_AUTHORITY_FORMAT` 4 · `ESTATE_FORMAT` 3 · `RRD_ESTATE_TEST_HOLD_AFTER_STEP_FILE` 2 · `RRD_ESTATE_TEST_HOLD_AFTER_EFFECT_FILE` 2 · `MAX_ESTATE_GRANTS` 2 · `ESTATE_READ` 2


**Functions, fields, modules, tests (snake_case)** (98): `estate_id` 147 · `rrd_estate` 111 · `estate_configuration` 88 · `estate_layout` 30 · `installed_estate` 26 · `estate_revision` 26 · `memory_estate` 19 · `adopted_estate_entries` 19 · `installed_estate_identity` 15 · `estate_control` 13 · `estate_configuration_sha256` 13 · `validate_recovery_estate` 12 · `seed_served_estate_for_visual_capture` 12 · `estate_context` 12 · `expected_estate_revision` 11 · `estate_read` 10 · `preview_estate` 9 · `estate_admin` 9 · `restore_estate_backup_store` 8 · `requires_estate_interference` 8 · `load_estate_configuration` 8 · `estate_open_elsewhere` 8 · `estate_process` 7 · `estate_journal` 7 · `validate_adopted_estate_entries` 6 · `reject_existing_estate` 6 · `read_estate` 6 · `existing_estate_entries` 6 · `estate_directory_error` 6 · `estate_audit_resource` 6 · `create_estate_directories` 6 · `based_on_estate_revision` 6 · `a_served_estate_is_found_proved_and_not_served_twice` 6 · `validate_estate_directories` 5 · `prune_estate_backups_store` 5 · `installed_estate_root` 5 · `audit_estate_operation` 5 · `opening_an_estate_another_engine_holds_is_refused_as_in_use` 4 · `local_estate_driver` 4 · `estate_organisation_is_validated_at_the_public_boundary` 4 · `estate_organisation` 4 · `estate_inventory` 4 · `adopt_estate_directory` 4 · `a_served_estate_records_its_endpoint_and_withdraws_it` 4 · `run_estate` 3 · `rrd_estate_status` 3 · `reconcile_estate_backup_store` 3 · `plan_memory_estate` 3 · `installed_two_estate_isolation_under_concurrent_load` 3 · `estate_root` 3 · `estate_organisation_golden` 3 · `estate_audit` 3 · `estate_action` 3 · `estate_acl` 3 · `create_estate` 3 · `commit_estate_plan` 3 · `chain_estate` 3 · `administer_estate_store` 3 · `segment_estate_exceeds_native_descriptor_ceiling_and_reopens` 2 · `seed_estate` 2 · `resource_estate` 2 · `reconcile_estate_store` 2 · `estate_word` 2 · `estate_resource` 2 · `estate_directory` 2 · `estate_authority` 2 · `estate_and_query` 2 · `build_memory_estate_plan` 2 · `an_installed_estate_is_never_adopted` 2 · `adopted_estate_entries_are_bound_only_when_present` 2 · `traversal_evidence_follows_visited_edges_not_estate_size` 1 · `the_first_flight_on_an_empty_estate_installs_the_flight_tables` 1 · `public_source_preview_uses_host_clock_on_an_installed_estate` 1 · `public_admin_phase_type_does_not_reintroduce_a_physical_estate_dependency` 1 · `prior_source_cannot_cross_estate_project_or_root` 1 · `permissive_estate_directory_cannot_advance_recovery` 1 · `native_startup_failure_estate` 1 · `native_setup_failure_estate` 1 · `native_lifecycle_failure_estate` 1 · `lifecycle_failure_retains_its_owned_estate_but_success_cleans_it` 1 · `inherited_estate_preview_adopts_the_folder_without_effects` 1 · `existing_project_preserves_inherited_estate_and_opens_after_replay` 1 · `every_estate_resource_and_receipt_survives_native_reopen` 1 · `estate_recovery_prune_and_restore_are_one_fenced_engine_workflow` 1 · `estate_recovery_posture_is_strict_bounded_by_identity_and_path_free` 1 · `estate_read_contract_has_frozen_field_names_and_strict_empty_payload` 1 · `estate_directories_ready` 1 · `estate_backup_jobs_are_a_separate_strict_public_resource` 1 · `disjoint_estate_search_does_not_walk_every_descriptor` 1 · `cross_estate_session_denials` 1 · `context_rejects_recall_work_above_the_effective_estate_ceiling` 1 · `clients_per_estate` 1 · `canonical_ten_estate_profile_admits_one_shared_pool_and_denies_overcommit` 1 · `canonical_estate_document_requires_authority_backup_and_recovery_fields` 1 · `canonical_estate_backup_and_recovery_match_mx_and_kv_reopen` 1 · `authorization_is_key_estate_action_and_time_bound` 1 · `authenticated_estate_read_returns_the_public_snapshot_only` 1 · `a_staged_restore_is_published_in_place_and_the_estate_opens_from_the_backup` 1


**String literals in non-test source** (`crates/**/src/**`, comments excluded, literals with fewer than two spaces; number = occurrences): `"--estate"` 12 · `"estate"` 8 · `"estate-a"` 6 · `"estate-authority"` 4 · `"estate-read"` 3 · `"server/state/estate"` 3 · `"/v1/estates/{estate}/read"` 2 · `"estate.create"` 2 · `"../../assets/install/default-estate-configuration-v3.toml"` 1 · `"/v1/estates/"` 1 · `"/v1/estates//read"` 1 · `"/v1/estates/a/b/read"` 1 · `"DataEstateOrganisation::is_default"` 1 · `"EstateSnapshot"` 1 · `"RRD_ESTATE_TEST_HOLD_AFTER_EFFECT_FILE"` 1 · `"RRD_ESTATE_TEST_HOLD_AFTER_STEP_FILE"` 1 · `"ReadEstate"` 1 · `"RuntimeEstateOrganisation::is_default"` 1 · `"default-estate-configuration-v3.toml"` 1 · `"estate configuration"` 1 · `"estate-0"` 1 · `"estate-authority-read"` 1 · `"estate-backup-reconcile-request-{estate_id}-{at}"` 1 · `"estate-backup-reconcile-v1"` 1 · `"estate-backup-reconcile-{estate_id}-{worker}-{at}"` 1 · `"estate-control"` 1 · `"estate-reconcile-request-{estate_id}-{at}"` 1 · `"estate-reconcile-v1"` 1 · `"estate-reconcile-{estate_id}-{worker}-{at}"` 1 · `"estate-recovery-closure-v1"` 1 · `"estate-recovery-prune-audit-v1"` 1 · `"estate-recovery-prune-v1"` 1 · `"estate-recovery-restore-audit-v1"` 1 · `"estate-recovery-restore-v1"` 1 · `"estate-recovery\0{kind}\0{estate_id}\0{instance_id}\0{idempotency_key}"` 1 · `"estate-{estate}"` 1 · `"estate-{}"` 1 · `"estate.activity.refresh"` 1 · `"estate.authority.apply"` 1 · `"estate.backup.completed"` 1 · `"estate.backup.failed"` 1 · `"estate.backup.lease"` 1 · `"estate.backup.prepared"` 1 · `"estate.backup.schedule"` 1 · `"estate.desired.set"` 1 · `"estate.instance.activity"` 1 · `"estate.instance.observe"` 1 · `"estate.operation.lease"` 1 · `"estate.operation.receipt"` 1 · `"estate.recovery.pin.created"` 1 · `"estate.recovery.pin.released"` 1 · `"estate.recovery.policy.set"` 1 · `"estate.recovery.prune.completed"` 1 · `"estate.recovery.prune.operation.completed"` 1 · `"estate.recovery.prune.operation.prepared"` 1 · `"estate.recovery.prune.prepared"` 1 · `"estate.recovery.restore.operation.completed"` 1 · `"estate.recovery.restore.operation.prepared"` 1 · `"estate.recovery.restore.verified"` 1 · `"estate_directories_ready"` 1 · `"family={family}|record={ordinal:08x}|version={version}|estate=rrflow|"` 1 · `"rrd-estate-admin: {error}"` 1 · `"rrd-estate-backup-create-v1"` 1 · `"rrd-estate-controller: {error}"` 1 · `"rrd-local-estate-backup-v1"` 1 · `"rrflow-estate-configuration-v3\0"` 1 · `"rrflow-estate-organisation-v1\0"` 1 · `"rrflow-installed-estate-v3"` 1 · `"server/state/estate-recovery/e"` 1 · `"server/state/estate-recovery/{estate_id}/{kind}/{identity}"` 1 · `"server/state/estate/e/document"` 1 · `"server/state/estate/{estate_id}/document"` 1 · `"trial-{trial:04}-estate-{estate:02}"` 1 · `"{ESTATE_DIRECTORY}/.bootstrap.{storage_root_id}.pending"` 1 · `"{ESTATE_DIRECTORY}/.config.{storage_root_id}.pending"` 1 · `"{ESTATE_DIRECTORY}/.serving.pending"` 1 · `"{ESTATE_DIRECTORY}/bootstrap.md"` 1 · `"{ESTATE_DIRECTORY}/{plan_id}"` 1 · `"{kind}\0{estate_id}\0{instance_id}\0{idempotency_key}"` 1


Longer literals (error and usage messages containing the word) are in the same grep and were not enumerated here; 225 source lines carry at least one such literal.


## Appendix B — every `pub` item whose name contains estate, by crate

Source: `git grep -nP` for `pub[(…)] (struct|enum|type|trait|fn|mod|const|static) <name containing Estate/estate/ESTATE>` over `crates/**/*.rs` on `development/main`: 112 items, 89 fully `pub`. No `pub mod` matches: the modules are declared privately (`mod estate;`, `mod estate_control;`, `mod estate_layout;`, `mod memory_estate;` in `rrd-engine/src/engine/mod.rs`; `mod estate_authority;`, `mod memory_estate;` in `rrd-contract/src/lib.rs`) and re-exported. Paths are relative to the crate.


### `crates/adapters/rrflow-cli` — 1

- **pub(super) fn** (1): `run_estate` (tests/support/sustained/estate.rs:11)


### `crates/authority/rrd-engine` — 29

- **pub fn** (11): `estate_configuration` (src/engine/core.rs:376); `installed_estate_identity` (src/engine/core.rs:380); `read_estate` (src/engine/estate.rs:4); `administer_estate_store` (src/engine/estate_control/admin.rs:9); `reconcile_estate_store` (src/engine/estate_control/admin.rs:223); `reconcile_estate_backup_store` (src/engine/estate_control/admin.rs:280); `prune_estate_backups_store` (src/engine/estate_control/recovery.rs:11); `restore_estate_backup_store` (src/engine/estate_control/recovery.rs:237); `adopt_estate_directory` (src/engine/estate_layout.rs:89); `load_estate_configuration` (src/engine/installation/profile.rs:369); `plan_memory_estate` (src/engine/memory_estate.rs:14)
- **pub enum** (2): `EstateAdminAction` (src/engine/estate_control.rs:63); `EstateAdminResult` (src/engine/estate_control.rs:100)
- **pub type** (3): `EstateReconcileOutcome` (src/engine/estate_control.rs:106); `EstateBackupReconcileOutcome` (src/engine/estate_control.rs:108); `InstalledEstateRecord` (src/engine/installation/records.rs:118)
- **pub(super) const** (1): `ESTATE_DIRECTORY` (src/engine/estate_layout.rs:10)
- **pub(super) struct** (3): `EstateFilesystem` (src/engine/estate_layout.rs:34); `EstateLayout` (src/engine/estate_layout.rs:166); `InstalledEstate` (src/engine/tests/installation/restore.rs:5)
- **pub(super) fn** (8): `existing_estate_entries` (src/engine/estate_layout.rs:308); `reject_existing_estate` (src/engine/estate_layout.rs:352); `create_estate_directories` (src/engine/estate_layout.rs:379); `validate_estate_directories` (src/engine/estate_layout.rs:383); `validate_recovery_estate` (src/engine/estate_layout.rs:400); `an_installed_estate_is_never_adopted` (src/engine/tests/installation/configuration_and_adoption.rs:310); `inherited_estate_preview_adopts_the_folder_without_effects` (src/engine/tests/installation/configuration_and_adoption.rs:492); `existing_project_preserves_inherited_estate_and_opens_after_replay` (src/engine/tests/installation/configuration_and_adoption.rs:516)
- **pub(crate) fn** (1): `build_memory_estate_plan` (src/engine/memory_estate.rs:216)


### `crates/authority/rrd-estate` — 17

- **pub const** (2): `ESTATE_AUTHORITY_FORMAT` (src/authority.rs:10); `ESTATE_FORMAT` (src/model.rs:5)
- **pub struct** (11): `EstateAuthorityState` (src/authority.rs:120); `EstateBackupJob` (src/backup_job/jobs.rs:44); `EstateOperation` (src/model.rs:238); `EstateDocument` (src/model.rs:266); `EstateRecoveryPolicy` (src/recovery/model.rs:18); `EstateRecoveryPoint` (src/recovery/model.rs:61); `EstateRetentionPin` (src/recovery/model.rs:86); `EstateRestoreEvidence` (src/recovery/model.rs:105); `EstateRecoveryPruneIntent` (src/recovery/model.rs:131); `EstateRetentionDecision` (src/recovery/model.rs:209); `EstateRepository` (src/requests.rs:79)
- **pub trait** (2): `EstateBackupDriver` (src/backup_reconcile.rs:30); `EstateDriver` (src/reconcile.rs:77)
- **pub enum** (2): `LocalEstatePermission` (src/local_authorization.rs:14); `EstateRetentionPinKind` (src/recovery/model.rs:79)


### `crates/kernel/rrd-core` — 1

- **pub struct** (1): `RuntimeEstateOrganisation` (src/schema/organisation.rs:88)


### `crates/transport/rrd-client` — 1

- **pub fn** (1): `read_estate` (src/operation/operations.rs:50)


### `crates/transport/rrd-contract` — 59

- **pub fn** (2): `validate_adopted_estate_entries` (src/attunement/digests.rs:6); `estate_configuration_sha256` (src/deployment/configuration.rs:216)
- **pub const** (3): `MAX_ADOPTED_ESTATE_ENTRIES` (src/attunement/installation.rs:196); `ADOPTED_ESTATE_DIRECTORY` (src/attunement/installation.rs:198); `ESTATE_CONFIGURATION_FORMAT_VERSION` (src/deployment/profile.rs:6)
- **pub struct** (37): `DataEstateOrganisation` (src/data.rs:210); `EstateConfigurationInput` (src/deployment/configuration.rs:9); `EstateConfiguration` (src/deployment/configuration.rs:72); `InstalledEstateIdentity` (src/deployment/profile.rs:16); `ReadEstate` (src/estate_authority.rs:9); `EstateActivityPolicySnapshot` (src/estate_authority.rs:77); `EstateAuthorityDesiredSnapshot` (src/estate_authority.rs:125); `EstateAuthorityObservedSnapshot` (src/estate_authority.rs:133); `EstateAuthorityResourceSnapshot` (src/estate_authority.rs:144); `EstateAuthorityReceiptSnapshot` (src/estate_authority.rs:163); `EstateAuthoritySnapshot` (src/estate_authority.rs:175); `EstateDesiredSnapshot` (src/estate_authority.rs:197); `EstateObservedSnapshot` (src/estate_authority.rs:208); `EstateActivitySnapshot` (src/estate_authority.rs:224); `EstateInstanceSnapshot` (src/estate_authority.rs:235); `EstateLeaseSnapshot` (src/estate_authority.rs:244); `EstateReceiptSnapshot` (src/estate_authority.rs:253); `EstateOperationSnapshot` (src/estate_authority.rs:262); `EstateSnapshot` (src/estate_authority.rs:282); `EstateMutationResult` (src/estate_authority.rs:297); `EstateBackupReceiptSnapshot` (src/estate_authority.rs:322); `EstateBackupRecoveryPolicySnapshot` (src/estate_authority.rs:331); `EstateBackupJobSnapshot` (src/estate_authority.rs:341); `EstateBackupJobsSnapshot` (src/estate_authority.rs:368); `EstateBackupMutationResult` (src/estate_authority.rs:376); `EstateRecoveryPolicySnapshot` (src/estate_authority.rs:384); `EstateRecoveryPointSnapshot` (src/estate_authority.rs:396); `EstateRetentionPinSnapshot` (src/estate_authority.rs:421); `EstateRestoreEvidenceSnapshot` (src/estate_authority.rs:434); `EstateRecoveryPruneIntentSnapshot` (src/estate_authority.rs:452); `EstateRecoverySnapshot` (src/estate_authority.rs:465); `EstateRetentionDecisionSnapshot` (src/estate_authority.rs:477); `EstateRecoveryMutationResult` (src/estate_authority.rs:487); `EstateRecoveryPruneResult` (src/estate_authority.rs:494); `EstateRecoveryRestoreResult` (src/estate_authority.rs:505); `PersistMemoryEstate` (src/memory_estate.rs:55); `MemoryEstatePlan` (src/memory_estate.rs:90)
- **pub enum** (12): `EstateDesiredPhase` (src/estate_authority.rs:13); `EstateObservedPhase` (src/estate_authority.rs:21); `EstateActivityClass` (src/estate_authority.rs:35); `EstateOperationKind` (src/estate_authority.rs:45); `EstateOperationState` (src/estate_authority.rs:56); `EstateReceiptBoundary` (src/estate_authority.rs:68); `EstateAuthorityResourceKind` (src/estate_authority.rs:87); `EstateAuthorityStatus` (src/estate_authority.rs:104); `EstateAuthorityReceiptBoundary` (src/estate_authority.rs:114); `EstateBackupJobState` (src/estate_authority.rs:304); `EstateBackupReceiptBoundary` (src/estate_authority.rs:314); `EstateRetentionPinKindSnapshot` (src/estate_authority.rs:414)
- **pub(super) fn** (5): `adopted_estate_entries_are_bound_only_when_present` (tests/attunement_contract/validation.rs:97); `estate_read_contract_has_frozen_field_names_and_strict_empty_payload` (tests/public_contract/estate_and_query.rs:6); `estate_backup_jobs_are_a_separate_strict_public_resource` (tests/public_contract/estate_and_query.rs:65); `estate_recovery_posture_is_strict_bounded_by_identity_and_path_free` (tests/public_contract/estate_and_query.rs:133); `estate_organisation_is_validated_at_the_public_boundary` (tests/public_contract/schema_and_retrieval.rs:86)


### `crates/transport/rrd-server` — 4

- **pub(super) fn** (3): `estate_action` (src/http/capabilities.rs:30); `authenticated_estate_read_returns_the_public_snapshot_only` (tests/http_process/data_transaction.rs:851); `estate_context` (tests/support/server_fixture.rs:34)
- **pub(in crate::http) fn** (1): `read_estate` (src/http/handlers/data.rs:58)


Crates with estate usage but no estate-named `pub` item (rename is internal: locals, comments, test names, strings): `rrd-lsm` 11 files, `rrd-store` 10, `rrd-query` 6, `rrflow-qualification-*` 17 (public **fields** `estates`, `estate`, `requires_estate_interference` in `rrflow-qualification-storage/src/{capacity.rs:25,33, fleet.rs:133, fleet_latency.rs:9, model.rs:126, sustained/model.rs:71}` are serialized into the storage schemas), `rrd-inference` 2, `rrd-attunement` 2 (public field `estate` at `src/inventory.rs:17`), `rrd-vector` 1, `rrd-security` 1 (a test comment), `evaluation/lab` 1.
