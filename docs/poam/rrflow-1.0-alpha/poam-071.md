# POAM-071 — two transactions on unrelated rows cannot both commit

**Status:** active parent-owned POA&M detail
**Coordinate:** `rrflow://rrflow-instance/data/poam/rrflow-1.0-alpha/poam-071`
**Owner:** [RRFlow 1.0 alpha POA&M](../rrflow-1.0-alpha.md)

This detail is governed by the parent ledger; it cannot independently change
priority or closure status.

## Observed deficiency

On 2026-11-11, a live probe of a release binary through the public API began two transactions and wrote unrelated rows; the second commit was refused. `prepare_semantic_commit` in `crates/persistence/rrd-store/src/access/semantic_commit.rs` compares one engram-wide cursor with the commit's expected cursor, so any commit between a transaction's snapshot and its commit refuses it. An engine test on two unrelated rows fails with `TransactionSnapshotStale { expected: 3, actual: 4 }`, and a companion test shows that two writers on one row do not lose an update. With 8 concurrent writers the probe recorded 370 commits, 1,056 conflicts and 30.8 commits per second, against 51.3 commits per second for one writer.

## Impact

Concurrent writers do not scale, and Postgres-compatible transaction behavior cannot be offered while unrelated writes conflict.

## Owning gates

C-02 owns the transaction primitives.

Roadmap owner: [Gate C](../../roadmap/rrflow-1.0/gate-c.md).

## Closure evidence

Closed when commits conflict only on overlapping data, unrelated concurrent transactions both commit, a lost update remains impossible, and a multi-writer run shows the conflict rate falling on disjoint keys.
