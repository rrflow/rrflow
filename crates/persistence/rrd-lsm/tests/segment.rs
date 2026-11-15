use rrd_lsm::{
    Database, DatabaseOptions, Durability, Error, Memtable, Mutation, Segment,
    SegmentCompressionPolicy, SegmentRowGroupBudget, WalWriter, WriteBatch,
    DEFAULT_COMPRESSION_MAXIMUM_PAGE_LOGICAL_BYTES,
    DEFAULT_COMPRESSION_MINIMUM_SAVINGS_BASIS_POINTS,
};
use std::io::{Seek, SeekFrom, Write};

fn table() -> Memtable {
    let directory = tempfile::tempdir().unwrap();
    let wal_path = directory.path().join("active.wal");
    let mut wal = WalWriter::create(&wal_path).unwrap();
    let first = WriteBatch::new(vec![
        Mutation::Put {
            key: b"alpha".to_vec(),
            value: b"one".to_vec(),
        },
        Mutation::Put {
            key: b"beta".to_vec(),
            value: b"two".to_vec(),
        },
    ])
    .unwrap();
    wal.append_write_batch(&first, Durability::Authoritative)
        .unwrap();
    let second = WriteBatch::new(vec![
        Mutation::Delete {
            key: b"alpha".to_vec(),
        },
        Mutation::Put {
            key: b"beta".to_vec(),
            value: b"three".to_vec(),
        },
    ])
    .unwrap();
    wal.append_write_batch(&second, Durability::Authoritative)
        .unwrap();
    drop(wal);
    let recovery = rrd_lsm::recover(&wal_path).unwrap();
    Memtable::recover(&recovery.batches).unwrap()
}

fn compressible_table() -> Memtable {
    let directory = tempfile::tempdir().unwrap();
    let wal_path = directory.path().join("compressible.wal");
    let mut wal = WalWriter::create(&wal_path).unwrap();
    let mutations = (0..128)
        .map(|index| Mutation::Put {
            key: format!("key-{index:04}").into_bytes(),
            value: format!("engram=rrflow|record={index:04}|")
                .repeat(16)
                .into_bytes(),
        })
        .collect();
    wal.append_write_batch(
        &WriteBatch::new(mutations).unwrap(),
        Durability::Authoritative,
    )
    .unwrap();
    drop(wal);
    Memtable::recover(&rrd_lsm::recover(&wal_path).unwrap().batches).unwrap()
}

#[test]
fn immutable_segment_preserves_mvcc_reads_and_content_identity() {
    let directory = tempfile::tempdir().unwrap();
    let segments = directory.path().join("segments");
    let (segment, path) = Segment::write_from_memtable(&segments, &table()).unwrap();
    assert!(path.ends_with(format!("{}.seg", segment.descriptor.id)));
    assert_eq!(segment.descriptor.minimum_sequence, 1);
    assert_eq!(segment.descriptor.maximum_sequence, 4);
    assert_eq!(segment.descriptor.entries, 4);
    assert_eq!(segment.get(b"alpha", 1).unwrap(), Some(b"one".to_vec()));
    assert_eq!(segment.get(b"alpha", 3).unwrap(), None);
    assert_eq!(segment.get(b"beta", 2).unwrap(), Some(b"two".to_vec()));
    assert_eq!(segment.get(b"beta", 4).unwrap(), Some(b"three".to_vec()));

    let reopened = Segment::open(&path).unwrap();
    assert_eq!(reopened.descriptor, segment.descriptor);
    assert_eq!(
        reopened.scan(b"a", Some(b"z"), 2).unwrap(),
        vec![
            (b"alpha".to_vec(), b"one".to_vec()),
            (b"beta".to_vec(), b"two".to_vec()),
        ]
    );
    let (deduplicated, same_path) = Segment::write_from_memtable(&segments, &table()).unwrap();
    assert_eq!(same_path, path);
    assert_eq!(deduplicated.descriptor, segment.descriptor);
}

