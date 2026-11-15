# D-02: Parse retires deleted files across Steps

**Status:** active implementation evidence; D-02 remains open

**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-d/d-02-parse-retirement-across-steps-20261114`

**Owner:** D-02 in [Gate D](../../../roadmap/rrflow-1.0/gate-d.md); kernel package K5 (source knowledge)

**Starting revision:** `main` at `3f5f2533a`.

**Where this came from.** Absorbing the archived Parse branches (`agent/p3-parse-executor-20261111` and its restack `local/p3-stack`): see the [ledger](../repository/archive-absorption-20261114.md). Their first executor was replaced on `main` by a rewrite, which is better in most ways but dropped one behaviour: the old executor stopped adding retirements when a commit was full. The archive also held the test for it.

**Defect on `main`.** In `engine/attunement/parse/step.rs`, the retirement of every file that left the project went into the batch's base records in one go, outside every bound. Consequences, the second reproduced:

- The selected policy's `max_commit_bytes` ("maximum encoded bytes in one atomic publication group") did not bound a deletion.
- When the store measured that base above the 16 MiB physical batch limit, the Step was refused. The next Step computed the same base. The job could never finish.

It takes very little to reach. The failing test deletes 40 Rust files of 100 three-line functions each, about 140 KB of source, and replans. On `main`:

```text
Parse Step 1 was refused: StorageAdmission("the Parse batch's base records
measure 35857815 bytes against the physical batch limit 16777216")
```

Removing a directory, switching branch, or dropping a parse binding would do the same. Retiring a fact costs far more physically than its JSON size, because it is removed from its indexes.

**Change:**

- `crates/authority/rrd-engine/src/engine/attunement/parse/step.rs`:
  - Each departed file's retirement is one group in the batch, gathered first, within the same commit bound as publication. What does not fit waits for the next Step.
  - Retirements are measured physically from their first group, so the bound reflects what they really cost.
  - Parsing waits until every departed file is retired. A Step that could not publish what it parsed would only parse it again.
  - Parse is finished when every departed file is retired and every pending file has its answer.
  - A single file whose retirement alone exceeds the physical limit is refused by name.
  - `Group` now holds either a retirement or a file's answer; the measurement and the fit calculation are shared functions.
- `crates/authority/rrd-engine/src/engine/tests/attunement/parse_step.rs`: two tests and their helpers (`at_parse_configured`, `finish_parse`, `replan_to_parse`).
- `crates/compute/rrd-attunement/src/parse.rs`, `tests/parse.rs`: `parse_source`, `ParseProposal`, `SyntaxNode` and `PARSE_PROPOSAL_VERSION` are removed. Nothing in the product called them; only their own tests did, so `main` was testing a function it never used. The seven tests now exercise `source_facts`, which the parse worker calls, and so add oracles the real function lacked: the exact output-limit boundary, cancellation in mid-parse, a panicking cancellation callback, the depth limit and invalid UTF-8.

**Traceability.** Sources read in full: the archived `engine/attunement/parse.rs` (first executor) and `tests/attunement/parse_executor.rs`; `main`'s `parse/step.rs`, `parse/publish.rs`, `parse/records/mutations.rs`. The executor code was not ported: `main`'s rewrite measures the physical batch and loses a stalled worker at a deadline, which the archive did not. Only the retirement rule, two tests and the dead-code removal were carried. The two `rrd-attunement` files are the archive's versions; their diff against `main` was read line by line and is exactly the removal and the test retargeting, and `src/facts.rs` is byte-identical on both sides.

**Oracles:**

- `a_deletion_larger_than_one_commit_retires_across_steps`: fails on `main` with the refusal above (committed as `6a2b6d013` before the fix). With the fix it passes: the deletion takes more than one Step, and afterwards only `kept.rs` has a state and only `kept` is a known symbol.
- `an_edit_reparses_only_the_edited_file_and_a_deletion_retires_its_facts`: after an edit and a deletion, the definitions are `after_edit` and `kept`; the change feed shows the second job wrote a live `source-parse` state for `edited.rs` only. This passed on `main` too; it covers a path `main` had no end-to-end test for.

**Commands and results, Windows, 2026-11-14:**

- `cargo test -p rrd-engine --lib --locked`: 481 passed, 0 failed, 13 ignored.
- `cargo test -p rrd-attunement --locked`: 61 passed across its targets, 0 failed.
- `cargo check --workspace --all-targets --locked`: passed.
- `cargo clippy -p rrd-engine -p rrd-attunement --all-targets --locked -- -D warnings`: passed.

**Not established:**

- Whether one real file's retirement can exceed the physical limit by itself. It would need a file whose facts were published in one batch but whose retirement costs more than 16 MiB; I did not measure retirement against publication for the same file. If it can happen, that file's Parse still cannot progress, now with an error naming the file.
- How many Steps a very large deletion takes under the default policy. The test uses a 512 KiB transaction admission to keep it small.
- Not run on Linux here; the development gate runs these tests on Linux. Parse has no supervisor on macOS.
- The supervised worker path is not exercised by these tests; they parse in process, as the other Parse engine tests do.

**Research decision:** none. **Trace decision:** none added. **Roadmap/POA&M change:** none; the defect is fixed in the commit that follows its failing test.
