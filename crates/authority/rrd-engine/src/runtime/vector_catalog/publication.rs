//! Vector artifact publication, bindings, runtime manifests and reopening the vector runtime.

use super::*;

pub(super) const CATALOGUE_MAX_KEYS: u64 = 100_000;
pub(super) const PUBLICATION_RETRIES: usize = 16;

/// Select only catalogue records and their exact immutable-object identities.
/// Optional candidate sources participate in the same physical transaction,
/// read-stamp validation, accumulator authentication and key budget.
pub(crate) fn read_vector_catalogue_versions<E: StorageEngine>(
    engine: &E,
    read: &ReadStamp,
    additional_sources: &[RuntimeVersionedSource],
    budget: RuntimeReadBudget,
) -> rrd_store::Result<RuntimeVersionedRead> {
    let mut sources = additional_sources.to_vec();
    for name in [
        VECTOR_ARTIFACT_RECORD_TYPE,
        QUANTIZATION_ARTIFACT_RECORD_TYPE,
        QUANTIZATION_LIFECYCLE_RECORD_TYPE,
    ] {
        sources.push(RuntimeVersionedSource::Records {
            kind: Some(RuntimeType::new(name)?),
        });
    }
    let mut malformed = None;
    let direct =
        engine
            .runtime()
            .read_versioned_expanding(read, &sources, budget, &mut |changes| {
                let mut references = BTreeSet::new();
                for change in changes {
                    let RuntimeMutation::Record { record } = &change.mutation else {
                        continue;
                    };
                    let object = match record.reference.kind.as_str() {
                        VECTOR_ARTIFACT_RECORD_TYPE => {
                            entry_from_record(record).map(|entry| entry.object.reference)
                        }
                        QUANTIZATION_ARTIFACT_RECORD_TYPE => quantization_entry_from_record(record)
                            .map(|entry| entry.object.reference),
                        _ => continue,
                    };
                    match object {
                        Ok(reference) => {
                            references.insert(reference);
                        }
                        Err(error) => {
                            malformed = Some(error.to_string());
                            return Vec::new();
                        }
                    }
                }
                references
                    .into_iter()
                    .map(|reference| RuntimeVersionedSource::Identity {
                        model: RuntimeLogicalModel::Object,
                        reference,
                    })
                    .collect()
            })?;
    if let Some(error) = malformed {
        return Err(StoreError::Substrate(error));
    }
    Ok(direct)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VectorArtifactPublication {
    pub catalog_revision: u64,
    pub entry: VectorArtifactCatalogEntry,
    pub commit: RuntimeCommitOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuantizationArtifactPublication {
    pub lifecycle_revision: u64,
    pub entry: QuantizationArtifactEntry,
    pub state: QuantizationArtifactState,
    pub commit: RuntimeCommitOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuantizationArtifactTransition {
    pub lifecycle_revision: u64,
    pub entry: QuantizationArtifactEntry,
    pub state: QuantizationArtifactState,
    pub commit: RuntimeCommitOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct VectorArtifactResidencyKey {
    pub scope: ScopeId,
    pub projection_id: ProjectionId,
    pub generation: u64,
    pub object_sha256: String,
    pub source: Option<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VectorArtifactBinding {
    pub kind: rrd_vector::VectorArtifactKind,
    pub descriptor: rrd_vector::VectorProjectionDescriptor,
    pub object: ObjectReference,
    pub source: Option<VectorCollectionAddress>,
}

impl VectorArtifactBinding {
    pub fn key(&self) -> VectorArtifactResidencyKey {
        VectorArtifactResidencyKey {
            scope: self.descriptor.scope().clone(),
            projection_id: self.descriptor.stamp().id.clone(),
            generation: self.descriptor.stamp().generation,
            object_sha256: self.object.sha256.clone(),
            source: self
                .source
                .as_ref()
                .map(|source| (source.collection_id.clone(), source.vector_name.clone())),
        }
    }

    pub fn decode_owned(
        &self,
        objects: &impl ImmutableObjectStore,
    ) -> Result<Arc<VectorArtifact>, Box<dyn std::error::Error>> {
        let bytes = objects.get(&self.object)?;
        let artifact = VectorArtifact::from_bytes(self.kind, &bytes)?;
        self.validate_artifact(&artifact)?;
        Ok(Arc::new(artifact))
    }

    pub fn decode_mapped(
        &self,
        objects: &impl ImmutableObjectStore,
    ) -> Result<Option<Arc<VectorArtifact>>, Box<dyn std::error::Error>> {
        let Some(file) = objects.verified_file(&self.object)? else {
            return Ok(None);
        };
        let Some(artifact) = VectorArtifact::open_mmap_file(self.kind, file)? else {
            return Ok(None);
        };
        self.validate_artifact(&artifact)?;
        Ok(Some(Arc::new(artifact)))
    }

    fn validate_artifact(
        &self,
        artifact: &VectorArtifact,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if artifact.kind() != self.kind || artifact.descriptor() != self.descriptor {
            return Err("loaded vector artifact differs from its durable binding".into());
        }
        if let (Some(source), VectorArtifact::Hnsw(index)) = (&self.source, artifact) {
            if !index.contains_only_source(source) {
                return Err("HNSW graph contains a foreign source".into());
            }
        }
        Ok(())
    }
}

pub struct VectorRuntimeManifest {
    pub runtime: VectorRuntime,
    pub bindings: BTreeMap<(ProjectionId, u64), VectorArtifactBinding>,
}

/// Publishes one immutable artifact without exposing it to serving until its
/// object and catalog binding have committed atomically.
pub fn publish_traced_vector_artifact<D>(
    data: &D,
    runtime: &mut VectorRuntime,
    expected_catalog_revision: u64,
    artifact: VectorArtifact,
    actor: &str,
    at: Millis,
) -> Result<VectorArtifactPublication, Box<dyn std::error::Error>>
where
    D: DataRuntimeAccess,
{
    publish_traced_vector_artifact_with_evidence(
        data,
        runtime,
        expected_catalog_revision,
        artifact,
        None,
        None,
        None,
        actor,
        at,
    )
}

/// Publishes one immutable artifact and its validated physical build evidence
/// in the same authoritative catalogue record, with the idempotent operation
/// that asked for it.
#[allow(clippy::too_many_arguments)]
pub fn publish_traced_vector_artifact_with_evidence<D>(
    data: &D,
    runtime: &mut VectorRuntime,
    expected_catalog_revision: u64,
    artifact: VectorArtifact,
    build_evidence: Option<rrd_vector::HnswBuildEvidence>,
    operation: Option<rrd_vector::VectorArtifactOperationReceipt>,
    source: Option<VectorCollectionAddress>,
    actor: &str,
    at: Millis,
) -> Result<VectorArtifactPublication, Box<dyn std::error::Error>>
where
    D: DataRuntimeAccess,
{
    if matches!(
        artifact.kind(),
        rrd_vector::VectorArtifactKind::ScalarQuantized
            | rrd_vector::VectorArtifactKind::ProductQuantized
            | rrd_vector::VectorArtifactKind::BinaryQuantized
            | rrd_vector::VectorArtifactKind::TurboQuant
    ) {
        return Err("quantized publication must use the quantization artifact lifecycle".into());
    }
    if operation
        .as_ref()
        .is_some_and(|receipt| receipt.source.is_some())
        || (source.is_some() && operation.is_none())
    {
        return Err("named source must be bound by its operation receipt".into());
    }
    if let Some(source) = &source {
        source.validate()?;
        match &artifact {
            VectorArtifact::Hnsw(index) if index.contains_only_source(source) => {}
            _ => return Err("artifact does not match its named source".into()),
        }
    }
    let descriptor = artifact.descriptor();
    descriptor.validate()?;
    let scope = descriptor.scope().clone();
    let stamp = descriptor.stamp().clone();

    // Validate the exact in-process transition on a clone. The serving view is
    // replaced only after the authoritative transaction succeeds.
    let mut next_runtime = runtime.clone();
    let next_revision = next_runtime.publish(expected_catalog_revision, artifact.clone())?;

    let revision_bytes = expected_catalog_revision.to_be_bytes();
    let generation_bytes = stamp.generation.to_be_bytes();
    let identity = TraceIdentity::derive(&[
        scope.as_str().as_bytes(),
        stamp.id.as_str().as_bytes(),
        &generation_bytes,
        &revision_bytes,
    ])?;
    let read = data.engine().runtime().read_stamp(&scope)?;
    let mut links = vec![TraceLink::Read { stamp: read }];
    if let Some(source) = &source {
        links.push(TraceLink::Source {
            source_kind: "named_vector".into(),
            source_id: rrd_vector::vector_source_digest(source)?,
            source_revision: format!("cursor:{}", stamp.source_cursor),
        });
    }
    let span = DurableTraceSpan::start(
        data.engine(),
        scope.clone(),
        actor,
        identity,
        None,
        TraceBoundary::Vector,
        "vector.projection.publish",
        at,
        TraceDataClass::Control,
        links,
        publication_attributes(&artifact, expected_catalog_revision),
    )?;

    let durable_entries = match vector_artifact_catalog_entries(data.engine(), &scope) {
        Ok(entries) => entries,
        Err(error) => {
            return finish_publication_error(data.engine(), span, "catalog_preflight", error);
        }
    };
    let durable_revision = durable_entries
        .last()
        .map_or(0, |entry| entry.catalog_revision);
    if durable_revision != expected_catalog_revision {
        return finish_publication_error(
            data.engine(),
            span,
            "catalog_preflight",
            format!(
                "vector catalog conflict: expected revision {expected_catalog_revision}, authoritative revision {durable_revision}"
            )
            .into(),
        );
    }

    // Both identities exist before any bytes are staged, so one their types refuse stores nothing.
    let identities = (
        VectorArtifactCatalogEntry::record_reference(&descriptor),
        RuntimeId::new(format!("{}.{}.vector-bytes", stamp.id, stamp.generation)),
    );
    let (record_reference, object_id) = match identities {
        (Ok(record), Ok(object)) => (record, object),
        (Err(error), _) | (_, Err(error)) => {
            return finish_publication_error(data.engine(), span, "catalog_identity", error.into());
        }
    };
    let bytes = artifact.as_bytes();
    let object = match data.stage_object(
        object_id.as_str(),
        Some(record_reference.clone()),
        artifact.kind().media_type(),
        bytes,
    ) {
        Ok(object) => object,
        Err(error) => {
            return finish_publication_error(data.engine(), span, "object_stage", error.into());
        }
    };
    let entry = VectorArtifactCatalogEntry::new_with_source(
        next_revision,
        artifact.kind(),
        descriptor,
        object.clone(),
        at,
        build_evidence,
        operation,
        source,
    )?;
    let record = catalog_record(&entry)?;

    let commit = match commit_catalog_record(data, &scope, actor, at, record, object) {
        Ok(commit) => commit,
        Err(error) => {
            return finish_publication_error(data.engine(), span, "catalog_commit", error);
        }
    };
    *runtime = next_runtime;
    let finish_attributes = RuntimeProperties::from([
        (
            "catalog_revision".into(),
            RuntimeValue::Unsigned(next_revision),
        ),
        (
            "catalog_entry_digest".into(),
            RuntimeValue::Digest(entry.entry_digest.clone()),
        ),
        (
            "object_sha256".into(),
            RuntimeValue::Digest(entry.object.sha256.clone()),
        ),
        (
            "object_length".into(),
            RuntimeValue::Unsigned(entry.object.length),
        ),
        (
            "object_backend".into(),
            RuntimeValue::String(entry.object.receipt.backend.clone()),
        ),
        (
            "commit_id".into(),
            RuntimeValue::Digest(commit.commit_id.clone()),
        ),
    ]);
    span.finish(
        data.engine(),
        TraceOutcome::Ok,
        vec![TraceLink::Projection { stamp }],
        finish_attributes,
    )?;
    Ok(VectorArtifactPublication {
        catalog_revision: next_revision,
        entry,
        commit,
    })
}

/// Reconstructs the complete serving view from authoritative catalog records
/// and content-addressed bytes. Any revision gap or cross-record mismatch is a
/// hard error; silently dropping a damaged projection would change planning.
pub fn reopen_vector_runtime<D>(
    data: &D,
    scope: &ScopeId,
    canonical: impl IntoIterator<Item = VectorCandidate>,
) -> Result<VectorRuntime, Box<dyn std::error::Error>>
where
    D: DataRuntimeAccess,
{
    let mut manifest = reopen_vector_runtime_metadata(data, scope, canonical)?;
    for binding in manifest.bindings.values() {
        manifest
            .runtime
            .install_loaded_artifact(binding.decode_owned(data.objects())?)?;
    }
    Ok(manifest.runtime)
}

/// Reconstructs exact planner metadata without eagerly reading immutable
/// artifact bodies. Physical residency policy can then load only paths that
/// could serve the current request.
pub fn reopen_vector_runtime_metadata<D>(
    data: &D,
    scope: &ScopeId,
    canonical: impl IntoIterator<Item = VectorCandidate>,
) -> Result<VectorRuntimeManifest, Box<dyn std::error::Error>>
where
    D: DataRuntimeAccess,
{
    let read = data.engine().runtime().read_stamp(scope)?;
    let direct = read_vector_catalogue_versions(
        data.engine(),
        &read,
        &[],
        RuntimeReadBudget::new(CATALOGUE_MAX_KEYS)?,
    )?;
    vector_runtime_metadata_from_changes(&direct.changes, scope, canonical)
}

/// Builds the serving view from the same authenticated versions as the
/// request's candidates. It performs no independent storage read.
pub(crate) fn vector_runtime_metadata_from_changes(
    changes: &[RuntimeChange],
    scope: &ScopeId,
    canonical: impl IntoIterator<Item = VectorCandidate>,
) -> Result<VectorRuntimeManifest, Box<dyn std::error::Error>> {
    let entries = vector_artifact_catalog_entries_from_changes(changes, scope)?;
    let mut runtime = VectorRuntime::new(canonical)?;
    let mut bindings = BTreeMap::new();
    for entry in entries {
        let source = entry.source().cloned();
        let expected_revision = runtime.catalog().revision;
        runtime.replay_catalog_descriptor(expected_revision, entry.descriptor.clone())?;
        let key = (
            entry.descriptor.stamp().id.clone(),
            entry.descriptor.stamp().generation,
        );
        if bindings
            .insert(
                key,
                VectorArtifactBinding {
                    kind: entry.kind,
                    descriptor: entry.descriptor,
                    object: entry.object,
                    source,
                },
            )
            .is_some()
        {
            return Err("duplicate vector artifact residency binding".into());
        }
    }
    // The durable event stream contains every generation so replay can prove
    // the exact catalogue revision and retirement chain. Serving residency,
    // however, must never expose those retired bodies: retain only the
    // descriptor currently selected by the reconstructed catalogue.
    bindings.retain(|(id, generation), binding| {
        runtime.catalog().entries.get(id).is_some_and(|active| {
            active == &binding.descriptor && active.stamp().generation == *generation
        })
    });
    let quantization = quantization_artifact_catalogue_from_changes(changes, scope)?;
    for entry in quantization.active_entries() {
        runtime.restore_quantization_lifecycle_active_descriptor(entry.descriptor.clone())?;
        let key = (
            entry.descriptor.stamp().id.clone(),
            entry.descriptor.stamp().generation,
        );
        if bindings
            .insert(
                key,
                VectorArtifactBinding {
                    kind: entry.kind,
                    descriptor: entry.descriptor.clone(),
                    object: entry.object.clone(),
                    source: Some(VectorCollectionAddress {
                        collection_id: entry.collection_id.to_string(),
                        vector_name: entry.vector_name.to_string(),
                    }),
                },
            )
            .is_some()
        {
            return Err("duplicate active quantization residency binding".into());
        }
    }
    Ok(VectorRuntimeManifest { runtime, bindings })
}
