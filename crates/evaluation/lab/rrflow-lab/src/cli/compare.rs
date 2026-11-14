//! Comparison reports: speeds, seek and answer rows.

use super::*;

pub(super) fn short_number(value: f64) -> String {
    if value >= 100.0 {
        grouped(value.round() as u64)
    } else if value >= 10.0 {
        format!("{value:.1}")
    } else {
        format!("{value:.2}")
    }
}

pub(super) fn print_speeds(speeds: &[&MeasuredSpeed]) {
    for speed in speeds {
        println!(
            "  {:<28}{:>2} client{}  {:>9} per second  {}",
            speed.system.as_str(),
            speed.concurrency,
            if speed.concurrency == 1 { " " } else { "s" },
            short_number(speed.queries_per_second),
            speed
                .latency_p99_ms
                .map_or("p99 not recorded".to_owned(), |latency| format!(
                    "p99 {} ms",
                    short_number(latency)
                ))
        );
    }
}

/// Enough digits to tell results apart, and not the sixteen a source may hold.
pub(super) fn recall_words(recall: f64) -> String {
    let text = format!("{recall:.4}");
    text.trim_end_matches('0').trim_end_matches('.').to_owned()
}

pub(super) fn print_seek_row(row: &ComparedSeek, details: bool, ranked_beside_a_test: bool) {
    let result = &row.result;
    println!(
        "  {:>9}/s  recall {:<6}  {:<16}  {}",
        short_number(result.queries_per_second),
        recall_words(result.recall),
        result
            .latency_p99_ms
            .map_or("p99 not stated".to_owned(), |latency| format!(
                "p99 {} ms",
                short_number(latency)
            )),
        result.system,
    );
    println!(
        "              {}, top {}, {}{}. {} ({}).",
        result.dataset,
        result
            .k
            .map_or("k not stated".to_owned(), |k| k.to_string()),
        result.hardware_class.words(),
        if result.filtered { ", filtered" } else { "" },
        row.source_title,
        result.origin.provenance.words(),
    );
    if row.stale {
        println!(
            "              Last read at its source on {}, so check it again.",
            row.checked_on
        );
    }
    if details {
        println!(
            "              Measured by {}, {}. Read on {}.",
            result.origin.measured_by,
            result
                .origin
                .published
                .as_deref()
                .unwrap_or("date not stated"),
            row.checked_on
        );
        println!("              {}", result.origin.url);
        println!("              Hardware: {}.", result.hardware);
        for limit in &result.limits {
            println!("              Note: {limit}");
        }
        if ranked_beside_a_test && !row.same_conditions {
            println!("              Differs: {}.", row.differs.join("; "));
        }
    }
}

pub(super) fn print_answer_row(row: &ListedAnswer, details: bool) {
    let result = &row.result;
    println!(
        "  {:>7}  {}  ({})",
        result.score, result.system, result.score_name
    );
    println!(
        "           Reader {}, judge {} ({}).{}",
        result.reader_model.as_deref().unwrap_or("not stated"),
        result.judge_model.as_deref().unwrap_or("not stated"),
        result.origin.provenance.words(),
        if result.disputed { " Disputed." } else { "" },
    );
    if row.stale {
        println!(
            "           Last read at its source on {}, so check it again.",
            row.checked_on
        );
    }
    if details {
        println!(
            "           Measured by {}, {}. Read on {}.",
            result.origin.measured_by,
            result
                .origin
                .published
                .as_deref()
                .unwrap_or("date not stated"),
            row.checked_on
        );
        println!("           {}", result.origin.url);
        for limit in &result.limits {
            println!("           Note: {limit}");
        }
    }
}

