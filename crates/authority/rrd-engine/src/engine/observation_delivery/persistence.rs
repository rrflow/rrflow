//! Bounded delivery records in the existing atomic control store.
use super::*;
use rrd_contract::{AttunementReadResult, TaskAttemptView};
use std::io::Write;

// Match the canonical control store's per-value bound.
const MAX_RECORD_BYTES: usize = 1024 * 1024;
const MAX_INDEX_BYTES: usize = 128 * 1024;
pub(super) type WriteEntry = (String, Option<Vec<u8>>, Option<Vec<u8>>);

#[derive(PartialEq, Eq)]
pub(super) struct ObservationAuthority {
    pub principal: CanonicalId,
    security_key: String,
    security_bytes: Vec<u8>,
    session_key: String,
    session_bytes: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DeliveryRecord {
    pub format_version: u16,
    pub owner: CanonicalId,
    pub request_sha256: String,
    pub capture_id: String,
    pub references: EngineReferences,
    pub source_json: String,
    pub payload_json: String,
    pub snapshot: ObservationDeliverySnapshot,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DeliveryIndexEntry {
    record_sha256: String,
    pub state: ObservationDeliveryState,
    pub expires_at_unix_ms: u64,
    pub lease_expires_at_unix_ms: Option<u64>,
}
impl DeliveryIndexEntry {
    pub fn from_record(record: &DeliveryRecord, bytes: &[u8]) -> Self {
        Self {
            record_sha256: digest::sha256_hex(bytes),
            state: record.snapshot.state,
            expires_at_unix_ms: record.snapshot.expires_at_unix_ms,
            lease_expires_at_unix_ms: record.snapshot.lease_expires_at_unix_ms,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DeliveryIndex {
    format_version: u16,
    pub entries: BTreeMap<String, DeliveryIndexEntry>,
}

fn corrupt() -> ServiceError {
    ServiceError::Storage("invalid observation delivery record or index".into())
}

struct BoundedJson {
    bytes: Vec<u8>,
    limit: usize,
}
impl Write for BoundedJson {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() > self.limit.saturating_sub(self.bytes.len()) {
            return Err(std::io::Error::other(
                "observation serialization exceeds bound",
            ));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
fn bounded_json(value: &impl Serialize, limit: usize) -> Result<Vec<u8>> {
    let mut output = BoundedJson {
        bytes: Vec::new(),
        limit,
    };
    serde_json::to_writer(&mut output, value).map_err(contract_json)?;
    Ok(output.bytes)
}

fn operation(request_type: &str) -> Result<String> {
    rrd_contract::endpoint_catalogue()
        .endpoints
        .into_iter()
        .find(|endpoint| endpoint.request_type == request_type)
        .map(|endpoint| endpoint.operation.to_string())
        .ok_or_else(|| {
            ServiceError::Contract("observation source operation is not catalogued".into())
        })
}

/// Reconstruct references from the retained public response, not caller metadata.
fn source_references(
    instance: &str,
    source: &ObservationSource,
    json: &str,
) -> Result<EngineReferences> {
    let mut references = EngineReferences {
        version: 1,
        instance_id: instance.into(),
        scope: String::new(),
        operation: String::new(),
        response_sha256: digest::sha256_hex(json.as_bytes()),
        task_id: None,
        attempt_id: None,
        job_id: None,
        plan_sha256: None,
        source_sha256: None,
        configuration_sha256: None,
        selector_policy_id: None,
        selector_routing_sha256: None,
        selector_narrowed_candidate_count: None,
    };
    match source {
        ObservationSource::TaskAttempt {
            request,
            expected_status,
            expected_outcome_sha256,
        } => {
            let result: TaskAttemptView = serde_json::from_str(json).map_err(contract_json)?;
            result.validate().map_err(contract)?;
            let attempt = result.attempt.ok_or_else(|| {
                ServiceError::Contract("observation source attempt is absent".into())
            })?;
            if attempt.attempt_id != request.attempt_id
                || attempt.status != *expected_status
                || attempt.status == TaskAttemptStatus::Reserved
                || attempt.outcome_sha256 != *expected_outcome_sha256
            {
                return Err(ServiceError::StorageConflict(
                    "observation source attempt changed".into(),
                ));
            }
            references.scope = request.scope.clone();
            references.operation = operation("ReadTaskAttempt")?;
            references.task_id = Some(request.task_id.to_string());
            references.attempt_id = Some(request.attempt_id.as_str().into());
            // Bind the configuration that produced the retained result, not
            // the later configuration authorizing this observation export.
            references.configuration_sha256 = attempt
                .endpoint_proposal
                .as_ref()
                .map(|proposal| proposal.configuration_sha256.clone())
                .or_else(|| {
                    attempt
                        .worker_output
                        .as_ref()
                        .map(|worker| worker.configuration_sha256.clone())
                });
            let routing = attempt
                .endpoint_proposal
                .as_ref()
                .and_then(|proposal| proposal.routing.as_ref())
                .or_else(|| {
                    attempt
                        .worker_output
                        .as_ref()
                        .and_then(|worker| worker.routing.as_ref())
                });
            if let Some(routing) = routing {
                references.selector_policy_id =
                    Some(routing.semantic_router.policy_id.as_str().to_owned());
                references.selector_routing_sha256 = Some(
                    rrd_contract::endpoint_selection_routing_sha256(routing).map_err(contract)?,
                );
                references.selector_narrowed_candidate_count =
                    Some(routing.semantic_router.narrowed_candidate_count);
            }
        }
        ObservationSource::AttunementJob {
            request,
            expected_revision,
            expected_plan_sha256,
        } => {
            let result: AttunementReadResult = serde_json::from_str(json).map_err(contract_json)?;
            result.validate().map_err(contract)?;
            if result.status.job.id != request.job_id
                || result.status.job.revision != *expected_revision
                || result.status.job.plan_sha256 != *expected_plan_sha256
                || result.plan.target.instance_id.as_str() != instance
            {
                return Err(ServiceError::StorageConflict(
                    "observation source job changed".into(),
                ));
            }
            references.scope = format!("instance:{instance}");
            references.operation = operation("ReadAttunement")?;
            references.job_id = Some(request.job_id.to_string());
            references.plan_sha256 = Some(result.plan.plan_sha256);
            references.source_sha256 = Some(result.plan.source_sha256);
        }
    }
    references
        .validate()
        .map_err(|message| ServiceError::Contract(message.into()))?;
    Ok(references)
}

fn validate_record(record: &DeliveryRecord) -> Result<()> {
    let snapshot = &record.snapshot;
    snapshot.validate().map_err(contract)?;
    snapshot.source.validate().map_err(contract)?;
    if record.format_version != 1
        || record.source_json.len() > MAX_OBSERVATION_SOURCE_BYTES
        || record.payload_json.len() > rrd_contract::MAX_OBSERVATION_EVENT_BYTES
        || record.request_sha256
            != operation_digest(&PrepareObservationDelivery {
                contract_version: OBSERVATION_DELIVERY_VERSION,
                delivery_id: snapshot.delivery_id.clone(),
                source: snapshot.source.clone(),
                expected_configuration_sha256: snapshot.configuration_sha256.clone(),
            })?
        || record.references
            != source_references(
                &record.references.instance_id,
                &snapshot.source,
                &record.source_json,
            )?
        || record.references.scope != snapshot.scope
        || record.references.response_sha256 != snapshot.source_response_sha256
        || digest::sha256_hex(record.payload_json.as_bytes()) != snapshot.payload_sha256
    {
        return Err(corrupt());
    }
    let encoded = rrd_observation::encode_event(
        &snapshot.event_id,
        &record.capture_id,
        snapshot.delivery_id.as_str(),
        snapshot.prepared_at_unix_ms,
        &record.references,
    )
    .map_err(|_| corrupt())?;
    if encoded != record.payload_json.as_bytes() {
        return Err(corrupt());
    }
    Ok(())
}
pub(super) fn encode_record(record: &DeliveryRecord) -> Result<Vec<u8>> {
    validate_record(record)?;
    bounded_json(record, MAX_RECORD_BYTES)
}
pub(super) fn encode_index(index: &DeliveryIndex) -> Result<Vec<u8>> {
    if index.format_version != 1 || index.entries.len() > 128 {
        return Err(corrupt());
    }
    bounded_json(index, MAX_INDEX_BYTES)
}

impl RrdEngine {
    pub(super) fn observation_authority(
        &self,
        session: &CorrelationId,
        token: &CorrelationId,
        action: SecurityAction,
        context: &EngineControlContext<'_>,
    ) -> Result<ObservationAuthority> {
        let (security_key, security_bytes) =
            rrd_security::SecurityRepository::new(&self.storage, self.instance.clone())
                .authority_snapshot()?
                .ok_or(ServiceError::PermissionDenied)?;
        let (session_bytes, state) = self.authorize(
            session,
            token,
            action,
            context.now,
            context.request_id,
            context.operation_id,
        )?;
        Ok(ObservationAuthority {
            principal: state.principal_id.ok_or(ServiceError::Unauthenticated)?,
            security_key,
            security_bytes,
            session_key: super::super::session::session_key(&self.instance, session),
            session_bytes,
        })
    }
    pub(super) fn observation_index_key(&self) -> String {
        format!("server/state/{}/observation-delivery/index", self.instance)
    }
    pub(super) fn observation_record_key(
        &self,
        owner: &CanonicalId,
        id: &CorrelationId,
    ) -> Result<String> {
        Ok(format!(
            "server/state/{}/observation-delivery/record/{}",
            self.instance,
            operation_digest(&(owner, id))?
        ))
    }
    pub(super) fn observation_uuid(
        &self,
        domain: &str,
        owner: &CanonicalId,
        id: &CorrelationId,
    ) -> Result<String> {
        // Retention can retire a delivery ID. A fresh preparation must not reuse
        // an immutable remote event from a retired generation. Replay reads the
        // saved identity; entropy is consumed only for a new prepared record.
        let mut entropy = [0u8; 32];
        getrandom::fill(&mut entropy).map_err(|_| {
            ServiceError::Storage("observation identity entropy unavailable".into())
        })?;
        let generation = digest::sha256_hex(&entropy);
        let keyed = self.keyed_id(
            "observation",
            &[domain, owner.as_str(), id.as_str(), &generation],
        )?;
        let hex = digest::sha256_hex(keyed.as_str().as_bytes());
        // Engine-derived UUIDv8 with RFC variant bits; domain separates capture and event.
        Ok(format!(
            "{}-{}-8{}-a{}-{}",
            &hex[..8],
            &hex[8..12],
            &hex[13..16],
            &hex[17..20],
            &hex[20..32]
        ))
    }
    pub(super) fn observation_index(&self) -> Result<(Option<Vec<u8>>, DeliveryIndex)> {
        let bytes = self.storage.control().get(&self.observation_index_key())?;
        let index = match &bytes {
            Some(bytes) if bytes.len() <= MAX_INDEX_BYTES => {
                serde_json::from_slice::<DeliveryIndex>(bytes).map_err(contract_json)?
            }
            Some(_) => return Err(corrupt()),
            None => DeliveryIndex {
                format_version: 1,
                entries: BTreeMap::new(),
            },
        };
        encode_index(&index)?;
        let prefix = format!(
            "server/state/{}/observation-delivery/record/",
            self.instance
        );
        if index.entries.iter().any(|(key, entry)| {
            !key.strip_prefix(&prefix).is_some_and(valid_digest)
                || !valid_digest(&entry.record_sha256)
                || entry.expires_at_unix_ms == 0
                || (entry.state == ObservationDeliveryState::InFlight)
                    != entry.lease_expires_at_unix_ms.is_some()
        }) {
            return Err(corrupt());
        }
        Ok((bytes, index))
    }
    pub(super) fn observation_record(
        &self,
        key: &str,
        entry: &DeliveryIndexEntry,
    ) -> Result<(Vec<u8>, DeliveryRecord)> {
        let bytes = self.storage.control().get(key)?.ok_or_else(corrupt)?;
        if bytes.len() > MAX_RECORD_BYTES || digest::sha256_hex(&bytes) != entry.record_sha256 {
            return Err(corrupt());
        }
        let record: DeliveryRecord = serde_json::from_slice(&bytes).map_err(contract_json)?;
        validate_record(&record)?;
        if key != self.observation_record_key(&record.owner, &record.snapshot.delivery_id)?
            || record.references.instance_id != self.instance.as_str()
            || *entry != DeliveryIndexEntry::from_record(&record, &bytes)
        {
            return Err(corrupt());
        }
        Ok((bytes, record))
    }
    pub(super) fn authorize_observation_source(
        &self,
        session: &CorrelationId,
        token: &CorrelationId,
        source: &ObservationSource,
        retained_source_json: Option<&str>,
        context: &EngineControlContext<'_>,
    ) -> Result<()> {
        // Recheck current source-read authority without requiring the historical version
        // still to be current. This does not advance an attempt or execute a job.
        match source {
            ObservationSource::TaskAttempt { request, .. } => {
                let current = self.read_task_attempt(
                    session,
                    token,
                    request,
                    context.now,
                    context.request_id,
                    context.operation_id,
                )?;
                if let Some(retained_json) = retained_source_json {
                    let retained: TaskAttemptView =
                        serde_json::from_str(retained_json).map_err(contract_json)?;
                    let retained = retained.attempt.ok_or_else(corrupt)?;
                    let current = current.attempt.ok_or_else(|| {
                        ServiceError::StorageConflict("observation source attempt is absent".into())
                    })?;
                    if retained.attempt_id != current.attempt_id
                        || retained.status != current.status
                        || retained.outcome_sha256 != current.outcome_sha256
                    {
                        return Err(ServiceError::StorageConflict(
                            "observation source attempt changed".into(),
                        ));
                    }
                    if retained
                        .worker_output
                        .as_ref()
                        .is_some_and(|output| output.source.is_some())
                        && current.worker_output.is_none()
                    {
                        return Err(ServiceError::PermissionDenied);
                    }
                }
            }
            ObservationSource::AttunementJob { request, .. } => {
                self.read_attunement_detail(
                    session,
                    token,
                    request,
                    context.now,
                    context.request_id,
                    context.operation_id,
                )?;
            }
        }
        Ok(())
    }
    pub(super) fn observation_source(
        &self,
        session: &CorrelationId,
        token: &CorrelationId,
        source: &ObservationSource,
        context: &EngineControlContext<'_>,
    ) -> Result<(String, EngineReferences, WriteEntry)> {
        let key = match source {
            ObservationSource::TaskAttempt { request, .. } => {
                self.attempt_key(&request.scope, &request.task_id, &request.attempt_id)
            }
            ObservationSource::AttunementJob { request, .. } => {
                super::super::installation::attunement_job_key(&self.instance, &request.job_id)
            }
        };
        let before = self
            .storage
            .control()
            .get(&key)?
            .ok_or_else(|| ServiceError::Contract("observation source is absent".into()))?;
        let bytes = match source {
            ObservationSource::TaskAttempt { request, .. } => bounded_json(
                &self.read_task_attempt(
                    session,
                    token,
                    request,
                    context.now,
                    context.request_id,
                    context.operation_id,
                )?,
                MAX_OBSERVATION_SOURCE_BYTES,
            )?,
            ObservationSource::AttunementJob { request, .. } => bounded_json(
                &self.read_attunement_detail(
                    session,
                    token,
                    request,
                    context.now,
                    context.request_id,
                    context.operation_id,
                )?,
                MAX_OBSERVATION_SOURCE_BYTES,
            )?,
        };
        let json = String::from_utf8(bytes).map_err(|_| corrupt())?;
        let references = source_references(self.instance.as_str(), source, &json)?;
        Ok((json, references, (key, Some(before.clone()), Some(before))))
    }
    pub(super) fn commit_observation(
        &self,
        mut writes: Vec<WriteEntry>,
        authority: &ObservationAuthority,
        configuration_bytes: Option<Vec<u8>>,
        context: &EngineControlContext<'_>,
    ) -> Result<()> {
        for (key, bytes) in [
            (&authority.security_key, &authority.security_bytes),
            (&authority.session_key, &authority.session_bytes),
        ] {
            writes.push((key.clone(), Some(bytes.clone()), Some(bytes.clone())));
        }
        writes.push((
            self.endpoint_configuration_key(),
            configuration_bytes.clone(),
            configuration_bytes,
        ));
        let transitions: Vec<_> = writes
            .into_iter()
            .map(|(key, expected, replacement)| ControlTransition {
                action: if replacement.is_none() {
                    "observation.retired"
                } else {
                    context.action
                }
                .into(),
                key,
                expected,
                replacement,
                at: context.now,
                actor: format!("session:{}", context.session_id.as_str()),
                request_id: context.request_id.into(),
                operation_id: context.operation_id.into(),
            })
            .collect();
        let _permit =
            self.admit_transaction(&transitions, context.request_id, context.operation_id)?;
        self.storage.control().commit_batch(&transitions)?;
        Ok(())
    }
}
fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
