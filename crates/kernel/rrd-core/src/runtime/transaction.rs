//! Data transactions and their read views.

use super::*;

/// A runtime commit bound to the exact state from which it was constructed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DataTransaction {
    pub contract_version: u16,
    pub read: ReadStamp,
    pub commit: RuntimeCommit,
}

impl DataTransaction {
    pub fn new(read: ReadStamp, commit: RuntimeCommit) -> Result<Self> {
        let transaction = Self {
            contract_version: DATA_RUNTIME_CONTRACT_VERSION,
            read,
            commit,
        };
        transaction.validate()?;
        Ok(transaction)
    }

    pub fn validate(&self) -> Result<()> {
        if self.contract_version != DATA_RUNTIME_CONTRACT_VERSION {
            return Err(Error::InvalidRuntime {
                reason: format!(
                    "unsupported transaction contract version {}",
                    self.contract_version
                ),
            });
        }
        self.read.validate()?;
        self.commit.validate()?;
        if self.read.scope != self.commit.scope {
            return Err(Error::InvalidRuntime {
                reason: "transaction read scope differs from commit scope".into(),
            });
        }
        if self.read.commit_cursor != self.commit.expected_cursor {
            return Err(Error::InvalidRuntime {
                reason: "transaction expected cursor differs from its read stamp".into(),
            });
        }
        Ok(())
    }

    pub fn digest(&self) -> String {
        let mut bytes = b"rrflow-data-transaction-v1\0".to_vec();
        bytes.extend_from_slice(&self.contract_version.to_be_bytes());
        bytes.extend_from_slice(&self.read.canonical_bytes());
        let commit = self.commit.canonical_bytes();
        bytes.extend_from_slice(&(commit.len() as u64).to_be_bytes());
        bytes.extend_from_slice(&commit);
        digest::sha256_hex(&bytes)
    }

    /// Applies pending structural mutations over the exact graph state named
    /// by this transaction's read stamp. The returned view is explicitly
    /// prospective: it is never accepted as committed evidence.
    pub fn preview(&self, base: &RuntimeGraphSnapshot) -> Result<DataTransactionView> {
        self.validate()?;
        if base.scope != self.read.scope {
            return Err(Error::InvalidRuntime {
                reason: "transaction preview scope differs from its read stamp".into(),
            });
        }
        if base.known_at_cursor != self.read.commit_cursor {
            return Err(Error::InvalidRuntime {
                reason: "transaction preview cursor differs from its read stamp".into(),
            });
        }

        let prospective_cursor = self
            .read
            .commit_cursor
            .checked_add(self.commit.mutations.len() as u64)
            .ok_or_else(|| Error::InvalidRuntime {
                reason: "transaction preview cursor overflowed".into(),
            })?;
        let mut records = base
            .records
            .iter()
            .cloned()
            .map(|record| (record.reference.clone(), record))
            .collect::<BTreeMap<_, _>>();
        let mut relations = base
            .relations
            .iter()
            .cloned()
            .map(|relation| (relation.reference.clone(), relation))
            .collect::<BTreeMap<_, _>>();
        for (ordinal, mutation) in self.commit.mutations.iter().enumerate() {
            match mutation {
                RuntimeMutation::Record { record } if record.valid_from <= base.valid_at => {
                    records.insert(record.reference.clone(), record.clone());
                }
                RuntimeMutation::Relation { relation } if relation.valid_from <= base.valid_at => {
                    relations.insert(relation.reference.clone(), relation.clone());
                }
                RuntimeMutation::Event { event } => {
                    let cursor = self.read.commit_cursor + ordinal as u64 + 1;
                    let event_ref = RuntimeRef::event(event.kind.clone(), cursor);
                    records.insert(
                        event_ref.clone(),
                        RuntimeRecord {
                            reference: event_ref.clone(),
                            valid_from: self.commit.at,
                            valid_to: None,
                            properties: event.properties.clone(),
                        },
                    );
                    if let Some(subject) = &event.subject {
                        let relation_ref = RuntimeRef::event(
                            RuntimeType::new("emitted").expect("static runtime type is valid"),
                            cursor,
                        );
                        relations.insert(
                            relation_ref.clone(),
                            RuntimeRelation {
                                reference: relation_ref,
                                from: subject.clone(),
                                to: event_ref,
                                valid_from: self.commit.at,
                                valid_to: None,
                                properties: RuntimeProperties::new(),
                            },
                        );
                    }
                }
                RuntimeMutation::Retire { retirement }
                    if retirement.effective_at <= base.valid_at =>
                {
                    retire_graph_identity(&mut records, &mut relations, retirement);
                }
                RuntimeMutation::Claim { .. }
                | RuntimeMutation::Schema { .. }
                | RuntimeMutation::Record { .. }
                | RuntimeMutation::Relation { .. }
                | RuntimeMutation::Vector { .. }
                | RuntimeMutation::SeriesSample { .. }
                | RuntimeMutation::Geo { .. }
                | RuntimeMutation::Object { .. }
                | RuntimeMutation::Retire { .. } => {}
            }
        }
        let records = records
            .into_values()
            .filter(|record| valid_at_window(record.valid_from, record.valid_to, base.valid_at))
            .collect();
        let relations = relations
            .into_values()
            .filter(|relation| {
                valid_at_window(relation.valid_from, relation.valid_to, base.valid_at)
            })
            .collect();
        let view = DataTransactionView {
            contract_version: DATA_RUNTIME_CONTRACT_VERSION,
            transaction_digest: self.digest(),
            read: self.read.clone(),
            valid_at: base.valid_at,
            prospective_cursor,
            pending_mutations: self.commit.mutations.len(),
            pending: self.commit.mutations.clone(),
            records,
            relations,
        };
        view.validate()?;
        Ok(view)
    }
}

