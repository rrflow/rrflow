//! Replaceable point-read acceleration over the current authenticated segment set.
//! Nodes retain only ordinals: the opened segments remain the owners of key bounds.
use crate::{Result, SegmentDescriptor};

#[derive(Debug)]
struct Node {
    segment: usize,
    furthest_end: usize,
}

#[derive(Debug, Default)]
pub(super) struct PointIndex {
    // Sorted by first key; each slice's midpoint is its balanced subtree root.
    nodes: Vec<Node>,
}

impl PointIndex {
    pub(super) fn new(descriptors: &[SegmentDescriptor]) -> Self {
        let mut nodes = (0..descriptors.len())
            .map(|segment| Node {
                segment,
                furthest_end: segment,
            })
            .collect::<Vec<_>>();
        nodes.sort_unstable_by(|a, b| {
            descriptors[a.segment]
                .first_key
                .cmp(&descriptors[b.segment].first_key)
                .then(a.segment.cmp(&b.segment))
        });
        fn augment(nodes: &mut [Node], descriptors: &[SegmentDescriptor]) -> Option<usize> {
            let middle = nodes.len() / 2;
            let (left, rest) = nodes.split_at_mut(middle);
            let (root, right) = rest.split_first_mut()?;
            let mut furthest = root.segment;
            for candidate in [augment(left, descriptors), augment(right, descriptors)]
                .into_iter()
                .flatten()
            {
                if descriptors[candidate].last_key > descriptors[furthest].last_key {
                    furthest = candidate;
                }
            }
            root.furthest_end = furthest;
            Some(furthest)
        }
        augment(&mut nodes, descriptors);
        Self { nodes }
    }

    // No allocation or page ownership per lookup. The returned visit count is
    // private characterization evidence, not a new public storage counter.
    pub(super) fn visit<'a>(
        &self,
        descriptor: impl Fn(usize) -> &'a SegmentDescriptor,
        key: &[u8],
        mut candidate: impl FnMut(usize) -> Result<()>,
    ) -> Result<usize> {
        fn walk<'a>(
            nodes: &[Node],
            descriptor: &impl Fn(usize) -> &'a SegmentDescriptor,
            key: &[u8],
            candidate: &mut impl FnMut(usize) -> Result<()>,
        ) -> Result<usize> {
            let middle = nodes.len() / 2;
            let Some(root) = nodes.get(middle) else {
                return Ok(0);
            };
            // Inclusive segment bounds: equality must remain a candidate.
            if descriptor(root.furthest_end).last_key.as_slice() < key {
                return Ok(1);
            }
            let mut visited = 1 + walk(&nodes[..middle], descriptor, key, candidate)?;
            let bounds = descriptor(root.segment);
            if bounds.first_key.as_slice() <= key {
                if key <= bounds.last_key.as_slice() {
                    candidate(root.segment)?;
                }
                visited += walk(&nodes[middle + 1..], descriptor, key, candidate)?;
            }
            Ok(visited)
        }
        walk(&self.nodes, &descriptor, key, &mut candidate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn descriptor(first: Vec<u8>, last: Vec<u8>) -> SegmentDescriptor {
        SegmentDescriptor {
            id: String::new(),
            level: 0,
            format_version: 0,
            schema_digest: String::new(),
            key_codec_digest: String::new(),
            page_format_digest: String::new(),
            first_key: first,
            last_key: last,
            minimum_sequence: 1,
            maximum_sequence: 1,
            entries: 1,
            bytes: 1,
            checksum: String::new(),
        }
    }

    #[test]
    fn interval_candidates_match_exhaustive_binary_ranges() {
        let keys = [
            vec![],
            vec![0],
            vec![0, 0],
            vec![0, 255],
            vec![1],
            vec![127],
            vec![255],
            vec![255, 0],
        ];
        let mut descriptors = Vec::new();
        for (i, first) in keys.iter().enumerate() {
            for last in &keys[i..] {
                descriptors.push(descriptor(first.clone(), last.clone()));
            }
        }
        // Duplicated endpoints, arbitrary manifest order and nested intervals.
        descriptors.extend(descriptors.clone());
        descriptors.reverse();
        for count in 0..=descriptors.len() {
            let descriptors = &descriptors[..count];
            let index = PointIndex::new(descriptors);
            for key in keys
                .iter()
                .chain([vec![0, 1], vec![2], vec![255, 255]].iter())
            {
                let expected = descriptors
                    .iter()
                    .enumerate()
                    .filter_map(|(i, d)| (d.first_key <= *key && *key <= d.last_key).then_some(i))
                    .collect::<Vec<_>>();
                let mut actual = Vec::new();
                index
                    .visit(
                        |i| &descriptors[i],
                        key,
                        |i| {
                            actual.push(i);
                            Ok(())
                        },
                    )
                    .unwrap();
                actual.sort_unstable();
                assert_eq!(actual, expected, "count={count}, key={key:?}");
            }
        }
    }

    #[test]
    fn disjoint_engram_search_does_not_walk_every_descriptor() {
        let descriptors = (0..1477u32)
            .rev()
            .map(|i| {
                descriptor(
                    (i * 4).to_be_bytes().to_vec(),
                    (i * 4 + 1).to_be_bytes().to_vec(),
                )
            })
            .collect::<Vec<_>>();
        let index = PointIndex::new(&descriptors);
        let mut maximum_visits = 0;
        for key in 0..1477u32 * 4 + 1 {
            let mut found = Vec::new();
            let visits = index
                .visit(
                    |i| &descriptors[i],
                    &key.to_be_bytes(),
                    |i| {
                        found.push(i);
                        Ok(())
                    },
                )
                .unwrap();
            maximum_visits = maximum_visits.max(visits);
            let expected = if key < 1477 * 4 && key % 4 < 2 {
                vec![1476 - key as usize / 4]
            } else {
                vec![]
            };
            assert_eq!(found, expected);
            assert!(visits <= 24, "linear metadata walk: {visits}");
        }
        println!(
            "point_range_index descriptors=1477 maximum_visits={maximum_visits} retained_bytes={}",
            index.nodes.capacity() * std::mem::size_of::<Node>()
        );
        assert_eq!(index.nodes.capacity(), descriptors.len());
    }

    #[test]
    fn candidate_errors_are_not_hidden() {
        let descriptors = vec![descriptor(vec![], vec![255])];
        let error = PointIndex::new(&descriptors)
            .visit(
                |i| &descriptors[i],
                &[1],
                |_| Err(crate::Error::InvalidSegment("selected corruption".into())),
            )
            .unwrap_err();
        assert!(matches!(error, crate::Error::InvalidSegment(_)));
    }
}
