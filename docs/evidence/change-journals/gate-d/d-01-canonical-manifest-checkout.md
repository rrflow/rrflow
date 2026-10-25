# D-01.2 native canonical-manifest checkout correction

**Status:** active bounded candidate correction; native acceptance pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-d/d-01-canonical-manifest-checkout`
**Owner:** OBJ-07 prerequisite, D-01.2, POAM-006

## Baseline and authorized scope

[PR 49](https://github.com/rrflow/rrflow-development/pull/49) carries the
pre-edit correction brief. Starting revision is
`14204d244b6cbeda532b250eb60bfbe7cea71ae8`, tree
`93a347d35bbed6f266e345faa06d6ebf87418e58`. The
[original implementation receipt](d-01-anchored-filesystem.md) stays immutable.
This correction belongs to the same native filesystem qualification package.

Windows candidate [run 39633264092](https://github.com/rrflow/rrflow-development/actions/runs/39633264092)
passed all six filesystem fixtures, then failed the manifest identity test:
Git's checkout conversion produced CRLF in `manifest-v3.json`; the canonical
serializer emitted LF. The logical data and digest matched. This is a verified
qualification gap, not permission to normalize or weaken a byte-exact oracle.

Create `crates/persistence/rrd-lsm/fixtures/.gitattributes` with one scoped
`manifest-v3.json text eol=lf` rule. The fixture blob, serializer, storage
formats, runtime code and exact-byte assertion remain unchanged. Update
POAM-006 with this evidence link and regenerate navigation/inventory. No files
are moved or deleted. Stop on other byte drift or any remaining failed native
check. No D-01, objective or POA&M lifecycle status changes.

## Review and decisions

Files read in full for this correction: `tests/manifest.rs` and
`fixtures/manifest-v3.json` under `crates/persistence/rrd-lsm/`;
`docs/roadmap/rrflow-1.0-execution/change-authoring.md`;
`docs/poam/rrflow-1.0-alpha/poam-006.md`; and the new attributes and journal.
The original package records the wider implementation review. No prior
attributes file was present. The failed job log identifies the exact differing
bytes. The scoped attributes rule and complete correction diff are reviewed;
generated output is verified mechanically.

No external research or upstream adaptation is necessary for this checkout
correction. Preserve existing diagnostics; no new runtime trace, resource
instrumentation or benchmark is relevant. The native failure log is retained
under ignored `target/d01-windows-candidate.log`; it contains test fixture data,
not credentials.

## Acceptance evidence

- First failing oracle: Windows job `119336801185` in run `39633264092`,
  `cargo test -p rrd-lsm --test filesystem --test manifest --locked`:
  six filesystem tests pass; eight manifest tests pass and one fails on CRLF.
- `git -c core.autocrlf=true checkout-index --force --prefix=<ignored-target>/
  -- crates/persistence/rrd-lsm/fixtures/manifest-v3.json`, followed by exact
  comparison to `git show HEAD:<fixture>` and carriage-return rejection:
  passed, 1,605 identical bytes, LF only.
- `cargo test --locked -p rrd-lsm --test filesystem --test manifest`:
  15 passed (six filesystem and nine manifest), exit 0.
- `cargo test --locked -p rrd-lsm --all-targets`:
  119 passed across 19 targets, exit 0. Log:
  ignored `target/d01-canonical-fixture-lsm.log`.

The final correction candidate must pass the entire native candidate pipeline;
that result and merge are not claimed in this pre-push record. Linux and macOS
are not executed locally. No runtime source changed, so the previous package
lint evidence is retained rather than rerunning unrelated compilation here.
Documentation, navigation, inventory and whitespace checks accompany commit.

## Change checklist

The PR brief binds owner, current/target behavior, exact paths, unchanged
semantics, first failure, acceptance commands and stop conditions before the
edit. Reviewed files and research/trace/resource decisions are explicit above.
The new evidence node and generated index preserve the old receipt. The
ordinary PR gate remains mandatory; no retry of the uncorrected Windows
candidate counts as remediation. Installation exclusion, full bootstrap and
D-01.5 native/offline qualification remain open in their owning packages.
