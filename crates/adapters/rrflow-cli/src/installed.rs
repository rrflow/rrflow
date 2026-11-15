//! Thin installed-product adapter over the engine, server, and generated SDK.
//!
//! This module owns no lifecycle state. Plans, installed records, credentials,
//! storage, and verification remain engine-owned; the CLI only translates
//! explicit operator arguments and renders typed results.

use crate::command::{
    Command, Execution, InstallAction, InstallMode, RepairAction, RestoreAction, VerifyLevel,
};
use rrd_client::{ClientConfig, RequestOptions, RrdClient};
use rrd_contract::{
    CanonicalId, CloseSession, CreateSession, InstallationTargetKind, RestorePublicationPlan,
    SessionLimits, PROTOCOL, PROTOCOL_VERSION,
};
use rrd_engine::{
    digest, InstallationPreview, InstallationVerificationStatus, RrdEngine, ServiceError,
};
use rrd_server::RrdHttpServer;
use serde::Serialize;
use std::error::Error;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

const MAX_INSTALLATION_PLAN_BYTES: u64 = 8 * 1024 * 1024;

type BoxError = Box<dyn Error>;
mod endpoint_credentials;

#[derive(Debug, Serialize)]
struct VersionReport {
    product: &'static str,
    product_version: &'static str,
    protocol: &'static str,
    protocol_version: u16,
    installation_plan_contract_version: u16,
    release_stage: &'static str,
    build: crate::build_identity::BuildReport,
}

#[derive(Debug, Serialize)]
struct ServeAnnouncement {
    status: &'static str,
    url: String,
    project_root: String,
    instance_id: String,
    product_version: String,
    build: crate::build_identity::BuildReport,
    readiness_path: &'static str,
    capability_path: &'static str,
    endpoint_catalogue_path: &'static str,
    openapi_path: &'static str,
}

