use super::format::{
    action_bytes, scan_archive_prefix, ArchiveAction, ArchiveHeader, ArchiveWriter,
    LogicalArchiveCheckpoint, LogicalArchiveInventory, LogicalArchiveOperation,
    LogicalArchiveProgress, PrefixActionReader, MAX_ACTION_BYTES,
};
use super::receipt::{
    export_paths, load_receipt, remove_export_artifacts, remove_receipt, write_receipt,
    ExportReceipt,
};
use crate::filesystem::AnchorInput;
use crate::{Error, Result, StorageEngine};
use rrd_core::{AuditEnvelope, Claim, RuntimeChange, RuntimeCommit, RuntimeMutation};
use std::collections::VecDeque;

const PAGE_SIZE: usize = 1_024;

#[derive(Clone, Copy)]
struct CheckpointPolicy {
    actions: usize,
    payload_bytes: usize,
}

// No action is retained in memory for batching. These bounds limit the private
// archive prefix since its last durable receipt; publication still syncs all
// bytes. One already-admitted action may cross the byte threshold.
const BACKUP_CHECKPOINTS: CheckpointPolicy = CheckpointPolicy {
    actions: 64,
    payload_bytes: 1024 * 1024,
};
const ACTION_CHECKPOINTS: CheckpointPolicy = CheckpointPolicy {
    actions: 1,
    payload_bytes: 1024 * 1024,
};

#[derive(Debug, Clone, PartialEq, Eq)]
struct SourceSummary {
    claim_sequence: u64,
    runtime_cursor: u64,
    action_count: u64,
    standalone_claims: u64,
    runtime_commits: u64,
    runtime_mutations: u64,
    payload_bytes: u64,
    runtime_audit_sha256: Option<String>,
}

pub fn export_logical_archive<E: StorageEngine>(
    engine: &E,
    destination: &(impl AnchorInput + ?Sized),
) -> Result<LogicalArchiveInventory> {
    let destination = &destination.anchor().map_err(archive_io)?;
    export_with_checkpoints(engine, destination, BACKUP_CHECKPOINTS, |_| Ok(()))
}

/// Exports a stable logical cut while publishing durable action/receipt
/// checkpoints to `progress`. Returning an error from the callback simulates
/// interruption and intentionally leaves the exact private partial/receipt
/// artifacts for the next call to reconcile and resume.
pub fn export_logical_archive_with_progress<E: StorageEngine>(
    engine: &E,
    destination: &(impl AnchorInput + ?Sized),
    progress: impl FnMut(&LogicalArchiveProgress) -> Result<()>,
) -> Result<LogicalArchiveInventory> {
    export_with_checkpoints(engine, destination, ACTION_CHECKPOINTS, progress)
}

