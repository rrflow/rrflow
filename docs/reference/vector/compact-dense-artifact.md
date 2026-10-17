# RRFlow compact dense vector artifact

**Status:** active implemented format and exact-search kernel; Arrow-native integration remains open
**Coordinate:** `rrflow://rrflow-instance/data/reference/vector/compact-dense-artifact`
**Owner:** immutable dense-vector artifact format, verified publication, mmap loading, exact kernels, and accelerator admission

`CompactDenseSegment` is RRFlow's current compact exact-vector projection. It
is a rebuildable artifact over canonical vector state, never a second vector
database or a mutation authority. The projection descriptor binds scope,
field, dimensions, metric, optional embedding model, generation, source
cursor, configuration digest, artifact digest, and filter coverage.

## Format and publication

Format version 1 contains:

- a fixed 128-byte little-endian header;
- canonical JSON identity, temporal, provenance, and payload metadata;
- a 64-byte-aligned row-major `f32` payload with zeroed padding; and
- a domain-separated SHA-256 digest over the header and payload.

Decode validates magic, version, flags, reserved bytes, lengths, offsets,
alignment, candidate count, dimensions, finite values, padding, canonical
metadata encoding and order, scope, source cursor, field, model binding, and
artifact digest. The complete artifact is bounded to 1 GiB and ten million
candidate versions.

`write_atomic` stages a uniquely named file, synchronizes it, publishes with a
fail-if-present hard link, synchronizes the containing directory, and reopens
the winner for verification. An existing target is accepted only when its
verified bytes are identical. `open_mmap` creates a verified read-only mapping
under the immutable-file contract.

## Exact execution

The scalar kernel decodes little-endian `f32` rows directly from artifact
bytes. On x86-64, `DenseKernel::Auto` selects AVX2 only after runtime feature
detection and uses unaligned vector loads. Cosine, dot, Euclidean, and
Manhattan results are differentially checked against the exact reference
oracle with an explicit floating-point tolerance.

This is not yet an Arrow physical page or a DataFusion `RecordBatch`. Metadata
is JSON, the vector region is RRFlow's own aligned row layout, and the exact
kernel reads it directly. Gate C-06 must decide and prove the canonical
Arrow-compatible persistent page layout; Gate F must expose stamped batches
through rrflowQL without falsely labeling this format as zero-copy Arrow.
Current builds receive the scope's whole vector-version family through C-04's
direct reader and reduce it in memory. E-04 must supply incremental,
collection-addressed vector input.

## Accelerator boundary

Optional dense and HNSW builder traits may produce bytes on an accelerator,
but accelerator output is untrusted. The CPU artifact is built first. Compact
dense output is admitted only when decoding succeeds and its bytes and
descriptor exactly equal the deterministic CPU artifact. HNSW accelerator
output additionally passes bounded semantic probes. Prefer/require policy
controls explicit CPU fallback.

No production CUDA, ROCm, Metal, Vulkan, or cuVS builder is installed by this
repository today. Fake adapters prove corruption, wrong generation, device
failure, parity, and fallback behavior; they do not prove that physical GPU
execution occurred.

## Executable evidence and open work

`rrd-vector/tests/compact_dense.rs` proves deterministic format bytes,
owned/mmap parity, scalar/SIMD parity, corruption rejection, stale-read
rejection, and non-overwriting publication. `rrd-vector/tests/accelerator.rs`
proves accelerator admission and fallback policy. Model-space rejection is
covered by `rrd-vector/tests/model_binding.rs`.

Sparse and multivector compact layouts, compact HNSW storage, native payload
bitmap access, background optimization, Arrow-compatible pages, and
fixed-hardware end-to-end comparisons remain open under Gates C, E, F, and J.
