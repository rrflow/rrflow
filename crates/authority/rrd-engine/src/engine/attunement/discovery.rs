//! Discover the existing installed job lineage without scanning unrelated state.
use super::*;
use rrd_contract::{AttunementJobPage, AttunementJobSummary, ListAttunementJobs};

impl RrdEngine {
    #[allow(clippy::too_many_arguments)]
    pub fn list_attunement_jobs(
        &self,
        session: &CorrelationId,
        token: &CorrelationId,
        request: &ListAttunementJobs,
        now: u64,
        request_id: &str,
        operation_id: &str,
    ) -> Result<AttunementJobPage> {
        request.validate().map_err(contract)?;
        self.authorize(
            session,
            token,
            SecurityAction::EngramRead,
            now,
            request_id,
            operation_id,
        )?;
        let page = self.collect_attunement_jobs(request, now)?;
        self.finish_attunement_discovery(request, page)
    }

    pub(in crate::engine) fn collect_attunement_jobs(
        &self,
        request: &ListAttunementJobs,
        now: u64,
    ) -> Result<AttunementJobPage> {
        request.validate().map_err(contract)?;
        let sequence = self.storage.control().sequence()?;
        let installed = self.attunement_installed_record()?;
        let mut previous = request.after_job_id.clone();
        let mut current = match &previous {
            Some(id) => {
                let (_, job, plan) = self.load_attunement(id)?;
                let generation = self.attunement_replan_generation(&job)?;
                self.attunement_discovery_successor(&job, &plan, generation)?
            }
            None => {
                let id = canonical_prefixed("job", &installed.attunement_plan_sha256)?;
                let (_, job, plan) = self.load_attunement(&id)?;
                let generation = self.attunement_replan_generation(&job)?;
                if generation != 0 || plan.plan_sha256 != installed.attunement_plan_sha256 {
                    return Err(ServiceError::ProjectBindingMismatch);
                }
                Some((job, plan, generation))
            }
        };
        let mut jobs = Vec::new();
        while let Some((job, plan, generation)) = current.take() {
            let successor = self.attunement_discovery_successor(&job, &plan, generation)?;
            let id = job.id.clone();
            jobs.push(AttunementJobSummary {
                job_id: id.clone(),
                predecessor_job_id: previous,
                generation,
                plan_id: plan.id.clone(),
                plan_sha256: plan.plan_sha256.clone(),
                source_sha256: plan.source_sha256.clone(),
                revision: job.revision,
                state: job.state,
                checkpoint_count: u16::try_from(job.checkpoints.len()).map_err(contract)?,
                next_phase: job.next_phase(&plan),
                created_at_unix_ms: job.created_at_unix_ms,
                updated_at_unix_ms: job.updated_at_unix_ms,
            });
            previous = Some(id);
            current = successor;
            if jobs.len() == usize::from(request.limit) {
                break;
            }
        }
        Ok(AttunementJobPage {
            contract_version: ATTUNEMENT_CONTRACT_VERSION,
            target: installed.installation.target,
            control_sequence: sequence,
            observed_at_unix_ms: now,
            after_job_id: request.after_job_id.clone(),
            next_after_job_id: current.as_ref().and(previous),
            jobs,
            source_verification: AttunementSourceVerification::NotChecked,
        })
    }

    pub(in crate::engine) fn finish_attunement_discovery(
        &self,
        request: &ListAttunementJobs,
        page: AttunementJobPage,
    ) -> Result<AttunementJobPage> {
        if self.storage.control().sequence()? != page.control_sequence {
            return Err(ServiceError::StorageConflict(
                "control state changed during attunement discovery; read a new page".into(),
            ));
        }
        page.validate_for(request, &self.instance)
            .map_err(contract)?;
        Ok(page)
    }

    fn attunement_discovery_successor(
        &self,
        job: &AttunementJob,
        plan: &AttunementPlan,
        generation: u64,
    ) -> Result<Option<(AttunementJob, AttunementPlan, u64)>> {
        let key = format!("{}/successor", attunement_job_key(&self.instance, &job.id));
        let Some(bytes) = self.storage.control().get(&key)? else {
            return Ok(None);
        };
        let preview: AttunementReplanPreview = decode(&bytes)?;
        preview
            .validate_for(&job.id, preview.predecessor_revision, plan)
            .map_err(contract)?;
        // A fenced predecessor advanced one revision into CancelRequested; a
        // succeeded predecessor keeps its revision and state (its effects
        // were read-only) and only gains the successor link.
        let lineage = match job.state {
            AttunementJobState::CancelRequested | AttunementJobState::Cancelled => preview
                .predecessor_revision
                .checked_add(1)
                .is_some_and(|revision| job.revision >= revision),
            AttunementJobState::Succeeded => job.revision == preview.predecessor_revision,
            _ => false,
        };
        if generation.checked_add(1) != Some(preview.generation) || !lineage {
            return Err(ServiceError::Storage(
                "attunement successor generation or predecessor state is inconsistent".into(),
            ));
        }
        let id = canonical_prefixed("job", &preview.plan.plan_sha256)?;
        let (_, successor, successor_plan) = self.load_attunement(&id)?;
        let successor_generation = self.attunement_replan_generation(&successor)?;
        if successor_plan != preview.plan
            || successor_generation != preview.generation
            || successor.id == job.id
        {
            return Err(ServiceError::Storage(
                "attunement successor binding is inconsistent".into(),
            ));
        }
        Ok(Some((successor, successor_plan, successor_generation)))
    }
}
