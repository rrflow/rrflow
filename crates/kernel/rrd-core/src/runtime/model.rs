//! Runtime values, records, relations, events, mutations and commits.

use super::*;

/// Dependency-free property values. Decimal values are strings deliberately:
/// their canonical identity must not depend on platform floating-point rules.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum RuntimeValue {
    Null,
    Bool(bool),
    Integer(i64),
    Unsigned(u64),
    Decimal(String),
    String(String),
    Digest(String),
    List(Vec<RuntimeValue>),
    Map(BTreeMap<String, RuntimeValue>),
}

pub type RuntimeProperties = BTreeMap<String, RuntimeValue>;

/// One immutable version of a graph node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeRecord {
    #[serde(flatten)]
    pub reference: RuntimeRef,
    pub valid_from: Millis,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valid_to: Option<Millis>,
    #[serde(default)]
    pub properties: RuntimeProperties,
}

/// One immutable version of a directed, typed graph edge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeRelation {
    #[serde(flatten)]
    pub reference: RuntimeRef,
    pub from: RuntimeRef,
    pub to: RuntimeRef,
    pub valid_from: Millis,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valid_to: Option<Millis>,
    #[serde(default)]
    pub properties: RuntimeProperties,
}

/// A lifecycle fact that need not itself be a graph node or edge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeEvent {
    pub kind: RuntimeType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<RuntimeRef>,
    #[serde(default)]
    pub properties: RuntimeProperties,
}

/// One catalogue-bound retirement fact.
///
/// Retirement is appended to the authenticated runtime log. Backends may
/// remove the target from their current-state materialization, but historical
/// reads continue to reconstruct the value before `effective_at`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeRetirement {
    pub model: crate::RuntimeLogicalModel,
    pub reference: RuntimeRef,
    pub effective_at: Millis,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "mutation", rename_all = "snake_case")]
pub enum RuntimeMutation {
    Claim {
        claim: Claim,
    },
    /// A complete, versioned registry replacing the prior registry for this
    /// scope. Storage validates revision continuity and every co-committed
    /// object against the resulting schema before writing anything.
    Schema {
        registry: RuntimeSchemaRegistry,
    },
    Record {
        record: RuntimeRecord,
    },
    Relation {
        relation: RuntimeRelation,
    },
    Event {
        event: RuntimeEvent,
    },
    Vector {
        vector: RuntimeVector,
    },
    SeriesSample {
        sample: RuntimeSeriesSample,
    },
    Geo {
        geo: RuntimeGeo,
    },
    Object {
        object: ObjectReference,
    },
    Retire {
        retirement: RuntimeRetirement,
    },
}

/// The caller-observed head is mandatory. This is the compare-and-swap that
/// turns concurrent writers into explicit conflicts rather than lost updates.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeCommit {
    pub scope: ScopeId,
    pub at: Millis,
    pub actor: String,
    pub expected_cursor: u64,
    pub mutations: Vec<RuntimeMutation>,
}

