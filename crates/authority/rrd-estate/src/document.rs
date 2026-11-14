//! Estate document maintenance and the public snapshot.

use super::*;

impl EstateDocument {
    pub fn new(id: CanonicalId, at: u64) -> Result<Self> {
        require_time(at)?;
        Ok(Self {
            format: ESTATE_FORMAT,
            id,
            revision: 1,
            created_at: at,
            updated_at: at,
            activity_policy: ActivityPolicy::default(),
            authority: EstateAuthorityState::default(),
            instances: BTreeMap::new(),
            operations: BTreeMap::new(),
            idempotency: BTreeMap::new(),
            backup_jobs: backup_job::empty_backup_jobs(),
            backup_idempotency: backup_job::empty_backup_idempotency(),
            recovery_policies: recovery::empty_recovery_policies(),
            recovery_points: recovery::empty_recovery_points(),
            recovery_pins: recovery::empty_recovery_pins(),
            restore_evidence: recovery::empty_restore_evidence(),
            recovery_idempotency: recovery::empty_recovery_idempotency(),
            recovery_prune_intents: recovery::empty_recovery_prune_intents(),
        })
    }

    pub fn validate(&self) -> Result<()> {
        if self.format != ESTATE_FORMAT || self.revision == 0 {
            return Err(Error::Invalid(
                "unsupported estate format or revision".into(),
            ));
        }
        require_time(self.created_at)?;
        require_time(self.updated_at)?;
        if self.updated_at < self.created_at {
            return Err(Error::Invalid(
                "estate updated_at precedes created_at".into(),
            ));
        }
        self.activity_policy.validate()?;
        self.authority.validate()?;
        if self.instances.len() > MAX_INSTANCES
            || self.operations.len() > MAX_OPERATIONS
            || self.idempotency.len() > MAX_IDEMPOTENCY_BINDINGS
        {
            return Err(Error::Invalid(
                "estate aggregate cardinality exceeded".into(),
            ));
        }
        for (key, instance) in &self.instances {
            validate_id_key(key, &instance.id)?;
            validate_desired(&instance.desired)?;
            validate_observed(&instance.observed, instance.desired.generation)?;
            require_time(instance.activity.evaluated_at)?;
            if instance.activity.class
                != self.activity_policy.classify(
                    instance.activity.last_meaningful_runtime_at,
                    instance.activity.evaluated_at,
                )
            {
                return Err(Error::Invalid(format!(
                    "instance {} activity classification is inconsistent",
                    instance.id
                )));
            }
        }
        for (key, operation) in &self.operations {
            validate_id_key(key, &operation.id)?;
            let instance = self
                .instances
                .get(operation.instance_id.as_str())
                .ok_or_else(|| {
                    Error::Invalid(format!(
                        "operation {} names an unknown instance",
                        operation.id
                    ))
                })?;
            if operation.desired_generation == 0
                || operation.desired_generation > instance.desired.generation
            {
                return Err(Error::Invalid(format!(
                    "operation {} has an invalid desired generation",
                    operation.id
                )));
            }
            validate_sha256(&operation.request_sha256, "operation request")?;
            require_time(operation.created_at)?;
            require_time(operation.updated_at)?;
            if operation.updated_at < operation.created_at {
                return Err(Error::Invalid(format!(
                    "operation {} updated_at precedes created_at",
                    operation.id
                )));
            }
            validate_error(operation.error.as_deref())?;
            if let Some(lease) = &operation.lease {
                if lease.epoch == 0
                    || lease.acquired_at == 0
                    || lease.expires_at <= lease.acquired_at
                {
                    return Err(Error::Invalid(format!(
                        "operation {} has an invalid lease",
                        operation.id
                    )));
                }
            }
            if operation.receipts.len() > MAX_RECEIPTS_PER_OPERATION {
                return Err(Error::Invalid("operation receipt limit exceeded".into()));
            }
            for receipt in &operation.receipts {
                if receipt.lease_epoch == 0 || receipt.at == 0 {
                    return Err(Error::Invalid("operation receipt is invalid".into()));
                }
                validate_sha256(&receipt.evidence_sha256, "operation receipt")?;
            }
        }
        for (key, binding) in &self.idempotency {
            validate_ascii_key(key, "idempotency key")?;
            validate_sha256(&binding.request_sha256, "idempotency request")?;
            require_time(binding.bound_at)?;
            if !self.operations.contains_key(binding.operation_id.as_str()) {
                return Err(Error::Invalid(format!(
                    "idempotency key {key} names an unknown operation"
                )));
            }
        }
        backup_job::validate_backup_state(self)?;
        recovery::validate_recovery_state(self)?;
        Ok(())
    }

