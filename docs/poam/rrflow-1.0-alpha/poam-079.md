# POAM-079 — most engine tests run on built-in defaults, not the installed configuration

**Status:** active parent-owned POA&M detail
**Coordinate:** `rrflow://rrflow-instance/data/poam/rrflow-1.0-alpha/poam-079`
**Owner:** [RRFlow 1.0 alpha POA&M](../rrflow-1.0-alpha.md)

This detail is governed by the parent ledger; it cannot independently change
priority or closure status.

## Observed deficiency

A code read at `de435238` on 2026-11-11 counted 115 calls to `RrdEngine::open(`, its definition included, and 29 calls to `open_installed(`.

- `RrdEngine::open` uses the built-in default configuration and default LSM options (`crates/authority/rrd-engine/src/engine/core.rs:212-268`).
- `open_installed` applies the sealed installed configuration and storage options.
- The two differ. For example, `StorageLimits::default()` has `cold_history: None` (`crates/transport/rrd-contract/src/deployment.rs:375-378`), while the shipped configuration turns the cold tier on (`crates/authority/rrd-engine/assets/install/default-engram-configuration-v3.toml:62-70`).

Separately, tests have no named tiers. Deeper tests are 44 `#[ignore]` tests run by exact name, plus environment-variable and feature gates. The full CI lane is a hand-written list of about 72 `cargo test` lines, repeated per platform.

## Impact

Most engine tests exercise a configuration no installation uses, so a passing suite says little about the shipped product. With no named short and deep tiers, the deep tiers can't be triggered by what a change affects.

## Owning gates

D-01 owns the installed lifecycle that tests must exercise. The candidate CI owner is [`docs/operations/ci.md`](../../operations/ci.md).

Roadmap owner: [Gate D](../../roadmap/rrflow-1.0/gate-d.md).

## Closure evidence

Closed when all of the following hold:
- Engine tests open with the shipped configuration by default, and any departure is named in the test.
- The built-in defaults equal the shipped file, or a test proves their declared differences.
- Tests are grouped into named size tiers. The short default and the deep tiers are selected from what a change affects.
