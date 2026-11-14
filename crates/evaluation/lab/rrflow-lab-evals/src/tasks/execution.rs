//! Running one task and collecting its system observations.

use super::*;

impl TaskRun {
    /// What a finished run lacks. A run that failed is a result and owes nothing further.
    pub fn missing_captures(&self, arm: &TaskArm, required: &[CaptureField]) -> Vec<CaptureField> {
        if !matches!(
            self.outcome,
            TaskRunOutcome::Planned | TaskRunOutcome::NoPlan
        ) {
            return Vec::new();
        }
        required
            .iter()
            .copied()
            .filter(|field| !self.holds(*field, arm))
            .collect()
    }

    fn holds(&self, field: CaptureField, arm: &TaskArm) -> bool {
        match field {
            CaptureField::Model => self.model_reported.is_some(),
            CaptureField::ProviderVersion => self.provider_version.is_some(),
            CaptureField::Tokens => self.input_tokens.is_some() && self.output_tokens.is_some(),
            CaptureField::CacheTokens => {
                self.cache_read_tokens.is_some() && self.cache_write_tokens.is_some()
            }
            CaptureField::Cost => self.cost_usd.is_some(),
            CaptureField::Turns => self.turns.is_some(),
            CaptureField::ToolCalls => self.tool_calls.is_some(),
            CaptureField::Retrieval => {
                !arm.tools.iter().any(|tool| tool.starts_with("mcp__")) || self.retrieval.is_some()
            }
            CaptureField::Transcript => self.reply_sha256.is_some(),
        }
    }

    /// Whether this run can count for or against its arm.
    pub fn counts(&self, arm: &TaskArm, required: &[CaptureField]) -> bool {
        self.outcome != TaskRunOutcome::ToolingFailed
            && self.missing_captures(arm, required).is_empty()
    }
}

/// What a run took from the total. A provider that reported no cost is charged at the run's limit.
pub fn charged_usd(run: &TaskRun, spend: &SpendLimits) -> f64 {
    run.cost_usd.unwrap_or(spend.max_usd_per_run)
}

/// Why an arm's MCP servers are not usable, from what the provider reported at startup.
pub(super) fn tooling_problem(arm: &TaskArm, reply: &AgentReply) -> Option<String> {
    arm.tools
        .iter()
        .filter_map(|tool| tool.strip_prefix("mcp__")?.split("__").next())
        .find_map(
            |server| match reply.mcp_servers.as_ref().and_then(|all| all.get(server)) {
                Some(status) if status == "connected" => None,
                Some(status) => Some(format!("the {server} MCP server was {status}")),
                None => Some(format!(
                    "the provider did not report the {server} MCP server"
                )),
            },
        )
}

/// Which files an MCP tool returned, read from the chunk identities in its replies. Any shape
/// of reply works, and a reply without identities shows as nothing returned.
pub(super) fn capture_retrieval(
    task: &AgentTask,
    reply: &AgentReply,
    chunk_files: &BTreeMap<String, String>,
    plan: Option<&AgentPlan>,
) -> RetrievalCapture {
    let mut returned = BTreeSet::new();
    for text in &reply.mcp_results {
        for token in text.split(|character: char| !character.is_ascii_alphanumeric()) {
            if let Some(path) = chunk_files.get(token) {
                returned.insert(normalize_path(path));
            }
        }
    }
    let named: BTreeSet<String> = plan
        .map(|plan| plan.files.iter().map(|path| normalize_path(path)).collect())
        .unwrap_or_default();
    let needed: BTreeSet<String> = task
        .expected_files
        .iter()
        .map(|path| normalize_path(path))
        .collect();
    let found: Vec<&String> = needed.intersection(&returned).collect();
    RetrievalCapture {
        calls: reply
            .tool_calls
            .iter()
            .flatten()
            .filter(|(name, _)| name.starts_with("mcp__"))
            .map(|(_, calls)| *calls)
            .sum(),
        bytes_returned: reply.mcp_results.iter().map(|text| text.len() as u64).sum(),
        files_returned: returned.len() as u64,
        needed_files_returned: found.len() as u64,
        needed_files_used: found.iter().filter(|path| named.contains(**path)).count() as u64,
    }
}

