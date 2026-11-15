//! RRFlow's primary installed-product entry point.
//!
//! The binary owns no raw database or provider-specific lifecycle. It plans
//! and applies the engine-owned installation contract, opens that installed
//! engram for service, authenticates live readiness, and verifies it read-only.

mod build_identity;
mod build_metadata;
mod command;
mod installed;

use clap::Parser;
use command::Cli;

/// Traces are opt-in and always use stderr so stdout remains the product's
/// answer/protocol channel. JSON traces are selected independently of command
/// output with `RRFLOW_TRACE_FORMAT=json`.
fn install_tracing() {
    let Ok(filter) = std::env::var("RRFLOW_TRACE") else {
        return;
    };
    let builder = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::new(filter))
        .with_writer(std::io::stderr);
    if std::env::var("RRFLOW_TRACE_FORMAT").as_deref() == Ok("json") {
        builder.json().init();
    } else {
        builder.init();
    }
}

fn main() -> std::process::ExitCode {
    // The parser child must not initialize tracing or emit source-bearing
    // diagnostics on its framed stdout. Its parent owns resource enforcement.
    let mut args = std::env::args_os();
    let _ = args.next();
    if args.next().as_deref() == Some(std::ffi::OsStr::new("--rrflow-internal-parse-worker-v1")) {
        return if args.next().is_none() && rrd_engine::run_internal_parse_worker().is_ok() {
            std::process::ExitCode::SUCCESS
        } else {
            std::process::ExitCode::FAILURE
        };
    }
    install_tracing();
    let cli = Cli::parse();
    finish(installed::execute(&cli.command, cli.json))
}

fn finish(
    result: Result<command::Execution, Box<dyn std::error::Error>>,
) -> std::process::ExitCode {
    match result {
        Ok(execution) => {
            if !execution.text.is_empty() {
                println!("{}", execution.text);
            }
            if execution.success {
                std::process::ExitCode::SUCCESS
            } else {
                std::process::ExitCode::FAILURE
            }
        }
        Err(error) => {
            eprintln!("error: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
