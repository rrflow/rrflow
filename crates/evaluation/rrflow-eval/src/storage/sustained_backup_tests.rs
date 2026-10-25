//! Contract histories, not native backup or capacity measurements.
use super::*;

const TICK: u64 = 10_000_000;

fn segment(campaign: &StorageCampaign, first: u64, count: u64) -> SustainedSegment {
    let (_, mut segment) = tests::fixture();
    segment.campaign_sha256 = campaign.sha256().unwrap();
    segment.started_ns = first * TICK;
    segment.finished_ns = (first + count) * TICK;
    segment.resources[0].elapsed_ns = segment.started_ns;
    segment.attempts.clear();
    segment.result.cases.truncate(1);
    let case = &mut segment.result.cases[0];
    case.ledger = AcknowledgementLedger::default();
    case.status = CaseStatus::Executed;
    case.faults.clear();
    case.limitation = None;
    for sequence in first..first + count {
        let id: Id = format!("operation-{sequence}").try_into().unwrap();
        let request = digest("fixture-request", &sequence).unwrap();
        let effect = digest("fixture-effect", &sequence).unwrap();
        let receipt = digest("fixture-receipt", &sequence).unwrap();
        let scheduled = sequence * TICK;
        case.ledger.expected.push(ExpectedOperation {
            logical_id: id.clone(),
            request_sha256: request.clone(),
            effect_sha256: effect.clone(),
            resolution: ExpectedResolution::Present,
        });
        case.ledger.attempts.push(Attempt {
            attempt_id: id.clone(),
            logical_id: id.clone(),
            request_sha256: request.clone(),
            started_ns: scheduled + 1 - segment.started_ns,
            finished_ns: scheduled + 2 - segment.started_ns,
            outcome: AttemptOutcome::Acknowledged {
                receipt_sha256: receipt.clone(),
            },
        });
        case.ledger.readback.push(Readback::Present {
            logical_id: id.clone(),
            request_sha256: request.clone(),
            effect_sha256: effect,
            receipt_sha256: receipt.clone(),
            applications: 1.try_into().unwrap(),
        });
        let kind = storage_load_kind(sequence);
        segment.attempts.push(LoadAttempt {
            write: (kind == StorageLoadKind::MixedWrite).then_some(WriteTransition {
                slot: sequence - first,
                generation: 0,
            }),
            sequence,
            kind,
            scheduled_ns: scheduled,
            started_ns: scheduled + 1,
            finished_ns: scheduled + 2,
            verification_finished_ns: Some(scheduled + 3),
            outcome: LoadOutcome::Acknowledged,
            logical_id: id,
            request_sha256: request,
            result_sha256: receipt,
        });
    }
    segment.manifest.required_cases.truncate(1);
    segment.manifest.required_cases[0].expected_ledger_sha256 =
        expected_digest(&case.ledger.expected).unwrap();
    segment.result.manifest_sha256 = manifest_digest(&segment.manifest).unwrap();
    segment
}

fn receipt(started_ns: u64, finished_ns: u64, verified_ns: u64) -> BackupEvidence {
    let hash = "a".repeat(64);
    let receipt = serde_json::from_value(serde_json::json!({
        "backup": {
            "backup_sha256": hash, "label":"contract-fixture", "created_at_unix_ms":1,
            "archive": { "format_version":1, "contract_version":1, "archive_sha256":hash,
                "action_count":1, "standalone_claims":0, "runtime_commits":1,
                "runtime_mutations":1,"payload_bytes":8,"claim_sequence":0,"runtime_cursor":1 },
            "claims":"included", "typed_runtime":"included", "catalogues":"included",
            "installed_control":"included", "object_payloads":"included",
            "projections":"rebuild_required", "invocation_telemetry":"excluded",
            "snapshot_leases":"excluded", "application_complete":true
        },
        "catalogue_revision":1,"catalogue_sha256":hash,"idempotent_replay":false
    }))
    .unwrap();
    BackupEvidence {
        started_ns,
        finished_ns,
        verified_ns,
        receipt,
        verification_catalogue_sha256: digest("fixture-catalogue", &1).unwrap(),
    }
}

fn complete(segment: &mut SustainedSegment, receipt: BackupEvidence) {
    segment.lanes = vec![LaneEvidence {
        lane: CapacityLane::BackupUnderWrite,
        repetitions: 1.try_into().unwrap(),
        receipt_sha256: digest("c07g.backup-evidence", &receipt).unwrap(),
        acknowledged_readback_verified: true,
    }];
    segment.backup = Some(receipt);
}

fn history() -> (StorageCampaign, SustainedSegment, SustainedSegment) {
    let (mut campaign, _) = tests::fixture();
    campaign.points[0].requests_per_second = 100.try_into().unwrap();
    let mut first = segment(&campaign, 0, 15);
    first.backup_started_ns = Some(14 * TICK);
    let mut second = segment(&campaign, 15, 5);
    second.index = 1;
    second.previous_sha256 = Some(digest("rrflow-storage-segment-v1", &first).unwrap());
    complete(&mut second, receipt(14 * TICK, 17 * TICK, 18 * TICK));
    (campaign, first, second)
}

