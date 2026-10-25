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

#[test]
fn javascript_and_webassembly_are_bounded_deterministic_and_restart_safe() {
    let (root, engine) = isolated_engine();
    let lease = engine
        .create_session(
            &session_request(5_000, 4),
            &id("function-session"),
            1_000,
            "request-function-session",
            "operation-function-session",
        )
        .unwrap();
    let javascript_definition = javascript("add-one", "(input) => ({ answer: input.value + 1 })");
    let wasm = webassembly(
        "wasm-answer",
        r#"(module
            (memory (export "memory") 1 1)
            (data (i32.const 0) "{\22answer\22:42}")
            (func (export "rrd_alloc") (param i32) (result i32) i32.const 1024)
            (func (export "rrd_run") (param i32 i32) (result i64) i64.const 13)
        )"#,
    );
    install(
        &engine,
        &lease,
        0,
        catalogue(1, vec![javascript_definition, wasm], vec![]),
        "one",
    );
    let installed = engine
        .function_catalogue(
            &lease.session_id,
            &lease.token,
            1_150,
            "request-catalogue-before-conflict",
            "operation-catalogue-before-conflict",
        )
        .unwrap();
    let conflict = engine.replace_function_catalogue(
        &lease.session_id,
        &lease.token,
        &ReplaceFunctionCatalogue {
            expected_revision: 0,
            catalogue: catalogue(
                1,
                vec![javascript(
                    "substituted-function",
                    "() => ({ substituted: true })",
                )],
                vec![],
            ),
        },
        1_151,
        "request-catalogue-conflict",
        "operation-catalogue-conflict",
    );
    assert!(matches!(conflict, Err(ServiceError::StorageConflict(_))));
    assert_eq!(
        engine
            .function_catalogue(
                &lease.session_id,
                &lease.token,
                1_152,
                "request-catalogue-after-conflict",
                "operation-catalogue-after-conflict",
            )
            .unwrap(),
        installed,
        "a stale replacement must not alter the immutable revision or head",
    );

    let input = QueryValue::Map(BTreeMap::from([("value".into(), QueryValue::Integer(41))]));
    let javascript_result = engine
        .execute_function(
            &lease.session_id,
            &lease.token,
            &ExecuteFunction {
                function_id: CanonicalId::new("add-one").unwrap(),
                input: input.clone(),
            },
            1_200,
            "request-js",
            "operation-js",
        )
        .unwrap();
    assert_eq!(
        javascript_result.receipt.output,
        QueryValue::Map(BTreeMap::from([("answer".into(), QueryValue::Integer(42))]))
    );
    assert_eq!(javascript_result.receipt.fuel_consumed, 0);
    let javascript_replay = engine
        .execute_function(
            &lease.session_id,
            &lease.token,
            &ExecuteFunction {
                function_id: CanonicalId::new("add-one").unwrap(),
                input: input.clone(),
            },
            1_200,
            "request-js",
            "operation-js",
        )
        .unwrap();
    assert_eq!(javascript_replay, javascript_result);
    let unsafe_integer_input = engine.execute_function(
        &lease.session_id,
        &lease.token,
        &ExecuteFunction {
            function_id: CanonicalId::new("add-one").unwrap(),
            input: QueryValue::Unsigned(rrd_contract::MAX_FUNCTION_JSON_SAFE_INTEGER + 1),
        },
        1_200,
        "request-unsafe-integer-input",
        "operation-unsafe-integer-input",
    );
    assert!(matches!(
        unsafe_integer_input,
        Err(ServiceError::Contract(_))
    ));

    let wasm_result = engine
        .execute_function(
            &lease.session_id,
            &lease.token,
            &ExecuteFunction {
                function_id: CanonicalId::new("wasm-answer").unwrap(),
                input,
            },
            1_201,
            "request-wasm",
            "operation-wasm",
        )
        .unwrap();
    assert_eq!(wasm_result.receipt.output, javascript_result.receipt.output);
    assert!(wasm_result.receipt.fuel_consumed > 0);

    drop(engine);
    let reopened = RrdEngine::open(root.path(), instance(), TOKEN_KEY).unwrap();
    let restored = reopened
        .function_catalogue(
            &lease.session_id,
            &lease.token,
            1_300,
            "request-list-functions",
            "operation-list-functions",
        )
        .unwrap();
    assert_eq!(restored.revision, 1);
    assert_eq!(restored.functions.len(), 2);
    let persisted_execution = reopened
        .storage
        .function_catalogue()
        .invocation_receipt(
            instance().as_str(),
            javascript_result.receipt.invocation_id.as_str(),
        )
        .unwrap()
        .unwrap();
    assert_eq!(
        persisted_execution.receipt_sha256,
        javascript_result.receipt.receipt_sha256
    );

    let mut endless = javascript("endless", "() => { while (true) {} }");
    endless.limits.max_interrupts = 1;
    let heap_hog = javascript("heap-hog", "() => new ArrayBuffer(16777216)");
    let stack_hog = javascript(
        "stack-hog",
        "() => { function recurse() { return recurse(); } return recurse(); }",
    );
    let random = javascript("random", "() => Math.random()");
    let unsafe_integer_output = javascript("unsafe-integer-output", "() => 9007199254740992");
    let imported = webassembly(
        "imported",
        r#"(module
            (import "env" "clock" (func))
            (memory (export "memory") 1 1)
            (func (export "rrd_alloc") (param i32) (result i32) i32.const 1024)
            (func (export "rrd_run") (param i32 i32) (result i64) i64.const 4)
        )"#,
    );
    let wasm_loop = webassembly(
        "wasm-loop",
        r#"(module
            (memory (export "memory") 1 1)
            (func (export "rrd_alloc") (param i32) (result i32) i32.const 1024)
            (func (export "rrd_run") (param i32 i32) (result i64)
                (loop $forever br $forever)
                i64.const 0)
        )"#,
    );
    let wasm_stack = webassembly(
        "wasm-stack",
        r#"(module
            (memory (export "memory") 1 1)
            (func (export "rrd_alloc") (param i32) (result i32) i32.const 1024)
            (func $recurse (result i64) call $recurse)
            (func (export "rrd_run") (param i32 i32) (result i64) call $recurse)
        )"#,
    );
    install(
        &reopened,
        &lease,
        1,
        catalogue(
            2,
            packaged_catalogue_functions(&restored)
                .into_iter()
                .chain([
                    endless,
                    heap_hog,
                    stack_hog,
                    random,
                    unsafe_integer_output,
                    imported,
                    wasm_loop,
                    wasm_stack,
                ])
                .collect(),
            vec![],
        ),
        "two",
    );
    let exhausted = reopened.execute_function(
        &lease.session_id,
        &lease.token,
        &ExecuteFunction {
            function_id: CanonicalId::new("endless").unwrap(),
            input: QueryValue::Null,
        },
        1_400,
        "request-endless",
        "operation-endless",
    );
    assert!(matches!(exhausted, Err(ServiceError::FunctionLimit(_))));
    for function_id in ["heap-hog", "stack-hog"] {
        let exhausted = reopened.execute_function(
            &lease.session_id,
            &lease.token,
            &ExecuteFunction {
                function_id: CanonicalId::new(function_id).unwrap(),
                input: QueryValue::Null,
            },
            1_400,
            &format!("request-{function_id}"),
            &format!("operation-{function_id}"),
        );
        assert!(
            matches!(exhausted, Err(ServiceError::FunctionLimit(_))),
            "{function_id} must exhaust its configured resource: {exhausted:?}"
        );
    }
    let denied_random = reopened.execute_function(
        &lease.session_id,
        &lease.token,
        &ExecuteFunction {
            function_id: CanonicalId::new("random").unwrap(),
            input: QueryValue::Null,
        },
        1_400,
        "request-random",
        "operation-random",
    );
    assert!(matches!(denied_random, Err(ServiceError::Function(_))));
    let denied_unsafe_integer = reopened.execute_function(
        &lease.session_id,
        &lease.token,
        &ExecuteFunction {
            function_id: CanonicalId::new("unsafe-integer-output").unwrap(),
            input: QueryValue::Null,
        },
        1_400,
        "request-unsafe-integer-output",
        "operation-unsafe-integer-output",
    );
    assert!(matches!(
        denied_unsafe_integer,
        Err(ServiceError::Contract(_))
    ));
    let denied_import = reopened.execute_function(
        &lease.session_id,
        &lease.token,
        &ExecuteFunction {
            function_id: CanonicalId::new("imported").unwrap(),
            input: QueryValue::Null,
        },
        1_401,
        "request-imported",
        "operation-imported",
    );
    assert!(matches!(denied_import, Err(ServiceError::Function(_))));
    let exhausted_fuel = reopened.execute_function(
        &lease.session_id,
        &lease.token,
        &ExecuteFunction {
            function_id: CanonicalId::new("wasm-loop").unwrap(),
            input: QueryValue::Null,
        },
        1_402,
        "request-wasm-loop",
        "operation-wasm-loop",
    );
    assert!(matches!(
        exhausted_fuel,
        Err(ServiceError::FunctionLimit(_))
    ));
    let exhausted_stack = reopened.execute_function(
        &lease.session_id,
        &lease.token,
        &ExecuteFunction {
            function_id: CanonicalId::new("wasm-stack").unwrap(),
            input: QueryValue::Null,
        },
        1_403,
        "request-wasm-stack",
        "operation-wasm-stack",
    );
    assert!(matches!(
        exhausted_stack,
        Err(ServiceError::FunctionLimit(_))
    ));
}

