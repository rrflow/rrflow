use rrd_contract::*;
use sha2::{Digest, Sha256};

fn digest(byte: char) -> String {
    std::iter::repeat_n(byte, 64).collect()
}

fn sha256_bytes(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn binary() -> TaskAssessmentDescriptor {
    TaskAssessmentDescriptor::Binary {
        proposition: "The source defines the selected operation".into(),
    }
}

fn reply_json(decision: &str, references: &str) -> Vec<u8> {
    format!(r#"{{"contract_version":1,"decision":{decision},"references":{references}}}"#)
        .into_bytes()
}

fn valid_source_reference(source: &str) -> String {
    format!(
        r#"[{{"input":"source","input_sha256":"{}","start_byte":0,"end_byte":1}}]"#,
        sha256_bytes(source.as_bytes())
    )
}

#[test]
fn all_three_assessment_modes_and_abstention_are_closed_and_typed() {
    let binary_reply = reply_json(
        r#"{"outcome":"answered","value":{"kind":"binary","value":true}}"#,
        &valid_source_reference("fn main() {}"),
    );
    assert!(TaskAssessmentReply::decode(
        &binary_reply,
        &binary(),
        "fn main() {}",
        "Check the operation definition"
    )
    .is_ok());

    let choice = TaskAssessmentDescriptor::Choice {
        question: "Which operation writes the receipt?".into(),
        options: vec![
            AssessmentOption {
                id: CanonicalId::new("commit").unwrap(),
                description: "Writes the receipt".into(),
            },
            AssessmentOption {
                id: CanonicalId::new("preview").unwrap(),
                description: "Returns a read-only preview".into(),
            },
        ],
    };
    let choice_reply = reply_json(
        r#"{"outcome":"answered","value":{"kind":"choice","option_id":"commit"}}"#,
        &valid_source_reference("fn main() {}"),
    );
    assert!(TaskAssessmentReply::decode(
        &choice_reply,
        &choice,
        "fn main() {}",
        "Compare the two operation contracts"
    )
    .is_ok());

    let score = TaskAssessmentDescriptor::Score {
        rubric: "Count the declared validation steps".into(),
        minimum: 0.0,
        maximum: 4.0,
        direction: AssessmentScoreDirection::HigherIsBetter,
    };
    let score_reply = reply_json(
        r#"{"outcome":"answered","value":{"kind":"score","value":3.5}}"#,
        &valid_source_reference("fn main() {}"),
    );
    assert!(TaskAssessmentReply::decode(
        &score_reply,
        &score,
        "fn main() {}",
        "Use only the supplied rubric"
    )
    .is_ok());

    let abstention = reply_json(
        r#"{"outcome":"abstained","reason":"The required operation is not present in these inputs"}"#,
        "[]",
    );
    assert!(TaskAssessmentReply::decode(
        &abstention,
        &binary(),
        "fn main() {}",
        "Check the operation definition"
    )
    .is_ok());
}

#[test]
fn assessment_decoder_rejects_missing_duplicate_forged_and_mismatched_references() {
    let source = "fn main() {}";
    let practice = "Check the operation definition";
    let answered = r#"{"outcome":"answered","value":{"kind":"binary","value":true}}"#;
    assert!(
        TaskAssessmentReply::decode(&reply_json(answered, "[]"), &binary(), source, practice)
            .is_err()
    );

    let one = valid_source_reference(source);
    let duplicate = one.replace("][", "},{");
    let duplicate = duplicate.replace("]", &format!(",{}]", &one[1..one.len() - 1]));
    assert!(TaskAssessmentReply::decode(
        &reply_json(answered, &duplicate),
        &binary(),
        source,
        practice
    )
    .is_err());

    let forged_digest = format!(
        r#"[{{"input":"source","input_sha256":"{}","start_byte":0,"end_byte":1}}]"#,
        digest('f')
    );
    assert!(TaskAssessmentReply::decode(
        &reply_json(answered, &forged_digest),
        &binary(),
        source,
        practice
    )
    .is_err());

    let beyond_end = format!(
        r#"[{{"input":"source","input_sha256":"{}","start_byte":0,"end_byte":999}}]"#,
        sha256_bytes(source.as_bytes())
    );
    assert!(TaskAssessmentReply::decode(
        &reply_json(answered, &beyond_end),
        &binary(),
        source,
        practice
    )
    .is_err());

    assert!(TaskAssessmentReply::decode(
        &reply_json(answered, &one),
        &TaskAssessmentDescriptor::Choice {
            question: "Choose".into(),
            options: vec![
                AssessmentOption {
                    id: CanonicalId::new("one").unwrap(),
                    description: "One".into(),
                },
                AssessmentOption {
                    id: CanonicalId::new("two").unwrap(),
                    description: "Two".into(),
                },
            ],
        },
        source,
        practice
    )
    .is_err());

    let unknown = br#"{"contract_version":1,"decision":{"outcome":"answered","value":{"kind":"binary","value":true}},"references":[],"extra":1}"#;
    assert!(TaskAssessmentReply::decode(unknown, &binary(), source, practice).is_err());
    let wrong_version = String::from_utf8(reply_json(
        r#"{"outcome":"answered","value":{"kind":"binary","value":true}}"#,
        &one,
    ))
    .unwrap()
    .replace("\"contract_version\":1", "\"contract_version\":2");
    assert!(
        TaskAssessmentReply::decode(wrong_version.as_bytes(), &binary(), source, practice).is_err()
    );
}

#[test]
fn assessment_mode_bounds_and_persisted_output_validation_are_strict() {
    assert!(TaskAssessmentDescriptor::Choice {
        question: "Choose".into(),
        options: vec![AssessmentOption {
            id: CanonicalId::new("only").unwrap(),
            description: "Only".into(),
        }],
    }
    .validate()
    .is_err());
    assert!(TaskAssessmentDescriptor::Score {
        rubric: "Bounded".into(),
        minimum: 1.0,
        maximum: f64::INFINITY,
        direction: AssessmentScoreDirection::HigherIsBetter,
    }
    .validate()
    .is_err());

    let source = "fn main() {}";
    let practice = "Check the operation definition";
    let valid = TaskAssessmentReply::decode(
        &reply_json(
            r#"{"outcome":"answered","value":{"kind":"binary","value":true}}"#,
            &valid_source_reference(source),
        ),
        &binary(),
        source,
        practice,
    )
    .unwrap();
    let output = TaskAssessmentOutput {
        contract_version: 1,
        scope: "engram:demo".into(),
        task_id: CanonicalId::new("task-one").unwrap(),
        attempt_id: CorrelationId::new("assessment-one").unwrap(),
        task_sha256: digest('a'),
        configuration_sha256: digest('b'),
        packet_sha256: digest('c'),
        request_sha256: digest('d'),
        response_sha256: digest('e'),
        content_sha256: digest('f'),
        controller_binding_id: CanonicalId::new("controller").unwrap(),
        service_id: CanonicalId::new("controller-service").unwrap(),
        descriptor: binary(),
        practice_identity: AssessmentPracticeIdentity {
            question_set_id: CanonicalId::new("set").unwrap(),
            question_id: CanonicalId::new("question").unwrap(),
            template_id: CanonicalId::new("template").unwrap(),
            template_revision: 1,
            template_sha256: digest('1'),
            requirement_id: CanonicalId::new("requirement").unwrap(),
        },
        practice_sha256: sha256_bytes(practice.as_bytes()),
        source_bytes: source.len() as u64,
        practice_bytes: practice.len() as u64,
        source: TaskWorkerSourceProvenance {
            contract_version: 1,
            captured_snapshot_sha256: digest('2'),
            captured_read: ContextReadStamp {
                runtime_manifest_sha256: digest('3'),
                runtime_cursor: 1,
                schema_revision: Some(1),
                catalogue_revision: 1,
            },
            project_id: CanonicalId::new("project").unwrap(),
            entry_identity: digest('4'),
            path_sha256: digest('5'),
            content_sha256: sha256_bytes(source.as_bytes()),
            policy_sha256: digest('6'),
            profile_sha256: digest('7'),
            captured_at_unix_ms: 1,
            validated_snapshot_sha256: digest('8'),
            validated_read: ContextReadStamp {
                runtime_manifest_sha256: digest('3'),
                runtime_cursor: 1,
                schema_revision: Some(1),
                catalogue_revision: 1,
            },
            validated_at_unix_ms: 1,
        },
        read: ContextReadStamp {
            runtime_manifest_sha256: digest('3'),
            runtime_cursor: 1,
            schema_revision: Some(1),
            catalogue_revision: 1,
        },
        decision: valid.decision,
        references: valid.references,
        usage: None,
        elapsed_ms: None,
        http_status: Some(200),
    };
    assert!(output.validate().is_ok());

    let mut abstained = output.clone();
    abstained.decision = TaskAssessmentDecision::Abstained {
        reason: "The evidence is insufficient".into(),
    };
    abstained.references.clear();
    assert!(abstained.validate().is_ok());

    let mut no_source_evidence = abstained.clone();
    no_source_evidence.source_bytes = 0;
    no_source_evidence.source.content_sha256 = sha256_bytes(b"");
    assert!(no_source_evidence.validate().is_ok());

    let mut forged = output.clone();
    forged.decision = TaskAssessmentDecision::Answered {
        value: TaskAssessmentValue::Score { value: f64::NAN },
    };
    assert!(forged.validate().is_err());

    let mut changed_range = output;
    changed_range.references[0].end_byte = 999;
    assert!(changed_range.validate().is_err());
}

#[test]
fn assessment_request_rejects_context_before_engine_dispatch() {
    let request = serde_json::json!({
        "contract_version": 1,
        "scope": "engram:demo",
        "task_id": "task-one",
        "attempt_id": "assessment-one",
        "expected_task_sha256": digest('a'),
        "expected_configuration_sha256": digest('b'),
        "max_storage_keys": 32,
        "source": {
            "contract_version": 1,
            "entry_identity": digest('c'),
            "expected_content_sha256": digest('d'),
            "maximum_input_bytes": 4096
        },
        "descriptor": { "mode": "binary", "proposition": "Check the selected input" },
        "practice_identity": {
            "question_set_id": "set",
            "question_id": "question",
            "template_id": "template",
            "template_revision": 1,
            "template_sha256": digest('e'),
            "requirement_id": "requirement"
        },
        "practice_text": "Use only the supplied source",
        "context": { "scope": "engram:demo" }
    });
    assert!(serde_json::from_value::<TaskAssessmentRequest>(request).is_err());
}
