//! The estate repository.

use super::*;

impl<'a, E: StorageEngine + ?Sized> EstateRepository<'a, E> {
    pub fn new(engine: &'a E, estate_id: CanonicalId) -> Self {
        let key = format!("server/state/estate/{estate_id}/document");
        Self {
            engine,
            estate_id,
            key,
        }
    }

    pub fn create(&self, context: &MutationContext) -> Result<EstateDocument> {
        self.create_idempotent(context)
            .map(|outcome| outcome.document)
    }

    pub fn create_idempotent(&self, context: &MutationContext) -> Result<CreateOutcome> {
        validate_context(context)?;
        let document = EstateDocument::new(self.estate_id.clone(), context.at)?;
        let replacement = encode(&document)?;
        let transition = ControlTransition {
            key: self.key.clone(),
            expected: None,
            replacement: Some(replacement),
            at: context.at,
            actor: context.actor.clone(),
            action: "estate.create".into(),
            request_id: context.request_id.clone(),
            operation_id: context.operation_id.to_string(),
        };
        match self.engine.control().commit(&transition) {
            Ok(_) => Ok(CreateOutcome {
                document,
                idempotent_replay: false,
            }),
            Err(rrd_store::Error::ControlConflict(_)) => {
                let existing = self.load()?.ok_or_else(|| {
                    Error::Invalid("estate create conflicted without state".into())
                })?;
                let mut after = 0;
                let mut scanned = 0;
                loop {
                    let page = self.engine.control().journal_since(after, 1_024)?;
                    scanned += page.len();
                    if page.iter().any(|entry| {
                        entry.key == self.key
                            && entry.action == "estate.create"
                            && entry.actor == context.actor
                            && entry.request_id == context.request_id
                            && entry.operation_id == context.operation_id.as_str()
                            && entry.at == context.at
                    }) {
                        return Ok(CreateOutcome {
                            document: existing,
                            idempotent_replay: true,
                        });
                    }
                    let Some(last) = page.last() else {
                        break;
                    };
                    after = last.sequence;
                    if page.len() < 1_024 {
                        break;
                    }
                    if scanned >= MAX_CREATE_REPLAY_JOURNAL_ENTRIES {
                        return Err(Error::Invalid(
                            "estate create replay is outside the bounded journal window".into(),
                        ));
                    }
                }
                Err(Error::AlreadyExists(self.estate_id.to_string()))
            }
            Err(error) => Err(error.into()),
        }
    }

    pub fn load(&self) -> Result<Option<EstateDocument>> {
        self.engine
            .control()
            .get(&self.key)?
            .map(|bytes| decode(&bytes))
            .transpose()
    }

    pub fn apply_authority(&self, request: &ApplyAuthority) -> Result<AuthorityMutationOutcome> {
        validate_context(&request.context)?;
        validate_ascii_key(&request.idempotency_key, "authority idempotency key")?;
        let request_sha256 = authority::authority_request_sha256(request);
        let Some(current_bytes) = self.engine.control().get(&self.key)? else {
            return Err(Error::NotFound(self.estate_id.to_string()));
        };
        let mut document = decode(&current_bytes)?;
        if let Some(binding) = document.authority.idempotency.get(&request.idempotency_key) {
            if binding.request_sha256 != request_sha256
                || binding.operation_id != request.context.operation_id
            {
                return Err(Error::IdempotencyConflict(request.idempotency_key.clone()));
            }
            return Ok(AuthorityMutationOutcome {
                document,
                idempotent_replay: true,
            });
        }
        if document
            .authority
            .idempotency
            .values()
            .any(|binding| binding.operation_id == request.context.operation_id)
        {
            return Err(Error::IdempotencyConflict(
                request.context.operation_id.to_string(),
            ));
        }
        if document.authority.idempotency.len() == MAX_AUTHORITY_IDEMPOTENCY_BINDINGS {
            return Err(Error::Invalid(
                "estate authority idempotency limit exceeded".into(),
            ));
        }
        document.authority.apply_batch(request)?;
        document.authority.idempotency.insert(
            request.idempotency_key.clone(),
            AuthorityIdempotencyBinding {
                operation_id: request.context.operation_id.clone(),
                request_sha256,
                bound_at: request.context.at,
            },
        );
        self.commit(
            current_bytes,
            document,
            &request.context,
            "estate.authority.apply",
        )
        .map(|document| AuthorityMutationOutcome {
            document,
            idempotent_replay: false,
        })
    }

