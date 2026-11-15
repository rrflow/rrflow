//! Keeping an installed project attuned while the engine serves.
//!
//! Under a sealed [`AttunementFollowPolicy`], the serving process calls
//! [`RrdEngine::follow_attunement`] on its own schedule. A cheap fingerprint of
//! the project's files decides whether anything changed; when it did, or on
//! the first call after the engine opened, the engine derives the successor
//! plan from the committed evidence, accepts it when it differs from the
//! newest job's plan, and steps the job to its end. Every effect goes through
//! the public attunement operations as the installed operator, so each step
//! carries its receipt, its audit entry and its lease exactly as a client's
//! would. A job another driver holds, or one that needs an operator, is left
//! alone.

use super::*;
use crate::engine::clock::{observe_required, ClockSource, HostSystemClock};
use rrd_contract::{AttunementJobSummary, CreateSession, ListAttunementJobs, SessionLimits};

/// The lease the follower takes on a job it steps.
const FOLLOW_LEASE_MS: u64 = 60_000;
/// Steps the follower takes in one call before yielding to the next interval.
const MAX_FOLLOW_STEPS: u64 = 10_000;
/// Times the follower repeats one Step a concurrent commit refused.
const MAX_STEP_CONFLICTS: u32 = 8;
/// Jobs the follower drives in one call: the pending one, then successors
/// until the newest plan matches what the evidence derives.
const MAX_FOLLOW_ROUNDS: usize = 4;

/// What one call to [`RrdEngine::follow_attunement`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FollowOutcome {
    /// No follow policy is sealed into this installation.
    Disabled,
    /// The project's files did not change since the last call.
    Unchanged,
    /// The newest job's plan already matches what the evidence derives.
    Current { job_id: CanonicalId },
    /// The newest job is held by another driver or needs an operator.
    Waiting {
        job_id: CanonicalId,
        state: AttunementJobState,
    },
    /// The follower stepped a job to an end.
    Attuned {
        job_id: CanonicalId,
        state: AttunementJobState,
        steps: u64,
    },
}

/// The outcome and the fingerprint to pass to the next call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FollowTick {
    pub fingerprint: Option<String>,
    pub outcome: FollowOutcome,
}

struct Follower<'a> {
    engine: &'a RrdEngine,
    clock: &'a dyn ClockSource,
    session: CorrelationId,
    token: CorrelationId,
    run: String,
    calls: u64,
}

