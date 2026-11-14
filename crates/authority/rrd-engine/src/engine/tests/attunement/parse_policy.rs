//! Selected parse policies and parse sources bound to the committed inventory.

use super::*;

#[test]
pub(super) fn selected_parse_policy_survives_installation_and_engine_reopen() {
    let policy = rrd_contract::SourceParsePolicy {
        format_version: 1,
        bindings: vec![rrd_contract::SourceParseBinding {
            extension: "rs".into(),
            language: CanonicalId::new("rust").unwrap(),
        }],
        max_files: 64,
        max_file_bytes: 1_048_576,
        max_input_bytes: 4_194_304,
        max_nodes: 10_000,
        max_depth: 64,
        max_output_bytes: 1_048_576,
        max_commit_bytes: 262_144,
        max_elapsed_ms: 10_000,
        max_memory_bytes: 8_388_608,
    };
    let proposal = crate::AttunementProposal {
        format_version: crate::ATTUNEMENT_PROPOSAL_FORMAT_VERSION,
        steps: [
            Phase::Connect,
            Phase::Inventory,
            Phase::Parse,
            Phase::Verify,
        ]
        .into_iter()
        .map(|phase| crate::AttunementProposalStep {
            phase,
            description: format!("{phase:?}"),
            estimated_items: 0,
            estimated_input_bytes: 0,
            parse_policy: (phase == Phase::Parse).then(|| policy.clone()),
        })
        .collect(),
        estimated_min_duration_ms: 0,
        estimated_max_duration_ms: 30_000,
    };
    let (installed, engine, _) =
        Installed::with_proposal(InstallationTargetKind::ExistingProject, Some(&proposal));
    let planned = installed
        .preview
        .attunement
        .phases
        .iter()
        .find(|step| step.phase == Phase::Parse)
        .unwrap();
    assert_eq!(planned.parse_policy.as_ref(), Some(&policy));
    drop(engine);
    let engine = installed.reopen(BASE + 4_000);
    let session = installed.session(&engine, "parse-policy-reopen", BASE + 4_100);
    let read = engine
        .read_attunement_detail(
            &session.session_id,
            &session.token,
            &rrd_contract::ReadAttunement {
                contract_version: rrd_contract::ATTUNEMENT_CONTRACT_VERSION,
                job_id: installed.job.clone(),
            },
            BASE + 4_200,
            "request-parse-policy-read",
            "operation-parse-policy-read",
        )
        .unwrap();
    assert_eq!(read.plan, installed.preview.attunement);
    assert_eq!(
        read.plan
            .phases
            .iter()
            .find(|step| step.phase == Phase::Parse)
            .unwrap()
            .parse_policy
            .as_ref(),
        Some(&policy)
    );

    let mut too_large = proposal.clone();
    let oversized = too_large
        .steps
        .iter_mut()
        .find(|step| step.phase == Phase::Parse)
        .unwrap()
        .parse_policy
        .as_mut()
        .unwrap();
    let (profile, _) = crate::engine::installation::DefaultInstallProfile::load("default").unwrap();
    oversized.max_file_bytes = profile.inventory_policy.limits.file_bytes + 1;
    oversized.max_input_bytes = oversized.max_file_bytes;
    let other = tempfile::tempdir().unwrap();
    std::fs::write(other.path().join("README.md"), b"existing source").unwrap();
    let result = RrdEngine::plan_installation_with_proposal(
        other.path(),
        InstallationTargetKind::ExistingProject,
        &RrdEngine::generate_installation_identity().unwrap(),
        "default",
        None,
        Some(&too_large),
        &installed.executable,
    );
    assert!(matches!(result, Err(ServiceError::ConfigurationLimit(_))));
    assert!(!other.path().join(".rrflow").exists());
    assert_eq!(
        std::fs::read(other.path().join("README.md")).unwrap(),
        b"existing source"
    );

    let mut streamed = proposal.clone();
    let transaction_limit = installed
        .preview
        .installation
        .configuration
        .storage
        .transaction_admission_bytes;
    let bounds = streamed
        .steps
        .iter_mut()
        .find(|step| step.phase == Phase::Parse)
        .unwrap()
        .parse_policy
        .as_mut()
        .unwrap();
    bounds.max_output_bytes = transaction_limit + 1;
    bounds.max_commit_bytes = transaction_limit;
    let identity = RrdEngine::generate_installation_identity().unwrap();
    assert!(RrdEngine::plan_installation_with_proposal(
        other.path(),
        InstallationTargetKind::ExistingProject,
        &identity,
        "default",
        None,
        Some(&streamed),
        &installed.executable,
    )
    .is_ok());
    streamed
        .steps
        .iter_mut()
        .find(|step| step.phase == Phase::Parse)
        .unwrap()
        .parse_policy
        .as_mut()
        .unwrap()
        .max_commit_bytes = transaction_limit + 1;
    assert!(matches!(
        RrdEngine::plan_installation_with_proposal(
            other.path(),
            InstallationTargetKind::ExistingProject,
            &identity,
            "default",
            None,
            Some(&streamed),
            &installed.executable,
        ),
        Err(ServiceError::ConfigurationLimit(_))
    ));
    assert!(!other.path().join(".rrflow").exists());
}