#[test]
fn v6_adaptive_writer_selects_raw_and_lz4_pages_deterministically() {
    let directory = tempfile::tempdir().unwrap();
    let segments = directory.path().join("segments");
    let (adaptive, adaptive_path) = Segment::write_from_memtable(&segments, &table()).unwrap();
    assert_eq!(
        adaptive.compression_policy(),
        SegmentCompressionPolicy::AdaptiveLz4 {
            minimum_savings_basis_points: DEFAULT_COMPRESSION_MINIMUM_SAVINGS_BASIS_POINTS,
            maximum_page_logical_bytes: DEFAULT_COMPRESSION_MAXIMUM_PAGE_LOGICAL_BYTES,
        }
    );
    let adaptive_open = adaptive.open_evidence();
    assert_eq!(adaptive_open.adaptive_lz4_policy_segment_count, 1);
    assert_eq!(adaptive_open.none_policy_segment_count, 0);
    assert!(adaptive_open.raw_page_count > 0);
    assert!(adaptive_open.compressed_page_count > 0);
    assert!(adaptive_open.stored_page_bytes < adaptive_open.logical_page_bytes);

    let (same, same_path) = Segment::write_from_memtable(&segments, &table()).unwrap();
    assert_eq!(same_path, adaptive_path);
    assert_eq!(same.descriptor, adaptive.descriptor);

    let none_directory = directory.path().join("none");
    let (none, _) = Segment::write_from_memtable_with_policy(
        &none_directory,
        &table(),
        SegmentCompressionPolicy::None,
    )
    .unwrap();
    let none_open = none.open_evidence();
    assert_eq!(none.compression_policy(), SegmentCompressionPolicy::None);
    assert_eq!(none_open.none_policy_segment_count, 1);
    assert_eq!(none_open.adaptive_lz4_policy_segment_count, 0);
    assert_eq!(none_open.raw_page_count, 6);
    assert_eq!(none_open.compressed_page_count, 0);
    assert_eq!(none_open.stored_page_bytes, none_open.logical_page_bytes);
    for sequence in 1..=4 {
        assert_eq!(
            none.scan(b"", None, sequence).unwrap(),
            adaptive.scan(b"", None, sequence).unwrap()
        );
    }
}

#[test]
fn adaptive_policy_rejects_invalid_threshold_and_page_limit() {
    let directory = tempfile::tempdir().unwrap();
    let invalid = [
        SegmentCompressionPolicy::AdaptiveLz4 {
            minimum_savings_basis_points: 0,
            maximum_page_logical_bytes: 1,
        },
        SegmentCompressionPolicy::AdaptiveLz4 {
            minimum_savings_basis_points: 10_000,
            maximum_page_logical_bytes: 1,
        },
        SegmentCompressionPolicy::AdaptiveLz4 {
            minimum_savings_basis_points: 1,
            maximum_page_logical_bytes: 0,
        },
        SegmentCompressionPolicy::AdaptiveLz4 {
            minimum_savings_basis_points: 1,
            maximum_page_logical_bytes: 1024 * 1024 * 1024 + 1,
        },
    ];
    for (index, segment_compression) in invalid.into_iter().enumerate() {
        let root = directory.path().join(format!("invalid-{index}"));
        let result = Database::create_with_options(
            &root,
            DatabaseOptions {
                segment_compression,
                ..DatabaseOptions::default()
            },
        );
        assert!(matches!(result, Err(Error::InvalidConfiguration(_))));
        assert!(!root.exists());
    }
}

#[test]
fn v6_bytes_match_the_checked_in_format_vector() {
    let directory = tempfile::tempdir().unwrap();
    let (_, path) =
        Segment::write_from_memtable(&directory.path().join("segments"), &table()).unwrap();
    let bytes = std::fs::read(path).unwrap();
    let actual = bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let fixture = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/segment-v6.hex");
    if std::env::var_os("RRFLOW_UPDATE_GOLDENS").is_some() {
        std::fs::write(fixture, format!("{actual}\n")).unwrap();
    }
    assert_eq!(
        format!("{actual}\n"),
        std::fs::read_to_string(fixture).unwrap()
    );
}

#[test]
fn corruption_and_truncation_fail_closed() {
    let directory = tempfile::tempdir().unwrap();
    let segments = directory.path().join("segments");
    let (_, path) = Segment::write_from_memtable(&segments, &table()).unwrap();
    let original = std::fs::read(&path).unwrap();

    let corrupt = directory.path().join("corrupt.seg");
    std::fs::write(&corrupt, &original).unwrap();
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .open(&corrupt)
        .unwrap();
    file.seek(SeekFrom::Start(50)).unwrap();
    file.write_all(&[original[50] ^ 0x80]).unwrap();
    file.sync_all().unwrap();
    assert!(matches!(
        Segment::open(&corrupt),
        Err(Error::InvalidSegment(_))
    ));

    let truncated = directory.path().join("truncated.seg");
    std::fs::write(&truncated, &original[..original.len() - 1]).unwrap();
    assert!(matches!(
        Segment::open(&truncated),
        Err(Error::InvalidSegment(_))
    ));
}

