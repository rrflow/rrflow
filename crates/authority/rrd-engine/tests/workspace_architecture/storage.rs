//! One engine storage authority, the storage closure, schemas and vector lifecycles.

use super::*;

#[test]
pub(super) fn one_engine_authority_owns_every_product_storage_opening() {
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
pub(super) fn pre_canonical_startup_authorities_are_absent_from_product_tree() {
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

    // The server binary is main.rs plus its binary-only process and cli modules.
    let read_sources = |paths: &[&str]| {
        paths
            .iter()
            .map(|path| {
                let source = fs::read_to_string(metadata.root.join(path))
                    .unwrap_or_else(|error| panic!("cannot read {path}: {error}"));
                source
                    .split("#[cfg(test)]")
                    .next()
                    .unwrap_or_default()
                    .to_owned()
            })
            .collect::<Vec<_>>()
            .join(
                "
",
            )
    };
    let server_product = read_sources(&[
        "crates/transport/rrd-server/src/main.rs",
        "crates/transport/rrd-server/src/process.rs",
        "crates/transport/rrd-server/src/cli.rs",
    ]);
    assert!(
        !server_product.contains("InitializeArgs")
            && !server_product.contains("rrd-security-bootstrap")
            && !server_product.contains("\"initialize\""),
        "the internal server regained an initializer or standalone bootstrap"
    );

    // The renderer is the crate root plus its spec, resources and validation modules.
    let kubernetes = read_sources(&[
        "crates/operations/rrd-kubernetes/src/lib.rs",
        "crates/operations/rrd-kubernetes/src/spec.rs",
        "crates/operations/rrd-kubernetes/src/resources.rs",
        "crates/operations/rrd-kubernetes/src/validation.rs",
    ]);
    assert!(
        kubernetes.contains("install")
            && kubernetes.contains("apply")
            && kubernetes.contains("--distribution-executable")
            && !kubernetes.contains("rrd-security-bootstrap")
            && !kubernetes.contains("initialize"),
        "Kubernetes startup must use one canonical install apply handoff"
    );

    let estate_control = module_source(
        &metadata.root,
        "crates/authority/rrd-engine/src/engine/estate_control.rs",
    );
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
pub(super) fn alpha_storage_closure_has_one_required_physical_dependency_and_current_reader() {
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

    let batch = module_source(&metadata.root, "crates/persistence/rrd-lsm/src/batch.rs");
    let manifest = module_source(&metadata.root, "crates/persistence/rrd-lsm/src/manifest.rs");
    let segment_module = module_source(&metadata.root, "crates/persistence/rrd-lsm/src/segment.rs");
    let segment_format = module_source(
        &metadata.root,
        "crates/persistence/rrd-lsm/src/segment/format.rs",
    );
    let segment = format!("{segment_module}\n{segment_format}");
    let vector_catalog = module_source(&metadata.root, "crates/compute/rrd-vector/src/catalog.rs");
    assert!(batch.contains("pub const BATCH_FORMAT_VERSION: u16 = 2;"));
    assert!(manifest.contains("pub const MANIFEST_FORMAT_VERSION: u16 = 3;"));
    assert!(segment.contains("pub const SEGMENT_FORMAT_VERSION: u16 = 6;"));
    assert!(segment.contains("pub const SEGMENT_MAGIC: &[u8; 8] = b\"RRDSEG06\";"));
    assert!(segment.contains("pub const INDEX_MAGIC: &[u8; 8] = b\"RRDIX006\";"));
    assert!(vector_catalog.contains("pub const VECTOR_ARTIFACT_CATALOG_VERSION: u16 = 3;"));
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
pub(super) fn schema_catalogue_has_one_explicit_logical_model_authority() {
    let metadata = workspace_metadata();
    let core_schema = module_source(&metadata.root, "crates/kernel/rrd-core/src/schema.rs");
    let core_runtime = module_source(&metadata.root, "crates/kernel/rrd-core/src/runtime.rs");
    let public_contract = module_source(&metadata.root, "crates/transport/rrd-contract/src/lib.rs");

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
pub(super) fn vector_writes_require_one_canonical_collection_address() {
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
        let source = module_source(&metadata.root, relative);
        assert!(
            source.contains(required),
            "{boundary} lost its mandatory canonical vector address"
        );
        assert!(
            !source.contains("collection: Option<VectorCollectionAddress>"),
            "{boundary} regained an optional vector address"
        );
    }

    let contract = module_source(&metadata.root, "crates/transport/rrd-contract/src/lib.rs");
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

    let runtime = module_source(&metadata.root, "crates/kernel/rrd-core/src/runtime.rs");
    for required in [
        "text(out, &vector.collection.collection_id);",
        "text(out, &vector.collection.vector_name);",
    ] {
        assert!(
            runtime.contains(required),
            "runtime commit identity omitted {required}"
        );
    }

    let keyspaces = module_source(
        &metadata.root,
        "crates/persistence/rrd-store/src/keyspaces.rs",
    );
    assert!(
        keyspaces.contains("validate_vector_source_key_parts(collection_id, vector_name, field)?;"),
        "vector source keys must fail closed before encoding an address"
    );
}

#[test]
pub(super) fn vector_projection_lifecycle_has_no_duplicate_turboquant_authority() {
    let metadata = workspace_metadata();
    let vector_runtime = module_source(&metadata.root, "crates/compute/rrd-vector/src/runtime.rs");
    let engine_runtime = module_source(
        &metadata.root,
        "crates/authority/rrd-engine/src/runtime/vector_catalog.rs",
    );
    let quantization = module_source(
        &metadata.root,
        "crates/authority/rrd-engine/src/engine/vector/quantization.rs",
    );
    let contract = module_source(&metadata.root, "crates/transport/rrd-contract/src/lib.rs");
    let collection = module_source(
        &metadata.root,
        "crates/authority/rrd-engine/src/engine/vector/collection.rs",
    );

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
pub(super) fn storage_semantics_use_repositories_over_one_transaction_port() {
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
