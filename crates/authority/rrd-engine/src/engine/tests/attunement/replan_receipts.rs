//! Replans after progress, repeated step plans and receipt readback.

use super::*;

pub(super) fn selected_plan_replans_after_progress_and_reopens(git: bool) {
    let configured = |phase, name: &str| crate::AttunementProposalStep {
        phase,
        description: name.into(),
        estimated_items: 1,
        estimated_input_bytes: 0,
        parse_policy: None,
    };
    let proposal = crate::AttunementProposal {
        format_version: crate::ATTUNEMENT_PROPOSAL_FORMAT_VERSION,
        steps: vec![
            configured(Phase::Connect, "first"),
            configured(Phase::Connect, "second"),
            configured(Phase::Verify, "verify"),
        ],
        estimated_min_duration_ms: 0,
        estimated_max_duration_ms: 1000,
    };
    let (installed, engine, session) = Installed::with_source(
        InstallationTargetKind::ExistingProject,
        Some(&proposal),
        |root| {
            if git {
                std::fs::create_dir(root.join(".git")).unwrap();
                std::fs::create_dir(root.join("src")).unwrap();
                std::fs::write(
                    root.join(".git/index"),
                    include_bytes!(
                        "../../../../../../compute/rrd-attunement/tests/fixtures/git-index/sha1-v2.index"
                    ),
                )
                .unwrap();
                std::fs::write(root.join(".gitignore"), b"*.log\nsrc/\n").unwrap();
                std::fs::write(root.join("src/main.rs"), b"tracked source\n").unwrap();
            }
        },
    );
    assert_eq!(installed.preview.attunement.phases.len(), 3);
    if git {
        std::fs::write(
            installed.root.path().join("ignored.log"),
            b"irrelevant edit\n",
        )
        .unwrap();
        std::fs::write(
            installed.root.path().join("src/untracked"),
            b"ignored sibling\n",
        )
        .unwrap();
    }
    let running = call(
        &engine,
        &session,
        &installed.start(30_000),
        "public-start",
        BASE + 1_000,
    )
    .unwrap();
    let first = call(
        &engine,
        &session,
        &step(&running.status.job),
        "public-step-1",
        running.status.job.updated_at_unix_ms + 1,
    )
    .unwrap();
    let second = call(
        &engine,
        &session,
        &step(&first.status.job),
        "public-step-2",
        first.status.job.updated_at_unix_ms + 1,
    )
    .unwrap();
    let second_at = second.status.job.updated_at_unix_ms;
    assert_eq!(second.status.job.checkpoints.len(), 2);
    std::fs::write(
        installed
            .root
            .path()
            .join(if git { "src/main.rs" } else { "README.md" }),
        "source changed after progress\n",
    )
    .unwrap();
    assert!(matches!(
        call(
            &engine,
            &session,
            &step(&second.status.job),
            "public-stale-step",
            second_at + 1
        ),
        Err(ServiceError::AttunementSourceChanged)
    ));
    let proposed = engine
        .preview_attunement_replan(
            &session.session_id,
            &session.token,
            &installed.job,
            second.status.job.revision,
            second_at + 2,
            "request-public-preview",
            "operation-public-preview",
        )
        .unwrap();
    assert_ne!(
        proposed.plan.source_sha256,
        installed.preview.attunement.source_sha256
    );
    assert_eq!(proposed.plan.phases, installed.preview.attunement.phases);
    let sequence = engine.storage.control().sequence().unwrap();
    let accepted = engine
        .accept_attunement_replan(
            &session.session_id,
            &session.token,
            &proposed,
            &proposed.proposal_sha256,
            &context("public-accept"),
            second_at + 3,
        )
        .unwrap();
    let accepted_at = accepted.predecessor.job.updated_at_unix_ms;
    assert_eq!(
        accepted.predecessor.job.checkpoints,
        second.status.job.checkpoints
    );
    assert!(accepted.successor.job.checkpoints.is_empty());
    let journal = engine
        .storage
        .control()
        .journal_since(sequence, 32)
        .unwrap();
    assert!(journal
        .iter()
        .any(|entry| entry.action == "attunement.replan_receipted"));
    drop(engine);
    let engine = installed.reopen(accepted_at + 1);
    let session = installed.session(&engine, "public-reopen", accepted_at + 2);
    let replay = engine
        .accept_attunement_replan(
            &session.session_id,
            &session.token,
            &proposed,
            &proposed.proposal_sha256,
            &context("public-accept"),
            accepted_at + 3,
        )
        .unwrap();
    assert!(replay.idempotent_replay);
    assert_eq!(replay.successor, accepted.successor);
    assert_eq!(replay.predecessor, accepted.predecessor);
}

