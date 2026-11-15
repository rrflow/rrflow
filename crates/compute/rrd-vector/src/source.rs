//! What a generation built for one source, a collection and named vector,
//! holds of each identity's complete vector history.

use crate::VectorCandidate;
use rrd_core::RuntimeRef;
use std::collections::BTreeMap;

/// What a generation for one source holds and the cursor it covers.
#[derive(Debug, Clone, PartialEq)]
pub struct SourceHistory {
    /// The versions, in cursor order.
    pub versions: Vec<VectorCandidate>,
    /// The newest cursor of any version ever in the source or of any exit
    /// from it, held or dropped. A generation built from a later read never
    /// covers less.
    pub source_cursor: u64,
}

/// The versions a generation for one source holds, in cursor order, so that
/// its own resolution, latest version first, answers as canonical resolution
/// over every source does (POAM-048), and the cursor it covers.
///
/// `candidates` must hold each identity's complete history, in every source.
/// An exit is a version outside the source that directly follows, in an
/// identity's history, a version in it: the move the store records as a
/// removal from the source. At an exit:
///
/// - every held version that applies only from the exit's valid time on is
///   dropped, because the exit supersedes it wherever it would apply;
/// - the version the source resolves to just before the exit applies is
///   copied with the exit's cursor, ending at the exit's valid time. The
///   copy starts before the exit applies, so its window is never empty.
///
/// A version outside the source that does not directly follow one in it
/// changes nothing here. The store records no change to the source for it,
/// so a build that extends a generation from the source's changes holds what
/// a full build holds.
pub fn source_history(
    candidates: impl IntoIterator<Item = VectorCandidate>,
    in_source: impl Fn(&VectorCandidate) -> bool,
) -> SourceHistory {
    let mut histories = BTreeMap::<RuntimeRef, Vec<VectorCandidate>>::new();
    for candidate in candidates {
        histories
            .entry(candidate.vector.reference.clone())
            .or_default()
            .push(candidate);
    }
    let mut versions = Vec::new();
    let mut source_cursor = 0;
    for mut history in histories.into_values() {
        history.sort_by_key(|candidate| candidate.source_cursor);
        let mut held = Vec::<VectorCandidate>::new();
        let mut follows_held = false;
        for candidate in history {
            if in_source(&candidate) {
                source_cursor = source_cursor.max(candidate.source_cursor);
                held.push(candidate);
                follows_held = true;
                continue;
            }
            if !std::mem::replace(&mut follows_held, false) {
                continue;
            }
            source_cursor = source_cursor.max(candidate.source_cursor);
            let exit_at = candidate.vector.valid_from;
            held.retain(|version| version.vector.valid_from < exit_at);
            if let Some(before) = held.iter().max_by_key(|version| version.source_cursor) {
                let mut copy = before.clone();
                copy.source_cursor = candidate.source_cursor;
                copy.vector.valid_to = Some(
                    before
                        .vector
                        .valid_to
                        .map_or(exit_at, |valid_to| valid_to.min(exit_at)),
                );
                held.push(copy);
            }
        }
        versions.extend(held);
    }
    versions.sort_by(|left, right| {
        left.source_cursor
            .cmp(&right.source_cursor)
            .then_with(|| left.vector.reference.cmp(&right.vector.reference))
    });
    SourceHistory {
        versions,
        source_cursor,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rrd_core::{
        RuntimeProperties, RuntimeVector, ScopeId, VectorCollectionAddress, VectorValue,
    };

    const SOURCE: &str = "documents";

    fn version(
        cursor: u64,
        collection: &str,
        valid_from: u64,
        valid_to: Option<u64>,
    ) -> VectorCandidate {
        VectorCandidate {
            scope: ScopeId::new("instance:source-history").unwrap(),
            source_cursor: cursor,
            vector: RuntimeVector {
                reference: RuntimeRef::new("embedding", "moving").unwrap(),
                subject: RuntimeRef::new("document", "moving").unwrap(),
                collection: VectorCollectionAddress {
                    collection_id: collection.into(),
                    vector_name: "body".into(),
                },
                field: "body".into(),
                valid_from,
                valid_to,
                // Each version's value names its cursor, so a copy is
                // recognisable by the version it copies.
                value: VectorValue::Dense {
                    values: vec![cursor as f32, 1.0],
                },
                provenance: None,
                properties: RuntimeProperties::new(),
            },
        }
    }

    fn in_source(candidate: &VectorCandidate) -> bool {
        candidate.vector.collection.collection_id == SOURCE
    }

    /// The version resolution selects at `valid_at`, and whether it is live.
    fn resolve(versions: &[VectorCandidate], valid_at: u64) -> Option<&VectorCandidate> {
        versions
            .iter()
            .filter(|version| version.vector.valid_from <= valid_at)
            .max_by_key(|version| version.source_cursor)
            .filter(|latest| {
                latest
                    .vector
                    .valid_to
                    .is_none_or(|valid_to| valid_at < valid_to)
            })
    }

    fn value(version: &VectorCandidate) -> &VectorValue {
        &version.vector.value
    }

    /// A deterministic generator, so a failure names its seed.
    struct Lcg(u64);

    impl Lcg {
        fn below(&mut self, bound: u64) -> u64 {
            self.0 = self
                .0
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            (self.0 >> 33) % bound
        }
    }

    /// Over randomized histories in three collections, at every valid time,
    /// the source's own resolution of its history finds the identity live
    /// with the same value exactly when canonical resolution finds it live
    /// in the source. The one allowed difference is a version outside the
    /// source that is not an exit: the source may still answer with what it
    /// held before that version, which the search rerank resolves away.
    #[test]
    fn a_source_resolves_its_history_as_every_source_resolves_the_whole() {
        let mut random = Lcg(0x5eed);
        for seed in 0..20_000 {
            let mut history = Vec::new();
            for cursor in 1..=1 + random.below(6) {
                let collection = ["documents", "archive", "mirror"][random.below(3) as usize];
                let valid_from = 1 + random.below(8);
                let valid_to = (random.below(4) == 0).then(|| valid_from + 1 + random.below(4));
                history.push(version(cursor, collection, valid_from, valid_to));
            }
            let held = source_history(history.clone(), in_source).versions;
            let mut seen = std::collections::BTreeSet::new();
            for version in &held {
                version.validate().unwrap_or_else(|error| {
                    panic!("seed {seed}: a held version is invalid: {error}")
                });
                assert!(in_source(version), "seed {seed}");
                assert!(seen.insert(version.source_cursor), "seed {seed}");
            }
            for valid_at in 0..=14 {
                let canonical = resolve(&history, valid_at);
                let expected = canonical.filter(|latest| in_source(latest)).map(value);
                let got = resolve(&held, valid_at).map(value);
                if expected.is_some() {
                    assert_eq!(got, expected, "seed {seed} at {valid_at}: {history:?}");
                    continue;
                }
                if got.is_none() {
                    continue;
                }
                // Live in the source while canonically elsewhere: only a
                // version outside the source that follows another outside
                // it may do that.
                let latest = history
                    .iter()
                    .filter(|version| version.vector.valid_from <= valid_at)
                    .max_by_key(|version| version.source_cursor)
                    .expect("the source holds a version, so one applies");
                let previous = history
                    .iter()
                    .filter(|version| version.source_cursor < latest.source_cursor)
                    .max_by_key(|version| version.source_cursor);
                assert!(
                    !in_source(latest) && previous.is_some_and(|previous| !in_source(previous)),
                    "seed {seed} at {valid_at}: the source answers {got:?} \
                     though {latest:?} applies: {history:?}"
                );
            }
        }
    }

    #[test]
    fn a_move_in_place_drops_the_version_and_a_later_move_ends_it() {
        // Moved out at its own valid time: never held after the move, and
        // the exit is covered.
        let moved = source_history(
            [version(1, SOURCE, 1, None), version(2, "archive", 1, None)],
            in_source,
        );
        assert!(moved.versions.is_empty());
        assert_eq!(moved.source_cursor, 2);
        // Moved out from valid time 50: held until then, by a copy that ends
        // there.
        let held = source_history(
            [version(1, SOURCE, 1, None), version(2, "archive", 50, None)],
            in_source,
        )
        .versions;
        assert_eq!(
            held.iter()
                .map(|version| (version.source_cursor, version.vector.valid_to))
                .collect::<Vec<_>>(),
            [(1, None), (2, Some(50))]
        );
        // Out and back: only the version that came back.
        let held = source_history(
            [
                version(1, SOURCE, 1, None),
                version(2, "archive", 1, None),
                version(3, SOURCE, 1, None),
            ],
            in_source,
        )
        .versions;
        assert_eq!(
            held.iter()
                .map(|version| version.source_cursor)
                .collect::<Vec<_>>(),
            [3]
        );
        // Never in the source, and an engram without moves: unchanged.
        let elsewhere = source_history([version(1, "archive", 1, None)], in_source);
        assert!(elsewhere.versions.is_empty());
        assert_eq!(elsewhere.source_cursor, 0);
        let unmoved = [version(1, SOURCE, 1, None), version(2, SOURCE, 5, Some(9))];
        let history = source_history(unmoved.clone(), in_source);
        assert_eq!(history.versions, unmoved);
        assert_eq!(history.source_cursor, 2);
    }
}
