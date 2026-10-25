//! Bounded reuse of proof material inside one physical read transaction.
//!
//! This never authenticates a change: the caller still verifies every proof
//! against its captured root. Do not retain this cache outside that read.

const SLOTS: usize = 128;
const DIGEST_BYTES: usize = 64;

struct Node {
    level: u8,
    index: u64,
    digest: [u8; DIGEST_BYTES],
}

pub(super) struct ProofNodes {
    slots: [Option<Node>; SLOTS],
}

impl ProofNodes {
    pub(super) fn new() -> Self {
        Self {
            slots: std::array::from_fn(|_| None),
        }
    }

    pub(super) fn read(
        &mut self,
        level: u8,
        index: u64,
        load: impl FnOnce() -> rrd_core::Result<Option<String>>,
    ) -> rrd_core::Result<Option<String>> {
        // Collisions replace a slot; they cannot alias two node identities.
        // The odd multipliers spread adjacent indices and tree levels.
        let slot = index
            .wrapping_mul(0x9e37)
            .wrapping_add(u64::from(level).wrapping_mul(0x85eb)) as usize
            % SLOTS;
        if let Some(node) = &self.slots[slot] {
            if node.level == level && node.index == index {
                return Ok(Some(String::from_utf8_lossy(&node.digest).into_owned()));
            }
        }
        let value = load()?;
        if let Some(digest) = &value {
            // Never retain unbounded or malformed backing values. Core proof
            // validation remains responsible for rejecting these values.
            if digest.len() == DIGEST_BYTES && digest.bytes().all(|b| b.is_ascii_hexdigit()) {
                let mut bytes = [0; DIGEST_BYTES];
                bytes.copy_from_slice(digest.as_bytes());
                self.slots[slot] = Some(Node {
                    level,
                    index,
                    digest: bytes,
                });
            }
        }
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn proof_nodes_reuse_exact_identities_and_bound_collision_storage() {
        let mut nodes = ProofNodes::new();
        let loads = Cell::new(0);
        let read = |nodes: &mut ProofNodes, level, index| {
            nodes
                .read(level, index, || {
                    loads.set(loads.get() + 1);
                    Ok(Some(format!("{index:064x}")))
                })
                .unwrap()
        };
        assert_eq!(read(&mut nodes, 1, 7), read(&mut nodes, 1, 7));
        assert_eq!(loads.get(), 1);
        read(&mut nodes, 2, 7);
        assert_eq!(loads.get(), 2);
        // Every index congruent modulo SLOTS collides, but must load its own value.
        for n in 0..10_000u64 {
            assert_eq!(
                read(&mut nodes, 1, 7 + n * SLOTS as u64),
                Some(format!("{:064x}", 7 + n * SLOTS as u64))
            );
        }
        assert_eq!(read(&mut nodes, 1, 7), Some(format!("{:064x}", 7)));
        assert!(nodes.slots.iter().flatten().count() <= SLOTS);
        assert!(std::mem::size_of::<ProofNodes>() <= 16 * 1024);
        let mut another = ProofNodes::new();
        assert_eq!(
            another.read(1, 7, || Ok(Some("ab".repeat(32)))).unwrap(),
            Some("ab".repeat(32))
        );
    }

    #[test]
    fn proof_nodes_do_not_cache_absence_errors_or_malformed_values() {
        let mut nodes = ProofNodes::new();
        assert_eq!(nodes.read(1, 1, || Ok(None)).unwrap(), None);
        assert!(nodes
            .read(1, 1, || Err(rrd_core::Error::InvalidRuntime {
                reason: "backing error".into()
            }))
            .is_err());
        for malformed in ["x".repeat(64), "a".repeat(65), String::new()] {
            assert_eq!(
                nodes.read(1, 1, || Ok(Some(malformed.clone()))).unwrap(),
                Some(malformed)
            );
            assert!(nodes.slots.iter().all(Option::is_none));
        }
        assert_eq!(
            nodes.read(1, 1, || Ok(Some("ab".repeat(32)))).unwrap(),
            Some("ab".repeat(32))
        );
    }

    #[test]
    fn proof_nodes_never_survive_into_a_new_transaction_or_hide_corruption() {
        use crate::{
            Durability, RrflowKvStore, RrflowMxStore, RuntimeReadBudget, RuntimeVersionedSource,
            StorageEngine,
        };
        use rrd_core::{
            RuntimeCommit, RuntimeLogicalModel, RuntimeMutation, RuntimeRecord, RuntimeRef,
            RuntimeSchemaRegistry, RuntimeType, ScopeId,
        };
        fn check(engine: &dyn StorageEngine) {
            let scope = ScopeId::new("instance:proof-corruption").unwrap();
            let mut schema = RuntimeSchemaRegistry::empty(1, "proof corruption");
            schema
                .define_record_table(
                    RuntimeType::new("document").unwrap(),
                    RuntimeLogicalModel::Relational,
                    Default::default(),
                )
                .unwrap();
            engine
                .runtime()
                .commit(&RuntimeCommit {
                    scope: scope.clone(),
                    at: 100,
                    actor: "test:proof-corruption".into(),
                    expected_cursor: 0,
                    mutations: vec![
                        RuntimeMutation::Schema { registry: schema },
                        RuntimeMutation::Record {
                            record: RuntimeRecord {
                                reference: RuntimeRef::new("document", "a").unwrap(),
                                valid_from: 100,
                                valid_to: None,
                                properties: Default::default(),
                            },
                        },
                    ],
                })
                .unwrap();
            let read = engine.runtime().read_stamp(&scope).unwrap();
            let sources = [RuntimeVersionedSource::Records {
                kind: Some(RuntimeType::new("document").unwrap()),
            }];
            let first = engine
                .runtime()
                .read_versioned(&read, &sources, RuntimeReadBudget::new(100).unwrap())
                .unwrap();
            assert_eq!(first.changes.len(), 1);
            // Corrupt the schema leaf's sibling material. The stamped root,
            // semantic record and native byte framing remain intact.
            let mut transaction = engine.begin_transaction().unwrap();
            transaction
                .put(
                    crate::keyspaces::runtime_accumulator_node_key(0, 0),
                    "00".repeat(32).into_bytes(),
                )
                .unwrap();
            transaction.commit(Durability::Authoritative).unwrap();
            let error = engine
                .runtime()
                .read_versioned(&read, &sources, RuntimeReadBudget::new(100).unwrap())
                .unwrap_err();
            assert!(
                error
                    .to_string()
                    .contains("does not match its accumulator root"),
                "{error}"
            );
        }
        check(&RrflowMxStore::new());
        let directory = tempfile::tempdir().unwrap();
        check(&RrflowKvStore::open(&directory.path().join("proof-corruption")).unwrap());
    }
}
