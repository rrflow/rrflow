//! Running trials and measuring their metrics.

use super::*;

pub(super) fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("run") => run_trial(&args[2..]),
        Some("summarize") => summarize(
            Path::new(args.get(2).ok_or("result directory required")?),
            args.get(3).map(Path::new),
        ),
        Some("verify") => verify(Path::new(args.get(2).ok_or("summary file required")?)),
        _ => Err("usage: rrflow-lab-experiments run <provider> <repo> <scenario> <arm> <expected> <prompt-file> <output-file> | summarize <result-dir> [new-summary-file] | verify <summary-file>".into()),
    }
}

pub(super) fn run_trial(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.len() != 7 {
        return Err("run expects seven arguments".into());
    }
    let provider = &args[0];
    let repo = PathBuf::from(&args[1]);
    let scenario = &args[2];
    let arm = &args[3];
    let expected = &args[4];
    if expected.trim().is_empty()
        || scenario.trim().is_empty()
        || !matches!(arm.as_str(), "baseline" | "runtime")
    {
        return Err(
            "screening requires a nonempty scenario and expected string and a valid arm".into(),
        );
    }
    let prompt = std::fs::read_to_string(&args[5])?;
    let output_file = PathBuf::from(&args[6]);
    let started = Instant::now();
    let output = match provider.as_str() {
        "codex" => Command::new("codex")
            .args([
                "exec",
                "--ephemeral",
                "--sandbox",
                "read-only",
                "--json",
                "--skip-git-repo-check",
                "--cd",
            ])
            .arg(&repo)
            .arg(&prompt)
            .output()?,
        "claude" => Command::new("claude")
            .args([
                "-p",
                "--permission-mode",
                "plan",
                "--allowedTools",
                "Read,Grep,Glob",
                "--output-format",
                "json",
                "--no-session-persistence",
            ])
            .arg(&prompt)
            .current_dir(&repo)
            .output()?,
        "gemini" => Command::new("gemini")
            .args([
                "--prompt",
                &prompt,
                "--approval-mode",
                "plan",
                "--output-format",
                "json",
                "--skip-trust",
            ])
            .current_dir(&repo)
            .output()?,
        other => return Err(format!("unsupported provider {other:?}").into()),
    };
    let latency_ms = started.elapsed().as_millis() as u64;
    let raw_output = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let (input_tokens, output_tokens, tool_calls) = metrics(provider, &raw_output);
    let trial = Trial {
        provider: provider.clone(),
        repository: repo.display().to_string(),
        scenario: scenario.clone(),
        arm: arm.clone(),
        success: output.status.success() && raw_output.contains(expected),
        expected: expected.clone(),
        attempts: 1,
        retries: 0,
        tool_calls,
        input_tokens,
        output_tokens,
        latency_ms,
        exit_code: output.status.code(),
        raw_output,
        stderr,
    };
    if let Some(parent) = output_file.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output_file)?;
    file.write_all(&serde_json::to_vec_pretty(&trial)?)?;
    println!(
        "{} {} {} {} success={} latency={}ms",
        provider,
        scenario,
        arm,
        repo.display(),
        trial.success,
        trial.latency_ms
    );
    Ok(())
}

pub(super) fn metrics(provider: &str, output: &str) -> (Option<u64>, Option<u64>, Option<u64>) {
    match provider {
        "claude" | "gemini" => {
            let value: Value = serde_json::from_str(output).unwrap_or(Value::Null);
            let usage = value.get("usage").or_else(|| {
                value
                    .get("stats")
                    .and_then(|v| v.get("models"))
                    .and_then(Value::as_object)
                    .and_then(|models| {
                        (models.len() == 1)
                            .then(|| models.values().next())
                            .flatten()
                    })
            });
            // Provider-native counters have different cache semantics; do not
            // add cache fields or infer a zero when the field is missing.
            let input = usage
                .and_then(|u| u.get("input_tokens").or_else(|| u.get("inputTokens")))
                .and_then(Value::as_u64);
            let output_tokens = usage
                .and_then(|u| u.get("output_tokens").or_else(|| u.get("outputTokens")))
                .and_then(Value::as_u64);
            // Model turns are not tool calls, and absence is not zero.
            (input, output_tokens, None)
        }
        "codex" => {
            let values: Vec<Value> = output
                .lines()
                .filter_map(|line| serde_json::from_str(line).ok())
                .collect();
            let usage = values.iter().find_map(|value| value.get("usage"));
            let input = usage
                .and_then(|u| u.get("input_tokens"))
                .and_then(Value::as_u64);
            let output_tokens = usage
                .and_then(|u| u.get("output_tokens"))
                .and_then(Value::as_u64);
            // A command-event count is not a complete provider tool count.
            (input, output_tokens, None)
        }
        _ => (None, None, None),
    }
}

#[cfg(test)]
mod tests;