/// Read-your-writes graph view for one uncommitted data transaction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DataTransactionView {
    pub contract_version: u16,
    pub transaction_digest: String,
    pub read: ReadStamp,
    pub valid_at: Millis,
    pub prospective_cursor: u64,
    pub pending_mutations: usize,
    pub pending: Vec<RuntimeMutation>,
    pub records: Vec<RuntimeRecord>,
    pub relations: Vec<RuntimeRelation>,
}

impl DataTransactionView {
    pub fn validate(&self) -> Result<()> {
        if self.contract_version != DATA_RUNTIME_CONTRACT_VERSION {
            return Err(Error::InvalidRuntime {
                reason: format!(
                    "unsupported transaction-view contract version {}",
                    self.contract_version
                ),
            });
        }
        self.read.validate()?;
        validate_digest("transaction view", &self.transaction_digest)?;
        let expected = self
            .read
            .commit_cursor
            .checked_add(self.pending_mutations as u64)
            .ok_or_else(|| Error::InvalidRuntime {
                reason: "transaction view cursor overflowed".into(),
            })?;
        if self.prospective_cursor != expected {
            return Err(Error::InvalidRuntime {
                reason: "transaction view cursor does not cover its pending mutations".into(),
            });
        }
        if self.pending.len() != self.pending_mutations {
            return Err(Error::InvalidRuntime {
                reason: "transaction view pending count does not match its mutations".into(),
            });
        }
        Ok(())
    }

    pub fn record(&self, reference: &RuntimeRef) -> Option<&RuntimeRecord> {
        self.records
            .iter()
            .find(|record| &record.reference == reference)
    }

    pub fn relation(&self, reference: &RuntimeRef) -> Option<&RuntimeRelation> {
        self.relations
            .iter()
            .find(|relation| &relation.reference == reference)
    }

    pub fn claim_writes(&self) -> impl Iterator<Item = &Claim> {
        self.pending.iter().filter_map(|mutation| match mutation {
            RuntimeMutation::Claim { claim } => Some(claim),
            _ => None,
        })
    }

    pub fn schema_write(&self) -> Option<&RuntimeSchemaRegistry> {
        self.pending.iter().find_map(|mutation| match mutation {
            RuntimeMutation::Schema { registry } => Some(registry),
            _ => None,
        })
    }

    pub fn events(&self) -> impl Iterator<Item = &RuntimeEvent> {
        self.pending.iter().filter_map(|mutation| match mutation {
            RuntimeMutation::Event { event } => Some(event),
            _ => None,
        })
    }

    pub fn vectors(&self) -> impl Iterator<Item = &RuntimeVector> {
        self.pending.iter().filter_map(|mutation| match mutation {
            RuntimeMutation::Vector { vector } => Some(vector),
            _ => None,
        })
    }

    pub fn series_samples(&self) -> impl Iterator<Item = &RuntimeSeriesSample> {
        self.pending.iter().filter_map(|mutation| match mutation {
            RuntimeMutation::SeriesSample { sample } => Some(sample),
            _ => None,
        })
    }

    pub fn geo_values(&self) -> impl Iterator<Item = &RuntimeGeo> {
        self.pending.iter().filter_map(|mutation| match mutation {
            RuntimeMutation::Geo { geo } => Some(geo),
            _ => None,
        })
    }

    pub fn objects(&self) -> impl Iterator<Item = &ObjectReference> {
        self.pending.iter().filter_map(|mutation| match mutation {
            RuntimeMutation::Object { object } => Some(object),
            _ => None,
        })
    }
}
