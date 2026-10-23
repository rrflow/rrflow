# C-07c installed publication and reclamation

**Status:** active implementation; native candidate acceptance pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-c/c-07c-installed-publication`
**Owner:** OBJ-02 / [C-07c/e](../../../roadmap/rrflow-1.0/gate-c-07.md) / POAM-035

[PR 78](https://github.com/rrflow/rrflow-development/pull/78) contains the
pre-edit brief. Branch starts at development main
`c148465068f675ed70068b4b0aea60725e3718dc`, integrates candidates through PR77
at `820cbf12fbc5830ffbae47260204c438b36662f6`, and keeps version 1.0.0.

## Change and traceability

The installed diagnostic-primary corpus previously covered five domain
boundaries. Native component coverage additionally exercised publication and
reclamation, but could not establish their installed authorization, durable
intent, receipt and retry composition. Extend the same private crash-test
mechanism and installed history runner; retain every previous domain case.

Add successful segment/successor-WAL sync, before/after CURRENT rename and
before/after collectible-file unlink boundaries. The after-rename/unlink pauses
occur before their following directory sync; they prove process-death ordering,
not loss of device/OS caches. Root/operation digest/nonce/one-use deadline
admission remains mandatory. Unlink selection is limited to native segment,
manifest and WAL objects during the authorized domain scope. Ordinary builds
compile these rendezvous calls out; no public setting can activate them.

The configured low flush target forces real maintenance through the installed
engine. Both fresh and existing histories require prior acknowledged values
and receipts to survive every kill. These maintenance pauses precede the new
semantic batch's WAL, so independent readback must find that target absent
before an explicit retry; retries then produce exactly one stable outcome.
Every case also kills recovery after verified manifest read. This separately
named recovery_manifest boundary handles a published empty successor WAL
without falsely claiming a nonexistent WAL payload pause.

The existing run helper now takes the explicit publication-history mode rather
than duplicating its lifecycle. New native histories each reconcile seven
logical calls, 25 attempts, six uncertain responses, 19 acknowledgements and
12 hit/kill/reap observations with no unresolved outcome or violation. The
three-platform required CI job executes these cases separately and checks both
bounded create-new receipt files, exact points and build identity. Acceptance
requires those named native steps; an older successful job cannot silently
stand in for the new composition. Failure artifacts remain retained.

## Reviewed files and decisions

Read the README/objective/C-07/POAM owning chain and change-authoring routine;
installed_crash.rs authorization/installation/control/readback/ledger runner;
fault_io/crash.rs and crash/installed.rs admission and scope; filesystem.rs
anchored remove/rename and filesystem/io.rs native sync; database.rs recovery,
prepare_write, flush_memtable, garbage_collect and remove_candidates; wal.rs
successor creation/sync; engine transaction.rs enter_domain placement;
ci-reusable.yml installed-crash job, check_workflow.py, acceptance.rs and
acceptance_tests.rs native step requirements. Reviewed the changed source diff
and source ownership. Existing component tests remain characterization owners;
no canonical engine behavior or persistent format is replaced.

Research decision: reuse accepted native publication/fault semantics; no
upstream source or new algorithm. Trace decision: existing bounded private
identity/timing receipts, no paths/content/secrets. Resource decision: existing
owned process/stream/deadline limits and exact cleanup; no disk fill, resource
allocation or long-running campaign is introduced by this package.

## Evidence and checklist

`cargo fmt -p rrd-lsm -p rrflow-cli -p rrflow-eval` passed.
`cargo clippy -p rrflow-cli --test installed_crash --features crash-test
--locked -- -D warnings` passed (20.61s).
`cargo test -p rrflow-eval --lib storage::acceptance --locked` passed six tests,
including rejection of absent required native steps.
`python -X utf8 scripts/ci/check_workflow.py` passed.
A focused native Windows fresh-publication execution is in progress; its
actual result, failures and any corrections will be appended before publication.

- [x] Pre-edit brief and one installed durability prerequisite.
- [x] Preserve prior domain corpus; extend canonical native control boundaries.
- [x] Explicit pre-WAL expectation, compound recovery identity and raw receipts.
- [x] Default-build exclusion and required native CI/acceptance composition.
- [ ] Focused native result, final source/diff/projection review and candidate CI.
- [ ] Complete C-07 profile, retained-owner and 24-hour acceptance matrix.

Stop on missing boundary, changed prior ACK, premature target effect,
unsafe reclamation, duplicate receipt/effect or default-build instrumentation.
No C-07/OBJ-02/POAM-035 closure or full alpha is claimed. Full native suites
and ordinary artifact exclusion remain required candidate checks.


### Native failure and publication repair

The focused Windows installed history failed after 182.86s at retry following
current_before_rename: the reopened engine could read prior data, but a new
publication hit the writer fence. Segment/successor-WAL and their compound
recovery cases had completed. PR refinement 6360398599 binds the production
repair before editing manifest.rs. A subprocess regression independently
reproduced the cause with a restarted process and an interrupted legacy
manifest temporary name: native error 80 / AlreadyExists. A process-local
AtomicU64 alone reused temporary names after restart. Readability did not imply
write readiness. The first failed native attempt is retained as failed evidence.

ManifestStore now admits a fresh 128-bit random writer namespace using the
already locked ring dependency before taking the writer lock. CURRENT,
manifest and checkpoint temporary files retain the create-new primitive and
append the monotonic counter inside that namespace. No interrupted bytes are
overwritten or removed, and publication ordering/fencing remain unchanged.
Entropy failure is explicit; no time/PID-only fallback or format change.
The subprocess regression covers all three families and verifies original
orphan bytes remain unchanged. It passed after the repair (0.05s); the full
manifest target passed 12 tests (0.41s), with one ignored subprocess entry
explicitly executed three times by its parent test. The exact installed
history is rerun next; component success is not its substitute.

The acceptance negative fixture now independently denies both a skipped new
publication execution and missing receipt validation. Its focused test passed;
workflow policy also passed again.


Dependency observation: PR74 job 120301240880 completed successfully with
eight native NTFS capacity receipts at candidate d8ae484068cac70f627f065bbf812782ea8dae05.
Both modes reported native error 112 at WAL write and segment/manifest/CURRENT
open, with exact recovery history checks. This corroborates the earlier filler
correction but does not replace this package's exact-candidate CI or close C-07.
The new manifest subprocess helper now has a 30-second deadline and owned
kill/reap guard instead of unbounded output capture.


### Corrected installed result and publication checks

The exact focused command `cargo test -p rrflow-cli --test installed_crash
--features crash-test --locked -- fresh_installed_publication_and_reclamation
--exact --nocapture` passed on native Windows after the repair (332.37s).
All six maintenance boundaries and six compound manifest-recovery kills
completed: seven logical calls, 25 attempts, 19 ACKs, six uncertain responses,
seven resolved-present operations, zero unresolved outcomes and zero violations.
Independent readback precedes target retry in each case. The full 18,749-byte
receipt is retained locally at target/c07c-publication-local-02/fresh.json,
SHA-256 `d2700fc91166edb383160e3c01529aea5453dc199b072ef315ca6af0f7c1465a`.
It binds executable `7cb782400cb841856b1862bee1232a32caf0a6452fc60f5a67ce2d32e5299f16`,
Windows x86_64, Rust 1.99.1, unoptimized debug crash-test build with null
release provenance. This is a local source-candidate characterization, not
ordinary-binary performance or exact committed candidate acceptance.

After the repair, strict rrd-lsm all-target Clippy passed (5.47s), and the
bounded subprocess regression passed again (0.09s). Strict diagnostic primary,
installed_crash and lifecycle_process Clippy passed (10.98s); rrflow-eval
all-target Clippy passed (15.82s). Generated parity remained unchanged at
44 operations/OpenAPI 03639d8aff602c966b800becaa5b8442d0e912904cb50383f1ecee86ad32fb9a.
Final source/diff review covered nonce admission before lock acquisition,
all three temporary-name families, native effect placement, ordinary cfg
exclusion, pre-WAL expectations, owned kill/reap and raw CI step requirements.

No local existing-mode run, Linux/macOS installed run, real disk fill or soak
was executed for this package. The required candidate matrix owns those native
mode/platform results. C-07 and POAM-035 remain open. Navigation/inventory are
regenerated after this entry; publication is a candidate, not a merge.
