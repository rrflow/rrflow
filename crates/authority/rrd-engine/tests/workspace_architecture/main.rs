use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

struct WorkspaceMetadata {
    root: PathBuf,
    packages: BTreeMap<String, PackageDependencies>,
    target_sources: BTreeSet<PathBuf>,
    dependency_inputs: Vec<DependencyInput>,
}

struct PackageDependencies {
    manifest: PathBuf,
    description: String,
    features: BTreeMap<String, BTreeSet<String>>,
    workspace: BTreeSet<String>,
    all: BTreeSet<String>,
    targets: BTreeSet<String>,
}

struct DependencyInput {
    owner: String,
    name: String,
    kind: String,
    optional: bool,
    features: BTreeSet<String>,
    path: Option<PathBuf>,
    source: Option<String>,
}

struct GitIndexEntry {
    mode: String,
    path: PathBuf,
}

mod dependencies;
mod retired_and_inputs;
mod storage;
mod submodules;
mod support;
mod text;

use submodules::*;
use support::*;
