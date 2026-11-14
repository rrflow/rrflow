//! Running a trial: corpus, pages and families.

use super::*;

pub fn run_physical_policy_trial(
    root: &Path,
    config: PhysicalPolicyConfig,
) -> Result<PhysicalPolicyTrial> {
    let config = config.validate()?;
    let started = Instant::now();
    let corpus = build_corpus(config)?;
    let operation_count = corpus.mutations.len() as u64;
    let mut table = Memtable::default();
    let first_sequence = 1;
    let last_sequence = operation_count;
    table.apply_owned_write_batch(
        WriteBatch::new(corpus.mutations.clone())?,
        first_sequence,
        last_sequence,
    )?;
    let encoded = encode(
        &table,
        integrated_row_group_budget(config)?,
        SegmentCompressionPolicy::None,
    )?;
    let segment_physical_bytes = encoded.bytes.len() as u64;
    let mut cursor = Cursor::new(&encoded.bytes);
    let metadata = parse_metadata(
        &mut cursor,
        segment_physical_bytes,
        encoded.checksum.clone(),
    )?;
    let pages = extract_pages(&encoded.bytes, &metadata.row_groups)?;
    let logical_page_bytes = pages.iter().try_fold(0u64, |total, page| {
        total
            .checked_add(page.bytes.len() as u64)
            .ok_or_else(|| Error::InvalidSegment("physical-policy page byte total overflow".into()))
    })?;

    let codecs = observe_codecs(&pages)?;
    let persisted_row_group_filter = observe_filters(&encoded.bytes, &metadata.row_groups, config)?;
    let cache_candidates = observe_caches(&pages, config.cache_bytes)?;
    let value_placement_candidate = observe_value_placement(&table)?;
    let families = observe_families(&pages)?;
    let reopened_point_miss = observe_reopened_database(root, config, &corpus)?;
    validate_persisted_filter_integration(
        metadata.row_groups.len(),
        &persisted_row_group_filter,
        &reopened_point_miss,
    )?;
    let integrated_cache_policies = observe_integrated_cache_policies(root, config, &corpus)?;
    validate_integrated_cache_policies(&integrated_cache_policies)?;
    let decisions = decide_candidates(
        &codecs,
        &persisted_row_group_filter,
        &cache_candidates,
        &integrated_cache_policies,
        &value_placement_candidate,
    );

    Ok(PhysicalPolicyTrial {
        evidence_version: PHYSICAL_POLICY_EVIDENCE_VERSION,
        evidence_scope: "c06j-scan-resistant-cache-integration".into(),
        config,
        corpus_digest: corpus.digest,
        operation_count,
        key_count: table.key_count() as u64,
        row_group_count: metadata.row_groups.len() as u64,
        page_count: pages.len() as u64,
        segment_format_version: SEGMENT_FORMAT_VERSION,
        segment_physical_bytes,
        logical_page_bytes,
        elapsed_nanoseconds: started.elapsed().as_nanos().try_into().unwrap_or(u64::MAX),
        peak_rss_bytes: peak_rss_bytes(),
        codecs,
        persisted_row_group_filter,
        cache_candidates,
        value_placement_candidate,
        families,
        reopened_point_miss,
        integrated_cache_policies,
        decisions,
        limitations: vec![
            "adaptive LZ4 and the exact-byte scan-resistant page cache are integrated; Zstandard and trace-only cache candidates remain unintegrated".into(),
            "value placement is a byte model only and lacks pointer publication, recovery, snapshot, corruption, range-read, and garbage-collection proof".into(),
            "the integrated cache comparison is single-threaded and records duplicate loads without proving load coalescing, lock scalability, cancellation fairness, or sustained concurrency".into(),
            "one machine and one deterministic corpus cannot establish release latency, every-workload policy, network behavior, or competitor superiority".into(),
            "operating-system device cache and competing host load are recorded but not controlled by this executable".into(),
        ],
    })
}

pub(super) fn build_corpus(config: PhysicalPolicyConfig) -> Result<Corpus> {
    let mut mutations = Vec::with_capacity(
        config.records_per_family * FAMILY_PREFIXES.len() * config.versions_per_key,
    );
    let mut expected_latest = BTreeMap::new();
    let mut digest_input = Vec::new();
    for (family_index, (_, prefix)) in FAMILY_PREFIXES.iter().enumerate() {
        for ordinal in 0..config.records_per_family {
            let key = corpus_key(prefix, ordinal, false);
            for version in 0..config.versions_per_key {
                let delete = version + 1 == config.versions_per_key && ordinal % 97 == 0;
                let mutation = if delete {
                    expected_latest.insert(key.clone(), None);
                    Mutation::Delete { key: key.clone() }
                } else {
                    let value = corpus_value(
                        config.seed,
                        family_index,
                        ordinal,
                        version,
                        config.value_bytes,
                    );
                    expected_latest.insert(key.clone(), Some(value.clone()));
                    Mutation::Put {
                        key: key.clone(),
                        value,
                    }
                };
                digest_mutation(&mut digest_input, &mutation)?;
                mutations.push(mutation);
            }
        }
    }
    Ok(Corpus {
        mutations,
        expected_latest,
        digest: sha256_hex(&digest_input),
    })
}

