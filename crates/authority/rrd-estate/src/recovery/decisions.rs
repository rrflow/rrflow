//! Retention decisions, public recovery snapshots and completed-recovery records.

use super::*;

pub fn retention_decision(
    document: &EstateDocument,
    instance_id: &CanonicalId,
    at: u64,
) -> Result<EstateRetentionDecision> {
    if at == 0 {
        return Err(Error::Invalid(
            "retention evaluation time must be non-zero".into(),
        ));
    }
    let policy = document
        .recovery_policies
        .get(instance_id.as_str())
        .ok_or_else(|| Error::NotFound(instance_id.to_string()))?;
    let mut points = document
        .recovery_points
        .values()
        .filter(|point| point.instance_id == *instance_id && point.pruned_at.is_none())
        .collect::<Vec<_>>();
    points.sort_by(|left, right| {
        (right.source_cut_at, &right.backup_id).cmp(&(left.source_cut_at, &left.backup_id))
    });
    let mut retained = document
        .recovery_pins
        .values()
        .filter(|pin| pin.is_live(at))
        .map(|pin| pin.backup_id.clone())
        .collect::<BTreeSet<_>>();
    retained.extend(
        points
            .iter()
            .take(usize::from(policy.minimum_recovery_points))
            .map(|point| point.backup_id.clone()),
    );
    if let Some(newest) = points.first() {
        retained.insert(newest.backup_id.clone());
    }
    let available = points
        .iter()
        .map(|point| point.backup_id.as_str())
        .collect::<BTreeSet<_>>();
    retained.retain(|backup_id| available.contains(backup_id.as_str()));
    let mut retained_backup_ids = retained.iter().cloned().collect::<Vec<_>>();
    retained_backup_ids.sort();
    let mut prune_candidate_backup_ids = points
        .iter()
        .filter(|point| !retained.contains(&point.backup_id))
        .map(|point| point.backup_id.clone())
        .collect::<Vec<_>>();
    prune_candidate_backup_ids.sort();
    Ok(EstateRetentionDecision {
        estate_revision: document.revision,
        instance_id: instance_id.clone(),
        evaluated_at: at,
        retained_backup_ids,
        prune_candidate_backup_ids,
    })
}

pub fn public_recovery_snapshot(document: &EstateDocument) -> rrd_contract::EstateRecoverySnapshot {
    rrd_contract::EstateRecoverySnapshot {
        estate_id: document.id.clone(),
        estate_revision: document.revision,
        policies: document
            .recovery_policies
            .values()
            .map(|policy| rrd_contract::EstateRecoveryPolicySnapshot {
                instance_id: policy.instance_id.clone(),
                revision: policy.revision,
                max_rpo_ms: policy.max_rpo_ms,
                max_rto_ms: policy.max_rto_ms,
                minimum_recovery_points: policy.minimum_recovery_points,
                retention_ms: policy.retention_ms,
                updated_at_unix_ms: policy.updated_at,
            })
            .collect(),
        recovery_points: document
            .recovery_points
            .values()
            .map(|point| rrd_contract::EstateRecoveryPointSnapshot {
                backup_sha256: point.backup_id.clone(),
                instance_id: point.instance_id.clone(),
                backup_job_id: point.backup_job_id.clone(),
                source_generation: point.source_generation,
                archive_sha256: point.archive_sha256.clone(),
                catalogue_sha256: point.catalogue_sha256.clone(),
                source_cut_at_unix_ms: point.source_cut_at,
                completed_at_unix_ms: point.completed_at,
                policy_revision: point.policy_revision,
                max_rpo_ms: point.max_rpo_ms,
                max_rto_ms: point.max_rto_ms,
                pruned_at_unix_ms: point.pruned_at,
            })
            .collect(),
        retention_pins: document
            .recovery_pins
            .values()
            .map(|pin| rrd_contract::EstateRetentionPinSnapshot {
                id: pin.id.clone(),
                backup_sha256: pin.backup_id.clone(),
                kind: match pin.kind {
                    EstateRetentionPinKind::Policy => {
                        rrd_contract::EstateRetentionPinKindSnapshot::Policy
                    }
                    EstateRetentionPinKind::ExplicitHold => {
                        rrd_contract::EstateRetentionPinKindSnapshot::ExplicitHold
                    }
                },
                created_at_unix_ms: pin.created_at,
                expires_at_unix_ms: pin.expires_at,
                released_at_unix_ms: pin.released_at,
            })
            .collect(),
        restore_evidence: document
            .restore_evidence
            .values()
            .map(|evidence| rrd_contract::EstateRestoreEvidenceSnapshot {
                restore_id: evidence.restore_id.clone(),
                instance_id: evidence.instance_id.clone(),
                backup_sha256: evidence.backup_id.clone(),
                started_at_unix_ms: evidence.started_at,
                completed_at_unix_ms: evidence.completed_at,
                duration_ms: evidence.duration_ms,
                recovery_point_age_ms: evidence.recovery_point_age_ms,
                restored_claim_sequence: evidence.restored_claim_sequence,
                restored_runtime_cursor: evidence.restored_runtime_cursor,
                closure_sha256: evidence.closure_sha256.clone(),
                policy_revision: evidence.policy_revision,
                rpo_within_objective: evidence.rpo_within_objective,
                rto_within_objective: evidence.rto_within_objective,
            })
            .collect(),
        prune_intents: document
            .recovery_prune_intents
            .values()
            .map(|intent| rrd_contract::EstateRecoveryPruneIntentSnapshot {
                id: intent.id.clone(),
                instance_id: intent.instance_id.clone(),
                based_on_estate_revision: intent.based_on_estate_revision,
                evaluated_at_unix_ms: intent.evaluated_at,
                expected_catalogue_sha256: intent.expected_catalogue_sha256.clone(),
                retained_backup_ids: intent.retained_backup_ids.clone(),
                prune_candidate_backup_ids: intent.prune_candidate_backup_ids.clone(),
                created_at_unix_ms: intent.created_at,
            })
            .collect(),
    }
}