#[test]
fn backup_chain_reconstructs_cross_segment_ack_overlap_and_preserves_schedule() {
    let (campaign, first, second) = history();
    validate_segment(&campaign, &first).unwrap();
    assert!(validate_segment(&campaign, &second)
        .unwrap_err()
        .contains("start"));
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("campaign");
    let mut writer = CampaignWriter::create(&path, campaign).unwrap();
    writer
        .record_event(&ProgressEvent::BackupStarted {
            segment: 0,
            point: first.point.clone(),
            seed: first.seed,
            started_ns: first.backup_started_ns.unwrap(),
        })
        .unwrap();
    writer.append(&first).unwrap();
    assert!(writer.backups.require_settled().is_err());
    writer.append(&second).unwrap();
    writer.backups.require_settled().unwrap();
    let row = writer.totals.values().next().unwrap();
    assert_eq!(row.attempts, 20);
    assert_eq!(row.schedule_origin_ns, Some(0));
    assert!(row.lanes.contains(&CapacityLane::BackupUnderWrite));
    writer
        .finish(
            CampaignStatus::Interrupted,
            vec!["contract-fixture".to_owned().try_into().unwrap()],
        )
        .unwrap();
    verify_campaign(&path).unwrap();
    // Altering the earlier start cannot be hidden by a later valid-looking receipt.
    let mut changed = first;
    changed.backup_started_ns = Some(14 * TICK + 1);
    std::fs::write(
        path.join("segment-00000000000000000000.json"),
        serde_json::to_vec(&changed).unwrap(),
    )
    .unwrap();
    assert!(verify_campaign(&path).is_err());
}

#[test]
fn backup_chain_rejects_missing_duplicate_mismatched_overdue_and_unfinished_work() {
    let (campaign, first, second) = history();
    let temp = tempfile::tempdir().unwrap();
    let mut writer =
        CampaignWriter::create(&temp.path().join("campaign"), campaign.clone()).unwrap();
    writer.append(&first).unwrap();
    for alteration in 0..8 {
        let mut invalid = second.clone();
        match alteration {
            0 => invalid.backup_started_ns = first.backup_started_ns,
            1 => invalid.backup.as_mut().unwrap().started_ns += 1,
            2 => invalid.lanes.clear(),
            3 => invalid.backup = None,
            4 => invalid.phase = LoadPhase::Warmup,
            5 => invalid.seed += 1,
            6 => {
                invalid.backup.as_mut().unwrap().verified_ns = 1_200_000_000;
                invalid.finished_ns = 1_300_000_000;
            }
            7 => {
                invalid
                    .backup
                    .as_mut()
                    .unwrap()
                    .receipt
                    .backup
                    .application_complete = false
            }
            _ => unreachable!(),
        }
        assert!(
            validate_next(
                &campaign,
                &invalid,
                1,
                &writer.previous,
                writer.last_finished_ns,
                &writer.totals,
                &writer.backups
            )
            .is_err(),
            "accepted alteration {alteration}"
        );
    }
    let mut expired = second.clone();
    expired.backup = None;
    expired.lanes.clear();
    expired.finished_ns = 1_200_000_000;
    assert!(validate_next(
        &campaign,
        &expired,
        1,
        &writer.previous,
        writer.last_finished_ns,
        &writer.totals,
        &writer.backups
    )
    .unwrap_err()
    .contains("deadline"));
    assert!(writer.backups.require_settled().is_err());
    writer.append(&second).unwrap();
    let mut duplicate = second;
    duplicate.index = 2;
    duplicate.previous_sha256 = writer.previous.clone();
    assert!(writer
        .append(&duplicate)
        .unwrap_err()
        .contains("duplicated"));
}

#[test]
fn backup_requires_real_interval_overlap_not_merely_an_ack_in_either_segment() {
    let (campaign, mut first, mut second) = history();
    first.backup_started_ns = Some(145_000_000); // after the first segment's final ACK
    complete(&mut second, receipt(145_000_000, 149_000_000, 180_000_000));
    second.previous_sha256 = Some(digest("rrflow-storage-segment-v1", &first).unwrap());
    let temp = tempfile::tempdir().unwrap();
    let mut writer = CampaignWriter::create(&temp.path().join("campaign"), campaign).unwrap();
    writer.append(&first).unwrap();
    assert!(writer
        .append(&second)
        .unwrap_err()
        .contains("overlapping ACK"));
}

#[test]
fn existing_same_segment_backup_receipts_still_validate() {
    let (campaign, mut first, _) = history();
    first.backup_started_ns = None;
    complete(&mut first, receipt(14 * TICK, 145_000_000, 146_000_000));
    validate_segment(&campaign, &first).unwrap();
}

#[test]
fn completed_campaign_cannot_drop_a_started_backup_even_after_final_readback() {
    let (campaign, first, _) = history();
    let temp = tempfile::tempdir().unwrap();
    let mut writer = CampaignWriter::create(&temp.path().join("campaign"), campaign).unwrap();
    writer.append(&first).unwrap();
    writer
        .record_event(&ProgressEvent::FinalReadback {
            point: first.point.clone(),
            seed: first.seed,
            killed_pid: 1,
            reopened_pid: 2,
            seed_records: 1,
            working_slots: 0,
            states_sha256: digest("fixture-readback", &1).unwrap(),
            started_ns: 200_000_000,
            finished_ns: 210_000_000,
        })
        .unwrap();
    assert!(writer
        .finish(CampaignStatus::Completed, vec![])
        .unwrap_err()
        .contains("unfinished backup"));
}
