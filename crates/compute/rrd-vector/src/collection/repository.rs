//! The vector collection repository and its operation budgets.

use super::*;

impl<'a, E: StorageEngine> VectorCollectionRepository<'a, E> {
    pub fn new(engine: &'a E, scope: ScopeId) -> Self {
        let key = format!("server/state/vector-collection-catalogue/{scope}");
        Self { engine, scope, key }
    }

    pub fn load(&self) -> CollectionResult<CollectionCatalogue> {
        let Some(bytes) = self.engine.control().get(&self.key)? else {
            return Ok(CollectionCatalogue::empty(self.scope.clone()));
        };
        let catalogue: CollectionCatalogue = serde_json::from_slice(&bytes)
            .map_err(|error| CollectionError::Integrity(error.to_string()))?;
        if catalogue.scope != self.scope {
            return integrity("collection catalogue scope differs from its control key");
        }
        catalogue.validate()?;
        Ok(catalogue)
    }

    pub fn operation_receipt(
        &self,
        idempotency_key: &str,
        operation_digest: &str,
    ) -> CollectionResult<Option<CollectionOperationReceipt>> {
        let catalogue = self.load()?;
        if catalogue.deletions.contains_key(idempotency_key)
            || catalogue.payload_operations.contains_key(idempotency_key)
        {
            return Err(CollectionError::IdempotencyConflict);
        }
        let Some(receipt) = catalogue.operations.get(idempotency_key) else {
            return Ok(None);
        };
        if receipt.operation_digest != operation_digest {
            return Err(CollectionError::IdempotencyConflict);
        }
        Ok(Some(receipt.clone()))
    }

    pub fn deletion_receipt(
        &self,
        idempotency_key: &str,
        operation_digest: &str,
    ) -> CollectionResult<Option<CollectionDeletionReceipt>> {
        let catalogue = self.load()?;
        if catalogue.operations.contains_key(idempotency_key)
            || catalogue.payload_operations.contains_key(idempotency_key)
        {
            return Err(CollectionError::IdempotencyConflict);
        }
        let Some(receipt) = catalogue.deletions.get(idempotency_key) else {
            return Ok(None);
        };
        if receipt.operation_digest != operation_digest {
            return Err(CollectionError::IdempotencyConflict);
        }
        Ok(Some(receipt.clone()))
    }

    pub fn payload_operation_receipt(
        &self,
        idempotency_key: &str,
        operation_digest: &str,
    ) -> CollectionResult<Option<PayloadIndexOperationReceipt>> {
        let catalogue = self.load()?;
        if catalogue.operations.contains_key(idempotency_key)
            || catalogue.deletions.contains_key(idempotency_key)
        {
            return Err(CollectionError::IdempotencyConflict);
        }
        let Some(receipt) = catalogue.payload_operations.get(idempotency_key) else {
            return Ok(None);
        };
        if receipt.operation_digest != operation_digest {
            return Err(CollectionError::IdempotencyConflict);
        }
        Ok(Some(receipt.clone()))
    }

