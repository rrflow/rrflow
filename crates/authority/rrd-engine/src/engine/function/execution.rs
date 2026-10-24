use super::super::*;
use rrd_store::{FunctionInvocationReceiptRecord, FUNCTION_INVOCATION_RECEIPT_FORMAT_VERSION};

#[cfg(test)]
thread_local! {
    static TEST_EXECUTION_COUNT: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
pub(in crate::engine) fn reset_test_execution_count() {
    TEST_EXECUTION_COUNT.set(0);
}

#[cfg(test)]
pub(in crate::engine) fn test_execution_count() -> u64 {
    TEST_EXECUTION_COUNT.get()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ExecutedFunction {
    pub output: QueryValue,
    pub interrupts_consumed: u32,
    pub fuel_consumed: u64,
}

impl RrdEngine {
    #[allow(clippy::too_many_arguments)]
    pub fn execute_function(
        &self,
        session_id: &CorrelationId,
        token: &CorrelationId,
        request: &ExecuteFunction,
        now: u64,
        request_id: &str,
        operation_id: &str,
    ) -> Result<FunctionExecutionResult> {
        request
            .validate()
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        self.authorize(
            session_id,
            token,
            SecurityAction::FunctionExecute,
            now,
            request_id,
            operation_id,
        )?;
        let _transaction_guard = self.admit_transaction(request, request_id, operation_id)?;
        let catalogue = self.load_current_function_catalogue()?;
        let definition = catalogue
            .functions
            .get(&request.function_id)
            .ok_or(ServiceError::FunctionNotFound)?;
        let artifact = catalogue
            .artifacts
            .get(definition.runtime.artifact_sha256())
            .ok_or_else(|| ServiceError::Storage("function artifact is missing".into()))?;
        let input_bytes = serde_json::to_vec(&request.input).map_err(contract_json)?;
        let input_sha256 = digest::sha256_hex(&input_bytes);
        let invocation_id = invocation_id(&[
            request_id,
            operation_id,
            catalogue.sha256().as_str(),
            definition.sha256().as_str(),
            &input_sha256,
        ])?;
        if let Some(stored) = self
            .storage
            .function_catalogue()
            .invocation_receipt(self.instance.as_str(), invocation_id.as_str())?
        {
            let receipt = decode_receipt_record(&stored)?;
            if receipt.invocation_id != invocation_id
                || receipt.catalogue_revision != catalogue.revision
                || receipt.catalogue_sha256 != catalogue.sha256()
                || receipt.function_id != request.function_id
                || receipt.function_definition_sha256 != definition.sha256()
                || receipt.input_sha256 != input_sha256
                || !matches!(receipt.proposal, FunctionInvocationProposal::ReturnValue)
            {
                return Err(ServiceError::IdempotencyConflict);
            }
            return Ok(FunctionExecutionResult { receipt });
        }
        super::runtime_profile::validate_runtime_identity(&definition.runtime)?;
        let content = artifact
            .decoded_content()
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        let executed = execute_definition(definition, &content, &request.input)?;
        let output_bytes = serde_json::to_vec(&executed.output).map_err(contract_json)?;
        let receipt = FunctionInvocationReceipt {
            contract_version: rrd_contract::FUNCTION_CONTRACT_VERSION,
            invocation_id,
            catalogue_revision: catalogue.revision,
            catalogue_sha256: catalogue.sha256(),
            function_id: request.function_id.clone(),
            function_definition_sha256: definition.sha256(),
            binding_id: None,
            mutation_index: None,
            runtime: definition.runtime.kind(),
            artifact_sha256: definition.runtime.artifact_sha256().into(),
            runtime_profile: definition.runtime.runtime_profile().clone(),
            runtime_build_sha256: definition.runtime.runtime_build_sha256().into(),
            input_schema_sha256: definition.input_schema_sha256.clone(),
            output_schema_sha256: definition.output_schema_sha256.clone(),
            input_sha256,
            output_sha256: digest::sha256_hex(&output_bytes),
            output: executed.output,
            proposal: FunctionInvocationProposal::ReturnValue,
            runtime_commit_sha256: None,
            limits: definition.limits.clone(),
            attempts: 1,
            interrupts_consumed: executed.interrupts_consumed,
            fuel_consumed: executed.fuel_consumed,
            receipt_sha256: String::new(),
        }
        .seal()
        .map_err(|error| ServiceError::Contract(error.to_string()))?;
        let record = encode_receipt_record(&receipt)?;
        self.storage
            .function_catalogue()
            .commit_standalone_receipt(self.instance.as_str(), &record)
            .map_err(ServiceError::from)?;
        let result = FunctionExecutionResult { receipt };
        result
            .validate()
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        Ok(result)
    }
}

pub(super) fn execute_definition(
    definition: &FunctionDefinition,
    artifact: &[u8],
    input: &QueryValue,
) -> Result<ExecutedFunction> {
    #[cfg(test)]
    TEST_EXECUTION_COUNT.set(TEST_EXECUTION_COUNT.get().saturating_add(1));
    definition
        .validate()
        .map_err(|error| ServiceError::Contract(error.to_string()))?;
    definition
        .input_schema
        .validate_value(input)
        .map_err(|error| ServiceError::Contract(error.to_string()))?;
    if digest::sha256_hex(artifact) != definition.runtime.artifact_sha256() {
        return Err(ServiceError::Storage(
            "function artifact bytes do not match the definition".into(),
        ));
    }
    let natural_input = query_value_to_json(input)?;
    let input_bytes = serde_json::to_vec(&natural_input).map_err(contract_json)?;
    if input_bytes.len() > definition.limits.max_input_bytes as usize {
        return Err(ServiceError::FunctionLimit(
            "function input exceeds its configured byte limit".into(),
        ));
    }
    let (output_bytes, interrupts_consumed, fuel_consumed) = match &definition.runtime {
        FunctionRuntime::JavaScriptEs2020 { .. } => {
            let source = std::str::from_utf8(artifact).map_err(|_| {
                ServiceError::Contract("JavaScript function artifact is not UTF-8".into())
            })?;
            super::javascript::execute(source, &input_bytes, definition)?
        }
        FunctionRuntime::WebAssemblyV1 { .. } => {
            super::webassembly::execute(artifact, &input_bytes, definition)?
        }
    };
    if output_bytes.len() > definition.limits.max_output_bytes as usize {
        return Err(ServiceError::FunctionLimit(
            "function output exceeds its configured byte limit".into(),
        ));
    }
    let output: serde_json::Value = serde_json::from_slice(&output_bytes)
        .map_err(|error| ServiceError::Function(format!("invalid JSON output: {error}")))?;
    let output = json_to_query_value(output)?;
    definition
        .output_schema
        .validate_value(&output)
        .map_err(|error| ServiceError::Contract(error.to_string()))?;
    Ok(ExecutedFunction {
        output,
        interrupts_consumed,
        fuel_consumed,
    })
}

pub(super) fn invocation_id(parts: &[&str]) -> Result<CanonicalId> {
    let identity = digest::sha256_hex(&serde_json::to_vec(parts).map_err(contract_json)?);
    CanonicalId::new(format!("function-{identity}"))
        .map_err(|error| ServiceError::Contract(error.to_string()))
}

pub(in crate::engine) fn encode_receipt_record(
    receipt: &FunctionInvocationReceipt,
) -> Result<FunctionInvocationReceiptRecord> {
    receipt
        .validate()
        .map_err(|error| ServiceError::Contract(error.to_string()))?;
    let canonical_receipt_json = serde_json::to_string(receipt).map_err(contract_json)?;
    Ok(FunctionInvocationReceiptRecord {
        format_version: FUNCTION_INVOCATION_RECEIPT_FORMAT_VERSION,
        invocation_id: receipt.invocation_id.as_str().into(),
        runtime_commit_sha256: receipt.runtime_commit_sha256.clone(),
        receipt_sha256: receipt.receipt_sha256.clone(),
        canonical_receipt_sha256: digest::sha256_hex(canonical_receipt_json.as_bytes()),
        canonical_receipt_json,
    })
}

pub(super) fn decode_receipt_record(
    record: &FunctionInvocationReceiptRecord,
) -> Result<FunctionInvocationReceipt> {
    record.validate().map_err(ServiceError::from)?;
    let receipt: FunctionInvocationReceipt =
        serde_json::from_str(&record.canonical_receipt_json).map_err(contract_json)?;
    receipt
        .validate()
        .map_err(|error| ServiceError::Contract(error.to_string()))?;
    if receipt.invocation_id.as_str() != record.invocation_id
        || receipt.runtime_commit_sha256 != record.runtime_commit_sha256
        || receipt.receipt_sha256 != record.receipt_sha256
    {
        return Err(ServiceError::Storage(
            "function receipt record metadata differs from its canonical receipt".into(),
        ));
    }
    Ok(receipt)
}

fn query_value_to_json(value: &QueryValue) -> Result<serde_json::Value> {
    Ok(match value {
        QueryValue::Null => serde_json::Value::Null,
        QueryValue::Bool(value) => serde_json::Value::Bool(*value),
        QueryValue::Integer(value) => {
            let maximum = rrd_contract::MAX_FUNCTION_JSON_SAFE_INTEGER as i64;
            if !(-maximum..=maximum).contains(value) {
                return Err(ServiceError::Contract(
                    "function integer input exceeds the JSON-v1 exact range".into(),
                ));
            }
            serde_json::Value::Number((*value).into())
        }
        QueryValue::Unsigned(value) => {
            if *value > rrd_contract::MAX_FUNCTION_JSON_SAFE_INTEGER {
                return Err(ServiceError::Contract(
                    "function unsigned input exceeds the JSON-v1 exact range".into(),
                ));
            }
            serde_json::Value::Number((*value).into())
        }
        QueryValue::Decimal(value) => {
            let number: serde_json::Number = value.parse().map_err(|_| {
                ServiceError::Contract("function decimal input is not a JSON number".into())
            })?;
            if number.to_string() != *value {
                return Err(ServiceError::Contract(
                    "function decimal input is not canonical in the JSON-v1 numeric domain".into(),
                ));
            }
            serde_json::Value::Number(number)
        }
        QueryValue::String(value) | QueryValue::Digest(value) => {
            serde_json::Value::String(value.clone())
        }
        QueryValue::List(values) => serde_json::Value::Array(
            values
                .iter()
                .map(query_value_to_json)
                .collect::<Result<_>>()?,
        ),
        QueryValue::Map(values) => serde_json::Value::Object(
            values
                .iter()
                .map(|(key, value)| Ok((key.clone(), query_value_to_json(value)?)))
                .collect::<Result<_>>()?,
        ),
    })
}

pub(super) fn json_to_query_value(value: serde_json::Value) -> Result<QueryValue> {
    Ok(match value {
        serde_json::Value::Null => QueryValue::Null,
        serde_json::Value::Bool(value) => QueryValue::Bool(value),
        serde_json::Value::Number(value) => {
            if let Some(value) = value.as_i64() {
                let maximum = rrd_contract::MAX_FUNCTION_JSON_SAFE_INTEGER as i64;
                if !(-maximum..=maximum).contains(&value) {
                    return Err(ServiceError::Contract(
                        "function integer output exceeds the JSON-v1 exact range".into(),
                    ));
                }
                QueryValue::Integer(value)
            } else if let Some(value) = value.as_u64() {
                if value > rrd_contract::MAX_FUNCTION_JSON_SAFE_INTEGER {
                    return Err(ServiceError::Contract(
                        "function unsigned output exceeds the JSON-v1 exact range".into(),
                    ));
                }
                QueryValue::Unsigned(value)
            } else {
                QueryValue::Decimal(value.to_string())
            }
        }
        serde_json::Value::String(value) => QueryValue::String(value),
        serde_json::Value::Array(values) => QueryValue::List(
            values
                .into_iter()
                .map(json_to_query_value)
                .collect::<Result<_>>()?,
        ),
        serde_json::Value::Object(values) => QueryValue::Map(
            values
                .into_iter()
                .map(|(key, value)| Ok((key, json_to_query_value(value)?)))
                .collect::<Result<_>>()?,
        ),
    })
}
