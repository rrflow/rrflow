use super::*;
use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use rrd_contract::{
    DataEventSchema, DataLogicalModel, DataRecordSchema, DataReference, DataSchemaMode,
    DataSchemaRegistry, DataTableSchema, ExecuteFunction, FunctionArtifact,
    FunctionArtifactMediaType, FunctionCapability, FunctionCatalogue, FunctionDefinition,
    FunctionInvocationReceipt, FunctionLimits, FunctionRuntime, FunctionRuntimeKind,
    FunctionValueSchema, FunctionValueShape, QueryValue, ReadDataSnapshot,
    ReplaceFunctionCatalogue, TransactionFunctionBinding, TransactionFunctionEffect,
    TransactionMutationKind, WebAssemblyAbi, FUNCTION_CONTRACT_VERSION,
    MAX_FUNCTION_CATALOGUE_ENCODED_BYTES, MAX_FUNCTION_WASM_BYTES,
};
use rrd_core::{DataTransaction, ScopeId};
use std::collections::{BTreeMap, BTreeSet};
use std::ops::{Deref, DerefMut};

fn limits() -> FunctionLimits {
    FunctionLimits {
        max_input_bytes: 32 * 1024,
        max_output_bytes: 32 * 1024,
        memory_bytes: 8 * 1024 * 1024,
        stack_bytes: 128 * 1024,
        max_interrupts: 10_000,
        max_fuel: 100_000,
    }
}

#[derive(Clone)]
struct PackagedFunction {
    artifact: FunctionArtifact,
    definition: FunctionDefinition,
}

impl Deref for PackagedFunction {
    type Target = FunctionDefinition;

    fn deref(&self) -> &Self::Target {
        &self.definition
    }
}

impl DerefMut for PackagedFunction {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.definition
    }
}

fn value_schema(id: &str, direction: &str) -> FunctionValueSchema {
    FunctionValueSchema {
        schema_id: CanonicalId::new(format!("{id}-{direction}")).unwrap(),
        revision: 1,
        shape: FunctionValueShape::CanonicalValueV1,
    }
}

fn javascript(id: &str, source: &str) -> PackagedFunction {
    let bytes = source.as_bytes();
    let artifact_sha256 = digest::sha256_hex(bytes);
    let artifact = FunctionArtifact {
        content_sha256: artifact_sha256.clone(),
        media_type: FunctionArtifactMediaType::JavaScriptUtf8,
        byte_length: bytes.len().try_into().unwrap(),
        content_base64: STANDARD.encode(bytes),
    };
    let input_schema = value_schema(id, "input");
    let output_schema = value_schema(id, "output");
    let definition = FunctionDefinition {
        function_id: CanonicalId::new(id).unwrap(),
        revision: 1,
        predecessor_sha256: None,
        runtime: FunctionRuntime::JavaScriptEs2020 {
            artifact_sha256,
            runtime_profile: super::super::function::runtime_profile::runtime_profile(
                FunctionRuntimeKind::JavaScriptEs2020,
            ),
            runtime_build_sha256: super::super::function::runtime_profile::runtime_build_sha256(
                FunctionRuntimeKind::JavaScriptEs2020,
            ),
        },
        input_schema_sha256: input_schema.sha256(),
        input_schema,
        output_schema_sha256: output_schema.sha256(),
        output_schema,
        limits: limits(),
        capabilities: BTreeSet::new(),
    };
    PackagedFunction {
        artifact,
        definition,
    }
}

fn webassembly(id: &str, wat: &str) -> PackagedFunction {
    let module = wat::parse_str(wat).unwrap();
    let artifact_sha256 = digest::sha256_hex(&module);
    let artifact = FunctionArtifact {
        content_sha256: artifact_sha256.clone(),
        media_type: FunctionArtifactMediaType::WebAssemblyBinary,
        byte_length: module.len().try_into().unwrap(),
        content_base64: STANDARD.encode(&module),
    };
    let input_schema = value_schema(id, "input");
    let output_schema = value_schema(id, "output");
    let definition = FunctionDefinition {
        function_id: CanonicalId::new(id).unwrap(),
        revision: 1,
        predecessor_sha256: None,
        runtime: FunctionRuntime::WebAssemblyV1 {
            artifact_sha256,
            runtime_profile: super::super::function::runtime_profile::runtime_profile(
                FunctionRuntimeKind::WebAssemblyV1,
            ),
            runtime_build_sha256: super::super::function::runtime_profile::runtime_build_sha256(
                FunctionRuntimeKind::WebAssemblyV1,
            ),
            abi: WebAssemblyAbi::JsonV1,
        },
        input_schema_sha256: input_schema.sha256(),
        input_schema,
        output_schema_sha256: output_schema.sha256(),
        output_schema,
        limits: limits(),
        capabilities: BTreeSet::new(),
    };
    PackagedFunction {
        artifact,
        definition,
    }
}

