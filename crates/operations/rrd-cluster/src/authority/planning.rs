//! Planning distributed authority placement.

use super::*;

pub fn plan_distributed_authority(
    cluster: ClusterId,
    revision: u64,
    updated_at: u64,
    nodes: Vec<PlacementNode>,
    tenants: Vec<TenantPlacementPolicy>,
    tables: Vec<LogicalTablePolicy>,
) -> Result<DistributedAuthorityCatalogue> {
    let mut node_map = BTreeMap::new();
    for node in nodes {
        let key = node.node.clone();
        if node_map.insert(key, node).is_some() {
            return Err(ClusterError::Invalid(
                "placement input contains a duplicate node".into(),
            ));
        }
    }
    let mut tenant_map = BTreeMap::new();
    for tenant in tenants {
        let key = tenant.tenant.clone();
        if tenant_map.insert(key, tenant).is_some() {
            return Err(ClusterError::Invalid(
                "placement input contains a duplicate tenant".into(),
            ));
        }
    }
    let mut table_map = BTreeMap::new();
    for table in tables {
        let key = table.table.clone();
        if table_map.insert(key, table).is_some() {
            return Err(ClusterError::Invalid(
                "placement input contains a duplicate logical table".into(),
            ));
        }
    }

    let mut shards = BTreeMap::new();
    let mut voter_load = BTreeMap::<NodeId, u32>::new();
    let mut learner_load = BTreeMap::<NodeId, u32>::new();
    for table in table_map.values() {
        let tenant = tenant_map
            .get(&table.tenant)
            .ok_or_else(|| ClusterError::Invalid("logical table names an unknown tenant".into()))?;
        for ordinal in 0..table.shard_count {
            let shard_value = table
                .first_shard
                .0
                .checked_add(u64::from(ordinal))
                .ok_or_else(|| ClusterError::Invalid("shard id allocation overflowed".into()))?;
            let shard = ShardId(shard_value);
            if shard == METADATA_SHARD_ID || shards.contains_key(&shard) {
                return Err(ClusterError::Invalid(
                    "logical table shard allocation overlaps metadata or another table".into(),
                ));
            }
            let replicas = select_replicas(&node_map, tenant, &mut voter_load, &mut learner_load)?;
            let placement = ShardPlacement {
                contract_version: CLUSTER_CONTRACT_VERSION,
                cluster: cluster.clone(),
                shard,
                epoch: 1,
                policy: tenant.placement.clone(),
                replicas,
            };
            placement.validate()?;
            let replica_health = placement
                .replicas
                .iter()
                .map(|replica| (replica.node.clone(), ReplicaHealth::Active))
                .collect();
            shards.insert(
                shard,
                ShardAuthority {
                    shard,
                    table: table.table.clone(),
                    range: hash_range(ordinal, table.shard_count),
                    placement,
                    replica_health,
                },
            );
        }
    }

    let catalogue = DistributedAuthorityCatalogue {
        format: DISTRIBUTED_AUTHORITY_FORMAT,
        cluster,
        metadata_shard: METADATA_SHARD_ID,
        revision,
        updated_at,
        nodes: node_map,
        tenants: tenant_map,
        tables: table_map,
        shards,
        transfers: BTreeMap::new(),
        reshards: BTreeMap::new(),
    };
    catalogue.validate()?;
    Ok(catalogue)
}