fn export_with_checkpoints<E: StorageEngine>(
    engine: &E,
    destination: &(impl AnchorInput + ?Sized),
    checkpoints: CheckpointPolicy,
    mut progress: impl FnMut(&LogicalArchiveProgress) -> Result<()>,
) -> Result<LogicalArchiveInventory> {
    let destination = &destination.anchor().map_err(archive_io)?;
    if destination.try_exists().map_err(archive_io)? {
        return Err(Error::Archive(format!(
            "archive target already exists: {}",
            destination.display()
        )));
    }
    let parent = destination
        .parent()
        .ok_or_else(|| Error::Archive("filesystem root has no parent".into()))?;
    parent.create_dir_all().map_err(archive_io)?;

    // The first bounded pass validates the complete source and determines the
    // immutable header without retaining the log. The second pass either
    // compares or appends each deterministic frame.
    let expected = scan_source(engine)?;
    let header = ArchiveHeader {
        claim_sequence: expected.claim_sequence,
        runtime_cursor: expected.runtime_cursor,
        action_count: expected.action_count,
    };
    let (partial_path, receipt_path) = export_paths(destination)?;
    if receipt_path.try_exists().map_err(archive_io)?
        && !partial_path.try_exists().map_err(archive_io)?
    {
        return Err(Error::Archive(
            "logical export receipt exists without its partial archive".into(),
        ));
    }
    if partial_path.try_exists().map_err(archive_io)? {
        if let Ok(inventory) = super::format::inspect_logical_archive(&partial_path) {
            if !inventory_matches_source(&inventory, &expected) {
                remove_export_artifacts(&partial_path, &receipt_path)?;
                return Err(Error::Archive(
                    "finalized logical export no longer matches the source cut".into(),
                ));
            }
            partial_path
                .rename_new_to(destination)
                .map_err(archive_io)?;
            remove_receipt(&receipt_path)?;
            return Ok(inventory);
        }
    }
    if !partial_path.try_exists().map_err(archive_io)? {
        let mut writer = ArchiveWriter::create(&partial_path, header)?;
        writer.sync()?;
        let prefix = writer.prefix_inventory()?;
        drop(writer);
        write_receipt(
            &receipt_path,
            &ExportReceipt::from_prefix(
                destination,
                expected.runtime_audit_sha256.clone(),
                &prefix,
            )?,
        )?;
    }

    let prefix = scan_archive_prefix(&partial_path)?;
    if prefix.header != header {
        remove_export_artifacts(&partial_path, &receipt_path)?;
        return Err(Error::Archive(
            "logical export source watermarks changed; discarded the stale partial cut".into(),
        ));
    }
    if !receipt_path.try_exists().map_err(archive_io)? {
        write_receipt(
            &receipt_path,
            &ExportReceipt::from_prefix(
                destination,
                expected.runtime_audit_sha256.clone(),
                &prefix,
            )?,
        )?;
    }
    let receipt = load_receipt::<ExportReceipt>(&receipt_path)?;
    receipt.validate_fixed(
        destination,
        header,
        expected.runtime_audit_sha256.as_deref(),
    )?;
    receipt.validate_prefix(&prefix)?;
    if receipt.completed_actions != prefix.completed_actions {
        write_receipt(
            &receipt_path,
            &ExportReceipt::from_prefix(
                destination,
                expected.runtime_audit_sha256.clone(),
                &prefix,
            )?,
        )?;
    }

    if engine.claims().sequence()? != header.claim_sequence
        || engine.runtime().cursor()? != header.runtime_cursor
    {
        remove_export_artifacts(&partial_path, &receipt_path)?;
        return Err(Error::Archive(
            "logical export source advanced before resume; discarded the stale partial cut".into(),
        ));
    }

    let mut prefix_reader = PrefixActionReader::open(&partial_path, &prefix)?;
    let mut writer = ArchiveWriter::resume(&partial_path, &prefix)?;
    let mut checkpoint = |writer: &mut ArchiveWriter| -> Result<()> {
        writer.sync()?;
        let durable = writer.prefix_inventory()?;
        let checkpoint = LogicalArchiveProgress {
            operation: LogicalArchiveOperation::Export,
            checkpoint: LogicalArchiveCheckpoint::ActionDurable,
            completed_actions: durable.completed_actions,
            action_count: header.action_count,
            claim_sequence: durable.claim_sequence,
            runtime_cursor: durable.runtime_cursor,
        };
        progress(&checkpoint)?;
        let receipt = ExportReceipt::from_prefix(
            destination,
            expected.runtime_audit_sha256.clone(),
            &durable,
        )?;
        write_receipt(&receipt_path, &receipt)?;
        progress(&LogicalArchiveProgress {
            checkpoint: LogicalArchiveCheckpoint::ReceiptDurable,
            ..checkpoint
        })
    };
    let mut pending_actions = 0usize;
    let mut pending_bytes = 0usize;
    let emitted = stream_source(engine, |action, payload| {
        if prefix_reader.compare_or_absent(action, payload)? {
            return Ok(());
        }
        writer.action(action, payload)?;
        pending_actions += 1;
        pending_bytes = pending_bytes
            .checked_add(payload.len())
            .ok_or_else(|| Error::Archive("export checkpoint byte count overflow".into()))?;
        if pending_actions >= checkpoints.actions || pending_bytes >= checkpoints.payload_bytes {
            checkpoint(&mut writer)?;
            pending_actions = 0;
            pending_bytes = 0;
        }
        Ok(())
    });
    let emitted = match emitted {
        Ok(value) => value,
        Err(error) => {
            if engine.claims().sequence()? != header.claim_sequence
                || engine.runtime().cursor()? != header.runtime_cursor
            {
                remove_export_artifacts(&partial_path, &receipt_path)?;
            }
            return Err(error);
        }
    };
    prefix_reader.finish()?;
    if emitted != expected {
        remove_export_artifacts(&partial_path, &receipt_path)?;
        return Err(Error::Archive(
            "logical export source changed between validation and publication".into(),
        ));
    }
    if pending_actions != 0 {
        checkpoint(&mut writer)?;
    }
    let inventory = writer.finish()?;
    if inventory.runtime_audit_sha256 != expected.runtime_audit_sha256 {
        remove_export_artifacts(&partial_path, &receipt_path)?;
        return Err(Error::Archive(
            "logical export audit head differs from the validated source".into(),
        ));
    }
    progress(&LogicalArchiveProgress {
        operation: LogicalArchiveOperation::Export,
        checkpoint: LogicalArchiveCheckpoint::ArchiveFinalized,
        completed_actions: inventory.action_count,
        action_count: inventory.action_count,
        claim_sequence: inventory.claim_sequence,
        runtime_cursor: inventory.runtime_cursor,
    })?;
    partial_path
        .rename_new_to(destination)
        .map_err(archive_io)?;
    remove_receipt(&receipt_path)?;
    Ok(inventory)
}