#[derive(Debug, Serialize)]
struct UiDiscoveryReport {
    status: &'static str,
    url: String,
    project_root: String,
    openapi_sha256: String,
    authentication: AuthenticatedReady,
    readiness: rrd_contract::Readiness,
    capabilities: rrd_contract::ServiceCapabilities,
    endpoint_catalogue: rrd_contract::EndpointCatalogue,
    openapi: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct AuthenticatedReady {
    principal_id: String,
    session_created: bool,
    session_closed: bool,
}

pub fn execute(command: &Command, json: bool) -> Result<Execution, BoxError> {
    match command {
        Command::Version => version(json),
        Command::Install { action } => install(action, json),
        Command::Serve {
            project,
            bind,
            endpoint_credentials,
            test_distribution_executable,
        } => serve(
            project,
            *bind,
            endpoint_credentials.as_deref(),
            test_distribution_executable.as_deref(),
            json,
        ),
        Command::Ready { project, address } => ready(project, *address, json),
        Command::Verify {
            project,
            level,
            test_distribution_executable,
        } => verify(
            project,
            *level,
            test_distribution_executable.as_deref(),
            json,
        ),
        Command::Restore { action } => restore(action, json),
        Command::Repair { action } => repair(action),
    }
}

fn repair(action: &RepairAction) -> Result<Execution, BoxError> {
    match action {
        RepairAction::Plan {
            project,
            test_distribution_executable,
        } => {
            let executable = distribution_executable_path(test_distribution_executable.as_deref())?;
            let plan = RrdEngine::plan_installed_tail_repair(project, &executable)?;
            Ok(serde_json::to_string_pretty(&plan)?.into())
        }
        RepairAction::Apply {
            project,
            plan,
            expect,
            test_distribution_executable,
        } => {
            let executable = distribution_executable_path(test_distribution_executable.as_deref())?;
            if std::fs::symlink_metadata(plan)?.file_type().is_symlink() {
                return Err("repair plan must not be a symlink".into());
            }
            let file = std::fs::File::open(plan)?;
            let metadata = file.metadata()?;
            if !metadata.is_file() || metadata.len() > MAX_INSTALLATION_PLAN_BYTES {
                return Err("repair plan must be a bounded regular file".into());
            }
            let mut bytes = Vec::with_capacity(metadata.len() as usize);
            file.take(MAX_INSTALLATION_PLAN_BYTES + 1)
                .read_to_end(&mut bytes)?;
            if bytes.len() as u64 != metadata.len() {
                return Err("repair plan changed while reading".into());
            }
            let plan: rrd_engine::InstalledTailRepairPlan = serde_json::from_slice(&bytes)?;
            let result =
                RrdEngine::apply_installed_tail_repair(project, &executable, &plan, expect)?;
            tracing::info!(target: "rrflow::installed", operation = "repair.apply",
                plan_sha256 = %result.plan_sha256, discarded_bytes = result.storage.discarded_bytes,
                idempotent_replay = result.idempotent_replay, "installed tail repair completed");
            Ok(serde_json::to_string_pretty(&result)?.into())
        }
    }
}

fn restore(action: &RestoreAction, json: bool) -> Result<Execution, BoxError> {
    match action {
        RestoreAction::Plan {
            project,
            restore,
            backup,
            test_distribution_executable,
        } => {
            let started = Instant::now();
            let executable = distribution_executable_path(test_distribution_executable.as_deref())?;
            let restore_id = CanonicalId::new(restore)?;
            let plan =
                RrdEngine::plan_restore_publication(project, &restore_id, backup, &executable)?;
            tracing::info!(
                target: "rrflow::installed",
                operation = "restore.plan",
                plan_sha256 = %plan.plan_sha256,
                restore_id = %plan.restore_id,
                backup_sha256 = %plan.backup_sha256,
                elapsed_ms = started.elapsed().as_millis() as u64,
                "installed lifecycle operation completed"
            );
            // Like an installation plan, a publication plan is the exact JSON
            // document `restore apply` accepts, whatever the rendering flag.
            Ok(serde_json::to_string_pretty(&plan)?.into())
        }
        RestoreAction::Apply {
            project,
            plan,
            expect,
            test_distribution_executable,
        } => {
            let started = Instant::now();
            let executable = distribution_executable_path(test_distribution_executable.as_deref())?;
            let plan = read_publication_plan(plan)?;
            let result = RrdEngine::apply_restore_publication(
                project,
                &plan,
                expect,
                wall_clock_millis()?,
                &executable,
            )?;
            tracing::info!(
                target: "rrflow::installed",
                operation = "restore.apply",
                plan_sha256 = %result.plan_sha256,
                runtime_cursor = result.runtime_cursor,
                control_journal_sequence = result.control_journal_sequence,
                idempotent_replay = result.idempotent_replay,
                elapsed_ms = started.elapsed().as_millis() as u64,
                "installed lifecycle operation completed"
            );
            let text = render(
                &result,
                json,
                format!(
                    "RRFlow restore published: {}\npublished root: {}\nquarantined root: {}\nruntime cursor: {}\ncontrol journal: {}\nidempotent replay: {}",
                    result.plan_sha256,
                    result.published_root,
                    result.quarantine_root,
                    result.runtime_cursor,
                    result.control_journal_sequence,
                    result.idempotent_replay,
                ),
            )?;
            Ok(Execution {
                text,
                success: true,
            })
        }
    }
}

fn version(json: bool) -> Result<Execution, BoxError> {
    let report = VersionReport {
        product: "RRFlow",
        product_version: env!("CARGO_PKG_VERSION"),
        protocol: PROTOCOL,
        protocol_version: PROTOCOL_VERSION,
        installation_plan_contract_version: rrd_contract::INSTALLATION_PLAN_CONTRACT_VERSION,
        release_stage: "pre-release",
        build: crate::build_identity::report()?,
    };
    Ok(render(
        &report,
        json,
        format!(
            "RRFlow {} (pre-release; {} protocol v{})",
            report.product_version, report.protocol, report.protocol_version
        ),
    )?
    .into())
}

fn install(action: &InstallAction, json: bool) -> Result<Execution, BoxError> {
    match action {
        InstallAction::Identity => Ok(serde_json::to_string_pretty(&serde_json::json!({
            "installation_identity": RrdEngine::generate_installation_identity()?,
        }))?
        .into()),
        InstallAction::AttunementSchema => {
            Ok(serde_json::to_string_pretty(&rrd_engine::attunement_proposal_schema()?)?.into())
        }
        InstallAction::Discover { project, profile } => {
            let discovery = RrdEngine::discover_nested_repositories(project, profile)?;
            Ok(serde_json::to_string_pretty(&discovery)?.into())
        }
        InstallAction::Plan {
            project,
            mode,
            identity,
            profile,
            configuration,
            attunement_proposal,
            allow_project_source_read,
            follow_interval_ms,
            test_distribution_executable,
        } => {
            let started = Instant::now();
            let executable = distribution_executable_path(test_distribution_executable.as_deref())?;
            let proposal = attunement_proposal
                .as_deref()
                .map(rrd_engine::load_attunement_proposal)
                .transpose()?;
            let preview = RrdEngine::plan_installation_with_options(
                project,
                target_kind(*mode),
                identity,
                profile,
                configuration.as_deref(),
                rrd_engine::InstallationPlanningOptions {
                    attunement_proposal: proposal,
                    allow_project_source_read: *allow_project_source_read,
                    attunement_follow: follow_interval_ms
                        .map(|interval_ms| rrd_contract::AttunementFollowPolicy { interval_ms }),
                },
                &executable,
            )?;
            tracing::info!(
                target: "rrflow::installed",
                operation = "install.plan",
                plan_sha256 = %preview.installation.plan_sha256,
                profile_sha256 = %preview.installation.profile_sha256,
                action_count = preview.installation.actions.len(),
                managed_path_count = preview.installation.managed_paths.len(),
                elapsed_ms = started.elapsed().as_millis() as u64,
                "installed lifecycle operation completed"
            );
            // A plan is an executable machine contract even when the operator
            // did not request global JSON rendering, so stdout is always the
            // exact JSON document accepted by `install apply`.
            Ok(serde_json::to_string_pretty(&preview)?.into())
        }
        InstallAction::Apply {
            project,
            mode,
            plan,
            expect,
            test_distribution_executable,
        } => {
            let started = Instant::now();
            let executable = distribution_executable_path(test_distribution_executable.as_deref())?;
            let preview = read_installation_plan(plan)?;
            let result = RrdEngine::apply_installation(
                project,
                target_kind(*mode),
                &preview,
                expect,
                &executable,
            )?;
            tracing::info!(
                target: "rrflow::installed",
                operation = "install.apply",
                plan_sha256 = %result.plan_sha256,
                runtime_cursor = result.runtime_cursor,
                control_journal_sequence = result.control_journal_sequence,
                idempotent_replay = result.idempotent_replay,
                elapsed_ms = started.elapsed().as_millis() as u64,
                "installed lifecycle operation completed"
            );
            let map = project_map_line(project);
            let text = render(
                &result,
                json,
                format!(
                    "RRFlow installed: {}\ninstance: {}\nruntime cursor: {}\ncontrol journal: {}\nidempotent replay: {}\n{map}",
                    result.plan_sha256,
                    preview.installation.target.instance_id,
                    result.runtime_cursor,
                    result.control_journal_sequence,
                    result.idempotent_replay,
                ),
            )?;
            Ok(Execution {
                text,
                success: true,
            })
        }
    }
}

/// Refresh the project's map and describe the outcome in one line. A map is
/// a replaceable projection, so a failure is reported and never blocks.
fn project_map_line(project: &Path) -> String {
    match RrdEngine::refresh_project_map(project) {
        Ok(map) => format!(
            "project map: {} ({} files{})",
            map.path.display(),
            map.files,
            if map.changed { ", updated" } else { "" }
        ),
        Err(error) => {
            tracing::warn!(
                target: "rrflow::installed",
                operation = "project_map.refresh",
                error = %error,
                "project map was not refreshed"
            );
            format!("project map: not refreshed ({error})")
        }
    }
}

fn serve(
    project: &Path,
    bind: std::net::SocketAddr,
    endpoint_credentials: Option<&Path>,
    test_distribution_executable: Option<&Path>,
    json: bool,
) -> Result<Execution, BoxError> {
    let started = Instant::now();
    let executable = distribution_executable_path(test_distribution_executable)?;
    let locator = RrdEngine::read_project_locator(project)?;
    let project_root = std::fs::canonicalize(project)?.display().to_string();
    #[cfg(feature = "crash-test")]
    rrd_engine::crash_test::configure_from_stdin(&project.join(&locator.storage_root))?;
    let mut engine = RrdEngine::open_installed(project, &executable)
        .map_err(|error| engram_open_elsewhere(project, error))?;
    if let Some(path) = endpoint_credentials {
        engine.install_endpoint_credentials(endpoint_credentials::load(path)?)?;
    }
    // Refreshed while this engine holds the writer lock; the map is a
    // replaceable view, so its outcome is reported and never stops serving.
    let map = project_map_line(project);
    // Issued while this engine holds the engram's writer lock, so a record
    // found at publication belongs to a process that no longer runs.
    let publisher = engine.serving_publisher()?;
    let server = RrdHttpServer::bind(engine, bind)?;
    let address = server.local_addr();
    let registration = publisher.publish(address)?;
    let announcement = ServeAnnouncement {
        status: "listening",
        url: format!("http://{address}"),
        project_root,
        instance_id: locator.identity.instance_id.to_string(),
        product_version: locator.product_version,
        build: crate::build_identity::report()?,
        readiness_path: "/v1/health/ready",
        capability_path: "/v1/capabilities",
        endpoint_catalogue_path: "/v1/schema/endpoints",
        openapi_path: "/v1/schema/openapi",
    };
    let text = if json {
        // One line is a deliberate process-supervision contract: callers can
        // parse readiness without guessing where a pretty document ends.
        serde_json::to_string(&announcement)?
    } else {
        format!(
            "RRFlow listening at {}\ninstance: {}\nUI discovery: {}/v1/schema/openapi\n{map}",
            announcement.url, announcement.instance_id, announcement.url
        )
    };
    println!("{text}");
    std::io::stdout().flush()?;
    tracing::info!(
        target: "rrflow::installed",
        operation = "serve.listening",
        instance_id = %announcement.instance_id,
        address = %address,
        serving_record = %registration.record().url,
        elapsed_ms = started.elapsed().as_millis() as u64,
        "installed lifecycle operation completed"
    );

    let served = (|| -> Result<(), BoxError> {
        runtime()?.block_on(server.serve_until(async {
            let _ = tokio::signal::ctrl_c().await;
        }))?;
        Ok(())
    })();
    let withdrawn = registration.withdraw();
    tracing::info!(
        target: "rrflow::installed",
        operation = "serve.closed",
        instance_id = %announcement.instance_id,
        address = %address,
        serving_record_withdrawn = withdrawn.is_ok(),
        elapsed_ms = started.elapsed().as_millis() as u64,
        "installed lifecycle operation completed"
    );
    served?;
    withdrawn?;
    Ok(Execution {
        text: String::new(),
        success: true,
    })
}

/// Another process holds the engram, so this one fails before binding. Name
/// the endpoint it recorded, if any; only `rrflow ready` can prove it.
fn engram_open_elsewhere(project: &Path, error: ServiceError) -> BoxError {
    if !matches!(error, ServiceError::EngramInUse(_)) {
        return error.into();
    }
    match RrdEngine::read_serving_record(project) {
        Ok(Some(record)) => format!(
            "this project's RRFlow engram is already open in another process; its served endpoint is recorded at {}. Run `rrflow ready --project` to prove it.",
            record.url
        )
        .into(),
        _ => "this project's RRFlow engram is already open in another process".into(),
    }
}

fn ready(
    project: &Path,
    address: Option<std::net::SocketAddr>,
    json: bool,
) -> Result<Execution, BoxError> {
    let started = Instant::now();
    let now = wall_clock_millis()?;
    let invocation = format!("{now}-{}", std::process::id());
    let locator = RrdEngine::read_project_locator(project)?;
    let (address, recorded) = match address {
        Some(address) => (address, false),
        None => {
            let record = RrdEngine::read_serving_record(project)?.ok_or(
                "no served endpoint is recorded for this project; start it with `rrflow serve`",
            )?;
            (record.address()?, true)
        }
    };
    let project_root = std::fs::canonicalize(project)?.display().to_string();
    let credential = RrdEngine::read_installed_api_key(project)?;
    let client = RrdClient::connect_local(
        address,
        locator.identity.instance_id,
        ClientConfig::default(),
    )?;
    let proof = runtime()?.block_on(async {
        let session = client
            .create_session(
                credential.principal_id.clone(),
                credential.credential(),
                CreateSession {
                    limits: SessionLimits {
                        idle_timeout_ms: 30_000,
                        absolute_timeout_ms: 60_000,
                        max_open_transactions: 1,
                    },
                },
                RequestOptions::mutation(
                    &format!("ready-session-request-{invocation}"),
                    &format!("ready-session-operation-{invocation}"),
                    &format!("ready-session-idempotency-{invocation}"),
                )?,
            )
            .await?;
        let discovery = async {
            Ok::<_, rrd_client::Error>((
                client.readiness().await?,
                client.capabilities().await?,
                client.endpoint_catalogue().await?,
                client.openapi_document().await?,
            ))
        }
        .await;
        let closed = client
            .close_session(
                &session,
                CloseSession {},
                RequestOptions::mutation(
                    &format!("ready-close-request-{invocation}"),
                    &format!("ready-close-operation-{invocation}"),
                    &format!("ready-close-idempotency-{invocation}"),
                )?,
            )
            .await;
        let discovery = discovery?;
        closed?;
        Ok::<_, rrd_client::Error>(discovery)
    });
    let (readiness, capabilities, endpoint_catalogue, openapi) =
        proof.map_err(|error| -> BoxError {
            if recorded {
                format!("the served endpoint recorded at http://{address} is not ready: {error}")
                    .into()
            } else {
                error.into()
            }
        })?;
    let openapi_sha256 = digest::sha256_hex(&serde_json::to_vec(&openapi)?);
    let report = UiDiscoveryReport {
        status: "ready",
        url: format!("http://{address}"),
        project_root,
        openapi_sha256,
        authentication: AuthenticatedReady {
            principal_id: credential.principal_id.to_string(),
            session_created: true,
            session_closed: true,
        },
        readiness,
        capabilities,
        endpoint_catalogue,
        openapi,
    };
    tracing::info!(
        target: "rrflow::installed",
        operation = "ready",
        instance_id = %report.capabilities.instance.id,
        runtime_cursor = report.readiness.runtime_cursor,
        endpoint_count = report.endpoint_catalogue.endpoints.len(),
        websocket_endpoint_count = report.endpoint_catalogue.websocket_endpoints.len(),
        openapi_sha256 = %report.openapi_sha256,
        elapsed_ms = started.elapsed().as_millis() as u64,
        "installed lifecycle operation completed"
    );
    let human = format!(
        "RRFlow ready at {}\ninstance: {}\nruntime cursor: {}\nHTTP operations: {}\nWebSocket operations: {}\nOpenAPI: {}",
        report.url,
        report.capabilities.instance.id,
        report.readiness.runtime_cursor,
        report.endpoint_catalogue.endpoints.len(),
        report.endpoint_catalogue.websocket_endpoints.len(),
        report.openapi_sha256,
    );
    Ok(Execution {
        text: render(&report, json, human)?,
        success: true,
    })
}

fn verify(
    project: &Path,
    level: VerifyLevel,
    distribution_executable: Option<&Path>,
    json: bool,
) -> Result<Execution, BoxError> {
    let started = Instant::now();
    let executable = distribution_executable_path(distribution_executable)?;
    let report = match level {
        VerifyLevel::Quick => RrdEngine::inspect_installed(project, &executable)?,
    };
    tracing::info!(
        target: "rrflow::installed",
        operation = "verify.quick",
        instance_id = %report.instance_id,
        plan_sha256 = %report.plan_sha256,
        runtime_cursor = report.runtime_cursor,
        control_journal_sequence = report.control_journal_sequence,
        attunement_source_current = report.attunement_source_current,
        clock_status = ?report.clock.status,
        clock_observed_at_unix_ms = report
            .clock
            .observation
            .as_ref()
            .map(|observation| observation.observed_at_unix_ms),
        clock_anchor_unix_ms = report.clock.anchor.observed_at_unix_ms,
        clock_rollback_ms = report.clock.rollback_ms,
        clock_maximum_rollback_ms = report.clock.maximum_rollback_ms,
        physical_file_count = report.physical.physical.files.len(),
        physical_bytes = report.physical.physical.total_bytes,
        elapsed_ms = started.elapsed().as_millis() as u64,
        "installed lifecycle operation completed"
    );
    let passed_checks = report.checks.iter().filter(|check| check.passed).count();
    let success = report.status == InstallationVerificationStatus::Passed;
    let human = format!(
        "RRFlow installation verification: {:?}\ninstance: {}\nplan: {}\nattunement source current: {}\nclock: {:?}\nclock anchor: {}\nclock rollback: {:?} ms\nruntime cursor: {}\ncontrol journal: {}\nchecks: {}/{} passed",
        report.status,
        report.instance_id,
        report.plan_sha256,
        report.attunement_source_current,
        report.clock.status,
        report.clock.anchor.observed_at_unix_ms,
        report.clock.rollback_ms,
        report.runtime_cursor,
        report.control_journal_sequence,
        passed_checks,
        report.checks.len(),
    );
    Ok(Execution {
        text: render(&report, json, human)?,
        success,
    })
}

fn wall_clock_millis() -> Result<u64, BoxError> {
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| std::io::Error::other("host clock is before the Unix epoch"))?;
    let millis = u64::try_from(elapsed.as_millis())
        .map_err(|_| std::io::Error::other("host clock is outside RRFlow's range"))?;
    if millis == 0 {
        return Err(std::io::Error::other("host clock is at the Unix epoch").into());
    }
    Ok(millis)
}

