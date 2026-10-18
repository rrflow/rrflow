//! Durable project-bound configuration for logical knowledge trees.
//!
//! A native binding is checked against the current vector catalogue when it is
//! saved. External bindings are configuration only; this module invokes no
//! provider, crawler, search, or synchronization capability.

use super::*;
use rrd_contract::{
    KnowledgePathBinding, KnowledgeTreeCatalogue, ListKnowledgeTrees, ReplaceKnowledgeTrees,
    ReplaceKnowledgeTreesResult,
};

const KNOWLEDGE_STATE_FORMAT: u16 = 1;
const MAX_KNOWLEDGE_REPLACEMENTS: u64 = 100_000;
const MAX_KNOWLEDGE_CONTROL_BYTES: usize = 1024 * 1024;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct KnowledgeState {
    format_version: u16,
    catalogue: KnowledgeTreeCatalogue,
    catalogue_sha256: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct KnowledgeReceipt {
    format_version: u16,
    session_id: CorrelationId,
    idempotency_key: CorrelationId,
    operation_sha256: String,
    catalogue: KnowledgeTreeCatalogue,
    catalogue_sha256: String,
}

impl RrdEngine {
    #[allow(clippy::too_many_arguments)]
    pub fn list_knowledge_trees(
        &self,
        session_id: &CorrelationId,
        token: &CorrelationId,
        request: &ListKnowledgeTrees,
        now: u64,
        request_id: &str,
        operation_id: &str,
    ) -> Result<KnowledgeTreeCatalogue> {
        request.validate().map_err(knowledge_contract)?;
        self.authorize(
            session_id,
            token,
            SecurityAction::KnowledgeTreeRead,
            now,
            request_id,
            operation_id,
        )?;
        self.knowledge_scope(&request.project_id, &request.scope)?;
        let key = knowledge_key(&request.project_id, &request.scope)?;
        let bytes = self.storage.control().get(&key)?;
        decode_knowledge_state(bytes.as_deref(), &request.project_id, &request.scope)
    }

    pub fn replace_knowledge_trees(
        &self,
        session_id: &CorrelationId,
        token: &CorrelationId,
        request: &ReplaceKnowledgeTrees,
        context: &RequestContext,
        now: u64,
    ) -> Result<ReplaceKnowledgeTreesResult> {
        context.validate(true).map_err(knowledge_contract)?;
        request.validate().map_err(knowledge_contract)?;
        let idempotency_key = context.idempotency_key.as_ref().ok_or_else(|| {
            ServiceError::Contract("knowledge replacement requires idempotency".into())
        })?;
        // Replays still require a currently active session and current write
        // grant. A durable receipt is never an authorization bypass.
        self.authorize(
            session_id,
            token,
            SecurityAction::KnowledgeTreeWrite,
            now,
            context.request_id.as_str(),
            context.operation_id.as_str(),
        )?;
        let scope = self.knowledge_scope(&request.project_id, &request.scope)?;
        let operation_sha256 = operation_digest(request)?;
        let key = knowledge_key(&request.project_id, &request.scope)?;
        let receipt_key = knowledge_receipt_key(&key, session_id, idempotency_key)?;
        let _gate = self.transaction_gate.lock().map_err(|_| {
            ServiceError::Storage("knowledge replacement transaction gate is poisoned".into())
        })?;
        if let Some(catalogue) = self.knowledge_receipt(
            &receipt_key,
            session_id,
            idempotency_key,
            request,
            &operation_sha256,
        )? {
            return Ok(ReplaceKnowledgeTreesResult {
                catalogue,
                idempotent_replay: true,
            });
        }
        let expected = self.storage.control().get(&key)?;
        let previous =
            decode_knowledge_state(expected.as_deref(), &request.project_id, &request.scope)?;
        if request.expected_revision != previous.revision {
            return Err(ServiceError::StorageConflict(format!(
                "knowledge catalogue revision is {}, expected {}",
                previous.revision, request.expected_revision
            )));
        }
        if previous.revision >= MAX_KNOWLEDGE_REPLACEMENTS {
            return Err(ServiceError::ConfigurationLimit(
                "knowledge replacement receipt budget exhausted".into(),
            ));
        }
        request
            .validate_against(&previous)
            .map_err(knowledge_contract)?;
        self.validate_native_knowledge_paths(request, scope)?;
        let catalogue = request
            .clone()
            .into_catalogue()
            .map_err(knowledge_contract)?;
        let catalogue_sha256 = catalogue.sha256().map_err(knowledge_contract)?;
        let state = KnowledgeState {
            format_version: KNOWLEDGE_STATE_FORMAT,
            catalogue: catalogue.clone(),
            catalogue_sha256: catalogue_sha256.clone(),
        };
        let receipt = KnowledgeReceipt {
            format_version: KNOWLEDGE_STATE_FORMAT,
            session_id: session_id.clone(),
            idempotency_key: idempotency_key.clone(),
            operation_sha256: operation_sha256.clone(),
            catalogue: catalogue.clone(),
            catalogue_sha256,
        };
        let transition = |key, expected, replacement, action: &str| ControlTransition {
            key,
            expected,
            replacement: Some(replacement),
            at: now,
            actor: format!("session:{}", session_id.as_str()),
            action: action.into(),
            request_id: context.request_id.as_str().into(),
            operation_id: context.operation_id.as_str().into(),
        };
        // Catalogue and exact accepted response become durable together. The
        // receipt preserves that response even if later replacements succeed.
        match self.storage.control().commit_batch(&[
            transition(
                key,
                expected,
                encode_knowledge_control(&state)?,
                "knowledge_trees.replaced",
            ),
            transition(
                receipt_key.clone(),
                None,
                encode_knowledge_control(&receipt)?,
                "knowledge_trees.receipted",
            ),
        ]) {
            Ok(_) => Ok(ReplaceKnowledgeTreesResult {
                catalogue,
                idempotent_replay: false,
            }),
            Err(error @ rrd_store::Error::ControlConflict(_)) => {
                if let Some(catalogue) = self.knowledge_receipt(
                    &receipt_key,
                    session_id,
                    idempotency_key,
                    request,
                    &operation_sha256,
                )? {
                    Ok(ReplaceKnowledgeTreesResult {
                        catalogue,
                        idempotent_replay: true,
                    })
                } else {
                    Err(error.into())
                }
            }
            Err(error) => Err(error.into()),
        }
    }

    fn knowledge_scope(&self, project_id: &CanonicalId, scope: &str) -> Result<ScopeId> {
        if self
            .installed_estate_identity()
            .is_none_or(|identity| &identity.project_id != project_id)
        {
            return Err(ServiceError::ProjectBindingMismatch);
        }
        self.query_scope(scope)
    }

    fn knowledge_receipt(
        &self,
        key: &str,
        session_id: &CorrelationId,
        idempotency_key: &CorrelationId,
        request: &ReplaceKnowledgeTrees,
        operation_sha256: &str,
    ) -> Result<Option<KnowledgeTreeCatalogue>> {
        let Some(bytes) = self.storage.control().get(key)? else {
            return Ok(None);
        };
        require_bounded_knowledge_record(&bytes)?;
        let receipt: KnowledgeReceipt = serde_json::from_slice(&bytes)
            .map_err(|_| ServiceError::Storage("knowledge receipt cannot be decoded".into()))?;
        validate_knowledge_snapshot(
            receipt.format_version,
            &receipt.catalogue,
            &receipt.catalogue_sha256,
            &request.project_id,
            &request.scope,
        )?;
        if &receipt.session_id != session_id || &receipt.idempotency_key != idempotency_key {
            return Err(ServiceError::Storage(
                "knowledge receipt identity differs from its control key".into(),
            ));
        }
        if receipt.operation_sha256 != operation_sha256 {
            return Err(ServiceError::IdempotencyConflict);
        }
        let accepted = request
            .clone()
            .into_catalogue()
            .map_err(knowledge_contract)?;
        if accepted != receipt.catalogue {
            return Err(ServiceError::Storage(
                "knowledge receipt content differs from its operation".into(),
            ));
        }
        Ok(Some(receipt.catalogue))
    }

    fn validate_native_knowledge_paths(
        &self,
        request: &ReplaceKnowledgeTrees,
        scope: ScopeId,
    ) -> Result<()> {
        let native_paths: Vec<_> = request
            .trees
            .iter()
            .flat_map(|tree| &tree.paths)
            .filter_map(|path| match &path.binding {
                KnowledgePathBinding::Native {
                    collection_id,
                    vector_name,
                } => Some((&path.id, collection_id, vector_name)),
                _ => None,
            })
            .collect();
        if native_paths.is_empty() {
            return Ok(());
        }
        let collections = rrd_vector::VectorCollectionRepository::new(&self.storage, scope)
            .load()
            .map_err(|error| ServiceError::Storage(error.to_string()))?;
        for (path, collection_id, vector_name) in native_paths {
            let collection_key = ProjectionId::new(collection_id.as_str())
                .map_err(|error| ServiceError::Contract(error.to_string()))?;
            let vector_key = ProjectionId::new(vector_name.as_str())
                .map_err(|error| ServiceError::Contract(error.to_string()))?;
            let collection = collections
                .collections
                .get(&collection_key)
                .ok_or_else(|| {
                    ServiceError::Contract(format!(
                        "knowledge path {path} names unknown native collection {collection_id}"
                    ))
                })?;
            if !collection.definition.vectors.contains_key(&vector_key) {
                return Err(ServiceError::Contract(format!(
                    "knowledge path {path} names unknown vector {vector_name} in {collection_id}"
                )));
            }
        }
        Ok(())
    }
}

fn knowledge_key(project_id: &CanonicalId, scope: &str) -> Result<String> {
    Ok(format!(
        "server/state/knowledge-trees/{}",
        operation_digest(&(project_id, scope))?
    ))
}

fn knowledge_receipt_key(
    catalogue_key: &str,
    session_id: &CorrelationId,
    idempotency_key: &CorrelationId,
) -> Result<String> {
    Ok(format!(
        "{catalogue_key}/receipt/{}",
        operation_digest(&(session_id, idempotency_key))?
    ))
}

fn decode_knowledge_state(
    bytes: Option<&[u8]>,
    project_id: &CanonicalId,
    scope: &str,
) -> Result<KnowledgeTreeCatalogue> {
    let Some(bytes) = bytes else {
        return KnowledgeTreeCatalogue::empty(project_id.clone(), scope.into())
            .map_err(knowledge_contract);
    };
    require_bounded_knowledge_record(bytes)?;
    let state: KnowledgeState = serde_json::from_slice(bytes)
        .map_err(|_| ServiceError::Storage("knowledge catalogue cannot be decoded".into()))?;
    validate_knowledge_snapshot(
        state.format_version,
        &state.catalogue,
        &state.catalogue_sha256,
        project_id,
        scope,
    )?;
    Ok(state.catalogue)
}

fn validate_knowledge_snapshot(
    format_version: u16,
    catalogue: &KnowledgeTreeCatalogue,
    catalogue_sha256: &str,
    project_id: &CanonicalId,
    scope: &str,
) -> Result<()> {
    if format_version != KNOWLEDGE_STATE_FORMAT
        || catalogue.project_id != *project_id
        || catalogue.scope != scope
        || catalogue.revision == 0
        || catalogue.revision > MAX_KNOWLEDGE_REPLACEMENTS
    {
        return Err(ServiceError::Storage(
            "knowledge catalogue identity, format or revision is invalid".into(),
        ));
    }
    let actual_digest = catalogue.sha256().map_err(|_| {
        ServiceError::Storage("persisted knowledge catalogue violates its contract".into())
    })?;
    if catalogue_sha256 != actual_digest {
        return Err(ServiceError::Storage(
            "knowledge catalogue digest differs from its content".into(),
        ));
    }
    Ok(())
}

fn require_bounded_knowledge_record(bytes: &[u8]) -> Result<()> {
    if bytes.len() > MAX_KNOWLEDGE_CONTROL_BYTES {
        return Err(ServiceError::Storage(
            "persisted knowledge record exceeds the control byte budget".into(),
        ));
    }
    Ok(())
}

fn encode_knowledge_control(value: &impl Serialize) -> Result<Vec<u8>> {
    let bytes = serde_json::to_vec(value).map_err(contract_json)?;
    if bytes.len() > MAX_KNOWLEDGE_CONTROL_BYTES {
        return Err(ServiceError::ConfigurationLimit(
            "knowledge state exceeds the control record byte budget".into(),
        ));
    }
    Ok(bytes)
}

fn knowledge_contract(error: rrd_contract::ContractError) -> ServiceError {
    ServiceError::Contract(error.to_string())
}