#[test]
pub(super) fn attunement_follows_a_bound_repeated_step_plan_after_reopen() {
    // Install a test-only plan through the actual control store. There is no
    // public operation for accepting custom attunement plans in this package.
    let (installed, engine, session) = Installed::new(InstallationTargetKind::ExistingProject);
    let mut plan = installed.preview.attunement.clone();
    plan.id = CanonicalId::new("test-variable-plan").unwrap();
    let mut second = plan.phases[0].clone();
    second.sequence = 2;
    second.configuration_sha256 = digest::sha256_hex(b"second-connect-configuration");
    // The default plan no longer ends in Verify; this test-only plan adds one
    // explicitly so the unavailable final step still exercises fail-closed.
    let verify = rrd_contract::AttunementPhasePlan {
        phase: Phase::Verify,
        sequence: 3,
        configuration_sha256: digest::sha256_hex(b"verify-configuration"),
        estimated_items: 1,
        estimated_input_bytes: 0,
        parse_policy: None,
    };
    plan.phases = vec![plan.phases[0].clone(), second, verify];
    plan.plan_sha256 = rrd_contract::attunement_plan_sha256(&plan).unwrap();
    plan.validate().unwrap();
    let job_id = CanonicalId::new("test-variable-job").unwrap();
    let mut job = installed.read(&engine, &session, BASE + 500).job;
    job.id = job_id.clone();
    job.plan_id = plan.id.clone();
    job.plan_sha256 = plan.plan_sha256.clone();
    job.validate(&plan).unwrap();
    let transition = |key, replacement| ControlTransition {
        key,
        expected: None,
        replacement: Some(replacement),
        at: BASE + 600,
        actor: "test:fixture".into(),
        action: "test.plan-fixture".into(),
        request_id: "test-plan".into(),
        operation_id: "test-plan".into(),
    };
    engine
        .storage
        .control()
        .commit_batch(&[
            transition(
                crate::engine::installation::attunement_plan_key(engine.instance_id(), &plan.id),
                serde_json::to_vec(&plan).unwrap(),
            ),
            transition(
                crate::engine::installation::attunement_job_key(engine.instance_id(), &job_id),
                serde_json::to_vec(&job).unwrap(),
            ),
            // This injected test job is not the installed initial job, so it
            // needs the same generation record as an admitted successor.
            transition(
                format!(
                    "{}/replan-generation",
                    crate::engine::installation::attunement_job_key(engine.instance_id(), &job_id)
                ),
                serde_json::to_vec(&1_u64).unwrap(),
            ),
        ])
        .unwrap();
    let start = AttunementCommand::Start {
        job_id: job_id.clone(),
        plan_sha256: plan.plan_sha256.clone(),
        expected_revision: 1,
        lease_ms: 30_000,
    };
    let running = call(&engine, &session, &start, "variable-start", BASE + 1_000).unwrap();
    let first = call(
        &engine,
        &session,
        &step(&running.status.job),
        "variable-first",
        running.status.job.updated_at_unix_ms + 1,
    )
    .unwrap();
    assert_eq!(first.status.job.current_phase, Some(Phase::Connect));
    let paused = call(
        &engine,
        &session,
        &AttunementCommand::Pause {
            job_id: job_id.clone(),
            expected_revision: first.status.job.revision,
            lease_generation: first.status.job.lease.as_ref().unwrap().generation,
        },
        "variable-pause",
        first.status.job.updated_at_unix_ms + 1,
    )
    .unwrap();
    let paused_at = paused.status.job.updated_at_unix_ms;
    drop(engine);
    let engine = installed.reopen(paused_at + 1);
    let renewed = installed.session(&engine, "variable-reopened", paused_at + 2);
    let read = engine
        .read_attunement(
            &renewed.session_id,
            &renewed.token,
            &job_id,
            paused_at + 3,
            "variable-read",
            "variable-read",
        )
        .unwrap();
    assert_eq!(read.job, paused.status.job);
    let running = call(
        &engine,
        &renewed,
        &resume(&read.job, paused_at + 4),
        "variable-resume",
        paused_at + 4,
    )
    .unwrap();
    let second = call(
        &engine,
        &renewed,
        &step(&running.status.job),
        "variable-second",
        running.status.job.updated_at_unix_ms + 1,
    )
    .unwrap();
    assert_eq!(second.status.job.current_phase, Some(Phase::Verify));
    assert_eq!(
        second.status.job.checkpoints[1].configuration_sha256,
        plan.phases[1].configuration_sha256
    );
    assert_eq!(
        second.status.job.checkpoints[1].input_sha256,
        second.status.job.checkpoints[0].output_sha256
    );
    let unavailable = call(
        &engine,
        &renewed,
        &step(&second.status.job),
        "variable-verify",
        second.status.job.updated_at_unix_ms + 1,
    )
    .unwrap();
    assert_eq!(unavailable.status.job.state, State::Failed);
    assert_eq!(unavailable.status.job.checkpoints.len(), 2);
    assert_eq!(
        unavailable
            .status
            .job
            .failure
            .as_ref()
            .unwrap()
            .code
            .as_str(),
        "phase-unavailable"
    );

    // A normal source edit after multiple completed, read-only Connect steps
    // must allow a successor without erasing the old checkpoint evidence.
    std::fs::write(
        installed.root.path().join("README.md"),
        "edited after progress\n",
    )
    .unwrap();
    let preview = engine
        .preview_attunement_replan(
            &renewed.session_id,
            &renewed.token,
            &job_id,
            unavailable.status.job.revision,
            unavailable.status.job.updated_at_unix_ms + 1,
            "request-variable-replan",
            "operation-variable-replan",
        )
        .unwrap();
    assert_ne!(preview.plan.source_sha256, plan.source_sha256);
    assert_eq!(preview.plan.phases, plan.phases);
    let result = engine
        .accept_attunement_replan(
            &renewed.session_id,
            &renewed.token,
            &preview,
            &preview.proposal_sha256,
            &context("variable-replan"),
            unavailable.status.job.updated_at_unix_ms + 2,
        )
        .unwrap();
    let accepted_at = result.predecessor.job.updated_at_unix_ms;
    assert_eq!(
        result.predecessor.job.checkpoints,
        unavailable.status.job.checkpoints
    );
    assert_eq!(result.successor.job.state, State::Pending);
    assert!(result.successor.job.checkpoints.is_empty());
    drop(engine);
    let engine = installed.reopen(accepted_at + 1);
    let renewed = installed.session(&engine, "variable-replan-reopen", accepted_at + 2);
    let replay = engine
        .accept_attunement_replan(
            &renewed.session_id,
            &renewed.token,
            &preview,
            &preview.proposal_sha256,
            &context("variable-replan"),
            accepted_at + 3,
        )
        .unwrap();
    assert!(replay.idempotent_replay);
    assert_eq!(replay.successor, result.successor);
    assert_eq!(replay.predecessor, result.predecessor);
}

