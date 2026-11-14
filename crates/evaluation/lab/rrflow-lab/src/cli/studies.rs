//! Study checks and plans, task audits and runs, repository import and schemas.

use super::*;

pub(super) struct StudyFiles {
    config: StudyConfig,
    catalog: TaskComparisonManifest,
    requirements: StudyRequirements,
    proof: Option<CaptureAudit>,
}

pub(super) fn load_study(arguments: &Arguments) -> Result<StudyFiles> {
    let sources: SourceList = read_json(arguments.required("sources")?)?;
    sources.validate()?;
    let requirements: StudyRequirements = read_json(arguments.required("requirements")?)?;
    requirements.validate(&sources)?;
    Ok(StudyFiles {
        config: read_json(arguments.required("config")?)?,
        catalog: load_tasks(arguments.required("catalog")?)?,
        requirements,
        proof: arguments
            .values
            .get("capture-proof")
            .map(|path| read_json(path))
            .transpose()?,
    })
}

pub(super) fn print_check(check: &StudyCheck) {
    println!(
        "{} ({:?}): {} runs across {} tasks, {} arms and {} models",
        check.study.as_str(),
        check.stage,
        check.runs,
        check.tasks,
        check.arms,
        check.models
    );
    if let Some(worst) = check.worst_case_usd {
        println!("at most ${worst}");
    }
    println!(
        "exact McNemar needs at least {} one-sided pairs, so this design {} reach significance",
        check.discordant_pairs_needed,
        if check.can_reach_significance {
            "can"
        } else {
            "cannot"
        }
    );
    for outcome in &check.outcomes {
        let label = match outcome.status {
            RequirementStatus::Met => "met    ",
            RequirementStatus::NotMet => "NOT MET",
            RequirementStatus::AdvisedNotMet => "advised",
        };
        println!("{label} {}: {}", outcome.id.as_str(), outcome.detail);
    }
    println!("{}", if check.ready { "ready" } else { "not ready" });
}