pub(super) fn select_replicas(
    nodes: &BTreeMap<NodeId, PlacementNode>,
    tenant: &TenantPlacementPolicy,
    voter_load: &mut BTreeMap<NodeId, u32>,
    learner_load: &mut BTreeMap<NodeId, u32>,
) -> Result<Vec<ReplicaPlacement>> {
    let candidates = nodes
        .values()
        .filter(|node| {
            node.state == PlacementNodeState::Active
                && tenant.allowed_regions.contains(&node.region)
        })
        .collect::<Vec<_>>();
    let mut selected = BTreeSet::<NodeId>::new();
    let mut voter_regions = BTreeSet::<RegionId>::new();
    let mut voter_zones = BTreeMap::<ZoneId, u8>::new();
    let mut replicas = Vec::new();

    while replicas.len() < usize::from(tenant.placement.voter_count) {
        let need_region = voter_regions.len() < usize::from(tenant.minimum_voter_regions);
        let need_zone = voter_zones.len() < usize::from(tenant.placement.minimum_voter_zones);
        let mut eligible = candidates
            .iter()
            .copied()
            .filter(|node| !selected.contains(&node.node))
            .filter(|node| {
                voter_load.get(&node.node).copied().unwrap_or(0) < node.voter_capacity
                    && voter_zones.get(&node.zone).copied().unwrap_or(0)
                        < tenant.placement.maximum_voters_per_zone
            })
            .collect::<Vec<_>>();
        eligible.sort_by_key(|node| {
            (
                need_region && voter_regions.contains(&node.region),
                need_zone && voter_zones.contains_key(&node.zone),
                voter_load.get(&node.node).copied().unwrap_or(0),
                voter_zones.get(&node.zone).copied().unwrap_or(0),
                node.node.clone(),
            )
        });
        let node = eligible.first().copied().ok_or_else(|| {
            ClusterError::Unavailable(
                "active node capacity cannot satisfy voter region/zone placement".into(),
            )
        })?;
        selected.insert(node.node.clone());
        voter_regions.insert(node.region.clone());
        *voter_zones.entry(node.zone.clone()).or_default() += 1;
        *voter_load.entry(node.node.clone()).or_default() += 1;
        replicas.push(ReplicaPlacement {
            node: node.node.clone(),
            zone: node.zone.clone(),
            role: ReplicaRole::Voter,
        });
    }
    if voter_regions.len() < usize::from(tenant.minimum_voter_regions) {
        return Err(ClusterError::Unavailable(
            "selected voters cannot satisfy tenant region diversity".into(),
        ));
    }

    for _ in 0..tenant.read_replica_count {
        let mut eligible = candidates
            .iter()
            .copied()
            .filter(|node| !selected.contains(&node.node))
            .filter(|node| {
                learner_load.get(&node.node).copied().unwrap_or(0) < node.learner_capacity
            })
            .collect::<Vec<_>>();
        eligible.sort_by_key(|node| {
            (
                learner_load.get(&node.node).copied().unwrap_or(0),
                node.node.clone(),
            )
        });
        let node = eligible.first().copied().ok_or_else(|| {
            ClusterError::Unavailable(
                "active node capacity cannot satisfy requested read replicas".into(),
            )
        })?;
        selected.insert(node.node.clone());
        *learner_load.entry(node.node.clone()).or_default() += 1;
        replicas.push(ReplicaPlacement {
            node: node.node.clone(),
            zone: node.zone.clone(),
            role: ReplicaRole::Learner,
        });
    }
    replicas.sort_by(|left, right| left.node.cmp(&right.node));
    Ok(replicas)
}

pub(super) fn hash_range(ordinal: u16, count: u16) -> ShardHashRange {
    let space = 1_u128 << 64;
    let start = u128::from(ordinal) * space / u128::from(count);
    let end_exclusive = (u128::from(ordinal) + 1) * space / u128::from(count);
    ShardHashRange {
        start_inclusive: start as u64,
        end_inclusive: (end_exclusive - 1) as u64,
    }
}

