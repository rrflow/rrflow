# RRFlow as-built baseline

**Status:** active evidence record of what exists on 2026-11-13; the owner marks each row Keep, Change or Remove
**Coordinate:** `rrflow://rrflow-instance/data/architecture/as-built-baseline`
**Owner:** current-state inventory against the owner's product vision; it changes no plan or status by itself
**Revisions read:** engine `main` at `297bd98f`; Connectome submodule at `19ee075c`

This record states what the code does today, so the owner can decide what
stays and what changes before more is built. It was produced by reading the
code, not by running it, except where a row says it was run. Each row has a
status:

- **Runs:** implemented and called on a public path; the caller is named.
- **Unwired:** code exists but no production path calls it.
- **Docs only:** described in a record, absent from code.

PR [#376](https://github.com/rrflow/rrflow-development/pull/376) (not merged
on this date) changes some rows; they say so.

## The owner's vision, in the owner's words

- SWAG is the System One controller, the decision router; it works with the
  semantic router. LFG is the CPU-burstable classifier that feeds SWAG.
- Whether the stack is Claude + jevdecisions, Codex + chef flash, SWAG + LFG or
  a custom one, it is bound through configured endpoints, never hardcoded.
- RRFlow combined with a quality LLM produces faster and higher-quality
  answers than those stacks; speed alone is not the goal.
- Connectome shows a unique neural visual, built on pre-made SVG art, with
  dynamic interactive modals for the second brain and the active instances;
  the second brain has its own neural styles.
- The UI shows the latest graph-reasoning results and what dynamically
  triggered rrflowKV, rrflowLSM and rrflowCLD for a request.
- True comparisons can be seen and run, then tuned.

## 1. Model endpoints and routing

| Area | Today | Evidence | Status | Decision |
|---|---|---|---|---|
| Endpoint bindings | Configured through `endpoint-configuration-read/preview/apply` (`/v1/configuration/endpoints/*`). A binding has id, workload class, kind, `in_process` or `url`, model id, credential reference, placement and limits. No shipped default profile; no CLI or file setup. | `rrd-contract/src/endpoint_binding.rs:195-212`; `rrd-engine/src/engine/endpoint_configuration.rs:23-50` | Runs | |
| Binding fields for routing | No protocol, capability, price, latency, health or locality fields; the routing policy in `lfg-swag.md` cannot be expressed. | `endpoint_binding.rs`; `docs/reference/inference/lfg-swag.md:45,247-256` | Docs only | |
| Model protocols | One: OpenAI-style `POST {url}/chat/completions` with a Bearer key. No Anthropic Messages, no MCP-as-model, no in-process LLM. | `rrd-inference/src/endpoint_chat.rs:213-226,303,342` | Runs | |
| Worker adapter choice | `openai_chat_completions_v1`, `swag_…`, `lfg_…`, `other` exist, but the engine never reads the field, so every call is sent as OpenAI chat; `lfg-swag.md:258-263` says an unknown adapter fails closed. | `rrd-contract/src/endpoint_worker.rs:18-28,50` (no caller of `is_native_chat_adapter`, verified) | Unwired | |
| SWAG | No code is named SWAG. "SWAG" is whichever binding the selection policy names as `controller_binding_id`; `task-endpoint-propose` sends it the task and candidates as JSON chat and records its choice. | `rrd-engine/src/engine/endpoint_proposal.rs:111-570,621-644` | Runs | |
| Semantic router | Only a ranked candidate list the caller supplies in the request; nothing produces it. Likely defect: a list that removes candidates fails packet validation. | `rrd-contract/src/endpoint_proposal.rs:83-178,376-386`; `engine/endpoint_proposal.rs:187-191` | Docs only (input type exists) | |
| LFG | Called only during endpoint selection, through a loopback-only URL inside the selection policy, not a configurable binding. | `rrd-inference/src/lfg_transport.rs:120-131,177-180`; `endpoint_policy.rs:15-47` | Runs (loopback only) | |
| Route decisions, reasoning-tree router | Types and validation only; nothing in the engine or server uses them. | `rrd-contract/src/router.rs:33`; `rrd-core/src/reasoning_tree.rs` | Unwired | |
| External stacks (jevdecisions, chef flash, Claude, Codex) | Not bindable. Claude, Codex and Gemini appear only as CLI subprocesses inside the Lab. | `crates/evaluation/lab/rrflow-lab-evals/src/provider.rs:16,46,120,284` | Docs only | |
| Hardcoded values | No provider host or model name in production Rust. Fixed: `/chat/completions`, Bearer, LFG loopback and path, port 9477, and Lab provider branches. | as cited above; `rrflow-cli/src/command.rs:51` | Runs | |
| Stale records | `system-overview.md:84-85` calls LFG and SWAG execution unimplemented; `lfg-swag.md:3` calls it an unmerged draft. Both are on main. | as cited | Docs drift | |

