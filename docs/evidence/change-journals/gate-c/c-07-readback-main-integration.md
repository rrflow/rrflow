# C-07: public readback integration with accepted main

**Status:** active integration evidence; candidate CI pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/c-07-readback-main-integration`
**Owner:** OBJ-02 / C-07 demonstration prerequisite / PR127

The [readback package](c-07-integrated-readback.md) retains its original
implementation and native proof. Its brief declared PR126 as a dependency.
After PR126 merged through successful merge-group run 40144945357,
PR127 integrated public main `f3c04e955fd8851c0fc466c45f008975e9d4c1a1`
by ordinary merge from `df0178eee5245b5b22848e8b0a84dd52c3817fd8`.
No runtime algorithm was rewritten during this integration.

Two documentation conflicts surfaced: POAM-035's independently added
paragraphs and the generated file inventory. Both deficiency paragraphs were
preserved in their existing owner. Inventory was regenerated from repository
tooling after staging the conflict resolutions; its first invocation refused
unmerged index entries, correctly preventing an incomplete projection.
The source delta against accepted main remains the example, its shared
response validation and its real-server test, as scoped in PR127.

Traceability, research, debugging and resource decisions remain in the original
journal; this integration adds no dependency, external capability, format,
authority, deadline change or new research claim. The complete conflict hunks,
resolved POAM-035 section and source delta against main were reviewed. The
existing native demonstration is not relabeled as a run of this merged source.

Acceptance after integration:

- `cargo test --locked -p rrd-client --test rrflowdb_local`: 2 passed,
  including the real installed database relation/atomicity/temporal/reopen flow.
- `cargo test --locked -p rrd-client --all-targets`: 10 passed, none failed;
  example targets compiled and contained no unit tests.
- `cargo clippy --locked -p rrd-client --all-targets -- -D warnings`: passed.
- Formatting, documentation policy, generated navigation/inventory and
  whitespace validation passed. No runtime test failure surfaced.

Linux/macOS and the complete installed capacity matrix were not rerun locally;
PR127 candidate CI remains required. No new native example binary receipt is
claimed for this merge, and previous dirty-source evidence retains its identity.

Checklist: existing brief and owner retained; main synchronized by normal
merge; both conflict intents preserved; generated outputs rebuilt; focused and
owning checks recorded below; no gate or alpha status advanced. Full candidate
CI is required for merge. The sustained-growth failure and actual 24-hour soak
remain C-07 obligations; this client demonstration cannot close them.
