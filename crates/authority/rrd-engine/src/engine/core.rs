use super::invocation::ActiveInvocation;
use super::*;
use rrd_store::filesystem::AnchoredPath;

/// One checked lowering of installed policy into the native storage owner.
/// The locator carries a projection for pre-open admission, subsequently
/// compared with the sealed canonical configuration before readiness.
pub(super) fn installed_storage_options(
    policy: &rrd_contract::StorageLimits,
) -> Result<rrd_store::DatabaseOptions> {
    policy
        .validate()
        .map_err(|error| ServiceError::Contract(error.to_string()))?;
    let native = |name: &str, value: u64| {
        usize::try_from(value).map_err(|_| {
            ServiceError::Contract(format!(
                "storage {name} exceeds this platform's address space"
            ))
        })
    };
    let mut options = rrd_store::DatabaseOptions {
        recovery_reserve_bytes: policy.recovery_reserve_bytes,
        ..rrd_store::DatabaseOptions::default()
    };
    options.mutable_limits.accounted_bytes =
        native("mutable_accounted_bytes", policy.mutable_accounted_bytes)?;
    options.mutable_limits.versions = native("mutable_versions", policy.mutable_versions)?;
    options.projected_output_bytes =
        native("projected_output_bytes", policy.projected_output_bytes)?;
    options.page_cache_bytes = native("page_cache_bytes", policy.page_cache_bytes)?;
    options.maintenance.wal_payload_max_bytes =
        native("wal_flush_target_bytes", policy.wal_flush_target_bytes)?;
    options.maintenance.memtable_max_versions = native(
        "memtable_flush_target_versions",
        policy.memtable_flush_target_versions,
    )?;
    options.compaction_limits.input_bytes = policy.compaction_input_bytes;
    options.compaction_limits.input_logical_bytes = policy.compaction_input_logical_bytes;
    options.compaction_limits.input_versions = policy.compaction_input_versions;
    options.compaction_limits.output_segments = policy.compaction_output_segments;
    options.compaction_limits.elapsed_ms = policy.compaction_elapsed_ms;
    options.compaction_limits.l0_segments = policy.compaction_l0_segments;
    options.mutable_retention.accounted_bytes =
        native("retained_mutable_bytes", policy.retained_mutable_bytes)?;
    options.mutable_retention.versions = native(
        "retained_mutable_versions",
        policy.retained_mutable_versions,
    )?;
    options.mutable_retention.generations = native(
        "retained_mutable_generations",
        policy.retained_mutable_generations,
    )?;
    options.reclamation.inventory_entries = policy.reclamation_inventory_entries;
    options.reclamation.metadata_bytes = policy.reclamation_metadata_bytes;
    options.reclamation.delete_files = policy.reclamation_delete_files;
    options.reclamation.delete_bytes = policy.reclamation_delete_bytes;
    options.reclamation.elapsed_ms = policy.reclamation_elapsed_ms;
    options.recovery.metadata_bytes = policy.recovery_metadata_bytes;
    options.recovery.files = policy.recovery_files;
    options.recovery.read_bytes = policy.recovery_read_bytes;
    options.recovery.wal_bytes = policy.recovery_wal_bytes;
    options.recovery.wal_frames = policy.recovery_wal_frames;
    options.recovery.elapsed_ms = policy.recovery_elapsed_ms;
    Ok(options)
}