#[test]
pub(super) fn attunement_receipt_readback_preserves_original_result_after_reopen() {
    let (installed, engine, session) = Installed::with_proposal(
        InstallationTargetKind::ExistingProject,
        Some(&proposal_with_unavailable_parse()),
    );
    assert!(engine
        .read_attunement_receipt(
            &session.session_id,
            &session.token,
            &installed.job,
            &id("connect"),
            BASE + 500,
            "receipt-missing",
            "receipt-missing",
        )
        .unwrap()
        .is_none());
    let started = call(
        &engine,
        &session,
        &installed.start(30_000),
        "start",
        BASE + 1_000,
    )
    .unwrap();
    let command = step(&started.status.job);
    let connected = call(
        &engine,
        &session,
        &command,
        "connect",
        started.status.job.updated_at_unix_ms + 1,
    )
    .unwrap();
    let inventoried = call(
        &engine,
        &session,
        &step(&connected.status.job),
        "inventory",
        connected.status.job.updated_at_unix_ms + 1,
    )
    .unwrap();
    assert_eq!(inventoried.status.job.current_phase, Some(Phase::Parse));
    let failed = call(
        &engine,
        &session,
        &step(&inventoried.status.job),
        "receipt-parse",
        inventoried.status.job.updated_at_unix_ms + 1,
    )
    .unwrap();
    assert_eq!(failed.status.job.state, State::Failed);
    let failure_at = failed.status.job.updated_at_unix_ms;
    let before = engine.storage.control().journal_since(0, 1024).unwrap();
    let count_receipts = |entries: &[rrd_store::ControlJournalEntry]| {
        entries
            .iter()
            .filter(|entry| entry.action == "attunement.receipted")
            .count()
    };
    let receipt = engine
        .read_attunement_receipt(
            &session.session_id,
            &session.token,
            &installed.job,
            &id("connect"),
            failure_at + 1,
            "receipt-read",
            "receipt-read",
        )
        .unwrap()
        .unwrap();
    assert_eq!(receipt.status, connected.status);
    assert_eq!(
        receipt.operation_sha256,
        digest::sha256_hex(&serde_json::to_vec(&command).unwrap())
    );
    // Independently reconstruct the pre-existing on-disk key from the
    // canonical principal and key text, rather than calling the new helper.
    let key_input = format!("[\"{}\",\"connect\"]", receipt.principal_id.as_str());
    let expected_key = format!(
        "server/state/{}/attunement/job/{}/receipt/{}",
        engine.instance_id(),
        installed.job,
        digest::sha256_hex(key_input.as_bytes())
    );
    assert!(before
        .iter()
        .any(|entry| { entry.action == "attunement.receipted" && entry.key == expected_key }));
    assert!(engine
        .read_attunement_receipt(
            &session.session_id,
            &id("invalid-token"),
            &installed.job,
            &id("connect"),
            failure_at + 2,
            "receipt-invalid",
            "receipt-invalid",
        )
        .is_err());
    assert!(engine
        .read_attunement_receipt(
            &session.session_id,
            &session.token,
            &CanonicalId::new("other-job").unwrap(),
            &id("connect"),
            failure_at + 3,
            "receipt-foreign",
            "receipt-foreign",
        )
        .unwrap()
        .is_none());
    let after = engine.storage.control().journal_since(0, 1024).unwrap();
    assert_eq!(count_receipts(&before), count_receipts(&after));
    assert_eq!(
        installed.read(&engine, &session, failure_at + 4).job,
        failed.status.job
    );
    drop(engine);
    let engine = installed.reopen(failure_at + 5);
    let renewed = installed.session(&engine, "receipt-renewed", failure_at + 6);
    let reopened = engine
        .read_attunement_receipt(
            &renewed.session_id,
            &renewed.token,
            &installed.job,
            &id("connect"),
            failure_at + 7,
            "receipt-reopened",
            "receipt-reopened",
        )
        .unwrap()
        .unwrap();
    assert_eq!(reopened, receipt);
    assert_eq!(
        installed.read(&engine, &renewed, failure_at + 8).job,
        failed.status.job
    );
}

