# POAM-032 — prompt flights removed without engine absorption

**Status:** active parent-owned POA&M detail
**Coordinate:** `rrflow://rrflow-instance/data/poam/rrflow-1.0-alpha/poam-032`
**Owner:** [RRFlow 1.0 alpha POA&M](../rrflow-1.0-alpha.md)

This detail record is part of the canonical RRFlow 1.0 alpha POA&M. The parent
ledger owns priority and lifecycle status; this file cannot change either
independently.

## Observed deficiency

The pre-rebrand workbench recorded prompt flights in the retired in-repository
workbench crate's `src/flight.rs` (1,756 lines): fresh, pruned, and full context
arms, durable flight rows and append-only events in the runtime log,
cohort comparison, source-routing candidates, optional Codex and Claude runner
arms, and provider trace envelopes. The D5 absorption map required
"engine-owned governed flight commands and append-only diagnostic events" and
behavior parity before the absorbed code was deleted (D5.3). The history shows
a different sequence:

- `98e6551` (2026-08-27) made the workbench a strict RRD client and answered
  flight writes, cluster samples, trace summaries, and source routing with
  explicit `501` responses, recorded as remaining D5.1/D5.3 work;
- `1483e57` (2026-08-27) moved `flight.rs` uncompiled into that crate's
  `migration/embedded-authority/` directory as migration input;
- `5448722` (2026-09-03) deleted that migration input, together with the
  engine preflight that backed the full context arm, without an engine flight
  operation; and
- `b5db6c2` (2026-09-04) deleted the remaining workbench crate.

At `0bdc400` the public catalogue held 33 operations and none recorded,
listed, or read a prompt flight. The separate Connectome client still carried
the flight inspector against the retired `/api/flights` routes, which no
engine serves.

## Impact

A prompt could not be recorded, compared across context arms, or replayed
through any public surface. The only implementation of the capability was
deleted rather than carried into its canonical boundary, and
the client's flight inspector rendered nothing it could obtain from the
engine.

## Owning gates

H-05, H-06

H-05 owns correlated diagnostic evidence for the prompt path; H-06 owns the
flight lens on public operations.

Roadmap owner: [Gate H](../../roadmap/rrflow-1.0/gate-h.md).

## Closure evidence

The [H-06a engine prompt-flight journal](../../evidence/change-journals/gate-h/h06a-engine-prompt-flight-journal.md)
records the first package: `flight-launch`, `flight-list`, and `flight-read`
in `rrd-contract`; `RrdEngine` records the `observe` arm with `fresh` and
`pruned` context at one stamp as a strict lifecycle record and events in one
runtime commit, with exact replay, conflict, denial, reopen, and real-process
client evidence.

The [H-06b per-prompt flight graph journal](../../evidence/change-journals/gate-h/h06b-per-prompt-flight-graph-journal.md)
records the second: each pruned flight writes one strict
`rrflow-prompt-flight-used` edge per record it used, in the same commit, so
rrflowQL traverses a prompt to its records and a record back to its prompts;
`flight-read` reads the record, its edges, and its events by identity, so its
cost is the flight's own size.

The row closes only when, in addition:

- each model-provider arm runs as an explicit, installed, opt-in adapter with
  its budget, audit, and redacted causal trace through the H-05b emission path;
- the full context arm returns once an engine-owned preflight context operation
  exists, and source-routing candidates return once an engine routing
  projection exists, each as public operations; and
- the Connectome flight lens consumes the three operations at the generated
  contract digest against an installed estate, with the retired diagnostics
  protocol and private routes removed.
