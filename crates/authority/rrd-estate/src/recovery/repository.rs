//! Recovery operations on the estate repository.

use super::*;

impl<'a, E: StorageEngine + ?Sized> EstateRepository<'a, E> {
    pub fn set_recovery_policy(
        &self,
        request: &SetRecoveryPolicy,
    ) -> Result<RecoveryMutationOutcome> {
        let candidate = EstateRecoveryPolicy {
            instance_id: request.instance_id.clone(),
            revision: 1,
            max_rpo_ms: request.max_rpo_ms,
            max_rto_ms: request.max_rto_ms,
            minimum_recovery_points: request.minimum_recovery_points,
            retention_ms: request.retention_ms,
            updated_at: request.context.at,
        };
        candidate.validate()?;
        let request_sha256 = recovery_request_sha256(&(
            "recovery-policy-v1",
            &request.instance_id,
            request.max_rpo_ms,
            request.max_rto_ms,
            request.minimum_recovery_points,
            request.retention_ms,
        ));
        self.recovery_mutation(
            &request.context,
            &request.idempotency_key,
            request_sha256,
            "estate.recovery.policy.set",
            |document| {
                if !document
                    .instances
                    .contains_key(request.instance_id.as_str())
                {
                    return Err(Error::NotFound(request.instance_id.to_string()));
                }
                deny_active_recovery_prune(document, &request.instance_id)?;
                let revision = document
                    .recovery_policies
                    .get(request.instance_id.as_str())
                    .map(|policy| {
                        policy.revision.checked_add(1).ok_or_else(|| {
                            Error::Invalid("recovery policy revision overflow".into())
                        })
                    })
                    .transpose()?
                    .unwrap_or(1);
                let mut policy = candidate.clone();
                policy.revision = revision;
                document
                    .recovery_policies
                    .insert(request.instance_id.to_string(), policy);
                Ok(())
            },
        )
    }

    pub fn pin_recovery_point(
        &self,
        request: &PinRecoveryPoint,
    ) -> Result<RecoveryMutationOutcome> {
        validate_sha256(&request.backup_id, "recovery point")?;
        if request
            .expires_at
            .is_some_and(|expires| expires <= request.context.at)
        {
            return Err(Error::Invalid(
                "recovery pin expiry must be after creation".into(),
            ));
        }
        let request_sha256 =
            recovery_request_sha256(&("recovery-pin-v1", &request.backup_id, request.expires_at));
        self.recovery_mutation(
            &request.context,
            &request.idempotency_key,
            request_sha256,
            "estate.recovery.pin.created",
            |document| {
                let point = document
                    .recovery_points
                    .get(&request.backup_id)
                    .ok_or_else(|| Error::NotFound(request.backup_id.clone()))?;
                let instance_id = point.instance_id.clone();
                if point.pruned_at.is_some() {
                    return Err(Error::Invalid(
                        "pruned recovery point cannot be pinned".into(),
                    ));
                }
                deny_active_recovery_prune(document, &instance_id)?;
                let pin = EstateRetentionPin {
                    id: request.context.operation_id.clone(),
                    backup_id: request.backup_id.clone(),
                    kind: EstateRetentionPinKind::ExplicitHold,
                    created_at: request.context.at,
                    expires_at: request.expires_at,
                    released_at: None,
                };
                if document.recovery_pins.contains_key(pin.id.as_str()) {
                    return Err(Error::IdempotencyConflict(pin.id.to_string()));
                }
                document.recovery_pins.insert(pin.id.to_string(), pin);
                Ok(())
            },
        )
    }

    pub fn release_recovery_pin(
        &self,
        request: &ReleaseRecoveryPin,
    ) -> Result<RecoveryMutationOutcome> {
        let request_sha256 = recovery_request_sha256(&("recovery-pin-release-v1", &request.pin_id));
        self.recovery_mutation(
            &request.context,
            &request.idempotency_key,
            request_sha256,
            "estate.recovery.pin.released",
            |document| {
                let instance_id = document
                    .recovery_pins
                    .get(request.pin_id.as_str())
                    .and_then(|pin| document.recovery_points.get(&pin.backup_id))
                    .map(|point| point.instance_id.clone())
                    .ok_or_else(|| Error::NotFound(request.pin_id.to_string()))?;
                deny_active_recovery_prune(document, &instance_id)?;
                let pin = document
                    .recovery_pins
                    .get_mut(request.pin_id.as_str())
                    .ok_or_else(|| Error::NotFound(request.pin_id.to_string()))?;
                if pin.kind == EstateRetentionPinKind::Policy {
                    return Err(Error::Invalid(
                        "policy recovery pins expire through policy, not manual release".into(),
                    ));
                }
                if pin.released_at.is_some() {
                    return Err(Error::Invalid("recovery pin is already released".into()));
                }
                pin.released_at = Some(request.context.at);
                Ok(())
            },
        )
    }