#[test]
fn sparse_segment_matches_the_memtable_for_point_range_and_mvcc_reads() {
    let directory = tempfile::tempdir().unwrap();
    let wal_path = directory.path().join("many.wal");
    let mut wal = WalWriter::create(&wal_path).unwrap();
    for phase in 0..3 {
        let operations = (0..200)
            .map(|index| {
                let key = format!("key:{index:04}").into_bytes();
                if phase == 2 && index % 5 == 0 {
                    Mutation::Delete { key }
                } else {
                    Mutation::Put {
                        key,
                        value: format!("value:{phase}:{index:04}|").repeat(8).into_bytes(),
                    }
                }
            })
            .collect();
        wal.append_write_batch(
            &WriteBatch::new(operations).unwrap(),
            Durability::Authoritative,
        )
        .unwrap();
    }
    drop(wal);
    let recovery = rrd_lsm::recover(&wal_path).unwrap();
    let table = Memtable::recover(&recovery.batches).unwrap();
    let (segment, path) =
        Segment::write_from_memtable(&directory.path().join("segments"), &table).unwrap();
    assert!(segment.row_group_count() >= 2);
    assert_eq!(&std::fs::read(&path).unwrap()[..8], b"RRDSEG06");
    assert_eq!(segment.descriptor.format_version, 6);
    assert_eq!(
        segment.descriptor.schema_digest,
        rrd_lsm::SEGMENT_SCHEMA_DIGEST
    );

    for snapshot in [1, 199, 200, 201, 399, 400, 401, 599, 600] {
        for index in 0..200 {
            let key = format!("key:{index:04}");
            assert_eq!(
                segment.get(key.as_bytes(), snapshot).unwrap().as_deref(),
                table.get(key.as_bytes(), snapshot),
                "point mismatch for {key} at sequence {snapshot}"
            );
        }
        for (start, end) in [
            (b"key:0000".as_slice(), Some(b"key:0200".as_slice())),
            (b"key:0063".as_slice(), Some(b"key:0131".as_slice())),
            (b"key:0190".as_slice(), None),
        ] {
            assert_eq!(
                segment.scan(start, end, snapshot).unwrap(),
                table.scan(start, end, snapshot),
                "range mismatch at sequence {snapshot}"
            );
        }
    }
}

fn rewrite_checksum(bytes: &mut Vec<u8>) {
    bytes.truncate(bytes.len() - 64);
    bytes.extend_from_slice(rrd_core::digest::sha256_hex(bytes).as_bytes());
}

fn first_page_descriptor(bytes: &[u8]) -> usize {
    let index_offset = u64::from_le_bytes(bytes[48..56].try_into().unwrap()) as usize;
    let group_header = index_offset + 32;
    let first_key_bytes = u32::from_le_bytes(
        bytes[group_header + 12..group_header + 16]
            .try_into()
            .unwrap(),
    ) as usize;
    let last_key_bytes = u32::from_le_bytes(
        bytes[group_header + 16..group_header + 20]
            .try_into()
            .unwrap(),
    ) as usize;
    group_header + 32 + first_key_bytes + last_key_bytes
}

fn first_compressed_page_descriptor(bytes: &[u8]) -> usize {
    first_page_descriptor_matching(bytes, |_, compression| compression == 1)
        .expect("default v6 vector contains an LZ4 page")
}

fn first_page_descriptor_matching(
    bytes: &[u8],
    mut predicate: impl FnMut(u8, u8) -> bool,
) -> Option<usize> {
    let first = first_page_descriptor(bytes);
    (0..6)
        .map(|page| first + page * 96)
        .find(|descriptor| predicate(bytes[*descriptor], bytes[*descriptor + 6]))
}

fn rewrite_page_digest(bytes: &mut [u8], descriptor: usize) {
    let page_offset =
        u64::from_le_bytes(bytes[descriptor + 24..descriptor + 32].try_into().unwrap()) as usize;
    let page_bytes =
        u64::from_le_bytes(bytes[descriptor + 32..descriptor + 40].try_into().unwrap()) as usize;
    let digest: [u8; 32] = ring::digest::digest(
        &ring::digest::SHA256,
        &bytes[page_offset..page_offset + page_bytes],
    )
    .as_ref()
    .try_into()
    .unwrap();
    bytes[descriptor + 64..descriptor + 96].copy_from_slice(&digest);
}