#[test]
pub(super) fn parse_source_is_bound_to_the_committed_inventory_after_reopen() {
    let proposal = crate::AttunementProposal {
        format_version: crate::ATTUNEMENT_PROPOSAL_FORMAT_VERSION,
        steps: [
            Phase::Connect,
            Phase::Inventory,
            Phase::Parse,
            Phase::Verify,
        ]
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
    };
    let (installed, engine, session) = Installed::with_source(
        InstallationTargetKind::ExistingProject,
        Some(&proposal),
        |root| std::fs::write(root.join("source.rs"), b"fn answer() { }\n").unwrap(),
    );
    let started = call(
        &engine,
        &session,
        &installed.start(30_000),
        "source-start",
        BASE + 1_000,
    )
    .unwrap();
    let connected = call(
        &engine,
        &session,
        &step(&started.status.job),
        "source-connect",
        BASE + 2_000,
    )
    .unwrap();
    let inventory = call(
        &engine,
        &session,
        &step(&connected.status.job),
        "source-inventory",
        BASE + 3_000,
    )
    .unwrap();
    let job = inventory.status.job;
    assert_eq!(job.current_phase, Some(Phase::Parse));
    let mut remaining_keys = 10_000;
    assert!(crate::engine::attunement::load_parse_inventory(
        &engine,
        &connected.status.job,
        &installed.preview.attunement,
        &mut remaining_keys
    )
    .is_err());
    let mut substituted = job.clone();
    let checkpoint = substituted.checkpoints.last_mut().unwrap();
    checkpoint.output_sha256 = "0".repeat(64);
    checkpoint.checkpoint_sha256 = rrd_contract::attunement_checkpoint_sha256(checkpoint).unwrap();
    let mut denied_keys = 1;
    assert!(matches!(
        crate::engine::attunement::load_parse_inventory(
            &engine,
            &substituted,
            &installed.preview.attunement,
            &mut denied_keys
        ),
        Err(ServiceError::ProjectBindingMismatch)
    ));
    assert_eq!(
        denied_keys, 1,
        "reject a changed checkpoint before inventory reads"
    );
    let mut other_plan = installed.preview.attunement.clone();
    other_plan.source_sha256 = "1".repeat(64);
    other_plan.plan_sha256 = rrd_contract::attunement_plan_sha256(&other_plan).unwrap();
    let mut other_job = job.clone();
    other_job.plan_sha256 = other_plan.plan_sha256.clone();
    for (index, checkpoint) in other_job.checkpoints.iter_mut().enumerate() {
        checkpoint.plan_sha256 = other_plan.plan_sha256.clone();
        if index == 0 {
            checkpoint.input_sha256 = other_plan.source_sha256.clone();
        }
        checkpoint.checkpoint_sha256 =
            rrd_contract::attunement_checkpoint_sha256(checkpoint).unwrap();
    }
    other_job.validate(&other_plan).unwrap();
    assert!(matches!(
        crate::engine::attunement::load_parse_inventory(
            &engine,
            &other_job,
            &other_plan,
            &mut remaining_keys
        ),
        Err(ServiceError::ProjectBindingMismatch)
    ));
    drop(engine);
    let engine = installed.reopen(BASE + 4_000);
    let snapshot = crate::engine::attunement::load_parse_inventory(
        &engine,
        &job,
        &installed.preview.attunement,
        &mut remaining_keys,
    )
    .unwrap();
    let source = snapshot
        .entries()
        .iter()
        .find(|entry| entry.path == "source.rs")
        .unwrap();
    let reopened_session = installed.session(&engine, "source-reopened", BASE + 4_100);
    let traversal = engine
        .execute_query(
            &reopened_session.session_id,
            &reopened_session.token,
            &rrd_contract::ExecuteQuery {
                scope: format!("instance:{}", engine.instance_id()),
                query: format!(
                    "FROM traverse:source-tree-contains START source-tree-snapshot:\"{}\" DIRECTION OUTGOING DEPTH 2 AT VALID {} KNOWN HEAD PROJECT node_id, depth",
                    job.checkpoints.last().unwrap().output_sha256,
                    BASE + 4_200
                ),
                parameters: Default::default(),
                budget: rrd_contract::QueryBudget::default(),
            },
            BASE + 4_200,
            "request-source-query",
            "operation-source-query",
        )
        .unwrap();
    assert!(
        traversal.rows.iter().any(|row| {
            row.values.get("node_id")
                == Some(&rrd_contract::QueryValue::String(source.identity.clone()))
        }),
        "rrflowQL must reach the actual inventoried source identity"
    );
    let root = engine.installed_project_root.as_ref().unwrap();
    let mut remaining_bytes = 1024;
    let mut output = Vec::new();
    crate::engine::installation::read_selected_source(
        root,
        &snapshot,
        source,
        1024,
        &mut remaining_bytes,
        &mut output,
        &|| Ok(()),
    )
    .unwrap();
    assert_eq!(output, b"fn answer() { }\n");
    std::fs::write(
        installed.root.path().join("source.rs"),
        b"fn changed() {}\n",
    )
    .unwrap();
    assert!(crate::engine::installation::read_selected_source(
        root,
        &snapshot,
        source,
        1024,
        &mut remaining_bytes,
        &mut output,
        &|| Ok(())
    )
    .is_err());
}
