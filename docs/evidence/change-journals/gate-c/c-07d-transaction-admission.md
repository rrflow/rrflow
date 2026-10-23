# C-07d finite transaction admission and reclamation backpressure

**Status:** active implemented candidate; native acceptance pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-c/c-07d-transaction-admission`
**Owner:** OBJ-02, C-07d, POAM-035

Baseline `6dd44d4879cfcd121e4bcab967ce9df3748e394a`, tree
`b5d6954561a93ffb97c18a002d3685c08629eb7a`.
[PR 65](https://github.com/rrflow/rrflow-development/pull/65) carried the
finite-admission continuation brief before implementation.

The former unbounded transaction Mutex wait becomes one FIFO gate with
configured outstanding request count, logical serialized bytes and wait time.
A bounded counting writer avoids a second serialized request allocation.
The active request retains its charge. Reservations are released on timeout,
pending cancellation, unwind or normal guard drop. Panic in the active effect
owner fences the gate, preserving the former poisoned-Mutex safety behavior.
Existing invocation reservation identity now also carries its monotonic deadline
and explicit pending-cancellation flag; a cancelled old handle cannot cancel a
new invocation reusing coordinates. Cancellation does not undo admitted effects.

Canonical replacements: core constructors and the existing gates in domain
transactions, unique-index installation, function catalogue/execution, prompt
flights, task commits, and operator claim assertion. Mutual exclusion and
existing commit, authorization, audit and recovery authorities remain intact.
Installed storage policy adds three required controls, propagated by the current
configuration/locator contract and generated language capabilities. Content-free
native snapshots report current/peak request and byte charge. These logical
charges do not establish transport preparse, heap/RSS or aggregate host limits.

Review also identified a maintenance-debt progress gap: partial reclamation
previously admitted further growth. ReclamationPending now defers the incoming
write before WAL append while each bounded retry drains work. It propagates
through native/store/engine typed pressure and confirmed-absent intent cleanup.
The partial-reclamation definition now retries with a fixed bound and checks
that denied attempts do not advance the visible sequence.

CI failure retained: candidate 6dd44d48, run 39884653587, kernel-storage job
120190336771 and optional-native job 120190337001 both failed the old
`!first.removed_wals.is_empty()` expectation. Automatic pre-write GC had already
removed the initial unreachable WAL. The characterization now asserts that
initial WAL is physically absent and the exact physical WAL set equals the
retained current/checkpoint closure. It does not waive reachability validation.

Read/review scope: owning README/C-07/POAM-035 and procedure; complete new
transaction_admission implementation; prior gate call sites and surrounding
commit/authorization logic; invocation begin/completion/coordinate identity;
core configuration binding; typed native/store/engine error paths; GC pending
work and removal ownership; maintenance and compaction characterizations;
StorageLimits, installation fixture construction, and generated SDK defaults.
Replacement sources stay at their canonical owners; no parallel mutex, queue
service, compatibility lane or implementation tree was retained.

No external source adaptation/research was needed. Trace/resource decisions:
preserve typed errors and durable receipts, expose content-free charge, bound
waiting without creating a second durable lifecycle, and distinguish explicit
pending cancellation from transport disconnect and rollback. Stop on bypassed
serialization, post-effect admission, hidden fallback or failed candidate CI.

Validation:
- `cargo check -p rrd-engine -p rrflow-cli --all-targets --locked`: passed,
  9.60 seconds after correcting the missed operator gate call site.
- `cargo clippy -p rrd-lsm -p rrd-store -p rrd-contract -p rrd-engine -p rrflow-cli -p rrflow-eval --all-targets --locked -- -D warnings`:
  passed, 11.09 seconds after replacing a test default-field reassignment with
  an explicit initializer.
- Typed offline installation and synthetic storage projection refresh, all five
  SDK generators, Rust/Go formatting, and generated parity completed. OpenAPI:
  `d41937932617054902fc4519cc755c5f37ba8cab154667f4b46e006bd12580e0`.
- New definitions compile for count/byte denial, timeout release, pending
  cancellation, subsequent acquisition, expired invocation and panic fencing.

Local runtime suites and soak were not run, as directed by the owner. Required
candidate CI remains the merge gate. No native capacity or alpha completion is
claimed from compilation. The source and generated diffs were reviewed; the
linked index and execution inventory are regenerated before commit.
Checklist: owner and pre-effect brief bound; source/replacement/failure oracles
identified; research/debug/resource decisions recorded; runtime definitions
preserved/extended; surfaced errors retained; configuration projections updated;
no roadmap or POAM status closure. Recovery/headroom, actual exhaustion,
capacity profiles, aggregate-host qualification and real soak remain C-07 work.
