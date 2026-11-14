//! Applying commands, membership bindings and request history.

use super::*;

pub(super) struct CommandApplication {
    pub(super) response: RrdRaftResponse,
    pub(super) operations: Vec<Mutation>,
}

pub(super) fn apply_command(
    database: &Database,
    shard: ShardId,
    state: &mut StateMachineData,
    log_id: LogId<u64>,
    command: RrdRaftCommand,
) -> std::result::Result<CommandApplication, StorageError<u64>> {
    command.validate().map_err(|error| {
        storage_error(
            ErrorSubject::Apply(log_id),
            ErrorVerb::Write,
            error.to_string(),
        )
    })?;
    if command.shard != shard {
        return Err(storage_error(
            ErrorSubject::Apply(log_id),
            ErrorVerb::Write,
            "replicated command targets a different shard",
        ));
    }
    let command_digest = command_identity_digest(&command);
    if let Some(previous) = state.requests.get(&command.request_id) {
        if previous.command_digest != command_digest {
            return Ok(CommandApplication {
                response: RrdRaftResponse {
                    accepted: false,
                    duplicate: false,
                    term: log_id.leader_id.term,
                    index: log_id.index,
                    state_digest: state.state_digest.clone(),
                    reason: "request id was reused with a different command identity".into(),
                    runtime_outcome: None,
                },
                operations: Vec::new(),
            });
        }
        let mut response = previous.response.clone();
        response.duplicate = true;
        response.term = log_id.leader_id.term;
        response.index = log_id.index;
        return Ok(CommandApplication {
            response,
            operations: Vec::new(),
        });
    }

    let current_index = state.last_applied.map_or(0, |applied| applied.index);
    let is_transition = matches!(
        &command.operation,
        RrdRaftOperation::PlacementTransition { .. }
    );
    let epoch_matches = match &command.operation {
        RrdRaftOperation::PlacementTransition { placement } => {
            state.placement_epoch.map_or(placement.epoch == 1, |epoch| {
                epoch.checked_add(1) == Some(placement.epoch)
            }) && command.placement_epoch == placement.epoch
        }
        _ => state.placement_epoch == Some(command.placement_epoch),
    };
    let current_membership_digest =
        membership_binding(&state.last_membership).map(|voters| membership_binding_digest(&voters));
    let membership_matches = is_transition
        || (state.placement_membership_digest.as_ref() == current_membership_digest.as_ref()
            && state.placement_membership_generation == Some(state.voter_membership_generation));
    let commit_index_matches = command
        .expected_commit_index
        .is_none_or(|expected| expected == current_index);
    let mut accepted = epoch_matches && membership_matches && commit_index_matches;
    let mut operations = Vec::new();
    let mut runtime_outcome = None;
    let mut reason = if !epoch_matches && is_transition {
        format!(
            "placement transition epoch {} is not the successor of state-machine epoch {}",
            command.placement_epoch,
            state
                .placement_epoch
                .map_or_else(|| "uninitialized".into(), |epoch| epoch.to_string())
        )
    } else if !epoch_matches {
        format!(
            "placement epoch {} does not match established state-machine epoch {}",
            command.placement_epoch,
            state
                .placement_epoch
                .map_or_else(|| "uninitialized".into(), |epoch| epoch.to_string())
        )
    } else if !membership_matches {
        "applied Raft membership no longer matches the established placement epoch".into()
    } else if !commit_index_matches {
        format!(
            "expected commit index {} but state machine was at {current_index}",
            command
                .expected_commit_index
                .expect("rejected CAS has expectation")
        )
    } else {
        String::new()
    };

    if accepted {
        match &command.operation {
            RrdRaftOperation::PlacementTransition { placement } => {
                if let Some(membership_voters) =
                    placement_membership_binding(placement, &state.last_membership)
                {
                    state.placement_epoch = Some(placement.epoch);
                    state.placement_digest = Some(placement.digest().map_err(|error| {
                        storage_error(
                            ErrorSubject::Apply(log_id),
                            ErrorVerb::Write,
                            error.to_string(),
                        )
                    })?);
                    state.placement_membership_digest =
                        Some(membership_binding_digest(&membership_voters));
                    state.placement_membership_generation = Some(state.voter_membership_generation);
                    reason = "quorum-committed placement epoch transition applied".into();
                } else {
                    accepted = false;
                    reason =
                        "placement voters and zones do not match applied Raft membership".into();
                }
            }
            RrdRaftOperation::Probe { .. } => {
                reason = "quorum-committed probe applied".into();
            }
            RrdRaftOperation::RuntimeCommit { commit } => {
                let commit_id = commit.digest();
                match rrflow_kv_commit_outcome(database, &commit_id) {
                    Ok(Some(outcome)) => {
                        reason = "canonical runtime transaction was already committed".into();
                        runtime_outcome = Some(outcome);
                    }
                    Ok(None) => match prepare_rrflow_kv_commit(database, commit) {
                        Ok(plan) => {
                            let (outcome, runtime_operations) =
                                plan.into_parts_for(database).map_err(|error| {
                                    storage_error(
                                        ErrorSubject::Apply(log_id),
                                        ErrorVerb::Write,
                                        error.to_string(),
                                    )
                                })?;
                            state.runtime_commit_count =
                                state.runtime_commit_count.checked_add(1).ok_or_else(|| {
                                    storage_error(
                                        ErrorSubject::Apply(log_id),
                                        ErrorVerb::Write,
                                        "runtime commit count overflowed",
                                    )
                                })?;
                            reason =
                                "quorum-committed canonical runtime transaction applied".into();
                            runtime_outcome = Some(outcome);
                            operations = runtime_operations;
                        }
                        Err(error) if deterministic_runtime_rejection(&error) => {
                            accepted = false;
                            reason = format!("canonical runtime transaction denied: {error}");
                        }
                        Err(error) => {
                            return Err(storage_error(
                                ErrorSubject::Apply(log_id),
                                ErrorVerb::Write,
                                error.to_string(),
                            ));
                        }
                    },
                    Err(error) => {
                        return Err(storage_error(
                            ErrorSubject::Apply(log_id),
                            ErrorVerb::Read,
                            error.to_string(),
                        ));
                    }
                }
            }
        }
    }

    if accepted {
        let mut bytes = b"rrflow.raft.state.transition.v1".to_vec();
        bytes.extend_from_slice(state.state_digest.as_bytes());
        bytes.extend_from_slice(&log_id.leader_id.term.to_be_bytes());
        bytes.extend_from_slice(&log_id.index.to_be_bytes());
        bytes.extend_from_slice(command.operation.digest().as_bytes());
        state.state_digest = sha256_hex(&bytes);
    }
    let response = RrdRaftResponse {
        accepted,
        duplicate: false,
        term: log_id.leader_id.term,
        index: log_id.index,
        state_digest: state.state_digest.clone(),
        reason,
        runtime_outcome,
    };
    state.requests.insert(
        command.request_id,
        AppliedRequest {
            first_applied_index: log_id.index,
            command_digest,
            response: response.clone(),
        },
    );
    Ok(CommandApplication {
        response,
        operations,
    })
}

