//! Settings, pack, key and definition commands and the dispatcher.

use super::*;

pub(super) fn settings_command(arguments: &[String]) -> Result<ExitCode> {
    match arguments.split_first() {
        None => show_settings(false),
        Some((flag, [])) if flag == "--json" => show_settings(true),
        Some((verb, rest)) => change_settings(verb, rest),
    }
}

pub(super) fn pack_bytes(path: &str) -> Result<Vec<u8>> {
    let metadata = std::fs::metadata(path).map_err(|error| format!("{path}: {error}"))?;
    if metadata.len() > MAX_PACK_BYTES {
        return Err(format!("{path} exceeds the pack size bound"));
    }
    std::fs::read(path).map_err(|error| format!("{path}: {error}"))
}

pub(super) fn print_json(value: &impl serde::Serialize) -> Result<ExitCode> {
    let text = serde_json::to_string_pretty(value).map_err(|error| error.to_string())?;
    println!("{text}");
    Ok(ExitCode::SUCCESS)
}

/// Writes a new file and never replaces one, so a key cannot be overwritten by accident.
pub(super) fn create_new(path: &str, text: &str, private: bool) -> Result<()> {
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    if private {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    #[cfg(not(unix))]
    let _ = private;
    let mut file = options
        .open(path)
        .map_err(|error| format!("{path}: {error}"))?;
    file.write_all(text.as_bytes())
        .map_err(|error| format!("{path}: {error}"))
}

pub(super) fn keygen(arguments: &Arguments) -> Result<ExitCode> {
    let key = generate_key()?;
    let trust = TrustedKeys {
        keys: vec![TrustedKey {
            id: key.key_id.clone(),
            public_key: key.public_hex,
        }],
    };
    let text = serde_json::to_string_pretty(&trust).map_err(|error| error.to_string())?;
    create_new(arguments.required("trust")?, &text, false)?;
    create_new(arguments.required("private")?, &key.private_hex, true)?;
    println!(
        "key {} created. Keep its private file out of every repository.",
        key.key_id
    );
    Ok(ExitCode::SUCCESS)
}

pub(super) fn build_definitions(
    arguments: &Arguments,
    base: Option<&VerifiedPack>,
) -> Result<ExitCode> {
    let files = lab_files(Path::new(arguments.required("lab")?))?;
    RrflowContent.check(&files)?;
    let key_path = arguments.required("key")?;
    let signing_key =
        std::fs::read_to_string(key_path).map_err(|error| format!("{key_path}: {error}"))?;
    let now = now_unix_s()?;
    let days = number(arguments, "days")?;
    let expires = days
        .checked_mul(86_400)
        .and_then(|seconds| seconds.checked_add(now))
        .ok_or("--days is too large")?;
    let signed = build_pack(&PackRequest {
        product: RRFLOW_PRODUCT,
        content_schema: RrflowContent.schema(),
        sequence: number(arguments, "sequence")?,
        published_at_unix_s: now,
        expires_at_unix_s: expires,
        files: &files,
        signing_key: signing_key.trim(),
        base: base.map(|pack| &pack.manifest),
    })?;
    let text = serde_json::to_string_pretty(&signed).map_err(|error| error.to_string())?;
    let out = arguments.required("out")?;
    std::fs::write(out, &text).map_err(|error| format!("{out}: {error}"))?;
    println!(
        "pack {out}: {} of {} files carried, {} bytes",
        signed.files.len(),
        files.len(),
        text.len()
    );
    for path in signed
        .files
        .keys()
        .take(if base.is_some() { 64 } else { 0 })
    {
        println!("  {path}");
    }
    Ok(ExitCode::SUCCESS)
}

pub(super) fn definitions(arguments: &[String]) -> Result<ExitCode> {
    let Some((verb, rest)) = arguments.split_first() else {
        return Err(USAGE.into());
    };
    let parsed = parse(rest)?;
    match verb.as_str() {
        "publish" => publish_definitions(&parsed),
        "update" => update_definitions(&parsed),
        "trust" => trust_keys(&parsed),
        "keygen" => keygen(&parsed),
        "build" => build_definitions(&parsed, None),
        "patch" => {
            let base = read_pack(
                &pack_bytes(parsed.required("base")?)?,
                &trusted_keys(&parsed)?,
            )?;
            build_definitions(&parsed, Some(&base))
        }
        "verify" => {
            let keys = trusted_keys(&parsed)?;
            let pack = read_pack(&pack_bytes(parsed.required("pack")?)?, &keys)?;
            print_json(&serde_json::json!({
                "product": pack.manifest.product,
                "sequence": pack.manifest.sequence,
                "published_at_unix_s": pack.manifest.published_at_unix_s,
                "expires_at_unix_s": pack.manifest.expires_at_unix_s,
                "manifest_sha256": pack.manifest_sha256,
                "files_listed": pack.manifest.files.len(),
                "files_carried": pack.carried.len(),
            }))
        }
        "install" => {
            let bytes = pack_bytes(parsed.required("pack")?)?;
            let outcome = open_store(&parsed)?.install(
                &bytes,
                &trusted_keys(&parsed)?,
                &RrflowContent,
                now_unix_s()?,
            )?;
            print_json(&outcome)
        }
        "status" => show_status(&parsed),
        "revert" => {
            let sequence = number(&parsed, "to")?;
            open_store(&parsed)?.revert(
                sequence,
                parsed.required("reason")?,
                &trusted_keys(&parsed)?,
                now_unix_s()?,
            )?;
            println!("reverted to pack {sequence}");
            Ok(ExitCode::SUCCESS)
        }
        _ => Err(USAGE.into()),
    }
}

pub(super) fn dispatch(arguments: &[String]) -> Result<ExitCode> {
    let Some((command, rest)) = arguments.split_first() else {
        return Err(USAGE.into());
    };
    if command == "definitions" {
        return definitions(rest);
    }
    if command == "settings" {
        return settings_command(rest);
    }
    let parsed = parse(rest)?;
    match command.as_str() {
        "validate" => {
            load_manifest(parsed.required("manifest")?)?;
            println!("manifest is valid");
            Ok(ExitCode::SUCCESS)
        }
        "inputs" => {
            let manifest = load_manifest(parsed.required("manifest")?)?;
            let inputs = prepare_inputs(&manifest)?;
            println!(
                "{}",
                String::from_utf8_lossy(&canonical_json(&inputs.identities)?).trim_end()
            );
            Ok(ExitCode::SUCCESS)
        }
        "run" => run(&parsed),
        "combine" => combine_runs(&parsed),
        "import" => import_results(&parsed),
        "sources" => check_sources(&parsed),
        "resolve-all" => resolve_all(&parsed),
        "launch-info" => {
            for (key, value) in launch_info(&load_system(parsed.required("system")?)?) {
                println!("{key}={value}");
            }
            Ok(ExitCode::SUCCESS)
        }
        "launch-args" => {
            let pod = match (parsed.values.get("pod"), parsed.values.get("volume")) {
                (Some(container), Some(volume)) => Some(Pod { container, volume }),
                (None, None) => None,
                _ => return Err("--pod and --volume go together".into()),
            };
            print_words(&launch_arguments(
                &load_system(parsed.required("system")?)?,
                parsed.required("data")?,
                parsed.required("lab")?,
                parsed.values.get("cpuset").map(String::as_str),
                pod,
                &|name| std::env::var(name).ok(),
            )?)
        }
        "vdbbench-args" => print_words(&vectordbbench_arguments(
            &load_system(parsed.required("system")?)?,
            parsed.required("data")?,
            parsed.required("ef")?,
        )?),
        "tasks-validate" => {
            let manifest = load_tasks(parsed.required("manifest")?)?;
            println!("{} tasks are valid", manifest.tasks.len());
            Ok(ExitCode::SUCCESS)
        }
        "tasks-order" => {
            let manifest = load_tasks(parsed.required("manifest")?)?;
            for key in run_order(&manifest) {
                println!(
                    "{}",
                    serde_json::to_string(&key).map_err(|error| error.to_string())?
                );
            }
            Ok(ExitCode::SUCCESS)
        }
        "tasks-digest" => {
            let manifest = load_tasks(parsed.required("manifest")?)?;
            println!("{}", manifest.digest()?.as_str());
            Ok(ExitCode::SUCCESS)
        }
        "tasks-run" => run_one_task(&parsed),
        "tasks-audit" => audit_tasks(&parsed),
        "study-check" => study_check(&parsed),
        "study-plan" => study_plan(&parsed),
        "tasks-combine" => combine_tasks(&parsed),
        "tasks-import" => import_repository(&parsed),
        "report-validate" => validate_report(&parsed),
        "schemas" => write_schemas(&parsed),
        "plan" => show_plan(&parsed),
        "compare" => compare_command(&parsed),
        _ => Err(USAGE.into()),
    }
}
