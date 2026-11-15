# Second-mind program: owner direction and capability ledger

**Status:** historical; superseded on 2026-11-13 by the [kernel contract](kernel-contract.md), which carries every package listed here; kept as the record of the work and decisions it describes
**Superseded by:** [RRFlow kernel contract](kernel-contract.md)
**Coordinate:** `rrflow://rrflow-instance/data/work-package/second-mind-program`
**Owner:** records the repository owner's direction of 2026-11-10 and orders the work; the [roadmap](rrflow-1.0.md) and [POA&M](../poam/rrflow-1.0-alpha.md) keep status authority
**Verified against:** engine `main` at `2d2fba85` and Connectome `main` after #46, on 2026-11-10

## The rule that orders everything

The engine has to be real before anything is compared: "if I am not ready yet since my multi modal engine is not true or real yet we need to get this on track and then compare." Work that needs no engine, such as code structure, can run alongside it.

## The product, in the owner's terms

RRFlow, LFG and SWAG work "truly as one tuned engine", with Connectome as a true second brain. A person can see the reasoning as a live visual model and trace through it. The AI can guide a person through any application as if sitting at the computer. Efficiency is proven by serving knowledge from the database instead of spending tokens. Nothing is hardcoded unless it is best practice or "what a senior AI engineer would do".

## Capability ledger

"Exists" means found in the code at the verified revision. "Missing" means no implementation was found.

### Engine

| Requirement | Exists | Missing |
|---|---|---|
| A true multi-model engine with Postgres capability | Atomic multi-model commits, schema and uniqueness enforcement, time travel, crash recovery on a live run | Per-row transaction conflicts, relationship delete rules, SQL, the wire protocol ([H-08](../reference/protocol/postgresql.md)) |
| LSM hot, rrflowCLD cold | Parquet writer, default-on on a branch awaiting landing ([tiered persistence](../reference/storage/tiered-persistence.md)) | Compaction into CLD, LSM reclaim, merged reads, backups |
| LSM optimized "to truly compete with fjall" | The custom rrflowKV LSM | A measured comparison against Fjall, and the optimization it calls for |
| CLD "truly tailored for AI", breaking from Parquet to go "truly custom like the rest of the multimodal engine" | The stock Parquet writer as a starting point | The custom format and its design research |
| A working prototype competitive with SurrealDB and Qdrant, now | The dogfood harness for SurrealDB and ArangoDB; earlier Qdrant search work | A current head-to-head on the same project and queries |
| Plans driven by evidence, not fixed steps | Capability catalogue and successor plans in #326 | [D-02](d02-evidence-driven-attunement-engineering-plan.md) packages P2 onward; project understanding starts at P3 |

### Connectome structure

