# POAM-022 — deployment facts and profile semantics

**Status:** active parent-owned POA&M detail
**Coordinate:** `rrflow://rrflow-instance/data/poam/rrflow-1.0-alpha/poam-022`
**Owner:** [RRFlow 1.0 alpha POA&M](../rrflow-1.0-alpha.md)

This detail record is part of the canonical RRFlow 1.0 alpha POA&M. The parent
ledger owns priority and lifecycle status; this file cannot change either
independently.

## Observed deficiency

At the recorded baseline, public `DeploymentMode` placed `rrflow_mx`,
`embedded`, `local_daemon`, `edge`, `remote`, and `distributed` in one scalar
even though they represented storage, composition, process locality, an
offline artifact, a client-relative transport view, and an unavailable cluster
claim. `RrdEngine` inferred it from persistent-root presence, the server
inferred it from TLS, generated SDKs repeated it, and the shared “deployment
conformance” fixture contained only two documents plus one exact-ID query; the
edge test never entered `RrdEngine`.

## Impact

The same installed instance can report a different supposed mode when TLS changes or a
different client observes it; a server over rrflowMX cannot be described truthfully;
unsupported cluster/artifact values are advertised as peers of real engine profiles; and
passing seed lookups can falsely qualify rrflowMX/rrflowKV, DataFusion, graph/index/recall,
reasoning, transport, or deployment equivalence.

## Owning gates

A-07, B-04, C-02 through C-04, D-01, E-01 through E-05, F-01 through F-05, G-04, G-05, H-01
through H-05, H-07, J-01 through J-05

Roadmap owners: [Gate A](../../roadmap/rrflow-1.0/gate-a.md),
[Gate B](../../roadmap/rrflow-1.0/gate-b.md),
[Gate C](../../roadmap/rrflow-1.0/gate-c.md),
[Gate D](../../roadmap/rrflow-1.0/gate-d.md),
[Gate E](../../roadmap/rrflow-1.0/gate-e.md),
[Gate F](../../roadmap/rrflow-1.0/gate-f.md),
[Gate G](../../roadmap/rrflow-1.0/gate-g.md),
[Gate H](../../roadmap/rrflow-1.0/gate-h.md), and
[Gate J](../../roadmap/rrflow-1.0/gate-j.md).

## Closure evidence

Replace the scalar with independently installed deployment-form, storage-profile,
endpoint-presentation, and security facts; remove root/TLS/location inference and
speculative active values with no alias; separate storage semantics, rrflowKV durability,
deployment-form, endpoint, cluster, and derived-artifact corpora; then prove identical
authorized transactions, temporal graph/index/vector/RRF behavior, stamped streamed
Arrow/DataFusion results, persisted reasoning/context/evidence, delivery, limits, and traces
across valid profiles, with rrflowKV-only crash/reopen and clean installed real-process
evidence.

## Current remediation evidence

Package `D01-02-canonical-engram-layout-and-configuration-v1` removes
`DeploymentMode` and its wire field without an alias. `DeploymentProfile` now
separates deployment form, storage profile, and endpoint presentation;
`RrdEngine` carries the explicit storage fact; the server derives only endpoint
presentation from the bound address; TLS remains a separate security fact; and
OpenAPI plus TypeScript and all five SDK fixtures use the generated structured
shape. Focused contract, engine, server-process, Rust-client, installed-primary-
binary, and SDK tests reject the old scalar and exercise the new discovery.

This is material remediation but not closure. The storage-semantic,
rrflowKV-durability, complete deployment-form, configured-network endpoint,
cluster, derived-artifact, streamed Arrow/DataFusion, native graph/index/vector,
reasoning/recall, delivery, trace, and clean release-candidate corpora in the
closure requirement remain incomplete. Parent status therefore remains
`Sequenced` and no owning roadmap gate is checked.
