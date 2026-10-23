use rrflow_eval::storage::*;
use serde_json::{json, Value};
use std::path::Path;
use std::process::Command;

fn hash(byte: char) -> String {
    std::iter::repeat_n(byte, 64).collect()
}

fn fixtures() -> (StorageManifest, StorageResult) {
    let expected = json!([
        {"logical_id":"write-a", "request_sha256":hash('a'), "effect_sha256":hash('b'), "resolution":"present"},
        {"logical_id":"write-b", "request_sha256":hash('c'), "effect_sha256":hash('d'), "resolution":"absent"},
        {"logical_id":"write-c", "request_sha256":hash('e'), "effect_sha256":hash('f'), "resolution":"either"}
    ]);
    let operations: Vec<ExpectedOperation> = serde_json::from_value(expected.clone()).unwrap();
    let configuration = rrd_contract::EstateConfiguration::default();
    let mut memtable = json!({});
    for field in [
        "versions",
        "approximate_bytes",
        "keys",
        "key_payload_bytes",
        "value_payload_bytes",
        "tombstones",
        "spilled_chains",
        "spilled_version_capacity",
        "version_record_bytes",
        "owned_bytes_lower_bound",
    ] {
        memtable[field] = json!(0);
    }
    memtable["max_versions"] = json!(524288);
    let mut maintenance = json!({});
    for field in [
        "automatic_flushes",
        "failed_flushes",
        "write_stalls",
        "oversized_batches",
        "automatic_compactions",
        "failed_compactions",
        "compaction_input_bytes",
        "compaction_output_bytes",
        "peak_compaction_buffer_bytes",
    ] {
        maintenance[field] = json!(0);
    }
    let mut page_cache = json!({});
    for field in [
        "resident_bytes",
        "entries",
        "hits",
        "misses",
        "evictions",
        "page_loads",
        "bytes_read",
        "bytes_decoded",
        "bytes_borrowed",
        "bytes_allocated",
        "bytes_copied",
        "bytes_decompressed",
        "filter_checks",
        "filter_negatives",
    ] {
        page_cache[field] = json!(0);
    }
    page_cache["capacity_bytes"] = json!(1048576);
    let mut io = json!({"requested_mode":"bounded", "max_request_bytes":65536});
    for field in [
        "mmap_segments",
        "io_uring_segments",
        "bounded_segments",
        "fallbacks",
        "read_operations",
        "mmap_reads",
        "io_uring_reads",
        "bounded_reads",
        "bytes_read",
        "peak_request_bytes",
    ] {
        io[field] = json!(0);
    }
    let manifest: StorageManifest = serde_json::from_value(json!({
        "format_version":1, "profile":"synthetic-contract-oracle", "evidence_class":"synthetic", "durability":"authoritative",
        "candidate":{"commit":"1".repeat(40), "tree":"2".repeat(40), "clean":true, "executable_sha256":hash('3'), "public_catalogue_sha256":catalogue_digest().unwrap(), "configuration_sha256":configuration.configuration_sha256},
        "environment":{"platform":"linux_ext4", "target":"x86_64-unknown-linux-gnu", "toolchain":"synthetic-rust", "cpu":"synthetic-cpu", "os_version":"synthetic-linux", "filesystem_device":"synthetic-ext4-device", "mount_options":"synthetic-default", "flush_primitive":"synthetic-sync-data", "measurement_tool":"synthetic-contract-v1"},
        "corpus_sha256":hash('4'), "installed_configuration":configuration,
        "storage_observation":{"observed_at_unix_ms":1, "profile":"rrflow_kv", "evidence_level":"rrflow_kv_counters", "rrflow_kv":{
            "physical_sequence":0, "manifest_generation":1, "durable_sequence":0, "memtable":memtable,
            "write_ahead_log":{"payload_bytes":0,"payload_max_bytes":67108864}, "maintenance":maintenance,
            "segment_set":{"segments":0,"segment_bytes":0,"l0_segments":0,"l0_compaction_trigger":8,"compaction_debt_segments":0,"compaction_target_segment_bytes":8388608},
            "page_cache":page_cache,"segment_io":io}},
        "budgets":{"atomic_batch_bytes":1048576,"atomic_mutations":1024,"mutable_bytes":134217728,"mutable_versions":1048576,"queue_bytes":4194304,"queue_requests":8,"queue_wait_ms":1000,"maintenance_bytes":16777216,"maintenance_workers":1,"cache_bytes":1048576,"decoded_bytes":1048576,"retained_bytes":1048576,"recovery_memory_bytes":1048576,"recovery_replay_bytes":16777216,"recovery_frames":10000,"recovery_ms":10000,"process_bytes":268435456,"host_bytes":536870912,"estates":2,"disk_bytes":1073741824,"temporary_disk_bytes":16777216,"recovery_reserve_bytes":16777216,"run_ms":60000,"attempts":100},
        "bindings":[{"operation":"transaction-commit", "availability":"available", "driver_sha256":hash('5')}],
        "required_cases":[{"id":"ack-history", "operation":"transaction-commit", "seed":7,"expected_ledger_sha256":expected_digest(&operations).unwrap()}]
    })).unwrap();
    let result: StorageResult = serde_json::from_value(json!({
        "format_version":1,"manifest_sha256":manifest_digest(&manifest).unwrap(),"cases":[{
            "id":"ack-history","seed":7,"status":"executed","faults":[],"limitation":null,"ledger":{
                "expected":expected,
                "attempts":[
                    {"attempt_id":"a1","logical_id":"write-a","request_sha256":hash('a'),"started_ns":0,"finished_ns":10,"outcome":{"kind":"acknowledged","receipt_sha256":hash('1')}},
                    {"attempt_id":"a2","logical_id":"write-a","request_sha256":hash('a'),"started_ns":11,"finished_ns":20,"outcome":{"kind":"acknowledged","receipt_sha256":hash('1')}},
                    {"attempt_id":"b1","logical_id":"write-b","request_sha256":hash('c'),"started_ns":21,"finished_ns":30,"outcome":{"kind":"rejected","reason":"before-effect-admission"}},
                    {"attempt_id":"c1","logical_id":"write-c","request_sha256":hash('e'),"started_ns":31,"finished_ns":40,"outcome":{"kind":"uncertain","reason":"response-lost"}}
                ],
                "readback":[
                    {"kind":"present","logical_id":"write-a","request_sha256":hash('a'),"effect_sha256":hash('b'),"receipt_sha256":hash('1'),"applications":1},
                    {"kind":"absent","logical_id":"write-b"},
                    {"kind":"present","logical_id":"write-c","request_sha256":hash('e'),"effect_sha256":hash('f'),"receipt_sha256":hash('2'),"applications":1}
                ]
            }
        }]
    })).unwrap();
    (manifest, result)
}

