use super::*;
use serde::Deserialize;
use std::collections::BTreeMap;

fn generation_budget() -> SourceGenerationProofBudget {
    SourceGenerationProofBudget {
        max_identity_bytes: 256,
        max_path_nodes: 64,
    }
}

fn verify_source_generation_at<'a>(
    scope: &ScopeId,
    project: &RuntimeId,
    cursor: u64,
    root: &str,
    size: u64,
    left: Option<&'a SourceGenerationNeighbor>,
    right: Option<&SourceGenerationNeighbor>,
) -> Result<Option<&'a SourceGenerationSelection>> {
    super::verify_source_generation_at(
        scope,
        project,
        cursor,
        root,
        size,
        left,
        right,
        generation_budget(),
    )
}

#[test]
fn ordered_generation_neighbors_prove_the_selection_and_reject_omissions() {
    let scope = ScopeId::new("instance:test").unwrap();
    let project = RuntimeId::new("project-test").unwrap();
    let mut history =
        SourceGenerationHistory::new(scope.clone(), project.clone(), generation_budget()).unwrap();
    let mut nodes = BTreeMap::new();
    let selections = [5_u64, 10, 30, 40].map(|cursor| SourceGenerationSelection {
        scope: scope.clone(),
        project: project.clone(),
        completed_cursor: cursor,
        generation_sha256: format!("{cursor:064x}"),
    });
    for selection in &selections {
        for node in history.append(selection).unwrap() {
            nodes.insert((node.level, node.index), node.digest);
        }
    }
    let root = history.accumulator().root.clone();
    let size = history.accumulator().tree_size;
    let neighbor = |index: usize| SourceGenerationNeighbor {
        selection: selections[index].clone(),
        inclusion: history
            .accumulator()
            .inclusion_proof(index as u64, |level, offset| {
                Ok(nodes.get(&(level, offset)).cloned())
            })
            .unwrap(),
    };
    let first = neighbor(0);
    let second = neighbor(1);
    let third = neighbor(2);
    let last = neighbor(3);
    let small_path_budget = SourceGenerationProofBudget {
        max_identity_bytes: 256,
        max_path_nodes: 1,
    };
    assert!(super::verify_source_generation_at(
        &scope,
        &project,
        40,
        &root,
        size,
        Some(&last),
        None,
        small_path_budget
    )
    .is_err());
    assert_eq!(
        verify_source_generation_at(&scope, &project, 4, &root, size, None, Some(&first)).unwrap(),
        None
    );
    assert_eq!(
        verify_source_generation_at(
            &scope,
            &project,
            5,
            &root,
            size,
            Some(&first),
            Some(&second)
        )
        .unwrap(),
        Some(&selections[0])
    );
    assert_eq!(
        verify_source_generation_at(
            &scope,
            &project,
            29,
            &root,
            size,
            Some(&second),
            Some(&third)
        )
        .unwrap(),
        Some(&selections[1])
    );
    assert_eq!(
        verify_source_generation_at(&scope, &project, 40, &root, size, Some(&last), None).unwrap(),
        Some(&selections[3])
    );
    // A valid inclusion proof for an older event cannot hide a later one.
    assert!(
        verify_source_generation_at(&scope, &project, 35, &root, size, Some(&first), None).is_err()
    );
    assert!(verify_source_generation_at(
        &scope,
        &project,
        35,
        &root,
        size,
        Some(&second),
        Some(&last)
    )
    .is_err());
    assert!(verify_source_generation_at(
        &scope,
        &project,
        35,
        &root,
        size,
        Some(&third),
        Some(&second)
    )
    .is_err());
    assert!(
        verify_source_generation_at(&scope, &project, 5, &root, size, None, Some(&second)).is_err()
    );
    assert!(verify_source_generation_at(&scope, &project, 35, &root, size, None, None).is_err());
    assert!(verify_source_generation_at(
        &scope,
        &project,
        35,
        &root,
        size + 1,
        Some(&third),
        Some(&last)
    )
    .is_err());
    assert!(verify_source_generation_at(
        &scope,
        &project,
        35,
        &"00".repeat(32),
        size,
        Some(&third),
        Some(&last)
    )
    .is_err());
    let mut tampered = third.clone();
    tampered.selection.generation_sha256 = "ff".repeat(32);
    assert!(verify_source_generation_at(
        &scope,
        &project,
        35,
        &root,
        size,
        Some(&tampered),
        Some(&last)
    )
    .is_err());
    let other = RuntimeId::new("project-other").unwrap();
    assert!(verify_source_generation_at(
        &scope,
        &other,
        35,
        &root,
        size,
        Some(&third),
        Some(&last)
    )
    .is_err());
    let empty = RuntimeLogAccumulator::new();
    assert_eq!(
        verify_source_generation_at(&scope, &project, 0, &empty.root, 0, None, None).unwrap(),
        None
    );
    assert!(verify_source_generation_at(&scope, &project, 0, &root, 0, None, None).is_err());
}

