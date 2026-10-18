use rrd_contract::{
    endpoint_profile_sha256, BindingKind, BindingLimits, CanonicalId, CredentialReference,
    DeclaredFootprint, EndpointBinding, EndpointLocator, EndpointProfile, EndpointProfileInput,
    GpuCapacity, GpuRequirement, HardwareInventory, MachineCapacity, WorkloadClass,
    ENDPOINT_PROFILE_CONTRACT_VERSION, HARDWARE_INVENTORY_CONTRACT_VERSION,
};
use std::path::Path;

const GIB: u64 = 1024 * 1024 * 1024;

fn fixture(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

fn ring_3_input() -> EndpointProfileInput {
    serde_json::from_str(&fixture("endpoint-profile-v1.json")).unwrap()
}

fn ring_3_inventory() -> HardwareInventory {
    serde_json::from_str(&fixture("hardware-inventory-v1.json")).unwrap()
}

fn id(value: &str) -> CanonicalId {
    CanonicalId::new(value).unwrap()
}

fn local_binding(name: &str, class: WorkloadClass) -> EndpointBinding {
    EndpointBinding {
        id: id(name),
        class,
        kind: BindingKind::Local,
        endpoint: EndpointLocator::Url {
            url: "http://127.0.0.1:8080".into(),
        },
        service_id: id("service"),
        manifest_sha256: None,
        credential: None,
        placement: Some(id("spark-3")),
        footprint: DeclaredFootprint {
            memory_bytes: GIB,
            gpu: GpuRequirement::None,
        },
        limits: BindingLimits {
            max_concurrency: 4,
            timeout_ms: 30_000,
            max_input_bytes: 1024 * 1024,
        },
    }
}

fn profile_of(bindings: Vec<EndpointBinding>) -> rrd_contract::Result<EndpointProfile> {
    EndpointProfile::from_input(
        1,
        EndpointProfileInput {
            contract_version: ENDPOINT_PROFILE_CONTRACT_VERSION,
            name: id("test"),
            bindings,
        },
    )
}

#[test]
fn ring_3_fixture_round_trips_orders_fallbacks_and_fits_its_inventory() {
    let input = ring_3_input();
    input.validate().unwrap();
    let profile = EndpointProfile::from_input(1, input).unwrap();
    profile.validate().unwrap();
    assert_eq!(
        profile.profile_sha256,
        endpoint_profile_sha256(&profile).unwrap()
    );

    let encoded = serde_json::to_vec(&profile).unwrap();
    let decoded: EndpointProfile = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(decoded, profile);

    let large_model = profile
        .fallbacks(WorkloadClass::LargeModelServing)
        .map(|binding| binding.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        large_model,
        [
            "reasoning-primary",
            "reasoning-fallback",
            "reasoning-subscription",
            "reasoning-api"
        ]
    );
    assert_eq!(
        profile
            .primary(WorkloadClass::CanonicalStore)
            .map(|binding| binding.id.as_str()),
        Some("rrflowkv")
    );
    assert!(profile.primary(WorkloadClass::VectorStore).is_some());
    let kinds = profile
        .bindings
        .iter()
        .map(|binding| binding.kind)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        kinds.len(),
        4,
        "the fixture carries one binding of every kind"
    );

    let loads = profile.validate_placement(&ring_3_inventory()).unwrap();
    let spark_3 = loads
        .iter()
        .find(|load| load.machine_id.as_str() == "spark-3")
        .unwrap();
    assert_eq!(
        spark_3.memory_declared_bytes,
        256 * 1024 * 1024 + 10 * GIB + 2 * GIB + 16 * GIB + 32 * GIB
    );
    assert_eq!(spark_3.gpu_declared_bytes, 12 * GIB);
    assert_eq!(spark_3.gpu_capacity_bytes, Some(128 * GIB));
    let proxmox = loads
        .iter()
        .find(|load| load.machine_id.as_str() == "proxmox-1")
        .unwrap();
    assert_eq!(proxmox.memory_declared_bytes, 4 * GIB);
    assert_eq!(proxmox.gpu_capacity_bytes, None);
}

#[test]
fn ring_3_fixture_digest_is_stable() {
    let profile = EndpointProfile::from_input(1, ring_3_input()).unwrap();
    assert_eq!(
        profile.profile_sha256,
        "611144dbc4056fa8a4df87e5c18c8ae26342d288319f41c727183c08dc3da4d2",
        "the canonical encoding of the ring-3 fixture changed; update the fixture record and this digest together"
    );
}

