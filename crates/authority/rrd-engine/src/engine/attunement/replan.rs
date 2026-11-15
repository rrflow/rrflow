//! Replanning an attunement job: receipts, generations, preview and acceptance.

use super::*;

impl RrdEngine {
    pub(super) fn validate_replan_receipt(
        &self,
        principal: &CanonicalId,
        old: &AttunementJob,
        old_plan: &AttunementPlan,
        receipt: ReplanReceipt,
    ) -> Result<AttunementReplanReceiptView> {
        if receipt.format_version != 1 || &receipt.principal_id != principal {
            return Err(ServiceError::Storage(
                "invalid attunement replan receipt identity".into(),
            ));
        }
        let link_key = format!("{}/successor", attunement_job_key(&self.instance, &old.id));
        let preview: AttunementReplanPreview =
            decode(
                &self.storage.control().get(&link_key)?.ok_or_else(|| {
                    ServiceError::Storage("attunement replan link missing".into())
                })?,
            )?;
        let result = receipt.result;
        result.predecessor.validate(old_plan).map_err(contract)?;
        result.successor.validate(&result.plan).map_err(contract)?;
        let operation_digest = attunement_replan_accept_sha256(&preview, &preview.proposal_sha256)
            .map_err(contract)?;
        let (current_generation, expected_generation) =
            (self.attunement_replan_generation(old)?, preview.generation);
        if preview
            .validate_for(&old.id, preview.predecessor_revision, old_plan)
            .is_err()
            || result.validate_for(&preview, old_plan).is_err()
            || receipt.operation_sha256 != operation_digest
            || current_generation.checked_add(1) != Some(expected_generation)
            || old.revision < result.predecessor.job.revision
            || (result.predecessor.job.state == AttunementJobState::Succeeded
                && old != &result.predecessor.job)
            || result.idempotent_replay
        {
            return Err(ServiceError::Storage(
                "invalid attunement replan receipt lineage".into(),
            ));
        }
        let (_, successor, stored_plan) = self.load_attunement(&result.successor.job.id)?;
        if stored_plan != preview.plan
            || self.attunement_replan_generation(&successor)? != expected_generation
        {
            return Err(ServiceError::Storage(
                "invalid attunement successor binding".into(),
            ));
        }
        Ok(AttunementReplanReceiptView {
            principal_id: receipt.principal_id,
            operation_sha256: receipt.operation_sha256,
            preview,
            predecessor_plan: old_plan.clone(),
            result,
        })
    }

    /// Read the original principal's receipt without replaying acceptance.
    #[allow(clippy::too_many_arguments)]
    pub fn read_attunement_replan_receipt(
        &self,
        session: &CorrelationId,
        token: &CorrelationId,
        job_id: &CanonicalId,
        idempotency_key: &CorrelationId,
        now: u64,
        request_id: &str,
        operation_id: &str,
    ) -> Result<Option<AttunementReplanReceiptView>> {
        let (_, authorization) = self.authorize(
            session,
            token,
            SecurityAction::EstateRead,
            now,
            request_id,
            operation_id,
        )?;
        let principal = authorization
            .principal_id
            .ok_or(ServiceError::Unauthenticated)?;
        let key =
            attunement_replan_receipt_key(&self.instance, job_id, &principal, idempotency_key)?;
        let Some(bytes) = self.storage.control().get(&key)? else {
            return Ok(None);
        };
        let receipt: ReplanReceipt = decode(&bytes)?;
        let (_, old, old_plan) = self.load_attunement(job_id)?;
        self.validate_replan_receipt(&principal, &old, &old_plan, receipt)
            .map(Some)
    }

    pub(in super::super) fn attunement_installed_record(&self) -> Result<InstalledEstateRecord> {
        let record: InstalledEstateRecord = decode(
            &self
                .storage
                .control()
                .get(INSTALLED_RECORD_KEY)?
                .ok_or_else(|| ServiceError::Storage("installed engram record missing".into()))?,
        )?;
        record.validate()?;
        if self.installed_estate_identity() != Some(&record.installation.target) {
            return Err(ServiceError::ProjectBindingMismatch);
        }
        Ok(record)
    }

