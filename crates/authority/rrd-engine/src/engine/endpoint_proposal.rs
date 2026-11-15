//! Engine-owned experimental task proposal. A proposal grants no worker effect.
use super::clock::{observe_required, ClockSource, HostSystemClock};
use super::reasoning_budget::{
    ActiveTaskCancellation, AttemptAuthorityGuard, TaskAttemptAdmission, TaskAttemptOutput,
    TaskCompletionAdmission,
};
use super::*;
use rrd_contract::{
    CredentialReference, EndpointSelectionPacket, EndpointSelectionProposal,
    EndpointSelectionRequest, EndpointSelectionTask, ReadTask, TaskAttemptInvalidation,
    ENDPOINT_SELECTION_VERSION,
};
use rrd_inference::endpoint_chat::{
    ChatMessage, ChatResponseFormat, ChatRole, EndpointChatFailure, EndpointChatRequest,
    PreparedEndpointChat,
};
use rrd_inference::lfg_transport::{LfgTransportConfig, PreparedLfgObservation};
use rrd_inference::{EmbeddingCancellation, NetworkPolicy};
pub(super) mod capacity;
use capacity::CapacityLease;

/// Immutable process-owned secrets, installed before sharing the engine.
/// No serialization, debugging output, environment scan or runtime resolver.
pub struct EndpointCredentials {
    values: BTreeMap<(CanonicalId, String), String>,
}

impl EndpointCredentials {
    pub fn new(entries: Vec<(CredentialReference, String)>) -> Result<Self> {
        if entries.len() > rrd_contract::MAX_PROFILE_BINDINGS {
            return Err(ServiceError::ConfigurationLimit(
                "too many endpoint credentials".into(),
            ));
        }
        let mut values = BTreeMap::new();
        for (reference, secret) in entries {
            if reference.name.is_empty()
                || reference.name.len() > 256
                || reference
                    .name
                    .chars()
                    .any(|c| c.is_whitespace() || c.is_control())
                || secret.is_empty()
                || secret.len() > 16_384
                || secret
                    .bytes()
                    .any(|b| b.is_ascii_whitespace() || b.is_ascii_control())
                || values
                    .insert((reference.provider, reference.name), secret)
                    .is_some()
            {
                return Err(ServiceError::Contract(
                    "invalid or duplicate endpoint credential".into(),
                ));
            }
        }
        Ok(Self { values })
    }
    pub(in crate::engine) fn resolve(&self, reference: &CredentialReference) -> Option<&str> {
        self.values
            .get(&(reference.provider.clone(), reference.name.clone()))
            .map(String::as_str)
    }
    pub(super) fn contains(&self, reference: &CredentialReference) -> bool {
        self.values
            .contains_key(&(reference.provider.clone(), reference.name.clone()))
    }
}

struct ProposalAuthority {
    security_key: String,
    security_bytes: Vec<u8>,
    session_key: String,
    session_bytes: Vec<u8>,
    configuration_key: String,
    configuration_bytes: Vec<u8>,
}

impl RrdEngine {
    /// Trusted process composition, not a public configuration operation.
    pub fn install_endpoint_credentials(&mut self, credentials: EndpointCredentials) -> Result<()> {
        if self.endpoint_credentials.is_some() {
            return Err(ServiceError::Contract(
                "endpoint credentials are already installed".into(),
            ));
        }
        self.endpoint_credentials = Some(credentials);
        Ok(())
    }

    /// Admit exactly one experimental call for a current task. Replay never
    /// dispatches again; read_task_attempt is the durable recovery operation.
    pub fn propose_task_endpoint(
        &self,
        session: &CorrelationId,
        token: &CorrelationId,
        request: &EndpointSelectionRequest,
        request_id: &str,
        operation_id: &str,
    ) -> Result<EndpointSelectionProposal> {
        self.propose_task_endpoint_with_clock(
            session,
            token,
            request,
            request_id,
            operation_id,
            &HostSystemClock,
        )
    }