pub(super) fn study_check(arguments: &Arguments) -> Result<ExitCode> {
    let files = load_study(arguments)?;
    let check = check_study(
        &files.config,
        &files.catalog,
        &files.requirements,
        files.proof.as_ref(),
    )?;
    print_check(&check);
    if let Some(path) = arguments.values.get("out") {
        std::fs::write(path, canonical_json(&check)?).map_err(|error| error.to_string())?;
    }
    Ok(if check.ready {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

pub(super) fn study_plan(arguments: &Arguments) -> Result<ExitCode> {
    let files = load_study(arguments)?;
    let manifest = plan_study(
        &files.config,
        &files.catalog,
        &files.requirements,
        files.proof.as_ref(),
    )?;
    let path = PathBuf::from(arguments.required("out")?);
    publish(&path, &canonical_json(&manifest)?)?;
    println!("{} {}", manifest.digest()?.as_str(), path.display());
    Ok(ExitCode::SUCCESS)
}

pub(super) fn audit_tasks(arguments: &Arguments) -> Result<ExitCode> {
    let manifest = load_tasks(arguments.required("manifest")?)?;
    let runs: Vec<TaskRun> = saved_task_runs(Path::new(arguments.required("in")?))?
        .into_iter()
        .map(|(_, run)| run)
        .collect();
    let audit = audit_runs(&manifest, &runs)?;
    for arm in &audit.arms {
        println!(
            "{} {}: {} of {} runs present, {} counted, {} failed, {} without tools, {}",
            arm.arm.as_str(),
            arm.model_name,
            arm.runs_present,
            arm.runs_expected,
            arm.runs_counted,
            arm.runs_failed,
            arm.runs_without_tools,
            if arm.proven { "proven" } else { "not proven" }
        );
        for field in &arm.fields {
            println!(
                "  {}: {} captured, {} missing",
                field.field.label(),
                field.captured,
                field.missing
            );
        }
    }
    for problem in &audit.problems {
        println!("problem: {problem}");
    }
    if let Some(path) = arguments.values.get("out") {
        std::fs::write(path, canonical_json(&audit)?).map_err(|error| error.to_string())?;
        println!("{} {path}", audit.digest()?.as_str());
    }
    println!(
        "{}",
        if audit.complete {
            "complete"
        } else {
            "incomplete"
        }
    );
    Ok(if audit.complete {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

/// Each imported chunk's file, keyed by the identity that appears in a tool's reply.
pub(super) fn load_chunk_files(path: &str) -> Result<BTreeMap<String, String>> {
    let text = std::fs::read_to_string(path).map_err(|error| format!("{path}: {error}"))?;
    let mut files = BTreeMap::new();
    for line in text.lines() {
        let chunk: serde_json::Value =
            serde_json::from_str(line).map_err(|error| format!("{path}: {error}"))?;
        let (Some(identity), Some(file)) = (chunk["id"].as_str(), chunk["path"].as_str()) else {
            return Err(format!("{path}: a line has no id or path"));
        };
        let short = identity.rsplit(':').next().unwrap_or(identity);
        files.insert(short.to_string(), file.to_string());
    }
    Ok(files)
}

pub(super) fn load_tasks(path: &str) -> Result<TaskComparisonManifest> {
    let bytes = std::fs::read(path).map_err(|error| format!("{path}: {error}"))?;
    let manifest: TaskComparisonManifest =
        serde_json::from_slice(&bytes).map_err(|error| format!("{path}: {error}"))?;
    manifest.validate()?;
    Ok(manifest)
}

pub(super) fn task_id(text: &str) -> Result<Id> {
    Id::try_from(text.to_string())
}

pub(super) fn saved_task_runs(directory: &Path) -> Result<Vec<(PathBuf, TaskRun)>> {
    let mut runs = Vec::new();
    if !directory.exists() {
        return Ok(runs);
    }
    for entry in std::fs::read_dir(directory).map_err(|error| error.to_string())? {
        let path = entry.map_err(|error| error.to_string())?.path();
        if path.to_string_lossy().ends_with(".run.json") {
            let bytes = std::fs::read(&path).map_err(|error| error.to_string())?;
            let run: TaskRun = serde_json::from_slice(&bytes)
                .map_err(|error| format!("{}: {error}", path.display()))?;
            runs.push((path, run));
        }
    }
    runs.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(runs)
}

pub(super) fn run_one_task(arguments: &Arguments) -> Result<ExitCode> {
    let manifest = load_tasks(arguments.required("manifest")?)?;
    let key = RunKey {
        task: task_id(arguments.required("task")?)?,
        arm: task_id(arguments.required("arm")?)?,
        model: task_id(arguments.required("model")?)?,
        repetition: arguments
            .required("repetition")?
            .parse()
            .map_err(|error| format!("repetition: {error}"))?,
    };
    let directory = PathBuf::from(arguments.required("out")?);
    std::fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    let name = format!(
        "{}.{}.{}.{}",
        key.task.as_str(),
        key.arm.as_str(),
        key.model.as_str(),
        key.repetition
    );
    let run_path = directory.join(format!("{name}.run.json"));
    if run_path.exists() {
        eprintln!("{name}: already measured");
        return Ok(ExitCode::SUCCESS);
    }
    let spend = manifest.require_runnable()?;
    let spent_usd = saved_task_runs(&directory)?
        .iter()
        .map(|(_, run)| charged_usd(run, spend))
        .sum();
    let chunk_files = arguments
        .values
        .get("chunks")
        .map(|path| load_chunk_files(path))
        .transpose()?;
    let (run, raw) = run_task(
        &manifest,
        &key,
        &RunInputs {
            provider: &ClaudeCode,
            checkout: Path::new(arguments.required("checkout")?),
            empty_directory: Path::new(arguments.required("empty")?),
            spent_usd,
            mcp_config: arguments.values.get("mcp-config").map(Path::new),
            chunk_files: chunk_files.as_ref(),
        },
    )?;
    publish(&directory.join(format!("{name}.reply.json")), &raw)?;
    publish(&run_path, &canonical_json(&run)?)?;
    eprintln!(
        "{name}: {:?}, matched historical file threshold {}",
        run.outcome, run.score.matches_file_threshold
    );
    Ok(ExitCode::SUCCESS)
}

/// Credentials come from the environment, as for a lab run, and never reach a file or log.
pub(super) fn import_repository(arguments: &Arguments) -> Result<ExitCode> {
    let credential = |name: &str| {
        std::env::var(name).map_err(|_| format!("{name} must name the engine's credential"))
    };
    let checkout = Path::new(arguments.required("checkout")?);
    let started = std::time::Instant::now();
    let text = read_repository_text(checkout)?;
    if let Some(path) = arguments.values.get("chunks-out") {
        let mut lines = Vec::new();
        for chunk in &text.chunks {
            lines.extend(
                serde_json::to_vec(&serde_json::json!({
                    "id": format!("record:{CHUNK_TABLE}:{}", chunk.reference_id()),
                    "path": chunk.path,
                    "text": chunk.text
                }))
                .map_err(|error| error.to_string())?,
            );
            lines.push(b'\n');
        }
        publish(Path::new(path), &lines)?;
    }
    let adapter = RrflowAdapter::connect_to(
        arguments.required("endpoint")?,
        &credential("RRFLOW_LAB_INSTANCE")?,
        &credential("RRFLOW_LAB_PRINCIPAL")?,
        credential("RRFLOW_LAB_SECRET")?,
        std::time::Duration::from_secs(300),
    )?;
    let records = text
        .chunks
        .iter()
        .map(|chunk| chunk.mutation())
        .collect::<Vec<_>>();
    adapter.import_records(
        CHUNK_TABLE,
        "add the lab's imported repository text",
        &records,
    )?;
    println!(
        "{}",
        serde_json::json!({
            "files": text.files,
            "files_left_out": text.files_left_out,
            "chunks": text.chunks.len(),
            "seconds": started.elapsed().as_secs_f64()
        })
    );
    Ok(ExitCode::SUCCESS)
}

pub(super) fn validate_report(arguments: &Arguments) -> Result<ExitCode> {
    let path = arguments.required("file")?;
    let bytes = std::fs::read(path).map_err(|error| format!("{path}: {error}"))?;
    let report: LabReport =
        serde_json::from_slice(&bytes).map_err(|error| format!("{path}: {error}"))?;
    report.validate()?;
    println!("report is valid");
    Ok(ExitCode::SUCCESS)
}

pub(super) fn combine_tasks(arguments: &Arguments) -> Result<ExitCode> {
    let manifest = load_tasks(arguments.required("manifest")?)?;
    let directory = PathBuf::from(arguments.required("in")?);
    let mut runs = Vec::new();
    let mut artifacts = Vec::new();
    for (path, run) in saved_task_runs(&directory)? {
        artifacts.push(artifact(&path, "one graded task run")?);
        runs.push(run);
    }
    let report = combine_task_runs(&manifest, &runs, artifacts, now_unix_ms())?;
    publish(
        Path::new(arguments.required("out")?),
        &canonical_json(&report)?,
    )?;
    Ok(ExitCode::SUCCESS)
}

pub(super) fn write_schemas(arguments: &Arguments) -> Result<ExitCode> {
    let directory = PathBuf::from(arguments.required("out")?);
    for (name, value) in [
        ("lab-manifest-v1.json", schema::<VectorComparisonManifest>()),
        ("lab-run-v1.json", schema::<SystemRun>()),
        ("lab-report-v1.json", schema::<LabReport>()),
        ("lab-custom-protocol-v1.json", schema::<CustomProtocol>()),
        ("lab-sources-v1.json", schema::<SourceList>()),
        ("lab-system-v1.json", schema::<SystemDefinition>()),
        ("lab-test-v1.json", schema::<TestDefinition>()),
        ("lab-settings-v1.json", schema::<LabSettings>()),
        ("lab-plan-v1.json", schema::<LabPlan>()),
        ("lab-reference-v1.json", schema::<ReferenceFile>()),
        ("lab-comparison-v1.json", schema::<Comparison>()),
        ("lab-study-config-v1.json", schema::<StudyConfig>()),
        (
            "lab-study-requirements-v1.json",
            schema::<StudyRequirements>(),
        ),
        ("lab-study-check-v1.json", schema::<StudyCheck>()),
        ("lab-capture-audit-v1.json", schema::<CaptureAudit>()),
        (
            "lab-task-manifest-v1.json",
            schema::<TaskComparisonManifest>(),
        ),
        ("lab-task-run-v1.json", schema::<TaskRun>()),
    ] {
        std::fs::write(directory.join(name), canonical_json(&value)?)
            .map_err(|error| error.to_string())?;
    }
    Ok(ExitCode::SUCCESS)
}