#[test]
fn v6_rejects_authenticated_compression_metadata_and_block_corruption() {
    let directory = tempfile::tempdir().unwrap();
    let (_, path) =
        Segment::write_from_memtable(&directory.path().join("segments"), &compressible_table())
            .unwrap();
    let original = std::fs::read(path).unwrap();
    let descriptor = first_compressed_page_descriptor(&original);

    let cases = [
        ("unknown-policy", 176, 2u8),
        ("unknown-policy-codec", 177, 2u8),
        ("nonzero-policy-reserved", 188, 1u8),
        ("unknown-page-encoding", descriptor + 5, 2u8),
        ("unknown-page-codec", descriptor + 6, 2u8),
        ("nonzero-page-reserved", descriptor + 7, 1u8),
    ];
    for (name, offset, replacement) in cases {
        let mut bytes = original.clone();
        bytes[offset] = replacement;
        rewrite_checksum(&mut bytes);
        let path = directory.path().join(format!("{name}.seg"));
        std::fs::write(&path, bytes).unwrap();
        assert!(matches!(
            Segment::open(&path),
            Err(Error::InvalidSegment(_))
        ));
    }

    for (name, offset, value) in [
        ("zero-threshold", 178, 0u64),
        ("impossible-threshold", 178, 9_999u64),
        ("zero-page-limit", 180, 0u64),
        ("excessive-page-limit", 180, 1024 * 1024 * 1024 + 1),
    ] {
        let mut bytes = original.clone();
        if offset == 178 {
            bytes[offset..offset + 2].copy_from_slice(&(value as u16).to_le_bytes());
        } else {
            bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
        }
        rewrite_checksum(&mut bytes);
        let path = directory.path().join(format!("{name}.seg"));
        std::fs::write(&path, bytes).unwrap();
        assert!(matches!(
            Segment::open(&path),
            Err(Error::InvalidSegment(_))
        ));
    }

    let mut none_policy_with_lz4 = original.clone();
    none_policy_with_lz4[176..188].fill(0);
    rewrite_checksum(&mut none_policy_with_lz4);
    let path = directory.path().join("none-policy-with-lz4.seg");
    std::fs::write(&path, none_policy_with_lz4).unwrap();
    assert!(matches!(
        Segment::open(&path),
        Err(Error::InvalidSegment(_))
    ));

    let compressed_logical = u64::from_le_bytes(
        original[descriptor + 40..descriptor + 48]
            .try_into()
            .unwrap(),
    );
    let mut page_above_policy_limit = original.clone();
    page_above_policy_limit[180..188]
        .copy_from_slice(&compressed_logical.saturating_sub(1).to_le_bytes());
    rewrite_checksum(&mut page_above_policy_limit);
    let path = directory.path().join("page-above-policy-limit.seg");
    std::fs::write(&path, page_above_policy_limit).unwrap();
    assert!(matches!(
        Segment::open(&path),
        Err(Error::InvalidSegment(_))
    ));

    for (name, physical_bytes) in [
        ("zero-compressed-length", 0),
        ("nonshrinking-compressed-length", compressed_logical),
    ] {
        let mut bytes = original.clone();
        bytes[descriptor + 32..descriptor + 40].copy_from_slice(&physical_bytes.to_le_bytes());
        rewrite_checksum(&mut bytes);
        let path = directory.path().join(format!("{name}.seg"));
        std::fs::write(&path, bytes).unwrap();
        assert!(matches!(
            Segment::open(&path),
            Err(Error::InvalidSegment(_))
        ));
    }

    let (_, raw_path) =
        Segment::write_from_memtable(&directory.path().join("raw-case"), &table()).unwrap();
    let raw_original = std::fs::read(raw_path).unwrap();
    let raw_descriptor =
        first_page_descriptor_matching(&raw_original, |_, compression| compression == 0)
            .expect("default v6 vector contains a raw page");
    let raw_logical = u64::from_le_bytes(
        raw_original[raw_descriptor + 40..raw_descriptor + 48]
            .try_into()
            .unwrap(),
    );
    let mut unequal_raw_lengths = raw_original;
    unequal_raw_lengths[raw_descriptor + 40..raw_descriptor + 48]
        .copy_from_slice(&(raw_logical + 1).to_le_bytes());
    rewrite_checksum(&mut unequal_raw_lengths);
    let path = directory.path().join("unequal-raw-lengths.seg");
    std::fs::write(&path, unequal_raw_lengths).unwrap();
    assert!(matches!(
        Segment::open(&path),
        Err(Error::InvalidSegment(_))
    ));

    let variable_compressed = first_page_descriptor_matching(&original, |kind, compression| {
        compression == 1 && matches!(kind, 2 | 6)
    })
    .expect("default v6 vector contains a variable-width LZ4 page");
    let decoded_length = u64::from_le_bytes(
        original[variable_compressed + 40..variable_compressed + 48]
            .try_into()
            .unwrap(),
    );
    for (name, declared_length) in [
        ("declared-decoded-length-too-small", decoded_length - 1),
        ("declared-decoded-length-too-large", decoded_length + 1),
    ] {
        let mut bytes = original.clone();
        bytes[variable_compressed + 40..variable_compressed + 48]
            .copy_from_slice(&declared_length.to_le_bytes());
        rewrite_checksum(&mut bytes);
        let path = directory.path().join(format!("{name}.seg"));
        std::fs::write(&path, bytes).unwrap();
        assert!(matches!(
            Segment::open(&path),
            Err(Error::InvalidSegment(reason)) if reason.contains("LZ4 page decode") || reason.contains("LZ4 page decoded")
        ));
    }

    let mut excessive_logical_length = original.clone();
    excessive_logical_length[descriptor + 40..descriptor + 48]
        .copy_from_slice(&u64::MAX.to_le_bytes());
    rewrite_checksum(&mut excessive_logical_length);
    let path = directory.path().join("excessive-logical-length.seg");
    std::fs::write(&path, excessive_logical_length).unwrap();
    assert!(matches!(
        Segment::open(&path),
        Err(Error::InvalidSegment(_))
    ));

    let mut changed_stored_block = original;
    let page_offset = u64::from_le_bytes(
        changed_stored_block[descriptor + 24..descriptor + 32]
            .try_into()
            .unwrap(),
    ) as usize;
    changed_stored_block[page_offset] ^= 0x80;
    rewrite_page_digest(&mut changed_stored_block, descriptor);
    rewrite_checksum(&mut changed_stored_block);
    let path = directory.path().join("changed-stored-block.seg");
    std::fs::write(&path, changed_stored_block).unwrap();
    assert!(matches!(
        Segment::open(&path),
        Err(Error::InvalidSegment(_))
    ));
}

