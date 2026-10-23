//! Installed/public-contract mixed-family corpus; expected values are generated
//! independently from request coordinates, never copied out of engine answers.
use rrd_client::{RequestOptions, RrdClient, Session};
use rrd_contract::*;
use rrflow_eval::storage::{digest, Digest, StorageLoadKind};
use std::collections::BTreeMap;

type Result<T> = std::result::Result<T, String>;

#[derive(Clone)]
pub(super) struct Workload {
    pub client: RrdClient,
    pub session: Session,
    pub valid_at: u64,
    pub dimensions: u32,
    pub seed: u64,
    pub records: u64,
    pub edges_per_record: u32,
    pub vectors_per_record: u32,
    pub payload_bytes: usize,
    pub query_budget: QueryBudget,
}

#[derive(Clone)]
pub(super) struct Prepared {
    pub id: String,
    pub kind: StorageLoadKind,
    pub request_sha256: Digest,
    pub effect_sha256: Digest,
    pub mutation: Option<CommitTransaction>,
    pub revision: u64,
}

pub(super) struct Completed {
    pub receipt_sha256: Digest,
}

fn id(value: &str) -> CanonicalId {
    CanonicalId::new(value).expect("bounded corpus identifier")
}
fn reference(kind: &str, value: &str) -> DataReference {
    DataReference {
        kind: id(kind),
        id: id(value),
    }
}
fn options(value: &str) -> RequestOptions {
    RequestOptions::new(value, value, Some(value), Some(u64::MAX)).unwrap()
}
fn check(value: bool, reason: &str) -> Result<()> {
    if value {
        Ok(())
    } else {
        Err(reason.into())
    }
}
fn error(value: impl std::fmt::Display) -> String {
    value.to_string()
}