    pub fn set_desired(&self, request: &SetDesired) -> Result<DesiredOutcome> {
        validate_context(&request.context)?;
        validate_ascii_key(&request.idempotency_key, "idempotency key")?;
        validate_target(&request.target)?;
        let request_sha256 = desired_request_sha256(request);
        let Some(current_bytes) = self.engine.control().get(&self.key)? else {
            return Err(Error::NotFound(self.estate_id.to_string()));
        };
        let mut document = decode(&current_bytes)?;

        if let Some(binding) = document.idempotency.get(&request.idempotency_key) {
            if binding.request_sha256 != request_sha256
                || binding.operation_id != request.context.operation_id
            {
                return Err(Error::IdempotencyConflict(request.idempotency_key.clone()));
            }
            let operation = document
                .operation(&binding.operation_id)
                .cloned()
                .ok_or_else(|| Error::Invalid("idempotency operation is missing".into()))?;
            return Ok(DesiredOutcome {
                document,
                operation,
                idempotent_replay: true,
            });
        }
        if document
            .operations
            .contains_key(request.context.operation_id.as_str())
        {
            return Err(Error::IdempotencyConflict(
                request.context.operation_id.to_string(),
            ));
        }
        if document.idempotency.len() == MAX_IDEMPOTENCY_BINDINGS
            || document.operations.len() == MAX_OPERATIONS
        {
            return Err(Error::Invalid("estate history is at its v1 bound".into()));
        }

        let previous = document.instances.get(request.instance_id.as_str());
        if previous.is_some_and(|instance| target_matches(&instance.desired, &request.target)) {
            return Err(Error::Invalid("desired target is unchanged".into()));
        }
        if previous.is_none() && document.instances.len() == MAX_INSTANCES {
            return Err(Error::Invalid("estate instance limit exceeded".into()));
        }
        let generation = previous
            .map(|instance| instance.desired.generation)
            .unwrap_or(0)
            .checked_add(1)
            .ok_or_else(|| Error::Invalid("desired generation overflow".into()))?;
        let kind = operation_kind(previous, &request.target);
        let desired = DesiredInstance {
            generation,
            phase: request.target.phase,
            deployment_ref: request.target.deployment_ref.clone(),
            version: request.target.version.clone(),
            configuration_sha256: request.target.configuration_sha256.clone(),
            updated_at: request.context.at,
        };
        let managed = previous.cloned().map_or_else(
            || ManagedInstance {
                id: request.instance_id.clone(),
                desired: desired.clone(),
                observed: ObservedInstance {
                    generation: 0,
                    phase: ObservedPhase::Unknown,
                    version: None,
                    process_id: None,
                    observed_at: request.context.at,
                    evidence_sha256: None,
                    error: None,
                },
                activity: InstanceActivity {
                    last_meaningful_runtime_at: None,
                    last_heartbeat_at: None,
                    class: ActivityClass::Unknown,
                    evaluated_at: request.context.at,
                },
            },
            |mut instance| {
                instance.desired = desired.clone();
                instance
            },
        );
        document
            .instances
            .insert(request.instance_id.to_string(), managed);
        for previous_operation in document.operations.values_mut().filter(|operation| {
            operation.instance_id == request.instance_id && !operation.state.is_terminal()
        }) {
            previous_operation.state = OperationState::Superseded;
            previous_operation.updated_at = request.context.at;
            previous_operation.error = Some(format!(
                "superseded by desired generation {generation} operation {}",
                request.context.operation_id
            ));
        }
        let operation = EstateOperation {
            id: request.context.operation_id.clone(),
            instance_id: request.instance_id.clone(),
            kind,
            desired_generation: generation,
            request_sha256: request_sha256.clone(),
            state: OperationState::Pending,
            attempts: 0,
            created_at: request.context.at,
            updated_at: request.context.at,
            lease: None,
            receipts: Vec::new(),
            error: None,
        };
        document
            .operations
            .insert(request.context.operation_id.to_string(), operation.clone());
        document.idempotency.insert(
            request.idempotency_key.clone(),
            IdempotencyBinding {
                operation_id: request.context.operation_id.clone(),
                request_sha256,
                bound_at: request.context.at,
            },
        );
        self.commit(
            current_bytes,
            document,
            &request.context,
            "estate.desired.set",
        )
        .map(|document| DesiredOutcome {
            document,
            operation,
            idempotent_replay: false,
        })
    }

