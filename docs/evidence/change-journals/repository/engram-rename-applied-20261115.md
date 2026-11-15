# The estate-to-engram rename, applied by hand

**Status:** active change evidence for POAM-078
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/repository/engram-rename-applied-20261115`
**Owner:** [POAM-078](../../../poam/rrflow-1.0-alpha/poam-078.md)

## Change brief

- **Outcome and package:** POAM-078; one canonical name, `engram`, in code, public contract, SDKs, paths and documentation.
- **Starting revision:** `8e22ac483` (residue collapse), on top of remote `b9d6e7a4d`.
- **Current behaviour:** `estate` named the same concept in types, fields, routes, crate and binary names, security action names and persisted keys.
- **Target behaviour:** every one of those reads `engram`; protected historical records keep their words.
- **Method:** a word-aware, case-preserving substitution (`estate`/`estates` only when not preceded by a letter, so `restate` and `tracestate` stay; `Estate` and `ESTATE` as words inside identifiers, so `LifecycleState` is untouched), applied to tracked text files, with 59 path moves done by `git mv` in the same change. Collisions found before applying: one duplicate loader (`load_engram_configuration`), deleted.
- **Protected, never edited:** `docs/evidence/`, `docs/history/`, `eval/inputs/`, `eval/results/`, the recorded provider captures (`endpoint-selection-observed`, `endpoint-chat-native`), `endpoint-profile-v1.json`, the schema-registry default digest fixture and its test, `key_codec/tests.rs` (frozen vectors) and the POAM-078 record.
- **Recorded captures:** tests read the recorded bytes and, in memory, read `identity.estate_id` as `engram_id` and re-bind the recorded configuration digest (`observed_configuration` helpers in the contract, inference, cli and engine test trees). The capture files themselves are unchanged.
- **Link repair:** protected records that link to renamed files had their link targets repointed (link text and bodies untouched); links to protected files that did not move were reverted to the old name.

## Reinstall requirement

Pre-release policy: no migration. Persisted `estate_id`, key prefixes, security action names and digests all change, so an existing install must be reinstalled.

## Regenerated through their owners' generators

`install-attunement-v3.json`, `public-contract-v1.json`, `engram-organisation-v1.json`, `plan-vectors.json`, the OpenAPI digest, SDKs and docs citations (`regen_surfaces.sh`), the storage qualification fixtures (`refresh_storage_contract` and `RRFLOW_UPDATE_STORAGE_GOLDENS`), the pilot study's catalogue digest (`eval/lab/studies/rrflow-planning-pilot.json`, `4a7f0827…` to `9d786aa8…`, because the task catalogue's wording was renamed), `fixtures/rrd-sdk-conformance-v1.json` instance id, `Cargo.lock`.

## Commands and results

- `cargo test --workspace --locked --no-fail-fast`: 284 test binaries reported ok, no failure or error lines.
- `python scripts/ci/check_code_shape.py`, `check_workflow.py`, `check_selected_tests.py`, `check_generated_surfaces.py`, `check_documentation.py`, `scripts/knowledge/render_navigation.py`, `build_execution_inventory.py --check`: all pass.
- CI policy unit tests (`test_execution_inventory`, `test_select_ci_lanes`, `test_ci_lanes_policy`, `test_code_shape`, `test_sdk_attunement_policy`): pass.
- SDKs: TypeScript `pnpm check` (7 tests), Python pytest with `PYTHONPATH=src`, Go `go test ./...`: pass.

## Surfaced failures and fixes

Duplicate profile loader; capture readers failing on the renamed field and digest (projection helper); frozen key-codec vector (restored, not regenerated); POAM-078 title damaged by the substitution (restored); broken links from protected records (repointed).

## Not run

Java and .NET SDK suites (only through the gate); the Connectome submodule is not renamed or regenerated (it is pinned and changes in its own repository); the live development gate before push.

## Remaining

Connectome rename and client regeneration for attunement contract version 3; readable plan-step wording made configurable.