impl RuntimeCommit {
    pub fn validate(&self) -> Result<()> {
        validate_text("runtime actor", &self.actor)?;
        if self.mutations.is_empty() {
            return Err(Error::InvalidRuntime {
                reason: "runtime commit must contain at least one mutation".into(),
            });
        }

        let mut identities = BTreeSet::new();
        let mut schema_count = 0;
        for mutation in &self.mutations {
            match mutation {
                RuntimeMutation::Claim { claim } => claim.validate()?,
                RuntimeMutation::Schema { registry } => {
                    schema_count += 1;
                    registry.validate()?;
                    if schema_count > 1 {
                        return Err(Error::InvalidRuntime {
                            reason: "runtime commit may contain at most one schema mutation".into(),
                        });
                    }
                }
                RuntimeMutation::Record { record } => {
                    validate_window(record.valid_from, record.valid_to)?;
                    validate_properties(&record.properties)?;
                    if !identities.insert(("record", record.reference.clone())) {
                        return duplicate_identity("record", &record.reference);
                    }
                }
                RuntimeMutation::Relation { relation } => {
                    validate_window(relation.valid_from, relation.valid_to)?;
                    validate_properties(&relation.properties)?;
                    if !identities.insert(("relation", relation.reference.clone())) {
                        return duplicate_identity("relation", &relation.reference);
                    }
                }
                RuntimeMutation::Event { event } => validate_properties(&event.properties)?,
                RuntimeMutation::Vector { vector } => {
                    vector.validate()?;
                    validate_properties(&vector.properties)?;
                    if !identities.insert(("vector", vector.reference.clone())) {
                        return duplicate_identity("vector", &vector.reference);
                    }
                }
                RuntimeMutation::SeriesSample { sample } => {
                    sample.validate()?;
                    validate_properties(&sample.properties)?;
                    if !identities.insert(("series_sample", sample.reference.clone())) {
                        return duplicate_identity("series sample", &sample.reference);
                    }
                }
                RuntimeMutation::Geo { geo } => {
                    geo.validate()?;
                    validate_properties(&geo.properties)?;
                    if !identities.insert(("geo", geo.reference.clone())) {
                        return duplicate_identity("geo", &geo.reference);
                    }
                }
                RuntimeMutation::Object { object } => {
                    object.validate()?;
                    validate_properties(&object.properties)?;
                    if !identities.insert(("object", object.reference.clone())) {
                        return duplicate_identity("object", &object.reference);
                    }
                }
                RuntimeMutation::Retire { retirement } => {
                    if retirement.effective_at == 0 {
                        return Err(Error::InvalidRuntime {
                            reason: "runtime retirement effective_at must be greater than zero"
                                .into(),
                        });
                    }
                    if retirement.model == crate::RuntimeLogicalModel::ReasoningClaim {
                        return Err(Error::InvalidRuntime {
                            reason:
                                "reasoning claims retire through their bitemporal claim contract"
                                    .into(),
                        });
                    }
                    if !identities.insert(("retirement", retirement.reference.clone())) {
                        return duplicate_identity("retirement", &retirement.reference);
                    }
                }
            }
        }
        Ok(())
    }

    /// Content identity for idempotency and cross-adapter conformance.
    pub fn digest(&self) -> String {
        digest::sha256_hex(&self.canonical_bytes())
    }

    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut out = b"rrd-engine-commit-v1\0".to_vec();
        text(&mut out, self.scope.as_str());
        out.extend_from_slice(&self.at.to_be_bytes());
        text(&mut out, &self.actor);
        out.extend_from_slice(&self.expected_cursor.to_be_bytes());
        out.extend_from_slice(&(self.mutations.len() as u64).to_be_bytes());
        for mutation in &self.mutations {
            encode_mutation(&mut out, mutation);
        }
        out
    }
}

/// A mutation after the store has assigned its global position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeChange {
    pub cursor: u64,
    pub commit_id: String,
    pub commit_ordinal: u64,
    pub scope: ScopeId,
    pub at: Millis,
    pub actor: String,
    pub mutation: RuntimeMutation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_digest: Option<String>,
    pub digest: String,
}

impl RuntimeChange {
    pub fn committed(
        cursor: u64,
        commit: &RuntimeCommit,
        commit_id: &str,
        commit_ordinal: u64,
        mutation: RuntimeMutation,
        previous_digest: Option<String>,
    ) -> Self {
        let mut change = Self {
            cursor,
            commit_id: commit_id.to_owned(),
            commit_ordinal,
            scope: commit.scope.clone(),
            at: commit.at,
            actor: commit.actor.clone(),
            mutation,
            previous_digest,
            digest: String::new(),
        };
        change.digest = digest::sha256_hex(&change.canonical_bytes());
        change
    }

    pub fn verify_digest(&self) -> bool {
        digest::sha256_hex(&self.canonical_bytes()) == self.digest
    }

    /// Canonical leaf input for the authenticated runtime log. The Merkle
    /// layer applies its own RFC 9162 domain separator to these bytes.
    pub fn authenticated_log_bytes(&self) -> Vec<u8> {
        self.canonical_bytes()
    }

    fn canonical_bytes(&self) -> Vec<u8> {
        let mut out = b"rrd-engine-change-v1\0".to_vec();
        out.extend_from_slice(&self.cursor.to_be_bytes());
        text(&mut out, &self.commit_id);
        out.extend_from_slice(&self.commit_ordinal.to_be_bytes());
        text(&mut out, self.scope.as_str());
        out.extend_from_slice(&self.at.to_be_bytes());
        text(&mut out, &self.actor);
        encode_mutation(&mut out, &self.mutation);
        optional_text(&mut out, self.previous_digest.as_deref());
        out
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeCommitOutcome {
    pub commit_id: String,
    pub first_cursor: u64,
    pub last_cursor: u64,
    pub count: usize,
    pub first_claim_sequence: Option<u64>,
    pub last_claim_sequence: Option<u64>,
    pub outbox_count: usize,
}
