//! The typed operation contract shared by the catalogue, clients and dispatch.

use super::{EndpointAuthentication, HttpMethod};
use schemars::JsonSchema;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

/// The request body of an operation that takes none.
pub type Empty = ();

/// The OpenAPI document, carried as JSON.
pub type OpenApiDocument = serde_json::Value;

/// What a client may do when an attempt's outcome is unknown, for example
/// after a lost response. Retries happen at one layer only: the client.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EndpointRetry {
    /// No effect: the client may repeat the request.
    Read,
    /// The engine stores the result under the idempotency key, or the
    /// operation is idempotent by its semantics, so a repeat with the same key
    /// returns the first outcome without a second effect.
    KeyedMutation,
    /// A repeat could cause a second effect or an external call. The client
    /// sends once and reads the recorded attempt instead of repeating it.
    SingleAttempt,
}

/// One catalogue operation, declared once in the catalogue rows.
pub trait Operation: Copy + Send + Sync + 'static {
    type Request: Serialize + DeserializeOwned + Send + Sync;
    type Response: Serialize + DeserializeOwned + Send + Sync;
    const ID: &'static str;
    const METHOD: HttpMethod;
    const PATH: &'static str;
    const AUTHENTICATION: EndpointAuthentication;
    const MUTATION: bool;
    const RETRY: EndpointRetry;
}
