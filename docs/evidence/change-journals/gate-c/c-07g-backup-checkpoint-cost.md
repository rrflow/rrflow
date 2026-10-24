# C-07g: bounded logical backup checkpoints

**Status:** active implementation evidence; installed sustained acceptance pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/c-07g-backup-checkpoint-cost`
**Owner:** OBJ-02 / C-07g / J-04 / POAM-035 / PR117

## Brief and measured trigger

[PR117](https://github.com/rrflow/rrflow-development/pull/117) was extended before
editing this measured backup cost. Starting revision:
`7f331491dfa35d8e6bbae9aa7b80b790c5b6726b`. This continues the same storage-cost
repair; it does not add a planning authority or alter version 1.0.0.

The preceding digest candidate `d512b932aabe50c71e7efcba270212e7f16fe1fe`
built optimized Linux in 247.5 seconds. Ordinary executable SHA-256:
`2e209292deb157d88b72993d0fae8019c5b59276af4fde161e40909f8eb81867`;
harness SHA-256:
`6092ffa26d513bca5dfb196b3fc255451a133b603a895f9bf44bdb2b61f66547`.
Its 1,000-record / 5,000-edge / 1,000-vector (384-dimensional) characterization
used four clients, 5 RPS, seed 7, four CPUs and a 2-GiB aggregate cgroup on a
32-GiB WSL host. It is not the isolated 8-GiB Desktop qualification profile.

The run completed, including final readback/restart and confirmed cleanup, but
failed its operating point: 577 acknowledged, 23 rejected (22 driver-slot,
one storage conflict), 24 latency overruns, zero unresolved effects and zero
missing metrics. Launcher elapsed 276.52 seconds includes a validator rebuild;
do not compare it as engine runtime. Backup creation took 11.892 seconds for
1,014 actions, plus 0.167 seconds verification; the segment barrier delayed
next dispatch up to 4.935 seconds. Default export synchronizes the archive and
publishes a durable receipt for every action. This observation motivates the
repair without asserting it explains every failure or all growth costs.

Native evidence remains at
`/home/devcontainers/rrflow-c07-native-79663cdd/digest-5rps-growth-1000-d512b932`;
small copies and derived timing analysis are retained operator-locally under
`target/c07-pr117-native-evidence`. Exit zero meant valid collected evidence,
not passing capacity. A subsequent 10,000-record characterization acknowledged
all 10,000 writes and independently verified 4,352 records before an explicit
operator interruption at 279.15 seconds to avoid spending another long cycle
on the older candidate. It is incomplete, not a timeout failure or passing
result. Its `operator-interruption.json` records exact launcher identity and
verified removal of the owned cgroup; raw launch/progress receipts remain
unchanged. No 24-hour soak started.

## Canonical boundary and replacement map

`crates/persistence/rrd-store/src/archive/export.rs` owns this repair. The
public default export and explicit with-progress entry point call the same
private implementation with an explicit internal checkpoint policy. The default
used by application backup checkpoints at 64 actions or 1 MiB accumulated
payload, whichever is reached first. One admitted action may cross the byte
threshold; its existing MAX_ACTION_BYTES limit still applies. Pending counters
retain no extra action payloads. The final partial batch is checkpointed before
the existing footer sync and atomic publication.

The explicit with-progress API retains every per-action durable callback and
its interruption semantics. No archive format, digest, receipt schema, source
cut, restore policy, callback name, application-backup coverage, normal
transaction acknowledgement, or transaction coordinator changes. Only private
export checkpoint frequency changes; an unacknowledged incomplete archive is
never treated as a published backup. Complete prefixes still reconcile with
older receipts; invalid/torn bytes remain fail-closed rather than acquiring a
new repair lane.

Traceability: the previous per-action writer sync/receipt sequence becomes one
local checkpoint closure, called by the same export implementation. Existing
source scans, receipt validation, prefix comparison, source-advancement denial,
footer sync and rename remain in place. Existing resilience tests are retained;
new boundary tests exercise multiple batches and byte-triggered checkpoints.
Stop on identity/callback drift, lost source validation, publication before
sync, or extra unbounded retained state.

Files read: README/AGENTS and C-07/POAM035/authoring routine from the
parent package; export implementation, format writer/replay state and prefix
reader, receipt validation and publication, logical_archive_resilience tests,
rrd-store manifest; application-backup snapshot/export path and engine backup
entry points; installed workload, backup lane and segment scheduling. Research
decision: this is a code-grounded correction of per-action persistence cost,
not an upstream algorithm adaptation or a claim of novel reasoning. Existing
progress receipts and native timings provide attribution; no runtime telemetry
side channel is added.

## Verification

- Existing first oracle: `cargo test --locked -p rrd-store --test logical_archive_resilience --jobs 6`
  passed all 15 cases in 3.45 seconds, including all canonical runtime families,
  audit round-trip, interrupted export/restore and source-advancement denial.
- `cargo test --locked -p rrd-store --lib archive::export::checkpoint_tests --jobs 6`
  passed three cases; the manual measurement is ignored. A stale unused test
  import produced one warning, was removed, and subsequent strict lint passed.
  Mixed 130-action history checkpoints at 64/128/130, preserves per-action API
  callbacks, restores claims/runtime history and matches archive bytes. Six
  injected interruptions cover archive-sync and receipt-publication boundaries,
  including the tail. Five 400,000-byte payloads prove byte-trigger/tail behavior.
- `cargo test --locked -p rrd-store --jobs 6`: 235 passed, zero failed, one
  deliberately ignored manual measurement.
- `cargo clippy --locked -p rrd-store --all-targets --jobs 6 -- -D warnings`: passed.
- `cargo test --locked -p rrflow-cli --test installed_reopen --jobs 6 -- sustained::installed_capacity_workload_smoke --exact --ignored --nocapture`:
  passed in 8.52 seconds. Actual records/relations/vectors, backup under writes,
  retained-reader drain, termination/reopen and replay remain verified.
  Log: operator-local `target/c07-backup-validation.log`.
- `cargo test --locked -p rrd-store --lib archive::export::checkpoint_tests::bounded_checkpoint_measurement --jobs 6 -- --exact --ignored --nocapture`:
  passed, identical inventory/digest. Windows native debug build, 512 actions:
  per-action 512 checkpoints / 1,004,512,000 ns; bounded eight checkpoints /
  142,243,500 ns. Source creation is outside the timer. This single development
  measurement is not a release SLA or full capacity qualification.
  Log: `target/c07-backup-measurement.log`.

## Change-authoring checklist and limits

- [x] Bind outcome, exact source boundary, first oracle, risk and stops in PR before edits.
- [x] Retain failed operating points and incomplete growth characterization under original identities.
- [x] Preserve one implementation, transaction authority and public archive/callback contracts.
- [x] Validate independent round-trip bytes and both durable interruption boundaries.
- [x] Run owning suite, strict lint and installed flow without changing acceptance limits.
- [ ] Complete new candidate CI and native sustained comparison before acceptance claims.

Changed paths: export implementation/tests, this journal, existing C-07 and
POAM035 links, generated navigation and inventory. C-07/POAM035 stay open.
Full profile matrix, 24-hour soak, query-growth optimization, comparative
reasoning quality/token savings and accounting UI are not proved by this repair.

Final authored diff review, formatting, documentation policy, generated navigation, 1,594-record execution inventory and staged whitespace checks passed. Native sustained comparison and candidate CI remain pending at commit.
