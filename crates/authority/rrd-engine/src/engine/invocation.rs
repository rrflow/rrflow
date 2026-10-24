use super::security::AuditEvent;
use super::transaction_admission::InvocationAdmission;
use super::*;
use std::sync::Arc;

/// Canonical operation identity presented by every embedded or transported
/// RRD call. Adapters select an operation; only the engine maps it to policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RrdOperation {
    ServiceInspect,
    UnknownRequest,
    SessionCreate,
    SessionRenew,
    SessionClose,
    TransactionBegin,
    TransactionPreview,
    TransactionCommit,
    TransactionAbort,
    EstateRead,
    EstateAdmin,
    EndpointConfigurationPreview,
    QueryExecute,
    QueryLivePoll,
    QueryIndexEnsure,
    QueryIndexList,
    BackupCreate,
    BackupList,
    RestoreCreate,
    VectorSearch,
    VectorCollectionEnsure,
    VectorCollectionList,
    VectorCollectionDelete,
    VectorPayloadIndexEnsure,
    VectorPayloadIndexList,
    VectorPayloadIndexDelete,
    VectorPointScroll,
    VectorPointRetrieve,
    ChangefeedRead,
    ChangefeedFollow,
    WebSocketConnect,
    SubscriptionOpen,
    SubscriptionConnect,
    SubscriptionAck,
    SubscriptionClose,
    AuditRead,
    AuditExport,
    FunctionCatalogueRead,
    FunctionCatalogueWrite,
    FunctionExecute,
    DiagnosticsRead,
    FlightLaunch,
    FlightRead,
    TaskWrite,
    TaskRead,
    MemoryContextRead,
    ReasoningRead,
    ReasoningWrite,
}

impl RrdOperation {
    pub fn mutates(self) -> bool {
        matches!(
            self,
            Self::SessionCreate
                | Self::SessionRenew
                | Self::SessionClose
                | Self::TransactionBegin
                | Self::TransactionPreview
                | Self::TransactionCommit
                | Self::TransactionAbort
                | Self::EstateAdmin
                | Self::SubscriptionOpen
                | Self::SubscriptionConnect
                | Self::SubscriptionAck
                | Self::SubscriptionClose
                | Self::QueryIndexEnsure
                | Self::BackupCreate
                | Self::RestoreCreate
                | Self::FunctionCatalogueWrite
                | Self::VectorCollectionEnsure
                | Self::VectorCollectionDelete
                | Self::VectorPayloadIndexEnsure
                | Self::VectorPayloadIndexDelete
                | Self::FlightLaunch
                | Self::TaskWrite
                | Self::ReasoningWrite
        )
    }

