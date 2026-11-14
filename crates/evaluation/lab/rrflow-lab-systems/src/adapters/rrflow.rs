//! RRFlow through the public typed client, the same path any application takes.
//! It never opens storage directly, so the lab measures what a caller gets.

use crate::adapter::{
    required_credentials, required_instance, split_by_size, CollectionSpec, ConnectionLimits,
    PointBatch, SearchRequest, SystemAdapter,
};

use crate::binding::{IndexSpec, SystemBinding};

use rrd_client::{ClientConfig, RequestOptions, RrdClient, Session};

use rrd_contract::{
    transaction_operation_sha256, BeginTransaction, CanonicalId, CommitTransaction, CreateSession,
    EnsureVectorIndexResult, ReadDataSchema, SessionLimits, TransactionMutation,
    MAX_TRANSACTION_CLAIMS, MAX_VECTOR_STORAGE_KEYS,
};

use rrflow_evaluation_core::LabMetric;

use rrflow_evaluation_core::Result;

use serde::de::DeserializeOwned;

use serde_json::{json, Value};

use std::collections::BTreeSet;

use std::sync::atomic::{AtomicU64, Ordering};

use std::sync::{Mutex, OnceLock, RwLock};

use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use tokio::runtime::Runtime;

const VECTOR_NAME: &str = "lab";

const FILTER_PROPERTY: &str = "category";

const HNSW_MAX_LEVEL: u8 = 6;

const HNSW_SEED: u64 = 7;

const VECTOR_FIELD: &str = "lab-embedding";

/// Each point is a subject record plus its vector, so two mutations.
const MUTATIONS_PER_POINT: usize = 2;

/// The server refuses a request body over one MiB, so commits split by encoded size.
const MAX_COMMIT_BYTES: usize = 960 * 1024;

const VALID_FROM: u64 = 100;

const VALID_AT: u64 = 200;

const SESSION_LEASE_MS: u64 = 3_600_000;

/// Renewing cannot pass a session's absolute expiry, and it invalidates the token another
/// client may be about to send. So a long run opens new sessions instead, and each earlier
/// one stays valid for whatever is still using it.
const SESSION_REPLACE_AFTER: Duration = Duration::from_secs(1_200);

#[derive(Debug, Clone)]
struct Prepared {
    record_table: String,
    vector_table: String,
    collection: String,
    index: IndexSpec,
}

struct Lease {
    session: Session,
    opened_at: Instant,
}

pub struct RrflowAdapter {
    runtime: Runtime,
    client: RrdClient,
    build_client: RrdClient,
    principal: CanonicalId,
    secret: String,
    lease: RwLock<Lease>,
    scope: String,
    run: String,
    counter: AtomicU64,
    prepared: OnceLock<Prepared>,
    access_paths: Mutex<BTreeSet<String>>,
}

fn decode<T: DeserializeOwned>(value: Value) -> Result<T> {
    serde_json::from_value(value).map_err(|error| error.to_string())
}

fn id(text: &str) -> Result<CanonicalId> {
    CanonicalId::new(text).map_err(|error| error.to_string())
}

fn open_session(
    runtime: &Runtime,
    client: &RrdClient,
    principal: &CanonicalId,
    secret: &str,
    options: RequestOptions,
) -> Result<Session> {
    runtime
        .block_on(client.create_session(
            principal.clone(),
            secret,
            CreateSession {
                limits: SessionLimits {
                    idle_timeout_ms: SESSION_LEASE_MS,
                    absolute_timeout_ms: SESSION_LEASE_MS,
                    max_open_transactions: 8,
                },
            },
            options,
        ))
        .map_err(|error| error.to_string())
}

/// The point mutations for one commit.
pub(crate) fn point_mutations(
    prepared_tables: (&str, &str, &str),
    batch: &PointBatch<'_>,
) -> Value {
    let (record_table, vector_table, collection) = prepared_tables;
    let mut mutations = Vec::with_capacity(batch.ids.len() * MUTATIONS_PER_POINT);
    for index in 0..batch.ids.len() {
        let point = format!("p{}", batch.ids[index]);
        mutations.push(json!({
            "mutation": "put_record",
            "reference": {"kind": record_table, "id": point},
            "valid_from": VALID_FROM,
            "properties": {}
        }));
        mutations.push(json!({
            "mutation": "put_vector",
            "reference": {"kind": vector_table, "id": point},
            "subject": {"kind": record_table, "id": point},
            "collection_id": collection,
            "vector_name": VECTOR_NAME,
            "field": VECTOR_FIELD,
            "valid_from": VALID_FROM,
            "value": {"kind": "dense", "values": batch.vector(index)},
            "properties": {"category": {"type": "unsigned", "value": batch.categories[index]}}
        }));
    }
    Value::Array(mutations)
}

