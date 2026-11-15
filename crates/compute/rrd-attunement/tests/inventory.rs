use rrd_attunement::inventory::*;

fn scope() -> SourceScope {
    SourceScope {
        engram: "engram-one".into(),
        project: "project-one".into(),
        root: "source".into(),
    }
}
fn revisions() -> Revisions {
    Revisions {
        policy_sha256: "1".repeat(64),
        walker: "walker-one".into(),
        detector: "detector-one".into(),
    }
}
fn limits() -> Limits {
    Limits {
        entries: 100,
        depth: 8,
        path_bytes: 512,
        metadata_bytes: 256,
        file_bytes: 128,
        content_bytes: 512,
    }
}
fn file(bytes: u64, revision: &str) -> FileObservation {
    FileObservation {
        bytes,
        executable: false,
        revision: revision.into(),
        classification_sha256: "2".repeat(64),
    }
}
fn builder(prior: Option<&Proposal>) -> Builder<'_> {
    Builder::new(scope(), revisions(), limits(), prior).unwrap()
}
fn sample(reverse: bool) -> Proposal {
    let mut b = builder(None);
    if reverse {
        b.file("src/a.rs", file(3, "a"), [b"abc"]).unwrap();
        b.file("README.md", file(0, "readme"), std::iter::empty::<&[u8]>())
            .unwrap();
        b.directory("src").unwrap();
    } else {
        b.directory("src").unwrap();
        b.file("README.md", file(0, "readme"), std::iter::empty::<&[u8]>())
            .unwrap();
        b.file("src/a.rs", file(3, "a"), [b"abc"]).unwrap();
    }
    b.finish(true).unwrap()
}
fn entry<'a>(p: &'a Proposal, path: &str) -> &'a Entry {
    p.entries().iter().find(|e| e.path == path).unwrap()
}

#[test]
fn committed_reconstruction_requires_the_expected_digest_and_valid_tree() {
    let original = sample(false);
    let encoded = serde_json::to_vec(&original).unwrap();
    let restored =
        Proposal::from_committed_bytes(&encoded, original.proposal_sha256(), encoded.len())
            .unwrap();
    assert_eq!(restored, original);
    assert!(Proposal::from_committed_bytes(&encoded, &"0".repeat(64), encoded.len()).is_err());
    assert!(Proposal::from_committed_bytes(
        &encoded,
        original.proposal_sha256(),
        encoded.len() - 1
    )
    .is_err());
    let mut tampered = serde_json::to_value(&original).unwrap();
    tampered["entries"][0]["identity"] = "0".repeat(64).into();
    assert!(Proposal::from_committed_bytes(
        &serde_json::to_vec(&tampered).unwrap(),
        original.proposal_sha256(),
        encoded.len() * 2
    )
    .is_err());
}

#[test]
fn fallible_content_cannot_become_eof_or_implicitly_reuse_metadata() {
    let prior = sample(false);
    let mut b = builder(Some(&prior));
    assert_eq!(
        b.file_fallible("src/a.rs", file(3, "a"), [Err::<&[u8], ()>(())], false),
        Err(InventoryError::ContentRead)
    );
    assert!(!b.finish(false).unwrap().is_complete());
    let mut b = builder(Some(&prior));
    b.directory("src").unwrap();
    b.file_fallible("src/a.rs", file(3, "a"), [Ok::<_, ()>(b"def")], false)
        .unwrap();
    let result = b.finish(true).unwrap();
    assert_eq!(result.accounting().reused_files, 0);
    assert_eq!(result.accounting().content_bytes, 3);
    assert_ne!(
        entry(&result, "src/a.rs").subtree_sha256,
        entry(&prior, "src/a.rs").subtree_sha256
    );
}

