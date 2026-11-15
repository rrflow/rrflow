use super::*;
use rrd_contract::{
    AssembleContext, ContextReadStamp, DataEngramOrganisation, DataPropertySchema,
    MemoryEngramPlan, MemoryWarp, PersistMemoryEngram, ProviderRepresentation, ResolveMemoryWarp,
    ResolveSeatIdentity, ResolvedMemoryWarp, SeatIdentity, MEMORY_PROVIDER_IDENTITY_KIND,
    MEMORY_REPRESENTS_KIND, MEMORY_SEAT_KIND,
};
use std::collections::BTreeSet;

impl RrdEngine {
    /// Builds canonical schema/record/relation mutations for one durable seat.
    /// The returned plan is committed through the existing authenticated data
    /// transaction API; this method creates no side registry or write path.
    #[allow(clippy::too_many_arguments)]
    pub fn plan_memory_engram(
        &self,
        session_id: &CorrelationId,
        token: &CorrelationId,
        request: &PersistMemoryEngram,
        now: u64,
        request_id: &str,
        operation_id: &str,
    ) -> Result<MemoryEngramPlan> {
        self.authorize(
            session_id,
            token,
            SecurityAction::TransactionPreview,
            now,
            request_id,
            operation_id,
        )?;
        let scope = self.query_scope(&request.scope)?;
        let current = self
            .storage
            .runtime()
            .schema(&scope)?
            .as_ref()
            .map(public_schema)
            .transpose()?;
        build_memory_engram_plan(request, current)
    }

    /// Resolves a stable `rrflow://` coordinate through the same context
    /// planner used by every other engine surface.
    #[allow(clippy::too_many_arguments)]
    pub fn resolve_memory_warp(
        &self,
        session_id: &CorrelationId,
        token: &CorrelationId,
        request: &ResolveMemoryWarp,
        now: u64,
        request_id: &str,
        operation_id: &str,
    ) -> Result<ResolvedMemoryWarp> {
        request
            .validate()
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        let (_, _, authorization) = self.authorize_resource_without_data_policy(
            session_id,
            token,
            SecurityAction::MemoryContextRead,
            &self.instance_resource(),
            now,
            request_id,
            operation_id,
        )?;
        self.query_scope(&request.scope)?;
        let warp = MemoryWarp::parse(&request.uri)
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        if warp.instance != self.instance {
            return Err(ServiceError::WrongScope);
        }
        let context = self.assemble_context_at(
            &AssembleContext {
                scope: request.scope.clone(),
                query: request.query.clone(),
                valid_at: request.valid_at,
                seeds: vec![warp.target.clone()],
                max_graph_depth: request.max_graph_depth,
                max_items: request.max_items,
                max_output_bytes: request.max_output_bytes,
                max_storage_keys: request.max_storage_keys,
            },
            authorization.as_ref(),
        )?;
        let expected = rrd_core::result_identity::typed(
            rrd_core::result_identity::ResultIdentityFamily::Record,
            &runtime_ref(&warp.target)?,
        );
        if !context.items.iter().any(|item| item.identity == expected) {
            return Err(ServiceError::MemoryTargetNotFound);
        }
        let resolved = ResolvedMemoryWarp {
            uri: warp.uri(),
            target: warp.target,
            context,
        };
        resolved
            .validate()
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        Ok(resolved)
    }

