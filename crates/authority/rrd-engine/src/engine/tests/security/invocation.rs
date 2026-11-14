//! Audit reads, overlapping invocations, policy grants and audited engine invocation.

use super::*;

#[test]
pub(super) fn audit_read_and_export_return_the_same_bounded_prefix_at_every_page_size() {
    let (_root, engine) = isolated_engine();
    let principal_id = CanonicalId::new("audit-page-reader").unwrap();
    let resource = ResourcePath {
        segments: vec![ResourceId::new(ResourceKind::Instance, instance().as_str()).unwrap()],
    };
    let grants = [
        SecurityAction::SessionCreate,
        SecurityAction::AuditRead,
        SecurityAction::AuditExport,
    ]
    .into_iter()
    .map(|action| ResourceGrant {
        action,
        resource_prefix: resource.clone(),
        data_policy: None,
    })
    .collect();
    let repository = SecurityRepository::new(&engine.storage, instance());
    repository
        .initialize(
            SecurityState {
                format_version: rrd_security::SECURITY_FORMAT,
                revision: 1,
                principals: [(
                    principal_id.clone(),
                    Principal {
                        id: principal_id.clone(),
                        kind: PrincipalKind::Service,
                        credential_sha256: digest::sha256_hex(b"audit-page-key"),
                        credential_revision: 1,
                        not_before_unix_ms: 1,
                        expires_at_unix_ms: u64::MAX,
                        disabled: false,
                        role_ids: Default::default(),
                        grants,
                    },
                )]
                .into_iter()
                .collect(),
                roles: Default::default(),
                identity_bindings: Default::default(),
                jwt_issuers: Default::default(),
            },
            1,
            "audit-page-test",
            "audit-page-initialize",
            "audit-page-initialize",
        )
        .unwrap();
    for index in 0..30 {
        repository
            .append_audit(&rrd_security::AuditEvent {
                audit_id: CanonicalId::new(format!("audit-seed-{index}")).unwrap(),
                at_unix_ms: 10 + index,
                principal_id: None,
                action: SecurityAction::QueryExecute,
                resource: resource.clone(),
                request_id: format!("seed-request-{index}"),
                operation_id: format!("seed-operation-{index}"),
                phase: AuditPhase::Completed,
                decision: AuditDecision::Allowed,
                status_code: 200,
                request_sha256: digest::sha256_hex(b"request"),
                response_sha256: digest::sha256_hex(b"response"),
            })
            .unwrap();
    }
    let lease = engine
        .create_authenticated_session(
            &principal_id,
            b"audit-page-key",
            &session_request(5_000, 2),
            &id("audit-page-session"),
            1_000,
            "audit-page-session",
            "audit-page-session",
        )
        .unwrap();
    // Authorization itself may append later audit records. All comparisons
    // stay within the already committed prefix, rather than assuming a static head.
    let expected = engine
        .read_audit(
            &lease.session_id,
            &lease.token,
            &rrd_contract::ReadAudit {
                after_sequence: 0,
                limit: 14,
            },
            1_100,
            "audit-reference",
            "audit-reference",
        )
        .unwrap()
        .records;
    for limit in 1..=14 {
        let mut after_sequence = 0;
        let mut assembled = Vec::new();
        let mut previous_head = None;
        while assembled.len() < expected.len() {
            let page = engine
                .read_audit(
                    &lease.session_id,
                    &lease.token,
                    &rrd_contract::ReadAudit {
                        after_sequence,
                        limit,
                    },
                    1_100,
                    &format!("audit-read-{limit}-{after_sequence}"),
                    &format!("audit-read-{limit}-{after_sequence}"),
                )
                .unwrap();
            let export = engine
                .export_audit(
                    &lease.session_id,
                    &lease.token,
                    &rrd_contract::ExportAudit {
                        after_sequence,
                        limit,
                    },
                    1_100,
                    &format!("audit-export-{limit}-{after_sequence}"),
                    &format!("audit-export-{limit}-{after_sequence}"),
                )
                .unwrap();
            page.validate().unwrap();
            export.validate().unwrap();
            let exported: Vec<rrd_contract::AuditRecordSnapshot> = export
                .json_lines
                .lines()
                .map(|line| serde_json::from_str(line).unwrap())
                .collect();
            assert_eq!(exported, page.records, "limit {limit}");
            assert_eq!(export.through_sequence, page.through_sequence);
            assert_eq!(export.chain_head_sha256, page.chain_head_sha256);
            assert_eq!(export.chain_anchor_sha256, page.chain_anchor_sha256);
            assert_eq!(page.chain_anchor_sha256, previous_head);
            assert!(!page.records.is_empty());
            assert!(page.through_sequence > after_sequence);
            after_sequence = page.through_sequence;
            previous_head = page.chain_head_sha256;
            assembled.extend(page.records);
        }
        assert_eq!(&assembled[..expected.len()], expected, "limit {limit}");
    }
}

