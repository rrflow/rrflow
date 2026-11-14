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

## Kept in the archive deliberately

| Ref | Why |
|---|---|
| `wip/rrflow-workbench-capture` | Not source: a 3.4 MB engine data store from the session that captured the original Workbench at `6448a4f`, the data behind those baseline screenshots |

## Still open

Every other ref under `refs/archive/20261114/`. Each gets its own section above when it is resolved.
