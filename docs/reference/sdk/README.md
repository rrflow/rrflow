# RRFlow SDK reference

**Status:** active SDK-reference index
**Coordinate:** `rrflow://rrflow-instance/data/reference-index/sdk`
**Owner:** supported language-client behavior and conformance discovery; linked from `docs/reference/README.md`

SDKs are outward clients of the public RRD protocol. They may provide typed
construction, transport, authentication, cancellation, retry, subscription,
and result APIs, but they do not own engine semantics or open rrflowMX,
rrflowKV, rrflowQL, DataFusion, graph, index, vector, reasoning, attunement, or
automation state directly.

The executable public contract and operation/capability catalogues are the one
language-neutral source. OpenAPI, internal client bindings, language operations
and models, reference coverage, and the shared conformance matrix are generated
projections. A language record documents only runtime, packaging, toolchain,
and qualification differences. A generated schema or compiling package is not
a supported SDK until its installed real-process conformance passes. See the
[generation design](../../roadmap/rrflow-1.0-execution/generated-surfaces.md).

## Language records

<!-- rrflow:generated-index:start -->
- [RRFlow .NET SDK](dotnet.md) — [`rrflow://rrflow-instance/data/reference/sdk/dotnet`](rrflow://rrflow-instance/data/reference/sdk/dotnet) — active implementation reference; alpha SDK and artifact qualification are incomplete
- [RRFlow Go SDK](go.md) — [`rrflow://rrflow-instance/data/reference/sdk/go`](rrflow://rrflow-instance/data/reference/sdk/go) — active implementation reference; alpha SDK and module qualification are incomplete
- [RRFlow Java SDK](java.md) — [`rrflow://rrflow-instance/data/reference/sdk/java`](rrflow://rrflow-instance/data/reference/sdk/java) — active implementation reference; alpha SDK and artifact qualification are incomplete
- [RRFlow Python SDK](python.md) — [`rrflow://rrflow-instance/data/reference/sdk/python`](rrflow://rrflow-instance/data/reference/sdk/python) — active implementation reference; alpha SDK and package qualification are incomplete
- [Build against local rrflowDB](rrflowdb-local.md) — [`rrflow://rrflow-instance/data/reference/sdk/rrflowdb-local`](rrflow://rrflow-instance/data/reference/sdk/rrflowdb-local) — active native database integration guide; pre-alpha, not PostgreSQL qualification
- [RRFlow Rust SDK](rust.md) — [`rrflow://rrflow-instance/data/reference/sdk/rust`](rrflow://rrflow-instance/data/reference/sdk/rust) — active implementation reference; alpha SDK qualification is incomplete
- [RRFlow TypeScript SDK](typescript.md) — [`rrflow://rrflow-instance/data/reference/sdk/typescript`](rrflow://rrflow-instance/data/reference/sdk/typescript) — active implementation reference; alpha SDK and package qualification are incomplete
<!-- rrflow:generated-index:end -->

Other language records enter this index only after their contract-derived
implementation, shared corpus behavior, and toolchain execution have been
reviewed. The [public-contract reference](../protocol/public-contract.md) owns
the language-neutral operation and wire authority; the
[release roadmap](../../roadmap/rrflow-1.0.md) alone owns completion.