pub fn public_retention_decision(
    decision: &EstateRetentionDecision,
) -> rrd_contract::EstateRetentionDecisionSnapshot {
    rrd_contract::EstateRetentionDecisionSnapshot {
        estate_revision: decision.estate_revision,
        instance_id: decision.instance_id.clone(),
        evaluated_at_unix_ms: decision.evaluated_at,
        retained_backup_ids: decision.retained_backup_ids.clone(),
        prune_candidate_backup_ids: decision.prune_candidate_backup_ids.clone(),
    }
}

pub(crate) fn ensure_recovery_policy(
    document: &mut EstateDocument,
    instance_id: &CanonicalId,
    at: u64,
) -> Result<EstateRecoveryPolicy> {
    if let Some(policy) = document.recovery_policies.get(instance_id.as_str()) {
        return Ok(policy.clone());
    }
    if document.recovery_policies.len() == MAX_RECOVERY_POLICIES {
        return Err(Error::Invalid("recovery policy limit exceeded".into()));
    }
    let policy = EstateRecoveryPolicy::default_for(instance_id.clone(), at);
    document
        .recovery_policies
        .insert(instance_id.to_string(), policy.clone());
    Ok(policy)
}

pub(crate) fn deny_active_recovery_prune(
    document: &EstateDocument,
    instance_id: &CanonicalId,
) -> Result<()> {
    if document
        .recovery_prune_intents
        .values()
        .any(|intent| intent.instance_id == *instance_id)
    {
        return Err(Error::Invalid(format!(
            "instance {instance_id} has an active recovery prune intent"
        )));
    }
    Ok(())
}

pub(crate) fn record_completed_recovery_point(
    document: &mut EstateDocument,
    job_id: &CanonicalId,
    completed_at: u64,
) -> Result<()> {
    let job = document
        .backup_jobs
        .get(job_id.as_str())
        .ok_or_else(|| Error::NotFound(job_id.to_string()))?;
    let backup_id = job
        .backup_id
        .clone()
        .ok_or_else(|| Error::Invalid("completed backup has no identity".into()))?;
    let archive_sha256 = job
        .archive_sha256
        .clone()
        .ok_or_else(|| Error::Invalid("completed backup has no archive".into()))?;
    let catalogue_sha256 = job
        .catalogue_sha256
        .clone()
        .ok_or_else(|| Error::Invalid("completed backup has no catalogue".into()))?;
    let policy = &job.recovery_policy;
    let point = EstateRecoveryPoint {
        backup_id: backup_id.clone(),
        instance_id: job.instance_id.clone(),
        backup_job_id: job.id.clone(),
        source_generation: job.source_generation,
        archive_sha256,
        catalogue_sha256,
        source_cut_at: job.created_at,
        completed_at,
        policy_revision: policy.revision,
        max_rpo_ms: policy.max_rpo_ms,
        max_rto_ms: policy.max_rto_ms,
        pruned_at: None,
    };
    if let Some(existing) = document.recovery_points.get(&backup_id) {
        if existing != &point {
            return Err(Error::Invalid(
                "backup identity recovery point conflict".into(),
            ));
        }
        return Ok(());
    }
    if document.recovery_points.len() == MAX_RECOVERY_POINTS
        || document.recovery_pins.len() == MAX_RECOVERY_PINS
    {
        return Err(Error::Invalid("recovery history limit exceeded".into()));
    }
    let pin_id = CanonicalId::new(format!("policy-{backup_id}"))
        .map_err(|error| Error::Invalid(error.to_string()))?;
    let expires_at = completed_at
        .checked_add(policy.retention_ms)
        .ok_or_else(|| Error::Invalid("recovery retention expiry overflow".into()))?;
    document.recovery_points.insert(backup_id.clone(), point);
    document.recovery_pins.insert(
        pin_id.to_string(),
        EstateRetentionPin {
            id: pin_id,
            backup_id,
            kind: EstateRetentionPinKind::Policy,
            created_at: completed_at,
            expires_at: Some(expires_at),
            released_at: None,
        },
    );
    Ok(())
}
