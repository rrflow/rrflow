use super::*;
use crate::ServiceErrorKind;
use rrd_contract::{
    AssembleContext, DataLogicalModel, DataPropertySchema, DataRecordSchema, DataReference,
    DataSchemaMode, DataSchemaRegistry, DataTableSchema, DataValueType, ExecuteQuery, QueryBudget,
    QueryValue,
};
use rrd_core::RuntimeValue;
use rrd_security::{
    DataPolicy, IdentityBinding, JwtIssueRequest, JwtIssuer, PolicyPredicate, Role,
};
use std::collections::{BTreeMap, BTreeSet};

fn overlapping_backup_invocation(now: u64, attempt: u64) -> Invocation {
    Invocation {
        context: mutation_context(
            &id("overlap-backup-key"),
            "overlap-backup-request",
            "overlap-backup-operation",
        ),
        resource: ResourcePath {
            segments: vec![ResourceId::new(ResourceKind::Instance, instance().as_str()).unwrap()],
        },
        observed_at_unix_ms: now,
        attempt,
        request_sha256: digest::sha256_hex(b"same-backup-request"),
    }
}

fn successful_invocation_completion() -> InvocationCompletion {
    InvocationCompletion {
        decision: AuditDecision::Allowed,
        status_code: 200,
        response_sha256: digest::sha256_hex(b"same-backup-result"),
    }
}

mod invocation;
mod policy;