    pub(super) fn attunement_source_observation(&self) -> Result<String> {
        let record = self.attunement_installed_record()?;
        let (profile, digest) =
            DefaultInstallProfile::load(record.installation.profile_id.as_str())?;
        if digest != record.installation.profile_sha256 {
            return Err(ServiceError::ProjectBindingMismatch);
        }
        let root = self
            .installed_project_root
            .as_ref()
            .ok_or(ServiceError::ProjectBindingMismatch)?;
        project_inventory_sha256(root, &profile)
    }

    pub(super) fn attunement_replan_generation(&self, job: &AttunementJob) -> Result<u64> {
        let key = format!(
            "{}/replan-generation",
            attunement_job_key(&self.instance, &job.id)
        );
        match self.storage.control().get(&key)? {
            Some(value) => {
                let generation: u64 = decode(&value)?;
                if generation == 0 {
                    return Err(ServiceError::Storage(
                        "invalid attunement replan generation".into(),
                    ));
                }
                Ok(generation)
            }
            None => {
                let record = self.attunement_installed_record()?;
                let initial = canonical_prefixed("job", &record.attunement_plan_sha256)?;
                if job.id != initial || job.plan_sha256 != record.attunement_plan_sha256 {
                    return Err(ServiceError::Storage(
                        "attunement replan generation missing".into(),
                    ));
                }
                Ok(0)
            }
        }
    }

    pub(super) fn make_attunement_replan(
        &self,
        job: &AttunementJob,
        old: &AttunementPlan,
    ) -> Result<AttunementReplanPreview> {
        // Connect and Inventory record read-only source observations, and Parse
        // publishes only engine-owned facts addressed by content, which a
        // successor's Parse reconciles itself: unchanged files keep their
        // facts, edited files are reparsed and departed files retired. A
        // successor retains their history and binds fresh source. Other
        // completed effects require explicit reconciliation first.
        if job.checkpoints.iter().any(|checkpoint| {
            !matches!(
                checkpoint.phase,
                AttunementPhase::Connect | AttunementPhase::Inventory | AttunementPhase::Parse
            )
        }) || !matches!(
            job.state,
            AttunementJobState::Pending
                | AttunementJobState::Running
                | AttunementJobState::Paused
                | AttunementJobState::Failed
                | AttunementJobState::Succeeded
        ) {
            return Err(ServiceError::StorageConflict(
                "attunement replan requires a locally fenced job with reconciled effects".into(),
            ));
        }
        if job.state == AttunementJobState::Failed
            && !job.failure.as_ref().is_some_and(|failure| {
                failure.phase != AttunementPhase::Connect
                    && failure.code.as_str() == "phase-unavailable"
                    && failure.retryable
            })
        {
            return Err(ServiceError::StorageConflict(
                "attunement failure requires explicit reconciliation before replan".into(),
            ));
        }
        let generation = self
            .attunement_replan_generation(job)?
            .checked_add(1)
            .ok_or_else(|| {
                ServiceError::ConfigurationLimit("attunement replan generation overflow".into())
            })?;
        let source = self.attunement_source_observation()?;
        let seed =
            attunement_replan_seed(&job.id, job.revision, &old.plan_sha256, generation, &source)
                .map_err(contract)?;
        let id = canonical_prefixed("attune", &seed)?;
        // After a committed Inventory the successor is derived from evidence:
        // the ordered capabilities whose preconditions the committed records
        // satisfy and that this engine can execute. Before that, the successor
        // only rebinds the predecessor's plan to fresh source.
        let evidence = evidence::attunement_evidence(self, job, old)?;
        let plan = match &evidence {
            Some(evidence) => {
                let plan = derive_attunement_plan(old, evidence, id, source).map_err(contract)?;
                let installed = self.attunement_installed_record()?;
                let (profile, _) =
                    DefaultInstallProfile::load(installed.installation.profile_id.as_str())?;
                validate_parse_policies(
                    plan.phases
                        .iter()
                        .filter_map(|step| step.parse_policy.as_ref()),
                    &profile,
                    &self.estate_configuration,
                )?;
                plan
            }
            None => {
                let mut plan = old.clone();
                plan.id = id;
                plan.source_sha256 = source;
                plan.plan_sha256 = attunement_plan_sha256(&plan).map_err(contract)?;
                plan.validate().map_err(contract)?;
                plan
            }
        };
        let proposal_sha256 =
            attunement_replan_preview_sha256(&job.id, job.revision, generation, &plan)
                .map_err(contract)?;
        Ok(AttunementReplanPreview {
            predecessor_job_id: job.id.clone(),
            predecessor_revision: job.revision,
            generation,
            plan,
            proposal_sha256,
            evidence,
        })
    }