    pub fn record_restore_evidence(
        &self,
        request: &RecordRestoreEvidence,
    ) -> Result<RecoveryMutationOutcome> {
        validate_sha256(&request.backup_id, "restore recovery point")?;
        validate_sha256(&request.closure_sha256, "restore closure")?;
        if request.started_at == 0
            || request.completed_at < request.started_at
            || request.context.at < request.completed_at
        {
            return Err(Error::Invalid("restore evidence time is invalid".into()));
        }
        let request_sha256 = recovery_request_sha256(&(
            "restore-evidence-v1",
            &request.restore_id,
            &request.instance_id,
            &request.backup_id,
            request.started_at,
            request.completed_at,
            request.restored_claim_sequence,
            request.restored_runtime_cursor,
            &request.closure_sha256,
        ));
        self.recovery_mutation(
            &request.context,
            &request.idempotency_key,
            request_sha256,
            "estate.recovery.restore.verified",
            |document| {
                let point = document
                    .recovery_points
                    .get(&request.backup_id)
                    .ok_or_else(|| Error::NotFound(request.backup_id.clone()))?;
                let point_instance = point.instance_id.clone();
                if point.instance_id != request.instance_id || point.pruned_at.is_some() {
                    return Err(Error::Invalid(
                        "restore recovery point is unavailable for the instance".into(),
                    ));
                }
                deny_active_recovery_prune(document, &point_instance)?;
                let duration_ms = request.completed_at - request.started_at;
                let recovery_point_age_ms = request
                    .started_at
                    .checked_sub(point.source_cut_at)
                    .ok_or_else(|| Error::Invalid("restore predates its recovery point".into()))?;
                let evidence = EstateRestoreEvidence {
                    restore_id: request.restore_id.clone(),
                    instance_id: request.instance_id.clone(),
                    backup_id: request.backup_id.clone(),
                    started_at: request.started_at,
                    completed_at: request.completed_at,
                    duration_ms,
                    recovery_point_age_ms,
                    restored_claim_sequence: request.restored_claim_sequence,
                    restored_runtime_cursor: request.restored_runtime_cursor,
                    closure_sha256: request.closure_sha256.clone(),
                    policy_revision: point.policy_revision,
                    rpo_within_objective: recovery_point_age_ms <= point.max_rpo_ms,
                    rto_within_objective: duration_ms <= point.max_rto_ms,
                };
                if document
                    .restore_evidence
                    .contains_key(request.restore_id.as_str())
                {
                    return Err(Error::IdempotencyConflict(request.restore_id.to_string()));
                }
                document
                    .restore_evidence
                    .insert(request.restore_id.to_string(), evidence);
                Ok(())
            },
        )
    }

    pub fn prepare_recovery_prune(
        &self,
        request: &PrepareRecoveryPrune,
    ) -> Result<RecoveryMutationOutcome> {
        validate_ordered_backup_ids(&request.retained_backup_ids, "retained recovery point")?;
        validate_ordered_backup_ids(
            &request.prune_candidate_backup_ids,
            "recovery prune candidate",
        )?;
        validate_sha256(
            &request.expected_catalogue_sha256,
            "recovery prune catalogue",
        )?;
        if request.prune_candidate_backup_ids.is_empty()
            || request.expected_estate_revision == 0
            || request.evaluated_at == 0
            || request.evaluated_at > request.context.at
        {
            return Err(Error::Invalid("recovery prune intent is invalid".into()));
        }
        let request_sha256 = recovery_request_sha256(&(
            "recovery-prune-prepare-v1",
            &request.instance_id,
            request.expected_estate_revision,
            request.evaluated_at,
            &request.expected_catalogue_sha256,
            &request.retained_backup_ids,
            &request.prune_candidate_backup_ids,
        ));
        self.recovery_mutation(
            &request.context,
            &request.idempotency_key,
            request_sha256,
            "estate.recovery.prune.prepared",
            |document| {
                if document.revision != request.expected_estate_revision {
                    return Err(Error::Invalid(
                        "recovery prune estate revision is stale".into(),
                    ));
                }
                deny_active_recovery_prune(document, &request.instance_id)?;
                let decision =
                    retention_decision(document, &request.instance_id, request.evaluated_at)?;
                if decision.retained_backup_ids != request.retained_backup_ids
                    || decision.prune_candidate_backup_ids != request.prune_candidate_backup_ids
                {
                    return Err(Error::Invalid(
                        "recovery prune decision no longer matches estate policy".into(),
                    ));
                }
                if document.recovery_prune_intents.len() == MAX_RECOVERY_PRUNE_INTENTS {
                    return Err(Error::Invalid(
                        "recovery prune intent history is at its v1 bound".into(),
                    ));
                }
                let intent = EstateRecoveryPruneIntent {
                    id: request.context.operation_id.clone(),
                    instance_id: request.instance_id.clone(),
                    based_on_estate_revision: request.expected_estate_revision,
                    evaluated_at: request.evaluated_at,
                    expected_catalogue_sha256: request.expected_catalogue_sha256.clone(),
                    retained_backup_ids: request.retained_backup_ids.clone(),
                    prune_candidate_backup_ids: request.prune_candidate_backup_ids.clone(),
                    created_at: request.context.at,
                };
                document
                    .recovery_prune_intents
                    .insert(intent.id.to_string(), intent);
                Ok(())
            },
        )
    }

