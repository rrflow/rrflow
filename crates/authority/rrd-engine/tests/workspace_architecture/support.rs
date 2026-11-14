//! Workspace metadata, dependency queries, Git index entries and source collectors.

use super::*;

pub(super) fn workspace_metadata() -> WorkspaceMetadata {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .ancestors()
        .find(|candidate| {
            candidate.join("crates").is_dir() && candidate.join("Cargo.toml").is_file()
        })
        .expect("rrd-engine must live under the workspace crates tree");
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo"));
    let output = Command::new(cargo)
        .current_dir(workspace_root)
        .args(["metadata", "--format-version=1", "--no-deps", "--locked"])
        .output()
        .expect("cargo metadata must start");
    assert!(
        output.status.success(),
        "cargo metadata failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let document: Value =
        serde_json::from_slice(&output.stdout).expect("cargo metadata must return JSON");
    let root = PathBuf::from(
        document["workspace_root"]
            .as_str()
            .expect("cargo metadata must identify the workspace root"),
    );
    let member_ids = document["workspace_members"]
        .as_array()
        .expect("cargo metadata must list workspace members")
        .iter()
        .map(|member| {
            member
                .as_str()
                .expect("workspace member IDs must be strings")
        })
        .collect::<BTreeSet<_>>();
    let package_values = document["packages"]
        .as_array()
        .expect("cargo metadata must list packages");
    let workspace_names = package_values
        .iter()
        .filter(|package| {
            member_ids.contains(package["id"].as_str().expect("package IDs must be strings"))
        })
        .map(|package| {
            package["name"]
                .as_str()
                .expect("package names must be strings")
        })
        .collect::<BTreeSet<_>>();

    assert_eq!(
        workspace_names.len(),
        member_ids.len(),
        "workspace package names must be unique"
    );

    let mut packages = BTreeMap::new();
    let mut target_sources = BTreeSet::new();
    let mut dependency_inputs = Vec::new();
    for package in package_values {
        let id = package["id"].as_str().expect("package IDs must be strings");
        if !member_ids.contains(id) {
            continue;
        }
        let name = package["name"]
            .as_str()
            .expect("package names must be strings");
        let manifest_path = PathBuf::from(
            package["manifest_path"]
                .as_str()
                .expect("package manifest paths must be strings"),
        );
        let manifest = manifest_path
            .strip_prefix(&root)
            .unwrap_or_else(|_| {
                panic!(
                    "workspace package manifest {} must be under {}",
                    manifest_path.display(),
                    root.display()
                )
            })
            .to_path_buf();
        let package_directory_name = manifest_path
            .parent()
            .and_then(Path::file_name)
            .and_then(|value| value.to_str())
            .expect("workspace package manifests must have UTF-8 parent directories");
        assert_eq!(
            package_directory_name, name,
            "workspace package {name} must live in a same-named directory; forwarding package boundaries are forbidden"
        );
        for target in package["targets"]
            .as_array()
            .expect("package targets must be an array")
        {
            let source = PathBuf::from(
                target["src_path"]
                    .as_str()
                    .expect("target source paths must be strings"),
            );
            let relative = source
                .strip_prefix(&root)
                .unwrap_or_else(|_| {
                    panic!(
                        "workspace target source {} must be under {}",
                        source.display(),
                        root.display()
                    )
                })
                .to_path_buf();
            target_sources.insert(relative);
        }
        let mut workspace = BTreeSet::new();
        let mut all = BTreeSet::new();
        let targets = package["targets"]
            .as_array()
            .expect("package targets must be an array")
            .iter()
            .map(|target| {
                target["name"]
                    .as_str()
                    .expect("target names must be strings")
                    .to_owned()
            })
            .collect();
        for dependency in package["dependencies"]
            .as_array()
            .expect("package dependencies must be an array")
        {
            let dependency_name = dependency["name"]
                .as_str()
                .expect("dependency names must be strings");
            dependency_inputs.push(DependencyInput {
                owner: name.to_owned(),
                name: dependency_name.to_owned(),
                kind: dependency["kind"].as_str().unwrap_or("normal").to_owned(),
                optional: dependency["optional"].as_bool().unwrap_or(false),
                features: dependency["features"]
                    .as_array()
                    .expect("dependency features must be an array")
                    .iter()
                    .map(|feature| {
                        feature
                            .as_str()
                            .expect("dependency features must be strings")
                            .to_owned()
                    })
                    .collect(),
                path: dependency["path"].as_str().map(PathBuf::from),
                source: dependency["source"].as_str().map(str::to_owned),
            });
            if workspace_names.contains(dependency_name) {
                assert!(
                    dependency["rename"].is_null(),
                    "workspace package {name} must depend on {dependency_name} by its canonical package name; Cargo rename aliases are forbidden"
                );
            }
            if dependency["kind"].as_str() == Some("dev") {
                continue;
            }
            all.insert(dependency_name.to_owned());
            if workspace_names.contains(dependency_name) {
                workspace.insert(dependency_name.to_owned());
            }
        }
        assert!(
            packages
                .insert(
                    name.to_owned(),
                    PackageDependencies {
                        manifest,
                        description: package["description"]
                            .as_str()
                            .unwrap_or_default()
                            .to_owned(),
                        features: package["features"]
                            .as_object()
                            .expect("package features must be an object")
                            .iter()
                            .map(|(feature, members)| {
                                (
                                    feature.clone(),
                                    members
                                        .as_array()
                                        .expect("feature members must be an array")
                                        .iter()
                                        .map(|member| {
                                            member
                                                .as_str()
                                                .expect("feature members must be strings")
                                                .to_owned()
                                        })
                                        .collect(),
                                )
                            })
                            .collect(),
                        workspace,
                        all,
                        targets,
                    },
                )
                .is_none(),
            "duplicate workspace package name {name}"
        );
    }

    WorkspaceMetadata {
        root,
        packages,
        target_sources,
        dependency_inputs,
    }
}

pub(super) fn production_workspace_dependencies<'a>(
    metadata: &'a WorkspaceMetadata,
    package: &str,
) -> &'a BTreeSet<String> {
    &metadata
        .packages
        .get(package)
        .unwrap_or_else(|| panic!("workspace package {package} is absent"))
        .workspace
}