#[test]
fn strict_contract_rejects_unknown_fields_and_credential_values() {
    let mut value: serde_json::Value =
        serde_json::from_str(&fixture("endpoint-profile-v1.json")).unwrap();
    value["bindings"][3]["credential"]["value"] = serde_json::json!("sk-should-never-be-here");
    assert!(serde_json::from_value::<EndpointProfileInput>(value.clone()).is_err());

    let mut value: serde_json::Value =
        serde_json::from_str(&fixture("endpoint-profile-v1.json")).unwrap();
    value["bindings"][3]["api_key"] = serde_json::json!("sk-should-never-be-here");
    assert!(serde_json::from_value::<EndpointProfileInput>(value).is_err());

    let mut value: serde_json::Value =
        serde_json::from_str(&fixture("hardware-inventory-v1.json")).unwrap();
    value["machines"][0]["hostname"] = serde_json::json!("spark-1.ring.example");
    assert!(serde_json::from_value::<HardwareInventory>(value).is_err());
}

#[test]
fn binding_kinds_enforce_credential_placement_and_locator_rules() {
    let mut subscription = local_binding("subscription", WorkloadClass::LargeModelServing);
    subscription.kind = BindingKind::Subscription;
    subscription.placement = None;
    assert!(
        subscription.validate().is_err(),
        "a subscription needs a credential reference"
    );
    subscription.credential = Some(CredentialReference {
        provider: id("infisical"),
        name: "provider-subscription-token".into(),
    });
    subscription.validate().unwrap();
    subscription.placement = Some(id("spark-1"));
    assert!(
        subscription.validate().is_err(),
        "a subscription has no placement machine"
    );

    let mut api = local_binding("api", WorkloadClass::LargeModelServing);
    api.kind = BindingKind::Api;
    api.placement = None;
    api.credential = Some(CredentialReference {
        provider: id("infisical"),
        name: "provider-api-key".into(),
    });
    api.endpoint = EndpointLocator::InProcess {
        component: id("provider"),
    };
    assert!(
        api.validate().is_err(),
        "an api binding is reached through a url"
    );

    let mut local = local_binding("local", WorkloadClass::VectorStore);
    local.placement = None;
    assert!(
        local.validate().is_err(),
        "a local binding names its placement machine"
    );

    let mut self_hosted = local_binding("self-hosted", WorkloadClass::ControlPlane);
    self_hosted.kind = BindingKind::SelfHosted;
    self_hosted.endpoint = EndpointLocator::InProcess {
        component: id("warden"),
    };
    assert!(
        self_hosted.validate().is_err(),
        "a self-hosted binding is reached through a url"
    );

    let mut with_user_information = local_binding("user-info", WorkloadClass::ControlPlane);
    with_user_information.endpoint = EndpointLocator::Url {
        url: "https://operator:secret@warden.estate.example".into(),
    };
    assert!(
        with_user_information.validate().is_err(),
        "credentials never ride in the url"
    );

    let mut without_scheme = local_binding("no-scheme", WorkloadClass::ControlPlane);
    without_scheme.endpoint = EndpointLocator::Url {
        url: "warden.estate.example:443".into(),
    };
    assert!(without_scheme.validate().is_err());

    let mut whitespace_credential = local_binding("whitespace", WorkloadClass::LargeModelServing);
    whitespace_credential.kind = BindingKind::Api;
    whitespace_credential.placement = None;
    whitespace_credential.credential = Some(CredentialReference {
        provider: id("infisical"),
        name: "provider api key".into(),
    });
    assert!(whitespace_credential.validate().is_err());
}