    /// Resolves a provider-independent seat as self only while at least one
    /// current provider identity represents it in the same temporal snapshot.
    #[allow(clippy::too_many_arguments)]
    pub fn resolve_seat_identity(
        &self,
        session_id: &CorrelationId,
        token: &CorrelationId,
        request: &ResolveSeatIdentity,
        now: u64,
        request_id: &str,
        operation_id: &str,
    ) -> Result<SeatIdentity> {
        request
            .validate()
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        self.authorize(
            session_id,
            token,
            SecurityAction::MemoryContextRead,
            now,
            request_id,
            operation_id,
        )?;
        let scope = self.query_scope(&request.scope)?;
        let key_budget = usize::try_from(request.max_storage_keys)
            .map_err(|_| ServiceError::Query("seat key budget exceeds usize".into()))?;
        let RuntimeDataSnapshotRead {
            read,
            selected_versions,
            read_evidence,
            snapshot,
        } = self
            .storage
            .runtime()
            .data_snapshot(&scope, request.valid_at, key_budget)?;
        if snapshot.known_at_cursor != read.commit_cursor
            || snapshot.schema_revision != read.schema_revision.unwrap_or(0)
        {
            return Err(ServiceError::StorageConflict(
                "seat snapshot differs from its captured read stamp".into(),
            ));
        }

        let seat_reference =
            RuntimeRef::new(MEMORY_SEAT_KIND, request.seat_id.as_str()).map_err(core_contract)?;
        let seat = snapshot
            .records
            .iter()
            .find(|entry| entry.value.reference == seat_reference)
            .ok_or(ServiceError::MemoryTargetNotFound)?;
        let display_name = required_string(&seat.value.properties, "display_name")?;
        let purpose = required_string(&seat.value.properties, "purpose")?;
        let provider_records = snapshot
            .records
            .iter()
            .filter(|entry| entry.value.reference.kind.as_str() == MEMORY_PROVIDER_IDENTITY_KIND)
            .map(|entry| (entry.value.reference.clone(), &entry.value))
            .collect::<BTreeMap<_, _>>();
        let mut representations = Vec::new();
        for relation in snapshot.relations.iter().filter(|entry| {
            entry.value.reference.kind.as_str() == MEMORY_REPRESENTS_KIND
                && entry.value.to == seat_reference
        }) {
            let Some(provider_identity) = provider_records.get(&relation.value.from) else {
                return Err(ServiceError::Storage(
                    "seat representation references a missing provider identity".into(),
                ));
            };
            let provider =
                CanonicalId::new(required_string(&provider_identity.properties, "provider")?)
                    .map_err(|error| ServiceError::Storage(error.to_string()))?;
            let subject_sha256 = required_digest(&provider_identity.properties, "subject_sha256")?;
            representations.push(ProviderRepresentation {
                id: canonical(relation.value.reference.id.as_str())?,
                provider_identity: canonical(provider_identity.reference.id.as_str())?,
                provider,
                subject_sha256,
            });
        }
        representations.sort_by(|left, right| left.id.cmp(&right.id));
        if representations.is_empty() {
            return Err(ServiceError::SeatNotRepresented);
        }
        let identity = SeatIdentity {
            uri: MemoryWarp::new(
                self.instance.clone(),
                DataReference {
                    kind: canonical(MEMORY_SEAT_KIND)?,
                    id: request.seat_id.clone(),
                },
            )
            .uri(),
            seat_id: request.seat_id.clone(),
            display_name,
            purpose,
            representations,
            read: ContextReadStamp {
                runtime_manifest_sha256: read.manifest_id,
                runtime_cursor: read.commit_cursor,
                schema_revision: read.schema_revision,
                catalogue_revision: read.catalog_revision,
            },
            selected_versions,
            read_evidence: crate::runtime::public_read_evidence(read_evidence)?,
        };
        identity
            .validate()
            .map_err(|error| ServiceError::Storage(error.to_string()))?;
        Ok(identity)
    }
}

/// Pure canonical memory-engram planner used by both authenticated previews
/// and the cold-start transaction. It performs no authentication or I/O.
pub(crate) fn build_memory_engram_plan(
    request: &PersistMemoryEngram,
    current: Option<DataSchemaRegistry>,
) -> Result<MemoryEngramPlan> {
    request
        .validate()
        .map_err(|error| ServiceError::Contract(error.to_string()))?;
    let (schema, schema_changed) = memory_engram_schema(current)?;
    let mut mutations = Vec::with_capacity(2 + request.representations.len() * 2);
    if schema_changed {
        mutations.push(TransactionMutation::PutSchema { registry: schema });
    }
    mutations.push(TransactionMutation::PutRecord {
        reference: DataReference {
            kind: canonical(MEMORY_SEAT_KIND)?,
            id: request.seat.id.clone(),
        },
        valid_from: request.valid_from,
        valid_to: None,
        properties: BTreeMap::from([
            (
                "display_name".into(),
                QueryValue::String(request.seat.display_name.clone()),
            ),
            (
                "purpose".into(),
                QueryValue::String(request.seat.purpose.clone()),
            ),
        ]),
    });
    let mut representations = request.representations.clone();
    representations.sort_by(|left, right| left.id.cmp(&right.id));
    for representation in representations {
        let provider_reference = DataReference {
            kind: canonical(MEMORY_PROVIDER_IDENTITY_KIND)?,
            id: representation.provider_identity,
        };
        mutations.push(TransactionMutation::PutRecord {
            reference: provider_reference.clone(),
            valid_from: request.valid_from,
            valid_to: None,
            properties: BTreeMap::from([
                (
                    "provider".into(),
                    QueryValue::String(representation.provider.to_string()),
                ),
                (
                    "subject_sha256".into(),
                    QueryValue::Digest(representation.subject_sha256),
                ),
            ]),
        });
        mutations.push(TransactionMutation::PutRelation {
            reference: DataReference {
                kind: canonical(MEMORY_REPRESENTS_KIND)?,
                id: representation.id,
            },
            from: provider_reference,
            to: DataReference {
                kind: canonical(MEMORY_SEAT_KIND)?,
                id: request.seat.id.clone(),
            },
            valid_from: request.valid_from,
            valid_to: None,
            properties: BTreeMap::new(),
        });
    }
    let plan = MemoryEngramPlan {
        operation_sha256: transaction_operation_sha256(&mutations),
        mutations,
    };
    plan.validate()
        .map_err(|error| ServiceError::Contract(error.to_string()))?;
    Ok(plan)
}

