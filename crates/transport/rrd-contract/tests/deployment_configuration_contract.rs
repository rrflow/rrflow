use rrd_contract::{
    estate_configuration_sha256, CanonicalId, ClockPolicy, DeploymentForm, DeploymentProfile,
    EndpointPresentation, EstateConfiguration, EstateConfigurationInput, InstalledEstateIdentity,
    QueryBudget, ReasoningLimits, RecallLimits, StorageProfileKind,
    ESTATE_CONFIGURATION_FORMAT_VERSION, MAX_CLOCK_ROLLBACK_MS,
};

fn identity() -> InstalledEstateIdentity {
    InstalledEstateIdentity {
        project_id: CanonicalId::new("rrflow").unwrap(),
        estate_id: CanonicalId::new("local-estate").unwrap(),
        instance_id: CanonicalId::new("rrflow-local").unwrap(),
    }
}

fn input() -> EstateConfigurationInput {
    EstateConfigurationInput {
        format_version: ESTATE_CONFIGURATION_FORMAT_VERSION,
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
    }
}

#[test]
fn canonical_identity_deployment_and_configuration_round_trip() {
    let identity = identity();
    identity.validate().unwrap();
    assert_eq!(identity.to_string(), "rrflow/local-estate/rrflow-local");

    let profile = DeploymentProfile {
        contract_version: 1,
        deployment_form: DeploymentForm::SingleNodeServer,
        storage_profile: StorageProfileKind::RrflowKv,
        endpoint_presentation: EndpointPresentation::LoopbackHttpWebsocket,
    };
    profile.validate().unwrap();

    let configuration = EstateConfiguration::from_input(7, input()).unwrap();
    configuration.validate().unwrap();
    assert_eq!(
        configuration.configuration_sha256,
        estate_configuration_sha256(&configuration).unwrap()
    );
    let encoded = serde_json::to_vec(&configuration).unwrap();
    let decoded: EstateConfiguration = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(decoded, configuration);
}

#[test]
fn configuration_rejects_invalid_limits_and_digest_drift() {
    let mut invalid = input();
    invalid.reasoning.max_steps = 0;
    assert!(EstateConfiguration::from_input(1, invalid).is_err());

    let mut configuration = EstateConfiguration::from_input(1, input()).unwrap();
    configuration.query.max_rows += 1;
    assert!(configuration.validate().is_err());
}

#[test]
fn clock_rollback_policy_is_strict_bounded_and_digest_bound() {
    let strict = EstateConfiguration::from_input(1, input()).unwrap();
    assert_eq!(strict.clock.maximum_rollback_ms, 0);

    let mut maximum = input();
    maximum.clock.maximum_rollback_ms = MAX_CLOCK_ROLLBACK_MS;
    let maximum = EstateConfiguration::from_input(1, maximum).unwrap();
    assert_ne!(maximum.configuration_sha256, strict.configuration_sha256);

    let mut unbounded = input();
    unbounded.clock.maximum_rollback_ms = MAX_CLOCK_ROLLBACK_MS + 1;
    assert!(EstateConfiguration::from_input(1, unbounded).is_err());
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
    let configuration = EstateConfiguration::from_input(1, input()).unwrap();
    let mut value = serde_json::to_value(configuration).unwrap();
    value["deployment_mode"] = serde_json::json!("local_daemon");
    assert!(serde_json::from_value::<EstateConfiguration>(value).is_err());

    let identity = serde_json::json!({
        "organization_id": "local",
        "project_id": "rrflow",
        "estate_id": "local-estate",
        "instance_id": "rrflow-local"
    });
    assert!(serde_json::from_value::<InstalledEstateIdentity>(identity).is_err());
}
