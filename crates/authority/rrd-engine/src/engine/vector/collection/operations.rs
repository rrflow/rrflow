//! Ensuring, listing and deleting vector collections and payload indexes.

use super::*;

impl RrdEngine {
    pub fn ensure_vector_collection(
        &self,
        session_id: &CorrelationId,
        token: &CorrelationId,
        request: &EnsureVectorCollection,
        context: &RequestContext,
        now: u64,
    ) -> Result<EnsureVectorCollectionResult> {
        context
            .validate(true)
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        let idempotency_key = context.idempotency_key.as_ref().ok_or_else(|| {
            ServiceError::Contract("vector collection ensure requires idempotency".into())
        })?;
        request
            .validate()
            .map_err(|error| ServiceError::Vector(error.to_string()))?;
        let (session_bytes, mut session) = self.load_authenticated(session_id, token)?;
        self.authorize_session_policy(&session, SecurityAction::VectorCollectionEnsure, now)?;
        let scope = self.query_scope(&request.scope)?;
        let operation_digest = operation_digest(request)?;
        let repository = rrd_vector::VectorCollectionRepository::new(&self.storage, scope);
        if let Some(receipt) = repository
            .operation_receipt(idempotency_key.as_str(), &operation_digest)
            .map_err(vector_collection_error)?
        {
            let catalogue = repository.load().map_err(vector_collection_error)?;
            return Ok(EnsureVectorCollectionResult {
                collection: public_vector_collection(&receipt.entry)?,
                catalogue_revision: catalogue.revision,
                idempotent_replay: true,
            });
        }
        // Exact completed operations remain replayable after lease expiry;
        // every new collection mutation still requires an active session.
        self.require_active_or_expire(
            session_id,
            session_bytes,
            &mut session,
            now,
            context.request_id.as_str(),
            context.operation_id.as_str(),
        )?;
        let definition = internal_vector_collection(request)?;
        let context = rrd_vector::CollectionMutationContext {
            at: now,
            actor: format!("session:{}", session_id.as_str()),
            request_id: context.request_id.as_str().into(),
            operation_id: context.operation_id.as_str().into(),
        };
        let (catalogue, idempotent_replay) = repository
            .ensure(
                &context,
                idempotency_key.as_str().into(),
                operation_digest,
                definition,
            )
            .map_err(vector_collection_error)?;
        let entry = catalogue
            .collections
            .get(&ProjectionId::new(request.collection_id.as_str()).map_err(core_vector)?)
            .ok_or_else(|| ServiceError::Vector("ensured vector collection disappeared".into()))?;
        Ok(EnsureVectorCollectionResult {
            collection: public_vector_collection(entry)?,
            catalogue_revision: catalogue.revision,
            idempotent_replay,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn list_vector_collections(
        &self,
        session_id: &CorrelationId,
        token: &CorrelationId,
        request: &ListVectorCollections,
        now: u64,
        request_id: &str,
        operation_id: &str,
    ) -> Result<VectorCollectionCatalogueSnapshot> {
        request
            .validate()
            .map_err(|error| ServiceError::Vector(error.to_string()))?;
        self.authorize(
            session_id,
            token,
            SecurityAction::VectorCollectionList,
            now,
            request_id,
            operation_id,
        )?;
        let scope = self.query_scope(&request.scope)?;
        let catalogue = rrd_vector::VectorCollectionRepository::new(&self.storage, scope)
            .load()
            .map_err(vector_collection_error)?;
        Ok(VectorCollectionCatalogueSnapshot {
            scope: request.scope.clone(),
            revision: catalogue.revision,
            collections: catalogue
                .collections
                .values()
                .map(public_vector_collection)
                .collect::<Result<_>>()?,
        })
    }

    pub fn ensure_vector_payload_index(
        &self,
        session_id: &CorrelationId,
        token: &CorrelationId,
        request: &EnsureVectorPayloadIndex,
        context: &RequestContext,
        now: u64,
    ) -> Result<EnsureVectorPayloadIndexResult> {
        context
            .validate(true)
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        let idempotency_key = context.idempotency_key.as_ref().ok_or_else(|| {
            ServiceError::Contract("vector payload-index ensure requires idempotency".into())
        })?;
        request
            .validate()
            .map_err(|error| ServiceError::Vector(error.to_string()))?;
        let (session_bytes, mut session) = self.load_authenticated(session_id, token)?;
        self.authorize_session_policy(&session, SecurityAction::VectorPayloadIndexEnsure, now)?;
        let scope = self.query_scope(&request.scope)?;
        let operation_digest = operation_digest(request)?;
        let repository = rrd_vector::VectorCollectionRepository::new(&self.storage, scope);
        if let Some(receipt) = repository
            .payload_operation_receipt(idempotency_key.as_str(), &operation_digest)
            .map_err(vector_collection_error)?
        {
            if receipt.deleted {
                return Err(ServiceError::IdempotencyConflict);
            }
            let catalogue = repository.load().map_err(vector_collection_error)?;
            return Ok(EnsureVectorPayloadIndexResult {
                collection: public_vector_collection(&receipt.collection)?,
                index: public_payload_index(&receipt.index)?,
                catalogue_revision: catalogue.revision,
                idempotent_replay: true,
            });
        }
        self.require_active_or_expire(
            session_id,
            session_bytes,
            &mut session,
            now,
            context.request_id.as_str(),
            context.operation_id.as_str(),
        )?;
        let collection_id =
            ProjectionId::new(request.collection_id.as_str()).map_err(core_vector)?;
        let definition = rrd_vector::PayloadIndexDefinition {
            field: ProjectionId::new(request.field.as_str()).map_err(core_vector)?,
            kind: internal_payload_index_kind(request.kind),
        };
        let mutation = collection_mutation_context(session_id, context, now);
        let (catalogue, index, idempotent_replay) = repository
            .ensure_payload_index(
                &mutation,
                idempotency_key.as_str().into(),
                operation_digest,
                &collection_id,
                definition,
            )
            .map_err(vector_collection_error)?;
        let collection = catalogue
            .collections
            .get(&collection_id)
            .ok_or_else(|| ServiceError::Vector("payload-index collection disappeared".into()))?;
        Ok(EnsureVectorPayloadIndexResult {
            collection: public_vector_collection(collection)?,
            index: public_payload_index(&index)?,
            catalogue_revision: catalogue.revision,
            idempotent_replay,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn list_vector_payload_indexes(
        &self,
        session_id: &CorrelationId,
        token: &CorrelationId,
        request: &ListVectorPayloadIndexes,
        now: u64,
        request_id: &str,
        operation_id: &str,
    ) -> Result<VectorPayloadIndexCatalogueSnapshot> {
        request
            .validate()
            .map_err(|error| ServiceError::Vector(error.to_string()))?;
        self.authorize(
            session_id,
            token,
            SecurityAction::VectorPayloadIndexList,
            now,
            request_id,
            operation_id,
        )?;
        let scope = self.query_scope(&request.scope)?;
        let catalogue = rrd_vector::VectorCollectionRepository::new(&self.storage, scope)
            .load()
            .map_err(vector_collection_error)?;
        let collection = catalogue
            .collections
            .get(&ProjectionId::new(request.collection_id.as_str()).map_err(core_vector)?)
            .ok_or_else(|| {
                ServiceError::Vector(format!(
                    "unknown vector collection {}",
                    request.collection_id
                ))
            })?;
        Ok(VectorPayloadIndexCatalogueSnapshot {
            scope: request.scope.clone(),
            collection_id: request.collection_id.clone(),
            collection_generation: collection.generation,
            catalogue_revision: catalogue.revision,
            indexes: collection
                .payload_indexes
                .values()
                .map(public_payload_index)
                .collect::<Result<_>>()?,
        })
    }

    pub fn delete_vector_payload_index(
        &self,
        session_id: &CorrelationId,
        token: &CorrelationId,
        request: &DeleteVectorPayloadIndex,
        context: &RequestContext,
        now: u64,
    ) -> Result<DeleteVectorPayloadIndexResult> {
        context
            .validate(true)
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        let idempotency_key = context.idempotency_key.as_ref().ok_or_else(|| {
            ServiceError::Contract("vector payload-index delete requires idempotency".into())
        })?;
        request
            .validate()
            .map_err(|error| ServiceError::Vector(error.to_string()))?;
        let (session_bytes, mut session) = self.load_authenticated(session_id, token)?;
        self.authorize_session_policy(&session, SecurityAction::VectorPayloadIndexDelete, now)?;
        let scope = self.query_scope(&request.scope)?;
        let operation_digest = operation_digest(request)?;
        let repository = rrd_vector::VectorCollectionRepository::new(&self.storage, scope);
        if let Some(receipt) = repository
            .payload_operation_receipt(idempotency_key.as_str(), &operation_digest)
            .map_err(vector_collection_error)?
        {
            if !receipt.deleted {
                return Err(ServiceError::IdempotencyConflict);
            }
            let catalogue = repository.load().map_err(vector_collection_error)?;
            return Ok(DeleteVectorPayloadIndexResult {
                collection: public_vector_collection(&receipt.collection)?,
                deleted_index: public_payload_index(&receipt.index)?,
                catalogue_revision: catalogue.revision,
                idempotent_replay: true,
            });
        }
        self.require_active_or_expire(
            session_id,
            session_bytes,
            &mut session,
            now,
            context.request_id.as_str(),
            context.operation_id.as_str(),
        )?;
        let collection_id =
            ProjectionId::new(request.collection_id.as_str()).map_err(core_vector)?;
        let field = ProjectionId::new(request.field.as_str()).map_err(core_vector)?;
        let catalogue = repository.load().map_err(vector_collection_error)?;
        let collection = catalogue.collections.get(&collection_id).ok_or_else(|| {
            ServiceError::Vector(format!(
                "unknown vector collection {}",
                request.collection_id
            ))
        })?;
        require_payload_index_not_in_use(
            self,
            &self.query_scope(&request.scope)?,
            &request.collection_id,
            collection,
            request.field.as_str(),
        )?;
        let mutation = collection_mutation_context(session_id, context, now);
        let (catalogue, deleted_index, idempotent_replay) = repository
            .delete_payload_index(
                &mutation,
                idempotency_key.as_str().into(),
                operation_digest,
                &collection_id,
                &field,
            )
            .map_err(vector_collection_error)?;
        let collection = catalogue
            .collections
            .get(&collection_id)
            .ok_or_else(|| ServiceError::Vector("payload-index collection disappeared".into()))?;
        Ok(DeleteVectorPayloadIndexResult {
            collection: public_vector_collection(collection)?,
            deleted_index: public_payload_index(&deleted_index)?,
            catalogue_revision: catalogue.revision,
            idempotent_replay,
        })
    }

    pub fn delete_vector_collection(
        &self,
        session_id: &CorrelationId,
        token: &CorrelationId,
        request: &DeleteVectorCollection,
        context: &RequestContext,
        now: u64,
    ) -> Result<DeleteVectorCollectionResult> {
        context
            .validate(true)
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        let idempotency_key = context.idempotency_key.as_ref().ok_or_else(|| {
            ServiceError::Contract("vector collection delete requires idempotency".into())
        })?;
        request
            .validate()
            .map_err(|error| ServiceError::Vector(error.to_string()))?;
        let (session_bytes, mut session) = self.load_authenticated(session_id, token)?;
        self.authorize_session_policy(&session, SecurityAction::VectorCollectionDelete, now)?;
        let scope = self.query_scope(&request.scope)?;
        let operation_digest = operation_digest(request)?;
        let read = self.storage.runtime().read_stamp(&scope)?;
        let direct = read_vector_versions(self, &read, request.max_storage_keys)?;
        let repository = rrd_vector::VectorCollectionRepository::new(&self.storage, scope.clone());
        if let Some(receipt) = repository
            .deletion_receipt(idempotency_key.as_str(), &operation_digest)
            .map_err(vector_collection_error)?
        {
            let catalogue = repository.load().map_err(vector_collection_error)?;
            return Ok(DeleteVectorCollectionResult {
                deleted_collection: public_vector_collection(&receipt.deleted_entry)?,
                catalogue_revision: catalogue.revision,
                selected_versions: direct.selected_versions,
                read_evidence: direct.read_evidence,
                idempotent_replay: true,
            });
        }
        self.require_active_or_expire(
            session_id,
            session_bytes,
            &mut session,
            now,
            context.request_id.as_str(),
            context.operation_id.as_str(),
        )?;
        let catalogue = repository.load().map_err(vector_collection_error)?;
        let collection_id =
            ProjectionId::new(request.collection_id.as_str()).map_err(core_vector)?;
        let collection = catalogue.collections.get(&collection_id).ok_or_else(|| {
            ServiceError::Vector(format!(
                "unknown vector collection {}",
                request.collection_id
            ))
        })?;
        self.require_empty_collection(request, &read, collection, &direct.changes)?;
        require_no_collection_artifacts(self, &scope, &request.collection_id, collection)?;
        let mutation = collection_mutation_context(session_id, context, now);
        let (catalogue, deleted_collection, idempotent_replay) = repository
            .delete(
                &mutation,
                idempotency_key.as_str().into(),
                operation_digest,
                &collection_id,
            )
            .map_err(vector_collection_error)?;
        Ok(DeleteVectorCollectionResult {
            deleted_collection: public_vector_collection(&deleted_collection)?,
            catalogue_revision: catalogue.revision,
            selected_versions: direct.selected_versions,
            read_evidence: direct.read_evidence,
            idempotent_replay,
        })
    }

    /// Refuses while the collection holds an identity at any valid time from
    /// the deletion on: while, at some such time, the identity's latest
    /// version names one of the collection's named vectors and is not
    /// retired. A move out of the collection ends that hold where the move
    /// takes effect (POAM-047).
    fn require_empty_collection(
        &self,
        request: &DeleteVectorCollection,
        read: &ReadStamp,
        collection: &rrd_vector::CollectionEntry,
        changes: &[RuntimeChange],
    ) -> Result<()> {
        let addressed = |candidate: &rrd_vector::VectorCandidate| {
            let address = &candidate.vector.collection;
            address.collection_id == request.collection_id.as_str()
                && collection
                    .definition
                    .vectors
                    .keys()
                    .any(|name| name.as_str() == address.vector_name)
        };
        let mut histories = BTreeMap::<rrd_core::RuntimeRef, Vec<_>>::new();
        for candidate in rrd_vector::candidates_from_changes(changes, &read.scope) {
            histories
                .entry(candidate.vector.reference.clone())
                .or_insert_with(Vec::new)
                .push(candidate);
        }
        for history in histories
            .values()
            .filter(|history| history.iter().any(addressed))
        {
            // An identity's latest version changes only where a version
            // starts to apply, so it is resolved at the deletion time and at
            // every later start.
            let times = std::iter::once(request.valid_at)
                .chain(
                    history
                        .iter()
                        .map(|candidate| candidate.vector.valid_from)
                        .filter(|valid_from| *valid_from > request.valid_at),
                )
                .collect::<std::collections::BTreeSet<_>>();
            for valid_at in times {
                let visibility = rrd_vector::VectorVisibilityRequest {
                    scope: read.scope.clone(),
                    read: read.clone(),
                    valid_at,
                    field: history[0].vector.field.clone(),
                    embedding_model: None,
                    filter: None,
                };
                let held = rrd_vector::latest_versions(&visibility, history)
                    .map_err(core_vector)?
                    .into_values()
                    .any(|latest| {
                        addressed(latest)
                            && latest
                                .vector
                                .valid_to
                                .is_none_or(|valid_to| valid_at < valid_to)
                    });
                if held {
                    return Err(ServiceError::Vector(format!(
                        "vector collection {} still contains live or future points; retire them through one transaction at or before the deletion time",
                        request.collection_id
                    )));
                }
            }
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub(in crate::engine) fn validate_collection_vector_mutations(
        &self,
        request: &CommitTransaction,
    ) -> Result<()> {
        if !request
            .mutations
            .iter()
            .any(|mutation| matches!(mutation, TransactionMutation::PutVector { .. }))
        {
            return Ok(());
        }
        let scope = ScopeId::new(format!("instance:{}", self.instance)).map_err(core_vector)?;
        let catalogue = rrd_vector::VectorCollectionRepository::new(&self.storage, scope)
            .load()
            .map_err(vector_collection_error)?;
        for mutation in &request.mutations {
            let TransactionMutation::PutVector {
                collection_id,
                vector_name,
                field,
                value,
                provenance,
                properties,
                ..
            } = mutation
            else {
                continue;
            };
            let collection = catalogue
                .collections
                .get(&ProjectionId::new(collection_id.as_str()).map_err(core_vector)?)
                .ok_or_else(|| {
                    ServiceError::Vector(format!("unknown vector collection {collection_id}"))
                })?;
            let config = collection
                .definition
                .vectors
                .get(&ProjectionId::new(vector_name.as_str()).map_err(core_vector)?)
                .ok_or_else(|| {
                    ServiceError::Vector(format!(
                        "unknown named vector {vector_name} in collection {collection_id}"
                    ))
                })?;
            if config.field != field.as_str() {
                return Err(ServiceError::Vector(
                    "vector mutation field differs from its named-vector contract".into(),
                ));
            }
            let query = match value {
                DataVectorValue::Dense { values } => VectorSearchQuery::Dense {
                    values: values.clone(),
                },
                DataVectorValue::Sparse {
                    dimensions,
                    indices,
                    values,
                } => VectorSearchQuery::Sparse {
                    dimensions: *dimensions,
                    indices: indices.clone(),
                    values: values.clone(),
                },
                DataVectorValue::MultiDense {
                    dimensions,
                    vectors,
                } => VectorSearchQuery::MultiDense {
                    dimensions: *dimensions,
                    vectors: vectors.clone(),
                    comparator: rrd_contract::MultiVectorComparator::MaxSim,
                },
            };
            validate_collection_query(config, &query)?;
            if let Some(model) = &config.embedding_model {
                let provenance = provenance.as_ref().ok_or_else(|| {
                    ServiceError::Vector(
                        "model-bound named vector requires embedding provenance".into(),
                    )
                })?;
                if provenance.model != model.name || provenance.model_sha256 != model.digest {
                    return Err(ServiceError::Vector(
                        "vector mutation provenance differs from its named-vector model".into(),
                    ));
                }
            }
            validate_payload_index_values(collection, properties)?;
        }
        Ok(())
    }
}
