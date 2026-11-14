use super::*;
use crate::engine::tests::attunement::{
    call as attunement_call, context as attunement_context,
    first_inventory_for_worker_with_setup_at, step as attunement_step,
};
use crate::AttunementCommand;
use rrd_contract::{
    ApplyEndpointConfiguration, CredentialReference, EndpointGenerationSettings, EndpointLocator,
    EndpointProfileInput, EndpointResponseFormat, EndpointTokenParameter, EndpointWorkerBinding,
    EndpointWorkerPolicy, ExecuteQuery, ObservationExportPolicy, ObservationSource, OpenTask,
    PrepareObservationDelivery, PreviewEndpointConfiguration, QueryBudget, QueryValue,
    ReadObservationDelivery, ReadTask, ReadTaskAttempt, TaskAttemptInvalidation,
    TaskWorkerSourceRequest,
};
use rrd_security::ResourceGrant;
use std::path::Path;
use std::sync::mpsc;
use std::time::{SystemTime, UNIX_EPOCH};

mod assessment;

struct SourceWorkerFixture {
    installed: crate::engine::tests::attunement::Installed,
    engine: RrdEngine,
    lease: SessionLease,
    job: rrd_contract::AttunementJob,
    principal_id: CanonicalId,
    worker_token: CorrelationId,
    selection: EndpointSelectionRequest,
    source: TaskWorkerSourceRequest,
    source_path: String,
    source_content: String,
    now: u64,
}

fn host_unix_ms() -> u64 {
    u64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis(),
    )
    .unwrap()
}

fn worker_packet_from_chat_wire(wire: &serde_json::Value) -> serde_json::Value {
    serde_json::from_str(
        wire["messages"][1]["content"]
            .as_str()
            .expect("worker packet is encoded in the user message"),
    )
    .expect("worker packet JSON is valid")
}

fn set_query_execute_policy(
    engine: &RrdEngine,
    lease: &SessionLease,
    at: u64,
    data_policy: Option<rrd_security::DataPolicy>,
    suffix: &str,
) {
    let repository =
        rrd_security::SecurityRepository::new(&engine.storage, engine.instance.clone());
    let mut authority = repository.load().unwrap().unwrap();
    let expected_revision = authority.revision;
    let mut count = 0;
    for principal in authority.principals.values_mut() {
        for grant in &mut principal.grants {
            if grant.action == SecurityAction::QueryExecute {
                grant.data_policy = data_policy.clone();
                count += 1;
            }
        }
    }
    for role in authority.roles.values_mut() {
        for grant in &mut role.grants {
            if grant.action == SecurityAction::QueryExecute {
                grant.data_policy = data_policy.clone();
                count += 1;
            }
        }
    }
    assert!(count > 0, "fixture has a QueryExecute authorization grant");
    authority.revision += 1;
    engine
        .replace_security_authority(
            &lease.session_id,
            &lease.token,
            expected_revision,
            authority,
            at,
            &format!("source-worker-query-policy-{suffix}"),
            &format!("source-worker-query-policy-{suffix}"),
        )
        .unwrap();
}