pub(super) fn production_dependencies<'a>(
    metadata: &'a WorkspaceMetadata,
    package: &str,
) -> &'a BTreeSet<String> {
    &metadata
        .packages
        .get(package)
        .unwrap_or_else(|| panic!("workspace package {package} is absent"))
        .all
}

pub(super) fn forbidden_edges(
    metadata: &WorkspaceMetadata,
    package: &str,
    forbidden: &BTreeSet<String>,
) -> BTreeSet<String> {
    production_workspace_dependencies(metadata, package)
        .intersection(forbidden)
        .cloned()
        .collect()
}

pub(super) fn assert_exact(actual: &BTreeSet<String>, expected: BTreeSet<String>, boundary: &str) {
    let added = actual.difference(&expected).cloned().collect::<Vec<_>>();
    let missing = expected.difference(actual).cloned().collect::<Vec<_>>();
    assert!(
        added.is_empty() && missing.is_empty(),
        "{boundary} changed; added={added:?}, missing={missing:?}, actual={actual:?}"
    );
}

pub(super) fn names(values: &[&str]) -> BTreeSet<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

pub(super) fn noncanonical_ascii_terms(value: &str, canonical_terms: &[&str]) -> Vec<String> {
    let lowercase = value.to_ascii_lowercase();
    let bytes = value.as_bytes();
    let mut violations = Vec::new();
    for canonical in canonical_terms {
        let needle = canonical.to_ascii_lowercase();
        for (offset, _) in lowercase.match_indices(&needle) {
            let end = offset + needle.len();
            let starts_at_boundary = offset == 0
                || (!bytes[offset - 1].is_ascii_alphanumeric() && bytes[offset - 1] != b'_');
            let ends_at_boundary =
                end == bytes.len() || (!bytes[end].is_ascii_alphanumeric() && bytes[end] != b'_');
            if starts_at_boundary && ends_at_boundary && &value[offset..end] != *canonical {
                violations.push(value[offset..end].to_owned());
            }
        }
    }
    violations
}