pub(crate) fn parse_point(reference: &str) -> Result<u64> {
    reference
        .strip_prefix('p')
        .and_then(|digits| digits.parse().ok())
        .ok_or_else(|| format!("unexpected point reference {reference}"))
}

pub(crate) fn search_mode(
    index: &IndexSpec,
    ef: Option<u32>,
    rerank: Option<u32>,
    k: u32,
) -> Value {
    match index {
        IndexSpec::TurboQuant { .. } | IndexSpec::Hnsw { .. } => {
            let ef = ef.unwrap_or(k);
            json!({
                "mode": "require_approximate",
                "exact_rerank": rerank.unwrap_or(ef),
                "ef_search": ef
            })
        }
        _ => json!({"mode": "exact"}),
    }
}

/// The tier a collection's generation lives in. A graph's artifact is JSON, and above the cached
/// tier's bound it would be decoded on every query, so a graph is pinned, and its system says so.
pub(crate) fn memory_tier(index: &IndexSpec) -> &'static str {
    match index {
        IndexSpec::Hnsw { .. } => "pinned",
        _ => "cached",
    }
}

/// The public request that declares the filter property a graph's generation covers.
pub(crate) fn payload_index_request(scope: &str, collection: &str) -> Value {
    json!({
        "scope": scope,
        "collection_id": collection,
        "field": FILTER_PROPERTY,
        "kind": "unsigned"
    })
}

/// The public request that builds a graph over a collection's vector.
pub(crate) fn hnsw_index_request(
    scope: &str,
    collection: &str,
    m: u32,
    ef_construction: u32,
) -> Value {
    json!({
        "scope": scope,
        "collection_id": collection,
        "vector_name": VECTOR_NAME,
        "configuration": {
            "kind": "hnsw",
            "m": m,
            "ef_construction": ef_construction,
            "max_level": HNSW_MAX_LEVEL,
            "seed": HNSW_SEED,
            "filter_properties": [FILTER_PROPERTY]
        },
        "max_storage_keys": MAX_VECTOR_STORAGE_KEYS
    })
}

