use rrd_contract::{
    engram_configuration_sha256, CanonicalId, ClockPolicy, DeploymentForm, DeploymentProfile,
    EndpointPresentation, EngramConfiguration, EngramConfigurationInput, InstalledEngramIdentity,
    QueryBudget, ReasoningLimits, RecallLimits, StorageProfileKind,
    ENGRAM_CONFIGURATION_FORMAT_VERSION, MAX_CLOCK_ROLLBACK_MS,
};

fn identity() -> InstalledEngramIdentity {
    InstalledEngramIdentity {
        project_id: CanonicalId::new("rrflow").unwrap(),
        engram_id: CanonicalId::new("local-engram").unwrap(),
        instance_id: CanonicalId::new("rrflow-local").unwrap(),
    }
}

fn input() -> EngramConfigurationInput {
    EngramConfigurationInput {
        format_version: ENGRAM_CONFIGURATION_FORMAT_VERSION,
        storage: rrd_contract::StorageLimits::default(),
        clock: ClockPolicy {
            maximum_rollback_ms: 0,
        },
        reasoning: ReasoningLimits {
            max_run_elapsed_ms: 900_000,
            max_steps: 256,
            max_step_elapsed_ms: 60_000,
        },
        recall: RecallLimits {
            max_graph_depth: 4,
            max_items: 128,
            max_output_bytes: 512 * 1024,
            max_storage_keys: 100_000,
        },
        query: QueryBudget::default(),
        attunement_follow: None,
    }
}

#[test]
fn canonical_identity_deployment_and_configuration_round_trip() {
    let identity = identity();
    identity.validate().unwrap();
    assert_eq!(identity.to_string(), "rrflow/local-engram/rrflow-local");

    let profile = DeploymentProfile {
        contract_version: 1,
        deployment_form: DeploymentForm::SingleNodeServer,
        storage_profile: StorageProfileKind::RrflowKv,
        endpoint_presentation: EndpointPresentation::LoopbackHttpWebsocket,
    };
    profile.validate().unwrap();

    let configuration = EngramConfiguration::from_input(7, input()).unwrap();
    configuration.validate().unwrap();
    assert_eq!(
        configuration.configuration_sha256,
        engram_configuration_sha256(&configuration).unwrap()
    );
    let encoded = serde_json::to_vec(&configuration).unwrap();
    let decoded: EngramConfiguration = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(decoded, configuration);
}

#[test]
fn configuration_rejects_invalid_limits_and_digest_drift() {
    let mut invalid = input();
    invalid.reasoning.max_steps = 0;
    assert!(EngramConfiguration::from_input(1, invalid).is_err());

    let mut configuration = EngramConfiguration::from_input(1, input()).unwrap();
    configuration.query.max_rows += 1;
    assert!(configuration.validate().is_err());
}

#[test]
fn read_retention_ceiling_is_positive_bounded_and_sealed() {
    let baseline = EngramConfiguration::from_input(1, input()).unwrap();
    for (views, bytes) in [(0, 1), (65_537, 1), (1, 0), (1, (1 << 40) + 1)] {
        let mut invalid = input();
        invalid.storage.native_transactions = views;
        invalid.storage.native_transaction_bytes = bytes;
        assert!(EngramConfiguration::from_input(1, invalid).is_err());
        let mut invalid = input();
        invalid.storage.active_read_views = views;
        invalid.storage.pinned_read_bytes = bytes;
        assert!(EngramConfiguration::from_input(1, invalid).is_err());
    }
    for field in 0..4 {
        let mut changed = input();
        if field == 0 {
            changed.storage.active_read_views += 1;
        } else if field == 1 {
            changed.storage.native_transactions += 1;
        } else if field == 2 {
            changed.storage.native_transaction_bytes += 1;
        } else {
            changed.storage.pinned_read_bytes += 1;
        }
        let changed = EngramConfiguration::from_input(1, changed).unwrap();
        assert_ne!(baseline.configuration_sha256, changed.configuration_sha256);
        let mut tampered = baseline.clone();
        tampered.storage = changed.storage;
        assert!(tampered.validate().is_err());
    }
}

#[test]
fn recovery_reserve_is_positive_bounded_and_sealed() {
    let baseline = EngramConfiguration::from_input(1, input()).unwrap();
    for reserve in [0, (1 << 40) + 1] {
        let mut invalid = input();
        invalid.storage.recovery_reserve_bytes = reserve;
        assert!(EngramConfiguration::from_input(1, invalid).is_err());
    }
    let mut changed = input();
    changed.storage.recovery_reserve_bytes += 1;
    let changed = EngramConfiguration::from_input(1, changed).unwrap();
    assert_ne!(baseline.configuration_sha256, changed.configuration_sha256);
    let mut tampered = baseline;
    tampered.storage.recovery_reserve_bytes += 1;
    assert!(tampered.validate().is_err());
}

