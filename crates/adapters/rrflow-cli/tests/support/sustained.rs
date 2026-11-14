//! Ordinary installed campaign. Explicit environment admission keeps a long
//! campaign out of ordinary correctness CI; failure artifacts are never green.
use super::Estate;
use rrd_client::RequestOptions;
use rrd_contract::{CloseSession, CreateSession, ReadStorageDiagnostics, SessionLimits};
use rrflow_qualification_storage::*;
use std::{
    io::Read,
    path::PathBuf,
    time::{Duration, Instant},
};
use tokio::task::JoinSet;
#[path = "sustained/cold.rs"]
mod cold;
#[path = "sustained/corpus.rs"]
mod corpus;
#[path = "sustained/damage.rs"]
mod damage;
#[path = "sustained/fleet.rs"]
mod fleet;
#[path = "sustained/lifecycle.rs"]
mod lifecycle;
#[path = "sustained/management.rs"]
mod management;
#[path = "sustained/native.rs"]
mod native;
#[path = "sustained/native_worker.rs"]
mod native_worker;
#[path = "sustained/reader.rs"]
mod reader;
#[path = "sustained/resource_window.rs"]
mod resource_window;
#[path = "sustained/sample_task.rs"]
mod sample_task;
#[path = "sustained/sample_trace.rs"]
mod sample_trace;
#[path = "sustained/workload.rs"]
mod workload;
use workload::{Prepared, Workload};

type DriverResult<T> = std::result::Result<T, String>;

/// Backup overlaps the first mixed write only when the campaign requests it.
fn backup_due(campaign: &StorageCampaign, phase: LoadPhase, sequence: u64) -> bool {
    phase == LoadPhase::Measured
        && sequence == 14
        && campaign
            .required_lanes
            .contains(&CapacityLane::BackupUnderWrite)
}

fn retain_failed_lifecycle<T>(
    estate: &super::owned_directory::OwnedDirectory,
    result: DriverResult<T>,
) -> DriverResult<T> {
    if result.is_err() {
        eprintln!(
            "native_lifecycle_failure_estate={}",
            serde_json::json!({"path": estate.retain(), "private_state": true})
        );
    }
    result
}

#[test]
fn lifecycle_failure_retains_its_owned_estate_but_success_cleans_it() {
    let parent = tempfile::tempdir().unwrap();
    let success =
        super::owned_directory::OwnedDirectory::new(tempfile::tempdir_in(parent.path()).unwrap());
    let success_path = success.path().to_owned();
    assert_eq!(
        retain_failed_lifecycle(&success, Ok::<_, String>(7)).unwrap(),
        7
    );
    drop(success);
    assert!(!success_path.exists());

    let failed =
        super::owned_directory::OwnedDirectory::new(tempfile::tempdir_in(parent.path()).unwrap());
    let failed_path = failed.path().to_owned();
    std::fs::write(failed_path.join("sentinel"), b"retained").unwrap();
    assert_eq!(
        retain_failed_lifecycle::<()>(&failed, Err("fixture-failure".into())).unwrap_err(),
        "fixture-failure"
    );
    drop(failed);
    assert_eq!(
        std::fs::read(failed_path.join("sentinel")).unwrap(),
        b"retained"
    );
}

fn evidence_id(value: impl Into<String>) -> Id {
    value.into().try_into().unwrap()
}
fn ns(clock: Instant) -> u64 {
    clock
        .elapsed()
        .as_nanos()
        .try_into()
        .expect("bounded campaign clock")
}
fn bounded_json<T: serde::de::DeserializeOwned>(path: &std::path::Path) -> DriverResult<T> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(MAX_DOCUMENT_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    decode(&bytes)
}

async fn session(
    estate: &Estate,
    client: rrd_client::RrdClient,
    seed: u64,
    campaign: &StorageCampaign,
    clients: u64,
    ordinal: u64,
) -> DriverResult<Workload> {
    let credential = rrd_engine::RrdEngine::read_installed_api_key(&estate.project)
        .map_err(|e| e.to_string())?;
    let name = format!("capacity-session-{ordinal}");
    let session = client
        .create_session(
            credential.principal_id.clone(),
            credential.credential(),
            CreateSession {
                limits: SessionLimits {
                    idle_timeout_ms: 600_000,
                    absolute_timeout_ms: 3_600_000,
                    max_open_transactions: clients
                        .try_into()
                        .map_err(|_| "client count overflow")?,
                },
            },
            RequestOptions::new(&name, &name, Some(&name), Some(u64::MAX)).unwrap(),
        )
        .await
        .map_err(|e| e.to_string())?;
    Ok(Workload {
        native_worker: Default::default(),
        client,
        session,
        valid_at: estate.at,
        dimensions: campaign
            .allocation
            .vector_dimensions
            .get()
            .try_into()
            .map_err(|_| "vector dimensions overflow")?,
        seed,
        records: campaign.allocation.records.get(),
        read_distribution: campaign.read_distribution.clone(),
        edges_per_record: campaign
            .allocation
            .edges
            .get()
            .div_ceil(campaign.allocation.records.get())
            .try_into()
            .map_err(|_| "edge fanout overflow")?,
        vectors_per_record: campaign
            .allocation
            .vectors
            .get()
            .div_ceil(campaign.allocation.records.get())
            .try_into()
            .map_err(|_| "vector fanout overflow")?,
        payload_bytes: campaign
            .allocation
            .logical_source_bytes
            .get()
            .div_ceil(campaign.allocation.records.get())
            .max(1024)
            .try_into()
            .map_err(|_| "source payload overflow")?,
        query_budget: serde_json::from_value(estate.plan["configuration"]["query"].clone())
            .map_err(|e| e.to_string())?,
    })
}