fn inventory_matches_source(inventory: &LogicalArchiveInventory, source: &SourceSummary) -> bool {
    inventory.format_version == 1
        && inventory.contract_version == 1
        && inventory.action_count == source.action_count
        && inventory.standalone_claims == source.standalone_claims
        && inventory.runtime_commits == source.runtime_commits
        && inventory.runtime_mutations == source.runtime_mutations
        && inventory.payload_bytes == source.payload_bytes
        && inventory.claim_sequence == source.claim_sequence
        && inventory.runtime_cursor == source.runtime_cursor
        && inventory.runtime_audit_sha256 == source.runtime_audit_sha256
}

fn scan_source(engine: &impl StorageEngine) -> Result<SourceSummary> {
    stream_source(engine, |_, _| Ok(()))
}

fn stream_source(
    engine: &impl StorageEngine,
    mut emit: impl FnMut(&ArchiveAction, &[u8]) -> Result<()>,
) -> Result<SourceSummary> {
    let claim_sequence = engine.claims().sequence()?;
    let runtime_cursor = engine.runtime().cursor()?;
    let mut claims = ClaimPager::new(engine, claim_sequence);
    let mut commits = RuntimeCommitPager::new(engine, runtime_cursor);
    let mut summary = SourceSummary {
        claim_sequence,
        runtime_cursor,
        action_count: 0,
        standalone_claims: 0,
        runtime_commits: 0,
        runtime_mutations: 0,
        payload_bytes: 0,
        runtime_audit_sha256: None,
    };

    while let Some(action) = commits.next_commit()? {
        let ArchiveAction::RuntimeCommit { commit, .. } = &action else {
            unreachable!("runtime pager only emits runtime commits");
        };
        let committed_claims = commit
            .mutations
            .iter()
            .filter_map(|mutation| match mutation {
                RuntimeMutation::Claim { claim } => Some(claim),
                _ => None,
            })
            .collect::<Vec<_>>();
        if let Some(first) = committed_claims.first() {
            loop {
                let actual = claims.next_claim()?.ok_or_else(|| {
                    Error::Archive("runtime claim is absent from the claim sequence log".into())
                })?;
                if actual.digest() == first.digest() {
                    break;
                }
                emit_counted(
                    &mut summary,
                    ArchiveAction::StandaloneClaim { claim: actual },
                    &mut emit,
                )?;
            }
            for expected in committed_claims.into_iter().skip(1) {
                let actual = claims.next_claim()?.ok_or_else(|| {
                    Error::Archive("runtime claim is absent from the claim sequence log".into())
                })?;
                if actual.digest() != expected.digest() {
                    return Err(Error::Archive(
                        "claim mutations in one runtime commit are not contiguous in sequence log"
                            .into(),
                    ));
                }
            }
        }
        emit_counted(&mut summary, action, &mut emit)?;
    }
    while let Some(claim) = claims.next_claim()? {
        emit_counted(
            &mut summary,
            ArchiveAction::StandaloneClaim { claim },
            &mut emit,
        )?;
    }
    claims.finish()?;
    commits.finish()?;
    if engine.claims().sequence()? != claim_sequence || engine.runtime().cursor()? != runtime_cursor
    {
        return Err(Error::Archive(
            "source watermarks changed during logical export; retry from a stable cut".into(),
        ));
    }
    Ok(summary)
}