#[test]
pub(super) fn overlapping_invocation_rejection_preserves_original_owner() {
    let (_root, persistent) = isolated_engine();
    for engine in [persistent, RrdEngine::rrflow_mx(instance(), TOKEN_KEY)] {
        let first = engine
            .begin_invocation(
                overlapping_backup_invocation(1_000, 1),
                RrdOperation::BackupCreate,
                InvocationCredential::Anonymous,
            )
            .unwrap();
        let (error, owns_unrelated_call) = std::thread::scope(|scope| {
            scope
                .spawn(|| {
                    let error = engine
                        .begin_invocation(
                            overlapping_backup_invocation(1_100, 2),
                            RrdOperation::BackupCreate,
                            InvocationCredential::Anonymous,
                        )
                        .unwrap_err();
                    (
                        error,
                        engine.has_active_invocation("unrelated-request", "unrelated-operation"),
                    )
                })
                .join()
                .unwrap()
        });
        assert!(
            !owns_unrelated_call,
            "rejected caller stole the active thread reservation"
        );
        assert!(engine.has_active_invocation("unrelated-request", "unrelated-operation"));
        assert_eq!(error.kind(), ServiceErrorKind::Conflict);
        assert!(
            error.retryable(),
            "in-flight work is not a permanent payload conflict"
        );
        let audit = SecurityRepository::new(&engine.storage, instance())
            .audit_since(0, 16)
            .unwrap();
        assert_eq!(
            audit
                .records
                .iter()
                .filter(|(_, record)| {
                    record.action == SecurityAction::BackupCreate
                        && record.phase == AuditPhase::Completed
                        && record.decision == AuditDecision::Failed
                        && record.status_code == 409
                })
                .count(),
            1,
            "the overlapping request must have its own completed denial"
        );
        engine
            .complete_invocation(&first, successful_invocation_completion())
            .unwrap();
        assert!(!engine.has_active_invocation("overlap-backup-request", "overlap-backup-operation"));
    }
}

#[test]
pub(super) fn overlapping_invocation_stale_completion_cannot_remove_a_later_attempt() {
    let (_root, persistent) = isolated_engine();
    for engine in [persistent, RrdEngine::rrflow_mx(instance(), TOKEN_KEY)] {
        let first = engine
            .begin_invocation(
                overlapping_backup_invocation(1_000, 1),
                RrdOperation::BackupCreate,
                InvocationCredential::Anonymous,
            )
            .unwrap();
        engine
            .complete_invocation(&first, successful_invocation_completion())
            .unwrap();
        let second = engine
            .begin_invocation(
                // Even identical caller metadata must receive a new reservation.
                overlapping_backup_invocation(1_000, 1),
                RrdOperation::BackupCreate,
                InvocationCredential::Anonymous,
            )
            .unwrap();
        assert!(matches!(
            engine.complete_invocation(&first, successful_invocation_completion()),
            Err(ServiceError::IdempotencyConflict)
        ));
        assert!(engine.has_active_invocation("overlap-backup-request", "overlap-backup-operation"));
        engine
            .complete_invocation(&second, successful_invocation_completion())
            .unwrap();
        assert!(!engine.has_active_invocation("overlap-backup-request", "overlap-backup-operation"));
    }
}