pub struct RunInputs<'a> {
    pub provider: &'a dyn AgentProvider,
    /// The task's repository at its revision.
    pub checkout: &'a Path,
    /// An empty directory, for an arm that must not see the repository.
    pub empty_directory: &'a Path,
    /// Spent so far in this study, which a run must not push past the total.
    pub spent_usd: f64,
    /// Where an arm with MCP tools finds its server.
    pub mcp_config: Option<&'a Path>,
    /// Each imported chunk's file, so what an MCP tool returned can be read.
    pub chunk_files: Option<&'a BTreeMap<String, String>>,
}

/// Runs one key from the plan. A provider's failure is a result and comes back as a run.
pub fn run_task(
    manifest: &TaskComparisonManifest,
    key: &RunKey,
    inputs: &RunInputs<'_>,
) -> Result<(TaskRun, Vec<u8>)> {
    manifest.validate()?;
    let spend = manifest.require_runnable()?;
    require(
        inputs.spent_usd + spend.max_usd_per_run <= spend.max_usd_total,
        "another run could pass the total spend limit",
    )?;
    let arm = manifest.arm(key.arm.as_str())?;
    require(
        arm.availability == Availability::Available,
        &format!("{} is declared as not running", arm.id.as_str()),
    )?;
    let uses_mcp = arm.tools.iter().any(|tool| tool.starts_with("mcp__"));
    require(
        uses_mcp == inputs.mcp_config.is_some(),
        &format!(
            "{} must have an MCP configuration exactly when it uses an MCP tool",
            arm.id.as_str()
        ),
    )?;
    require(
        uses_mcp == inputs.chunk_files.is_some(),
        &format!(
            "{} must have a chunk index exactly when it uses an MCP tool",
            arm.id.as_str()
        ),
    )?;
    let task = manifest
        .tasks
        .iter()
        .find(|task| task.id == key.task)
        .ok_or_else(|| format!("no task named {}", key.task.as_str()))?;
    let model = manifest
        .models
        .iter()
        .find(|model| model.id == key.model)
        .ok_or_else(|| format!("no model named {}", key.model.as_str()))?;
    let prompt = task_prompt(task, arm);
    let started = std::time::Instant::now();
    let reply = inputs.provider.run(&AgentRequest {
        prompt: &prompt,
        directory: if arm.repository_access {
            inputs.checkout
        } else {
            inputs.empty_directory
        },
        model: &model.model,
        effort: model.effort.as_deref(),
        tools: &arm.tools,
        mcp_config: inputs.mcp_config,
        max_seconds: spend.max_seconds_per_run,
        max_usd: spend.max_usd_per_run,
    });
    let seconds = started.elapsed().as_secs_f64();
    let manifest_sha256 = manifest.digest()?;
    let blank = |outcome, reason: Option<String>, reply: &AgentReply| TaskRun {
        schema_version: TASKS_SCHEMA_VERSION,
        manifest_sha256: manifest_sha256.clone(),
        task: key.task.clone(),
        arm: key.arm.clone(),
        model: key.model.clone(),
        repetition: key.repetition,
        outcome,
        reason,
        score: PlanScore {
            files_expected: task.expected_files.len() as u64,
            records_expected: task.expected_records.len() as u64,
            ..PlanScore::default()
        },
        seconds,
        input_tokens: reply.input_tokens,
        output_tokens: reply.output_tokens,
        cost_usd: reply.cost_usd,
        turns: reply.turns,
        reply_sha256: None,
        model_reported: reply.model_reported.clone(),
        provider_version: reply.provider_version.clone(),
        cache_read_tokens: reply.cache_read_tokens,
        cache_write_tokens: reply.cache_write_tokens,
        tool_calls: reply.tool_calls.clone(),
        retrieval: None,
    };
    let reply = match reply {
        Ok(reply) => reply,
        Err(error) => {
            return Ok((
                blank(
                    TaskRunOutcome::ProviderFailed,
                    Some(error),
                    &AgentReply::default(),
                ),
                Vec::new(),
            ))
        }
    };
    let digest = Digest::try_from(sha256_hex(&reply.raw))?;
    let tooling = tooling_problem(arm, &reply)
        .filter(|_| reply.exit_code == Some(0) || reply.mcp_servers.is_some());
    let mut run = if reply.timed_out {
        blank(TaskRunOutcome::TimedOut, None, &reply)
    } else if let Some(problem) = tooling {
        blank(TaskRunOutcome::ToolingFailed, Some(problem), &reply)
    } else if reply.exit_code != Some(0) {
        blank(
            TaskRunOutcome::ProviderFailed,
            Some(format!("the provider exited with {:?}", reply.exit_code)),
            &reply,
        )
    } else if let Some(plan) = parse_plan(&reply.text) {
        let mut run = blank(TaskRunOutcome::Planned, None, &reply);
        run.score = score_plan(task, &plan, &manifest.preregistration);
        run.retrieval = inputs
            .chunk_files
            .map(|chunks| capture_retrieval(task, &reply, chunks, Some(&plan)));
        run
    } else {
        let mut run = blank(TaskRunOutcome::NoPlan, None, &reply);
        run.retrieval = inputs
            .chunk_files
            .map(|chunks| capture_retrieval(task, &reply, chunks, None));
        run
    };
    run.reply_sha256 = Some(digest);
    Ok((run, reply.raw))
}