fn manifest_json() -> Value {
    serde_json::to_value(fixtures().0).unwrap()
}
fn manifest_rejected(value: Value) -> bool {
    match serde_json::from_value::<StorageManifest>(value) {
        Err(_) => true,
        Ok(m) => m.validate().is_err(),
    }
}

#[test]
fn admits_closed_manifest_and_independent_acknowledgement_history() {
    let (manifest, result) = fixtures();
    manifest.validate().unwrap();
    let report = validate_result(&manifest, &result).unwrap();
    assert!(report.contract_passed);
    let ledger = &report.cases[0].ledger;
    assert_eq!(
        (
            ledger.logical_calls,
            ledger.attempts,
            ledger.acknowledged,
            ledger.rejected,
            ledger.uncertain
        ),
        (3, 4, 2, 1, 1)
    );
    assert_eq!(
        (
            ledger.resolved_present,
            ledger.resolved_absent,
            ledger.unresolved
        ),
        (2, 1, 0)
    );
    assert!(matches!(report.evidence_class, EvidenceClass::Synthetic));
    assert!(matches!(
        report.effective_settings[0].kind,
        PolicyKind::SoftFlushTarget
    ));
}

#[test]
fn rejects_unknown_fields_and_invalid_scalar_domains() {
    for pointer in [
        "",
        "/candidate",
        "/environment",
        "/budgets",
        "/bindings/0",
        "/required_cases/0",
        "/installed_configuration",
        "/storage_observation/rrflow_kv/memtable",
    ] {
        let mut input = manifest_json();
        input
            .pointer_mut(pointer)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("unknown".into(), json!(true));
        assert!(manifest_rejected(input), "{pointer}");
    }
    for value in [
        json!(0),
        json!(-1),
        json!(1.5),
        json!(u64::MAX),
        json!("unlimited"),
    ] {
        let mut input = manifest_json();
        input["budgets"]["atomic_batch_bytes"] = value;
        assert!(manifest_rejected(input));
    }
    for (pointer, value) in [
        ("/format_version", json!(2)),
        ("/candidate/commit", json!("bad")),
        ("/candidate/clean", json!(false)),
        ("/candidate/executable_sha256", json!("A".repeat(64))),
        ("/profile", json!("../escape")),
        ("/environment/cpu", json!("")),
        ("/environment/target", json!("aarch64-apple-darwin")),
        ("/environment/target", json!("x86_64-unknown-notlinux-gnu")),
        ("/bindings/0/availability", json!("unavailable")),
        ("/bindings/0/availability", json!("experimental")),
        ("/bindings/0/operation", json!("invented-operation")),
        ("/required_cases", json!([])),
        ("/installed_configuration/revision", json!(0)),
    ] {
        let mut input = manifest_json();
        *input.pointer_mut(pointer).unwrap() = value;
        assert!(manifest_rejected(input), "{pointer}");
    }
}