impl RrflowAdapter {
    pub fn connect(binding: &SystemBinding, limits: &ConnectionLimits) -> Result<Self> {
        let (principal, secret) = required_credentials(binding)?;
        let instance = required_instance(binding)?;
        let address = binding
            .endpoint
            .trim_start_matches("http://")
            .trim_end_matches('/')
            .parse()
            .map_err(|error| format!("RRFlow endpoint must be ADDRESS:PORT: {error}"))?;
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .map_err(|error| error.to_string())?;
        let instance_id = id(&instance)?;
        let config = ClientConfig {
            request_timeout: limits.request_timeout,
            ..ClientConfig::default()
        };
        let client = runtime
            .block_on(async { RrdClient::connect_local(address, instance_id.clone(), config) })
            .map_err(|error| error.to_string())?;
        // A graph is built inside one request, so it gets the time a system has to become ready.
        let build_config = ClientConfig {
            request_timeout: limits.readiness_timeout,
            ..ClientConfig::default()
        };
        let build_client = runtime
            .block_on(async { RrdClient::connect_local(address, instance_id, build_config) })
            .map_err(|error| error.to_string())?;
        let started = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| error.to_string())?
            .as_nanos();
        let run = format!("lab-{}-{started}", std::process::id());
        let options = RequestOptions::mutation(
            &format!("{run}-session-request"),
            &format!("{run}-session-operation"),
            &format!("{run}-session"),
        )
        .map_err(|error| error.to_string())?;
        let principal = id(&principal)?;
        let session = open_session(&runtime, &client, &principal, &secret, options)?;
        let scope = format!("instance:{}", client.instance_id());
        Ok(Self {
            runtime,
            client,
            build_client,
            principal,
            secret,
            lease: RwLock::new(Lease {
                session,
                opened_at: Instant::now(),
            }),
            scope,
            run,
            counter: AtomicU64::new(0),
            prepared: OnceLock::new(),
            access_paths: Mutex::new(BTreeSet::new()),
        })
    }

    /// Connect to a running engine by address for the planning tasks, which have no system
    /// binding or workload: one timeout serves both the query client and the build client.
    pub fn connect_to(
        endpoint: &str,
        instance: &str,
        principal: &str,
        secret: String,
        request_timeout: Duration,
    ) -> Result<Self> {
        let address = endpoint
            .trim_start_matches("http://")
            .trim_end_matches('/')
            .parse()
            .map_err(|error| format!("RRFlow endpoint must be ADDRESS:PORT: {error}"))?;
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .map_err(|error| error.to_string())?;
        let instance_id = id(instance)?;
        let config = ClientConfig {
            request_timeout,
            ..ClientConfig::default()
        };
        let client = runtime
            .block_on(async { RrdClient::connect_local(address, instance_id.clone(), config) })
            .map_err(|error| error.to_string())?;
        let build_config = ClientConfig {
            request_timeout,
            ..ClientConfig::default()
        };
        let build_client = runtime
            .block_on(async { RrdClient::connect_local(address, instance_id, build_config) })
            .map_err(|error| error.to_string())?;
        let started = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| error.to_string())?
            .as_nanos();
        let run = format!("lab-{}-{started}", std::process::id());
        let options = RequestOptions::mutation(
            &format!("{run}-session-request"),
            &format!("{run}-session-operation"),
            &format!("{run}-session"),
        )
        .map_err(|error| error.to_string())?;
        let principal = id(principal)?;
        let session = open_session(&runtime, &client, &principal, &secret, options)?;
        let scope = format!("instance:{}", client.instance_id());
        Ok(Self {
            runtime,
            client,
            build_client,
            principal,
            secret,
            lease: RwLock::new(Lease {
                session,
                opened_at: Instant::now(),
            }),
            scope,
            run,
            counter: AtomicU64::new(0),
            prepared: OnceLock::new(),
            access_paths: Mutex::new(BTreeSet::new()),
        })
    }

    /// Adds `table` to the schema as an open relational table, then stores `records`, each
    /// one `put` mutation, in commits bounded by the server's request size and claim count.
    pub fn import_records(&self, table: &str, migration: &str, records: &[Value]) -> Result<()> {
        let session = self.session()?;
        let installed = self
            .runtime
            .block_on(self.client.read_data_schema(
                &session,
                ReadDataSchema {
                    scope: self.scope.clone(),
                    max_storage_keys: 4_096,
                },
                self.read_options("schema")?,
            ))
            .map_err(|error| error.to_string())?;
        let mut registry =
            serde_json::to_value(installed.schema.ok_or("RRFlow returned no data schema")?)
                .map_err(|error| error.to_string())?;
        registry["revision"] = json!(registry["revision"].as_u64().unwrap_or(0) + 1);
        registry["migration"] = json!(migration);
        registry["tables"][table] = json!({"model": "relational", "mode": "strict"});
        registry["records"][table] = json!({
            "properties": {},
            "allow_additional_properties": true,
            "unique_properties": []
        });
        self.commit(decode(
            json!([{"mutation": "put_schema", "registry": registry}]),
        )?)?;
        let mut mutations: Vec<Vec<TransactionMutation>> = Vec::with_capacity(records.len());
        let mut sizes = Vec::with_capacity(records.len());
        for record in records {
            let one: Vec<TransactionMutation> = decode(json!([record]))?;
            sizes.push(
                serde_json::to_vec(&one)
                    .map_err(|error| error.to_string())?
                    .len(),
            );
            mutations.push(one);
        }
        for group in split_by_size(sizes, MAX_COMMIT_BYTES, MAX_TRANSACTION_CLAIMS) {
            self.commit(mutations[group].iter().flatten().cloned().collect())?;
        }
        Ok(())
    }

    fn key(&self, label: &str) -> String {
        let serial = self.counter.fetch_add(1, Ordering::Relaxed);
        format!("{}-{label}-{serial}", self.run)
    }

    fn read_options(&self, label: &str) -> Result<RequestOptions> {
        let key = self.key(label);
        RequestOptions::read(&format!("{key}-request"), &format!("{key}-operation"))
            .map_err(|error| error.to_string())
    }

    fn mutation_options(&self, label: &str) -> Result<RequestOptions> {
        let key = self.key(label);
        RequestOptions::mutation(&format!("{key}-request"), &format!("{key}-operation"), &key)
            .map_err(|error| error.to_string())
    }

    /// A long ingest or search run can outlive one lease.
    fn session(&self) -> Result<Session> {
        {
            let lease = self.lease.read().map_err(|_| "session lock poisoned")?;
            if lease.opened_at.elapsed() < SESSION_REPLACE_AFTER {
                return Ok(lease.session.clone());
            }
        }
        let mut lease = self.lease.write().map_err(|_| "session lock poisoned")?;
        if lease.opened_at.elapsed() >= SESSION_REPLACE_AFTER {
            lease.session = open_session(
                &self.runtime,
                &self.client,
                &self.principal,
                &self.secret,
                self.mutation_options("session")?,
            )?;
            lease.opened_at = Instant::now();
        }
        Ok(lease.session.clone())
    }

    fn prepared(&self) -> Result<&Prepared> {
        self.prepared
            .get()
            .ok_or_else(|| "collection is not prepared".into())
    }

    fn commit(&self, mutations: Vec<TransactionMutation>) -> Result<()> {
        let session = self.session()?;
        let begin = self.mutation_options("begin")?;
        let transaction = self
            .runtime
            .block_on(self.client.begin_transaction(
                &session,
                BeginTransaction {
                    scope: id("data")?,
                    timeout_ms: 120_000,
                },
                begin,
            ))
            .map_err(|error| error.to_string())?;
        let mut options = self.mutation_options("commit")?;
        options.deadline_unix_ms = Some(transaction.expires_at_unix_ms);
        self.runtime
            .block_on(self.client.commit_transaction(
                &session,
                &transaction.transaction_id,
                CommitTransaction {
                    operation_sha256: transaction_operation_sha256(&mutations),
                    mutations,
                },
                options,
            ))
            .map_err(|error| error.to_string())?;
        Ok(())
    }
}