## 2. Question to answer, and answer quality

| Area | Today | Evidence | Status | Decision |
|---|---|---|---|---|
| Question → context → LLM → answer | Only through a task: open, configure budget, `task-endpoint-propose`, then `task-worker-execute` with `AssembleContext`. Output is draft text. A raw question cannot be asked. | `engine/endpoint_worker.rs:368-518` | Runs | |
| Real model with context | Never run. The one real-model receipt (BF16 SWAG model) had `context: null`; tests use synthetic loopback servers. | `docs/evidence/change-journals/gate-g/task-worker-native-receipts/available/native-worker.json` | Not run | |
| Prompt flights | Assemble context and record events; no model is called (`Observe` only). | `rrd-contract/src/flight.rs:48-50` | Runs | |
| MCP and edge adapters | `rrflow-mcp` exposes one read-only tool, `rrflow_context`; `rrflow-edge` is an offline feature-hash index. Nothing exposes RRFlow as an OpenAI- or Anthropic-compatible endpoint. | `crates/adapters/rrflow-mcp/src/main.rs:105-145` | Runs | |
| Answer-quality measurement | None. The Lab measures vector recall and file recall@10/50; no nDCG, no answer grading. | `rrflow-lab-benchmarks/src/driver.rs:97,363-395`; `eval/lab/retrieval_signal.py:153-276` | Docs only | |

## 3. What a request triggers, and whether it is visible

| Area | Today | Evidence | Status | Decision |
|---|---|---|---|---|
| rrflowQL path choice | Fixed rules: identity, range, adjacency and traversal take their own path; otherwise a ready index by leading-field match; else a versioned read or DataFusion. No cost model. No vector operator (`NEAR` is plan item J5). | `rrd-query/src/plan.rs:292-341,440-490,651-684` | Runs | |
| Context assembly | Not yet dynamic, contrary to the design. It runs the same stages every time (seed, BM25, exact vector if a local embedder exists, graph BFS, fusion with fixed weights) over the whole snapshot, and on main rebuilds BM25 per request; #376 reads persistent postings instead. The planned dynamic path is J6 (one planner for context and rrflowQL) and J11 (cycles that grow with effort until evidence stops improving) in the [one-engine joins plan](../roadmap/one-engine-joins-engineering-plan.md). | `rrd-engine/src/engine/context.rs:10-14,104,215,294,333` | Runs | |
| Vector search | The only path chosen per request today: exact scan, HNSW or TurboQuant by a calibrated cost estimate. The owner's rule is selection by reasoning need, not cheapest; the planner does not yet take a quality target. | `rrd-vector/src/plan.rs:111-210`; `cost.rs:55-65` | Runs | |
| Hybrid retrieval and retrieval query | Defined, called only from tests. | `engine/retrieval.rs:17`; `engine/retrieval_query.rs:40` | Unwired | |
| rrflowLSM | `rrd-lsm` (WAL, memtable, segments, manifest); compaction constants are fixed (POAM-077). rrflowKV is the store over it. | `crates/persistence/rrd-lsm`; `rrd-store/src/rrflow_kv.rs:1-6` | Runs | |
| rrflowCLD cold tier | Not built. Cold history copies changefeed pages to Parquet, never moves hot data, and only `cold-history-read` reads it. "rrflowCLD" in code is a set of process-local hit counters. | `engine/cold_history.rs:310`; `runtime/coldlight.rs:1-60`; plan item J8 | Docs only | |
| Decision returned with the answer | Query plan with every candidate and the reason it was chosen or rejected; context stages with access path and reason; vector access path. | `rrd-contract/src/lib.rs:545-607,1859-1890`; `memory_context.rs:203-301` | Runs | |
| Decision saved as a trace | Only prompt flights save their context stages. Spans across QL, KV and vector exist only in test-called methods. No hop tracing, no metrics endpoint, no per-layer timing. | `engine/flight.rs:892-968`; `runtime/query.rs:70`; plan items J1, J12 | Unwired | |