#[test]
fn v6_rejects_authenticated_length_flags_and_page_corruption() {
    let directory = tempfile::tempdir().unwrap();
    let segments = directory.path().join("segments");
    let (_, path) = Segment::write_from_memtable(&segments, &table()).unwrap();
    let original = std::fs::read(path).unwrap();

    let mut wrong_length = original.clone();
    wrong_length[56..64].copy_from_slice(&999u64.to_le_bytes());
    rewrite_checksum(&mut wrong_length);
    let wrong_length_path = directory.path().join("wrong-length.seg");
    std::fs::write(&wrong_length_path, wrong_length).unwrap();
    assert!(matches!(
        Segment::open(&wrong_length_path),
        Err(Error::InvalidSegment(_))
    ));

    let mut unknown_flags = original.clone();
    unknown_flags[12..16].copy_from_slice(&2u32.to_le_bytes());
    rewrite_checksum(&mut unknown_flags);
    let unknown_flags_path = directory.path().join("unknown-flags.seg");
    std::fs::write(&unknown_flags_path, unknown_flags).unwrap();
    assert!(matches!(
        Segment::open(&unknown_flags_path),
        Err(Error::InvalidSegment(_))
    ));

    let mut corrupt_body = original;
    corrupt_body[256] ^= 0xff;
    rewrite_checksum(&mut corrupt_body);
    let corrupt_body_path = directory.path().join("corrupt-body.seg");
    std::fs::write(&corrupt_body_path, corrupt_body).unwrap();
    assert!(matches!(
        Segment::open(&corrupt_body_path),
        Err(Error::InvalidSegment(_))
    ));

    let mut corrupt_padding =
        std::fs::read(segments.read_dir().unwrap().next().unwrap().unwrap().path()).unwrap();
    corrupt_padding[296] = 1;
    rewrite_checksum(&mut corrupt_padding);
    let corrupt_padding_path = directory.path().join("corrupt-padding.seg");
    std::fs::write(&corrupt_padding_path, corrupt_padding).unwrap();
    assert!(matches!(
        Segment::open(&corrupt_padding_path),
        Err(Error::InvalidSegment(_))
    ));

    let mut impossible_entries =
        std::fs::read(segments.read_dir().unwrap().next().unwrap().unwrap().path()).unwrap();
    let index_offset = u64::from_le_bytes(impossible_entries[48..56].try_into().unwrap()) as usize;
    let first_key_bytes = u32::from_le_bytes(
        impossible_entries[index_offset + 44..index_offset + 48]
            .try_into()
            .unwrap(),
    ) as usize;
    let last_key_bytes = u32::from_le_bytes(
        impossible_entries[index_offset + 48..index_offset + 52]
            .try_into()
            .unwrap(),
    ) as usize;
    let first_page = index_offset + 32 + 32 + first_key_bytes + last_key_bytes;
    impossible_entries[first_page + 32..first_page + 40].copy_from_slice(&u64::MAX.to_le_bytes());
    rewrite_checksum(&mut impossible_entries);
    let impossible_entries_path = directory.path().join("impossible-entries.seg");
    std::fs::write(&impossible_entries_path, impossible_entries).unwrap();
    assert!(matches!(
        Segment::open(&impossible_entries_path),
        Err(Error::InvalidSegment(_))
    ));
}

fn first_filter_offset(bytes: &[u8]) -> usize {
    let index_offset = u64::from_le_bytes(bytes[48..56].try_into().unwrap()) as usize;
    let group_header = index_offset + 32;
    let first_key_bytes = u32::from_le_bytes(
        bytes[group_header + 12..group_header + 16]
            .try_into()
            .unwrap(),
    ) as usize;
    let last_key_bytes = u32::from_le_bytes(
        bytes[group_header + 16..group_header + 20]
            .try_into()
            .unwrap(),
    ) as usize;
    group_header + 32 + first_key_bytes + last_key_bytes + 6 * 96
}

