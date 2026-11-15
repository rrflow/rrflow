# Project map: install and serve write it again

**Status:** active implementation evidence

**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-d/project-map-callers-restored-20261114`

**Owner:** [Gate D](../../../roadmap/rrflow-1.0/gate-d.md), installed lifecycle ([contract](../../../reference/deployment/installed-lifecycle.md#project-map))

**Starting revision:** `main` at `833f23466`.

**Owner direction, 2026-10-31:** importing a project automatically creates a mapping file, uses it, and evolves it over time.

**Defect.** `dd27aa96a` (2026-10-31) delivered that: `rrflow install apply` and `rrflow serve` wrote `.rrflow/project-map.md`. Merge `4c2f2a6a1` (2026-11-09) kept the engine function and silently dropped everything that reached it. Since then on `main`:

- `RrdEngine::refresh_project_map` had no caller, so no installed project ever got a map.
- Its test, `the_project_map_describes_the_project_and_keeps_the_owners_notes`, was gone.
- `validate_recovery_estate` no longer admitted `project-map.md` or `.project-map.pending` below `.rrflow`.
- The contract's "Project map" section was gone, while [the map's journal](project-map.md) still linked to it and still said install and serve write the map.

The D-02 plan noticed the dead code on 2026-11-11 and left it for package K5. This was found again while absorbing the archived branch `wip/project-map-pre-main` ([ledger](../repository/archive-absorption-20261114.md)).

**Change:**

- `crates/adapters/rrflow-cli/src/installed.rs`: `project_map_line` refreshes the map after `install apply` and in `serve`, under the engine's writer lock, and adds one line to the text output. A failure is logged as `project_map.refresh` and reported in that line; it never blocks.
- `crates/authority/rrd-engine/src/engine/estate_layout.rs`: `validate_recovery_estate` admits the map and its pending file. The two names come from the map's own path constants, so they have one owner.
- `crates/authority/rrd-engine/src/engine/project_map.rs`: the pending-file constant is visible to the layout.
- `crates/authority/rrd-engine/src/engine/tests/installation/project_map.rs` (new): the restored test, and a second one.
- `crates/adapters/rrflow-cli/tests/installed_lifecycle/lifecycle.rs`: the installed binary is checked.
- `docs/reference/deployment/installed-lifecycle.md`: the "Project map" section is back, with what is still missing.
- `docs/roadmap/d02-evidence-driven-attunement-engineering-plan.md`: its "dead code" finding now says when it was repaired.

**Traceability.** Source: `dd27aa96a`, read in full for the five paths above. Nothing was taken from the archived draft, which used a third-party ignore-rule crate that `main` replaced with its own Git reader. Canonical destination: the same files, at today's paths (`tests/installation.rs` is now a folder).

**Oracles, before and after:**

- `the_project_map_describes_the_project_and_keeps_the_owners_notes`: a project with a `.gitignore`, a manifest and three folders gets a map naming its kind, start-here file and folder table; ignored folders are absent; the owner's notes survive a refresh; an unchanged project is not rewritten; an added file updates the table.
- `an_installed_project_that_holds_its_map_still_reopens_replays_and_recovers`: with a map and a leftover pending file present, recovery validation accepts the layout, install replays, verification passes, the engine reopens, and the project's source precondition is the same as before the map existed. With the two allowlist entries removed this test fails at recovery validation with `ProjectBindingMismatch`; with them it passes. Run both ways on 2026-11-14.
- Installed binary (`installed_lifecycle`): after `install apply` the map exists and starts with `# Project map: `; replaying install does not rewrite it; after the map is deleted, `serve` writes it again with identical content.

**Commands and results, Windows, 2026-11-14:**

- `cargo test -p rrd-engine --lib --locked -- engine::tests::installation::project_map`: 2 passed.
- `cargo test -p rrd-engine --lib --locked`: 479 passed, 0 failed, 13 ignored.
- `cargo test -p rrflow-cli --test installed_lifecycle --locked`: 11 passed, 0 failed. Both `installed_engine_plans_applies_serves_discovers_authenticates_and_reopens` (existing project) and `fresh_installed_engine_survives_restart_and_restore` (fresh project) run the lifecycle that holds the map assertions.
- `cargo test -p rrflow-cli --test lifecycle_process --locked`: 11 passed, 1 ignored.
- `python scripts/ci/check_documentation.py`, `build_execution_inventory.py --check`, `check_selected_tests.py`, `check_code_shape.py`: passed.

**Not done:**

- The map is not a recall record, is not read by context assembly, and Connectome does not show it. Kernel package K5 owns making it a projection of the Inventory records.
- With `--json`, install and serve do not report the map; only the text output and the trace do.
- A project larger than the profile's inventory bound gets the "not refreshed" line and no map.
- Not run on Linux or macOS here; the development gate runs the engine tests on Linux.

**Research decision:** none. **Trace decision:** the existing `rrflow::installed` target, operation `project_map.refresh`, on failure only. **Roadmap/POA&M change:** none.