    pub fn ensure(
        &self,
        context: &CollectionMutationContext,
        idempotency_key: String,
        operation_digest: String,
        definition: VectorCollectionDefinition,
    ) -> CollectionResult<(CollectionCatalogue, bool)> {
        context.validate()?;
        definition.validate()?;
        if idempotency_key.is_empty() || !valid_digest(&operation_digest) {
            return invalid("collection ensure requires an idempotency key and SHA-256 digest");
        }
        if self
            .operation_receipt(&idempotency_key, &operation_digest)?
            .is_some()
        {
            return Ok((self.load()?, true));
        }
        let expected = self.engine.control().get(&self.key)?;
        let mut catalogue = match &expected {
            Some(bytes) => serde_json::from_slice(bytes)
                .map_err(|error| CollectionError::Integrity(error.to_string()))?,
            None => CollectionCatalogue::empty(self.scope.clone()),
        };
        catalogue.validate()?;
        if catalogue.collections.len() >= MAX_COLLECTIONS
            && !catalogue.collections.contains_key(&definition.id)
        {
            return Err(CollectionError::Budget("collection limit exceeded".into()));
        }
        require_operation_budget(&catalogue)?;
        let configuration_digest = definition.config_digest()?;
        let entry = match catalogue.collections.get(&definition.id) {
            Some(existing) if existing.configuration_digest == configuration_digest => {
                existing.clone()
            }
            Some(existing) => CollectionEntry {
                definition,
                payload_indexes: existing.payload_indexes.clone(),
                generation: existing.generation.checked_add(1).ok_or_else(|| {
                    CollectionError::Integrity("collection generation overflow".into())
                })?,
                created_at: existing.created_at,
                updated_at: context.at,
                configuration_digest,
            },
            None => CollectionEntry {
                definition,
                payload_indexes: BTreeMap::new(),
                generation: 1,
                created_at: context.at,
                updated_at: context.at,
                configuration_digest,
            },
        };
        entry.validate()?;
        catalogue
            .collections
            .insert(entry.definition.id.clone(), entry.clone());
        catalogue.operations.insert(
            idempotency_key.clone(),
            CollectionOperationReceipt {
                operation_digest: operation_digest.clone(),
                entry,
            },
        );
        catalogue.revision = catalogue
            .revision
            .checked_add(1)
            .ok_or_else(|| CollectionError::Integrity("catalogue revision overflow".into()))?;
        catalogue.validate()?;
        let replacement = serde_json::to_vec(&catalogue)
            .map_err(|error| CollectionError::Integrity(error.to_string()))?;
        match self.engine.control().commit_catalog(
            &self.scope,
            &ControlTransition {
                key: self.key.clone(),
                expected,
                replacement: Some(replacement),
                at: context.at,
                actor: context.actor.clone(),
                action: "vector_collection.ensured".into(),
                request_id: context.request_id.clone(),
                operation_id: context.operation_id.clone(),
            },
        ) {
            Ok(_) => Ok((catalogue, false)),
            Err(rrd_store::Error::ControlConflict(_)) => {
                if self
                    .operation_receipt(&idempotency_key, &operation_digest)?
                    .is_some()
                {
                    Ok((self.load()?, true))
                } else {
                    Err(CollectionError::Store(rrd_store::Error::ControlConflict(
                        self.key.clone(),
                    )))
                }
            }
            Err(error) => Err(CollectionError::Store(error)),
        }
    }

    pub fn delete(
        &self,
        context: &CollectionMutationContext,
        idempotency_key: String,
        operation_digest: String,
        collection_id: &ProjectionId,
    ) -> CollectionResult<(CollectionCatalogue, CollectionEntry, bool)> {
        context.validate()?;
        validate_operation_identity(&idempotency_key, &operation_digest, "collection delete")?;
        if let Some(receipt) = self.deletion_receipt(&idempotency_key, &operation_digest)? {
            return Ok((self.load()?, receipt.deleted_entry, true));
        }
        let expected = self.engine.control().get(&self.key)?;
        let mut catalogue = catalogue_from_expected(&self.scope, &expected)?;
        require_operation_budget(&catalogue)?;
        let deleted_entry = catalogue.collections.remove(collection_id).ok_or_else(|| {
            CollectionError::Invalid(format!("unknown vector collection {collection_id}"))
        })?;
        catalogue.deletions.insert(
            idempotency_key.clone(),
            CollectionDeletionReceipt {
                operation_digest: operation_digest.clone(),
                deleted_entry: deleted_entry.clone(),
            },
        );
        bump_catalogue_revision(&mut catalogue)?;
        let (catalogue, replayed) = self.commit_mutation(
            context,
            expected,
            catalogue,
            "vector_collection.deleted",
            || self.deletion_receipt(&idempotency_key, &operation_digest),
        )?;
        let deleted_entry = if replayed {
            self.deletion_receipt(&idempotency_key, &operation_digest)?
                .ok_or_else(|| {
                    CollectionError::Integrity("winning deletion receipt disappeared".into())
                })?
                .deleted_entry
        } else {
            deleted_entry
        };
        Ok((catalogue, deleted_entry, replayed))
    }