pub(super) fn git_index_entries(root: &Path) -> Vec<GitIndexEntry> {
    let output = Command::new("git")
        .current_dir(root)
        .args(["ls-files", "--stage", "-z"])
        .output()
        .expect("git ls-files --stage must start");
    assert!(
        output.status.success(),
        "git ls-files --stage failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|record| !record.is_empty())
        .map(|record| {
            let record = std::str::from_utf8(record).expect("repository paths must be valid UTF-8");
            let (metadata, path) = record
                .split_once('\t')
                .expect("git index record must contain a path");
            GitIndexEntry {
                mode: metadata
                    .split_whitespace()
                    .next()
                    .expect("git index record must contain a mode")
                    .to_owned(),
                path: PathBuf::from(path),
            }
        })
        .collect()
}

pub(super) fn is_build_or_install_input(path: &Path) -> bool {
    let encoded = path.to_string_lossy();
    if encoded.starts_with(".cargo/")
        || encoded.starts_with(".github/workflows/")
        || encoded.starts_with("deploy/")
        || encoded.starts_with("scripts/")
        || encoded.contains("/install/")
        || encoded.contains("/templates/")
    {
        return true;
    }
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    matches!(
        name,
        "AGENTS.md"
            | "Cargo.lock"
            | "Cargo.toml"
            | "Dockerfile"
            | "Makefile"
            | "build.rs"
            | "clippy.toml"
            | "deny.toml"
            | "global.json"
            | "go.mod"
            | "go.sum"
            | "package.json"
            | "pnpm-lock.yaml"
            | "pom.xml"
            | "pyproject.toml"
            | "ruff.toml"
            | "rust-toolchain"
            | "rust-toolchain.toml"
            | "rustfmt.toml"
            | "uv.lock"
    ) || ["csproj", "props", "slnx", "targets"]
        .iter()
        .any(|extension| path.extension().is_some_and(|value| value == *extension))
}

pub(super) fn is_active_product_text(path: &Path) -> bool {
    let encoded = path.to_string_lossy();
    if path == Path::new("README.md") {
        return true;
    }
    if !encoded.starts_with("crates/") && !encoded.starts_with("docs/") {
        return false;
    }
    matches!(
        path.extension().and_then(|extension| extension.to_str()),
        Some("json" | "jsonl" | "md" | "rs" | "toml")
    )
}

pub(super) fn contains_host_specific_absolute_path(source: &str) -> bool {
    let lower = source.to_ascii_lowercase();
    if ["/workspace/", "/users/", "/tmp/", "file:///"]
        .iter()
        .any(|value| lower.contains(value))
    {
        return true;
    }
    if lower.match_indices("/home/").any(|(offset, _)| {
        let owner = lower[offset + "/home/".len()..]
            .split(['/', '\\', ' ', '\"', '\'', '\n', '\r', '\t'])
            .next()
            .unwrap_or_default();
        owner != "runner"
    }) {
        return true;
    }
    lower.as_bytes().windows(9).any(|window| {
        window[0].is_ascii_alphabetic()
            && window[1] == b':'
            && matches!(window[2], b'/' | b'\\')
            && &window[3..8] == b"users"
            && matches!(window[8], b'/' | b'\\')
    })
}

pub(super) fn collect_rust_sources(
    directory: &Path,
    violations: &mut Vec<PathBuf>,
    forbidden: &str,
) {
    let mut entries = fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("cannot inspect {}: {error}", directory.display()))
        .collect::<Result<Vec<_>, _>>()
        .unwrap_or_else(|error| panic!("cannot enumerate {}: {error}", directory.display()));
    entries.sort_by_key(fs::DirEntry::path);

    for entry in entries {
        let path = entry.path();
        let file_type = entry
            .file_type()
            .unwrap_or_else(|error| panic!("cannot inspect {}: {error}", path.display()));
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            collect_rust_sources(&path, violations, forbidden);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            let source = fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
            if source
                .to_ascii_lowercase()
                .contains(&forbidden.to_ascii_lowercase())
            {
                violations.push(path);
            }
        }
    }
}

pub(super) fn collect_authority_opening_violations(crates: &Path, violations: &mut Vec<PathBuf>) {
    collect_authority_package_tree(crates, crates, violations);
}

pub(super) fn collect_authority_package_tree(
    crates: &Path,
    directory: &Path,
    violations: &mut Vec<PathBuf>,
) {
    if directory.join("Cargo.toml").is_file() {
        let name = directory
            .file_name()
            .expect("workspace package directory must have a name");
        if name == "rrd-engine" || name == "rrd-store" {
            return;
        }
        let source = directory.join("src");
        if source.is_dir() {
            collect_authority_source_tree(crates, &source, violations);
        }
        return;
    }

    let mut packages = fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("cannot inspect {}: {error}", directory.display()))
        .collect::<Result<Vec<_>, _>>()
        .unwrap_or_else(|error| panic!("cannot enumerate {}: {error}", directory.display()));
    packages.sort_by_key(fs::DirEntry::path);
    for package in packages {
        let file_type = package
            .file_type()
            .unwrap_or_else(|error| panic!("cannot inspect {}: {error}", package.path().display()));
        if file_type.is_symlink() || !file_type.is_dir() {
            continue;
        }
        collect_authority_package_tree(crates, &package.path(), violations);
    }
}

