//! Flight reads and versions, identities, context uses, evidence and events.

use super::*;

pub(super) struct FlightRead {
    pub(super) record: Option<PromptFlightRecord>,
    pub(super) uses: Vec<PromptFlightUse>,
    pub(super) events: Vec<PromptFlightEvent>,
    pub(super) selected_versions: u64,
    pub(super) read_evidence: rrd_contract::ReadEvidence,
}

/// The versions of one flight row seen at a read stamp. The first version
/// fixes creation order; the latest version is the current projection.
#[derive(Default)]
pub(super) struct FlightVersions {
    pub(super) first_cursor: u64,
    pub(super) latest_cursor: u64,
    pub(super) latest: Option<RuntimeRecord>,
}

impl FlightVersions {
    pub(super) fn observe(&mut self, cursor: u64, record: &RuntimeRecord) {
        if self.first_cursor == 0 || cursor < self.first_cursor {
            self.first_cursor = cursor;
        }
        if cursor >= self.latest_cursor {
            self.latest_cursor = cursor;
            self.latest = Some(record.clone());
        }
    }

    pub(super) fn record(self, scope: &str) -> Result<PromptFlightRecord> {
        let latest = self
            .latest
            .ok_or_else(|| ServiceError::Storage("prompt flight row has no version".into()))?;
        public_record(&latest, scope, self.first_cursor, self.latest_cursor)
    }
}

pub(super) fn assemble_flight(
    record: PromptFlightRecord,
    uses: Vec<PromptFlightUse>,
    events: Vec<PromptFlightEvent>,
) -> Result<PromptFlight> {
    if uses.len() as u64 != record.use_count {
        return Err(ServiceError::Storage(format!(
            "prompt flight {} has {} edges at this stamp but records {}",
            record.flight_id,
            uses.len(),
            record.use_count
        )));
    }
    if events.len() as u64 != record.event_count {
        return Err(ServiceError::Storage(format!(
            "prompt flight {} has {} events at this stamp but records {}",
            record.flight_id,
            events.len(),
            record.event_count
        )));
    }
    let flight = PromptFlight {
        record,
        uses,
        events,
    };
    flight
        .validate()
        .map_err(|error| ServiceError::Storage(error.to_string()))?;
    Ok(flight)
}

/// Deterministic flight identity: one session and one idempotency key name
/// exactly one flight, so a replayed launch finds its row without a second
/// receipt store.
pub(super) fn flight_id(
    instance: &CanonicalId,
    session_id: &CorrelationId,
    idempotency_key: &CorrelationId,
) -> Result<CanonicalId> {
    let identity = digest::sha256_hex(
        format!(
            "rrd-prompt-flight-v1\0{}\0{}\0{}",
            instance.as_str(),
            session_id.as_str(),
            idempotency_key.as_str()
        )
        .as_bytes(),
    );
    CanonicalId::new(format!("flight-{identity}"))
        .map_err(|error| ServiceError::Contract(error.to_string()))
}

pub(super) fn flight_reference(flight_id: &CanonicalId) -> Result<RuntimeRef> {
    RuntimeRef::new(PROMPT_FLIGHT_KIND, flight_id.as_str()).map_err(core_contract)
}

pub(super) fn record_kind() -> Result<RuntimeType> {
    RuntimeType::new(PROMPT_FLIGHT_KIND).map_err(core_contract)
}

pub(super) fn event_kind() -> Result<RuntimeType> {
    RuntimeType::new(PROMPT_FLIGHT_EVENT_KIND).map_err(core_contract)
}

pub(super) fn use_kind() -> Result<RuntimeType> {
    RuntimeType::new(PROMPT_FLIGHT_USE_KIND).map_err(core_contract)
}

/// Edge identity: the flight identity and the item's packet rank. It is
/// canonical, unique within the flight, and computable from the record alone.
pub(super) fn use_reference(flight_id: &CanonicalId, rank: u64) -> Result<RuntimeRef> {
    RuntimeRef::new(PROMPT_FLIGHT_USE_KIND, format!("{flight_id}.{rank}")).map_err(core_contract)
}

/// One edge per context item that is a canonical record, in packet order.
/// Claim items are not graph nodes and get no edge.
pub(super) fn context_uses(packet: &ContextPacket) -> Result<Vec<(RuntimeRef, PromptFlightUse)>> {
    let mut uses = Vec::new();
    for (position, item) in packet.items.iter().enumerate() {
        let Some(rest) = item.identity.strip_prefix("record:") else {
            continue;
        };
        let (target, reference) = context_record_reference(rest)?;
        let used = PromptFlightUse {
            reference,
            rank: u64::try_from(position + 1)
                .map_err(|_| ServiceError::Contract("context rank exceeds u64".into()))?,
            score: item.score,
            evidence: item
                .evidence
                .iter()
                .map(|evidence| evidence.kind)
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect(),
        };
        used.validate()
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        uses.push((target, used));
    }
    Ok(uses)
}