#[test]
fn rejects_budget_overflow_cross_field_conflicts_and_false_effective_policy() {
    for (field, value) in [
        ("process_bytes", 1),
        ("estates", MAX_EXACT_INTEGER),
        ("atomic_batch_bytes", 134217729),
        ("atomic_mutations", 1048577),
        ("disk_bytes", 33554432),
        ("cache_bytes", 1),
        ("decoded_bytes", 1),
        ("mutable_versions", 524288),
        ("recovery_ms", 60001),
        ("recovery_reserve_bytes", 16777217),
        ("run_ms", MAX_EXACT_INTEGER),
        ("attempts", 10001),
    ] {
        let mut input = manifest_json();
        input["budgets"][field] = json!(value);
        assert!(manifest_rejected(input), "{field}");
    }
}

#[test]
fn rejects_lost_ack_rejected_effect_corruption_and_duplicate_application() {
    let (manifest, baseline) = fixtures();
    for change in 0..5 {
        let mut input = serde_json::to_value(&baseline).unwrap();
        let ledger = &mut input["cases"][0]["ledger"];
        match change {
            0 => ledger["readback"][0] = json!({"kind":"absent","logical_id":"write-a"}),
            1 => {
                ledger["attempts"][0]["outcome"] =
                    json!({"kind":"acknowledged","receipt_sha256":hash('9')})
            }
            2 => ledger["readback"][0]["effect_sha256"] = json!(hash('9')),
            3 => ledger["readback"][0]["applications"] = json!(2),
            _ => {
                ledger["readback"][1] = json!({"kind":"present","logical_id":"write-b","request_sha256":hash('c'),"effect_sha256":hash('d'),"receipt_sha256":hash('3'),"applications":1});
            }
        }
        let result = serde_json::from_value(input).unwrap();
        let report = validate_result(&manifest, &result).unwrap();
        assert!(!report.contract_passed, "mutation {change}");
        assert!(!report.cases[0].ledger.violations.is_empty());
    }
}

#[test]
fn uncertainty_requires_explicit_readback_and_freezes_expected_content() {
    let (manifest, mut result) = fixtures();
    result.cases[0].ledger.readback.pop();
    let report = validate_result(&manifest, &result).unwrap();
    assert_eq!(report.cases[0].ledger.unresolved, 1);
    assert!(!report.contract_passed);
    result.cases[0].ledger.readback.push(Readback::Absent {
        logical_id: Id::try_from("write-c".to_owned()).unwrap(),
    });
    assert!(validate_result(&manifest, &result).unwrap().contract_passed);
    result.cases[0].ledger.expected[0].effect_sha256 = Digest::try_from(hash('9')).unwrap();
    assert!(validate_result(&manifest, &result)
        .unwrap_err()
        .contains("ledger changed"));
}

#[test]
fn rejecting_all_work_cannot_pass_a_required_effect() {
    let (manifest, mut result) = fixtures();
    for attempt in &mut result.cases[0].ledger.attempts {
        attempt.outcome = AttemptOutcome::Rejected {
            reason: Id::try_from("admission-failed".to_owned()).unwrap(),
        };
    }
    result.cases[0].ledger.readback[0] = Readback::Absent {
        logical_id: Id::try_from("write-a".to_owned()).unwrap(),
    };
    result.cases[0].ledger.readback[2] = Readback::Absent {
        logical_id: Id::try_from("write-c".to_owned()).unwrap(),
    };
    let report = validate_result(&manifest, &result).unwrap();
    assert!(!report.contract_passed);
    assert!(report.cases[0]
        .ledger
        .violations
        .iter()
        .any(|v| v.as_str() == "required-effect-absent"));
}

