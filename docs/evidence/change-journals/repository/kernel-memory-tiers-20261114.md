# Kernel contract: memory tiers, consolidation and the answer-close structure

**Status:** active planning evidence; no runtime behavior changed

**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/repository/kernel-memory-tiers-20261114`

**Owner:** the [RRFlow kernel contract](../../../roadmap/kernel-contract.md)

**Starting revision:** `development/main` at `1b283d40`.

**Owner direction, 2026-11-14:**
- Make rrflowKV modern, drawing on SurrealDB, HydraDB and Qdrant.
- Treat rrflowLSM as short-term memory and rrflowCLD as long-term memory, with the context, LSM, CLD and TurboQuant tunable and triggered dynamically.
- Rebuild "neurons" and hops while idle.
- Make the second brain the operator's global preferences.
- Show that LFG and SWAG improve efficiency and answer quality.
- End every answer with its assumptions, its gaps and an offer of another pass.

**Method:**
- **Code survey:** a read-only survey of `development/main` covering eight areas: the KV and LSM structures, the LSM-to-CLD path, the planner and routing, background work, preferences, LFG and SWAG, honesty structures, and the lab. Each finding was labelled as on a live path, unused, or documentation only. Its findings fill the "Today" columns.
- **Web research:**
  - HydraDB's own descriptions: a three-tier layout, versioned temporal graphs, and multi-index retrieval followed by graph expansion. These are vendor claims and are labelled as such.
  - Learned LSM tuning: AXE (VLDB 2025) and CAMAL.
  - Key-value separation and its garbage collection: DumpKV (2025).
  - Agent-memory consolidation: MemoryBank decay, Zep Graphiti invalidation, A-MEM note evolution, and Letta sleep-time compute.

**Changed paths:**
- `docs/roadmap/kernel-contract.md`:
  - the owner's words in section 1;
  - the `answer-close-v1` default structure in S8;
  - a new S9 for memory tiers and consolidation;
  - section 4.1, the K10 rrflowKV technique queue;
  - section 4.2, the four-arm LFG and SWAG proof in K3;
  - package K16 for idle consolidation;
  - two open decisions in section 6.
- This record.

**Recorded disagreement:** "CLD favors postgres" is accepted for rrflowDB's analytical reads only. rrflowDB writes need the WAL and conflict checks, so they stay on the hot tier and are demoted later.

**Checks:** `render_navigation.py` and `check_documentation.py`.

**Not established:**
- No package is implemented.
- Every demotion and consolidation threshold is `unmeasured`.
- There is no evidence that LFG or SWAG improves answer quality, because the judge model and its spending limit are still the owner's decision.
- "LD" is read as link distance until the owner confirms it.
- The HydraDB figures are the vendor's own and were not reproduced.