pub(super) fn placement_membership_binding(
    placement: &ShardPlacement,
    membership: &StoredMembership<u64, RrdRaftNode>,
) -> Option<BTreeSet<(String, String)>> {
    let raft_voters = membership_binding(membership)?;
    let declared_voters = placement
        .voters()
        .map(|replica| (replica.node.to_string(), replica.zone.to_string()))
        .collect::<BTreeSet<_>>();
    (raft_voters == declared_voters).then_some(raft_voters)
}

pub(super) fn membership_binding(
    membership: &StoredMembership<u64, RrdRaftNode>,
) -> Option<BTreeSet<(String, String)>> {
    let voter_ids = membership.voter_ids().collect::<Vec<_>>();
    let raft_voters = voter_ids
        .iter()
        .copied()
        .map(|id| membership.membership().get_node(&id))
        .collect::<Option<Vec<_>>>()?;
    if raft_voters.is_empty() || raft_voters.iter().any(|node| node.validate().is_err()) {
        return None;
    }
    let raft_voters = raft_voters
        .into_iter()
        .map(|node| (node.canonical_id.clone(), node.zone.clone()))
        .collect::<BTreeSet<_>>();
    (raft_voters.len() == voter_ids.len()).then_some(raft_voters)
}

pub(super) fn membership_binding_digest(voters: &BTreeSet<(String, String)>) -> String {
    let mut bytes = b"rrflow.raft.membership-binding.v1".to_vec();
    for (canonical_id, zone) in voters {
        bytes.extend_from_slice(&(canonical_id.len() as u64).to_be_bytes());
        bytes.extend_from_slice(canonical_id.as_bytes());
        bytes.extend_from_slice(&(zone.len() as u64).to_be_bytes());
        bytes.extend_from_slice(zone.as_bytes());
    }
    sha256_hex(&bytes)
}

pub(super) fn prune_request_history(state: &mut StateMachineData, through_index: u64) {
    let oldest_retained = through_index.saturating_sub(RRFLOW_RAFT_REQUEST_RETENTION_LOGS - 1);
    state
        .requests
        .retain(|_, request| request.first_applied_index >= oldest_retained);
}

pub(super) fn deterministic_runtime_rejection(error: &StoreError) -> bool {
    matches!(
        error,
        StoreError::Kernel(_)
            | StoreError::RuntimeConflict { .. }
            | StoreError::DanglingRuntimeReference(_)
            | StoreError::RuntimeSchemaMissing(_)
            | StoreError::RuntimeSchemaConflict { .. }
    )
}

pub(super) fn command_identity_digest(command: &RrdRaftCommand) -> String {
    let mut bytes = b"rrflow.raft.command.identity.v1".to_vec();
    bytes.extend_from_slice(&command.shard.0.to_be_bytes());
    bytes.extend_from_slice(&command.placement_epoch.to_be_bytes());
    match command.expected_commit_index {
        Some(index) => {
            bytes.push(1);
            bytes.extend_from_slice(&index.to_be_bytes());
        }
        None => bytes.push(0),
    }
    bytes.extend_from_slice(command.operation.digest().as_bytes());
    sha256_hex(&bytes)
}

pub(super) fn blank_response(state: &StateMachineData, log_id: LogId<u64>) -> RrdRaftResponse {
    RrdRaftResponse {
        accepted: true,
        duplicate: false,
        term: log_id.leader_id.term,
        index: log_id.index,
        state_digest: state.state_digest.clone(),
        reason: "raft protocol entry applied".into(),
        runtime_outcome: None,
    }
}