impl Workload {
    /// Conservative owned request/encoding allowance, including escaping,
    /// vector float formatting and record/edge identities. Segment admission
    /// includes both the prepared owner and its in-flight clone.
    pub fn retained_request_bound(&self) -> u64 {
        self.payload_bytes as u64 * 8
            + u64::from(self.vectors_per_record) * (u64::from(self.dimensions) * 32 + 4096)
            + u64::from(self.edges_per_record) * 4096
            + 8192
    }
    fn scope(&self) -> String {
        format!("instance:{}", self.client.instance_id())
    }
    fn body(&self, revision: u64) -> String {
        // Baseline payload is at least 1 KiB; larger-than-memory profiles freeze
        // a larger source payload. Vector and relation bytes are additional.
        let prefix = format!("{:016x}:{revision:016x}:", self.seed);
        let mut body = prefix.into_bytes();
        let mut state = self.seed ^ revision ^ 0xa0761d6478bd642f;
        while body.len() < self.payload_bytes {
            state = state.wrapping_add(0x9e3779b97f4a7c15);
            let mut word = state;
            word = (word ^ (word >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
            word = (word ^ (word >> 27)).wrapping_mul(0x94d049bb133111eb);
            word ^= word >> 31;
            for byte in word.to_le_bytes() {
                if body.len() == self.payload_bytes {
                    break;
                }
                body.push(b'!' + byte % 90);
            }
        }
        String::from_utf8(body).expect("generated ASCII")
    }
    fn vector(&self, revision: u64) -> DataVectorValue {
        DataVectorValue::Dense {
            values: (0..self.dimensions)
                .map(|n| {
                    ((self.seed.wrapping_add(revision).wrapping_add(u64::from(n))) % 257) as f32
                        / 256.0
                })
                .collect(),
        }
    }
    fn mutations(&self, key: &str, revision: u64) -> Vec<TransactionMutation> {
        let mut mutations = vec![TransactionMutation::PutRecord {
            reference: reference("capacity_document", key),
            valid_from: self.valid_at,
            valid_to: None,
            properties: BTreeMap::from([
                ("body".into(), QueryValue::String(self.body(revision))),
                ("revision".into(), QueryValue::Unsigned(revision)),
            ]),
        }];
        for ordinal in 0..self.edges_per_record {
            mutations.push(TransactionMutation::PutRelation {
                reference: reference("capacity_link", &format!("edge-{ordinal}-{key}")),
                from: reference("capacity_document", key),
                to: reference("capacity_document", "anchor"),
                valid_from: self.valid_at,
                valid_to: None,
                properties: BTreeMap::from([("revision".into(), QueryValue::Unsigned(revision))]),
            });
        }
        for ordinal in 0..self.vectors_per_record {
            mutations.push(TransactionMutation::PutVector {
                reference: reference("capacity_vector", &format!("vector-{ordinal}-{key}")),
                subject: reference("capacity_document", key),
                collection_id: id("capacity_documents"),
                vector_name: id("body"),
                field: id("embedding"),
                valid_from: self.valid_at,
                valid_to: None,
                value: self.vector(revision.wrapping_add(u64::from(ordinal))),
                provenance: None,
                properties: BTreeMap::new(),
            });
        }
        mutations
    }
    fn read_ordinal(&self, revision: u64) -> u64 {
        // Stable SplitMix64 coordinates, including uniform and hot-set lanes.
        let mut x = self
            .seed
            .wrapping_add(revision.wrapping_mul(0x9e3779b97f4a7c15));
        x = (x ^ (x >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        x = (x ^ (x >> 27)).wrapping_mul(0x94d049bb133111eb);
        let width = if revision.is_multiple_of(4) {
            self.records.min(64)
        } else {
            self.records
        };
        (x ^ (x >> 31)) % width
    }
    pub fn seed_record(&self, ordinal: u64) -> Result<Prepared> {
        self.prepare(
            format!("seed-{ordinal:020}"),
            StorageLoadKind::MixedWrite,
            ordinal,
        )
    }
    async fn commit(&self, key: &str, request: CommitTransaction) -> Result<CommitReceipt> {
        let lease = self
            .client
            .begin_transaction(
                &self.session,
                BeginTransaction {
                    scope: id("data"),
                    timeout_ms: 60_000,
                },
                options(&format!("{key}-begin")),
            )
            .await
            .map_err(error)?;
        self.client
            .commit_transaction(&self.session, &lease.transaction_id, request, options(key))
            .await
            .map_err(error)
    }
    pub async fn initialize(&self) -> Result<()> {
        let tables = [
            ("capacity_document", DataLogicalModel::Document),
            ("capacity_link", DataLogicalModel::GraphRelation),
            ("capacity_vector", DataLogicalModel::Vector),
        ]
        .into_iter()
        .map(|(name, model)| {
            (
                id(name),
                DataTableSchema {
                    model,
                    mode: DataSchemaMode::Schemaless,
                    properties: BTreeMap::new(),
                    allow_additional_properties: true,
                },
            )
        })
        .collect();
        let mutations = vec![
            TransactionMutation::PutSchema {
                registry: DataSchemaRegistry {
                    revision: 1,
                    migration: "installed capacity corpus".into(),
                    catalogue: DataCatalogueIdentity::default(),
                    tables,
                    records: BTreeMap::new(),
                    relations: BTreeMap::new(),
                    events: BTreeMap::new(),
                },
            },
            TransactionMutation::PutRecord {
                reference: reference("capacity_document", "anchor"),
                valid_from: self.valid_at,
                valid_to: None,
                properties: BTreeMap::new(),
            },
        ];
        self.commit(
            "capacity-schema",
            CommitTransaction {
                operation_sha256: transaction_operation_sha256(&mutations),
                mutations,
            },
        )
        .await?;
        self.client
            .ensure_vector_collection(
                &self.session,
                EnsureVectorCollection {
                    scope: self.scope(),
                    collection_id: id("capacity_documents"),
                    vectors: vec![NamedVectorDefinition {
                        name: id("body"),
                        field: id("embedding"),
                        kind: VectorValueKind::Dense,
                        dimensions: self.dimensions,
                        metric: VectorSearchMetric::Dot,
                        embedding_model: None,
                        memory_tier: VectorMemoryTier::Cold,
                    }],
                },
                options("capacity-collection"),
            )
            .await
            .map_err(error)?;
        Ok(())
    }
    pub fn prepare(&self, key: String, kind: StorageLoadKind, revision: u64) -> Result<Prepared> {
        let mutation = if kind == StorageLoadKind::MixedWrite {
            let mutations = self.mutations(&key, revision);
            Some(CommitTransaction {
                operation_sha256: transaction_operation_sha256(&mutations),
                mutations,
            })
        } else {
            None
        };
        let request_sha256 = digest(
            "c07g.public-request",
            &(&key, kind, revision, self.seed, self.dimensions, &mutation),
        )?;
        let effect_sha256 = digest(
            "c07g.expected-effect",
            &(
                kind,
                &key,
                revision,
                self.read_ordinal(revision),
                self.seed,
                self.dimensions,
                self.records,
                self.edges_per_record,
                self.vectors_per_record,
                self.payload_bytes,
            ),
        )?;
        Ok(Prepared {
            id: key,
            kind,
            request_sha256,
            effect_sha256,
            mutation,
            revision,
        })
    }
    pub async fn execute(&self, request: &Prepared) -> Result<Completed> {
        let receipt_sha256 = match request.kind {
            StorageLoadKind::MixedWrite => {
                let mut receipt = self
                    .commit(
                        &request.id,
                        request
                            .mutation
                            .clone()
                            .ok_or("missing prepared mutation")?,
                    )
                    .await?;
                receipt.idempotent_replay = false;
                digest("c07g.commit-receipt", &receipt)?
            }
            StorageLoadKind::PointRead => {
                let ordinal = self.read_ordinal(request.revision);
                self.verify_key(&format!("seed-{ordinal:020}"), ordinal)
                    .await?;
                digest("c07g.read-receipt", &request.effect_sha256)?
            }
            StorageLoadKind::RangeRead => {
                let first = self.read_ordinal(request.revision);
                let width =
                    (self.query_budget.max_output_bytes as usize / 2 / (self.payload_bytes + 2048))
                        .clamp(1, 16) as u64;
                let end = first.saturating_add(width).min(self.records);
                let filter = format!("id >= \"seed-{first:020}\" AND id < \"seed-{end:020}\"");
                let result = self
                    .query(
                        "record:capacity_document",
                        &filter,
                        "id, body, revision",
                        None,
                    )
                    .await?;
                check(
                    result.rows.len() as u64 == end - first,
                    "bounded range lost or added records",
                )?;
                let mut expected = (first..end)
                    .map(|n| (format!("seed-{n:020}"), n))
                    .collect::<BTreeMap<_, _>>();
                for row in result.rows {
                    let Some(QueryValue::String(key)) = row.values.get("id") else {
                        return Err("range identifier missing".into());
                    };
                    let ordinal = expected
                        .remove(key)
                        .ok_or("range returned duplicate or unrelated identity")?;
                    check(
                        row.values.get("body") == Some(&QueryValue::String(self.body(ordinal)))
                            && row.values.get("revision") == Some(&QueryValue::Unsigned(ordinal)),
                        "range differs from generated source",
                    )?;
                }
                check(expected.is_empty(), "range incomplete")?;
                digest("c07g.read-receipt", &request.effect_sha256)?
            }
            StorageLoadKind::Maintenance => {
                let lease = self
                    .client
                    .begin_transaction(
                        &self.session,
                        BeginTransaction {
                            scope: id("data"),
                            timeout_ms: 60_000,
                        },
                        options(&format!("{}-begin", request.id)),
                    )
                    .await
                    .map_err(error)?;
                let cancelled = self
                    .client
                    .abort_transaction(
                        &self.session,
                        &lease.transaction_id,
                        AbortTransaction {},
                        options(&request.id),
                    )
                    .await
                    .map_err(error)?;
                digest("c07g.cancel-receipt", &cancelled)?
            }
        };
        Ok(Completed { receipt_sha256 })
    }
    async fn query(
        &self,
        source: &str,
        filter: &str,
        projection: &str,
        cursor: Option<u64>,
    ) -> Result<QueryResult> {
        let known = cursor.map_or_else(|| "HEAD".into(), |v| v.to_string());
        let result=self.client.execute_query(&self.session,ExecuteQuery {scope:self.scope(),
            query:format!("FROM {source} AT VALID {} KNOWN {known} WHERE {filter} PROJECT {projection} EXPLAIN CONTRACT",self.valid_at),
            parameters:BTreeMap::new(),budget:self.query_budget.clone()},
            RequestOptions::read("capacity-read","capacity-read").unwrap()).await.map_err(error)?;
        check(!result.execution.truncated, "capacity read truncated")?;
        Ok(result)
    }
    pub async fn verify(&self, request: &Prepared) -> Result<()> {
        if request.kind == StorageLoadKind::MixedWrite {
            self.verify_key(&request.id, request.revision).await
        } else {
            let ordinal = self.read_ordinal(request.revision);
            self.verify_key(&format!("seed-{ordinal:020}"), ordinal)
                .await
        }
    }
    async fn verify_key(&self, key: &str, revision: u64) -> Result<()> {
        let vectors = self
            .client
            .retrieve_vector_points(
                &self.session,
                RetrieveVectorPoints {
                    scope: self.scope(),
                    collection_id: id("capacity_documents"),
                    vector_name: id("body"),
                    valid_at: self.valid_at,
                    references: (0..self.vectors_per_record)
                        .map(|n| reference("capacity_vector", &format!("vector-{n}-{key}")))
                        .collect(),
                    max_storage_keys: self
                        .query_budget
                        .max_storage_keys
                        .min(MAX_VECTOR_STORAGE_KEYS),
                },
                RequestOptions::read("capacity-vector-read", "capacity-vector-read").unwrap(),
            )
            .await
            .map_err(error)?;
        check(
            vectors.missing.is_empty() && vectors.points.len() == self.vectors_per_record as usize,
            "mixed batch lost vector",
        )?;
        let mut seen = std::collections::BTreeSet::new();
        for vector in &vectors.points {
            let ordinal = (0..self.vectors_per_record)
                .find(|n| {
                    vector.reference == reference("capacity_vector", &format!("vector-{n}-{key}"))
                })
                .ok_or("vector reference differs")?;
            check(
                seen.insert(ordinal)
                    && vector.subject == reference("capacity_document", key)
                    && vector.value == self.vector(revision.wrapping_add(u64::from(ordinal))),
                "mixed vector differs from independent input",
            )?;
        }
        let filter = format!("id = {}", serde_json::to_string(key).map_err(error)?);
        let record = self
            .query(
                "record:capacity_document",
                &filter,
                "id, body, revision",
                Some(vectors.known_at_cursor),
            )
            .await?;
        check(record.rows.len() == 1, "mixed batch lost record")?;
        check(
            record.rows[0].values.get("body") == Some(&QueryValue::String(self.body(revision)))
                && record.rows[0].values.get("revision") == Some(&QueryValue::Unsigned(revision)),
            "record readback differs",
        )?;
        let filter = format!("from_id = {}", serde_json::to_string(key).map_err(error)?);
        let relation = self
            .query(
                "relation:capacity_link",
                &filter,
                "id, from_id, to_id, revision",
                Some(vectors.known_at_cursor),
            )
            .await?;
        check(
            relation.rows.len() == self.edges_per_record as usize,
            "mixed batch lost or duplicated relation",
        )?;
        let mut expected = (0..self.edges_per_record)
            .map(|n| format!("edge-{n}-{key}"))
            .collect::<std::collections::BTreeSet<_>>();
        for row in relation.rows {
            let values = &row.values;
            let Some(QueryValue::String(identity)) = values.get("id") else {
                return Err("relation identity missing".into());
            };
            check(
                expected.remove(identity)
                    && values.get("from_id") == Some(&QueryValue::String(key.into()))
                    && values.get("to_id") == Some(&QueryValue::String("anchor".into()))
                    && values.get("revision") == Some(&QueryValue::Unsigned(revision)),
                "relation readback differs",
            )?;
        }
        check(expected.is_empty(), "relation set incomplete")
    }
}