/// Rotate only between bounded segments, after every in-flight request has
/// joined. Closing the old lease prevents a day of accumulated live sessions.
async fn rotate_session(
    estate: &Estate,
    work: &mut Workload,
    campaign: &StorageCampaign,
    clients: u64,
    ordinal: &mut u64,
    opened: &mut Instant,
) -> DriverResult<()> {
    if opened.elapsed() < Duration::from_secs(120) {
        return Ok(());
    }
    let key = format!("capacity-close-{ordinal}");
    work.client
        .close_session(
            &work.session,
            CloseSession {},
            RequestOptions::new(&key, &key, Some(&key), Some(u64::MAX)).unwrap(),
        )
        .await
        .map_err(|e| e.to_string())?;
    *ordinal = ordinal.checked_add(1).ok_or("session ordinal overflow")?;
    *work = session(
        estate,
        work.client.clone(),
        work.seed,
        campaign,
        clients,
        *ordinal,
    )
    .await?;
    *opened = Instant::now();
    Ok(())
}

async fn sample(
    workload: &Workload,
    pid: u32,
    clock: Instant,
    allocation: &Option<NativeAllocationReceipt>,
) -> DriverResult<ResourceSample> {
    sample_with_identity(workload, pid, clock, allocation)
        .await
        .map(|(sample, _, _)| sample)
}

#[derive(Clone, Copy)]
struct SampleTiming {
    total_ns: u64,
    host_ns: u64,
    rpc_ns: u64,
    memory_ns: u64,
    enforcement_ns: u64,
}

async fn sample_with_identity(
    workload: &Workload,
    pid: u32,
    clock: Instant,
    allocation: &Option<NativeAllocationReceipt>,
) -> DriverResult<(ResourceSample, String, SampleTiming)> {
    let started = Instant::now();
    let rpc_started = Instant::now();
    let request_id = sample_trace::next_request_id().map_err(str::to_string)?;
    let snapshot = workload
        .client
        .read_storage_diagnostics(
            &workload.session,
            ReadStorageDiagnostics {
                scope: format!("instance:{}", workload.client.instance_id()),
            },
            RequestOptions::read(&request_id, &request_id).map_err(|e| e.to_string())?,
        )
        .await
        .map_err(|e| e.to_string())?;
    let rpc_ns = ns(rpc_started);
    let missing = |reason: &str| NativeMetric::Unavailable {
        reason: evidence_id(reason),
    };
    let native = workload.native_worker.acquire(pid, allocation).await?;
    // This is the parent-side completion time of the real native observation.
    let elapsed_ns = ns(clock);
    let host_ns = native.host_ns;
    let memory_ns = native.memory_ns;
    let enforcement_ns = native.enforcement_ns;
    let value = ResourceSample {
        elapsed_ns,
        pid,
        resident_bytes: NativeMetric::Observed {
            value: native.resident_bytes,
        },
        private_bytes: native.private_bytes,
        mapped_bytes: native.mapped_bytes,
        host_resident_bytes: NativeMetric::Observed {
            value: native.host_resident_bytes,
        },
        read_bytes: NativeMetric::Observed {
            value: native.read_bytes,
        },
        written_bytes: NativeMetric::Observed {
            value: native.written_bytes,
        },
        cpu_time_ns: NativeMetric::Observed {
            value: native.cpu_time_ns,
        },
        retained_bytes: match &snapshot.admission {
            Some(value) => NativeMetric::Observed {
                value: value
                    .retained_mutable_bytes
                    .checked_add(value.retained_projected_bytes)
                    .and_then(|bytes| bytes.checked_add(value.retained_source_bytes))
                    .and_then(|bytes| bytes.checked_add(value.native_transaction_bytes))
                    .ok_or("retained sum overflow")?,
            },
            None => missing("public-retained-gauge-unavailable"),
        },
        queue_requests: match &snapshot.admission {
            Some(value) => NativeMetric::Observed {
                value: value.outstanding_requests,
            },
            None => missing("public-queue-gauge-unavailable"),
        },
        maintenance_debt_segments: match &snapshot.rrflow_kv {
            Some(value) => NativeMetric::Observed {
                value: value.segment_set.compaction_debt_segments,
            },
            None => missing("public-debt-gauge-unavailable"),
        },
        allocation_enforcement: native.enforcement,
        allocation_receipt: allocation.clone(),
        storage: Some(snapshot),
    };
    let timing = SampleTiming {
        total_ns: ns(started),
        host_ns,
        rpc_ns,
        memory_ns,
        enforcement_ns,
    };
    sample_trace::record(sample_trace::Observation {
        request_id: &request_id,
        pid,
        elapsed_ns,
        total_ns: timing.total_ns,
        host_ns: timing.host_ns,
        rpc_ns: timing.rpc_ns,
        memory_ns: timing.memory_ns,
        enforcement_ns: timing.enforcement_ns,
    });
    Ok((value, request_id, timing))
}

struct ProgressContext<'a> {
    writer: &'a CampaignWriter,
    point: &'a OperatingPoint,
}

struct PointRun<'a> {
    campaign: &'a StorageCampaign,
    point: &'a OperatingPoint,
    workload: Workload,
    pid: u32,
    clock: Instant,
    allocation: Option<NativeAllocationReceipt>,
    generations: Vec<u64>,
    previous_resources: Vec<ResourceSample>,
    backup: management::ManagementTask<BackupEvidence>,
    sample_gate: Option<std::sync::Arc<sample_task::SampleGate>>,
}

#[path = "sustained/campaign.rs"]
mod campaign;
#[path = "sustained/capacity_workload.rs"]
mod capacity_workload;
#[path = "sustained/estate.rs"]
mod estate;
#[path = "sustained/point_run.rs"]
mod point_run;

use campaign::*;
use estate::*;
use point_run::*;