pub struct RrdEngine {
    pub(crate) storage: StorageProfile,
    pub(crate) storage_profile_kind: StorageProfileKind,
    pub(crate) estate_configuration: EstateConfiguration,
    pub(crate) installed_estate: Option<InstalledEstateIdentity>,
    pub(crate) installed_deployment: Option<DeploymentProfile>,
    /// The managed persistent container `.rrflow/rrd` of an installed estate.
    /// Backups and restore staging roots live beside its storage roots; it is
    /// `None` for component and test roots opened outside an installation.
    pub(crate) installed_storage_container: Option<AnchoredPath>,
    pub(crate) objects: ObjectStoreBox,
    pub(crate) storage_root: Option<AnchoredPath>,
    pub(in crate::engine) component_operation_parent: Option<AnchoredPath>,
    pub(in crate::engine) instance: CanonicalId,
    pub(crate) token_key: [u8; 32],
    /// Serializes the local definition/validation/commit boundary. rrflowKV
    /// already owns the cross-process writer lock; this closes the
    /// in-process race between publishing a unique index and committing data.
    pub(crate) transaction_gate: super::transaction_admission::TransactionGate,
    /// Tracks invocation reservations currently executing through a supported
    /// adapter. Session-backed methods use this to distinguish a correctly
    /// wrapped call from a direct embedded call and make the latter leave a
    /// durable authorization or denial record as well.
    pub(in crate::engine) active_invocations: Mutex<BTreeMap<(String, String), ActiveInvocation>>,
    /// One process-local physical authority for decoded vector artifacts.
    /// Durable catalogue/object state remains in `storage` and `objects`;
    /// this bounded manager alone owns serving residency across requests.
    pub(crate) vector_residency: Mutex<crate::VectorResidencyManager>,
    /// Process-local optional HNSW build adapters. Their bytes are never
    /// authoritative until the CPU differential coordinator admits them.
    pub(crate) hnsw_accelerators: Mutex<rrd_vector::HnswAcceleratorRegistry>,
    /// Process-local delivery handles for durable task Stop, keyed by task and
    /// fenced by attempt identity. Control state remains authoritative.
    pub(in crate::engine) active_task_cancellations:
        Mutex<BTreeMap<String, (CorrelationId, rrd_inference::EmbeddingCancellation)>>,
    /// Process-local executable inference adapters. Exact model, trust, and
    /// resource descriptors are public; credentials and runtime sessions are
    /// never serialized into durable RRD state.
    pub(crate) embedding_backends: Mutex<rrd_inference::EmbeddingBackendRegistry>,
}
impl RrdEngine {
    pub fn create_new(root: &Path, instance: CanonicalId, token_key: [u8; 32]) -> Result<Self> {
        let root =
            AnchoredPath::from_path(root).map_err(|e| ServiceError::Storage(e.to_string()))?;
        Self::create_new_anchored(&root, instance, token_key)
    }
    pub(super) fn create_new_anchored(
        root: &AnchoredPath,
        instance: CanonicalId,
        token_key: [u8; 32],
    ) -> Result<Self> {
        Self::create_new_anchored_with_options(root, instance, token_key, Default::default())
    }

    pub(super) fn create_new_anchored_with_options(
        root: &AnchoredPath,
        instance: CanonicalId,
        token_key: [u8; 32],
        options: rrd_store::DatabaseOptions,
    ) -> Result<Self> {
        let operation_parent = root.parent();
        let storage = RrflowKvStore::create_new_anchored(root, options)?;
        Self::compose_persistent(
            operation_parent,
            instance,
            token_key,
            storage,
            true,
            crate::VectorResidencyLimits::default(),
        )
    }
    pub fn open_existing(root: &Path, instance: CanonicalId, token_key: [u8; 32]) -> Result<Self> {
        let root =
            AnchoredPath::from_path(root).map_err(|e| ServiceError::Storage(e.to_string()))?;
        Self::open_existing_anchored(&root, instance, token_key)
    }
    pub(super) fn open_existing_anchored(
        root: &AnchoredPath,
        instance: CanonicalId,
        token_key: [u8; 32],
    ) -> Result<Self> {
        Self::open_existing_anchored_with_options(root, instance, token_key, Default::default())
    }

