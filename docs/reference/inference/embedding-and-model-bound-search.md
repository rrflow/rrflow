# RRFlow embedding and model-bound search

**Status:** active implemented compute contract; durable deployment integration remains open
**Coordinate:** `rrflow://rrflow-instance/data/reference/inference/embedding-and-model-bound-search`
**Owner:** provider-neutral embedding execution, model provenance, and same-stamp embedding search through `RrdEngine`

Inference is subordinate compute inside the one RRFlow engine. An embedding is
not authoritative because a model produced it. Canonical source identity,
source bytes, model identity, authorization, read stamp, transaction commit,
and the engine's vector/index rules remain the admission boundary.

## Current boundaries

For embeddings, `rrd-inference` defines validated model, backend, trust,
network, resource, request, batch, job, and prepared-vector types. `RrdEngine`
owns one process-local `EmbeddingBackendRegistry` and exposes authenticated
operations to list installed models, generate embeddings, and embed then
search.

Separately, B-03 defines a location-free router-model manifest and independent
runtime handshake in `rrd-contract`. `rrd-inference` verifies the manifest,
backend binding, observed schema/capabilities/resources/runtime, and exact
model, tokenizer, runtime, and grammar bytes before it can call a model loader.
That opaque admission is not a `RouterBackend`, installed-model catalogue,
route authorization, or dispatch path; D-05 and G-01 through G-06 own those
remaining behaviors.

Executable backend code, sessions, accelerator handles, and credentials remain
process-local. The registry revision is observable but is not durable engram
configuration. A restart therefore requires the operator or future installer
to install the same backend again. Gate D must make configured adapter
selection and attunement explicit without serializing provider credentials as
RRFlow records.

The generic contract permits text or image model descriptors and local or
remote trust boundaries. The current built-in executable backends are text
only:

- `FeatureHashBackend` is deterministic, dependency-free, local, and useful as
  a pipeline oracle. It is not a semantic-quality embedding model.
- the optional `fastembed-local` feature uses caller-supplied ONNX, tokenizer,
  and external-initializer bytes plus a caller-supplied runtime-configuration
  digest. Its content identity covers every supplied component and that
  configuration digest. The dependency is compiled without its model-hub or
  TLS defaults.

There is no built-in remote OpenAI, Anthropic, Gemini, Grok, or other provider
adapter in this boundary, and no executable image-embedding backend. Those
adapters must conform to the same descriptor, trust, network, budget,
provenance, and engine-authorization rules.

## Engine flows

`generate_embeddings` validates and authorizes the request, captures a
`ReadStamp`, dispatches a bounded batch through the exact selected backend,
and returns generated vectors with source digest, model digest, dimensions,
normalization, backend, registry revision, execution target, and trust
evidence. It does **not** persist those vectors. A caller that wants durable
memory must submit the returned values and provenance through an authorized
`RrdEngine` transaction.

```text
bytes + exact backend identity
            |
            v
 RrdEngine authorization + ReadStamp
            |
            v
 process-local embedding backend
            |
            v
 provenance-bound generated vector
       |                         |
       v                         v
return to caller       authorized transaction proposal
                                 |
                                 v
                       rrflowKV/rrflowMX commit path
```

The lower-level `EmbeddingCoordinator::prepare` additionally reads an
identified source before and after inference, rejects an inference-time
change, and can construct a `DataTransaction` bound to the original stamp. Its
compare-and-swap commit fails if canonical runtime state advanced after
inference.

`embed_and_search_vectors` captures one read stamp, requires the selected
backend to match the named vector's exact model name, revision digest,
dimensions, and dense shape, generates one query vector, and invokes
`search_vectors_at` at that stamp. It commits no temporary query vector.
The embedding job and prepared vector carry that required collection plus
named-vector address through persistence. Current search reads canonical
versions through C-04's direct stamped path; E-04 must replace process-local
candidate materialization with native persistent vector access. DataFusion does
not participate in this operation today; a later rrflowQL path must preserve
the same address, model, and read-stamp identity.

## Admission invariants

Before backend dispatch, the coordinator enforces:

- exact backend/model equality and unique batch job identities;
- declared modality and accepted media type;
- per-input, batch-input, batch-byte, and output-value limits;
- explicit network permission for a network-requiring backend;
- matching remote execution and provider trust identities;
- lowercase SHA-256 source and model digests; and
- dense, finite, correctly dimensioned and correctly normalized output.

Equal dimensions never imply that two embedding spaces are compatible.
Model-bound exact search and projection construction reject vectors whose
provenance differs from the requested binding. A named-vector definition may
omit an embedding-model binding under the general vector contract, but it may
never omit its collection address; engine-owned embed-and-search requires both
the address and an exact model binding.

## Executable evidence and open work

Current evidence includes `rrd-inference/tests/pipeline.rs` for provenance,
source-race, transaction-CAS, trust, network, batching, resource behavior, and
router-model pre-load rejection before a loader call;
`rrd-vector/tests/model_binding.rs` for embedding-space isolation; and
`engine::tests::native_inference` for public batch generation plus same-stamp
search.

This evidence does not establish semantic retrieval quality, durable provider
installation, multimodal backend coverage, native persistent vector access,
or dynamic reasoning/context routing. Gates D, E, F, G, and H own those
outcomes.
