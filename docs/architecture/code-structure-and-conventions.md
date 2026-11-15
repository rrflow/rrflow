# Code structure and conventions

**Status:** active proposed standard; awaiting the owner's approval before any file moves; enforcement and moves are kernel packages K12 and K13
**Coordinate:** `rrflow://rrflow-instance/data/architecture/code-structure-and-conventions`
**Owner:** the canonical folder layout, naming, file-size, module, error, schema and dependency rules for the engine workspace and Connectome, and how each rule is enforced
**Measured at:** engine `main` `297bd98f`, Connectome `19ee075c`, 2026-11-13

The owner's words of 2026-11-13: "files that are far too long need to be clearly refactored", "establish canonical patterns", folders and files that "reflect platform industry terminology", Rust "orthogonal and based off existing patterns", the UI on "arktype" with "schemas based off the engine", and code aligned "to ensure it never drifts or comes back again". Every rule below is enforced by a check that fails CI. A rule without a check is not a rule.

## 1. What exists today

| Area | Measured |
|---|---|
| Engine size | 30 crates, 787 `.rs` files, about 204k lines |
| Long engine files | 61 non-test files over 1,000 lines and 147 over 500. The largest are `rrd-contract/src/lib.rs` (7,669 lines, 261 public types, an 850-line `endpoint_catalogue()`), `rrd-lsm/src/database.rs` (3,056), `rrd-core/src/runtime.rs` (2,825) and `rrd-lsm/src/segment/mod.rs` (2,802) |
| Engine lint and layout rules | No `clippy.toml`, `rustfmt.toml`, `deny.toml` or `[workspace.lints]`; `too_many_arguments` allowed 132 times. `workspace_architecture.rs` enforces the package-to-path map, dependency sets for 7 crates, forbidden edges and vocabulary, but nothing on file size, module style, errors or tier order |
| Engine module, error and naming patterns | Both `mod.rs` and file-named modules, plus 23 `#[path]` attributes. Errors are hand-written in five styles, with about 125 `Result<_, String>` and 109 `Box<dyn Error>` signatures. About 35 SHA-256 validators and 13 hand-written hex encoders sit beside `rrd-core::digest`. "engram" appears 3,100 times and "engram" 115, with miscased renames such as `InstalledengramRecord` |
| Engine layering | Group directories do not match dependency tiers: `transport/` holds both the lowest crate (`rrd-contract`) and the highest (`rrd-server`). `rrd-maintenance` and `rrd-kubernetes` have no consumers |
| Connectome size | 406 TS/TSX files (67,686 lines) and 41 Rust files (18,064 lines) |
| Connectome layout | Five organizing schemes at once: layer folders, `screens/{pages,views}`, a half-built `shell/shared/spokes`, the `rrflow/` client, and workspace areas. Modals live in 3 places, stores in 2, components in 3 and connection logic in 4. 45 imports break the written spoke rules |
| Long Connectome files | 26 product files over 500 lines (`workbenchLanes.tsx` 1,343, `quantization.tsx` 1,253, `session.ts` 1,196); Rust `view.rs` 1,733 |
| Connectome schemas | arktype used once and valibot twice, both on inherited code. 84 IPC view types are hand-written copies of Rust structs. Configuration loads with no schema check. 15 `JSON.parse` calls are unvalidated |
| Connectome enforcement | Ten `pnpm check` scripts; the only import gate is RRFlow isolation. No file-size or naming rule |

## 2. Engine rules