    pub(super) fn propose_task_endpoint_with_clock(
        &self,
        session: &CorrelationId,
        token: &CorrelationId,
        request: &EndpointSelectionRequest,
        request_id: &str,
        operation_id: &str,
        clock: &dyn ClockSource,
    ) -> Result<EndpointSelectionProposal> {
        let started = Instant::now();
        request
            .validate()
            .map_err(|e| ServiceError::Contract(e.to_string()))?;
        let now = observe_required(clock, "endpoint-proposal-authorize")?.observed_at_unix_ms;
        let (security_key, security_bytes) =
            rrd_security::SecurityRepository::new(&self.storage, self.instance.clone())
                .authority_snapshot()?
                .ok_or(ServiceError::PermissionDenied)?;
        let mut session_bytes = None;
        for action in [
            SecurityAction::EndpointPropose,
            SecurityAction::TaskRead,
            SecurityAction::ReasoningWrite,
        ] {
            let (bytes, _) =
                self.authorize(session, token, action, now, request_id, operation_id)?;
            if session_bytes.as_ref().is_some_and(|prior| prior != &bytes) {
                return Err(ServiceError::TaskAttemptInvalidated(
                    TaskAttemptInvalidation::AuthorizationChanged,
                ));
            }
            session_bytes = Some(bytes);
        }
        let (configuration_bytes, configuration) = self.load_endpoint_configuration()?;
        let authority = ProposalAuthority {
            security_key,
            security_bytes,
            session_key: super::session::session_key(&self.instance, session),
            session_bytes: session_bytes.ok_or(ServiceError::PermissionDenied)?,
            configuration_key: self.endpoint_configuration_key(),
            configuration_bytes: configuration_bytes
                .ok_or_else(|| ServiceError::Contract("endpoint configuration is absent".into()))?,
        };
        let profile = configuration
            .profile
            .as_ref()
            .ok_or_else(|| ServiceError::Contract("endpoint profile is absent".into()))?;
        let policy = profile
            .selection_policy
            .as_ref()
            .ok_or_else(|| ServiceError::Contract("endpoint selector policy is absent".into()))?;
        if !policy.allow_network || !policy.allow_unverified_identity {
            return Err(ServiceError::PermissionDenied);
        }
        if configuration
            .sha256()
            .map_err(|e| ServiceError::Contract(e.to_string()))?
            != request.expected_configuration_sha256
        {
            return Err(ServiceError::TaskAttemptInvalidated(
                TaskAttemptInvalidation::InputsChanged,
            ));
        }
        let read = self
            .storage
            .runtime()
            .read_stamp(&self.query_scope(&request.scope)?)?;
        let task = self.read_task_record_at(&read, &task_read(request))?;
        // A request that names a prompt flight is admitted only against the
        // retained flight: its task, cohort and context request must match.
        if let Some(flight) = request.flight.as_ref() {
            let observed = self
                .read_prompt_flight(
                    session,
                    token,
                    &rrd_contract::ReadPromptFlight {
                        scope: request.scope.clone(),
                        flight_id: flight.flight_id.clone(),
                        max_storage_keys: request.max_storage_keys,
                    },
                    now,
                    request_id,
                    operation_id,
                )?
                .flight;
            validate_flight_identity(request, flight, &observed.record)?;
        }
        // A routed request is offered only the candidates its router narrowed
        // to. The packet validator requires exactly this allowance.
        let allowance = policy
            .allowance(&configuration)
            .map_err(|e| ServiceError::Contract(e.to_string()))?;
        let allowance = match &request.routing {
            Some(routing) => routing
                .narrow_allowance(&allowance)
                .map_err(|e| ServiceError::Contract(e.to_string()))?,
            None => allowance,
        };
        let mut packet = EndpointSelectionPacket {
            contract_version: ENDPOINT_SELECTION_VERSION,
            purpose: policy.purpose,
            task: EndpointSelectionTask::from_task(&task)
                .map_err(|e| ServiceError::Contract(e.to_string()))?,
            read_manifest_sha256: read.manifest_id.clone(),
            read_cursor: read.commit_cursor,
            controller_binding_id: policy.controller_binding_id.clone(),
            allowance,
            lfg: None,
            routing: request.routing.clone(),
        };
        if packet.task.task_sha256 != request.expected_task_sha256 {
            return Err(ServiceError::TaskAttemptInvalidated(
                TaskAttemptInvalidation::InputsChanged,
            ));
        }
        let packet_sha256 = packet
            .sha256(&configuration)
            .map_err(|e| ServiceError::Contract(e.to_string()))?;
        let binding = profile
            .bindings
            .iter()
            .find(|binding| binding.id == policy.controller_binding_id)
            .ok_or_else(|| ServiceError::Contract("controller binding is absent".into()))?;
        // Preflight the configured controller without retaining a packet borrow.
        drop(prepare_controller(
            &configuration,
            binding,
            policy,
            &packet,
        )?);
        let prepared_lfg = policy
            .lfg
            .as_ref()
            .map(|lfg| {
                let text = serde_json::to_string(&packet.task).map_err(contract_json)?;
                let config = LfgTransportConfig {
                    base_url: &lfg.base_url,
                    assessment_sha256: &lfg.assessment_sha256,
                    rules_sha256: &lfg.rules_sha256,
                    timeout: Duration::from_millis(lfg.timeout_ms),
                    maximum_input_bytes: usize::try_from(lfg.maximum_input_bytes).map_err(
                        |_| {
                            ServiceError::ConfigurationLimit(
                                "LFG input bound exceeds platform".into(),
                            )
                        },
                    )?,
                    maximum_request_bytes: usize::try_from(lfg.maximum_request_bytes).map_err(
                        |_| {
                            ServiceError::ConfigurationLimit(
                                "LFG request bound exceeds platform".into(),
                            )
                        },
                    )?,
                    maximum_response_bytes: usize::try_from(lfg.maximum_response_bytes).map_err(
                        |_| {
                            ServiceError::ConfigurationLimit(
                                "LFG response bound exceeds platform".into(),
                            )
                        },
                    )?,
                };
                PreparedLfgObservation::new(
                    &config,
                    request.attempt_id.as_str(),
                    packet.task.known_at_cursor,
                    &text,
                )
                .map_err(|error| ServiceError::EndpointCallFailed {
                    reason: error.reason,
                    execution_may_continue: error.execution_may_continue,
                })
            })
            .transpose()?;
        let mut admission =
            self.proposal_completion_admission(&authority, &packet, request, &read)?;
        if let Some(reason) = admission.invalidation {
            return Err(ServiceError::TaskAttemptInvalidated(reason));
        }
        if binding.credential.as_ref().is_some_and(|reference| {
            !self
                .endpoint_credentials
                .as_ref()
                .is_some_and(|credentials| credentials.contains(reference))
        }) {
            return Err(ServiceError::EndpointCallFailed {
                reason: "configured endpoint credential is unavailable",
                execution_may_continue: false,
            });
        }
        let lease = CapacityLease {
            request: request.clone(),
            controller_binding_id: binding.id.clone(),
        };
        admission.writes.push(self.endpoint_capacity_transition(
            &lease,
            Some((
                policy.maximum_concurrent_proposals,
                binding.limits.max_concurrency,
            )),
        )?);
        let cancellation = EmbeddingCancellation::default();
        let context = EngineControlContext {
            now,
            session_id: session,
            action: "endpoint-proposal",
            request_id,
            operation_id,
        };
        let request_sha256 = operation_digest(request)?;
        let key = self.budget_key(&request.scope, &request.task_id);
        let mut reservation = {
            let mut active = self
                .active_task_cancellations
                .lock()
                .map_err(|_| ServiceError::Storage("task cancellation lock poisoned".into()))?;
            let reservation = self.reserve_controlled_task_attempt(
                &TaskAttemptAdmission {
                    started,
                    scope: &request.scope,
                    task_id: &request.task_id,
                    attempt_id: &request.attempt_id,
                    request_sha256: &request_sha256,
                    cancellation: &cancellation,
                },
                || Ok(packet_sha256.clone()),
                &admission,
                clock,
                &context,
            )?;
            active.insert(key.clone(), (request.attempt_id.clone(), cancellation));
            reservation
        };
        let _active = ActiveTaskCancellation {
            engine: self,
            key,
            attempt_id: request.attempt_id.clone(),
        };
        let result = (|| {
            // Check live authorization (including expiry) immediately before
            // credential lookup and dispatch; completion repeats it below.
            let at = observe_required(clock, "endpoint-proposal-dispatch")?.observed_at_unix_ms;
            for action in [
                SecurityAction::EndpointPropose,
                SecurityAction::TaskRead,
                SecurityAction::ReasoningWrite,
            ] {
                self.authorize(session, token, action, at, request_id, operation_id)?;
            }
            let current = self
                .storage
                .runtime()
                .read_stamp(&self.query_scope(&request.scope)?)?;
            if let Some(reason) = self
                .proposal_completion_admission(&authority, &packet, request, &current)?
                .invalidation
            {
                return Err(ServiceError::TaskAttemptInvalidated(reason));
            }
            if let Some(lfg) = prepared_lfg {
                let started =
                    tracing::event_enabled!(target: "rrd_engine::lfg", tracing::Level::DEBUG)
                        .then(Instant::now);
                let observed = match lfg
                    .execute(NetworkPolicy::Allow, reservation.execution_control())
                {
                    Ok(observed) => {
                        tracing::debug!(
                            target: "rrd_engine::lfg",
                            operation_sha256 = %request_sha256,
                            outcome = "transport_completed",
                            elapsed_ms = observed.elapsed_ms,
                            analyzer_request_sha256 = %observed.request_sha256,
                            analyzer_response_sha256 = %observed.response_sha256,
                            "LFG transport completed"
                        );
                        observed
                    }
                    Err(error) => {
                        if let Some(started) = started {
                            tracing::debug!(
                                target: "rrd_engine::lfg",
                                operation_sha256 = %request_sha256,
                                outcome = "transport_failed",
                                elapsed_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
                                execution_may_continue = error.execution_may_continue,
                                "LFG transport failed"
                            );
                        }
                        return Err(ServiceError::EndpointCallFailed {
                            reason: error.reason,
                            execution_may_continue: error.execution_may_continue,
                        });
                    }
                };
                let value = observed.observation;
                let advisory = rrd_contract::LfgAdvisory {
                    request_id: request.attempt_id.clone(),
                    revision: value.revision,
                    input_sha256: value.input_sha256,
                    assessment_sha256: value.assessment_sha256,
                    rules_sha256: value.rules_sha256,
                    request_sha256: observed.request_sha256,
                    response_sha256: observed.response_sha256,
                    status: value.status,
                    intent_status: value.intent.status,
                    intent_category: value.intent.category,
                    suggested_role: value.suggested_role,
                    suggested_effort: value.suggested_effort,
                    reason_ids: value.reason_ids,
                    elapsed_ms: observed.elapsed_ms,
                };
                packet.lfg = Some(advisory.clone());
                packet.validate_for(&configuration).map_err(|e| {
                    // A completed transport that cannot be checkpointed is the
                    // one silent exit on this path; name it for the diagnostics.
                    tracing::debug!(
                        target: "rrd_engine::lfg",
                        operation_sha256 = %request_sha256,
                        outcome = "advisory_rejected",
                        error = %e,
                        "LFG advisory rejected before checkpoint"
                    );
                    ServiceError::Contract(e.to_string())
                })?;
                let invalidation = self
                    .checkpoint_lfg_advisory(
                        &mut reservation,
                        advisory,
                        |read| {
                            let at = observe_required(clock, "lfg-checkpoint-authorize")?
                                .observed_at_unix_ms;
                            let mut admission = self.proposal_completion_admission(
                                &authority, &packet, request, read,
                            )?;
                            for action in [
                                SecurityAction::EndpointPropose,
                                SecurityAction::TaskRead,
                                SecurityAction::ReasoningWrite,
                            ] {
                                if self
                                    .authorize(session, token, action, at, request_id, operation_id)
                                    .is_err()
                                {
                                    admission.invalidation =
                                        Some(TaskAttemptInvalidation::AuthorizationChanged);
                                }
                            }
                            Ok(admission)
                        },
                        clock,
                        &context,
                    )
                    .map_err(|_| {
                        tracing::debug!(
                            target: "rrd_engine::lfg",
                            operation_sha256 = %request_sha256,
                            outcome = "checkpoint_failed",
                            execution_may_continue = true,
                            "LFG checkpoint failed"
                        );
                        ServiceError::EndpointCallFailed {
                            reason: "LFG completed but its checkpoint could not be committed",
                            execution_may_continue: true,
                        }
                    })?;
                tracing::debug!(
                    target: "rrd_engine::lfg",
                    operation_sha256 = %request_sha256,
                    outcome = "checkpoint_committed",
                    invalidated = invalidation.is_some(),
                    "LFG checkpoint committed"
                );
                if let Some(reason) = invalidation {
                    return Err(ServiceError::TaskAttemptInvalidated(reason));
                }
                let at =
                    observe_required(clock, "endpoint-controller-dispatch")?.observed_at_unix_ms;
                for action in [
                    SecurityAction::EndpointPropose,
                    SecurityAction::TaskRead,
                    SecurityAction::ReasoningWrite,
                ] {
                    self.authorize(session, token, action, at, request_id, operation_id)?;
                }
                let current = self
                    .storage
                    .runtime()
                    .read_stamp(&self.query_scope(&request.scope)?)?;
                if let Some(reason) = self
                    .proposal_completion_admission(&authority, &packet, request, &current)?
                    .invalidation
                {
                    return Err(ServiceError::TaskAttemptInvalidated(reason));
                }
            }
            let final_packet_sha256 = packet
                .sha256(&configuration)
                .map_err(|e| ServiceError::Contract(e.to_string()))?;
            let prepared = prepare_controller(&configuration, binding, policy, &packet)?;
            let observed = prepared
                .execute(
                    NetworkPolicy::Allow,
                    reservation.execution_control(),
                    |reference, _control| {
                        self.endpoint_credentials
                            .as_ref()
                            .and_then(|credentials| credentials.resolve(reference))
                            .map(str::to_owned)
                            .ok_or_else(|| "configured endpoint credential is unavailable".into())
                    },
                )
                .map_err(endpoint_failure)?;
            let proposal = EndpointSelectionProposal {
                contract_version: ENDPOINT_SELECTION_VERSION,
                purpose: packet.purpose,
                scope: request.scope.clone(),
                task_id: request.task_id.clone(),
                attempt_id: request.attempt_id.clone(),
                task_sha256: packet.task.task_sha256.clone(),
                packet_sha256: final_packet_sha256,
                configuration_sha256: observed.configuration_sha256,
                allowance_sha256: observed.allowance_sha256,
                controller_binding_id: binding.id.clone(),
                service_id: CanonicalId::new(observed.service_id)
                    .map_err(|e| ServiceError::Contract(e.to_string()))?,
                flight: request.flight.clone(),
                routing: packet.routing.clone(),
                request_sha256: observed.request_sha256,
                response_sha256: observed.response_sha256,
                content_sha256: observed.content_sha256,
                choice: observed.choice,
                usage: observed.usage,
                elapsed_ms: observed.elapsed_ms,
                http_status: observed.http_status,
                lfg: packet.lfg.clone(),
            };
            proposal
                .validate_for(request, &packet, &configuration)
                .map_err(|e| ServiceError::Contract(e.to_string()))?;
            Ok(proposal.into())
        })();
        let uncertain = matches!(
            &result,
            Err(ServiceError::EndpointCallFailed {
                execution_may_continue: true,
                ..
            })
        );
        match self.finish_controlled_task_attempt(
            reservation,
            result,
            |read| {
                let mut admission =
                    self.proposal_completion_admission(&authority, &packet, request, read)?;
                let at = observe_required(clock, "endpoint-proposal-completion-authorize")?
                    .observed_at_unix_ms;
                for action in [
                    SecurityAction::EndpointPropose,
                    SecurityAction::TaskRead,
                    SecurityAction::ReasoningWrite,
                ] {
                    if self
                        .authorize(session, token, action, at, request_id, operation_id)
                        .is_err()
                    {
                        admission.invalidation =
                            Some(TaskAttemptInvalidation::AuthorizationChanged);
                    }
                }
                if !uncertain {
                    admission
                        .writes
                        .push(self.endpoint_capacity_transition(&lease, None)?);
                }
                Ok(admission)
            },
            clock,
            &context,
        )? {
            TaskAttemptOutput::EndpointProposal(proposal) => Ok(*proposal),
            TaskAttemptOutput::Embedding(_)
            | TaskAttemptOutput::Worker(_)
            | TaskAttemptOutput::Assessment(_) => {
                Err(ServiceError::Storage("unexpected task output kind".into()))
            }
        }
    }