#[test]
fn placement_validation_rejects_unknown_machines_and_overflow() {
    let inventory = HardwareInventory {
        contract_version: HARDWARE_INVENTORY_CONTRACT_VERSION,
        machines: vec![
            MachineCapacity {
                id: id("spark-3"),
                memory_bytes: 4 * GIB,
                gpu: Some(GpuCapacity { memory_bytes: GIB }),
                disk_bytes: 64 * GIB,
            },
            MachineCapacity {
                id: id("proxmox-1"),
                memory_bytes: 4 * GIB,
                gpu: None,
                disk_bytes: 64 * GIB,
            },
        ],
    };

    let mut elsewhere = local_binding("elsewhere", WorkloadClass::VectorStore);
    elsewhere.placement = Some(id("spark-9"));
    assert!(profile_of(vec![elsewhere])
        .unwrap()
        .validate_placement(&inventory)
        .is_err());

    let fits = profile_of(vec![
        local_binding("one", WorkloadClass::VectorStore),
        local_binding("two", WorkloadClass::CanonicalStore),
        local_binding("three", WorkloadClass::AlwaysWarmSmallModels),
        local_binding("four", WorkloadClass::EmbeddingAndReranking),
    ])
    .unwrap();
    let loads = fits.validate_placement(&inventory).unwrap();
    assert_eq!(loads.len(), 2);
    assert_eq!(
        loads
            .iter()
            .find(|load| load.machine_id.as_str() == "spark-3")
            .unwrap()
            .memory_declared_bytes,
        4 * GIB
    );

    let overflows = profile_of(vec![
        local_binding("one", WorkloadClass::VectorStore),
        local_binding("two", WorkloadClass::CanonicalStore),
        local_binding("three", WorkloadClass::AlwaysWarmSmallModels),
        local_binding("four", WorkloadClass::EmbeddingAndReranking),
        local_binding("five", WorkloadClass::ControlPlane),
    ])
    .unwrap();
    assert!(overflows.validate_placement(&inventory).is_err());

    let mut needs_gpu = local_binding("gpu", WorkloadClass::EmbeddingAndReranking);
    needs_gpu.footprint.gpu = GpuRequirement::Required { memory_bytes: GIB };
    profile_of(vec![needs_gpu.clone()])
        .unwrap()
        .validate_placement(&inventory)
        .unwrap();
    needs_gpu.placement = Some(id("proxmox-1"));
    assert!(
        profile_of(vec![needs_gpu.clone()])
            .unwrap()
            .validate_placement(&inventory)
            .is_err(),
        "a GPU binding cannot be placed on a machine without one"
    );
    needs_gpu.placement = Some(id("spark-3"));
    needs_gpu.footprint.gpu = GpuRequirement::Required {
        memory_bytes: GIB + 1,
    };
    assert!(profile_of(vec![needs_gpu])
        .unwrap()
        .validate_placement(&inventory)
        .is_err());
}

#[test]
fn profile_rejects_duplicates_zero_revision_bad_versions_and_digest_drift() {
    assert!(profile_of(vec![
        local_binding("same", WorkloadClass::VectorStore),
        local_binding("same", WorkloadClass::CanonicalStore),
    ])
    .is_err());

    assert!(EndpointProfile::from_input(0, ring_3_input()).is_err());

    let mut wrong_version = ring_3_input();
    wrong_version.contract_version = ENDPOINT_PROFILE_CONTRACT_VERSION + 1;
    assert!(wrong_version.validate().is_err());

    let mut drifted = EndpointProfile::from_input(1, ring_3_input()).unwrap();
    drifted.bindings[0].limits.max_concurrency += 1;
    assert!(drifted.validate().is_err());

    let mut empty = ring_3_input();
    empty.bindings.clear();
    assert!(empty.validate().is_err());

    let mut wrong_inventory_version = ring_3_inventory();
    wrong_inventory_version.contract_version = HARDWARE_INVENTORY_CONTRACT_VERSION + 1;
    assert!(wrong_inventory_version.validate().is_err());

    let mut duplicate_machine = ring_3_inventory();
    duplicate_machine.machines[1].id = duplicate_machine.machines[0].id.clone();
    assert!(duplicate_machine.validate().is_err());
}

#[test]
fn json_schema_generation_covers_the_profile_and_inventory() {
    let profile = serde_json::to_value(schemars::schema_for!(EndpointProfile)).unwrap();
    let rendered = profile.to_string();
    for required in [
        "\"bindings\"",
        "\"profile_sha256\"",
        "large_model_serving",
        "always_warm_small_models",
        "embedding_and_reranking",
        "vector_store",
        "canonical_store",
        "weight_updates",
        "behavior_updates",
        "control_plane",
        "self_hosted",
        "subscription",
        "in_process",
    ] {
        assert!(rendered.contains(required), "schema lacks {required}");
    }
    assert!(
        !rendered.contains("\"value\""),
        "a credential reference schema must carry no value field"
    );

    let inventory = serde_json::to_value(schemars::schema_for!(HardwareInventory)).unwrap();
    let rendered = inventory.to_string();
    for required in ["\"machines\"", "\"memory_bytes\"", "\"disk_bytes\""] {
        assert!(rendered.contains(required), "schema lacks {required}");
    }
}
