use std::process::Command;

#[test]
fn existing_retained_provider_evidence_still_verifies() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../eval/results/2026-08-18-summary.json");
    let output = Command::new(env!("CARGO_BIN_EXE_rrflow-eval"))
        .arg("verify")
        .arg(path)
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().trim(),
        "verified evaluation evidence: trials=8 successes=8 regressions=0"
    );
}

#[test]
fn existing_summary_preserves_paired_regression_and_token_accounting() {
    let temp = tempfile::tempdir().unwrap();
    for (arm, success) in [("baseline", true), ("runtime", false)] {
        let trial = serde_json::json!({"provider":"codex","repository":"fixture","scenario":"stale","arm":arm,"success":success,"expected":"answer","attempts":1,"tool_calls":2,"input_tokens":10,"output_tokens":5,"latency_ms":100,"exit_code":0,"raw_output":"fixture","stderr":""});
        std::fs::write(
            temp.path().join(format!("{arm}.json")),
            serde_json::to_vec(&trial).unwrap(),
        )
        .unwrap();
    }
    let output = Command::new(env!("CARGO_BIN_EXE_rrflow-eval"))
        .arg("summarize")
        .arg(temp.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout)
        .starts_with("trials=2 success=1/2 regressions=1 total_latency_ms=200 reported_tokens=30"));
}

#[test]
fn provider_run_argument_rejection_does_not_invoke_a_provider() {
    let output = Command::new(env!("CARGO_BIN_EXE_rrflow-eval"))
        .arg("run")
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("run expects seven arguments"));
}