pub(super) fn authority_schema_transition(
    current: Option<&RuntimeSchemaRegistry>,
    kind: &RuntimeType,
) -> Result<Option<RuntimeSchemaRegistry>> {
    let record_schema = authority_record_schema();
    let table_schema = RuntimeTableSchema::strict(RuntimeLogicalModel::KeyValue);
    if let Some(current) = current {
        let tables = current
            .catalogue_tables()
            .map_err(|error| ClusterError::Invalid(error.to_string()))?;
        match (tables.get(kind), current.records.get(kind)) {
            (Some(table), Some(record)) if table == &table_schema && record == &record_schema => {
                return Ok(None);
            }
            (Some(_), _) | (_, Some(_)) => {
                return Err(ClusterError::Denied(
                    "distributed authority runtime type collides with another schema".into(),
                ));
            }
            (None, None) => {}
        }
        let mut next = current.clone();
        next.revision = next
            .revision
            .checked_add(1)
            .ok_or_else(|| ClusterError::Invalid("runtime schema revision overflowed".into()))?;
        next.migration = "install distributed metadata-shard authority".into();
        next.tables = tables;
        next.define_record_table(kind.clone(), RuntimeLogicalModel::KeyValue, record_schema)
            .map_err(|error| ClusterError::Invalid(error.to_string()))?;
        next.validate()
            .map_err(|error| ClusterError::Invalid(error.to_string()))?;
        return Ok(Some(next));
    }

    let mut registry =
        RuntimeSchemaRegistry::empty(1, "install distributed metadata-shard authority");
    registry
        .define_record_table(kind.clone(), RuntimeLogicalModel::KeyValue, record_schema)
        .map_err(|error| ClusterError::Invalid(error.to_string()))?;
    registry
        .validate()
        .map_err(|error| ClusterError::Invalid(error.to_string()))?;
    Ok(Some(registry))
}

pub(super) fn authority_record_schema() -> RuntimeRecordSchema {
    RuntimeRecordSchema {
        properties: BTreeMap::from([
            (
                "catalogue_sha256".into(),
                RuntimePropertySchema::required(RuntimeValueType::Digest),
            ),
            (
                "cluster".into(),
                RuntimePropertySchema::required(RuntimeValueType::String),
            ),
            (
                "contract_version".into(),
                RuntimePropertySchema::required(RuntimeValueType::Unsigned),
            ),
            (
                "payload".into(),
                RuntimePropertySchema::required(RuntimeValueType::String),
            ),
            (
                "revision".into(),
                RuntimePropertySchema::required(RuntimeValueType::Unsigned),
            ),
            (
                "updated_at".into(),
                RuntimePropertySchema::required(RuntimeValueType::Unsigned),
            ),
        ]),
        allow_additional_properties: false,
        unique_properties: BTreeSet::new(),
    }
}

pub(super) fn property_string<'a>(properties: &'a RuntimeProperties, key: &str) -> Result<&'a str> {
    match properties.get(key) {
        Some(RuntimeValue::String(value)) => Ok(value),
        _ => Err(ClusterError::Denied(format!(
            "distributed authority runtime property {key} has the wrong type"
        ))),
    }
}

pub(super) fn property_digest<'a>(properties: &'a RuntimeProperties, key: &str) -> Result<&'a str> {
    match properties.get(key) {
        Some(RuntimeValue::Digest(value)) if is_sha256(value) => Ok(value),
        _ => Err(ClusterError::Denied(format!(
            "distributed authority runtime property {key} is not a SHA-256 digest"
        ))),
    }
}

pub(super) fn property_unsigned(properties: &RuntimeProperties, key: &str) -> Result<u64> {
    match properties.get(key) {
        Some(RuntimeValue::Unsigned(value)) => Ok(*value),
        _ => Err(ClusterError::Denied(format!(
            "distributed authority runtime property {key} has the wrong type"
        ))),
    }
}

pub(super) fn validate_text(value: &str, label: &str) -> Result<()> {
    if value.is_empty() || value.len() > 256 || value.as_bytes().contains(&0) {
        return Err(ClusterError::Invalid(format!(
            "{label} must contain 1..=256 non-NUL bytes"
        )));
    }
    Ok(())
}

pub(super) fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

pub(super) fn reshard_rank(state: ReshardState) -> u8 {
    match state {
        ReshardState::Planned => 0,
        ReshardState::Copying => 1,
        ReshardState::CaughtUp => 2,
        ReshardState::Cutover => 3,
        ReshardState::Retired => 4,
    }
}

pub(super) fn invalid_encoding(error: serde_json::Error) -> ClusterError {
    ClusterError::Invalid(format!("distributed authority encoding failed: {error}"))
}
