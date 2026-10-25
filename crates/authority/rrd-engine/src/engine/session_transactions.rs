//! Bounded session working state and durable terminal transaction receipts.
//! Both physical placements use the same control transaction and journal.

use super::*;

const TERMINAL_TRANSACTION_FORMAT: u16 = 1;

pub(super) const fn terminal_transaction_format() -> u16 {
    TERMINAL_TRANSACTION_FORMAT
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TerminalTransaction {
    format_version: u16,
    record: TransactionRecord,
}

pub(super) fn terminal_transaction_key(
    instance: &CanonicalId,
    session: &CorrelationId,
    transaction: &CorrelationId,
) -> String {
    let identity =
        serde_json::to_vec(&(session, transaction)).expect("transaction identities serialize");
    format!(
        "server/state/{instance}/terminal-transaction/{}",
        digest::sha256_hex(&identity)
    )
}

impl RrdEngine {
    pub(super) fn transaction_record(
        &self,
        state: &SessionState,
        transaction_id: &CorrelationId,
    ) -> Result<Option<TransactionRecord>> {
        if let Some(record) = state.transactions.get(transaction_id) {
            return Ok(Some(record.clone()));
        }
        let key = terminal_transaction_key(&self.instance, &state.session_id, transaction_id);
        let Some(bytes) = self.storage.control().get(&key)? else {
            return Ok(None);
        };
        let terminal: TerminalTransaction =
            serde_json::from_slice(&bytes).map_err(contract_json)?;
        if terminal.format_version != TERMINAL_TRANSACTION_FORMAT
            || terminal.record.lease.session_id != state.session_id
            || terminal.record.lease.transaction_id != *transaction_id
            || terminal.record.lease.state == TransactionState::Open
        {
            return Err(ServiceError::Contract(
                "invalid terminal transaction record".into(),
            ));
        }
        Ok(Some(terminal.record))
    }

    #[allow(clippy::too_many_arguments)]
    pub(in crate::engine) fn replace_session(
        &self,
        session_id: &CorrelationId,
        expected: Vec<u8>,
        state: &mut SessionState,
        now: u64,
        action: &str,
        request_id: &str,
        operation_id: &str,
    ) -> Result<Vec<u8>> {
        let mut next = state.clone();
        // One bounded quantum also handles bulk close/expiry: never turn a
        // session transition into an unbounded fan-out or prune replay history.
        // Open records (including uncertain commit intents) are never moved.
        let terminal_id = next
            .transactions
            .iter()
            .find(|(_, record)| record.lease.state != TransactionState::Open)
            .map(|(id, _)| id.clone());
        let transition = |key, expected, replacement, action: &str| ControlTransition {
            key,
            expected,
            replacement: Some(replacement),
            at: now,
            actor: "rrd-engine".into(),
            action: action.into(),
            request_id: request_id.into(),
            operation_id: operation_id.into(),
        };
        let mut transitions = Vec::with_capacity(2);
        if let Some(id) = terminal_id {
            let record = next
                .transactions
                .remove(&id)
                .expect("selected terminal exists");
            let terminal = TerminalTransaction {
                format_version: TERMINAL_TRANSACTION_FORMAT,
                record,
            };
            transitions.push(transition(
                terminal_transaction_key(&self.instance, session_id, &id),
                None,
                serde_json::to_vec(&terminal).map_err(contract_json)?,
                "transaction.retained",
            ));
        }
        let replacement = serde_json::to_vec(&next).map_err(contract_json)?;
        transitions.push(transition(
            session_key(&self.instance, session_id),
            Some(expected),
            replacement.clone(),
            action,
        ));
        // The session CAS fences both placements. Failure cannot publish only
        // a receipt or remove only its working-state copy. Update the caller
        // after success so multi-stage commit never reintroduces a moved record.
        self.storage.control().commit_batch(&transitions)?;
        *state = next;
        Ok(replacement)
    }
}