#[test]
fn largest_admitted_public_catalogue_fits_one_physical_batch_and_reopens() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("maximum-function-catalogue");
    let engine = RrdEngine::open(&path, instance(), TOKEN_KEY).unwrap();
    let lease = engine
        .create_session(
            &session_request(10_000, 2),
            &id("maximum-function-catalogue-session"),
            1_000,
            "request-maximum-function-catalogue-session",
            "operation-maximum-function-catalogue-session",
        )
        .unwrap();

    let mut packaged = Vec::new();
    let (accepted, rejected) = loop {
        let index = packaged.len();
        packaged.push(maximum_webassembly(
            &format!("maximum-wasm-{index:02}"),
            index as u32,
        ));
        let candidate = catalogue(1, packaged.clone(), Vec::new());
        match candidate.validate() {
            Ok(()) => continue,
            Err(error) => {
                packaged.pop();
                break (catalogue(1, packaged, Vec::new()), (candidate, error));
            }
        }
    };
    let encoded_bytes = serde_json::to_vec(&accepted).unwrap().len();
    assert!(encoded_bytes <= MAX_FUNCTION_CATALOGUE_ENCODED_BYTES);
    assert!(encoded_bytes > MAX_FUNCTION_CATALOGUE_ENCODED_BYTES - 2 * MAX_FUNCTION_WASM_BYTES);
    assert!(rejected.1.to_string().contains("encoded bytes"));

    let installed = engine
        .replace_function_catalogue(
            &lease.session_id,
            &lease.token,
            &ReplaceFunctionCatalogue {
                expected_revision: 0,
                catalogue: accepted.clone(),
            },
            1_100,
            "request-maximum-function-catalogue-install",
            "operation-maximum-function-catalogue-install",
        )
        .unwrap();
    assert_eq!(installed, accepted);
    let before_rejection = engine.storage.physical_store_evidence().unwrap();
    assert!(matches!(
        engine.replace_function_catalogue(
            &lease.session_id,
            &lease.token,
            &ReplaceFunctionCatalogue {
                expected_revision: 1,
                catalogue: rejected.0,
            },
            1_200,
            "request-over-limit-function-catalogue",
            "operation-over-limit-function-catalogue",
        ),
        Err(ServiceError::Contract(message)) if message.contains("encoded bytes")
    ));
    assert_eq!(
        engine
            .storage
            .physical_store_evidence()
            .unwrap()
            .physical_sequence,
        before_rejection.physical_sequence,
        "over-limit admission must happen before any physical mutation"
    );
    drop(engine);

    let reopened = RrdEngine::open(&path, instance(), TOKEN_KEY).unwrap();
    assert_eq!(
        reopened.load_current_function_catalogue().unwrap(),
        accepted
    );
}

