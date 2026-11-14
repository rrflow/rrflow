//! Screening files, paired regressions, summaries and verification.

use super::*;

pub(super) const MAX_SCREENING_FILE_BYTES: u64 = 8 * 1024 * 1024;
pub(super) const MAX_SCREENING_TRIALS: usize = 256;

pub(super) fn read_screening_file(path: &Path) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(MAX_SCREENING_FILE_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_SCREENING_FILE_BYTES {
        return Err("screening file exceeds 8 MiB".into());
    }
    Ok(bytes)
}

pub(super) fn validate_trial(trial: &Trial) -> Result<(), Box<dyn std::error::Error>> {
    if trial.provider.trim().is_empty()
        || trial.repository.trim().is_empty()
        || trial.scenario.trim().is_empty()
        || !matches!(trial.arm.as_str(), "baseline" | "runtime")
        || trial.expected.trim().is_empty()
        || trial.attempts
            != trial
                .retries
                .checked_add(1)
                .ok_or("attempt count overflow")?
        || trial.success
            != (trial.exit_code == Some(0) && trial.raw_output.contains(&trial.expected))
    {
        return Err("invalid or inconsistent screening trial".into());
    }
    Ok(())
}

pub(super) fn paired_regressions(trials: &[Trial]) -> Result<usize, Box<dyn std::error::Error>> {
    if trials.is_empty() {
        return Err("screening has no trials".into());
    }
    let mut pairs: BTreeMap<(&str, &str, &str), [Option<bool>; 2]> = BTreeMap::new();
    for trial in trials {
        validate_trial(trial)?;
        let pair = pairs
            .entry((&trial.provider, &trial.repository, &trial.scenario))
            .or_default();
        let arm = usize::from(trial.arm == "runtime");
        if pair[arm].replace(trial.success).is_some() {
            return Err("duplicate provider/repository/scenario arm".into());
        }
    }
    if pairs.values().any(|pair| pair.iter().any(Option::is_none)) {
        return Err("screening has an incomplete provider/repository/scenario pair".into());
    }
    Ok(pairs
        .values()
        .filter(|pair| pair[0] == Some(true) && pair[1] == Some(false))
        .count())
}

pub(super) fn screening_summary(
    mut trials: Vec<Trial>,
) -> Result<ScreeningSummary, Box<dyn std::error::Error>> {
    if trials.len() < 8 || trials.len() > MAX_SCREENING_TRIALS {
        return Err("screening summary requires 8 through 256 trials".into());
    }
    trials.sort_by(|a, b| {
        (&a.provider, &a.repository, &a.scenario, &a.arm).cmp(&(
            &b.provider,
            &b.repository,
            &b.scenario,
            &b.arm,
        ))
    });
    let mut providers = BTreeSet::new();
    let mut repositories = BTreeSet::new();
    for trial in &trials {
        providers.insert(trial.provider.clone());
        repositories.insert(trial.repository.clone());
    }
    if providers.len() < 2 || repositories.len() < 2 {
        return Err("screening requires at least two distinct providers and repositories".into());
    }
    let paired_regressions = paired_regressions(&trials)?;
    Ok(ScreeningSummary {
        format_version: 1,
        evidence_class: "provider_screening_only".into(),
        trials: trials.len(),
        successes: trials.iter().filter(|trial| trial.success).count(),
        paired_regressions,
        providers: providers.into_iter().collect(),
        repositories: repositories.into_iter().collect(),
        trials_detail: trials,
    })
}

pub(super) fn summarize(
    dir: &Path,
    output: Option<&Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut trials = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.extension().and_then(|value| value.to_str()) == Some("json") {
            if trials.len() == MAX_SCREENING_TRIALS {
                return Err("screening exceeds 256 trials".into());
            }
            let trial: Trial = serde_json::from_slice(&read_screening_file(&path)?)?;
            validate_trial(&trial)?;
            trials.push(trial);
        }
    }
    trials.sort_by(|a, b| {
        (&a.provider, &a.repository, &a.scenario, &a.arm).cmp(&(
            &b.provider,
            &b.repository,
            &b.scenario,
            &b.arm,
        ))
    });
    let success = trials.iter().filter(|trial| trial.success).count();
    let regressions = paired_regressions(&trials)?;
    let latency = trials.iter().try_fold(0u64, |total, trial| {
        total
            .checked_add(trial.latency_ms)
            .ok_or("latency overflow")
    })?;
    let known_token_pairs = trials
        .iter()
        .filter(|trial| trial.input_tokens.zip(trial.output_tokens).is_some())
        .count();
    if let Some(path) = output {
        let report = screening_summary(trials.clone())?;
        let bytes = serde_json::to_vec_pretty(&report)?;
        if bytes.len() as u64 > MAX_SCREENING_FILE_BYTES {
            return Err("screening summary exceeds 8 MiB".into());
        }
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)?;
        file.write_all(&bytes)?;
    }
    println!(
        "trials={} success={}/{} regressions={} total_latency_ms={} known_token_pairs={}/{}",
        trials.len(),
        success,
        trials.len(),
        regressions,
        latency,
        known_token_pairs,
        trials.len()
    );
    println!("screening only: treatment and answer correctness not verified; provider token counters are not aggregated");
    for trial in trials {
        println!(
            "{} {} {} success={} retries={} tools={:?} tokens={:?}+{:?} latency={}ms",
            trial.provider,
            trial.scenario,
            trial.arm,
            trial.success,
            trial.retries,
            trial.tool_calls,
            trial.input_tokens,
            trial.output_tokens,
            trial.latency_ms
        );
    }
    Ok(())
}

pub(super) fn verify(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let report: ScreeningSummary = serde_json::from_slice(&read_screening_file(path)?)?;
    let expected = screening_summary(report.trials_detail)?;
    if report.format_version != expected.format_version
        || report.evidence_class != expected.evidence_class
        || report.trials != expected.trials
        || report.successes != expected.successes
        || report.paired_regressions != expected.paired_regressions
        || report.providers != expected.providers
        || report.repositories != expected.repositories
    {
        return Err("screening summary disagrees with its trial records".into());
    }
    println!("verified screening consistency: trials={} successes={} regressions={}; treatment and answer correctness not verified", expected.trials, expected.successes, expected.paired_regressions);
    Ok(())
}