#[test]
fn v6_rejects_authenticated_filter_corruption() {
    let directory = tempfile::tempdir().unwrap();
    let (_, path) =
        Segment::write_from_memtable(&directory.path().join("segments"), &table()).unwrap();
    let original = std::fs::read(path).unwrap();
    let index_offset = u64::from_le_bytes(original[48..56].try_into().unwrap()) as usize;
    let group_header = index_offset + 32;

    let cases = [
        ("unknown-filter-format", index_offset + 18, 2u8),
        ("nonzero-index-reserved", index_offset + 21, 1u8),
        ("zero-unique-keys", group_header + 24, 0u8),
        ("wrong-filter-words", group_header + 28, 2u8),
    ];
    for (name, offset, replacement) in cases {
        let mut bytes = original.clone();
        bytes[offset] = replacement;
        rewrite_checksum(&mut bytes);
        let path = directory.path().join(format!("{name}.seg"));
        std::fs::write(&path, bytes).unwrap();
        assert!(matches!(
            Segment::open(&path),
            Err(Error::InvalidSegment(_))
        ));
    }

    let mut changed_words = original;
    let filter_offset = first_filter_offset(&changed_words);
    changed_words[filter_offset] ^= 0x01;
    rewrite_checksum(&mut changed_words);
    let changed_words_path = directory.path().join("changed-filter-words.seg");
    std::fs::write(&changed_words_path, changed_words).unwrap();
    assert!(matches!(
        Segment::open(&changed_words_path),
        Err(Error::InvalidSegment(_))
    ));
}

fn superseded_segment_prefix(version: u16) -> Vec<u8> {
    let magic = match version {
        1 => b"RRDSEG01".as_slice(),
        2 => b"RRDSEG02".as_slice(),
        3 => b"RRDSEG03".as_slice(),
        4 => b"RRDSEG04".as_slice(),
        5 => b"RRDSEG05".as_slice(),
        _ => unreachable!("test constructs only superseded pre-release formats"),
    };
    let mut bytes = magic.to_vec();
    bytes.extend_from_slice(&version.to_le_bytes());
    bytes
}

#[test]
fn superseded_and_unknown_segment_formats_are_rejected() {
    let directory = tempfile::tempdir().unwrap();
    for version in [1, 2, 3, 4, 5] {
        let path = directory.path().join(format!("format-{version}.seg"));
        std::fs::write(&path, superseded_segment_prefix(version)).unwrap();
        assert!(matches!(
            Segment::open(&path),
            Err(Error::UnsupportedVersion {
                object: "segment",
                version: rejected
            }) if rejected == version
        ));
    }

    let segments = directory.path().join("segments");
    let (_, current) = Segment::write_from_memtable(&segments, &table()).unwrap();
    let mut bytes = std::fs::read(current).unwrap();
    bytes[8..10].copy_from_slice(&7u16.to_le_bytes());
    let unknown = directory.path().join("format-7.seg");
    std::fs::write(&unknown, bytes).unwrap();
    assert!(matches!(
        Segment::open(&unknown),
        Err(Error::UnsupportedVersion {
            object: "segment",
            version: 7
        })
    ));
}

#[test]
fn versions_remain_exact_when_one_key_exceeds_the_row_group_target() {
    let directory = tempfile::tempdir().unwrap();
    let mut database = rrd_lsm::Database::create(directory.path()).unwrap();
    let operations = (0..160)
        .map(|version| Mutation::Put {
            key: b"one-key".to_vec(),
            value: vec![version as u8; 1024],
        })
        .collect();
    database
        .write_owned(
            WriteBatch::new(operations).unwrap(),
            Durability::Authoritative,
        )
        .unwrap();
    database.flush_memtable(1).unwrap();

    for sequence in [1, 63, 64, 65, 127, 128, 160] {
        assert_eq!(
            database
                .get(b"one-key", rrd_lsm::Snapshot { sequence })
                .unwrap(),
            Some(vec![(sequence - 1) as u8; 1024])
        );
    }
}

#[test]
fn configured_row_group_row_budget_applies_to_flush_compaction_and_reopen() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("database");
    let budget = SegmentRowGroupBudget {
        max_rows: 2,
        target_bytes: 1024 * 1024,
    };
    let options = DatabaseOptions {
        segment_row_group_budget: budget,
        ..DatabaseOptions::default()
    };
    let mut database = Database::create_with_options(&root, options).unwrap();
    assert_eq!(database.segment_row_group_budget(), budget);

    let first = (0..6)
        .map(|index| Mutation::Put {
            key: format!("key:{index:02}").into_bytes(),
            value: format!("first:{index}").into_bytes(),
        })
        .collect();
    database
        .write_owned(WriteBatch::new(first).unwrap(), Durability::Authoritative)
        .unwrap();
    database.flush_memtable(1).unwrap();

    let first_descriptor = database.manifest().segments[0].clone();
    let first_segment = Segment::open(
        &root
            .join("segments")
            .join(format!("{}.seg", first_descriptor.id)),
    )
    .unwrap();
    assert_eq!(first_segment.row_group_budget(), budget);
    assert_eq!(first_segment.row_group_count(), 3);

    let second = (0..6)
        .map(|index| Mutation::Put {
            key: format!("key:{index:02}").into_bytes(),
            value: format!("second:{index}").into_bytes(),
        })
        .collect();
    database
        .write_owned(WriteBatch::new(second).unwrap(), Durability::Authoritative)
        .unwrap();
    database.flush_memtable(2).unwrap();
    let snapshot = database.snapshot();
    let outcome = database.compact(&[], 3).unwrap().unwrap();
    assert_eq!(outcome.output_segments, 1);

    let compacted_descriptor = database.manifest().segments[0].clone();
    let compacted = Segment::open(
        &root
            .join("segments")
            .join(format!("{}.seg", compacted_descriptor.id)),
    )
    .unwrap();
    assert_eq!(compacted.row_group_budget(), budget);
    assert_eq!(compacted.row_group_count(), 3);
    assert_eq!(
        database.get(b"key:03", snapshot).unwrap(),
        Some(b"second:3".to_vec())
    );

    drop(database);
    let reopened = Database::open_with_options(&root, options).unwrap();
    assert_eq!(reopened.segment_row_group_budget(), budget);
    assert_eq!(
        reopened.get(b"key:03", snapshot).unwrap(),
        Some(b"second:3".to_vec())
    );
    drop(reopened);

    let reopened_with_new_writer_defaults = Database::open(&root).unwrap();
    assert_eq!(
        reopened_with_new_writer_defaults.segment_row_group_budget(),
        SegmentRowGroupBudget::default()
    );
    assert_eq!(
        reopened_with_new_writer_defaults
            .get(b"key:03", snapshot)
            .unwrap(),
        Some(b"second:3".to_vec())
    );
}

