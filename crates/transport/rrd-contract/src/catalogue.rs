//! The endpoint catalogue: every public operation, its route, authentication and security action.

use crate::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Declares one family's operations once. Each row emits the catalogue
/// descriptor (type names via `stringify!`) and a marker whose associated
/// request and response are the real contract types, so a wrong type name is
/// a compile error rather than a string that drifts.
macro_rules! operations {
    ($(
        $marker:ident = $operation:literal,
        $method:ident $path:literal,
        $authentication:ident,
        $mutation:literal,
        $action:ident,
        $request:ident => $response:ident,
        $retry:ident;
    )+) => {
        /// Typed markers for this family's operations.
        pub mod markers {
            $(
                #[derive(Debug, Clone, Copy, PartialEq, Eq)]
                pub struct $marker;
                impl crate::Operation for $marker {
                    type Request = crate::$request;
                    type Response = crate::$response;
                    const ID: &'static str = $operation;
                    const METHOD: crate::HttpMethod = crate::HttpMethod::$method;
                    const PATH: &'static str = $path;
                    const AUTHENTICATION: crate::EndpointAuthentication =
                        crate::EndpointAuthentication::$authentication;
                    const MUTATION: bool = $mutation;
                    const RETRY: crate::EndpointRetry = crate::EndpointRetry::$retry;
                }
            )+
        }

        pub(crate) fn endpoints() -> Vec<EndpointDescriptor> {
            vec![$(
                endpoint(
                    $operation,
                    HttpMethod::$method,
                    $path,
                    EndpointAuthentication::$authentication,
                    $mutation,
                    SecurityAction::$action,
                    (stringify!($request), stringify!($response)),
                    EndpointRetry::$retry,
                ),
            )+]
        }
    };
}

mod data;
mod lifecycle;
mod operation;
mod platform;
mod reasoning;
mod vector;

pub use operation::{Empty, EndpointRetry, OpenApiDocument, Operation};

/// One typed marker per catalogue operation, generated from the same rows as
/// `endpoint_catalogue()`.
pub mod operations {
    pub use super::data::markers::*;
    pub use super::lifecycle::markers::*;
    pub use super::platform::markers::*;
    pub use super::reasoning::markers::*;
    pub use super::vector::markers::*;
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum HttpMethod {
    Get,
    Post,
    Delete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EndpointAuthentication {
    Public,
    ApiKey,
    SessionBearer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "source", rename_all = "snake_case")]
pub enum EndpointAction {
    Fixed { action: SecurityAction },
}

impl EndpointAction {
    pub const fn fixed(action: SecurityAction) -> Self {
        Self::Fixed { action }
    }