    pub fn acquire_lease(&self, request: &LeaseRequest) -> Result<EstateDocument> {
        validate_context(&request.context)?;
        if !(1_000..=3_600_000).contains(&request.lease_ms) {
            return Err(Error::Invalid("lease_ms must be in 1000..=3600000".into()));
        }
        self.update(&request.context, "estate.operation.lease", |document| {
            let operation = operation_mut(document, &request.context.operation_id)?;
            if operation.state.is_terminal() {
                return Err(Error::Invalid("terminal operation cannot be leased".into()));
            }
            if operation.lease.as_ref().is_some_and(|lease| {
                lease.expires_at > request.context.at && lease.owner != request.worker
            }) {
                return Err(Error::LeaseBusy(operation.id.to_string()));
            }
            if operation.lease.as_ref().is_some_and(|lease| {
                lease.expires_at > request.context.at && lease.owner == request.worker
            }) {
                return Ok(());
            }
            let epoch = operation
                .lease
                .as_ref()
                .map(|lease| lease.epoch)
                .unwrap_or(0)
                .checked_add(1)
                .ok_or_else(|| Error::Invalid("lease epoch overflow".into()))?;
            let expires_at = request
                .context
                .at
                .checked_add(request.lease_ms)
                .ok_or_else(|| Error::Invalid("lease expiry overflow".into()))?;
            operation.attempts = operation
                .attempts
                .checked_add(1)
                .ok_or_else(|| Error::Invalid("operation attempts overflow".into()))?;
            if operation.state == OperationState::Pending {
                operation.state = OperationState::Leased;
            }
            operation.updated_at = request.context.at;
            operation.lease = Some(OperationLease {
                owner: request.worker.clone(),
                epoch,
                acquired_at: request.context.at,
                expires_at,
            });
            Ok(())
        })
    }

    pub fn record_receipt(&self, request: &ReceiptRequest) -> Result<EstateDocument> {
        validate_context(&request.context)?;
        validate_sha256(&request.evidence_sha256, "receipt evidence")?;
        validate_error(request.error.as_deref())?;
        self.update(&request.context, "estate.operation.receipt", |document| {
            let operation = operation_mut(document, &request.context.operation_id)?;
            let target = match request.boundary {
                ReceiptBoundary::Prepared => OperationState::Prepared,
                ReceiptBoundary::Applied => OperationState::Applied,
                ReceiptBoundary::Completed => OperationState::Succeeded,
                ReceiptBoundary::Failed => OperationState::Failed,
            };
            if let Some(existing) = operation.receipts.iter().find(|receipt| {
                receipt.boundary == request.boundary && receipt.lease_epoch == request.lease_epoch
            }) {
                if existing.evidence_sha256 == request.evidence_sha256 {
                    return Ok(());
                }
                return Err(Error::Invalid("receipt boundary was rebound".into()));
            }
            validate_lease(
                operation,
                &request.worker,
                request.lease_epoch,
                request.context.at,
            )?;
            if operation.receipts.len() == MAX_RECEIPTS_PER_OPERATION {
                return Err(Error::Invalid("operation receipt limit exceeded".into()));
            }
            validate_state_advance(operation.state, target)?;
            operation.receipts.push(OperationReceipt {
                boundary: request.boundary,
                lease_epoch: request.lease_epoch,
                at: request.context.at,
                evidence_sha256: request.evidence_sha256.clone(),
            });
            operation.state = target;
            operation.updated_at = request.context.at;
            operation.error = request.error.clone();
            Ok(())
        })
    }

