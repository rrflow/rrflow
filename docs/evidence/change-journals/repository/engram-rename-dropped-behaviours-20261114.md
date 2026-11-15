# Five behaviours a merge dropped from main, restored

**Status:** active change evidence; POAM-078 remains open

**Coordinate:** `rrflow://rrflow-instance/data/evidence/change-journal/repository/engram-rename-dropped-behaviours-20261114`

**Owner:** [POAM-078](../../../poam/rrflow-1.0-alpha/poam-078.md); the endpoint selection work of Gate G

**Starting revision:** `main` at `7ebfebe92`.

**Where this came from.** The archived worktree `wip/engram-rename-diagnosis-20261106` held a repaired blanket rename. Its tree cannot be applied. A review of it ([plan input](engram-rename-review-20261114.md)) found that merge `4c2f2a6a1` (2026-11-09) had silently dropped five behaviours from `main` that the worktree kept. Each was re-proven on `main` before being changed.

| Item | On `main` before | Now |
|---|---|---|
| Project map has a caller, a test and its recovery entries | Dropped: no caller | Restored on its own: [journal](../gate-d/project-map-callers-restored-20261114.md) |
| An unimplemented worker adapter fails closed | **Reproduced.** A worker bound to an `Other` adapter was run as an OpenAI-style chat call and reported "chat transport failed; remote outcome unknown", execution may continue. The reference docs said the engine fails closed. | `execute.rs` refuses before any reservation, spend or call: `EndpointCallFailed { execution_may_continue: false }` |
| A selection that names a prompt flight is checked against it | **Reproduced.** A wrong cohort digest, a wrong context-request digest, and a flight that belongs to no task were all accepted and reached the controller. | `endpoint_proposal.rs` reads the retained flight and requires its scope, task, cohort and context request to match; otherwise `TaskAttemptInvalidated(InputsChanged)` before any dispatch |
| Routing input narrows the candidates the engine offers | **Read, not run.** The packet carried the full allowance while its validator expects the narrowed one, so a routed request that narrows could not pass its own validation. Nothing builds routed requests yet, so it is latent. | The proposal and the worker both use the narrowed allowance, and the worker output carries the routing |
| Observation exports carry the selector identity | Absent, though `docs/reference/inference/lfg-swag.md` documents `selector_policy_id`, `selector_routing_sha256` and `selector_narrowed_candidate_count` as existing | Three optional fields on `EngineReferences`, validated, filled by the engine from the retained attempt's routing |

## Oracles

- `a_worker_adapter_the_engine_does_not_implement_is_refused_before_any_effect`: failing on `main` (commit `Worker: failing test…`), passing now. It also requires the budget record and the stored control state to be unchanged and the provider listener to receive nothing.
- `a_selection_must_match_the_prompt_flight_it_names`: failing on `main` (commit `Selection: failing test…`), passing now. It launches real flights through the engine and requires `InputsChanged` for three wrong identities and no connection to the controller.
- `selector_references_are_validated_and_stay_absent_when_unrouted` (observation crate): absent fields are not serialised; empty id, malformed digest and a zero count are refused.

## Not established

- **Routing.** No engine test sends a routed request. The narrowing in the proposal and in the worker follows the contract's own validator and the existing 77 reasoning-budget tests still pass, but nothing proves a routed selection can be proposed and then executed end to end. Selector references are filled by the same untested path.
- The positive flight case (a matching flight proceeds) is exercised by no test; only the three refusals are.
- Not run on Linux or macOS here.

## Left in the archive's review, not done

The rename itself (about 345 files, the `rrd-estate` crate, wire fields shared with Connectome and the SDKs), the removal of the half-rename residue named in POAM-078, and the capture-fixture technique that renames a field without rewriting recorded bytes. They are one package; the [review](engram-rename-review-20261114.md) holds the map, the hazards and a recommended order.

**Research decision:** none. **Trace decision:** none. **Roadmap/POA&M change:** POAM-078 gains a pointer to the review.
