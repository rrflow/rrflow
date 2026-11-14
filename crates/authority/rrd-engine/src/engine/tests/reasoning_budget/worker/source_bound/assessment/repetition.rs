//! Repeated assessments, changed source identity and resumed work ceilings.

use super::*;

#[test]
pub(super) fn repeated_assessment_is_blocked_after_unrelated_git_policy_and_head_advance() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = host_unix_ms().saturating_sub(30_000);
    let mut fixture =
        source_worker_fixture_for_path_at(&listener, true, true, "nested/alpha.txt", base);
    configure_assessment_budget_with_repetition(&mut fixture, Some(1));
    publish_assessment_policy(&mut fixture, true, true, true);
    let request = assessment_request(&fixture, "repetition-first", binary_descriptor());
    let before = inspect_budget(&fixture, fixture.now + 1);
    assert_eq!(before.reserved_steps, 0);

    let (first_output, first_packet) = run_assessment(
        &fixture,
        &listener,
        &request,
        serde_json::json!({
            "outcome": "answered",
            "value": { "kind": "binary", "value": true }
        }),
        Some("source"),
    );
    assert_eq!(
        first_output.decision,
        TaskAssessmentDecision::Answered {
            value: TaskAssessmentValue::Binary { value: true }
        }
    );
    assert_eq!(
        first_packet["source"]["entry_identity"],
        fixture.source.entry_identity
    );
    let after_first = inspect_budget(&fixture, fixture.now + 2);
    assert_eq!(after_first.reserved_steps, 1);
    assert_eq!(after_first.stopped, None);

    let project_id = fixture
        .installed
        .preview
        .installation
        .target
        .project_id
        .clone();
    let before_head = crate::engine::attunement::load_accepted_source(
        &fixture.engine,
        &project_id,
        8 * 1024 * 1024,
        10_000,
    )
    .unwrap();
    let before_entry = before_head
        .proposal
        .entries()
        .iter()
        .find(|entry| entry.path == fixture.source_path)
        .unwrap()
        .clone();
    std::fs::write(
        fixture.installed.root.path().join(".git/index"),
        include_bytes!(
            "../../../../../../../../../compute/rrd-attunement/tests/fixtures/git-index/sha1-v3.index"
        ),
    )
    .unwrap();
    std::fs::write(
        fixture.installed.root.path().join(".gitignore"),
        b"*.tmp\nignored/\n",
    )
    .unwrap();
    std::fs::write(
        fixture.installed.root.path().join("nested/.gitignore"),
        b"*.bak\n",
    )
    .unwrap();
    let host_floor = host_unix_ms().max(fixture.job.updated_at_unix_ms + 1);
    let changed_head = refresh_unrelated_with_host_floor(
        fixture.installed.root.path(),
        &fixture.engine,
        &fixture.lease,
        &project_id,
        &mut fixture.job,
        "assessment-unrelated-beta.txt",
        Some(host_floor),
    );
    fixture.now = host_unix_ms().max(fixture.job.updated_at_unix_ms + 1);
    let after_head = crate::engine::attunement::load_accepted_source(
        &fixture.engine,
        &project_id,
        8 * 1024 * 1024,
        10_000,
    )
    .unwrap();
    let after_entry = after_head
        .proposal
        .entries()
        .iter()
        .find(|entry| entry.path == fixture.source_path)
        .unwrap();
    assert_ne!(changed_head, before_head.proposal_sha256);
    assert_ne!(
        before_head.proposal.revisions().policy_sha256,
        after_head.proposal.revisions().policy_sha256
    );
    assert_eq!(after_head.proposal_sha256, changed_head);
    assert_eq!(after_entry.identity, before_entry.identity);
    assert_eq!(after_entry.kind, before_entry.kind);

    let mut repeated = request.clone();
    repeated.attempt_id = id("assessment-repetition-distinct-attempt");
    let before_repeat = inspect_budget(&fixture, fixture.now + 1);
    assert_eq!(before_repeat, after_first);
    assert!(matches!(
        fixture.engine.execute_task_assessment_with_clock(
            &fixture.lease.session_id,
            &fixture.lease.token,
            &repeated,
            "assessment-repetition-distinct-attempt",
            "assessment-repetition-distinct-attempt",
            &crate::engine::clock::FixedClock::at(fixture.now + 1),
        ),
        Err(ServiceError::ReasoningBudgetStopped(
            rrd_contract::ReasoningBudgetStop::RepeatedWork
        ))
    ));
    let after_repeat = inspect_budget(&fixture, fixture.now + 2);
    assert_eq!(after_repeat.reserved_steps, before_repeat.reserved_steps);
    assert_eq!(
        after_repeat.charged_elapsed_ms,
        before_repeat.charged_elapsed_ms
    );
    assert_eq!(
        after_repeat.stopped,
        Some(rrd_contract::ReasoningBudgetStop::RepeatedWork)
    );
    assert!(first_output.validate().is_ok());
    listener.set_nonblocking(true).unwrap();
    assert!(matches!(
        listener.accept(),
        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock
    ));
}

