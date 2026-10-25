use super::*;

pub(super) fn parse_correlation(value: &str) -> std::result::Result<CorrelationId, ApiError> {
    CorrelationId::new(value)
        .map_err(|error| ApiError::new(ErrorCode::InvalidArgument, error.to_string(), false))
}

pub(super) fn session_action<'a>(path: &'a str, action: &str) -> Option<&'a str> {
    path.strip_prefix("/v1/sessions/")?
        .strip_suffix(&format!("/{action}"))
        .filter(|id| !id.is_empty() && !id.contains('/'))
}

pub(super) fn session_id(path: &str) -> Option<&str> {
    let id = path.strip_prefix("/v1/sessions/")?;
    (!id.is_empty() && !id.contains('/')).then_some(id)
}

pub(super) fn transaction_action<'a>(path: &'a str, action: &str) -> Option<&'a str> {
    path.strip_prefix("/v1/transactions/")?
        .strip_suffix(&format!("/{action}"))
        .filter(|id| !id.is_empty() && !id.contains('/'))
}

pub(super) fn transaction_id(path: &str) -> Option<&str> {
    let id = path.strip_prefix("/v1/transactions/")?;
    (!id.is_empty() && !id.contains('/')).then_some(id)
}

pub(super) fn estate_action<'a>(path: &'a str, action: &str) -> Option<&'a str> {
    path.strip_prefix("/v1/estates/")?
        .strip_suffix(&format!("/{action}"))
        .filter(|id| !id.is_empty() && !id.contains('/'))
}

