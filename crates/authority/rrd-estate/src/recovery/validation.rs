//! Recovery state validation, request digests and empty defaults.

use super::*;

pub(crate) fn validate_recovery_state(document: &EstateDocument) -> Result<()> {
    if document.recovery_policies.len() > MAX_RECOVERY_POLICIES
        || document.recovery_points.len() > MAX_RECOVERY_POINTS
        || document.recovery_pins.len() > MAX_RECOVERY_PINS
        || document.restore_evidence.len() > MAX_RESTORE_EVIDENCE
        || document.recovery_idempotency.len() > MAX_RECOVERY_IDEMPOTENCY_BINDINGS
        || document.recovery_prune_intents.len() > MAX_RECOVERY_PRUNE_INTENTS
    {
        return Err(Error::Invalid(
            "estate recovery cardinality exceeded".into(),
        ));
    }
    for (key, policy) in &document.recovery_policies {
        validate_id_key(key, &policy.instance_id)?;
        policy.validate()?;
        if !document.instances.contains_key(key) {
            return Err(Error::Invalid(format!(
                "recovery policy {key} names an unknown instance"
            )));
        }
    }
    for (key, point) in &document.recovery_points {
        if key != &point.backup_id {
            return Err(Error::Invalid("recovery point key is invalid".into()));
        }
        validate_sha256(&point.backup_id, "recovery point")?;
        validate_sha256(&point.archive_sha256, "recovery archive")?;
        validate_sha256(&point.catalogue_sha256, "recovery catalogue")?;
        let job = document
            .backup_jobs
            .get(point.backup_job_id.as_str())
            .ok_or_else(|| Error::Invalid(format!("recovery point {key} has no backup job")))?;
        let policy = &job.recovery_policy;
        if point.source_generation == 0
            || point.source_cut_at == 0
            || point.completed_at < point.source_cut_at
            || point.policy_revision == 0
            || point.max_rpo_ms == 0
            || point.max_rto_ms == 0
            || !document.instances.contains_key(point.instance_id.as_str())
            || job.state != super::super::BackupJobState::Succeeded
            || job.backup_id.as_deref() != Some(point.backup_id.as_str())
            || job.instance_id != point.instance_id
            || job.source_generation != point.source_generation
            || job.created_at != point.source_cut_at
            || job.updated_at != point.completed_at
            || job.archive_sha256.as_deref() != Some(point.archive_sha256.as_str())
            || job.catalogue_sha256.as_deref() != Some(point.catalogue_sha256.as_str())
            || policy.revision != point.policy_revision
            || policy.max_rpo_ms != point.max_rpo_ms
            || policy.max_rto_ms != point.max_rto_ms
            || point.pruned_at.is_some_and(|at| at < point.completed_at)
        {
            return Err(Error::Invalid(format!("recovery point {key} is invalid")));
        }
    }
    for job in document
        .backup_jobs
        .values()
        .filter(|job| job.state == super::super::BackupJobState::Succeeded)
    {
        let backup_id = job
            .backup_id
            .as_deref()
            .ok_or_else(|| Error::Invalid("succeeded backup has no identity".into()))?;
        if !document.recovery_points.contains_key(backup_id) {
            return Err(Error::Invalid(format!(
                "policy-bound backup {} has no recovery point",
                job.id
            )));
        }
    }
    for (key, pin) in &document.recovery_pins {
        validate_id_key(key, &pin.id)?;
        validate_sha256(&pin.backup_id, "recovery pin point")?;
        if pin.created_at == 0
            || pin.expires_at.is_some_and(|at| at <= pin.created_at)
            || pin.released_at.is_some_and(|at| at < pin.created_at)
            || !document.recovery_points.contains_key(&pin.backup_id)
            || document
                .recovery_points
                .get(&pin.backup_id)
                .and_then(|point| point.pruned_at)
                .is_some_and(|pruned_at| pin.is_live(pruned_at))
        {
            return Err(Error::Invalid(format!("recovery pin {key} is invalid")));
        }
    }
    for point in document.recovery_points.values() {
        let policy_pins = document
            .recovery_pins
            .values()
            .filter(|pin| {
                pin.backup_id == point.backup_id && pin.kind == EstateRetentionPinKind::Policy
            })
            .collect::<Vec<_>>();
        let job = &document.backup_jobs[point.backup_job_id.as_str()];
        let policy = &job.recovery_policy;
        let expected_expiry = point
            .completed_at
            .checked_add(policy.retention_ms)
            .ok_or_else(|| Error::Invalid("recovery policy expiry overflow".into()))?;
        if policy_pins.len() != 1
            || policy_pins[0].created_at != point.completed_at
            || policy_pins[0].expires_at != Some(expected_expiry)
            || policy_pins[0].released_at.is_some()
        {
            return Err(Error::Invalid(format!(
                "recovery point {} must have exactly one policy pin",
                point.backup_id
            )));
        }
    }
    for (key, evidence) in &document.restore_evidence {
        validate_id_key(key, &evidence.restore_id)?;
        validate_sha256(&evidence.backup_id, "restore recovery point")?;
        validate_sha256(&evidence.closure_sha256, "restore closure")?;
        let point = document
            .recovery_points
            .get(&evidence.backup_id)
            .ok_or_else(|| Error::Invalid(format!("restore evidence {key} has no point")))?;
        if evidence.started_at == 0
            || evidence.completed_at < evidence.started_at
            || evidence.duration_ms != evidence.completed_at - evidence.started_at
            || evidence.policy_revision == 0
            || evidence.instance_id != point.instance_id
            || evidence.recovery_point_age_ms
                != evidence.started_at.saturating_sub(point.source_cut_at)
            || evidence.policy_revision != point.policy_revision
            || evidence.rpo_within_objective != (evidence.recovery_point_age_ms <= point.max_rpo_ms)
            || evidence.rto_within_objective != (evidence.duration_ms <= point.max_rto_ms)
            || point
                .pruned_at
                .is_some_and(|pruned_at| evidence.completed_at > pruned_at)
        {
            return Err(Error::Invalid(format!("restore evidence {key} is invalid")));
        }
    }
    for (key, binding) in &document.recovery_idempotency {
        validate_ascii_key(key, "recovery idempotency key")?;
        validate_sha256(&binding.request_sha256, "recovery idempotency request")?;
        if binding.bound_at == 0 {
            return Err(Error::Invalid(
                "recovery idempotency binding time is invalid".into(),
            ));
        }
    }
    let mut prune_instances = BTreeSet::new();
    for (key, intent) in &document.recovery_prune_intents {
        validate_id_key(key, &intent.id)?;
        validate_sha256(
            &intent.expected_catalogue_sha256,
            "recovery prune catalogue",
        )?;
        validate_ordered_backup_ids(&intent.retained_backup_ids, "retained recovery point")?;
        validate_ordered_backup_ids(
            &intent.prune_candidate_backup_ids,
            "recovery prune candidate",
        )?;
        if intent.based_on_estate_revision == 0
            || intent.evaluated_at == 0
            || intent.created_at < intent.evaluated_at
            || intent.prune_candidate_backup_ids.is_empty()
            || !document.instances.contains_key(intent.instance_id.as_str())
            || !prune_instances.insert(intent.instance_id.as_str())
            || intent
                .retained_backup_ids
                .iter()
                .any(|backup_id| !document.recovery_points.contains_key(backup_id))
            || intent
                .prune_candidate_backup_ids
                .iter()
                .any(|backup_id| !document.recovery_points.contains_key(backup_id))
        {
            return Err(Error::Invalid(format!(
                "recovery prune intent {key} is invalid"
            )));
        }
    }
    Ok(())
}

