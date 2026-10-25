# C-07c installed domain-transaction crash integration

**Status:** active implementation evidence; candidate CI required; C-07 remains open
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-c/c-07c-installed-domain-crashes`
**Owner:** OBJ-02 prerequisite, [C-07c](../../../roadmap/rrflow-1.0/gate-c-07.md), [POAM-035](../../../poam/rrflow-1.0-alpha/poam-035.md)

## Change brief and baseline

[PR 60](https://github.com/rrflow/rrflow-development/pull/60) began at public
main `14c0ef5f0228f92ae3db00f25904564f1575d08f`, tree
`070eb687cb3ac413ed06e5928de456c47d0098e6`. The preimplementation brief and
[baseline journal](c-07c-installed-domain-baseline.md) preceded implementation;
seed `adac3ff2728b4e4044883cbd5e9d8d4c8961cfe4` has tree
`a8efc9bb88abded6507bb0d8535c564cd573ae29`. Its canceled CI run
`39723482748` is not acceptance. No unrelated changes were present.

Previously, exact native crash points were exercised by the LSM unit child,
while the ordinary installed engine qualified transport-response loss. This
package connects five exact domain-transaction boundaries to fresh/existing
installed primary executables, authenticated clients and independent histories.
It adds a nondefault diagnostic feature; it changes no default persistence
policy, semantic contract, format, version or runtime activation surface.

The four-crate `crash-test` feature reuses the existing rendezvous source.
Private stdin admits at most 4 KiB and one schedule. The locator supplies the
root; the exact operation digest selects the authorized domain scope after
intent persistence. Recovery arms only after installed preflight, before native
open. Points are pre-WAL, complete append, successful native sync, visible
batch and persisted receipt before response. Each point also interrupts the
following WAL replay. Normal distributions exclude the controller.

Stop conditions remain lost ACK, partial batch, wrong target, duplicate effects,
leaked child, default-build activation, relaxed existing deadlines or a repair
outside this declared scope. No such production failure was observed here.

## Implementation traceability and review

The existing `fault_io::crash` unit route remains under `cfg(test)` with its
40 boundary cases, 80 compound recovery kills and two 100-cycle histories.
The same source is compiled under a hidden diagnostic module only for the
explicit feature. Nothing was moved, deleted, replaced with a fallback or
copied from upstream. The native observation points preserve the same WAL,
memtable, intent and receipt owners. The normal artifact exclusion and ordinary
lifecycle corpus characterize the unchanged distribution path.

Files read in full and reread for this change:

- `crates/persistence/rrd-lsm/{Cargo.toml,src/lib.rs,src/wal.rs,src/database.rs,src/fault_io/crash.rs}`
  and new `src/fault_io/crash/installed.rs`: feature closure, append/replay,
  publication context, scope and owned rendezvous.
- `crates/persistence/rrd-store/{Cargo.toml,src/lib.rs}` and
  `crates/authority/rrd-engine/{Cargo.toml,src/lib.rs,src/engine/transaction.rs,src/engine/installation.rs}`:
  composition, `commit_transaction`, `open_installed_anchored`.
- `crates/adapters/rrflow-cli/{Cargo.toml,src/installed.rs,tests/support/process.rs,tests/lifecycle_process.rs}`
  and new `tests/installed_crash.rs`: private input, bounded event capture,
  owned child termination, real installation and independent reconciliation.
- `.github/workflows/ci-reusable.yml`, `scripts/ci/check_workflow.py`,
  `docs/operations/ci.md` and the C-07 owner: required native matrix,
  ordinary-artifact exclusion and evidence limits.

README, alpha outcome, POAM-035, the authoring procedure, installed lifecycle
and prior crash journals supplied the owning context. Created paths are the
two Rust files above and this journal; other implementation changes are the
listed files. Generated journal navigation and execution inventory are
replaceable projections. The complete diff was reviewed before commit.

Research/source adaptation: existing accepted durability contracts and code
were sufficient; no upstream algorithm or source was imported. Trace/debug:
bounded sanitized test receipts record binary identity, nonce/PID/point,
hit/reap time and client attempts. No payloads or credentials enter receipts.
No production trace or allocation policy changes were needed. Resource limits
here bound the harness; they are not C-07d process-memory proof.

## Failure oracle and local evidence

The first diagnostic build exposed a SHA-256 formatting mismatch with the
locked SHA-2 API; explicit byte-to-hex formatting corrected it without changing
dependencies. With physical observations deliberately unwired, the focused
fresh installed test failed after 119.13 seconds waiting for its exact event.
It reaped the owned child. Unwired executable SHA-256:
`f8982550c1fa5efff8431f2f3ae5daf627d540f04cf48548b85ec148fd1c49aa`.

The first wired focused run passed in 369.57 seconds, with five domain hits,
five interrupted recoveries and 21 reconciled attempts. Its executable was
`fed0e305df4d549bb70d257b7cd707dcf6954fdc7fe56d5dbbc0dd0d020b4301`.
Review then moved recovery arming after preflight so executable hashing cannot
consume the existing 30-second native pause deadline. The final suite includes
that placement and negative admission controls.

Final local diagnostic command:

```text
cargo test -p rrflow-cli --test installed_crash --features crash-test --locked -- --nocapture --test-threads=2
```

Windows x86_64, Rust 1.99.1: **3 passed, 0 failed, 0 ignored**, 440.31 seconds.
Both estate histories contain six logical calls, 21 attempts, 16 ACKs, five
uncertain outcomes, six resolved-present operations and zero violations or
unresolved outcomes. Each has ten validated domain/recovery hits. The third
parent rejects six invalid schedules and commits an unrelated operation without
triggering the selected boundary. Executable SHA-256:
`684f566dd338eeaaf9a27f417265983f735a3e5e47fd9594ed458153406d15bf`.
Raw local log SHA-256:
`38689dee7eaeddc93f7f55dd334a9fb573ad60fb769252774bec075b5a277e6c`.
The local build reports unqualified provenance; it is not a signed distribution.

`cargo test -p rrflow-cli --test lifecycle_process --features crash-test --locked`
passed **13 tests**, with the one explicitly invoked subprocess fixture ignored
at top level. `cargo test -p rrd-lsm --locked` passed **131 tests**; its ignored
native child is explicitly invoked by the parent corpus. Linux-only RSS cases
execute zero tests on this Windows host and are not claimed as passed.

`cargo clippy -p rrflow-cli --bin rrflow --test installed_crash --test lifecycle_process --features crash-test --locked -- -D warnings`
passed. Formatting, workflow policy and whitespace checks passed. The initial
documentation check rejected this journal's missing active/historical status
classification; the status line was corrected before publication.

Both additional strict lint commands passed:

```text
cargo clippy -p rrd-lsm --all-targets --all-features --locked -- -D warnings
cargo clippy -p rrflow-cli --test installed_lifecycle --test lifecycle_process --locked -- -D warnings
```

The rebuilt ordinary primary contains no `C07C_NATIVE_PAUSE_V1=` bytes when
scanned across 1 MiB chunk boundaries. It is 156,127,744 bytes, SHA-256
`f151c3de9b6e29d4ae9dcf16a1da8a10e3d4488d0c21269d6882d8d75fcdf0ee`.
This exclusion is repeated before diagnostic compilation in each native CI job.

`cargo test -p rrflow-cli --test installed_lifecycle --test lifecycle_process --locked -- --nocapture`
passed the ordinary primary's **5 lifecycle tests** in 315.72 seconds and
**10 helper tests** in 2.01 seconds. The helper's explicit subprocess fixture
remains top-level ignored. Fresh/existing restore and transport histories bind
the ordinary executable digest above, separately from diagnostic histories.

Final repository checks: `cargo fmt --all -- --check`,
`python -X utf8 scripts/ci/check_workflow.py`,
`python -X utf8 scripts/ci/check_documentation.py`,
`python -X utf8 scripts/knowledge/render_navigation.py --check`,
`python -X utf8 scripts/ci/build_execution_inventory.py --check`, and
`git diff --check` passed after regeneration. Navigation contains 17 generated
indexes, 396 nodes and 1729 edges; inventory contains 1383 paths. Documentation
policy validates 397 statuses and 395 classified coordinates. Native Linux/macOS
execution, remaining workspace partitions and SDK conformance are delegated to
the required candidate CI, not claimed from this local Windows run.

## Remaining qualification and change checklist

Publication/reclamation composition and the installed 100-cycle operating
profiles remain C-07c work. Hard resource admission, retained raw/mapped/LZ4
owners, real ext4/APFS/NTFS exhaustion, frozen capacity profiles and the real
24-hour soak remain C-07d-g. These process kills do not prove loss of volatile
OS/device caches. No full alpha, filesystem capacity or competitive claim is
made; C-07, POAM-035 and all parent outcomes stay open.

Read-only infrastructure discovery found zero registered development runners
and no `CI_SELF_HOSTED_ENABLED` variable. GitHub's
[job limits](https://docs.github.com/en/enterprise-cloud%40latest/actions/reference/limits)
allow six hours on hosted runners; an uninterrupted 24-hour soak needs its
declared dedicated host. No runner was registered or soak launched. This is
infrastructure availability evidence, not storage acceptance.

Checklist: owner and bounded scope bound before edits; exact failure oracle
executed; native helper/owning suite and lint run; full-file/diff review;
trace/research/resource decisions recorded; failures retained; generated
navigation/inventory refreshed; required candidate CI and merge bound to PR 60.
No new product deficiency beyond the existing POAM-035 scope was demonstrated.
Exact candidate CI and merge are external effect records and must pass before
landing; local results alone do not satisfy that gate.