    fn proposal_completion_admission(
        &self,
        authority: &ProposalAuthority,
        packet: &EndpointSelectionPacket,
        request: &EndpointSelectionRequest,
        read: &ReadStamp,
    ) -> Result<TaskCompletionAdmission> {
        let mut admission = TaskCompletionAdmission::default();
        for (key, bytes, reason) in [
            (
                &authority.security_key,
                &authority.security_bytes,
                TaskAttemptInvalidation::AuthorizationChanged,
            ),
            (
                &authority.session_key,
                &authority.session_bytes,
                TaskAttemptInvalidation::AuthorizationChanged,
            ),
            (
                &authority.configuration_key,
                &authority.configuration_bytes,
                TaskAttemptInvalidation::InputsChanged,
            ),
        ] {
            if self.storage.control().get(key)?.as_ref() != Some(bytes) {
                admission.invalidation = Some(reason);
            } else {
                admission.guards.push(AttemptAuthorityGuard {
                    key: key.clone(),
                    expected: bytes.clone(),
                });
            }
        }
        let task = self.read_task_record_at(read, &task_read(request))?;
        if packet.validate_task(&task).is_err() {
            admission.invalidation = Some(TaskAttemptInvalidation::InputsChanged);
        }
        Ok(admission)
    }
}

fn task_read(request: &EndpointSelectionRequest) -> ReadTask {
    ReadTask {
        scope: request.scope.clone(),
        task_id: request.task_id.clone(),
        max_storage_keys: request.max_storage_keys,
    }
}
fn prepare_controller<'a>(
    configuration: &'a rrd_contract::EndpointConfiguration,
    binding: &rrd_contract::EndpointBinding,
    policy: &rrd_contract::EndpointSelectionPolicy,
    packet: &'a EndpointSelectionPacket,
) -> Result<PreparedEndpointChat<'a>> {
    let chat = EndpointChatRequest {
        model: binding.service_id.clone(),
        messages: vec![
            ChatMessage {
                role: ChatRole::System,
                content: policy.system_prompt.clone(),
            },
            ChatMessage {
                role: ChatRole::User,
                content: serde_json::to_string(packet).map_err(contract_json)?,
            },
        ],
        max_tokens: policy.maximum_output_tokens,
        temperature: f64::from(policy.temperature_milli) / 1000.0,
        response_format: ChatResponseFormat::JsonObject,
    };
    PreparedEndpointChat::new(configuration, &binding.id, &packet.allowance, &chat)
        .map_err(endpoint_failure)
}
fn validate_flight_identity(
    request: &EndpointSelectionRequest,
    expected: &rrd_contract::EndpointSelectionFlightIdentity,
    observed: &rrd_contract::PromptFlightRecord,
) -> Result<()> {
    if observed.scope != request.scope
        || observed.task_id.as_ref() != Some(&request.task_id)
        || observed.cohort_sha256 != expected.prompt_cohort_sha256
    {
        return Err(ServiceError::TaskAttemptInvalidated(
            TaskAttemptInvalidation::InputsChanged,
        ));
    }
    let observed_context = observed
        .context
        .as_ref()
        .map(|context| context.request_sha256.as_str());
    if observed_context != expected.context_request_sha256.as_deref() {
        return Err(ServiceError::TaskAttemptInvalidated(
            TaskAttemptInvalidation::InputsChanged,
        ));
    }
    Ok(())
}

pub(super) fn endpoint_failure(error: EndpointChatFailure) -> ServiceError {
    ServiceError::EndpointCallFailed {
        reason: error.reason,
        execution_may_continue: error.execution_may_continue(),
    }
}
