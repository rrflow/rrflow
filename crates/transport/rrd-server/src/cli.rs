//! Command-line arguments, mutual TLS and JWT key loading.

use super::*;

pub(super) fn parse_args(arguments: impl Iterator<Item = String>) -> Result<Args, String> {
    let mut project = None;
    let mut distribution_executable = None;
    let mut bind = "127.0.0.1:9477".parse().expect("static bind address");
    let mut jwt_key_file = None;
    let mut ready_file = None;
    let mut shutdown_request_file = None;
    let mut shutdown_complete_file = None;
    let mut tls_certificate_file = None;
    let mut tls_private_key_file = None;
    let mut tls_client_ca_file = None;
    let mut arguments = arguments;
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--project" => {
                project = Some(PathBuf::from(required_value(&mut arguments, "--project")?));
            }
            "--distribution-executable" => {
                distribution_executable = Some(PathBuf::from(required_value(
                    &mut arguments,
                    "--distribution-executable",
                )?));
            }
            "--bind" => {
                let value = required_value(&mut arguments, "--bind")?;
                bind = value
                    .parse()
                    .map_err(|error| format!("invalid --bind {value:?}: {error}"))?;
            }
            "--jwt-key-file" => {
                jwt_key_file = Some(PathBuf::from(required_value(
                    &mut arguments,
                    "--jwt-key-file",
                )?));
            }
            "--ready-file" => {
                ready_file = Some(PathBuf::from(required_value(
                    &mut arguments,
                    "--ready-file",
                )?));
            }
            "--shutdown-request-file" => {
                shutdown_request_file = Some(PathBuf::from(required_value(
                    &mut arguments,
                    "--shutdown-request-file",
                )?));
            }
            "--shutdown-complete-file" => {
                shutdown_complete_file = Some(PathBuf::from(required_value(
                    &mut arguments,
                    "--shutdown-complete-file",
                )?));
            }
            "--tls-cert" => {
                tls_certificate_file =
                    Some(PathBuf::from(required_value(&mut arguments, "--tls-cert")?));
            }
            "--tls-key" => {
                tls_private_key_file =
                    Some(PathBuf::from(required_value(&mut arguments, "--tls-key")?));
            }
            "--tls-client-ca" => {
                tls_client_ca_file = Some(PathBuf::from(required_value(
                    &mut arguments,
                    "--tls-client-ca",
                )?));
            }
            "--help" | "-h" => return Err(usage().into()),
            value => return Err(format!("unknown argument {value:?}\n{}", usage())),
        }
    }
    if shutdown_request_file.is_some() != shutdown_complete_file.is_some() {
        return Err(format!(
            "--shutdown-request-file and --shutdown-complete-file must be provided together\n{}",
            usage()
        ));
    }
    let tls_file_count = [
        tls_certificate_file.is_some(),
        tls_private_key_file.is_some(),
        tls_client_ca_file.is_some(),
    ]
    .into_iter()
    .filter(|present| *present)
    .count();
    if tls_file_count != 0 && tls_file_count != 3 {
        return Err(format!(
            "--tls-cert, --tls-key, and --tls-client-ca must be provided together\n{}",
            usage()
        ));
    }
    if let (Some(request), Some(complete)) = (&shutdown_request_file, &shutdown_complete_file) {
        if !request.is_absolute() || !complete.is_absolute() || request == complete {
            return Err("shutdown control files must be distinct absolute paths".into());
        }
    }
    if let Some(ready) = &ready_file {
        if !ready.is_absolute()
            || shutdown_request_file.as_ref() == Some(ready)
            || shutdown_complete_file.as_ref() == Some(ready)
        {
            return Err(
                "readiness file must be absolute and distinct from shutdown control files".into(),
            );
        }
    }
    Ok(Args {
        project: project.ok_or_else(|| format!("--project is required\n{}", usage()))?,
        distribution_executable: distribution_executable
            .ok_or_else(|| format!("--distribution-executable is required\n{}", usage()))?,
        bind,
        jwt_key_file,
        ready_file,
        shutdown_request_file,
        shutdown_complete_file,
        tls_certificate_file,
        tls_private_key_file,
        tls_client_ca_file,
    })
}

pub(super) fn load_mtls(
    certificate_file: Option<PathBuf>,
    private_key_file: Option<PathBuf>,
    client_ca_file: Option<PathBuf>,
) -> Result<Option<RrdMutualTlsServerConfig>, Box<dyn std::error::Error + Send + Sync>> {
    let (certificate_file, private_key_file, client_ca_file) =
        match (certificate_file, private_key_file, client_ca_file) {
            (None, None, None) => return Ok(None),
            (Some(certificate), Some(key), Some(ca)) => (certificate, key, ca),
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "--tls-cert, --tls-key, and --tls-client-ca are required together",
                )
                .into());
            }
        };
    let certificate_chain =
        rustls_pemfile::certs(&mut BufReader::new(File::open(certificate_file)?))
            .collect::<std::result::Result<Vec<_>, _>>()?;
    let private_key =
        rustls_pemfile::private_key(&mut BufReader::new(File::open(private_key_file)?))?
            .ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "TLS private key is absent")
            })?;
    let mut client_roots = RootCertStore::empty();
    for certificate in rustls_pemfile::certs(&mut BufReader::new(File::open(client_ca_file)?)) {
        client_roots
            .add(certificate?)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    }
    Ok(Some(RrdMutualTlsServerConfig::new(
        certificate_chain,
        private_key,
        client_roots,
    )?))
}

pub(super) fn load_jwt_key(
    path: PathBuf,
) -> Result<RrdJwtVerificationKey, Box<dyn std::error::Error + Send + Sync>> {
    let metadata = std::fs::metadata(&path)?;
    if !metadata.is_file() || metadata.len() > 64 * 1024 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "JWT key must be a bounded regular file",
        )
        .into());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o077 != 0 {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "JWT key file must be owner-only",
            )
            .into());
        }
    }
    RrdJwtVerificationKey::new(std::fs::read(path)?).map_err(Into::into)
}

pub(super) fn required_value(
    arguments: &mut impl Iterator<Item = String>,
    option: &str,
) -> Result<String, String> {
    arguments
        .next()
        .ok_or_else(|| format!("{option} requires a value"))
}

pub(super) fn usage() -> &'static str {
    "usage: rrd-server --project PROJECT --distribution-executable RRFLOW [--bind 127.0.0.1:9477] [--jwt-key-file PATH] [--ready-file PATH] [--tls-cert PATH --tls-key PATH --tls-client-ca PATH] [--shutdown-request-file PATH --shutdown-complete-file PATH]"
}

#[cfg(test)]
mod tests;