impl SystemAdapter for RrflowAdapter {
    fn observed_version(&self) -> Result<String> {
        let capabilities = self
            .runtime
            .block_on(self.client.capabilities())
            .map_err(|error| error.to_string())?;
        Ok(format!(
            "{} {}",
            capabilities.implementation.as_str(),
            capabilities.implementation_version
        ))
    }

    fn prepare(&self, spec: &CollectionSpec<'_>) -> Result<()> {
        let session = self.session()?;
        let collection = spec.name.replace('_', "-");
        let prepared = Prepared {
            record_table: format!("{}_point", spec.name.replace('-', "_")),
            vector_table: format!("{}_vector", spec.name.replace('-', "_")),
            collection: collection.clone(),
            index: spec.index.clone(),
        };
        let installed = self
            .runtime
            .block_on(self.client.read_data_schema(
                &session,
                ReadDataSchema {
                    scope: self.scope.clone(),
                    max_storage_keys: 4_096,
                },
                self.read_options("schema")?,
            ))
            .map_err(|error| error.to_string())?;
        let mut registry =
            serde_json::to_value(installed.schema.ok_or("RRFlow returned no data schema")?)
                .map_err(|error| error.to_string())?;
        registry["revision"] = json!(registry["revision"].as_u64().unwrap_or(0) + 1);
        registry["migration"] = json!(format!("add comparison lab table {collection}"));
        registry["tables"][&prepared.record_table] =
            json!({"model": "relational", "mode": "strict"});
        registry["tables"][&prepared.vector_table] =
            json!({"model": "vector", "mode": "schemaless"});
        registry["records"][&prepared.record_table] = json!({
            "properties": {},
            "allow_additional_properties": true,
            "unique_properties": []
        });
        let metric = match spec.metric {
            LabMetric::Cosine => "cosine",
            LabMetric::Dot => "dot",
            LabMetric::Euclidean => "euclidean",
        };
        let ensure = decode(json!({
            "scope": self.scope,
            "collection_id": collection,
            "vectors": [{
                "name": VECTOR_NAME,
                "field": VECTOR_FIELD,
                "kind": "dense",
                "dimensions": spec.dimensions,
                "metric": metric,
                "memory_tier": memory_tier(spec.index)
            }]
        }))?;
        self.runtime
            .block_on(self.client.ensure_vector_collection(
                &session,
                ensure,
                self.mutation_options("collection")?,
            ))
            .map_err(|error| error.to_string())?;
        if matches!(spec.index, IndexSpec::Hnsw { .. }) {
            self.runtime
                .block_on(self.client.ensure_vector_payload_index(
                    &session,
                    decode(payload_index_request(&self.scope, &collection))?,
                    self.mutation_options("payload-index")?,
                ))
                .map_err(|error| error.to_string())?;
        }
        self.commit(decode(
            json!([{"mutation": "put_schema", "registry": registry}]),
        )?)?;
        let _ = self.prepared.set(prepared);
        Ok(())
    }