#[test]
fn transaction_binding_effects_commit_atomically_and_failures_leave_data_unchanged() {
    let (_root, engine) = isolated_engine();
    let lease = engine
        .create_session(
            &session_request(10_000, 4),
            &id("function-binding-session"),
            1_000,
            "request-function-binding-session",
            "operation-function-binding-session",
        )
        .unwrap();
    let mut event_function = javascript(
        "emit-person-event",
        "(input) => ({ mutation_index: input.mutation_index })",
    );
    event_function
        .capabilities
        .insert(FunctionCapability::EmitEvent);
    let event_binding = binding(
        "person-event",
        &event_function,
        TransactionMutationKind::PutRecord,
        Some(CanonicalId::new("person").unwrap()),
        TransactionFunctionEffect::AppendEvent {
            kind: CanonicalId::new("person-changed").unwrap(),
        },
    );
    install(
        &engine,
        &lease,
        0,
        catalogue(1, vec![event_function], vec![event_binding]),
        "binding-one",
    );

    let transaction = engine
        .begin_transaction(
            &lease.session_id,
            &lease.token,
            &BeginTransaction {
                scope: CanonicalId::new("data").unwrap(),
                timeout_ms: 5_000,
            },
            &mutation_context(
                &id("begin-binding-one"),
                "request-begin-binding-one",
                "operation-begin-binding-one",
            ),
            1_100,
        )
        .unwrap();
    let person = CanonicalId::new("person").unwrap();
    let changed = CanonicalId::new("person-changed").unwrap();
    let registry = DataSchemaRegistry {
        revision: 1,
        migration: "install transaction function binding fixture".into(),
        catalogue: DataCatalogueIdentity::default(),
        tables: BTreeMap::from([
            (
                person.clone(),
                DataTableSchema {
                    model: DataLogicalModel::Document,
                    mode: DataSchemaMode::Strict,
                    properties: BTreeMap::new(),
                    allow_additional_properties: false,
                },
            ),
            (
                changed.clone(),
                DataTableSchema {
                    model: DataLogicalModel::Event,
                    mode: DataSchemaMode::Strict,
                    properties: BTreeMap::new(),
                    allow_additional_properties: false,
                },
            ),
        ]),
        records: BTreeMap::from([(
            person.clone(),
            DataRecordSchema {
                properties: BTreeMap::new(),
                allow_additional_properties: true,
                unique_properties: BTreeSet::new(),
            },
        )]),
        relations: BTreeMap::new(),
        events: BTreeMap::from([(
            changed.clone(),
            DataEventSchema {
                subject_required: false,
                subject_types: BTreeSet::new(),
                properties: BTreeMap::new(),
                allow_additional_properties: true,
            },
        )]),
    };
    let mutations = vec![
        TransactionMutation::PutSchema { registry },
        TransactionMutation::PutRecord {
            reference: DataReference {
                kind: person.clone(),
                id: CanonicalId::new("alice").unwrap(),
            },
            valid_from: 1_200,
            valid_to: None,
            properties: BTreeMap::new(),
        },
    ];
    let commit = CommitTransaction {
        operation_sha256: rrd_contract::transaction_operation_sha256(&mutations),
        mutations,
    };
    engine
        .commit_transaction(
            &lease.session_id,
            &lease.token,
            &transaction.transaction_id,
            &id("commit-binding-one"),
            &commit,
            1_200,
            "request-commit-binding-one",
            "operation-commit-binding-one",
        )
        .unwrap();
    let snapshot = engine
        .read_data_snapshot(
            &lease.session_id,
            &lease.token,
            &ReadDataSnapshot {
                valid_at: 1_200,
                max_storage_keys: 100,
            },
            1_201,
            "request-binding-snapshot",
            "operation-binding-snapshot",
        )
        .unwrap();
    assert!(snapshot.entries.iter().any(|entry| {
        matches!(
            &entry.value,
            TransactionMutation::AppendEvent { kind, .. } if kind == &changed
        )
    }));
    let binding_audit = SecurityRepository::new(&engine.storage, instance())
        .audit_since(0, 64)
        .unwrap();
    assert!(!binding_audit.records.iter().any(|(_, record)| {
        record.action == SecurityAction::FunctionExecute
            && record.phase == rrd_contract::AuditPhase::Completed
            && record.decision == AuditDecision::Allowed
    }), "successful transaction functions are evidenced by atomic receipts, not a pre-commit allowed audit");
    let (_, state) = engine
        .load_authenticated(&lease.session_id, &lease.token)
        .unwrap();
    let record = engine
        .transaction_record(&state, &transaction.transaction_id)
        .unwrap()
        .unwrap();
    let function_receipt: FunctionInvocationReceipt =
        record.commit_intent.unwrap().function_receipts[0].clone();
    let stored_receipt = engine
        .storage
        .function_catalogue()
        .invocation_receipt(instance().as_str(), function_receipt.invocation_id.as_str())
        .unwrap()
        .unwrap();
    assert_eq!(
        stored_receipt.receipt_sha256,
        function_receipt.receipt_sha256
    );
    assert_eq!(
        stored_receipt.runtime_commit_sha256,
        function_receipt.runtime_commit_sha256
    );
    assert!(engine
        .storage
        .runtime()
        .audit(function_receipt.runtime_commit_sha256.as_deref().unwrap())
        .unwrap()
        .is_some());
    engine
        .require_committed_transaction_function_receipts(std::slice::from_ref(&function_receipt))
        .unwrap();
    let mut absent_receipt = function_receipt.clone();
    absent_receipt.invocation_id = CanonicalId::new("function-absent-receipt").unwrap();
    absent_receipt.receipt_sha256.clear();
    let absent_receipt = absent_receipt.seal().unwrap();
    assert!(matches!(
        engine
            .require_committed_transaction_function_receipts(std::slice::from_ref(&absent_receipt)),
        Err(ServiceError::OperationDigestMismatch)
    ));

    let reject = javascript("reject-person", "() => false");
    let reject_binding = binding(
        "reject-person",
        &reject,
        TransactionMutationKind::PutRecord,
        Some(person.clone()),
        TransactionFunctionEffect::RequireTrue,
    );
    install(
        &engine,
        &lease,
        1,
        catalogue(2, vec![reject], vec![reject_binding]),
        "binding-two",
    );
    let transaction = engine
        .begin_transaction(
            &lease.session_id,
            &lease.token,
            &BeginTransaction {
                scope: CanonicalId::new("data").unwrap(),
                timeout_ms: 5_000,
            },
            &mutation_context(
                &id("begin-binding-two"),
                "request-begin-binding-two",
                "operation-begin-binding-two",
            ),
            1_300,
        )
        .unwrap();
    let before = engine.storage.runtime().cursor().unwrap();
    let mutations = vec![TransactionMutation::PutRecord {
        reference: DataReference {
            kind: person,
            id: CanonicalId::new("bob").unwrap(),
        },
        valid_from: 1_300,
        valid_to: None,
        properties: BTreeMap::new(),
    }];
    let rejected = engine.commit_transaction(
        &lease.session_id,
        &lease.token,
        &transaction.transaction_id,
        &id("commit-binding-two"),
        &CommitTransaction {
            operation_sha256: rrd_contract::transaction_operation_sha256(&mutations),
            mutations,
        },
        1_301,
        "request-commit-binding-two",
        "operation-commit-binding-two",
    );
    assert!(matches!(rejected, Err(ServiceError::Function(_))));
    assert_eq!(engine.storage.runtime().cursor().unwrap(), before);
}