fn verify_source_file_query_policy(
    engine: &RrdEngine,
    lease: &SessionLease,
    scope: &str,
    entry_identity: &str,
    at: u64,
) {
    let query = |projection: &str| {
        ExecuteQuery {
        scope: scope.to_owned(),
        query: format!(
            "FROM record:source-tree-entry AT VALID {at} KNOWN HEAD WHERE identity = \"{entry_identity}\" PROJECT {projection}"
        ),
        parameters: Default::default(),
        budget: QueryBudget::default(),
    }
    };
    set_query_execute_policy(
        engine,
        lease,
        at,
        Some(rrd_security::DataPolicy {
            tenant: None,
            rows: Vec::new(),
            allowed_fields: Some(std::collections::BTreeSet::from([
                "identity".into(),
                "path".into(),
                "source_file".into(),
            ])),
        }),
        "field-allowed",
    );
    let selected = engine
        .execute_query(
            &lease.session_id,
            &lease.token,
            &query("identity, path, source_file"),
            at + 1,
            "source-worker-source-file-query-allowed",
            "source-worker-source-file-query-allowed",
        )
        .unwrap();
    assert_eq!(selected.rows.len(), 1);

    set_query_execute_policy(
        engine,
        lease,
        at + 2,
        Some(rrd_security::DataPolicy {
            tenant: None,
            rows: Vec::new(),
            allowed_fields: Some(std::collections::BTreeSet::from([
                "identity".into(),
                "path".into(),
            ])),
        }),
        "field-denied",
    );
    assert!(matches!(
        engine.execute_query(
            &lease.session_id,
            &lease.token,
            &query("identity, path, source_file"),
            at + 3,
            "source-worker-source-file-query-denied",
            "source-worker-source-file-query-denied",
        ),
        Err(ServiceError::PermissionDenied)
    ));

    set_query_execute_policy(
        engine,
        lease,
        at + 4,
        Some(rrd_security::DataPolicy {
            tenant: None,
            rows: vec![rrd_security::PolicyPredicate {
                field: "identity".into(),
                value: rrd_core::RuntimeValue::String("0".repeat(64)),
            }],
            allowed_fields: Some(std::collections::BTreeSet::from([
                "identity".into(),
                "path".into(),
                "source_file".into(),
            ])),
        }),
        "row-denied",
    );
    let filtered = engine
        .execute_query(
            &lease.session_id,
            &lease.token,
            &query("identity, path, source_file"),
            at + 5,
            "source-worker-source-file-row-denied",
            "source-worker-source-file-row-denied",
        )
        .unwrap();
    assert!(filtered.rows.is_empty());
    set_query_execute_policy(engine, lease, at + 6, None, "restored");
}

fn source_worker_fixture(listener: &TcpListener, allow_source_export: bool) -> SourceWorkerFixture {
    source_worker_fixture_for_path(listener, allow_source_export, false, "alpha.txt")
}

fn source_worker_fixture_with_git(
    listener: &TcpListener,
    allow_source_export: bool,
    git_project: bool,
) -> SourceWorkerFixture {
    source_worker_fixture_for_path(
        listener,
        allow_source_export,
        git_project,
        if git_project {
            "nested/alpha.txt"
        } else {
            "alpha.txt"
        },
    )
}

fn source_worker_fixture_for_path(
    listener: &TcpListener,
    allow_source_export: bool,
    git_project: bool,
    source_path: &str,
) -> SourceWorkerFixture {
    source_worker_fixture_for_path_at(
        listener,
        allow_source_export,
        git_project,
        source_path,
        crate::engine::tests::attunement::BASE,
    )
}