pub(super) fn context_record_reference(value: &str) -> Result<(RuntimeRef, DataReference)> {
    let target = rrd_core::result_identity::parse_reference(value).ok_or_else(|| {
        ServiceError::Storage("context record identity is malformed or noncanonical".into())
    })?;
    let canonical = |value: &str| {
        CanonicalId::new(value).map_err(|_| {
            ServiceError::Storage(
                "context record reference is not representable by the public flight contract"
                    .into(),
            )
        })
    };
    let reference = DataReference {
        kind: canonical(target.kind.as_str())?,
        id: canonical(target.id.as_str())?,
    };
    Ok((target, reference))
}

pub(super) fn use_properties(used: &PromptFlightUse) -> RuntimeProperties {
    RuntimeProperties::from([
        ("rank".to_owned(), RuntimeValue::Unsigned(used.rank)),
        (
            "score".to_owned(),
            RuntimeValue::Decimal(used.score.to_string()),
        ),
        (
            "evidence".to_owned(),
            RuntimeValue::List(
                used.evidence
                    .iter()
                    .map(|kind| RuntimeValue::String(evidence_text(*kind).into()))
                    .collect(),
            ),
        ),
    ])
}

pub(super) fn public_use(relation: &RuntimeRelation) -> Result<PromptFlightUse> {
    let properties = &relation.properties;
    let score = match properties.get("score") {
        Some(RuntimeValue::Decimal(value)) => value
            .parse::<f64>()
            .map_err(|_| flight_storage("use score is not a decimal"))?,
        _ => return Err(flight_storage("use score is missing or not a decimal")),
    };
    let evidence = match properties.get("evidence") {
        Some(RuntimeValue::List(values)) => values
            .iter()
            .map(|value| match value {
                RuntimeValue::String(text) => parse_evidence(text),
                _ => Err(flight_storage("use evidence entry is not text")),
            })
            .collect::<Result<Vec<_>>>()?,
        _ => return Err(flight_storage("use evidence is missing or not a list")),
    };
    let used = PromptFlightUse {
        reference: public_change_ref(&relation.to)?,
        rank: unsigned(properties, "rank")?,
        score,
        evidence,
    };
    used.validate()
        .map_err(|error| flight_storage(&error.to_string()))?;
    Ok(used)
}

pub(super) fn evidence_text(kind: ContextEvidenceKind) -> &'static str {
    match kind {
        ContextEvidenceKind::Seed => "seed",
        ContextEvidenceKind::Text => "text",
        ContextEvidenceKind::Graph => "graph",
        ContextEvidenceKind::Vector => "vector",
    }
}

pub(super) fn parse_evidence(text: &str) -> Result<ContextEvidenceKind> {
    match text {
        "seed" => Ok(ContextEvidenceKind::Seed),
        "text" => Ok(ContextEvidenceKind::Text),
        "graph" => Ok(ContextEvidenceKind::Graph),
        "vector" => Ok(ContextEvidenceKind::Vector),
        other => Err(flight_storage(&format!(
            "unknown use evidence kind {other:?}"
        ))),
    }
}

/// Combines the logical I/O of the flight's two stamped reads into the one
/// evidence the response reports, under the caller's single key budget.
pub(super) fn merge_read_evidence(
    key_budget: u64,
    first: ReadEvidence,
    second: ReadEvidence,
) -> Result<ReadEvidence> {
    fn sum(left: u64, right: u64) -> Result<u64> {
        left.checked_add(right)
            .ok_or_else(|| flight_storage("read evidence counter overflowed"))
    }
    let mut paths = BTreeMap::<ReadAccessPath, ReadPathEvidence>::new();
    for path in first.paths.into_iter().chain(second.paths) {
        match paths.get_mut(&path.path) {
            Some(total) => {
                total.point_reads = sum(total.point_reads, path.point_reads)?;
                total.range_scans = sum(total.range_scans, path.range_scans)?;
                total.keys_examined = sum(total.keys_examined, path.keys_examined)?;
                total.values_decoded = sum(total.values_decoded, path.values_decoded)?;
                total.decoded_bytes = sum(total.decoded_bytes, path.decoded_bytes)?;
            }
            None => {
                paths.insert(path.path, path);
            }
        }
    }
    let current = ReadStampValidationMethod::AuthenticatedCurrentHead;
    let stamp_validation =
        if first.stamp_validation.method == current && second.stamp_validation.method == current {
            first.stamp_validation
        } else {
            ReadStampValidationEvidence {
                method: ReadStampValidationMethod::Rfc9162DirectVersions,
                change_reads: sum(
                    first.stamp_validation.change_reads,
                    second.stamp_validation.change_reads,
                )?,
                proof_nodes: first
                    .stamp_validation
                    .proof_nodes
                    .saturating_add(second.stamp_validation.proof_nodes),
            }
        };
    let evidence = ReadEvidence {
        contract_version: first.contract_version,
        key_budget,
        point_reads: sum(first.point_reads, second.point_reads)?,
        range_scans: sum(first.range_scans, second.range_scans)?,
        keys_examined: sum(first.keys_examined, second.keys_examined)?,
        values_decoded: sum(first.values_decoded, second.values_decoded)?,
        decoded_bytes: sum(first.decoded_bytes, second.decoded_bytes)?,
        stamp_validation,
        paths: paths.into_values().collect(),
    };
    evidence
        .validate()
        .map_err(|error| flight_storage(&error.to_string()))?;
    Ok(evidence)
}

