# Bi-temporal reasoning and the three builds: owner direction of 2026-11-16

**Status:** active owner direction record; not an accepted decision, plan or status. The [kernel contract](kernel-contract.md) remains the one active engineering plan, and promoting any item below to a package needs the owner's decision
**Coordinate:** `rrflow://rrflow-instance/data/work-package/bitemporal-reasoning-owner-direction`
**Owner:** records, verbatim, what the repository owner said on 2026-11-16 about the three builds, the engine's spine, bi-temporal reasoning and multi-session folding, and maps each part to the record that owns it or to none; the [roadmap](rrflow-1.0.md) and [POA&M](../poam/rrflow-1.0-alpha.md) keep status authority
**Verified against:** `main` at `cce43a20b` on 2026-11-16, by searching `docs/` and `crates/` for the terms named in each row

This record continues the [DevWorld direction](devworld-owner-direction-20261116.md) and the [storage, concurrency and DevAssist direction](storage-and-devassist-owner-direction-20261116.md) of the same day.

## The owner's words

Typos are the owner's and are kept.

On the three builds and the engine's spine:

> I should be able to have clear UI via connectome during devlopment that shows all of the latency, the individual dynamic links what schema for the task, the classifier code, and once I am absolutely proven that we have each one built so during development we have clear UI we have the private dev lab build and we have the customer build.  We have to be able to get rrflowDB completely optimized in rust while having acid, and other enhancements for rrflowLM rrflowCLD rrflowQL rrflowKV to flawlessly integrate to me it makes sense to have rrflowKV rrflowQL rrflowDB as the main dynamic aspect of the engine that then has clear pathways that allows for the rrflowLSM built to be optimized and first rival fjall and then surpass fjall, I need rrflowCLD to truly maximize its use to identify rrflowDB it should be able to bridge rrflowKV to its KV via RRflowQL and its transactional relationships right?  true machine language that are true triggers and dynamic optimized flows that are crystal clear, the knowledge paths are just extended resources that are optimized with rrflowKV/rrflowCLD and again we build it to properly have it offer to migrate into their primary.  the secondbrain has been discussed but i believe handwaving and all of my merges it was lost.  christ only knows what the fuck else.  I need this fucking cleaned up I need it crystal clear what the fuck it is and it needs to show true signs of challenging surrealDB so i can bring swag in have the same performance because we have it so each stage of thinking gets clear binary score choice guidance and then we have LFG become my dynamic engine for the prompt recipe system to help users always have prompts that start to get saved easily updated for that project and executed by clyffy my designated AI to work with frontierAI and for the developer and truly be their right hand assistant, i have other outside tools i hope the user will see and take advantage of devagotchi, nomen, shippinAI, etc

On bi-temporal and multi-session folding:

> look you need to get serious bi-tempoloar look at hte latest notest and we have to be able to have one thinking actually be multi session catamorphism

> you understand that the in-process reasoning needs to allow it so when we have the devtoggle for schemas we need to learn the techniques to haev behavioral state properly then generating the necessary templates, the lexicon/dictionary, all of these micro optimzations folded into rrflow are what make this bleeding edge and truely bi-temporal

On the macOS retained-install test, the owner challenged its source. The refusal comes from `directory_security_sha256` in `crates/authority/rrd-engine/src/engine/token_key/private_file/unix.rs`, introduced by commit `fb78758d4` (2026-11-15), which implements the read for Linux only. It is an engine gap, not a documented macOS limit, and a test asserting the refusal does not close [POAM-006](../poam/rrflow-1.0-alpha/poam-006.md).

## Where each part is owned today