    pub(super) fn open_existing_anchored_with_options(
        root: &AnchoredPath,
        instance: CanonicalId,
        token_key: [u8; 32],
        options: rrd_store::DatabaseOptions,
    ) -> Result<Self> {
        let operation_parent = root.parent();
        let root = root
            .pin_directory()
            .map_err(|e| ServiceError::Storage(e.to_string()))?;
        validate_existing_object_layout(&root)?;
        let storage = RrflowKvStore::open_existing_anchored(&root, options)?;
        Self::compose_persistent(
            operation_parent,
            instance,
            token_key,
            storage,
            false,
            crate::VectorResidencyLimits::default(),
        )
    }

    /// Opens a local engine authority for pre-release engine-owned estate
    /// control operations that do not yet resolve an installed-project
    /// locator. The constructor remains private to `rrd-engine`; outward
    /// adapters call typed operations.
    pub(in crate::engine) fn open_local_authority(
        root: &Path,
        instance: CanonicalId,
    ) -> Result<Self> {
        Self::open_with_token_key_file(root, instance)
    }

    pub(in crate::engine) fn query_scope(&self, requested: &str) -> Result<ScopeId> {
        let expected_scope = format!("instance:{}", self.instance);
        if requested != expected_scope {
            return Err(ServiceError::WrongScope);
        }
        ScopeId::new(requested.to_owned()).map_err(|error| ServiceError::Query(error.to_string()))
    }

    pub fn open(root: &Path, instance: CanonicalId, token_key: [u8; 32]) -> Result<Self> {
        Self::open_with_vector_residency(
            root,
            instance,
            token_key,
            crate::VectorResidencyLimits::default(),
        )
    }

    /// Opens the persistent engine before loading or creating its local
    /// token-signing key. rrflowKV intentionally requires an empty directory
    /// when it creates a new database, so file-backed credentials must never
    /// be created in the engine root first.
    fn open_with_token_key_file(root: &Path, instance: CanonicalId) -> Result<Self> {
        let mut engine = Self::open(root, instance, [0_u8; TOKEN_KEY_BYTES])?;
        engine.token_key = load_or_create_token_key(
            &engine
                .require_persistent_root("local authority")?
                .join("RRD.SECRET"),
        )
        .map_err(|error| ServiceError::Storage(error.to_string()))?;
        Ok(engine)
    }

    pub fn open_with_vector_residency(
        root: &Path,
        instance: CanonicalId,
        token_key: [u8; 32],
        limits: crate::VectorResidencyLimits,
    ) -> Result<Self> {
        let root =
            AnchoredPath::from_path(root).map_err(|e| ServiceError::Storage(e.to_string()))?;
        let operation_parent = root.parent();
        let storage = RrflowKvStore::open(&root)?;
        Self::compose_persistent(operation_parent, instance, token_key, storage, true, limits)
    }

    pub(super) fn compose_persistent(
        operation_parent: Option<AnchoredPath>,
        instance: CanonicalId,
        token_key: [u8; 32],
        storage: RrflowKvStore,
        create_objects: bool,
        limits: crate::VectorResidencyLimits,
    ) -> Result<Self> {
        let root = storage.anchored_path().clone();
        let objects = if create_objects {
            rrd_store::LocalObjectStore::open_anchored(&root.join("immutable"))?
        } else {
            rrd_store::LocalObjectStore::open_existing_anchored(&root.join("immutable"))?
        };
        let vector_residency = crate::VectorResidencyManager::new(limits)
            .map_err(|error| ServiceError::Vector(error.to_string()))?;
        Ok(Self {
            storage: StorageProfile::rrflow_kv(storage),
            storage_profile_kind: StorageProfileKind::RrflowKv,
            estate_configuration: EstateConfiguration::default(),
            installed_estate: None,
            installed_deployment: None,
            installed_storage_container: None,
            objects: ObjectStoreBox::new(objects),
            storage_root: Some(root),
            component_operation_parent: operation_parent,
            instance,
            token_key,
            transaction_gate: super::transaction_admission::TransactionGate::default(),
            active_invocations: Mutex::new(BTreeMap::new()),
            vector_residency: Mutex::new(vector_residency),
            hnsw_accelerators: Mutex::new(rrd_vector::HnswAcceleratorRegistry::default()),
            active_task_cancellations: Mutex::new(BTreeMap::new()),
            embedding_backends: Mutex::new(rrd_inference::EmbeddingBackendRegistry::default()),
        })
    }