pub(super) fn capabilities(
    engine: &RrdEngine,
    instance: &CanonicalId,
    deployment: DeploymentProfile,
    security_enforced: bool,
    tls_enabled: bool,
    jwt_enabled: bool,
) -> ServiceCapabilities {
    let configuration = engine.estate_configuration();
    let backend = match engine.storage_profile_kind() {
        rrd_contract::StorageProfileKind::RrflowMx => "rrflow_mx",
        rrd_contract::StorageProfileKind::RrflowKv => "rrflow_kv",
    };
    let mut capabilities = vec![
        CapabilityDescriptor {
            name: CanonicalId::new("changefeed-follow").unwrap(),
            contract_version: 1,
            status: CapabilityStatus::Experimental,
            limits: BTreeMap::from([(
                CanonicalId::new("max-wait-ms").unwrap(),
                rrd_contract::MAX_CHANGEFEED_WAIT_MS,
            )]),
            limitation: Some(
                "bounded authenticated long-poll fallback over the same commit-ordered replay used by durable WebSocket subscriptions"
                    .into(),
            ),
        },
        CapabilityDescriptor {
            name: CanonicalId::new("changefeed-replay").unwrap(),
            contract_version: 1,
            status: CapabilityStatus::Experimental,
            limits: BTreeMap::from([(
                CanonicalId::new("max-page-changes").unwrap(),
                rrd_contract::MAX_CHANGEFEED_PAGE,
            )]),
            limitation: Some(
                "authenticated retained cursor replay with lossless claim and typed data snapshots"
                    .into(),
            ),
        },
        CapabilityDescriptor {
            name: CanonicalId::new("claim-transactions").unwrap(),
            contract_version: 1,
            status: CapabilityStatus::Experimental,
            limits: BTreeMap::from([(
                CanonicalId::new("max-mutations").unwrap(),
                rrd_contract::MAX_TRANSACTION_CLAIMS as u64,
            )]),
            limitation: Some(format!(
                "schema-less reasoning-claim transactions on {backend}; schema-bound models use the multi-model transaction contract"
            )),
        },
        CapabilityDescriptor {
            name: CanonicalId::new("multi-model-transactions").unwrap(),
            contract_version: 1,
            status: CapabilityStatus::Experimental,
            limits: BTreeMap::from([(
                CanonicalId::new("max-mutations").unwrap(),
                rrd_contract::MAX_TRANSACTION_CLAIMS as u64,
            )]),
            limitation: Some(
                "atomic schema, claim, record, relation, event, vector, series, geo, and pre-staged object-reference commits with durable all-model prospective read-your-writes; mutating rrflowQL remains open"
                    .into(),
            ),
        },
        CapabilityDescriptor {
            name: CanonicalId::new("context-assembly").unwrap(),
            contract_version: 1,
            status: CapabilityStatus::Experimental,
            limits: BTreeMap::from([
                (
                    CanonicalId::new("max-graph-depth").unwrap(),
                    u64::from(configuration.recall.max_graph_depth),
                ),
                (
                    CanonicalId::new("max-items").unwrap(),
                    configuration.recall.max_items,
                ),
                (
                    CanonicalId::new("max-output-bytes").unwrap(),
                    configuration.recall.max_output_bytes,
                ),
                (
                    CanonicalId::new("max-storage-keys").unwrap(),
                    configuration.recall.max_storage_keys,
                ),
            ]),
            limitation: Some(
                "bounded authenticated temporal context assembly with source and read evidence; attunement and release recall-quality gates remain open"
                    .into(),
            ),
        },
        CapabilityDescriptor {
            name: CanonicalId::new("exact-rrflowql-query").unwrap(),
            contract_version: 1,
            status: CapabilityStatus::Experimental,
            limits: BTreeMap::from([
                (
                    CanonicalId::new("max-query-bytes").unwrap(),
                    rrd_contract::MAX_QUERY_BYTES as u64,
                ),
                (
                    CanonicalId::new("max-output-bytes").unwrap(),
                    configuration.query.max_output_bytes,
                ),
                (
                    CanonicalId::new("max-rows").unwrap(),
                    configuration.query.max_rows,
                ),
                (
                    CanonicalId::new("max-storage-keys").unwrap(),
                    configuration.query.max_storage_keys,
                ),
                (
                    CanonicalId::new("max-memory-bytes").unwrap(),
                    configuration.query.max_memory_bytes,
                ),
                (
                    CanonicalId::new("max-spill-bytes").unwrap(),
                    configuration.query.max_spill_bytes,
                ),
                (
                    CanonicalId::new("max-elapsed-ms").unwrap(),
                    configuration.query.max_elapsed_ms,
                ),
            ]),
            limitation: Some(
                "exact session-scoped rrflowQL reads over authenticated typed versions with Arrow/DataFusion compute; mutating rrflowQL remains open"
                    .into(),
            ),
        },
        CapabilityDescriptor {
            name: CanonicalId::new("endpoint-catalogue").unwrap(),
            contract_version: 1,
            status: CapabilityStatus::Available,
            limits: BTreeMap::from([
                (
                    CanonicalId::new("http-endpoint-count").unwrap(),
                    rrd_contract::endpoint_catalogue().endpoints.len() as u64,
                ),
                (
                    CanonicalId::new("websocket-endpoint-count").unwrap(),
                    rrd_contract::endpoint_catalogue().websocket_endpoints.len() as u64,
                ),
            ]),
            limitation: Some(
                "machine-readable operation catalogue and OpenAPI 3.1 schemas; generated language packages and shared conformance remain F5 work"
                    .into(),
            ),
        },
        CapabilityDescriptor {
            name: CanonicalId::new(rrd_contract::PARQUET_EXPORT_PROFILE).unwrap(),
            contract_version: 1,
            status: CapabilityStatus::Experimental,
            limits: BTreeMap::from([
                (CanonicalId::new("max-rows").unwrap(),
                    configuration.query.max_rows.min(rrd_contract::MAX_PARQUET_EXPORT_ROWS as u64)),
                (CanonicalId::new("max-fields").unwrap(), rrd_contract::MAX_PARQUET_EXPORT_COLUMNS as u64),
                (CanonicalId::new("max-source-bytes").unwrap(), rrd_contract::MAX_PARQUET_EXPORT_INPUT_BYTES as u64),
                (CanonicalId::new("max-output-bytes").unwrap(),
                    configuration.query.max_output_bytes.min(rrd_contract::MAX_PARQUET_EXPORT_BYTES as u64)),
                (CanonicalId::new("row-group-rows").unwrap(), rrd_contract::MAX_PARQUET_ROW_GROUP_ROWS as u64),
                (CanonicalId::new("max-memory-bytes").unwrap(), configuration.query.max_memory_bytes),
                (CanonicalId::new("max-elapsed-ms").unwrap(), configuration.query.max_elapsed_ms),
            ]),
            limitation: Some(
                "authenticated bounded query-result export only; uncompressed typed Arrow profile; no import, streaming, destination writes or global allocator qualification".into(),
            ),
        },
        CapabilityDescriptor {
            name: CanonicalId::new("estate-authority-read").unwrap(),
            contract_version: 1,
            status: CapabilityStatus::Experimental,
            limits: BTreeMap::new(),
            limitation: Some(
                "read-only estate snapshots; estate mutations remain unavailable"
                    .into(),
            ),
        },
        CapabilityDescriptor {
            name: CanonicalId::new("governed-reasoning-runner").unwrap(),
            contract_version: 1,
            status: CapabilityStatus::Unavailable,
            limits: BTreeMap::from([
                (
                    CanonicalId::new("max-run-elapsed-ms").unwrap(),
                    configuration.reasoning.max_run_elapsed_ms,
                ),
                (
                    CanonicalId::new("max-step-elapsed-ms").unwrap(),
                    configuration.reasoning.max_step_elapsed_ms,
                ),
                (
                    CanonicalId::new("max-steps").unwrap(),
                    configuration.reasoning.max_steps,
                ),
            ]),
            limitation: Some(
                "operator ceilings are configured, but the governed reasoning executor and activation receipts are not implemented; private exploratory reasoning remains outside this durable-effect capability"
                    .into(),
            ),
        },
        CapabilityDescriptor {
            name: CanonicalId::new("jwt-session-credentials").unwrap(),
            contract_version: 1,
            status: if jwt_enabled {
                CapabilityStatus::Available
            } else {
                CapabilityStatus::Unavailable
            },
            limits: BTreeMap::from([(
                CanonicalId::new("max-token-bytes").unwrap(),
                rrd_engine::MAX_JWT_CREDENTIAL_BYTES,
            )]),
            limitation: Some(if jwt_enabled {
                "RRD-issued HS256 bearer exchange bound to issuer, audience, key id, principal, credential revision, expiry, and current policy"
                    .into()
            } else {
                "configure an owner-only JWT key file and matching persisted issuer"
                    .into()
            }),
        },
        CapabilityDescriptor {
            name: CanonicalId::new("knowledge-tree-catalogue").unwrap(),
            contract_version: rrd_contract::KNOWLEDGE_TREE_CONTRACT_VERSION,
            status: if engine.installed_estate_identity().is_some() {
                CapabilityStatus::Experimental
            } else {
                CapabilityStatus::Unavailable
            },
            limits: BTreeMap::from([
                (CanonicalId::new("max-trees").unwrap(), rrd_contract::MAX_KNOWLEDGE_TREES as u64),
                (CanonicalId::new("max-paths").unwrap(), rrd_contract::MAX_KNOWLEDGE_PATHS as u64),
                (CanonicalId::new("max-catalogue-bytes").unwrap(), rrd_contract::MAX_KNOWLEDGE_CATALOGUE_BYTES as u64),
            ]),
            limitation: Some(if engine.installed_estate_identity().is_some() {
                "authenticated project-bound catalogue configuration with revision compare-and-swap and idempotent receipts; external bindings do not establish provider execution or synchronization".into()
            } else {
                "knowledge trees require the identity of an installed project estate".into()
            }),
        },
        CapabilityDescriptor {
            name: CanonicalId::new("lifecycle-journal").unwrap(),
            contract_version: 1,
            status: CapabilityStatus::Available,
            limits: BTreeMap::new(),
            limitation: None,
        },
        CapabilityDescriptor {
            name: CanonicalId::new("live-subscriptions").unwrap(),
            contract_version: 1,
            status: CapabilityStatus::Available,
            limits: BTreeMap::from([
                (
                    CanonicalId::new("max-in-flight").unwrap(),
                    u64::from(rrd_contract::MAX_SUBSCRIPTION_IN_FLIGHT),
                ),
                (
                    CanonicalId::new("max-retention-cursors").unwrap(),
                    rrd_contract::MAX_SUBSCRIPTION_RETENTION_CURSORS,
                ),
            ]),
            limitation: Some(
                "authenticated WebSocket push with durable ACK cursors, fenced reconnect, bounded delivery windows, and one global commit-cursor order"
                    .into(),
            ),
        },
        CapabilityDescriptor {
            name: CanonicalId::new("local-transport-leases").unwrap(),
            contract_version: 1,
            status: CapabilityStatus::Experimental,
            limits: BTreeMap::from([(
                CanonicalId::new("max-body-bytes").unwrap(),
                RRD_MAX_BODY_BYTES as u64,
            )]),
            limitation: Some(if security_enforced {
                if tls_enabled {
                    "API-key or configured JWT principal sessions over TLS 1.3 mutual authentication"
                        .into()
                } else if jwt_enabled {
                    "API-key or JWT principal sessions on loopback; configure mTLS for remote transport"
                        .into()
                } else {
                    "principal-authenticated policy-bound loopback sessions; configure mTLS for remote transport"
                        .into()
                }
            } else {
                "development availability leases; no security authority is initialized".into()
            }),
        },
        CapabilityDescriptor {
            name: CanonicalId::new("logical-backup-restore").unwrap(),
            contract_version: 1,
            status: CapabilityStatus::Experimental,
            limits: BTreeMap::new(),
            limitation: Some(
                "content-authenticated logical backup and restore-to-generated-new-root; object payloads are referenced-only and restore never switches the active instance"
                    .into(),
            ),
        },
        CapabilityDescriptor {
            name: CanonicalId::new("remote-listen").unwrap(),
            contract_version: 1,
            status: if tls_enabled {
                CapabilityStatus::Experimental
            } else {
                CapabilityStatus::Unavailable
            },
            limits: BTreeMap::new(),
            limitation: Some(if tls_enabled {
                "TLS 1.3 mTLS plus application policy are enforced; certificate reload/revocation and distributed qualification remain open"
                    .into()
            } else {
                "plain HTTP is loopback-only; configure mTLS and initialize security for remote bind"
                    .into()
            }),
        },
        CapabilityDescriptor {
            name: CanonicalId::new("security-policy").unwrap(),
            contract_version: 1,
            status: if security_enforced {
                CapabilityStatus::Experimental
            } else {
                CapabilityStatus::Unavailable
            },
            limits: BTreeMap::new(),
            limitation: Some(if security_enforced {
                "revisioned principals, inherited roles, credentials, issuers, external identity bindings, exact resource grants, and compiled tenant/row/field rrflowQL policy; constrained operations without an injector deny"
                    .into()
            } else {
                "no persistent security authority is initialized for this instance".into()
            }),
        },
        CapabilityDescriptor {
            name: CanonicalId::new("security-audit").unwrap(),
            contract_version: 1,
            status: if security_enforced {
                CapabilityStatus::Experimental
            } else {
                CapabilityStatus::Unavailable
            },
            limits: BTreeMap::from([(
                CanonicalId::new("max-page-records").unwrap(),
                MAX_AUDIT_PAGE_RECORDS,
            )]),
            limitation: Some(if security_enforced {
                "independently hash-chained redacted JSON audit with protected bounded read and canonical JSON Lines export; external sink delivery remains adapter-owned"
                    .into()
            } else {
                "audit requires initialized persistent security authority".into()
            }),
        },
        CapabilityDescriptor {
            name: CanonicalId::new("vector-search").unwrap(),
            contract_version: 1,
            status: CapabilityStatus::Experimental,
            limits: BTreeMap::from([
                (
                    CanonicalId::new("max-storage-keys").unwrap(),
                    rrd_contract::MAX_VECTOR_STORAGE_KEYS,
                ),
                (
                    CanonicalId::new("max-top-k").unwrap(),
                    rrd_contract::MAX_VECTOR_SEARCH_TOP_K,
                ),
            ]),
            limitation: Some(
                "named collections, typed payload indexes and filters, direct stamped dense/sparse/multi-vector reads, and persisted HNSW/quantization/TurboQuant artifact serving; release recall-quality gates remain open"
                    .into(),
            ),
        },
    ];
    capabilities.sort_by(|left, right| left.name.cmp(&right.name));
    ServiceCapabilities {
        protocol: PROTOCOL.into(),
        protocol_version: PROTOCOL_VERSION,
        implementation: CanonicalId::new("rrflow").unwrap(),
        implementation_version: env!("CARGO_PKG_VERSION").into(),
        deployment,
        installed_estate: engine.installed_estate_identity().cloned(),
        configuration: configuration.clone(),
        instance: ResourceId {
            kind: ResourceKind::Instance,
            id: instance.clone(),
        },
        capabilities,
        product_capabilities: product_capability_catalogue(),
    }
}

pub(super) fn unix_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}