    pub fn record_observation(&self, request: &ObservationRequest) -> Result<EstateDocument> {
        validate_context(&request.context)?;
        validate_sha256(&request.evidence_sha256, "observation evidence")?;
        validate_version(request.version.as_deref())?;
        validate_error(request.error.as_deref())?;
        self.update(&request.context, "estate.instance.observe", |document| {
            let operation = document
                .operation(&request.context.operation_id)
                .cloned()
                .ok_or_else(|| Error::NotFound(request.context.operation_id.to_string()))?;
            validate_lease(
                &operation,
                &request.worker,
                request.lease_epoch,
                request.context.at,
            )?;
            if !matches!(
                operation.state,
                OperationState::Prepared | OperationState::Applied
            ) {
                return Err(Error::Invalid(
                    "observation requires a prepared or applied operation".into(),
                ));
            }
            let instance = document
                .instances
                .get_mut(operation.instance_id.as_str())
                .ok_or_else(|| Error::NotFound(operation.instance_id.to_string()))?;
            instance.observed = ObservedInstance {
                generation: operation.desired_generation,
                phase: request.phase,
                version: request.version.clone(),
                process_id: request.process_id,
                observed_at: request.context.at,
                evidence_sha256: Some(request.evidence_sha256.clone()),
                error: request.error.clone(),
            };
            Ok(())
        })
    }

    pub fn record_activity(&self, evidence: &ActivityEvidence) -> Result<EstateDocument> {
        validate_context(&evidence.context)?;
        self.update(&evidence.context, "estate.instance.activity", |document| {
            let instance = document
                .instances
                .get_mut(evidence.instance_id.as_str())
                .ok_or_else(|| Error::NotFound(evidence.instance_id.to_string()))?;
            instance.activity.last_meaningful_runtime_at = max_option(
                instance.activity.last_meaningful_runtime_at,
                evidence.meaningful_runtime_at,
            );
            instance.activity.last_heartbeat_at =
                max_option(instance.activity.last_heartbeat_at, evidence.heartbeat_at);
            instance.activity.class = document.activity_policy.classify(
                instance.activity.last_meaningful_runtime_at,
                evidence.context.at,
            );
            instance.activity.evaluated_at = evidence.context.at;
            Ok(())
        })
    }

    pub fn refresh_activity(&self, context: &MutationContext) -> Result<EstateDocument> {
        validate_context(context)?;
        self.update(context, "estate.activity.refresh", |document| {
            for instance in document.instances.values_mut() {
                instance.activity.class = document
                    .activity_policy
                    .classify(instance.activity.last_meaningful_runtime_at, context.at);
                instance.activity.evaluated_at = context.at;
            }
            Ok(())
        })
    }

    pub(super) fn update(
        &self,
        context: &MutationContext,
        action: &str,
        mutate: impl FnOnce(&mut EstateDocument) -> Result<()>,
    ) -> Result<EstateDocument> {
        let Some(current_bytes) = self.engine.control().get(&self.key)? else {
            return Err(Error::NotFound(self.estate_id.to_string()));
        };
        let mut document = decode(&current_bytes)?;
        mutate(&mut document)?;
        self.commit(current_bytes, document, context, action)
    }

    pub(super) fn commit(
        &self,
        expected: Vec<u8>,
        mut document: EstateDocument,
        context: &MutationContext,
        action: &str,
    ) -> Result<EstateDocument> {
        if context.at < document.updated_at {
            return Err(Error::Invalid(
                "mutation timestamp precedes the estate revision".into(),
            ));
        }
        document.revision = document
            .revision
            .checked_add(1)
            .ok_or_else(|| Error::Invalid("estate revision overflow".into()))?;
        document.updated_at = context.at;
        document.validate()?;
        let replacement = encode(&document)?;
        self.engine.control().commit(&ControlTransition {
            key: self.key.clone(),
            expected: Some(expected),
            replacement: Some(replacement),
            at: context.at,
            actor: context.actor.clone(),
            action: action.into(),
            request_id: context.request_id.clone(),
            operation_id: context.operation_id.to_string(),
        })?;
        Ok(document)
    }
}
