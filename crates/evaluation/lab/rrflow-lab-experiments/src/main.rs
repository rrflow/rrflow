//! Controlled paired evaluation runner. Provider output is evidence, not prose.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Trial {
    provider: String,
    repository: String,
    scenario: String,
    arm: String,
    success: bool,
    expected: String,
    attempts: u64,
    #[serde(default)]
    retries: u64,
    tool_calls: Option<u64>,
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
    latency_ms: u64,
    exit_code: Option<i32>,
    raw_output: String,
    stderr: String,
}

/// Mechanical screening only: this is not an independent quality oracle or
/// proof that the named treatment was actually attached to the provider.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ScreeningSummary {
    format_version: u16,
    evidence_class: String,
    trials: usize,
    successes: usize,
    paired_regressions: usize,
    providers: Vec<String>,
    repositories: Vec<String>,
    trials_detail: Vec<Trial>,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("rrflow-lab-experiments: {error}");
        std::process::exit(1);
    }
}

mod screening;
mod trials;

use screening::*;
use trials::*;
