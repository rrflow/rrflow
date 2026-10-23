use super::super::*;
use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use rrd_store::{
    FunctionArtifactMediaTypeRecord, FunctionArtifactRecord, FunctionCatalogueMembershipRecord,
    FunctionCataloguePublication, FunctionCatalogueSnapshot, FunctionDefinitionRecord,
    TransactionFunctionBindingRecord, FUNCTION_CATALOGUE_STATE_FORMAT_VERSION,
};

impl RrdEngine {
    #[allow(clippy::too_many_arguments)]
    pub fn replace_function_catalogue(
        &self,
        session_id: &CorrelationId,
        token: &CorrelationId,
        request: &ReplaceFunctionCatalogue,
        now: u64,
        request_id: &str,
        operation_id: &str,
    ) -> Result<FunctionCatalogue> {
        request
            .validate()
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        self.authorize(
            session_id,
            token,
            SecurityAction::FunctionCatalogueWrite,
            now,
            request_id,
            operation_id,
        )?;
        let _transaction_guard = self.admit_transaction(request, request_id, operation_id)?;
        for definition in request.catalogue.functions.values() {
            super::runtime_profile::validate_runtime_identity(&definition.runtime)?;
        }
        let stored_current = self
            .storage
            .function_catalogue()
            .current(self.instance.as_str())?;
        let current = decode_catalogue_snapshot(stored_current.as_ref())?;
        if current.revision == request.catalogue.revision
            && current.sha256() == request.catalogue.sha256()
        {
            return Ok(current);
        }
        if current.revision != request.expected_revision {
            return Err(ServiceError::StorageConflict(format!(
                "function catalogue expected revision {} but current revision is {}",
                request.expected_revision, current.revision
            )));
        }
        self.validate_definition_lineage(&current, &request.catalogue)?;
        let publication = encode_publication(
            request,
            stored_current
                .as_ref()
                .map(|snapshot| snapshot.head.membership_sha256.clone()),
            now,
            session_id,
            request_id,
            operation_id,
        )?;
        let stored = self
            .storage
            .function_catalogue()
            .publish(self.instance.as_str(), &publication)
            .map_err(map_function_store_error)?;
        let accepted = decode_catalogue_snapshot(Some(&stored))?;
        if accepted != request.catalogue {
            return Err(ServiceError::Storage(
                "published function catalogue differs from the accepted request".into(),
            ));
        }
        Ok(accepted)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn function_catalogue(
        &self,
        session_id: &CorrelationId,
        token: &CorrelationId,
        now: u64,
        request_id: &str,
        operation_id: &str,
    ) -> Result<FunctionCatalogue> {
        self.authorize(
            session_id,
            token,
            SecurityAction::FunctionCatalogueRead,
            now,
            request_id,
            operation_id,
        )?;
        self.load_current_function_catalogue()
    }

    pub(in crate::engine) fn load_current_function_catalogue(&self) -> Result<FunctionCatalogue> {
        let snapshot = self
            .storage
            .function_catalogue()
            .current(self.instance.as_str())?;
        decode_catalogue_snapshot(snapshot.as_ref())
    }

    pub(in crate::engine) fn load_function_catalogue_revision(
        &self,
        revision: u64,
    ) -> Result<FunctionCatalogue> {
        if revision == 0 {
            return Ok(FunctionCatalogue::empty());
        }
        let snapshot = self
            .storage
            .function_catalogue()
            .revision(self.instance.as_str(), revision)?
            .ok_or(ServiceError::FunctionCatalogueRevisionNotFound)?;
        decode_catalogue_snapshot(Some(&snapshot))
    }

    fn validate_definition_lineage(
        &self,
        current: &FunctionCatalogue,
        replacement: &FunctionCatalogue,
    ) -> Result<()> {
        for (id, next) in &replacement.functions {
            let previous = match current.functions.get(id) {
                Some(previous) => Some(previous.clone()),
                None => self.latest_function_definition(id, current.revision)?,
            };
            validate_function_definition_successor(id, previous.as_ref(), next)?;
        }
        for (id, next) in &replacement.transaction_bindings {
            let previous = match current.transaction_bindings.get(id) {
                Some(previous) => Some(previous.clone()),
                None => self.latest_transaction_binding(id, current.revision)?,
            };
            validate_transaction_binding_successor(id, previous.as_ref(), next)?;
        }
        Ok(())
    }

    fn latest_function_definition(
        &self,
        function_id: &CanonicalId,
        through_revision: u64,
    ) -> Result<Option<FunctionDefinition>> {
        for revision in (1..=through_revision).rev() {
            let catalogue = self.load_function_catalogue_revision(revision)?;
            if let Some(definition) = catalogue.functions.get(function_id) {
                return Ok(Some(definition.clone()));
            }
        }
        Ok(None)
    }

    fn latest_transaction_binding(
        &self,
        binding_id: &CanonicalId,
        through_revision: u64,
    ) -> Result<Option<TransactionFunctionBinding>> {
        for revision in (1..=through_revision).rev() {
            let catalogue = self.load_function_catalogue_revision(revision)?;
            if let Some(binding) = catalogue.transaction_bindings.get(binding_id) {
                return Ok(Some(binding.clone()));
            }
        }
        Ok(None)
    }
}

fn encode_publication(
    request: &ReplaceFunctionCatalogue,
    predecessor_sha256: Option<String>,
    now: u64,
    session_id: &CorrelationId,
    request_id: &str,
    operation_id: &str,
) -> Result<FunctionCataloguePublication> {
    let artifacts = request
        .catalogue
        .artifacts
        .values()
        .map(|artifact| {
            Ok(FunctionArtifactRecord {
                content_sha256: artifact.content_sha256.clone(),
                media_type: match artifact.media_type {
                    FunctionArtifactMediaType::JavaScriptUtf8 => {
                        FunctionArtifactMediaTypeRecord::JavaScriptUtf8
                    }
                    FunctionArtifactMediaType::WebAssemblyBinary => {
                        FunctionArtifactMediaTypeRecord::WebAssemblyBinary
                    }
                },
                content: artifact
                    .decoded_content()
                    .map_err(|error| ServiceError::Contract(error.to_string()))?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let definitions = request
        .catalogue
        .functions
        .values()
        .map(|definition| {
            Ok(FunctionDefinitionRecord {
                format_version: FUNCTION_CATALOGUE_STATE_FORMAT_VERSION,
                function_id: definition.function_id.as_str().into(),
                revision: definition.revision,
                predecessor_sha256: definition.predecessor_sha256.clone(),
                artifact_sha256: definition.runtime.artifact_sha256().into(),
                runtime_profile: definition.runtime.runtime_profile().as_str().into(),
                runtime_build_sha256: definition.runtime.runtime_build_sha256().into(),
                input_schema_sha256: definition.input_schema_sha256.clone(),
                output_schema_sha256: definition.output_schema_sha256.clone(),
                definition_sha256: definition.sha256(),
                canonical_definition_json: serde_json::to_string(definition)
                    .map_err(contract_json)?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let bindings = request
        .catalogue
        .transaction_bindings
        .values()
        .map(|binding| {
            Ok(TransactionFunctionBindingRecord {
                format_version: FUNCTION_CATALOGUE_STATE_FORMAT_VERSION,
                binding_id: binding.binding_id.as_str().into(),
                revision: binding.revision,
                predecessor_sha256: binding.predecessor_sha256.clone(),
                function_id: binding.function_id.as_str().into(),
                function_definition_sha256: binding.function_definition_sha256.clone(),
                binding_sha256: binding.sha256(),
                canonical_binding_json: serde_json::to_string(binding).map_err(contract_json)?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let membership = FunctionCatalogueMembershipRecord {
        format_version: FUNCTION_CATALOGUE_STATE_FORMAT_VERSION,
        revision: request.catalogue.revision,
        predecessor_sha256,
        catalogue_sha256: request.catalogue.sha256(),
        artifact_sha256: request.catalogue.artifacts.keys().cloned().collect(),
        function_definition_sha256: request
            .catalogue
            .functions
            .iter()
            .map(|(id, definition)| (id.as_str().into(), definition.sha256()))
            .collect(),
        transaction_binding_sha256: request
            .catalogue
            .transaction_bindings
            .iter()
            .map(|(id, binding)| (id.as_str().into(), binding.sha256()))
            .collect(),
        published_at_unix_ms: now,
        published_by: format!("session:{}", session_id.as_str()),
        request_id: request_id.into(),
        operation_id: operation_id.into(),
        membership_sha256: String::new(),
    }
    .seal()
    .map_err(map_function_store_error)?;
    let publication = FunctionCataloguePublication {
        expected_revision: request.expected_revision,
        membership,
        artifacts,
        definitions,
        bindings,
    };
    publication.validate().map_err(map_function_store_error)?;
    Ok(publication)
}

fn decode_catalogue_snapshot(
    snapshot: Option<&FunctionCatalogueSnapshot>,
) -> Result<FunctionCatalogue> {
    let Some(snapshot) = snapshot else {
        return Ok(FunctionCatalogue::empty());
    };
    let artifacts = snapshot
        .artifacts
        .iter()
        .map(|artifact| {
            let artifact = FunctionArtifact {
                content_sha256: artifact.content_sha256.clone(),
                media_type: match artifact.media_type {
                    FunctionArtifactMediaTypeRecord::JavaScriptUtf8 => {
                        FunctionArtifactMediaType::JavaScriptUtf8
                    }
                    FunctionArtifactMediaTypeRecord::WebAssemblyBinary => {
                        FunctionArtifactMediaType::WebAssemblyBinary
                    }
                },
                byte_length: artifact.content.len().try_into().map_err(|_| {
                    ServiceError::Storage("function artifact length exceeds u32".into())
                })?,
                content_base64: STANDARD.encode(&artifact.content),
            };
            Ok((artifact.content_sha256.clone(), artifact))
        })
        .collect::<Result<BTreeMap<_, _>>>()?;
    let functions = snapshot
        .definitions
        .iter()
        .map(|record| {
            let definition: FunctionDefinition =
                serde_json::from_str(&record.canonical_definition_json).map_err(contract_json)?;
            definition
                .validate()
                .map_err(|error| ServiceError::Contract(error.to_string()))?;
            if definition.function_id.as_str() != record.function_id
                || definition.revision != record.revision
                || definition.predecessor_sha256 != record.predecessor_sha256
                || definition.runtime.artifact_sha256() != record.artifact_sha256
                || definition.runtime.runtime_profile().as_str() != record.runtime_profile
                || definition.runtime.runtime_build_sha256() != record.runtime_build_sha256
                || definition.input_schema_sha256 != record.input_schema_sha256
                || definition.output_schema_sha256 != record.output_schema_sha256
                || definition.sha256() != record.definition_sha256
            {
                return Err(ServiceError::Storage(
                    "function definition metadata differs from canonical definition".into(),
                ));
            }
            Ok((definition.function_id.clone(), definition))
        })
        .collect::<Result<BTreeMap<_, _>>>()?;
    let transaction_bindings = snapshot
        .bindings
        .iter()
        .map(|record| {
            let binding: TransactionFunctionBinding =
                serde_json::from_str(&record.canonical_binding_json).map_err(contract_json)?;
            if binding.binding_id.as_str() != record.binding_id
                || binding.revision != record.revision
                || binding.predecessor_sha256 != record.predecessor_sha256
                || binding.function_id.as_str() != record.function_id
                || binding.function_definition_sha256 != record.function_definition_sha256
                || binding.sha256() != record.binding_sha256
            {
                return Err(ServiceError::Storage(
                    "function binding metadata differs from canonical binding".into(),
                ));
            }
            Ok((binding.binding_id.clone(), binding))
        })
        .collect::<Result<BTreeMap<_, _>>>()?;
    let catalogue = FunctionCatalogue {
        contract_version: rrd_contract::FUNCTION_CONTRACT_VERSION,
        revision: snapshot.head.revision,
        artifacts,
        functions,
        transaction_bindings,
    };
    catalogue
        .validate()
        .map_err(|error| ServiceError::Contract(error.to_string()))?;
    if catalogue.sha256() != snapshot.head.catalogue_sha256 {
        return Err(ServiceError::Storage(
            "function catalogue digest differs from its active membership".into(),
        ));
    }
    Ok(catalogue)
}

fn validate_function_definition_successor(
    id: &CanonicalId,
    previous: Option<&FunctionDefinition>,
    next: &FunctionDefinition,
) -> Result<()> {
    match previous {
        Some(previous) if previous == next => Ok(()),
        Some(previous)
            if previous.revision.checked_add(1) == Some(next.revision)
                && next.predecessor_sha256.as_deref() == Some(previous.sha256().as_str()) =>
        {
            Ok(())
        }
        Some(_) => Err(ServiceError::Contract(format!(
            "function definition {id} does not extend its latest immutable revision"
        ))),
        None if next.revision == 1 && next.predecessor_sha256.is_none() => Ok(()),
        None => Err(ServiceError::Contract(format!(
            "new function definition {id} must begin at revision one"
        ))),
    }
}

fn validate_transaction_binding_successor(
    id: &CanonicalId,
    previous: Option<&TransactionFunctionBinding>,
    next: &TransactionFunctionBinding,
) -> Result<()> {
    match previous {
        Some(previous) if previous == next => Ok(()),
        Some(previous)
            if previous.revision.checked_add(1) == Some(next.revision)
                && next.predecessor_sha256.as_deref() == Some(previous.sha256().as_str()) =>
        {
            Ok(())
        }
        Some(_) => Err(ServiceError::Contract(format!(
            "transaction function binding {id} does not extend its latest immutable revision"
        ))),
        None if next.revision == 1 && next.predecessor_sha256.is_none() => Ok(()),
        None => Err(ServiceError::Contract(format!(
            "new transaction function binding {id} must begin at revision one"
        ))),
    }
}

fn map_function_store_error(error: rrd_store::Error) -> ServiceError {
    match error {
        rrd_store::Error::FunctionConstraint(message)
            if message.contains("expected revision")
                || message.contains("already contains other bytes") =>
        {
            ServiceError::StorageConflict(message)
        }
        other => ServiceError::from(other),
    }
}
