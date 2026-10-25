# C-07f: maintain allocation pressure during metadata fill

**Status:** active implementation evidence; native candidate qualification pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/c-07f-metadata-pressure`
**Owner:** OBJ-02 / C-07f / POAM-035 / PR122

## Brief and traceability

[PR122](https://github.com/rrflow/rrflow-development/pull/122) was opened from
public main `94d0567fb865fb49a90aaeeef4b5861707c9f8a7`, with opening revision
`7f0d43819f4d812250e2e90db7951862ac305fb6`, before implementation. The failure
oracle is PR117 run `40124044612`, macOS job `120962408662`: at
`manifest_before_create` in fresh mode, all 8192 metadata files were created
without a native capacity error. The original count limit correctly failed
instead of pretending to exhaust the filesystem.

The fixture filled data once, then created metadata files. The target keeps
that same owned data file full while metadata transactions proceed. Every
128 successful metadata creations on macOS, `fill_remaining_data` drains
newly available allocation through the existing `fill_data` owner. Every drain
shares the initial 25-second clock and remaining aggregate byte allowance.
There is no new file-count allowance, timeout, injected error, host-wide fill,
engine policy or durability change. Ext4 inode-first and NTFS MFT behavior
remain intact. The actual selected engine syscall still must return a native
capacity code; prior acknowledgements and target atomicity must survive recovery.

Changed: `tests/support/capacity.rs`, this linked record and owner links.
Generated: navigation and execution inventory. No implementation moved or
removed. `Filler::fill`, `fill_metadata`, `fill_data`, `Drop`, native selected
error/resume and capacity-case recovery were read, along with existing filler
tests and the owned image runner. The new helper extends that same fixture;
there is no alternate capacity driver or runtime integration.

## Research and diagnostic decisions

The [Apple File System Reference](https://developer.apple.com/support/apple-file-system/Apple-File-System-Reference.pdf)
describes copy-on-write metadata and transaction-associated free queues.
Space becoming reusable during later metadata transactions is an inference
to test, not a proven cause of the failed CI machine. No Apple implementation
was copied. A refill count and cumulative data bytes are included in the
existing bounded fixture diagnostics and receipt. Private engine contents are
not logged. A post-change native failure remains a failure and must be retained.

Stop conditions: no increased limits, unsupported native success claim,
unsafe cleanup, modifications outside owned filler entries, or engine effect
changes to make a fixture pass. Native CI must resolve the hypothesis; unit
control-flow evidence cannot prove APFS exhaustion.

## Verification

The first attempted installed-crash command omitted its required `crash-test`
feature and was rejected by Cargo before tests ran. The command was corrected.
The focused refill test passed: allocation released between intervals remains
charged to one allowance, and an expired original clock cannot write or sync.
The existing delayed-allocation, partial-write and noncapacity-error tests
remain in the owning installed-crash target. Operator command logs are retained
under `target/c07-pr122-evidence/`.

Windows acceptance commands (locked dependencies, shared target, debug symbols disabled):

- `cargo test -p rrflow-cli --test installed_crash --features crash-test --locked metadata_refills_share -- --nocapture`: one passed; 37.91 seconds including compilation.
- `cargo test -p rrflow-cli --test installed_crash --features crash-test --locked`: eleven passed, one explicitly owned-volume native helper ignored; 146.07 seconds test runtime. Fresh/existing domain, publication and reclamation histories passed.
- `cargo clippy -p rrflow-cli --test installed_crash --features crash-test --locked -- -D warnings`: passed; 20.69 seconds.
- `cargo fmt --all -- --check`, `python scripts/ci/check_documentation.py`, `python scripts/knowledge/render_navigation.py --check`, `python scripts/ci/build_execution_inventory.py --check` and `git diff --check`: passed before publication.

The opening-only CI run `40129165473` was canceled while queued, before the
implementation candidate, to avoid unnecessary runner competition. Native
APFS exhaustion/recovery is not available on this Windows host and remains
an explicit candidate CI requirement, as does the three-platform matrix.
Capacity profiles and 24-hour soak remain unqualified. C-07, POAM-035 and
alpha status are unchanged.

Checklist: owner and brief before edits; concrete native first failure;
canonical fixture boundary and unchanged safety limits; primary-source
research with explicit inference; bounded diagnostics; focused then owning
validation; missing checks and failure recorded. The full changed source file was reread and the
complete diff reviewed before candidate publication. No merge or native acceptance is
claimed by this implementation record.
