//! Validation and canonical naming of identities, links and values.

use super::*;

pub(super) fn validate_hex_identity(kind: &'static str, value: &str, width: usize) -> Result<()> {
    if value.len() != width
        || value
            .bytes()
            .any(|byte| !byte.is_ascii_hexdigit() || byte.is_ascii_uppercase())
        || value.bytes().all(|byte| byte == b'0')
    {
        return invalid(format!(
            "{kind} must be {width} lowercase hexadecimal characters and not all zero"
        ));
    }
    Ok(())
}

pub(super) fn validate_link(link: &TraceLink) -> Result<()> {
    match link {
        TraceLink::Request {
            request_id,
            operation_id,
            parent_request_id,
            idempotency_digest,
        } => {
            validate_bounded_text("trace request id", request_id, MAX_TRACE_STRING_BYTES)?;
            validate_bounded_text(
                "trace request operation",
                operation_id,
                MAX_TRACE_NAME_BYTES,
            )?;
            if let Some(parent_request_id) = parent_request_id {
                validate_bounded_text(
                    "trace parent request id",
                    parent_request_id,
                    MAX_TRACE_STRING_BYTES,
                )?;
            }
            if let Some(idempotency_digest) = idempotency_digest {
                validate_digest("trace idempotency", idempotency_digest)?;
            }
            Ok(())
        }
        TraceLink::ActorScope { actor, scope } => {
            validate_bounded_text("trace actor", actor, MAX_TRACE_STRING_BYTES)?;
            validate_bounded_text("trace scope", scope.as_str(), MAX_TRACE_STRING_BYTES)
        }
        TraceLink::Authorization {
            policy_revision,
            authorization_digest,
            ..
        } => {
            if *policy_revision == 0 {
                return invalid("trace authorization policy revision must be nonzero");
            }
            validate_digest("trace authorization", authorization_digest)
        }
        TraceLink::ReasoningCursor { cursor } => cursor.validate(),
        TraceLink::RuntimeCursor { .. } => Ok(()),
        TraceLink::Read { stamp } => stamp.validate(),
        TraceLink::Snapshot { snapshot_id, .. } => validate_bounded_text(
            "trace snapshot id",
            snapshot_id.as_str(),
            MAX_TRACE_STRING_BYTES,
        ),
        TraceLink::Plan { plan_digest } => validate_digest("trace plan", plan_digest),
        TraceLink::Projection { stamp } => stamp.validate(),
        TraceLink::Source {
            source_kind,
            source_id,
            source_revision,
        } => {
            validate_bounded_text("trace source kind", source_kind, MAX_TRACE_NAME_BYTES)?;
            validate_bounded_text("trace source id", source_id, MAX_TRACE_STRING_BYTES)?;
            validate_bounded_text(
                "trace source revision",
                source_revision,
                MAX_TRACE_STRING_BYTES,
            )
        }
        TraceLink::Commit {
            commit_id,
            first_cursor,
            last_cursor,
        } => {
            validate_digest("trace commit", commit_id)?;
            if *first_cursor == 0 || last_cursor < first_cursor {
                return invalid(
                    "trace commit cursors must be nonzero and ordered first through last",
                );
            }
            Ok(())
        }
        TraceLink::Resource {
            resource_kind,
            resource_id,
        } => {
            validate_bounded_text("trace resource kind", resource_kind, MAX_TRACE_NAME_BYTES)?;
            validate_bounded_text("trace resource id", resource_id, MAX_TRACE_STRING_BYTES)
        }
        TraceLink::CausalSpan { .. } => Ok(()),
        TraceLink::Output { output_digest, .. } => validate_digest("trace output", output_digest),
    }
}

pub(super) fn validate_value(value: &RuntimeValue, depth: usize, nodes: &mut usize) -> Result<()> {
    *nodes = nodes.saturating_add(1);
    if *nodes > MAX_TRACE_VALUE_NODES || depth > MAX_TRACE_VALUE_DEPTH {
        return invalid("runtime trace attribute tree exceeds its structural budget");
    }
    match value {
        RuntimeValue::String(value)
        | RuntimeValue::Digest(value)
        | RuntimeValue::Decimal(value) => {
            if value.len() > MAX_TRACE_STRING_BYTES {
                return invalid(format!(
                    "runtime trace string exceeds {MAX_TRACE_STRING_BYTES} bytes"
                ));
            }
        }
        RuntimeValue::List(values) => {
            for value in values {
                validate_value(value, depth + 1, nodes)?;
            }
        }
        RuntimeValue::Map(values) => {
            for (name, value) in values {
                validate_bounded_text("trace map key", name, MAX_TRACE_NAME_BYTES)?;
                validate_value(value, depth + 1, nodes)?;
            }
        }
        RuntimeValue::Null
        | RuntimeValue::Bool(_)
        | RuntimeValue::Integer(_)
        | RuntimeValue::Unsigned(_) => {}
    }
    Ok(())
}

