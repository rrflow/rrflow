# Offline paired-outcome review

**Status:** active offline preparation tool; independent review and engine admission remain separate
**Coordinate:** `rrflow://rrflow-instance/data/reference/inference/learning/outcome-review`
**Owner:** controller-learning offline outcome review exchange

The [controller-training contract](controller-training.md) owns dataset and
qualification requirements. This tool prepares a review queue for G-06a; it
does not select providers, start training, write engram state or activate a
checkpoint. A successful task-family prediction is not proof that a provider
would perform the downstream task best.

## Prepare an explicit export

Use Python 3.10+ and standard-library dependencies only:

```sh
python scripts/inference/review_outcomes.py --manifest export/manifest.json --output-dir review-packets
python scripts/inference/test_review_outcomes.py
```

The caller supplies already authorized, redacted regular files in a trusted
export directory. The tool is not a secret scanner, provenance authenticator
or adversarial live-filesystem sandbox. No network access or sibling-checkout
imports occur. Protect packets as carefully as their source inputs; they
contain the complete exported task text and model outcomes.

The manifest is JSON with exactly these fields (`<sha256>` means a lowercase
64-character digest of the exact referenced bytes):

```json
{
  "schemaVersion": 1,
  "purpose": "provisional intent development review",
  "tasks": {"path": "tasks.jsonl", "sha256": "<sha256>"},
  "candidates": [
    {"candidateId": "baseline", "artifactSha256": "<sha256>",
     "outcomes": {"path": "baseline.jsonl", "sha256": "<sha256>"}},
    {"candidateId": "custom", "artifactSha256": "<sha256>",
     "outcomes": {"path": "custom.jsonl", "sha256": "<sha256>"}}
  ]
}
```

Each task JSONL object has exactly `taskId`, `project`, `sourceSha256`, `input`,
`labelOrigin` and `provisionalLabel` (string or null). `sourceSha256` identifies
the retained source record; `artifactSha256` identifies the retained model
artifact manifest. These are caller assertions, not verified source contents.
Keep those source records/manifests available for independent inspection.

Each candidate outcome object has exactly `taskId`, `status`, `output` and
`verification`. Status is `success`, `error` or `abstained`; output preserves
the exported JSON value, including errors and raw model output. Verification
has `state` (`unknown`, `pass`, `fail`, `disputed`) and a nonempty `basis` naming
the evaluator/revision or explaining uncertainty. Even a supplied `pass`
remains an unreviewed assertion. Candidate files must contain exactly one
outcome for every task, including failed calls; row order can differ.

## Bounds and changed inputs

Inputs permit 2–16 unique candidates, 1–10,000 unique tasks, 10 MiB per file,
32 MiB aggregate input, 64 KiB per JSONL row and 2,048 characters per named
text field. JSONL needs a final newline. Duplicate JSON keys, non-finite
numbers, unknown fields, escaping paths, changed digests and incomplete
candidate/task coverage fail before publication. Bounds are exchange limits,
not a runtime context-window policy.

Packets retain the manifest, its digest, the tool digest, every task and its
paired outcomes. Their filename is the SHA-256 of the complete packet bytes.
Publication links a flushed temporary file without replacing an existing
packet; identical inputs/tool bytes reuse the same packet. Corrupted existing
packets are rejected. Filesystems must support hard links. This is not an
engine durable receipt or a power-loss recovery guarantee.

After an ordinary edit, regenerate the affected export and its manifest
digest, then run again. New bytes produce a new packet; earlier packets remain
inspectable. Review decisions must name the exact packet digest and cannot
silently transfer to a new packet. This tool does not persist decisions or
replan engine work. Every packet explicitly remains
`pending_independent_review`, `trainingAdmitted: false` and
`deploymentQualified: false`. Real independent review, consent, split checks,
acceptance evaluation and authorized engine effects are still required.

Use the [explicit label-review validator](review-decisions.md) to check a
completed review against the original packet/export. Its output remains
development evidence and does not admit training or activate a model.
