# Main went red on a stale fixture; the gate now tests what pins a changed crate

**Status:** active change evidence

**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/repository/ci-pinned-identity-20261114`

**Owner:** the [candidate CI pipeline](../../../operations/ci.md)

**Starting revision:** `main` at `3f5f2533a`, red: [run 43657707227](https://github.com/rrflow/rrflow-development/actions/runs/43657707227), alarm [#392](https://github.com/rrflow/rrflow-development/issues/392).

**What failed.** `engine fast lane`, `cargo test -p rrflow-cli --test installed_reopen`, test `sustained::point_run::failed_campaign_preserves_closed_cause_and_readback_provenance`:

```text
left: "public catalogue identity differs from validator"
right: "failed case must retain failure provenance"
```

**Cause.** The push that went red did not cause it. `c2547985c` (H-04, merged as #391 earlier on 2026-11-14) added retry classes to the endpoint catalogue. That changed the catalogue's digest. The storage qualification fixture records that digest as `public_catalogue_sha256` (`crates/evaluation/qualification/rrflow-qualification-storage/tests/fixtures/storage/manifest.json`), and it was not refreshed. From that commit on, `main` was broken:

- every fixture-reading test in `rrflow-qualification-storage` failed (confirmed locally on 2026-11-14 before the refresh);
- one `rrflow-cli` test that reads the same fixture failed.

**Why the gate let it through.** The development gate tests the crates a change touches and only compiles the crates that depend on them. #391 touched `rrd-contract`. Both failing crates were dependents, so they were compiled and never run. The breakage showed on the first later push that touched `rrflow-cli`.

**Change:**

- **The evidence.** The three fixtures were refreshed with the crate's own tool, `cargo run -p rrflow-qualification-storage --example refresh_storage_contract`: four digest lines in `manifest.json`, `result.json` and `report.json`.
- **The source.** `scripts/ci/select_ci_lanes.py` gains `PINNED_BY`: owner crate to the crates whose tests hold a golden derived from the owner's public identity. When the owner changes, those crates are tested and linted, not only compiled. Today: `rrd-contract` to `rrflow-qualification-storage`.
- `scripts/ci/test_select_ci_lanes.py`: a contract change selects the pinning crate as a tested package; a crate that pins nothing selects nothing extra; every entry must be a workspace member that depends on its owner, so the table cannot name a crate that stopped pinning.
- `docs/operations/ci.md` states the rule and its limit.

**Before and after:**

- Before the refresh: `cargo test -p rrflow-qualification-storage --locked` failed across its suite.
- After: 43 unit and 11 contract tests pass. `cargo test -p rrflow-cli --test installed_reopen --locked`: 39 passed, 0 failed, including the test that failed in the gate.
- `python -m unittest test_select_ci_lanes test_ci_lanes_policy` in `scripts/ci`: 30 passed.
- Replaying #391's change set through the selector now selects `-p rrd-contract -p rrflow-qualification-storage`.

**Not covered:**

- Only goldens listed in `PINNED_BY` are protected. A dependent whose behaviour, not a golden, breaks is still found by the nightly full matrix and not by the gate. I searched for other places that record the catalogue digest and found none; I did not audit every golden in the workspace for other owners.
- The nightly matrix would have caught this within a day. It had last run before #391 merged.

**Research decision:** none. **Trace decision:** none. **Roadmap/POA&M change:** none.