#[test]
fn transaction_retry_reuses_prepared_receipts_pinned_before_the_crash_window() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("function-retry");
    let engine = RrdEngine::open(&path, instance(), TOKEN_KEY).unwrap();
    let lease = engine
        .create_session(
            &session_request(5_000, 2),
            &id("function-retry-session"),
            1_000,
            "request-function-retry-session",
            "operation-function-retry-session",
        )
        .unwrap();
    let binding_id = CanonicalId::new("claim-validator").unwrap();
    let accepted = javascript("claim-policy", "() => true");
    let accepted_binding = binding(
        binding_id.as_str(),
        &accepted,
        TransactionMutationKind::AssertClaim,
        None,
        TransactionFunctionEffect::RequireTrue,
    );
    let accepted_definition_sha256 = accepted.sha256();
    let accepted_binding_sha256 = accepted_binding.sha256();
    install(
        &engine,
        &lease,
        0,
        catalogue(1, vec![accepted], vec![accepted_binding]),
        "retry-one",
    );
    let transaction = engine
        .begin_transaction(
            &lease.session_id,
            &lease.token,
            &begin_request(),
            &mutation_context(
                &id("function-retry-begin"),
                "request-function-retry-begin",
                "operation-function-retry-begin",
            ),
            1_100,
        )
        .unwrap();
    let request = commit_request("revision-one-must-win");
    let runtime_at = 1_150;
    let base = public_runtime_commit(
        &request,
        &lease.session_id,
        &instance(),
        transaction.read_cursor,
        runtime_at,
    )
    .unwrap();
    let revision_one = engine.load_current_function_catalogue().unwrap();
    let prepared_functions = engine
        .prepare_transaction_function_bindings(
            &revision_one,
            &request,
            &transaction.transaction_id,
            base,
            None,
            runtime_at,
            "request-function-retry-prepare",
            "operation-function-retry-prepare",
        )
        .unwrap();
    let prepared_receipt = prepared_functions.receipts[0].clone();
    persist_function_commit_intent(
        &engine,
        &lease,
        &transaction.transaction_id,
        "function-retry-commit",
        &request,
        runtime_at,
        &prepared_functions.commit.digest(),
        1,
        &prepared_functions.receipts,
        "function-retry-prepare",
    );

    let mut rejected = javascript("claim-policy", "() => false");
    rejected.revision = 2;
    rejected.predecessor_sha256 = Some(accepted_definition_sha256);
    let mut rejected_binding = binding(
        binding_id.as_str(),
        &rejected,
        TransactionMutationKind::AssertClaim,
        None,
        TransactionFunctionEffect::RequireTrue,
    );
    rejected_binding.revision = 2;
    rejected_binding.predecessor_sha256 = Some(accepted_binding_sha256);
    install(
        &engine,
        &lease,
        1,
        catalogue(2, vec![rejected], vec![rejected_binding]),
        "retry-two",
    );
    drop(engine);

    let reopened = RrdEngine::open(&path, instance(), TOKEN_KEY).unwrap();
    let receipt = reopened
        .commit_transaction(
            &lease.session_id,
            &lease.token,
            &transaction.transaction_id,
            &id("function-retry-commit"),
            &request,
            1_300,
            "request-function-retry-commit",
            "operation-function-retry-commit",
        )
        .unwrap();
    assert_eq!(receipt.last_runtime_cursor, Some(1));
    let stored_receipt = reopened
        .storage
        .function_catalogue()
        .invocation_receipt(instance().as_str(), prepared_receipt.invocation_id.as_str())
        .unwrap()
        .unwrap();
    assert_eq!(
        stored_receipt.receipt_sha256,
        prepared_receipt.receipt_sha256
    );
    assert_eq!(
        stored_receipt.runtime_commit_sha256,
        prepared_receipt.runtime_commit_sha256
    );
    assert_eq!(
        reopened.load_current_function_catalogue().unwrap().revision,
        2,
        "recovery must retain the newer head while executing pinned revision one",
    );
}

