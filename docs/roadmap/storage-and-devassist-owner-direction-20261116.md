# Storage concurrency, boundaries and DevAssist: owner direction of 2026-11-16

**Status:** active owner direction record; not an accepted decision, plan or status. The [kernel contract](kernel-contract.md) remains the one active engineering plan, and promoting any item below to a package needs the owner's decision
**Coordinate:** `rrflow://rrflow-instance/data/work-package/storage-and-devassist-owner-direction`
**Owner:** records, verbatim, what the repository owner said on 2026-11-16 about rrflowLSM concurrency, the storage, query and orchestration boundaries, preference schemas and DevAssist, and maps each part to the record that owns it or to none; the [roadmap](rrflow-1.0.md) and [POA&M](../poam/rrflow-1.0-alpha.md) keep status authority
**Verified against:** `main` at `0310f2bb7` on 2026-11-16, by reading the named files and searching `docs/` and `crates/` for the terms named in each row

This record continues the [DevWorld owner direction](devworld-owner-direction-20261116.md) of the same day. It covers what the owner said afterwards, in a planning conversation with an AI session. That conversation is not in the repository; the passages below are the only copy.

## The owner's words

Excerpts, in the order they were said. Typos are the owner's and are kept.

On rrflowLSM and Fjall:

> Fjall has an issue where concurrent access to the directory is not possible due to the API lock. This is a prime example to get serious as a true system principal architect of rrflowLSM that is supposed to be dynamic combined rrflowkv rrflowdb rrflowql rrflow cld. rrflowLSM takes fjall and completely makes it mine by optimized to not only have fjall capabilities completely captured and performing as well it is supposed to be able to be tuned and optimized for AI. [...] Rrflowlsm is built to be its true own product but tailored to be for bi Temporal reasoning the first turnkey full reasoning engine that any vibe coder.could integrate with as we.offer.the free tier of hosting for x storage.

On DevWorld and the audience:

> One optimized instance of rrflow runs where it built to be the second brain. We refer to it as DevWorld, DevU because the concept and framing we want to explore and optimize is ways to have effective rot prevention process and tasks to support a true first reasoning engine for those serious about their projects and work. Tailored for devs, solo founders, but made for vibe coders finally starting to search and want to become the true context engineer be able to actually understand fundamental and talk with confidence

On concurrency, after the session recommended one process owning each engram and rejected several writer processes:

> Wrong [...] real reasoning is constantly running dumps and dumps of catamorphism by having what ever options are available [...] there is no way one single process can successfully hit and then pool effectively and fast as hell maximizing every overpriced mb of ram

On SWAG, LFG and the component library:

> we can fold the best aspects of a judge/system one controller I.e swag where I think it can compete at a 9Gb tuned swag is the module and the llm we need to truly exist on hugging face. Lfg needs to be properly clearly defined. It is experimental. Based on a very natural and non taxing way can the user get a custom series of components library that we can build so a mix of machine based triggers and hooks trigger as a fabric for dynamic factoids knowledgedrips and didyouknow we also are smart and build minimal requirements properly defined and organized per domain or boundary. [...] LSM CLD the schemas should not be restrictions but actually just helping the correct lanes in the most rapid of ways where SWAG brings clarity rapidly

On schemas, DevAssist and memory placement:

> these schemas it's literally just global+project preferences when variant official overwrite we default to project supersede. [...] First rrflow has UI to toggle. You enable DevAssist. I have been leaving this to clyffy but we need this included it is essential for you to finally understand the true goal.of rrflow, and rrg. We know qdrant is fast as hell and probably one of if not the best for semantic BMS and turbo quant implementation from Google. We know that neo4j is slow and probably ruined now but rrflowKV was built with lessons.learned from surrealdb which they are too locked in on. We showed research backed up a dynamic engine vectordb, relationship document/persistence if multimodal and guided with rrflowLSM for to rrflow cld that takes the cold seek times and we move memories to accomodate the reality of the human mind. Focus will shift attention we will nudge passively and make clear we need to finish the task in a supportive way that overtime attunes.to that operators cadence and expressions. We.know that the real issue right now is that no one has taken the time to truly build that engine that evolves. This is more about being able to have schemas respond based off our defined formulas because there are safe assumptions once we have behavioral stateState Machine driven by telemetry.State Machine driven by telemetry, metadata,