#[test]
pub(super) fn embedded_engine_cannot_bypass_policy_grants() {
    let (_root, engine) = isolated_engine();
    let principal_id = CanonicalId::new("backup-denied").unwrap();
    let resource = ResourcePath {
        segments: vec![ResourceId::new(ResourceKind::Instance, instance().as_str()).unwrap()],
    };
    let principal = Principal {
        id: principal_id.clone(),
        kind: PrincipalKind::Service,
        credential_sha256: digest::sha256_hex(b"session-only-key"),
        credential_revision: 1,
        not_before_unix_ms: 1,
        expires_at_unix_ms: u64::MAX,
        disabled: false,
        role_ids: Default::default(),
        grants: vec![ResourceGrant {
            action: SecurityAction::SessionCreate,
            resource_prefix: resource,
            data_policy: None,
        }],
    };
    SecurityRepository::new(&engine.storage, instance())
        .initialize(
            SecurityState {
                format_version: rrd_security::SECURITY_FORMAT,
                revision: 1,
                principals: [(principal_id.clone(), principal)].into_iter().collect(),
                roles: Default::default(),
                identity_bindings: Default::default(),
                jwt_issuers: Default::default(),
            },
            1,
            "security-test",
            "request-security",
            "operation-security",
        )
        .unwrap();

    let lease = engine
        .create_authenticated_session(
            &principal_id,
            b"session-only-key",
            &session_request(5_000, 2),
            &id("create-key"),
            1_000,
            "request-create",
            "operation-create",
        )
        .unwrap();
    let before = engine.storage.control().journal_since(0, 64).unwrap();

    let denied = engine.create_instance_backup(
        &lease.session_id,
        &lease.token,
        &id("backup-key"),
        &rrd_contract::CreateInstanceBackup {
            label: "must-not-exist".into(),
            created_at_unix_ms: 1_100,
        },
        1_100,
        "request-backup",
        "operation-backup",
    );

    assert!(matches!(denied, Err(ServiceError::PermissionDenied)));
    let after = engine.storage.control().journal_since(0, 64).unwrap();
    assert_eq!(
        after.len(),
        before.len() + 2,
        "a denied embedded call must append only its audit record and audit head"
    );
    assert!(after
        .iter()
        .all(|entry| !entry.action.starts_with("backup.")));
    let denied_audit = SecurityRepository::new(&engine.storage, instance())
        .audit_since(0, 64)
        .unwrap()
        .records
        .into_iter()
        .map(|(_, record)| record)
        .find(|record| record.request_id == "request-backup")
        .expect("the direct embedded denial is audited");
    assert_eq!(denied_audit.action, SecurityAction::BackupCreate);
    assert_eq!(denied_audit.phase, AuditPhase::Completed);
    assert_eq!(denied_audit.decision, AuditDecision::Denied);
}

#[test]
pub(super) fn diagnostic_snapshot_requires_its_exact_grant() {
    let (_root, engine) = isolated_engine();
    let principal_id = CanonicalId::new("diagnostic-denied").unwrap();
    let resource = ResourcePath {
        segments: vec![ResourceId::new(ResourceKind::Instance, instance().as_str()).unwrap()],
    };
    SecurityRepository::new(&engine.storage, instance())
        .initialize(
            SecurityState {
                format_version: rrd_security::SECURITY_FORMAT,
                revision: 1,
                principals: [(
                    principal_id.clone(),
                    Principal {
                        id: principal_id.clone(),
                        kind: PrincipalKind::Service,
                        credential_sha256: digest::sha256_hex(b"diagnostic-key"),
                        credential_revision: 1,
                        not_before_unix_ms: 1,
                        expires_at_unix_ms: u64::MAX,
                        disabled: false,
                        role_ids: Default::default(),
                        grants: vec![ResourceGrant {
                            action: SecurityAction::SessionCreate,
                            resource_prefix: resource,
                            data_policy: None,
                        }],
                    },
                )]
                .into_iter()
                .collect(),
                roles: Default::default(),
                identity_bindings: Default::default(),
                jwt_issuers: Default::default(),
            },
            1,
            "security-test",
            "request-security",
            "operation-security",
        )
        .unwrap();
    let lease = engine
        .create_authenticated_session(
            &principal_id,
            b"diagnostic-key",
            &session_request(5_000, 2),
            &id("diagnostic-session"),
            1_000,
            "request-diagnostic-session",
            "operation-diagnostic-session",
        )
        .unwrap();

    let result = engine.read_diagnostic_snapshot(
        &lease.session_id,
        &lease.token,
        &rrd_contract::ReadDiagnosticSnapshot {
            scope: "instance:test-instance".into(),
            graph_valid_at_unix_ms: 1_100,
            graph_known_at_cursor: None,
            graph_compare_cursor: 0,
            graph_max_records: None,
            runtime_max_scanned_changes: 1_024,
            changes_after_cursor: 0,
            change_limit: 8,
            audit_after_sequence: 0,
            audit_limit: 8,
        },
        1_100,
        "request-diagnostic-read",
        "operation-diagnostic-read",
    );

    assert!(matches!(result, Err(ServiceError::PermissionDenied)));
    let storage = engine.read_storage_diagnostics(
        &lease.session_id,
        &lease.token,
        &rrd_contract::ReadStorageDiagnostics {
            scope: "instance:test-instance".into(),
        },
        1_100,
        "storage-denied",
        "storage-denied",
    );
    assert!(matches!(storage, Err(ServiceError::PermissionDenied)));
}

