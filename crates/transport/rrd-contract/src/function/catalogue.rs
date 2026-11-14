//! Function catalogues, execution requests, invocation receipts and results.

use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FunctionCatalogue {
    pub contract_version: u16,
    pub revision: u64,
    pub artifacts: BTreeMap<String, FunctionArtifact>,
    pub functions: BTreeMap<CanonicalId, FunctionDefinition>,
    pub transaction_bindings: BTreeMap<CanonicalId, TransactionFunctionBinding>,
}

impl FunctionCatalogue {
    pub fn empty() -> Self {
        Self {
            contract_version: FUNCTION_CONTRACT_VERSION,
            revision: 0,
            artifacts: BTreeMap::new(),
            functions: BTreeMap::new(),
            transaction_bindings: BTreeMap::new(),
        }
    }

    pub fn validate(&self) -> Result<()> {
        if self.contract_version != FUNCTION_CONTRACT_VERSION {
            return invalid("unsupported function contract version");
        }
        if self.revision == 0
            && (!self.artifacts.is_empty()
                || !self.functions.is_empty()
                || !self.transaction_bindings.is_empty())
        {
            return invalid("function catalogue revision zero must be empty");
        }
        if self.artifacts.len() > MAX_FUNCTIONS_PER_CATALOGUE
            || self.functions.len() > MAX_FUNCTIONS_PER_CATALOGUE
            || self.transaction_bindings.len() > MAX_TRANSACTION_FUNCTION_BINDINGS_PER_CATALOGUE
        {
            return invalid("function catalogue exceeds the v1 entry bound");
        }
        let mut artifact_bytes = 0_usize;
        for (content_sha256, artifact) in &self.artifacts {
            if content_sha256 != &artifact.content_sha256 {
                return invalid("function artifact map key does not match content_sha256");
            }
            artifact.validate()?;
            artifact_bytes = artifact_bytes
                .checked_add(artifact.byte_length as usize)
                .ok_or_else(|| crate::ContractError("function artifact bytes overflow".into()))?;
        }
        if artifact_bytes > MAX_FUNCTION_CATALOGUE_ARTIFACT_BYTES {
            return invalid("function catalogue artifact bytes exceed the physical v1 bound");
        }
        let mut referenced_artifacts = BTreeSet::new();
        for (function_id, function) in &self.functions {
            if function_id != &function.function_id {
                return invalid("function catalogue key does not match function_id");
            }
            function.validate()?;
            let artifact = self
                .artifacts
                .get(function.runtime.artifact_sha256())
                .ok_or_else(|| {
                    crate::ContractError(
                        "function definition references an unknown artifact".into(),
                    )
                })?;
            if artifact.media_type != function.runtime.expected_media_type() {
                return invalid("function artifact media type does not match its runtime");
            }
            referenced_artifacts.insert(function.runtime.artifact_sha256());
        }
        if referenced_artifacts.len() != self.artifacts.len() {
            return invalid("function catalogue contains an unreferenced artifact");
        }
        for (binding_id, binding) in &self.transaction_bindings {
            if binding_id != &binding.binding_id {
                return invalid("transaction function catalogue key does not match binding_id");
            }
            binding.validate(&self.functions)?;
        }
        let encoded_bytes = serde_json::to_vec(self)
            .map_err(|error| crate::ContractError(error.to_string()))?
            .len();
        if encoded_bytes > MAX_FUNCTION_CATALOGUE_ENCODED_BYTES {
            return invalid(format!(
                "function catalogue is {encoded_bytes} encoded bytes; maximum is {MAX_FUNCTION_CATALOGUE_ENCODED_BYTES}"
            ));
        }
        Ok(())
    }

