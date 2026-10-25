# C-07g proof reads and handle ownership integration

**Status:** active immutable integration evidence; native characterization pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-c/c-07g-proof-handle-integration`
**Owner:** OBJ-02 / C-07g-h / POAM-035 / PR134

[PR134](https://github.com/rrflow/rrflow-development/pull/134) published the
integration extension before the ordinary merge. Starting head was
`ee18ef54abc318221496c205082dbee07412841b`. Incoming PR130 head is
`f4a38d5c203cbfa8d1dbfd976b841c8a96071d15`, including PR131
`6911f397b33b9282acad2c5be437cb811ba6740c`. The earlier
[proof-node repair journal](c-07g-proof-node-reuse.md) owns its initial baseline,
research, exact review coverage, first failing oracle and original native
readback failure. This successor preserves that immutable record.

## Brief and implementation traceability

Combine the declared bounded-file prerequisite with transaction-local proof
reuse so a fresh installed candidate can exercise both at full corpus size.
PR130's IoContext/SegmentFile retains anchored identity, bounded native leases
and public counters. PR131's capacity.rs/capacity_metadata.rs retains native
allocation settlement and its failure/recovery tests. PR134's private
ProofNodes and versioned_read closures retain per-read material reuse while
every inclusion proof is verified. No format, timeout, resource ceiling,
installation binding, native exhaustion oracle or semantic authority changes.
No implementation is removed. Stop for a source conflict or weakened oracle.

There were no source conflicts. Only the generated execution inventory
conflicted; its Git index was resolved before regeneration. Git object
comparison verified all 45 incoming non-document paths match PR130 exactly,
including generated schemas and clients. Both POAM failure histories and all
three incoming immutable journals survive. This new journal, its generated
index and inventory are the integration additions. The roadmap remains open.

Review: full new proof helper and direct-read tests were reviewed in the
preceding journal. This integration reread the full handle pool, handle-pool
unit tests, segment_handles integration test, metadata settlement helper and
its tests, incoming handle/integration journals, change-authoring procedure,
Desktop preparation/launch scripts, and sustained/corpus.rs. Relevant incoming
diffs for native identities, segment open/read, installed policy lowering,
public diagnostics, error conversions and capacity scheduling were inspected.
This is dependency integration review, not an exhaustive engine audit.
Mechanical byte equality is preservation evidence, not proof of correctness.

Research and trace/resource decisions remain with the two owning repair
journals; integration adds no algorithm or borrowed source. Public physical
counters, exact native receipts and original failed estate remain intact.
The fresh preflight must seal the integrated source/tree, executable, harness,
configuration, corpus and allocation before load. It must create a fresh
installation; replacing an executable in the original sealed installation is
not an accepted measurement path.

## Validation and limits

Executed on Windows with locked offline dependencies, shared target
`target/endpoint-profile-build`, dev/test debug disabled:

- `cargo test -p rrd-store --test direct_read_paths --test adjacency_reads --locked`: five passed.
- `cargo test -p rrd-lsm --test segment_handles --locked`: two passed, 29.69 seconds test time. Windows does not exercise Unix RLIMIT; Linux/native evidence remains separate.
- `cargo test -p rrd-store --all-targets --locked`: 243 passed, one existing ignored, 23.000 seconds including build.
- `cargo test -p rrd-engine --lib --locked`: 207 passed, two existing ignored, 33.422 seconds including build.
- `cargo clippy -p rrd-store --all-targets --locked -- -D warnings`: passed, 4.907 seconds.

Logs: `target/c07-proof-integrated-focused.log`,
`target/c07-proof-integrated-suite-{0,1,2}.log`; command receipts:
`target/c07-proof-integrated-commands.json`. No integrated test failure was
observed. PowerShell formatted Cargo stderr as NativeCommandError in the first
redirected log even though the processes returned zero; subsequent commands
used Python direct subprocess capture. One GitHub queue query failed shell
quoting and was replaced with structured JSON input. Neither is hidden as a
successful invocation.

Full candidate and protected merge-queue CI, final native full-size readback,
profile repetitions and actual 24-hour soak have not passed for this candidate.
The earlier 100000-ACK/34191-readback two-hour failure remains failed. The
original operation deadline, two-hour setup deadline and resource/corpus limits
are preserved. No after-change installed performance gain or alpha closure is
claimed. Dependency PR130 remains subject to its complete candidate pipeline.

Checklist: pre-effect brief published; exact parents and owner identified;
canonical implementation/test preservation mapped and checked; only generated
inventory conflict resolved; focused and owning checks run; failures/limitations
recorded; source/diff review scope explicit; linked successor journal supplied;
no roadmap or POAM closure; protected candidate/merge checks still required.

Documentation validation first rejected the journal status without the required
active/historical classification; it was corrected to active. Navigation,
documentation, execution inventory, workflow, format and whitespace checks
then passed. This classification is record metadata, not gate completion.