fn source_worker_fixture_for_path_at(
    listener: &TcpListener,
    allow_source_export: bool,
    git_project: bool,
    source_path: &str,
    base: u64,
) -> SourceWorkerFixture {
    let (installed, engine, bootstrap_lease, job, _, _) = if git_project {
        first_inventory_for_worker_with_setup_at(
            |root| {
                std::fs::create_dir_all(root.join(".git")).unwrap();
                std::fs::write(
                    root.join(".git/config"),
                    b"[core]\nrepositoryFormatVersion=0\nignoreCase=false\n",
                )
                .unwrap();
                std::fs::write(
                root.join(".git/index"),
                include_bytes!("../../../../../../../compute/rrd-attunement/tests/fixtures/git-index/sha1-v2.index"),
            )
            .unwrap();
                std::fs::write(root.join(".gitignore"), b"*.log\nignored/\n").unwrap();
                std::fs::create_dir_all(root.join("nested")).unwrap();
                std::fs::write(root.join("nested/.gitignore"), b"*.tmp\n").unwrap();
                std::fs::write(root.join("nested/alpha.txt"), "source nested/alpha.txt\n").unwrap();
                std::fs::create_dir_all(root.join("ignored")).unwrap();
                std::fs::write(root.join("ignored/tracked.txt"), "tracked source\n").unwrap();
            },
            base,
        )
    } else {
        first_inventory_for_worker_with_setup_at(|_| {}, base)
    };
    let installed_record = engine.attunement_installed_record().unwrap();
    let project_id = installed_record.installation.target.project_id.clone();
    let accepted = crate::engine::attunement::load_accepted_source(
        &engine,
        &project_id,
        8 * 1024 * 1024,
        10_000,
    )
    .unwrap();
    let source_content = std::fs::read_to_string(installed.root.path().join(source_path)).unwrap();
    let entry = accepted
        .proposal
        .entries()
        .iter()
        .find(|entry| entry.path == source_path)
        .unwrap();
    let scope = format!("instance:{}", engine.instance_id());
    let selected_at = job.updated_at_unix_ms + 1;
    let snapshot_rows = engine
        .execute_query(
            &bootstrap_lease.session_id,
            &bootstrap_lease.token,
            &ExecuteQuery {
                scope: scope.clone(),
                query: format!(
                    "FROM record:source-tree-snapshot AT VALID {selected_at} KNOWN HEAD WHERE proposal_sha256 = \"{}\" PROJECT proposal_sha256, revisions",
                    accepted.proposal_sha256
                ),
                parameters: Default::default(),
                budget: QueryBudget::default(),
            },
            selected_at,
            "source-worker-public-snapshot-query",
            "source-worker-public-snapshot-query",
        )
        .unwrap();
    assert!(!snapshot_rows.execution.truncated);
    assert_eq!(snapshot_rows.rows.len(), 1);
    let snapshot = &snapshot_rows.rows[0];
    let queried_snapshot = match snapshot.values.get("proposal_sha256") {
        Some(QueryValue::String(value) | QueryValue::Digest(value)) => value,
        value => panic!("unexpected typed snapshot digest: {value:?}"),
    };
    assert_eq!(queried_snapshot, &accepted.proposal_sha256);
    let policy_sha256 = match snapshot.values.get("revisions") {
        Some(QueryValue::Map(values)) => match values.get("policy_sha256") {
            Some(QueryValue::String(value) | QueryValue::Digest(value)) => value.clone(),
            value => panic!("unexpected typed inventory policy digest: {value:?}"),
        },
        value => panic!("unexpected typed snapshot revisions: {value:?}"),
    };
    assert_eq!(policy_sha256.len(), 64);
    assert!(policy_sha256.bytes().all(|byte| byte.is_ascii_hexdigit()));
    let entry_rows = engine
        .execute_query(
            &bootstrap_lease.session_id,
            &bootstrap_lease.token,
            &ExecuteQuery {
                scope,
                query: format!(
                    "FROM record:source-tree-entry AT VALID {selected_at} KNOWN HEAD WHERE identity = \"{}\" PROJECT identity, path, source_file",
                    entry.identity
                ),
                parameters: Default::default(),
                budget: QueryBudget::default(),
            },
            selected_at,
            "source-worker-public-entry-query",
            "source-worker-public-entry-query",
        )
        .unwrap();
    assert!(!entry_rows.execution.truncated);
    assert_eq!(entry_rows.rows.len(), 1);
    let queried_entry = &entry_rows.rows[0];
    let entry_identity = match queried_entry.values.get("identity") {
        Some(QueryValue::String(value) | QueryValue::Digest(value)) => value.clone(),
        value => panic!("unexpected typed entry identity: {value:?}"),
    };
    assert_eq!(entry_identity, entry.identity);
    let queried_path = match queried_entry.values.get("path") {
        Some(QueryValue::String(value)) => value,
        value => panic!("unexpected typed entry path: {value:?}"),
    };
    assert_eq!(queried_path, source_path);
    let (content_sha256, queried_bytes) = match queried_entry.values.get("source_file") {
        Some(QueryValue::Map(values)) => match values.get("content_sha256") {
            Some(QueryValue::String(value) | QueryValue::Digest(value)) => {
                let bytes = match values.get("bytes") {
                    Some(QueryValue::Unsigned(bytes)) => *bytes,
                    value => panic!("unexpected typed file byte count: {value:?}"),
                };
                (value.clone(), bytes)
            }
            value => panic!("unexpected typed file content digest: {value:?}"),
        },
        value => panic!("unexpected typed source_file projection: {value:?}"),
    };
    assert_eq!(queried_bytes, source_content.len() as u64);
    assert_eq!(
        content_sha256,
        digest::sha256_hex(source_content.as_bytes())
    );
    let source = TaskWorkerSourceRequest {
        contract_version: rrd_contract::TASK_WORKER_SOURCE_VERSION,
        entry_identity,
        expected_content_sha256: content_sha256,
        maximum_input_bytes: 64 * 1024,
    };

    let repository =
        rrd_security::SecurityRepository::new(&engine.storage, engine.instance.clone());
    let mut authority = repository.load().unwrap().unwrap();
    let expected_revision = authority.revision;
    let bootstrap_principal = authority
        .principals
        .get(&installed.preview.installation.initial_principal_id)
        .unwrap();
    assert!(!bootstrap_principal
        .grants
        .iter()
        .any(|grant| grant.action == SecurityAction::ProjectSourceRead));
    let principal_id = CanonicalId::new("source-worker-principal").unwrap();
    let token = id("source-worker-principal-token");
    let mut principal = bootstrap_principal.clone();
    principal.id = principal_id.clone();
    principal.credential_sha256 = digest::sha256_hex(token.as_str().as_bytes());
    principal.grants.push(ResourceGrant {
        action: SecurityAction::ProjectSourceRead,
        resource_prefix: ResourcePath {
            segments: vec![
                ResourceId::new(ResourceKind::Instance, engine.instance.as_str()).unwrap(),
                ResourceId::new(ResourceKind::Project, project_id.as_str()).unwrap(),
                ResourceId::new(ResourceKind::Record, entry.identity.clone()).unwrap(),
            ],
        },
        data_policy: None,
    });
    authority.principals.insert(principal_id.clone(), principal);
    authority.revision += 1;
    let grant_at = host_unix_ms().max(job.updated_at_unix_ms + 1);
    engine
        .replace_security_authority(
            &bootstrap_lease.session_id,
            &bootstrap_lease.token,
            expected_revision,
            authority,
            grant_at,
            "source-worker-grant",
            "source-worker-grant",
        )
        .unwrap();
    let lease = engine
        .create_authenticated_session(
            &principal_id,
            token.as_str().as_bytes(),
            &crate::engine::tests::session_request(10_000, 2),
            &id("source-worker-principal-session"),
            grant_at + 1,
            "source-worker-principal-session",
            "source-worker-principal-session",
        )
        .unwrap();

    let scope = format!("instance:{}", engine.instance_id());
    let query_policy_at = grant_at + 2;
    verify_source_file_query_policy(&engine, &lease, &scope, &entry.identity, query_policy_at);

    let profile = proposal::configured().profile.unwrap();
    let selector = profile.selection_policy.unwrap();
    let candidate = selector.candidates[0].clone();
    let mut bindings = profile.bindings;
    for binding in &mut bindings {
        binding.endpoint = EndpointLocator::Url {
            url: format!("http://{}/v1", listener.local_addr().unwrap()),
        };
        binding.credential = None;
        binding.limits.timeout_ms = 5_000;
    }
    let input = EndpointProfileInput {
        contract_version: profile.contract_version,
        name: profile.name,
        bindings,
        selection_policy: Some(selector),
        observation_policy: None,
        assessment_policy: None,
        worker_policy: Some(EndpointWorkerPolicy {
            contract_version: rrd_contract::ENDPOINT_WORKER_VERSION,
            allow_network: true,
            allow_context_export: true,
            allow_source_export,
            allow_unverified_identity: true,
            maximum_concurrent_calls: 1,
            bindings: vec![EndpointWorkerBinding {
                binding_id: candidate.binding_id,
                protocol_evidence_sha256: digest::sha256_hex(b"controlled source worker fixture"),
                adapter: Default::default(),
                system_prompt: "Return a bounded draft. Do not mutate the project.".into(),
                generation: EndpointGenerationSettings {
                    maximum_output_tokens: 256,
                    token_parameter: EndpointTokenParameter::MaxTokens,
                    temperature_milli: None,
                    response_format: EndpointResponseFormat::Text,
                },
                maximum_response_bytes: 16_384,
                effort_values: Default::default(),
                allow_default_effort: true,
            }],
        }),
    };
    let at = query_policy_at + 7;
    let plan = engine
        .preview_endpoint_configuration(
            &lease.session_id,
            &lease.token,
            &PreviewEndpointConfiguration {
                expected_revision: 0,
                profile: Some(input),
            },
            at,
            "source-worker-config-preview",
            "source-worker-config-preview",
        )
        .unwrap();
    let configuration = engine
        .apply_endpoint_configuration(
            &lease.session_id,
            &lease.token,
            &ApplyEndpointConfiguration {
                approved_plan_sha256: plan.plan_sha256.clone(),
                plan,
            },
            at + 1,
            "source-worker-config-apply",
            "source-worker-config-apply",
        )
        .unwrap();
    let scope = format!("instance:{}", engine.instance_id());
    let task_id = engine
        .open_task(
            &lease.session_id,
            &lease.token,
            &id("source-worker-open"),
            &OpenTask {
                scope: scope.clone(),
                title: "Inspect one selected file".into(),
                goal: "Return a bounded draft from selected source.".into(),
            },
            at + 2,
            "source-worker-open",
            "source-worker-open",
        )
        .unwrap()
        .task
        .task_id;
    let task = engine
        .read_task(
            &lease.session_id,
            &lease.token,
            &ReadTask {
                scope: scope.clone(),
                task_id: task_id.clone(),
                max_storage_keys: MAX_TASK_STORAGE_KEYS,
            },
            at + 3,
            "source-worker-read-task",
            "source-worker-read-task",
        )
        .unwrap();
    let selection = EndpointSelectionRequest {
        scope,
        task_id,
        attempt_id: id("source-worker-selection"),
        max_storage_keys: MAX_TASK_STORAGE_KEYS,
        expected_task_sha256: EndpointSelectionTask::from_task(&task.task)
            .unwrap()
            .task_sha256,
        expected_configuration_sha256: configuration.sha256().unwrap(),
        flight: None,
        routing: None,
    };
    SourceWorkerFixture {
        installed,
        engine,
        lease,
        job,
        principal_id,
        worker_token: token,
        selection,
        source,
        source_path: source_path.into(),
        source_content,
        now: at + 4,
    }
}