#[test]
fn substituted_runtime_build_in_a_prepared_receipt_fails_closed_after_reopen() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("function-runtime-substitution");
    let engine = RrdEngine::open(&path, instance(), TOKEN_KEY).unwrap();
    let lease = engine
        .create_session(
            &session_request(5_000, 2),
            &id("function-runtime-substitution-session"),
            1_000,
            "request-function-runtime-substitution-session",
            "operation-function-runtime-substitution-session",
        )
        .unwrap();
    let accepted = javascript("runtime-bound-policy", "() => true");
    let accepted_binding = binding(
        "runtime-bound-validator",
        &accepted,
        TransactionMutationKind::AssertClaim,
        None,
        TransactionFunctionEffect::RequireTrue,
    );
    install(
        &engine,
        &lease,
        0,
        catalogue(1, vec![accepted], vec![accepted_binding]),
        "runtime-substitution",
    );
    let transaction = engine
        .begin_transaction(
            &lease.session_id,
            &lease.token,
            &begin_request(),
            &mutation_context(
                &id("function-runtime-substitution-begin"),
                "request-function-runtime-substitution-begin",
                "operation-function-runtime-substitution-begin",
            ),
            1_100,
        )
        .unwrap();
    let request = commit_request("substituted-runtime-build-must-not-commit");
    let runtime_at = 1_150;
    let base = public_runtime_commit(
        &request,
        &lease.session_id,
        &instance(),
        transaction.read_cursor,
        runtime_at,
    )
    .unwrap();
    let prepared = engine
        .prepare_transaction_function_bindings(
            &engine.load_current_function_catalogue().unwrap(),
            &request,
            &transaction.transaction_id,
            base,
            None,
            runtime_at,
            "request-function-runtime-substitution-prepare",
            "operation-function-runtime-substitution-prepare",
        )
        .unwrap();
    let expected_commit_sha256 = prepared.commit.digest();
    let invocation_id = prepared.receipts[0].invocation_id.clone();
    let mut substituted_receipt = prepared.receipts[0].clone();
    substituted_receipt.runtime_build_sha256 =
        digest::sha256_hex(b"unavailable-substituted-runtime-build");
    let substituted_receipt = substituted_receipt.seal().unwrap();
    persist_function_commit_intent(
        &engine,
        &lease,
        &transaction.transaction_id,
        "function-runtime-substitution-commit",
        &request,
        runtime_at,
        &expected_commit_sha256,
        1,
        &[substituted_receipt],
        "function-runtime-substitution-prepare",
    );
    drop(engine);

    let reopened = RrdEngine::open(&path, instance(), TOKEN_KEY).unwrap();
    let before_cursor = reopened.storage.runtime().cursor().unwrap();
    let rejected = reopened.commit_transaction(
        &lease.session_id,
        &lease.token,
        &transaction.transaction_id,
        &id("function-runtime-substitution-commit"),
        &request,
        1_300,
        "request-function-runtime-substitution-commit",
        "operation-function-runtime-substitution-commit",
    );
    assert!(matches!(
        rejected,
        Err(ServiceError::OperationDigestMismatch)
    ));
    assert_eq!(reopened.storage.runtime().cursor().unwrap(), before_cursor);
    assert_eq!(reopened.storage.claims().sequence().unwrap(), 0);
    assert!(reopened
        .storage
        .runtime()
        .commit_outcome(&expected_commit_sha256)
        .unwrap()
        .is_none());
    assert!(reopened
        .storage
        .function_catalogue()
        .invocation_receipt(instance().as_str(), invocation_id.as_str())
        .unwrap()
        .is_none());
}

