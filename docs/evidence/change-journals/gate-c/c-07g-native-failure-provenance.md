# C-07g native campaign failure provenance

**Status:** active implementation evidence; native failures remain open
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journals/gate-c/c-07g-native-failure-provenance`
**Owner:** [C-07f/g](../../../roadmap/rrflow-1.0/gate-c-07.md), OBJ-02 and POAM-035

Canonical starting revision: 20276f4509250403c4d8df11e1b0fe30f0f6d6ef.
PR 72 pre-edit comments 6361629989 and 6361708504 bind these corrections.
The actual combined characterization used clean b1e08c8170f6ece8fdc8250c08c2db41aef972ec,
optimized ordinary executable SHA256
71afdaf9a66d288def318ca252b494953727f1c3abb2fac6bc342aa35757f868,
and harness SHA256 53de633927ddfc4ebf43486e3654a8fb3d1a6b74eeab13ae6120fa8bfc7c7989.

## Native observation and failure

The explicit Windows launcher bound a four-CPU, 4 GiB aggregate/process Job
allocation on a 24-logical-CPU, 68112736256-byte host. The frozen characterization
used 32 documents, 96 relations, 64 eight-dimensional vectors, four clients,
20 offered requests/second, two actual reopens, and backup/retained-reader lanes.
It is a short characterization, never a 24-hour qualification. The initial
manifest validator rejected an understated 1 MiB I/O reservation; before launch,
the input was corrected to the native observed 16 MiB request reservation.

The run failed after 55.86s. Warmup reconciled 200 attempts and two reopens. The
first measured segment contained 184 acknowledgements, nine uncertain requests
and seven driver concurrency rejections. The workload discarded error details
with `ok().and_then(Result::ok)` and created a failed case with no `faults`.
The strict evidence validator consequently reported missing failure provenance,
masking the request classification. No successful capacity envelope is claimed.
The retained reader's page remained stable; source cancellation released pins
while preserving the 225-byte output, and final release returned output to zero.
Those partial observations do not turn the failed segment into a pass.

Raw local artifacts remain at `target/c07-windows-characterization-01/evidence`:
campaign, progress journal, completed warmup, failed measured segment, result
and outer cleanup receipt. The campaign digest is
76c8c54d2da22c0e02b7a11ac7fab398f96bcb21c9fadf4557148add85434037.
The failed segment digest is
ee7f22fc53487947013117fea633798e19cfa69fe82a4ed1eeb3f9ad397a6ccd.
These workspace artifacts are not substituted for candidate CI artifacts.

## Canonical correction and traceability

`tests/support/sustained.rs` now retains request versus deadline versus driver
saturation outcomes. Uncertain attempts carry sanitized closed error codes and
detail digests. Independent readback failures accumulate one bounded rolling
digest including logical identity and cause, producing the required existing
`Failure` record. The original validator, failed raw segments, thresholds,
attempt limits and lack of automatic uncertain-write retry remain intact.
`support/sustained/workload.rs` projects the typed client error vocabulary;
server messages, transport details and credentials are hashed rather than
emitted. No evidence schema, runtime policy or public operation is added.

Current PR72 macOS job 120332251997 independently timed out awaiting the first
WAL-capacity native error after resumption. This is not proof of a metadata
failure or lost acknowledgement. `support/capacity.rs` now records the selected
pause, actual filler counts/codes and client result class. Missing native events
terminate and reap only the owned engine and retain its exit and bounded 2 KiB
stream tails. Native error, volume/count/time and exact readback requirements
remain unchanged. The actual APFS cause still requires native evidence.

Reviewed: complete completion collection and case construction; workload client
call/error sites; public client error enum; evidence failed-case validation;
progress failure events; Filler fill/cleanup, resume_after_fill, capacity_case;
Child line/kill and Process capture/resume/finish; native I/O observation and
WAL reservation; native capacity launcher admission/cleanup. Existing owners
remain canonical. Research/source adaptation is unnecessary for this reporting
repair; no unverified filesystem explanation is asserted. Stop on relaxing an
oracle, exposing message bodies, unbounded capture, or introducing another
evidence authority.

## Validation and remaining work

`cargo test -p rrflow-cli --test installed_reopen --locked -- sustained::failed_campaign_preserves_closed_cause_and_readback_provenance --exact`
passed: one regression, 0.00s. It first reproduces rejection of absent provenance,
then verifies the corrected record is reported as a real failed case, not green.
Closed-code handling rejects arbitrary detail text. Strict CLI installed-reopen
and crash-feature installed-crash Clippy passed (1.03s) after replacing a flagged
unnecessary unwrap with pattern matching. Formatting initially reported the new
code; cargo fmt applied only changes in the three edited support files.

Changed paths: those three support files, this journal, POAM-035 and generated
navigation/inventory. No implementation was moved/deleted or version changed.
The real load error and macOS native-event timeout remain open until diagnosed
and corrected. The next frozen campaign and candidate CI are required; no
capacity profile, 24-hour soak, C-07 or alpha completion status changes.
