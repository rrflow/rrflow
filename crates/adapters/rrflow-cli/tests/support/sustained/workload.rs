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
    fn scope(&self) -> String {
        format!("instance:{}", self.client.instance_id())
    }
    fn body(&self, revision: u64) -> String {
        // Exactly 1 KiB baseline payload; vector and relation bytes are additional.
        let prefix = format!("{:016x}:{revision:016x}:", self.seed);
        format!("{prefix}{}", "x".repeat(1024 - prefix.len()))
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
        vec![
            TransactionMutation::PutRecord {
                reference: reference("capacity_document", key),
                valid_from: self.valid_at,
                valid_to: None,
                properties: BTreeMap::from([
                    ("body".into(), QueryValue::String(self.body(revision))),
                    ("revision".into(), QueryValue::Unsigned(revision)),
                ]),
            },
            TransactionMutation::PutRelation {
                reference: reference("capacity_link", key),
                from: reference("capacity_document", key),
                to: reference("capacity_document", "anchor"),
                valid_from: self.valid_at,
                valid_to: None,
                properties: BTreeMap::from([("revision".into(), QueryValue::Unsigned(revision))]),
            },
            TransactionMutation::PutVector {
                reference: reference("capacity_vector", key),
                subject: reference("capacity_document", key),
                collection_id: id("capacity_documents"),
                vector_name: id("body"),
                field: id("embedding"),
                valid_from: self.valid_at,
                valid_to: None,
                value: self.vector(revision),
                provenance: None,
                properties: BTreeMap::new(),
            },
        ]
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
        let seed = self.prepare("baseline".into(), StorageLoadKind::MixedWrite, 0)?;
        self.execute(&seed).await?;
        self.verify(&seed).await?;
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
                if kind == StorageLoadKind::MixedWrite {
                    &key
                } else {
                    "baseline"
                },
                if kind == StorageLoadKind::MixedWrite {
                    revision
                } else {
                    0
                },
                self.seed,
                self.dimensions,
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
                self.verify_key("baseline", 0).await?;
                digest("c07g.read-receipt", &request.effect_sha256)?
            }
            StorageLoadKind::RangeRead => {
                // A bounded range includes independently known baseline and any
                // completed mixed writes. It may never hide truncation.
                let result = self
                    .query(
                        "record:capacity_document",
                        "id >= \"baseline\" AND id < \"baselinz\"",
                        "id, body, revision",
                        None,
                    )
                    .await?;
                check(result.rows.len() == 1, "bounded range lost baseline")?;
                check(
                    result.rows[0].values.get("body") == Some(&QueryValue::String(self.body(0))),
                    "range value differs",
                )?;
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
            parameters:BTreeMap::new(),budget:QueryBudget {max_storage_keys:1_000_000,max_rows:64,max_output_bytes:512*1024,
                max_batch_rows:64,max_memory_bytes:64*1024*1024,max_spill_bytes:64*1024*1024,max_elapsed_ms:30_000}},
            RequestOptions::read("capacity-read","capacity-read").unwrap()).await.map_err(error)?;
        check(!result.execution.truncated, "capacity read truncated")?;
        Ok(result)
    }
    pub async fn verify(&self, request: &Prepared) -> Result<()> {
        if request.kind == StorageLoadKind::MixedWrite {
            self.verify_key(&request.id, request.revision).await
        } else {
            self.verify_key("baseline", 0).await
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
                    references: vec![reference("capacity_vector", key)],
                    max_storage_keys: 1_000_000,
                },
                RequestOptions::read("capacity-vector-read", "capacity-vector-read").unwrap(),
            )
            .await
            .map_err(error)?;
        check(
            vectors.missing.is_empty() && vectors.points.len() == 1,
            "mixed batch lost vector",
        )?;
        let vector = &vectors.points[0];
        check(
            vector.reference == reference("capacity_vector", key)
                && vector.subject == reference("capacity_document", key)
                && vector.value == self.vector(revision),
            "mixed vector differs from independent input",
        )?;
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
        let relation = self
            .query(
                "relation:capacity_link",
                &filter,
                "id, from_id, to_id, revision",
                Some(vectors.known_at_cursor),
            )
            .await?;
        check(relation.rows.len() == 1, "mixed batch lost relation")?;
        let values = &relation.rows[0].values;
        check(
            values.get("from_id") == Some(&QueryValue::String(key.into()))
                && values.get("to_id") == Some(&QueryValue::String("anchor".into()))
                && values.get("revision") == Some(&QueryValue::Unsigned(revision)),
            "relation readback differs",
        )
    }
}
