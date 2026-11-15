//! Readiness, shutdown, owned exit, file identity, records and outcomes.

use super::*;

pub(super) fn wait_for_readiness(
    request: &DriverRequest,
    deployment: &LocalDeployment,
    instance_root: &Path,
    stderr_path: &Path,
    child: &mut std::process::Child,
) -> std::result::Result<(), DriverError> {
    let LocalReadiness::File { path, timeout_ms } = &deployment.readiness else {
        return Ok(());
    };
    let path = instance_root.join(path);
    let deadline = Instant::now() + Duration::from_millis(*timeout_ms);
    loop {
        match std::fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.file_type().is_file() => return Ok(()),
            Ok(_) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(permanent(
                    request,
                    "managed process readiness artifact is not a direct file",
                ));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(retryable(
                    request,
                    format!("cannot inspect managed process readiness: {error}"),
                ));
            }
        }
        if let Some(status) = child.try_wait().map_err(|error| {
            retryable(
                request,
                format!("cannot inspect process before readiness: {error}"),
            )
        })? {
            return Err(retryable(
                request,
                spawned_exit_message(status, stderr_path),
            ));
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(retryable(
                request,
                format!(
                    "managed process did not publish readiness within {timeout_ms} ms: {}",
                    diagnostic_stderr(stderr_path)
                ),
            ));
        }
        thread::sleep(Duration::from_millis(10));
    }
}