#[test]
pub(super) fn attunement_receipt_readback_is_principal_scoped_and_authorized() {
    let (installed, engine, _) = Installed::new(InstallationTargetKind::FreshProject);
    let repository = SecurityRepository::new(&engine.storage, engine.instance_id().clone());
    let mut authority = repository.load().unwrap().unwrap();
    let revision = authority.revision;
    authority.revision += 1;
    let resource = ResourcePath {
        segments: vec![
            ResourceId::new(ResourceKind::Instance, engine.instance_id().as_str()).unwrap(),
        ],
    };
    for (name, read) in [("other-reader", true), ("without-read", false)] {
        let principal_id = CanonicalId::new(name).unwrap();
        let mut grants = vec![ResourceGrant {
            action: SecurityAction::SessionCreate,
            resource_prefix: resource.clone(),
            data_policy: None,
        }];
        if read {
            grants.push(ResourceGrant {
                action: SecurityAction::EstateRead,
                resource_prefix: resource.clone(),
                data_policy: None,
            });
        }
        authority.principals.insert(
            principal_id.clone(),
            Principal {
                id: principal_id,
                kind: PrincipalKind::Service,
                credential_sha256: digest::sha256_hex(name.as_bytes()),
                credential_revision: 1,
                not_before_unix_ms: 1,
                expires_at_unix_ms: u64::MAX,
                disabled: false,
                role_ids: Default::default(),
                grants,
            },
        );
    }
    repository
        .replace(
            revision,
            authority,
            BASE + 300,
            "test",
            "security-update",
            "security-update",
        )
        .unwrap();
    let owner = installed.session(&engine, "receipt-owner", BASE + 500);
    let started = call(
        &engine,
        &owner,
        &installed.start(30_000),
        "owner-receipt",
        BASE + 1_000,
    )
    .unwrap();
    let started_at = started.status.job.updated_at_unix_ms;
    let make_session = |name: &str| {
        engine
            .create_authenticated_session(
                &CanonicalId::new(name).unwrap(),
                name.as_bytes(),
                &CreateSession {
                    limits: SessionLimits {
                        idle_timeout_ms: 120_000,
                        absolute_timeout_ms: 120_000,
                        max_open_transactions: 2,
                    },
                },
                &id(name),
                started_at + 1,
                &format!("request-{name}"),
                &format!("operation-{name}"),
            )
            .unwrap()
    };
    let other = make_session("other-reader");
    let denied = make_session("without-read");
    assert!(engine
        .read_attunement_receipt(
            &other.session_id,
            &other.token,
            &installed.job,
            &id("owner-receipt"),
            started_at + 2,
            "other-read",
            "other-read"
        )
        .unwrap()
        .is_none());
    assert!(matches!(
        engine.read_attunement_receipt(
            &denied.session_id,
            &denied.token,
            &installed.job,
            &id("owner-receipt"),
            started_at + 3,
            "denied-read",
            "denied-read"
        ),
        Err(ServiceError::PermissionDenied)
    ));
    assert!(engine
        .read_attunement_receipt(
            &owner.session_id,
            &owner.token,
            &installed.job,
            &id("owner-receipt"),
            started_at + 4,
            "owner-read",
            "owner-read"
        )
        .unwrap()
        .is_some());
}

