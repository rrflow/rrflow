//! Maintaining and reading cold history.

use super::*;

impl RrdEngine {
    /// Read engine-owned cold-history status or one authenticated generation.
    /// Restricted row/field grants are denied because whole change records
    /// cannot yet be filtered by this operation.
    #[allow(clippy::too_many_arguments)]
    pub fn read_cold_history(
        &self,
        session: &CorrelationId,
        token: &CorrelationId,
        request: &ReadColdHistory,
        now: u64,
        request_id: &str,
        operation_id: &str,
    ) -> Result<ColdHistoryReadResult> {
        request
            .validate()
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        self.enforce_query_configuration("cold_history.read", &request.budget)?;
        self.authorize_resource_without_data_policy(
            session,
            token,
            SecurityAction::ChangefeedRead,
            &self.instance_resource(),
            now,
            request_id,
            operation_id,
        )?;
        let scope = self.query_scope(&request.scope)?;
        let deadline = Instant::now()
            .checked_add(Duration::from_millis(request.budget.max_elapsed_ms))
            .ok_or(ServiceError::DeadlineExceeded)?;
        let key = format!("server/state/{}/cold-history/head", self.instance);
        let state = match self.storage.control().get(&key)? {
            Some(bytes) => serde_json::from_slice::<ColdState>(&bytes).map_err(contract_json)?,
            None => ColdState {
                version: COLD_STATE_VERSION,
                generation: 0,
                through_cursor: 0,
                last_receipt_sha256: None,
                pending: None,
                failure: None,
            },
        };
        if state.version != COLD_STATE_VERSION
            || (state.generation == 0) != state.last_receipt_sha256.is_none()
            || (state.generation == 0 && state.through_cursor != 0)
        {
            return Err(ServiceError::Storage("invalid cold history state".into()));
        }
        let mut result = ColdHistoryReadResult {
            scope: request.scope.clone(),
            enabled: self.estate_configuration.storage.cold_history.is_some(),
            current_generation: state.generation,
            through_cursor: state.through_cursor,
            pending: state.pending.is_some(),
            failure: state.failure.clone(),
            generation: None,
            changes: Vec::new(),
        };
        let Some(generation) = request.generation else {
            let mut output = CountOutput {
                limit: request.budget.max_output_bytes,
                used: 0,
            };
            serde_json::to_writer(&mut output, &result).map_err(|_| {
                ServiceError::ConfigurationLimit(
                    "cold history response exceeds output-byte budget".into(),
                )
            })?;
            check_deadline(deadline)?;
            return Ok(result);
        };
        if generation > state.generation {
            return Err(ServiceError::Changefeed(
                "cold history generation is not published".into(),
            ));
        }
        let chain_reads = state
            .generation
            .checked_sub(generation)
            .and_then(|value| value.checked_add(1))
            .ok_or_else(|| {
                ServiceError::Storage("cold history generation range overflow".into())
            })?;
        if chain_reads > request.budget.max_storage_keys {
            return Err(ServiceError::ConfigurationLimit(
                "cold history receipt chain exceeds storage-key budget".into(),
            ));
        }
        let mut expected_sha = state
            .last_receipt_sha256
            .clone()
            .ok_or_else(|| ServiceError::Storage("cold history head digest is missing".into()))?;
        let mut expected_through = state.through_cursor;
        let mut selected = None;
        for current in (generation..=state.generation).rev() {
            check_deadline(deadline)?;
            if !valid_sha256(&expected_sha) {
                return Err(ServiceError::Storage(
                    "cold history receipt digest is invalid".into(),
                ));
            }
            let key = format!(
                "server/state/{}/cold-history/generation/{current:020}",
                self.instance
            );
            let bytes =
                self.storage.control().get(&key)?.ok_or_else(|| {
                    ServiceError::Storage("cold history receipt is missing".into())
                })?;
            if digest::sha256_hex(&bytes) != expected_sha {
                return Err(ServiceError::Storage(
                    "cold history receipt digest mismatch".into(),
                ));
            }
            let receipt: ColdReceipt = serde_json::from_slice(&bytes).map_err(contract_json)?;
            receipt.pending.stamp.validate().map_err(core_contract)?;
            if receipt.version != COLD_STATE_VERSION
                || receipt.generation != current
                || receipt.pending.stamp.scope != scope
                || receipt.pending.through_cursor != expected_through
                || receipt.pending.requested_after >= receipt.pending.through_cursor
                || receipt.pending.through_cursor > receipt.pending.stamp.commit_cursor
                || !valid_sha256(&receipt.pending.source_sha256)
                || !valid_sha256(&receipt.pending.policy_sha256)
                || !valid_sha256(&receipt.pending.artifact_sha256)
                || receipt.pending.artifact_bytes == 0
                || receipt.pending.scan_limit == 0
                || receipt.pending.scan_limit > 65_536
                || receipt.pending.rows > receipt.pending.scan_limit
                || receipt.pending.row_groups > receipt.pending.rows
                || (current == 1) != receipt.previous_receipt_sha256.is_none()
            {
                return Err(ServiceError::Storage(
                    "cold history receipt is invalid".into(),
                ));
            }
            if current == generation {
                selected = Some(receipt);
                break;
            }
            expected_through = receipt.pending.requested_after;
            expected_sha = receipt.previous_receipt_sha256.ok_or_else(|| {
                ServiceError::Storage("cold history receipt chain is incomplete".into())
            })?;
        }
        let receipt = selected
            .ok_or_else(|| ServiceError::Storage("cold history generation is missing".into()))?;
        if receipt.pending.artifact_bytes > request.max_artifact_bytes
            || receipt.pending.rows > request.budget.max_rows
        {
            return Err(ServiceError::ConfigurationLimit(
                "cold history generation exceeds the requested artifact or row budget".into(),
            ));
        }
        let store = self.storage.as_rrflow_kv().ok_or_else(|| {
            ServiceError::Storage("cold history requires persistent rrflowKV".into())
        })?;
        let path = store
            .anchored_path()
            .join("cold-history")
            .join(format!("{}.parquet", receipt.pending.artifact_sha256));
        let file = path.open_read().map_err(io_error)?;
        if file.metadata().map_err(io_error)?.len() != receipt.pending.artifact_bytes {
            return Err(ServiceError::Storage(
                "cold history artifact length mismatch".into(),
            ));
        }
        let length = native(receipt.pending.artifact_bytes)?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(length.saturating_add(1))
            .map_err(|_| {
                ServiceError::ConfigurationLimit("cold history artifact allocation failed".into())
            })?;
        file.take(receipt.pending.artifact_bytes.saturating_add(1))
            .read_to_end(&mut bytes)
            .map_err(io_error)?;
        if bytes.len() != length {
            return Err(ServiceError::Storage(
                "cold history artifact changed while reading".into(),
            ));
        }
        let expectation = ColdHistoryReadExpectation {
            stamp: receipt.pending.stamp.clone(),
            requested_after: receipt.pending.requested_after,
            through_cursor: receipt.pending.through_cursor,
            rows: receipt.pending.rows,
            artifact_sha256: receipt.pending.artifact_sha256.clone(),
        };
        let decoded = decode_cold_history_page(
            bytes,
            &expectation,
            ColdHistoryReadLimits {
                max_rows: native(request.budget.max_rows)?,
                max_encoded_bytes: native(request.max_artifact_bytes)?,
                max_decoded_bytes: native(request.budget.max_memory_bytes / 32)?,
                max_memory_bytes: native(request.budget.max_memory_bytes)?,
                max_batch_rows: native(request.budget.max_batch_rows)?,
                deadline,
            },
        )
        .map_err(ServiceError::from)?;
        if decoded.row_groups as u64 != receipt.pending.row_groups {
            return Err(ServiceError::Storage(
                "cold history row-group count differs from its receipt".into(),
            ));
        }
        if source_digest_parts(
            &receipt.pending.stamp,
            receipt.pending.requested_after,
            receipt.pending.through_cursor,
            receipt.pending.stamp.commit_cursor,
            &decoded.changes,
        ) != receipt.pending.source_sha256
        {
            return Err(ServiceError::Storage(
                "cold history decoded changes differ from the prepared source".into(),
            ));
        }
        result.changes = decoded
            .changes
            .iter()
            .map(public_runtime_change)
            .collect::<Result<Vec<_>>>()?;
        result.generation = Some(ColdHistoryGenerationSnapshot {
            generation,
            requested_after_cursor: receipt.pending.requested_after,
            through_cursor: receipt.pending.through_cursor,
            source_head_cursor: receipt.pending.stamp.commit_cursor,
            source_manifest_sha256: receipt.pending.stamp.manifest_id,
            policy_sha256: receipt.pending.policy_sha256,
            artifact_sha256: receipt.pending.artifact_sha256,
            artifact_bytes: receipt.pending.artifact_bytes,
            rows: receipt.pending.rows,
            row_groups: receipt.pending.row_groups,
            observed_arrow_bytes: decoded.observed_arrow_bytes as u64,
            declared_uncompressed_bytes: decoded.declared_uncompressed_bytes,
        });
        let mut output = CountOutput {
            limit: request.budget.max_output_bytes,
            used: 0,
        };
        serde_json::to_writer(&mut output, &result).map_err(|_| {
            ServiceError::ConfigurationLimit(
                "cold history response exceeds output-byte budget".into(),
            )
        })?;
        check_deadline(deadline)?;
        Ok(result)
    }