fn selected_request_host(
    engine: &RrdEngine,
    lease: &SessionLease,
    listener: &TcpListener,
    selection: &EndpointSelectionRequest,
) -> TaskWorkerRequest {
    let at = host_unix_ms();
    let configuration = engine
        .read_endpoint_configuration(&lease.session_id, &lease.token, at, "read", "read")
        .unwrap();
    let candidate = configuration
        .profile
        .unwrap()
        .selection_policy
        .unwrap()
        .candidates[0]
        .clone();
    let proposal = std::thread::scope(|scope| {
        let server = scope.spawn(|| {
            let (mut socket, body) = read_wire(listener);
            let wire: serde_json::Value = serde_json::from_slice(&body).unwrap();
            let choice = EndpointSelectionChoice::Select {
                binding_id: candidate.binding_id,
                effort: None,
            };
            reply(
                &mut socket,
                wire["model"].as_str().unwrap(),
                &serde_json::to_string(&choice).unwrap(),
            );
        });
        let result = engine.propose_task_endpoint(
            &lease.session_id,
            &lease.token,
            selection,
            "source-worker-host-selection",
            "source-worker-host-selection",
        );
        server.join().unwrap();
        result.unwrap()
    });
    TaskWorkerRequest {
        scope: proposal.scope.clone(),
        task_id: proposal.task_id.clone(),
        attempt_id: id("source-worker-host-attempt"),
        selection_attempt_id: proposal.attempt_id.clone(),
        expected_selection_sha256: operation_digest(&proposal).unwrap(),
        expected_task_sha256: proposal.task_sha256,
        expected_configuration_sha256: proposal.configuration_sha256,
        max_storage_keys: MAX_TASK_STORAGE_KEYS,
        context: None,
        source: None,
    }
}

