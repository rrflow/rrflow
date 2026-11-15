# POAM-082 — the TypeScript SDK silently rounds 64-bit integers

**Status:** active parent-owned POA&M detail
**Coordinate:** `rrflow://rrflow-instance/data/poam/rrflow-1.0-alpha/poam-082`
**Owner:** [RRFlow 1.0 alpha POA&M](../rrflow-1.0-alpha.md)

This detail is governed by the parent ledger; it cannot independently change
priority or closure status.

## Observed deficiency

The F-06 worktree found this on 2026-11-07 and recorded it only there. It was archived uncommitted and absorbed on 2026-11-14 ([ledger](../../evidence/change-journals/repository/archive-absorption-20261114.md)). A code read of `main` at `fdb339d08` confirms it still holds:

- `sdks/typescript/src/transport.ts:35` decodes every response with plain `JSON.parse`. JavaScript numbers are doubles, so an integer above 2^53 is rounded without an error: `JSON.parse('{"a":9007199254740993}').a` is `9007199254740992` (run on 2026-11-14).
- The public contract declares 3,559 `uint64` positions in the generated TypeScript types (`sdks/typescript/src/generated/rrd-openapi.ts`), typed as `number`.

The archived journal reproduced it twice: a controlled `cold-history-read` response whose `through_cursor` was 9007199254740993 came back as 9007199254740992, and a real daemon round trip where the Python client read the exact value and the TypeScript client did not. I did not re-run those two reproductions.

## Impact

A TypeScript client, including anything built on this SDK, can read a wrong cursor, count, size or time from the engine and act on it, with nothing to show the value changed. Values this large are not produced by small installations today, which is why no test has caught it; the types promise the full range.

## Owning gates

H-04 owns the SDKs' wire fidelity and conformance.

Roadmap owner: [Gate H](../../roadmap/rrflow-1.0/gate-h.md).

## Closure evidence

Closed when all of the following hold:
- The TypeScript SDK returns every 64-bit integer the engine sends exactly, or refuses a value it cannot represent; it never returns a different number.
- The shared SDK conformance corpus includes a value above 2^53, and all five SDKs read it back exactly.
