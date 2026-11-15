# Split the public contract's lib.rs into domain modules

**Status:** active change evidence

**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/repository/contract-lib-split-20261113`

**Owner:** rule R2 of the code structure and conventions standard (kernel package K13). This is the first file split.

**Starting revision:** `development/main` at `501a16bc`.

**Owner direction, 2026-11-13:** "get these properly fully broken down this is a nightmare".

**Change:**
- `crates/transport/rrd-contract/src/lib.rs` goes from 7,669 lines to 332. Its 261 public types and their impls move by exact item spans (syn, with attributes and docs included) into 22 domain modules:
  - `protocol`, `query`, `vector`, `vector_search`, `retrieval`, `points`;
  - `changefeed`, `backup`, `audit`;
  - `catalogue`, with `catalogue/{reasoning, vector, lifecycle, data, platform}`;
  - `openapi`, `estate_authority`, `error`, `identity`, `conformance`, `envelope`;
  - `session`, `data`, `transaction`, `commit`, `health`, `validation`.
- The 850-line `endpoint_catalogue()` becomes five per-domain lists, joined and then sorted exactly as before.
- Each module is re-exported with `pub use module::*`, so every public path is unchanged.
- Private items used across modules become `pub(crate)`.
- Three source-text rules in `workspace_architecture.rs` now read the whole contract crate instead of `lib.rs`, so they still hold wherever a type lives.

**Characterization (unchanged behaviour):**
- `cargo test -p rrd-contract`: 200 passed, 0 failed, the same as before.
- That count includes the test that pins `OPENAPI_DOCUMENT_SHA256` to the generated document.
- `python3 scripts/ci/check_generated_surfaces.py --check`: 83 operations; OpenAPI `4bd01d8b…`, unchanged; 7 signal projections verified.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: clean.
- `cargo fmt --all -- --check`: clean.
- `cargo test -p rrd-engine --test workspace_architecture`: 34 passed.

**Not done here:**
- `lib.rs` is still over the 200-line entry limit, because its header re-exports the older submodules.
- `attunement.rs` (2,055), `function.rs` (1,182), `diagnostic.rs` (1,010) and `websocket.rs` (1,000) stay on the ratchet list for later splits.