fn refresh_inventory_without_source_edits(
    engine: &RrdEngine,
    lease: &SessionLease,
    job: &mut rrd_contract::AttunementJob,
    minimum_at: u64,
) {
    let preview_at = (job.updated_at_unix_ms + 1).max(minimum_at);
    let preview = engine
        .preview_attunement_replan(
            &lease.session_id,
            &lease.token,
            &job.id,
            job.revision,
            preview_at,
            "source-worker-legacy-replan-preview",
            "source-worker-legacy-replan-preview",
        )
        .unwrap();
    let accepted = engine
        .accept_attunement_replan(
            &lease.session_id,
            &lease.token,
            &preview,
            &preview.proposal_sha256,
            &attunement_context("source-worker-legacy-replan-accept"),
            preview_at + 1,
        )
        .unwrap();
    let successor = &accepted.successor.job;
    let started = attunement_call(
        engine,
        lease,
        &AttunementCommand::Start {
            job_id: successor.id.clone(),
            plan_sha256: successor.plan_sha256.clone(),
            expected_revision: successor.revision,
            lease_ms: 30_000,
        },
        "source-worker-legacy-replan-start",
        (successor.updated_at_unix_ms + 1).max(preview_at + 2),
    )
    .unwrap();
    let connected = attunement_call(
        engine,
        lease,
        &attunement_step(&started.status.job),
        "source-worker-legacy-replan-connect",
        (started.status.job.updated_at_unix_ms + 1).max(preview_at + 3),
    )
    .unwrap();
    let inventory = attunement_call(
        engine,
        lease,
        &attunement_step(&connected.status.job),
        "source-worker-legacy-replan-inventory",
        (connected.status.job.updated_at_unix_ms + 1).max(preview_at + 4),
    )
    .unwrap();
    *job = inventory.status.job;
}

