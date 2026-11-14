//! Auxiliary executables, retired vocabulary and repository-closed source inputs.

use super::*;

#[test]
pub(super) fn outward_cli_owns_current_auxiliary_executables_while_physical_crates_own_none() {
    let metadata = workspace_metadata();
    let product_executables = names(&[
        "rrd-backup-controller",
        "rrd-estate-admin",
        "rrd-estate-controller",
        "rrd-recovery-controller",
    ]);
    let cli_targets = &metadata.packages["rrflow-cli"].targets;
    let missing = product_executables
        .difference(cli_targets)
        .cloned()
        .collect::<Vec<_>>();
    assert!(
        missing.is_empty(),
        "rrflow-cli must own every thin product executable; missing={missing:?}"
    );
    assert!(
        !cli_targets.contains("rrd-security-bootstrap"),
        "standalone security bootstrap must not remain a Cargo target"
    );
    for package in ["rrd-estate", "rrd-security"] {
        let unexpected = metadata.packages[package]
            .targets
            .intersection(&product_executables)
            .cloned()
            .collect::<Vec<_>>();
        assert!(
            unexpected.is_empty(),
            "{package} must remain a physical component library, not an independent product authority; targets={unexpected:?}"
        );
    }
}

#[test]
pub(super) fn mcp_uses_the_authoritative_context_operation() {
    let metadata = workspace_metadata();
    let mcp = fs::read_to_string(metadata.root.join("crates/adapters/rrflow-mcp/src/main.rs"))
        .expect("MCP source must be readable");
    assert!(
        mcp.contains("rrflow_context") && mcp.contains("authority.assemble_context"),
        "MCP must expose only the engine context assembly boundary"
    );
}

#[test]
pub(super) fn engine_has_no_transport_dependencies() {
    let metadata = workspace_metadata();
    let forbidden = names(&[
        "axum",
        "hyper",
        "hyper-util",
        "k8s-openapi",
        "kube",
        "reqwest",
        "rustls",
        "rustls-pemfile",
        "tiny_http",
        "tokio-rustls",
        "tonic",
    ]);
    let observed = production_dependencies(&metadata, "rrd-engine")
        .intersection(&forbidden)
        .cloned()
        .collect::<BTreeSet<_>>();
    assert!(
        observed.is_empty(),
        "rrd-engine must remain transport-independent; forbidden direct dependencies: {observed:?}"
    );
}

#[test]
pub(super) fn retired_service_name_is_absent() {
    let metadata = workspace_metadata();
    let forbidden = ["Rrd", "Service"].concat();
    let mut violations = Vec::new();

    for relative in ["crates/authority/rrd-engine", "crates/transport/rrd-server"] {
        collect_rust_sources(&metadata.root.join(relative), &mut violations, &forbidden);
    }

    violations.sort();
    assert!(
        violations.is_empty(),
        "the retired service symbol must not survive as an alias, declaration, import, or use; found in: {violations:#?}"
    );
}

#[test]
pub(super) fn retired_fixed_reasoning_ledger_api_is_absent() {
    let metadata = workspace_metadata();
    let retired_symbols = [
        ["Reasoning", "Run"].concat(),
        ["Reasoning", "Payload"].concat(),
        ["Reasoning", "State"].concat(),
        ["Decision", "Kind"].concat(),
        ["reasoning_", "run_v1"].concat(),
        ["reasoning ", "ledger"].concat(),
    ];
    let mut violations = Vec::new();
    for relative in [
        "crates/kernel/rrd-core",
        "crates/authority/rrd-engine",
        "crates/adapters/rrflow-cli",
    ] {
        for retired in &retired_symbols {
            collect_rust_sources(&metadata.root.join(relative), &mut violations, retired);
        }
    }
    violations.sort();
    violations.dedup();
    assert!(
        violations.is_empty(),
        "the retired fixed-stage router API remains in: {violations:#?}"
    );
}