#[test]
fn traversal_order_and_chunk_partition_do_not_change_the_proposal() {
    assert_eq!(sample(false), sample(true));
    let mut b = builder(None);
    b.directory("src").unwrap();
    b.file("README.md", file(0, "readme"), std::iter::empty::<&[u8]>())
        .unwrap();
    b.file("src/a.rs", file(3, "a"), [b"a".as_slice(), b"bc"])
        .unwrap();
    let proposal = b.finish(true).unwrap();
    assert_eq!(proposal, sample(false));
    // Independent NIST SHA-256 'abc' vector, not computed by this builder.
    let EntryKind::File { content_sha256, .. } = &entry(&proposal, "src/a.rs").kind else {
        panic!()
    };
    assert_eq!(
        content_sha256,
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn unchanged_refresh_does_not_request_content_and_preserves_identity() {
    let before = sample(false);
    let mut b = builder(Some(&before));
    b.directory("src").unwrap();
    let must_not_read = || {
        std::iter::from_fn(|| -> Option<&'static [u8]> {
            panic!("content requested on unchanged file")
        })
    };
    b.file("src/a.rs", file(3, "a"), must_not_read()).unwrap();
    b.file("README.md", file(0, "readme"), must_not_read())
        .unwrap();
    let after = b.finish(true).unwrap();
    assert_eq!(before.entries(), after.entries());
    assert_eq!(before.root_sha256(), after.root_sha256());
    assert_eq!(after.changes(), &Changes::default());
    assert_eq!(after.accounting().content_bytes, 0);
    assert_eq!(after.accounting().reused_files, 2);
    assert_ne!(before.proposal_sha256(), after.proposal_sha256());
}

#[test]
fn edit_updates_only_file_and_ancestor_digests() {
    let before = sample(false);
    let mut b = builder(Some(&before));
    b.directory("src").unwrap();
    b.file("README.md", file(0, "readme"), std::iter::empty::<&[u8]>())
        .unwrap();
    b.file("src/a.rs", file(3, "changed"), [b"xyz"]).unwrap();
    let after = b.finish(true).unwrap();
    assert_eq!(entry(&before, "README.md"), entry(&after, "README.md"));
    assert_eq!(
        entry(&before, "src/a.rs").identity,
        entry(&after, "src/a.rs").identity
    );
    assert_eq!(
        after
            .changes()
            .modified
            .iter()
            .map(|c| c.path.as_str())
            .collect::<Vec<_>>(),
        ["", "src", "src/a.rs"]
    );
    assert_eq!(after.accounting().content_bytes, 3);
}

#[test]
fn deletion_and_unique_rename_preserve_add_remove_evidence() {
    let before = sample(false);
    let mut b = builder(Some(&before));
    b.file("renamed.rs", file(3, "new-object"), [b"abc"])
        .unwrap();
    let after = b.finish(true).unwrap();
    assert_eq!(
        after
            .changes()
            .removed
            .iter()
            .map(|c| c.path.as_str())
            .collect::<Vec<_>>(),
        ["README.md", "src", "src/a.rs"]
    );
    assert_eq!(after.changes().added[0].path, "renamed.rs");
    assert_eq!(after.changes().rename_candidates.len(), 1);
    let candidate = &after.changes().rename_candidates[0];
    assert_eq!(
        candidate.removed_identity,
        entry(&before, "src/a.rs").identity
    );
    assert_eq!(
        candidate.added_identity,
        entry(&after, "renamed.rs").identity
    );
    assert_ne!(candidate.removed_identity, candidate.added_identity);
}

#[test]
fn ambiguous_equal_content_never_guesses_a_rename() {
    let before = sample(false);
    let mut b = builder(Some(&before));
    b.file("one", file(3, "one"), [b"abc"]).unwrap();
    b.file("two", file(3, "two"), [b"abc"]).unwrap();
    assert!(b
        .finish(true)
        .unwrap()
        .changes()
        .rename_candidates
        .is_empty());
}

#[test]
fn each_semantic_revision_invalidates_reuse() {
    let before = sample(false);
    for changed in 0..3 {
        let mut rev = revisions();
        match changed {
            0 => rev.policy_sha256 = "3".repeat(64),
            1 => rev.walker = "walker-other".into(),
            _ => rev.detector = "detector-other".into(),
        }
        let mut b = Builder::new(scope(), rev, limits(), Some(&before)).unwrap();
        b.file("README.md", file(0, "readme"), std::iter::empty::<&[u8]>())
            .unwrap();
        b.directory("src").unwrap();
        b.file("src/a.rs", file(3, "a"), [b"abc"]).unwrap();
        let after = b.finish(true).unwrap();
        assert_eq!(after.accounting().reused_files, 0);
        assert_eq!(after.accounting().content_bytes, 3);
        assert_ne!(after.root_sha256(), before.root_sha256());
    }
}

#[test]
fn metadata_replacement_rehashes_even_when_length_matches() {
    let before = sample(false);
    let mut b = builder(Some(&before));
    b.directory("src").unwrap();
    b.file("src/a.rs", file(3, "replacement"), [b"xyz"])
        .unwrap();
    let after = b.finish(true).unwrap();
    assert_eq!(after.accounting().reused_files, 0);
    assert_ne!(
        entry(&before, "src/a.rs").subtree_sha256,
        entry(&after, "src/a.rs").subtree_sha256
    );
}

#[test]
fn incomplete_enumeration_and_explicit_errors_cannot_be_reused() {
    for has_error in [false, true] {
        let mut b = builder(None);
        if has_error {
            b.leaf(
                "gone",
                EntryKind::Error {
                    class: ErrorClass::Vanished,
                    retryable: true,
                },
            )
            .unwrap();
        }
        let partial = b.finish(has_error).unwrap();
        assert!(!partial.is_complete());
        assert!(matches!(
            Builder::new(scope(), revisions(), limits(), Some(&partial)),
            Err(InventoryError::IncompletePrevious)
        ));
        if has_error {
            assert_eq!(partial.accounting().errors, 1);
        }
    }
}

#[test]
fn rejects_escaped_ambiguous_duplicate_and_uncontained_paths() {
    for path in [
        "",
        "/etc",
        "../escape",
        "a/../b",
        "a//b",
        "a/",
        "./a",
        "C:/host",
        "a\\b",
        "a\0b",
    ] {
        assert_eq!(
            builder(None).directory(path),
            Err(InventoryError::InvalidPath)
        );
    }
    let mut b = builder(None);
    b.directory("a").unwrap();
    assert_eq!(b.directory("a"), Err(InventoryError::DuplicatePath));
    let mut b = builder(None);
    b.directory("missing/child").unwrap();
    assert_eq!(b.finish(true), Err(InventoryError::MissingParent));
    let mut b = builder(None);
    b.leaf(
        "link",
        EntryKind::Symlink {
            rule: "no-follow".into(),
        },
    )
    .unwrap();
    b.directory("link/child").unwrap();
    assert_eq!(b.finish(true), Err(InventoryError::InvalidParent));
}

#[test]
fn policy_leaves_retain_decisions_without_file_contents_or_targets() {
    let mut b = builder(None);
    b.directory(".github").unwrap();
    b.leaf(
        ".env",
        EntryKind::Excluded {
            rule: "secret-path".into(),
        },
    )
    .unwrap();
    b.leaf(
        "link",
        EntryKind::Symlink {
            rule: "no-follow".into(),
        },
    )
    .unwrap();
    b.leaf(
        "mount",
        EntryKind::Mount {
            rule: "no-cross-device".into(),
        },
    )
    .unwrap();
    let p = b.finish(true).unwrap();
    assert!(p.is_complete());
    assert_eq!(p.accounting().content_bytes, 0);
    assert_eq!(p.entries().len(), 5);
}

#[test]
fn resource_limits_are_checked_before_content_is_requested() {
    let mut cap = limits();
    cap.entries = 1;
    assert_eq!(
        Builder::new(scope(), revisions(), cap, None)
            .unwrap()
            .directory("a"),
        Err(InventoryError::EntryLimit)
    );
    let mut b = builder(None);
    let should_not_read = std::iter::from_fn(|| -> Option<&[u8]> { panic!("read beyond bound") });
    assert_eq!(
        b.file("huge", file(129, "huge"), should_not_read),
        Err(InventoryError::ContentLimit)
    );
    let mut cap = limits();
    cap.depth = 1;
    assert_eq!(
        Builder::new(scope(), revisions(), cap, None)
            .unwrap()
            .directory("a/b"),
        Err(InventoryError::InvalidPath)
    );
    let mut cap = limits();
    cap.path_bytes = 1;
    assert_eq!(
        Builder::new(scope(), revisions(), cap, None)
            .unwrap()
            .directory("ab"),
        Err(InventoryError::InvalidPath)
    );
    let mut cap = limits();
    cap.content_bytes = 128;
    let mut b = Builder::new(scope(), revisions(), cap, None).unwrap();
    b.file("a", file(128, "a"), [[0; 128]]).unwrap();
    assert_eq!(
        b.file("b", file(1, "b"), [b"b"]),
        Err(InventoryError::ContentLimit)
    );
}

#[test]
fn rejected_file_cannot_be_silently_omitted_from_complete_proposal() {
    let mut b = builder(None);
    assert_eq!(
        b.file("truncated", file(4, "source"), [b"abc"]),
        Err(InventoryError::ContentLength)
    );
    let rejected = b.finish(true).unwrap();
    assert!(!rejected.is_complete());
    assert_eq!(rejected.accounting().content_bytes, 3);
    assert_eq!(rejected.accounting().rejected_observations, 1);
}

#[test]
fn stalled_chunk_source_is_rejected_after_one_call() {
    let mut calls = 0;
    let chunks = std::iter::from_fn(|| {
        calls += 1;
        Some(b"".as_slice())
    });
    let mut b = builder(None);
    assert_eq!(
        b.file("stalled", file(1, "one"), chunks),
        Err(InventoryError::ContentLength)
    );
    assert_eq!(calls, 1);
    assert!(!b.finish(true).unwrap().is_complete());
}

#[test]
fn failed_reads_still_consume_the_shared_byte_allowance() {
    let mut cap = limits();
    cap.file_bytes = 3;
    cap.content_bytes = 3;
    let mut b = Builder::new(scope(), revisions(), cap, None).unwrap();
    assert_eq!(
        b.file("short", file(3, "one"), [b"ab"]),
        Err(InventoryError::ContentLength)
    );
    assert_eq!(
        b.file("retry", file(2, "two"), [b"cd"]),
        Err(InventoryError::ContentLimit)
    );
    let p = b.finish(true).unwrap();
    assert_eq!(p.accounting().content_bytes, 2);
    assert_eq!(p.accounting().rejected_observations, 2);
    assert!(!p.is_complete());
}

#[test]
fn invalid_admission_and_changed_limits_cannot_reuse_previous_output() {
    let before = sample(false);
    let mut cap = limits();
    cap.content_bytes -= 1;
    let mut b = Builder::new(scope(), revisions(), cap, Some(&before)).unwrap();
    b.directory("src").unwrap();
    b.file("src/a.rs", file(3, "a"), [b"abc"]).unwrap();
    let p = b.finish(true).unwrap();
    assert_eq!(p.accounting().reused_files, 0);
    assert_eq!(p.accounting().content_bytes, 3);
    let mut invalid = revisions();
    invalid.policy_sha256 = "unknown".into();
    assert!(matches!(
        Builder::new(scope(), invalid, limits(), None),
        Err(InventoryError::InvalidRevision)
    ));
    let mut invalid = scope();
    invalid.root = "C:/host".into();
    assert!(matches!(
        Builder::new(invalid, revisions(), limits(), None),
        Err(InventoryError::InvalidScope)
    ));
    let mut b = builder(None);
    b.directory("a").unwrap();
    assert_eq!(b.directory("a"), Err(InventoryError::DuplicatePath));
    assert!(!b.finish(true).unwrap().is_complete());
}

#[test]
fn a_caller_cannot_inject_an_uncomputed_file_digest() {
    let mut b = builder(None);
    assert_eq!(
        b.leaf(
            "forged",
            EntryKind::File {
                observation: file(3, "one"),
                content_sha256: "0".repeat(64)
            }
        ),
        Err(InventoryError::InvalidObservation)
    );
    assert!(!b.finish(true).unwrap().is_complete());
}

#[test]
fn prior_source_cannot_cross_engram_project_or_root() {
    let before = sample(false);
    for field in 0..3 {
        let mut source = scope();
        match field {
            0 => source.engram = "other".into(),
            1 => source.project = "other".into(),
            _ => source.root = "other".into(),
        }
        assert!(matches!(
            Builder::new(source, revisions(), limits(), Some(&before)),
            Err(InventoryError::ForeignPrevious)
        ));
    }
}

#[test]
fn even_an_empty_root_digest_is_bound_to_its_source_scope() {
    let before = builder(None).finish(true).unwrap();
    let mut other = scope();
    other.engram = "other-engram".into();
    let after = Builder::new(other, revisions(), limits(), None)
        .unwrap()
        .finish(true)
        .unwrap();
    assert_ne!(before.root_sha256(), after.root_sha256());
    assert_ne!(before.entries()[0].identity, after.entries()[0].identity);
}

#[test]
fn metadata_only_change_is_reobserved_without_inventing_content_changes() {
    let before = sample(false);
    let mut b = builder(Some(&before));
    b.directory("src").unwrap();
    b.file("README.md", file(0, "readme"), std::iter::empty::<&[u8]>())
        .unwrap();
    b.file("src/a.rs", file(3, "same-bytes-new-observation"), [b"abc"])
        .unwrap();
    let after = b.finish(true).unwrap();
    assert_eq!(after.accounting().content_bytes, 3);
    assert_eq!(after.root_sha256(), before.root_sha256());
    assert_eq!(after.changes(), &Changes::default());
    assert_ne!(after.proposal_sha256(), before.proposal_sha256());
}