    pub fn instance(&self, id: &CanonicalId) -> Option<&ManagedInstance> {
        self.instances.get(id.as_str())
    }

    pub fn operation(&self, id: &CanonicalId) -> Option<&EstateOperation> {
        self.operations.get(id.as_str())
    }
}

pub fn public_snapshot(document: &EstateDocument) -> rrd_contract::EstateSnapshot {
    rrd_contract::EstateSnapshot {
        format_version: document.format,
        id: document.id.clone(),
        revision: document.revision,
        created_at_unix_ms: document.created_at,
        updated_at_unix_ms: document.updated_at,
        activity_policy: rrd_contract::EstateActivityPolicySnapshot {
            idle_after_ms: document.activity_policy.idle_after_ms,
            stale_after_ms: document.activity_policy.stale_after_ms,
            neglected_after_ms: document.activity_policy.neglected_after_ms,
        },
        authority: rrd_contract::EstateAuthoritySnapshot {
            catalogue_sha256: document.authority.catalogue_sha256(),
            resources: document
                .authority
                .resources
                .values()
                .map(|resource| rrd_contract::EstateAuthorityResourceSnapshot {
                    id: resource.id.clone(),
                    kind: public_authority_kind(resource.kind),
                    parent_ids: resource
                        .parent_id
                        .iter()
                        .chain(resource.secondary_parent_id.iter())
                        .cloned()
                        .collect(),
                    name: resource.name.clone(),
                    spec_sha256: resource.spec_sha256.clone(),
                    desired: resource.desired.as_ref().map(|state| {
                        rrd_contract::EstateAuthorityDesiredSnapshot {
                            generation: state.generation,
                            spec_sha256: state.spec_sha256.clone(),
                            updated_at_unix_ms: state.updated_at,
                        }
                    }),
                    observed: resource.observed.as_ref().map(|state| {
                        rrd_contract::EstateAuthorityObservedSnapshot {
                            generation: state.generation,
                            status: public_authority_status(state.status),
                            observed_at_unix_ms: state.observed_at,
                            evidence_sha256: state.evidence_sha256.clone(),
                            error: state.error.clone(),
                        }
                    }),
                    secret_reference_ids: resource.secret_reference_ids.clone(),
                    created_at_unix_ms: resource.created_at,
                    updated_at_unix_ms: resource.updated_at,
                })
                .collect(),
            receipts: document
                .authority
                .receipts
                .values()
                .map(|receipt| rrd_contract::EstateAuthorityReceiptSnapshot {
                    id: receipt.id.clone(),
                    resource_id: receipt.resource_id.clone(),
                    operation_id: receipt.operation_id.clone(),
                    lease_epoch: receipt.lease_epoch,
                    boundary: public_authority_receipt_boundary(receipt.boundary),
                    at_unix_ms: receipt.at,
                    evidence_sha256: receipt.evidence_sha256.clone(),
                })
                .collect(),
            idempotency_binding_count: u32::try_from(document.authority.idempotency.len())
                .expect("bounded authority idempotency count fits u32"),
        },
        instances: document
            .instances
            .values()
            .map(|instance| rrd_contract::EstateInstanceSnapshot {
                id: instance.id.clone(),
                desired: rrd_contract::EstateDesiredSnapshot {
                    generation: instance.desired.generation,
                    phase: match instance.desired.phase {
                        DesiredPhase::Running => rrd_contract::EstateDesiredPhase::Running,
                        DesiredPhase::Stopped => rrd_contract::EstateDesiredPhase::Stopped,
                        DesiredPhase::Absent => rrd_contract::EstateDesiredPhase::Absent,
                    },
                    deployment_ref: instance.desired.deployment_ref.clone(),
                    version: instance.desired.version.clone(),
                    configuration_sha256: instance.desired.configuration_sha256.clone(),
                    updated_at_unix_ms: instance.desired.updated_at,
                },
                observed: rrd_contract::EstateObservedSnapshot {
                    generation: instance.observed.generation,
                    phase: public_observed_phase(instance.observed.phase),
                    version: instance.observed.version.clone(),
                    process_id: instance.observed.process_id,
                    observed_at_unix_ms: instance.observed.observed_at,
                    evidence_sha256: instance.observed.evidence_sha256.clone(),
                    error: instance.observed.error.clone(),
                },
                activity: rrd_contract::EstateActivitySnapshot {
                    last_meaningful_runtime_at_unix_ms: instance
                        .activity
                        .last_meaningful_runtime_at,
                    last_heartbeat_at_unix_ms: instance.activity.last_heartbeat_at,
                    class: match instance.activity.class {
                        ActivityClass::Unknown => rrd_contract::EstateActivityClass::Unknown,
                        ActivityClass::Active => rrd_contract::EstateActivityClass::Active,
                        ActivityClass::Idle => rrd_contract::EstateActivityClass::Idle,
                        ActivityClass::Stale => rrd_contract::EstateActivityClass::Stale,
                        ActivityClass::Neglected => rrd_contract::EstateActivityClass::Neglected,
                    },
                    evaluated_at_unix_ms: instance.activity.evaluated_at,
                },
            })
            .collect(),
        operations: document
            .operations
            .values()
            .map(|operation| rrd_contract::EstateOperationSnapshot {
                id: operation.id.clone(),
                instance_id: operation.instance_id.clone(),
                kind: match operation.kind {
                    OperationKind::Provision => rrd_contract::EstateOperationKind::Provision,
                    OperationKind::Start => rrd_contract::EstateOperationKind::Start,
                    OperationKind::Stop => rrd_contract::EstateOperationKind::Stop,
                    OperationKind::Restart => rrd_contract::EstateOperationKind::Restart,
                    OperationKind::Upgrade => rrd_contract::EstateOperationKind::Upgrade,
                    OperationKind::Delete => rrd_contract::EstateOperationKind::Delete,
                },
                desired_generation: operation.desired_generation,
                request_sha256: operation.request_sha256.clone(),
                state: match operation.state {
                    OperationState::Pending => rrd_contract::EstateOperationState::Pending,
                    OperationState::Leased => rrd_contract::EstateOperationState::Leased,
                    OperationState::Prepared => rrd_contract::EstateOperationState::Prepared,
                    OperationState::Applied => rrd_contract::EstateOperationState::Applied,
                    OperationState::Succeeded => rrd_contract::EstateOperationState::Succeeded,
                    OperationState::Failed => rrd_contract::EstateOperationState::Failed,
                    OperationState::Superseded => rrd_contract::EstateOperationState::Superseded,
                },
                attempts: operation.attempts,
                created_at_unix_ms: operation.created_at,
                updated_at_unix_ms: operation.updated_at,
                lease: operation
                    .lease
                    .as_ref()
                    .map(|lease| rrd_contract::EstateLeaseSnapshot {
                        owner: lease.owner.clone(),
                        epoch: lease.epoch,
                        acquired_at_unix_ms: lease.acquired_at,
                        expires_at_unix_ms: lease.expires_at,
                    }),
                receipts: operation
                    .receipts
                    .iter()
                    .map(|receipt| rrd_contract::EstateReceiptSnapshot {
                        boundary: match receipt.boundary {
                            ReceiptBoundary::Prepared => {
                                rrd_contract::EstateReceiptBoundary::Prepared
                            }
                            ReceiptBoundary::Applied => {
                                rrd_contract::EstateReceiptBoundary::Applied
                            }
                            ReceiptBoundary::Completed => {
                                rrd_contract::EstateReceiptBoundary::Completed
                            }
                            ReceiptBoundary::Failed => rrd_contract::EstateReceiptBoundary::Failed,
                        },
                        lease_epoch: receipt.lease_epoch,
                        at_unix_ms: receipt.at,
                        evidence_sha256: receipt.evidence_sha256.clone(),
                    })
                    .collect(),
                error: operation.error.clone(),
            })
            .collect(),
        idempotency_binding_count: u32::try_from(document.idempotency.len())
            .expect("bounded estate idempotency count fits u32"),
    }
}

pub(super) fn public_observed_phase(phase: ObservedPhase) -> rrd_contract::EstateObservedPhase {
    match phase {
        ObservedPhase::Unknown => rrd_contract::EstateObservedPhase::Unknown,
        ObservedPhase::Provisioning => rrd_contract::EstateObservedPhase::Provisioning,
        ObservedPhase::Starting => rrd_contract::EstateObservedPhase::Starting,
        ObservedPhase::Running => rrd_contract::EstateObservedPhase::Running,
        ObservedPhase::Stopping => rrd_contract::EstateObservedPhase::Stopping,
        ObservedPhase::Stopped => rrd_contract::EstateObservedPhase::Stopped,
        ObservedPhase::Deleting => rrd_contract::EstateObservedPhase::Deleting,
        ObservedPhase::Absent => rrd_contract::EstateObservedPhase::Absent,
        ObservedPhase::Failed => rrd_contract::EstateObservedPhase::Failed,
    }
}