| Id | Rule | Industry basis | Enforced by |
|---|---|---|---|
| R1 | A non-test source file has at most 800 lines; a test file at most 1,500 | file-size budgets in large Rust codebases; a reviewable unit | `scripts/ci/check_code_shape.py` with a recorded **ratchet**: every file over the limit today is listed with its current length. A listed file may only shrink, and it leaves the list when under the limit. A new file over the limit fails |
| R2 | `lib.rs` and `main.rs` hold only module declarations, re-exports and the entry point, at most 200 lines | the facade pattern of tokio, serde and the Rust API guidelines | the same check |
| R3 | File-named modules (`foo.rs` with `foo/`). No new `mod.rs` and no `#[path]` | Rust 2018 module idiom | the same check, with the 10 `mod.rs` files and 23 `#[path]` attributes on the ratchet list |
| R4 | Crates form declared tiers, and a crate depends only on lower tiers | layered architecture, as in rustc's own crate graph | one tier table in `workspace_architecture.rs`; every edge checked, not just 7 crates |
| R5 | One `Error` enum per library crate in `error.rs` (`thiserror`), with `pub type Result<T>`. No `Result<_, String>` or `Box<dyn Error>` in library crates | Rust API guidelines C-GOOD-ERR | a source check on the ratchet: counts may only fall |
| R6 | One digest type: `Sha256Digest` (validated lowercase hex), with one encoder and one validator, in a tier-0 crate both `rrd-core` and `rrd-contract` use | one owner per concept | the check refuses `Sha256::new` and hex encoders outside that crate |
| R7 | Configuration naming: `Options` for constructor inputs, `Settings` only for S1 settings, `Policy` for rules, `Limits` for caps. One `open(path, Options)` instead of `open_with_*` and `create_with_*` fan-out. More than three optional inputs means an options struct | builder and options patterns of the API guidelines | `too_many_arguments` stops being allowed: its 132 escapes go on the ratchet |
| R8 | `[workspace.lints]`: `unsafe_code` denied outside named crates, `clippy::undocumented_unsafe_blocks`, `unreachable_pub`, `missing_debug_implementations`; `cargo-deny` for licences and advisories | standard workspace hygiene | CI clippy and `cargo deny check` |
| R9 | Tests: unit tests in a sibling `tests.rs` (`#[cfg(test)] mod tests;`); integration and public-API tests in `tests/`. `rrd-engine/src/engine/tests/` (42.8k lines inside the crate) moves to `tests/` wherever it uses the public API | Cargo conventions | the shape check |
| R10 | Names: `rrd-*` internal, `rrflow-*` outward (existing rule). Types carry no crate prefix except the facade (`RrdEngine`, `RrdClient`). The engram-to-engram rename is finished with correct casing, and its shims are removed (POAM-078) | consistent vocabulary | a vocabulary check like the existing retired-name bans |
| R11 | No unconsumed crates or lab code in production crates: `rrd-maintenance` is removed and `physical_policy_lab.rs` moves to the Lab. `rrd-cluster`, `rrd-kubernetes` and the pgvector adapter are product scope (owner, 2026-11-13). They serve the shippin panel and cloud synchronization, so they move to the cloud tier and gain real consumers and tests (kernel package K15) | one current implementation | the architecture test lists every crate with a consumer or an owner decision |

**Target tiers** (directories follow tiers after the moves):

| Tier | Directory | Crates |
|---|---|---|
| 0 | `crates/foundation/` | digest and id primitives (new), `rrd-contract`, `rrd-core` |
| 1 | `crates/storage/` | `rrd-lsm`, `rrd-store` (rrflowKV and rrflowCLD) |
| 2 | `crates/compute/` | `rrd-query`, `rrd-vector`, `rrd-inference`, `rrd-attunement` |
| 3 | `crates/governance/` | `rrd-security`, `rrd-engram` (renamed to engram), `rrd-observation` |
| 4 | `crates/authority/` | `rrd-engine` |
| 5 | `crates/interfaces/` | `rrd-server`, `rrd-client`, `rrflow-cli`, `rrflow-mcp`, `rrflow-edge` |
| 5 | `crates/cloud/` | `rrd-cluster` (replication and synchronization), `rrd-kubernetes` (operator for the shippin.cloud deployment), the pgvector adapter. Feature-gated and optional: never required for local readiness, and consuming only public engine contracts |
| — | `crates/evaluation/` | the Lab and qualification, dev-only, as today |

