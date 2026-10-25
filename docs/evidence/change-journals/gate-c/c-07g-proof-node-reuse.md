# C-07g authenticated read cost and review correction

**Status:** active implementation evidence; final native qualification pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-c/c-07g-proof-node-reuse`
**Owner:** OBJ-02 / C-07g-h / POAM-035 / PR134

## Boundary and journal correction

[PR134](https://github.com/rrflow/rrflow-development/pull/134) published its
change brief before implementation. Driver: current Codex session. Baseline
public main is `dd225072bcc639c89b0eef968625cc73303ea998`, tree
`0ee0344e039b94ea23f124bb2a50209fc7263882`; opening commit is
`f2abd3d92d192f5f775e7c349b92620e90811190`. PR130's bounded handles are a
prerequisite for the final large-estate candidate; this store change can be
component-tested independently. The only push target is public development.

The earlier PR133 review described selected code paths. Its prose did not
provide an exact full-file/range ledger and is not an exhaustive engine audit.
This successor corrects that evidence limit without rewriting the earlier
immutable receipt. A fresh Git-derived count at main found 20 workspace crates
and 272987 Rust lines including tests. That count is inventory, not reviewed
coverage. Repository-wide review and the full alpha remain open. Neither this
journal nor the earlier plan certifies every engine implementation.

## Retained native failure and measured cause

Frozen candidate `8a6752b255cbd9c42636c506de54de06474776f9`, tree
`d6f05e3db40122517a653eaf9a7f53c838f0e5ec`, binary SHA-256
`aeb6a9b260c3793c24ee8549864b195991a020715e6323f480413ae524bb65ed`,
acknowledged all 100000 corpus operations and independently verified 34191.
The driver then failed `corpus setup exceeded its frozen deadline`, after
7201.18 seconds against the original 7200000-ms setup limit. No load segment
or actual 24-hour soak started. This is incomplete readback, not a demonstrated
lost acknowledgement and not successful capacity qualification.

The private failed estate remains under the owned Linux guest base
`/home/devcontainers/rrflow-c07-native-79663cdd`, directory
`desktop-integrated-files-8a6752b2/rrflow reopen zDA4u8`. Its evidence is
`desktop-integrated-preflight-8a6752b2/evidence`. Retained identities:

| Artifact | Bytes | SHA-256 |
|---|---:|---|
| progress.jsonl | 50121914 | `3523bb4fa34593bd4ba0591016853f35d0344d510d6b2c4f870fec7895bff1d3` |
| result.json | 427 | `2768c75d776aa1ae0644f10b302db99c2941c21a5c7ffcef3ad6d80307b35c7b` |
| campaign.json | 6874 | `b1e0a73c8fe1c6ee7d23429c4d0bedbb2e19eb5c37ba10928c2579978c339fa2` |

The last observation reported 128 open/peak segment handles, zero handle
denials, 574775296 resident bytes and zero failed flushes/compactions. It also
reported 108818953060 decoded bytes and 1421 automatic compactions. Those are
cumulative observations, not a causal explanation or a whole-campaign peak.
The host remained 4 CPUs/8 GiB, with the original 2-GiB engine allocation.

A separate full copy, `handles-readback-diagnostic-1`, preserved the original
estate. The public HTTP diagnostic replay queried 200 corpus records with
vectors, stamped record reads and five-edge adjacency reads. Mean observed
calls were 10.827 ms vector, 14.688 ms record and 19.476 ms relation. The sampled
record/relationship requests used 104/216 accumulator-proof backing reads;
DataFusion reported 881/1070 microseconds. Native perf recorded 178 CPU samples
with no lost samples; segment version selection and native gets appeared.
The sample is diagnostic, not a comparative benchmark or proof that this is
the only setup cost. Raw sanitized output and perf data are in
`handles-readback-diagnostic-1-evidence`; its request script is retained locally
at `target/profile_c07_handle_readback.py`. Credential values were not printed.

## Implemented change and traceability

`access/versioned_read.rs::read_versioned_expanding` now owns one private
`ProofNodes` instance for the lifetime of its captured physical transaction
and read stamp. `open_accumulator` and `authenticate_changes` reuse loaded
(level,index) digests through that instance. Every selected change still runs
`RuntimeInclusionProof::verify_change` against the same stamped root.

The helper contains 128 direct-mapped slots with fixed 64-byte digest arrays,
retaining no heap buffers; its complete size is tested below 16 KiB. A collision
replaces one slot and compares the full identity before any hit. Missing,
malformed and failed loads do not become cached successful reads. No cache
survives the function call, enters persisted state or crosses authorization.
The existing metered reader continues to charge actual backing reads and bytes;
cache hits avoid those reads, not cryptographic verification. Existing budget
and evidence contracts remain unchanged. There is no external dependency,
new endpoint, schema, storage format, global cache or alternate engine path.

Current behavior -> replacement evidence: repeated calls to
`read_accumulator_node` in the two existing proof-loader closures -> scoped
reuse in those closures, with the same core verification; the direct-read
fixture verifies exact changes, retained/current temporal results and native
reopen; the new corruption test performs a second real MX/KV transaction after
replacing a proof node and requires the next read to fail. No source is moved
or deleted. Source changes are the private helper, access module registration,
versioned-read wiring and direct-read characterization test. Supporting
changes are this journal, the existing C-07/POAM035 failure links and generated
navigation/file inventory.

## Review coverage, research and stop decisions

Files reread in full for the changed boundary:

- `crates/persistence/rrd-store/src/access/mod.rs`;
- `crates/persistence/rrd-store/src/access/proof_nodes.rs` (including tests);
- `crates/persistence/rrd-store/src/access/versioned_read.rs`;
- `crates/persistence/rrd-store/tests/direct_read_paths.rs`;
- `crates/compute/rrd-query/src/pipeline.rs`;
- `crates/adapters/rrflow-cli/tests/support/sustained/corpus.rs`;
- `crates/authority/rrd-engine/src/engine/maintenance.rs`.

Other inspected sections, not full-file review claims: store runtime_state
AccessRead/read_stamp_with/read_accumulator_node; runtime repository
read_stamp/read_versioned/read_versioned_expanding; native Database::get and
get_many; core authenticated_log inclusion verification/from_nodes/proof
construction; engine query authorization/stamp pipeline and vector-point read
entry; query execute loading and DataFusion dispatch; installed workload
query/verify/verify_key; transaction admission acquire/drop; write diagnostics
collector and physical diagnostic projection. Existing direct/adjacency test
oracles, README, AGENTS, change-authoring procedure, C-07/POAM035 sections and
prior handle/integration/review journals informed this package. Unread code is
not promoted to reviewed status by the generated inventory or passing tests.

[RFC 9162 section 2.1.3](https://www.rfc-editor.org/rfc/rfc9162.html#section-2.1.3)
requires verification of the inclusion path against the expected root. Reusing
loaded node material within an immutable transaction is a local implementation
decision; the existing algorithm and verification remain. No upstream source
or benchmark claim is copied. Reject global proof caching and skipped proof
verification. Stop for root/transaction mixing, weakened key budgets,
unbounded retained memory, changed format or unexplained semantic regression.
Physical public counters and RuntimeReadEvidence remain the debug signals;
no prompts, credentials or source text enter the new cache diagnostics.

## Failure oracle and validation

The new 32-record oracle failed on unchanged code: 189 proof-node backing
reads. The repair used 63 and returned identical changes on MX, KV and reopened
KV. Exact observed key allowance succeeds; one less fails. The existing
nine-family fixture still matches its independent change-log/snapshot oracle.
Its retained/current counters intentionally changed from 49/89 point reads to
29/40, and decoded bytes from 16336/18074 to 15056/14938. The original exact
counter assertions first failed and were updated to the measured new counts,
not removed. Three helper tests cover collisions, fixed memory, malformed/
missing/error loads and cross-transaction corruption detection.

Executed commands and retained logs under the local repository `target/`:

- `cargo test -p rrd-store --test direct_read_paths --locked adjacent_versions_reuse -- --nocapture`: expected first failure, 189 reads (`c07-proof-first-oracle.log`).
- `cargo test -p rrd-store --lib proof_nodes --locked`: three pass (`c07-proof-helper-corrected.log`).
- `cargo test -p rrd-store --test direct_read_paths --test adjacency_reads --locked -- --nocapture`: two direct-read and three adjacency tests pass (`c07-proof-direct-final.log`).
- `cargo test -p rrd-store --all-targets --locked`: 243 pass, one existing ignored test, 41 test targets, 19.75 s (`c07-proof-suite-0.log`).
- `cargo clippy -p rrd-store --all-targets --locked -- -D warnings`: pass, 8.937 s (`c07-proof-suite-1.log`).
- `cargo test -p rrd-query --all-targets --locked`: 70 pass, 10 test targets, 102.625 s (`c07-proof-suite-2.log`).

Local test environment: Windows, locked offline dependencies,
`CARGO_TARGET_DIR=D:/localdev/devlabs/rrflow/target/endpoint-profile-build`,
`CARGO_PROFILE_DEV_DEBUG=0`, `CARGO_PROFILE_TEST_DEBUG=0`. Command/exit/duration
receipts are retained in `target/c07-proof-commands.json`.

Other surfaced errors: the first corruption fixture used the wrong durability
variant and omitted a borrowed native path; compilation exposed both, and they
were corrected to `Authoritative` and the existing anchored-open signature.
An unused-mut warning was removed before strict clippy. The initial native
profiling helper read the outer plan rather than its installation member;
correcting the field preceded all measurements. The first diagnostic build
used a nonexistent default cargo path; the corrected launch uses the owned
pinned tools and shared build cache. None is represented as a successful run.

`cargo test -p rrd-engine --lib --locked` passed: 206 tests, two existing
ignored native/loom cases, 58.922 s including build (`c07-proof-engine.log`,
`c07-proof-engine-command.json`). Native after-change public observation did not run: the first serve attempt
correctly rejected Cargo's hard-linked output (link count two); serving an
independent copy then failed ProjectBindingMismatch because the existing
installation belongs to the prior executable. Both stderr receipts remain
in `proof-nodes-after-evidence` and `proof-nodes-after-copy-evidence` in the
owned guest. No guard was disabled and no prior installation binding was
rewritten. A newly installed exact candidate is required for public-flow
qualification. The modified-source diagnostic release build passed in
3m 59s; that build alone proves neither readiness nor speed.

Document checks passed: navigation (17 indexes, 608 nodes, 2413 edges),
documentation ownership/links (609 statuses, 607 coordinates), execution
inventory (1732 records), workflow policy and whitespace. Their commands were
`python scripts/knowledge/render_navigation.py`,
`python scripts/ci/check_documentation.py`,
`python scripts/ci/build_execution_inventory.py` and `--check`,
`python scripts/ci/check_workflow.py`, and `git diff --check`. Candidate/merge-queue CI and final
frozen full-size/profile/24-hour qualification remain separate requirements.
No end-to-end performance gain, C-07 closure or full-engine review is claimed
by the component results. The original two-hour failure remains failed.

Change checklist: pre-effect brief and exact baseline bound; canonical owners
and replacement evidence named; scope/risk/research/debug decisions recorded;
smallest failing oracle retained; source and tests implemented at the existing
store boundary; changed code and complete diff reread; command results and
failures retained; successor correction avoids rewriting historical evidence;
no roadmap completion status changes. The stale C-07g foundation row is updated to the latest exact-source result;
older failures remain in their linked journals. POAM035 gains this verified readback
failure and repair reference, while all required native/sustained exits remain
open. PR134 is draft until its dependency and candidate acceptance are ready.