fn refresh_unrelated(
    root: &Path,
    engine: &RrdEngine,
    lease: &SessionLease,
    project_id: &CanonicalId,
    job: &mut rrd_contract::AttunementJob,
    name: &str,
) -> String {
    refresh_unrelated_with_host_floor(root, engine, lease, project_id, job, name, None)
}

fn refresh_unrelated_with_host_floor(
    root: &Path,
    engine: &RrdEngine,
    lease: &SessionLease,
    project_id: &CanonicalId,
    job: &mut rrd_contract::AttunementJob,
    name: &str,
    host_floor: Option<u64>,
) -> String {
    let next_at = |minimum: u64| host_floor.unwrap_or_default().max(minimum);
    std::fs::write(root.join(name), format!("unrelated edit {name}\n")).unwrap();
    let preview_at = next_at(job.updated_at_unix_ms + 1);
    let preview = engine
        .preview_attunement_replan(
            &lease.session_id,
            &lease.token,
            &job.id,
            job.revision,
            preview_at,
            &format!("source-worker-replan-preview-{name}"),
            &format!("source-worker-replan-preview-{name}"),
        )
        .unwrap();
    let accept_at = next_at(job.updated_at_unix_ms + 2);
    let accepted = engine
        .accept_attunement_replan(
            &lease.session_id,
            &lease.token,
            &preview,
            &preview.proposal_sha256,
            &attunement_context(&format!("source-worker-replan-accept-{name}")),
            accept_at,
        )
        .unwrap();
    let successor = &accepted.successor.job;
    let start_at = next_at(successor.updated_at_unix_ms + 1);
    let started = attunement_call(
        engine,
        lease,
        &AttunementCommand::Start {
            job_id: successor.id.clone(),
            plan_sha256: successor.plan_sha256.clone(),
            expected_revision: successor.revision,
            lease_ms: 30_000,
        },
        &format!("source-worker-replan-start-{name}"),
        start_at,
    )
    .unwrap();
    let connect_at = next_at(started.status.job.updated_at_unix_ms + 1);
    let connected = attunement_call(
        engine,
        lease,
        &attunement_step(&started.status.job),
        &format!("source-worker-replan-connect-{name}"),
        connect_at,
    )
    .unwrap();
    let inventory_at = next_at(connected.status.job.updated_at_unix_ms + 1);
    let refreshed = attunement_call(
        engine,
        lease,
        &attunement_step(&connected.status.job),
        &format!("source-worker-replan-inventory-{name}"),
        inventory_at,
    )
    .unwrap();
    *job = refreshed.status.job;
    crate::engine::attunement::load_accepted_source(engine, project_id, 8 * 1024 * 1024, 10_000)
        .unwrap()
        .proposal_sha256
}

fn revoke_source_read(
    engine: &RrdEngine,
    lease: &SessionLease,
    principal_id: &CanonicalId,
    at: u64,
) {
    let repository =
        rrd_security::SecurityRepository::new(&engine.storage, engine.instance.clone());
    let mut authority = repository.load().unwrap().unwrap();
    let expected_revision = authority.revision;
    let principal = authority.principals.get_mut(principal_id).unwrap();
    let prior = principal.grants.len();
    principal
        .grants
        .retain(|grant| grant.action != SecurityAction::ProjectSourceRead);
    assert!(principal.grants.len() < prior);
    authority.revision += 1;
    engine
        .replace_security_authority(
            &lease.session_id,
            &lease.token,
            expected_revision,
            authority,
            at,
            "source-worker-revoke",
            "source-worker-revoke",
        )
        .unwrap();
}

