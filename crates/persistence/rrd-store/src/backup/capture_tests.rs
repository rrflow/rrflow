use super::*;
use crate::{StorageTransaction, TransactionCommit, TransactionRollback};
use std::sync::atomic::AtomicUsize;

struct Counted<'a> {
    inner: &'a dyn StorageEngine,
    journal_rows: AtomicUsize,
}
struct CountedRead<'a> {
    inner: Box<dyn StorageTransaction + 'a>,
    rows: &'a AtomicUsize,
}
impl StorageTransaction for CountedRead<'_> {
    fn snapshot_sequence(&self) -> u64 {
        self.inner.snapshot_sequence()
    }
    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        self.inner.get(key)
    }
    fn scan(&self, start: &[u8], end: &[u8], limit: usize) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        let rows = self.inner.scan(start, end, limit)?;
        let prefix = checked_key(keyspaces::META, &keyspaces::control_journal_prefix())?;
        self.rows.fetch_add(
            rows.iter()
                .filter(|(key, _)| key.starts_with(&prefix))
                .count(),
            Ordering::Relaxed,
        );
        Ok(rows)
    }
    fn put(&mut self, key: Vec<u8>, value: Vec<u8>) -> Result<()> {
        self.inner.put(key, value)
    }
    fn delete(&mut self, key: Vec<u8>) -> Result<()> {
        self.inner.delete(key)
    }
    fn commit(self: Box<Self>, durability: Durability) -> Result<TransactionCommit> {
        self.inner.commit(durability)
    }
    fn rollback(self: Box<Self>) -> Result<TransactionRollback> {
        self.inner.rollback()
    }
}
impl StorageEngine for Counted<'_> {
    fn begin_transaction(&self) -> Result<Box<dyn StorageTransaction + '_>> {
        Ok(Box::new(CountedRead {
            inner: self.inner.begin_transaction()?,
            rows: &self.journal_rows,
        }))
    }
    fn claims(&self) -> crate::ClaimRepository<'_> {
        crate::ClaimRepository::new(self)
    }
    fn control(&self) -> crate::ControlRepository<'_> {
        crate::ControlRepository::new(self)
    }
    fn function_catalogue(&self) -> crate::FunctionCatalogueRepository<'_> {
        crate::FunctionCatalogueRepository::new(self)
    }
    fn projections(&self) -> crate::ProjectionRepository<'_> {
        crate::ProjectionRepository::new(self)
    }
    fn runtime(&self) -> crate::RuntimeRepository<'_> {
        crate::RuntimeRepository::new(self)
    }
    fn invocations(&self) -> crate::InvocationRepository<'_> {
        crate::InvocationRepository::new(self)
    }
}

fn exercise(source: &impl StorageEngine) {
    let root = tempfile::tempdir().unwrap();
    let objects = LocalObjectStore::open(root.path().join("objects")).unwrap();
    let live = ScopeId::new("instance:live").unwrap();
    let removed = ScopeId::new("instance:removed").unwrap();
    let live_key = format!("server/state/index-catalogue/{}", live.as_str());
    let removed_key = format!("server/state/index-catalogue/{}", removed.as_str());
    for (scope, key, versions) in [
        (
            &live,
            &live_key,
            vec![
                Some(b"v1".as_slice()),
                Some(b"v2".as_slice()),
                None,
                Some(b"v3".as_slice()),
            ],
        ),
        (&removed, &removed_key, vec![Some(b"old".as_slice()), None]),
    ] {
        let mut before = None;
        for (index, after) in versions.into_iter().enumerate() {
            source
                .control()
                .commit_catalog(
                    scope,
                    &tests::transition(key, before, after, index as u64 + 1),
                )
                .unwrap();
            before = after;
        }
    }
    source
        .control()
        .commit_catalog(
            &removed,
            &tests::transition(
                &format!(
                    "server/state/vector-collection-catalogue/{}",
                    removed.as_str()
                ),
                None,
                Some(b"retained"),
                10,
            ),
        )
        .unwrap();
    source
        .control()
        .commit(&tests::transition(
            "server/state/install/record",
            None,
            Some(b"installed"),
            20,
        ))
        .unwrap();
    let mut counts = Vec::new();
    for growth in [0, 96] {
        for index in 0..growth {
            source
                .control()
                .commit(&tests::transition(
                    &format!("server/state/one/session/{index}"),
                    None,
                    Some(&[b'x'; 1024]),
                    30 + index,
                ))
                .unwrap();
        }
        let head = source.control().sequence().unwrap();
        let counted = Counted {
            inner: source,
            journal_rows: AtomicUsize::new(0),
        };
        let catalogue = root.path().join(format!("catalogue-{growth}"));
        let entry = create_application_backup(
            &counted,
            &objects,
            &catalogue,
            "single-pass",
            200,
            &tests::families(),
        )
        .unwrap();
        counts.push((head as usize, counted.journal_rows.load(Ordering::Relaxed)));
        // Recorded from the original two-pass implementation before replacement.
        // The backup identity binds every retained artifact's digest.
        assert_eq!(
            entry.backup_id,
            if growth == 0 {
                "9097d42dd0a6adc1f3f69ed82a3c47da199222a54000d3516205ab5f08fa7f4f"
            } else {
                "53a0af37d1870a209c7cd937cda9e1e0d03c81746e3ed6f057bb27eb4dd6c446"
            }
        );
        println!(
            "capture_fixture growth={growth} backup={} catalogue={} control={}",
            entry.backup_id,
            entry.catalogue_manifest.as_ref().unwrap().manifest_sha256,
            entry.control_archive.as_ref().unwrap().archive_sha256
        );
        assert_eq!(entry.control_archive.as_ref().unwrap().entry_count, 1);
        assert_eq!(entry.catalogue_manifest.as_ref().unwrap().scope_count, 2);
        assert_eq!(entry.catalogue_manifest.as_ref().unwrap().record_count, 2);
        assert_eq!(
            verify_backup_catalogue(&catalogue).unwrap().backups,
            vec![entry.clone()]
        );
        let target = root.path().join(format!("restored-{growth}"));
        restore_catalogued_backup(&catalogue, &entry.backup_id, &target, 300).unwrap();
        let restored = RrflowKvStore::open_existing(&target).unwrap();
        assert_eq!(
            restored.control().get(&live_key).unwrap(),
            Some(b"v3".to_vec())
        );
        assert_eq!(restored.control().get(&removed_key).unwrap(), None);
        assert_eq!(
            restored
                .runtime()
                .read_stamp(&live)
                .unwrap()
                .catalog_revision,
            4
        );
        assert_eq!(
            restored
                .runtime()
                .read_stamp(&removed)
                .unwrap()
                .catalog_revision,
            3
        );
        assert_eq!(
            restored
                .control()
                .get("server/state/install/record")
                .unwrap(),
            Some(b"installed".to_vec())
        );
        assert_eq!(
            restored
                .control()
                .get("server/state/one/session/0")
                .unwrap(),
            None
        );
    }
    for (expected, read) in counts {
        assert_eq!(read, expected, "backup must scan each journal row once");
    }
}

#[test]
fn application_backup_captures_control_and_catalogues_once() {
    exercise(&crate::RrflowMxStore::new());
    let root = tempfile::tempdir().unwrap();
    let source = RrflowKvStore::open(root.path()).unwrap();
    exercise(&source);
}