#[test]
fn clock_rollback_policy_is_strict_bounded_and_digest_bound() {
    let strict = EngramConfiguration::from_input(1, input()).unwrap();
    assert_eq!(strict.clock.maximum_rollback_ms, 0);

    let mut maximum = input();
    maximum.clock.maximum_rollback_ms = MAX_CLOCK_ROLLBACK_MS;
    let maximum = EngramConfiguration::from_input(1, maximum).unwrap();
    assert_ne!(maximum.configuration_sha256, strict.configuration_sha256);

    let mut unbounded = input();
    unbounded.clock.maximum_rollback_ms = MAX_CLOCK_ROLLBACK_MS + 1;
    assert!(EngramConfiguration::from_input(1, unbounded).is_err());
}

#[test]
fn deployment_axes_reject_inconsistent_combinations() {
    let embedded_network = DeploymentProfile {
        contract_version: 1,
        deployment_form: DeploymentForm::Embedded,
        storage_profile: StorageProfileKind::RrflowMx,
        endpoint_presentation: EndpointPresentation::NetworkHttpWebsocket,
    };
    assert!(embedded_network.validate().is_err());

    let clustered_loopback = DeploymentProfile {
        contract_version: 1,
        deployment_form: DeploymentForm::ClusteredServer,
        storage_profile: StorageProfileKind::RrflowKv,
        endpoint_presentation: EndpointPresentation::LoopbackHttpWebsocket,
    };
    assert!(clustered_loopback.validate().is_err());

    let clustered_volatile = DeploymentProfile {
        contract_version: 1,
        deployment_form: DeploymentForm::ClusteredServer,
        storage_profile: StorageProfileKind::RrflowMx,
        endpoint_presentation: EndpointPresentation::NetworkHttpWebsocket,
    };
    assert!(clustered_volatile.validate().is_err());
}

#[test]
fn strict_contract_rejects_unknown_and_legacy_scalar_fields() {
    let configuration = EngramConfiguration::from_input(1, input()).unwrap();
    let mut value = serde_json::to_value(configuration).unwrap();
    value["deployment_mode"] = serde_json::json!("local_daemon");
    assert!(serde_json::from_value::<EngramConfiguration>(value).is_err());

    let identity = serde_json::json!({
        "organization_id": "local",
        "project_id": "rrflow",
        "engram_id": "local-engram",
        "instance_id": "rrflow-local"
    });
    assert!(serde_json::from_value::<InstalledEngramIdentity>(identity).is_err());
}

#[test]
fn checkpoint_quota_is_finite_restart_configuration_and_digest_bound() {
    for entries in [0, 65_537, u64::MAX] {
        let mut config = input();
        config.storage.checkpoint_entries = entries;
        assert!(EngramConfiguration::from_input(1, config).is_err());
    }
    let baseline = EngramConfiguration::from_input(1, input()).unwrap();
    let mut changed = input();
    changed.storage.checkpoint_entries = 7;
    let changed = EngramConfiguration::from_input(1, changed).unwrap();
    assert_ne!(baseline.configuration_sha256, changed.configuration_sha256);
    let mut tampered = baseline;
    tampered.storage.checkpoint_entries = 7;
    assert!(tampered.validate().is_err());
}

fn cld_policy(input: u64, output: u64, memory: u64) -> rrd_contract::ColdHistoryPolicy {
    rrd_contract::ColdHistoryPolicy {
        interval_ms: 1_000,
        max_changes: 512,
        max_input_bytes: input,
        max_output_bytes: output,
        max_memory_bytes: memory,
        max_elapsed_ms: 30_000,
        row_group_rows: 64,
        compression: rrd_contract::ColdHistoryCompression::Zstd { level: 3 },
        dictionary: true,
        statistics: true,
    }
}

/// A policy must be able to encode the largest page it admits. On a real
/// install a 1.19 MB change under 8 MiB bounds and a 64 MiB budget was
/// refused by the encoder forever, stalling rrflowCLD with no way forward.
#[test]
fn rrflow_cld_policy_that_cannot_encode_its_own_input_is_refused() {
    const MIB: u64 = 1024 * 1024;
    assert!(
        cld_policy(8 * MIB, 8 * MIB, 64 * MIB).validate().is_err(),
        "the budget cannot hold the working set of one full input page"
    );
    let required = cld_policy(8 * MIB, 8 * MIB, 4 * 1024 * MIB).minimum_working_memory_bytes();
    assert!(cld_policy(8 * MIB, 8 * MIB, required).validate().is_ok());
    assert!(cld_policy(8 * MIB, 8 * MIB, required - 1)
        .validate()
        .is_err());
}
