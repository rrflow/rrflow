use rrd_contract::*;

fn identity() -> InstalledEstateIdentity {
    InstalledEstateIdentity {
        project_id: CanonicalId::new("project").unwrap(),
        estate_id: CanonicalId::new("estate").unwrap(),
        instance_id: CanonicalId::new("instance").unwrap(),
    }
}
fn plan() -> EndpointConfigurationPlan {
    let current = EndpointConfiguration {
        format_version: ENDPOINT_CONFIGURATION_VERSION,
        identity: identity(),
        revision: 0,
        profile: None,
        last_applied_plan_sha256: None,
    };
    let mut plan = EndpointConfigurationPlan {
        format_version: ENDPOINT_CONFIGURATION_VERSION,
        identity: identity(),
        expected_revision: 0,
        expected_configuration_sha256: current.sha256().unwrap(),
        profile: None,
        issued_at_unix_ms: 1,
        expires_at_unix_ms: 1 + ENDPOINT_CONFIGURATION_PREVIEW_TTL_MS,
        plan_sha256: String::new(),
    };
    plan.plan_sha256 = plan.sha256().unwrap();
    plan
}
#[test]
fn configuration_contract_binds_identity_revision_content_expiry_and_exact_approval() {
    let original = plan();
    original.validate().unwrap();
    let mut oversized_digest = original.clone();
    oversized_digest.plan_sha256 = "a".repeat(MAX_ENDPOINT_CONFIGURATION_BYTES + 1);
    assert!(oversized_digest.validate().is_err());
    let mut changed = original.clone();
    changed.identity.project_id = CanonicalId::new("other-project").unwrap();
    assert!(changed.validate().is_err());
    changed = original.clone();
    changed.expected_revision = u64::MAX;
    changed.plan_sha256 = changed.sha256().unwrap();
    assert!(changed.validate().is_err());
    changed = original.clone();
    changed.expires_at_unix_ms += 1;
    changed.plan_sha256 = changed.sha256().unwrap();
    assert!(changed.validate().is_err());
    assert!(ApplyEndpointConfiguration {
        plan: original.clone(),
        approved_plan_sha256: "0".repeat(64)
    }
    .validate()
    .is_err());
    ApplyEndpointConfiguration {
        plan: original.clone(),
        approved_plan_sha256: original.plan_sha256.clone(),
    }
    .validate()
    .unwrap();
    let mut json = serde_json::to_value(&original).unwrap();
    json["secret_value"] = "must-not-be-accepted".into();
    assert!(serde_json::from_value::<EndpointConfigurationPlan>(json).is_err());
    assert!(PreviewEndpointConfiguration {
        expected_revision: u64::MAX,
        profile: None
    }
    .validate()
    .is_err());
}
#[test]
fn configuration_catalogue_projects_authenticated_read_preview_and_mutation() {
    let catalogue = endpoint_catalogue();
    let openapi = openapi_document().unwrap();
    for (op, mutation, action) in [
        ("read", false, SecurityAction::EstateRead),
        ("preview", false, SecurityAction::EstateAdmin),
        ("apply", true, SecurityAction::EstateAdmin),
    ] {
        let name = format!("endpoint-configuration-{op}");
        let endpoint = catalogue
            .endpoints
            .iter()
            .find(|e| e.operation.as_str() == name)
            .unwrap();
        assert_eq!(
            endpoint.authentication,
            EndpointAuthentication::SessionBearer
        );
        assert_eq!(endpoint.mutation, mutation);
        assert_eq!(endpoint.action.fixed_action(), Some(action));
        assert!(openapi["paths"][&endpoint.path]["post"]["requestBody"].is_object());
    }
}
