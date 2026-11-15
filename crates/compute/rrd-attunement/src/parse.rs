//! Bounded, pure syntax proposals over bytes supplied by the engine.
//!
//! This module neither opens a project file nor establishes that a source is
//! authorized or committed. The caller must bind the supplied source identity
//! and digest to an accepted Inventory checkpoint before publishing anything.

use rrd_core::digest::sha256_hex;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tree_sitter::{Language, Parser, Tree};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParseLimits {
    pub input_bytes: u64,
    /// Maximum syntax nodes visited, including nodes omitted from the output.
    pub nodes: u64,
    pub depth: u64,
    pub output_bytes: u64,
    pub elapsed_micros: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    UnsupportedLanguage,
    SourceDigestMismatch,
    InvalidSourceIdentity,
    InvalidEncoding,
    LimitExceeded,
    Cancelled,
    ParserUnavailable,
}

fn grammar(language: &str) -> Option<(Language, &'static str)> {
    match language {
        "rust" => Some((
            tree_sitter_rust::LANGUAGE.into(),
            "tree-sitter-0.24.7/rust-0.23.3",
        )),
        "python" => Some((
            tree_sitter_python::LANGUAGE.into(),
            "tree-sitter-0.24.7/python-0.23.6",
        )),
        "go" => Some((
            tree_sitter_go::LANGUAGE.into(),
            "tree-sitter-0.24.7/go-0.23.4",
        )),
        _ => None,
    }
}

/// Languages with a linked grammar, in canonical order. The engine binds
/// these ids to inventory evidence; a language absent here is never parsed.
pub const SUPPORTED_LANGUAGES: [&str; 3] = ["go", "python", "rust"];

/// Identity of the linked grammar, or None when no such parser is installed.
pub fn parser_revision(language: &str) -> Option<&'static str> {
    grammar(language).map(|(_, revision)| revision)
}

/// The selected language is explicit; this function never guesses from a path.
/// Grammars are linked build inputs and are not fetched during execution.
pub(crate) struct Parsed {
    pub tree: Tree,
    pub language: Language,
    pub revision: &'static str,
    pub started: Instant,
    pub deadline: Duration,
}

/// Validates the request and parses `source` within `limits`.
pub(crate) fn parse_tree(
    source_id: &str,
    expected_sha256: &str,
    language: &str,
    source: &[u8],
    limits: &ParseLimits,
    cancelled: &impl Fn() -> bool,
) -> Result<Parsed, ParseError> {
    let started = Instant::now();
    if source_id.is_empty() || source_id.len() > 256 {
        return Err(ParseError::InvalidSourceIdentity);
    }
    if limits.input_bytes == 0
        || limits.nodes == 0
        || limits.depth == 0
        || limits.output_bytes == 0
        || limits.elapsed_micros == 0
        || source.len() as u64 > limits.input_bytes
        || source.len() > u32::MAX as usize
    {
        return Err(ParseError::LimitExceeded);
    }
    if cancelled() {
        return Err(ParseError::Cancelled);
    }
    if sha256_hex(source) != expected_sha256 {
        return Err(ParseError::SourceDigestMismatch);
    }
    std::str::from_utf8(source).map_err(|_| ParseError::InvalidEncoding)?;
    let (grammar, revision) = grammar(language).ok_or(ParseError::UnsupportedLanguage)?;
    let deadline = Duration::from_micros(limits.elapsed_micros);
    if started.elapsed() >= deadline {
        return Err(ParseError::LimitExceeded);
    }
    let mut parser = Parser::new();
    parser
        .set_language(&grammar)
        .map_err(|_| ParseError::ParserUnavailable)?;
    let remaining = deadline.saturating_sub(started.elapsed()).as_micros();
    if remaining == 0 {
        return Err(ParseError::LimitExceeded);
    }
    parser.set_timeout_micros(u64::try_from(remaining).unwrap_or(u64::MAX));
    let mut stopped = None;
    let tree = parser.parse_with(
        &mut |offset, _| {
            if stopped.is_some() {
                &source[0..0]
            } else {
                // Tree-sitter invokes this closure from a C callback. Do not
                // allow an operator-supplied cancellation check to unwind
                // across the FFI boundary.
                match std::panic::catch_unwind(std::panic::AssertUnwindSafe(cancelled)) {
                    Ok(true) => {
                        stopped = Some(ParseError::Cancelled);
                        &source[0..0]
                    }
                    Err(_) => {
                        stopped = Some(ParseError::ParserUnavailable);
                        &source[0..0]
                    }
                    Ok(false) if started.elapsed() >= deadline => {
                        stopped = Some(ParseError::LimitExceeded);
                        &source[0..0]
                    }
                    Ok(false) => source.get(offset..).unwrap_or_default(),
                }
            }
        },
        None,
    );
    if cancelled() {
        return Err(ParseError::Cancelled);
    }
    if let Some(reason) = stopped {
        return Err(reason);
    }
    if started.elapsed() >= deadline {
        return Err(ParseError::LimitExceeded);
    }
    let tree = tree.ok_or(ParseError::LimitExceeded)?;
    Ok(Parsed {
        tree,
        language: grammar,
        revision,
        started,
        deadline,
    })
}
