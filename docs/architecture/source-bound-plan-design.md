# Source-bound plans and explicit replanning

**Status:** active proposed supporting architecture; no runtime behavior or gate acceptance claimed
**Coordinate:** `rrflow://rrflow-instance/data/architecture/source-bound-plan-design`
**Owner:** explanatory design for the existing engine, inventory, objective and roadmap owners

[ADR-0001](../decisions/0001-single-engine-authority.md) and
[ADR-0002](../decisions/0002-adaptive-reasoning-governed-effects.md) own the
engine and effect boundaries. [OBJ-14](../objectives/rrflow-1.0-alpha.md#required-outcomes),
[D-02/D-03](../roadmap/rrflow-1.0/gate-d.md#project-workflow-build-packages),
and [POAM-038](../poam/rrflow-1.0-alpha/poam-038.md),
[039](../poam/rrflow-1.0-alpha/poam-039.md), and
[041](../poam/rrflow-1.0-alpha/poam-041.md) own required outcomes and evidence.
This proposal cannot relax them or change their status.

## The verified starting point

At development main `86adada6dab6411cc58cb9aaa931e35472c0f452`, installation
sets `AttunementPlan.source_sha256` from an installation precondition inventory.
That inventory is bounded and rejects some observed races, but its own source
states that it is not an atomic snapshot or D-03 inventory. At that historical
baseline the installed engine did not retain the project-root anchor. Current
development main `e73ba9287dafba5d8fb8fb6a990fd145ffd01ab3` does retain the
admitted `installed_project_root` when opening an installed engram
([root-capability journal](../evidence/change-journals/gate-d/d-02-retained-project-root.md));
that capability alone neither authorizes source reads nor establishes a snapshot.
`Start` still checks the plan digest but not the current source. Connect puts
the stored source digest into its first checkpoint without re-reading the
project. The next Inventory step reports `phase-unavailable`. No general
engine-owned per-prompt handoff, effect-scoped question records, or supported
attunement replan is implemented here. An installation plan, a status API, a
schema, or a Git commit is not evidence that these behaviors exist.

The existing `AttunementJob::validate_transition` rejects a change to an
existing job's `plan_id` or `plan_sha256`. Receipt readback resolves the plan
recorded by the receipt; control replay currently validates against the
current job's plan. Because the supported transition does not retarget that
job, an artificial test that rewrites its plan would not prove a supported
replan failure. A real successor and cross-job replay still require their own
implementation and evidence.

## Proposed ownership and identities

`RrdEngine` owns the accepted plan, its source binding, authorization,
question answers, effect admission, jobs, receipts and supersession. A client,
model, SWAG, a watcher or an adapter may supply an input or proposal; none may
accept a plan, authorize an effect, update job state or publish a receipt on
its own. A prompt may lead to a small plan, several effects or no effect. Model
reasoning and exploratory reading remain open-ended under ADR-0002. The types
below are semantic requirements for future contracts, not implemented schemas.

An effect-bearing plan version binds, at minimum:

- stable engram/project identity, immutable plan ID and digest, version and
  optional predecessor; requester and intended recipient or capability;
- a source binding and configuration/policy identities, with explicit scope,
  completeness, budget and authorization requirements;
- the intended observable effects, their dependencies, bounded resources and
  the open questions each effect actually depends on;
- the handoff and approval identity appropriate to the effect, including its
  exact plan digest; and
- typed extensible intent/proposals whose unknown variants cannot authorize
  unknown effects. No record represents hidden model reasoning.

Question discovery is incremental: ask only when a missing fact is required to
choose or authorize a pending effect and cannot be obtained within the already
authorized scope. Record why that effect needs the answer and any safe work
that can proceed without it. The engine must enforce declared dependencies;
whether the proposed dependency set is actually sufficient still needs an
independent test and cannot be proven merely by making the field mandatory.
An approved effect-bearing plan is the official execution plan for that scoped
work; provisional drafts remain clearly identified and have no effect authority.

For source-dependent effects, the source binding names a committed D-03
`SourceTreeSnapshot` and its root digest, project/engram and logical root,
inventory policy and implementation
revisions, completeness/errors, and the relevant captured input identities.
Where a Git revision exists, bind its algorithm and commit identity as
additional provenance. Unstaged, staged and admitted untracked content still
requires the inventory binding: a commit ID alone cannot describe a working
tree. Non-Git projects use an explicit not-applicable revision, never a made-up
commit. Git's own [status documentation](https://git-scm.com/docs/git-status)
distinguishes index, working-tree and untracked states; its
[ignore rules](https://git-scm.com/docs/gitignore) do not exclude tracked files.
Git discovery is optional and cannot become a default runtime dependency. An
effect that genuinely has no project-source dependency must declare that fact;
it does not need a full source snapshot merely because another effect does.

For a project mutation, declare the complete dependency set, including relevant
absence and directory/policy facts where additions, deletions or discovery
could change the result. A narrow set is valid only when the engine can justify
its completeness for that effect; otherwise bind the larger required boundary.
Attunement must satisfy POAM-038's existing requirement to recompute its bound
source identity before **each phase**. Selective project-effect dependencies do
not waive that requirement. The installation precondition stays a distinct
identity and is never relabelled as a D-03 snapshot.

## Ordinary edit and explicit replan

1. Exploration may form a provisional proposal without a committed tree. At
   the durable/effect boundary, authorized capture produces a complete bounded
   source identity or reports an explicit incomplete/error result. The engine
   creates the versioned plan and presents its scope, assumptions, necessary
   decisions, effects and digest. An unanswered question blocks only an effect
   that names it; unrelated exploration and effects can continue.
2. An authorized actor accepts the exact plan and handoff. Before a dependent
   effect the engine reacquires the permitted root capability, validates
   engram, source, policy, configuration, authorization, budget, plan and job
   revision, and re-reads the bound source as required by the owning gate.
   A digest comparison after an incomplete or raced scan is not verification.
3. If an ordinary edit changes a bound input, the engine returns a typed stale
   result identifying the changed scope and next action. It performs no
   prohibited project or engram domain effect for that attempted mutation.
   Existing security/audit behavior is separate. The owner must resolve how the
   objective's no-write oracle accounts for any authorized audit record; this
   proposal does not change that requirement. A status read may report the last
   observed state, not claim it is current without an actual check.
4. The actor requests a replan. The engine captures a new bounded source
   identity and produces a new immutable plan and digest, with a link to its
   predecessor and a clear change summary. Discovery or proposing a plan does
   not activate an external capability. Decisions and approvals carry forward
   only if their exact scope, principal, policy and expiry still authorize the
   new version; changed effect or approval terms require a new authorization.
   An answer that changes the plan requires a new version under POAM-039.
5. On explicit acceptance of the replacement, the engine atomically records
   the accepted version and supersession against the expected previous plan
   and job revision, fences prior workers, and records the decision and
   receipt through its existing durable authority. Under the current immutable
   job contract, the replacement uses a distinct job bound to the new plan;
   the previous job is not retargeted. Changing that contract would require a
   separate explicit design and evidence. A conflict means refresh and retry,
   not silently replacing a newer version. Old jobs, plans, checkpoints and
   receipts remain attributable to the old source. Previously published output
   is not silently relabelled current or deleted; reuse needs matching
   dependency, policy, authorization and output evidence. The successor link,
   job creation and fencing remain engine-owned future behavior.

The replan operation must be available without reinstalling or deleting the
engram. Cancellation and recovery must remain possible even when ordinary work
budgets are exhausted, as the existing attunement control already intends.
Replan is not an automatic approval or an overwrite of the old plan.

The public view should distinguish a proposal awaiting acceptance, a running
or paused job, a reported stale input, an incomplete scan, a question blocking
one effect, and an uncertain external result. It should name what can continue,
what needs an answer, and the next available action; it must not promise that
background work started merely because a plan exists. Each message derives
from engine records or a labeled current observation, rather than a separate
client-maintained lifecycle.

## The race between checking and changing files

An anchored root prevents a class of path-escape attacks; it does not freeze
ordinary editors. Hashing a file, checking its digest and then replacing it
still leaves a time-of-check/time-of-use window. POSIX
[rename](https://pubs.opengroup.org/onlinepubs/9799919799/functions/rename.html)
is atomic as a rename, but its atomicity is not a comparison of the old file's
contents with an approved digest. Git's
[update-ref](https://git-scm.com/docs/git-update-ref) has an explicit old-value
check for a *ref*; that does not make arbitrary working-tree file edits
conditional. Windows [file-sharing modes](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilea)
can exclude conflicting opens while a handle is held, but their exact coverage,
existing handles, rename/delete behavior, parent directories and platform must
be qualified; this proposal does not assume a portable tree-wide lock.

For a claim that a project-owned mutation cannot overwrite an intervening edit,
the concrete execution adapter must prove a conditional effect whose validation
and publication share an enforceable boundary against all relevant writers,
including replacement, deletion, creation and parent/path substitution. A
cooperative lock alone is insufficient against nonparticipating editors. An
atomic rename alone, a filesystem watcher, or pre/post metadata checks alone
are insufficient. If no qualified boundary is available for the selected
filesystem and operation, fail closed before that project mutation and present
the proposal for a supported application path; do not claim the guarantee or
silently discard the user's edit. Native platform and concurrent-writer tests
must establish any future adapter's actual boundary.

For derived engram state, consuming immutable captured bytes can honestly
prove *which snapshot* produced an output. It does not by itself prove the
live project still matched at the later commit. Publication that claims
current-source consistency needs a qualified common synchronization boundary
or must be withheld; a snapshot-relative result may be labeled as such without
pretending it is current. Engine transaction atomicity cannot extend to an
uncoordinated external filesystem merely by checking a digest first.

## Interruption, replay and unknown outcomes

A worker binds plan digest, source binding, job revision, lease generation,
principal, effect identity and idempotency key. Replan admission and worker
effect admission must serialize through the engine: an old worker cannot
publish after a replacement is accepted. Before accepting replacement, the
engine must settle or explicitly surface an already-started external effect;
fencing an engine commit cannot retract a request already sent to a provider.
Pending, unknown or disputed outcomes are not converted to success, cancelled,
or safe-to-repeat by assumption. Replay returns a prior accepted receipt only
for the same principal, operation and immutable plan/effect binding; a changed
payload conflicts. It must not rerun an external effect with an uncertain
outcome unless that capability's proven idempotency or reconciliation permits
it. Restart reconstructs the authoritative plan/job/receipt relation and
maintains lease fencing. These are target requirements, not current evidence.

## Alternatives and unresolved engineering decisions

- **Mutate the existing plan:** simple display, but destroys approval and
  replay identity. Use a new linked plan version instead.
- **Require a fresh full-project scan for every thought:** easy to describe,
  but violates ADR-0002. Bind the needed source at effect boundaries and retain
  POAM-038's stricter attunement phase rule.
- **Treat Git HEAD or metadata as source truth:** cheap, but misses working-tree
  changes or content changes that preserve metadata. Bind actual admitted
  content and completeness; any acceleration needs an independently proven
  invalidation guarantee. D-03 also requires zero-content-read unchanged
  reruns: reconciling that performance oracle with adversarial same-metadata
  edits needs a qualified change signal or a documented evidence limitation,
  not a silent weakening of either requirement.
- **Check then overwrite:** simple and unsafe under an uncoordinated writer.
  Require a qualified conditional effect, or refuse the stronger mutation.

No particular portable filesystem primitive or new dependency is selected by
this design. Although the installed engine now retains an admitted root, its
use for authorized source capture and the required snapshot and freshness
evidence remain open. The conditional project mutation adapter, proof of
complete scoped dependencies,
and safe reuse of prior outputs require implementation packages and native
evidence. SWAG may suggest plans, choose available capabilities, or adjust
effort under approved budgets; it does not become the plan or lifecycle owner.
Adaptation inside the approved capability, configuration and budget bounds can
remain fluid. A change to a bound input or authorized effect requires the
appropriate revalidation or a newly accepted plan, rather than silently
changing what the user approved.

## Acceptance scenarios for future implementation

Use the shipped `rrflow` and real installed engrams; independently observe
source artifacts, engine readback, receipts and audit. At an exact candidate:

| Scenario | Required observable result |
|---|---|
| Edit after plan, before Start or a phase | A source/configuration mismatch refuses the dependent effect with a typed stale result; no prohibited domain or project write; audit behavior is independently checked. |
| Explicit replan after the edit | A new digest and predecessor link are shown; fresh acceptance permits valid work without reinstall; the old version and its receipts remain readable after restart. |
| Competing replans or old worker | Exactly one expected-revision acceptance wins; a prior worker cannot publish a later engine effect. An already-issued external effect is reconciled or surfaced as uncertain. |
| Edit at a barrier after source check, before publication | The qualified project mutation boundary refuses or serializes the conflicting writer. If enforcement is unavailable, no claimed safe mutation occurs. Test file replacement, deletion, new paths and parent/symlink substitution on each supported native platform. |
| Changed question or unrelated edit | A dependent effect is blocked or replanned as appropriate; an unrelated effect and exploratory read remain available when their complete source dependencies genuinely do not change. |
| Kill, restart and replay | At each plan acceptance, effect and receipt boundary, durable readback binds the correct plan/source and accepted effect; replay does not duplicate accepted effects or guess unknown outcomes. |
| Dirty Git and non-Git projects | Equal Git HEAD with different working bytes yields different applicable source identity; non-Git input is supported without invented revision. |
| Independent audit | Derive expected effects from the accepted plan and compare separately observed files and memory with read-back receipts and audit; do not infer success from plan-derived action lists. |

The broader capacity, soak, public-client and alpha gates remain independently
open until their own evidence passes.
