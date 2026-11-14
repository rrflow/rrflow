//! Trace contract constants and the trace and span identifiers.

use super::*;

pub const RUNTIME_TRACE_CONTRACT_VERSION: u16 = 1;
pub const RUNTIME_TRACE_EVENT_TYPE: &str = "runtime_trace";

pub(super) const MAX_TRACE_NAME_BYTES: usize = 160;
pub(super) const MAX_TRACE_LINKS: usize = 32;
pub(super) const MAX_TRACE_ATTRIBUTES: usize = 64;
pub(super) const MAX_TRACE_VALUE_DEPTH: usize = 8;
pub(super) const MAX_TRACE_VALUE_NODES: usize = 1_024;
pub(super) const MAX_TRACE_STRING_BYTES: usize = 8 * 1_024;

macro_rules! hex_ident {
    ($name:ident, $bytes:literal, $label:literal) => {
        #[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(try_from = "String", into = "String")]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self> {
                let value = value.into();
                validate_hex_identity($label, &value, $bytes)?;
                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl TryFrom<String> for $name {
            type Error = Error;

            fn try_from(value: String) -> Result<Self> {
                Self::new(value)
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> Self {
                value.0
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

// W3C Trace Context widths: 16-byte trace ID and 8-byte parent/span ID.
hex_ident!(TraceId, 32, "trace id");
hex_ident!(SpanId, 16, "span id");
