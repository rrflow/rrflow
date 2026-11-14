//! Running the server: shutdown signals and readiness and completion files.

use super::*;

pub(super) async fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if std::env::var_os("RUST_LOG").is_some() {
        tracing_subscriber::fmt()
            .json()
            .with_env_filter(EnvFilter::from_default_env())
            .try_init()?;
    }
    let args = parse_args(std::env::args().skip(1))
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;
    let engine = RrdEngine::open_installed(&args.project, &args.distribution_executable)?;
    let tls = load_mtls(
        args.tls_certificate_file,
        args.tls_private_key_file,
        args.tls_client_ca_file,
    )?;
    let jwt = args.jwt_key_file.map(load_jwt_key).transpose()?;
    let server = match (tls, jwt) {
        (Some(tls), Some(jwt)) => RrdHttpServer::bind_mtls_with_jwt(engine, args.bind, tls, jwt)?,
        (Some(tls), None) => RrdHttpServer::bind_mtls(engine, args.bind, tls)?,
        (None, Some(jwt)) => RrdHttpServer::bind_with_jwt(engine, args.bind, jwt)?,
        (None, None) => RrdHttpServer::bind(engine, args.bind)?,
    };
    eprintln!(
        "rrd-server: {}://{}",
        if server.is_tls() { "https" } else { "http" },
        server.local_addr()
    );
    if let Some(path) = &args.ready_file {
        write_readiness(
            path,
            if server.is_tls() { "https" } else { "http" },
            server.local_addr(),
        )?;
    }
    let shutdown_request_file = args.shutdown_request_file;
    let shutdown_complete_file = args.shutdown_complete_file;
    server
        .serve_until(shutdown_signal(shutdown_request_file))
        .await?;
    if let Some(path) = shutdown_complete_file {
        write_shutdown_completion(&path)?;
    }
    Ok(())
}

pub(super) async fn shutdown_signal(request_file: Option<PathBuf>) {
    let Some(path) = request_file else {
        let _ = tokio::signal::ctrl_c().await;
        return;
    };
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {}
        _ = wait_for_shutdown_request(path) => {}
    }
}

pub(super) async fn wait_for_shutdown_request(path: PathBuf) {
    loop {
        if path.is_file() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

pub(super) fn write_shutdown_completion(path: &Path) -> io::Result<()> {
    use std::io::Write;

    let temporary = path.with_extension("complete.new");
    let _ = std::fs::remove_file(&temporary);
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    file.write_all(b"rrd-server-graceful-shutdown-v1\n")?;
    file.sync_all()?;
    std::fs::rename(&temporary, path)?;
    #[cfg(unix)]
    std::fs::File::open(path.parent().expect("shutdown marker has a parent"))?.sync_all()?;
    Ok(())
}

pub(super) fn write_readiness(path: &Path, scheme: &str, address: SocketAddr) -> io::Result<()> {
    use std::io::Write;

    let bytes = serde_json::to_vec(&serde_json::json!({
        "format": 1,
        "status": "ready",
        "url": format!("{scheme}://{address}"),
    }))?;
    let temporary = path.with_extension("ready.new");
    let _ = std::fs::remove_file(&temporary);
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    std::fs::rename(&temporary, path)?;
    #[cfg(unix)]
    std::fs::File::open(path.parent().expect("readiness marker has a parent"))?.sync_all()?;
    Ok(())
}
