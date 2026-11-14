use super::*;

#[test]
fn reopened_corpus_progress_roundtrips_without_claiming_completion() {
    let (campaign, _) = super::super::sustained::tests::fixture();
    let event = ProgressEvent::ReopenedCorpusVerified {
        point: campaign.points[0].id.clone(),
        seed: campaign.seeds[0],
        ordinal: 0,
        effect_sha256: digest("test-effect", &0u64).unwrap(),
    };
    let bytes = serde_json::to_vec(&event).unwrap();
    let decoded: ProgressEvent = serde_json::from_slice(&bytes).unwrap();
    assert!(matches!(
        decoded,
        ProgressEvent::ReopenedCorpusVerified { ordinal: 0, .. }
    ));
    assert!(String::from_utf8(bytes)
        .unwrap()
        .contains("reopened_corpus_verified"));
    let mut audit = ProgressAudit::new(&campaign);
    let gap = ProgressEvent::ReopenedCorpusVerified {
        point: campaign.points[0].id.clone(),
        seed: campaign.seeds[0],
        ordinal: 1,
        effect_sha256: digest("test-effect", &1u64).unwrap(),
    };
    assert!(audit.observe(&campaign, &gap).is_err());
    audit.observe(&campaign, &event).unwrap();
    assert!(
        audit.observe(&campaign, &event).is_err(),
        "duplicate cannot count twice"
    );
    assert!(
        audit.observe(&campaign, &gap).is_err(),
        "readback cannot exceed corpus"
    );
    let mut segment = super::super::sustained::tests::fixture().1;
    let expected = audit.points[&(segment.point.clone(), segment.seed)]
        .reopened_corpus_digest
        .clone()
        .unwrap();
    segment.reopens = Some(ReopenEvidence {
        started_ns: 0,
        finished_ns: 1,
        corpus_records_verified: 1,
        corpus_readback_sha256: expected,
        cycles: vec![],
    });
    audit.bind_segment(&segment).unwrap();
    segment.reopens.as_mut().unwrap().corpus_records_verified = 0;
    assert!(
        audit.bind_segment(&segment).is_err(),
        "partial progress cannot bind a complete lane"
    );
    segment.reopens.as_mut().unwrap().corpus_records_verified = 1;
    segment.reopens.as_mut().unwrap().corpus_readback_sha256 = digest("wrong", &1u64).unwrap();
    assert!(
        audit.bind_segment(&segment).is_err(),
        "wrong digest cannot bind"
    );
}

#[test]
fn completed_progress_requires_the_exact_backup_start_in_the_segment_chain() {
    let (campaign, mut segment) = super::super::sustained::tests::fixture();
    let mut audit = ProgressAudit::new(&campaign);
    audit
        .observe(
            &campaign,
            &ProgressEvent::BackupStarted {
                segment: segment.index,
                point: segment.point.clone(),
                seed: segment.seed,
                started_ns: 1,
            },
        )
        .unwrap();
    assert!(audit.require_bound(CampaignStatus::Completed).is_err());
    audit.require_bound(CampaignStatus::Interrupted).unwrap();
    segment.backup_started_ns = Some(1);
    audit.bind_segment(&segment).unwrap();
    audit.require_bound(CampaignStatus::Completed).unwrap();
    // A duplicated or changed start cannot be absorbed as the same receipt.
    audit.bind_segment(&segment).unwrap();
    assert!(audit.require_bound(CampaignStatus::Completed).is_err());
    let mut missing = ProgressAudit::new(&campaign);
    missing.bind_segment(&segment).unwrap();
    assert!(missing.require_bound(CampaignStatus::Completed).is_err());
}

#[test]
fn progress_survives_failed_run_and_rejects_missing_truncated_or_changed_bytes() {
    let temporary = tempfile::tempdir().unwrap();
    let identity = digest("progress-fixture", &1).unwrap();
    let mut writer = ProgressJournal::create(temporary.path(), &identity).unwrap();
    writer
        .append(
            &ProgressEvent::Failure {
                reason: "driver-deadline".to_owned().try_into().unwrap(),
            },
            MAX_DOCUMENT_BYTES,
        )
        .unwrap();
    writer.checkpoint().unwrap();
    let receipt = writer.finish().unwrap();
    assert_eq!(receipt.records, 2);
    verify(temporary.path(), &receipt, &identity, MAX_DOCUMENT_BYTES).unwrap();
    assert!(verify(
        temporary.path(),
        &receipt,
        &digest("other", &1).unwrap(),
        MAX_DOCUMENT_BYTES
    )
    .is_err());
    let path = temporary.path().join("progress.jsonl");
    let original = std::fs::read(&path).unwrap();
    let mut changed = original.clone();
    let at = changed
        .windows(b"driver-deadline".len())
        .position(|s| s == b"driver-deadline")
        .unwrap();
    changed[at] = b'x';
    std::fs::write(&path, changed).unwrap();
    assert!(verify(temporary.path(), &receipt, &identity, MAX_DOCUMENT_BYTES).is_err());
    std::fs::write(&path, &original[..original.len() - 1]).unwrap();
    assert!(verify(temporary.path(), &receipt, &identity, MAX_DOCUMENT_BYTES).is_err());
    std::fs::remove_file(&path).unwrap();
    assert!(verify(temporary.path(), &receipt, &identity, MAX_DOCUMENT_BYTES).is_err());
}

#[test]
fn progress_quota_rejection_preserves_last_checkpoint() {
    let temporary = tempfile::tempdir().unwrap();
    let identity = digest("progress-fixture", &1).unwrap();
    let mut writer = ProgressJournal::create(temporary.path(), &identity).unwrap();
    let before = std::fs::read(temporary.path().join("progress.jsonl")).unwrap();
    assert!(writer
        .append(
            &ProgressEvent::Failure {
                reason: "not-admitted".to_owned().try_into().unwrap(),
            },
            writer.bytes()
        )
        .is_err());
    let receipt = writer.finish().unwrap();
    assert_eq!(
        std::fs::read(temporary.path().join("progress.jsonl")).unwrap(),
        before
    );
    assert_eq!(receipt.records, 1);
    verify(temporary.path(), &receipt, &identity, MAX_DOCUMENT_BYTES).unwrap();
}