    /// Creates one process-local RRD authority backed by volatile rrflowMX.
    /// The full composition remains intact: sessions, transactions, policy,
    /// queries, audit, changefeeds, and subscriptions use the same engine
    /// methods as embedded and daemon profiles. Durability-only operations
    /// fail explicitly because rrflowMX has no storage path.
    pub fn rrflow_mx(instance: CanonicalId, token_key: [u8; 32]) -> Self {
        Self::rrflow_mx_with_vector_residency(
            instance,
            token_key,
            crate::VectorResidencyLimits::default(),
        )
        .expect("default vector residency limits are valid")
    }

    pub fn rrflow_mx_with_vector_residency(
        instance: CanonicalId,
        token_key: [u8; 32],
        limits: crate::VectorResidencyLimits,
    ) -> Result<Self> {
        let vector_residency = crate::VectorResidencyManager::new(limits)
            .map_err(|error| ServiceError::Vector(error.to_string()))?;
        Ok(Self {
            storage: StorageProfile::rrflow_mx(),
            storage_profile_kind: StorageProfileKind::RrflowMx,
            estate_configuration: EstateConfiguration::default(),
            installed_estate: None,
            installed_deployment: None,
            installed_storage_container: None,
            objects: ObjectStoreBox::new(MemoryObjectStore::new()),
            storage_root: None,
            component_operation_parent: None,
            instance,
            token_key,
            transaction_gate: super::transaction_admission::TransactionGate::default(),
            active_invocations: Mutex::new(BTreeMap::new()),
            vector_residency: Mutex::new(vector_residency),
            hnsw_accelerators: Mutex::new(rrd_vector::HnswAcceleratorRegistry::default()),
            active_task_cancellations: Mutex::new(BTreeMap::new()),
            embedding_backends: Mutex::new(rrd_inference::EmbeddingBackendRegistry::default()),
        })
    }

    pub fn vector_residency_snapshot(&self) -> Result<crate::VectorResidencySnapshot> {
        self.vector_residency
            .lock()
            .map(|manager| manager.snapshot())
            .map_err(|_| ServiceError::Storage("vector residency lock is poisoned".into()))
    }

    pub fn has_persistent_root(&self) -> bool {
        self.storage_root.is_some()
    }

    pub fn storage_profile_kind(&self) -> StorageProfileKind {
        self.storage_profile_kind
    }

    pub fn estate_configuration(&self) -> &EstateConfiguration {
        &self.estate_configuration
    }

    pub fn installed_estate_identity(&self) -> Option<&InstalledEstateIdentity> {
        self.installed_estate.as_ref()
    }

    pub fn installed_deployment_profile(&self) -> Option<&DeploymentProfile> {
        self.installed_deployment.as_ref()
    }

    pub(in crate::engine) fn enforce_query_configuration(
        &self,
        operation: &'static str,
        requested: &rrd_contract::QueryBudget,
    ) -> Result<()> {
        if let Err(error) = self.estate_configuration.validate_query_budget(requested) {
            let configured = &self.estate_configuration.query;
            tracing::debug!(
                target: "rrflow::configuration",
                operation,
                instance_id = %self.instance,
                configuration_revision = self.estate_configuration.revision,
                configuration_sha256 = %self.estate_configuration.configuration_sha256,
                requested_max_storage_keys = requested.max_storage_keys,
                configured_max_storage_keys = configured.max_storage_keys,
                requested_max_rows = requested.max_rows,
                configured_max_rows = configured.max_rows,
                requested_max_output_bytes = requested.max_output_bytes,
                configured_max_output_bytes = configured.max_output_bytes,
                requested_max_batch_rows = requested.max_batch_rows,
                configured_max_batch_rows = configured.max_batch_rows,
                requested_max_memory_bytes = requested.max_memory_bytes,
                configured_max_memory_bytes = configured.max_memory_bytes,
                requested_max_spill_bytes = requested.max_spill_bytes,
                configured_max_spill_bytes = configured.max_spill_bytes,
                requested_max_elapsed_ms = requested.max_elapsed_ms,
                configured_max_elapsed_ms = configured.max_elapsed_ms,
                reason = %error,
                "estate configuration rejected query budget"
            );
            return Err(ServiceError::ConfigurationLimit(error.to_string()));
        }
        Ok(())
    }