#[test]
fn configured_row_group_byte_budget_splits_only_at_key_boundaries() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("database");
    let budget = SegmentRowGroupBudget {
        max_rows: 100,
        target_bytes: 256,
    };
    let mut database = Database::create_with_options(
        &root,
        DatabaseOptions {
            segment_row_group_budget: budget,
            ..DatabaseOptions::default()
        },
    )
    .unwrap();
    let operations = (0u8..4)
        .flat_map(|index| {
            [0u8, 1].map(move |version| Mutation::Put {
                key: format!("key:{index:02}").into_bytes(),
                value: vec![index * 2 + version; 96],
            })
        })
        .collect();
    database
        .write_owned(
            WriteBatch::new(operations).unwrap(),
            Durability::Authoritative,
        )
        .unwrap();
    database.flush_memtable(1).unwrap();

    let descriptor = database.manifest().segments[0].clone();
    let segment =
        Segment::open(&root.join("segments").join(format!("{}.seg", descriptor.id))).unwrap();
    assert_eq!(segment.row_group_budget(), budget);
    assert_eq!(segment.row_group_count(), 4);
    for sequence in 1..=8 {
        for index in 0u8..4 {
            let key = format!("key:{index:02}");
            let first_sequence = u64::from(index) * 2 + 1;
            let expected = if sequence < first_sequence {
                None
            } else {
                let version = u8::from(sequence > first_sequence);
                Some(vec![index * 2 + version; 96])
            };
            assert_eq!(
                segment.get(key.as_bytes(), sequence).unwrap(),
                expected,
                "key {key} at sequence {sequence}"
            );
        }
    }
}

#[test]
fn invalid_configured_and_authenticated_row_group_budgets_fail_closed() {
    let directory = tempfile::tempdir().unwrap();
    let mut invalid_budgets = vec![
        (
            "zero-rows",
            SegmentRowGroupBudget {
                max_rows: 0,
                target_bytes: 1024,
            },
        ),
        (
            "zero-bytes",
            SegmentRowGroupBudget {
                max_rows: 1,
                target_bytes: 0,
            },
        ),
        (
            "over-segment-bytes",
            SegmentRowGroupBudget {
                max_rows: 1,
                target_bytes: 1024 * 1024 * 1024 + 1,
            },
        ),
    ];
    if let Ok(max_rows) = usize::try_from(u64::from(u32::MAX) + 1) {
        invalid_budgets.push((
            "unrepresentable-rows",
            SegmentRowGroupBudget {
                max_rows,
                target_bytes: 1024,
            },
        ));
    }
    for (name, budget) in invalid_budgets {
        let root = directory.path().join(name);
        let result = Database::create_with_options(
            &root,
            DatabaseOptions {
                segment_row_group_budget: budget,
                ..DatabaseOptions::default()
            },
        );
        assert!(matches!(result, Err(Error::InvalidConfiguration(_))));
        assert!(!root.exists(), "invalid options must not create storage");
    }

    let segments = directory.path().join("segments");
    let (_, path) = Segment::write_from_memtable(&segments, &table()).unwrap();
    let original = std::fs::read(path).unwrap();
    for (name, field, value) in [
        ("zero-authenticated-bytes", 160, 0),
        ("zero-authenticated-rows", 168, 0),
        ("over-authenticated-bytes", 160, 1024 * 1024 * 1024 + 1),
        ("over-authenticated-rows", 168, u64::from(u32::MAX) + 1),
    ] {
        let mut bytes = original.clone();
        bytes[field..field + 8].copy_from_slice(&value.to_le_bytes());
        rewrite_checksum(&mut bytes);
        let path = directory.path().join(format!("{name}.seg"));
        std::fs::write(&path, bytes).unwrap();
        assert!(matches!(
            Segment::open(&path),
            Err(Error::InvalidSegment(_))
        ));
    }
}