fn restore_source_read(
    engine: &RrdEngine,
    lease: &SessionLease,
    principal_id: &CanonicalId,
    project_id: &CanonicalId,
    entry_identity: &str,
    at: u64,
) {
    let repository =
        rrd_security::SecurityRepository::new(&engine.storage, engine.instance.clone());
    let mut authority = repository.load().unwrap().unwrap();
    let expected_revision = authority.revision;
    let principal = authority.principals.get_mut(principal_id).unwrap();
    assert!(!principal
        .grants
        .iter()
        .any(|grant| grant.action == SecurityAction::ProjectSourceRead));
    principal.grants.push(ResourceGrant {
        action: SecurityAction::ProjectSourceRead,
        resource_prefix: ResourcePath {
            segments: vec![
                ResourceId::new(ResourceKind::Instance, engine.instance.as_str()).unwrap(),
                ResourceId::new(ResourceKind::Project, project_id.as_str()).unwrap(),
                ResourceId::new(ResourceKind::Record, entry_identity.to_owned()).unwrap(),
            ],
        },
        data_policy: None,
    });
    authority.revision += 1;
    engine
        .replace_security_authority(
            &lease.session_id,
            &lease.token,
            expected_revision,
            authority,
            at,
            "source-worker-source-read-restore",
            "source-worker-source-read-restore",
        )
        .unwrap();
}

fn set_worker_export_flags(
    engine: &RrdEngine,
    lease: &SessionLease,
    at: u64,
    allow_context_export: bool,
    allow_source_export: bool,
    suffix: &str,
) {
    let current = engine.load_endpoint_configuration().unwrap().1;
    let profile = current.profile.unwrap();
    let mut worker_policy = profile.worker_policy.unwrap();
    worker_policy.allow_context_export = allow_context_export;
    worker_policy.allow_source_export = allow_source_export;
    let input = EndpointProfileInput {
        contract_version: profile.contract_version,
        name: profile.name,
        bindings: profile.bindings,
        selection_policy: profile.selection_policy,
        worker_policy: Some(worker_policy),
        observation_policy: profile.observation_policy,
        assessment_policy: profile.assessment_policy,
    };
    let preview = engine
        .preview_endpoint_configuration(
            &lease.session_id,
            &lease.token,
            &PreviewEndpointConfiguration {
                expected_revision: current.revision,
                profile: Some(input),
            },
            at,
            &format!("source-worker-export-preview-{suffix}"),
            &format!("source-worker-export-preview-{suffix}"),
        )
        .unwrap();
    engine
        .apply_endpoint_configuration(
            &lease.session_id,
            &lease.token,
            &rrd_contract::ApplyEndpointConfiguration {
                approved_plan_sha256: preview.plan_sha256.clone(),
                plan: preview,
            },
            at + 1,
            &format!("source-worker-export-apply-{suffix}"),
            &format!("source-worker-export-apply-{suffix}"),
        )
        .unwrap();
}

fn attach_source_data_policy(fixture: &SourceWorkerFixture, at: u64) {
    let repository = rrd_security::SecurityRepository::new(
        &fixture.engine.storage,
        fixture.engine.instance.clone(),
    );
    let mut authority = repository.load().unwrap().unwrap();
    let expected_revision = authority.revision;
    let principal = authority.principals.get_mut(&fixture.principal_id).unwrap();
    let grant = principal
        .grants
        .iter_mut()
        .find(|grant| grant.action == SecurityAction::ProjectSourceRead)
        .unwrap();
    grant.data_policy = Some(rrd_security::DataPolicy {
        tenant: None,
        rows: Vec::new(),
        allowed_fields: Some(std::collections::BTreeSet::from(["content".into()])),
    });
    authority.revision += 1;
    fixture
        .engine
        .replace_security_authority(
            &fixture.lease.session_id,
            &fixture.lease.token,
            expected_revision,
            authority,
            at,
            "source-worker-data-policy",
            "source-worker-data-policy",
        )
        .unwrap();
}

fn inspect_budget(fixture: &SourceWorkerFixture, at: u64) -> TaskReasoningBudget {
    fixture
        .engine
        .inspect_task_reasoning_budget(
            &fixture.lease.session_id,
            &fixture.lease.token,
            &fixture.selection.scope,
            &fixture.selection.task_id,
            at,
            "source-worker-budget-inspect",
            "source-worker-budget-inspect",
        )
        .unwrap()
        .unwrap()
}

mod dispatch;
mod source_changes;