/// Characterizes POAM-029. The diagnostic read compares a before/after stamp
/// that includes the control sequence, and every audited request, reads
/// included, appends to the control journal. One audited read inside each
/// assembly window therefore discards every attempt although no runtime state
/// changes, which is what steady client traffic does to the live read. When
/// POAM-029 is fixed, the observed read must assemble and this test inverts.
#[test]
pub(super) fn an_audited_read_inside_every_assembly_window_exhausts_the_diagnostic_read() {
    let (_root, engine) = isolated_engine();
    let principal_id = CanonicalId::new("diagnostic-reader").unwrap();
    let resource = ResourcePath {
        segments: vec![ResourceId::new(ResourceKind::Instance, instance().as_str()).unwrap()],
    };
    let grants = [
        SecurityAction::SessionCreate,
        SecurityAction::DiagnosticsRead,
        SecurityAction::ChangefeedRead,
    ]
    .into_iter()
    .map(|action| ResourceGrant {
        action,
        resource_prefix: resource.clone(),
        data_policy: None,
    })
    .collect();
    SecurityRepository::new(&engine.storage, instance())
        .initialize(
            SecurityState {
                format_version: rrd_security::SECURITY_FORMAT,
                revision: 1,
                principals: [(
                    principal_id.clone(),
                    Principal {
                        id: principal_id.clone(),
                        kind: PrincipalKind::Service,
                        credential_sha256: digest::sha256_hex(b"diagnostic-key"),
                        credential_revision: 1,
                        not_before_unix_ms: 1,
                        expires_at_unix_ms: u64::MAX,
                        disabled: false,
                        role_ids: Default::default(),
                        grants,
                    },
                )]
                .into_iter()
                .collect(),
                roles: Default::default(),
                identity_bindings: Default::default(),
                jwt_issuers: Default::default(),
            },
            1,
            "security-test",
            "request-security",
            "operation-security",
        )
        .unwrap();
    let lease = engine
        .create_authenticated_session(
            &principal_id,
            b"diagnostic-key",
            &session_request(5_000, 2),
            &id("diagnostic-session"),
            1_000,
            "request-diagnostic-session",
            "operation-diagnostic-session",
        )
        .unwrap();
    let scope = format!("instance:{}", instance().as_str());
    let request = rrd_contract::ReadDiagnosticSnapshot {
        scope: scope.clone(),
        graph_valid_at_unix_ms: 1_100,
        graph_known_at_cursor: None,
        graph_compare_cursor: 0,
        graph_max_records: None,
        runtime_max_scanned_changes: 1_024,
        changes_after_cursor: 0,
        change_limit: 1,
        audit_after_sequence: 0,
        audit_limit: 1,
    };

    // Alone, the read assembles on its first attempt.
    let quiet = engine
        .read_diagnostic_snapshot(
            &lease.session_id,
            &lease.token,
            &request,
            1_100,
            "request-diagnostic-quiet",
            "operation-diagnostic-quiet",
        )
        .unwrap();
    assert_eq!(quiet.read.assembly_attempts, 1);
    let storage = engine
        .read_storage_diagnostics(
            &lease.session_id,
            &lease.token,
            &rrd_contract::ReadStorageDiagnostics {
                scope: scope.clone(),
            },
            1_100,
            "storage-observation",
            "storage-observation",
        )
        .unwrap();
    storage.validate().unwrap();
    assert_eq!(storage.profile, quiet.storage.profile);
    assert!(matches!(
        engine.read_storage_diagnostics(
            &lease.session_id,
            &lease.token,
            &rrd_contract::ReadStorageDiagnostics {
                scope: "instance:another-estate".into()
            },
            1_100,
            "storage-wrong-scope",
            "storage-wrong-scope"
        ),
        Err(ServiceError::WrongScope)
    ));

    let runtime_cursor = engine.storage.runtime().cursor().unwrap();
    let control_sequence = engine.storage.control().sequence().unwrap();
    let mut attempts = Vec::new();
    let busy = engine.read_diagnostic_snapshot_during_assembly(
        &lease.session_id,
        &lease.token,
        &request,
        1_200,
        "request-diagnostic-busy",
        "operation-diagnostic-busy",
        &mut |attempt| {
            attempts.push(attempt);
            engine
                .read_storage_diagnostics(
                    &lease.session_id,
                    &lease.token,
                    &rrd_contract::ReadStorageDiagnostics {
                        scope: scope.clone(),
                    },
                    1_200,
                    &format!("storage-during-{attempt}"),
                    &format!("storage-during-{attempt}"),
                )?
                .validate()
                .unwrap();

            engine
                .read_changefeed(
                    &lease.session_id,
                    &lease.token,
                    &rrd_contract::ReadChangefeed {
                        scope: scope.clone(),
                        after_cursor: 0,
                        limit: 1,
                    },
                    1_200,
                    &format!("request-concurrent-read-{attempt}"),
                    &format!("operation-concurrent-read-{attempt}"),
                )
                .map(|_| ())
        },
    );

    assert!(
        matches!(busy, Err(ServiceError::StorageConflict(_))),
        "POAM-029: an audited read in every window must still exhaust the diagnostic read until it is fixed; got {busy:?}"
    );
    assert_eq!(attempts, [1, 2, 3]);
    assert_eq!(
        engine.storage.runtime().cursor().unwrap(),
        runtime_cursor,
        "only reads ran, so no runtime state changed"
    );
    assert!(engine.storage.control().sequence().unwrap() > control_sequence);
}

