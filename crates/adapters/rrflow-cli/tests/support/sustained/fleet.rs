//! Independent installed estates, sharing only the bounded workload oracle.
use super::super::process;
use super::*;
#[path = "fleet_campaign.rs"]
mod campaign;
pub(super) use campaign::{execute, Participation};
use rrd_contract::{ErrorCode, InspectStorageCursor, StorageCursorAction};

struct Member {
    estate: Estate,
    child: Option<process::Process>,
    work: Workload,
    original_receipts: Digest,
    final_writes: Vec<(Prepared, Digest)>,
}
impl Member {
    async fn create(ordinal: u64, clock: Instant) -> DriverResult<Self> {
        let estate = Estate::install("fresh");
        let (child, client, _) = estate.start_for(clock, Duration::from_secs(600));
        let credential = rrd_engine::RrdEngine::read_installed_api_key(&estate.project)
            .map_err(|e| e.to_string())?;
        let name = format!("fleet-session-{ordinal}");
        let session = client
            .create_session(
                credential.principal_id.clone(),
                credential.credential(),
                CreateSession {
                    limits: SessionLimits {
                        idle_timeout_ms: 300_000,
                        absolute_timeout_ms: 600_000,
                        max_open_transactions: 4,
                    },
                },
                RequestOptions::mutation(&name, &name, &name).map_err(|e| e.to_string())?,
            )
            .await
            .map_err(|e| e.to_string())?;
        let work = Workload {
            client,
            session,
            valid_at: estate.at,
            dimensions: 16,
            seed: 73 + ordinal,
            records: 8,
            read_distribution: ReadKeyDistribution::Uniform {},
            edges_per_record: 3,
            vectors_per_record: 2,
            payload_bytes: 1024,
            query_budget: serde_json::from_value(estate.plan["configuration"]["query"].clone())
                .map_err(|e| e.to_string())?,
        };
        work.initialize().await?;
        let mut original_receipts = digest("c07g.fleet-seed-receipts", &ordinal)?;
        for row in 0..work.records {
            let operation = work.seed_record(row)?;
            let receipt = work.execute(&operation).await?;
            if receipt.idempotent_replay {
                return Err("fresh estate replayed foreign seed state".into());
            }
            original_receipts = digest(
                "c07g.fleet-seed-receipts",
                &(original_receipts, row, receipt.receipt_sha256),
            )?;
        }
        // A separate pass evaluates generated record/relation/vector content.
        for row in 0..work.records {
            work.verify(&work.seed_record(row)?).await?;
        }
        Ok(Self {
            estate,
            child: Some(child),
            work,
            original_receipts,
            final_writes: Vec::new(),
        })
    }
    async fn reopen_verify(
        &mut self,
        ordinal: u64,
        clock: Instant,
    ) -> DriverResult<serde_json::Value> {
        let killed = super::super::kill(self.child.take().ok_or("missing fleet child")?, clock);
        let (child, client, identity) = self.estate.start_for(clock, Duration::from_secs(300));
        self.child = Some(child);
        self.work.client = client;
        let mut replayed = digest("c07g.fleet-seed-receipts", &ordinal)?;
        for row in 0..self.work.records {
            let operation = self.work.seed_record(row)?;
            self.work.verify(&operation).await?; // Before any retry can rewrite.
            let receipt = self.work.execute(&operation).await?;
            if !receipt.idempotent_replay {
                return Err("reopen duplicated a seed effect".into());
            }
            replayed = digest(
                "c07g.fleet-seed-receipts",
                &(replayed, row, receipt.receipt_sha256),
            )?;
        }
        if replayed != self.original_receipts {
            return Err("fleet seed receipts changed across restart".into());
        }
        for (operation, original) in &self.final_writes {
            self.work.verify(operation).await?;
            let receipt = self.work.execute(operation).await?;
            if !receipt.idempotent_replay || receipt.receipt_sha256 != *original {
                return Err("fleet write replay changed receipt/effect".into());
            }
        }
        Ok(
            serde_json::json!({"estate":ordinal,"killed":killed,"reopened":identity,
            "seed_receipts_sha256":replayed,"seed_records":self.work.records,"working_slots":self.final_writes.len()}),
        )
    }
}