    /// A proposal does not accept a plan or authorize any source mutation.
    #[allow(clippy::too_many_arguments)]
    pub fn preview_attunement_replan(
        &self,
        session: &CorrelationId,
        token: &CorrelationId,
        job_id: &CanonicalId,
        expected_revision: u64,
        now: u64,
        request_id: &str,
        operation_id: &str,
    ) -> Result<AttunementReplanPreview> {
        let started = Instant::now();
        self.authorize(
            session,
            token,
            SecurityAction::EstateAdmin,
            now,
            request_id,
            operation_id,
        )?;
        let (_, job, plan) = self.load_attunement(job_id)?;
        if job.revision != expected_revision {
            return Err(ServiceError::StorageConflict(
                "attunement job revision changed".into(),
            ));
        }
        let preview = self.make_attunement_replan(&job, &plan)?;
        self.authorize(
            session,
            token,
            SecurityAction::EstateAdmin,
            elapsed_time(now, started)?,
            request_id,
            operation_id,
        )?;
        Ok(preview)
    }

    /// Atomically fence the prior job and publish the accepted successor.
    #[allow(clippy::too_many_arguments)]
    pub fn accept_attunement_replan(
        &self,
        session: &CorrelationId,
        token: &CorrelationId,
        preview: &AttunementReplanPreview,
        expected_sha256: &str,
        context: &RequestContext,
        now: u64,
    ) -> Result<AttunementReplanResult> {
        let started = Instant::now();
        context.validate(true).map_err(contract)?;
        check_deadline(context, now)?;
        let (_, auth) = self.authorize(
            session,
            token,
            SecurityAction::EstateAdmin,
            now,
            context.request_id.as_str(),
            context.operation_id.as_str(),
        )?;
        let principal = auth.principal_id.ok_or(ServiceError::Unauthenticated)?;
        let _ = encode(preview)?;
        if expected_sha256 != preview.proposal_sha256 {
            return Err(ServiceError::OperationDigestMismatch);
        }
        let operation_sha256 =
            attunement_replan_accept_sha256(preview, expected_sha256).map_err(contract)?;
        let receipt_key = attunement_replan_receipt_key(
            &self.instance,
            &preview.predecessor_job_id,
            &principal,
            context.idempotency_key.as_ref().ok_or_else(|| {
                ServiceError::Contract("attunement replan requires idempotency".into())
            })?,
        )?;
        let _gate = self.admit_transaction(
            preview,
            context.request_id.as_str(),
            context.operation_id.as_str(),
        )?;
        let at = elapsed_time(now, started)?;
        check_deadline(context, at)?;
        self.authorize(
            session,
            token,
            SecurityAction::EstateAdmin,
            at,
            context.request_id.as_str(),
            context.operation_id.as_str(),
        )?;
        let (old_bytes, old, old_plan) = self.load_attunement(&preview.predecessor_job_id)?;
        if let Some(bytes) = self.storage.control().get(&receipt_key)? {
            let receipt: ReplanReceipt = decode(&bytes)?;
            if receipt.principal_id != principal || receipt.operation_sha256 != operation_sha256 {
                return Err(ServiceError::IdempotencyConflict);
            }
            let view = self.validate_replan_receipt(&principal, &old, &old_plan, receipt)?;
            if view.preview != *preview {
                return Err(ServiceError::IdempotencyConflict);
            }
            return Ok(AttunementReplanResult {
                idempotent_replay: true,
                ..view.result
            });
        }
        if old.revision != preview.predecessor_revision {
            return Err(ServiceError::StorageConflict(
                "attunement job revision changed".into(),
            ));
        }
        let refreshed = self.make_attunement_replan(&old, &old_plan)?;
        if refreshed.plan.source_sha256 != preview.plan.source_sha256 {
            return Err(ServiceError::AttunementSourceChanged);
        }
        if refreshed != *preview {
            return Err(ServiceError::OperationDigestMismatch);
        }
        let at = elapsed_time(now, started)?;
        check_deadline(context, at)?;
        self.authorize(
            session,
            token,
            SecurityAction::EstateAdmin,
            at,
            context.request_id.as_str(),
            context.operation_id.as_str(),
        )?;
        let finished = elapsed_time(now, started)?;
        check_deadline(context, finished)?;
        if finished <= old.updated_at_unix_ms {
            return Err(ServiceError::Contract(
                "attunement transition time must advance".into(),
            ));
        }
        let mut predecessor = old.clone();
        if old.state != AttunementJobState::Succeeded {
            predecessor.revision = old
                .revision
                .checked_add(1)
                .ok_or_else(|| ServiceError::Contract("attunement revision overflow".into()))?;
            predecessor.updated_at_unix_ms = finished;
            predecessor.state = AttunementJobState::CancelRequested;
            predecessor.current_phase = predecessor.next_phase(&old_plan);
            predecessor.failure = None;
            old.validate_transition(&predecessor, &old_plan)
                .map_err(contract)?;
        }
        let successor = AttunementJob {
            contract_version: ATTUNEMENT_CONTRACT_VERSION,
            id: canonical_prefixed("job", &preview.plan.plan_sha256)?,
            plan_id: preview.plan.id.clone(),
            plan_sha256: preview.plan.plan_sha256.clone(),
            revision: 1,
            state: AttunementJobState::Pending,
            current_phase: None,
            checkpoints: Vec::new(),
            lease: None,
            failure: None,
            created_at_unix_ms: finished,
            updated_at_unix_ms: finished,
        };
        let result = AttunementReplanResult {
            predecessor: status(predecessor, &old_plan, finished)?,
            successor: status(successor, &preview.plan, finished)?,
            plan: preview.plan.clone(),
            idempotent_replay: false,
        };
        let receipt = ReplanReceipt {
            format_version: 1,
            principal_id: principal,
            operation_sha256,
            result: result.clone(),
        };
        let transition = |key, expected, replacement, action: &str| ControlTransition {
            key,
            expected,
            replacement: Some(replacement),
            at: finished,
            actor: format!("session:{}", session.as_str()),
            action: action.into(),
            request_id: context.request_id.as_str().into(),
            operation_id: context.operation_id.as_str().into(),
        };
        let old_key = attunement_job_key(&self.instance, &old.id);
        let new_key = attunement_job_key(&self.instance, &result.successor.job.id);
        self.storage.control().commit_batch(&[
            transition(
                old_key.clone(),
                Some(old_bytes),
                encode(&result.predecessor.job)?,
                "attunement.replan_fenced",
            ),
            transition(
                attunement_plan_key(&self.instance, &preview.plan.id),
                None,
                encode(&preview.plan)?,
                "attunement.replan_plan_created",
            ),
            transition(
                new_key.clone(),
                None,
                encode(&result.successor.job)?,
                "attunement.replan_job_created",
            ),
            transition(
                format!("{new_key}/replan-generation"),
                None,
                encode(&preview.generation)?,
                "attunement.replan_generation",
            ),
            transition(
                format!("{old_key}/successor"),
                None,
                encode(preview)?,
                "attunement.replan_linked",
            ),
            transition(
                receipt_key,
                None,
                encode(&receipt)?,
                "attunement.replan_receipted",
            ),
        ])?;
        Ok(result)
    }
}