pub(super) fn collect_physical_import_violations(
    root: &Path,
    directory: &Path,
    physical: &[&str],
    violations: &mut Vec<PathBuf>,
) {
    let mut entries = fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("cannot inspect {}: {error}", directory.display()))
        .collect::<Result<Vec<_>, _>>()
        .unwrap_or_else(|error| panic!("cannot enumerate {}: {error}", directory.display()));
    entries.sort_by_key(fs::DirEntry::path);
    for entry in entries {
        let path = entry.path();
        let file_type = entry
            .file_type()
            .unwrap_or_else(|error| panic!("cannot inspect {}: {error}", path.display()));
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            collect_physical_import_violations(root, &path, physical, violations);
            continue;
        }
        if path.extension().is_none_or(|extension| extension != "rs") {
            continue;
        }
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
        if physical.iter().any(|component| {
            source.contains(&format!("use {component}"))
                || source.contains(&format!("{component}::"))
                || source.contains(&format!("extern crate {component}"))
        }) {
            violations.push(
                path.strip_prefix(root)
                    .expect("outward source must be workspace-relative")
                    .to_path_buf(),
            );
        }
    }
}

pub(super) fn collect_authority_source_tree(
    crates: &Path,
    directory: &Path,
    violations: &mut Vec<PathBuf>,
) {
    let mut entries = fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("cannot inspect {}: {error}", directory.display()))
        .collect::<Result<Vec<_>, _>>()
        .unwrap_or_else(|error| panic!("cannot enumerate {}: {error}", directory.display()));
    entries.sort_by_key(fs::DirEntry::path);
    for entry in entries {
        let path = entry.path();
        let file_type = entry
            .file_type()
            .unwrap_or_else(|error| panic!("cannot inspect {}: {error}", path.display()));
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            collect_authority_source_tree(crates, &path, violations);
            continue;
        }
        if path.extension().is_none_or(|extension| extension != "rs") {
            continue;
        }
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
        // Source-level unit fixtures are allowed to inspect persistence. The
        // product surface before the first cfg(test) boundary is not.
        let production = source.split("#[cfg(test)]").next().unwrap_or(&source);
        if [
            "RrflowKvStore::open(",
            "EmbeddedOperator",
            ".runtime_store(",
            "rrd_engine::operator",
        ]
        .iter()
        .any(|forbidden| production.contains(forbidden))
        {
            violations.push(
                path.strip_prefix(crates.parent().expect("crates has a workspace parent"))
                    .expect("crate source must be workspace-relative")
                    .to_path_buf(),
            );
        }
    }
}

pub(super) fn collect_identity_violations(
    root: &Path,
    violations: &mut Vec<PathBuf>,
    forbidden: &[&[u8]],
) {
    let output = Command::new("git")
        .current_dir(root)
        .args([
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
        ])
        .output()
        .expect("git ls-files must start");
    assert!(
        output.status.success(),
        "git ls-files failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    for encoded in output.stdout.split(|byte| *byte == 0) {
        if encoded.is_empty() {
            continue;
        }
        let relative = PathBuf::from(
            std::str::from_utf8(encoded).expect("repository paths must be valid UTF-8"),
        );
        let path = root.join(&relative);
        if !path.is_file() {
            continue;
        }

        let path_bytes = relative.to_string_lossy().to_ascii_lowercase();
        let bytes = fs::read(&path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
        let lowercase_bytes = bytes.iter().map(u8::to_ascii_lowercase).collect::<Vec<_>>();
        if forbidden.iter().any(|value| {
            contains_identity(path_bytes.as_bytes(), value)
                || contains_identity(&lowercase_bytes, value)
        }) {
            violations.push(relative);
        }
    }
}

pub(super) fn contains_identity(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() {
        return false;
    }
    let skill_prefix = [b"rrflow".as_slice(), b"-engine".as_slice()].concat();
    haystack
        .windows(needle.len())
        .enumerate()
        .any(|(offset, window)| {
            if window != needle {
                return false;
            }
            needle != skill_prefix
                || !haystack[offset + needle.len()..].starts_with(b"-development")
        })
}

/// A module's whole source: the file plus every file of its child-module folder, in path order.
///
/// Rules about a module's source hold for the module however it is split into files. A crate root
/// (`lib.rs` or `main.rs`) stands for every source file of its crate.
pub(super) fn module_source(root: &Path, relative: &str) -> String {
    fn collect(directory: &Path, files: &mut Vec<PathBuf>) {
        let Ok(entries) = fs::read_dir(directory) else {
            return;
        };
        for entry in entries {
            let path = entry.expect("module source entry must be readable").path();
            if path.is_dir() {
                collect(&path, files);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                files.push(path);
            }
        }
    }
    let file = root.join(relative);
    let mut files = Vec::new();
    if matches!(
        file.file_name().and_then(|name| name.to_str()),
        Some("lib.rs" | "main.rs")
    ) {
        collect(
            file.parent().expect("crate root has a source directory"),
            &mut files,
        );
    } else {
        files.push(file.clone());
        collect(&file.with_extension(""), &mut files);
    }
    files.sort();
    files
        .iter()
        .map(|path| fs::read_to_string(path).expect("module source must be readable"))
        .collect::<Vec<_>>()
        .join("\n")
}
