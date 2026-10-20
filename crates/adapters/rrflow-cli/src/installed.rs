//! Thin installed-product adapter over the engine, server, and generated SDK.
//!
//! This module owns no lifecycle state. Plans, installed records, credentials,
//! storage, and verification remain engine-owned; the CLI only translates
//! explicit operator arguments and renders typed results.

use crate::command::{Command, Execution, InstallAction, InstallMode, RestoreAction, VerifyLevel};
use rrd_client::{ClientConfig, RequestOptions, RrdClient};
use rrd_contract::{
    CanonicalId, CloseSession, CreateSession, InstallationTargetKind, RestorePublicationPlan,
    SessionLimits, PROTOCOL, PROTOCOL_VERSION,
};
use rrd_engine::{digest, InstallationPreview, RrdEngine};
use rrd_server::RrdHttpServer;
use serde::Serialize;
use std::error::Error;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

const MAX_INSTALLATION_PLAN_BYTES: u64 = 8 * 1024 * 1024;

type BoxError = Box<dyn Error>;

#[derive(Debug, Serialize)]
struct VersionReport {
    product: &'static str,
    product_version: &'static str,
    protocol: &'static str,
    protocol_version: u16,
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

pub fn execute(command: &Command, now: u64, json: bool) -> Result<Execution, BoxError> {
    match command {
        Command::Version => version(json),
        Command::Install { action } => install(action, now, json),
        Command::Serve {
            project,
            bind,
            test_distribution_executable,
        } => serve(
            project,
            *bind,
            test_distribution_executable.as_deref(),
            json,
        ),
        Command::Ready { project, address } => ready(project, *address, now, json),
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
        Command::Restore { action } => restore(action, now, json),
    }
}

fn restore(action: &RestoreAction, now: u64, json: bool) -> Result<Execution, BoxError> {
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
            let result =
                RrdEngine::apply_restore_publication(project, &plan, expect, now, &executable)?;
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

fn install(action: &InstallAction, now: u64, json: bool) -> Result<Execution, BoxError> {
    match action {
        InstallAction::Plan {
            project,
            mode,
            profile,
            configuration,
            test_distribution_executable,
        } => {
            let started = Instant::now();
            let executable = distribution_executable_path(test_distribution_executable.as_deref())?;
            let preview = RrdEngine::plan_installation(
                project,
                target_kind(*mode),
                profile,
                configuration.as_deref(),
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
                now,
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
            let text = render(
                &result,
                json,
                format!(
                    "RRFlow installed: {}\ninstance: {}\nruntime cursor: {}\ncontrol journal: {}\nidempotent replay: {}",
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

fn serve(
    project: &Path,
    bind: std::net::SocketAddr,
    test_distribution_executable: Option<&Path>,
    json: bool,
) -> Result<Execution, BoxError> {
    let started = Instant::now();
    let executable = distribution_executable_path(test_distribution_executable)?;
    let locator = RrdEngine::read_project_locator(project)?;
    let project_root = std::fs::canonicalize(project)?.display().to_string();
    let engine = RrdEngine::open_installed(project, &executable)?;
    let server = RrdHttpServer::bind(engine, bind)?;
    let address = server.local_addr();
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
            "RRFlow listening at {}\ninstance: {}\nUI discovery: {}/v1/schema/openapi",
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
        elapsed_ms = started.elapsed().as_millis() as u64,
        "installed lifecycle operation completed"
    );

    runtime()?.block_on(server.serve_until(async {
        let _ = tokio::signal::ctrl_c().await;
    }))?;
    tracing::info!(
        target: "rrflow::installed",
        operation = "serve.closed",
        instance_id = %announcement.instance_id,
        address = %address,
        elapsed_ms = started.elapsed().as_millis() as u64,
        "installed lifecycle operation completed"
    );
    Ok(Execution {
        text: String::new(),
        success: true,
    })
}

fn ready(
    project: &Path,
    address: std::net::SocketAddr,
    now: u64,
    json: bool,
) -> Result<Execution, BoxError> {
    let started = Instant::now();
    let invocation = format!("{now}-{}", std::process::id());
    let locator = RrdEngine::read_project_locator(project)?;
    let project_root = std::fs::canonicalize(project)?.display().to_string();
    let credential = RrdEngine::read_installed_api_key(project)?;
    let client = RrdClient::connect_local(
        address,
        locator.identity.instance_id,
        ClientConfig::default(),
    )?;
    let (readiness, capabilities, endpoint_catalogue, openapi) = runtime()?.block_on(async {
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
        physical_file_count = report.physical.physical.files.len(),
        physical_bytes = report.physical.physical.total_bytes,
        elapsed_ms = started.elapsed().as_millis() as u64,
        "installed lifecycle operation completed"
    );
    let human = format!(
        "RRFlow installation verified\ninstance: {}\nplan: {}\nattunement source current: {}\nruntime cursor: {}\ncontrol journal: {}\nchecks: {} passed",
        report.instance_id,
        report.plan_sha256,
        report.attunement_source_current,
        report.runtime_cursor,
        report.control_journal_sequence,
        report.checks.len(),
    );
    Ok(Execution {
        text: render(&report, json, human)?,
        success: true,
    })
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