    pub fn sha256(&self) -> String {
        sha256(&serde_json::to_vec(self).expect("function catalogue serializes"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReplaceFunctionCatalogue {
    pub expected_revision: u64,
    pub catalogue: FunctionCatalogue,
}

impl ReplaceFunctionCatalogue {
    pub fn validate(&self) -> Result<()> {
        self.catalogue.validate()?;
        let next_revision = self
            .expected_revision
            .checked_add(1)
            .ok_or_else(|| crate::ContractError("function catalogue revision overflow".into()))?;
        if self.catalogue.revision != next_revision {
            return invalid("replacement catalogue revision must follow expected_revision");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ListFunctionCatalogue {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExecuteFunction {
    pub function_id: CanonicalId,
    pub input: QueryValue,
}

impl ExecuteFunction {
    pub fn validate(&self) -> Result<()> {
        validate_function_value(&self.input)?;
        let bytes = serde_json::to_vec(&self.input)
            .map_err(|error| crate::ContractError(error.to_string()))?;
        if bytes.len() > MAX_FUNCTION_INPUT_BYTES {
            return invalid("function input exceeds the public v1 bound");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "proposal", rename_all = "snake_case", deny_unknown_fields)]
pub enum FunctionInvocationProposal {
    ReturnValue,
    RequireTrue,
    AppendEvent {
        kind: CanonicalId,
        properties_sha256: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FunctionInvocationReceipt {
    pub contract_version: u16,
    pub invocation_id: CanonicalId,
    pub catalogue_revision: u64,
    pub catalogue_sha256: String,
    pub function_id: CanonicalId,
    pub function_definition_sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub binding_id: Option<CanonicalId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mutation_index: Option<u32>,
    pub runtime: FunctionRuntimeKind,
    pub artifact_sha256: String,
    pub runtime_profile: CanonicalId,
    pub runtime_build_sha256: String,
    pub input_schema_sha256: String,
    pub output_schema_sha256: String,
    pub input_sha256: String,
    pub output_sha256: String,
    pub output: QueryValue,
    pub proposal: FunctionInvocationProposal,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_commit_sha256: Option<String>,
    pub limits: FunctionLimits,
    pub attempts: u8,
    pub interrupts_consumed: u32,
    pub fuel_consumed: u64,
    pub receipt_sha256: String,
}

impl FunctionInvocationReceipt {
    pub fn seal(mut self) -> Result<Self> {
        self.receipt_sha256.clear();
        self.validate_components()?;
        self.receipt_sha256 = sha256(&self.bytes_without_digest());
        Ok(self)
    }

    pub fn validate(&self) -> Result<()> {
        self.validate_components()?;
        validate_sha256(&self.receipt_sha256, "function receipt_sha256")?;
        if sha256(&self.bytes_without_digest()) != self.receipt_sha256 {
            return invalid("function receipt digest does not match its fields");
        }
        Ok(())
    }

    fn validate_components(&self) -> Result<()> {
        if self.contract_version != FUNCTION_CONTRACT_VERSION
            || self.catalogue_revision == 0
            || self.attempts != 1
        {
            return invalid("function invocation coordinates are invalid");
        }
        for (field, value) in [
            ("catalogue_sha256", self.catalogue_sha256.as_str()),
            (
                "function_definition_sha256",
                self.function_definition_sha256.as_str(),
            ),
            ("artifact_sha256", self.artifact_sha256.as_str()),
            ("runtime_build_sha256", self.runtime_build_sha256.as_str()),
            ("input_schema_sha256", self.input_schema_sha256.as_str()),
            ("output_schema_sha256", self.output_schema_sha256.as_str()),
            ("input_sha256", self.input_sha256.as_str()),
            ("output_sha256", self.output_sha256.as_str()),
        ] {
            validate_sha256(value, &format!("function {field}"))?;
        }
        if let Some(commit_sha256) = &self.runtime_commit_sha256 {
            validate_sha256(commit_sha256, "function runtime_commit_sha256")?;
        }
        self.limits.validate()?;
        if self.interrupts_consumed > self.limits.max_interrupts
            || self.fuel_consumed > self.limits.max_fuel
        {
            return invalid("function receipt consumption exceeds its admitted budget");
        }
        validate_function_value(&self.output)?;
        let output = serde_json::to_vec(&self.output)
            .map_err(|error| crate::ContractError(error.to_string()))?;
        if output.len() > self.limits.max_output_bytes as usize
            || sha256(&output) != self.output_sha256
        {
            return invalid("function receipt output bytes or digest are invalid");
        }
        let transaction_coordinates = self.binding_id.is_some()
            && self.mutation_index.is_some()
            && self.runtime_commit_sha256.is_some();
        match &self.proposal {
            FunctionInvocationProposal::ReturnValue
                if self.binding_id.is_some()
                    || self.mutation_index.is_some()
                    || self.runtime_commit_sha256.is_some() =>
            {
                return invalid("standalone function receipt has transaction coordinates");
            }
            FunctionInvocationProposal::RequireTrue
                if !transaction_coordinates || self.output != QueryValue::Bool(true) =>
            {
                return invalid("require_true receipt is not an accepted transaction proposal");
            }
            FunctionInvocationProposal::AppendEvent {
                properties_sha256, ..
            } => {
                if !transaction_coordinates || !matches!(self.output, QueryValue::Map(_)) {
                    return invalid("append_event receipt is not a complete transaction proposal");
                }
                validate_sha256(properties_sha256, "function append_event properties_sha256")?;
                if properties_sha256 != &self.output_sha256 {
                    return invalid("append_event proposal digest does not match function output");
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn bytes_without_digest(&self) -> Vec<u8> {
        let mut unsigned = self.clone();
        unsigned.receipt_sha256.clear();
        serde_json::to_vec(&unsigned).expect("function receipt fields serialize")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FunctionExecutionResult {
    pub receipt: FunctionInvocationReceipt,
}

impl FunctionExecutionResult {
    pub fn validate(&self) -> Result<()> {
        self.receipt.validate()?;
        if !matches!(
            self.receipt.proposal,
            FunctionInvocationProposal::ReturnValue
        ) {
            return invalid("standalone function execution returned a transaction proposal");
        }
        Ok(())
    }
}