#[test]
pub(super) fn retired_function_authority_vocabulary_is_absent() {
    let metadata = workspace_metadata();
    let retired_symbols = [
        ["Automation", "Catalogue"].concat(),
        ["ReplaceAutomation", "Catalogue"].concat(),
        ["ListAutomation", "Catalogue"].concat(),
        ["Automation", "Head"].concat(),
        ["Function", "Trigger"].concat(),
        ["automation_", "catalogue"].concat(),
        ["automation_", "revision"].concat(),
        ["trigger_", "id"].concat(),
    ];
    let mut violations = Vec::new();
    for relative in [
        "crates/transport/rrd-contract",
        "crates/authority/rrd-engine",
    ] {
        for retired in &retired_symbols {
            collect_rust_sources(&metadata.root.join(relative), &mut violations, retired);
        }
    }
    violations.sort();
    violations.dedup();
    assert!(
        violations.is_empty(),
        "retired function-catalogue or transaction-binding vocabulary remains in: {violations:#?}"
    );
    assert!(
        !metadata
            .root
            .join("crates/authority/rrd-engine/src/engine")
            .join(["auto", "mation.rs"].concat())
            .exists(),
        "the overloaded engine function module must be removed"
    );
}

#[test]
pub(super) fn retired_pre_release_identity_is_absent_from_the_active_repository() {
    let metadata = workspace_metadata();
    let retired_brand = ["vy", "rm"].concat();
    let stale_composition = ["rrflow", "-engine"].concat();
    let stale_runtime = ["rrflow", "-runtime"].concat();
    let mut violations = Vec::new();
    collect_identity_violations(
        &metadata.root,
        &mut violations,
        &[
            retired_brand.as_bytes(),
            stale_composition.as_bytes(),
            stale_runtime.as_bytes(),
        ],
    );
    violations.sort();
    violations.dedup();
    assert!(
        violations.is_empty(),
        "retired or split-engine identity remains in the active repository: {violations:#?}"
    );
}