pub(super) fn corpus_key(prefix: &[u8], ordinal: usize, missing: bool) -> Vec<u8> {
    let numeric = ordinal
        .saturating_mul(2)
        .saturating_add(usize::from(missing));
    let mut key = Vec::with_capacity(prefix.len() + 8);
    key.extend_from_slice(prefix);
    key.extend_from_slice(format!("{numeric:08x}").as_bytes());
    key
}

pub(super) fn corpus_value(
    seed: u64,
    family: usize,
    ordinal: usize,
    version: usize,
    bytes: usize,
) -> Vec<u8> {
    let mut output = Vec::with_capacity(bytes);
    let structured =
        format!("family={family}|record={ordinal:08x}|version={version}|estate=rrflow|");
    let mut state = seed
        ^ (family as u64).rotate_left(11)
        ^ (ordinal as u64).rotate_left(29)
        ^ (version as u64).rotate_left(47);
    while output.len() < bytes {
        match family {
            0 | 3 => output.extend_from_slice(structured.as_bytes()),
            1 | 2 => output.extend_from_slice(b"from~edge~to|valid_from|valid_to|"),
            4 => output.extend_from_slice(b"state=ready|branch=open|lease=held|"),
            5 => output.extend_from_slice(&(ordinal as u64).to_be_bytes()),
            6 => {
                output.extend_from_slice(&(ordinal as u32).to_le_bytes());
                output.extend_from_slice(&(version as u32 + 1).to_le_bytes());
            }
            7 => {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                output.extend_from_slice(&state.to_le_bytes());
            }
            _ => unreachable!(),
        }
    }
    output.truncate(bytes);
    output
}

pub(super) fn digest_mutation(output: &mut Vec<u8>, mutation: &Mutation) -> Result<()> {
    let (tag, key, value) = match mutation {
        Mutation::Put { key, value } => (1u8, key.as_slice(), Some(value.as_slice())),
        Mutation::Delete { key } => (2u8, key.as_slice(), None),
    };
    output.push(tag);
    output.extend_from_slice(
        &u64::try_from(key.len())
            .map_err(|_| Error::InvalidConfiguration("corpus key length exceeds u64".into()))?
            .to_le_bytes(),
    );
    output.extend_from_slice(key);
    if let Some(value) = value {
        output.extend_from_slice(
            &u64::try_from(value.len())
                .map_err(|_| Error::InvalidConfiguration("corpus value length exceeds u64".into()))?
                .to_le_bytes(),
        );
        output.extend_from_slice(value);
    } else {
        output.extend_from_slice(&0u64.to_le_bytes());
    }
    Ok(())
}

pub(super) fn extract_pages(
    encoded: &[u8],
    row_groups: &[super::super::format::RowGroupDescriptor],
) -> Result<Vec<PageSample>> {
    let mut pages =
        Vec::with_capacity(row_groups.len() * super::super::format::PAGES_PER_ROW_GROUP);
    for (row_group, group) in row_groups.iter().enumerate() {
        let family = family_for_range(&group.first_key, &group.last_key);
        for descriptor in &group.pages {
            let start = usize::try_from(descriptor.offset)
                .map_err(|_| Error::InvalidSegment("page offset exceeds usize".into()))?;
            let end = start
                .checked_add(descriptor.physical_bytes)
                .ok_or_else(|| Error::InvalidSegment("page range overflow".into()))?;
            let bytes = encoded
                .get(start..end)
                .ok_or_else(|| Error::InvalidSegment("page range escaped encoded segment".into()))?
                .to_vec();
            if descriptor.physical_bytes != descriptor.logical_bytes {
                return Err(Error::InvalidSegment(
                    "the none-policy segment-v6 candidate corpus unexpectedly contained compression"
                        .into(),
                ));
            }
            pages.push(PageSample {
                id: row_group * super::super::format::PAGES_PER_ROW_GROUP
                    + descriptor.kind as usize,
                row_group,
                row_count: u64::from(group.row_count),
                family: family.clone(),
                kind: page_kind_name(descriptor.kind),
                bytes,
            });
        }
    }
    Ok(pages)
}

pub(super) fn page_kind_name(kind: PageKind) -> &'static str {
    match kind {
        PageKind::KeyOffsets => "key-offsets",
        PageKind::KeyData => "key-data",
        PageKind::SequenceValues => "sequence-values",
        PageKind::ValueValidity => "value-validity",
        PageKind::ValueOffsets => "value-offsets",
        PageKind::ValueData => "value-data",
    }
}

pub(super) fn family_for_range(first: &[u8], last: &[u8]) -> String {
    let first_family = family_name(first);
    let last_family = family_name(last);
    if first_family == last_family {
        first_family.into()
    } else {
        "mixed".into()
    }
}

pub(super) fn family_name(key: &[u8]) -> &'static str {
    FAMILY_PREFIXES
        .iter()
        .find_map(|(name, prefix)| key.starts_with(prefix).then_some(*name))
        .unwrap_or("unknown")
}

pub(super) fn observe_codecs(pages: &[PageSample]) -> Result<Vec<CodecObservation>> {
    ["none", "lz4", "zstd"]
        .into_iter()
        .map(|codec| observe_codec(codec, pages))
        .collect()
}