#[test]
fn durable_function_receipt_closes_a_lost_ack_without_reexecution() {
    super::super::function::reset_test_execution_count();
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("function-lost-ack");
    let engine = RrdEngine::open(&path, instance(), TOKEN_KEY).unwrap();
    let lease = engine
        .create_session(
            &session_request(5_000, 2),
            &id("function-lost-ack-session"),
            1_000,
            "request-function-lost-ack-session",
            "operation-function-lost-ack-session",
        )
        .unwrap();
    let binding_id = CanonicalId::new("lost-ack-validator").unwrap();
    let accepted = javascript("lost-ack-policy", "() => true");
    let accepted_definition_sha256 = accepted.sha256();
    let accepted_binding = binding(
        binding_id.as_str(),
        &accepted,
        TransactionMutationKind::AssertClaim,
        None,
        TransactionFunctionEffect::RequireTrue,
    );
    let accepted_binding_sha256 = accepted_binding.sha256();
    install(
        &engine,
        &lease,
        0,
        catalogue(1, vec![accepted], vec![accepted_binding]),
        "lost-ack-one",
    );
    let transaction = engine
        .begin_transaction(
            &lease.session_id,
            &lease.token,
            &begin_request(),
            &mutation_context(
                &id("function-lost-ack-begin"),
                "request-function-lost-ack-begin",
                "operation-function-lost-ack-begin",
            ),
            1_100,
        )
        .unwrap();
    let request = commit_request("durable-receipt-must-win");
    let runtime_at = 1_150;
    let base = public_runtime_commit(
        &request,
        &lease.session_id,
        &instance(),
        transaction.read_cursor,
        runtime_at,
    )
    .unwrap();
    let prepared = engine
        .prepare_transaction_function_bindings(
            &engine.load_current_function_catalogue().unwrap(),
            &request,
            &transaction.transaction_id,
            base,
            None,
            runtime_at,
            "request-function-lost-ack-prepare",
            "operation-function-lost-ack-prepare",
        )
        .unwrap();
    assert_eq!(super::super::function::test_execution_count(), 1);
    let expected_commit_sha256 = prepared.commit.digest();
    let prepared_receipt = prepared.receipts[0].clone();
    persist_function_commit_intent(
        &engine,
        &lease,
        &transaction.transaction_id,
        "function-lost-ack-commit",
        &request,
        runtime_at,
        &expected_commit_sha256,
        1,
        &prepared.receipts,
        "function-lost-ack-prepare",
    );

    let scope = ScopeId::new(format!("instance:{}", instance())).unwrap();
    let read = engine.storage.runtime().read_stamp(&scope).unwrap();
    assert_eq!(read.commit_cursor, transaction.read_cursor);
    let data_transaction = DataTransaction::new(read, prepared.commit).unwrap();
    let receipt_records = prepared
        .receipts
        .iter()
        .map(super::super::function::encode_receipt_record)
        .collect::<std::result::Result<Vec<_>, _>>()
        .unwrap();
    let committed = engine
        .storage
        .runtime()
        .commit_data_transaction_with_function_receipts(
            &data_transaction,
            instance().as_str(),
            &receipt_records,
        )
        .unwrap();
    assert_eq!(committed.last_cursor, 1);
    assert_eq!(super::super::function::test_execution_count(), 1);

    let mut rejected = javascript("lost-ack-policy", "() => false");
    rejected.revision = 2;
    rejected.predecessor_sha256 = Some(accepted_definition_sha256);
    let mut rejected_binding = binding(
        binding_id.as_str(),
        &rejected,
        TransactionMutationKind::AssertClaim,
        None,
        TransactionFunctionEffect::RequireTrue,
    );
    rejected_binding.revision = 2;
    rejected_binding.predecessor_sha256 = Some(accepted_binding_sha256);
    install(
        &engine,
        &lease,
        1,
        catalogue(2, vec![rejected], vec![rejected_binding]),
        "lost-ack-two",
    );
    drop(engine);

    let reopened = RrdEngine::open(&path, instance(), TOKEN_KEY).unwrap();
    assert_eq!(
        reopened.load_current_function_catalogue().unwrap().revision,
        2
    );
    let before_retry_cursor = reopened.storage.runtime().cursor().unwrap();
    let recovered = reopened
        .commit_transaction(
            &lease.session_id,
            &lease.token,
            &transaction.transaction_id,
            &id("function-lost-ack-commit"),
            &request,
            1_300,
            "request-function-lost-ack-commit",
            "operation-function-lost-ack-commit",
        )
        .unwrap();
    assert!(recovered.idempotent_replay);
    assert_eq!(recovered.last_runtime_cursor, Some(1));
    assert_eq!(
        reopened.storage.runtime().cursor().unwrap(),
        before_retry_cursor
    );
    assert_eq!(reopened.storage.claims().sequence().unwrap(), 1);
    assert_eq!(
        super::super::function::test_execution_count(),
        1,
        "recovery must use the durable build-bound receipt instead of executing a function again",
    );
    let stored_receipt = reopened
        .storage
        .function_catalogue()
        .invocation_receipt(instance().as_str(), prepared_receipt.invocation_id.as_str())
        .unwrap()
        .unwrap();
    assert_eq!(
        stored_receipt.receipt_sha256,
        prepared_receipt.receipt_sha256
    );
    assert_eq!(
        stored_receipt.runtime_commit_sha256,
        Some(expected_commit_sha256)
    );
}

