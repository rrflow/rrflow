use super::*;

fn limits() -> FunctionLimits {
    FunctionLimits {
        max_input_bytes: 4_096,
        max_output_bytes: 4_096,
        memory_bytes: 1024 * 1024,
        stack_bytes: 64 * 1024,
        max_interrupts: 1_000,
        max_fuel: 100_000,
    }
}

fn schema(id: &str, shape: FunctionValueShape) -> FunctionValueSchema {
    FunctionValueSchema {
        schema_id: CanonicalId::new(id).unwrap(),
        revision: 1,
        shape,
    }
}

fn javascript(id: &str, source: &str) -> (FunctionArtifact, FunctionDefinition) {
    let content_sha256 = sha256(source.as_bytes());
    let artifact = FunctionArtifact {
        content_sha256: content_sha256.clone(),
        media_type: FunctionArtifactMediaType::JavaScriptUtf8,
        byte_length: source.len() as u32,
        content_base64: STANDARD.encode(source),
    };
    let input_schema = schema("canonical-input", FunctionValueShape::CanonicalValueV1);
    let output_schema = schema("canonical-output", FunctionValueShape::CanonicalValueV1);
    let definition = FunctionDefinition {
        function_id: CanonicalId::new(id).unwrap(),
        revision: 1,
        predecessor_sha256: None,
        runtime: FunctionRuntime::JavaScriptEs2020 {
            artifact_sha256: content_sha256,
            runtime_profile: CanonicalId::new("javascript-es2020-json-v1").unwrap(),
            runtime_build_sha256: "1".repeat(64),
        },
        input_schema_sha256: input_schema.sha256(),
        input_schema,
        output_schema_sha256: output_schema.sha256(),
        output_schema,
        limits: limits(),
        capabilities: BTreeSet::new(),
    };
    (artifact, definition)
}

fn maximum_webassembly(id: &str, marker: u32) -> (FunctionArtifact, FunctionDefinition) {
    let mut content = vec![0_u8; MAX_FUNCTION_WASM_BYTES];
    content[..8].copy_from_slice(b"\0asm\x01\0\0\0");
    content[8..12].copy_from_slice(&marker.to_be_bytes());
    let content_sha256 = sha256(&content);
    let artifact = FunctionArtifact {
        content_sha256: content_sha256.clone(),
        media_type: FunctionArtifactMediaType::WebAssemblyBinary,
        byte_length: content.len().try_into().unwrap(),
        content_base64: STANDARD.encode(content),
    };
    let input_schema = schema(&format!("{id}-input"), FunctionValueShape::CanonicalValueV1);
    let output_schema = schema(
        &format!("{id}-output"),
        FunctionValueShape::CanonicalValueV1,
    );
    let definition = FunctionDefinition {
        function_id: CanonicalId::new(id).unwrap(),
        revision: 1,
        predecessor_sha256: None,
        runtime: FunctionRuntime::WebAssemblyV1 {
            artifact_sha256: content_sha256,
            runtime_profile: CanonicalId::new("webassembly-json-v1").unwrap(),
            runtime_build_sha256: "1".repeat(64),
            abi: WebAssemblyAbi::JsonV1,
        },
        input_schema_sha256: input_schema.sha256(),
        input_schema,
        output_schema_sha256: output_schema.sha256(),
        output_schema,
        limits: limits(),
        capabilities: BTreeSet::new(),
    };
    (artifact, definition)
}

#[test]
fn catalogue_rejects_digest_identity_capability_and_retry_substitution() {
    let (artifact, function) = javascript("validator", "(input) => input.allowed === true");
    let function_id = function.function_id.clone();
    let binding_id = CanonicalId::new("validate-record").unwrap();
    let mut catalogue = FunctionCatalogue {
        contract_version: FUNCTION_CONTRACT_VERSION,
        revision: 1,
        artifacts: BTreeMap::from([(artifact.content_sha256.clone(), artifact)]),
        functions: BTreeMap::from([(function_id.clone(), function.clone())]),
        transaction_bindings: BTreeMap::from([(
            binding_id.clone(),
            TransactionFunctionBinding {
                binding_id,
                revision: 1,
                predecessor_sha256: None,
                function_id,
                function_definition_sha256: function.sha256(),
                mutation: TransactionMutationKind::PutRecord,
                kind: Some(CanonicalId::new("person").unwrap()),
                effect: TransactionFunctionEffect::RequireTrue,
                max_attempts: 1,
            },
        )]),
    };
    catalogue.validate().unwrap();
    catalogue
        .artifacts
        .values_mut()
        .next()
        .unwrap()
        .content_sha256 = "0".repeat(64);
    assert!(catalogue.validate().is_err());

    let (artifact, mut event_function) = javascript("event-writer", "() => ({ ok: true })");
    let event_function_id = event_function.function_id.clone();
    let event_binding_id = CanonicalId::new("write-event").unwrap();
    let mut event_binding = TransactionFunctionBinding {
        binding_id: event_binding_id.clone(),
        revision: 1,
        predecessor_sha256: None,
        function_id: event_function_id.clone(),
        function_definition_sha256: event_function.sha256(),
        mutation: TransactionMutationKind::AppendEvent,
        kind: None,
        effect: TransactionFunctionEffect::AppendEvent {
            kind: CanonicalId::new("derived-event").unwrap(),
        },
        max_attempts: 1,
    };
    let mut event_catalogue = FunctionCatalogue {
        contract_version: FUNCTION_CONTRACT_VERSION,
        revision: 1,
        artifacts: BTreeMap::from([(artifact.content_sha256.clone(), artifact)]),
        functions: BTreeMap::from([(event_function_id.clone(), event_function.clone())]),
        transaction_bindings: BTreeMap::from([(event_binding_id.clone(), event_binding.clone())]),
    };
    assert!(event_catalogue.validate().is_err());
    event_function
        .capabilities
        .insert(FunctionCapability::EmitEvent);
    event_binding.function_definition_sha256 = event_function.sha256();
    event_catalogue
        .functions
        .insert(event_function_id, event_function);
    event_catalogue
        .transaction_bindings
        .insert(event_binding_id.clone(), event_binding);
    event_catalogue.validate().unwrap();
    event_catalogue
        .transaction_bindings
        .get_mut(&event_binding_id)
        .unwrap()
        .max_attempts = 2;
    assert!(event_catalogue.validate().is_err());

    assert!(ReplaceFunctionCatalogue {
        expected_revision: u64::MAX,
        catalogue: FunctionCatalogue {
            contract_version: FUNCTION_CONTRACT_VERSION,
            revision: u64::MAX,
            artifacts: BTreeMap::new(),
            functions: BTreeMap::new(),
            transaction_bindings: BTreeMap::new(),
        },
    }
    .validate()
    .is_err());
}