    pub const fn fixed_action(self) -> Option<SecurityAction> {
        match self {
            Self::Fixed { action } => Some(action),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EndpointDescriptor {
    pub operation: CanonicalId,
    pub method: HttpMethod,
    pub path: String,
    pub authentication: EndpointAuthentication,
    pub mutation: bool,
    pub action: EndpointAction,
    pub request_type: String,
    pub response_type: String,
    /// What a client may do when the outcome of an attempt is unknown.
    pub retry: EndpointRetry,
    /// The reference group; it carries no authorization meaning.
    pub family: OperationFamily,
    /// One plain line naming what the operation does.
    pub summary: String,
    /// Whole sentences describing what the engine implements for it.
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WebSocketEndpointDescriptor {
    pub operation: CanonicalId,
    pub path: String,
    pub authentication: EndpointAuthentication,
    pub connect_action: SecurityAction,
    pub frame_type: String,
    pub family: OperationFamily,
    pub summary: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EndpointCatalogue {
    pub protocol: String,
    pub protocol_version: u16,
    pub endpoints: Vec<EndpointDescriptor>,
    pub websocket_endpoints: Vec<WebSocketEndpointDescriptor>,
    /// Every operation family, in reference order, with its title and
    /// description.
    pub families: Vec<OperationFamilyDescriptor>,
}

impl EndpointCatalogue {
    /// Resolves one concrete HTTP request against the authoritative endpoint
    /// templates. Transports use this instead of maintaining a second route
    /// capability table.
    pub fn resolve_http(&self, method: HttpMethod, path: &str) -> Option<&EndpointDescriptor> {
        self.endpoints.iter().find(|endpoint| {
            endpoint.method == method && endpoint_path_matches(&endpoint.path, path)
        })
    }

    pub fn validate(&self) -> Result<()> {
        validate_protocol(&self.protocol, self.protocol_version)?;
        if self.endpoints.is_empty() || self.endpoints.len() > 256 {
            return invalid("endpoint catalogue must contain 1..=256 endpoints");
        }
        if self.websocket_endpoints.len() > 32 {
            return invalid("endpoint catalogue may contain at most 32 WebSocket endpoints");
        }
        let mut operations = BTreeSet::new();
        let mut routes = BTreeSet::new();
        for endpoint in &self.endpoints {
            if endpoint.path.is_empty()
                || endpoint.path.len() > 256
                || !endpoint.path.starts_with("/v1/")
                || !endpoint.path.is_ascii()
                || endpoint.request_type.is_empty()
                || endpoint.request_type.len() > 128
                || endpoint.response_type.is_empty()
                || endpoint.response_type.len() > 128
                || !endpoint.request_type.is_ascii()
                || !endpoint.response_type.is_ascii()
            {
                return invalid("endpoint descriptor contains an invalid path or type name");
            }
            if !operations.insert(endpoint.operation.clone())
                || !routes.insert((endpoint.method, endpoint.path.clone()))
            {
                return invalid("endpoint operations and method/path pairs must be unique");
            }
            if endpoint.mutation && endpoint.method == HttpMethod::Get {
                return invalid("mutating endpoints may not use GET");
            }
            if (endpoint.retry == EndpointRetry::Read) == endpoint.mutation {
                return invalid("only non-mutating endpoints may declare read retry");
            }
            if !operation_reference::valid_summary(&endpoint.summary)
                || !operation_reference::valid_description(&endpoint.description)
            {
                return invalid(
                    "endpoint descriptor needs a one-line summary and a description of whole sentences",
                );
            }
        }
        if self
            .endpoints
            .windows(2)
            .any(|pair| pair[0].operation > pair[1].operation)
        {
            return invalid("endpoint descriptors must be sorted by operation");
        }
        let mut websocket_operations = BTreeSet::new();
        let mut websocket_paths = BTreeSet::new();
        for endpoint in &self.websocket_endpoints {
            if endpoint.path.is_empty()
                || endpoint.path.len() > 256
                || !endpoint.path.starts_with("/v1/")
                || !endpoint.path.is_ascii()
                || endpoint.frame_type.is_empty()
                || endpoint.frame_type.len() > 128
                || !endpoint.frame_type.is_ascii()
                || endpoint.authentication != EndpointAuthentication::SessionBearer
                || !operation_reference::valid_summary(&endpoint.summary)
                || !operation_reference::valid_description(&endpoint.description)
            {
                return invalid("WebSocket endpoint descriptor is invalid");
            }
            if !websocket_operations.insert(endpoint.operation.clone())
                || !websocket_paths.insert(endpoint.path.clone())
            {
                return invalid("WebSocket endpoint operations and paths must be unique");
            }
        }
        if self
            .websocket_endpoints
            .windows(2)
            .any(|pair| pair[0].operation > pair[1].operation)
        {
            return invalid("WebSocket endpoint descriptors must be sorted by operation");
        }
        if self.families.is_empty() || self.families.len() > 32 {
            return invalid("endpoint catalogue must describe 1..=32 operation families");
        }
        let mut families = BTreeSet::new();
        for family in &self.families {
            if !families.insert(family.family)
                || !operation_reference::valid_summary(&family.title)
                || !operation_reference::valid_description(&family.description)
            {
                return invalid(
                    "operation families must be unique with a one-line title and a description",
                );
            }
        }
        if self
            .endpoints
            .iter()
            .map(|endpoint| endpoint.family)
            .chain(
                self.websocket_endpoints
                    .iter()
                    .map(|endpoint| endpoint.family),
            )
            .any(|family| !families.contains(&family))
        {
            return invalid("every operation must belong to a described family");
        }
        Ok(())
    }
}

pub(crate) fn endpoint_path_matches(template: &str, concrete: &str) -> bool {
    let mut template_segments = template.split('/');
    let mut concrete_segments = concrete.split('/');
    loop {
        match (template_segments.next(), concrete_segments.next()) {
            (Some(expected), Some(actual)) => {
                let parameter = expected.starts_with('{') && expected.ends_with('}');
                if (parameter && actual.is_empty()) || (!parameter && expected != actual) {
                    return false;
                }
            }
            (None, None) => return true,
            _ => return false,
        }
    }
}

pub fn endpoint_catalogue() -> EndpointCatalogue {
    let mut endpoints = Vec::new();
    endpoints.extend(reasoning::endpoints());
    endpoints.extend(vector::endpoints());
    endpoints.extend(lifecycle::endpoints());
    endpoints.extend(data::endpoints());
    endpoints.extend(platform::endpoints());
    endpoints.sort_by(|left, right| left.operation.cmp(&right.operation));
    let catalogue = EndpointCatalogue {
        protocol: PROTOCOL.into(),
        protocol_version: PROTOCOL_VERSION,
        endpoints,
        websocket_endpoints: vec![websocket_endpoint(
            "websocket-connect",
            "/v1/ws",
            SecurityAction::WebSocketConnect,
            "WebSocketFrame",
        )],
        families: operation_families(),
    };
    debug_assert!(catalogue.validate().is_ok());
    catalogue
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn endpoint(
    operation: &str,
    method: HttpMethod,
    path: &str,
    authentication: EndpointAuthentication,
    mutation: bool,
    action: SecurityAction,
    (request_type, response_type): (&str, &str),
    retry: EndpointRetry,
) -> EndpointDescriptor {
    let reference = operation_reference::operation_reference(operation)
        .expect("every catalogue operation has reference text");
    EndpointDescriptor {
        operation: CanonicalId::new(operation).expect("static endpoint operation is canonical"),
        method,
        path: path.into(),
        authentication,
        mutation,
        action: EndpointAction::fixed(action),
        request_type: request_type.into(),
        response_type: response_type.into(),
        retry,
        family: reference.family,
        summary: reference.summary.into(),
        description: reference.description.into(),
    }
}

pub(crate) fn websocket_endpoint(
    operation: &str,
    path: &str,
    connect_action: SecurityAction,
    frame_type: &str,
) -> WebSocketEndpointDescriptor {
    let reference = operation_reference::operation_reference(operation)
        .expect("every catalogue operation has reference text");
    WebSocketEndpointDescriptor {
        operation: CanonicalId::new(operation).expect("static WebSocket operation is canonical"),
        path: path.into(),
        authentication: EndpointAuthentication::SessionBearer,
        connect_action,
        frame_type: frame_type.into(),
        family: reference.family,
        summary: reference.summary.into(),
        description: reference.description.into(),
    }
}