    pub(crate) fn action(self) -> SecurityAction {
        match self {
            Self::ServiceInspect => SecurityAction::ServiceInspect,
            Self::UnknownRequest => SecurityAction::UnknownRequest,
            Self::SessionCreate => SecurityAction::SessionCreate,
            Self::SessionRenew => SecurityAction::SessionRenew,
            Self::SessionClose => SecurityAction::SessionClose,
            Self::TransactionBegin => SecurityAction::TransactionBegin,
            Self::TransactionPreview => SecurityAction::TransactionPreview,
            Self::TransactionCommit => SecurityAction::TransactionCommit,
            Self::TransactionAbort => SecurityAction::TransactionAbort,
            Self::EstateRead => SecurityAction::EstateRead,
            Self::EstateAdmin | Self::EndpointConfigurationPreview => SecurityAction::EstateAdmin,
            Self::QueryExecute => SecurityAction::QueryExecute,
            Self::QueryLivePoll => SecurityAction::QueryLivePoll,
            Self::QueryIndexEnsure => SecurityAction::QueryIndexEnsure,
            Self::QueryIndexList => SecurityAction::QueryIndexList,
            Self::BackupCreate => SecurityAction::BackupCreate,
            Self::BackupList => SecurityAction::BackupList,
            Self::RestoreCreate => SecurityAction::RestoreCreate,
            Self::VectorSearch => SecurityAction::VectorSearch,
            Self::VectorCollectionEnsure => SecurityAction::VectorCollectionEnsure,
            Self::VectorCollectionList => SecurityAction::VectorCollectionList,
            Self::VectorCollectionDelete => SecurityAction::VectorCollectionDelete,
            Self::VectorPayloadIndexEnsure => SecurityAction::VectorPayloadIndexEnsure,
            Self::VectorPayloadIndexList => SecurityAction::VectorPayloadIndexList,
            Self::VectorPayloadIndexDelete => SecurityAction::VectorPayloadIndexDelete,
            Self::VectorPointScroll => SecurityAction::VectorPointScroll,
            Self::VectorPointRetrieve => SecurityAction::VectorPointRetrieve,
            Self::ChangefeedRead => SecurityAction::ChangefeedRead,
            Self::ChangefeedFollow => SecurityAction::ChangefeedFollow,
            Self::WebSocketConnect => SecurityAction::WebSocketConnect,
            Self::SubscriptionOpen => SecurityAction::SubscriptionOpen,
            Self::SubscriptionConnect => SecurityAction::SubscriptionConnect,
            Self::SubscriptionAck => SecurityAction::SubscriptionAck,
            Self::SubscriptionClose => SecurityAction::SubscriptionClose,
            Self::AuditRead => SecurityAction::AuditRead,
            Self::AuditExport => SecurityAction::AuditExport,
            Self::FunctionCatalogueRead => SecurityAction::FunctionCatalogueRead,
            Self::FunctionCatalogueWrite => SecurityAction::FunctionCatalogueWrite,
            Self::FunctionExecute => SecurityAction::FunctionExecute,
            Self::DiagnosticsRead => SecurityAction::DiagnosticsRead,
            Self::FlightLaunch => SecurityAction::FlightLaunch,
            Self::FlightRead => SecurityAction::FlightRead,
            Self::TaskWrite => SecurityAction::TaskWrite,
            Self::TaskRead => SecurityAction::TaskRead,
            Self::MemoryContextRead => SecurityAction::MemoryContextRead,
            Self::ReasoningRead => SecurityAction::ReasoningRead,
            Self::ReasoningWrite => SecurityAction::ReasoningWrite,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invocation {
    pub context: RequestContext,
    pub resource: ResourcePath,
    pub observed_at_unix_ms: u64,
    pub attempt: u64,
    pub request_sha256: String,
}

#[derive(Clone, Copy)]
pub enum InvocationCredential<'a> {
    Anonymous,
    ApiKey {
        principal_id: &'a CanonicalId,
        credential: &'a [u8],
    },
    Jwt {
        token: &'a str,
        signing_key: &'a [u8],
    },
    Session {
        session_id: &'a CorrelationId,
        token: &'a CorrelationId,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorizedInvocation {
    invocation: Invocation,
    operation: RrdOperation,
    principal_id: Option<CanonicalId>,
    // Pointer identity distinguishes admissions even when callers reuse every
    // public coordinate. Cloning the handle preserves its completion authority.
    reservation: Arc<InvocationAdmission>,
}

pub(in crate::engine) struct ActiveInvocation {
    owner: std::thread::ThreadId,
    reservation: Arc<InvocationAdmission>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvocationCompletion {
    pub decision: AuditDecision,
    pub status_code: u16,
    pub response_sha256: String,
}

impl AuthorizedInvocation {
    /// Deny this invocation while waiting to acquire the transaction gate.
    /// Already admitted effects must still be reconciled; this is not rollback.
    pub fn cancel_pending(&self) {
        self.reservation.cancel();
    }
}

impl RrdEngine {
    pub fn begin_invocation(
        &self,
        invocation: Invocation,
        operation: RrdOperation,
        credential: InvocationCredential<'_>,
    ) -> Result<AuthorizedInvocation> {
        let admission_started = std::time::Instant::now();
        self.validate_invocation(&invocation, operation)?;
        let repository =
            rrd_security::SecurityRepository::new(&self.storage, self.instance.clone());
        let security_enforced = repository.is_initialized()?;
        let mut authenticated_principal = None;
        let authorization = match credential {
            InvocationCredential::Anonymous if security_enforced => {
                Err(ServiceError::Unauthenticated)
            }
            InvocationCredential::Anonymous => Ok(None),
            InvocationCredential::ApiKey {
                principal_id,
                credential,
            } if security_enforced => (|| {
                self.authenticate_principal(principal_id, credential)?;
                authenticated_principal = Some(principal_id.clone());
                self.authorize_principal(
                    principal_id,
                    operation.action(),
                    &invocation.resource,
                    invocation.observed_at_unix_ms,
                )?;
                Ok(Some(principal_id.clone()))
            })(),
            InvocationCredential::ApiKey { .. } => Ok(None),
            InvocationCredential::Jwt { token, signing_key } if security_enforced => (|| {
                let principal_id = repository.authenticate_jwt(
                    token,
                    signing_key,
                    invocation.observed_at_unix_ms,
                )?;
                authenticated_principal = Some(principal_id.clone());
                self.authorize_principal(
                    &principal_id,
                    operation.action(),
                    &invocation.resource,
                    invocation.observed_at_unix_ms,
                )?;
                Ok(Some(principal_id))
            })(
            ),
            InvocationCredential::Jwt { .. } => Err(ServiceError::Unauthenticated),
            InvocationCredential::Session { session_id, token } => (|| {
                let (principal, credential_revision) = self.invocation_session_principal(
                    session_id,
                    token,
                    operation,
                    &invocation.context,
                    invocation.observed_at_unix_ms,
                )?;
                authenticated_principal = principal.clone();
                if security_enforced {
                    let principal_id = principal.as_ref().ok_or(ServiceError::Unauthenticated)?;
                    let authorization = self.authorize_principal(
                        principal_id,
                        operation.action(),
                        &invocation.resource,
                        invocation.observed_at_unix_ms,
                    )?;
                    if credential_revision != Some(authorization.credential_revision) {
                        return Err(ServiceError::Unauthenticated);
                    }
                }
                Ok(principal)
            })(),
        };

        let principal_id = match authorization {
            Ok(principal_id) => principal_id,
            Err(error) => {
                let (decision, status_code) = audit_failure(&error);
                self.append_invocation_audit(
                    &invocation,
                    operation,
                    authenticated_principal,
                    AuditPhase::Completed,
                    decision,
                    status_code,
                    digest::sha256_hex(error.to_string().as_bytes()),
                )?;
                return Err(error);
            }
        };

        self.append_invocation_audit(
            &invocation,
            operation,
            principal_id.clone(),
            AuditPhase::Authorized,
            AuditDecision::Allowed,
            100,
            digest::sha256_hex(b"rrd-audit-completion-pending"),
        )?;
        let coordinates = invocation_coordinates(&invocation);
        let mut active = self
            .active_invocations
            .lock()
            .expect("active invocation mutex");
        let std::collections::btree_map::Entry::Vacant(entry) = active.entry(coordinates) else {
            drop(active);
            let error = ServiceError::InvocationInProgress;
            self.append_invocation_audit(
                &invocation,
                operation,
                principal_id,
                AuditPhase::Completed,
                AuditDecision::Failed,
                409,
                digest::sha256_hex(error.to_string().as_bytes()),
            )?;
            return Err(error);
        };
        let reservation = Arc::new(InvocationAdmission::new(
            admission_started,
            invocation
                .context
                .deadline_unix_ms
                .map(|deadline| deadline.saturating_sub(invocation.observed_at_unix_ms)),
        ));
        entry.insert(ActiveInvocation {
            owner: std::thread::current().id(),
            reservation: Arc::clone(&reservation),
        });
        Ok(AuthorizedInvocation {
            invocation,
            operation,
            principal_id,
            reservation,
        })
    }

    pub fn complete_invocation(
        &self,
        authorized: &AuthorizedInvocation,
        completion: InvocationCompletion,
    ) -> Result<()> {
        validate_sha256(&completion.response_sha256)?;
        let coordinates = invocation_coordinates(&authorized.invocation);
        let mut active = self
            .active_invocations
            .lock()
            .expect("active invocation mutex");
        if !active
            .get(&coordinates)
            .is_some_and(|entry| Arc::ptr_eq(&entry.reservation, &authorized.reservation))
        {
            return Err(ServiceError::IdempotencyConflict);
        }
        self.append_invocation_audit(
            &authorized.invocation,
            authorized.operation,
            authorized.principal_id.clone(),
            AuditPhase::Completed,
            completion.decision,
            completion.status_code,
            completion.response_sha256,
        )?;
        active.remove(&coordinates);
        Ok(())
    }

    /// Records a request rejected before authorization could complete, such as
    /// a malformed transport envelope or missing credential. The operation
    /// still determines the policy/audit action; the adapter cannot supply one.
    pub fn record_invocation_failure(
        &self,
        invocation: Invocation,
        operation: RrdOperation,
        principal_id: Option<CanonicalId>,
        completion: InvocationCompletion,
    ) -> Result<()> {
        self.validate_invocation_common(&invocation)?;
        if completion.decision == AuditDecision::Allowed {
            return Err(ServiceError::Contract(
                "a rejected invocation cannot have an allowed decision".into(),
            ));
        }
        validate_sha256(&completion.response_sha256)?;
        self.append_invocation_audit(
            &invocation,
            operation,
            principal_id,
            AuditPhase::Completed,
            completion.decision,
            completion.status_code,
            completion.response_sha256,
        )
    }

    /// Records an intentionally anonymous public endpoint, such as liveness
    /// or capability inspection. These endpoints never receive application
    /// mutation authority.
    pub fn record_public_invocation(
        &self,
        invocation: Invocation,
        operation: RrdOperation,
        completion: InvocationCompletion,
    ) -> Result<()> {
        if operation.mutates() {
            return Err(ServiceError::Contract(
                "a mutating operation cannot use the public invocation path".into(),
            ));
        }
        self.validate_invocation_common(&invocation)?;
        validate_sha256(&completion.response_sha256)?;
        self.append_invocation_audit(
            &invocation,
            operation,
            None,
            AuditPhase::Completed,
            completion.decision,
            completion.status_code,
            completion.response_sha256,
        )
    }

    fn validate_invocation(&self, invocation: &Invocation, operation: RrdOperation) -> Result<()> {
        self.validate_invocation_common(invocation)?;
        invocation
            .context
            .validate(operation.mutates())
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        Ok(())
    }

    fn validate_invocation_common(&self, invocation: &Invocation) -> Result<()> {
        invocation
            .context
            .validate(false)
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        invocation
            .resource
            .validate()
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        let target = invocation
            .resource
            .segments
            .iter()
            .find(|segment| segment.kind == ResourceKind::Instance);
        if target.is_none_or(|target| target.id != self.instance) {
            return Err(ServiceError::WrongScope);
        }
        if invocation
            .context
            .deadline_unix_ms
            .is_some_and(|deadline| deadline <= invocation.observed_at_unix_ms)
        {
            return Err(ServiceError::DeadlineExceeded);
        }
        validate_sha256(&invocation.request_sha256)
    }

    #[allow(clippy::too_many_arguments)]
    fn append_invocation_audit(
        &self,
        invocation: &Invocation,
        operation: RrdOperation,
        principal_id: Option<CanonicalId>,
        phase: AuditPhase,
        decision: AuditDecision,
        status_code: u16,
        response_sha256: String,
    ) -> Result<()> {
        self.append_audit(AuditEvent {
            at_unix_ms: invocation.observed_at_unix_ms,
            attempt: invocation.attempt,
            principal_id,
            action: operation.action(),
            resource: invocation.resource.clone(),
            request_id: invocation.context.request_id.as_str().into(),
            operation_id: invocation.context.operation_id.as_str().into(),
            phase,
            decision,
            status_code,
            request_sha256: invocation.request_sha256.clone(),
            response_sha256,
        })
    }

    pub(super) fn invocation_admission(
        &self,
        request_id: &str,
        operation_id: &str,
    ) -> Result<Option<Arc<InvocationAdmission>>> {
        let active = self
            .active_invocations
            .lock()
            .map_err(|_| ServiceError::Storage("active invocation mutex poisoned".into()))?;
        Ok(active
            .get(&(request_id.into(), operation_id.into()))
            .map(|entry| Arc::clone(&entry.reservation)))
    }

    pub(in crate::engine) fn has_active_invocation(
        &self,
        request_id: &str,
        operation_id: &str,
    ) -> bool {
        self.active_invocations
            .lock()
            .expect("active invocation mutex")
            .iter()
            .any(|(coordinates, active)| {
                coordinates == &(request_id.into(), operation_id.into())
                    || active.owner == std::thread::current().id()
            })
    }

    #[allow(clippy::too_many_arguments)]
    pub(in crate::engine) fn record_direct_authorization(
        &self,
        principal_id: Option<CanonicalId>,
        action: SecurityAction,
        resource: &ResourcePath,
        now: u64,
        request_id: &str,
        operation_id: &str,
        error: Option<&ServiceError>,
    ) -> Result<()> {
        if !self.security_enforced()? || self.has_active_invocation(request_id, operation_id) {
            return Ok(());
        }
        let request_sha256 = digest::sha256_hex(
            &serde_json::to_vec(&(action, resource, request_id, operation_id, now))
                .map_err(contract_json)?,
        );
        let (phase, decision, status_code, response_sha256) = match error {
            Some(error) => {
                let (decision, status_code) = audit_failure(error);
                (
                    AuditPhase::Completed,
                    decision,
                    status_code,
                    digest::sha256_hex(error.to_string().as_bytes()),
                )
            }
            None => (
                AuditPhase::Authorized,
                AuditDecision::Allowed,
                100,
                digest::sha256_hex(b"rrd-audit-completion-pending"),
            ),
        };
        self.append_audit(AuditEvent {
            at_unix_ms: now,
            attempt: 1,
            principal_id,
            action,
            resource: resource.clone(),
            request_id: request_id.into(),
            operation_id: operation_id.into(),
            phase,
            decision,
            status_code,
            request_sha256,
            response_sha256,
        })
    }
}

fn invocation_coordinates(invocation: &Invocation) -> (String, String) {
    (
        invocation.context.request_id.as_str().into(),
        invocation.context.operation_id.as_str().into(),
    )
}

fn validate_sha256(value: &str) -> Result<()> {
    if value.len() == 64
        && value
            .as_bytes()
            .iter()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        Ok(())
    } else {
        Err(ServiceError::Contract(
            "invocation digest must be lowercase SHA-256 hex".into(),
        ))
    }
}

pub(super) fn audit_failure(error: &ServiceError) -> (AuditDecision, u16) {
    match error.kind() {
        ServiceErrorKind::Unauthenticated => (AuditDecision::Denied, 401),
        ServiceErrorKind::PermissionDenied => (AuditDecision::Denied, 403),
        ServiceErrorKind::InvalidArgument => (AuditDecision::Failed, 400),
        ServiceErrorKind::NotFound => (AuditDecision::Failed, 404),
        ServiceErrorKind::Conflict => (AuditDecision::Failed, 409),
        ServiceErrorKind::FailedPrecondition => (AuditDecision::Failed, 412),
        ServiceErrorKind::ResourceExhausted => (AuditDecision::Failed, 429),
        ServiceErrorKind::DeadlineExceeded => (AuditDecision::Failed, 504),
        ServiceErrorKind::Internal => (AuditDecision::Failed, 500),
    }
}