pub(in super::super) fn read_budget(max_storage_keys: u64) -> Result<RuntimeReadBudget> {
    RuntimeReadBudget::new(max_storage_keys)
        .map_err(|error| ServiceError::Contract(error.to_string()))
}

pub(super) fn version_count(count: usize) -> Result<u64> {
    u64::try_from(count)
        .map_err(|_| ServiceError::Storage("selected prompt flight versions exceed u64".into()))
}

pub(in super::super) fn public_read_stamp(read: &ReadStamp) -> ContextReadStamp {
    ContextReadStamp {
        runtime_manifest_sha256: read.manifest_id.clone(),
        runtime_cursor: read.commit_cursor,
        schema_revision: read.schema_revision,
        catalogue_revision: read.catalog_revision,
    }
}

pub(super) fn flight_event(
    now: u64,
    elapsed_ms: u64,
    stage: PromptFlightStage,
    kind: &str,
    label: &str,
    detail: String,
    data: BTreeMap<String, QueryValue>,
) -> Result<PromptFlightEvent> {
    Ok(PromptFlightEvent {
        ordinal: 0,
        at_unix_ms: now.saturating_add(elapsed_ms),
        elapsed_ms,
        stage,
        kind: CanonicalId::new(kind).map_err(|error| ServiceError::Contract(error.to_string()))?,
        label: label.into(),
        detail,
        data,
    })
}

/// The context stage event carries the engine-owned plan decisions, not the
/// packet items: a client renders which avenues were selected or skipped and
/// why, and fetches items through `context-assemble` under its own grant.
pub(super) fn context_assembled_event(
    now: u64,
    elapsed_ms: u64,
    packet: &ContextPacket,
) -> Result<PromptFlightEvent> {
    let stages = packet
        .plan
        .stages
        .iter()
        .map(|stage| {
            let mut entry = BTreeMap::from([
                (
                    "kind".to_owned(),
                    QueryValue::String(enum_text(&stage.kind)?),
                ),
                (
                    "status".to_owned(),
                    QueryValue::String(enum_text(&stage.status)?),
                ),
                ("exact".to_owned(), QueryValue::Bool(stage.exact)),
                (
                    "reason".to_owned(),
                    QueryValue::String(stage.reason.clone()),
                ),
                (
                    "decision_sha256".to_owned(),
                    QueryValue::Digest(stage.decision_sha256.clone()),
                ),
            ]);
            if let Some(path) = &stage.access_path {
                entry.insert(
                    "access_path".to_owned(),
                    QueryValue::String(enum_text(path)?),
                );
            }
            Ok(QueryValue::Map(entry))
        })
        .collect::<Result<Vec<_>>>()?;
    let (kind, label) = if packet.items.is_empty() {
        ("context_empty", "Prompt matched no canonical context")
    } else {
        ("context_assembled", "Prompt-matched context assembled")
    };
    flight_event(
        now,
        elapsed_ms,
        PromptFlightStage::Context,
        kind,
        label,
        format!(
            "{} item(s), {} byte(s), truncated={}, selected_versions={} at runtime cursor {}.",
            packet.items.len(),
            packet.output_bytes,
            packet.truncated,
            packet.selected_versions,
            packet.read.runtime_cursor
        ),
        BTreeMap::from([
            (
                "context_items".into(),
                QueryValue::Unsigned(packet.items.len() as u64),
            ),
            (
                "output_bytes".into(),
                QueryValue::Unsigned(packet.output_bytes),
            ),
            ("truncated".into(), QueryValue::Bool(packet.truncated)),
            (
                "plan_sha256".into(),
                QueryValue::Digest(packet.plan.plan_sha256.clone()),
            ),
            (
                "packet_sha256".into(),
                QueryValue::Digest(packet.packet_sha256.clone()),
            ),
            ("stages".into(), QueryValue::List(stages)),
        ]),
    )
}

pub(super) fn enum_text<T: Serialize>(value: &T) -> Result<String> {
    match serde_json::to_value(value).map_err(contract_json)? {
        serde_json::Value::String(text) => Ok(text),
        other => Err(ServiceError::Contract(format!(
            "context plan field {other} is not a text enumeration"
        ))),
    }
}

pub(super) fn context_summary(
    assemble: &rrd_contract::AssembleContext,
    packet: &ContextPacket,
) -> Result<PromptFlightContextSummary> {
    Ok(PromptFlightContextSummary {
        request_sha256: context_request_sha256(assemble)
            .map_err(|error| ServiceError::Contract(error.to_string()))?,
        plan_sha256: packet.plan.plan_sha256.clone(),
        packet_sha256: packet.packet_sha256.clone(),
        items: packet.items.len() as u64,
        output_bytes: packet.output_bytes,
        truncated: packet.truncated,
        selected_versions: packet.selected_versions,
    })
}
