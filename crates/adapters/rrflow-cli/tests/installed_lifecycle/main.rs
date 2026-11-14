//! Real-process proof for the installed, UI-discoverable walking product.
//!
//! This is one bounded child lifecycle, not a generic process-kill matrix: the
//! exact primary executable publishes its address, serves generated discovery,
//! accepts its generated credential, survives restart, and passes the read-only
//! verifier. The initial process is terminated after acknowledged writes; subsequent
//! Unix shutdowns use graceful SIGINT. Only test-owned children are affected.

#[path = "../support/loopback_worker.rs"]
mod loopback_worker;
#[path = "../support/running_engine.rs"]
mod running_engine;
#[path = "../support/source_worker.rs"]
mod source_worker;

use running_engine::*;
use source_worker::*;

use rrd_client::{ClientConfig, RequestOptions, RrdClient};

use rrd_contract::{
    transaction_operation_sha256, AssessmentInputKind, AssessmentOption,
    AssessmentPracticeIdentity, AssessmentScoreDirection, BackupCoverageSnapshot, BeginTransaction,
    CanonicalId, CloseSession, CommitTransaction, CreateInstanceBackup, CreateSession,
    EndpointAssessmentPolicy, EndpointGenerationSettings, EndpointResponseFormat,
    EndpointTokenParameter, ErrorCode, ExecuteQuery, QueryBudget, QueryResult, QueryValue,
    RestoreInstanceBackup, SessionLimits, TaskAssessmentDecision, TaskAssessmentDescriptor,
    TaskAssessmentRequest, TaskAssessmentValue, TransactionMutation, TASK_ASSESSMENT_VERSION,
};

use rrd_engine::RrdEngine;

use std::collections::BTreeMap;

use std::io::{Read, Write};

use std::net::{TcpListener, TcpStream};

use std::path::{Path, PathBuf};

use std::process::{Command, Output};

use std::time::{Duration, SystemTime, UNIX_EPOCH};

const LOOPBACK_CONFIGURATION_BYTES: &[u8] = include_bytes!(
    "../../../../transport/rrd-contract/tests/fixtures/endpoint-selection-observed/available/configuration.json"
);

const LOOPBACK_ALLOWANCE_BYTES: &[u8] = include_bytes!(
    "../../../../transport/rrd-contract/tests/fixtures/endpoint-selection-observed/available/allowance.json"
);

#[path = "../support/adaptive_installed.rs"]
mod adaptive_installed;

#[path = "../support/process.rs"]
mod process;

#[path = "../support/storage_history.rs"]
mod storage_history;

// Authenticated readiness includes the generated discovery catalogue (>4 MiB).
const OUTPUT_LIMIT: usize = 16 * 1024 * 1024;

#[cfg(unix)]
use std::time::Instant;

struct StandaloneDistribution {
    root: tempfile::TempDir,
    executable: PathBuf,
}

impl StandaloneDistribution {
    fn stage() -> Self {
        let source = std::env::var_os("RRFLOW_TEST_BINARY")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(env!("CARGO_BIN_EXE_rrflow")));
        let root = tempfile::Builder::new()
            .prefix("rrflow standalone ")
            .tempdir()
            .unwrap();
        let executable = root.path().join(if cfg!(windows) {
            "rrflow.exe"
        } else {
            "rrflow"
        });
        process::stage_executable(&source, &executable)
            .expect("stage the actual primary executable");
        for directory in ["working", "home", "temporary", "empty-path"] {
            std::fs::create_dir(root.path().join(directory)).unwrap();
        }
        Self { root, executable }
    }

    fn command(&self) -> Command {
        let mut command = Command::new(&self.executable);
        command
            .env_clear()
            .current_dir(self.root.path().join("working"));
        // Native OS discovery is the only inherited environment. No Cargo,
        // provider, checkout, credential, proxy or developer PATH survives.
        for name in ["SystemRoot", "WINDIR"] {
            if let Some(value) = std::env::var_os(name) {
                command.env(name, value);
            }
        }
        for name in ["HOME", "USERPROFILE"] {
            command.env(name, self.root.path().join("home"));
        }
        for name in ["TMP", "TEMP", "TMPDIR"] {
            command.env(name, self.root.path().join("temporary"));
        }
        command.env("PATH", self.root.path().join("empty-path"));
        command.env("LANG", "C");
        command
    }
}

thread_local! {
    // The fixture outlives this test's child handles and is removed when its
    // test thread exits. No global working-directory/environment mutation.
    static DISTRIBUTION: StandaloneDistribution = StandaloneDistribution::stage();
}

fn binary() -> PathBuf {
    DISTRIBUTION.with(|distribution| distribution.executable.clone())
}

mod commands;
mod engrams_and_history;
mod lifecycle;
mod plans;
mod replan;

use commands::*;
use lifecycle::*;
use replan::*;