**First splits**, one PR each, with behavior unchanged:

1. **`rrd-contract/src/lib.rs` → about 15 modules:**
   - `protocol` (constants, limits)
   - `query`
   - `vector`
   - `retrieval`
   - `points`
   - `changefeed`
   - `backup`
   - `audit`
   - `catalogue` (with `endpoint_catalogue()` split per domain)
   - `engram_authority`
   - `error`
   - `identity`
   - `deployment`
   - `session`
   - `schema_values`
   - `transaction`
   - `commit`

   Proof: the OpenAPI digest constant is unchanged, every SDK projection regenerates with no diff, and all `rrd-contract` tests pass untouched.
2. **`rrd-lsm/src/database.rs`** → `database/{options, failpoints, open, transactions, reads, compaction, gc, snapshot}.rs`.
3. **`rrd-core/src/runtime.rs`**, **`rrd-lsm/src/segment/mod.rs`**, **`rrd-core/src/trace.rs`**, **`rrd-engine/src/engine/installation.rs`**, **`rrd-store/src/backup.rs`**, **`rrd-store/src/rrflow_kv.rs`** and **`rrd-vector/src/hnsw.rs`**, by the split points recorded in the 2026-11-13 survey.
4. The remaining files on the ratchet list, largest first.

## 3. Connectome rules

**Target layout.** This builds on the 2026-09 proposal (parking lot 004, option 2B), which was never committed, and replaces the five schemes:

```
src/
  app/                 entry, providers, router setup, shell layout
  routes/              TanStack file-based routes: thin, compose features only
  features/<feature>/  components/  hooks/  schemas/  services/  state/  index.ts
  devkit/              dev component library: inspectors, metric and leak panels, trace viewer,
                       component registry and gallery (MCPL concept applied to dev UI)
  shared/ui/           primitives, theme, tokens (teal and burnt orange)
  shared/lib/          pure utilities only
  contracts/generated/ arktype schemas and types generated from the engine OpenAPI
  contracts/ipc/       types generated from the Tauri Rust structs (ts-rs or specta)
  platform/            tauri and browser adapters
src-tauri/src/
  main.rs (thin)  lib.rs  commands/  application/  adapters/{rrd,config,filesystem,process}  ipc/
```

**Features:**
- `workspace`: projects, instances, pods and lanes;
- `connections`;
- `knowledge`: graph, schema, records, vectors;
- `reasoning`: tasks, plans, traces, the reasoning graph;
- `second-brain`;
- `lab`: systems, datasets, benchmarks, experiments, reports;
- `settings`;
- `guides`;
- `diagnostics`;
- `query`: the inherited editor, re-authored.

| Id | Rule | Enforced by |
|---|---|---|
| C1 | Dependencies run one way: `routes` → `features` → `shared`, `contracts`, `platform`. A feature imports another feature only through its `index.ts`. `shared` never imports a feature | an import-boundary check (dependency-cruiser) in `pnpm check` |
| C2 | arktype is the only runtime schema library. Engine schemas are generated from the OpenAPI document, IPC schemas from Rust. Every `JSON.parse`, configuration load and import is validated. valibot is removed with the inherited Docker code | a check that refuses other schema libraries and unvalidated `JSON.parse` |
| C3 | The configuration store has one arktype schema with versioned migrations. A load that fails validation is reported, never silently patched | a test that loads every historical fixture |
| C4 | TS and TSX files at most 400 lines; Tauri Rust follows R1 | the shape check with a ratchet, as in R1 |
| C5 | Names: feature folders in kebab-case; components `PascalCase.tsx`; hooks `useThing.ts`; schemas `thing.schema.ts`; one component per file | biome `useFilenamingConvention` and the shape check |
| C6 | No screen is deleted before its replacement reproduces it (`tests/visual/PARITY.md`) | the existing visual capture and compare |
| C7 | Every setting is rendered from its schema: no hand-written settings form | the settings feature reads only generated S1 schemas |
| C8 | The dev UI shows the truth: each instance and lane shows RSS, memory events, handles against the limit, threads, cache bytes, queue depth, queue wait p95, rejections and pressure stall, and flags any value still rising after warmup | devkit panels fed by engine diagnostics; a leak soak test |