fn read_installation_plan(path: &Path) -> Result<InstallationPreview, BoxError> {
    let metadata = std::fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || metadata.len() > MAX_INSTALLATION_PLAN_BYTES
    {
        return Err("installation plan must be a bounded regular file, not a symlink".into());
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    std::fs::File::open(path)?
        .take(MAX_INSTALLATION_PLAN_BYTES + 1)
        .read_to_end(&mut bytes)?;
    let preview: InstallationPreview = serde_json::from_slice(&bytes)?;
    preview.validate()?;
    Ok(preview)
}

fn read_publication_plan(path: &Path) -> Result<RestorePublicationPlan, BoxError> {
    let metadata = std::fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || metadata.len() > MAX_INSTALLATION_PLAN_BYTES
    {
        return Err(
            "restore publication plan must be a bounded regular file, not a symlink".into(),
        );
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    std::fs::File::open(path)?
        .take(MAX_INSTALLATION_PLAN_BYTES + 1)
        .read_to_end(&mut bytes)?;
    let plan: RestorePublicationPlan = serde_json::from_slice(&bytes)?;
    plan.validate()?;
    Ok(plan)
}

fn target_kind(mode: InstallMode) -> InstallationTargetKind {
    match mode {
        InstallMode::Fresh => InstallationTargetKind::FreshProject,
        InstallMode::Existing => InstallationTargetKind::ExistingProject,
    }
}

fn distribution_executable_path(path: Option<&Path>) -> Result<PathBuf, BoxError> {
    if let Some(path) = path {
        if !cfg!(debug_assertions) {
            return Err("the test distribution surrogate is disabled in release builds".into());
        }
        return Ok(path.to_owned());
    }
    std::env::current_exe().map_err(Into::into)
}

fn runtime() -> Result<tokio::runtime::Runtime, BoxError> {
    Ok(tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?)
}

fn render<T: Serialize>(value: &T, json: bool, human: String) -> Result<String, BoxError> {
    if json {
        Ok(serde_json::to_string_pretty(value)?)
    } else {
        Ok(human)
    }
}
