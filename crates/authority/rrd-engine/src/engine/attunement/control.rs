//! Controlling an attunement job: start, step, pause, resume, cancel and connect.

use super::*;

impl RrdEngine {
    /// One finite, explicitly authorized transition. The job and its immutable
    /// accepted response commit together through the existing control WAL/CAS.
    /// A current principal may replay across renewed sessions; workers remain
    /// fenced by the original session, revision and lease generation.
    pub fn control_attunement(
        &self,
        session: &CorrelationId,
        token: &CorrelationId,
        command: &AttunementCommand,
        context: &RequestContext,
        now: u64,
    ) -> Result<AttunementControlResult> {
        let started = Instant::now();
        context.validate(true).map_err(contract)?;
        check_deadline(context, now)?;
        let (_, auth) = self.authorize(
            session,
            token,
            SecurityAction::EngramAdmin,
            now,
            context.request_id.as_str(),
            context.operation_id.as_str(),
        )?;
        let principal = auth.principal_id.ok_or(ServiceError::Unauthenticated)?;
        // Bound encoding before digesting or acquiring the transaction gate.
        let operation_sha256 = digest::sha256_hex(&encode(command)?);
        let (job_id, expected_revision) = command.binding();
        let key = attunement_job_key(&self.instance, job_id);
        let receipt_key = attunement_receipt_key(
            &self.instance,
            job_id,
            &principal,
            context.idempotency_key.as_ref().ok_or_else(|| {
                ServiceError::Contract("attunement mutation requires idempotency".into())
            })?,
        )?;
        let _gate = self.admit_transaction(
            command,
            context.request_id.as_str(),
            context.operation_id.as_str(),
        )?;
        let admitted_at = elapsed_time(now, started)?;
        check_deadline(context, admitted_at)?;
        self.authorize(
            session,
            token,
            SecurityAction::EngramAdmin,
            admitted_at,
            context.request_id.as_str(),
            context.operation_id.as_str(),
        )?;
        let (before_bytes, before, plan) = self.load_attunement(job_id)?;
        if let Some(bytes) = self.storage.control().get(&receipt_key)? {
            let receipt: Receipt = decode(&bytes)?;
            if receipt.format_version != 1
                || receipt.principal_id != principal
                || receipt.operation_sha256 != operation_sha256
            {
                return Err(ServiceError::IdempotencyConflict);
            }
            receipt.status.validate(&plan).map_err(contract)?;
            if receipt.status.job.id != *job_id {
                return Err(ServiceError::ProjectBindingMismatch);
            }
            return Ok(AttunementControlResult {
                status: receipt.status,
                idempotent_replay: true,
            });
        }
        if before.revision != expected_revision {
            return Err(ServiceError::StorageConflict(
                "attunement job revision changed".into(),
            ));
        }
        // Reserve cancellation even when the normal work/receipt budget is
        // exhausted. At most the request and its acknowledgement add two
        // transitions; terminal jobs cannot grow this exception indefinitely.
        let cancelling = matches!(command, AttunementCommand::Cancel { .. })
            || (matches!(command, AttunementCommand::Step { .. })
                && before.state == AttunementJobState::CancelRequested);
        if before.revision >= MAX_JOB_TRANSITIONS && !cancelling {
            return Err(ServiceError::ConfigurationLimit(
                "attunement transition/receipt limit reached".into(),
            ));
        }
        if let AttunementCommand::Step {
            lease_generation, ..
        } = command
        {
            if !cancelling && before.current_phase == Some(AttunementPhase::Inventory) {
                require_worker(&before, &worker(session)?, *lease_generation, admitted_at)?;
                return inventory::execute(
                    self,
                    inventory::Work {
                        session,
                        token,
                        context,
                        now,
                        started,
                        principal,
                        operation_sha256,
                        before_bytes,
                        before,
                        plan,
                        receipt_key,
                        lease_generation: *lease_generation,
                    },
                    _gate,
                );
            }
            // A Parse step without a parse policy has nothing to parse with; it
            // still fails as unavailable below.
            let parse_selected = plan
                .phases
                .get(before.checkpoints.len())
                .is_some_and(|step| {
                    step.phase == AttunementPhase::Parse && step.parse_policy.is_some()
                });
            if !cancelling && before.current_phase == Some(AttunementPhase::Parse) && parse_selected
            {
                require_worker(&before, &worker(session)?, *lease_generation, admitted_at)?;
                return parse::execute(
                    self,
                    inventory::Work {
                        session,
                        token,
                        context,
                        now,
                        started,
                        principal,
                        operation_sha256,
                        before_bytes,
                        before,
                        plan,
                        receipt_key,
                        lease_generation: *lease_generation,
                    },
                    _gate,
                );
            }
        }
        // Other controls still observe the installed precondition. This cannot
        // authorize a project-owned file mutation.
        let requires_source = matches!(
            command,
            AttunementCommand::Start { .. } | AttunementCommand::Resume { .. }
        ) || (matches!(command, AttunementCommand::Step { .. })
            && !cancelling);
        if requires_source && self.attunement_source_observation()? != plan.source_sha256 {
            return Err(ServiceError::AttunementSourceChanged);
        }
        let at = elapsed_time(now, started)?;
        check_deadline(context, at)?;
        if at <= before.updated_at_unix_ms {
            return Err(ServiceError::Contract(
                "attunement transition time must advance".into(),
            ));
        }
        let mut next = before.clone();
        next.revision = before
            .revision
            .checked_add(1)
            .ok_or_else(|| ServiceError::Contract("attunement revision overflow".into()))?;
        next.updated_at_unix_ms = at;
        let holder = worker(session)?;
        let action = match command {
            AttunementCommand::Start {
                plan_sha256,
                lease_ms,
                ..
            } => {
                if before.state != AttunementJobState::Pending || *plan_sha256 != plan.plan_sha256 {
                    return Err(ServiceError::Contract(
                        "start must bind the pending installed plan".into(),
                    ));
                }
                claim(&mut next, &plan, &holder, *lease_ms, at)?;
                "attunement.started"
            }
            AttunementCommand::Resume { request, lease_ms } => {
                request.validate_for(&before, &plan).map_err(contract)?;
                if request.requested_at_unix_ms > at {
                    return Err(ServiceError::Contract(
                        "resume time is in the future".into(),
                    ));
                }
                claim(&mut next, &plan, &holder, *lease_ms, at)?;
                "attunement.resumed"
            }
            AttunementCommand::Pause {
                lease_generation, ..
            } => {
                require_worker(&before, &holder, *lease_generation, at)?;
                next.state = AttunementJobState::Paused;
                next.lease = None;
                "attunement.paused"
            }
            AttunementCommand::RecoverExpired { .. } => {
                if before.state != AttunementJobState::Running
                    || before
                        .lease
                        .as_ref()
                        .is_none_or(|lease| at < lease.expires_at_unix_ms)
                {
                    return Err(ServiceError::StorageConflict(
                        "attunement worker is not expired".into(),
                    ));
                }
                next.state = AttunementJobState::Paused;
                next.lease = None;
                "attunement.lease_recovered"
            }
            AttunementCommand::Cancel { request } => {
                request.validate_for(&before, &plan).map_err(contract)?;
                if request.requested_at_unix_ms > at {
                    return Err(ServiceError::Contract(
                        "cancel time is in the future".into(),
                    ));
                }
                next.state = AttunementJobState::CancelRequested;
                next.current_phase = next.next_phase(&plan);
                next.failure = None;
                "attunement.cancel_requested"
            }
            AttunementCommand::Step {
                lease_generation, ..
            } => {
                if before.state == AttunementJobState::CancelRequested {
                    // This worker owns no asynchronous/external resources. CAS
                    // invalidates any stale step before publication.
                    next.state = AttunementJobState::Cancelled;
                    next.current_phase = None;
                    next.lease = None;
                    "attunement.cancelled"
                } else {
                    require_worker(&before, &holder, *lease_generation, at)?;
                    if before.current_phase == Some(AttunementPhase::Connect) {
                        self.attunement_connect(&plan, &mut next, at)?;
                        "attunement.connect_checkpointed"
                    } else {
                        let phase = before
                            .current_phase
                            .ok_or_else(|| ServiceError::Contract("missing worker phase".into()))?;
                        next.state = AttunementJobState::Failed;
                        next.lease = None;
                        next.failure = Some(AttunementFailure {
                            phase, code: CanonicalId::new("phase-unavailable").map_err(contract)?,
                            message: "This attunement phase has no installed executor; no source or index work was performed.".into(),
                            retryable: true,
                            evidence_sha256: operation_digest(&("phase-unavailable-v1", &plan.plan_sha256, phase))?,
                            observed_at_unix_ms: at,
                        });
                        "attunement.phase_unavailable"
                    }
                }
            }
        };
        // Recheck authorization after waiting/computation, then account for
        // the time spent doing that check before evaluating the lease/deadline.
        let authorization_at = elapsed_time(now, started)?;
        self.authorize(
            session,
            token,
            SecurityAction::EngramAdmin,
            authorization_at,
            context.request_id.as_str(),
            context.operation_id.as_str(),
        )?;
        let finished = elapsed_time(now, started)?;
        check_deadline(context, finished)?;
        // Never publish a worker result after its lease expired while waiting
        // or computing. Claim leases also start before their publication.
        if let AttunementCommand::Step {
            lease_generation, ..
        }
        | AttunementCommand::Pause {
            lease_generation, ..
        } = command
        {
            if before.state == AttunementJobState::Running {
                require_worker(&before, &holder, *lease_generation, finished)?;
            }
        }
        if next.state == AttunementJobState::Running
            && next
                .lease
                .as_ref()
                .is_none_or(|lease| finished >= lease.expires_at_unix_ms)
        {
            return Err(ServiceError::DeadlineExceeded);
        }
        next.updated_at_unix_ms = finished;
        before.validate_transition(&next, &plan).map_err(contract)?;
        let accepted = status(next, &plan, finished)?;
        let receipt = Receipt {
            format_version: 1,
            principal_id: principal,
            operation_sha256,
            status: accepted.clone(),
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
        self.storage.control().commit_batch(&[
            transition(key, Some(before_bytes), encode(&accepted.job)?, action),
            transition(receipt_key, None, encode(&receipt)?, "attunement.receipted"),
        ])?;
        Ok(AttunementControlResult {
            status: accepted,
            idempotent_replay: false,
        })
    }

    pub(super) fn attunement_connect(
        &self,
        plan: &AttunementPlan,
        next: &mut AttunementJob,
        at: u64,
    ) -> Result<()> {
        let configuration: EngramConfiguration = decode(
            &self
                .storage
                .control()
                .get(&engram_configuration_key(&self.instance))?
                .ok_or_else(|| ServiceError::Storage("installed configuration missing".into()))?,
        )?;
        configuration.validate().map_err(contract)?;
        if configuration != self.engram_configuration {
            return Err(ServiceError::ProjectBindingMismatch);
        }
        let read = self
            .storage
            .runtime()
            .read_stamp(&ScopeId::new(format!("instance:{}", self.instance)).map_err(contract)?)?;
        let output_sha256 =
            operation_digest(&("attunement-connect-v1", &plan.target, &configuration, &read))?;
        let planned = plan
            .phases
            .get(next.checkpoints.len())
            .ok_or_else(|| ServiceError::Contract("attunement plan has no next step".into()))?;
        if planned.phase != AttunementPhase::Connect {
            return Err(ServiceError::Contract(
                "attunement step is not connect".into(),
            ));
        }
        let input_sha256 = next.checkpoints.last().map_or_else(
            || plan.source_sha256.clone(),
            |checkpoint| checkpoint.output_sha256.clone(),
        );
        let mut checkpoint = AttunementPhaseCheckpoint {
            contract_version: ATTUNEMENT_CONTRACT_VERSION,
            job_id: next.id.clone(),
            plan_sha256: plan.plan_sha256.clone(),
            phase: AttunementPhase::Connect,
            sequence: planned.sequence,
            configuration_sha256: planned.configuration_sha256.clone(),
            input_sha256,
            output_sha256,
            coordinates: AttunementRuntimeCoordinates {
                runtime_manifest_sha256: read.manifest_id,
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
        advance_after_checkpoint(next, plan);
        Ok(())
    }
}
