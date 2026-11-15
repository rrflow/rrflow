# D-02: a stored plan reads back in the words it was approved in

**Status:** active implementation evidence; D-02, OBJ-14 and POAM-039 remain open

**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-d/d-02-readable-plan-steps-20261114`

**Owner:** D-02 in [Gate D](../../../roadmap/rrflow-1.0/gate-d.md); OBJ-14 (provable flow); [POAM-039](../../../poam/rrflow-1.0-alpha/poam-039.md)

**Starting revision:** `main` at `fb1b3a33e`.

**Where this came from.** The archived branch `codex/readable-attunement-plan` (draft PR #225, 2026-10-30 and 31) did this and never landed. It is ported by hand; see the [ledger](../repository/archive-absorption-20261114.md).

**Current behavior before this change.** A proposal's steps had descriptions, and the installation preview showed them. The stored plan kept only a digest of each step's configuration. After install, an authorized plan read could not say what any step was for; the contract told operators to keep the preview file if they wanted the words.

**Target behavior.** Each step of a stored attunement plan carries its description. The plan digest covers it, so the words cannot change without changing the plan's identity. An authorized plan read returns them, before and after a replan and a restart.

**Change:**

- `rrd-contract/src/attunement/plan.rs`: `AttunementPhasePlan` gains `description`, required, validated as non-blank, at most `MAX_ATTUNEMENT_DESCRIPTION_BYTES` (1,024) and free of NUL. The JSON schema carries the length bound.
- `rrd-contract/src/attunement/installation.rs`: `ATTUNEMENT_CONTRACT_VERSION` is 3. The description bound moves here from the engine, so it has one owner.
- `rrd-engine/src/engine/installation/profile.rs`: a profile or proposal step's text becomes the plan step's description.
- `rrd-contract/src/attunement/derivation.rs`: a step the engine derives from evidence gets a description built from that evidence. The branch never designed this, because derived plans did not exist then.
- Regenerated from the contract by their own generators: the OpenAPI digest (`0e688c6b…` to `cdb35615…`), the TypeScript, Python, Go, Java and .NET endpoint files, and the seven signal projections. The six references that cite the digest by hand are updated.
- The golden fixture is regenerated and renamed `install-attunement-v3.json`.
- `docs/reference/deployment/installed-lifecycle.md` and POAM-039 no longer say descriptions are lost.

**What a derived step says.** Only facts the derivation already holds: the step's name, and for the steps selected on source files, how many files have a bound grammar and which grammars. For example `parse: selected from project evidence; 37 files with a bound grammar (python, rust)`. It does not say what a step will achieve. Seven of the eleven capabilities are not executable today, and a description that promised an outcome would be a claim the engine cannot back.

**OpenAPI review.** Read from the regenerated TypeScript types: the document changed only by the `description` property on the plan step, wherever a plan appears, and by `contract_version` 2 becoming 3. Nothing else.

**Compatibility.** Attunement plans, jobs and commands at version 1 or 2 are rejected; a stored version 2 plan cannot be read. There is no migration, as for every earlier pre-release contract change. An existing installed project must be reinstalled. Installation results stay at version 1 and the product version stays `1.0.0`. A description is visible to every principal allowed to read the plan and must not hold a secret.

**Oracles:**

- `plan_preserves_and_binds_readable_step_descriptions` (contract): version 2 is refused; a step without a description is not a plan; changing a description without recomputing the digest is refused, and with it the plan and the installation plan get different identities; empty, blank, NUL and over-long (513 two-byte characters) descriptions are refused.
- `selected_plan_replans_after_progress_and_reopens` (engine, both variants): the preview's steps read `first`, `second`, `verify`, and the stored plan read through `read_attunement_detail` reads the same.
- `test_installed_python_stale_plan_replans_and_starts` (real binary over HTTP): the plan read after install has the three descriptions, and the successor's stored plan read after an accepted replan has the same identity as the accepted plan and the same three descriptions.
- Every existing test of derived plans passes with derived descriptions validated by the plan's own rule.

**Commands and results, Windows, 2026-11-14:**

- `cargo check --workspace --all-targets --locked`: passed.
- `cargo test -p rrd-contract -p rrflow-qualification-storage --locked`: 255 passed, 0 failed.
- `cargo test -p rrd-engine --lib --locked`: 481 passed, 0 failed, 13 ignored.
- `cargo test -p rrd-server --locked`: 37 passed. `cargo test -p rrd-client --locked`: 33 passed.
- `cargo test -p rrflow-cli --test installed_lifecycle --test installed_reopen --locked`: 50 passed, 14 ignored.
- `cargo clippy -p rrd-contract -p rrd-engine --all-targets --locked -- -D warnings`: passed.
- `python scripts/ci/check_generated_surfaces.py --check`: 83 HTTP operations, 7 signal projections, OpenAPI `cdb35615c9dbbb028c9173535cfdb64579d360a87beed2c9f9a5f14152ca4820`.
- `pnpm check` in `sdks/typescript`: generation check, lint, typecheck and 7 tests passed.
- Python: `tests/test_client.py` 7 passed; `tests/test_installed_attunement.py` against a debug build of this tree, 2 passed. Both were run with the source path set directly, because `uv` could not rebuild its environment on this machine (access denied removing `.venv/lib64`).

**Not established:**

- The Go, Java and .NET SDK tests were not run here; the gate's SDK conformance job runs them.
- Connectome does not show these descriptions yet. Its pinned client must be regenerated against version 3 before it can read a plan from this engine.
- No test reads the descriptions after a process restart through the Python client; the engine test reads the stored plan, and the installed lifecycle tests reopen the engine with version 3 plans.
- Whether the derived wording is what the owner wants shown to a layman. It is deliberately factual and bare.

**Research decision:** none. **Trace decision:** none. **Roadmap/POA&M change:** POAM-039's text is corrected; it stays open (handoffs, open questions and automatic planning are not addressed).