    pub(in crate::engine) fn enforce_recall_configuration(
        &self,
        operation: &'static str,
        requested: &rrd_contract::AssembleContext,
    ) -> Result<()> {
        if let Err(error) = self.estate_configuration.recall.validate_request(requested) {
            let configured = &self.estate_configuration.recall;
            tracing::debug!(
                target: "rrflow::configuration",
                operation,
                instance_id = %self.instance,
                configuration_revision = self.estate_configuration.revision,
                configuration_sha256 = %self.estate_configuration.configuration_sha256,
                requested_max_graph_depth = requested.max_graph_depth,
                configured_max_graph_depth = configured.max_graph_depth,
                requested_max_items = requested.max_items,
                configured_max_items = configured.max_items,
                requested_max_output_bytes = requested.max_output_bytes,
                configured_max_output_bytes = configured.max_output_bytes,
                requested_max_storage_keys = requested.max_storage_keys,
                configured_max_storage_keys = configured.max_storage_keys,
                reason = %error,
                "estate configuration rejected recall request"
            );
            return Err(ServiceError::ConfigurationLimit(error.to_string()));
        }
        Ok(())
    }

    pub(in crate::engine) fn bind_installed_state(
        &mut self,
        identity: InstalledEstateIdentity,
        deployment: DeploymentProfile,
        configuration: EstateConfiguration,
        storage_container: AnchoredPath,
    ) -> Result<()> {
        identity
            .validate()
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        configuration
            .validate()
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        deployment
            .validate()
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        if identity.instance_id != self.instance
            || deployment.storage_profile != self.storage_profile_kind
        {
            return Err(ServiceError::ProjectBindingMismatch);
        }
        self.installed_estate = Some(identity);
        self.installed_deployment = Some(deployment);
        self.installed_storage_container = Some(storage_container);
        self.estate_configuration = configuration;
        Ok(())
    }

    pub(in crate::engine) fn require_persistent_root(
        &self,
        operation: &str,
    ) -> Result<&AnchoredPath> {
        self.storage_root.as_ref().ok_or_else(|| {
            ServiceError::Backup(format!(
                "{operation} requires a persistent RRFlow database; rrflowMX is non-durable"
            ))
        })
    }

    pub fn instance_id(&self) -> &CanonicalId {
        &self.instance
    }

    pub fn readiness(&self, observed_at_unix_ms: u64) -> Result<Readiness> {
        let backend = self.storage.physical_store_evidence()?.backend;
        Ok(Readiness {
            observed_at_unix_ms,
            claim_sequence: self.storage.claims().sequence()?,
            runtime_cursor: self.storage.runtime().cursor()?,
            backend: CanonicalId::new(backend)
                .map_err(|error| ServiceError::Contract(error.to_string()))?,
        })
    }
}

pub(super) fn validate_existing_object_layout(root: &AnchoredPath) -> Result<()> {
    for relative in [
        "",
        "immutable",
        "immutable/objects",
        "immutable/objects/sha256",
        "immutable/staging",
        "immutable/quarantine",
    ] {
        let path = root.join(relative);
        let metadata = path.metadata().map_err(|error| {
            ServiceError::Storage(format!(
                "installed object-store path cannot be inspected ({}): {error}",
                path.display()
            ))
        })?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(ServiceError::Storage(format!(
                "installed object-store path is missing, symbolic, or invalid: {}",
                path.display()
            )));
        }
    }
    Ok(())
}
