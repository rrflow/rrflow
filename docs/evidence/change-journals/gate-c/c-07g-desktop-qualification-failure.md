# C-07g: nominal Desktop corpus-write deadline failure

**Status:** active retained failure evidence; qualification remains open
**Coordinate:** `rrflow://rrflow-instance/data/evidence/c-07g-desktop-qualification-failure`
**Owner:** OBJ-02, C-07g, POAM-035
**Pull request:** [111](https://github.com/rrflow/rrflow-development/pull/111)

This immutable continuation starts at
`b1d1fb55b9cd61df29e4b1908803196b111b3136`, tree
`52aa8357d1e8e6c59eb3be6157341efd9f1641db`. The PR continuation brief preceded
these evidence edits. The [launch record](c-07g-desktop-qualification-launch.md)
remains unchanged. No engine code, frozen input, allocation, operation deadline,
acceptance threshold or source revision was changed; no campaign was retried.

## Observed failure

The read-only operator inspection at 2026-10-24 03:24:16 UTC found the guest
service failed. Launcher elapsed time was 6202458 ms, harness exit 101, and
`cleanup_confirmed` was true. The failure occurred during seed-7 corpus setup:
50688 intents and 50685 acknowledgements, with last acknowledged ordinal 50684.
No completed corpus verification, lifecycle phase, accepted segment, or 24-hour
soak phase exists for this campaign. `qualification_passed` is false.

The harness stderr states `installed sustained driver failed: Err("corpus
write deadline")` at `tests/support/sustained.rs:967`. The result limitation is
`installed-driver-failed`; launcher reason is `launch-failed`. These observations
do not establish an out-of-memory failure or identify a proven root cause.

Retained identities:

- Source `c206c270ee5a2d1a3e07f7ffe6e2d4ec0466be61`; ordinary primary
  `5c460fe660bac0c4a11957e5b35bc03ad742ed7f635a932f314674f01c1d5ebc`.
- Campaign `d6d3ff1fafbb4b48dad9ef0bced117bab396f87c94400b4e539cc692206af4de`;
  input-file digest `6b04f1f374f970516a272000f9c1ca609c29794f2808b83ac5ec8b4492c2ac33`.
- Progress receipt: 101376 records, 21421582 bytes, SHA-256
  `4a5283ab7088916a41ba2e1d3082574493c4e741bd4b32686f69b047cc45e930`;
  total artifact accounting 21428314 bytes, zero accepted segments.
- Last corpus receipt
  `91925f51439ef8c508a22811f046a8aa9ac1b4538249a7b9f0e1b4020535aa9c`.

The final bounded diagnostic sample recorded 634 segments (1414786770 bytes),
8 L0 segments, 81 flushes, 563 automatic compactions and one failed compaction.
Segment I/O reported 1981981149033 cumulative bytes read and 58990994 read
operations. Page-cache capacity was 4194304 bytes, with 15739438 hits and
54678212 misses. Process RSS at the final sample was 414531584 bytes; this is
not the peak aggregate cgroup measurement. The earlier 03:16 observation had
1984212992 peak aggregate bytes under the unchanged 2147483648-byte/no-swap
limit. Neither sample proves the cause of the write timeout.

## Remediation boundary

The next source package must reproduce and attribute the timed-out corpus
write, including the read/validation path and compaction failure. Preserve the
same allocation, corpus and bounded operation oracle. The high read volume is
a reason to inspect storage work amplification, not proof of a selected fix.
Do not hide the failure by extending the timer or reporting setup as soak.
The independent Linux25 characterization has a different host allocation and
cannot replace this failed nominal Desktop qualification.

## Evidence and checklist

Read the complete launch record and operator inspection helpers; read the
POAM-035 current campaign clauses, retained result/launch objects, final progress
records and bounded launcher log tails. Retained private artifacts remain under
`/home/devcontainers/rrflow-c07-native-79663cdd/desktop-qualification-26/`
inside the VM. Sanitized observations are under `desktop-vm/` in the WSL owner
root. Bootstrap/SSH credentials are excluded. Operator helpers are
`target/inspect_c07_vm_campaign.py` and `target/inspect_c07_vm_failure.py`.

Scope is this new journal, the POAM-035 result link and generated navigation/
inventory. Existing traces and resource observations were preserved; no new
trace or upstream adaptation was needed. Independent SHA-256, byte-count and line-count verification of the retained
progress file matched its result receipt. Native inspection passed as evidence
collection; the campaign itself failed. No unchanged Rust tests were rerun.
The first documentation check rejected a missing active/historical status
marker in this new record; the header was corrected. Documentation, navigation,
execution inventory and whitespace checks then passed. The complete new record,
POAM change and generated diff were reviewed. C-07 and POAM-035 remain open;
this result closes no gate. Candidate CI is required before PR111 merges.

Before landing this evidence, integrated accepted public main
`2255c2bee9c2e4c2f7b44f053dc73a92feebc7b6` after PR109/110 merged with
passing candidate CI. The merge itself changed no files: this branch already
contained their exact prerequisite source. The immutable failed campaign keeps
its original executable/source identities; no result is rebound to the merge.
Regenerate and check documentation/navigation/inventory and whitespace for
this journal update. No runtime suite was rerun for this evidence-only merge;
the new candidate pipeline remains required.