    /// Offer one configured cold-history quantum. The server coalesces offers;
    /// embedded callers own their own scheduling.
    pub fn maintain_cold_history(&self) -> Result<bool> {
        let Some(policy) = self.estate_configuration.storage.cold_history.as_ref() else {
            return Ok(false);
        };
        let _guard = match self.cold_history_gate.try_lock() {
            Ok(guard) => guard,
            Err(TryLockError::WouldBlock) => return Ok(false),
            Err(TryLockError::Poisoned(_)) => {
                return Err(ServiceError::Storage(
                    "cold history maintenance lock poisoned".into(),
                ))
            }
        };
        let now =
            observe_required(&HostSystemClock, "cold_history.maintenance")?.observed_at_unix_ms;
        match self.maintain_cold_history_at(policy, now) {
            Ok(progress) => Ok(progress),
            Err(error) => {
                if self.record_cold_history_failure(&error, now).is_err() {
                    tracing::warn!(target: "rrflow::parquet_compactor", "could not persist cold history failure status");
                }
                Err(error)
            }
        }
    }

    pub(super) fn maintain_cold_history_at(
        &self,
        policy: &ColdHistoryPolicy,
        now: u64,
    ) -> Result<bool> {
        policy
            .validate()
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        let Some(store) = self.storage.as_rrflow_kv() else {
            return Ok(false);
        };
        let deadline = Instant::now()
            .checked_add(Duration::from_millis(policy.max_elapsed_ms))
            .ok_or(ServiceError::DeadlineExceeded)?;
        let policy_sha256 = digest::sha256_hex(&serde_json::to_vec(policy).map_err(contract_json)?);
        let key = format!("server/state/{}/cold-history/head", self.instance);
        let previous = self.storage.control().get(&key)?;
        let mut state = match previous.as_deref() {
            Some(bytes) => serde_json::from_slice::<ColdState>(bytes).map_err(contract_json)?,
            None => ColdState {
                version: COLD_STATE_VERSION,
                generation: 0,
                through_cursor: 0,
                last_receipt_sha256: None,
                pending: None,
                failure: None,
            },
        };
        if state.version != COLD_STATE_VERSION
            || (state.generation == 0) != state.last_receipt_sha256.is_none()
            || (state.generation == 0 && state.through_cursor != 0)
        {
            return Err(ServiceError::Storage("invalid cold history state".into()));
        }
        if state
            .last_receipt_sha256
            .as_ref()
            .is_some_and(|hash| !valid_sha256(hash))
        {
            return Err(ServiceError::Storage(
                "invalid cold history previous receipt digest".into(),
            ));
        }
        if state.generation > 0 && state.pending.is_some() {
            verify_head(self, store.anchored_path(), &state, deadline)?;
        }
        if let Some(pending) = &state.pending {
            let expected_scope =
                ScopeId::new(format!("instance:{}", self.instance)).map_err(core_contract)?;
            pending.stamp.validate().map_err(core_contract)?;
            if pending.stamp.scope != expected_scope
                || pending.requested_after != state.through_cursor
                || pending.through_cursor <= pending.requested_after
                || pending.through_cursor > pending.stamp.commit_cursor
                || pending.scan_limit == 0
                || pending.scan_limit > 65_536
                || pending.artifact_bytes == 0
                || pending.artifact_bytes > 1_073_741_824
                || pending.rows > pending.scan_limit
                || pending.row_groups > pending.rows
                || !valid_sha256(&pending.source_sha256)
                || !valid_sha256(&pending.policy_sha256)
                || !valid_sha256(&pending.artifact_sha256)
            {
                return Err(ServiceError::Storage(
                    "invalid pending cold history coordinates".into(),
                ));
            }
        }
        if state
            .pending
            .as_ref()
            .is_some_and(|pending| pending.policy_sha256 != policy_sha256)
        {
            // A newly authorized policy may change after a pending quantum.
            // Preserve the committed cursor and replan rather than reinterpret
            // old work under different limits or compression settings.
            state.pending = None;
            state.failure = None;
            self.commit_cold_control(transition(
                &key,
                previous,
                &state,
                now,
                "cold_history.replanned",
            )?)?;
            return Ok(true);
        }

        if let Some(pending) = state.pending.clone() {
            if pending.scan_limit > policy.max_changes
                || pending.artifact_bytes > policy.max_output_bytes
            {
                return Err(ServiceError::Storage(
                    "pending cold history exceeds its admitted policy".into(),
                ));
            }
            let page = match self.storage.runtime().read_changes_bounded_bytes(
                &pending.stamp,
                pending.requested_after,
                native(pending.scan_limit)?,
                policy.max_input_bytes,
                deadline,
            ) {
                Ok(page) => page,
                Err(
                    rrd_store::Error::ReadStampMismatch(_)
                    | rrd_store::Error::ReadStampUnavailable(_),
                ) => {
                    // No archive has been committed for this interval. Keep
                    // the previous head and plan again from the new stamp.
                    state.pending = None;
                    state.failure = None;
                    self.commit_cold_control(transition(
                        &key,
                        previous,
                        &state,
                        now,
                        "cold_history.source_replanned",
                    )?)?;
                    return Ok(true);
                }
                Err(error) => return Err(cold_source_error(error)),
            };
            if page.through_cursor != pending.through_cursor
                || source_digest(&pending.stamp, &page) != pending.source_sha256
            {
                return Err(ServiceError::StorageConflict(
                    "cold history source changed; prepared work cannot be published".into(),
                ));
            }
            let artifact = encode(&pending.stamp, &page, policy, deadline)?;
            if digest::sha256_hex(&artifact.artifact.bytes) != pending.artifact_sha256
                || artifact.artifact.bytes.len() as u64 != pending.artifact_bytes
                || page.changes.len() as u64 != pending.rows
                || artifact.artifact.row_groups as u64 != pending.row_groups
            {
                return Err(ServiceError::Storage(
                    "cold history replay does not match prepared artifact".into(),
                ));
            }
            let directory = store.anchored_path().join("cold-history");
            directory.create_dir_all().map_err(io_error)?;
            let final_path = directory.join(format!("{}.parquet", pending.artifact_sha256));
            if !final_path.try_exists().map_err(io_error)?
                && !store.try_admit_immutable_file(pending.artifact_bytes)?
            {
                return Ok(false);
            }
            publish_file(
                &directory,
                &final_path,
                &pending.artifact_sha256,
                &artifact.artifact.bytes,
                deadline,
            )?;

            let generation = state
                .generation
                .checked_add(1)
                .ok_or_else(|| ServiceError::Storage("cold history generation overflow".into()))?;
            let receipt = ColdReceipt {
                version: COLD_STATE_VERSION,
                generation,
                previous_receipt_sha256: state.last_receipt_sha256.clone(),
                pending,
            };
            let receipt_bytes = serde_json::to_vec(&receipt).map_err(contract_json)?;
            let receipt_sha256 = digest::sha256_hex(&receipt_bytes);
            let receipt_key = format!(
                "server/state/{}/cold-history/generation/{generation:020}",
                self.instance
            );
            state.generation = generation;
            state.through_cursor = receipt.pending.through_cursor;
            state.last_receipt_sha256 = Some(receipt_sha256);
            state.pending = None;
            state.failure = None;
            check_deadline(deadline)?;
            self.commit_cold_control_batch(&[
                transition_bytes(
                    &receipt_key,
                    None,
                    receipt_bytes,
                    now,
                    "cold_history.generation",
                ),
                transition(&key, previous, &state, now, "cold_history.published")?,
            ])?;
            tracing::info!(target: "rrflow::parquet_compactor", generation, rows = receipt.pending.rows, bytes = receipt.pending.artifact_bytes, "cold history generation published");
            return Ok(true);
        }

        let scope = ScopeId::new(format!("instance:{}", self.instance)).map_err(core_contract)?;
        let stamp = self.storage.runtime().read_stamp(&scope)?;
        if state.through_cursor > stamp.commit_cursor {
            return Err(ServiceError::Storage(
                "cold history cursor exceeds the source head".into(),
            ));
        }
        if state.through_cursor == stamp.commit_cursor {
            return Ok(false);
        }
        if state.generation > 0 {
            verify_head(self, store.anchored_path(), &state, deadline)?;
        }
        let mut scan_limit = policy.max_changes;
        let (page, artifact) = loop {
            check_deadline(deadline)?;
            let page = self
                .storage
                .runtime()
                .read_changes_bounded_bytes(
                    &stamp,
                    state.through_cursor,
                    native(scan_limit)?,
                    policy.max_input_bytes,
                    deadline,
                )
                .map_err(cold_source_error)?;
            if page.through_cursor <= state.through_cursor {
                return Err(ServiceError::Storage(
                    "cold history source did not advance".into(),
                ));
            }
            match encode_cold_history_page(
                &stamp,
                &page,
                limits(policy, deadline)?,
                encoding(policy),
            ) {
                Ok(artifact) => break (page, artifact),
                Err(rrd_query::Error::Budget(_)) if scan_limit > 1 => {
                    scan_limit = (scan_limit / 2).max(1)
                }
                Err(error) => return Err(cold_query_error(error, deadline)),
            }
        };
        let source_sha256 = source_digest(&stamp, &page);
        let pending = ColdPending {
            stamp,
            requested_after: page.requested_after,
            through_cursor: page.through_cursor,
            scan_limit,
            source_sha256,
            policy_sha256,
            artifact_sha256: digest::sha256_hex(&artifact.artifact.bytes),
            artifact_bytes: artifact.artifact.bytes.len() as u64,
            rows: page.changes.len() as u64,
            row_groups: artifact.artifact.row_groups as u64,
        };
        state.pending = Some(pending);
        state.failure = None;
        check_deadline(deadline)?;
        self.commit_cold_control(transition(
            &key,
            previous,
            &state,
            now,
            "cold_history.prepared",
        )?)?;
        Ok(true)
    }

