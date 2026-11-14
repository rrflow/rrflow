mod replan;

use super::*;

use crate::engine::clock::FixedClock;

use crate::{AttunementCommand, AttunementControlResult, InstallationPreview};

use rrd_contract::{
    AttunementJob, AttunementJobState as State, AttunementPhase as Phase, AttunementStatus,
    CancelAttunement, InstallationTargetKind, ResumeAttunement, SessionLease,
};

use rrd_core::{RuntimeMutation, RuntimeValue, ScopeId};

use std::path::{Path, PathBuf};

use std::sync::{Arc, Barrier};

// Fixture times start from BASE, but a committed transition records its supplied
// time plus real elapsed work. Every time after one follows the completion it
// returned, never a fixed later offset that assumes how long the call took.
pub(super) const BASE: u64 = 1_800_000_000_000;

mod adaptive_selection;

mod derivation;

mod discovery;

mod inventory_delta;

pub(super) use inventory_delta::first_inventory_for_worker_with_setup_at;

mod observation_delivery;

// Parse needs a native supervisor for its worker (Windows and Linux).
#[cfg(any(windows, target_os = "linux"))]
mod parse_step;

mod startup_binding;

pub(super) struct Installed {
    pub(super) root: tempfile::TempDir,
    executable: PathBuf,
    pub(super) preview: InstallationPreview,
    job: CanonicalId,
}

impl Installed {
    fn new(mode: InstallationTargetKind) -> (Self, RrdEngine, SessionLease) {
        Self::with_proposal(mode, None)
    }
    fn with_proposal(
        mode: InstallationTargetKind,
        proposal: Option<&crate::AttunementProposal>,
    ) -> (Self, RrdEngine, SessionLease) {
        Self::with_source(mode, proposal, |_| {})
    }
    fn with_source(
        mode: InstallationTargetKind,
        proposal: Option<&crate::AttunementProposal>,
        setup: impl FnOnce(&Path),
    ) -> (Self, RrdEngine, SessionLease) {
        Self::with_source_at(mode, proposal, setup, BASE)
    }
    pub(super) fn with_source_at(
        mode: InstallationTargetKind,
        proposal: Option<&crate::AttunementProposal>,
        setup: impl FnOnce(&Path),
        base: u64,
    ) -> (Self, RrdEngine, SessionLease) {
        Self::with_configuration_at(mode, proposal, setup, None, base)
    }
    fn with_configuration(
        mode: InstallationTargetKind,
        proposal: Option<&crate::AttunementProposal>,
        setup: impl FnOnce(&Path),
        configuration: Option<&Path>,
    ) -> (Self, RrdEngine, SessionLease) {
        Self::with_configuration_at(mode, proposal, setup, configuration, BASE)
    }
    fn with_configuration_at(
        mode: InstallationTargetKind,
        proposal: Option<&crate::AttunementProposal>,
        setup: impl FnOnce(&Path),
        configuration: Option<&Path>,
        base: u64,
    ) -> (Self, RrdEngine, SessionLease) {
        let root = tempfile::tempdir().unwrap();
        setup(root.path());
        if mode == InstallationTargetKind::ExistingProject {
            std::fs::write(root.path().join("README.md"), "real project source\n").unwrap();
        }
        let executable = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
        let preview = RrdEngine::plan_installation_with_proposal(
            root.path(),
            mode,
            &RrdEngine::generate_installation_identity().unwrap(),
            "default",
            configuration,
            proposal,
            &executable,
        )
        .unwrap();
        RrdEngine::apply_installation_at(
            root.path(),
            mode,
            &preview,
            &preview.installation.plan_sha256,
            base,
            &executable,
        )
        .unwrap();
        let engine = RrdEngine::open_installed_with_clock(
            root.path(),
            &executable,
            &FixedClock::at(base + 1),
        )
        .unwrap();
        let job =
            CanonicalId::new(format!("job-{}", &preview.attunement.plan_sha256[..24])).unwrap();
        let installed = Self {
            root,
            executable,
            preview,
            job,
        };
        let session = installed.session(&engine, "first-session", base + 100);
        (installed, engine, session)
    }
    fn session(&self, engine: &RrdEngine, key: &str, at: u64) -> SessionLease {
        let credential = RrdEngine::read_installed_api_key(self.root.path()).unwrap();
        engine
            .create_authenticated_session(
                &credential.principal_id,
                credential.credential().as_bytes(),
                &CreateSession {
                    // Long enough for a debug build to step a large Parse,
                    // whose test clock follows the job's own transition times.
                    limits: SessionLimits {
                        idle_timeout_ms: 900_000,
                        absolute_timeout_ms: 900_000,
                        max_open_transactions: 2,
                    },
                },
                &id(key),
                at,
                &format!("request-{key}"),
                &format!("operation-{key}"),
            )
            .unwrap()
    }
    pub(super) fn reopen(&self, at: u64) -> RrdEngine {
        RrdEngine::open_installed_with_clock(
            self.root.path(),
            &self.executable,
            &FixedClock::at(at),
        )
        .unwrap()
    }
    fn start(&self, lease_ms: u64) -> AttunementCommand {
        AttunementCommand::Start {
            job_id: self.job.clone(),
            expected_revision: 1,
            plan_sha256: self.preview.attunement.plan_sha256.clone(),
            lease_ms,
        }
    }
    fn read(&self, engine: &RrdEngine, session: &SessionLease, at: u64) -> AttunementStatus {
        engine
            .read_attunement(
                &session.session_id,
                &session.token,
                &self.job,
                at,
                "request-read-job",
                "operation-read-job",
            )
            .unwrap()
    }
}