    fn ingest(&self, batch: &PointBatch<'_>) -> Result<()> {
        let prepared = self.prepared()?;
        let tables = (
            prepared.record_table.as_str(),
            prepared.vector_table.as_str(),
            prepared.collection.as_str(),
        );
        let mut points = Vec::with_capacity(batch.ids.len());
        let mut sizes = Vec::with_capacity(batch.ids.len());
        for index in 0..batch.ids.len() {
            let one = PointBatch {
                ids: &batch.ids[index..=index],
                vectors: batch.vector(index),
                categories: &batch.categories[index..=index],
                dimensions: batch.dimensions,
            };
            let mutations: Vec<TransactionMutation> = decode(point_mutations(tables, &one))?;
            sizes.push(
                serde_json::to_vec(&mutations)
                    .map_err(|error| error.to_string())?
                    .len(),
            );
            points.push(mutations);
        }
        let per_commit = MAX_TRANSACTION_CLAIMS / MUTATIONS_PER_POINT;
        for group in split_by_size(sizes, MAX_COMMIT_BYTES, per_commit) {
            self.commit(points[group].iter().flatten().cloned().collect())?;
        }
        Ok(())
    }

    fn finish_ingest(&self, _deadline: Instant) -> Result<()> {
        let prepared = self.prepared()?;
        if let IndexSpec::Hnsw { m, ef_construction } = &prepared.index {
            let session = self.session()?;
            let built: EnsureVectorIndexResult = self
                .runtime
                .block_on(self.build_client.ensure_vector_index(
                    &session,
                    decode(hnsw_index_request(
                        &self.scope,
                        &prepared.collection,
                        *m,
                        *ef_construction,
                    ))?,
                    self.mutation_options("graph")?,
                ))
                .map_err(|error| error.to_string())?;
            eprintln!("graph built over {} vectors", built.index.indexed_vectors);
            return Ok(());
        }
        let IndexSpec::TurboQuant { bits, seed } = &prepared.index else {
            return Ok(());
        };
        // A quantized generation declares no filter property, so it cannot cover a filter.
        // Filtered searches then report the engine's refusal, as any caller would see, rather
        // than quietly falling back to an exact scan.
        let session = self.session()?;
        let built = self
            .runtime
            .block_on(self.client.build_vector_quantization(
                &session,
                decode(json!({
                    "scope": self.scope,
                    "collection_id": prepared.collection,
                    "vector_name": VECTOR_NAME,
                    "method": {"method": "turbo_quant", "bits": bits, "seed": seed},
                    "max_storage_keys": MAX_VECTOR_STORAGE_KEYS
                }))?,
                self.mutation_options("quantize")?,
            ))
            .map_err(|error| error.to_string())?;
        self.runtime
            .block_on(self.client.activate_vector_quantization(
                &session,
                decode(json!({
                    "scope": self.scope,
                    "artifact_id": built.artifact.artifact_id,
                    "generation": built.artifact.generation
                }))?,
                self.mutation_options("activate")?,
            ))
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    fn search(&self, request: &SearchRequest<'_>) -> Result<Vec<u64>> {
        let prepared = self.prepared()?;
        let session = self.session()?;
        let mut body = json!({
            "scope": self.scope,
            "valid_at": VALID_AT,
            "collection_id": prepared.collection,
            "vector_name": VECTOR_NAME,
            "query": {"kind": "dense", "values": request.vector},
            "top_k": request.k,
            "mode": search_mode(
                &prepared.index,
                request.setting.ef,
                request.setting.exact_rerank,
                request.k
            ),
            "max_storage_keys": MAX_VECTOR_STORAGE_KEYS
        });
        if let Some(category) = request.category {
            body["filter"] = json!({
                "kind": "condition",
                "condition": {
                    "property": "category",
                    "operator": {"operator": "equals", "value": {"type": "unsigned", "value": category}}
                }
            });
        }
        let result = self
            .runtime
            .block_on(self.client.search_vectors(
                &session,
                decode(body)?,
                self.read_options("search")?,
            ))
            .map_err(|error| error.to_string())?;
        if let Ok(mut paths) = self.access_paths.lock() {
            paths.insert(result.access_path.as_str().to_string());
        }
        result
            .hits
            .iter()
            .map(|hit| parse_point(hit.reference.id.as_str()))
            .collect()
    }

    fn index_evidence(&self) -> Result<Option<String>> {
        let paths = self.access_paths.lock().map_err(|_| "lock poisoned")?;
        Ok(Some(format!(
            "access paths the engine reported: {}",
            paths.iter().cloned().collect::<Vec<_>>().join(", ")
        )))
    }

    fn teardown(&self) -> Result<()> {
        // RRFlow retains history by design and the lab uses a fresh estate per run,
        // so there is nothing to remove here.
        Ok(())
    }
}

#[cfg(test)]
mod tests;
