# C-07d installed storage-policy implementation

**Status:** active immutable change-journal evidence
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-c/c-07d-installed-storage-policy`
**Owner:** OBJ-02 C-07d installed resource-admission boundary

[PR 65](https://github.com/rrflow/rrflow-development/pull/65) bound this runtime
scope before edits. Starting revision is
`1158929580817401c454e70d7790b0fa826fb162`, composed from accepted main
`72dd666b254f3f403d11f942c1180b108cc40d21` and the preserved native/store/
engine/restore resource-limit changes. The owner explicitly directed code
implementation without another planning package or repeated local tests.

## Implemented boundary

`StorageLimits` in the existing deployment contract now carries hard mutable
bytes/versions, retained projected-output bytes, page-cache bytes and soft
WAL/memtable flush targets. Configuration format 3 validates and digests those
fields; product version remains 1.0.0. The bundled v3 TOML replaces v2 directly,
and both bundled profiles select it. There is no fallback decoder.

The native lowering in `engine/core.rs` validates checked host-size conversion.
Installation creation/resumption, ordinary open, read-only preflight,
post-commit verification and exact install replay consume that policy.
Locator format 2 carries its pre-open projection; installed record and active
configuration comparisons reject mismatches before readiness. Backup restore,
object/control verification and restore-publication inspection consume the same
options instead of defaulting independently. Existing component constructors
retain their explicit default behavior outside installed composition.

Traceability: the earlier active-state/output/restore implementations remain
at their canonical owners. The former configuration asset maps directly to
the v3 asset; profile includes, digests, existing configuration fixtures and
SDK capability fixtures change together. Existing install/reopen/recovery and
restore characterizations remain in place. No semantic implementation is
deleted, no separate scheduler is added, and no API advertises total RSS bounds.

Reviewed source boundaries: deployment configuration validation/digest/export;
core persistent constructors; installation load/plan/apply/locator/inspection/
replay and committed-state verification; restore/publication opens; native
options/mutable/output definitions and configured store restore APIs; bundled
profile/template inputs; all changed fixture bodies and generated diffs.
The changed paths are recorded mechanically in Git and the generated inventory.
The installed-lifecycle reference documents the exact controls and exclusions.

Research/source adaptation: existing RRFlow ownership and accounting semantics,
no new algorithm or upstream code. Trace/resource/debug: preserve existing
native counters and expose the effective policy through ordinary configuration
discovery. Limits do not claim heap/RSS, queue, compaction-debt or capacity proof.

## Commands, corrections and omissions

No local runtime tests or benchmarks were executed. Existing test inputs were
updated for the new required contract; they were compiled, not run.

- `cargo check -p rrd-engine -p rrflow-cli --all-targets --locked` passed initially
  in 54.97 seconds. A later edit placed a new options argument in the adjacent
  record literal rather than the verification call; fmt/compilation exposed
  the syntax and missing-argument errors. The argument was moved to the call;
  the corrected all-target check passed.
- `cargo clippy -p rrd-contract -p rrd-engine -p rrflow-cli --all-targets --locked
  -- -D warnings` passed in 21.64 seconds. `cargo fmt --all` formatted the code.
- Locked TypeScript dependencies were reused without downloads. Initial SDK
  generation warned about Node 24.21 versus required 24.22; Node 24.22.0 was
  installed and the generator rerun explicitly under it. Python generation
  initially failed because Ruff was absent from PATH; `uv run --no-project
  --with ruff==0.17.1 python sdks/python/scripts/generate.py` succeeded.
- Existing TypeScript, Python, Go, Java and .NET generators produced the new
  endpoint/schema projections. Windows-generated Python/Java/.NET output was
  normalized to the repository's LF representation. Five existing capability
  fixtures now carry the v3 policy and corresponding configuration digest.
- `python -X utf8 scripts/ci/check_generated_surfaces.py --write` rendered seven
  signal projections against 42 HTTP operations. OpenAPI identity is
  `f6222edb5c2c247566cbb244a4b33098aa0567a4b883957dad0c0bfbca83d30f`;
  the unchanged signal identity is
  `239df2ced5974d4351ca01565369646964cb65c63d0fbae01dddccb4c3ac5a07`.
  Existing reference digest citations and navigation/inventory were updated.
  The first inventory generation encountered the removed v2 asset still in
  Git's index; staging the direct replacement before regeneration corrected
  that tooling precondition. No runtime path or fallback was added.

The prior composition-only workflow 39879031091 was canceled as superseded,
not counted as passing. Required exact-candidate CI remains the merge gate.
No new policy-specific installed execution, SDK conformance, native exhaustion,
capacity measurement or 24-hour soak result is claimed. Existing format-2
configuration and format-1 locator inputs are intentionally rejected; no
automatic migration or release compatibility is implied.

Checklist: pre-edit scope recorded; one canonical configuration/lowering;
source/asset/fixture traceability; full changed diff review; compilation and
generation corrections retained; no hidden failed test or invented acceptance;
explicit research/debug/resource decisions; linked journal and projections.
Stop on ignored limits, early ACK, fallback authority, unsupported completion
or failed candidate CI. C-07, POAM-035 and alpha status remain open.