#[test]
fn result_identity_and_nested_admission_cannot_be_forged() {
    let (manifest, result) = fixtures();
    for pointer in [
        "",
        "/cases/0",
        "/cases/0/ledger",
        "/cases/0/ledger/expected/0",
        "/cases/0/ledger/attempts/0",
        "/cases/0/ledger/attempts/0/outcome",
        "/cases/0/ledger/readback/0",
    ] {
        let mut input = serde_json::to_value(&result).unwrap();
        input
            .pointer_mut(pointer)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("unknown".into(), json!(true));
        assert!(
            decode::<StorageResult>(&serde_json::to_vec(&input).unwrap()).is_err(),
            "{pointer}"
        );
    }
    for (pointer, value) in [
        ("/manifest_sha256", json!(hash('0'))),
        ("/cases/0/id", json!("unlisted")),
        (
            "/cases/0/ledger/attempts/0/request_sha256",
            json!(hash('0')),
        ),
        ("/cases/0/ledger/attempts/0/attempt_id", json!("a2")),
        ("/cases/0/ledger/readback/0/logical_id", json!("unlisted")),
    ] {
        let mut input = serde_json::to_value(&result).unwrap();
        *input.pointer_mut(pointer).unwrap() = value;
        assert!(
            validate_result(&manifest, &serde_json::from_value(input).unwrap()).is_err(),
            "{pointer}"
        );
    }
    let mut native = manifest.clone();
    native.evidence_class = EvidenceClass::NativeProcess;
    native.durability = DurabilityClass::Buffered;
    assert!(native.validate().is_err());
    let secret = b"{\"credential-that-must-not-be-echoed\":true}";
    let error = decode::<StorageManifest>(secret).unwrap_err();
    assert!(!error.contains("credential-that"));
    assert!(error.len() < 128);
}

#[test]
fn digest_binds_semantics_not_json_whitespace_and_rejects_duplicate_keys() {
    let (manifest, _) = fixtures();
    let pretty: StorageManifest = decode(&serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();
    let compact: StorageManifest = decode(&serde_json::to_vec(&manifest).unwrap()).unwrap();
    assert_eq!(
        manifest_digest(&pretty).unwrap(),
        manifest_digest(&compact).unwrap()
    );
    let mut bytes = serde_json::to_vec(&manifest).unwrap();
    bytes.splice(1..1, b"\"format_version\":1,".iter().copied());
    assert!(decode::<StorageManifest>(&bytes).is_err());
}

#[test]
fn coverage_failures_are_retained_not_silently_passed() {
    let (manifest, baseline) = fixtures();
    for status in [
        CaseStatus::Skipped,
        CaseStatus::Unimplemented,
        CaseStatus::Failed,
    ] {
        let mut result = baseline.clone();
        result.cases[0].status = status;
        result.cases[0].limitation = Some(Id::try_from("facility-unavailable".to_owned()).unwrap());
        result.cases[0].ledger.attempts.clear();
        result.cases[0].ledger.readback.clear();
        if status == CaseStatus::Failed {
            result.cases[0].faults.push(Failure {
                phase: FailurePhase::Recovery,
                code: Id::try_from("io-failure".to_owned()).unwrap(),
                occurrence: Limit::try_from(1).unwrap(),
                artifact_sha256: Digest::try_from(hash('8')).unwrap(),
            });
        }
        let report = validate_result(&manifest, &result).unwrap();
        assert!(!report.contract_passed);
        assert_eq!(report.failed + report.skipped + report.unimplemented, 1);
    }
    let mut missing = baseline.clone();
    missing.cases.clear();
    assert!(validate_result(&manifest, &missing).is_err());
    let mut wrong = baseline.clone();
    wrong.cases[0].seed += 1;
    assert!(validate_result(&manifest, &wrong).is_err());
    wrong = baseline.clone();
    wrong.cases[0].ledger.attempts[0].finished_ns = u64::MAX;
    assert!(validate_result(&manifest, &wrong).is_err());
    wrong = baseline.clone();
    let duplicate = wrong.cases[0].ledger.readback[0].clone();
    wrong.cases[0].ledger.readback.push(duplicate);
    assert!(validate_result(&manifest, &wrong).is_err());
}

fn golden(path: &Path, value: &Value) {
    let mut rendered = serde_json::to_string_pretty(value).unwrap();
    rendered.push('\n');
    if std::env::var_os("RRFLOW_UPDATE_STORAGE_GOLDENS").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, &rendered).unwrap();
    }
    assert_eq!(
        std::fs::read_to_string(path).unwrap(),
        rendered,
        "{}",
        path.display()
    );
}