    pub fn ensure_payload_index(
        &self,
        context: &CollectionMutationContext,
        idempotency_key: String,
        operation_digest: String,
        collection_id: &ProjectionId,
        definition: PayloadIndexDefinition,
    ) -> CollectionResult<(CollectionCatalogue, PayloadIndexEntry, bool)> {
        context.validate()?;
        validate_operation_identity(&idempotency_key, &operation_digest, "payload-index ensure")?;
        if let Some(receipt) =
            self.payload_operation_receipt(&idempotency_key, &operation_digest)?
        {
            if receipt.deleted {
                return Err(CollectionError::IdempotencyConflict);
            }
            return Ok((self.load()?, receipt.index, true));
        }
        let expected = self.engine.control().get(&self.key)?;
        let mut catalogue = catalogue_from_expected(&self.scope, &expected)?;
        require_operation_budget(&catalogue)?;
        let collection = catalogue
            .collections
            .get_mut(collection_id)
            .ok_or_else(|| {
                CollectionError::Invalid(format!("unknown vector collection {collection_id}"))
            })?;
        if collection.payload_indexes.len() >= MAX_PAYLOAD_INDEXES
            && !collection.payload_indexes.contains_key(&definition.field)
        {
            return Err(CollectionError::Budget(
                "collection payload-index limit exceeded".into(),
            ));
        }
        let configuration_digest = definition.config_digest()?;
        let index = match collection.payload_indexes.get(&definition.field) {
            Some(existing) if existing.configuration_digest == configuration_digest => {
                existing.clone()
            }
            Some(existing) => PayloadIndexEntry {
                definition,
                generation: existing.generation.checked_add(1).ok_or_else(|| {
                    CollectionError::Integrity("payload-index generation overflow".into())
                })?,
                created_at: existing.created_at,
                updated_at: context.at,
                configuration_digest,
            },
            None => PayloadIndexEntry {
                definition,
                generation: 1,
                created_at: context.at,
                updated_at: context.at,
                configuration_digest,
            },
        };
        index.validate()?;
        let changed = collection
            .payload_indexes
            .get(&index.definition.field)
            .is_none_or(|existing| existing != &index);
        collection
            .payload_indexes
            .insert(index.definition.field.clone(), index.clone());
        if changed {
            bump_collection_generation(collection, context.at)?;
        }
        let receipt = PayloadIndexOperationReceipt {
            operation_digest: operation_digest.clone(),
            collection: collection.clone(),
            index: index.clone(),
            deleted: false,
        };
        catalogue
            .payload_operations
            .insert(idempotency_key.clone(), receipt);
        bump_catalogue_revision(&mut catalogue)?;
        let (catalogue, replayed) = self.commit_mutation(
            context,
            expected,
            catalogue,
            "vector_payload_index.ensured",
            || self.payload_operation_receipt(&idempotency_key, &operation_digest),
        )?;
        let index = if replayed {
            self.payload_operation_receipt(&idempotency_key, &operation_digest)?
                .ok_or_else(|| {
                    CollectionError::Integrity("winning payload-index receipt disappeared".into())
                })?
                .index
        } else {
            index
        };
        Ok((catalogue, index, replayed))
    }

    pub fn delete_payload_index(
        &self,
        context: &CollectionMutationContext,
        idempotency_key: String,
        operation_digest: String,
        collection_id: &ProjectionId,
        field: &ProjectionId,
    ) -> CollectionResult<(CollectionCatalogue, PayloadIndexEntry, bool)> {
        context.validate()?;
        validate_operation_identity(&idempotency_key, &operation_digest, "payload-index delete")?;
        if let Some(receipt) =
            self.payload_operation_receipt(&idempotency_key, &operation_digest)?
        {
            if !receipt.deleted {
                return Err(CollectionError::IdempotencyConflict);
            }
            return Ok((self.load()?, receipt.index, true));
        }
        let expected = self.engine.control().get(&self.key)?;
        let mut catalogue = catalogue_from_expected(&self.scope, &expected)?;
        require_operation_budget(&catalogue)?;
        let collection = catalogue
            .collections
            .get_mut(collection_id)
            .ok_or_else(|| {
                CollectionError::Invalid(format!("unknown vector collection {collection_id}"))
            })?;
        let index = collection.payload_indexes.remove(field).ok_or_else(|| {
            CollectionError::Invalid(format!(
                "unknown payload index {field} in collection {collection_id}"
            ))
        })?;
        bump_collection_generation(collection, context.at)?;
        let receipt = PayloadIndexOperationReceipt {
            operation_digest: operation_digest.clone(),
            collection: collection.clone(),
            index: index.clone(),
            deleted: true,
        };
        catalogue
            .payload_operations
            .insert(idempotency_key.clone(), receipt);
        bump_catalogue_revision(&mut catalogue)?;
        let (catalogue, replayed) = self.commit_mutation(
            context,
            expected,
            catalogue,
            "vector_payload_index.deleted",
            || self.payload_operation_receipt(&idempotency_key, &operation_digest),
        )?;
        let index = if replayed {
            self.payload_operation_receipt(&idempotency_key, &operation_digest)?
                .ok_or_else(|| {
                    CollectionError::Integrity("winning payload-index receipt disappeared".into())
                })?
                .index
        } else {
            index
        };
        Ok((catalogue, index, replayed))
    }