#[test]
fn persisted_row_group_filters_prune_reopened_point_misses() {
    let directory = tempfile::tempdir().unwrap();
    let options = DatabaseOptions {
        segment_row_group_budget: SegmentRowGroupBudget {
            max_rows: 32,
            target_bytes: 1024 * 1024,
        },
        ..DatabaseOptions::default()
    };
    let mut database = Database::create_with_options(directory.path(), options).unwrap();
    let operations = (0..200)
        .map(|index| Mutation::Put {
            key: format!("key:{:04}", index * 2).into_bytes(),
            value: vec![index as u8; 128],
        })
        .collect();
    database
        .write_owned(
            WriteBatch::new(operations).unwrap(),
            Durability::Authoritative,
        )
        .unwrap();
    database
        .write_owned(
            WriteBatch::new(vec![Mutation::Delete {
                key: b"key:0000".to_vec(),
            }])
            .unwrap(),
            Durability::Authoritative,
        )
        .unwrap();
    database.flush_memtable(1).unwrap();
    let snapshot = database.snapshot();
    drop(database);

    let reopened = Database::open_with_options(directory.path(), options).unwrap();
    let open = reopened.segment_open_evidence();
    assert!(open.persisted_filter_count > 0);
    assert!(open.persisted_filter_bytes > 0);
    assert_eq!(open.semantic_page_operations, 0);
    assert_eq!(open.semantic_page_bytes, 0);
    let before_present = reopened.page_cache_stats();
    assert_eq!(
        reopened.get(b"key:0002", snapshot).unwrap(),
        Some(vec![1; 128])
    );
    assert_eq!(reopened.get(b"key:0000", snapshot).unwrap(), None);
    let after_present = reopened.page_cache_stats();
    assert!(after_present.loads > before_present.loads);
    drop(reopened);

    let reopened = Database::open_with_options(directory.path(), options).unwrap();
    let before = reopened.page_cache_stats();

    for index in 0..200 {
        let missing = format!("key:{:04}", index * 2 + 1);
        assert_eq!(reopened.get(missing.as_bytes(), snapshot).unwrap(), None);
    }
    let after = reopened.page_cache_stats();
    let checks = after.filter_checks - before.filter_checks;
    let negatives = after.filter_negatives - before.filter_negatives;
    let loads = after.loads - before.loads;
    assert!(checks > 0 && checks <= 199);
    assert!(negatives > 0);
    assert!(loads < checks);
}

#[test]
fn an_open_v6_segment_detects_later_page_tampering_on_read() {
    let directory = tempfile::tempdir().unwrap();
    let segments = directory.path().join("segments");
    let (segment, path) = Segment::write_from_memtable(&segments, &table()).unwrap();
    let original = std::fs::read(&path).unwrap();
    let mut file = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
    file.seek(SeekFrom::Start(256)).unwrap();
    file.write_all(&[original[256] ^ 0x40]).unwrap();
    file.sync_all().unwrap();

    assert!(matches!(
        segment.get(b"alpha", 1),
        Err(Error::InvalidSegment(_))
    ));
}

#[test]
fn database_page_cache_is_shared_bounded_and_observable() {
    let directory = tempfile::tempdir().unwrap();
    let mut database =
        rrd_lsm::Database::create_with_page_cache(directory.path(), 4 * 1024).unwrap();
    let operations = (0..300)
        .map(|index| Mutation::Put {
            key: format!("cache:{index:04}").into_bytes(),
            value: vec![(index % 251) as u8; 1024],
        })
        .collect();
    database
        .write_owned(
            WriteBatch::new(operations).unwrap(),
            Durability::Authoritative,
        )
        .unwrap();
    database.flush_memtable(1).unwrap();
    let snapshot = database.snapshot();

    assert_eq!(database.page_cache_stats().entries, 0);
    database.get(b"cache:0001", snapshot).unwrap();
    let after_miss = database.page_cache_stats();
    assert_eq!(after_miss.misses, 6);
    assert_eq!(after_miss.loads, 6);
    assert!(after_miss.bytes_read > 0);
    assert!(after_miss.bytes_decoded >= after_miss.bytes_read);
    database.get(b"cache:0001", snapshot).unwrap();
    let after_hit = database.page_cache_stats();
    assert!(after_hit.hits >= 5);
    assert!(after_hit.loads <= after_miss.loads + 1);
    assert!(after_hit.bytes_read >= after_miss.bytes_read);
    for index in [70, 140, 210, 299] {
        database
            .get(format!("cache:{index:04}").as_bytes(), snapshot)
            .unwrap();
    }
    let final_stats = database.page_cache_stats();
    assert!(final_stats.resident_bytes <= final_stats.capacity_bytes);
    assert!(final_stats.evictions > 0);
}
