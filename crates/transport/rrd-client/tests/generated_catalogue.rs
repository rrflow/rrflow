#[path = "../build_support.rs"]
mod build_support;

use rrd_contract::{EndpointAuthentication, EndpointDescriptor, HttpMethod};

fn fixed_session_post(endpoint: &EndpointDescriptor) -> bool {
    endpoint.authentication == EndpointAuthentication::SessionBearer
        && endpoint.method == HttpMethod::Post
        && !endpoint.path.contains('{')
}

#[test]
fn generated_catalogue_is_deterministic_complete_and_fails_closed() {
    let catalogue = rrd_contract::endpoint_catalogue();
    let first = build_support::render(&catalogue).unwrap();
    assert_eq!(first, build_support::render(&catalogue).unwrap());
    let mut reordered = catalogue.clone();
    reordered.endpoints.reverse();
    assert!(build_support::render(&reordered).is_err());
    let generated_ids = first
        .lines()
        .filter(|line| line.trim_start().starts_with("const ID:"))
        .count();
    assert_eq!(generated_ids, catalogue.endpoints.len());
    for endpoint in &catalogue.endpoints {
        let expected = format!(
            "const ID: &'static str = {:?};",
            endpoint.operation.as_str()
        );
        assert_eq!(
            first.matches(&expected).count(),
            1,
            "{}",
            endpoint.operation
        );
    }
    let mut invalid = catalogue.clone();
    invalid.endpoints[0].request_type = "Bad<Type>".into();
    assert!(build_support::render(&invalid).is_err());
    let mut duplicate = catalogue.clone();
    duplicate.endpoints.push(duplicate.endpoints[0].clone());
    assert!(build_support::render(&duplicate).is_err());
}

#[test]
fn fixed_session_metadata_has_exact_catalogue_coverage() {
    let catalogue = rrd_contract::endpoint_catalogue();
    let output = build_support::render(&catalogue).unwrap();
    let selected = catalogue.endpoints.iter().filter(|e| fixed_session_post(e));
    assert_eq!(
        output.matches("impl crate::OperationMetadata for ").count(),
        selected.clone().count()
    );
    for endpoint in selected {
        assert_eq!(
            output
                .matches(&format!("const PATH: &'static str = {:?};", endpoint.path))
                .count(),
            1
        );
    }
    // The callable public marker and the landed internal attunement methods
    // compile against this same build-time projection, not a saved Rust copy.
    use rrd_client::{operations::TaskEndpointPropose, OperationMetadata};
    let proposal = catalogue
        .endpoints
        .iter()
        .find(|e| e.operation.as_str() == TaskEndpointPropose::OPERATION)
        .unwrap();
    assert_eq!(TaskEndpointPropose::PATH, proposal.path);
    assert_eq!(TaskEndpointPropose::MUTATION, proposal.mutation);
    fn callable<O: rrd_client::ValidatedOperation>() {}
    callable::<rrd_client::operations::TaskWorkerExecute>();
    callable::<rrd_client::operations::TaskBudgetResume>();
}

#[test]
fn changed_owner_metadata_and_additions_flow_into_the_projection() {
    // Perturb the real catalogue solely to test the compiler, never as routing data.
    let mut catalogue = rrd_contract::endpoint_catalogue();
    let index = catalogue
        .endpoints
        .iter()
        .position(fixed_session_post)
        .unwrap();
    let mut added = catalogue.endpoints[index].clone();
    added.operation = rrd_contract::CanonicalId::new("zz-projection-probe").unwrap();
    added.path.push_str("/projection-probe");
    added.mutation = !added.mutation;
    // The catalogue rejects a retry class that contradicts the mutation flag.
    added.retry = if added.mutation {
        rrd_contract::EndpointRetry::KeyedMutation
    } else {
        rrd_contract::EndpointRetry::Read
    };
    let marker = "pub struct ZzProjectionProbe;";
    assert!(!build_support::render(&catalogue).unwrap().contains(marker));
    catalogue.endpoints.push(added.clone());
    catalogue
        .endpoints
        .sort_by(|a, b| a.operation.cmp(&b.operation));
    let output = build_support::render(&catalogue).unwrap();
    assert!(output.contains(marker));
    let metadata = output
        .split("impl crate::OperationMetadata for ZzProjectionProbe {")
        .nth(1)
        .unwrap()
        .split('}')
        .next()
        .unwrap();
    assert!(metadata.contains(&format!("const PATH: &'static str = {:?};", added.path)));
    assert!(metadata.contains(&format!("const MUTATION: bool = {};", added.mutation)));
    catalogue
        .endpoints
        .retain(|e| e.operation != added.operation);
    assert!(!build_support::render(&catalogue).unwrap().contains(marker));
}

#[test]
fn unsupported_transport_has_no_callable_metadata() {
    let mut catalogue = rrd_contract::endpoint_catalogue();
    let index = catalogue
        .endpoints
        .iter()
        .position(fixed_session_post)
        .unwrap();
    let path = catalogue.endpoints[index].path.clone();
    catalogue.endpoints[index].authentication = EndpointAuthentication::Public;
    let output = build_support::render(&catalogue).unwrap();
    assert!(!output.contains(&format!("const PATH: &'static str = {path:?};")));
    catalogue.endpoints[index].authentication = EndpointAuthentication::SessionBearer;
    catalogue.endpoints[index].path.push_str("/{id}");
    assert!(!build_support::render(&catalogue)
        .unwrap()
        .contains(&format!(
            "const PATH: &'static str = {:?};",
            catalogue.endpoints[index].path
        )));
}

#[test]
fn invalid_marker_path_and_empty_catalogue_are_rejected() {
    let catalogue = rrd_contract::endpoint_catalogue();
    let index = catalogue
        .endpoints
        .iter()
        .position(fixed_session_post)
        .unwrap();
    let mut invalid = catalogue.clone();
    invalid.endpoints[index].path.push_str("/bad\"path");
    assert!(build_support::render(&invalid)
        .unwrap_err()
        .contains("path"));
    let mut invalid = catalogue.clone();
    invalid.endpoints[index].operation = rrd_contract::CanonicalId::new("not_a_marker").unwrap();
    invalid
        .endpoints
        .sort_by(|a, b| a.operation.cmp(&b.operation));
    assert!(build_support::render(&invalid)
        .unwrap_err()
        .contains("operation name"));
    let mut invalid = catalogue;
    invalid.endpoints.clear();
    assert!(build_support::render(&invalid).is_err());
}