#[test]
fn generation_history_rejects_unordered_or_foreign_appends_without_mutation() {
    let scope = ScopeId::new("instance:test").unwrap();
    let project = RuntimeId::new("project-test").unwrap();
    let mut history =
        SourceGenerationHistory::new(scope.clone(), project.clone(), generation_budget()).unwrap();
    let selection = SourceGenerationSelection {
        scope,
        project,
        completed_cursor: 8,
        generation_sha256: "ab".repeat(32),
    };
    history.append(&selection).unwrap();
    let before = history.accumulator().clone();
    assert!(history.append(&selection).is_err());
    let mut earlier = selection.clone();
    earlier.completed_cursor = 7;
    assert!(history.append(&earlier).is_err());
    let mut foreign = selection.clone();
    foreign.project = RuntimeId::new("project-other").unwrap();
    foreign.completed_cursor = 9;
    assert!(history.append(&foreign).is_err());
    let mut malformed = selection.clone();
    malformed.completed_cursor = 9;
    malformed.generation_sha256 = "AA".repeat(32);
    assert!(history.append(&malformed).is_err());
    assert_eq!(history.accumulator(), &before);
}

#[test]
fn generation_proof_budgets_are_explicit_and_fail_closed() {
    let scope = ScopeId::new("instance:test").unwrap();
    let project = RuntimeId::new("project-test").unwrap();
    let none = SourceGenerationProofBudget {
        max_identity_bytes: 0,
        max_path_nodes: 64,
    };
    assert!(SourceGenerationHistory::new(scope.clone(), project.clone(), none).is_err());
    let too_many = SourceGenerationProofBudget {
        max_identity_bytes: 256,
        max_path_nodes: 65,
    };
    assert!(SourceGenerationHistory::new(scope.clone(), project.clone(), too_many).is_err());
    let tight = SourceGenerationProofBudget {
        max_identity_bytes: 3,
        max_path_nodes: 1,
    };
    assert!(SourceGenerationHistory::new(scope.clone(), project.clone(), tight).is_err());
    let history =
        SourceGenerationHistory::new(scope.clone(), project.clone(), generation_budget()).unwrap();
    assert!(super::verify_source_generation_at(
        &scope,
        &project,
        0,
        &history.accumulator().root,
        0,
        None,
        None,
        none
    )
    .is_err());
    assert!(super::verify_source_generation_at(
        &scope,
        &project,
        0,
        &history.accumulator().root,
        0,
        None,
        None,
        tight
    )
    .is_err());
}

#[test]
fn generation_writer_reopens_from_authenticated_tail_and_preserves_order() {
    let scope = ScopeId::new("instance:test").unwrap();
    let project = RuntimeId::new("project-test").unwrap();
    let mut original =
        SourceGenerationHistory::new(scope.clone(), project.clone(), generation_budget()).unwrap();
    let mut nodes = BTreeMap::new();
    let selection = |cursor| SourceGenerationSelection {
        scope: scope.clone(),
        project: project.clone(),
        completed_cursor: cursor,
        generation_sha256: format!("{cursor:064x}"),
    };
    for cursor in [3, 7, 11] {
        for node in original.append(&selection(cursor)).unwrap() {
            nodes.insert((node.level, node.index), node.digest);
        }
    }
    let retained = original.accumulator().clone();
    let tail = SourceGenerationNeighbor {
        selection: selection(11),
        inclusion: retained
            .inclusion_proof(2, |level, index| Ok(nodes.get(&(level, index)).cloned()))
            .unwrap(),
    };
    assert!(SourceGenerationHistory::from_trusted_accumulator(
        scope.clone(),
        project.clone(),
        retained.clone(),
        None,
        generation_budget()
    )
    .is_err());
    let mut forged = tail.clone();
    forged.selection.completed_cursor = 12;
    assert!(SourceGenerationHistory::from_trusted_accumulator(
        scope.clone(),
        project.clone(),
        retained.clone(),
        Some(&forged),
        generation_budget()
    )
    .is_err());
    let mut reopened = SourceGenerationHistory::from_trusted_accumulator(
        scope.clone(),
        project.clone(),
        retained,
        Some(&tail),
        generation_budget(),
    )
    .unwrap();
    assert!(reopened.append(&selection(11)).is_err());
    reopened.append(&selection(15)).unwrap();
    original.append(&selection(15)).unwrap();
    assert_eq!(reopened.accumulator(), original.accumulator());
}

