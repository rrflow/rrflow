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

#[test]
fn workspace_packages_use_the_canonical_grouped_layout() {
    let metadata = workspace_metadata();
    let expected = [
        ("rrd-core", "crates/kernel/rrd-core/Cargo.toml"),
        ("rrd-lsm", "crates/persistence/rrd-lsm/Cargo.toml"),
        ("rrd-store", "crates/persistence/rrd-store/Cargo.toml"),
        ("rrd-query", "crates/compute/rrd-query/Cargo.toml"),
        ("rrd-vector", "crates/compute/rrd-vector/Cargo.toml"),
        ("rrd-inference", "crates/compute/rrd-inference/Cargo.toml"),
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
        ("rrflow-eval", "crates/evaluation/rrflow-eval/Cargo.toml"),
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

#[test]
fn engine_has_exact_composition() {
    let metadata = workspace_metadata();
    assert_exact(
        production_workspace_dependencies(&metadata, "rrd-engine"),
        names(&[
            "rrd-contract",
            "rrd-estate",
            "rrd-security",
            "rrd-cluster",
            "rrd-core",
            "rrd-inference",
            "rrd-operator-knowledge",
            "rrd-query",
            "rrd-store",
            "rrd-vector",
        ]),
        "rrd-engine production workspace dependencies",
    );
}

#[test]
fn daemon_depends_only_on_engine_and_contract() {
    let metadata = workspace_metadata();
    assert_exact(
        production_workspace_dependencies(&metadata, "rrd-server"),
        names(&["rrd-contract", "rrd-engine"]),
        "rrd-server production workspace dependencies",
    );
}

#[test]
fn mcp_adapter_uses_only_engine_and_public_protocol_boundaries() {
    let metadata = workspace_metadata();
    assert_exact(
        production_workspace_dependencies(&metadata, "rrflow-mcp"),
        names(&["rrd-client", "rrd-contract", "rrd-engine"]),
        "rrflow-mcp embedded/daemon production boundaries",
    );
}

#[test]
fn public_contract_and_client_stay_implementation_free() {
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
fn outward_consumers_cannot_bypass_the_engine_boundary() {
    let metadata = workspace_metadata();
    let internal_components = names(&[
        "rrd-estate",
        "rrd-security",
        "rrd-cluster",
        "rrd-core",
        "rrd-inference",
        "rrd-lsm",
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
fn outward_product_sources_do_not_import_physical_components() {
    let metadata = workspace_metadata();
    let physical = [
        "rrd_cluster",
        "rrd_core",
        "rrd_estate",
        "rrd_graph",
        "rrd_inference",
        "rrd_lsm",
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
fn tracked_and_untracked_text_files_have_no_trailing_horizontal_whitespace() {
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

#[test]
fn one_engine_authority_owns_every_product_storage_opening() {
    let metadata = workspace_metadata();
    let mut violations = Vec::new();
    collect_authority_opening_violations(&metadata.root.join("crates"), &mut violations);
    violations.sort();
    assert!(
        violations.is_empty(),
        "production code outside rrd-engine/rrd-store must not open physical storage or revive a second embedded handle: {violations:#?}"
    );

    let engine_library =
        fs::read_to_string(metadata.root.join("crates/authority/rrd-engine/src/lib.rs"))
            .expect("rrd-engine library source must be readable");
    assert!(
        !engine_library.contains("pub mod operator;"),
        "the engine-owned operator implementation must remain private"
    );
    let mut duplicate_handles = Vec::new();
    collect_rust_sources(
        &metadata.root.join("crates/authority/rrd-engine/src"),
        &mut duplicate_handles,
        "EmbeddedOperator",
    );
    collect_rust_sources(
        &metadata.root.join("crates/authority/rrd-engine/src"),
        &mut duplicate_handles,
        "pub fn runtime_store",
    );
    duplicate_handles.sort();
    duplicate_handles.dedup();
    assert!(
        duplicate_handles.is_empty(),
        "rrd-engine must expose only RrdEngine as its storage-opening authority: {duplicate_handles:#?}"
    );
}

#[test]
fn pre_canonical_startup_authorities_are_absent_from_product_tree() {
    let metadata = workspace_metadata();
    for relative in [
        ".rrflow/instance.toml",
        "crates/authority/rrd-engine/src/runtime/instance.rs",
        "crates/authority/rrd-engine/src/engine/security_bootstrap.rs",
        "crates/authority/rrd-engine/tests/runtime_instance.rs",
        "crates/adapters/rrflow-cli/src/dev.rs",
        "crates/adapters/rrflow-cli/src/dev/supervisor.rs",
        "crates/adapters/rrflow-cli/src/bin/rrd-security-bootstrap.rs",
        "crates/adapters/rrflow-cli/tests/operator_surface.rs",
        "crates/adapters/rrflow-cli/tests/security_bootstrap.rs",
    ] {
        assert!(
            !metadata.root.join(relative).exists(),
            "pre-canonical authority path remains: {relative}"
        );
    }

    let mut violations = Vec::new();
    for relative in [
        "crates/authority/rrd-engine/src",
        "crates/adapters/rrflow-cli/src",
        "crates/adapters/rrflow-mcp/src",
        "crates/transport/rrd-server/src",
        "crates/transport/rrd-client/examples",
        "crates/authority/rrd-estate/src/commands",
        "crates/operations/rrd-kubernetes/src",
    ] {
        for forbidden in [
            "InstanceManifest",
            "InstanceBinding",
            "ProjectAuthorityBinding",
            "open_project_store",
            "canonical_project_root_for_store",
            "open_bound(",
            "ensure_dedicated",
            "apply_security_bootstrap",
            "SecurityBootstrapOutcome",
        ] {
            collect_rust_sources(&metadata.root.join(relative), &mut violations, forbidden);
        }
    }
    violations.sort();
    violations.dedup();
    assert!(
        violations.is_empty(),
        "pre-canonical startup symbols remain in product source: {violations:#?}"
    );

    let server = fs::read_to_string(
        metadata
            .root
            .join("crates/transport/rrd-server/src/main.rs"),
    )
    .expect("server source must be readable");
    let server_product = server.split("#[cfg(test)]").next().unwrap_or(&server);
    assert!(
        !server_product.contains("InitializeArgs")
            && !server_product.contains("rrd-security-bootstrap")
            && !server_product.contains("\"initialize\""),
        "the internal server regained an initializer or standalone bootstrap"
    );

    let kubernetes = fs::read_to_string(
        metadata
            .root
            .join("crates/operations/rrd-kubernetes/src/lib.rs"),
    )
    .expect("Kubernetes renderer source must be readable");
    assert!(
        kubernetes.contains("install")
            && kubernetes.contains("apply")
            && kubernetes.contains("--distribution-executable")
            && !kubernetes.contains("rrd-security-bootstrap")
            && !kubernetes.contains("initialize"),
        "Kubernetes startup must use one canonical install apply handoff"
    );

    let estate_control = fs::read_to_string(
        metadata
            .root
            .join("crates/authority/rrd-engine/src/engine/estate_control.rs"),
    )
    .expect("estate control source must be readable");
    assert!(
        !estate_control.contains("RrdEngine::open(")
            && !estate_control.contains("RrdEngine::open_existing(")
            && estate_control.contains("RrdEngine::open_installed(")
            && !estate_control.contains("join(\"backups\")")
            && !estate_control.contains("join(\"restores\")"),
        "the estate controller opens a managed instance only as an installed estate and keeps no catalogue or staging lane outside the instance"
    );
}

#[test]
fn alpha_storage_closure_has_one_required_physical_dependency_and_current_reader() {
    let metadata = workspace_metadata();

    let lsm_edges = metadata
        .dependency_inputs
        .iter()
        .filter(|dependency| dependency.name == "rrd-lsm")
        .map(|dependency| {
            (
                dependency.owner.as_str(),
                dependency.kind.as_str(),
                dependency.optional,
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        lsm_edges,
        vec![
            ("rrd-cluster", "normal", true),
            ("rrd-store", "normal", false),
        ],
        "rrd-lsm must have one required alpha owner; the only optional edge is the unavailable post-alpha cluster adapter"
    );

    let required_lsm_owners = lsm_edges
        .iter()
        .filter_map(|(owner, kind, optional)| {
            (*kind != "dev" && !optional).then_some((*owner).to_owned())
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(
        required_lsm_owners,
        names(&["rrd-store"]),
        "the alpha executable must reach the physical substrate only through rrd-store"
    );

    let engine_cluster = metadata
        .dependency_inputs
        .iter()
        .find(|dependency| dependency.owner == "rrd-engine" && dependency.name == "rrd-cluster")
        .expect("rrd-engine must declare its cluster contract dependency");
    assert_eq!(
        engine_cluster.features,
        names(&["object-transfer"]),
        "the alpha engine may consume only cluster transfer contracts; consensus features remain unavailable"
    );
    let cluster = &metadata.packages["rrd-cluster"];
    assert_eq!(
        cluster.features.get("default"),
        Some(&BTreeSet::new()),
        "rrd-cluster must not activate an implementation by default"
    );
    let engine = &metadata.packages["rrd-engine"];
    assert_eq!(
        engine.features.get("default"),
        Some(&names(&["full"])),
        "rrd-engine's alpha composition changed"
    );
    assert!(
        engine
            .features
            .get("full")
            .is_some_and(|features| !features.iter().any(|feature| feature.contains("openraft"))),
        "the alpha engine composition must not activate the post-alpha OpenRaft adapter"
    );

    for package in metadata.packages.values() {
        assert!(
            !package.all.contains("fjall"),
            "Fjall must remain absent from workspace dependency metadata"
        );
    }

    let lsm_lz4 = metadata
        .dependency_inputs
        .iter()
        .find(|dependency| dependency.owner == "rrd-lsm" && dependency.name == "lz4_flex")
        .expect("rrd-lsm must declare its production LZ4 block codec");
    assert_eq!(lsm_lz4.kind, "normal");
    assert!(!lsm_lz4.optional);
    assert_exact(
        &lsm_lz4.features,
        names(&["checked-decode", "safe-decode", "safe-encode", "std"]),
        "rrd-lsm production LZ4 safety features",
    );
    let lsm_zstd = metadata
        .dependency_inputs
        .iter()
        .find(|dependency| dependency.owner == "rrd-lsm" && dependency.name == "zstd")
        .expect("rrd-lsm must retain Zstd only as a physical-policy lab candidate");
    assert!(lsm_zstd.optional);
    assert!(
        metadata.packages["rrd-lsm"]
            .features
            .get("physical-policy-lab")
            .is_some_and(|features| features.contains("dep:zstd")),
        "Zstd must remain gated behind rrd-lsm's physical-policy lab"
    );

    let batch = fs::read_to_string(
        metadata
            .root
            .join("crates/persistence/rrd-lsm/src/batch.rs"),
    )
    .expect("batch codec source must be readable");
    let manifest = fs::read_to_string(
        metadata
            .root
            .join("crates/persistence/rrd-lsm/src/manifest.rs"),
    )
    .expect("manifest source must be readable");
    let segment_module = fs::read_to_string(
        metadata
            .root
            .join("crates/persistence/rrd-lsm/src/segment/mod.rs"),
    )
    .expect("segment module source must be readable");
    let segment_format = fs::read_to_string(
        metadata
            .root
            .join("crates/persistence/rrd-lsm/src/segment/format.rs"),
    )
    .expect("segment format source must be readable");
    let segment = format!("{segment_module}\n{segment_format}");
    let vector_catalog = fs::read_to_string(
        metadata
            .root
            .join("crates/compute/rrd-vector/src/catalog.rs"),
    )
    .expect("vector artifact catalog source must be readable");
    assert!(batch.contains("pub const BATCH_FORMAT_VERSION: u16 = 2;"));
    assert!(manifest.contains("pub const MANIFEST_FORMAT_VERSION: u16 = 3;"));
    assert!(segment.contains("pub const SEGMENT_FORMAT_VERSION: u16 = 6;"));
    assert!(segment.contains("pub const SEGMENT_MAGIC: &[u8; 8] = b\"RRDSEG06\";"));
    assert!(segment.contains("pub const INDEX_MAGIC: &[u8; 8] = b\"RRDIX006\";"));
    assert!(vector_catalog.contains("pub const VECTOR_ARTIFACT_CATALOG_VERSION: u16 = 2;"));
    for (name, source) in [
        ("batch", batch.as_str()),
        ("manifest", manifest.as_str()),
        ("segment", segment.as_str()),
    ] {
        for retired in [
            "decode_legacy",
            "SegmentStorage::Legacy",
            "PersistentBackend",
            "PersistentEngine",
        ] {
            assert!(
                !source.contains(retired),
                "{name} reader revived retired branch {retired}"
            );
        }
    }
    assert!(
        !vector_catalog.contains("LEGACY_VECTOR_ARTIFACT_CATALOG_VERSION"),
        "the alpha executable revived the pre-1.0 vector artifact catalogue reader"
    );
}

#[test]
fn schema_catalogue_has_one_explicit_logical_model_authority() {
    let metadata = workspace_metadata();
    let core_schema =
        fs::read_to_string(metadata.root.join("crates/kernel/rrd-core/src/schema.rs"))
            .expect("runtime schema source must be readable");
    let core_runtime =
        fs::read_to_string(metadata.root.join("crates/kernel/rrd-core/src/runtime.rs"))
            .expect("runtime snapshot source must be readable");
    let public_contract = fs::read_to_string(
        metadata
            .root
            .join("crates/transport/rrd-contract/src/lib.rs"),
    )
    .expect("public contract source must be readable");

    let optional_table_field =
        "#[serde(default, skip_serializing_if = \"BTreeMap::is_empty\")]\n    pub tables:";
    assert!(
        !core_schema.contains(optional_table_field),
        "RuntimeSchemaRegistry tables must not regain an omitted encoding"
    );
    assert!(
        !public_contract.contains(optional_table_field),
        "DataSchemaRegistry tables must not regain an omitted wire encoding"
    );
    assert!(core_schema.contains("runtime schema must declare an explicit canonical table map"));
    assert!(public_contract.contains("data schema requires an explicit non-empty table map"));
    for paired_author in [
        "pub fn define_record_table(",
        "pub fn define_relation_table(",
        "pub fn define_event_table(",
    ] {
        assert!(
            core_schema.contains(paired_author),
            "runtime schema lost paired authoring API {paired_author}"
        );
    }
    for retired_fallback in [
        "legacy_model",
        "if schema.tables.is_empty()",
        "persisted pre-G03 migration form",
    ] {
        assert!(
            !core_schema.contains(retired_fallback)
                && !core_runtime.contains(retired_fallback)
                && !public_contract.contains(retired_fallback),
            "schema model derivation or missing-table fallback revived: {retired_fallback}"
        );
    }
    assert!(
        core_schema.contains("Ok(self.tables.clone())"),
        "catalogue_tables must return the validated canonical table map"
    );
    assert!(
        core_runtime.contains("runtime value type {} is absent from schema revision {}"),
        "snapshot assembly must fail closed when a table model is absent"
    );
}

#[test]
fn vector_writes_require_one_canonical_collection_address() {
    let metadata = workspace_metadata();
    let required_address_sources = [
        (
            "kernel runtime vector",
            "crates/kernel/rrd-core/src/data.rs",
            "pub collection: VectorCollectionAddress,",
        ),
        (
            "inference embedding job",
            "crates/compute/rrd-inference/src/lib.rs",
            "pub collection: VectorCollectionAddress,",
        ),
        (
            "persistent vector source",
            "crates/persistence/rrd-store/src/access/vector.rs",
            "pub collection: VectorCollectionAddress,",
        ),
        (
            "dense vector artifact",
            "crates/compute/rrd-vector/src/compact.rs",
            "collection: VectorCollectionAddress,",
        ),
        (
            "quantized vector artifact",
            "crates/compute/rrd-vector/src/quantized_segment.rs",
            "collection: VectorCollectionAddress,",
        ),
        (
            "TurboQuant vector artifact",
            "crates/compute/rrd-vector/src/turbo_segment.rs",
            "collection: VectorCollectionAddress,",
        ),
        (
            "offline edge embedding",
            "crates/authority/rrd-engine/src/edge.rs",
            "pub collection: VectorCollectionAddress,",
        ),
    ];
    for (boundary, relative, required) in required_address_sources {
        let source = fs::read_to_string(metadata.root.join(relative))
            .unwrap_or_else(|error| panic!("cannot read {relative}: {error}"));
        assert!(
            source.contains(required),
            "{boundary} lost its mandatory canonical vector address"
        );
        assert!(
            !source.contains("collection: Option<VectorCollectionAddress>"),
            "{boundary} regained an optional vector address"
        );
    }

    let contract = fs::read_to_string(
        metadata
            .root
            .join("crates/transport/rrd-contract/src/lib.rs"),
    )
    .expect("public contract source must be readable");
    let put_vector = contract
        .split_once("    PutVector {")
        .expect("public PutVector mutation must exist")
        .1
        .split_once("    AppendSeriesSample {")
        .expect("public PutVector mutation must precede AppendSeriesSample")
        .0;
    for required in ["collection_id: CanonicalId,", "vector_name: CanonicalId,"] {
        assert!(
            put_vector.contains(required),
            "public PutVector lost required address component {required}"
        );
    }
    assert!(
        !put_vector.contains("Option<CanonicalId>"),
        "public PutVector regained an optional address component"
    );

    let runtime = fs::read_to_string(metadata.root.join("crates/kernel/rrd-core/src/runtime.rs"))
        .expect("runtime commit source must be readable");
    for required in [
        "text(out, &vector.collection.collection_id);",
        "text(out, &vector.collection.vector_name);",
    ] {
        assert!(
            runtime.contains(required),
            "runtime commit identity omitted {required}"
        );
    }

    let keyspaces = fs::read_to_string(
        metadata
            .root
            .join("crates/persistence/rrd-store/src/keyspaces.rs"),
    )
    .expect("rrflowKV keyspace source must be readable");
    assert!(
        keyspaces.contains("validate_vector_source_key_parts(collection_id, vector_name, field)?;"),
        "vector source keys must fail closed before encoding an address"
    );
}

#[test]
fn vector_projection_lifecycle_has_no_duplicate_turboquant_authority() {
    let metadata = workspace_metadata();
    let vector_runtime = fs::read_to_string(
        metadata
            .root
            .join("crates/compute/rrd-vector/src/runtime.rs"),
    )
    .expect("vector runtime source must be readable");
    let engine_runtime = fs::read_to_string(
        metadata
            .root
            .join("crates/authority/rrd-engine/src/runtime/vector_catalog.rs"),
    )
    .expect("engine vector reconstruction source must be readable");
    let quantization = fs::read_to_string(
        metadata
            .root
            .join("crates/authority/rrd-engine/src/engine/vector/quantization.rs"),
    )
    .expect("engine quantization source must be readable");
    let contract = fs::read_to_string(
        metadata
            .root
            .join("crates/transport/rrd-contract/src/lib.rs"),
    )
    .expect("public contract source must be readable");
    let collection = fs::read_to_string(
        metadata
            .root
            .join("crates/authority/rrd-engine/src/engine/vector/collection.rs"),
    )
    .expect("engine vector collection source must be readable");

    let suppression = ["suppress_legacy_", "turboquant"].concat();
    let ensure_adapter = ["ensure_", "turboquant_index_authorized"].concat();
    assert!(
        !vector_runtime.contains(&suppression) && !engine_runtime.contains(&suppression),
        "vector reconstruction regained a post-replay TurboQuant suppression path"
    );
    assert!(
        !quantization.contains(&ensure_adapter),
        "TurboQuant regained an alternate ensure_vector_index adapter"
    );
    let index_configuration = contract
        .split_once("pub enum VectorIndexConfiguration")
        .expect("public vector index configuration must exist")
        .1
        .split_once("impl VectorIndexConfiguration")
        .expect("public vector index configuration must precede validation")
        .0;
    assert!(
        !index_configuration.contains("TurboQuant"),
        "generic vector index configuration regained TurboQuant"
    );
    for function in [
        "fn require_no_collection_artifacts",
        "fn require_payload_index_not_in_use",
    ] {
        let generic_catalogue_branch = collection
            .split_once(function)
            .expect("vector collection protection function must exist")
            .1
            .split_once("let quantization =")
            .expect("generic protection must precede quantization lifecycle protection")
            .0;
        assert!(
            !generic_catalogue_branch
                .to_ascii_lowercase()
                .contains("turboquant"),
            "generic vector catalogue protection regained a TurboQuant branch"
        );
    }
}

#[test]
fn storage_semantics_use_repositories_over_one_transaction_port() {
    let metadata = workspace_metadata();
    let store_source = metadata.root.join("crates/persistence/rrd-store/src");
    let engine = fs::read_to_string(store_source.join("engine.rs"))
        .expect("rrd-store engine source must be readable");
    let trait_body = engine
        .split_once("pub trait StorageEngine")
        .expect("StorageEngine declaration must exist")
        .1
        .split_once("/// rrflowMX")
        .expect("StorageEngine declaration must precede rrflowMX")
        .0;
    for required in [
        "fn begin_transaction",
        "fn claims",
        "fn control",
        "fn projections",
        "fn runtime",
        "fn invocations",
        "fn physical_store_evidence",
    ] {
        assert!(
            trait_body.contains(required),
            "StorageEngine is missing its narrow {required} boundary"
        );
    }
    for forbidden in [
        "fn append_batch",
        "fn commit_control",
        "fn runtime_commit",
        "fn get_projection",
        "fn record_invocation",
        "fn removal_report",
    ] {
        assert!(
            !trait_body.contains(forbidden),
            "StorageEngine regained semantic mega-trait method {forbidden}"
        );
    }

    let repository_root = store_source.join("repository");
    for name in [
        "claims.rs",
        "control.rs",
        "invocation.rs",
        "projection.rs",
        "runtime.rs",
    ] {
        let source = fs::read_to_string(repository_root.join(name))
            .unwrap_or_else(|error| panic!("cannot read repository {name}: {error}"));
        assert!(
            source.contains("begin_transaction()"),
            "semantic repository {name} does not consume the shared transaction port"
        );
        for forbidden in [
            "rrd_lsm::",
            "RrflowKvStore",
            "RrflowMxStore",
            "write_owned(",
            "Database",
        ] {
            assert!(
                !source.contains(forbidden),
                "semantic repository {name} bypasses the transaction port through {forbidden}"
            );
        }
    }
}

#[test]
fn outward_cli_owns_current_auxiliary_executables_while_physical_crates_own_none() {
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
fn mcp_uses_the_authoritative_context_operation() {
    let metadata = workspace_metadata();
    let mcp = fs::read_to_string(metadata.root.join("crates/adapters/rrflow-mcp/src/main.rs"))
        .expect("MCP source must be readable");
    assert!(
        mcp.contains("rrflow_context") && mcp.contains("authority.assemble_context"),
        "MCP must expose only the engine context assembly boundary"
    );
}

#[test]
fn engine_has_no_transport_dependencies() {
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
fn retired_service_name_is_absent() {
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
fn retired_fixed_reasoning_ledger_api_is_absent() {
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
fn retired_function_authority_vocabulary_is_absent() {
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
fn retired_pre_release_identity_is_absent_from_the_active_repository() {
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
fn every_workspace_target_source_is_tracked() {
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
fn workspace_source_inputs_are_repository_closed() {
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
    for source in &metadata.target_sources {
        let path = metadata.root.join(source);
        let canonical = fs::canonicalize(&path)
            .unwrap_or_else(|error| panic!("cannot resolve {}: {error}", path.display()));
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

    for entry in git_index_entries(&metadata.root) {
        if entry.mode == "160000" {
            violations.push(format!(
                "tracked Git submodule supplies repository input at {}",
                entry.path.display()
            ));
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

    violations.sort();
    assert!(
        violations.is_empty(),
        "RRFlow first-party source must be supplied by this workspace without Git, submodule, or escaping-path inputs: {violations:#?}"
    );
}

#[test]
fn tracked_paths_are_case_insensitively_unique() {
    let metadata = workspace_metadata();
    let mut folded_paths = BTreeMap::<String, BTreeSet<PathBuf>>::new();
    for entry in git_index_entries(&metadata.root) {
        folded_paths
            .entry(entry.path.to_string_lossy().to_lowercase())
            .or_default()
            .insert(entry.path);
    }
    let collisions = folded_paths
        .into_iter()
        .filter_map(|(folded, paths)| (paths.len() > 1).then_some((folded, paths)))
        .collect::<Vec<_>>();
    assert!(
        collisions.is_empty(),
        "tracked paths collide on case-insensitive filesystems: {collisions:#?}"
    );

    let mut folded_packages = BTreeMap::<String, BTreeSet<String>>::new();
    for name in metadata.packages.keys() {
        folded_packages
            .entry(name.to_lowercase())
            .or_default()
            .insert(name.clone());
    }
    let package_collisions = folded_packages
        .into_iter()
        .filter_map(|(folded, names)| (names.len() > 1).then_some((folded, names)))
        .collect::<Vec<_>>();
    assert!(
        package_collisions.is_empty(),
        "workspace package names collide case-insensitively: {package_collisions:#?}"
    );
}

#[test]
fn workspace_manifest_descriptions_use_canonical_product_spelling() {
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
fn active_product_text_uses_canonical_rrflowql_spelling() {
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
fn architecture_text_detectors_reject_host_and_spelling_drift() {
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
fn active_build_and_install_inputs_are_host_independent() {
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

fn workspace_metadata() -> WorkspaceMetadata {
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

fn production_workspace_dependencies<'a>(
    metadata: &'a WorkspaceMetadata,
    package: &str,
) -> &'a BTreeSet<String> {
    &metadata
        .packages
        .get(package)
        .unwrap_or_else(|| panic!("workspace package {package} is absent"))
        .workspace
}

fn production_dependencies<'a>(
    metadata: &'a WorkspaceMetadata,
    package: &str,
) -> &'a BTreeSet<String> {
    &metadata
        .packages
        .get(package)
        .unwrap_or_else(|| panic!("workspace package {package} is absent"))
        .all
}

fn forbidden_edges(
    metadata: &WorkspaceMetadata,
    package: &str,
    forbidden: &BTreeSet<String>,
) -> BTreeSet<String> {
    production_workspace_dependencies(metadata, package)
        .intersection(forbidden)
        .cloned()
        .collect()
}

fn assert_exact(actual: &BTreeSet<String>, expected: BTreeSet<String>, boundary: &str) {
    let added = actual.difference(&expected).cloned().collect::<Vec<_>>();
    let missing = expected.difference(actual).cloned().collect::<Vec<_>>();
    assert!(
        added.is_empty() && missing.is_empty(),
        "{boundary} changed; added={added:?}, missing={missing:?}, actual={actual:?}"
    );
}

fn names(values: &[&str]) -> BTreeSet<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

fn noncanonical_ascii_terms(value: &str, canonical_terms: &[&str]) -> Vec<String> {
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

fn git_index_entries(root: &Path) -> Vec<GitIndexEntry> {
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

fn is_build_or_install_input(path: &Path) -> bool {
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

fn is_active_product_text(path: &Path) -> bool {
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

fn contains_host_specific_absolute_path(source: &str) -> bool {
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

fn collect_rust_sources(directory: &Path, violations: &mut Vec<PathBuf>, forbidden: &str) {
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

fn collect_authority_opening_violations(crates: &Path, violations: &mut Vec<PathBuf>) {
    collect_authority_package_tree(crates, crates, violations);
}

fn collect_authority_package_tree(crates: &Path, directory: &Path, violations: &mut Vec<PathBuf>) {
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

fn collect_physical_import_violations(
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

fn collect_authority_source_tree(crates: &Path, directory: &Path, violations: &mut Vec<PathBuf>) {
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

fn collect_identity_violations(root: &Path, violations: &mut Vec<PathBuf>, forbidden: &[&[u8]]) {
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

fn contains_identity(haystack: &[u8], needle: &[u8]) -> bool {
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