fn emit_counted(
    summary: &mut SourceSummary,
    action: ArchiveAction,
    emit: &mut impl FnMut(&ArchiveAction, &[u8]) -> Result<()>,
) -> Result<()> {
    let payload = action_bytes(&action)?;
    emit(&action, &payload)?;
    summary.action_count = summary
        .action_count
        .checked_add(1)
        .ok_or_else(|| Error::Archive("archive action count overflow".into()))?;
    summary.payload_bytes = summary
        .payload_bytes
        .checked_add(payload.len() as u64)
        .ok_or_else(|| Error::Archive("archive payload count overflow".into()))?;
    match action {
        ArchiveAction::StandaloneClaim { .. } => {
            summary.standalone_claims += 1;
        }
        ArchiveAction::RuntimeCommit { commit, audit } => {
            summary.runtime_commits += 1;
            summary.runtime_mutations = summary
                .runtime_mutations
                .checked_add(commit.mutations.len() as u64)
                .ok_or_else(|| Error::Archive("runtime mutation count overflow".into()))?;
            summary.runtime_audit_sha256 = Some(audit.digest);
        }
    }
    Ok(())
}

struct ClaimPager<'a, E> {
    engine: &'a E,
    head: u64,
    after: u64,
    page: VecDeque<Claim>,
}

impl<'a, E: StorageEngine> ClaimPager<'a, E> {
    fn new(engine: &'a E, head: u64) -> Self {
        Self {
            engine,
            head,
            after: 0,
            page: VecDeque::new(),
        }
    }

    fn next_claim(&mut self) -> Result<Option<Claim>> {
        if self.page.is_empty() && self.after < self.head {
            let through = self.head.min(self.after.saturating_add(PAGE_SIZE as u64));
            let page = self.engine.claims().claims_in_range(self.after, through)?;
            let expected = usize::try_from(through - self.after)
                .map_err(|_| Error::Archive("claim page exceeds usize".into()))?;
            if page.len() != expected {
                return Err(Error::Archive(format!(
                    "claim log is discontinuous in ({}, {through}]",
                    self.after
                )));
            }
            self.after = through;
            self.page.extend(page);
        }
        Ok(self.page.pop_front())
    }

    fn finish(&self) -> Result<()> {
        if self.after != self.head || !self.page.is_empty() {
            return Err(Error::Archive(
                "claim export did not consume its captured watermark".into(),
            ));
        }
        Ok(())
    }
}

struct RuntimeCommitPager<'a, E> {
    engine: &'a E,
    head: u64,
    fetched_through: u64,
    validated_through: u64,
    page: VecDeque<RuntimeChange>,
    pending: Option<RuntimeChange>,
    previous_change_sha256: Option<String>,
    previous_audit_sha256: Option<String>,
}

impl<'a, E: StorageEngine> RuntimeCommitPager<'a, E> {
    fn new(engine: &'a E, head: u64) -> Self {
        Self {
            engine,
            head,
            fetched_through: 0,
            validated_through: 0,
            page: VecDeque::new(),
            pending: None,
            previous_change_sha256: None,
            previous_audit_sha256: None,
        }
    }

    fn next_commit(&mut self) -> Result<Option<ArchiveAction>> {
        let Some(first) = self.next_change()? else {
            return Ok(None);
        };
        if first.commit_ordinal != 0 {
            return Err(Error::Archive(
                "runtime commit begins at a non-zero ordinal".into(),
            ));
        }
        let commit_id = first.commit_id.clone();
        let scope = first.scope.clone();
        let at = first.at;
        let actor = first.actor.clone();
        let expected_cursor = first.cursor - 1;
        let mut mutations = Vec::new();
        let mut encoded_bound = 0usize;
        let mut change = first;
        loop {
            let ordinal = mutations.len() as u64;
            if change.cursor != self.validated_through + 1
                || change.commit_ordinal != ordinal
                || change.commit_id != commit_id
                || change.scope != scope
                || change.at != at
                || change.actor != actor
                || change.previous_digest != self.previous_change_sha256
                || !change.verify_digest()
            {
                return Err(Error::Archive(format!(
                    "runtime change {} failed chain or commit validation",
                    change.cursor
                )));
            }
            encoded_bound = encoded_bound
                .checked_add(serde_json::to_vec(&change.mutation)?.len())
                .ok_or_else(|| Error::Archive("runtime commit size overflow".into()))?;
            if encoded_bound > MAX_ACTION_BYTES {
                return Err(Error::Archive(
                    "runtime commit exceeds the archive action memory bound".into(),
                ));
            }
            self.validated_through = change.cursor;
            self.previous_change_sha256 = Some(change.digest.clone());
            mutations.push(change.mutation);
            match self.next_change()? {
                Some(next) if next.commit_id == commit_id => change = next,
                Some(next) => {
                    self.pending = Some(next);
                    break;
                }
                None => break,
            }
        }
        let commit = RuntimeCommit {
            scope,
            at,
            actor,
            expected_cursor,
            mutations,
        };
        commit.validate()?;
        if commit.digest() != commit_id {
            return Err(Error::Archive(format!(
                "runtime commit at cursor {} has a mismatched content identity",
                expected_cursor + 1
            )));
        }
        let audit = self.engine.runtime().audit(&commit_id)?.ok_or_else(|| {
            Error::Archive(format!("runtime commit {commit_id} has no audit envelope"))
        })?;
        let expected_audit = AuditEnvelope::accepted_commit_at_read(
            &commit,
            audit.read.as_ref(),
            &commit_id,
            self.validated_through,
            self.previous_audit_sha256.clone(),
        )?;
        if audit != expected_audit {
            return Err(Error::Archive(format!(
                "runtime commit {commit_id} audit envelope is missing or divergent"
            )));
        }
        self.previous_audit_sha256 = Some(audit.digest.clone());
        Ok(Some(ArchiveAction::RuntimeCommit {
            commit,
            audit: Box::new(audit),
        }))
    }

