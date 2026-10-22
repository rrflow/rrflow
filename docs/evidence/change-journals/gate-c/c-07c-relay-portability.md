# C-07c relay portability correction

**Status:** active bounded correction evidence; exact candidate CI remains required
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-c/c-07c-relay-portability`
**Owner:** OBJ-02 prerequisite, [C-07c](../../../roadmap/rrflow-1.0/gate-c-07.md)

## Baseline, failure and brief

This corrects the test relay introduced by the
[installed-history package](c-07c-client-history.md) in
[PR 58](https://github.com/rrflow/rrflow-development/pull/58). Starting revision
`0c31de1340e8f2a5be04d9e40b6b8037dfeabb39`, tree
`30037edcc76792479b68176330ad591e1d6ca126`, had passed local Windows tests.
[Candidate run 39711991250](https://github.com/rrflow/rrflow-development/actions/runs/39711991250)
finished with 15 successful jobs, failed macOS portability and failed ci-gate.
No merge or candidate acceptance followed.

All three isolated receipts passed: Linux/ext4, macOS/APFS and Windows/NTFS,
five cases and two complete histories per host. Those 78 retained attempts do
not override the later ordinary macOS failure. Its fresh-project history
panicked at `ResponseLoss::finish` with `relay read`; existing-project history
passed. The ordinary suite reported four passed, one failed, 418.18 seconds.
The initial diagnostic omitted phase and OS error kind. Its exact native
cause cannot be conclusively recovered from that log.

The pre-edit PR brief was extended to include explicit accepted-socket mode,
bounded interrupted-I/O handling, sanitized phase/kind diagnostics and native
helper regressions in the already executed `lifecycle_process` target. The
production engine, five installed cases, request schedule, 60-second aggregate
deadline and byte bounds remain unchanged. Stop on a further unexplained
failure rather than waive checks or increase timeouts.

## Code review, research and implementation

Read `tests/support/response_loss.rs`, `tests/lifecycle_process.rs`, the changed
installed fixture, `crates/transport/rrd-client/src/transport.rs` and the native
workflow steps. Reviewed the C-07 and CI owners, original immutable journal,
candidate step results and sanitized native failure log. Reread both modified
Rust files and their full diff before commit; only generated indexes/inventory
are mechanically reconciled.

The relay polls a nonblocking listener, but had assumed that an accepted
socket used blocking reads. The primary
[accept API documentation](https://man7.org/linux/man-pages/man2/accept.2.html)
distinguishes Linux and BSD flag inheritance and requires callers to set their
needed accepted-socket flags explicitly. The
[Rust TcpListener API](https://doc.rust-lang.org/std/net/struct.TcpListener.html#method.accept)
does not supply a portable clearing guarantee. No upstream implementation is
copied. This reference behavior maps only to C-07c's test transport boundary.

`forward` now explicitly sets its accepted socket to blocking mode. Shared
`bounded_io` recalculates the remaining deadline for each operation and retries
only `Interrupted`; a genuine timeout, reset or other error still fails. Read,
write and connect use that helper; accept handles interruption inside its
existing bounded polling loop. Diagnostics include only a static phase and
closed error kind, never an OS error message or request/response bytes.

Five additional helper cases cover a forced nonblocking accepted stream with
fragmented header/body, complete response suppression and idle guard cleanup,
interrupted I/O/deadline/error-message redaction, a genuinely stalled native
read, and ambiguous/oversized request rejection before any backend connection.
Every socket belongs to the fixture; denied cases assert no connection to an
owned backend listener. No arbitrary host port is used as a failure sink.
Existing native CI already executes this helper target on all three platforms;
the installed collector still requires exactly five installed cases.

This is a demonstrated portability defect and bounded correction. Reproducing
the flag-inheritance failure mechanism does not establish the discarded errno
of the original macOS event. Any subsequent failure now retains that missing
diagnostic and still blocks acceptance. No production semantics or byte format
is replaced, so replacement traceability is not applicable.

## Validation and checklist

- Before the fix, `cargo test -p rrflow-cli --test lifecycle_process
  relay_normalizes_nonblocking_accept_before_fragmented_request --locked --
  --exact` failed deterministically at the first wait: the relay returned before
  fragmented bytes arrived. Zero passed, one failed, seven filtered, 0.00 seconds.
- After the fix, the first three relay regressions passed in 0.19 seconds.
  The expanded `cargo test -p rrflow-cli --test lifecycle_process --locked`
  passed ten tests in 2.02 seconds. Its one existing ignored child fixture is
  explicitly invoked by parent tests; no new case is ignored.
- `cargo clippy -p rrflow-cli --all-targets --locked -- -D warnings` passed.
  `cargo fmt --all --check` and `git diff --check` passed.
- `cargo test -p rrflow-cli --test installed_lifecycle --locked -- --nocapture`
  passed all five tests in 300.82 seconds, zero failed/ignored/filtered. The
  actual output passed the required-case and lifecycle/history collectors;
  fresh/existing each retained 13 attempts and zero violations. This verifies
  the corrected relay locally, not yet on the failed native platform.
- `python -X utf8 scripts/ci/check_documentation.py` passed with 393 document
  statuses and 391 classified coordinates. Navigation regeneration/check
  passed with 17 indexes, 392 nodes and 1,716 edges; execution-inventory
  regeneration/check passed with 1,375 records. Both projections are refreshed
  after this final journal edit before commit.

Not rerun locally: unchanged evaluation/Python/architecture suites already
recorded in the first journal. Candidate CI must rerun them on the new revision.
Native macOS/Linux isolation and Windows hosted-principal isolation require
their hosted lanes; no local result substitutes. Larger C-07 crash/capacity and
soak cases remain not run, as described in the original package.

The PR owns the updated brief; this linked immutable successor retains the
failed candidate instead of rewriting the earlier journal. Scope and risk stay
inside test qualification; no external service, production capability, schema,
default dependency or version changes. The remaining qualification gap stays
under existing POAM-035. No objective, roadmap package or POA&M item closes.
Full C-07c internal barriers/100-cycle corpus and d-h remain unqualified;
DuckDB runtime remains planned. New exact-candidate native CI is required.