**Configuration categories** (Connectome settings, mapped to S1):

1. Workspace and projects: serving, instances, pods, lanes, credential mapping.
2. Connections: profiles, templates, authentication.
3. Engine: generated from S1 (storage, planner, vector, lexical, graph, reasoning effort, LFG, System One, endpoints).
4. Lab: systems, runs, templates, linked paths.
5. Appearance: theme, scale, sidebar, motion.
6. Editor and query.
7. Visualization: graph and diagram defaults.
8. Input and commands: keybindings.
9. Diagnostics and updates.
10. Experimental: feature flags.

Session recovery state is not configuration and moves out of the settings tree.

**First moves**, one PR each:

1. Enforcement with ratchets in both repositories, so nothing gets worse from the first merge.
2. `contracts/generated` with arktype schemas from the engine; the configuration schema and migrations; the engine client repinned to `main`.
3. TanStack file routes beside wouter, then one feature per PR in workspace-area order. Each moved file is split under the limit, with visual parity captured before and compared after:
   - `workbenchLanes.tsx` → `features/lab/`;
   - `quantization.tsx` → `features/knowledge/vectors/`;
   - `session.ts` → `contracts/ipc` plus `platform/tauri`.
4. Inherited views re-authored or retired under C6.

## 4. Pods and lanes

This implements S6 of the [kernel contract](../roadmap/kernel-contract.md#36-s6-project-engram-instance-and-pod), using the 2026-11-13 research and the [SurrealDB multi-project probe](../evidence/surreal-projects-20261113/README.md).

**Lab pod** (`eval/lab/pod.sh`, new on 2026-11-11):
- RRFlow runs as a sibling container under the same limits as every other system.
- Servers get cpuset and memory limits, but no CPU quota, which is measured to inflate tail latency.
- Build is separated from measurement by a quiet window, judged by pressure stall.
- Warmup is not scored; at least 5 interleaved repetitions run, reported as a median with a confidence interval.
- A run whose spread exceeds 5% is marked "inconclusive".
- Pressure stall and host load are recorded every second, including Windows-side CPU.
- `.wslconfig` is pinned and recorded, `autoMemoryReclaim` is off, and the VM clock is checked against the host.
- Every resource is labelled for reaper-style cleanup of orphaned containers.
- A socket proxy limits the runner to the lab's own containers.

**Instances and lanes:**
- One process or container per project, never a shared namespace (that is the probe's result).
- Per-instance CPU weight, memory high and max, I/O weight and a pids cap.
- Background compaction and indexing run at idle priority.
- Caches are per instance.
- Lanes, with defaults:
  - `count` 4;
  - `max_in_flight` 2 per lane;
  - `queue_depth` 16;
  - `queue_timeout_ms` 2,000;
  - deficit round-robin between lanes;
  - a refusal when a lane is full.
- Enforcement: cgroups v2 on Linux, Job Objects on Windows.

**Accepted when:**
- Another project's write storm raises a project's p95 by at most 1.25× across 10 interleaved repetitions.
- Jain's fairness index across equal lanes is at least 0.9.
- Canary records never cross projects.
- A soak of 30 minutes or more shows memory, handles and threads levelling off after warmup and returning to baseline after a lane closes.

## 5. Owner decisions

1. **Approve this structure** as written, or with changes. No file moves before approval.
2. **`rrd-cluster`, `rrd-kubernetes` and `pgvector`:** decided 2026-11-13. They are product scope for the shippin panel and for synchronizing with the cloud offering, comparable to SurrealDB Cloud or ArangoDB's managed service and backed by shippinAI. They move to `crates/cloud/` under K15.
3. **Limits:** 800 lines for Rust and 400 for TS/TSX are the proposed caps. The default is these values.
