//! Bounded source-observation groups; only the final transaction certifies a
//! complete snapshot. Progress belongs to the same engine-owned job and phase.
use super::*;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Progress {
    format_version: u32,
    job_id: CanonicalId,
    plan_sha256: String,
    phase_index: u64,
    proposal_sha256: String,
    publication_sha256: String,
    prior_head_sha256: Option<String>,
    completed_groups: usize,
    read: ReadStamp,
}

enum PublicationError {
    Store(rrd_store::Error),
    Engine(ServiceError),
}
impl From<rrd_store::Error> for PublicationError {
    fn from(error: rrd_store::Error) -> Self {
        Self::Store(error)
    }
}
impl From<ServiceError> for PublicationError {
    fn from(error: ServiceError) -> Self {
        Self::Engine(error)
    }
}
impl From<PublicationError> for ServiceError {
    fn from(error: PublicationError) -> Self {
        match error {
            PublicationError::Store(error) => error.into(),
            PublicationError::Engine(error) => error,
        }
    }
}
impl PublicationError {
    fn known_size_rejection(&self) -> bool {
        matches!(self, Self::Store(rrd_store::Error::WriteBatchLimit { .. }))
    }
}

pub(super) struct Publication<'a> {
    pub engine: &'a RrdEngine,
    pub work: &'a Work<'a>,
    pub proposal: &'a Proposal,
    pub prior_bytes: &'a Option<Vec<u8>>,
    pub check: &'a dyn Fn() -> Result<()>,
    pub recheck_source: &'a dyn Fn() -> Result<()>,
    pub output_limit: u64,
}