pub(super) fn link_value(link: &TraceLink) -> RuntimeValue {
    let mut value = BTreeMap::new();
    match link {
        TraceLink::Request {
            request_id,
            operation_id,
            parent_request_id,
            idempotency_digest,
        } => {
            value.insert("kind".into(), RuntimeValue::String("request".into()));
            value.insert(
                "request_id".into(),
                RuntimeValue::String(request_id.clone()),
            );
            value.insert(
                "operation_id".into(),
                RuntimeValue::String(operation_id.clone()),
            );
            if let Some(parent_request_id) = parent_request_id {
                value.insert(
                    "parent_request_id".into(),
                    RuntimeValue::String(parent_request_id.clone()),
                );
            }
            if let Some(idempotency_digest) = idempotency_digest {
                value.insert(
                    "idempotency_digest".into(),
                    RuntimeValue::Digest(idempotency_digest.clone()),
                );
            }
        }
        TraceLink::ActorScope { actor, scope } => {
            value.insert("kind".into(), RuntimeValue::String("actor_scope".into()));
            value.insert("actor".into(), RuntimeValue::String(actor.clone()));
            value.insert("scope".into(), RuntimeValue::String(scope.to_string()));
        }
        TraceLink::Authorization {
            decision,
            policy_revision,
            authorization_digest,
        } => {
            value.insert("kind".into(), RuntimeValue::String("authorization".into()));
            value.insert(
                "decision".into(),
                RuntimeValue::String(audit_decision_name(*decision).into()),
            );
            value.insert(
                "policy_revision".into(),
                RuntimeValue::Unsigned(*policy_revision),
            );
            value.insert(
                "authorization_digest".into(),
                RuntimeValue::Digest(authorization_digest.clone()),
            );
        }
        TraceLink::ReasoningCursor { cursor } => {
            value.insert(
                "kind".into(),
                RuntimeValue::String("reasoning_cursor".into()),
            );
            value.insert(
                "cursor_id".into(),
                RuntimeValue::String(cursor.id.to_string()),
            );
            value.insert(
                "tree_id".into(),
                RuntimeValue::String(cursor.tree_id.to_string()),
            );
            value.insert(
                "tree_revision".into(),
                RuntimeValue::Unsigned(cursor.tree_revision),
            );
            value.insert(
                "node_id".into(),
                RuntimeValue::String(cursor.node_id.to_string()),
            );
            value.insert("step".into(), RuntimeValue::Unsigned(cursor.step));
            value.insert(
                "read_manifest_sha256".into(),
                RuntimeValue::Digest(cursor.read.manifest_id.clone()),
            );
        }
        TraceLink::RuntimeCursor { cursor } => {
            value.insert("kind".into(), RuntimeValue::String("runtime_cursor".into()));
            value.insert("cursor".into(), RuntimeValue::Unsigned(*cursor));
        }
        TraceLink::Read { stamp } => {
            value.insert("kind".into(), RuntimeValue::String("read".into()));
            value.insert(
                "contract_version".into(),
                RuntimeValue::Unsigned(u64::from(stamp.contract_version)),
            );
            value.insert(
                "scope".into(),
                RuntimeValue::String(stamp.scope.to_string()),
            );
            if let Some(schema_revision) = stamp.schema_revision {
                value.insert(
                    "schema_revision".into(),
                    RuntimeValue::Unsigned(schema_revision),
                );
            }
            value.insert(
                "catalog_revision".into(),
                RuntimeValue::Unsigned(stamp.catalog_revision),
            );
            value.insert(
                "commit_cursor".into(),
                RuntimeValue::Unsigned(stamp.commit_cursor),
            );
            if let Some(head_digest) = &stamp.head_digest {
                value.insert(
                    "head_digest".into(),
                    RuntimeValue::Digest(head_digest.clone()),
                );
            }
            value.insert(
                "manifest_id".into(),
                RuntimeValue::Digest(stamp.manifest_id.clone()),
            );
        }
        TraceLink::Snapshot {
            snapshot_id,
            cursor,
        } => {
            value.insert("kind".into(), RuntimeValue::String("snapshot".into()));
            value.insert(
                "snapshot_id".into(),
                RuntimeValue::String(snapshot_id.to_string()),
            );
            value.insert("cursor".into(), RuntimeValue::Unsigned(*cursor));
        }
        TraceLink::Plan { plan_digest } => {
            value.insert("kind".into(), RuntimeValue::String("plan".into()));
            value.insert(
                "plan_digest".into(),
                RuntimeValue::Digest(plan_digest.clone()),
            );
        }
        TraceLink::Projection { stamp } => {
            value.insert("kind".into(), RuntimeValue::String("projection".into()));
            value.insert(
                "contract_version".into(),
                RuntimeValue::Unsigned(u64::from(stamp.contract_version)),
            );
            value.insert("id".into(), RuntimeValue::String(stamp.id.to_string()));
            value.insert(
                "generation".into(),
                RuntimeValue::Unsigned(stamp.generation),
            );
            value.insert(
                "source_cursor".into(),
                RuntimeValue::Unsigned(stamp.source_cursor),
            );
            value.insert(
                "config_digest".into(),
                RuntimeValue::Digest(stamp.config_digest.clone()),
            );
            value.insert(
                "artifact_digest".into(),
                RuntimeValue::Digest(stamp.artifact_digest.clone()),
            );
            value.insert(
                "state".into(),
                RuntimeValue::String(projection_state_name(stamp.state).into()),
            );
        }
        TraceLink::Source {
            source_kind,
            source_id,
            source_revision,
        } => {
            value.insert("kind".into(), RuntimeValue::String("source".into()));
            value.insert(
                "source_kind".into(),
                RuntimeValue::String(source_kind.clone()),
            );
            value.insert("source_id".into(), RuntimeValue::String(source_id.clone()));
            value.insert(
                "source_revision".into(),
                RuntimeValue::String(source_revision.clone()),
            );
        }
        TraceLink::Commit {
            commit_id,
            first_cursor,
            last_cursor,
        } => {
            value.insert("kind".into(), RuntimeValue::String("commit".into()));
            value.insert("commit_id".into(), RuntimeValue::Digest(commit_id.clone()));
            value.insert("first_cursor".into(), RuntimeValue::Unsigned(*first_cursor));
            value.insert("last_cursor".into(), RuntimeValue::Unsigned(*last_cursor));
        }
        TraceLink::Resource {
            resource_kind,
            resource_id,
        } => {
            value.insert("kind".into(), RuntimeValue::String("resource".into()));
            value.insert(
                "resource_kind".into(),
                RuntimeValue::String(resource_kind.clone()),
            );
            value.insert(
                "resource_id".into(),
                RuntimeValue::String(resource_id.clone()),
            );
        }
        TraceLink::CausalSpan {
            trace_id,
            span_id,
            relation,
        } => {
            value.insert("kind".into(), RuntimeValue::String("causal_span".into()));
            value.insert(
                "trace_id".into(),
                RuntimeValue::String(trace_id.to_string()),
            );
            value.insert("span_id".into(), RuntimeValue::String(span_id.to_string()));
            value.insert(
                "relation".into(),
                RuntimeValue::String(causal_relation_name(*relation).into()),
            );
        }
        TraceLink::Output {
            output_digest,
            item_count,
            byte_count,
        } => {
            value.insert("kind".into(), RuntimeValue::String("output".into()));
            value.insert(
                "output_digest".into(),
                RuntimeValue::Digest(output_digest.clone()),
            );
            value.insert("item_count".into(), RuntimeValue::Unsigned(*item_count));
            value.insert("byte_count".into(), RuntimeValue::Unsigned(*byte_count));
        }
    }
    RuntimeValue::Map(value)
}

