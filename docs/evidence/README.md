# RRFlow evidence index

**Status:** active evidence index; test plans are not execution evidence
**Coordinate:** `rrflow://rrflow-instance/data/evidence-index/rrflow-evidence`
**Owner:** reproducible proof and acceptance-test-plan discovery; linked from `docs/README.md`

Evidence is generated or measured output bound to a source revision, inputs,
environment, command, and owning roadmap gate. A design note, passing compile,
unchecked scenario, or copied benchmark claim is not evidence.

| Subject | Durable warp | Checkout record | State |
|---|---|---|---|
| Linked repository change journals | [`rrflow://rrflow-instance/data/evidence-index/change-journals`](rrflow://rrflow-instance/data/evidence-index/change-journals) | [`change-journals/`](change-journals/) | immutable per-package receipts with generated discovery; roadmap and POA&M owners retain status authority |
| Acceptance test plans | [`rrflow://rrflow-instance/data/evidence-index/test-plans`](rrflow://rrflow-instance/data/evidence-index/test-plans) | [`test-plans/`](test-plans/) | scenario definitions only; individual cells become evidence only through their owning gate |
| D-01 native installed foundation | n/a | [qualification observations](d01-native-qualification.json), [Linux](d01-native-installed-linux-x86_64.json), [macOS](d01-native-installed-macos-arm64.json), [Windows](d01-native-installed-windows-amd64.json) | candidate `9b6c3571`, 17 successful required jobs, five actual isolated cases per platform and both durable modes; [Gate D acceptance](../roadmap/rrflow-1.0/gate-d-evidence.md#d-01-accepted-installed-foundation-2026-10-21) owns the bounded result and remaining limits |
| C-06j rrflowKV scan-resistant cache integration | n/a | [`c06j-rrflowkv-scan-resistant-cache-linux-x86_64.json`](c06j-rrflowkv-scan-resistant-cache-linux-x86_64.json) | clean revision `5b1c31d`, artifact SHA-256 `8a008ee33bb50ca197783227cfcfbb4d58945dd2f26dca8cdf12e1804106b9ad`: three zero-exit production-reader children preserve exact durable/semantic identity and 1 MiB capacity while exact LRU reloads 48 post-scan hot pages and scope-aware scan-resistant LRU reloads zero with 18,200 same-scope suppressions, 48 promotions, and 48 protected entries; contributes the final measured slice to accepted C-06, but is not release, concurrency, scale, latency, DataFusion, or competitor proof |
| C-06i rrflowKV adaptive page-compression integration | n/a | [`c06i-rrflowkv-adaptive-page-compression-linux-x86_64.json`](c06i-rrflowkv-adaptive-page-compression-linux-x86_64.json) | clean revision `eb7445e`: segment v6 integrated adaptive LZ4, 50 raw and 784 compressed reopened pages, 1,418,038 stored/8,988,877 logical page bytes, 336,041 query-decompressed bytes, exact isolated child identities, and zero child failures; bounded slice contributing to accepted C-06, not release, cross-platform, all-workload, or competitor proof |
| C-06i rrflowKV persisted-filter integration | n/a | [`c06i-rrflowkv-persisted-filter-linux-x86_64.json`](c06i-rrflowkv-persisted-filter-linux-x86_64.json) | clean revision `07a6bb8`: 139 authenticated filters/11,080 raw bytes, zero member false negatives, 0.634% observed false positives, zero semantic-page open work, and 114 page loads for 16,106 in-range miss checks; bounded slice contributing to accepted C-06, not release, cross-platform, all-workload, or competitor proof |
| C-06i rrflowKV physical-policy candidate screen | n/a | [`c06i-rrflowkv-physical-policy-linux-x86_64.json`](c06i-rrflowkv-physical-policy-linux-x86_64.json) | clean revision `f7257fa` fixed-machine candidate selection; not an integrated production policy, release proof, or competitor claim |
| Historical mixed-storage soak | n/a | [`m4-storage-mixed-soak.json`](m4-storage-mixed-soak.json) | retained raw artifact; not RRFlow 1.0 qualification |
| Historical vector measurement | n/a | [`m5-vector-local-10000x128.json`](m5-vector-local-10000x128.json) | retained raw artifact; not RRFlow 1.0 qualification |
| Historical edge measurement | n/a | [`m6-edge-local-10000x128.json`](m6-edge-local-10000x128.json) | retained raw artifact; not RRFlow 1.0 qualification |
| Historical quantization measurement | n/a | [`g04-w04-quantization-local-512x64.json`](g04-w04-quantization-local-512x64.json) | retained raw artifact; not RRFlow 1.0 qualification |