## 4. Reasoning graphs and latest results

| Area | Today | Evidence | Status | Decision |
|---|---|---|---|---|
| Stored reasoning records | Prompt flights with `-used` edges to their context, tasks, budgets, attempts, assessments and knowledge trees, each read by id or page. | `engine/flight.rs:1-16`; `engine/knowledge.rs:38` | Runs | |
| Reasoning tree | Types only. | `rrd-core/src/reasoning_tree.rs:140,403` | Unwired | |
| "Latest reasoning result" read | None; repeated-cycle reasoning is plan item J11. | `docs/roadmap/one-engine-joins-engineering-plan.md` | Docs only | |

## 5. Second brain and active instances

| Area | Today | Evidence | Status | Decision |
|---|---|---|---|---|
| Project engram | Installation writes one per project. | `engine/installation.rs:1195` | Runs | |
| Engram planning and warp APIs | Called only from tests. | `engine/memory_engram.rs:13`; `engine/memory_engram.rs:14,45,106` | Unwired | |
| Personal and org engrams, project links, preference order run → project → user → org → core | Not built. | `docs/decisions/0005-second-brain-personal-and-org-engrams.md:16-18,53-55,73-78` | Docs only | |
| Active instances in Connectome | A project selector and a "Manage project engines" modal (status, serve, stop, up to four). | `connectome/src/components/ActionBar/projects.tsx:222` | Runs | |
| Second-brain view | None. | — | Docs only | |

## 6. Connectome visuals

| Area | Today | Evidence | Status | Decision |
|---|---|---|---|---|
| Routing | wouter, not TanStack Router; no file-based routes (verified in `package.json`). Workspace at `/runtime/:id/:section/:detail`, areas Start, Knowledge, Reasoning, System. | `connectome/src/screens/connectome/index.tsx:145-247`; `RuntimeConnection/areas.ts:25-62` | Runs | |
| Inherited SurrealDB-era views | Nine views under `/c/:connection/:view` still on the vendored SurrealDB client; `/control/:id` calls operations the engine lacks; `/diagnostics/:id` is unreachable. | `index.tsx:45-67,196-242`; `rrflow/control-plane.ts:103` | Runs / dead | |
| Graph renderer | One native renderer: Sigma + graphology + ForceAtlas2, plain discs coloured by group, used by the records graph, similarity map, Workbench preview and inventory neighbourhood. A second inherited Sigma stack and an xyflow designer remain. | `RuntimeConnection/graph.tsx:183-425`; `components/RelationGraph` | Runs | |
| SVG art | Four pictograms (one five-node teal-to-orange mark) and the app icon. No illustration library, no neural or brain art. | `src/shared/assets/pictograms/` | Runs | |
| Tokens and colour | Obsidian/slate neutrals, burnt-orange scale, teal-to-orange gradient. Mantine `violet` is aliased to orange, so two group colours render the same. | `shared/util/mantine.tsx:51-143`; `groups.ts:13,16` | Runs | |
| Animation | Six Lottie files that nothing plays; CSS keyframes only. | `constants.tsx:227-253` | Unwired | |
| Modals and popups | Six mechanisms, mostly inherited; native ones are the quantization modal and the project-engines modal. | `shell/components/App/index.tsx:39-50`; `src/modals/` | Runs | |
| Reasoning and tier activity views | None for context stages, rrflowQL plans, hops, BM25, LSM or CLD. Context-assemble is refused by the native allowlist. TurboQuant compare is the one per-request access-path view; rrflowKV shows process counters. | `src-tauri/src/rrd/transport.rs:39-43`; `tabs/quantization.tsx:657-707`; `pipeline.tsx` | Docs only | |
| Engine client version | Pinned to engine `91b6c678`, 528 commits behind `main`. | `connectome/package.json`; `src-tauri/Cargo.toml:47-48` | Runs (stale) | |
| August Workbench | Survives only as 85 reference screenshots. | `connectome/tests/visual/baseline/workbench/6448a4f/` | Docs only | |

