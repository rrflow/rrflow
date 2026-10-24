//! C-07c's production-binary response-loss/reopen history. Internal storage
//! barriers, the 100-cycle qualification corpus and power loss are separate.
use super::*;
use rrd_contract::{AbortTransaction, CommitReceipt, CorrelationId, TransactionLease};
use rrflow_eval::storage::{
    digest, AcknowledgementLedger, Attempt, AttemptOutcome, Digest, ExpectedOperation,
    ExpectedResolution, Id, Readback,
};
use std::time::Instant;

#[path = "response_loss.rs"]
mod response_loss;

fn id(value: &str) -> Id {
    value.to_owned().try_into().unwrap()
}

fn options(value: &str) -> RequestOptions {
    RequestOptions::new(value, value, Some(value), Some(u64::MAX)).unwrap()
}

fn client(address: &str, instance: CanonicalId) -> RrdClient {
    RrdClient::connect_local(
        address.parse().unwrap(),
        instance,
        ClientConfig {
            request_timeout: Duration::from_secs(60),
            max_attempts: 1,
            ..ClientConfig::default()
        },
    )
    .unwrap()
}

fn request(name: &str, at: u64) -> CommitTransaction {
    let mutations = (0..2)
        .map(|index| TransactionMutation::AssertClaim {
            subject: CanonicalId::new(format!("history-{name}-{index}")).unwrap(),
            predicate: CanonicalId::new("history").unwrap(),
            object: format!("requested-{name}-{index}"),
            valid_from: at,
            tx_time: at,
            producer: CanonicalId::new("history-oracle").unwrap(),
            confidence: None,
        })
        .collect::<Vec<_>>();
    CommitTransaction {
        operation_sha256: transaction_operation_sha256(&mutations),
        mutations,
    }
}

fn expected_values(name: &str) -> BTreeMap<String, String> {
    (0..2)
        .map(|index| {
            (
                format!("claim:history-{name}-{index}:history"),
                format!("requested-{name}-{index}"),
            )
        })
        .collect()
}

fn receipt_digest(receipt: &CommitReceipt) -> Digest {
    let mut stable = receipt.clone();
    stable.idempotent_replay = false;
    digest("c07c.receipt", &stable).unwrap()
}

fn attempt(
    ledger: &mut AcknowledgementLedger,
    name: &str,
    request: &CommitTransaction,
    start: u64,
    clock: Instant,
    outcome: AttemptOutcome,
) {
    ledger.attempts.push(Attempt {
        attempt_id: id(&format!("attempt-{}", ledger.attempts.len())),
        logical_id: id(name),
        request_sha256: digest("c07c.request", request).unwrap(),
        started_ns: start,
        finished_ns: clock.elapsed().as_nanos().try_into().unwrap(),
        outcome,
    });
}

async fn begin(client: &RrdClient, session: &rrd_client::Session, name: &str) -> TransactionLease {
    client
        .begin_transaction(
            session,
            BeginTransaction {
                scope: CanonicalId::new("claims").unwrap(),
                timeout_ms: 600_000,
            },
            options(&format!("history-begin-{name}")),
        )
        .await
        .unwrap()
}

async fn commit(
    client: &RrdClient,
    session: &rrd_client::Session,
    transaction: &CorrelationId,
    name: &str,
    request: &CommitTransaction,
) -> rrd_client::Result<CommitReceipt> {
    client
        .commit_transaction(
            session,
            transaction,
            request.clone(),
            options(&format!("history-commit-{name}")),
        )
        .await
}

async fn abort(
    client: &RrdClient,
    session: &rrd_client::Session,
    lease: &TransactionLease,
    name: &str,
) {
    client
        .abort_transaction(
            session,
            &lease.transaction_id,
            AbortTransaction {},
            options(&format!("history-abort-{name}")),
        )
        .await
        .unwrap();
}