async fn deny_foreign_sessions(members: &[Member]) -> DriverResult<()> {
    for (target, member) in members.iter().enumerate() {
        let foreign = &members[(target + 1) % members.len()].work.session;
        let response = member
            .work
            .client
            .read_storage_diagnostics(
                foreign,
                ReadStorageDiagnostics {
                    scope: format!("instance:{}", member.work.client.instance_id()),
                },
                RequestOptions::read("fleet-foreign", "fleet-foreign").unwrap(),
            )
            .await;
        match response {
            Err(rrd_client::Error::Api { error, .. })
                if matches!(
                    error.code,
                    ErrorCode::Unauthenticated | ErrorCode::PermissionDenied | ErrorCode::NotFound
                ) => {}
            _ => {
                return Err(
                    "foreign session was not explicitly denied by destination estate".into(),
                )
            }
        }
    }
    Ok(())
}

async fn concurrent_frame(members: &mut [Member], generation: u64) -> DriverResult<()> {
    let mut tasks = JoinSet::new();
    // Exactly four outstanding clients per estate: one mixed write and three
    // different read/cancel operations, no optimistic-write rejection hidden.
    for (estate, member) in members.iter().enumerate() {
        for (client, kind) in [
            StorageLoadKind::MixedWrite,
            StorageLoadKind::PointRead,
            StorageLoadKind::RangeRead,
            StorageLoadKind::Maintenance,
        ]
        .into_iter()
        .enumerate()
        {
            let work = member.work.clone();
            let name = format!("fleet-{estate}-{generation}-{client}");
            let operation = if kind == StorageLoadKind::MixedWrite {
                work.prepare_load(
                    name,
                    WriteTransition {
                        slot: 0,
                        generation,
                    },
                )?
            } else {
                work.prepare(name, kind, generation)?
            };
            tasks.spawn(async move {
                let result =
                    tokio::time::timeout(Duration::from_secs(30), work.execute(&operation))
                        .await
                        .map_err(|_| "fleet operation exceeded deadline".to_owned())?;
                Ok::<_, String>((estate, operation, result?))
            });
        }
    }
    while let Some(completed) = tasks.join_next().await {
        let (estate, operation, receipt) = completed.map_err(|e| e.to_string())??;
        if receipt.idempotent_replay {
            return Err("fresh fleet request unexpectedly replayed".into());
        }
        members[estate].work.verify(&operation).await?;
        if operation.kind == StorageLoadKind::MixedWrite {
            members[estate].final_writes.clear();
            members[estate]
                .final_writes
                .push((operation, receipt.receipt_sha256));
        }
    }
    Ok(())
}

#[test]
#[ignore = "explicit native independent-estate correctness; not a capacity or soak qualification"]
fn installed_two_estate_isolation_under_concurrent_load() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let clock=Instant::now();
        let mut members=Vec::new();
        for ordinal in 0..2 {members.push(Member::create(ordinal,clock).await.unwrap());}
        assert_ne!(members[0].child.as_ref().unwrap().id(),members[1].child.as_ref().unwrap().id());
        deny_foreign_sessions(&members).await.unwrap();
        let mut readers=Vec::new();
        for member in &members {readers.push(reader::HeldReader::open(&member.work,0,clock).await.unwrap());}
        // Insert, update, retire and recreate identical identities with different
        // per-estate payloads. The canonical oracle checks all model families.
        for generation in 0..4 {concurrent_frame(&mut members,generation).await.unwrap();}
        for (member,reader) in members.iter().zip(readers) {
            let evidence=reader.finish(&member.work,clock).await.unwrap();
            println!("fleet_reader_ownership={}",serde_json::to_string(&evidence).unwrap());
        }
        // Both directions must still deny after concurrent canonical mutations.
        deny_foreign_sessions(&members).await.unwrap();
        for (ordinal,member) in members.iter_mut().enumerate() {
            let receipt=member.reopen_verify(ordinal as u64,clock).await.unwrap();
            println!("fleet_reopen_readback={receipt}");
        }
        deny_foreign_sessions(&members).await.unwrap();
        // A cursor from the old process cannot be silently recovered/recreated.
        for member in &members {
            assert!(member.work.client.inspect_storage_cursor(&member.work.session,InspectStorageCursor {
                scope:format!("instance:{}",member.work.client.instance_id()),
                cursor_id:rrd_contract::CorrelationId::new("capacity-reader-0").unwrap(),
                action:StorageCursorAction::Observe {},
            },RequestOptions::read("fleet-expired-reader","fleet-expired-reader").unwrap()).await.is_err());
        }
        println!("installed_fleet_isolation={}",serde_json::json!({"estates":2,"clients_per_estate":4,
            "write_transitions":4,"cross_estate_session_denials":6,"ordinary_primary":true,"capacity_qualification":false}));
        for mut member in members {super::super::kill(member.child.take().unwrap(),clock);}
    });
}