fn maximum_webassembly(id: &str, marker: u32) -> PackagedFunction {
    let mut module = vec![0_u8; MAX_FUNCTION_WASM_BYTES];
    module[..8].copy_from_slice(b"\0asm\x01\0\0\0");
    module[8..12].copy_from_slice(&marker.to_be_bytes());
    let artifact_sha256 = digest::sha256_hex(&module);
    let artifact = FunctionArtifact {
        content_sha256: artifact_sha256.clone(),
        media_type: FunctionArtifactMediaType::WebAssemblyBinary,
        byte_length: module.len().try_into().unwrap(),
        content_base64: STANDARD.encode(module),
    };
    let input_schema = value_schema(id, "input");
    let output_schema = value_schema(id, "output");
    let definition = FunctionDefinition {
        function_id: CanonicalId::new(id).unwrap(),
        revision: 1,
        predecessor_sha256: None,
        runtime: FunctionRuntime::WebAssemblyV1 {
            artifact_sha256,
            runtime_profile: super::super::function::runtime_profile::runtime_profile(
                FunctionRuntimeKind::WebAssemblyV1,
            ),
            runtime_build_sha256: super::super::function::runtime_profile::runtime_build_sha256(
                FunctionRuntimeKind::WebAssemblyV1,
            ),
            abi: WebAssemblyAbi::JsonV1,
        },
        input_schema_sha256: input_schema.sha256(),
        input_schema,
        output_schema_sha256: output_schema.sha256(),
        output_schema,
        limits: limits(),
        capabilities: BTreeSet::new(),
    };
    PackagedFunction {
        artifact,
        definition,
    }
}

fn packaged_catalogue_functions(catalogue: &FunctionCatalogue) -> Vec<PackagedFunction> {
    catalogue
        .functions
        .values()
        .map(|definition| PackagedFunction {
            artifact: catalogue
                .artifacts
                .get(definition.runtime.artifact_sha256())
                .unwrap()
                .clone(),
            definition: definition.clone(),
        })
        .collect()
}

fn binding(
    binding_id: &str,
    function: &FunctionDefinition,
    mutation: TransactionMutationKind,
    kind: Option<CanonicalId>,
    effect: TransactionFunctionEffect,
) -> TransactionFunctionBinding {
    TransactionFunctionBinding {
        binding_id: CanonicalId::new(binding_id).unwrap(),
        revision: 1,
        predecessor_sha256: None,
        function_id: function.function_id.clone(),
        function_definition_sha256: function.sha256(),
        mutation,
        kind,
        effect,
        max_attempts: 1,
    }
}

fn catalogue(
    revision: u64,
    functions: Vec<PackagedFunction>,
    transaction_bindings: Vec<TransactionFunctionBinding>,
) -> FunctionCatalogue {
    let artifacts = functions
        .iter()
        .map(|function| {
            (
                function.artifact.content_sha256.clone(),
                function.artifact.clone(),
            )
        })
        .collect();
    FunctionCatalogue {
        contract_version: FUNCTION_CONTRACT_VERSION,
        revision,
        artifacts,
        functions: functions
            .into_iter()
            .map(|function| (function.definition.function_id.clone(), function.definition))
            .collect(),
        transaction_bindings: transaction_bindings
            .into_iter()
            .map(|binding| (binding.binding_id.clone(), binding))
            .collect(),
    }
}

fn install(
    engine: &RrdEngine,
    lease: &rrd_contract::SessionLease,
    expected_revision: u64,
    catalogue: FunctionCatalogue,
    suffix: &str,
) {
    engine
        .replace_function_catalogue(
            &lease.session_id,
            &lease.token,
            &ReplaceFunctionCatalogue {
                expected_revision,
                catalogue,
            },
            1_100 + expected_revision,
            &format!("request-catalogue-{suffix}"),
            &format!("operation-catalogue-{suffix}"),
        )
        .unwrap();
}

#[allow(clippy::too_many_arguments)]
fn persist_function_commit_intent(
    engine: &RrdEngine,
    lease: &rrd_contract::SessionLease,
    transaction_id: &CorrelationId,
    idempotency_key: &str,
    request: &CommitTransaction,
    runtime_at: u64,
    runtime_commit_sha256: &str,
    function_catalogue_revision: u64,
    function_receipts: &[FunctionInvocationReceipt],
    suffix: &str,
) {
    let session_key = format!(
        "server/state/test-instance/session/{}",
        lease.session_id.as_str()
    );
    let before = engine.storage.control().get(&session_key).unwrap().unwrap();
    let mut session_state: Value = serde_json::from_slice(&before).unwrap();
    session_state["transactions"][transaction_id.as_str()]["commit_intent"] = serde_json::json!({
        "idempotency_key": idempotency_key,
        "operation_sha256": request.operation_sha256.clone(),
        "runtime_at_unix_ms": runtime_at,
        "runtime_commit_sha256": runtime_commit_sha256,
        "function_catalogue_revision": function_catalogue_revision,
        "function_receipts": function_receipts,
    });
    engine
        .storage
        .control()
        .commit(&ControlTransition {
            key: session_key,
            expected: Some(before),
            replacement: Some(serde_json::to_vec(&session_state).unwrap()),
            at: runtime_at,
            actor: "rrd-engine-test".into(),
            action: "transaction.commit_prepared".into(),
            request_id: format!("request-{suffix}"),
            operation_id: format!("operation-{suffix}"),
        })
        .unwrap();
}

mod catalogue_security;
mod execution;
mod transactions;