    fn next_change(&mut self) -> Result<Option<RuntimeChange>> {
        if let Some(change) = self.pending.take() {
            return Ok(Some(change));
        }
        if self.page.is_empty() && self.fetched_through < self.head {
            let page =
                self.engine
                    .runtime()
                    .changes_since(self.fetched_through, PAGE_SIZE, None)?;
            if page.head_cursor != self.head
                || page.requested_after != self.fetched_through
                || page.through_cursor <= self.fetched_through
                || page.through_cursor > self.head
                || page.changes.is_empty()
            {
                return Err(Error::Archive(
                    "runtime watermark or page continuity changed during export".into(),
                ));
            }
            self.fetched_through = page.through_cursor;
            self.page.extend(page.changes);
        }
        Ok(self.page.pop_front())
    }

    fn finish(&self) -> Result<()> {
        if self.fetched_through != self.head
            || self.validated_through != self.head
            || !self.page.is_empty()
            || self.pending.is_some()
        {
            return Err(Error::Archive(
                "runtime export did not consume its captured watermark".into(),
            ));
        }
        Ok(())
    }
}

fn archive_io(error: std::io::Error) -> Error {
    Error::Archive(error.to_string())
}

#[cfg(test)]
mod checkpoint_tests {
    use super::*;
    use crate::RrflowKvStore;
    use rrd_core::{Predicate, Producer, ScopeId, Subject};
    use std::path::Path;

    fn source(path: &Path, actions: u64, value_bytes: usize) -> RrflowKvStore {
        let store = RrflowKvStore::open(path).unwrap();
        for index in 0..actions {
            let claim = Claim::new(
                Subject::new(format!("checkpoint:{index}")).unwrap(),
                Predicate::new("value").unwrap(),
                "x".repeat(value_bytes),
                index + 1,
                index + 1,
                Producer {
                    actor: "archive-checkpoint-test".into(),
                    on_behalf_of: None,
                    session: None,
                },
            );
            if index % 2 == 0 {
                store.claims().append_batch(&[claim]).unwrap();
            } else {
                store
                    .runtime()
                    .commit(&RuntimeCommit {
                        scope: ScopeId::new("instance:checkpoint").unwrap(),
                        at: index + 1,
                        actor: "archive-checkpoint-test".into(),
                        expected_cursor: store.runtime().cursor().unwrap(),
                        mutations: vec![RuntimeMutation::Claim { claim }],
                    })
                    .unwrap();
            }
        }
        store
    }

