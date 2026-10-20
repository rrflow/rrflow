# RRFlow inference reference

**Status:** active stable-reference index
**Coordinate:** `rrflow://rrflow-instance/data/reference-index/inference`
**Owner:** provider-neutral model execution, provenance, and model-bound query discovery; linked from `docs/reference/README.md`

Inference is a governed compute capability of the one RRFlow engine. It may
produce proposed vectors or query vectors, but it never becomes a persistence,
search, context, or reasoning authority.

| Subject | Durable warp | Checkout record | State |
|---|---|---|---|
| Embedding, router-model admission, and model-bound search | [`rrflow://rrflow-instance/data/reference/inference/embedding-and-model-bound-search`](rrflow://rrflow-instance/data/reference/inference/embedding-and-model-bound-search) | [`embedding-and-model-bound-search.md`](embedding-and-model-bound-search.md) | provider-neutral embedding/provenance and B-03 router pre-load admission exist; executable routing, durable adapter configuration, and native vector access remain open |
| LFG CPU analysis and SWAG System One controller | [`rrflow://rrflow-instance/data/reference/inference/lfg-swag`](rrflow://rrflow-instance/data/reference/inference/lfg-swag) | [`lfg-swag.md`](lfg-swag.md) | target configuration, dispatch and scoped inference-cache contract; execution unimplemented |
| SWAG learning and Spark preparation | [`rrflow://rrflow-instance/data/reference-index/inference/learning`](rrflow://rrflow-instance/data/reference-index/inference/learning) | [`learning/README.md`](learning/README.md) | target training, evaluation and RROFLOW promotion contracts; hardware and runtime unqualified |
