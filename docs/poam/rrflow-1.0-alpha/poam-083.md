# POAM-083 — SDK conformance tests can pass without running

**Status:** active parent-owned POA&M detail
**Coordinate:** `rrflow://rrflow-instance/data/poam/rrflow-1.0-alpha/poam-083`
**Owner:** [RRFlow 1.0 alpha POA&M](../rrflow-1.0-alpha.md)

This detail is governed by the parent ledger; it cannot independently change
priority or closure status.

## Observed deficiency

The F-06 worktree found this on 2026-11-07 and recorded it only there. It was archived uncommitted and absorbed on 2026-11-14 ([ledger](../../evidence/change-journals/repository/archive-absorption-20261114.md)). A code read of `main` at `fdb339d08` confirms it still holds:

- `crates/transport/rrd-client/tests/sdk_conformance.rs:51-53`: `rust_sdk_passes_shared_real_daemon_conformance` returns at once when `RRD_SDK_CONFORMANCE_MANIFEST` is not set. `cargo test -p rrd-client` therefore reports it as passed although it checked nothing.
- `scripts/ci/run_sdk_conformance.py` sets that variable when it runs the Rust client, but accepts the process's exit status alone. If the variable were ever dropped or misnamed, the Rust leg would pass empty and nothing would notice.

The archived work marked the test `#[ignore]`, made the manifest mandatory, and made the runner require the line `SDK conformance OK: language=rust corpus_sha256=…` in the output.

Reading the other four clients on 2026-11-14 found the same shape in two of them:

- **.NET** (`sdks/dotnet/tests/Rrflow.Rrd.Client.Tests/SdkConformanceTests.cs:37-40`) returns at once without the manifest, so an ordinary `dotnet test` counts it as passed.
- **Java** (`sdks/java/src/test/java/io/rrflow/rrd/SdkConformanceTest.java:41-42`) is skipped by an assumption. That is reported as skipped, not passed, but the runner still accepts its exit status alone.
- Python, Go and TypeScript refuse to start without the manifest.

## Remediation in progress

The Rust leg is fixed on 2026-11-14 ([ledger](../../evidence/change-journals/repository/archive-absorption-20261114.md)): the test is ignored by default and panics without its manifest, the runner selects it by name with `--ignored` and fails a successful process that did not print the Rust test's own completion line, and the name is owned in `scripts/ci/selected_tests.py`. Proven locally: plain `cargo test -p rrd-client --test sdk_conformance` reports 0 passed and 1 ignored; with `--ignored` and no manifest it fails. The .NET and Java legs are not changed.

## Impact

"The Rust SDK passes conformance" is counted in ordinary test runs where it did not run, and the gate's own proof of it rests on an environment variable rather than on evidence the checks executed.

## Owning gates

H-04 owns SDK conformance.

Roadmap owner: [Gate H](../../roadmap/rrflow-1.0/gate-h.md).

## Closure evidence

Closed when all of the following hold:
- No SDK conformance test can report success without executing against a daemon. Rust: done. .NET and Java: open.
- The conformance runner requires each language's own completion line, with the corpus digest, and fails a successful process that did not print it. Rust: done, with a test. Python and Go also print their own line, but the runner does not require it; for every language but Rust the runner prints the line itself after the process exits.