pub(super) fn write_shutdown_request(
    path: &Path,
    request: &DriverRequest,
    record: &ProcessRecord,
) -> std::io::Result<()> {
    let bytes = serde_json::to_vec(&serde_json::json!({
        "format": 1,
        "instance_id": request.instance_id,
        "operation_id": request.operation_id,
        "pid": record.pid,
        "process_start_time_unix_s": record.process_start_time_unix_s,
    }))
    .map_err(std::io::Error::other)?;
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    match options.open(path) {
        Ok(mut file) => {
            file.write_all(&bytes)?;
            file.sync_all()?;
            sync_parent(path)
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
        Err(error) => Err(error),
    }
}

pub(super) fn wait_for_owned_exit(
    record: &ProcessRecord,
    timeout: Duration,
    request: &DriverRequest,
) -> std::result::Result<bool, DriverError> {
    let deadline = Instant::now() + timeout;
    loop {
        let system = system_for(record.pid);
        match system.process(Pid::from_u32(record.pid)) {
            None => return Ok(true),
            Some(process)
                if matches!(
                    process.status(),
                    ProcessStatus::Zombie | ProcessStatus::Dead
                ) =>
            {
                let _ = process.wait();
                return Ok(true);
            }
            Some(process) => {
                if process.start_time() != record.process_start_time_unix_s {
                    // The recorded process has exited and its PID has already
                    // been reused. Never inspect or signal the replacement.
                    // Request-file shutdown still verifies the original
                    // process's completion marker in `stop` before accepting
                    // this as a graceful exit.
                    return Ok(true);
                }
                match process
                    .exe()
                    .and_then(|path| std::fs::canonicalize(path).ok())
                {
                    Some(executable) if !same_executable_file(&executable, &record.executable) => {
                        return Err(permanent(
                            request,
                            "refusing fallback kill after managed PID executable identity changed",
                        ));
                    }
                    Some(_) => {
                        if Instant::now() >= deadline {
                            return Ok(false);
                        }
                    }
                    None => {
                        if Instant::now() >= deadline {
                            return Err(permanent(
                                request,
                                "refusing fallback kill because managed PID executable identity could not be authenticated",
                            ));
                        }
                    }
                }
            }
        }
        thread::sleep(Duration::from_millis(10));
    }
}

pub(super) fn kill_owned_process(
    record: &ProcessRecord,
    request: &DriverRequest,
) -> std::result::Result<(), DriverError> {
    let system = system_for(record.pid);
    let Some(process) = system.process(Pid::from_u32(record.pid)) else {
        return Ok(());
    };
    let executable = process
        .exe()
        .and_then(|path| std::fs::canonicalize(path).ok());
    if process.start_time() != record.process_start_time_unix_s
        || executable
            .as_deref()
            .is_none_or(|path| !same_executable_file(path, &record.executable))
    {
        return Err(permanent(
            request,
            "refusing to kill a PID whose executable/start identity changed",
        ));
    }
    if !process.kill() {
        return Err(retryable(request, "operating system rejected process kill"));
    }
    if !wait_for_owned_exit(record, PROCESS_STOP_TIMEOUT, request)? {
        return Err(retryable(request, "process did not exit after kill"));
    }
    Ok(())
}

pub(super) fn same_executable_file(observed: &Path, expected: &Path) -> bool {
    if observed == expected {
        return true;
    }
    same_platform_file(observed, expected)
}

#[cfg(unix)]
pub(super) fn same_platform_file(observed: &Path, expected: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;

    let Ok(observed) = std::fs::metadata(observed) else {
        return false;
    };
    let Ok(expected) = std::fs::metadata(expected) else {
        return false;
    };
    observed.dev() == expected.dev() && observed.ino() == expected.ino()
}

#[cfg(windows)]
pub(super) fn same_platform_file(observed: &Path, expected: &Path) -> bool {
    let Ok(observed) = windows_file_identity(observed) else {
        return false;
    };
    let Ok(expected) = windows_file_identity(expected) else {
        return false;
    };
    observed == expected
}

#[cfg(windows)]
pub(super) fn windows_file_identity(path: &Path) -> std::io::Result<(u32, u64)> {
    use std::mem::MaybeUninit;
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::{
        GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION,
    };

    let file = File::open(path)?;
    let mut information = MaybeUninit::<BY_HANDLE_FILE_INFORMATION>::uninit();
    // SAFETY: `file` owns a valid handle for the duration of the call and the
    // API initializes the complete output structure when it returns nonzero.
    let succeeded =
        unsafe { GetFileInformationByHandle(file.as_raw_handle(), information.as_mut_ptr()) };
    if succeeded == 0 {
        return Err(std::io::Error::last_os_error());
    }
    // SAFETY: a nonzero return from GetFileInformationByHandle guarantees the
    // output structure was initialized.
    let information = unsafe { information.assume_init() };
    let file_index =
        (u64::from(information.nFileIndexHigh) << 32) | u64::from(information.nFileIndexLow);
    Ok((information.dwVolumeSerialNumber, file_index))
}

#[cfg(not(any(unix, windows)))]
pub(super) fn same_platform_file(_observed: &Path, _expected: &Path) -> bool {
    false
}

pub(super) fn write_record(path: &Path, record: &ProcessRecord) -> std::io::Result<()> {
    let temporary = path.with_extension("json.new");
    let bytes = serde_json::to_vec(record).map_err(std::io::Error::other)?;
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = match options.open(&temporary) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            std::fs::remove_file(&temporary)?;
            options.open(&temporary)?
        }
        Err(error) => return Err(error),
    };
    file.write_all(&bytes)?;
    file.sync_all()?;
    std::fs::rename(&temporary, path)?;
    sync_parent(path)?;
    Ok(())
}

#[cfg(unix)]
pub(super) fn sync_parent(path: &Path) -> std::io::Result<()> {
    File::open(path.parent().expect("process record has a parent"))?.sync_all()
}

#[cfg(not(unix))]
pub(super) fn sync_parent(_path: &Path) -> std::io::Result<()> {
    Ok(())
}

pub(super) fn effect_digest(request: &DriverRequest) -> String {
    digest::sha256_hex(
        &serde_json::to_vec(&(request, "local-process-effect-v1"))
            .expect("local effect fields serialize"),
    )
}

pub(super) fn permanent(request: &DriverRequest, message: impl Into<String>) -> DriverError {
    let message = message.into();
    DriverError::permanent(
        message.clone(),
        digest::sha256_hex(
            &serde_json::to_vec(&(&request.operation_id, "permanent", &message))
                .expect("local error fields serialize"),
        ),
    )
}

pub(super) fn retryable(request: &DriverRequest, message: impl Into<String>) -> DriverError {
    let message = message.into();
    DriverError::retryable(
        message.clone(),
        digest::sha256_hex(
            &serde_json::to_vec(&(&request.operation_id, "retryable", &message))
                .expect("local error fields serialize"),
        ),
    )
}
