# POAM-083 — the Rust SDK conformance test passes without running

**Status:** active parent-owned POA&M detail
**Coordinate:** `rrflow://rrflow-instance/data/poam/rrflow-1.0-alpha/poam-083`
**Owner:** [RRFlow 1.0 alpha POA&M](../rrflow-1.0-alpha.md)

This detail is governed by the parent ledger; it cannot independently change
priority or closure status.

## Observed deficiency

The F-06 worktree found this on 2026-11-07 and recorded it only there. It was archived uncommitted and absorbed on 2026-11-14 ([ledger](../../evidence/change-journals/repository/archive-absorption-20261114.md)). A code read of `main` at `fdb339d08` confirms it still holds:

- `crates/transport/rrd-client/tests/sdk_conformance.rs:51-53`: `rust_sdk_passes_shared_real_daemon_conformance` returns at once when `RRD_SDK_CONFORMANCE_MANIFEST` is not set. `cargo test -p rrd-client` therefore reports it as passed although it checked nothing.
- `scripts/ci/run_sdk_conformance.py` sets that variable when it runs the Rust client, but accepts the process's exit status alone. If the variable were ever dropped or misnamed, the Rust leg would pass empty and nothing would notice.

The archived work marked the test `#[ignore]`, made the manifest mandatory, and made the runner require the line `SDK conformance OK: language=rust corpus_sha256=…` in the output. None of that is on `main`.

## Impact

"The Rust SDK passes conformance" is counted in ordinary test runs where it did not run, and the gate's own proof of it rests on an environment variable rather than on evidence the checks executed.

## Owning gates

H-04 owns SDK conformance.

Roadmap owner: [Gate H](../../roadmap/rrflow-1.0/gate-h.md).

## Closure evidence

Closed when all of the following hold:
- The Rust conformance test cannot report success without executing against a daemon: it is ignored by default and fails when run without its manifest.
- The conformance runner requires each language's completion line, with the corpus digest, and fails a successful process that did not print it. A test covers that.
