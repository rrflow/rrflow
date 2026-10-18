use super::*;

pub struct RrdEngine {
    pub(crate) storage: StorageProfile,
    pub(crate) storage_profile_kind: StorageProfileKind,
    pub(crate) estate_configuration: EstateConfiguration,
    pub(crate) installed_estate: Option<InstalledEstateIdentity>,
    pub(crate) installed_deployment: Option<DeploymentProfile>,
    /// The managed persistent container `.rrflow/rrd` of an installed estate.
    /// Backups and restore staging roots live beside its storage roots; it is
    /// `None` for component and test roots opened outside an installation.
    pub(crate) installed_storage_container: Option<PathBuf>,
    pub(crate) objects: ObjectStoreBox,
    pub(crate) storage_root: Option<PathBuf>,
    pub(in crate::engine) instance: CanonicalId,
    pub(crate) token_key: [u8; 32],
    /// Serializes the local definition/validation/commit boundary. rrflowKV
    /// already owns the cross-process writer lock; this closes the
    /// in-process race between publishing a unique index and committing data.
    pub(crate) transaction_gate: Mutex<()>,
    /// Tracks invocation reservations currently executing through a supported
    /// adapter. Session-backed methods use this to distinguish a correctly
    /// wrapped call from a direct embedded call and make the latter leave a
    /// durable authorization or denial record as well.
    pub(crate) active_invocations: Mutex<BTreeMap<(String, String), std::thread::ThreadId>>,
    /// One process-local physical authority for decoded vector artifacts.
    /// Durable catalogue/object state remains in `storage` and `objects`;
    /// this bounded manager alone owns serving residency across requests.
    pub(crate) vector_residency: Mutex<crate::VectorResidencyManager>,
    /// Process-local optional HNSW build adapters. Their bytes are never
    /// authoritative until the CPU differential coordinator admits them.
    pub(crate) hnsw_accelerators: Mutex<rrd_vector::HnswAcceleratorRegistry>,
    /// Process-local executable inference adapters. Exact model, trust, and
    /// resource descriptors are public; credentials and runtime sessions are
    /// never serialized into durable RRD state.
    pub(crate) embedding_backends: Mutex<rrd_inference::EmbeddingBackendRegistry>,
}
impl RrdEngine {
    pub fn create_new(root: &Path, instance: CanonicalId, token_key: [u8; 32]) -> Result<Self> {
        if std::fs::symlink_metadata(root).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
            return Err(ServiceError::ProjectBindingMismatch);
        }
        let storage = RrflowKvStore::create_new(root)?;
        Self::compose_persistent(
            root,
            instance,
            token_key,
            storage,
            crate::VectorResidencyLimits::default(),
        )
    }

    pub fn open_existing(root: &Path, instance: CanonicalId, token_key: [u8; 32]) -> Result<Self> {
        validate_existing_object_layout(root)?;
        let storage = RrflowKvStore::open_existing(root)?;
        Self::compose_persistent(
            root,
            instance,
            token_key,
            storage,
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
        Self::open_with_token_key_file(root, instance, &root.join("RRD.SECRET"))
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
    fn open_with_token_key_file(
        root: &Path,
        instance: CanonicalId,
        token_key_file: &Path,
    ) -> Result<Self> {
        let mut engine = Self::open(root, instance, [0_u8; TOKEN_KEY_BYTES])?;
        engine.token_key = load_or_create_token_key(token_key_file)
            .map_err(|error| ServiceError::Storage(error.to_string()))?;
        Ok(engine)
    }

    pub fn open_with_vector_residency(
        root: &Path,
        instance: CanonicalId,
        token_key: [u8; 32],
        limits: crate::VectorResidencyLimits,
    ) -> Result<Self> {
        let storage = RrflowKvStore::open(root)?;
        Self::compose_persistent(root, instance, token_key, storage, limits)
    }

    fn compose_persistent(
        root: &Path,
        instance: CanonicalId,
        token_key: [u8; 32],
        storage: RrflowKvStore,
        limits: crate::VectorResidencyLimits,
    ) -> Result<Self> {
        let objects = rrd_store::LocalObjectStore::open(root.join("immutable"))?;
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
            storage_root: Some(root.to_path_buf()),
            instance,
            token_key,
            transaction_gate: Mutex::new(()),
            active_invocations: Mutex::new(BTreeMap::new()),
            vector_residency: Mutex::new(vector_residency),
            hnsw_accelerators: Mutex::new(rrd_vector::HnswAcceleratorRegistry::default()),
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
            instance,
            token_key,
            transaction_gate: Mutex::new(()),
            active_invocations: Mutex::new(BTreeMap::new()),
            vector_residency: Mutex::new(vector_residency),
            hnsw_accelerators: Mutex::new(rrd_vector::HnswAcceleratorRegistry::default()),
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
        storage_container: PathBuf,
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

    pub(in crate::engine) fn require_persistent_root(&self, operation: &str) -> Result<&Path> {
        self.storage_root.as_deref().ok_or_else(|| {
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

pub(super) fn validate_existing_object_layout(root: &Path) -> Result<()> {
    for relative in [
        "",
        "immutable",
        "immutable/objects",
        "immutable/objects/sha256",
        "immutable/staging",
        "immutable/quarantine",
    ] {
        let path = root.join(relative);
        let metadata = std::fs::symlink_metadata(&path).map_err(|error| {
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
