# RRFlow roadmaps

**Status:** active roadmap index
**Coordinate:** `rrflow://rrflow-instance/data/roadmap-index/rrflow-roadmaps`
**Owner:** roadmap discovery; linked from `docs/README.md`

A roadmap records ordered delivery outcomes and acceptance evidence. It links
to architecture and decisions but does not redefine them.

The current [alpha objective](../objectives/rrflow-1.0-alpha.md) defines the
result. The current [POA&M](../poam/rrflow-1.0-alpha.md) tracks verified gaps
against the roadmap gates.

The canonical RRFlow 1.0 roadmap's links-only
[chapter index](rrflow-1.0/) provides direct warp points to its checklist,
dependency spine, gate requirements, and accepted evidence.

The canonical roadmap is accompanied by a subordinate link-only
[execution portal](rrflow-1.0-execution-map.md), its generated
[execution-record index](rrflow-1.0-execution/), the generated
[whole-repository file plan](rrflow-1.0-file-plan.jsonl), and linked
[change journals](../evidence/change-journals/). The roadmap owns requirements
and completion; the active change record owns exact in-progress paths; generated
views and journals provide discovery and evidence without changing status.
The completed C-06j work is recorded by the
[scan-resistant cache integration plan](c06j-rrflowkv-scan-resistant-cache-engineering-plan.md).
It specifies a configurable exact-byte production cache and a real persisted
mixed-family differential. The canonical roadmap accepts C-06 from the complete
C-06a-through-C-06j evidence corpus. The accepted
[D-01 installed foundation](rrflow-1.0/gate-d-evidence.md#d-01-accepted-installed-foundation-2026-10-21)
and [dependency spine](rrflow-1.0/executable-dependency-spine.md) define the
current handoff; this index does not maintain another active-item pointer.
The completed C-06i compression slice is bound by the
[adaptive page-compression plan](c06i-rrflowkv-adaptive-page-compression-engineering-plan.md).
It specifies a direct segment-v6 replacement with authenticated per-page
adaptive LZ4, bounded decode, explicit ownership, and integrated evidence. The
completed
[persisted-filter integration plan](c06i-rrflowkv-persisted-filter-integration-engineering-plan.md)
records the direct segment-v4-to-v5 replacement and first selected production
policy. The completed
[physical-policy selection plan](c06i-rrflowkv-physical-policy-selection-engineering-plan.md)
remains the measured input; C-06j resolves the cache decision and rejects value
placement for the accepted format. The completed C-06h
[adversarial qualification plan](c06h-rrflowkv-adversarial-qualification-engineering-plan.md)
and C-06g
[projected-read plan](c06g-rrflowkv-projected-read-engineering-plan.md) remain
supporting history. None can change canonical dependency order or completion
status.

| Release | Durable warp | Checkout record | State |
|---|---|---|---|
| RRFlow 1.0 | [`rrflow://rrflow-instance/data/roadmap/rrflow-1.0`](rrflow://rrflow-instance/data/roadmap/rrflow-1.0) | [`rrflow-1.0.md`](rrflow-1.0.md) | [canonical completion checklist](rrflow-1.0.md#checklist-item-warps) and [active dependency spine](rrflow-1.0/executable-dependency-spine.md); maturity remains owned by the root README |

| Supporting record | Durable warp | Checkout record | Authority |
|---|---|---|---|
| RRFlow 1.0 execution navigation | [`rrflow://rrflow-instance/data/execution-map/rrflow-1.0`](rrflow://rrflow-instance/data/execution-map/rrflow-1.0) | [`rrflow-1.0-execution-map.md`](rrflow-1.0-execution-map.md) | stable links to canonical requirements, active scope, generated inventory, procedure, and evidence; roadmap checkboxes remain canonical |
| C-06j rrflowKV scan-resistant cache plan | [`rrflow://rrflow-instance/data/work-package/c-06j-rrflowkv-scan-resistant-cache`](rrflow://rrflow-instance/data/work-package/c-06j-rrflowkv-scan-resistant-cache) | [`c06j-rrflowkv-scan-resistant-cache-engineering-plan.md`](c06j-rrflowkv-scan-resistant-cache-engineering-plan.md) | completed exact-byte scope-aware cache integration and real mixed-family proof; canonical C-06 accepted |
| C-06i rrflowKV adaptive page-compression plan | [`rrflow://rrflow-instance/data/work-package/c-06i-rrflowkv-adaptive-page-compression`](rrflow://rrflow-instance/data/work-package/c-06i-rrflowkv-adaptive-page-compression) | [`c06i-rrflowkv-adaptive-page-compression-engineering-plan.md`](c06i-rrflowkv-adaptive-page-compression-engineering-plan.md) | completed segment-v6 adaptive-LZ4 production slice and fixed-machine integration evidence contributing to accepted C-06 |
| C-06i rrflowKV persisted-filter integration plan | [`rrflow://rrflow-instance/data/work-package/c-06i-rrflowkv-persisted-filter-integration`](rrflow://rrflow-instance/data/work-package/c-06i-rrflowkv-persisted-filter-integration) | [`c06i-rrflowkv-persisted-filter-integration-engineering-plan.md`](c06i-rrflowkv-persisted-filter-integration-engineering-plan.md) | completed segment-v5 persisted-filter production slice and evidence |
| C-06i rrflowKV physical-policy selection plan | [`rrflow://rrflow-instance/data/work-package/c-06i-rrflowkv-physical-policy-selection`](rrflow://rrflow-instance/data/work-package/c-06i-rrflowkv-physical-policy-selection) | [`c06i-rrflowkv-physical-policy-selection-engineering-plan.md`](c06i-rrflowkv-physical-policy-selection-engineering-plan.md) | completed candidate-screen evidence and retained-policy handoff |
| C-06h rrflowKV adversarial qualification plan | [`rrflow://rrflow-instance/data/work-package/c-06h-rrflowkv-adversarial-qualification`](rrflow://rrflow-instance/data/work-package/c-06h-rrflowkv-adversarial-qualification) | [`c06h-rrflowkv-adversarial-qualification-engineering-plan.md`](c06h-rrflowkv-adversarial-qualification-engineering-plan.md) | completed supporting package history |
| C-06g rrflowKV projected-read engineering plan | [`rrflow://rrflow-instance/data/work-package/c-06g-rrflowkv-projected-read`](rrflow://rrflow-instance/data/work-package/c-06g-rrflowkv-projected-read) | [`c06g-rrflowkv-projected-read-engineering-plan.md`](c06g-rrflowkv-projected-read-engineering-plan.md) | completed supporting package history |