/// Wilson's interval, which stays sensible at few runs and at rates near 0 or 1.
pub fn wilson_interval(successes: u64, total: u64) -> (f64, f64) {
    if total == 0 {
        return (0.0, 0.0);
    }
    let n = total as f64;
    let p = successes as f64 / n;
    let z2 = Z95 * Z95;
    let center = p + z2 / (2.0 * n);
    let spread = Z95 * ((p * (1.0 - p) + z2 / (4.0 * n)) / n).sqrt();
    let denominator = 1.0 + z2 / n;
    (
        ((center - spread) / denominator).max(0.0),
        ((center + spread) / denominator).min(1.0),
    )
}

/// Two-sided exact McNemar test on the pairs where the two arms disagreed.
pub fn exact_mcnemar(only_first: u64, only_second: u64) -> f64 {
    let n = only_first + only_second;
    if n == 0 {
        return 1.0;
    }
    let smaller = only_first.min(only_second);
    let mut log_term = -(n as f64) * std::f64::consts::LN_2;
    let mut tail = log_term.exp();
    for i in 1..=smaller {
        log_term += ((n - i + 1) as f64).ln() - (i as f64).ln();
        tail += log_term.exp();
    }
    (2.0 * tail).min(1.0)
}

pub(super) fn median(mut values: Vec<f64>) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(f64::total_cmp);
    let middle = values.len() / 2;
    Some(if values.len() % 2 == 1 {
        values[middle]
    } else {
        (values[middle - 1] + values[middle]) / 2.0
    })
}

pub(super) fn mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len().max(1) as f64
}

pub(super) fn id(text: &str) -> Result<Id> {
    Id::try_from(text.to_string())
}

pub(super) fn system_id(arm: &Id, model: &Id) -> Result<Id> {
    id(&format!("{}.{}", arm.as_str(), model.as_str()))
}

pub(super) fn metric(
    key: &str,
    label: &str,
    unit: &str,
    better: MetricDirection,
) -> Result<MetricDefinition> {
    Ok(MetricDefinition {
        id: id(key)?,
        label: label.into(),
        unit: unit.into(),
        better,
    })
}

pub(super) fn observation(
    system: &Id,
    metric_id: &str,
    case: BTreeMap<String, String>,
    value: f64,
    samples: u64,
    interval: Option<(f64, f64)>,
) -> Result<Observation> {
    Ok(Observation {
        system: system.clone(),
        metric: id(metric_id)?,
        case,
        value,
        samples,
        low: interval.map(|bounds| bounds.0),
        high: interval.map(|bounds| bounds.1),
    })
}

