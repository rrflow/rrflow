//! Real installed rrflowKV + HTTP proof of the runnable local database example.
#[path = "../examples/support/rrflowdb.rs"]
mod database;

use database::{options, read_options};
use rrd_client::{ClientConfig, RrdClient, Session};
use rrd_contract::{
    CloseSession, CreateSession, DataSchemaSnapshot, InstallationTargetKind, QueryValue,
    ReadDataSchema, SessionLimits, TransactionMutation,
};
use rrd_engine::RrdEngine;
use rrd_server::RrdHttpServer;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::oneshot;

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

fn executable() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml")
}

async fn start(
    project: &Path,
) -> (
    RrdClient,
    oneshot::Sender<()>,
    tokio::task::JoinHandle<std::result::Result<(), rrd_server::HttpError>>,
) {
    let engine = RrdEngine::open_installed(project, &executable()).unwrap();
    let instance = engine.instance_id().clone();
    let server = RrdHttpServer::bind(engine, "127.0.0.1:0".parse().unwrap()).unwrap();
    let address = server.local_addr();
    let (shutdown, stopped) = oneshot::channel();
    let task = tokio::spawn(server.serve_until(async {
        let _ = stopped.await;
    }));
    (
        RrdClient::connect_local(address, instance, ClientConfig::default()).unwrap(),
        shutdown,
        task,
    )
}

async fn login(client: &RrdClient, project: &Path, key: &str) -> Session {
    let credential = RrdEngine::read_installed_api_key(project).unwrap();
    client
        .create_session(
            credential.principal_id.clone(),
            credential.credential(),
            CreateSession {
                limits: SessionLimits {
                    idle_timeout_ms: 60_000,
                    absolute_timeout_ms: 300_000,
                    max_open_transactions: 4,
                },
            },
            options(key).unwrap(),
        )
        .await
        .unwrap()
}

async fn schema(client: &RrdClient, session: &Session, key: &str) -> DataSchemaSnapshot {
    client
        .read_data_schema(
            session,
            ReadDataSchema {
                scope: format!("instance:{}", client.instance_id()),
                max_storage_keys: 4096,
            },
            read_options(key).unwrap(),
        )
        .await
        .unwrap()
}