fn memory_engram_schema(current: Option<DataSchemaRegistry>) -> Result<(DataSchemaRegistry, bool)> {
    let mut registry = current.unwrap_or_else(|| DataSchemaRegistry {
        revision: 0,
        migration: "memory engram bootstrap".into(),
        organisation: DataEngramOrganisation::default(),
        tables: BTreeMap::new(),
        records: BTreeMap::new(),
        relations: BTreeMap::new(),
        events: BTreeMap::new(),
    });
    let seat_kind = canonical(MEMORY_SEAT_KIND)?;
    let provider_kind = canonical(MEMORY_PROVIDER_IDENTITY_KIND)?;
    let represents_kind = canonical(MEMORY_REPRESENTS_KIND)?;
    let seat_properties = BTreeMap::from([
        ("display_name".into(), required(DataValueType::String)),
        ("purpose".into(), required(DataValueType::String)),
    ]);
    let provider_properties = BTreeMap::from([
        ("provider".into(), required(DataValueType::String)),
        ("subject_sha256".into(), required(DataValueType::Digest)),
    ]);
    let expected_tables = [
        (
            seat_kind.clone(),
            DataTableSchema {
                model: DataLogicalModel::GraphNode,
                mode: DataSchemaMode::Strict,
                properties: BTreeMap::new(),
                allow_additional_properties: false,
            },
        ),
        (
            provider_kind.clone(),
            DataTableSchema {
                model: DataLogicalModel::GraphNode,
                mode: DataSchemaMode::Strict,
                properties: BTreeMap::new(),
                allow_additional_properties: false,
            },
        ),
        (
            represents_kind.clone(),
            DataTableSchema {
                model: DataLogicalModel::GraphRelation,
                mode: DataSchemaMode::Strict,
                properties: BTreeMap::new(),
                allow_additional_properties: false,
            },
        ),
    ];
    let expected_records = [
        (
            seat_kind.clone(),
            DataRecordSchema {
                properties: seat_properties,
                allow_additional_properties: false,
                unique_properties: BTreeSet::new(),
            },
        ),
        (
            provider_kind.clone(),
            DataRecordSchema {
                properties: provider_properties,
                allow_additional_properties: false,
                unique_properties: BTreeSet::new(),
            },
        ),
    ];
    let expected_relation = DataRelationSchema {
        from: BTreeSet::from([provider_kind]),
        to: BTreeSet::from([seat_kind]),
        properties: BTreeMap::new(),
        allow_additional_properties: false,
        unique_pair: true,
        max_outgoing: Some(1),
        max_incoming: None,
    };
    let mut changed = false;
    for (kind, expected) in expected_tables {
        changed |= install_exact(&mut registry.tables, kind, expected, "table")?;
    }
    for (kind, expected) in expected_records {
        changed |= install_exact(&mut registry.records, kind, expected, "record schema")?;
    }
    changed |= install_exact(
        &mut registry.relations,
        represents_kind,
        expected_relation,
        "relation schema",
    )?;
    if changed {
        registry.revision = registry.revision.checked_add(1).ok_or_else(|| {
            ServiceError::Contract("memory engram schema revision overflowed".into())
        })?;
        registry.migration = "install rrflow memory engram v1".into();
    }
    Ok((registry, changed))
}

fn install_exact<T: PartialEq>(
    entries: &mut BTreeMap<CanonicalId, T>,
    kind: CanonicalId,
    expected: T,
    description: &str,
) -> Result<bool> {
    match entries.get(&kind) {
        Some(existing) if existing == &expected => Ok(false),
        Some(_) => Err(ServiceError::StorageConflict(format!(
            "memory engram {description} {kind} conflicts with the canonical definition"
        ))),
        None => {
            entries.insert(kind, expected);
            Ok(true)
        }
    }
}

fn required(value_type: DataValueType) -> DataPropertySchema {
    DataPropertySchema {
        value_type,
        required: true,
    }
}

pub(super) fn canonical(value: &str) -> Result<CanonicalId> {
    CanonicalId::new(value).map_err(|error| ServiceError::Contract(error.to_string()))
}

pub(super) fn required_string(properties: &RuntimeProperties, name: &str) -> Result<String> {
    match properties.get(name) {
        Some(RuntimeValue::String(value)) => Ok(value.clone()),
        _ => Err(ServiceError::Storage(format!(
            "memory engram record lacks string property {name}"
        ))),
    }
}

pub(super) fn required_digest(properties: &RuntimeProperties, name: &str) -> Result<String> {
    match properties.get(name) {
        Some(RuntimeValue::Digest(value)) => Ok(value.clone()),
        _ => Err(ServiceError::Storage(format!(
            "memory engram record lacks digest property {name}"
        ))),
    }
}
