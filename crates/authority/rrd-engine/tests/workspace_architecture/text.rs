//! Canonical manifest descriptions, rrflowQL spelling and host-independent text.

use super::*;

#[test]
pub(super) fn workspace_manifest_descriptions_use_canonical_product_spelling() {
    let metadata = workspace_metadata();
    let canonical_terms = [
        "rrflowDB",
        "rrflowKV",
        "rrflowMX",
        "rrflowQL",
        "RRFlow",
        "RRD",
        "DataFusion",
    ];
    let mut violations = Vec::new();
    for (package_name, package) in &metadata.packages {
        for term in noncanonical_ascii_terms(&package.description, &canonical_terms) {
            violations.push(format!(
                "{package_name} description uses {term:?} instead of its canonical spelling"
            ));
        }
    }
    violations.sort();
    assert!(
        violations.is_empty(),
        "workspace package metadata has non-canonical product spelling: {violations:#?}"
    );
}

#[test]
pub(super) fn active_product_text_uses_canonical_rrflowql_spelling() {
    let metadata = workspace_metadata();
    let forbidden = ["RRFlow", "QL"].concat();
    let mut violations = Vec::new();
    for entry in git_index_entries(&metadata.root) {
        if !is_active_product_text(&entry.path) {
            continue;
        }
        let path = metadata.root.join(&entry.path);
        // `git ls-files --stage` still reports an intentionally deleted path
        // until the convergence commit is created. Deleted source is not active
        // product text; the repository-closure checks still reject untracked
        // replacement targets and the architecture absence test names each
        // forbidden pre-canonical path explicitly.
        if !path.exists() {
            continue;
        }
        let bytes = fs::read(&path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
        if bytes
            .windows(forbidden.len())
            .any(|window| window == forbidden.as_bytes())
        {
            violations.push(entry.path);
        }
    }
    violations.sort();
    assert!(
        violations.is_empty(),
        "active RRFlow source or documentation uses a non-canonical rrflowQL spelling: {violations:#?}"
    );
}

#[test]
pub(super) fn architecture_text_detectors_reject_host_and_spelling_drift() {
    let canonical_terms = [
        "rrflowDB",
        "rrflowKV",
        "rrflowMX",
        "rrflowQL",
        "RRFlow",
        "RRD",
        "DataFusion",
    ];
    assert!(noncanonical_ascii_terms("RRFlow query engine", &canonical_terms).is_empty());
    assert!(noncanonical_ascii_terms("rrflowQL query engine", &canonical_terms).is_empty());
    assert_eq!(
        noncanonical_ascii_terms("rrflowql query engine", &canonical_terms),
        vec!["rrflowql"]
    );
    assert!(contains_host_specific_absolute_path(
        "tool = /workspace/operator/private-generator"
    ));
    assert!(contains_host_specific_absolute_path(
        "tool = C:\\Users\\operator\\private-generator.exe"
    ));
    assert!(!contains_host_specific_absolute_path(
        "tool = scripts/generate.py"
    ));
}

#[test]
pub(super) fn active_build_and_install_inputs_are_host_independent() {
    let metadata = workspace_metadata();
    let mut violations = Vec::new();
    for entry in git_index_entries(&metadata.root) {
        if !is_build_or_install_input(&entry.path) {
            continue;
        }
        let path = metadata.root.join(&entry.path);
        let bytes = fs::read(&path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
        let source = std::str::from_utf8(&bytes).unwrap_or_else(|error| {
            panic!(
                "active build/install input {} must be UTF-8 text: {error}",
                entry.path.display()
            )
        });
        if contains_host_specific_absolute_path(source) {
            violations.push(entry.path);
        }
    }
    violations.sort();
    assert!(
        violations.is_empty(),
        "active build/install inputs contain a host-specific absolute path: {violations:#?}"
    );
}
