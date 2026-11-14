//! Preparation runs, argument resolution, validation, logs and platform environment.

use super::*;

pub(super) fn run_preparation(
    request: &DriverRequest,
    deployment: &LocalDeployment,
    instance_root: &Path,
    stdout: &File,
    stderr: &File,
    stderr_path: &Path,
) -> std::result::Result<(), DriverError> {
    if deployment.preparation_arguments.is_empty() {
        return Ok(());
    }
    let arguments = deployment
        .preparation_arguments
        .iter()
        .map(|argument| resolve_argument(argument, request, instance_root))
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let stdout = stdout
        .try_clone()
        .map_err(|error| retryable(request, format!("cannot clone preparation stdout: {error}")))?;
    let stderr = stderr
        .try_clone()
        .map_err(|error| retryable(request, format!("cannot clone preparation stderr: {error}")))?;
    let mut command = Command::new(&deployment.executable);
    command
        .args(arguments)
        .current_dir(instance_root)
        .env_clear()
        .envs(&deployment.environment)
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr));
    configure_platform_environment(&mut command, request)?;
    let launch = rrd_core::child_launch_lock();
    let spawned = command.spawn();
    drop(launch);
    let mut child = spawned.map_err(|error| {
        retryable(
            request,
            format!("cannot run deployment preparation: {error}"),
        )
    })?;
    let deadline = Instant::now() + PROCESS_PREPARATION_TIMEOUT;
    loop {
        if let Some(status) = child.try_wait().map_err(|error| {
            retryable(
                request,
                format!("cannot inspect deployment preparation: {error}"),
            )
        })? {
            return if status.success() {
                Ok(())
            } else {
                Err(retryable(
                    request,
                    format!(
                        "deployment preparation failed: {}",
                        spawned_exit_message(status, stderr_path)
                    ),
                ))
            };
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(retryable(
                request,
                "deployment preparation exceeded its ten-second deadline",
            ));
        }
        thread::sleep(Duration::from_millis(10));
    }
}

pub(super) fn resolve_argument(
    argument: &LocalArgument,
    request: &DriverRequest,
    instance_root: &Path,
) -> std::result::Result<String, DriverError> {
    match argument {
        LocalArgument::Literal(value) => Ok(value.clone()),
        LocalArgument::InstanceId => Ok(request.instance_id.to_string()),
        LocalArgument::InstanceRoot => path_argument(instance_root, request),
        LocalArgument::InstancePath(relative) => {
            path_argument(&instance_root.join(relative), request)
        }
        LocalArgument::DesiredVersion => Ok(request.desired.version.clone()),
        LocalArgument::ConfigurationSha256 => Ok(request.desired.configuration_sha256.clone()),
    }
}

pub(super) fn path_argument(
    path: &Path,
    request: &DriverRequest,
) -> std::result::Result<String, DriverError> {
    path.to_str()
        .map(str::to_owned)
        .ok_or_else(|| permanent(request, "instance path is not valid UTF-8"))
}

pub(super) fn validate_argument(value: &str) -> std::result::Result<(), String> {
    if value.len() > 4_096 || value.as_bytes().contains(&0) {
        return Err("local deployment argument is invalid".into());
    }
    Ok(())
}

pub(super) fn validate_relative_path(path: &Path) -> std::result::Result<(), String> {
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_) | Component::CurDir))
    {
        return Err("instance-relative argument path is invalid".into());
    }
    Ok(())
}

pub(super) fn validate_control_filename(path: &Path) -> std::result::Result<(), String> {
    let mut components = path.components();
    if !matches!(components.next(), Some(Component::Normal(_))) || components.next().is_some() {
        return Err("process control paths must be direct instance filenames".into());
    }
    Ok(())
}

pub(super) fn validate_sha256(value: &str) -> std::result::Result<(), String> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        return Err("local deployment executable SHA-256 is invalid".into());
    }
    Ok(())
}

pub(super) fn file_sha256(path: &Path) -> std::io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    let bytes = hasher.finalize();
    let mut encoded = String::with_capacity(bytes.len() * 2);
    const HEX: &[u8; 16] = b"0123456789abcdef";
    for byte in bytes {
        encoded.push(HEX[usize::from(byte >> 4)] as char);
        encoded.push(HEX[usize::from(byte & 0x0f)] as char);
    }
    Ok(encoded)
}

pub(super) fn open_process_log(path: &Path) -> std::io::Result<File> {
    let mut options = OpenOptions::new();
    options.write(true).create(true).append(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path)
}

#[cfg(windows)]
pub(super) fn configure_platform_environment(
    command: &mut Command,
    request: &DriverRequest,
) -> std::result::Result<(), DriverError> {
    let system_root = std::env::var_os("SystemRoot").ok_or_else(|| {
        permanent(
            request,
            "Windows managed process launch requires the SystemRoot platform variable",
        )
    })?;
    command.env("SystemRoot", system_root);
    Ok(())
}

#[cfg(not(windows))]
pub(super) fn configure_platform_environment(
    _command: &mut Command,
    _request: &DriverRequest,
) -> std::result::Result<(), DriverError> {
    Ok(())
}

pub(super) fn spawned_exit_message(status: std::process::ExitStatus, stderr_path: &Path) -> String {
    format!(
        "spawned process exited during startup ({status}): {}",
        diagnostic_stderr(stderr_path)
    )
}

pub(super) fn diagnostic_stderr(stderr_path: &Path) -> String {
    std::fs::read(stderr_path)
        .ok()
        .map(|bytes| {
            let start = bytes.len().saturating_sub(4_096);
            String::from_utf8_lossy(&bytes[start..]).trim().to_owned()
        })
        .filter(|stderr| !stderr.is_empty())
        .unwrap_or_else(|| "managed process emitted no stderr".into())
}

pub(super) fn system_for(pid: u32) -> System {
    let mut system = System::new();
    system.refresh_processes_specifics(
        ProcessesToUpdate::Some(&[Pid::from_u32(pid)]),
        true,
        ProcessRefreshKind::nothing()
            .without_tasks()
            .with_exe(UpdateKind::Always),
    );
    system
}

pub(super) fn prepare_shutdown_artifacts(
    instance_root: &Path,
    shutdown: &LocalShutdown,
) -> std::io::Result<()> {
    let LocalShutdown::RequestFile {
        request, complete, ..
    } = shutdown
    else {
        return Ok(());
    };
    for relative in [request, complete] {
        let path = instance_root.join(relative);
        match std::fs::remove_file(&path) {
            Ok(()) => sync_parent(&path)?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

pub(super) fn prepare_readiness_artifact(
    instance_root: &Path,
    readiness: &LocalReadiness,
) -> std::io::Result<()> {
    let LocalReadiness::File { path, .. } = readiness else {
        return Ok(());
    };
    let path = instance_root.join(path);
    match std::fs::remove_file(&path) {
        Ok(()) => sync_parent(&path),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}