#[tokio::test]
async fn installed_typed_attunement_controls_replan_and_reopen() {
    use rrd_contract::{
        AcceptAttunementReplan, AttunementJobState, PauseAttunement, PreviewAttunementReplan,
        ReadAttunement, ReadAttunementReceipt, RecoverExpiredAttunement, ResumeAttunement,
        ResumeAttunementWithLease, StartAttunement, StepAttunement,
        ATTUNEMENT_CONTRACT_VERSION as VERSION,
    };
    let temporary = tempfile::tempdir().unwrap();
    let project = temporary.path().join("project");
    std::fs::create_dir(&project).unwrap();
    std::fs::write(project.join("source.txt"), "initial").unwrap();
    let preview = RrdEngine::plan_installation(
        &project,
        InstallationTargetKind::ExistingProject,
        &RrdEngine::generate_installation_identity().unwrap(),
        "default",
        None,
        &executable(),
    )
    .unwrap();
    RrdEngine::apply_installation(
        &project,
        InstallationTargetKind::ExistingProject,
        &preview,
        &preview.installation.plan_sha256,
        &executable(),
    )
    .unwrap();
    let plan = preview.attunement;
    let (client, shutdown, server) = start(&project).await;
    let session = login(&client, &project, "typed-session").await;
    let discovery = rrd_contract::ListAttunementJobs {
        contract_version: VERSION,
        after_job_id: None,
        limit: 1,
    };
    let initial = client
        .list_attunement_jobs(
            &session,
            discovery.clone(),
            read_options("typed-discover").unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(initial.jobs.len(), 1);
    assert_eq!(initial.jobs[0].plan_sha256, plan.plan_sha256);
    assert_eq!(initial.next_after_job_id, None);
    let job_id = initial.jobs[0].job_id.clone();
    let read = ReadAttunement {
        contract_version: VERSION,
        job_id: job_id.clone(),
    };
    let start_request = StartAttunement {
        contract_version: VERSION,
        job_id: job_id.clone(),
        plan_sha256: plan.plan_sha256.clone(),
        expected_revision: 1,
        lease_ms: 60_000,
    };
    let mut foreign_plan = plan.clone();
    foreign_plan.target.instance_id = rrd_contract::CanonicalId::new("foreign-instance").unwrap();
    foreign_plan.plan_sha256 = rrd_contract::attunement_plan_sha256(&foreign_plan).unwrap();
    let foreign_request = StartAttunement {
        plan_sha256: foreign_plan.plan_sha256.clone(),
        ..start_request.clone()
    };
    assert!(matches!(
        client
            .start_attunement(
                &session,
                foreign_request,
                &foreign_plan,
                options("typed-foreign").unwrap()
            )
            .await,
        Err(rrd_client::Error::ResponseIdentityMismatch)
    ));
    assert_eq!(
        client
            .read_attunement(
                &session,
                read.clone(),
                read_options("typed-before").unwrap()
            )
            .await
            .unwrap()
            .status
            .job
            .revision,
        1
    );
    let started = client
        .start_attunement(
            &session,
            start_request.clone(),
            &plan,
            options("typed-start").unwrap(),
        )
        .await
        .unwrap();
    assert!(started.validate_for(&job_id, 1, &plan).is_ok());
    assert!(started.validate_for(&job_id, 2, &plan).is_err());
    assert!(started
        .validate_for(
            &rrd_contract::CanonicalId::new("job-foreign").unwrap(),
            1,
            &plan
        )
        .is_err());
    let replay = client
        .start_attunement(
            &session,
            start_request,
            &plan,
            options("typed-start").unwrap(),
        )
        .await
        .unwrap();
    assert!(replay.idempotent_replay);
    assert_eq!(started.status.job, replay.status.job);
    let paused = client
        .pause_attunement(
            &session,
            PauseAttunement {
                contract_version: VERSION,
                job_id: job_id.clone(),
                expected_revision: started.status.job.revision,
                lease_generation: started.status.job.lease.as_ref().unwrap().generation,
            },
            &plan,
            options("typed-pause").unwrap(),
        )
        .await
        .unwrap();
    let resume = |status: &rrd_contract::AttunementStatus, lease_ms| ResumeAttunementWithLease {
        request: ResumeAttunement {
            contract_version: VERSION,
            job_id: job_id.clone(),
            plan_sha256: plan.plan_sha256.clone(),
            expected_job_revision: status.job.revision,
            last_checkpoint_sha256: status
                .job
                .checkpoints
                .last()
                .map(|c| c.checkpoint_sha256.clone()),
            resume_from_phase: status.job.next_phase(&plan).unwrap(),
            requested_at_unix_ms: now(),
        },
        lease_ms,
    };
    let first_resume = resume(&paused.status, 1_000);
    let resumed = client
        .resume_attunement(
            &session,
            first_resume.clone(),
            &plan,
            options("typed-resume").unwrap(),
        )
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(1_200)).await;
    let recovered = client
        .recover_expired_attunement(
            &session,
            RecoverExpiredAttunement {
                contract_version: VERSION,
                job_id: job_id.clone(),
                expected_revision: resumed.status.job.revision,
            },
            &plan,
            options("typed-recover").unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(recovered.status.job.state, AttunementJobState::Paused);
    let resumed = client
        .resume_attunement(
            &session,
            resume(&recovered.status, 60_000),
            &plan,
            options("typed-resume-again").unwrap(),
        )
        .await
        .unwrap();
    let stepped = client
        .step_attunement(
            &session,
            StepAttunement {
                contract_version: VERSION,
                job_id: job_id.clone(),
                expected_revision: resumed.status.job.revision,
                lease_generation: resumed.status.job.lease.as_ref().unwrap().generation,
            },
            &plan,
            options("typed-step").unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(stepped.status.job.checkpoints.len(), 1);
    std::fs::write(project.join("source.txt"), "edit one").unwrap();
    let stale = client
        .step_attunement(
            &session,
            StepAttunement {
                contract_version: VERSION,
                job_id: job_id.clone(),
                expected_revision: stepped.status.job.revision,
                lease_generation: stepped.status.job.lease.as_ref().unwrap().generation,
            },
            &plan,
            options("typed-stale-step").unwrap(),
        )
        .await
        .unwrap_err();
    assert!(matches!(stale, rrd_client::Error::Api { .. }));
    let request = PreviewAttunementReplan {
        contract_version: VERSION,
        job_id: job_id.clone(),
        expected_revision: stepped.status.job.revision,
    };
    let first = client
        .preview_attunement_replan(
            &session,
            request.clone(),
            &plan,
            read_options("typed-preview-one").unwrap(),
        )
        .await
        .unwrap();
    let mut corrupt = first.clone();
    corrupt.proposal_sha256 = "0".repeat(64);
    assert!(corrupt
        .validate_for(&job_id, request.expected_revision, &plan)
        .is_err());
    std::fs::write(project.join("source.txt"), "edit two").unwrap();
    let stale_accept = AcceptAttunementReplan {
        contract_version: VERSION,
        proposal_sha256: first.proposal_sha256.clone(),
        preview: first,
    };
    assert!(client
        .accept_attunement_replan(
            &session,
            stale_accept,
            &plan,
            options("typed-stale-accept").unwrap()
        )
        .await
        .is_err());
    let proposal = client
        .preview_attunement_replan(
            &session,
            request,
            &plan,
            read_options("typed-preview-two").unwrap(),
        )
        .await
        .unwrap();
    let accept = AcceptAttunementReplan {
        contract_version: VERSION,
        proposal_sha256: proposal.proposal_sha256.clone(),
        preview: proposal,
    };
    let accepted = client
        .accept_attunement_replan(
            &session,
            accept.clone(),
            &plan,
            options("typed-accept").unwrap(),
        )
        .await
        .unwrap();
    assert!(!accepted.idempotent_replay);
    let predecessor_page = client
        .list_attunement_jobs(
            &session,
            discovery.clone(),
            read_options("typed-discover-predecessor").unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(predecessor_page.next_after_job_id, Some(job_id.clone()));
    let continuation = rrd_contract::ListAttunementJobs {
        after_job_id: predecessor_page.next_after_job_id,
        ..discovery
    };
    let successor_page = client
        .list_attunement_jobs(
            &session,
            continuation.clone(),
            read_options("typed-discover-successor").unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(successor_page.jobs[0].job_id, accepted.successor.job.id);
    assert_eq!(successor_page.jobs[0].generation, 1);
    assert_eq!(successor_page.next_after_job_id, None);
    let receipt_request = ReadAttunementReceipt {
        contract_version: VERSION,
        job_id: job_id.clone(),
        idempotency_key: rrd_contract::CorrelationId::new("typed-accept").unwrap(),
    };
    let receipt = client
        .read_attunement_replan_receipt(
            &session,
            receipt_request.clone(),
            read_options("typed-receipt").unwrap(),
        )
        .await
        .unwrap();
    let mut foreign = receipt.clone();
    foreign.receipt.as_mut().unwrap().principal_id =
        rrd_contract::CanonicalId::new("foreign").unwrap();
    assert!(foreign
        .validate_for(
            &receipt_request,
            session.principal_id(),
            client.instance_id()
        )
        .is_err());
    let mut wrong = accepted.clone();
    wrong.successor.job.id = rrd_contract::CanonicalId::new("job-foreign").unwrap();
    assert!(wrong.validate_for(&accept.preview, &plan).is_err());
    shutdown.send(()).unwrap();
    server.await.unwrap().unwrap();
    let (reopened, shutdown, server) = start(&project).await;
    let session = login(&reopened, &project, "typed-reopen-session").await;
    let recovered_page = reopened
        .list_attunement_jobs(
            &session,
            continuation,
            read_options("typed-discover-reopen").unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(recovered_page.jobs, successor_page.jobs);
    assert_eq!(
        recovered_page.source_verification,
        rrd_contract::AttunementSourceVerification::NotChecked
    );
    let resume_replay = reopened
        .resume_attunement(
            &session,
            first_resume,
            &plan,
            options("typed-resume").unwrap(),
        )
        .await
        .unwrap();
    assert!(resume_replay.idempotent_replay);
    let replay = reopened
        .accept_attunement_replan(&session, accept, &plan, options("typed-accept").unwrap())
        .await
        .unwrap();
    assert!(replay.idempotent_replay);
    assert_eq!(replay.successor.job, accepted.successor.job);
    let receipt = reopened
        .read_attunement_replan_receipt(
            &session,
            receipt_request,
            read_options("typed-receipt-reopen").unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        receipt.receipt.unwrap().result.successor.job,
        accepted.successor.job
    );
    let audit = reopened
        .read_audit(
            &session,
            rrd_contract::ReadAudit {
                after_sequence: 0,
                limit: 1_024,
            },
            read_options("typed-audit-reopen").unwrap(),
        )
        .await
        .unwrap();
    assert!(
        audit.records.iter().any(|record| {
            record.request_id == "request-typed-accept"
                && record.operation_id == "operation-typed-accept"
                && record.principal_id.as_ref() == Some(session.principal_id())
                && record.action == rrd_contract::SecurityAction::EngramAdmin
                && record.decision == rrd_contract::AuditDecision::Allowed
        }),
        "the accepted replan authorization must remain auditable after reopen"
    );
    assert_eq!(
        reopened
            .read_attunement(&session, read, read_options("typed-old-job").unwrap())
            .await
            .unwrap()
            .status
            .job
            .state,
        AttunementJobState::CancelRequested
    );
    shutdown.send(()).unwrap();
    server.await.unwrap().unwrap();
}

#[tokio::test]
async fn installed_attunement_read_reports_stored_plan_and_unchecked_source_after_reopen() {
    let temporary = tempfile::tempdir().unwrap();
    let project = temporary.path().join("project");
    std::fs::create_dir(&project).unwrap();
    let preview = RrdEngine::plan_installation(
        &project,
        InstallationTargetKind::FreshProject,
        &RrdEngine::generate_installation_identity().unwrap(),
        "default",
        None,
        &executable(),
    )
    .unwrap();
    RrdEngine::apply_installation(
        &project,
        InstallationTargetKind::FreshProject,
        &preview,
        &preview.installation.plan_sha256,
        &executable(),
    )
    .unwrap();
    let request = rrd_contract::ReadAttunement {
        contract_version: rrd_contract::ATTUNEMENT_CONTRACT_VERSION,
        job_id: rrd_contract::CanonicalId::new(format!(
            "job-{}",
            &preview.attunement.plan_sha256[..24]
        ))
        .unwrap(),
    };
    let (client, shutdown, server) = start(&project).await;
    let session = login(&client, &project, "attunement-session").await;
    let first = client
        .read_attunement(
            &session,
            request.clone(),
            read_options("attunement-read").unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(first.plan, preview.attunement);
    assert_eq!(
        first.status.job.state,
        rrd_contract::AttunementJobState::Pending
    );
    assert_eq!(
        first.source_verification,
        rrd_contract::AttunementSourceVerification::NotChecked
    );
    std::fs::write(project.join("new-source.txt"), "source changed").unwrap();
    let changed = client
        .read_attunement(
            &session,
            request.clone(),
            read_options("attunement-changed").unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(changed.plan, first.plan);
    assert_eq!(changed.status.job, first.status.job);
    assert_eq!(
        changed.source_verification,
        rrd_contract::AttunementSourceVerification::NotChecked
    );
    let missing = rrd_contract::ReadAttunement {
        job_id: rrd_contract::CanonicalId::new("job-missing").unwrap(),
        ..request.clone()
    };
    assert!(client
        .read_attunement(
            &session,
            missing,
            read_options("attunement-missing").unwrap()
        )
        .await
        .is_err());
    shutdown.send(()).unwrap();
    server.await.unwrap().unwrap();
    let (reopened, shutdown, server) = start(&project).await;
    let session = login(&reopened, &project, "attunement-reopened-session").await;
    let after = reopened
        .read_attunement(
            &session,
            request,
            read_options("attunement-reopened").unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(after.plan, first.plan);
    assert_eq!(after.status.job, first.status.job);
    assert_eq!(
        after.source_verification,
        rrd_contract::AttunementSourceVerification::NotChecked
    );
    shutdown.send(()).unwrap();
    server.await.unwrap().unwrap();
}

#[tokio::test]
async fn installed_attunement_cancel_survives_restart_and_replays() {
    let temporary = tempfile::tempdir().unwrap();
    let project = temporary.path().join("project");
    std::fs::create_dir(&project).unwrap();
    let preview = RrdEngine::plan_installation(
        &project,
        InstallationTargetKind::FreshProject,
        &RrdEngine::generate_installation_identity().unwrap(),
        "default",
        None,
        &executable(),
    )
    .unwrap();
    RrdEngine::apply_installation(
        &project,
        InstallationTargetKind::FreshProject,
        &preview,
        &preview.installation.plan_sha256,
        &executable(),
    )
    .unwrap();
    let read = rrd_contract::ReadAttunement {
        contract_version: rrd_contract::ATTUNEMENT_CONTRACT_VERSION,
        job_id: rrd_contract::CanonicalId::new(format!(
            "job-{}",
            &preview.attunement.plan_sha256[..24]
        ))
        .unwrap(),
    };
    let (client, shutdown, server) = start(&project).await;
    let session = login(&client, &project, "cancel-session").await;
    let before = client
        .read_attunement(
            &session,
            read.clone(),
            read_options("cancel-before").unwrap(),
        )
        .await
        .unwrap();
    let receipt_request = rrd_contract::ReadAttunementReceipt {
        contract_version: rrd_contract::ATTUNEMENT_CONTRACT_VERSION,
        job_id: read.job_id.clone(),
        idempotency_key: rrd_contract::CorrelationId::new("cancel-once").unwrap(),
    };
    assert!(client
        .read_attunement_receipt(
            &session,
            receipt_request.clone(),
            read_options("receipt-before").unwrap()
        )
        .await
        .unwrap()
        .receipt
        .is_none());
    let cancel = rrd_contract::CancelAttunement {
        contract_version: rrd_contract::ATTUNEMENT_CONTRACT_VERSION,
        job_id: read.job_id.clone(),
        plan_sha256: before.plan.plan_sha256.clone(),
        expected_job_revision: before.status.job.revision,
        last_checkpoint_sha256: None,
        reason: "operator requested cancellation".into(),
        requested_at_unix_ms: now(),
    };
    let closed = login(&client, &project, "cancel-closed-session").await;
    client
        .close_session(&closed, CloseSession {}, options("cancel-close").unwrap())
        .await
        .unwrap();
    assert!(client
        .cancel_attunement(&closed, cancel.clone(), options("cancel-denied").unwrap())
        .await
        .is_err());
    let after_denial = client
        .read_attunement(
            &session,
            read.clone(),
            read_options("cancel-after-denial").unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(after_denial.status.job, before.status.job);
    let mut wrong = cancel.clone();
    wrong.plan_sha256 = "0".repeat(64);
    assert!(client
        .cancel_attunement(&session, wrong, options("wrong-plan").unwrap())
        .await
        .is_err());
    let accepted = client
        .cancel_attunement(&session, cancel.clone(), options("cancel-once").unwrap())
        .await
        .unwrap();
    assert!(!accepted.idempotent_replay);
    assert_eq!(
        accepted.status.job.state,
        rrd_contract::AttunementJobState::CancelRequested
    );
    assert_eq!(accepted.status.job.revision, before.status.job.revision + 1);
    let receipt = client
        .read_attunement_receipt(
            &session,
            receipt_request.clone(),
            read_options("receipt-after").unwrap(),
        )
        .await
        .unwrap()
        .receipt
        .unwrap();
    assert_eq!(receipt.status, accepted.status);
    assert_eq!(receipt.plan, before.plan);
    assert_eq!(receipt.principal_id, *session.principal_id());
    assert_eq!(receipt.operation_sha256.len(), 64);
    assert_eq!(
        receipt.source_verification,
        rrd_contract::AttunementSourceVerification::NotChecked
    );
    let result = rrd_contract::AttunementReceiptReadResult {
        receipt: Some(receipt.clone()),
    };
    result
        .validate_for(
            &receipt_request,
            session.principal_id(),
            client.instance_id(),
        )
        .unwrap();
    let mut forged = result.clone();
    forged.receipt.as_mut().unwrap().operation_sha256 = "not-a-digest".into();
    assert!(forged
        .validate_for(
            &receipt_request,
            session.principal_id(),
            client.instance_id()
        )
        .is_err());
    let mut wrong_job = receipt_request.clone();
    wrong_job.job_id = rrd_contract::CanonicalId::new("job-other").unwrap();
    assert!(result
        .validate_for(&wrong_job, session.principal_id(), client.instance_id())
        .is_err());
    let other = rrd_contract::CanonicalId::new("other-principal").unwrap();
    assert!(result
        .validate_for(&receipt_request, &other, client.instance_id())
        .is_err());
    let other_instance = rrd_contract::CanonicalId::new("other-instance").unwrap();
    assert!(result
        .validate_for(&receipt_request, session.principal_id(), &other_instance)
        .is_err());
    let mut missing = receipt_request.clone();
    missing.idempotency_key = rrd_contract::CorrelationId::new("different-key").unwrap();
    assert!(client
        .read_attunement_receipt(&session, missing, read_options("receipt-missing").unwrap())
        .await
        .unwrap()
        .receipt
        .is_none());
    assert!(client
        .read_attunement_receipt(
            &closed,
            receipt_request.clone(),
            read_options("receipt-denied").unwrap()
        )
        .await
        .is_err());
    let mut changed = cancel.clone();
    changed.reason = "different intent".into();
    assert!(client
        .cancel_attunement(&session, changed, options("cancel-once").unwrap())
        .await
        .is_err());
    assert!(client
        .cancel_attunement(&session, cancel.clone(), options("stale-revision").unwrap())
        .await
        .is_err());
    shutdown.send(()).unwrap();
    server.await.unwrap().unwrap();
    let (client, shutdown, server) = start(&project).await;
    let session = login(&client, &project, "cancel-reopened-session").await;
    let after_receipt = client
        .read_attunement_receipt(
            &session,
            receipt_request,
            read_options("receipt-reopened").unwrap(),
        )
        .await
        .unwrap()
        .receipt
        .unwrap();
    assert_eq!(after_receipt, receipt);
    let after = client
        .read_attunement(&session, read, read_options("cancel-after").unwrap())
        .await
        .unwrap();
    assert_eq!(after.status.job, accepted.status.job);
    let replay = client
        .cancel_attunement(&session, cancel, options("cancel-once").unwrap())
        .await
        .unwrap();
    assert!(replay.idempotent_replay);
    assert_eq!(replay.status, accepted.status);
    shutdown.send(()).unwrap();
    server.await.unwrap().unwrap();
}

#[tokio::test]
async fn parquet_export_rejects_truncation_and_foreign_engine_sessions() {
    let temporary = tempfile::tempdir().unwrap();
    let first = temporary.path().join("first");
    let second = temporary.path().join("second");
    for project in [&first, &second] {
        std::fs::create_dir(project).unwrap();
        let plan = RrdEngine::plan_installation(
            project,
            InstallationTargetKind::FreshProject,
            &RrdEngine::generate_installation_identity().unwrap(),
            "default",
            None,
            &executable(),
        )
        .unwrap();
        RrdEngine::apply_installation(
            project,
            InstallationTargetKind::FreshProject,
            &plan,
            &plan.installation.plan_sha256,
            &executable(),
        )
        .unwrap();
    }
    let (first_client, first_stop, first_server) = start(&first).await;
    let (second_client, second_stop, second_server) = start(&second).await;
    let first_session = login(&first_client, &first, "export-first-session").await;
    let at = now();
    database::initialize(&first_client, &first_session, at, "export-init")
        .await
        .unwrap();
    let transaction = database::begin(&first_client, &first_session, "export-add-begin")
        .await
        .unwrap();
    let mut extra = database::task_record("another task", "open", at);
    if let TransactionMutation::PutRecord { reference, .. } = &mut extra {
        reference.id = database::id("second-task");
    } else {
        panic!("fixture must be a record");
    }
    database::commit(
        &first_client,
        &first_session,
        &transaction,
        vec![extra],
        "export-add",
    )
    .await
    .unwrap();
    let query = rrd_contract::ExecuteQuery {
        scope: format!("instance:{}", first_client.instance_id()),
        query: format!(
            "FROM record:{} AT VALID {at} KNOWN HEAD PROJECT id, title, status",
            database::TASK
        ),
        parameters: Default::default(),
        budget: rrd_contract::QueryBudget {
            max_rows: 1,
            ..Default::default()
        },
    };
    let partial = first_client
        .execute_query(
            &first_session,
            query.clone(),
            read_options("export-partial-query").unwrap(),
        )
        .await
        .unwrap();
    assert!(partial.execution.truncated);
    let request = rrd_contract::ExportQuery {
        query,
        max_output_bytes: 524288,
    };
    let denied = first_client
        .export_query(
            &first_session,
            request.clone(),
            read_options("export-truncated").unwrap(),
        )
        .await
        .unwrap_err();
    assert!(matches!(denied, rrd_client::Error::Api { error, .. }
        if error.code == rrd_contract::ErrorCode::ResourceExhausted));
    let mut foreign = request;
    foreign.query.scope = format!("instance:{}", second_client.instance_id());
    assert!(second_client
        .export_query(
            &first_session,
            foreign,
            read_options("export-foreign-session").unwrap()
        )
        .await
        .is_err());
    first_stop.send(()).unwrap();
    second_stop.send(()).unwrap();
    first_server.await.unwrap().unwrap();
    second_server.await.unwrap().unwrap();
}

#[tokio::test]
async fn installed_native_database_relations_atomicity_temporal_reads_and_reopen() {
    let temporary = tempfile::tempdir().unwrap();
    let project = temporary.path().join("project");
    std::fs::create_dir(&project).unwrap();
    let preview = RrdEngine::plan_installation(
        &project,
        InstallationTargetKind::FreshProject,
        &RrdEngine::generate_installation_identity().unwrap(),
        "default",
        None,
        &executable(),
    )
    .unwrap();
    RrdEngine::apply_installation(
        &project,
        InstallationTargetKind::FreshProject,
        &preview,
        &preview.installation.plan_sha256,
        &executable(),
    )
    .unwrap();
    let (client, shutdown, server) = start(&project).await;
    client.capabilities().await.unwrap();
    let session = login(&client, &project, "db-session").await;
    let initial_schema = schema(&client, &session, "db-before").await.schema.unwrap();
    let created_at = now();
    let created = database::initialize(&client, &session, created_at, "db-init")
        .await
        .unwrap();
    assert_eq!(created.mutation_count, 6);
    let installed_schema = schema(&client, &session, "db-after").await;
    for (kind, table) in &initial_schema.tables {
        assert_eq!(
            installed_schema.schema.as_ref().unwrap().tables.get(kind),
            Some(table)
        );
    }
    assert_eq!(
        installed_schema.schema.as_ref().unwrap().revision,
        initial_schema.revision + 1
    );
    installed_schema.read_evidence.validate().unwrap();
    let joined = database::relationships(
        &client,
        &session,
        created_at,
        installed_schema.known_at_cursor,
        "db-join",
    )
    .await
    .unwrap();
    assert_eq!(joined.rows.len(), 1);
    assert_eq!(
        joined.rows[0].values["right.to_id"],
        QueryValue::String("first-task".into())
    );
    let before = database::tasks(&client, &session, created_at, "db-tasks-before")
        .await
        .unwrap();
    assert_eq!(
        before.rows[0].values["status"],
        QueryValue::String("open".into())
    );
    let scope = format!("instance:{}", client.instance_id());
    let initial_view = database::read_view(&client, &session, created_at, "db-view")
        .await
        .unwrap();
    initial_view.verify(&scope, "open").unwrap();
    assert!(initial_view.verify(&scope, "done").is_err());
    assert!(initial_view.verify("instance:foreign", "open").is_err());
    for fault in 0..15 {
        let mut invalid = initial_view.clone();
        match fault {
            0 => invalid.tasks.execution.truncated = true,
            1 => invalid.relationships.known_at_cursor += 1,
            2 => invalid.relationships.schema_revision += 1,
            3 => invalid.tasks.rows.clear(),
            4 => invalid
                .relationships
                .rows
                .push(invalid.relationships.rows[0].clone()),
            5 => {
                invalid.relationships.rows[0].values.insert(
                    "right.to_id".into(),
                    QueryValue::String("another-task".into()),
                );
            }
            6 => invalid.relationships.execution.truncated = true,
            7 => invalid.tasks.execution.returned_rows += 1,
            8 => invalid.documents.known_at_cursor += 1,
            9 => {
                invalid.documents.rows[0]
                    .values
                    .insert("metadata".into(), QueryValue::Null);
            }
            10 => {
                invalid.document_relationships.rows[0]
                    .values
                    .insert("right.reason".into(), QueryValue::String("forged".into()));
            }
            11 => invalid.documents.execution.truncated = true,
            12 => invalid.document_relationships.rows.clear(),
            13 => {
                invalid.document_relationships.rows[0].values.insert(
                    "left.id".into(),
                    QueryValue::String("wrong-document".into()),
                );
            }
            _ => {
                invalid.document_relationships.rows[0].values.insert(
                    "right.to_id".into(),
                    QueryValue::String("wrong-task".into()),
                );
            }
        }
        assert!(
            invalid.verify(&scope, "open").is_err(),
            "accepted fault {fault}"
        );
    }

    // A second init must not overwrite schema or application data.
    assert!(
        database::initialize(&client, &session, created_at, "db-init-again")
            .await
            .is_err()
    );
    assert_eq!(
        schema(&client, &session, "db-after-second-init").await,
        installed_schema
    );

    // Unique violation after a valid task update must roll back both mutations.
    let transaction = database::begin(&client, &session, "db-invalid-begin")
        .await
        .unwrap();
    let rejected_at = created_at + 1;
    assert!(database::commit(
        &client,
        &session,
        &transaction,
        vec![
            database::task_record("must not survive", "done", rejected_at),
            database::document_record("must not survive", 2, rejected_at),
            database::project_record("duplicate-project", "local-ai", rejected_at),
        ],
        "db-invalid-commit"
    )
    .await
    .is_err());
    database::abort(&client, &session, &transaction, "db-invalid-abort")
        .await
        .unwrap();
    let unchanged = database::tasks(&client, &session, rejected_at, "db-after-invalid")
        .await
        .unwrap();
    assert_eq!(unchanged.rows, before.rows);
    assert_eq!(unchanged.known_at_cursor, before.known_at_cursor);
    database::read_view(&client, &session, rejected_at, "db-invalid-view")
        .await
        .unwrap()
        .verify(&scope, "open")
        .unwrap();

    // Required typed fields cannot silently accept another type.
    let mut invalid = database::task_record("bad type", "done", rejected_at);
    if let TransactionMutation::PutRecord { properties, .. } = &mut invalid {
        properties.insert("status".into(), QueryValue::Unsigned(1));
    }
    let transaction = database::begin(&client, &session, "db-type-begin")
        .await
        .unwrap();
    assert!(database::commit(
        &client,
        &session,
        &transaction,
        vec![invalid],
        "db-type-commit"
    )
    .await
    .is_err());
    database::abort(&client, &session, &transaction, "db-type-abort")
        .await
        .unwrap();

    // A canceled transaction cannot subsequently commit its proposed write.
    let transaction = database::begin(&client, &session, "db-cancel-begin")
        .await
        .unwrap();
    database::abort(&client, &session, &transaction, "db-cancel-abort")
        .await
        .unwrap();
    assert!(database::commit(
        &client,
        &session,
        &transaction,
        vec![
            database::task_record("canceled", "done", rejected_at),
            database::document_record("canceled", 7, rejected_at),
        ],
        "db-cancel-commit"
    )
    .await
    .is_err());

    let updated_at = created_at + 2;
    database::read_view(&client, &session, updated_at, "db-cancel-view")
        .await
        .unwrap()
        .verify(&scope, "open")
        .unwrap();
    let stale = database::begin(&client, &session, "db-stale-begin")
        .await
        .unwrap();
    let transaction = database::begin(&client, &session, "db-update-begin")
        .await
        .unwrap();
    let mutations = vec![database::task_record(
        "Build with rrflowDB",
        "done",
        updated_at,
    )];
    let committed = database::commit(
        &client,
        &session,
        &transaction,
        mutations.clone(),
        "db-update-commit",
    )
    .await
    .unwrap();
    let replay = database::commit(
        &client,
        &session,
        &transaction,
        mutations,
        "db-update-commit",
    )
    .await
    .unwrap();
    assert!(replay.idempotent_replay);
    assert_eq!(
        replay.runtime_commit_sha256,
        committed.runtime_commit_sha256
    );
    assert!(database::commit(
        &client,
        &session,
        &stale,
        vec![database::task_record("stale write", "lost", updated_at)],
        "db-stale-commit"
    )
    .await
    .is_err());
    database::abort(&client, &session, &stale, "db-stale-abort")
        .await
        .unwrap();
    let updated = database::tasks(&client, &session, updated_at, "db-updated")
        .await
        .unwrap();
    assert_eq!(
        updated.rows[0].values["status"],
        QueryValue::String("done".into())
    );
    let updated_view = database::read_view(&client, &session, updated_at, "db-updated-view")
        .await
        .unwrap();
    updated_view.verify(&scope, "done").unwrap();
    assert!(updated_view.verify(&scope, "open").is_err());
    assert!(updated_view.tasks.known_at_cursor > before.known_at_cursor);
    // A commit between the two queries must not mix HEAD with the captured view.
    let earlier_join = database::relationships(
        &client,
        &session,
        created_at,
        before.known_at_cursor,
        "db-earlier-join",
    )
    .await
    .unwrap();
    let earlier_view = database::DatabaseView {
        tasks: before.clone(),
        relationships: earlier_join,
        documents: database::documents(
            &client,
            &session,
            created_at,
            before.known_at_cursor,
            "db-earlier-documents",
        )
        .await
        .unwrap(),
        document_relationships: database::document_relationships(
            &client,
            &session,
            created_at,
            before.known_at_cursor,
            "db-earlier-document-join",
        )
        .await
        .unwrap(),
    };
    earlier_view.verify(&scope, "open").unwrap();
    assert_eq!(earlier_view.relationships.rows, joined.rows);
    let mixed = database::DatabaseView {
        tasks: before.clone(),
        relationships: updated_view.relationships.clone(),
        documents: earlier_view.documents.clone(),
        document_relationships: earlier_view.document_relationships.clone(),
    };
    assert!(mixed.verify(&scope, "open").is_err());
    assert_eq!(
        database::tasks(&client, &session, created_at, "db-history")
            .await
            .unwrap()
            .rows,
        before.rows
    );

    // Scope and resource budgets are enforced by the real authenticated server.
    for (scope, max_storage_keys) in [
        ("instance:foreign".into(), 4096),
        (format!("instance:{}", client.instance_id()), 1),
    ] {
        assert!(client
            .read_data_schema(
                &session,
                ReadDataSchema {
                    scope,
                    max_storage_keys
                },
                read_options("db-denied").unwrap()
            )
            .await
            .is_err());
    }
    shutdown.send(()).unwrap();
    server.await.unwrap().unwrap();
    drop(client);

    let (reopened, shutdown, server) = start(&project).await;
    let session = login(&reopened, &project, "db-reopen-session").await;
    let durable = database::tasks(&reopened, &session, updated_at, "db-reopened")
        .await
        .unwrap();
    assert_eq!(durable.rows, updated.rows);
    assert_eq!(durable.known_at_cursor, updated.known_at_cursor);
    assert_eq!(durable.schema_revision, updated.schema_revision);
    assert_eq!(
        database::relationships(
            &reopened,
            &session,
            updated_at,
            durable.known_at_cursor,
            "db-reopened-join"
        )
        .await
        .unwrap()
        .rows,
        joined.rows
    );
    database::read_view(&reopened, &session, updated_at, "db-reopened-view")
        .await
        .unwrap()
        .verify(&scope, "done")
        .unwrap();
    assert_eq!(
        schema(&reopened, &session, "db-reopened-schema")
            .await
            .schema,
        installed_schema.schema
    );
    reopened
        .close_session(&session, CloseSession {}, options("db-close").unwrap())
        .await
        .unwrap();
    assert!(reopened
        .read_data_schema(
            &session,
            ReadDataSchema {
                scope: format!("instance:{}", reopened.instance_id()),
                max_storage_keys: 4096,
            },
            read_options("db-closed-session").unwrap()
        )
        .await
        .is_err());
    shutdown.send(()).unwrap();
    server.await.unwrap().unwrap();
}