pub(super) fn recovery_request_sha256(value: &impl Serialize) -> String {
    digest::sha256_hex(&serde_json::to_vec(value).expect("validated recovery request serializes"))
}

pub(super) fn validate_ordered_backup_ids(values: &[String], name: &str) -> Result<()> {
    let mut previous: Option<&str> = None;
    for value in values {
        validate_sha256(value, name)?;
        if previous.is_some_and(|prior| prior >= value.as_str()) {
            return Err(Error::Invalid(format!(
                "{name} identities are not uniquely ordered"
            )));
        }
        previous = Some(value);
    }
    Ok(())
}

pub(crate) fn empty_recovery_policies() -> BTreeMap<String, EstateRecoveryPolicy> {
    BTreeMap::new()
}

pub(crate) fn empty_recovery_points() -> BTreeMap<String, EstateRecoveryPoint> {
    BTreeMap::new()
}

pub(crate) fn empty_recovery_pins() -> BTreeMap<String, EstateRetentionPin> {
    BTreeMap::new()
}

pub(crate) fn empty_restore_evidence() -> BTreeMap<String, EstateRestoreEvidence> {
    BTreeMap::new()
}

pub(crate) fn empty_recovery_idempotency() -> BTreeMap<String, RecoveryIdempotencyBinding> {
    BTreeMap::new()
}

pub(crate) fn empty_recovery_prune_intents() -> BTreeMap<String, EstateRecoveryPruneIntent> {
    BTreeMap::new()
}