impl Follower<'_> {
    fn now(&self) -> Result<u64> {
        Ok(observe_required(self.clock, "attunement.follow")?.observed_at_unix_ms)
    }

    fn context(&mut self, mutation: bool) -> Result<RequestContext> {
        self.calls += 1;
        let name = format!("follow-{}-{}", self.run, self.calls);
        Ok(RequestContext {
            request_id: CorrelationId::new(name.clone()).map_err(contract)?,
            operation_id: CorrelationId::new(name.clone()).map_err(contract)?,
            idempotency_key: mutation
                .then(|| CorrelationId::new(name).map_err(contract))
                .transpose()?,
            deadline_unix_ms: None,
        })
    }

    fn newest(&mut self) -> Result<Option<AttunementJobSummary>> {
        let mut after = None;
        let mut newest: Option<AttunementJobSummary> = None;
        loop {
            let context = self.context(false)?;
            let page = self.engine.list_attunement_jobs(
                &self.session,
                &self.token,
                &ListAttunementJobs {
                    contract_version: ATTUNEMENT_CONTRACT_VERSION,
                    after_job_id: after,
                    limit: 64,
                },
                self.now()?,
                context.request_id.as_str(),
                context.operation_id.as_str(),
            )?;
            for job in page.jobs {
                if newest.as_ref().is_none_or(|current| {
                    (job.generation, job.created_at_unix_ms)
                        > (current.generation, current.created_at_unix_ms)
                }) {
                    newest = Some(job);
                }
            }
            match page.next_after_job_id {
                Some(next) => after = Some(next),
                None => return Ok(newest),
            }
        }
    }

    fn control(&mut self, command: AttunementCommand) -> Result<AttunementJob> {
        let context = self.context(true)?;
        Ok(self
            .engine
            .control_attunement(&self.session, &self.token, &command, &context, self.now()?)?
            .status
            .job)
    }

    /// Starts a pending job and steps it until it leaves Running.
    fn drive(&mut self, job: AttunementJob) -> Result<FollowOutcome> {
        let (_, _, plan) = self.engine.load_attunement(&job.id)?;
        let job = self.control(AttunementCommand::Start {
            job_id: job.id.clone(),
            plan_sha256: plan.plan_sha256.clone(),
            expected_revision: job.revision,
            lease_ms: FOLLOW_LEASE_MS,
        })?;
        self.step_to_end(job)
    }

    /// Recovers a running job whose lease expired, which an earlier pass that
    /// failed mid-job leaves behind, resumes it from its last checkpoint
    /// under a new lease and steps it to the end.
    fn recover(&mut self, job: AttunementJob) -> Result<FollowOutcome> {
        let paused = self.control(AttunementCommand::RecoverExpired {
            job_id: job.id.clone(),
            expected_revision: job.revision,
        })?;
        let (_, _, plan) = self.engine.load_attunement(&paused.id)?;
        let resume_from_phase = paused
            .current_phase
            .ok_or_else(|| contract("a recovered job has no phase to resume"))?;
        let requested_at_unix_ms = self.now()?;
        let resumed = self.control(AttunementCommand::Resume {
            request: rrd_contract::ResumeAttunement {
                contract_version: ATTUNEMENT_CONTRACT_VERSION,
                job_id: paused.id.clone(),
                plan_sha256: plan.plan_sha256.clone(),
                expected_job_revision: paused.revision,
                last_checkpoint_sha256: paused
                    .checkpoints
                    .last()
                    .map(|checkpoint| checkpoint.checkpoint_sha256.clone()),
                resume_from_phase,
                requested_at_unix_ms,
            },
            lease_ms: FOLLOW_LEASE_MS,
        })?;
        self.step_to_end(resumed)
    }

    fn step_to_end(&mut self, mut job: AttunementJob) -> Result<FollowOutcome> {
        let mut steps = 0;
        let mut conflicts = 0;
        while job.state == AttunementJobState::Running && steps < MAX_FOLLOW_STEPS {
            let lease = job
                .lease
                .as_ref()
                .ok_or_else(|| contract("a running job has no lease"))?;
            match self.control(AttunementCommand::Step {
                job_id: job.id.clone(),
                expected_revision: job.revision,
                lease_generation: lease.generation,
            }) {
                Ok(next) => {
                    job = next;
                    steps += 1;
                    conflicts = 0;
                }
                // A Step another writer's commit raced lost nothing: it did
                // not commit. Read the job again and repeat it, a bounded
                // number of times; past that the pass ends and a later pass
                // recovers the job once its lease expires.
                Err(ServiceError::StorageConflict(_)) if conflicts < MAX_STEP_CONFLICTS => {
                    conflicts += 1;
                    job = self.engine.load_attunement(&job.id)?.1;
                }
                Err(error) => return Err(error),
            }
        }
        Ok(FollowOutcome::Attuned {
            job_id: job.id,
            state: job.state,
            steps,
        })
    }
}

impl RrdEngine {
    /// One follow pass; see the module documentation. Pass the fingerprint
    /// the previous call returned, or `None` on the first call.
    pub fn follow_attunement(&self, last_fingerprint: Option<&str>) -> Result<FollowTick> {
        self.follow_attunement_with_clock(&HostSystemClock, last_fingerprint)
    }