impl Publication<'_> {
    fn at(&self) -> Result<u64> {
        elapsed_time(self.work.now, self.work.started)
    }

    fn authorize(&self) -> Result<()> {
        (self.check)()?;
        self.engine.authorize(
            self.work.session,
            self.work.token,
            SecurityAction::EngramAdmin,
            self.at()?,
            self.work.context.request_id.as_str(),
            self.work.context.operation_id.as_str(),
        )?;
        Ok(())
    }

    fn transition(
        &self,
        key: String,
        expected: Option<Vec<u8>>,
        replacement: Option<Vec<u8>>,
        action: &str,
        at: u64,
    ) -> ControlTransition {
        ControlTransition {
            key,
            expected,
            replacement,
            at,
            actor: format!("session:{}", self.work.session.as_str()),
            action: action.into(),
            request_id: self.work.context.request_id.as_str().into(),
            operation_id: self.work.context.operation_id.as_str().into(),
        }
    }

    fn finish(
        &self,
        commit: &RuntimeCommit,
        progress: Option<(&str, &[u8])>,
        permit: &mut TransactionPermit<'_>,
    ) -> std::result::Result<AttunementControlResult, PublicationError> {
        self.authorize()?;
        // Reuse validation can take time even when the remaining commit fits
        // one batch. Every completion rechecks source at this shared boundary.
        (self.recheck_source)()?;
        (self.check)()?;
        let (_, _, accepted) = self.engine.storage.runtime().commit_with_control_builder(
            commit,
            |read, outcome| {
                (self.check)()?;
                if outcome.last_cursor != read.commit_cursor {
                    return Err(ServiceError::Storage(
                        "Inventory checkpoint cursor mismatch".into(),
                    )
                    .into());
                }
                let finished = self.at()?;
                let before = &self.work.before;
                if finished <= before.updated_at_unix_ms {
                    return Err(contract("attunement transition time must advance").into());
                }
                let mut next = before.clone();
                next.revision = next
                    .revision
                    .checked_add(1)
                    .ok_or_else(|| contract("attunement revision overflow"))?;
                next.updated_at_unix_ms = finished;
                let selected = self
                    .work
                    .plan
                    .phases
                    .get(next.checkpoints.len())
                    .ok_or_else(|| contract("missing selected Inventory step"))?;
                if selected.phase != AttunementPhase::Inventory {
                    return Err(contract("selected phase is not Inventory").into());
                }
                let input_sha256 = next.checkpoints.last().map_or_else(
                    || self.work.plan.source_sha256.clone(),
                    |c| c.output_sha256.clone(),
                );
                let mut checkpoint = AttunementPhaseCheckpoint {
                    contract_version: ATTUNEMENT_CONTRACT_VERSION,
                    job_id: next.id.clone(),
                    plan_sha256: self.work.plan.plan_sha256.clone(),
                    phase: AttunementPhase::Inventory,
                    sequence: selected.sequence,
                    configuration_sha256: selected.configuration_sha256.clone(),
                    input_sha256,
                    output_sha256: self.proposal.proposal_sha256().into(),
                    coordinates: AttunementRuntimeCoordinates {
                        runtime_manifest_sha256: read.manifest_id.clone(),
                        runtime_cursor: read.commit_cursor,
                        schema_revision: read.schema_revision,
                        catalogue_revision: read.catalog_revision,
                    },
                    committed_at_unix_ms: finished,
                    checkpoint_sha256: "0".repeat(64),
                };
                checkpoint.checkpoint_sha256 =
                    attunement_checkpoint_sha256(&checkpoint).map_err(contract)?;
                next.checkpoints.push(checkpoint);
                advance_after_checkpoint(&mut next, &self.work.plan);
                before
                    .validate_transition(&next, &self.work.plan)
                    .map_err(contract)?;
                let accepted = status(next, &self.work.plan, finished)?;
                let receipt = Receipt {
                    format_version: 1,
                    principal_id: self.work.principal.clone(),
                    operation_sha256: self.work.operation_sha256.clone(),
                    status: accepted.clone(),
                };
                let head = records::Head {
                    format_version: 1,
                    project_id: self.work.plan.target.project_id.clone(),
                    proposal_sha256: self.proposal.proposal_sha256().into(),
                    read: read.clone(),
                };
                let mut transitions = vec![
                    self.transition(
                        attunement_job_key(&self.engine.instance, &before.id),
                        Some(self.work.before_bytes.clone()),
                        Some(encode(&accepted.job)?),
                        "attunement.inventory_checkpointed",
                        finished,
                    ),
                    self.transition(
                        self.work.receipt_key.clone(),
                        None,
                        Some(encode(&receipt)?),
                        "attunement.receipted",
                        finished,
                    ),
                    self.transition(
                        records::head_key(&self.engine.instance),
                        self.prior_bytes.clone(),
                        Some(encode(&head)?),
                        "attunement.inventory_published",
                        finished,
                    ),
                ];
                if let Some((key, bytes)) = progress {
                    transitions.push(self.transition(
                        key.into(),
                        Some(bytes.into()),
                        None,
                        "attunement.inventory_progress_completed",
                        finished,
                    ));
                }
                bounded(&(commit, &transitions), self.output_limit)?;
                permit.charge(
                    &(commit, &transitions),
                    self.engine
                        .engram_configuration
                        .storage
                        .transaction_admission_bytes,
                )?;
                (self.check)()?;
                Ok::<_, PublicationError>((transitions, accepted))
            },
        )?;
        Ok(AttunementControlResult {
            status: accepted,
            idempotent_replay: false,
        })
    }

    pub(super) fn publish(
        &self,
        mut full: RuntimeCommit,
        mut permit: TransactionPermit<'_>,
        publication_sha256: &str,
    ) -> Result<AttunementControlResult> {
        let key = format!(
            "server/state/{}/inventory/progress/{}/{}",
            self.engine.instance,
            self.work.before.id,
            self.work.before.checkpoints.len()
        );
        let mut progress_bytes = self.engine.storage.control().get(&key)?;
        let prior_head_sha256 = self
            .prior_bytes
            .as_ref()
            .map(|bytes| digest::sha256_hex(bytes));
        let mut offset = 0;
        if let Some(bytes) = &progress_bytes {
            let progress: Progress = decode(bytes)?;
            if progress.format_version != 2
                || progress.job_id != self.work.before.id
                || progress.plan_sha256 != self.work.plan.plan_sha256
                || progress.phase_index != self.work.before.checkpoints.len() as u64
                || progress.read.scope != full.scope
                || !records::is_digest(&progress.proposal_sha256)
                || !records::is_digest(&progress.publication_sha256)
                || progress
                    .prior_head_sha256
                    .as_ref()
                    .is_some_and(|digest| !records::is_digest(digest))
                || progress.completed_groups == 0
            {
                return Err(ServiceError::Storage(
                    "invalid Inventory progress identity".into(),
                ));
            }
            self.engine
                .storage
                .runtime()
                .validate_read_stamp(&progress.read)?;
            let current = self.engine.storage.runtime().read_stamp(&full.scope)?;
            if progress.proposal_sha256 == self.proposal.proposal_sha256()
                && progress.publication_sha256 == publication_sha256
                && progress.prior_head_sha256 == prior_head_sha256
                && progress.read == current
            {
                offset = progress.completed_groups;
            }
        } else {
            // A small inventory keeps the original single-transaction path.
            match self.finish(&full, None, &mut permit) {
                Ok(accepted) => return Ok(accepted),
                Err(error) if error.known_size_rejection() => (),
                Err(error) => return Err(error.into()),
            }
        }
        let parts = records::partition(std::mem::take(&mut full.mutations))?;
        if offset > parts.groups.len() {
            return Err(ServiceError::Storage(
                "Inventory progress exceeds its proposal".into(),
            ));
        }
        let mut width = parts.groups.len() - offset;
        while offset < parts.groups.len() {
            self.authorize()?;
            let end = (offset + width).min(parts.groups.len());
            let mut mutations = Vec::new();
            if offset == 0 {
                mutations.extend(parts.schema.iter().cloned());
            }
            mutations.extend(parts.groups[offset..end].iter().flatten().cloned());
            let read = self.engine.storage.runtime().read_stamp(&full.scope)?;
            let commit = RuntimeCommit {
                scope: full.scope.clone(),
                at: self.at()?,
                actor: full.actor.clone(),
                expected_cursor: read.commit_cursor,
                mutations,
            };
            let result = self.engine.storage.runtime().commit_with_control_builder(
                &commit,
                |read, outcome| {
                    (self.check)()?;
                    if outcome.last_cursor != read.commit_cursor {
                        return Err(contract("Inventory group cursor mismatch").into());
                    }
                    let progress = Progress {
                        format_version: 2,
                        job_id: self.work.before.id.clone(),
                        plan_sha256: self.work.plan.plan_sha256.clone(),
                        phase_index: self.work.before.checkpoints.len() as u64,
                        proposal_sha256: self.proposal.proposal_sha256().into(),
                        publication_sha256: publication_sha256.into(),
                        prior_head_sha256: prior_head_sha256.clone(),
                        completed_groups: end,
                        read: read.clone(),
                    };
                    let replacement = encode(&progress)?;
                    let at = self.at()?;
                    let transitions = vec![
                        self.transition(
                            attunement_job_key(&self.engine.instance, &self.work.before.id),
                            Some(self.work.before_bytes.clone()),
                            Some(self.work.before_bytes.clone()),
                            "attunement.inventory_group_guarded",
                            at,
                        ),
                        self.transition(
                            key.clone(),
                            progress_bytes.clone(),
                            Some(replacement.clone()),
                            "attunement.inventory_group_published",
                            at,
                        ),
                    ];
                    bounded(&(&commit, &transitions), self.output_limit)?;
                    permit.charge(
                        &(&commit, &transitions),
                        self.engine
                            .engram_configuration
                            .storage
                            .transaction_admission_bytes,
                    )?;
                    (self.check)()?;
                    Ok::<_, PublicationError>((transitions, replacement))
                },
            );
            match result {
                Ok((_, _, replacement)) => {
                    progress_bytes = Some(replacement);
                    offset = end;
                }
                Err(error) if error.known_size_rejection() && width > 1 => {
                    width = (width / 2).max(1);
                }
                Err(error) => return Err(error.into()),
            }
        }
        // Groups alone never certify complete input. The shared finish boundary
        // revalidates source before publishing the new complete head.
        (self.check)()?;
        let read = self.engine.storage.runtime().read_stamp(&full.scope)?;
        full.expected_cursor = read.commit_cursor;
        full.at = self.at()?;
        full.mutations = if parts.groups.is_empty() {
            parts.schema.into_iter().chain(parts.complete).collect()
        } else {
            parts.complete
        };
        self.finish(
            &full,
            progress_bytes.as_deref().map(|bytes| (key.as_str(), bytes)),
            &mut permit,
        )
        .map_err(Into::into)
    }
}