#[test]
fn catalogue_identity_lineage_cannot_reset_after_an_inactive_membership() {
    let (root, engine) = isolated_engine();
    let lease = engine
        .create_session(
            &session_request(5_000, 2),
            &id("function-lineage-session"),
            1_000,
            "request-function-lineage-session",
            "operation-function-lineage-session",
        )
        .unwrap();
    let original = javascript("lineage-policy", "() => true");
    let original_definition_sha256 = original.sha256();
    let original_binding = binding(
        "lineage-binding",
        &original,
        TransactionMutationKind::AssertClaim,
        None,
        TransactionFunctionEffect::RequireTrue,
    );
    let original_binding_sha256 = original_binding.sha256();
    install(
        &engine,
        &lease,
        0,
        catalogue(1, vec![original.clone()], vec![original_binding.clone()]),
        "lineage-one",
    );
    install(
        &engine,
        &lease,
        1,
        catalogue(2, vec![], vec![]),
        "lineage-inactive",
    );

    let reset_definition = javascript("lineage-policy", "() => false");
    let reset_definition_result = engine.replace_function_catalogue(
        &lease.session_id,
        &lease.token,
        &ReplaceFunctionCatalogue {
            expected_revision: 2,
            catalogue: catalogue(3, vec![reset_definition], vec![]),
        },
        1_200,
        "request-lineage-definition-reset",
        "operation-lineage-definition-reset",
    );
    assert!(matches!(
        reset_definition_result,
        Err(ServiceError::Contract(_))
    ));

    let reset_binding = binding(
        "lineage-binding",
        &original,
        TransactionMutationKind::PutSchema,
        None,
        TransactionFunctionEffect::RequireTrue,
    );
    let reset_binding_result = engine.replace_function_catalogue(
        &lease.session_id,
        &lease.token,
        &ReplaceFunctionCatalogue {
            expected_revision: 2,
            catalogue: catalogue(3, vec![original.clone()], vec![reset_binding]),
        },
        1_201,
        "request-lineage-binding-reset",
        "operation-lineage-binding-reset",
    );
    assert!(matches!(
        reset_binding_result,
        Err(ServiceError::Contract(_))
    ));
    assert_eq!(
        engine.load_current_function_catalogue().unwrap().revision,
        2
    );

    let mut successor = javascript("lineage-policy", "() => false");
    successor.revision = 2;
    successor.predecessor_sha256 = Some(original_definition_sha256);
    let mut successor_binding = binding(
        "lineage-binding",
        &successor,
        TransactionMutationKind::AssertClaim,
        None,
        TransactionFunctionEffect::RequireTrue,
    );
    successor_binding.revision = 2;
    successor_binding.predecessor_sha256 = Some(original_binding_sha256);
    install(
        &engine,
        &lease,
        2,
        catalogue(3, vec![successor], vec![successor_binding]),
        "lineage-three",
    );
    drop(engine);

    let reopened = RrdEngine::open(root.path(), instance(), TOKEN_KEY).unwrap();
    let restored = reopened.load_current_function_catalogue().unwrap();
    assert_eq!(restored.revision, 3);
    assert_eq!(
        restored.functions[&CanonicalId::new("lineage-policy").unwrap()].revision,
        2
    );
    assert_eq!(
        restored.transaction_bindings[&CanonicalId::new("lineage-binding").unwrap()].revision,
        2
    );
}

