# C-07a CLI discovery correction

**Status:** active correction evidence; C-07 qualification remains open
**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/gate-c/c-07a-cli-discovery`
**Owner:** OBJ-02 prerequisite, [C-07a](../../../roadmap/rrflow-1.0/gate-c-07.md)

At candidate `05c96e8518df5a46969f2e3fd2ae22cf63cf7914`, tree
`d53106e19fb7857b214ce146e58d55677bf7492c`, final review found the top-level
usage message still listed only provider commands. Storage dispatch and its
own usage were implemented and tested, but invoking the evaluator without
arguments did not discover that new surface.

Within [PR 55](https://github.com/rrflow/rrflow-development/pull/55)'s declared
`src/main.rs` scope, add `storage <schema|validate|verify>` to the top-level
usage. The [implementation journal](c-07a-storage-measurement-contract.md)
remains immutable: its one-line main diff described that earlier candidate;
this correction adds the usage line. No model, schema, validation, provider
dispatch or engine behavior changes. Generated journal navigation and inventory
are the only other changed paths.

The complete main source had been read, and the exact usage/dispatch diff was
reviewed. The smallest oracle is the no-argument command's exit failure with
the storage command family present in its bounded usage. Python's bounded
subprocess invocation of `target/debug/rrflow-eval.exe` confirmed exit 1 and
the new usage text. `cargo test --locked -p rrflow-eval` again passed all 14
integration tests; strict Clippy and workspace formatting passed.
Documentation passed with 384 statuses/382 coordinates; navigation passed
with 17 indexes, 383 nodes and 1,669 edges; inventory passed with 1,360 records;
diff checks passed. An initial PowerShell stderr-redirection wrapper treated
Cargo's compilation message as a terminating shell error before the usage
assertion. The explicit subprocess result check replaced that wrapper; it
was not an evaluator failure.
No new runtime or synthetic workload is needed to prove this text correction.
The previous candidate CI is superseded, not acceptance for this revision.

Checklist: same owner/brief/baseline and scope; preservation of all command
behavior; focused usage oracle; no source adaptation, external research or
runtime tracing needed; same bounded diagnostic; reviewed diff; new linked
correction record rather than rewriting evidence. Stop on semantic scope
expansion. C-07, J-04, POAM-035 and alpha status stay open. Native fault/scale
qualification remains unrun by this correction.
