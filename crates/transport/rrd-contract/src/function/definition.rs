//! Function limits, artifacts, runtimes, value schemas, capabilities and definitions.

use super::*;

pub const FUNCTION_CONTRACT_VERSION: u16 = 1;
pub const MAX_FUNCTIONS_PER_CATALOGUE: usize = 64;
pub const MAX_TRANSACTION_FUNCTION_BINDINGS_PER_CATALOGUE: usize = 128;
pub const MAX_FUNCTION_SOURCE_BYTES: usize = 64 * 1024;
pub const MAX_FUNCTION_WASM_BYTES: usize = 256 * 1024;
/// Maximum decoded executable content in one catalogue replacement.
///
/// This aggregate bound is intentionally lower than the sum of every
/// per-artifact maximum: one replacement must remain one atomic rrflowKV
/// batch after typed definitions, bindings, membership, keys, and framing are
/// added.
pub const MAX_FUNCTION_CATALOGUE_ARTIFACT_BYTES: usize = 3 * 1024 * 1024;
/// Maximum canonical JSON bytes accepted for one function catalogue.
///
/// The public representation includes base64 artifacts. The physical store
/// performs a second exact current-format admission check; this conservative
/// public bound leaves room for immutable-record envelopes and escaped
/// canonical definition/binding JSON without exposing the WAL format here.
pub const MAX_FUNCTION_CATALOGUE_ENCODED_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_FUNCTION_INPUT_BYTES: usize = 256 * 1024;
pub const MAX_FUNCTION_OUTPUT_BYTES: usize = 256 * 1024;
pub const MAX_FUNCTION_VALUE_DEPTH: usize = 64;
pub const MAX_FUNCTION_VALUE_ITEMS: usize = 8_192;
pub const MAX_FUNCTION_SCHEMA_DEPTH: usize = 32;
pub const MAX_FUNCTION_SCHEMA_ITEMS: usize = 1_024;
pub const MAX_FUNCTION_JSON_SAFE_INTEGER: u64 = 9_007_199_254_740_991;
pub(super) const MIN_FUNCTION_MEMORY_BYTES: u64 = 64 * 1024;
pub(super) const MAX_FUNCTION_MEMORY_BYTES: u64 = 64 * 1024 * 1024;
pub(super) const MIN_FUNCTION_STACK_BYTES: u32 = 16 * 1024;
pub(super) const MAX_FUNCTION_STACK_BYTES: u32 = 1024 * 1024;
pub(super) const MAX_FUNCTION_INTERRUPTS: u32 = 1_000_000;
pub(super) const MAX_FUNCTION_FUEL: u64 = 1_000_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FunctionRuntimeKind {
    JavaScriptEs2020,
    WebAssemblyV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FunctionArtifactMediaType {
    JavaScriptUtf8,
    WebAssemblyBinary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FunctionArtifact {
    pub content_sha256: String,
    pub media_type: FunctionArtifactMediaType,
    pub byte_length: u32,
    pub content_base64: String,
}

impl FunctionArtifact {
    pub fn decoded_content(&self) -> Result<Vec<u8>> {
        validate_sha256(&self.content_sha256, "function artifact content_sha256")?;
        let content = STANDARD
            .decode(&self.content_base64)
            .map_err(|_| crate::ContractError("function artifact content is not base64".into()))?;
        let expected_length: usize = self
            .byte_length
            .try_into()
            .map_err(|_| crate::ContractError("function artifact length exceeds usize".into()))?;
        if content.len() != expected_length || content.is_empty() {
            return invalid("function artifact byte_length does not match non-empty content");
        }
        let maximum = match self.media_type {
            FunctionArtifactMediaType::JavaScriptUtf8 => MAX_FUNCTION_SOURCE_BYTES,
            FunctionArtifactMediaType::WebAssemblyBinary => MAX_FUNCTION_WASM_BYTES,
        };
        if content.len() > maximum {
            return invalid(format!(
                "function artifact exceeds its {maximum}-byte media bound"
            ));
        }
        if sha256(&content) != self.content_sha256 {
            return invalid("function artifact digest does not match decoded content");
        }
        match self.media_type {
            FunctionArtifactMediaType::JavaScriptUtf8 => {
                let source = std::str::from_utf8(&content).map_err(|_| {
                    crate::ContractError("JavaScript function artifact is not UTF-8".into())
                })?;
                if source.trim().is_empty() {
                    return invalid("JavaScript function artifact is empty");
                }
            }
            FunctionArtifactMediaType::WebAssemblyBinary => {
                if !content.starts_with(b"\0asm") {
                    return invalid("WebAssembly function artifact lacks the module magic");
                }
            }
        }
        Ok(content)
    }

    pub fn validate(&self) -> Result<()> {
        self.decoded_content().map(|_| ())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WebAssemblyAbi {
    /// The module exports `memory`, `rrd_alloc(i32) -> i32`, and
    /// `rrd_run(i32, i32) -> i64`. The host writes canonical JSON input at
    /// the allocated pointer. The result packs an output pointer in the high
    /// 32 bits and a UTF-8 JSON byte length in the low 32 bits.
    JsonV1,
}

/// An executable reference. Artifact bytes live once in the catalogue's
/// content-addressed artifact set and never inside a definition or membership.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "runtime", rename_all = "snake_case", deny_unknown_fields)]
pub enum FunctionRuntime {
    JavaScriptEs2020 {
        artifact_sha256: String,
        runtime_profile: CanonicalId,
        runtime_build_sha256: String,
    },
    WebAssemblyV1 {
        artifact_sha256: String,
        runtime_profile: CanonicalId,
        runtime_build_sha256: String,
        abi: WebAssemblyAbi,
    },
}

impl FunctionRuntime {
    pub fn kind(&self) -> FunctionRuntimeKind {
        match self {
            Self::JavaScriptEs2020 { .. } => FunctionRuntimeKind::JavaScriptEs2020,
            Self::WebAssemblyV1 { .. } => FunctionRuntimeKind::WebAssemblyV1,
        }
    }

    pub fn artifact_sha256(&self) -> &str {
        match self {
            Self::JavaScriptEs2020 {
                artifact_sha256, ..
            }
            | Self::WebAssemblyV1 {
                artifact_sha256, ..
            } => artifact_sha256,
        }
    }

    pub fn runtime_profile(&self) -> &CanonicalId {
        match self {
            Self::JavaScriptEs2020 {
                runtime_profile, ..
            }
            | Self::WebAssemblyV1 {
                runtime_profile, ..
            } => runtime_profile,
        }
    }

    pub fn runtime_build_sha256(&self) -> &str {
        match self {
            Self::JavaScriptEs2020 {
                runtime_build_sha256,
                ..
            }
            | Self::WebAssemblyV1 {
                runtime_build_sha256,
                ..
            } => runtime_build_sha256,
        }
    }

    pub fn expected_media_type(&self) -> FunctionArtifactMediaType {
        match self {
            Self::JavaScriptEs2020 { .. } => FunctionArtifactMediaType::JavaScriptUtf8,
            Self::WebAssemblyV1 { .. } => FunctionArtifactMediaType::WebAssemblyBinary,
        }
    }

    pub fn validate(&self) -> Result<()> {
        validate_sha256(self.artifact_sha256(), "function artifact_sha256")?;
        validate_sha256(self.runtime_build_sha256(), "function runtime_build_sha256")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "shape", rename_all = "snake_case", deny_unknown_fields)]
pub enum FunctionValueShape {
    CanonicalValueV1,
    Null,
    Boolean,
    Integer,
    Unsigned,
    Decimal,
    String {
        max_bytes: u32,
    },
    Digest,
    List {
        items: Box<FunctionValueShape>,
        max_items: u32,
    },
    Object {
        #[serde(default)]
        properties: BTreeMap<String, FunctionValueShape>,
        #[serde(default)]
        required: BTreeSet<String>,
        allow_additional_properties: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FunctionValueSchema {
    pub schema_id: CanonicalId,
    pub revision: u64,
    pub shape: FunctionValueShape,
}

impl FunctionValueSchema {
    pub fn validate(&self) -> Result<()> {
        if self.revision == 0 {
            return invalid("function value schema revision must be non-zero");
        }
        let mut items = 0_usize;
        validate_schema_shape(&self.shape, 0, &mut items)
    }

    pub fn validate_value(&self, value: &QueryValue) -> Result<()> {
        self.validate()?;
        validate_function_value(value)?;
        validate_value_shape(&self.shape, value, self.schema_id.as_str())
    }

    pub fn sha256(&self) -> String {
        sha256(&serde_json::to_vec(self).expect("function schema serializes"))
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum FunctionCapability {
    /// Allows a transaction-function result object to be lowered into one
    /// typed append-only event in the same authoritative data transaction.
    EmitEvent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FunctionLimits {
    pub max_input_bytes: u32,
    pub max_output_bytes: u32,
    pub memory_bytes: u64,
    pub stack_bytes: u32,
    /// QuickJS interrupt-callback budget. The runtime-build identity pins
    /// callback scheduling; exhaustion is an uncatchable terminal failure.
    pub max_interrupts: u32,
    /// Wasm instruction fuel. Wasmi traps deterministically at exhaustion.
    pub max_fuel: u64,
}

impl FunctionLimits {
    pub fn validate(&self) -> Result<()> {
        if self.max_input_bytes == 0
            || self.max_input_bytes as usize > MAX_FUNCTION_INPUT_BYTES
            || self.max_output_bytes == 0
            || self.max_output_bytes as usize > MAX_FUNCTION_OUTPUT_BYTES
        {
            return invalid("function input/output byte limits are outside the v1 bounds");
        }
        if !(MIN_FUNCTION_MEMORY_BYTES..=MAX_FUNCTION_MEMORY_BYTES).contains(&self.memory_bytes)
            || !(MIN_FUNCTION_STACK_BYTES..=MAX_FUNCTION_STACK_BYTES).contains(&self.stack_bytes)
            || u64::from(self.stack_bytes) >= self.memory_bytes
        {
            return invalid("function memory/stack limits are outside the v1 bounds");
        }
        if self.max_interrupts == 0
            || self.max_interrupts > MAX_FUNCTION_INTERRUPTS
            || self.max_fuel == 0
            || self.max_fuel > MAX_FUNCTION_FUEL
        {
            return invalid("function interrupt/fuel limits are outside the v1 bounds");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FunctionDefinition {
    pub function_id: CanonicalId,
    pub revision: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub predecessor_sha256: Option<String>,
    pub runtime: FunctionRuntime,
    pub input_schema: FunctionValueSchema,
    pub input_schema_sha256: String,
    pub output_schema: FunctionValueSchema,
    pub output_schema_sha256: String,
    pub limits: FunctionLimits,
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub capabilities: BTreeSet<FunctionCapability>,
}

impl FunctionDefinition {
    pub fn validate(&self) -> Result<()> {
        validate_revision_lineage(
            self.revision,
            self.predecessor_sha256.as_deref(),
            "function definition",
        )?;
        self.runtime.validate()?;
        self.input_schema.validate()?;
        self.output_schema.validate()?;
        validate_sha256(&self.input_schema_sha256, "function input_schema_sha256")?;
        validate_sha256(&self.output_schema_sha256, "function output_schema_sha256")?;
        if self.input_schema.sha256() != self.input_schema_sha256
            || self.output_schema.sha256() != self.output_schema_sha256
        {
            return invalid("function schema digest does not match its typed schema");
        }
        self.limits.validate()
    }

    pub fn sha256(&self) -> String {
        sha256(&serde_json::to_vec(self).expect("function definition serializes"))
    }
}
