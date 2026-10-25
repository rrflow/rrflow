# PR 30: integration with accepted public main

**Status:** active integration evidence; candidate CI pending
**Coordinate:** `rrflow://rrflow-instance/data/evidence/pr-30-main-integration`
**Owner:** J-02 development-container prerequisite

The [original package](development-container-latest-stable-baseline-journal.md) retains its evidence and scope.
Starting head `5797ca7b27fc35abb1e3fea6589e566b436ecbb1` integrates public main
`f3c04e955fd8851c0fc466c45f008975e9d4c1a1` by ordinary merge.
The PR brief was updated before integration. The sole merge conflict was the
generated file inventory, regenerated from the staged merged tree.
No runtime implementation was deleted, moved or rewritten. Existing package
source and tests retain their canonical destinations and failure oracles.

Reviewed the original change diff, original journal, merge conflicts, and
final source delta; README and change-authoring ownership govern integration.
The full new record is read before commit. Navigation, inventory, and this
record are integration-specific additions.
Research and runtime trace/resource decisions remain in the original record;
no new algorithm or runtime behavior is introduced.

Validation: `python scripts/ci/check_documentation.py`,
`python scripts/ci/check_workflow.py`,
`python scripts/knowledge/render_navigation.py --check`, and
`python scripts/ci/build_execution_inventory.py --check` all passed.
The first documentation check on PR118 rejected a status without the required
active/historical classification. The new records now use active status.
No Rust suite, container rebuild, SDK conformance or complete candidate CI is
claimed from this local integration. Existing evidence retains its original
revision. Candidate and merge-group CI must pass before landing.

Checklist: existing owning outcome and brief; ordinary merge preserving both
intents; generated conflict resolved mechanically; final diff review; exact
checks recorded; no roadmap checkbox, alpha completion or POAM closure.
The shared C-07 native capacity failure remains open in POAM-035. A successful
documentation check is not native storage qualification. PR 30 owns final
candidate and merge evidence.