## 7. Comparisons and tuning

| Area | Today | Evidence | Status | Decision |
|---|---|---|---|---|
| Lab | `rrflow-lab` CLI and scripts; adapters for Qdrant, SurrealDB, Milvus, custom and RRFlow; Lance through a Python shim. | `crates/evaluation/lab/rrflow-lab-systems/src/adapters/` | Runs (CLI only) | |
| ArangoDB, engine-routed RRFlow system | Not built (plan item J10). | `docs/roadmap/one-engine-joins-engineering-plan.md:104` | Docs only | |
| Run from Connectome or an engine operation | Not possible. | — | Docs only | |

## 8. Fragmentation

Connectome, from the survey above:

- Two Sigma graph stacks, two schema-graph views, two inspectors, two
  prompt-flight UIs, and the API reference twice.
- Five transport clients: the generated SDK, native `invoke`, the retired diagnostics fetch (`src/rrflow/diagnostics.ts`),
  the control-plane fetch and the inherited SurrealDB client.
- Unused components: BetaBadge, BrutalistButton, FancyRadio, FloatingButton,
  HoverIcon, PropertyValue, Tile, the Placeholder page, two never-mounted
  modals, the empty `src/spokes/rrflow` and the Lottie files.

Engine and repository:

| Finding | Evidence | Kind | Decision |
|---|---|---|---|
| About 378k lines of Rust in 30 crates, 151k of it in test paths; `rrd-engine` is 103k, of which 51k is tests. Only one binary ships, `rrflow`. | `scripts/build_rrflow.py:110,165` | Size | |
| `rrd-maintenance` has no dependents; `rrd-cluster` (15k) is reachable only behind a feature no shipped binary enables; `pgvector_postgres.rs` and `fastembed_local.rs` compile only under `--all-features`. | `docs/poam/rrflow-1.0-alpha/poam-008.md:28`; `rrd-engine/Cargo.toml:16` | Unwired | |
| The Lab is seven crates (13k lines) plus Python and shell; the agent-provider runner is written twice; `eval/lab/retrieval_signal.py` has its own BM25 and report builder; `eval/lab/rrflow_settings.py` rewrites engram TOML with regexes; install and serve are driven by hand in four places; about twelve benchmark runners live in crate `examples/`. | `lab-experiments/src/main.rs:87-101`; `lab-evals/src/provider.rs:14,284`; `run-real-project-workbench.ps1:569-631` | Duplicate | |
| One contract source but seven SDK and signal generators; `sha256_hex` re-implemented about twelve times beside `rrd-core/src/digest.rs:57`; workspace member lists maintained by hand in six places. | `scripts/ci/check_generated_surfaces.py:497-611`; `scripts/ci/workspace_packages.py` | Duplicate | |
| The legacy JSON vector segment is still a runtime artifact; the dissolved `rrflow-eval` crate is still documented in `eval/README.md` and nine other records. | `rrd-vector/src/compact.rs:3`; `runtime.rs:21,97` | Superseded | |
| Planning is spread thin: Gate D has five planning records, "one engine" has three, the Lab has four owners, alpha status spans six records, and 937 of 1,184 Markdown files are change journals. | `docs/roadmap/README.md:51-54,78-79` | Competing owners | |
| Layered, not duplicated: `rrd-attunement` with engine attunement, `rrd-inference` behind the engine, one Rust BM25, one configuration loader. | — | OK | |

## Reading this record

A decision in the last column does not change a plan by itself. Rows marked
Change or Remove become packages in the
[RRFlow 1.0 roadmap](../roadmap/rrflow-1.0.md) and gaps in the
[POA&M](../poam/rrflow-1.0-alpha.md), each accepted by a real run.