#[test]
fn generation_neighbor_proofs_match_linear_history_for_every_cursor() {
    let scope = ScopeId::new("instance:oracle").unwrap();
    let project = RuntimeId::new("project-oracle").unwrap();
    let mut history =
        SourceGenerationHistory::new(scope.clone(), project.clone(), generation_budget()).unwrap();
    let mut nodes = BTreeMap::new();
    let mut entries = Vec::new();
    for index in 0_u64..17 {
        let entry = SourceGenerationSelection {
            scope: scope.clone(),
            project: project.clone(),
            completed_cursor: 2 + index * 3,
            generation_sha256: format!("{:064x}", index + 1),
        };
        for node in history.append(&entry).unwrap() {
            nodes.insert((node.level, node.index), node.digest);
        }
        entries.push(entry);
        for cursor in 0..=55 {
            let split = entries.partition_point(|entry| entry.completed_cursor <= cursor);
            let neighbor = |position: usize| SourceGenerationNeighbor {
                selection: entries[position].clone(),
                inclusion: history
                    .accumulator()
                    .inclusion_proof(position as u64, |level, node| {
                        Ok(nodes.get(&(level, node)).cloned())
                    })
                    .unwrap(),
            };
            let left = split.checked_sub(1).map(neighbor);
            let right = (split < entries.len()).then(|| neighbor(split));
            let observed = verify_source_generation_at(
                &scope,
                &project,
                cursor,
                &history.accumulator().root,
                history.accumulator().tree_size,
                left.as_ref(),
                right.as_ref(),
            )
            .unwrap();
            let expected = entries
                .iter()
                .rev()
                .find(|entry| entry.completed_cursor <= cursor);
            assert_eq!(
                observed,
                expected,
                "history length {} cursor {cursor}",
                entries.len()
            );
        }
    }
}

#[derive(Deserialize)]
struct Golden {
    version: u16,
    leaves: Vec<String>,
    root: String,
    proofs: Vec<GoldenProof>,
}

#[derive(Deserialize)]
struct GoldenProof {
    leaf_index: u64,
    path: Vec<String>,
}

#[test]
fn matches_checked_in_cross_language_vector() {
    let golden: Golden =
        serde_json::from_str(include_str!("../../fixtures/authenticated-log-v1.json")).unwrap();
    assert_eq!(golden.version, RUNTIME_LOG_ACCUMULATOR_VERSION);
    let mut accumulator = RuntimeLogAccumulator::new();
    let mut nodes = BTreeMap::new();
    for leaf in &golden.leaves {
        for node in accumulator.append(leaf.as_bytes()).unwrap() {
            nodes.insert((node.level, node.index), node.digest);
        }
    }
    assert_eq!(accumulator.root, golden.root);
    for expected in golden.proofs {
        let proof = accumulator
            .inclusion_proof(expected.leaf_index, |level, index| {
                Ok(nodes.get(&(level, index)).cloned())
            })
            .unwrap();
        assert_eq!(proof.path, expected.path);
        proof
            .verify(
                golden.leaves[expected.leaf_index as usize].as_bytes(),
                &golden.root,
            )
            .unwrap();
    }
}

