use rrd_core::{Claim, Predicate, Producer, RuntimeCommit, RuntimeMutation, ScopeId, Subject};
use rrd_store::{
    filesystem::AnchoredPath, ControlTransition, DatabaseOptions, Durability, Error, MutableLimits,
    MutableResource, RrflowKvInspector, RrflowKvStore, StorageEngine,
};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

fn options(limits: MutableLimits) -> DatabaseOptions {
    DatabaseOptions {
        mutable_limits: limits,
        ..Default::default()
    }
}

fn assert_limit(error: Error, expected: MutableResource, expected_limit: usize) {
    assert!(
        matches!(error, Error::MutableLimit { resource, limit, observed }
        if resource == expected && limit == expected_limit && observed > limit),
        "{error}"
    );
}

fn files(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn visit(root: &Path, path: &Path, result: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in std::fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            let kind = entry.file_type().unwrap();
            assert!(!kind.is_symlink());
            if kind.is_dir() {
                visit(root, &entry.path(), result);
            } else {
                result.insert(
                    entry.path().strip_prefix(root).unwrap().to_owned(),
                    std::fs::read(entry.path()).unwrap(),
                );
            }
        }
    }
    let mut result = BTreeMap::new();
    visit(root, root, &mut result);
    result
}

fn rows(store: &RrflowKvStore) -> Vec<(Vec<u8>, Vec<u8>)> {
    let result = store
        .begin_transaction()
        .unwrap()
        .scan(&[0], &[255], 1024)
        .unwrap();
    assert!(result.len() < 1024);
    result
}

#[test]
fn physical_transaction_rejection_preserves_prior_ack_and_allows_subsequent_work() {
    for (limits, resource, limit) in [
        (
            MutableLimits {
                accounted_bytes: 1024,
                versions: 128,
            },
            MutableResource::AccountedBytes,
            1024,
        ),
        (
            MutableLimits {
                accounted_bytes: 64 * 1024,
                versions: 4,
            },
            MutableResource::Versions,
            4,
        ),
    ] {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().join("store");
        let store = RrflowKvStore::create_new_with_options(&root, options(limits)).unwrap();
        let mut seed = store.begin_transaction().unwrap();
        seed.put(b"ack".to_vec(), b"original".to_vec()).unwrap();
        seed.commit(Durability::Authoritative).unwrap();
        let before = rows(&store);
        let mut denied = store.begin_transaction().unwrap();
        denied.delete(b"ack".to_vec()).unwrap();
        for index in 0..4 {
            denied
                .put(format!("new-{index}").into_bytes(), vec![index; 512])
                .unwrap();
        }
        assert_limit(
            denied.commit(Durability::Authoritative).unwrap_err(),
            resource,
            limit,
        );
        assert_eq!(rows(&store), before);
        drop(store);
        let store = RrflowKvStore::open_existing_with_options(&root, options(limits)).unwrap();
        assert_eq!(rows(&store), before);
        let mut valid = store.begin_transaction().unwrap();
        valid.delete(b"ack".to_vec()).unwrap();
        valid.put(b"next".to_vec(), b"accepted".to_vec()).unwrap();
        valid.commit(Durability::Authoritative).unwrap();
        drop(store);
        let store = RrflowKvStore::open_existing_with_options(&root, options(limits)).unwrap();
        assert_eq!(rows(&store), vec![(b"next".to_vec(), b"accepted".to_vec())]);
    }
}

#[test]
fn open_and_read_only_inspection_keep_typed_replay_limits_and_preserve_files() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("inspect");
    let store = RrflowKvStore::create_new(&root).unwrap();
    let scope = ScopeId::new("instance:inspection-limit").unwrap();
    let request = RuntimeCommit {
        scope: scope.clone(),
        at: 100,
        actor: "inspection-oracle".into(),
        expected_cursor: 0,
        mutations: vec![RuntimeMutation::Claim {
            claim: Claim::new(
                Subject::new("document").unwrap(),
                Predicate::new("status").unwrap(),
                "acknowledged",
                100,
                100,
                Producer {
                    actor: "inspection-oracle".into(),
                    on_behalf_of: None,
                    session: None,
                },
            ),
        }],
    };
    let transition = ControlTransition {
        key: "server/state/inspection/record".into(),
        expected: None,
        replacement: Some(b"owned-control".to_vec()),
        at: 100,
        actor: "inspection-oracle".into(),
        action: "inspection.seed".into(),
        request_id: "seed".into(),
        operation_id: "seed".into(),
    };
    store
        .runtime()
        .commit_with_control_transitions(&request, std::slice::from_ref(&transition))
        .unwrap();
    let stamp = store.runtime().read_stamp(&scope).unwrap();
    drop(store);
    let before = files(&root);
    let anchor = AnchoredPath::open_root(&root).unwrap();
    for (limits, resource) in [
        (
            MutableLimits {
                accounted_bytes: 1,
                ..Default::default()
            },
            MutableResource::AccountedBytes,
        ),
        (
            MutableLimits {
                versions: 1,
                ..Default::default()
            },
            MutableResource::Versions,
        ),
    ] {
        assert_limit(
            RrflowKvStore::open_with_options(&root, options(limits))
                .err()
                .expect("deny generic open"),
            resource,
            1,
        );
        assert_eq!(files(&root), before);
        assert_limit(
            RrflowKvStore::open_existing_with_options(&root, options(limits))
                .err()
                .expect("deny explicit open"),
            resource,
            1,
        );
        assert_eq!(files(&root), before);
        assert_limit(
            RrflowKvInspector::open_with_options(&root, options(limits))
                .err()
                .expect("deny inspection"),
            resource,
            1,
        );
        assert_eq!(files(&root), before);
        assert_limit(
            RrflowKvInspector::open_anchored_with_options(&anchor, options(limits))
                .err()
                .expect("deny anchored inspection"),
            resource,
            1,
        );
        assert_eq!(files(&root), before);
    }
    for limits in [
        MutableLimits {
            versions: 0,
            ..Default::default()
        },
        MutableLimits {
            accounted_bytes: 0,
            ..Default::default()
        },
    ] {
        assert!(RrflowKvInspector::open_with_options(&root, options(limits)).is_err());
        assert_eq!(files(&root), before);
    }
    let adequate = MutableLimits {
        accounted_bytes: 1024 * 1024,
        versions: 4096,
    };
    for anchored in [false, true] {
        let inspector = if anchored {
            RrflowKvInspector::open_anchored_with_options(&anchor, options(adequate))
        } else {
            RrflowKvInspector::open_with_options(&root, options(adequate))
        }
        .unwrap();
        assert_eq!(inspector.runtime_read_stamp(&scope).unwrap(), stamp);
        assert_eq!(inspector.inspection().claim_sequence, 1);
        assert_eq!(inspector.inspection().runtime_cursor, 1);
        assert_eq!(inspector.inspection().control_journal_sequence, 1);
        assert_eq!(
            inspector.control_record(&transition.key).unwrap(),
            transition.replacement
        );
        drop(inspector);
        assert_eq!(files(&root), before);
    }
    let reopened = RrflowKvStore::open_existing_with_options(&root, options(adequate)).unwrap();
    assert_eq!(reopened.runtime().read_stamp(&scope).unwrap(), stamp);
    assert_eq!(
        reopened.control().get(&transition.key).unwrap(),
        transition.replacement
    );
    assert!(reopened
        .runtime()
        .commit_outcome(&request.digest())
        .unwrap()
        .is_some());
}