pub(super) fn print_comparison(
    comparison: &Comparison,
    report: Option<(&str, &LabReport)>,
    details: bool,
) {
    println!("Seek time");
    if let Some(conditions) = &comparison.conditions {
        println!(
            "{} ({}), top {}, recall {} or better.",
            conditions.title,
            conditions
                .dataset
                .as_deref()
                .unwrap_or("generated data, with no published name"),
            conditions.k,
            conditions.recall_target
        );
    }
    if let Some((path, report)) = report {
        let pack = report
            .environment
            .get("note.definitions_sequence")
            .map_or("no pack recorded".to_owned(), |sequence| {
                format!("pack {sequence}")
            });
        println!();
        println!("Measured in this run ({path}, {pack})");
        let plain: Vec<_> = comparison
            .measured
            .iter()
            .filter(|speed| !speed.filtered)
            .collect();
        print_speeds(&plain);
        let filtered: Vec<_> = comparison
            .measured
            .iter()
            .filter(|speed| speed.filtered)
            .collect();
        if !filtered.is_empty() {
            println!("  With a filter:");
            print_speeds(&filtered);
        }
        if !comparison.without_speed.is_empty() {
            println!("  No speed to show:");
            for entry in &comparison.without_speed {
                println!("    {}: {}", entry.system.as_str(), entry.reason);
            }
        }
    }
    let same: Vec<_> = comparison
        .published_seek
        .iter()
        .filter(|row| row.same_conditions)
        .collect();
    let context: Vec<_> = comparison
        .published_seek
        .iter()
        .filter(|row| !row.same_conditions)
        .collect();
    if comparison.conditions.is_some() {
        println!();
        if same.is_empty() {
            println!("Published under the same conditions: none. Nothing below can be ranked against this run.");
        } else {
            println!("Published under the same conditions");
            for row in &same {
                print_seek_row(row, details, comparison.conditions.is_some());
            }
        }
    }
    if !context.is_empty() {
        println!();
        println!("Published under other conditions, for context only");
        for row in &context {
            print_seek_row(row, details, comparison.conditions.is_some());
        }
    }
    println!();
    println!("Answer quality");
    println!(
        "{} No ranking is accepted across these benchmarks, because each has its own data, reader and judge, so read a score only beside others from the same benchmark with the same reader and judge.",
        comparison.answers_note
    );
    let mut benchmarks: BTreeMap<&str, Vec<&ListedAnswer>> = BTreeMap::new();
    for row in &comparison.published_answers {
        benchmarks
            .entry(row.result.benchmark.as_str())
            .or_default()
            .push(row);
    }
    for (benchmark, rows) in benchmarks {
        println!();
        println!("{benchmark}");
        for row in rows {
            print_answer_row(row, details);
        }
    }
    if !details {
        println!();
        println!("Add --details for each source's address, notes and what differs.");
    }
}

pub(super) fn compare_command(arguments: &Arguments) -> Result<ExitCode> {
    let home = lab_home()?;
    let keys = home.trusted_keys(&pinned_keys()?)?;
    let store = home.store()?;
    let now = now_unix_s()?;
    if store.status(&keys, now)?.current_sequence.is_none() {
        return Err(NOTHING_INSTALLED.into());
    }
    let files = store.current_files(&keys)?;
    let references = references_of(&files)?;
    let definitions = Definitions::read(&files)?;
    let report = arguments
        .values
        .get("report")
        .map(|path| -> Result<(String, LabReport)> {
            let metadata = std::fs::metadata(path).map_err(|error| format!("{path}: {error}"))?;
            if metadata.len() > MAX_MANIFEST_BYTES {
                return Err(format!("{path} is larger than a report may be"));
            }
            let bytes = std::fs::read(path).map_err(|error| format!("{path}: {error}"))?;
            let report: LabReport =
                serde_json::from_slice(&bytes).map_err(|error| format!("{path}: {error}"))?;
            report
                .validate()
                .map_err(|error| format!("{path}: {error}"))?;
            Ok((path.clone(), report))
        })
        .transpose()?;
    let conditions = report.as_ref().and_then(|(_, report)| {
        definitions
            .tests
            .get(&report.test.id)
            .map(|test| TestConditions {
                test: test.test.id.clone(),
                title: test.test.title.clone(),
                dataset: test.reference_dataset.clone(),
                k: test.workload.k,
                recall_target: test.quality.recall_target,
                // The lab starts each system as a server on a many-core host.
                hardware_class: HardwareClass::MultiCoreServer,
            })
    });
    let comparison = compare(
        &references,
        conditions.as_ref(),
        report.as_ref().map(|(_, report)| report),
        now,
    )?;
    if arguments.json() {
        return print_json(&comparison);
    }
    print_comparison(
        &comparison,
        report
            .as_ref()
            .map(|(path, report)| (path.as_str(), report)),
        arguments.switches.contains("details"),
    );
    Ok(ExitCode::SUCCESS)
}