    #[test]
    fn batched_export_preserves_bytes_restore_and_per_action_callbacks() {
        let dir = tempfile::tempdir().unwrap();
        let store = source(&dir.path().join("source"), 130, 8);
        let baseline = dir.path().join("per-action");
        let mut per_action = Vec::new();
        let expected = export_logical_archive_with_progress(&store, &baseline, |p| {
            if p.checkpoint == LogicalArchiveCheckpoint::ReceiptDurable {
                per_action.push(p.completed_actions);
            }
            Ok(())
        })
        .unwrap();
        assert_eq!(per_action, (1..=130).collect::<Vec<_>>());
        let archive = dir.path().join("batched");
        let mut checkpoints = Vec::new();
        let actual = export_with_checkpoints(&store, &archive, BACKUP_CHECKPOINTS, |p| {
            if p.checkpoint == LogicalArchiveCheckpoint::ReceiptDurable {
                checkpoints.push(p.completed_actions);
            }
            Ok(())
        })
        .unwrap();
        assert_eq!(checkpoints, [64, 128, 130]);
        assert_eq!(actual, expected);
        assert_eq!(
            std::fs::read(&archive).unwrap(),
            std::fs::read(baseline).unwrap()
        );
        let restored = dir.path().join("restored");
        crate::restore_logical_archive_to_new_root(&archive, &restored, 200).unwrap();
        let reopened = RrflowKvStore::open(&restored).unwrap();
        assert_eq!(
            reopened.claims().claims_in_range(0, 130).unwrap(),
            store.claims().claims_in_range(0, 130).unwrap()
        );
        let reexported = dir.path().join("restored-export");
        assert_eq!(
            export_logical_archive(&reopened, &reexported).unwrap(),
            expected
        );
        assert_eq!(
            std::fs::read(reexported).unwrap(),
            std::fs::read(archive).unwrap()
        );
    }

    #[test]
    fn interruption_at_batched_durable_boundaries_resumes_identically() {
        let dir = tempfile::tempdir().unwrap();
        let store = source(&dir.path().join("source"), 130, 8);
        let baseline = dir.path().join("baseline");
        let expected = export_logical_archive(&store, &baseline).unwrap();
        for (ordinal, checkpoint) in [
            LogicalArchiveCheckpoint::ActionDurable,
            LogicalArchiveCheckpoint::ReceiptDurable,
        ]
        .into_iter()
        .enumerate()
        {
            for count in [64, 128, 130] {
                let archive = dir.path().join(format!("interrupted-{ordinal}-{count}"));
                let error = export_with_checkpoints(&store, &archive, BACKUP_CHECKPOINTS, |p| {
                    if p.checkpoint == checkpoint && p.completed_actions == count {
                        return Err(Error::FaultInjected("batched_checkpoint"));
                    }
                    Ok(())
                })
                .unwrap_err();
                assert!(matches!(error, Error::FaultInjected("batched_checkpoint")));
                assert!(!archive.exists());
                assert_eq!(export_logical_archive(&store, &archive).unwrap(), expected);
                assert_eq!(
                    std::fs::read(archive).unwrap(),
                    std::fs::read(&baseline).unwrap()
                );
            }
        }
    }

    #[test]
    fn payload_bound_checkpoints_before_action_count_and_flushes_tail() {
        let dir = tempfile::tempdir().unwrap();
        let store = source(&dir.path().join("source"), 5, 400_000);
        let mut checkpoints = Vec::new();
        let archive = dir.path().join("bounded");
        export_with_checkpoints(&store, &archive, BACKUP_CHECKPOINTS, |p| {
            if p.checkpoint == LogicalArchiveCheckpoint::ReceiptDurable {
                checkpoints.push(p.completed_actions);
            }
            Ok(())
        })
        .unwrap();
        assert_eq!(checkpoints, [3, 5]);
        let baseline = dir.path().join("baseline");
        export_logical_archive_with_progress(&store, &baseline, |_| Ok(())).unwrap();
        assert_eq!(
            std::fs::read(archive).unwrap(),
            std::fs::read(baseline).unwrap()
        );
    }

    #[test]
    #[ignore = "manual bounded native checkpoint comparison, not capacity qualification"]
    fn bounded_checkpoint_measurement() {
        let dir = tempfile::tempdir().unwrap();
        let store = source(&dir.path().join("source"), 512, 1024);
        let mut baseline = None;
        for (name, policy) in [
            ("per-action", ACTION_CHECKPOINTS),
            ("bounded", BACKUP_CHECKPOINTS),
        ] {
            let mut checkpoints = 0;
            let started = std::time::Instant::now();
            let inventory = export_with_checkpoints(&store, &dir.path().join(name), policy, |p| {
                if p.checkpoint == LogicalArchiveCheckpoint::ReceiptDurable {
                    checkpoints += 1;
                }
                Ok(())
            })
            .unwrap();
            println!("archive_checkpoint_measurement mode={name} actions={} checkpoints={checkpoints} elapsed_ns={} qualification=false", inventory.action_count, started.elapsed().as_nanos());
            if let Some(expected) = &baseline {
                assert_eq!(&inventory, expected);
            } else {
                baseline = Some(inventory);
            }
        }
    }
}