#[test]
fn exact_security_actions_gate_catalogue_execution_and_transaction_bindings() {
    let (_root, engine) = isolated_engine();
    let admin_id = CanonicalId::new("function-admin").unwrap();
    let limited_id = CanonicalId::new("function-reader").unwrap();
    let resource = ResourcePath {
        segments: vec![ResourceId::new(ResourceKind::Instance, instance().as_str()).unwrap()],
    };
    let principal =
        |principal_id: CanonicalId, credential: &[u8], actions: &[SecurityAction]| Principal {
            id: principal_id,
            kind: PrincipalKind::Service,
            credential_sha256: digest::sha256_hex(credential),
            credential_revision: 1,
            not_before_unix_ms: 1,
            expires_at_unix_ms: u64::MAX,
            disabled: false,
            role_ids: BTreeSet::new(),
            grants: actions
                .iter()
                .copied()
                .map(|action| ResourceGrant {
                    action,
                    resource_prefix: resource.clone(),
                    data_policy: None,
                })
                .collect(),
        };
    SecurityRepository::new(&engine.storage, instance())
        .initialize(
            SecurityState {
                format_version: rrd_security::SECURITY_FORMAT,
                revision: 1,
                principals: BTreeMap::from([
                    (
                        admin_id.clone(),
                        principal(
                            admin_id.clone(),
                            b"function-admin-key",
                            &[
                                SecurityAction::SessionCreate,
                                SecurityAction::FunctionCatalogueRead,
                                SecurityAction::FunctionCatalogueWrite,
                                SecurityAction::FunctionExecute,
                            ],
                        ),
                    ),
                    (
                        limited_id.clone(),
                        principal(
                            limited_id.clone(),
                            b"function-reader-key",
                            &[
                                SecurityAction::SessionCreate,
                                SecurityAction::FunctionCatalogueRead,
                                SecurityAction::TransactionBegin,
                                SecurityAction::TransactionCommit,
                            ],
                        ),
                    ),
                ]),
                roles: BTreeMap::new(),
                identity_bindings: BTreeMap::new(),
                jwt_issuers: BTreeMap::new(),
            },
            1,
            "function-security-test",
            "request-function-security",
            "operation-function-security",
        )
        .unwrap();
    let admin = engine
        .create_authenticated_session(
            &admin_id,
            b"function-admin-key",
            &session_request(5_000, 2),
            &id("function-admin-session"),
            1_000,
            "request-function-admin-session",
            "operation-function-admin-session",
        )
        .unwrap();
    let limited = engine
        .create_authenticated_session(
            &limited_id,
            b"function-reader-key",
            &session_request(5_000, 2),
            &id("function-reader-session"),
            1_000,
            "request-function-reader-session",
            "operation-function-reader-session",
        )
        .unwrap();
    let definition = javascript("secured-policy", "() => true");
    let binding = binding(
        "secured-binding",
        &definition,
        TransactionMutationKind::AssertClaim,
        None,
        TransactionFunctionEffect::RequireTrue,
    );
    install(
        &engine,
        &admin,
        0,
        catalogue(1, vec![definition], vec![binding]),
        "secured",
    );

    assert_eq!(
        engine
            .function_catalogue(
                &limited.session_id,
                &limited.token,
                1_100,
                "request-function-list-allowed",
                "operation-function-list-allowed",
            )
            .unwrap()
            .revision,
        1,
    );
    let denied_execute = engine.execute_function(
        &limited.session_id,
        &limited.token,
        &ExecuteFunction {
            function_id: CanonicalId::new("secured-policy").unwrap(),
            input: QueryValue::Null,
        },
        1_101,
        "request-function-execute-denied",
        "operation-function-execute-denied",
    );
    assert!(matches!(
        denied_execute,
        Err(ServiceError::PermissionDenied)
    ));
    let denied_write = engine.replace_function_catalogue(
        &limited.session_id,
        &limited.token,
        &ReplaceFunctionCatalogue {
            expected_revision: 1,
            catalogue: catalogue(
                2,
                vec![javascript("replacement-policy", "() => true")],
                vec![],
            ),
        },
        1_102,
        "request-function-write-denied",
        "operation-function-write-denied",
    );
    assert!(matches!(denied_write, Err(ServiceError::PermissionDenied)));

    let transaction = engine
        .begin_transaction(
            &limited.session_id,
            &limited.token,
            &begin_request(),
            &mutation_context(
                &id("function-binding-begin"),
                "request-function-binding-begin",
                "operation-function-binding-begin",
            ),
            1_110,
        )
        .unwrap();
    let before = engine.storage.runtime().cursor().unwrap();
    let request = commit_request("function-binding-must-be-denied");
    let denied_binding = engine.commit_transaction(
        &limited.session_id,
        &limited.token,
        &transaction.transaction_id,
        &id("function-binding-commit"),
        &request,
        1_120,
        "request-function-binding-denied",
        "operation-function-binding-denied",
    );
    assert!(matches!(
        denied_binding,
        Err(ServiceError::PermissionDenied)
    ));
    assert_eq!(engine.storage.runtime().cursor().unwrap(), before);

    let records = SecurityRepository::new(&engine.storage, instance())
        .audit_since(0, 64)
        .unwrap()
        .records;
    for (request_id, action) in [
        (
            "request-function-execute-denied",
            SecurityAction::FunctionExecute,
        ),
        (
            "request-function-write-denied",
            SecurityAction::FunctionCatalogueWrite,
        ),
        (
            "request-function-binding-denied",
            SecurityAction::FunctionExecute,
        ),
    ] {
        assert!(records.iter().any(|(_, record)| {
            record.request_id == request_id
                && record.action == action
                && record.phase == AuditPhase::Completed
                && record.decision == AuditDecision::Denied
        }));
    }
}