pub(super) fn system_observations(
    system: &Id,
    runs: &[&TaskRun],
    list: &mut Vec<Observation>,
) -> Result<()> {
    let usable: Vec<&&TaskRun> = runs
        .iter()
        .filter(|run| {
            matches!(
                run.outcome,
                TaskRunOutcome::Planned | TaskRunOutcome::NoPlan
            )
        })
        .collect();
    let total = usable.len() as u64;
    let solved = usable
        .iter()
        .filter(|run| run.score.matches_file_threshold)
        .count() as u64;
    let none = BTreeMap::new;
    if total > 0 {
        list.push(observation(
            system,
            "file_match_rate",
            none(),
            solved as f64 / total as f64,
            total,
            Some(wilson_interval(solved, total)),
        )?);
        let recalls: Vec<f64> = usable.iter().map(|run| run.score.file_recall).collect();
        let precisions: Vec<f64> = usable.iter().map(|run| run.score.file_precision).collect();
        list.push(observation(
            system,
            "file_recall",
            none(),
            mean(&recalls),
            total,
            None,
        )?);
        list.push(observation(
            system,
            "file_precision",
            none(),
            mean(&precisions),
            total,
            None,
        )?);
    }
    let cited: u64 = usable.iter().map(|run| run.score.records_cited).sum();
    let expected: u64 = usable.iter().map(|run| run.score.records_expected).sum();
    if expected > 0 {
        list.push(observation(
            system,
            "records_cited",
            none(),
            cited as f64 / expected as f64,
            total,
            None,
        )?);
    }
    list.push(observation(
        system,
        "runs_without_a_plan",
        none(),
        runs.iter()
            .filter(|run| run.outcome != TaskRunOutcome::Planned)
            .count() as f64,
        runs.len() as u64,
        None,
    )?);
    let retrievals: Vec<&RetrievalCapture> = runs
        .iter()
        .filter_map(|run| run.retrieval.as_ref())
        .collect();
    if !retrievals.is_empty() {
        let returned: u64 = retrievals
            .iter()
            .map(|retrieval| retrieval.needed_files_returned)
            .sum();
        let used: u64 = retrievals
            .iter()
            .map(|retrieval| retrieval.needed_files_used)
            .sum();
        let needed: u64 = runs
            .iter()
            .filter(|run| run.retrieval.is_some())
            .map(|run| run.score.files_expected)
            .sum();
        list.push(observation(
            system,
            "needed_returned",
            none(),
            returned as f64 / needed.max(1) as f64,
            needed,
            None,
        )?);
        if returned > 0 {
            list.push(observation(
                system,
                "returned_used",
                none(),
                used as f64 / returned as f64,
                returned,
                None,
            )?);
        }
    }
    let medians: [(&str, Vec<f64>); 6] = [
        ("seconds", runs.iter().map(|run| run.seconds).collect()),
        (
            "input_tokens",
            runs.iter()
                .filter_map(|run| run.input_tokens.map(|value| value as f64))
                .collect(),
        ),
        (
            "output_tokens",
            runs.iter()
                .filter_map(|run| run.output_tokens.map(|value| value as f64))
                .collect(),
        ),
        (
            "cost_usd",
            runs.iter().filter_map(|run| run.cost_usd).collect(),
        ),
        (
            "cache_read_tokens",
            runs.iter()
                .filter_map(|run| run.cache_read_tokens.map(|value| value as f64))
                .collect(),
        ),
        (
            "tool_calls",
            runs.iter()
                .filter_map(|run| {
                    run.tool_calls
                        .as_ref()
                        .map(|calls| calls.values().sum::<u64>() as f64)
                })
                .collect(),
        ),
    ];
    for (metric_id, values) in medians {
        let samples = values.len() as u64;
        if let Some(value) = median(values) {
            list.push(observation(
                system,
                metric_id,
                none(),
                value,
                samples,
                None,
            )?);
        }
    }
    let mut by_task: BTreeMap<&Id, (u64, u64)> = BTreeMap::new();
    for run in usable {
        let entry = by_task.entry(&run.task).or_default();
        entry.1 += 1;
        if run.score.matches_file_threshold {
            entry.0 += 1;
        }
    }
    for (task, (solved, total)) in by_task {
        list.push(observation(
            system,
            "file_match_rate",
            BTreeMap::from([("task".to_string(), task.as_str().to_string())]),
            solved as f64 / total as f64,
            total,
            Some(wilson_interval(solved, total)),
        )?);
    }
    Ok(())
}