#[test]
pub(super) fn attunement_receipt_readback_rejects_corrupt_identity() {
    let (installed, engine, session) = Installed::new(InstallationTargetKind::FreshProject);
    let started = call(
        &engine,
        &session,
        &installed.start(30_000),
        "receipt-corrupt",
        BASE + 1_000,
    )
    .unwrap();
    let started_at = started.status.job.updated_at_unix_ms;
    let key = engine
        .storage
        .control()
        .journal_since(0, 1024)
        .unwrap()
        .into_iter()
        .find(|entry| entry.action == "attunement.receipted")
        .unwrap()
        .key;
    let original = engine.storage.control().get(&key).unwrap().unwrap();
    let mut value: serde_json::Value = serde_json::from_slice(&original).unwrap();
    value["operation_sha256"] = serde_json::json!("not-a-digest");
    engine
        .storage
        .control()
        .commit(&ControlTransition {
            key,
            expected: Some(original),
            replacement: Some(serde_json::to_vec(&value).unwrap()),
            at: started_at + 1,
            actor: "test".into(),
            action: "test.receipt-corruption".into(),
            request_id: "corrupt-request".into(),
            operation_id: "corrupt-operation".into(),
        })
        .unwrap();
    assert!(matches!(
        engine.read_attunement_receipt(
            &session.session_id,
            &session.token,
            &installed.job,
            &id("receipt-corrupt"),
            started_at + 2,
            "corrupt-read",
            "corrupt-read"
        ),
        Err(ServiceError::Storage(_))
    ));
}

/// An explicit operator proposal that selects a capability without an
/// installed executor. The default plan no longer contains one, so the tests
/// that characterize fail-closed behaviour at an unavailable step opt in.
pub(super) fn proposal_with_unavailable_parse() -> crate::AttunementProposal {
    crate::AttunementProposal {
        format_version: crate::ATTUNEMENT_PROPOSAL_FORMAT_VERSION,
        steps: [Phase::Connect, Phase::Inventory, Phase::Parse]
            .into_iter()
            .map(|phase| crate::AttunementProposalStep {
                phase,
                description: format!("{phase:?}"),
                estimated_items: 0,
                estimated_input_bytes: 0,
                parse_policy: None,
            })
            .collect(),
        estimated_min_duration_ms: 0,
        estimated_max_duration_ms: 30_000,
    }
}
