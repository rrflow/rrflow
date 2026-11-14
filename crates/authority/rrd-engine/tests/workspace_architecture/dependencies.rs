//! Package grouping, dependency edges and outward boundaries.

use super::*;

#[test]
pub(super) fn workspace_packages_use_the_canonical_grouped_layout() {
    let metadata = workspace_metadata();
    let expected = [
        ("rrd-core", "crates/kernel/rrd-core/Cargo.toml"),
        ("rrd-lsm", "crates/persistence/rrd-lsm/Cargo.toml"),
        ("rrd-store", "crates/persistence/rrd-store/Cargo.toml"),
        ("rrd-query", "crates/compute/rrd-query/Cargo.toml"),
        ("rrd-vector", "crates/compute/rrd-vector/Cargo.toml"),
        ("rrd-inference", "crates/compute/rrd-inference/Cargo.toml"),
        ("rrd-attunement", "crates/compute/rrd-attunement/Cargo.toml"),
        ("rrd-security", "crates/authority/rrd-security/Cargo.toml"),
        ("rrd-estate", "crates/authority/rrd-estate/Cargo.toml"),
        ("rrd-engine", "crates/authority/rrd-engine/Cargo.toml"),
        ("rrd-contract", "crates/transport/rrd-contract/Cargo.toml"),
        ("rrd-client", "crates/transport/rrd-client/Cargo.toml"),
        ("rrd-server", "crates/transport/rrd-server/Cargo.toml"),
        ("rrflow-cli", "crates/adapters/rrflow-cli/Cargo.toml"),
        ("rrflow-mcp", "crates/adapters/rrflow-mcp/Cargo.toml"),
        ("rrflow-edge", "crates/adapters/rrflow-edge/Cargo.toml"),
        ("rrd-cluster", "crates/operations/rrd-cluster/Cargo.toml"),
        (
            "rrd-kubernetes",
            "crates/operations/rrd-kubernetes/Cargo.toml",
        ),
        (
            "rrd-maintenance",
            "crates/operations/rrd-maintenance/Cargo.toml",
        ),
        (
            "rrd-operator-knowledge",
            "crates/operations/rrd-operator-knowledge/Cargo.toml",
        ),
        (
            "rrd-observation",
            "crates/operations/rrd-observation/Cargo.toml",
        ),
        (
            "rrflow-evaluation-core",
            "crates/evaluation/rrflow-evaluation-core/Cargo.toml",
        ),
        (
            "rrflow-lab-systems",
            "crates/evaluation/lab/rrflow-lab-systems/Cargo.toml",
        ),
        (
            "rrflow-lab-datasets",
            "crates/evaluation/lab/rrflow-lab-datasets/Cargo.toml",
        ),
        (
            "rrflow-lab-benchmarks",
            "crates/evaluation/lab/rrflow-lab-benchmarks/Cargo.toml",
        ),
        (
            "rrflow-lab-evals",
            "crates/evaluation/lab/rrflow-lab-evals/Cargo.toml",
        ),
        (
            "rrflow-lab-definitions",
            "crates/evaluation/lab/rrflow-lab-definitions/Cargo.toml",
        ),
        ("rrflow-lab", "crates/evaluation/lab/rrflow-lab/Cargo.toml"),
        (
            "rrflow-lab-experiments",
            "crates/evaluation/lab/rrflow-lab-experiments/Cargo.toml",
        ),
        (
            "rrflow-qualification-storage",
            "crates/evaluation/qualification/rrflow-qualification-storage/Cargo.toml",
        ),
    ]
    .into_iter()
    .map(|(name, path)| (name.to_owned(), PathBuf::from(path)))
    .collect::<BTreeMap<_, _>>();
    let actual = metadata
        .packages
        .iter()
        .map(|(name, package)| (name.clone(), package.manifest.clone()))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(actual, expected, "workspace package paths changed");

    let groups = fs::read_dir(metadata.root.join("crates"))
        .expect("crates directory must be readable")
        .collect::<Result<Vec<_>, _>>()
        .expect("crates entries must be readable")
        .into_iter()
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .map(|entry| {
            entry
                .file_name()
                .into_string()
                .expect("crate group names must be UTF-8")
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(
        groups,
        names(&[
            "adapters",
            "authority",
            "compute",
            "evaluation",
            "kernel",
            "operations",
            "persistence",
            "transport",
        ]),
        "crates must contain only the canonical source groups"
    );
}

/// RRFlow Lab's crates by dependency tier. A lab crate may depend only on
/// crates in a lower tier; the `rrflow-lab` tool alone composes every domain.
pub(super) const LAB_TIERS: [&[&str]; 4] = [
    &["rrflow-lab-systems", "rrflow-lab-datasets"],
    &[
        "rrflow-lab-benchmarks",
        "rrflow-lab-experiments",
        "rrflow-lab-evals",
        "rrflow-lab-probes",
        "rrflow-lab-tuning",
    ],
    // A definition pack carries tests, systems and references, so it reads the
    // benchmark catalog and references below it.
    &["rrflow-lab-definitions"],
    &["rrflow-lab"],
];

/// Where an evaluation crate sits: the shared layer, RRFlow Lab, or release
/// qualification. Product crates are outside it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum EvaluationArea {
    Shared,
    Lab,
    Qualification,
}

pub(super) fn evaluation_area(manifest: &Path) -> Option<EvaluationArea> {
    let parts = manifest
        .components()
        .map(|part| part.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    match parts
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["crates", "evaluation", "lab", ..] => Some(EvaluationArea::Lab),
        ["crates", "evaluation", "qualification", ..] => Some(EvaluationArea::Qualification),
        ["crates", "evaluation", ..] => Some(EvaluationArea::Shared),
        _ => None,
    }
}

/// The evaluation layer measures RRFlow from outside: a product crate may use
/// it only in its own tests. RRFlow Lab and release qualification share
/// `rrflow-evaluation-core` and never depend on each other, a lab crate depends
/// only on lower tiers, and a crate's name follows where it sits.
#[test]
pub(super) fn evaluation_layer_is_measured_from_outside_and_keeps_its_parts_apart() {
    let metadata = workspace_metadata();
    let area = |name: &str| evaluation_area(&metadata.packages[name].manifest);
    let lab_tier = |name: &str| LAB_TIERS.iter().position(|tier| tier.contains(&name));
    for name in metadata.packages.keys() {
        match area(name) {
            Some(EvaluationArea::Lab) => assert!(
                lab_tier(name).is_some(),
                "lab crate {name} must be named rrflow-lab or rrflow-lab-<domain> and placed in LAB_TIERS"
            ),
            Some(EvaluationArea::Qualification) => assert!(
                name.starts_with("rrflow-qualification-"),
                "qualification crate {name} must be named rrflow-qualification-<gate>"
            ),
            Some(EvaluationArea::Shared) | None => {}
        }
    }
    for dependency in &metadata.dependency_inputs {
        if !metadata.packages.contains_key(&dependency.name) {
            continue;
        }
        let (owner, used) = (dependency.owner.as_str(), dependency.name.as_str());
        let (from, to) = (area(owner), area(used));
        if to.is_some() && from.is_none() {
            assert_eq!(
                dependency.kind, "dev",
                "{owner} may use evaluation crate {used} only in its tests"
            );
        }
        assert!(
            !matches!(
                (from, to),
                (
                    Some(EvaluationArea::Lab),
                    Some(EvaluationArea::Qualification)
                ) | (
                    Some(EvaluationArea::Qualification),
                    Some(EvaluationArea::Lab)
                )
            ),
            "{owner} and {used} cross between RRFlow Lab and release qualification"
        );
        if owner == "rrflow-evaluation-core" {
            assert!(to.is_none(), "rrflow-evaluation-core depends on {used}");
        }
        if let (Some(EvaluationArea::Lab), Some(EvaluationArea::Lab)) = (from, to) {
            assert!(
                lab_tier(used) < lab_tier(owner),
                "lab crate {owner} depends on {used}, which is not in a lower tier"
            );
        }
    }
}

#[test]
pub(super) fn attunement_proposals_have_no_runtime_or_store_dependency() {
    let metadata = workspace_metadata();
    assert_exact(
        &metadata.packages["rrd-attunement"].all,
        names(&[
            "rrd-core",
            "serde",
            "serde_json",
            "gix-index",
            "gix-ignore",
            "gix-config",
            "filetime",
            "tree-sitter",
            "tree-sitter-rust",
            "tree-sitter-python",
            "tree-sitter-go",
            // The trait tree-sitter's query matches implement; it does not
            // re-export it.
            "streaming-iterator",
        ]),
        "attunement uses pure buffer parsing, kernel hashes and serialized observations",
    );
}

#[test]
pub(super) fn engine_has_exact_composition() {
    let metadata = workspace_metadata();
    assert_exact(
        production_workspace_dependencies(&metadata, "rrd-engine"),
        names(&[
            "rrd-attunement",
            "rrd-contract",
            "rrd-estate",
            "rrd-security",
            "rrd-cluster",
            "rrd-core",
            "rrd-inference",
            "rrd-observation",
            "rrd-operator-knowledge",
            "rrd-query",
            "rrd-store",
            "rrd-vector",
        ]),
        "rrd-engine production workspace dependencies",
    );
}

#[test]
pub(super) fn daemon_depends_only_on_engine_and_contract() {
    let metadata = workspace_metadata();
    assert_exact(
        production_workspace_dependencies(&metadata, "rrd-server"),
        names(&["rrd-contract", "rrd-engine"]),
        "rrd-server production workspace dependencies",
    );
}

#[test]
pub(super) fn observation_transport_has_no_authority_or_storage_dependency() {
    let metadata = workspace_metadata();
    assert_exact(
        &metadata.packages["rrd-observation"].all,
        names(&[
            "rrd-contract",
            "rrd-core",
            "serde",
            "serde_json",
            "chrono",
            "ureq",
        ]),
        "observation transport is a stateless codec and bounded HTTP adapter",
    );
    let consumers = metadata
        .dependency_inputs
        .iter()
        .filter(|dependency| dependency.name == "rrd-observation")
        .map(|dependency| {
            (
                dependency.owner.as_str(),
                dependency.kind.as_str(),
                dependency.optional,
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(consumers, vec![("rrd-engine", "normal", true)]);
    let engine = &metadata.packages["rrd-engine"];
    assert!(engine.features["full"].contains("dep:rrd-observation"));
    assert!(!engine.features["edge"].contains("dep:rrd-observation"));
}

#[test]
pub(super) fn mcp_adapter_uses_only_engine_and_public_protocol_boundaries() {
    let metadata = workspace_metadata();
    assert_exact(
        production_workspace_dependencies(&metadata, "rrflow-mcp"),
        names(&["rrd-client", "rrd-contract", "rrd-engine"]),
        "rrflow-mcp embedded/daemon production boundaries",
    );
}

#[test]
pub(super) fn public_contract_and_client_stay_implementation_free() {
    let metadata = workspace_metadata();
    assert_exact(
        production_workspace_dependencies(&metadata, "rrd-contract"),
        BTreeSet::new(),
        "rrd-contract production workspace dependencies",
    );
    assert_exact(
        production_workspace_dependencies(&metadata, "rrd-client"),
        names(&["rrd-contract"]),
        "rrd-client production workspace dependencies",
    );
}

#[test]
pub(super) fn outward_consumers_cannot_bypass_the_engine_boundary() {
    let metadata = workspace_metadata();
    let internal_components = names(&[
        "rrd-attunement",
        "rrd-estate",
        "rrd-security",
        "rrd-cluster",
        "rrd-core",
        "rrd-inference",
        "rrd-lsm",
        "rrd-observation",
        "rrd-query",
        "rrd-operator-knowledge",
        "rrd-store",
        "rrd-vector",
    ]);

    assert_exact(
        &forbidden_edges(&metadata, "rrflow-cli", &internal_components),
        BTreeSet::new(),
        "CLI must use only the public embedded engine or daemon client boundary",
    );
    assert_exact(
        &forbidden_edges(&metadata, "rrflow-mcp", &internal_components),
        BTreeSet::new(),
        "MCP adapter must use only the engine boundary",
    );
}

#[test]
pub(super) fn outward_product_sources_do_not_import_physical_components() {
    let metadata = workspace_metadata();
    let physical = [
        "rrd_attunement",
        "rrd_cluster",
        "rrd_core",
        "rrd_estate",
        "rrd_graph",
        "rrd_inference",
        "rrd_lsm",
        "rrd_observation",
        "rrd_operator_knowledge",
        "rrd_query",
        "rrd_security",
        "rrd_store",
        "rrd_vector",
    ];
    let mut violations = Vec::new();
    for relative in [
        "crates/transport/rrd-client/src",
        "crates/transport/rrd-server/src",
        "crates/adapters/rrflow-cli/src",
        "crates/adapters/rrflow-cli/examples",
        "crates/adapters/rrflow-mcp/src",
    ] {
        let directory = metadata.root.join(relative);
        if directory.is_dir() {
            collect_physical_import_violations(
                &metadata.root,
                &directory,
                &physical,
                &mut violations,
            );
        }
    }
    violations.sort();
    violations.dedup();
    assert!(
        violations.is_empty(),
        "outward product source imports a physical/domain component instead of rrd-engine, rrd-client, or rrd-contract: {violations:#?}"
    );
}

#[test]
pub(super) fn tracked_and_untracked_text_files_have_no_trailing_horizontal_whitespace() {
    let metadata = workspace_metadata();
    let output = Command::new("git")
        .current_dir(&metadata.root)
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
    let mut violations = Vec::new();
    for encoded in output.stdout.split(|byte| *byte == 0) {
        if encoded.is_empty() {
            continue;
        }
        let relative = PathBuf::from(
            std::str::from_utf8(encoded).expect("repository paths must be valid UTF-8"),
        );
        let path = metadata.root.join(&relative);
        if !path.is_file() {
            continue;
        }
        let bytes = fs::read(&path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
        if bytes.contains(&0) || std::str::from_utf8(&bytes).is_err() {
            continue;
        }
        for (index, encoded_line) in bytes.split(|byte| *byte == b'\n').enumerate() {
            let line = encoded_line.strip_suffix(b"\r").unwrap_or(encoded_line);
            if line.last().is_some_and(|byte| matches!(byte, b' ' | b'\t')) {
                violations.push(format!("{}:{}", relative.display(), index + 1));
            }
        }
    }
    assert!(
        violations.is_empty(),
        "tracked or untracked text contains trailing horizontal whitespace: {violations:#?}"
    );
}
