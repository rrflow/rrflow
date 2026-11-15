//! Durable, read-only Inventory through the existing selected plan and store.
use super::*;
use crate::engine::transaction_admission::TransactionPermit;
use rrd_attunement::inventory::{Proposal, SourceScope};
mod publication;
mod records;
mod reuse;

/// Resolve only the committed Inventory immediately preceding a Parse step.
/// The caller must authorize the session and fence the loaded job and lease.
#[allow(dead_code)] // Consumed by the installed Parse executor after worker admission.
pub(in crate::engine) fn load_parse_inventory(
    engine: &RrdEngine,
    job: &AttunementJob,
    plan: &AttunementPlan,
    remaining_keys: &mut u64,
) -> Result<Proposal> {
    job.validate(plan).map_err(contract)?;
    if job.state != AttunementJobState::Running || job.current_phase != Some(AttunementPhase::Parse)
    {
        return Err(ServiceError::ProjectBindingMismatch);
    }
    let checkpoint = job
        .checkpoints
        .last()
        .filter(|checkpoint| checkpoint.phase == AttunementPhase::Inventory)
        .ok_or(ServiceError::ProjectBindingMismatch)?;
    let installed = engine.attunement_installed_record()?;
    let (profile, digest) =
        DefaultInstallProfile::load(installed.installation.profile_id.as_str())?;
    if digest != installed.installation.profile_sha256 {
        return Err(ServiceError::ProjectBindingMismatch);
    }
    records::load_for_parse(
        engine,
        plan,
        checkpoint,
        profile.inventory_policy.limits.output_bytes.min(
            engine
                .engram_configuration
                .storage
                .transaction_admission_bytes,
        ),
        profile.inventory_policy.limits.entries,
        remaining_keys,
    )
}

#[cfg(test)]
pub(crate) use records::LegacySourceFileProjectionGuard;

pub(in crate::engine) struct AcceptedInventory {
    pub head_bytes: Vec<u8>,
    pub project_id: CanonicalId,
    pub proposal_sha256: String,
    pub read: ReadStamp,
    pub proposal: Proposal,
}

pub(in crate::engine) fn load_accepted_source(
    engine: &RrdEngine,
    project_id: &CanonicalId,
    max_bytes: u64,
    max_entries: u64,
) -> Result<AcceptedInventory> {
    let mut remaining_keys = engine.engram_configuration.query.max_storage_keys;
    let (head_bytes, head, proposal) = records::load_accepted(
        engine,
        project_id,
        max_bytes,
        max_entries,
        &mut remaining_keys,
    )?
    .ok_or(ServiceError::ProjectBindingMismatch)?;
    Ok(AcceptedInventory {
        head_bytes,
        project_id: head.project_id,
        proposal_sha256: head.proposal_sha256,
        read: head.read,
        proposal,
    })
}

pub(in crate::engine) fn source_head_key(instance: &CanonicalId) -> String {
    records::head_key(instance)
}

pub(super) struct Work<'a> {
    pub session: &'a CorrelationId,
    pub token: &'a CorrelationId,
    pub context: &'a RequestContext,
    pub now: u64,
    pub started: Instant,
    pub principal: CanonicalId,
    pub operation_sha256: String,
    pub before_bytes: Vec<u8>,
    pub before: AttunementJob,
    pub plan: AttunementPlan,
    pub receipt_key: String,
    pub lease_generation: u64,
}

pub(super) fn execute(
    engine: &RrdEngine,
    work: Work<'_>,
    mut permit: TransactionPermit<'_>,
) -> Result<AttunementControlResult> {
    let holder = worker(work.session)?;
    let invocation = engine.invocation_admission(
        work.context.request_id.as_str(),
        work.context.operation_id.as_str(),
    )?;
    let check = || {
        if let Some(invocation) = &invocation {
            invocation.check()?;
        }
        let at = elapsed_time(work.now, work.started)?;
        check_deadline(work.context, at)?;
        require_worker(&work.before, &holder, work.lease_generation, at)
    };
    check()?;
    let installed = engine.attunement_installed_record()?;
    let (profile, profile_sha256) =
        DefaultInstallProfile::load(installed.installation.profile_id.as_str())?;
    if profile_sha256 != installed.installation.profile_sha256 {
        return Err(ServiceError::ProjectBindingMismatch);
    }
    let root = engine
        .installed_project_root
        .as_ref()
        .ok_or(ServiceError::ProjectBindingMismatch)?;
    let policy = &profile.inventory_policy;
    let output_limit = policy.limits.output_bytes.min(
        engine
            .engram_configuration
            .storage
            .transaction_admission_bytes,
    );
    let mut remaining_keys = engine.engram_configuration.query.max_storage_keys;
    let (prior_bytes, prior) = records::load_prior(
        engine,
        &work.plan,
        output_limit,
        policy.limits.entries,
        &mut remaining_keys,
    )?;
    let source_scope = SourceScope {
        engram: work.plan.target.engram_id.as_str().into(),
        project: work.plan.target.project_id.as_str().into(),
        root: policy.logical_root.as_str().into(),
    };
    let (precondition, proposal) = super::super::installation::prepare_project_inventory(
        root,
        &profile,
        source_scope,
        prior.as_ref(),
        &check,
    )?;
    if precondition != work.plan.source_sha256 {
        return Err(ServiceError::AttunementSourceChanged);
    }
    check()?;
    let scope = ScopeId::new(format!("instance:{}", engine.instance)).map_err(contract)?;
    let read = engine.storage.runtime().read_stamp(&scope)?;
    let at = elapsed_time(work.now, work.started)?;
    let mutations = records::mutations(engine, &scope, &read, &proposal, &precondition, at)?;
    let mut commit = RuntimeCommit {
        scope,
        at,
        actor: format!("session:{}", work.session.as_str()),
        expected_cursor: read.commit_cursor,
        mutations,
    };
    bounded(&commit, output_limit)?;
    permit.charge(
        &commit,
        engine
            .engram_configuration
            .storage
            .transaction_admission_bytes,
    )?;
    let recheck_source = || {
        if project_inventory_sha256(root, &profile)? != work.plan.source_sha256 {
            return Err(ServiceError::AttunementSourceChanged);
        }
        Ok(())
    };
    let (mutations, publication_sha256) = reuse::filter(
        engine,
        &read,
        std::mem::take(&mut commit.mutations),
        &mut remaining_keys,
        &check,
    )?;
    commit.mutations = mutations;
    publication::Publication {
        engine,
        work: &work,
        proposal: &proposal,
        prior_bytes: &prior_bytes,
        check: &check,
        recheck_source: &recheck_source,
        output_limit,
    }
    .publish(commit, permit, &publication_sha256)
}

fn bounded(value: &impl Serialize, limit: u64) -> Result<()> {
    struct Counter {
        bytes: u64,
        limit: u64,
    }
    impl std::io::Write for Counter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.bytes = self
                .bytes
                .checked_add(bytes.len() as u64)
                .filter(|b| *b <= self.limit)
                .ok_or_else(|| std::io::Error::other("Inventory output byte bound"))?;
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    serde_json::to_writer(&mut Counter { bytes: 0, limit }, value).map_err(|_| {
        ServiceError::ConfigurationLimit("Inventory output byte bound exceeded".into())
    })
}