#[test]
pub(super) fn changed_selected_source_gets_new_assessment_identity_without_budget_reset() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = host_unix_ms().saturating_sub(30_000);
    let mut fixture =
        source_worker_fixture_for_path_at(&listener, true, true, "nested/alpha.txt", base);
    configure_assessment_budget_with_repetition(&mut fixture, Some(1));
    publish_assessment_policy(&mut fixture, true, true, true);
    let first_request = assessment_request(&fixture, "changed-source-first", binary_descriptor());
    let first_before = inspect_budget(&fixture, fixture.now + 1);
    assert_eq!(first_before.reserved_steps, 0);
    let (first_output, first_packet) = run_assessment(
        &fixture,
        &listener,
        &first_request,
        serde_json::json!({
            "outcome": "answered",
            "value": { "kind": "binary", "value": true }
        }),
        Some("source"),
    );
    assert_eq!(
        first_packet["source"]["content_sha256"],
        fixture.source.expected_content_sha256
    );
    let first_after = inspect_budget(&fixture, fixture.now + 2);
    assert_eq!(first_after.reserved_steps, 1);
    assert_eq!(first_after.stopped, None);

    let changed_content = "selected source changed to a distinct admitted input\n";
    std::fs::write(
        fixture.installed.root.path().join(&fixture.source_path),
        changed_content,
    )
    .unwrap();
    let refresh_at = host_unix_ms().max(fixture.job.updated_at_unix_ms + 1);
    refresh_inventory_without_source_edits(
        &fixture.engine,
        &fixture.lease,
        &mut fixture.job,
        refresh_at,
    );
    fixture.now = host_unix_ms().max(fixture.job.updated_at_unix_ms + 1);
    let project_id = fixture
        .installed
        .preview
        .installation
        .target
        .project_id
        .clone();
    let current = crate::engine::attunement::load_accepted_source(
        &fixture.engine,
        &project_id,
        8 * 1024 * 1024,
        10_000,
    )
    .unwrap();
    let selected = current
        .proposal
        .entries()
        .iter()
        .find(|entry| entry.path == fixture.source_path)
        .unwrap();
    let content_sha256 = match &selected.kind {
        rrd_attunement::inventory::EntryKind::File {
            observation,
            content_sha256,
        } => {
            assert_eq!(observation.bytes, changed_content.len() as u64);
            assert_eq!(
                content_sha256,
                &digest::sha256_hex(changed_content.as_bytes())
            );
            content_sha256.clone()
        }
        other => panic!("selected source is not an admitted file: {other:?}"),
    };
    let second_source = TaskWorkerSourceRequest {
        contract_version: rrd_contract::TASK_WORKER_SOURCE_VERSION,
        entry_identity: selected.identity.clone(),
        expected_content_sha256: content_sha256,
        maximum_input_bytes: fixture.source.maximum_input_bytes,
    };
    fixture.source_content = changed_content.to_owned();

    let historical = fixture
        .engine
        .read_task_assessment(
            &fixture.lease.session_id,
            &fixture.lease.token,
            &ReadTaskAssessment {
                contract_version: TASK_ASSESSMENT_VERSION,
                scope: first_request.scope.clone(),
                task_id: first_request.task_id.clone(),
                attempt_id: first_request.attempt_id.clone(),
            },
            fixture.now + 1,
            "assessment-read-old-authorized-output-after-source-edit",
            "assessment-read-old-authorized-output-after-source-edit",
        )
        .unwrap();
    assert_eq!(historical.assessment, Some(first_output.clone()));

    let mut second_request =
        assessment_request(&fixture, "changed-source-second", binary_descriptor());
    second_request.source = second_source;
    let before_second = inspect_budget(&fixture, fixture.now + 2);
    assert_eq!(before_second.reserved_steps, 1);
    assert_eq!(before_second.stopped, None);
    assert_eq!(before_second.deadline_unix_ms, first_after.deadline_unix_ms);
    let (second_output, second_packet) = run_assessment(
        &fixture,
        &listener,
        &second_request,
        serde_json::json!({
            "outcome": "answered",
            "value": { "kind": "binary", "value": false }
        }),
        Some("source"),
    );
    assert_ne!(
        first_output.source.content_sha256,
        second_output.source.content_sha256
    );
    assert_eq!(
        second_output.source.content_sha256,
        digest::sha256_hex(changed_content.as_bytes())
    );
    assert_eq!(second_packet["source_content"], changed_content);
    let after_second = inspect_budget(&fixture, fixture.now + 3);
    assert_eq!(after_second.reserved_steps, 2);
    assert_eq!(after_second.stopped, None);
    assert_eq!(after_second.deadline_unix_ms, first_after.deadline_unix_ms);
    assert!(after_second.charged_elapsed_ms >= first_after.charged_elapsed_ms);
    listener.set_nonblocking(true).unwrap();
    assert!(matches!(
        listener.accept(),
        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock
    ));
}

