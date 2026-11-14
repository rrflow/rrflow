//! Source-parse bindings and policy.

use super::*;

/// An explicit suffix-to-parser choice; it is not source discovery or authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SourceParseBinding {
    /// Case-sensitive final extension, without a leading dot.
    #[schemars(schema_with = "source_parse_extension_schema")]
    pub extension: String,
    pub language: CanonicalId,
}

/// Versioned, project-selected limits for one authorized Parse step.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SourceParsePolicy {
    #[schemars(schema_with = "source_parse_version_schema")]
    pub format_version: u16,
    #[schemars(schema_with = "source_parse_bindings_schema")]
    pub bindings: Vec<SourceParseBinding>,
    #[schemars(schema_with = "source_parse_limit_schema")]
    pub max_files: u64,
    #[schemars(schema_with = "source_parse_file_limit_schema")]
    pub max_file_bytes: u64,
    #[schemars(schema_with = "source_parse_limit_schema")]
    pub max_input_bytes: u64,
    #[schemars(schema_with = "source_parse_limit_schema")]
    pub max_nodes: u64,
    #[schemars(schema_with = "source_parse_limit_schema")]
    pub max_depth: u64,
    #[schemars(schema_with = "source_parse_limit_schema")]
    pub max_output_bytes: u64,
    /// Maximum encoded bytes in one atomic publication group.
    #[schemars(schema_with = "source_parse_limit_schema")]
    pub max_commit_bytes: u64,
    #[schemars(schema_with = "source_parse_limit_schema")]
    pub max_elapsed_ms: u64,
    #[schemars(schema_with = "source_parse_limit_schema")]
    pub max_memory_bytes: u64,
}

pub(super) const SOURCE_PARSE_POLICY_VERSION: u16 = 1;
pub(super) const MAX_SOURCE_PARSE_BINDINGS: usize = 64;
pub(super) const MAX_SOURCE_PARSE_EXTENSION_BYTES: usize = 32;
// JSON clients using ECMAScript Number must preserve every integer exactly.
pub(super) const MAX_SOURCE_PARSE_WIRE_INTEGER: u64 = (1_u64 << 53) - 1;

pub(super) fn source_parse_version_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
    schemars::json_schema!({"type": "integer", "const": SOURCE_PARSE_POLICY_VERSION})
}

pub(super) fn source_parse_extension_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
    schemars::json_schema!({
        "type": "string", "minLength": 1, "maxLength": MAX_SOURCE_PARSE_EXTENSION_BYTES,
        "pattern": "^[A-Za-z0-9_+\\-]+$"
    })
}

pub(super) fn source_parse_limit_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
    schemars::json_schema!({"type": "integer", "minimum": 1, "maximum": MAX_SOURCE_PARSE_WIRE_INTEGER})
}

pub(super) fn source_parse_file_limit_schema(
    _: &mut schemars::SchemaGenerator,
) -> schemars::Schema {
    schemars::json_schema!({"type": "integer", "minimum": 1, "maximum": u32::MAX})
}

pub(super) fn source_parse_bindings_schema(
    generator: &mut schemars::SchemaGenerator,
) -> schemars::Schema {
    let mut schema = <Vec<SourceParseBinding>>::json_schema(generator);
    schema.insert("minItems".into(), serde_json::json!(1));
    schema.insert(
        "maxItems".into(),
        serde_json::json!(MAX_SOURCE_PARSE_BINDINGS),
    );
    schema
}

impl SourceParsePolicy {
    pub fn validate(&self) -> Result<()> {
        if self.format_version != SOURCE_PARSE_POLICY_VERSION
            || self.bindings.is_empty()
            || self.bindings.len() > MAX_SOURCE_PARSE_BINDINGS
        {
            return invalid("invalid source parse policy version or binding count");
        }
        for binding in &self.bindings {
            if binding.extension.is_empty()
                || binding.extension.len() > MAX_SOURCE_PARSE_EXTENSION_BYTES
                || !binding
                    .extension
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'+'))
            {
                return invalid("invalid source parse extension");
            }
        }
        if self
            .bindings
            .windows(2)
            .any(|pair| pair[0].extension >= pair[1].extension)
        {
            return invalid("source parse extensions must be unique and ordered");
        }
        for value in [
            self.max_files,
            self.max_file_bytes,
            self.max_input_bytes,
            self.max_nodes,
            self.max_depth,
            self.max_output_bytes,
            self.max_commit_bytes,
            self.max_elapsed_ms,
            self.max_memory_bytes,
        ] {
            if value == 0 || value > MAX_SOURCE_PARSE_WIRE_INTEGER {
                return invalid("source parse limit is outside the positive protocol range");
            }
        }
        if self.max_file_bytes > u64::from(u32::MAX)
            || self.max_file_bytes > self.max_input_bytes
            || self.max_commit_bytes > self.max_output_bytes
        {
            return invalid("source parse file or commit limit exceeds its aggregate limit");
        }
        Ok(())
    }
}
