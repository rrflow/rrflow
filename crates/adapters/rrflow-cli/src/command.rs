//! The deliberately small primary RRFlow product surface.
//!
//! Data, reasoning, recall, backup, and administration operations are exposed
//! through authenticated generated clients after `serve`; the primary binary
//! does not reopen a raw database or create a second lifecycle authority.

use clap::{Parser, Subcommand, ValueEnum};

pub struct Execution {
    pub text: String,
    pub success: bool,
}

impl From<String> for Execution {
    fn from(text: String) -> Self {
        Self {
            text,
            success: true,
        }
    }
}

#[derive(Parser, Debug)]
#[command(
    name = "rrflow",
    about = "Install, run, discover, and verify one RRFlow reasoning and recall engine",
    long_about = "RRFlow is an independently installable reasoning and recall engine. Install a sealed plan, run its authenticated service, then use generated SDK operations for governed effects."
)]
pub struct Cli {
    /// Emit machine-readable JSON.
    #[arg(long, global = true)]
    pub json: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Command {
    /// Print the immutable RRFlow product and protocol version.
    Version,
    /// Preview or apply one explicit, content-bound installation plan.
    Install {
        #[command(subcommand)]
        action: InstallAction,
    },
    /// Start the installed engine's loopback HTTP service for SDK and UI clients.
    Serve {
        #[arg(long, default_value = ".")]
        project: std::path::PathBuf,
        #[arg(long, default_value = "127.0.0.1:9477")]
        bind: std::net::SocketAddr,
        /// Debug-test surrogate for the distribution executable.
        #[arg(long = "test-distribution-executable", hide = true)]
        test_distribution_executable: Option<std::path::PathBuf>,
    },
    /// Authenticate against a running estate and return UI discovery documents.
    Ready {
        #[arg(long, default_value = ".")]
        project: std::path::PathBuf,
        #[arg(long, default_value = "127.0.0.1:9477")]
        address: std::net::SocketAddr,
    },
    /// Perform bounded, read-only verification of an installed estate.
    Verify {
        #[arg(long, default_value = ".")]
        project: std::path::PathBuf,
        #[arg(long, value_enum, default_value_t = VerifyLevel::Quick)]
        level: VerifyLevel,
        /// Debug-test surrogate for the distribution executable.
        #[arg(long = "test-distribution-executable", hide = true)]
        test_distribution_executable: Option<std::path::PathBuf>,
    },
    /// Preview or apply the publication of a verified restore while the estate is stopped.
    Restore {
        #[command(subcommand)]
        action: RestoreAction,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub enum RestoreAction {
    /// Emit an exact, digest-bound publication plan for one staged restore and perform no writes.
    Plan {
        #[arg(long, default_value = ".")]
        project: std::path::PathBuf,
        /// The restore id the `restore-create` operation staged.
        #[arg(long)]
        restore: String,
        /// The SHA-256 identity of the catalogued backup the restore was staged from.
        #[arg(long)]
        backup: String,
        /// Debug-test surrogate for the distribution executable.
        #[arg(long = "test-distribution-executable", hide = true)]
        test_distribution_executable: Option<std::path::PathBuf>,
    },
    /// Publish only the supplied plan after its SHA-256 is explicitly accepted.
    Apply {
        #[arg(long, default_value = ".")]
        project: std::path::PathBuf,
        #[arg(long)]
        plan: std::path::PathBuf,
        #[arg(long)]
        expect: String,
        /// Debug-test surrogate for the distribution executable.
        #[arg(long = "test-distribution-executable", hide = true)]
        test_distribution_executable: Option<std::path::PathBuf>,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub enum InstallAction {
    /// Generate a new identity input without writing project or engine state.
    Identity,
    /// Emit an exact, deterministic preview and perform no project writes.
    Plan {
        #[arg(long, default_value = ".")]
        project: std::path::PathBuf,
        #[arg(long, value_enum, default_value_t = InstallMode::Existing)]
        mode: InstallMode,
        /// Identity from `install identity`; retain it across previews for this installation.
        #[arg(long)]
        identity: String,
        #[arg(long, default_value = "default")]
        profile: String,
        /// Operator-authored estate ceilings sealed into the installation plan.
        #[arg(long)]
        configuration: Option<std::path::PathBuf>,
        /// Debug-test surrogate for the distribution executable.
        #[arg(long = "test-distribution-executable", hide = true)]
        test_distribution_executable: Option<std::path::PathBuf>,
    },
    /// Apply only the supplied plan after its SHA-256 is explicitly accepted.
    Apply {
        #[arg(long, default_value = ".")]
        project: std::path::PathBuf,
        #[arg(long, value_enum, default_value_t = InstallMode::Existing)]
        mode: InstallMode,
        #[arg(long)]
        plan: std::path::PathBuf,
        #[arg(long)]
        expect: String,
        /// Debug-test surrogate for the distribution executable.
        #[arg(long = "test-distribution-executable", hide = true)]
        test_distribution_executable: Option<std::path::PathBuf>,
    },
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallMode {
    Fresh,
    Existing,
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerifyLevel {
    Quick,
}