    pub(in crate::engine) fn follow_attunement_with_clock(
        &self,
        clock: &dyn ClockSource,
        last_fingerprint: Option<&str>,
    ) -> Result<FollowTick> {
        if self.engram_configuration.attunement_follow.is_none() {
            return Ok(FollowTick {
                fingerprint: None,
                outcome: FollowOutcome::Disabled,
            });
        }
        let root = self
            .installed_project_root
            .as_ref()
            .ok_or(ServiceError::ProjectBindingMismatch)?;
        let installed = self.attunement_installed_record()?;
        let (profile, digest) =
            DefaultInstallProfile::load(installed.installation.profile_id.as_str())?;
        if digest != installed.installation.profile_sha256 {
            return Err(ServiceError::ProjectBindingMismatch);
        }
        let fingerprint = crate::engine::installation::project_change_fingerprint(root, &profile)?;
        if last_fingerprint == Some(fingerprint.as_str()) {
            return Ok(FollowTick {
                fingerprint: Some(fingerprint),
                outcome: FollowOutcome::Unchanged,
            });
        }
        let key = self.installed_operator_key()?;
        let now = observe_required(clock, "attunement.follow")?.observed_at_unix_ms;
        let session = self.create_authenticated_session(
            &key.principal_id,
            key.credential().as_bytes(),
            &CreateSession {
                // The longest session the contract allows, so one long Parse
                // does not outlive it; a pass it does outlive is recovered.
                limits: SessionLimits {
                    idle_timeout_ms: rrd_contract::MAX_LEASE_MS,
                    absolute_timeout_ms: rrd_contract::MAX_LEASE_MS,
                    max_open_transactions: 2,
                },
            },
            &CorrelationId::new(format!("follow-session-{now}")).map_err(contract)?,
            now,
            &format!("follow-session-{now}"),
            &format!("follow-session-{now}"),
        )?;
        let mut follower = Follower {
            engine: self,
            clock,
            session: session.session_id,
            token: session.token,
            run: now.to_string(),
            calls: 0,
        };
        // Drive what is pending, then bring the newest job's plan up to what
        // the evidence derives: a fresh install is inventoried and parsed in
        // one pass, and an edit is relearned in one pass.
        let mut outcome = FollowOutcome::Unchanged;
        for _ in 0..MAX_FOLLOW_ROUNDS {
            let Some(newest) = follower.newest()? else {
                break;
            };
            let (_, job, plan) = self.load_attunement(&newest.job_id)?;
            match job.state {
                AttunementJobState::Pending => {
                    outcome = follower.drive(job)?;
                }
                AttunementJobState::Running
                    if job.lease.as_ref().is_some_and(|lease| {
                        lease.expires_at_unix_ms <= follower.now().unwrap_or(0)
                    }) =>
                {
                    outcome = follower.recover(job)?;
                }
                AttunementJobState::Succeeded => {
                    let context = follower.context(false)?;
                    let preview = self.preview_attunement_replan(
                        &follower.session,
                        &follower.token,
                        &job.id,
                        job.revision,
                        follower.now()?,
                        context.request_id.as_str(),
                        context.operation_id.as_str(),
                    )?;
                    let phases = |plan: &AttunementPlan| {
                        plan.phases
                            .iter()
                            .map(|step| step.phase)
                            .collect::<Vec<_>>()
                    };
                    if preview.plan.source_sha256 == plan.source_sha256
                        && phases(&preview.plan) == phases(&plan)
                    {
                        if matches!(outcome, FollowOutcome::Unchanged) {
                            outcome = FollowOutcome::Current { job_id: job.id };
                        }
                        break;
                    }
                    let context = follower.context(true)?;
                    let accepted = self.accept_attunement_replan(
                        &follower.session,
                        &follower.token,
                        &preview,
                        &preview.proposal_sha256,
                        &context,
                        follower.now()?,
                    )?;
                    outcome = follower.drive(accepted.successor.job)?;
                }
                state => {
                    outcome = FollowOutcome::Waiting {
                        job_id: job.id,
                        state,
                    };
                    break;
                }
            }
        }
        // Only a settled pass remembers the fingerprint; after waiting on a
        // job, or one that did not succeed, the next pass looks again even if
        // no file changed, so a job a failed pass left behind is recovered.
        let settled = matches!(
            outcome,
            FollowOutcome::Current { .. }
                | FollowOutcome::Attuned {
                    state: AttunementJobState::Succeeded,
                    ..
                }
        );
        Ok(FollowTick {
            fingerprint: settled.then_some(fingerprint),
            outcome,
        })
    }
}