| Direction | Owner today | State |
|---|---|---|
| Connectome shows latency, each dynamic link, the schema chosen for the task and the classifier code during development | [Connectome client contract](../reference/client/connectome.md#development-loop) and kernel contract K12 | Direction exists; not verified here. Per-stage latency and classifier-code views have no record found |
| Three builds: Connectome development UI, private development lab, customer build, each after its parts are proven | No record found | Open. No record defines the three builds or their separation |
| rrflowDB optimized in Rust with ACID | [Kernel contract](kernel-contract.md), [system overview](../architecture/system-overview.md) | Named; no record states the optimization target or the ACID property to prove |
| rrflowKV, rrflowQL and rrflowDB as the dynamic core, with pathways down to rrflowLSM | System overview terminology | Named. The pathways are not defined as a contract in any record found |
| rrflowLSM first rivals Fjall, then surpasses it | K10 "LSM against Fjall" and the borrow-or-build floor check | Partly; see the [storage direction](storage-and-devassist-owner-direction-20261116.md#storage) for the measured and missing capabilities |
| rrflowCLD identifies rrflowDB and bridges rrflowKV to its KV through rrflowQL and transactional relationships | System overview rrflowCLD row: a columnar copy inside rrflowKV, read by the planner | Not verified. The owner asked whether this is right; no record states a bridge, and nothing here checks it against the code |
| Knowledge paths are extended resources on rrflowKV and rrflowCLD, with an offer to migrate into the user's primary database | No record found | Open |
| Each stage of thinking gets a binary score, a choice and guidance | Kernel contract S2 and K8 name classifier output; no record gives each stage a binary score | Open |
| rrflowDB challenges SurrealDB so SWAG can be added at the same performance | [Comparison lab](../../eval/lab/README.md); the storage direction records a 15.3 against 567 queries/s gap on vector search | Open; no SurrealDB challenge result is recorded in this record |
| LFG as the engine of a prompt-recipe system: prompts saved, updated per project, executed by Clyffy | K8 and the [DevWorld direction](devworld-owner-direction-20261116.md) question 3 | Open. "Prompt recipe" appears nowhere in `docs/` or `crates/` |
| Outside tools the user can adopt: devagotchi, nomen, shippinAI | K15 names shippin; the other two have no record found | Open |
| The second brain was discussed but lost in merges | [DevWorld direction](devworld-owner-direction-20261116.md), ADR-0005, K11 | Recorded now; not implemented |
| Serious bi-temporal reasoning | [Multi-model query reference](../reference/query/multi-model.md) defines bitemporal `record:<kind>` and `claim` collections and reads at "the bitemporal graph snapshot visible at the requested coordinates". The system overview binds a read to policy, valid time and one `ReadStamp`; the [index catalogue](../reference/query/index-catalogue.md) says a newer artifact "is not valid for an older bitemporal read" | Specified in references. Whether the engine implements every part is not checked here. No record defines what rrflowLSM needs to be "tailored for bi-temporal reasoning" |
| One thinking spans several sessions as a catamorphism | Only a 2026-11-08 Connectome change-journal line about trigger and catamorphism details ([h-06](../evidence/change-journals/gate-h/h-06-connectome-workbench-recovery-20261108.md)); the task-budget-resume contract restarts one budgeted task | Open. No design, schema or code states a multi-session fold |
| The in-process reasoning must allow the fold | No record found | Open |
| With the schema toggle on, learn the techniques to hold behavioural state properly, then generate the templates, the lexicon and dictionary | Kernel contract S8 and K9 (reasoning structure, filled from evidence); [second-mind program](second-mind-program.md) row for a growing lexicon; K14 lexicon on the saved prompts | Direction exists in parts. The behavioural state and the template generation have no owner; the [storage direction](storage-and-devassist-owner-direction-20261116.md) already lists its state machine as open |
| Many micro-optimizations folded into rrflow | Kernel contract K10 technique queue | Direction exists; nothing here selects or measures them |

## Open questions for the owner

1. A reading of "one thinking is a multi-session catamorphism": each session appends facts stamped with valid time and knowledge time, and a fold function combines them into the thinking's current state, so that re-running the fold as of any past knowledge time gives the same answer. Is that right?
2. Must the fold be associative, so sessions can fold in parallel and merge in any order?
3. What does a fold produce: a plan, a set of resolved claims, or an answer?
4. Is the bridge from rrflowKV to rrflowCLD's KV through rrflowQL, as you described, the intended design, or the question to be checked against the code first?
5. Should the macOS retained-install read be implemented now, replacing the refusal test?

Nothing here has been implemented, tested or measured.