#[test]
fn catalogue_encoded_limit_admits_the_largest_max_artifact_prefix_and_rejects_the_next() {
    let mut candidate = FunctionCatalogue {
        contract_version: FUNCTION_CONTRACT_VERSION,
        revision: 1,
        artifacts: BTreeMap::new(),
        functions: BTreeMap::new(),
        transaction_bindings: BTreeMap::new(),
    };
    let mut accepted = None;
    let rejection = loop {
        let index = candidate.functions.len();
        let id = format!("maximum-wasm-{index:02}");
        let (artifact, definition) = maximum_webassembly(&id, index as u32);
        candidate
            .artifacts
            .insert(artifact.content_sha256.clone(), artifact);
        candidate
            .functions
            .insert(definition.function_id.clone(), definition);
        match candidate.validate() {
            Ok(()) => accepted = Some(candidate.clone()),
            Err(error) => break error,
        }
    };
    let accepted = accepted.expect("at least one maximum-size artifact is admitted");
    let encoded = serde_json::to_vec(&accepted).unwrap();
    assert!(encoded.len() <= MAX_FUNCTION_CATALOGUE_ENCODED_BYTES);
    assert!(
        encoded.len() > MAX_FUNCTION_CATALOGUE_ENCODED_BYTES - 2 * MAX_FUNCTION_WASM_BYTES,
        "the positive fixture must exercise the upper catalogue range"
    );
    assert!(
        accepted
            .artifacts
            .values()
            .map(|artifact| artifact.byte_length as usize)
            .sum::<usize>()
            <= MAX_FUNCTION_CATALOGUE_ARTIFACT_BYTES
    );
    assert!(rejection.to_string().contains("encoded bytes"));
}

#[test]
fn artifact_digest_covers_decoded_bytes_and_schema_is_enforced() {
    let module = b"\0asm\x01\0\0\0";
    let mut artifact = FunctionArtifact {
        content_sha256: sha256(module),
        media_type: FunctionArtifactMediaType::WebAssemblyBinary,
        byte_length: module.len() as u32,
        content_base64: STANDARD.encode(module),
    };
    artifact.validate().unwrap();
    artifact.content_base64.push('A');
    assert!(artifact.validate().is_err());

    let boolean = schema("boolean-result", FunctionValueShape::Boolean);
    boolean.validate_value(&QueryValue::Bool(true)).unwrap();
    assert!(boolean
        .validate_value(&QueryValue::String("true".into()))
        .is_err());
}

#[test]
fn invocation_receipt_binds_proposal_coordinates_budgets_and_output() {
    let output = QueryValue::Bool(true);
    let output_sha256 = sha256(&serde_json::to_vec(&output).unwrap());
    let receipt = FunctionInvocationReceipt {
        contract_version: FUNCTION_CONTRACT_VERSION,
        invocation_id: CanonicalId::new("function-receipt-one").unwrap(),
        catalogue_revision: 1,
        catalogue_sha256: "1".repeat(64),
        function_id: CanonicalId::new("validator").unwrap(),
        function_definition_sha256: "2".repeat(64),
        binding_id: None,
        mutation_index: None,
        runtime: FunctionRuntimeKind::JavaScriptEs2020,
        artifact_sha256: "3".repeat(64),
        runtime_profile: CanonicalId::new("javascript-es2020-json-v1").unwrap(),
        runtime_build_sha256: "4".repeat(64),
        input_schema_sha256: "5".repeat(64),
        output_schema_sha256: "6".repeat(64),
        input_sha256: "7".repeat(64),
        output_sha256,
        output,
        proposal: FunctionInvocationProposal::ReturnValue,
        runtime_commit_sha256: None,
        limits: limits(),
        attempts: 1,
        interrupts_consumed: 1,
        fuel_consumed: 0,
        receipt_sha256: String::new(),
    }
    .seal()
    .unwrap();
    receipt.validate().unwrap();

    let mut tampered = receipt.clone();
    tampered.runtime_build_sha256 = "8".repeat(64);
    assert!(tampered.validate().is_err());

    let mut unbound_proposal = receipt;
    unbound_proposal.receipt_sha256.clear();
    unbound_proposal.proposal = FunctionInvocationProposal::RequireTrue;
    assert!(unbound_proposal.seal().is_err());
}

#[test]
fn function_values_fail_closed_on_depth_and_item_amplification() {
    let mut nested = QueryValue::Null;
    for _ in 0..=MAX_FUNCTION_VALUE_DEPTH {
        nested = QueryValue::List(vec![nested]);
    }
    assert!(validate_function_value(&nested).is_err());
    assert!(validate_function_value(&QueryValue::List(vec![
        QueryValue::Null;
        MAX_FUNCTION_VALUE_ITEMS
    ]))
    .is_err());
}