#[test]
fn matches_rfc_domain_separation_and_authenticates_every_shape() {
    let mut accumulator = RuntimeLogAccumulator::new();
    assert_eq!(
        accumulator.root,
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    let leaves = (0_u64..257)
        .map(|index| format!("runtime-change-{index}"))
        .collect::<Vec<_>>();
    let mut nodes = BTreeMap::new();
    for (index, leaf) in leaves.iter().enumerate() {
        for node in accumulator.append(leaf.as_bytes()).unwrap() {
            nodes.insert((node.level, node.index), node.digest);
        }
        for target in [0, index / 2, index] {
            let proof = accumulator
                .inclusion_proof(target as u64, |level, node_index| {
                    Ok(nodes.get(&(level, node_index)).cloned())
                })
                .unwrap();
            proof
                .verify(leaves[target].as_bytes(), &accumulator.root)
                .unwrap();
            assert!(proof.path.len() <= 9);
        }
    }
}

#[test]
fn corrupt_and_malformed_proofs_fail_closed() {
    let mut accumulator = RuntimeLogAccumulator::new();
    let mut nodes = BTreeMap::new();
    for leaf in [b"zero".as_slice(), b"one", b"two", b"three", b"four"] {
        for node in accumulator.append(leaf).unwrap() {
            nodes.insert((node.level, node.index), node.digest);
        }
    }
    let proof = accumulator
        .inclusion_proof(3, |level, index| Ok(nodes.get(&(level, index)).cloned()))
        .unwrap();
    proof.verify(b"three", &accumulator.root).unwrap();
    assert!(proof.verify(b"tampered", &accumulator.root).is_err());

    let mut corrupt = proof.clone();
    corrupt.path[0].replace_range(0..2, "ff");
    assert!(corrupt.verify(b"three", &accumulator.root).is_err());

    let mut excess = proof.clone();
    excess.path.push(accumulator.root.clone());
    assert!(excess.verify(b"three", &accumulator.root).is_err());

    let mut malformed = accumulator.clone();
    malformed.frontier[0] = None;
    assert!(malformed.validate().is_err());

    // 64 bytes, with a two-byte character across the first hex pair.
    let split_character = format!("a\u{e9}{}", "a".repeat(61));
    assert_eq!(split_character.len(), 64);
    let mut multibyte = proof.clone();
    multibyte.path[0] = split_character.clone();
    assert!(multibyte.verify(b"three", &accumulator.root).is_err());
    assert!(proof.verify(b"three", &split_character).is_err());
}

#[test]
fn digests_accept_exactly_what_encoding_produces() {
    for byte in 0..=u8::MAX {
        let digest = [byte; 32];
        let encoded = encode_digest(digest);
        assert_eq!(encoded, format!("{byte:02x}").repeat(32));
        assert_eq!(parse_digest("digest", &encoded).unwrap(), digest);
    }
    let mut mixed = [0_u8; 32];
    for (index, slot) in mixed.iter_mut().enumerate() {
        *slot = (index as u8).wrapping_mul(37);
    }
    assert_eq!(
        parse_digest("digest", &encode_digest(mixed)).unwrap(),
        mixed
    );

    let lowercase = "digest must contain lowercase hexadecimal";
    let length = "digest must be a 64-character SHA-256 digest";
    for (input, reason) in [
        ("AB".repeat(32), lowercase),
        ("aB".repeat(32), lowercase),
        ("+a".repeat(32), lowercase),
        ("zz".repeat(32), lowercase),
        (" a".repeat(32), lowercase),
        (format!("a\u{e9}{}", "a".repeat(61)), lowercase),
        (format!("\u{e9}{}", "a".repeat(62)), lowercase),
        ("ab".repeat(31), length),
        (format!("{}a", "ab".repeat(32)), length),
        (String::new(), length),
    ] {
        match parse_digest("digest", &input) {
            Err(Error::InvalidRuntime { reason: actual }) => {
                assert_eq!(actual, reason, "{input:?}");
            }
            other => panic!("{input:?} was not refused: {other:?}"),
        }
    }
}

#[test]
fn digest_parser_rejects_multibyte_text_without_panicking() {
    let malformed = format!("a{}b", "é".repeat(31));
    assert_eq!(malformed.len(), 64);
    assert!(parse_digest("test digest", &malformed).is_err());
}

#[test]
fn retained_prefix_frontier_reconstructs_after_later_appends() {
    let mut accumulator = RuntimeLogAccumulator::new();
    let mut nodes = BTreeMap::new();
    let leaves = (0_u64..19)
        .map(|index| format!("leaf-{index}"))
        .collect::<Vec<_>>();
    let mut retained = None;
    for (index, leaf) in leaves.iter().enumerate() {
        for node in accumulator.append(leaf.as_bytes()).unwrap() {
            nodes.insert((node.level, node.index), node.digest);
        }
        if index == 10 {
            retained = Some(accumulator.clone());
        }
    }
    let retained = retained.unwrap();
    let reconstructed =
        RuntimeLogAccumulator::from_nodes(retained.tree_size, &retained.root, |level, index| {
            Ok(nodes.get(&(level, index)).cloned())
        })
        .unwrap();
    assert_eq!(reconstructed, retained);
    let proof = reconstructed
        .inclusion_proof(7, |level, index| Ok(nodes.get(&(level, index)).cloned()))
        .unwrap();
    proof.verify(leaves[7].as_bytes(), &retained.root).unwrap();
}
