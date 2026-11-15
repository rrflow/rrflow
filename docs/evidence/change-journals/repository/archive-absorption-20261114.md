# Absorption ledger for the 2026-11-14 archive

**Status:** active change evidence; updated as each archived ref is resolved

**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/repository/archive-absorption-20261114`

**Owner:** the one-branch rule in `AGENTS.md`

**Starting revision:** `main` at `2b14c3ff`.

## Why this exists

On 2026-11-14 the repository had 35 worktrees and 172 local branches. They were collapsed to one checkout on `main`. Everything `main` did not contain was first saved under `refs/archive/20261114/` and in `D:/localdev/devlabs/rrflow-archive-20261114.bundle`: 81 refs, of which 46 distinct tips carried commits `main` lacked (106 commits, 1,466 file versions differing from `main`).

Saving is not absorbing. Owner, 2026-11-14: every archived ref is reviewed and its valuable work is brought into `main` properly, by hand, in today's structure and naming. Nothing is merged blindly, and nothing is called absorbed without proof.

## Method

A ref leaves the archive only when one of these is proven and recorded here:

- **Ancestor:** its commit is already an ancestor of `main`.
- **Same change:** every commit it carries has a commit on `main` with the same `git patch-id`.
- **Contained or duplicate:** another archived ref still holds all of its commits.
- **Empty:** it changed zero bytes since it left `main`.
- **Reviewed:** each item it changed was read and is on `main` (cited), superseded (cited), or ported, with the destination named.

A verified deficiency that `main` does not track goes into the POA&M. The bundle file is kept regardless.

## Resolved by proof, nothing to port

| Ref | Proof |
|---|---|
| `branch/agent/rrflowdb-postgres-identity` | Same change as `112443b3e` on `main` (patch-id `17f16f55fdb2bbec`); `main` later added the PostgreSQL contract and H-08 |
| `branch/agent/rrflowdb-postgres-plan` | Same changes as `112443b3e`, `9fbec8fc9`, `59d12a4a1` |
| `branch/agent/c07-backup-scheduling` | Every commit has the same change on `main` |
| `branch/agent/vector-cost-probe-20261111` | Every commit has the same change on `main` |
| `head/f06-main-integration-20261107` | Every commit has the same change on `main`; its uncommitted work is `wip/f06-main-integration-20261107`, still open below |
| `branch/agent/source-bound-replan-design` | Contained in `branch/codex/d02-historical-plan-replay`, still open below |
| `branch/backup/agent-project-map-editor-20261109` | Contained in `branch/agent/project-map`, still open below |
| `head/d03-snapshot-visibility`, `head/parser-241-reconcile-20261107`, `head/rrflow`, `head/vector-read-path-worktree` | Exact duplicates of branch refs still open below |
| `branch/agent/c07-run-progress`, `branch/codex/ci-alpha-throughput`, `branch/codex/connectome-real-graph-evidence` | Empty: 0 bytes of diff since leaving `main` (package-opening commits) |
| 31 `head/*` refs of worktrees | Ancestor: each checked-out commit is already on `main` |
| `branch/codex/d02-historical-plan-replay`, `branch/codex/d02-immutable-job-design` | Reviewed: 206 of 210 added lines verbatim in `docs/architecture/source-bound-plan-design.md`; the other 4 are the same passage in `main`'s later wording (lines 125-136) |
| `branch/codex/ci-macos-cache-warm` | Every added line on `main` (109 of 109) |
| `branch/codex/ci-conflict-reconciliation` | Every added line on `main` (20 of 20) |
| `branch/agent/leftovers-20261110` | Every added line on `main` (15 of 15) |
| `branch/tmp/agent/lab-definitions-automatic` | Its one file is byte-identical on `main` |
| `wip/comparison-pr258-integration-trial-20261106` | Every added line on `main` (65 of 65) |

## Reviewed

### `wip/c07-independent-review` (left `main` 2026-10-26)

Uncommitted work: an LSM change and a review document with six findings.

| Item | Status | Evidence |
|---|---|---|
| Read views share one `Arc<Manifest>` instead of copying the manifest per read | On `main` | `crates/persistence/rrd-lsm/src/database.rs:242`, `segment/reader/views.rs:16,57,78`, `database/open.rs:158,327` |
| Reader lifetime tests | On `main` | 98 of 101 added lines present in `segment/reader_lifetime_tests.rs` |
| Finding 1: tasks orchestrate nothing | Superseded | Later task worker, assessment and endpoint proposal operations |
| Finding 2: project ingestion cannot run in the worker | Superseded | Parse and follow mode, #376 |
| Finding 3: audit pages can reject valid history | Tracked | POAM-030 |
| Finding 4: diagnostics fail during audited traffic | Tracked | POAM-029 |
| Finding 5a: task and flight lists read every version before paging | **Was untracked; still true on `main`** | Recorded as [POAM-081](../../../poam/rrflow-1.0-alpha/poam-081.md): `task.rs:245-251`, `flight/launch.rs:317-323` |
| Finding 5b: a read view charged the whole manifest's bytes | On `main` | `views.rs:90-108` charges only segments not already pinned; `tests/projected_lifetime.rs:452` |
| Finding 6: context rebuilds BM25 and bypasses the planner | Tracked | Kernel package K4, POAM-075 |

Verdict: absorbed. One deficiency was recovered.

### `branch/agent/lsm-database-split-20261113` (left `main` 2026-11-13)

| Item | Status | Evidence |
|---|---|---|
| Split of `rrd-lsm/src/database.rs` into `database/*.rs` | On `main` | 573 of 573 added lines in `database/transactions.rs`; the other 9 changed files are byte-identical on `main` |
| Its journal, with the module table and the characterization results (253 `rrd-lsm` tests) | **Was missing; ported** | Restored unchanged as [lsm-database-split-20261113.md](lsm-database-split-20261113.md) |

Verdict: absorbed.

### The three C-07 merge-trial worktrees (left `main` 2026-10-29)

Each was an uncommitted trial merge of one branch onto another, made to see whether it still applied. Their files were later split into folders on `main`, so a same-path comparison proves nothing. Each was checked against the whole of `main` instead: every added line (generated SDK output excluded) searched at any path, and every declared identifier searched as a word. Lines found nowhere were then read one by one.

| Ref | Added lines checked | Nowhere on `main` | Declared identifiers nowhere on `main` |
|---|---|---|---|
| `wip/h04-c07-merge-preflight` | 2,460 | 39 | 1 |
| `wip/c07-pr206-preflight` | 2,079 | 72 | 0 |
| `wip/c07-h04-stale-preflight` | 2,644 | 146 | 0 |

`wip/h04-c07-merge-preflight` (C-07 native capacity and the storage cursor, merged onto the H-04 candidate):

| Item | Status | Evidence |
|---|---|---|
| Storage cursor contract tests, three of them | On `main` | `crates/transport/rrd-contract/tests/public_contract/storage_cursor.rs:82,129,169` |
| Range and record-kind cursor checks over a real server | On `main` | `rrflow-cli/tests/support/running_engine.rs:200`, `rrd-client/tests/real_server/negotiation.rs:869` |
| Sampler trace test and its helpers | On `main` | `rrflow-cli/tests/support/sustained/campaign.rs:176,262,347`; CI selects it at `ci-reusable.yml:774` |
| `ReaderSelection` and its identity test | On `main` | `support/sustained/reader.rs:47,60`; `rrflow-qualification-storage/src/sustained/tests.rs:6` |
| Native worker fixtures and their selectors | On `main` | `support/sustained/native_worker.rs:273,567,599,680`; `scripts/ci/selected_tests.py:36-39` |
| Failure capture for `hdiutil create` | On `main` | `scripts/ci/run_native_capacity.py:37,54,97` |
| Test that a busy create is attempted once | Superseded | `main` retries a finished busy create five times and never retries an unfinished one: `scripts/ci/test_native_capacity.py:380,397,528` |
| Manual preflight running three fresh and existing pairs | Superseded | `c07-native-preflight.yml` now runs selected cases through `scripts/ci/run_macos_capacity_preflight.py:18-25` |
| The other unmatched lines | Superseded | An older contract digest, imports from `rrflow_eval::storage` (now `rrflow-qualification-storage`), and visibility keywords |

`wip/c07-pr206-preflight` (the endpoint proposal controller, merged onto a C-07 branch):

| Item | Status | Evidence |
|---|---|---|
| Controlled attempt reserve and finish | On `main` | `engine/reasoning_budget/reserve.rs:47`, `finish.rs:29,56` |
| Capacity lease state and its corruption check | On `main`, extended | `engine/endpoint_proposal/capacity.rs:175` checks the format version and the bound across outstanding, worker and assessment leases (`:231`) |
| An attempt cannot hold two outputs | On `main`, extended | `rrd-contract/src/reasoning_budget.rs:330-340` now covers result, proposal and worker output |
| Proposal bound to the policy allowance and the task scope | On `main` | `rrd-contract/src/endpoint_proposal.rs:376,569` |
| Endpoint failure keeps "may continue" truthful | On `main` | `engine/endpoint_proposal.rs:253,270,646` |
| Unknown request fields refused | On `main` | `engine/tests/reasoning_budget/proposal.rs:274` |
| Installed controller proof and fault relay scripts | On `main` | `scripts/inference/installed_controller_proof.py:836`, `installed_controller_fault_relay.py:78` |
| The other unmatched lines | Superseded | Old catalogue counts (63 endpoints, 54 security actions), an old contract digest, old fixture digests, earlier wording of `lfg-swag.md` |

`wip/c07-h04-stale-preflight` (the first form of the attunement replan, merged onto the landed C-07 parent):

| Item | Status | Evidence |
|---|---|---|
| Replan receipt validation, source observation, generation counter | On `main` | `engine/attunement/replan.rs:6,98,113,127` |
| Replan precondition | Superseded by a wider rule | The trial allowed a replan only after at most one checkpoint and an Inventory failure. `main` allows it after Connect, Inventory and Parse checkpoints and for any retryable `phase-unavailable` failure past Connect: `engine/attunement/replan.rs:155-193` |
| Preview bound to the predecessor revision | On `main` | `rrd-contract/src/attunement/replan.rs:141` |
| Installed, HTTP, contract and crash tests | On `main` | `rrflow-cli/tests/installed_lifecycle/replan.rs:59`, `rrd-server/tests/http_process/installed_flows.rs:6`, `rrd-contract/tests/attunement_contract/operations.rs:24`, `validation.rs:492`, `engine/tests/attunement/replan.rs:532` |
| Python installed attunement test | On `main`, stricter | `sdks/python/tests/test_installed_attunement.py:34`; CI requires two passing tests where the trial required one: `ci-reusable.yml:593` |
| Generated client operations module | Superseded | Renamed to `crate::operations`: `rrd-client/src/attunement.rs:3` |

Verdict for all three: nothing to port. This check also met the miscased `InstalledengramRecord`, `MemoryengramPlan` and `DataengramOrganisation` on `main`; they are already tracked in POAM-078 and belong to the rename.

### `branch/wip/project-map-pre-main` (left `main` 2026-10-30)

One local commit, an early draft of `dd27aa96a` "Create and refresh a project map for installed projects", which is in `main`'s history. Whole-tree check: 478 added lines, 166 nowhere on `main`.

| Item | Status | Evidence |
|---|---|---|
| `refresh_project_map`, the census and the owner's notes markers | On `main`, later form | `crates/authority/rrd-engine/src/engine/project_map.rs:36-81` reads `project_entries` and the shared language table instead of its own walk |
| Ignore rules through the third-party `ignore` crate, and its test | Superseded | `main` reads Git's index and ignore files itself (`installation/inventory/git_inputs.rs`, `rrd-attunement/src/git.rs`) and has no `ignore` dependency; `inventory/tests.rs:368,423` cover ignored folders |
| Install and serve call the map and print one line | **Was lost from `main`; restored** | Merge `4c2f2a6a1` (2026-11-09) dropped the caller that `dd27aa96a` added, leaving the function with none. Restored in `crates/adapters/rrflow-cli/src/installed.rs` |
| Install recovery admits `project-map.md` and its pending file | **Was lost; restored** | `engine/estate_layout.rs`, `validate_recovery_estate` |
| Test `the_project_map_describes_the_project_and_keeps_the_owners_notes` | **Was lost; restored** | `engine/tests/installation/project_map.rs`, with a second test for recovery, replay and reopening |
| The contract's "Project map" section | **Was lost; restored** | `docs/reference/deployment/installed-lifecycle.md#project-map`, which the map's journal already linked to |

Verdict: absorbed. Nothing was taken from the draft itself; the four lost pieces were restored from `main`'s own commit and are recorded in [their journal](../gate-d/project-map-callers-restored-20261114.md).

### `wip/c07-write-conflict-diagnosis` (left `main` 2026-10-30)

Its committed work is on `main`. The uncommitted part was a trial merge of the source-bound replan branch. Whole-tree check: 7,175 added lines, 362 nowhere on `main`, one declared identifier nowhere on `main`.

| Item | Status | Evidence |
|---|---|---|
| Source-bound replan, selected plans, the `rrd-attunement` crate and their tests | On `main` | `engine/attunement/{control,inventory,replan}.rs`, `engine/installation/profile.rs:138,174`, `rrd-contract/tests/attunement_contract/operations.rs:6,379`, `validation.rs:508`, `rrd-engine/tests/workspace_architecture/dependencies.rs:219`, `rrflow-cli/tests/installed_lifecycle/plans.rs:6` |
| Whole-set revalidation of the inventory after reading | On `main` | `installation/inventory/scan.rs:148-149` |
| A plan must end with a Verify step | Superseded on purpose | `335fb83a6` (2026-11-03) made Verify optional but last-only: `installation/profile.rs:144-148`, `rrd-contract/src/attunement/plan.rs:75` |
| Unit test `source_change_and_stale_transaction_keep_distinct_recovery_actions` | **Was missing; ported** | `crates/transport/rrd-server/src/http/response.rs`. `main` tested only the stale-transaction side of the two errors mapped ahead of the generic table |
| Journal `c-07-recovery-inventory-integration.md` | **Was missing; ported unchanged** | [gate-c](../gate-c/c-07-recovery-inventory-integration.md). Its numbers were recomputed on 2026-11-14 from the retained files under `target/c07-semantic-4k-fa945bc7/evidence` and `target/c07-cadence-4k/evidence`: 150 mixed writes, one overlapping pair (336 rejected, 337 acknowledged, 41,657,100 ns), both SHA-256 values, the two cadence groups and their finish order all match |
| The other unmatched lines | Superseded | Earlier wording, old digests, and test timing constants |

Verdict: absorbed.

### `branch/codex/attunement-job-discovery` (left `main` 2026-10-31)

One commit: a paged list of a project's attunement jobs. Whole-tree check: 336 added lines, 32 nowhere on `main`, no declared identifier missing.

| Item | Status | Evidence |
|---|---|---|
| Contract types, engine listing, HTTP handler, client operation | On `main` | `rrd-contract/src/attunement/discovery.rs`, `engine/attunement/discovery.rs:7-145` |
| Successor lineage rule | On `main`, wider | `discovery.rs:119-130` also accepts a succeeded predecessor, which the branch refused |
| Five engine tests | On `main` | `engine/tests/attunement/discovery.rs`; the unmatched lines are fixed time offsets that `main` replaced with the returned job's own times |

Verdict: nothing to port.

### `branch/agent/hdiutil-held-helper-20261111` and `branch/agent/macos-image-teardown-20261111` (left `main` 2026-11-11)

Four commits and one commit. The teardown branch's single change (`bcac69311`) is the same work as the held-helper branch's `70e4170f7`, which its journal says it carried. Later that day `c2cc5baa7` on `main` replaced the per-case image create with one template per suite, so neither branch applied any more.

| Item | Status | Evidence |
|---|---|---|
| Retry a busy create whose escaped helper still holds the output | **Was missing; ported by hand** | `main`'s one remaining create refused exactly the failure that was observed (`stderr_open: true`). Now in `scripts/ci/run_native_capacity.py` (`darwin_template`, `is_busy_darwin_create`, `held_output`) and `run_offline_lifecycle.py` (`wait_closed`), with three tests that fail against the old guard |
| Capacity tests keep simulated retry lines out of real job logs | **Ported** | `scripts/ci/test_native_capacity.py`, `setUp` |
| Wait for the device node after each detach, and its four tests | Superseded | On `main` no create follows a detach. The branch's own preflight measured every wait as 0 ms |
| POAM-074's wrong file attribution, and the held-output cause | **Was missing; ported** | [POAM-074](../../../poam/rrflow-1.0-alpha/poam-074.md) |
| The branch's journal: cause, run 43017327376, two preflight runs | **Carried** | Into [the port's journal](../gate-c/c-07-hdiutil-held-output-retry-20261114.md), with the branch named as the source. Not restored as a file, because its "changed paths" describe code `main` does not have |

Verdict: both absorbed. The port is unit-tested only; nothing has run on macOS.

### Five refs checked against the whole of `main`, nothing to port

Each was checked the same way as the trial worktrees: every added line searched at any path, every declared identifier searched as a word, and the lines found nowhere read one by one.

| Ref | Lines checked | Nowhere on `main` | Identifiers missing | What the unmatched lines are |
|---|---|---|---|---|
| `branch/agent/d04-source-spans-reconcile-wip-20261109` | 4,346 | 119 | 0 | A merge of `main` as it stood on 2026-11-09. The lines are older forms of code `main` later split: worker packets (`endpoint_worker/execute.rs`), batched versioned scans (`rrd-store/src/access/versioned_read/ranges.rs:72-84`), HNSW cost sampling and reachability repair (`rrd-vector/src/hnsw/{config.rs:15,graph.rs:168}`), page charge accounting (`rrd-lsm/src/segment/reader/cursors.rs:135`). The "no loaded binding" search refusal is on `main` as the `refuse` closure at `engine/vector/search/operations.rs:339-352` |
| `branch/agent/vector-generation-read-path-wip-20261109` | 8,652 | 222 | 0 | Also a merge of `main`. Its two red regressions are live tests on `main` (`engine/tests/vector_index/hnsw_read_path.rs:820,884`) and passed in the engine library run on 2026-11-14 (479 passed, 0 failed). Its one new paragraph in `docs/reference/vector/search.md` described the defect those tests exposed; `main`'s line 100 describes the fix |
| `branch/agent/d03-snapshot-visibility-wip-20261109` | 246 | 20 | 0 | The storage-contract preflight job. On `main` at `ci-reusable.yml:679-709,1131,1156,1186`, pointing at `rrflow-qualification-storage` where the branch said `rrflow-eval` |
| `branch/agent/rrflow-cld-triggers-20261110` | 256 | 5 | 0 | A visibility keyword and three lines of its journal's header; the code is at `engine/diagnostic/storage.rs:109` |
| `branch/agent/turboquant-scan-20261111` | 345 | 80 | 1 | `turbo_segment.rs` is the same change as `24214d224` on `main` (`turbo_segment/config.rs:48`). The calibration example was rewritten on `main` by `d43578850` with nine rounds and a median with range; the missing identifier `median_ns` is its old helper |

### The lab and contract-split refs (left `main` 2026-11-12 and 2026-11-13)

| Ref | Item | Status | Evidence |
|---|---|---|---|
| `branch/agent/lab-core-crate-20261112` | The shared lab vocabulary crate | On `main`, renamed | `crates/evaluation/rrflow-evaluation-core/src/lib.rs:17,25,43,54,86,122` |
| | Nesting under `crates/evaluation/workbench/lab` | Superseded | The layout approved on 2026-11-12 is `rrflow-evaluation-core`, `lab/` and `qualification/`; `rrflow-eval` no longer exists |
| `branch/agent/workbench-lab-data-20261112` | Moving the lab's 94 data files to `eval/workbench/lab` | Superseded; no data lost | Compared file by file with `main`: 67 are the same blob at `eval/lab/…` or `eval/schemas/…`, 23 more are identical once the path text is replaced, and 4 (`README.md`, `pod.sh`, `pod_run.sh`, `run_system.sh`) differ only in folder depth and in naming `rrflow-lab` where the branch said `rrflow-eval`. None is absent |
| `branch/agent/contract-lib-split-20261113` | The split of `rrd-contract/src/lib.rs` into 22 modules | On `main` | `c1dfbb346`, same subject; `main` has since split further |
| | Architecture rules read the whole crate | On `main` | `rrd-engine/tests/workspace_architecture/support.rs:631` (`module_source`) |
| | Its journal: method, owner direction, characterization results | **Was missing; ported unchanged** | [contract-lib-split-20261113.md](contract-lib-split-20261113.md) |

### The four Parse refs (left `main` 2026-11-11)

`branch/local/p3-stack` is a restack of `branch/agent/p3-parse-executor-20261111`: the two tips differ only in five documentation files, four of which `main` has. Whole-tree checks: `p3-enable-parse` 16 lines, 10 nowhere on `main`; `p3-parse-step-executor` 479 and 28; `p3-parse-executor` 1,236 and 602; `p3-stack` 1,111 and 604.

| Ref | Item | Status | Evidence |
|---|---|---|---|
| `p3-enable-parse` | Parse is executable, so derived plans select it | On `main`, narrower on purpose | `engine/attunement/evidence.rs:12-25` enables it only where a supervisor exists (Windows and Linux); the branch enabled it everywhere |
| `p3-parse-step-executor` | The step, publish and worker split | Superseded | Every function it declares exists on `main`. `main` replaced its JSON-size budget with a physical measurement (`parse/step.rs`), which fixed the 2026-11-12 batch-size failure |
| `p3-parse-executor`, `p3-stack` | Source facts, worker protocol, supervisors | On `main` | `rrd-attunement/src/{facts.rs,worker.rs}` byte-identical; supervisors at `engine/attunement/{windows,linux}_supervisor.rs` |
| | The first executor (`run_batch`, `FileOutcome`, `ParseCommit`, commit shares, physical retries) | Superseded | `main`'s rewrite: `parse/step.rs`, `parse/worker.rs`, `parse/publish.rs`, `parse/records/` |
| | Retirements stop when a commit is full | **Was lost in the rewrite; restored** | On `main` every departed file was retired in one unbounded batch. Deleting 40 small files made every Parse Step fail (35.8 MB against a 16 MiB limit) and the job could never finish. Fixed in `parse/step.rs` |
| | Test `a_deletion_larger_than_one_commit_retires_across_steps` | **Ported; it failed on `main`** | `engine/tests/attunement/parse_step.rs`, rewritten on `main`'s helpers; committed failing as `6a2b6d013`, then fixed |
| | Test `an_edit_reparses_only_the_edited_file_and_a_deletion_retires_its_facts` | **Ported** | Same file; `main` had no end-to-end test of the deletion half or of the change feed |
| | Removal of the unused node proposal (`parse_source`, `ParseProposal`, `SyntaxNode`) | **Ported** | `rrd-attunement/src/parse.rs`, `tests/parse.rs`; only its own tests called it |
| | Tests for query read-back, multi-step parsing and oversize rejection | On `main`, later form | `parse_step.rs`: `parsed_facts_are_read_through_their_commit_indexes`, `a_file_limit_spreads_parse_over_steps_without_a_checkpoint_until_done`, `facts_larger_than_one_physical_batch_are_published_across_steps` |
| | Test that a Step replay commits nothing | Superseded | The general replay path returns the stored receipt before dispatch (`engine/attunement/control.rs:58-72`); no Parse-specific test was added |

Verdict: all four absorbed. The work is recorded in [its journal](../gate-d/d-02-parse-retirement-across-steps-20261114.md).

### `branch/agent/project-map` and `branch/agent/project-map-final` (one commit, `e305e80f7`)

Seven commits, all made through the owner's editor.

| Commits | What they are | Status | Evidence |
|---|---|---|---|
| `9b60e959f`, `94e685c51`, `16c7318a7`, `2e4eb9d14` (2026-11-08 and 09) | Real-project workbench scripts, prompt sets, results, 57 crate changes | On `main` | `94c08bc98` carries the same tree. `git diff 94c08bc98 2e4eb9d14` is five files: four copies of one 176 MB `rrflow-runner.exe` and a 53 MB captured `connectome-ready.json`, left out on purpose because they cannot be pushed |
| `af19567b7` (2026-11-14 17:00) | "Replace estate terminology with engram" | Nothing to port | Checked mechanically: all 27 files are exactly a case-insensitive replacement of "estate" by "engram", including inside other words (`LifecycleState` became `Lifecyclengram`, "restatements" became "rengramments") and inside recorded prompts and results |
| `21924472e` (17:01) | "Refactor code structure" | Nothing to port | All 3 files are exactly `usengram` put back to `useState` |
| `e305e80f7` (17:07) | "Add endpoint selection and worker functionality" | Nothing to port as code; intent recorded | 28 file moves with identical content, and the Connectome gitlink moved to `f7f481252`. No endpoint or worker code. With the paths inside the files unrenamed, the tip cannot compile |

The moves in `e305e80f7` state a wanted structure, which belongs to the rename package and is not lost with the ref: `crates/authority/rrd-estate` to `rrd-engram`; `engine/estate_control.rs` and `estate_layout.rs` to `engram_*`; `rrd-contract/src/memory_estate.rs` to `memory_engram.rs`; and the seven `rrd-contract/src/endpoint_*.rs` files into `src/endpoints/`.

Verdict: nothing to port. The two unpushable files exist only in the bundle, at `2e4eb9d14`.

### `branch/codex/readable-attunement-plan` (left `main` 2026-10-31)

Six commits. Whole-tree check: 739 added lines, 291 nowhere on `main`, two declared identifiers nowhere on `main`.

| Item | Status | Evidence |
|---|---|---|
| A stored plan step keeps its readable description, bound by the plan digest | **Was missing; ported by hand** | `rrd-contract/src/attunement/plan.rs`, `installation.rs` (contract version 3), `rrd-engine/src/engine/installation/profile.rs` |
| A description for steps the engine derives from evidence | **New; the branch predates derived plans** | `rrd-contract/src/attunement/derivation.rs`, `derived_step_description` |
| Contract test `plan_preserves_and_binds_readable_step_descriptions` | **Ported, with one more case** | `rrd-contract/tests/attunement_contract/validation.rs` |
| Engine and installed Python read-back assertions | **Ported** | `engine/tests/attunement/replan_receipts.rs`, `sdks/python/tests/test_installed_attunement.py` |
| Fixture renamed to `install-attunement-v3.json` | **Ported** | Regenerated by the golden test |
| Generated SDK files and OpenAPI digest | Regenerated, not copied | The branch's generated output was for 70 operations; `main` has 83 |
| The five other commits (offline lifecycle repair, bounded Inventory, Inventory evidence) | On `main` | They integrate neighbouring work `main` already has; no declared identifier of theirs is missing |
| Its six journals | Carried in substance | They record a candidate that never landed and cite PRs by number. The facts that still hold are in [the port's journal](../gate-d/d-02-readable-plan-steps-20261114.md) |

Verdict: absorbed. It cost a contract version: version 2 attunement state is rejected.

### `wip/f06-main-integration-20261107`: first items absorbed, the ref stays open

Uncommitted F-06 cold-storage hardening on top of a head `main` already has. Its two recorded defects describe `main` today, so they are in the POA&M now. The code items are not ported yet.

| Item | Status | Evidence |
|---|---|---|
| TypeScript SDK silently rounds 64-bit integers | **Was untracked; verified on `main`** | [POAM-082](../../../poam/rrflow-1.0-alpha/poam-082.md): `sdks/typescript/src/transport.ts:35` |
| Rust SDK conformance test passes without running | **Was untracked; verified on `main`** | [POAM-083](../../../poam/rrflow-1.0-alpha/poam-083.md): `rrd-client/tests/sdk_conformance.rs:51-53` |
| The Rust conformance test is ignored by default, requires its manifest, and the runner requires its completion line | **Ported** | `rrd-client/tests/sdk_conformance.rs`, `scripts/ci/run_sdk_conformance.py` (`run`, `required_output`), two tests in `scripts/ci/test_sdk_private_manifest.py`, name owned in `scripts/ci/selected_tests.py`. The same defect exists in the .NET client and a milder form in Java; POAM-083 stays open for them |
| Fair scheduling between native maintenance and CLD publication | **Ported; the starvation was reproduced on `main`** | With compaction debt present the cold copy was never offered work (failing test `d9c80c921`). Fixed in `engine/maintenance.rs`, `engine/core.rs` and `rrd-server/src/http/maintenance.rs`; three tests |
| A timeout keeps its identity through cold encode, decode and Parquet export | **Ported** | `rrd-query` (`Error::DeadlineExceeded`, `ColdHistoryReadError`, the Parquet sink's recorded failure), `engine/cold_history/{files,maintain}.rs`, `engine/query_export.rs`; eight tests. [Journal](../gate-f/f-06-cold-history-hardening-20261114.md) |
| Bounded, deadline-checked artifact read | **Ported** | `read_cold_artifact` in `engine/cold_history/files.rs`; test `artifact_read_checks_deadline_and_exact_length` |
| Kill-at-each-boundary publication test | **Ported; passes on `main`** | `engine/cold_history/tests.rs`, `publication_survives_process_termination_at_each_boundary`, with test-only checkpoints in `files.rs` and `maintain.rs` |
| Restricted-grant denial test | **Ported; passes on `main`** | `cold_history_denies_row_or_field_restricted_changefeed_grants`, same file |
| Lock-free admission test | **Ported; passes on `main`** | `rrd-store/src/rrflow_kv/tests.rs`, `immutable_file_admission_skips_held_database_locks` |
| Installed read-back proves coverage, not just existence | **Ported; passes on `main`** | `rrflow-cli/tests/installed_lifecycle/engrams_and_history.rs`: three known transactions, the copy must reach their last cursor, and each acknowledged cursor appears exactly once with its exact mutation, before and after a restart |
| SDK cold-history conformance in the Rust, Python, Go, Java, .NET and TypeScript clients | **Ported** | The shared corpus gains a `cold_history` domain (`rrd-contract/src/sdk_conformance.rs`, `fixtures/rrd-sdk-conformance-v1.json`); the harness publishes one generation before any client runs; each client reads generation 1 and requires the seeded actor. Unlike the archive, the corpus carries no policy: `main` installs with cold history on by default, so the installation identity is unchanged. Run against a real daemon here for Rust, Python and Go; Java, .NET and the TypeScript run are proven only by the gate |

| The worktree's own 1,276-line journal | **Kept as a record** | `main`'s journal is a 174-line rewrite and 927 of the archive's 1,051 substantive lines are not on it. Preserved unchanged except title, status, coordinate and a provenance note: [f-06-derived-cold-history-worktree-20261107.md](../gate-f/f-06-derived-cold-history-worktree-20261107.md) |

Verdict: every code and test item of this ref is on `main`, the two SDK defects are in the POA&M, and the journal is kept.

Also recorded while verifying this ref, though not from it: the 2026-11-14 nightly failed on Windows creating its disposable account, [POAM-084](../../../poam/rrflow-1.0-alpha/poam-084.md).

### `wip/engram-rename-diagnosis-20261106` (left `main` 2026-11-06)

A repaired blanket estate-to-engram substitution on a main eight days old, saved with a merge of the `main` of 2026-11-07. Its tree cannot be applied: 617 files differ, 238 still differ from today's `main`, mostly module splits since. What it holds:

| Item | Status | Evidence |
|---|---|---|
| The rename itself, and the old-to-new map | **Applied on `main` 2026-11-15 (`eb6c89a38`, repaired by `44143b72f`; gate green)** | [Review](engram-rename-review-20261114.md) (a reviewer's reading, claims not re-run) and POAM-078, which now points at it. [Journal](engram-rename-applied-20261115.md). Connectome's own rename stays open in its repository |
| The attempt's own journal: OpenAPI digests, 289 changed leaves, the reopen failure on a persisted `estate_id`, operation descriptions | **Kept as a record** | [engram-connectome-repair-2026-11-07.md](../gate-h/engram-connectome-repair-2026-11-07.md), unchanged but for its header; its POAM-053 collides with an unrelated entry, so no POA&M file was taken |
| Project map has no caller | **Restored** | [Journal](../gate-d/project-map-callers-restored-20261114.md) |
| An unimplemented worker adapter must fail closed | **Reproduced on `main`; fixed** | `engine/endpoint_worker/execute.rs`; test failing first |
| A selection must match the prompt flight it names | **Reproduced on `main`; fixed** | `engine/endpoint_proposal.rs`; test failing first |
| Routing narrows the offered candidates | **Read on `main`, not run; fixed** | Proposal and worker. No engine test sends a routed request |
| Observation exports carry the selector identity | **Missing on `main`; ported** | `rrd-observation/src/references.rs`, `observation_delivery/persistence.rs`; validation test |
| Capture-fixture technique for renaming a field without rewriting recorded bytes | **Done** | `observed_configuration` projection helpers; [journal](engram-rename-applied-20261115.md) |
| Its `connectome` gitlink and Workbench paragraph | Superseded | `main` pins a newer Connectome |

All five dropped behaviours are in [one journal](engram-rename-dropped-behaviours-20261114.md).

**Resolved.** The ref was in the bundle (`d7ef0a13b`) and was deleted from `refs/archive` on 2026-11-15 after the rename landed.

Verdict: the rename landed; this ref is resolved.

### `branch/backup/main-before-sync-fix-2026-10-30` (left `main` 2026-10-30)

One file, `.claude/settings.json`: two permission rules pointing at a long-gone scratchpad and two `additionalDirectories` under a deleted worktree. Nothing in it applies to any other checkout, and the owner's standing rule is that permission rules live in the ignored `.claude/settings.local.json`. **Dropped, nothing absorbed.** It was in the bundle and was deleted from `refs/archive` on 2026-11-15.

### `wip/rrflow-existing-project` (left `main` 2026-11-14)

The uncommitted work of draft PR #247 on a `main` of 2026-11-02 (838 commits behind): 18 files, +1,360 −121. An earlier entry here called it superseded by an owner decision and deleted it; that was wrong, the owner never decided the move-aside behaviour, and the ref was restored from the bundle.

**Absorbed 2026-11-15, by hand, in `main` `8c60d3101` (gate green).** It leaves an existing `.rrflow` and `.rrflow/rrd` in place, binds their native identity and security descriptor in the sealed plan, and stages the locator and bootstrap privately behind publication proofs. It replaces the relocation into `.rrflow/previous/` that #236 had chosen.

| Item | Status | Evidence |
|---|---|---|
| Contract fields, retain rule, publication proof | **Ported** | `attunement/installation.rs`; contract test for retained plans |
| Native directory security digest | **Ported; Linux fixed** | Windows owner, group and DACL; Linux uid, gid, mode and ACLs. The archived Linux read never worked on an O_PATH handle (EBADF, caught by the gate); it now reads through `/proc/self/fd` |
| Retained layout, ancestor binding, installation, recovery, open | **Ported against `main`'s split modules** | `engram_layout.rs`, `installation/*` |
| Its tests | **Ported** | Preservation oracle un-ignored and passing; `retained_*` tests; process death at each of eleven install stages; in-process stage recovery |
| #236's relocation | **Removed** | `adopted_engram_entries`, its constants, `adopt_*` helpers and tests |
| Leniency for a non-private `.rrflow` | **Added, not in the archived work** | The archived version admitted it everywhere, which let a fresh engram's permission drift pass (`permissive_engram_directory_cannot_advance_recovery` failed). Leniency now applies only while installing or when a retained install left its proof |
| Legacy `.rrflow/instance.toml` | **Changed** | Left in place instead of refused |

[Journal](../gate-d/existing-project-retained-admission-20261115.md). Not proved: macOS, crash windows inside a staging write, re-admission after a changed retained descriptor, a run against the owner's real checkout.

**Resolved.** The ref was in the bundle (`d76fee096`) and was deleted from `refs/archive` on 2026-11-15 after the gate passed.

## Remote branches on `rrflow/rrflow-development` (checked 2026-11-15)

The archive covered local refs only. The remote also holds 37 branches. 19 appear above. The rest:

| Branch | Verdict | Evidence |
|---|---|---|
| 13 branches with no commit that `main` lacks (`as-built-baseline`, `ci-test-selectors`, `core-runtime-split`, `core-trace-split`, `lab-benchmarks`, `lab-datasets`, `lab-definitions-20261113`, `lab-evals`, `lab-tool`, `lsm-segment-split`, `qualification-split`, `store-backup-split`, `vector-hnsw-split`) | Contained in `main` | `git rev-list development/main..<branch>` is 0 for each |
| `agent/c07-windows-characterization-evidence` | Content on `main`; only a merge commit differs | `c-07g-windows-characterization-terminal.md` exists on `main` |
| `agent/poam-flush-stalls` | Content on `main`; merge commits only | `gate-e/e-04-flush-stall-record.md` exists on `main` |
| `copilot/fix-installed-domain-crash` | **Superseded** | Retry on a busy `hdiutil create`; `main`'s `run_native_capacity.py` has its own, wider `is_busy_darwin_create` retry (five attempts, partial-image cleanup) |
| `gh-readonly-queue/main/pr-285-…` | Merge-queue scratch ref for PRs #285 and #287 | `docs/guides/getting-started.md` (from #287) is on `main`; #285's content not re-proven |

Remaining remote refs checked 2026-11-15 (second pass):

| Ref | Verdict | Evidence |
|---|---|---|
| `gh-readonly-queue/main/pr-285`, `-349`, `-367`, `-376`, `-382` | Contained | Merge-queue scratch refs. `git log --no-merges main..<ref>` lists nothing: every non-merge commit is on `main`, and `b5821ac88` (the #285 NEON repair) is an ancestor of `main` |
| `origin/backup/agent/c07-backup-scheduling`, `c07-run-progress`, `rrflowdb-postgres-identity`, `rrflowdb-postgres-plan` | Contained | Each file the branch adds (`c-07g-integrated-native-growth.md`, the three `rrflowdb-postgresql-*` journals, `docs/reference/protocol/postgresql.md`) exists on `main`; `c07-run-progress` has no file change |
| `origin/codex/consolidate-project-structure-and-context` | **Superseded** | 2026-09-02 design from before the RRFlow name: `scripts/project_shape.py` and a consolidation contract. `main` derives inventory from Git and enforces shape with `scripts/ci/check_code_shape.py`; the contract's one-engine flow is owned by the README and ADRs |

The `origin` remote is the official repository and was only read.

Remote tips are covered before any `development` branch is deleted: 61 are ancestors of `main`, 19 are in the archive bundle, and the 3 left (`c07-windows-characterization-evidence`, `poam-flush-stalls`, `copilot/fix-installed-domain-crash`) are in `D:\localdev\devlabsrflow-remote-tips-20261115.bundle`, verified complete.

## `wip/rrflow-workbench-capture` (absorbed 2026-11-15)

Not source. Its commit `eb27cfa18` adds five files to `6448a4f` under `.rrflow`: `instance.toml` (id `rrflow-workbench-capture`), a manifest, `CURRENT`, an empty `MANIFEST.LOCK` and one 3.4 MB preallocated WAL. Reading the WAL shows a single record: the manual assertion `crate:connectome-ui` / `binary` / "connectome" built from the original Workbench crate by `operator:capture-seed`, plus its invocation record. The data is that one fact in a pre-release store format that today's engine does not read and that pre-release policy does not migrate. The ref stays in the bundle at `eb27cfa18`.

## Still open

Nothing. All 81 archived refs are absorbed, superseded or dropped, each with evidence above.
