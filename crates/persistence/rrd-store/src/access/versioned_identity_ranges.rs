use super::*;
use rrd_core::{RuntimeCommit, RuntimeProperties, RuntimeRecord, ScopeId};

fn interval(lower: Bound<&str>, upper: Bound<&str>) -> RuntimeVersionedSource {
    RuntimeVersionedSource::IdentityRange {
        model: RuntimeLogicalModel::Relational,
        kind: RuntimeType::new("document").unwrap(),
        lower: lower.map(|s| RuntimeId::new(s).unwrap()),
        upper: upper.map(|s| RuntimeId::new(s).unwrap()),
    }
}

#[test]
fn identity_intervals_admit_disjoint_ranges_and_reject_every_overlap_order() {
    let scope = ScopeId::new("instance:ranges").unwrap();
    let left = interval(Bound::Included("a"), Bound::Excluded("m"));
    let right = interval(Bound::Included("m"), Bound::Excluded("z"));
    let identity = RuntimeVersionedSource::Identity {
        model: RuntimeLogicalModel::Relational,
        reference: RuntimeRef::new("document", "b").unwrap(),
    };
    let family = RuntimeVersionedSource::Records {
        kind: Some(RuntimeType::new("document").unwrap()),
    };
    for second in [
        interval(Bound::Included("b"), Bound::Excluded("z")),
        identity,
        family,
    ] {
        for pair in [[left.clone(), second.clone()], [second, left.clone()]] {
            assert!(SourceRanges::default().admit(&pair, &scope).is_err());
        }
    }
    let mut ranges = SourceRanges::default();
    assert_eq!(
        ranges
            .admit(&[left.clone(), right.clone(), left], &scope)
            .unwrap()
            .len(),
        2
    );
    assert!(ranges.admit(&[right], &scope).unwrap().is_empty());
    assert!(SourceRanges::default()
        .admit(&[interval(Bound::Unbounded, Bound::Unbounded)], &scope)
        .is_err());
    assert!(SourceRanges::default()
        .admit(
            &[interval(Bound::Included("z"), Bound::Included("a"))],
            &scope
        )
        .is_err());
    assert!(SourceRanges::default()
        .admit(
            &[interval(Bound::Excluded("a"), Bound::Included("a"))],
            &scope
        )
        .unwrap()
        .is_empty());
}

#[test]
fn identity_interval_keys_preserve_string_order_and_validate_returned_identity() {
    let scope = ScopeId::new("instance:ranges").unwrap();
    let ids = [
        "a",
        "aa",
        "abcdefgh",
        "abcdefghx",
        "b",
        "q\"uote",
        "slash\\id",
        "é",
        "雪",
    ];
    for lower in ids {
        for upper in ids.into_iter().filter(|upper| *upper >= lower) {
            for inclusive_lower in [false, true] {
                for inclusive_upper in [false, true] {
                    let source = interval(
                        if inclusive_lower {
                            Bound::Included(lower)
                        } else {
                            Bound::Excluded(lower)
                        },
                        if inclusive_upper {
                            Bound::Included(upper)
                        } else {
                            Bound::Excluded(upper)
                        },
                    );
                    let (space, start, end) = source_interval(&source, &scope).unwrap();
                    let encoded = serde_json::to_vec(&source).unwrap();
                    assert_eq!(
                        serde_json::from_slice::<RuntimeVersionedSource>(&encoded).unwrap(),
                        source
                    );
                    for id in ids {
                        let reference = RuntimeRef::new("document", id).unwrap();
                        let address = VersionAddress {
                            space,
                            reference: reference.clone(),
                            effective_at: 10,
                        };
                        let expected = (id > lower || (inclusive_lower && id == lower))
                            && (id < upper || (inclusive_upper && id == upper));
                        let key = keyspaces::runtime_version_key(space, &scope, &reference, 10, 1);
                        assert_eq!(start <= key && key < end, expected);
                        assert_eq!(source_accepts_address(&source, &address).unwrap(), expected);
                    }
                }
            }
        }
    }
    let record = RuntimeRecord {
        reference: RuntimeRef::new("document", "z").unwrap(),
        valid_from: 10,
        valid_to: None,
        properties: RuntimeProperties::new(),
    };
    let commit = RuntimeCommit {
        scope: scope.clone(),
        at: 10,
        actor: "range-test".into(),
        expected_cursor: 0,
        mutations: vec![RuntimeMutation::Record {
            record: record.clone(),
        }],
    };
    let change = RuntimeChange::committed(
        1,
        &commit,
        &commit.digest(),
        0,
        commit.mutations[0].clone(),
        None,
    );
    let read = ReadStamp::authenticated(
        scope.clone(),
        Some(1),
        0,
        1,
        Some(change.digest.clone()),
        "11".repeat(32),
    )
    .unwrap();
    let key = keyspaces::runtime_version_key(
        keyspaces::RUNTIME_RECORD_VERSIONS,
        &scope,
        &record.reference,
        10,
        1,
    );
    assert!(validate_version_entry(
        &interval(Bound::Included("a"), Bound::Excluded("m")),
        &read,
        &key,
        &change
    )
    .is_err());
}