    /// Publication runs beside foreground requests and appends to the same
    /// control journal, so it waits at the engine's transaction gate like
    /// every other writer. Two writers that skip it meet as a physical write
    /// conflict, which clients see as a runtime conflict at an unchanged
    /// cursor. The gate is held for the commit alone, never for encoding.
    fn commit_cold_control(
        &self,
        transition: rrd_store::ControlTransition,
    ) -> Result<rrd_store::ControlJournalEntry> {
        let _gate = self.admit_transaction(&transition, COLD_REQUEST_ID, COLD_OPERATION_ID)?;
        Ok(self.storage.control().commit(&transition)?)
    }

    fn commit_cold_control_batch(
        &self,
        transitions: &[rrd_store::ControlTransition],
    ) -> Result<Vec<rrd_store::ControlJournalEntry>> {
        let _gate = self.admit_transaction(transitions, COLD_REQUEST_ID, COLD_OPERATION_ID)?;
        Ok(self.storage.control().commit_batch(transitions)?)
    }

    fn record_cold_history_failure(&self, error: &ServiceError, now: u64) -> Result<()> {
        let code = match error.kind() {
            ServiceErrorKind::DeadlineExceeded => ColdHistoryFailureCode::DeadlineExceeded,
            ServiceErrorKind::ResourceExhausted => ColdHistoryFailureCode::ResourceLimit,
            ServiceErrorKind::Conflict => ColdHistoryFailureCode::SourceConflict,
            _ => ColdHistoryFailureCode::StorageOrOther,
        };
        let key = format!("server/state/{}/cold-history/head", self.instance);
        let previous = self.storage.control().get(&key)?;
        let mut state = match previous.as_deref() {
            Some(bytes) => serde_json::from_slice::<ColdState>(bytes).map_err(contract_json)?,
            None => ColdState {
                version: COLD_STATE_VERSION,
                generation: 0,
                through_cursor: 0,
                last_receipt_sha256: None,
                pending: None,
                failure: None,
            },
        };
        if state.version != COLD_STATE_VERSION
            || (state.generation == 0) != state.last_receipt_sha256.is_none()
            || (state.generation == 0 && state.through_cursor != 0)
        {
            return Err(ServiceError::Storage("invalid cold history state".into()));
        }
        if state
            .failure
            .as_ref()
            .is_some_and(|failure| failure.code == code)
        {
            return Ok(());
        }
        state.failure = Some(ColdHistoryFailure {
            code,
            since_unix_ms: now,
        });
        self.commit_cold_control(transition(
            &key,
            previous,
            &state,
            now,
            "cold_history.failure",
        )?)?;
        Ok(())
    }
}