#[test]
fn generated_schemas_and_synthetic_receipts_match_goldens() {
    let manifest_schema = schema::<StorageManifest>();
    assert_eq!(manifest_schema["$defs"]["Limit"]["minimum"], json!(1));
    assert_eq!(
        manifest_schema["$defs"]["Limit"]["maximum"],
        json!(MAX_EXACT_INTEGER)
    );
    assert_eq!(
        manifest_schema["$defs"]["Digest"]["pattern"],
        json!("^[0-9a-f]{64}$")
    );
    assert_eq!(manifest_schema["$defs"]["Id"]["maxLength"], json!(96));
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let schema_root = root.join("../../../eval/schemas");
    golden(
        &schema_root.join("storage-manifest-v1.json"),
        &schema::<StorageManifest>(),
    );
    golden(
        &schema_root.join("storage-result-v1.json"),
        &schema::<StorageResult>(),
    );
    golden(
        &schema_root.join("storage-report-v1.json"),
        &schema::<ValidationReport>(),
    );
    golden(
        &schema_root.join("storage-campaign-v1.json"),
        &schema::<StorageCampaign>(),
    );
    golden(
        &schema_root.join("storage-segment-v1.json"),
        &schema::<SustainedSegment>(),
    );
    golden(
        &schema_root.join("storage-campaign-result-v1.json"),
        &schema::<CampaignResult>(),
    );
    golden(
        &schema_root.join("storage-progress-event-v1.json"),
        &schema::<ProgressEvent>(),
    );
    golden(
        &schema_root.join("storage-fleet-result-v1.json"),
        &schema::<FleetResult>(),
    );
    golden(
        &schema_root.join("storage-fleet-trial-v1.json"),
        &schema::<FleetTrialResult>(),
    );
    golden(
        &schema_root.join("storage-acceptance-manifest-v1.json"),
        &schema::<StorageAcceptanceManifest>(),
    );
    golden(
        &schema_root.join("storage-acceptance-report-v1.json"),
        &schema::<StorageAcceptanceReport>(),
    );
    golden(
        &schema_root.join("storage-candidate-ci-v1.json"),
        &schema::<CandidateCiReceipt>(),
    );
    golden(
        &schema_root.join("storage-launch-v1.json"),
        &schema::<StorageLaunchReceipt>(),
    );
    let (manifest, result) = fixtures();
    let fixture_root = root.join("tests/fixtures/storage");
    golden(
        &fixture_root.join("manifest.json"),
        &serde_json::to_value(&manifest).unwrap(),
    );
    golden(
        &fixture_root.join("result.json"),
        &serde_json::to_value(&result).unwrap(),
    );
    golden(
        &fixture_root.join("report.json"),
        &serde_json::to_value(validate_result(&manifest, &result).unwrap()).unwrap(),
    );
}

#[test]
fn cli_is_bounded_preserves_reports_and_retains_failed_oracle() {
    let temp = tempfile::tempdir().unwrap();
    let (manifest, mut result) = fixtures();
    result.cases[0].ledger.readback.clear();
    let manifest_path = temp.path().join("manifest.json");
    let result_path = temp.path().join("result.json");
    let report_path = temp.path().join("report.json");
    std::fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    std::fs::write(&result_path, serde_json::to_vec(&result).unwrap()).unwrap();
    let invoke = || {
        Command::new(env!("CARGO_BIN_EXE_rrflow-eval"))
            .args(["storage", "verify"])
            .arg(&manifest_path)
            .arg(&result_path)
            .arg(&report_path)
            .output()
            .unwrap()
    };
    let output = invoke();
    assert!(!output.status.success());
    let retained = std::fs::read(&report_path).unwrap();
    let report: ValidationReport = decode(&retained).unwrap();
    assert_eq!(report.cases[0].ledger.unresolved, 3);
    assert!(!invoke().status.success());
    assert_eq!(std::fs::read(&report_path).unwrap(), retained);
    std::fs::write(&manifest_path, vec![b' '; MAX_DOCUMENT_BYTES as usize + 1]).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_rrflow-eval"))
        .args(["storage", "validate"])
        .arg(manifest_path)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("8 MiB"));
}