The last sentence ends there; the message was cut off.

On the boundaries:

> we have strict boundaries for storage, query execution, database orchestration right? That is literally why we track independently the relationship QL the storage and the orchestration DB KV if we get serious the whole paraquete and fjall rrflow and paraquet get absorbed properly across our design.

On what to do next:

> Get this fixed and authored into main so I can get this actually moving forward [...] connectome should have a dev build for advancing it and optimizing it running test and comparisons generating dynamic visuals for social media And getting the DevWorld properly implemented

The owner also said the earlier RRFlow releases on an old computer and a laptop are not to be pursued: "focus on my words".

## Where each part is owned today

### Storage

| Direction | Owner today | State |
|---|---|---|
| Many reasoning runs fold results back at once; one writer at a time is rejected | [Kernel contract](kernel-contract.md#41-k10-rrflowkv-technique-queue) K10 lists WAL group commit, to adopt when "concurrent writers wait on sync". [C-07](rrflow-1.0/gate-c.md) proves reader and compaction concurrency | Open, and the code does the opposite. `Database` is documented as "Single-writer RRFlow LSM database" (`crates/persistence/rrd-lsm/src/database.rs:235`) and its write methods take `&mut self`. rrflowKV holds it as `RwLock<Database>` (`crates/persistence/rrd-store/src/rrflow_kv/types.rs:77`), so every commit waits for one lock. No record states concurrent writers as a requirement |
| A second process cannot open the same directory | No record owns it as a requirement | Open. rrflowLSM has the same rule as Fjall: opening takes an exclusive `MANIFEST.LOCK` and a second opener gets `DatabaseWriterLock` (`crates/persistence/rrd-lsm/src/manifest/open.rs:122-125`). Fjall states it in its own README: "A single database may **not** be loaded in parallel from separate *processes*" ([fjall 3.0.3](https://docs.rs/crate/fjall/3.0.3/source/README.md)) |
| Fjall's capabilities fully captured, performing as well, then tuned for AI | K10 "LSM against Fjall", and the [borrow-or-build floor check](kernel-contract.md#42-borrow-or-build) of rrflowKV point reads and writes against Fjall | Partly. The floor check measures speed only. No record lists Fjall's capabilities to capture. Three that rrflowLSM lacks, from [fjall 3.0.4](https://docs.rs/crate/fjall/=3.0.4): optimistic multi-writer serializable transactions, one LSM tree per keyspace, and optional key-value separation (queued in K10, "absent" today) |
| Parquet absorbed into rrflowCLD | K10: "versioned CLD reader (Parquet generation first, custom format later from measured access)"; [tiered persistence](../reference/storage/tiered-persistence.md) | Partly built. Scoped runtime change pages are published as Parquet, as a derived copy that reclaims nothing; F-06 owns the canonical copy, the merged read and reclamation. rrflowLSM segment values are still opaque bytes in one value column ([current format](../reference/storage/rrflowkv-current-format.md)) |
| rrflowLSM guides into rrflowCLD, which takes the cold seek times; "we move memories" to match the human mind | Kernel contract, owner decision of 2026-11-14: "CLD is a columnar copy first, not a demotion tier"; every version stays in rrflowLSM | Decided as a copy. "Move" today reads as placement that changes with use, which the 2026-11-14 decision does not cover. Needs the owner's reading (question 2) |
| Strict boundaries for storage, query execution and database orchestration | [System overview](../architecture/system-overview.md#canonical-component-terminology) names rrflowLSM, rrflowCLD, rrflowKV, rrflowDB and rrflowQL. [Code structure and conventions](../architecture/code-structure-and-conventions.md) puts `rrd-lsm` and `rrd-store` in the storage tier | Partly. Query is `rrd-query` and orchestration is `rrd-engine`. rrflowLSM is `rrd-lsm`. rrflowCLD has no storage-tier home: its writer is `crates/authority/rrd-engine/src/engine/cold_history.rs`, and `parquet` is a dependency of `rrd-engine`, `rrd-query` and `rrflow-cli`, not of a storage crate |
| rrflowLSM as its own product, tailored for bi-temporal reasoning | System overview row for rrflowLSM | Named. No record defines what it offers on its own, or what "tailored for bi-temporal reasoning" requires of it |
| Qdrant-class semantic and keyword search with TurboQuant; rrflowKV built on lessons from SurrealDB | Kernel contract sections 4.1 and 4.2 | Recorded with a measured gap: 15.3 queries/s against Qdrant's 567 at recall 0.95 on 20k vectors |
| A free hosting tier with a set amount of storage | K15 cloud tier and synchronization | Open. No record states a free tier or a storage allowance |

### Preferences, DevAssist and the controller

| Direction | Owner today | State |
|---|---|---|
| Schemas are global plus project preferences; project supersedes by default | [ADR-0005](../decisions/0005-second-brain-personal-and-org-engrams.md) item 3: run, then project, then user, then org, then core | Decided and consistent; not implemented ([POAM-077](../poam/rrflow-1.0-alpha/poam-077.md)). "when variant official overwrite" may be ADR-0005's policies, which only tighten. Needs the owner's reading (question 3) |
| Schemas are lanes that route quickly, not restrictions | [ADR-0002](../decisions/0002-adaptive-reasoning-governed-effects.md): "Do not build a schema of thought". Kernel contract S8 toggle | Decided; S8 is not implemented |
| Schemas respond by defined formulas, driven by a behavioural state machine fed by telemetry and metadata | No record found | Open. No state list, event list or formula exists |
| DevAssist, enabled by a UI toggle, included in RRFlow instead of left to Clyffy | No record found; the name appears nowhere in `docs/` or `crates/` | Open |
| DevAssist notices attention shifting, nudges passively to finish the task, and attunes to the operator's cadence and expressions | No record found | Open |
| A component library where triggers and hooks form a fabric for factoids, knowledge drips and did-you-know, with minimal requirements per domain or boundary | The historical [second-mind program](second-mind-program.md) records the same ask of 2026-11-10; the design is Connectome parking lot 008, status PROPOSED | Recorded, not in the kernel contract's package list by that name |
| SWAG can compete as a tuned model of about 9 GB; the module and its model must exist on Hugging Face | [LFG and SWAG reference](../reference/inference/lfg-swag.md) | Open. No record names a model size, a tuned SWAG model or a publication target |
| LFG is experimental and must be clearly defined | K8, and question 3 of the [DevWorld direction](devworld-owner-direction-20261116.md#open-questions-for-the-owner) | Open; the same question stands |
| DevWorld, also DevU, is the one optimized instance that is the second brain; its framing is rot prevention | K11 and ADR-0005; the DevWorld direction record | Decided as the personal engram; not implemented. "DevU" and "rot prevention" are new wording with no owner |
| The engine that evolves | Kernel contract K10 "learned tuning", and RROFLOW in the system overview | Direction exists; RROFLOW is unimplemented |

### Connectome

| Direction | Owner today | State |
|---|---|---|
| A Connectome dev build for advancing and optimizing it | [Connectome client contract](../reference/client/connectome.md#development-loop): `scripts/dev/connectome_dev.py`, from the owner direction of 2026-11-14 | Present. Not run in this session |
| The dev build runs tests and comparisons | K12 "Lab routes"; the [comparison lab](../../eval/lab/README.md) | Decided in K12; not verified here |
| It generates dynamic visuals for social media | DevWorld direction record, Connectome row | Open; Connectome changes land in its own repository |
| DevWorld properly implemented | K11, H-10, POAM-077 | Not implemented |

## Corrections to what the session told the owner

The session answered from pages fetched before reading this checkout, and three statements were wrong against `main`:

- It said neither rrflowLSM nor rrflowCLD is named in the repository. Both have rows in the system overview.
- It said Parquet is not absorbed at all. A Parquet cold-history writer exists and is enabled by default; what is missing is listed in the storage table above.
- It drafted a personal-engram decision as if none existed. ADR-0005 was accepted on 2026-11-11 and already decides it, with an org engram added.

Its recommendation of one writer per engram was rejected by the owner and is withdrawn.

## Open questions for the owner

1. Concurrency has two readings. One is many writers committing at once inside one engine process, sharing one page cache and memtable budget. The other is several processes opening one engram directory. Which is required, or both?
2. Does rrflowCLD stay a copy while rrflowLSM keeps every version (2026-11-14), or do memories now move between the two as they are used?
3. Is "official overwrite" the org policy of ADR-0005, or a separate override?
4. What does "rrg" stand for?
5. What else drives the behavioural state machine after "telemetry, metadata,"?

Nothing here has been implemented, tested or measured.
