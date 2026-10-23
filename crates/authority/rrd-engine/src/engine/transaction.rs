use super::*;

impl RrdEngine {
    pub fn begin_transaction(
        &self,
        session_id: &CorrelationId,
        token: &CorrelationId,
        request: &BeginTransaction,
        context: &RequestContext,
        now: u64,
    ) -> Result<TransactionLease> {
        self.begin_transaction_with_intent(session_id, token, request, context, now, None)
    }

    /// Begin a transaction whose persisted idempotency record is also bound to
    /// a higher-level canonical intent. Ordinary public transactions pass no
    /// intent and retain their exact pre-existing operation identity.
    pub(super) fn begin_transaction_with_intent(
        &self,
        session_id: &CorrelationId,
        token: &CorrelationId,
        request: &BeginTransaction,
        context: &RequestContext,
        now: u64,
        intent_sha256: Option<&str>,
    ) -> Result<TransactionLease> {
        context
            .validate(true)
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        let idempotency_key = context.idempotency_key.as_ref().ok_or_else(|| {
            ServiceError::Contract("transaction begin requires idempotency".into())
        })?;
        request
            .validate()
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        if !matches!(request.scope.as_str(), "claims" | "data") {
            return Err(ServiceError::WrongScope);
        }
        let _transaction_guard = self.admit_transaction(
            request,
            context.request_id.as_str(),
            context.operation_id.as_str(),
        )?;
        let (bytes, mut state) = self.load_authenticated(session_id, token)?;
        self.authorize_session_policy(&state, SecurityAction::TransactionBegin, now)?;
        let operation_sha256 = match intent_sha256 {
            Some(intent_sha256) => operation_digest(&(request, intent_sha256))?,
            None => operation_digest(request)?,
        };
        let transaction_id = self.keyed_id(
            "transaction",
            &[session_id.as_str(), idempotency_key.as_str()],
        )?;
        if let Some(existing) = state.transactions.get(&transaction_id) {
            if existing.begin_idempotency_key != *idempotency_key
                || existing.begin_operation_sha256 != operation_sha256
            {
                return Err(ServiceError::IdempotencyConflict);
            }
            return Ok(existing.lease.clone());
        }
        // Exact idempotent replays above remain observable after the session
        // lease expires. Only creation of a new transaction requires a live
        // session. This ordering is required for crash/reopen recovery: a
        // client can recover the stable transaction identity and replay the
        // commit without creating a second transaction.
        self.require_active_or_expire(
            session_id,
            bytes.clone(),
            &mut state,
            now,
            context.request_id.as_str(),
            context.operation_id.as_str(),
        )?;
        let open = state
            .transactions
            .values()
            .filter(|transaction| transaction.lease.state == TransactionState::Open)
            .count();
        if open >= usize::from(state.limits.max_open_transactions) {
            return Err(ServiceError::TransactionQuota);
        }
        let expires_at = now
            .checked_add(request.timeout_ms)
            .ok_or_else(|| ServiceError::Contract("transaction expiry overflow".into()))?
            .min(state.absolute_expires_at_unix_ms);
        let scope = ScopeId::new(format!("instance:{}", self.instance))
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        let read = self.storage.runtime().read_stamp(&scope)?;
        let transaction = TransactionLease {
            transaction_id: transaction_id.clone(),
            session_id: session_id.clone(),
            scope: request.scope.clone(),
            read_cursor: read.commit_cursor,
            expires_at_unix_ms: expires_at,
            state: TransactionState::Open,
        };
        state.transactions.insert(
            transaction_id,
            TransactionRecord {
                lease: transaction.clone(),
                read,
                begin_idempotency_key: idempotency_key.clone(),
                begin_operation_sha256: operation_sha256,
                prepared: None,
                commit_intent: None,
                commit_receipt: None,
                abort: None,
            },
        );
        touch_session(&mut state, now);
        self.replace_session(
            session_id,
            bytes,
            state,
            now,
            "transaction.began",
            context.request_id.as_str(),
            context.operation_id.as_str(),
        )?;
        Ok(transaction)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn commit_transaction(
        &self,
        session_id: &CorrelationId,
        token: &CorrelationId,
        transaction_id: &CorrelationId,
        idempotency_key: &CorrelationId,
        request: &CommitTransaction,
        now: u64,
        request_id: &str,
        operation_id: &str,
    ) -> Result<CommitReceipt> {
        request
            .validate()
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        let _transaction_guard = self.admit_transaction(request, request_id, operation_id)?;
        let actual_digest = request.computed_operation_sha256();
        if actual_digest != request.operation_sha256 {
            return Err(ServiceError::OperationDigestMismatch);
        }
        let (mut bytes, mut state) = self.load_authenticated(session_id, token)?;
        self.authorize_session_policy(&state, SecurityAction::TransactionCommit, now)?;
        let principal_id = state.principal_id.clone();
        self.validate_collection_vector_mutations(request)?;
        let record = state
            .transactions
            .get(transaction_id)
            .ok_or(ServiceError::TransactionNotFound)?;
        let transaction_scope = record.lease.scope.clone();
        let read = record.read.clone();
        if record
            .prepared
            .as_ref()
            .is_some_and(|prepared| prepared.operation_sha256 != request.operation_sha256)
        {
            return Err(ServiceError::IdempotencyConflict);
        }
        if record.lease.state == TransactionState::Committed {
            let intent = record
                .commit_intent
                .as_ref()
                .ok_or(ServiceError::TransactionClosed)?;
            require_same_commit(intent, idempotency_key, &request.operation_sha256)?;
            let mut stored = record
                .commit_receipt
                .clone()
                .ok_or(ServiceError::TransactionClosed)?;
            stored.idempotent_replay = true;
            return Ok(stored);
        }
        if record.lease.state != TransactionState::Open {
            return Err(ServiceError::TransactionClosed);
        }
        let mut prepared_runtime_functions = None;
        if let Some(intent) = &record.commit_intent {
            require_same_commit(intent, idempotency_key, &request.operation_sha256)?;
        } else {
            self.require_active_or_expire(
                session_id,
                bytes.clone(),
                &mut state,
                now,
                request_id,
                operation_id,
            )?;
            if now
                >= state
                    .transactions
                    .get(transaction_id)
                    .expect("transaction retained")
                    .lease
                    .expires_at_unix_ms
            {
                state
                    .transactions
                    .get_mut(transaction_id)
                    .expect("transaction retained")
                    .lease
                    .state = TransactionState::Expired;
                touch_session(&mut state, now);
                self.replace_session(
                    session_id,
                    bytes,
                    state,
                    now,
                    "transaction.expired",
                    request_id,
                    operation_id,
                )?;
                return Err(ServiceError::TransactionExpired);
            }
            let runtime_at = state
                .transactions
                .get(transaction_id)
                .expect("transaction retained")
                .prepared
                .as_ref()
                .map(|prepared| prepared.runtime_at_unix_ms)
                .unwrap_or(now);
            let runtime_identity = if matches!(transaction_scope.as_str(), "claims" | "data") {
                let catalogue = self.load_current_function_catalogue()?;
                self.authorize_transaction_function_bindings(
                    &state,
                    &catalogue,
                    now,
                    request_id,
                    operation_id,
                )?;
                let commit = public_runtime_commit(
                    request,
                    session_id,
                    &self.instance,
                    read.commit_cursor,
                    runtime_at,
                )?;
                let prepared = self.prepare_transaction_function_bindings(
                    &catalogue,
                    request,
                    transaction_id,
                    commit,
                    principal_id.clone(),
                    runtime_at,
                    request_id,
                    operation_id,
                )?;
                let identity = (
                    runtime_at,
                    prepared.commit.digest(),
                    catalogue.revision,
                    prepared.receipts.clone(),
                );
                prepared_runtime_functions = Some(prepared);
                identity
            } else {
                return Err(ServiceError::WrongScope);
            };
            state
                .transactions
                .get_mut(transaction_id)
                .expect("transaction retained")
                .commit_intent = Some(CommitIntent {
                idempotency_key: idempotency_key.clone(),
                operation_sha256: request.operation_sha256.clone(),
                runtime_at_unix_ms: runtime_identity.0,
                runtime_commit_sha256: runtime_identity.1,
                function_catalogue_revision: runtime_identity.2,
                function_receipts: runtime_identity.3,
            });
            bytes = self.replace_session(
                session_id,
                bytes,
                state.clone(),
                now,
                "transaction.commit_prepared",
                request_id,
                operation_id,
            )?;
        }
        // Diagnostic scope starts after authorized intent persistence. Neither
        // authentication nor the intent's own physical write can select it.
        #[cfg(feature = "crash-test")]
        let _domain_crash = rrd_store::crash_test::enter_domain(&request.operation_sha256);
        let accepted_receipt = if matches!(transaction_scope.as_str(), "claims" | "data") {
            let intent = state
                .transactions
                .get(transaction_id)
                .and_then(|record| record.commit_intent.as_ref())
                .ok_or(ServiceError::TransactionClosed)?;
            let runtime_at = intent.runtime_at_unix_ms;
            let expected_commit = &intent.runtime_commit_sha256;
            let prepared_receipts = intent.function_receipts.clone();
            let commit = match prepared_runtime_functions.take() {
                Some(prepared) => prepared.commit,
                None => {
                    let catalogue =
                        self.load_function_catalogue_revision(intent.function_catalogue_revision)?;
                    self.authorize_transaction_function_bindings(
                        &state,
                        &catalogue,
                        now,
                        request_id,
                        operation_id,
                    )?;
                    let commit = public_runtime_commit(
                        request,
                        session_id,
                        &self.instance,
                        read.commit_cursor,
                        runtime_at,
                    )?;
                    self.apply_prepared_transaction_function_receipts(
                        &catalogue,
                        request,
                        transaction_id,
                        commit,
                        &prepared_receipts,
                        runtime_at,
                    )?
                }
            };
            if commit.digest() != *expected_commit {
                return Err(ServiceError::OperationDigestMismatch);
            }
            let (outcome, idempotent_replay) = if let Some(outcome) =
                self.storage.runtime().commit_outcome(expected_commit)?
            {
                self.require_committed_transaction_function_receipts(&prepared_receipts)?;
                (outcome, true)
            } else {
                let transaction = DataTransaction::new(read.clone(), commit.clone())
                    .map_err(|error| ServiceError::Contract(error.to_string()))?;
                let receipt_records = prepared_receipts
                    .iter()
                    .map(super::function::encode_receipt_record)
                    .collect::<Result<Vec<_>>>()?;
                match self
                    .storage
                    .runtime()
                    .commit_data_transaction_with_function_receipts(
                        &transaction,
                        self.instance.as_str(),
                        &receipt_records,
                    ) {
                    Ok(outcome) => (outcome, false),
                    Err(error) => match self.storage.runtime().commit_outcome(expected_commit)? {
                        Some(outcome) => {
                            self.require_committed_transaction_function_receipts(
                                &prepared_receipts,
                            )?;
                            (outcome, true)
                        }
                        None => {
                            // These typed failures are raised before publication
                            // (or by snapshot conflict before the physical commit).
                            // An absent outcome plus that proof permits explicit
                            // abort. Unknown storage/I/O failures retain their
                            // intent for exact retry and crash reconciliation.
                            if matches!(
                                error,
                                rrd_store::Error::Kernel(_)
                                    | rrd_store::Error::TransactionLimit { .. }
                                    | rrd_store::Error::WriteBatchLimit { .. }
                                    | rrd_store::Error::MutableLimit { .. }
                                    | rrd_store::Error::MaintenanceLimit { .. }
                                    | rrd_store::Error::DiskAdmission { .. }
                                    | rrd_store::Error::ReclamationPending
                                    | rrd_store::Error::ReclamationLimit { .. }
                                    | rrd_store::Error::RuntimeConflict { .. }
                                    | rrd_store::Error::TransactionConflict { .. }
                                    | rrd_store::Error::DanglingRuntimeReference(_)
                                    | rrd_store::Error::RuntimeTargetNotFound(_)
                                    | rrd_store::Error::RuntimeSchemaMissing(_)
                                    | rrd_store::Error::RuntimeSchemaConflict { .. }
                                    | rrd_store::Error::IndexConstraint(_)
                            ) {
                                state
                                    .transactions
                                    .get_mut(transaction_id)
                                    .expect("transaction retained")
                                    .commit_intent = None;
                                self.replace_session(
                                    session_id,
                                    bytes,
                                    state,
                                    now,
                                    "transaction.commit_rejected",
                                    request_id,
                                    operation_id,
                                )?;
                            }
                            return Err(match error {
                                rrd_store::Error::TransactionLimit { .. } => {
                                    ServiceError::StorageAdmission(error.to_string())
                                }
                                other => other.into(),
                            });
                        }
                    },
                }
            };
            runtime_receipt(
                transaction_id,
                &request.operation_sha256,
                &outcome,
                idempotent_replay,
            )
        } else {
            return Err(ServiceError::WrongScope);
        };
        let record = state
            .transactions
            .get_mut(transaction_id)
            .expect("transaction retained");
        record.lease.state = TransactionState::Committed;
        record.commit_receipt = Some(accepted_receipt.clone());
        touch_or_expire_after_commit(&mut state, now);
        self.replace_session(
            session_id,
            bytes,
            state,
            now,
            "transaction.committed",
            request_id,
            operation_id,
        )?;
        #[cfg(feature = "crash-test")]
        if let Some(root) = &self.storage_root {
            rrd_store::crash_test::boundary(
                rrd_store::crash_test::Point::ReceiptPersisted,
                root.diagnostic_path(),
            );
        }
        Ok(accepted_receipt)
    }

    pub fn abort_transaction(
        &self,
        session_id: &CorrelationId,
        token: &CorrelationId,
        transaction_id: &CorrelationId,
        request: &AbortTransaction,
        context: &RequestContext,
        now: u64,
    ) -> Result<TransactionLease> {
        context
            .validate(true)
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        let idempotency_key = context.idempotency_key.as_ref().ok_or_else(|| {
            ServiceError::Contract("transaction abort requires idempotency".into())
        })?;
        let _transaction_guard = self.admit_transaction(
            request,
            context.request_id.as_str(),
            context.operation_id.as_str(),
        )?;
        let (bytes, mut state) = self.authorize(
            session_id,
            token,
            SecurityAction::TransactionAbort,
            now,
            context.request_id.as_str(),
            context.operation_id.as_str(),
        )?;
        let transaction = state
            .transactions
            .get_mut(transaction_id)
            .ok_or(ServiceError::TransactionNotFound)?;
        let operation_sha256 = operation_digest(request)?;
        let action = match transaction.lease.state {
            TransactionState::Open if transaction.commit_intent.is_some() => {
                return Err(ServiceError::CommitInProgress);
            }
            TransactionState::Open if now >= transaction.lease.expires_at_unix_ms => {
                transaction.lease.state = TransactionState::Expired;
                "transaction.expired"
            }
            TransactionState::Open => {
                transaction.lease.state = TransactionState::Aborted;
                transaction.abort = Some(AbortRecord {
                    idempotency_key: idempotency_key.clone(),
                    operation_sha256,
                });
                "transaction.aborted"
            }
            TransactionState::Aborted => {
                let accepted = transaction
                    .abort
                    .as_ref()
                    .ok_or(ServiceError::TransactionClosed)?;
                if accepted.idempotency_key != *idempotency_key
                    || accepted.operation_sha256 != operation_sha256
                {
                    return Err(ServiceError::IdempotencyConflict);
                }
                return Ok(transaction.lease.clone());
            }
            TransactionState::Expired => return Err(ServiceError::TransactionExpired),
            TransactionState::Committed => return Err(ServiceError::TransactionClosed),
        };
        let transaction = transaction.lease.clone();
        touch_session(&mut state, now);
        self.replace_session(
            session_id,
            bytes,
            state,
            now,
            action,
            context.request_id.as_str(),
            context.operation_id.as_str(),
        )?;
        if transaction.state == TransactionState::Expired {
            return Err(ServiceError::TransactionExpired);
        }
        Ok(transaction)
    }

    pub fn preview_transaction(
        &self,
        session_id: &CorrelationId,
        token: &CorrelationId,
        transaction_id: &CorrelationId,
        request: &PreviewTransaction,
        context: &RequestContext,
        now: u64,
    ) -> Result<TransactionPreview> {
        context
            .validate(true)
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        let idempotency_key = context.idempotency_key.as_ref().ok_or_else(|| {
            ServiceError::Contract("transaction prepare requires idempotency".into())
        })?;
        request
            .validate()
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        let _transaction_guard = self.admit_transaction(
            request,
            context.request_id.as_str(),
            context.operation_id.as_str(),
        )?;
        let (bytes, mut state) = self.authorize(
            session_id,
            token,
            SecurityAction::TransactionPreview,
            now,
            context.request_id.as_str(),
            context.operation_id.as_str(),
        )?;
        let transaction = state
            .transactions
            .get(transaction_id)
            .ok_or(ServiceError::TransactionNotFound)?;
        if transaction.lease.state != TransactionState::Open {
            return Err(ServiceError::TransactionClosed);
        }
        if transaction.commit_intent.is_some() {
            return Err(ServiceError::CommitInProgress);
        }
        if now >= transaction.lease.expires_at_unix_ms {
            state
                .transactions
                .get_mut(transaction_id)
                .expect("transaction retained")
                .lease
                .state = TransactionState::Expired;
            self.replace_session(
                session_id,
                bytes,
                state,
                now,
                "transaction.expired",
                context.request_id.as_str(),
                context.operation_id.as_str(),
            )?;
            return Err(ServiceError::TransactionExpired);
        }
        if !matches!(transaction.lease.scope.as_str(), "claims" | "data") {
            return Err(ServiceError::WrongScope);
        }
        let operation_sha256 = transaction_operation_sha256(&request.mutations);
        let prepared = transaction.prepared.clone();
        if let Some(prepared) = &prepared {
            if prepared.idempotency_key != *idempotency_key
                || prepared.operation_sha256 != operation_sha256
            {
                return Err(ServiceError::IdempotencyConflict);
            }
        }
        let runtime_at = prepared
            .as_ref()
            .map(|prepared| prepared.runtime_at_unix_ms)
            .unwrap_or(now);
        let commit_request = CommitTransaction {
            operation_sha256: operation_sha256.clone(),
            mutations: request.mutations.clone(),
        };
        self.validate_collection_vector_mutations(&commit_request)?;
        let commit = public_runtime_commit(
            &commit_request,
            session_id,
            &self.instance,
            transaction.read.commit_cursor,
            runtime_at,
        )?;
        let data_transaction = DataTransaction::new(transaction.read.clone(), commit)
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        let key_budget = usize::try_from(request.max_storage_keys)
            .map_err(|_| ServiceError::Query("preview key budget exceeds usize".into()))?;
        let valid_at = request.valid_at.unwrap_or(now);
        let prospective = public_data_snapshot(self.storage.runtime().preview_data_snapshot(
            &data_transaction,
            valid_at,
            key_budget,
        )?)?;
        let preview = TransactionPreview {
            transaction_id: transaction_id.clone(),
            read_cursor: transaction.lease.read_cursor,
            operation_sha256: operation_sha256.clone(),
            mutations: request.mutations.clone(),
            prospective,
            idempotent_replay: prepared.is_some(),
        };
        if prepared.is_some() {
            return Ok(preview);
        }
        state
            .transactions
            .get_mut(transaction_id)
            .expect("transaction retained")
            .prepared = Some(PreparedRecord {
            idempotency_key: idempotency_key.clone(),
            operation_sha256,
            runtime_at_unix_ms: runtime_at,
        });
        touch_session(&mut state, now);
        self.replace_session(
            session_id,
            bytes,
            state,
            now,
            "transaction.prepared",
            context.request_id.as_str(),
            context.operation_id.as_str(),
        )?;
        Ok(preview)
    }
}

fn touch_or_expire_after_commit(state: &mut SessionState, now: u64) {
    if state.status == SessionStatus::Active
        && now < state.idle_expires_at_unix_ms
        && now < state.absolute_expires_at_unix_ms
    {
        touch_session(state, now);
        return;
    }
    state.status = SessionStatus::Expired;
    for transaction in state.transactions.values_mut() {
        if transaction.lease.state == TransactionState::Open && transaction.commit_intent.is_none()
        {
            transaction.lease.state = TransactionState::Expired;
        }
    }
}

fn require_same_commit(
    intent: &CommitIntent,
    idempotency_key: &CorrelationId,
    operation_sha256: &str,
) -> Result<()> {
    if intent.idempotency_key != *idempotency_key || intent.operation_sha256 != operation_sha256 {
        return Err(ServiceError::IdempotencyConflict);
    }
    Ok(())
}
