//! Engine-owned physical upkeep; logical cursors and external effects are unchanged.
use super::{Result, RrdEngine};

impl RrdEngine {
    /// Offer one housekeeping quantum to the installed native storage owner.
    /// Lock admission never queues; admitted work uses the configured native
    /// byte/count/time limits. Volatile estates have no physical maintenance.
    pub fn maintain_storage(&self) -> Result<bool> {
        self.storage.as_rrflow_kv().map_or(Ok(false), |store| {
            store.try_maintenance_step().map_err(Into::into)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rrd_store::{Durability, StorageEngine};

    #[test]
    fn idle_maintenance_engine_preserves_logical_cursors_and_mutable_acknowledgements() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("engine");
        let engine = RrdEngine::open(
            &path,
            rrd_contract::CanonicalId::new("idle").unwrap(),
            [0; 32],
        )
        .unwrap();
        let store = engine.storage.as_rrflow_kv().unwrap();
        for ordinal in 0..10 {
            let mut tx = store.begin_transaction().unwrap();
            tx.put(format!("idle-test-{ordinal}").into_bytes(), b"ack".to_vec())
                .unwrap();
            tx.commit(Durability::Authoritative).unwrap();
            store.flush(ordinal + 1).unwrap();
        }
        let mut tx = store.begin_transaction().unwrap();
        tx.put(b"idle-tail".to_vec(), b"ack".to_vec()).unwrap();
        tx.commit(Durability::Authoritative).unwrap();
        let cursors = || {
            (
                store.claims().sequence().unwrap(),
                store.runtime().cursor().unwrap(),
                store.control().sequence().unwrap(),
            )
        };
        let before = cursors();
        assert!(
            store
                .physical_store_evidence()
                .unwrap()
                .compaction_debt_segments
                .unwrap()
                > 0
        );
        for _ in 0..30 {
            if !engine.maintain_storage().unwrap() {
                break;
            }
        }
        assert_eq!(
            store
                .physical_store_evidence()
                .unwrap()
                .compaction_debt_segments,
            Some(0)
        );
        assert_eq!(before, cursors());
        drop(engine);
        let reopened = RrdEngine::open(
            &path,
            rrd_contract::CanonicalId::new("idle").unwrap(),
            [0; 32],
        )
        .unwrap();
        let tx = reopened.storage.begin_transaction().unwrap();
        assert_eq!(tx.get(b"idle-tail").unwrap(), Some(b"ack".to_vec()));
    }
}