pub(super) fn enum_name(value: TracePhase) -> &'static str {
    match value {
        TracePhase::Start => "start",
        TracePhase::Annotation => "annotation",
        TracePhase::Finish => "finish",
    }
}

pub(super) fn outcome_name(value: TraceOutcome) -> &'static str {
    match value {
        TraceOutcome::Running => "running",
        TraceOutcome::Ok => "ok",
        TraceOutcome::Error => "error",
        TraceOutcome::Denied => "denied",
        TraceOutcome::Cancelled => "cancelled",
    }
}

pub(super) fn data_class_name(value: TraceDataClass) -> &'static str {
    match value {
        TraceDataClass::Control => "control",
        TraceDataClass::Operator => "operator",
        TraceDataClass::Content => "content",
    }
}

pub(super) fn audit_decision_name(value: AuditDecision) -> &'static str {
    match value {
        AuditDecision::Allow => "allow",
        AuditDecision::Deny => "deny",
    }
}

pub(super) fn causal_relation_name(value: TraceCausalRelation) -> &'static str {
    match value {
        TraceCausalRelation::FollowsFrom => "follows_from",
        TraceCausalRelation::RetryOf => "retry_of",
        TraceCausalRelation::Resumes => "resumes",
    }
}

pub(super) fn projection_state_name(value: crate::ProjectionState) -> &'static str {
    match value {
        crate::ProjectionState::Building => "building",
        crate::ProjectionState::Ready => "ready",
        crate::ProjectionState::Quarantined => "quarantined",
        crate::ProjectionState::Retiring => "retiring",
    }
}

pub(super) fn validate_digest(kind: &'static str, value: &str) -> Result<()> {
    if value.len() != 64 || value.bytes().any(|byte| !byte.is_ascii_hexdigit()) {
        return invalid(format!("{kind} digest must be 64 hexadecimal characters"));
    }
    Ok(())
}