pub(super) fn run(
    project: &Path,
    mode: &str,
    at: u64,
    build: &serde_json::Value,
    captured: &mut Vec<u8>,
) {
    let locator = RrdEngine::read_project_locator(project).unwrap();
    let credential = RrdEngine::read_installed_api_key(project).unwrap();
    let clock = Instant::now();
    let mut ledger = AcknowledgementLedger::default();
    // Freeze the oracle before any request; it never copies engine results.
    let requests = ["a", "b", "c", "d"].map(|name| request(name, at));
    for (name, request) in ["a", "b", "c", "d"].iter().zip(&requests) {
        ledger.expected.push(ExpectedOperation {
            logical_id: id(name),
            request_sha256: digest("c07c.request", request).unwrap(),
            effect_sha256: digest("c07c.effect", &expected_values(name)).unwrap(),
            resolution: if *name == "c" {
                ExpectedResolution::Absent
            } else {
                ExpectedResolution::Present
            },
        });
    }
    let expected_digest = digest("c07c.expected", &ledger.expected).unwrap();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let (mut process, address) = RunningEngine::start(project, captured);
        let direct = client(&address, locator.identity.instance_id.clone());
        let session = direct.create_session(credential.principal_id.clone(), credential.credential(),
            CreateSession { limits: SessionLimits { idle_timeout_ms: 600_000, absolute_timeout_ms: 3_600_000, max_open_transactions: 2 } },
            options("history-session")).await.unwrap();
        let a = begin(&direct, &session, "a").await;
        let started = clock.elapsed().as_nanos().try_into().unwrap();
        let accepted_a = commit(&direct, &session, &a.transaction_id, "a", &requests[0]).await.unwrap();
        assert_eq!(accepted_a.first_runtime_cursor, Some(4));
        assert_eq!(accepted_a.last_runtime_cursor, Some(5));
        attempt(&mut ledger, "a", &requests[0], started, clock, AttemptOutcome::Acknowledged { receipt_sha256: receipt_digest(&accepted_a) });

        let stale = begin(&direct, &session, "c").await;
        let b = begin(&direct, &session, "b").await;
        let relay = response_loss::ResponseLoss::start(address.parse().unwrap());
        let relayed = client(&relay.address.to_string(), locator.identity.instance_id.clone());
        let started = clock.elapsed().as_nanos().try_into().unwrap();
        let lost = commit(&relayed, &session, &b.transaction_id, "b", &requests[1]).await.unwrap_err();
        assert!(matches!(lost, rrd_client::Error::Transport(_)), "suppressed response must be transport uncertainty");
        relay.finish();
        attempt(&mut ledger, "b", &requests[1], started, clock, AttemptOutcome::Uncertain { reason: id("response-lost") });

        let started = clock.elapsed().as_nanos().try_into().unwrap();
        let rejected = commit(&direct, &session, &stale.transaction_id, "c", &requests[2]).await.unwrap_err();
        assert!(matches!(rejected, rrd_client::Error::Api { error, .. } if error.code == ErrorCode::Conflict));
        attempt(&mut ledger, "c", &requests[2], started, clock, AttemptOutcome::Rejected { reason: id("stale-cursor") });
        abort(&direct, &session, &stale, "c").await;
        // Filling both slots after explicit abort proves rejected work can be
        // cleaned up and its quota reused. A subsequent valid write must work.
        let d = begin(&direct, &session, "d").await;
        let spare = begin(&direct, &session, "spare").await;
        abort(&direct, &session, &spare, "spare").await;
        let started = clock.elapsed().as_nanos().try_into().unwrap();
        let accepted_d = commit(&direct, &session, &d.transaction_id, "d", &requests[3]).await.unwrap();
        assert_eq!(accepted_d.first_runtime_cursor, Some(8));
        assert_eq!(accepted_d.last_runtime_cursor, Some(9));
        attempt(&mut ledger, "d", &requests[3], started, clock, AttemptOutcome::Acknowledged { receipt_sha256: receipt_digest(&accepted_d) });
        let mut stable_receipts = BTreeMap::from([("a", receipt_digest(&accepted_a)), ("d", receipt_digest(&accepted_d))]);

        for cycle in 0..3 {
            process.terminate(captured);
            let (restarted, address) = RunningEngine::start(project, captured);
            process = restarted;
            let direct = client(&address, locator.identity.instance_id.clone());
            for (name, lease, request, first, last) in [
                ("a", &a, &requests[0], 4, 5), ("b", &b, &requests[1], 6, 7), ("d", &d, &requests[3], 8, 9),
            ] {
                let started = clock.elapsed().as_nanos().try_into().unwrap();
                let receipt = commit(&direct, &session, &lease.transaction_id, name, request).await.unwrap();
                assert!(receipt.idempotent_replay);
                assert_eq!(receipt.mutation_count, 2);
                assert_eq!(receipt.first_runtime_cursor, Some(first));
                assert_eq!(receipt.last_runtime_cursor, Some(last));
                let receipt_hash = receipt_digest(&receipt);
                if let Some(previous) = stable_receipts.insert(name, receipt_hash.clone()) { assert_eq!(previous, receipt_hash); }
                attempt(&mut ledger, name, request, started, clock, AttemptOutcome::Acknowledged { receipt_sha256: receipt_hash });
            }
            let query = direct.execute_query(&session, ExecuteQuery {
                scope: format!("instance:{}", direct.instance_id()),
                query: format!("FROM claim:history AT VALID {at} KNOWN HEAD PROJECT subject, object EXPLAIN CONTRACT"),
                parameters: BTreeMap::new(), budget: QueryBudget {
                    max_storage_keys: 75_000, max_rows: 768, max_output_bytes: 393_216,
                    max_batch_rows: 192, max_memory_bytes: 33_554_432, max_spill_bytes: 67_108_864,
                    max_elapsed_ms: 20_000,
                },
            }, RequestOptions::read(&format!("history-read-{cycle}"), &format!("history-read-{cycle}")).unwrap()).await.unwrap();
            assert_eq!(query.rows.len(), 6);
            assert_eq!(query.known_at_cursor, 9, "exact retries cannot append duplicate versions");
            assert!(!query.execution.truncated);
            let mut observed = BTreeMap::new();
            for row in query.rows {
                let QueryValue::String(value) = &row.values["object"] else { panic!("history value type"); };
                assert!(observed.insert(row.identity, value.clone()).is_none());
            }
            let all_expected = ["a", "b", "d"].into_iter().flat_map(expected_values).collect::<BTreeMap<_, _>>();
            assert_eq!(observed, all_expected, "recovered atomic effects differ from predeclared values");
            ledger.readback.clear();
            for (name, request) in [("a", &requests[0]), ("b", &requests[1]), ("d", &requests[3])] {
                let values = observed.iter().filter(|(key, _)| key.starts_with(&format!("claim:history-{name}-")))
                    .map(|(key, value)| (key.clone(), value.clone())).collect::<BTreeMap<_, _>>();
                ledger.readback.push(Readback::Present {
                    logical_id: id(name), request_sha256: digest("c07c.request", request).unwrap(),
                    effect_sha256: digest("c07c.effect", &values).unwrap(), receipt_sha256: stable_receipts[name].clone(),
                    applications: 1.try_into().unwrap(),
                });
            }
            ledger.readback.push(Readback::Absent { logical_id: id("c") });
            assert!(ledger.reconcile(32, 600_000_000_000).unwrap().violations.is_empty());
        }
        process.stop(captured);
    });
    assert!(!String::from_utf8_lossy(captured).contains(credential.credential()));
    let report = ledger.reconcile(32, 600_000_000_000).unwrap();
    assert_eq!(report.attempts, 13);
    assert_eq!(report.acknowledged, 11);
    assert_eq!(report.rejected, 1);
    assert_eq!(report.uncertain, 1);
    assert_eq!(report.unresolved, 0);
    assert!(report.violations.is_empty());
    println!(
        "native_storage_history={}",
        serde_json::json!({
            "mode": mode, "build": {
                "executable_sha256": build["executable_sha256"],
                "metadata_sha256": build["metadata_sha256"],
                "public_contract_sha256": build["public_contract_sha256"],
                "signal_catalogue_sha256": build["signal_catalogue_sha256"],
            }, "forced_reopens": 3,
            "expected_sha256": expected_digest, "ledger": ledger, "report": report,
        })
    );
}
