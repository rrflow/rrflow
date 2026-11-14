//! Bounded, deterministic JavaScript and WebAssembly execution and catalogue batches.

use super::*;

#[test]
pub(super) fn javascript_and_webassembly_are_bounded_deterministic_and_restart_safe() {
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
pub(super) fn largest_admitted_public_catalogue_fits_one_physical_batch_and_reopens() {
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