pub(super) fn context(key: &str) -> RequestContext {
    mutation_context(
        &id(key),
        &format!("request-{key}"),
        &format!("operation-{key}"),
    )
}

pub(super) fn call(
    engine: &RrdEngine,
    session: &SessionLease,
    command: &AttunementCommand,
    key: &str,
    at: u64,
) -> crate::Result<AttunementControlResult> {
    engine.control_attunement(
        &session.session_id,
        &session.token,
        command,
        &context(key),
        at,
    )
}

pub(super) fn step(job: &AttunementJob) -> AttunementCommand {
    AttunementCommand::Step {
        job_id: job.id.clone(),
        expected_revision: job.revision,
        lease_generation: job.lease.as_ref().map_or(0, |lease| lease.generation),
    }
}

fn resume(job: &AttunementJob, at: u64) -> AttunementCommand {
    AttunementCommand::Resume {
        request: ResumeAttunement {
            contract_version: rrd_contract::ATTUNEMENT_CONTRACT_VERSION,
            job_id: job.id.clone(),
            plan_sha256: job.plan_sha256.clone(),
            expected_job_revision: job.revision,
            last_checkpoint_sha256: job
                .checkpoints
                .last()
                .map(|checkpoint| checkpoint.checkpoint_sha256.clone()),
            resume_from_phase: job.current_phase.unwrap(),
            requested_at_unix_ms: at,
        },
        lease_ms: 30_000,
    }
}

fn cancel(job: &AttunementJob, at: u64) -> AttunementCommand {
    AttunementCommand::Cancel {
        request: CancelAttunement {
            contract_version: rrd_contract::ATTUNEMENT_CONTRACT_VERSION,
            job_id: job.id.clone(),
            plan_sha256: job.plan_sha256.clone(),
            expected_job_revision: job.revision,
            last_checkpoint_sha256: job
                .checkpoints
                .last()
                .map(|checkpoint| checkpoint.checkpoint_sha256.clone()),
            reason: "user stopped indexing".into(),
            requested_at_unix_ms: at,
        },
    }
}

#[test]
fn publicly_selected_plan_replans_after_progress_and_reopens() {
    selected_plan_replans_after_progress_and_reopens(false);
}

mod control;
mod crash;
mod inventory;
mod parse_policy;
mod replan_receipts;

use replan_receipts::*;
