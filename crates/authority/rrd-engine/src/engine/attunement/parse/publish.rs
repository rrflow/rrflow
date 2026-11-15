//! Commits one Parse Step: the batch's records and the job's transition in one
//! transaction, or the transition alone when the batch changed no record. An
//! unfinished batch renews the worker's lease; a finished one appends the Parse
//! checkpoint.

use super::super::inventory::Work;
use super::*;

/// The batch one Step would publish: its records, whether it finishes Parse,
/// and the summary of every file's state after it.
pub(super) struct Batch<'a> {
    pub mutations: Vec<RuntimeMutation>,
    pub finished: bool,
    pub summary_sha256: &'a str,
}

fn runtime_commit(
    work: &Work<'_>,
    scope: &ScopeId,
    mutations: Vec<RuntimeMutation>,
    cursor: u64,
) -> Result<RuntimeCommit> {
    Ok(RuntimeCommit {
        scope: scope.clone(),
        at: elapsed_time(work.now, work.started)?,
        actor: format!("session:{}", work.session.as_str()),
        expected_cursor: cursor,
        mutations,
    })
}

/// Exact physical bytes the store would write to publish this batch with its
/// job transition, measured without committing (POAM-065).
pub(super) fn measure(
    engine: &RrdEngine,
    work: &Work<'_>,
    check: &dyn Fn() -> Result<()>,
    scope: &ScopeId,
    batch: Batch<'_>,
) -> Result<usize> {
    let read = engine.storage.runtime().read_stamp(scope)?;
    let commit = runtime_commit(work, scope, batch.mutations, read.commit_cursor)?;
    engine
        .storage
        .runtime()
        .measure_commit_with_control(&commit, |read, _| {
            transition(
                engine,
                work,
                check,
                scope,
                batch.finished,
                batch.summary_sha256,
                Some(read),
            )
            .map(|(transitions, _)| transitions)
        })
}

/// Commits the batch's records and the job's transition in one transaction, or
/// the transition alone when the batch changed no record.
pub(super) fn publish(
    engine: &RrdEngine,
    work: &Work<'_>,
    check: &dyn Fn() -> Result<()>,
    scope: &ScopeId,
    batch: Batch<'_>,
) -> Result<AttunementControlResult> {
    let Batch {
        mutations,
        finished,
        summary_sha256,
    } = batch;
    let transition = |read: Option<&ReadStamp>| {
        transition(engine, work, check, scope, finished, summary_sha256, read)
    };
    engine.authorize(
        work.session,
        work.token,
        SecurityAction::EngramAdmin,
        elapsed_time(work.now, work.started)?,
        work.context.request_id.as_str(),
        work.context.operation_id.as_str(),
    )?;
    let accepted = if mutations.is_empty() {
        let (transitions, accepted) = transition(None)?;
        engine.storage.control().commit_batch(&transitions)?;
        accepted
    } else {
        let read = engine.storage.runtime().read_stamp(scope)?;
        let commit = runtime_commit(work, scope, mutations, read.commit_cursor)?;
        let (_, _, accepted) =
            engine
                .storage
                .runtime()
                .commit_with_control_builder(&commit, |read, outcome| {
                    if outcome.last_cursor != read.commit_cursor {
                        return Err(ServiceError::Storage(
                            "Parse checkpoint cursor mismatch".into(),
                        ));
                    }
                    transition(Some(read))
                })?;
        accepted
    };
    Ok(AttunementControlResult {
        status: accepted,
        idempotent_replay: false,
    })
}

/// The job's next state and receipt for this Step.
fn transition(
    engine: &RrdEngine,
    work: &Work<'_>,
    check: &dyn Fn() -> Result<()>,
    scope: &ScopeId,
    finished: bool,
    summary_sha256: &str,
    read: Option<&ReadStamp>,
) -> Result<(Vec<ControlTransition>, AttunementStatus)> {
    check()?;
    let at = elapsed_time(work.now, work.started)?;
    let before = &work.before;
    if at <= before.updated_at_unix_ms {
        return Err(contract("attunement transition time must advance"));
    }
    let mut next = before.clone();
    next.revision = next
        .revision
        .checked_add(1)
        .ok_or_else(|| contract("attunement revision overflow"))?;
    next.updated_at_unix_ms = at;
    let action = if finished {
        let selected = work
            .plan
            .phases
            .get(next.checkpoints.len())
            .ok_or_else(|| contract("missing selected Parse step"))?;
        let read = match read {
            Some(read) => read.clone(),
            None => engine.storage.runtime().read_stamp(scope)?,
        };
        let input_sha256 = next.checkpoints.last().map_or_else(
            || work.plan.source_sha256.clone(),
            |c| c.output_sha256.clone(),
        );
        let mut checkpoint = AttunementPhaseCheckpoint {
            contract_version: ATTUNEMENT_CONTRACT_VERSION,
            job_id: next.id.clone(),
            plan_sha256: work.plan.plan_sha256.clone(),
            phase: AttunementPhase::Parse,
            sequence: selected.sequence,
            configuration_sha256: selected.configuration_sha256.clone(),
            input_sha256,
            output_sha256: summary_sha256.into(),
            coordinates: AttunementRuntimeCoordinates {
                runtime_manifest_sha256: read.manifest_id.clone(),
                runtime_cursor: read.commit_cursor,
                schema_revision: read.schema_revision,
                catalogue_revision: read.catalog_revision,
            },
            committed_at_unix_ms: at,
            checkpoint_sha256: "0".repeat(64),
        };
        checkpoint.checkpoint_sha256 =
            attunement_checkpoint_sha256(&checkpoint).map_err(contract)?;
        next.checkpoints.push(checkpoint);
        advance_after_checkpoint(&mut next, &work.plan);
        "attunement.parse_checkpointed"
    } else {
        // Unfinished work renews the worker's lease for the span it was granted.
        let lease = next
            .lease
            .as_mut()
            .ok_or_else(|| contract("a running Parse has no lease"))?;
        let span = lease
            .expires_at_unix_ms
            .saturating_sub(lease.acquired_at_unix_ms);
        lease.generation = next.revision;
        lease.acquired_at_unix_ms = at;
        lease.expires_at_unix_ms = at
            .checked_add(span)
            .ok_or_else(|| contract("lease time overflow"))?;
        "attunement.parse_progressed"
    };
    before
        .validate_transition(&next, &work.plan)
        .map_err(contract)?;
    let accepted = status(next, &work.plan, at)?;
    let receipt = Receipt {
        format_version: 1,
        principal_id: work.principal.clone(),
        operation_sha256: work.operation_sha256.clone(),
        status: accepted.clone(),
    };
    let control =
        |key: String, expected: Option<Vec<u8>>, bytes: Vec<u8>, action: &str| ControlTransition {
            key,
            expected,
            replacement: Some(bytes),
            at,
            actor: format!("session:{}", work.session.as_str()),
            action: action.into(),
            request_id: work.context.request_id.as_str().into(),
            operation_id: work.context.operation_id.as_str().into(),
        };
    let transitions = vec![
        control(
            attunement_job_key(&engine.instance, &before.id),
            Some(work.before_bytes.clone()),
            encode(&accepted.job)?,
            action,
        ),
        control(
            work.receipt_key.clone(),
            None,
            encode(&receipt)?,
            "attunement.receipted",
        ),
    ];
    Ok((transitions, accepted))
}
