# C-07: segment encoder working set

**Status:** active implementation and component evidence; installed qualification remains open
**Coordinate:** `rrflow://rrflow-instance/data/evidence/c-07-encoder-working-set`
**Owner:** OBJ-02 / C-07d,g / POAM-035 / PR128

## Bound package and checklist

[PR128](https://github.com/rrflow/rrflow-development/pull/128) was opened with
its reviewable change brief before implementation, from public main
`94d0567fb865fb49a90aaeeef4b5861707c9f8a7`. Opening commit
`2823e20c19ff53dde2d3e1095dfc0ca782ffce4d` and ordinary integration of PR126
produced starting source `5a3142ceee90ee7f28b85a8583e5637deb44ee8b`.
This package owns encoder allocation at the existing physical boundary.
No product version, segment format, durability ordering, transaction owner,
resource cap, timeout, schema or external dependency changes.

- [x] Bind outcome, C-07 and POAM-035 before editing.
- [x] Inspect current implementation, failed native evidence and existing diff.
- [x] Define exact-byte oracle and source-preservation map before replacement.
- [x] Record research, trace, resource and debugging decisions.
- [x] Run the smallest characterization before the owning storage suite.
- [x] Retain failed measurements and distinguish them from accepted evidence.
- [x] Keep created/changed paths and generated indexes reviewable.
- [x] Complete final documentation/navigation/inventory validation and diff review.
- [ ] Complete candidate CI and installed growth qualification; no closure claimed.

Complete function reads cover `RowGroupBuilder`, `encode_versions`,
`append_page`, `Database::prepare_write`, write/idle compaction selection and
merge/reclamation, `Segment` record cursor/page-cache/write paths, and
`Memtable::from_versions`. The complete server maintenance worker was read,
as were the installed corpus seed loop, failure retention, this package's
encoder test module and generated change diff. README and the change-authoring
procedure were read; C-07, POAM-035, native qualification and prior growth
journals were inspected in relevant sections, not represented as full reads.

Implementation paths are `crates/persistence/rrd-lsm/src/segment/format.rs`
and new sibling `format_encoding_tests.rs`. Supporting changes are this
journal, C-07 and POAM-035, their generated navigation and file inventory.
No file or requirement is deleted. Stop for changed bytes/format, unsafe buffer
access, authority duplication, weakened qualification or unrelated scope.

## Retained failure and attribution

Exact tested source `c31ed202179cfd63bef1d13b61ff424bfb99b1fd`, PR126 head
`d0dded5184d2508e6654f00ed0eb99d8e0200190`, passed all 23 candidate CI checks
in [run 40139825377](https://github.com/rrflow/rrflow-development/actions/runs/40139825377).
The canonical collector receipt SHA-256 is
`2669a488f046131d1c6c51547fd2855af1c996cb7db5e2b1af59e8f6dd0497fb`.
All 24 retained selected native capacity receipts have zero unresolved entries
and violations. That is CI evidence, not sustained profile acceptance.

Its full Desktop preflight failed after 3,929,933 ms at **69,246 acknowledged
records / 69,248 intents**, on the unchanged 30-second corpus-write deadline.
The continuation refused qualification. **No actual 24-hour soak started.**
The private original estate was retained and left unchanged. At failure,
RSS was 438,108,160 bytes, private memory 485,892,096, 818 compactions and
111 flushes had completed with zero failures, and 867 segments occupied
1,944,916,289 bytes. There was no reported OOM, fencing, allocation denial,
active view or transaction. This rules out neither lock delay nor other costs.

Operator `target/c07-c31-failure-evidence/manifest.json` binds retained raw
progress, result, launch, continuation and sanitized profile artifacts.
Result SHA-256: `7ab8933932a66fbb47b753821e27518ad2f241902b46193be4447bda6a6c66e2`;
progress: `5f7efe1ad3555b08257248eaeb2d9427fa5f9c2d30b99eba8b34783189134e43`;
perf report: `d388e16fb619d38c0cd0686aa0d84ef89aee0ff550f7cced6a75f635a56199cd`.
No private estate, credential or raw stack-memory capture is published.

A 25-second native `perf record -F 99 -g --call-graph dwarf` capture on a
separate estate copy attributed 23.33% self samples to kernel page
initialization, with reallocations under `RowGroupBuilder::append` and
`append_page`; LZ4, memcpy and frees were also visible. The copy's server was
terminated after profiling. This identifies useful allocation work to remove,
not the sole cause of the original deadline.

Research is primary local code and measured native behavior. No upstream code
was copied. Existing failure and capacity contracts remain the source of the
acceptance bar. Trace uses existing perf and resource/maintenance counters;
there is no new runtime lifecycle or unbounded global cache.

## Replacement and proof

The previous encoder retained every row-group payload before encoding and
allocated initialized LZ4 scratch per page. The same encoder now flushes each
completed row group into the output and resets its column buffers for reuse.
Only row-group descriptors remain for the index. One initialized scratch
vector grows as needed within admitted page limits; compression receives the
exact required slice and output uses only its returned length. Ownership ends
with this encode call. Whole output and input remain live: this is not a claim
of constant total memory or a new streaming format.

Six digests were captured with the original encoder before replacement. Raw,
adaptive and raw-fallback policies, two group budgets, mixed payload lengths,
tombstones and multiple versions must retain exact encoded bytes. The new
encoder passed all six. Existing independent MVCC/compression, crash,
maintenance, transaction and reopen checks remain replacement evidence.

| Command | Result |
|---|---|
| `cargo test --locked -p rrd-lsm --lib segment_v6_encoding_matches_pre_reuse_bytes -- --nocapture` | Old characterization and new exact-byte oracle passed. |
| `cargo test --locked -p rrd-lsm --all-targets` | Passed on Windows; library 48 passed / 2 intentional ignores, all integration targets passed. |
| `cargo clippy --locked -p rrd-lsm --all-targets -- -D warnings` | Passed. |
| Native release `encoding_working_set_profile --ignored --nocapture --test-threads=1` | See paired measurements below; diagnostic, not capacity acceptance. |

## Native measurement integrity

Initial operator preparation hit two script syntax errors and an incorrect
Cargo path; these were corrected before measurement and their logs retained.
The first baseline/candidate comparison was **invalid**: a shared Cargo target
reused the old executable after switching extracted source roots. Identical
RSS/page-fault behavior triggered inspection; the candidate artifact reported
`fresh: true`. Those timings are not used as optimization evidence.
A separate candidate target directory and explicitly retained, different
binary hashes correct this. Alternating baseline/candidate order reduces time
ordering bias, but three local trials are not a performance guarantee.

The isolated release build completed in 27.56 seconds and reported a freshly
compiled candidate artifact. Baseline binary SHA-256 is
`34a0faace847c8086ff9e10daf8ba4dfc9fe9e9621ee36ca8a35b6561868f573`;
candidate is `070877b107f5028f3ab9c21d492b1ab693d52b7dcc7e5a1e0ec0eb25e30baf2f`.
Each trial encoded 2,048 keys with three versions, 32 times, producing
358,833,152 bytes with checksum
`56bd3dabfbdea5a4ddbd69fca3d887623dec505f2d4d956f23095a9fbe373aac`.

| Trial | Baseline peak RSS KiB | Candidate peak RSS KiB | Baseline encode ms | Candidate encode ms |
|---|---:|---:|---:|---:|
| 0 | 72,556 | 45,748 | 1,363 | 1,385 |
| 1 | 72,540 | 45,772 | 1,520 | 1,331 |
| 2 | 72,488 | 45,704 | 1,691 | 1,842 |

Median peak RSS fell about 37%; minor faults fell from approximately 20,462
to 13,459. Timing varies in both directions, so no throughput improvement is
claimed. These process measurements include fixture/input/output and do not
prove aggregate engine bounds. Guest `encoder-paired-evidence/` retains the
six raw trials, build output and executable identities. Source archives are
bound by SHA-256: baseline
`fc6a6b5d77de0f899a50e1a5a70e33364d3db56b454cc259592401c2ae63af32`,
candidate `8cfa9ad5340d1025ae81046c46725a0bcf778c1a24563fb76e6f338f4b4cc7f3`.

The first documentation whitespace check detected Windows CRLF introduced by
operator text writes; files were restored to LF before final validation.
Documentation validation then rejected a status lacking `active` or
`historical`; the journal now declares `active`.

Final `cargo fmt --all -- --check`, documentation-policy, navigation and
execution-inventory checks and `git diff --check` passed. The complete changed
source/test diff and supporting owner/journal edits were reread. Native paired
raw evidence was copied to operator `target/c07-encoder-paired-evidence/` with
a 24-file SHA-256 manifest. No profile or capacity closure is inferred.

## Remaining work and status

The original installed deadline, complete Desktop preflight, profile matrix,
independent readback, 100 reopen cycles on the new candidate and actual 24-hour
soak remain required. Component checks and encoder measurements cannot close
C-07. Other operating systems and cross-engine integration are delegated to
candidate CI; the entire workspace was not rerun locally because the affected
physical storage suite is the owning first oracle. POAM-035 remains open;
its latest failure is now recorded with this remediation. No alpha or gate
status advances. PR128 candidate CI and merge evidence are pending.