#[test]
pub(super) fn engine_invocation_owns_authorization_and_completion_audit() {
    let (_root, engine) = isolated_engine();
    let principal_id = CanonicalId::new("query-reader").unwrap();
    let resource = ResourcePath {
        segments: vec![ResourceId::new(ResourceKind::Instance, instance().as_str()).unwrap()],
    };
    let principal = Principal {
        id: principal_id.clone(),
        kind: PrincipalKind::Service,
        credential_sha256: digest::sha256_hex(b"query-key"),
        credential_revision: 1,
        not_before_unix_ms: 1,
        expires_at_unix_ms: u64::MAX,
        disabled: false,
        role_ids: Default::default(),
        grants: vec![
            ResourceGrant {
                action: SecurityAction::SessionCreate,
                resource_prefix: resource.clone(),
                data_policy: None,
            },
            ResourceGrant {
                action: SecurityAction::QueryExecute,
                resource_prefix: resource.clone(),
                data_policy: None,
            },
        ],
    };
    let repository = SecurityRepository::new(&engine.storage, instance());
    repository
        .initialize(
            SecurityState {
                format_version: rrd_security::SECURITY_FORMAT,
                revision: 1,
                principals: [(principal_id.clone(), principal)].into_iter().collect(),
                roles: Default::default(),
                identity_bindings: Default::default(),
                jwt_issuers: Default::default(),
            },
            1,
            "security-test",
            "request-security",
            "operation-security",
        )
        .unwrap();
    let lease = engine
        .create_authenticated_session(
            &principal_id,
            b"query-key",
            &session_request(5_000, 2),
            &id("create-query-session"),
            1_000,
            "request-create-query-session",
            "operation-create-query-session",
        )
        .unwrap();
    let invocation = Invocation {
        context: RequestContext {
            request_id: id("request-query"),
            operation_id: id("operation-query"),
            idempotency_key: None,
            deadline_unix_ms: Some(2_000),
        },
        resource,
        observed_at_unix_ms: 1_100,
        attempt: 1,
        request_sha256: digest::sha256_hex(b"query-request"),
    };
    let authorized = engine
        .begin_invocation(
            invocation,
            RrdOperation::QueryExecute,
            InvocationCredential::Session {
                session_id: &lease.session_id,
                token: &lease.token,
            },
        )
        .unwrap();
    engine
        .complete_invocation(
            &authorized,
            InvocationCompletion {
                decision: AuditDecision::Allowed,
                status_code: 200,
                response_sha256: digest::sha256_hex(b"query-response"),
            },
        )
        .unwrap();

    let page = repository.audit_since(0, 16).unwrap();
    let records = page
        .records
        .iter()
        .filter(|(_, record)| record.action == SecurityAction::QueryExecute)
        .map(|(_, record)| record)
        .collect::<Vec<_>>();
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].phase, AuditPhase::Authorized);
    assert_eq!(records[1].phase, AuditPhase::Completed);
    assert_eq!(records[1].decision, AuditDecision::Allowed);
    assert_eq!(records[1].principal_id, Some(principal_id));
    assert_eq!(
        records[1].previous_audit_sha256.as_deref(),
        Some(records[0].audit_sha256.as_str())
    );
}