    fn commit_mutation<R>(
        &self,
        context: &CollectionMutationContext,
        expected: Option<Vec<u8>>,
        catalogue: CollectionCatalogue,
        action: &str,
        replay: impl FnOnce() -> CollectionResult<Option<R>>,
    ) -> CollectionResult<(CollectionCatalogue, bool)> {
        catalogue.validate()?;
        let replacement = serde_json::to_vec(&catalogue)
            .map_err(|error| CollectionError::Integrity(error.to_string()))?;
        match self.engine.control().commit_catalog(
            &self.scope,
            &ControlTransition {
                key: self.key.clone(),
                expected,
                replacement: Some(replacement),
                at: context.at,
                actor: context.actor.clone(),
                action: action.into(),
                request_id: context.request_id.clone(),
                operation_id: context.operation_id.clone(),
            },
        ) {
            Ok(_) => Ok((catalogue, false)),
            Err(rrd_store::Error::ControlConflict(_)) => {
                if replay()?.is_some() {
                    Ok((self.load()?, true))
                } else {
                    Err(CollectionError::Store(rrd_store::Error::ControlConflict(
                        self.key.clone(),
                    )))
                }
            }
            Err(error) => Err(CollectionError::Store(error)),
        }
    }
}

pub(super) fn catalogue_from_expected(
    scope: &ScopeId,
    expected: &Option<Vec<u8>>,
) -> CollectionResult<CollectionCatalogue> {
    let catalogue = match expected {
        Some(bytes) => serde_json::from_slice(bytes)
            .map_err(|error| CollectionError::Integrity(error.to_string()))?,
        None => CollectionCatalogue::empty(scope.clone()),
    };
    catalogue.validate()?;
    Ok(catalogue)
}

pub(super) fn require_operation_budget(catalogue: &CollectionCatalogue) -> CollectionResult<()> {
    let count = catalogue
        .operations
        .len()
        .checked_add(catalogue.deletions.len())
        .and_then(|count| count.checked_add(catalogue.payload_operations.len()))
        .ok_or_else(|| CollectionError::Integrity("operation receipt count overflow".into()))?;
    if count >= MAX_OPERATION_RECEIPTS {
        return Err(CollectionError::Budget(
            "operation receipt limit exceeded".into(),
        ));
    }
    Ok(())
}

pub(super) fn validate_operation_identity(
    idempotency_key: &str,
    operation_digest: &str,
    operation: &str,
) -> CollectionResult<()> {
    if idempotency_key.is_empty() || !valid_digest(operation_digest) {
        return invalid(format!(
            "{operation} requires an idempotency key and SHA-256 digest"
        ));
    }
    Ok(())
}

pub(super) fn bump_catalogue_revision(catalogue: &mut CollectionCatalogue) -> CollectionResult<()> {
    catalogue.revision = catalogue
        .revision
        .checked_add(1)
        .ok_or_else(|| CollectionError::Integrity("catalogue revision overflow".into()))?;
    Ok(())
}

pub(super) fn bump_collection_generation(
    collection: &mut CollectionEntry,
    at: u64,
) -> CollectionResult<()> {
    collection.generation = collection
        .generation
        .checked_add(1)
        .ok_or_else(|| CollectionError::Integrity("collection generation overflow".into()))?;
    collection.updated_at = at;
    Ok(())
}

pub(super) fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

pub(super) fn invalid<T>(message: impl Into<String>) -> CollectionResult<T> {
    Err(CollectionError::Invalid(message.into()))
}

pub(super) fn integrity<T>(message: impl Into<String>) -> CollectionResult<T> {
    Err(CollectionError::Integrity(message.into()))
}