    pub fn complete_recovery_prune(
        &self,
        request: &CompleteRecoveryPrune,
    ) -> Result<RecoveryMutationOutcome> {
        validate_sha256(
            &request.resulting_catalogue_sha256,
            "completed recovery prune catalogue",
        )?;
        let request_sha256 = recovery_request_sha256(&(
            "recovery-prune-complete-v1",
            &request.intent_id,
            &request.resulting_catalogue_sha256,
        ));
        self.recovery_mutation(
            &request.context,
            &request.idempotency_key,
            request_sha256,
            "estate.recovery.prune.completed",
            |document| {
                let intent = document
                    .recovery_prune_intents
                    .get(request.intent_id.as_str())
                    .cloned()
                    .ok_or_else(|| Error::NotFound(request.intent_id.to_string()))?;
                if request.context.at < intent.created_at
                    || request.resulting_catalogue_sha256 == intent.expected_catalogue_sha256
                {
                    return Err(Error::Invalid(
                        "completed recovery prune evidence is invalid".into(),
                    ));
                }
                for backup_id in &intent.prune_candidate_backup_ids {
                    let point = document
                        .recovery_points
                        .get_mut(backup_id)
                        .ok_or_else(|| Error::NotFound(backup_id.clone()))?;
                    point.pruned_at = Some(request.context.at);
                }
                document
                    .recovery_prune_intents
                    .remove(request.intent_id.as_str());
                Ok(())
            },
        )
    }

    fn recovery_mutation(
        &self,
        context: &MutationContext,
        idempotency_key: &str,
        request_sha256: String,
        action: &str,
        mutate: impl FnOnce(&mut EstateDocument) -> Result<()>,
    ) -> Result<RecoveryMutationOutcome> {
        validate_context(context)?;
        validate_ascii_key(idempotency_key, "recovery idempotency key")?;
        let Some(current_bytes) = self.engine.control().get(&self.key)? else {
            return Err(Error::NotFound(self.estate_id.to_string()));
        };
        let mut document = super::super::decode(&current_bytes)?;
        if let Some(binding) = document.recovery_idempotency.get(idempotency_key) {
            if binding.operation_id != context.operation_id
                || binding.request_sha256 != request_sha256
            {
                return Err(Error::IdempotencyConflict(idempotency_key.into()));
            }
            return Ok(RecoveryMutationOutcome {
                document,
                idempotent_replay: true,
            });
        }
        if document
            .recovery_idempotency
            .values()
            .any(|binding| binding.operation_id == context.operation_id)
            || document
                .operations
                .contains_key(context.operation_id.as_str())
            || document
                .backup_jobs
                .contains_key(context.operation_id.as_str())
        {
            return Err(Error::IdempotencyConflict(context.operation_id.to_string()));
        }
        mutate(&mut document)?;
        document.recovery_idempotency.insert(
            idempotency_key.into(),
            RecoveryIdempotencyBinding {
                operation_id: context.operation_id.clone(),
                request_sha256,
                bound_at: context.at,
            },
        );
        let document = self.commit(current_bytes, document, context, action)?;
        Ok(RecoveryMutationOutcome {
            document,
            idempotent_replay: false,
        })
    }
}