#[test]
pub(super) fn every_workspace_target_source_is_tracked() {
    let metadata = workspace_metadata();
    let output = Command::new("git")
        .current_dir(&metadata.root)
        .args(["ls-files", "-z", "--cached"])
        .output()
        .expect("git ls-files must start");
    assert!(
        output.status.success(),
        "git ls-files failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let tracked = output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|path| !path.is_empty())
        .map(|path| {
            PathBuf::from(std::str::from_utf8(path).expect("repository paths must be UTF-8"))
        })
        .collect::<BTreeSet<_>>();
    let missing = metadata
        .target_sources
        .difference(&tracked)
        .cloned()
        .collect::<Vec<_>>();

    assert!(
        missing.is_empty(),
        "Cargo target sources must exist in a clean checkout; untracked targets: {missing:#?}"
    );
}

#[test]
pub(super) fn workspace_source_inputs_are_repository_closed() {
    let metadata = workspace_metadata();
    let canonical_root = fs::canonicalize(&metadata.root)
        .unwrap_or_else(|error| panic!("cannot resolve {}: {error}", metadata.root.display()));
    let member_roots = metadata
        .packages
        .values()
        .map(|package| {
            let directory = metadata
                .root
                .join(&package.manifest)
                .parent()
                .expect("workspace package manifest must have a parent")
                .to_path_buf();
            fs::canonicalize(&directory)
                .unwrap_or_else(|error| panic!("cannot resolve {}: {error}", directory.display()))
        })
        .collect::<BTreeSet<_>>();

    let mut violations = Vec::new();
    let ui_root = metadata.root.join("connectome");
    let canonical_ui =
        fs::canonicalize(&ui_root).unwrap_or_else(|_| canonical_root.join("connectome"));
    for member in &member_roots {
        if member.starts_with(&canonical_ui) {
            violations.push(format!(
                "engine workspace member enters optional UI source at {}",
                member.display()
            ));
        }
    }
    for source in &metadata.target_sources {
        let path = metadata.root.join(source);
        let canonical = fs::canonicalize(&path)
            .unwrap_or_else(|error| panic!("cannot resolve {}: {error}", path.display()));
        if source.starts_with("connectome") || canonical.starts_with(&canonical_ui) {
            violations.push(format!(
                "Cargo target enters optional UI source at {}",
                source.display()
            ));
        }
        if !canonical.starts_with(&canonical_root) {
            violations.push(format!(
                "Cargo target {} resolves outside the repository to {}",
                source.display(),
                canonical.display()
            ));
        }
    }

    for dependency in &metadata.dependency_inputs {
        if dependency
            .source
            .as_deref()
            .is_some_and(|source| source.starts_with("git+"))
        {
            violations.push(format!(
                "{} depends on Git source {} as {}",
                dependency.owner,
                dependency.source.as_deref().unwrap_or_default(),
                dependency.name
            ));
        }
        let Some(path) = &dependency.path else {
            continue;
        };
        let canonical = fs::canonicalize(path)
            .unwrap_or_else(|error| panic!("cannot resolve {}: {error}", path.display()));
        if canonical.starts_with(&canonical_ui) {
            violations.push(format!(
                "{} depends on optional UI source at {}",
                dependency.owner,
                path.display()
            ));
        }
        if !canonical.starts_with(&canonical_root) {
            violations.push(format!(
                "{} depends on local package {} outside the repository at {}",
                dependency.owner,
                dependency.name,
                canonical.display()
            ));
        } else if !member_roots.contains(&canonical) {
            violations.push(format!(
                "{} depends on local package {} at {}, but that package is not a workspace member",
                dependency.owner,
                dependency.name,
                canonical.display()
            ));
        }
    }

    let lock_path = metadata.root.join("Cargo.lock");
    let lock = fs::read_to_string(&lock_path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", lock_path.display()));
    if lock
        .lines()
        .any(|line| line.trim_start().starts_with("source = \"git+"))
    {
        violations.push("Cargo.lock contains a transitive Git source".to_owned());
    }

    let mut gitlinks = BTreeSet::new();
    for entry in git_index_entries(&metadata.root) {
        if entry.mode == "160000" {
            gitlinks.insert(entry.path.clone());
        }
        if entry.mode != "120000" {
            continue;
        }
        let path = metadata.root.join(&entry.path);
        match fs::canonicalize(&path) {
            Ok(canonical) if canonical.starts_with(&canonical_root) => {}
            Ok(canonical) => violations.push(format!(
                "tracked symlink {} escapes the repository to {}",
                entry.path.display(),
                canonical.display()
            )),
            Err(error) => violations.push(format!(
                "tracked symlink {} cannot be resolved: {error}",
                entry.path.display()
            )),
        }
    }
    violations.extend(submodule_violations(
        &gitlinks,
        &declared_submodules(&metadata.root),
    ));

    violations.sort();
    assert!(
        violations.is_empty(),
        "RRFlow source must come from this workspace or a declared, pinned HTTPS submodule, without Git dependencies or escaping-path inputs: {violations:#?}"
    );
}

#[test]
pub(super) fn connectome_ui_is_pinned_to_its_development_repository() {
    let metadata = workspace_metadata();
    let entries = git_index_entries(&metadata.root);
    assert!(entries
        .iter()
        .any(|entry| entry.mode == "100644" && entry.path == Path::new(".gitmodules")));
    assert!(entries
        .iter()
        .any(|entry| entry.mode == "160000" && entry.path == Path::new("connectome")));
    let declarations = declared_submodules(&metadata.root);
    let connectome = declarations
        .iter()
        .filter(|declaration| declaration.name == "connectome")
        .collect::<Vec<_>>();
    assert_eq!(
        connectome.len(),
        1,
        "Connectome must have one named declaration"
    );
    assert_eq!(connectome[0].path.as_deref(), Some("connectome"));
    assert_eq!(
        connectome[0].url.as_deref(),
        Some("https://github.com/rrflow/connectome-development.git")
    );
}