#[test]
pub(super) fn repeated_work_resume_preserves_ceiling_and_admits_changed_source() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = host_unix_ms().saturating_sub(30_000);
    let mut fixture =
        source_worker_fixture_for_path_at(&listener, true, true, "nested/alpha.txt", base);
    configure_assessment_budget_with_repetition(&mut fixture, Some(1));
    publish_assessment_policy(&mut fixture, true, true, true);

    let first_request =
        assessment_request(&fixture, "resume-after-repeat-first", binary_descriptor());
    let (first_output, _) = run_assessment(
        &fixture,
        &listener,
        &first_request,
        serde_json::json!({
            "outcome": "answered",
            "value": { "kind": "binary", "value": true }
        }),
        Some("source"),
    );
    assert!(first_output.validate().is_ok());
    let after_first = inspect_budget(&fixture, fixture.now + 2);
    assert_eq!(after_first.reserved_steps, 1);
    assert_eq!(after_first.stopped, None);

    let mut repeated_request = first_request.clone();
    repeated_request.attempt_id = id("assessment-resume-after-repeat-second");
    assert!(matches!(
        fixture.engine.execute_task_assessment_with_clock(
            &fixture.lease.session_id,
            &fixture.lease.token,
            &repeated_request,
            "assessment-resume-after-repeat-second",
            "assessment-resume-after-repeat-second",
            &crate::engine::clock::FixedClock::at(fixture.now + 1),
        ),
        Err(ServiceError::ReasoningBudgetStopped(
            rrd_contract::ReasoningBudgetStop::RepeatedWork
        ))
    ));
    let stopped_budget = inspect_budget(&fixture, fixture.now + 2);
    assert_eq!(stopped_budget.reserved_steps, after_first.reserved_steps);
    assert_eq!(
        stopped_budget.deadline_unix_ms,
        after_first.deadline_unix_ms
    );
    assert_eq!(
        stopped_budget.stopped,
        Some(rrd_contract::ReasoningBudgetStop::RepeatedWork)
    );

    let first_resume = fixture
        .engine
        .resume_task_budget_with_clock(
            &fixture.lease.session_id,
            &fixture.lease.token,
            &rrd_contract::ResumeTaskBudget {
                scope: first_request.scope.clone(),
                task_id: first_request.task_id.clone(),
                expected_budget_sha256: operation_digest(&stopped_budget).unwrap(),
            },
            "assessment-resume-after-repeat-first-resume",
            "assessment-resume-after-repeat-first-resume",
            &crate::engine::clock::FixedClock::at(fixture.now + 3),
        )
        .unwrap()
        .budget
        .unwrap();
    assert_eq!(first_resume.stopped, None);
    assert_eq!(first_resume.reserved_steps, stopped_budget.reserved_steps);
    assert_eq!(
        first_resume.deadline_unix_ms,
        stopped_budget.deadline_unix_ms
    );
    assert_eq!(first_resume.active_attempt, None);
    assert!(!first_resume.execution_may_continue);
    assert_eq!(first_resume.stop_expected_sha256, None);
    assert_eq!(
        first_resume.charged_elapsed_ms,
        stopped_budget.charged_elapsed_ms
    );
    assert_eq!(
        first_resume.equivalent_attempts,
        stopped_budget.equivalent_attempts
    );

    let mut repeated_again = first_request.clone();
    repeated_again.attempt_id = id("assessment-resume-after-repeat-third");
    assert!(matches!(
        fixture.engine.execute_task_assessment_with_clock(
            &fixture.lease.session_id,
            &fixture.lease.token,
            &repeated_again,
            "assessment-resume-after-repeat-third",
            "assessment-resume-after-repeat-third",
            &crate::engine::clock::FixedClock::at(fixture.now + 4),
        ),
        Err(ServiceError::ReasoningBudgetStopped(
            rrd_contract::ReasoningBudgetStop::RepeatedWork
        ))
    ));
    let repeated_again_budget = inspect_budget(&fixture, fixture.now + 5);
    assert_eq!(
        repeated_again_budget.reserved_steps,
        first_resume.reserved_steps
    );
    assert_eq!(
        repeated_again_budget.deadline_unix_ms,
        first_resume.deadline_unix_ms
    );
    assert_eq!(
        repeated_again_budget.charged_elapsed_ms,
        first_resume.charged_elapsed_ms
    );
    assert_eq!(
        repeated_again_budget.equivalent_attempts,
        first_resume.equivalent_attempts
    );
    assert_eq!(
        repeated_again_budget.stopped,
        Some(rrd_contract::ReasoningBudgetStop::RepeatedWork)
    );
    assert!(fixture
        .engine
        .read_task_attempt(
            &fixture.lease.session_id,
            &fixture.lease.token,
            &ReadTaskAttempt {
                scope: first_request.scope.clone(),
                task_id: first_request.task_id.clone(),
                attempt_id: repeated_again.attempt_id.clone(),
            },
            fixture.now + 5,
            "assessment-repeated-work-rejection-has-no-attempt",
            "assessment-repeated-work-rejection-has-no-attempt",
        )
        .unwrap()
        .attempt
        .is_none());

    let second_resume = fixture
        .engine
        .resume_task_budget_with_clock(
            &fixture.lease.session_id,
            &fixture.lease.token,
            &rrd_contract::ResumeTaskBudget {
                scope: first_request.scope.clone(),
                task_id: first_request.task_id.clone(),
                expected_budget_sha256: operation_digest(&repeated_again_budget).unwrap(),
            },
            "assessment-resume-after-repeat-second-resume",
            "assessment-resume-after-repeat-second-resume",
            &crate::engine::clock::FixedClock::at(fixture.now + 6),
        )
        .unwrap()
        .budget
        .unwrap();
    assert_eq!(second_resume.stopped, None);
    assert_eq!(
        second_resume.reserved_steps,
        repeated_again_budget.reserved_steps
    );
    assert_eq!(
        second_resume.deadline_unix_ms,
        repeated_again_budget.deadline_unix_ms
    );
    assert_eq!(second_resume.active_attempt, None);
    assert!(!second_resume.execution_may_continue);
    assert_eq!(second_resume.stop_expected_sha256, None);
    assert_eq!(
        second_resume.charged_elapsed_ms,
        repeated_again_budget.charged_elapsed_ms
    );
    assert_eq!(
        second_resume.equivalent_attempts,
        repeated_again_budget.equivalent_attempts
    );

    let changed_content = "selected source changed after repeated work was stopped\n";
    std::fs::write(
        fixture.installed.root.path().join(&fixture.source_path),
        changed_content,
    )
    .unwrap();
    let refresh_at = host_unix_ms().max(fixture.job.updated_at_unix_ms + 1);
    refresh_inventory_without_source_edits(
        &fixture.engine,
        &fixture.lease,
        &mut fixture.job,
        refresh_at,
    );
    fixture.now = host_unix_ms().max(fixture.job.updated_at_unix_ms + 1);

    let project_id = fixture
        .installed
        .preview
        .installation
        .target
        .project_id
        .clone();
    let accepted = crate::engine::attunement::load_accepted_source(
        &fixture.engine,
        &project_id,
        8 * 1024 * 1024,
        10_000,
    )
    .unwrap();
    let selected = accepted
        .proposal
        .entries()
        .iter()
        .find(|entry| entry.path == fixture.source_path)
        .unwrap();
    assert_eq!(selected.identity, fixture.source.entry_identity);
    let current_content_sha256 = match &selected.kind {
        rrd_attunement::inventory::EntryKind::File {
            observation,
            content_sha256,
        } => {
            assert_eq!(observation.bytes, changed_content.len() as u64);
            assert_eq!(
                content_sha256,
                &digest::sha256_hex(changed_content.as_bytes())
            );
            content_sha256.clone()
        }
        other => panic!("selected source is not an admitted file: {other:?}"),
    };
    fixture.source_content = changed_content.to_owned();

    let before_changed_assessment = inspect_budget(&fixture, fixture.now + 1);
    assert_eq!(
        before_changed_assessment.reserved_steps,
        first_resume.reserved_steps
    );
    assert_eq!(before_changed_assessment.stopped, None);
    assert_eq!(
        before_changed_assessment.deadline_unix_ms,
        first_resume.deadline_unix_ms
    );
    assert_eq!(
        before_changed_assessment.equivalent_attempts,
        first_resume.equivalent_attempts
    );
    let mut changed_request = assessment_request(
        &fixture,
        "resume-after-repeat-changed-source",
        binary_descriptor(),
    );
    changed_request.source = TaskWorkerSourceRequest {
        contract_version: rrd_contract::TASK_WORKER_SOURCE_VERSION,
        entry_identity: selected.identity.clone(),
        expected_content_sha256: current_content_sha256,
        maximum_input_bytes: fixture.source.maximum_input_bytes,
    };
    let (changed_output, changed_packet) = run_assessment(
        &fixture,
        &listener,
        &changed_request,
        serde_json::json!({
            "outcome": "answered",
            "value": { "kind": "binary", "value": false }
        }),
        Some("source"),
    );
    assert_eq!(changed_packet["source_content"], changed_content);
    assert_eq!(
        changed_output.source.content_sha256,
        digest::sha256_hex(changed_content.as_bytes())
    );
    let completed_budget = inspect_budget(&fixture, fixture.now + 3);
    assert_eq!(
        completed_budget.reserved_steps,
        first_resume.reserved_steps + 1
    );
    assert_eq!(
        completed_budget.deadline_unix_ms,
        first_resume.deadline_unix_ms
    );
    assert_eq!(completed_budget.stopped, None);
    assert!(completed_budget.charged_elapsed_ms >= first_resume.charged_elapsed_ms);
    assert_eq!(completed_budget.equivalent_attempts.len(), 2);
    assert_eq!(
        completed_budget.equivalent_attempts.values().sum::<u64>(),
        2
    );
    assert_eq!(
        completed_budget.deadline_unix_ms,
        before_changed_assessment.deadline_unix_ms
    );

    let reopen_at = host_unix_ms();
    assert!(reopen_at < fixture.lease.absolute_expires_at_unix_ms);
    assert!(reopen_at < completed_budget.deadline_unix_ms);
    let SourceWorkerFixture {
        installed,
        engine,
        lease,
        job,
        principal_id,
        worker_token,
        selection,
        source,
        source_path,
        source_content,
        now: _,
    } = fixture;
    drop(engine);
    let reopened = installed.reopen(reopen_at);
    let reopened_fixture = SourceWorkerFixture {
        installed,
        engine: reopened,
        lease,
        job,
        principal_id,
        worker_token,
        selection,
        source,
        source_path,
        source_content,
        now: reopen_at,
    };
    assert_eq!(
        inspect_budget(&reopened_fixture, reopen_at + 1),
        completed_budget
    );
    assert_eq!(
        reopened_fixture
            .engine
            .read_task_assessment(
                &reopened_fixture.lease.session_id,
                &reopened_fixture.lease.token,
                &ReadTaskAssessment {
                    contract_version: TASK_ASSESSMENT_VERSION,
                    scope: changed_request.scope.clone(),
                    task_id: changed_request.task_id.clone(),
                    attempt_id: changed_request.attempt_id.clone(),
                },
                reopen_at + 2,
                "assessment-repeated-work-recovery-read-after-reopen",
                "assessment-repeated-work-recovery-read-after-reopen",
            )
            .unwrap()
            .assessment,
        Some(changed_output)
    );
    assert_eq!(
        reopened_fixture
            .engine
            .read_task_assessment(
                &reopened_fixture.lease.session_id,
                &reopened_fixture.lease.token,
                &ReadTaskAssessment {
                    contract_version: TASK_ASSESSMENT_VERSION,
                    scope: first_request.scope.clone(),
                    task_id: first_request.task_id.clone(),
                    attempt_id: first_request.attempt_id.clone(),
                },
                reopen_at + 3,
                "assessment-repeated-work-first-output-read-after-reopen",
                "assessment-repeated-work-first-output-read-after-reopen",
            )
            .unwrap()
            .assessment,
        Some(first_output)
    );
    for rejected in [&repeated_request, &repeated_again] {
        assert!(reopened_fixture
            .engine
            .read_task_attempt(
                &reopened_fixture.lease.session_id,
                &reopened_fixture.lease.token,
                &ReadTaskAttempt {
                    scope: rejected.scope.clone(),
                    task_id: rejected.task_id.clone(),
                    attempt_id: rejected.attempt_id.clone(),
                },
                reopen_at + 4,
                "assessment-repeated-work-rejected-attempt-read-after-reopen",
                "assessment-repeated-work-rejected-attempt-read-after-reopen",
            )
            .unwrap()
            .attempt
            .is_none());
    }
    assert!(matches!(
        reopened_fixture.engine.execute_task_assessment_with_clock(
            &reopened_fixture.lease.session_id,
            &reopened_fixture.lease.token,
            &changed_request,
            "assessment-repeated-work-recovery-no-replay",
            "assessment-repeated-work-recovery-no-replay",
            &crate::engine::clock::FixedClock::at(reopen_at + 5),
        ),
        Err(ServiceError::ReasoningAttemptRecorded {
            outcome_sha256: Some(_)
        })
    ));
    assert_eq!(
        inspect_budget(&reopened_fixture, reopen_at + 6),
        completed_budget
    );
    listener.set_nonblocking(true).unwrap();
    assert!(matches!(
        listener.accept(),
        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock
    ));
}