| Requirement | Exists | Missing |
|---|---|---|
| Platform-standard names, modular, colocated | `src/features/runtime` and an import-direction gate (#46) | `pages/`, `views/`, top-level `hooks`, `shared/util`, large Rust files |

### Second mind and honesty

| Requirement | Exists | Missing |
|---|---|---|
| A living visualizer that is the second brain running, drilled down, zoomed and expanded to a full viewport inside Connectome, on bleeding-edge research | A Sigma graph of stored records with zoom and expand-to-window; a live change stream in the client that no view subscribes to | A graph that updates live, drill-down through levels of detail, a model of reasoning, and the research |
| The second brain persists while project instances are stopped and started | One project maps to one engram and one instance; nothing sits above them | A long-lived brain service and its contract with project instances |
| Two projects need not share RBAC, with stronger leakage measures once this is proven | Per-engram sessions and authorization | RBAC across a shared brain, then researched leakage controls |
| Requests, chat and "even my rage" interpreted as constructional data | LFG's prompt hook journal; 743 of the owner's real prompts in `eval/inputs/staging` | Interpretation that feeds plans |
| Thresholds "where honesty trumps moving forward blindly" | A hook passes a frustration reading to the session | Measured thresholds and a required realignment step |
| A lexicon that grows, from layman words to specific analyses | Nothing | The lexicon, its analyses and its growth |
| Real industry signs of reward hacking | Repository rules against hiding failures | A researched catalogue and automatic checks |

### Measurement

| Requirement | Exists | Missing |
|---|---|---|
| See how it works across reasoning efforts, with token use tracked | Token and effort fields in the evaluation lab and the endpoint worker | Comparison runs across storage settings; a friendly Connectome view |
| Consider that "cold storage may never be needed" | Nothing measures it | The experiment below |
| Compete with "claude+jevdecisions" and "codex+chef flash" | A dogfood harness: SurrealDB 11/13, ArangoDB 12/13, RRFlow 9/13 by default and 10/13 at maximum limits, at `ae01ce32`, grading one project's loading and query | Any reasoning comparison against the named systems |

The storage experiment does not reverse the decision that CLD is canonical once compaction moves data into it. It measures what the tier buys: answer quality, tokens, latency and disk, with the tier on and off, at each reasoning effort.

### Developer tools

| Requirement | Exists | Missing |
|---|---|---|
| True dev tools monitoring RRFlow, LFG and SWAG and how fluidly they work together | Engine diagnostics; Connectome's Storage and Diagnostics views | Any LFG or SWAG monitoring in Connectome; one shared timeline |
| Track the hops for deep reasoning | Prompt flights with ordered, timed stages. The Workbench visual states it shows event order, "not inferred MCP hops or model decisions" | Recorded hops with identities, durations and tokens across all three systems |

### MCPL guided interface

| Requirement | Exists | Missing |
|---|---|---|
| Scan any app, Python or TypeScript or React, to find every input and its expected action | `data-guide` anchors placed by hand, checked by `scripts/check-guides.mjs` | The scanner |
| A library of A2UI and a fabric of signals and hooks, part database machine language, part AI-populated | The MCPL lane and authoring panels for Did You Know, Knowledge Drop and Factoid events. Design in Connectome parking lot 008, status PROPOSED | The approved schema, the signal fabric, AI population |
| AI moves the cursor naturally, collects layman context, types the real input and submits | One guide of five steps that waits for the person's own actions | A cursor driver, input entry, and a consent boundary for submitting |
| Show extreme token efficiency from factoids and Knowledge Drops served by the database | Events authored as templates | Serving them from RAG, and measured savings |

### Knowledge, connectors and import

These follow once the system "actually proves to all work".

| Requirement | Exists | Missing |
|---|---|---|
| Knowledge paths with connectors clearly connected and relationships built | Persisted [knowledge trees](../reference/knowledge-trees.md) in the engine; external providers unqualified | Connectome's editor, which PR #10 had and was never rebuilt; the connector contract |
| Import from ArangoDB, SurrealDB, MongoDB, Atlassian "and the rest" | An inherited CSV, JSON and NDJSON import dialog | Every source importer. The engine's adapters today are only CLI, edge and MCP |

### Harness, workbench and UI library

| Requirement | Exists | Missing |
|---|---|---|
| Import a new workbench or harness from outside, with visuals | Workbench lanes, template import from files, the evaluation lab and signed definition packs | Importing an external harness |
| AI runs and customizes those tests | Checklist generation that claims only what it checked | AI-run customized tests, within the owner's approval rule for model calls |
| A vast UI library, truly hyper-visual, fast and low on resources | 55 shared components and visual baselines | A catalogue, and render and memory budgets. No frame or size budget was found |

## Carried over from the branch audit

Of 53 Connectome branches, 48 are verified on main. Work still owed:

- **Knowledge trees and SSH reconnect** from PR #10. The owner decided on 2026-10-24 to rebuild the trees on main, and it has not happened. About 1,760 lines remain on `agent/rrflow-native-connectome`.
- **View registry**, Sprint 009 step F4b. The only draft is unpushed, on one machine, and used the notice approach the owner rejected on 2026-10-28.

## Starting points for the lexicon

Proposed for review, drawn from failures in this project. Each phrase triggers a check rather than an apology.

| The person says | Check |
|---|---|
| "cheerleading" | Does each success claim in the last messages name a run, a file or a number that exists? |
| "hand waving" | Was the behavior run, or does only a type, mock or document show it? |
| "test loop" | Have tests been edited or rerun repeatedly with no product behavior changing? |
| "fake", "not real" | Run it through the public interface on real data and read the result with an independent tool. |
| "hardcoded" | Does this value come from evidence or configuration, or is it a literal? |
| "reward hacking" | Were tests deleted, skipped or weakened, fixtures special-cased, expected values copied from output, or merges left empty? |

## Order of work

Each step ends with a run someone can repeat.

1. **Land what is ready.** #326, the merge alarm (#327 and Connectome #47), rrflowCLD and its Connectome card. Exit: main carries them and a live failure raises the alarm.
2. **Make the engine real.** rrflowCLD reclaim and merged reads, per-row transactions, relationship rules, SQL, then Postgres. Exit: an independent client exercises each on a real project.
3. **Fix Connectome,** alongside step 2. Finish its structure, then start the living visualizer on the existing graph and change stream. Exit: the architecture gate covers all of `src`, and a change committed to the engine appears on the graph without a reload. Re-authoring the inherited views waits, by the owner's decision of 2026-11-11, and the ten modules nothing imports are kept. When that work resumes, each view must be at least as capable as the SurrealDB-era client it replaces.
4. **Instrument before comparing.** Hops, tokens and one timeline across RRFlow, LFG and SWAG. Exit: one prompt can be traced end to end.
5. **Finish evidence-driven reasoning,** D-02 P2 to P5. Exit: the real-run ledger in that plan.
6. **Compare.** Efforts, storage settings and the named competitors, on real projects. Exit: published numbers, including losses.
7. **Build the honesty layer** on the 743 prompts, and measure it. Exit: scored against what the owner actually meant.
8. **Build MCPL guidance.** Scanner, then anchors, then cursor, then AI-entered input. Exit: guiding a real app the system was not built around.
9. **Build the second-mind visuals and UI library,** under render and memory budgets. Research comes first.
10. **Add connectors, importers and external harnesses.**

## Open for the owner

- What are "jevdecisions" and "chef flash"? Earlier prompts place jevdecisions as an alternative to SWAG. Neither could be identified, and the comparison needs them.
- Does Connectome parking lot 008 stand as the A2UI design?
- Atlassian: Jira, Confluence or both?