#[test]
pub(super) fn denied_engine_invocation_is_audited_without_domain_mutation() {
    let (_root, engine) = isolated_engine();
    let principal_id = CanonicalId::new("session-only").unwrap();
    let resource = ResourcePath {
        segments: vec![ResourceId::new(ResourceKind::Instance, instance().as_str()).unwrap()],
    };
    let principal = Principal {
        id: principal_id.clone(),
        kind: PrincipalKind::Service,
        credential_sha256: digest::sha256_hex(b"session-key"),
        credential_revision: 1,
        not_before_unix_ms: 1,
        expires_at_unix_ms: u64::MAX,
        disabled: false,
        role_ids: Default::default(),
        grants: vec![ResourceGrant {
            action: SecurityAction::SessionCreate,
            resource_prefix: resource.clone(),
            data_policy: None,
        }],
    };
    let repository = SecurityRepository::new(&engine.storage, instance());
    repository
        .initialize(
            SecurityState {
                format_version: rrd_security::SECURITY_FORMAT,
                revision: 1,
                principals: [(principal_id.clone(), principal)].into_iter().collect(),
                roles: Default::default(),
                identity_bindings: Default::default(),
                jwt_issuers: Default::default(),
            },
            1,
            "security-test",
            "request-security",
            "operation-security",
        )
        .unwrap();
    let lease = engine
        .create_authenticated_session(
            &principal_id,
            b"session-key",
            &session_request(5_000, 2),
            &id("create-session-only"),
            1_000,
            "request-create-session-only",
            "operation-create-session-only",
        )
        .unwrap();
    let before_runtime = engine.storage.runtime().cursor().unwrap();
    let denied = engine.begin_invocation(
        Invocation {
            context: mutation_context(
                &id("backup-idempotency"),
                "request-denied-backup",
                "operation-denied-backup",
            ),
            resource,
            observed_at_unix_ms: 1_100,
            attempt: 2,
            request_sha256: digest::sha256_hex(b"backup-request"),
        },
        RrdOperation::BackupCreate,
        InvocationCredential::Session {
            session_id: &lease.session_id,
            token: &lease.token,
        },
    );
    assert!(matches!(denied, Err(ServiceError::PermissionDenied)));
    assert_eq!(engine.storage.runtime().cursor().unwrap(), before_runtime);

    let page = repository.audit_since(0, 16).unwrap();
    let denied = page
        .records
        .iter()
        .find(|(_, record)| record.action == SecurityAction::BackupCreate)
        .map(|(_, record)| record)
        .unwrap();
    assert_eq!(denied.phase, AuditPhase::Completed);
    assert_eq!(denied.decision, AuditDecision::Denied);
}
