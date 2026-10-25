use crate::{Error, Memtable, Result, SegmentDescriptor, VersionedValue};
use serde::{Deserialize, Serialize};
use std::io::{Read, Seek, SeekFrom};

#[cfg(test)]
#[path = "format_encoding_tests.rs"]
mod encoding_tests;

pub const SEGMENT_FORMAT_VERSION: u16 = 6;
pub const SEGMENT_MAGIC: &[u8; 8] = b"RRDSEG06";
pub const INDEX_MAGIC: &[u8; 8] = b"RRDIX006";
pub const PAGE_ALIGNMENT: usize = 64;
pub const SEGMENT_HEADER_BYTES: usize = 256;
pub const INDEX_HEADER_BYTES: usize = 32;
pub const ROW_GROUP_HEADER_BYTES: usize = 32;
pub const PAGE_DESCRIPTOR_BYTES: usize = 96;
pub const PAGES_PER_ROW_GROUP: usize = 6;
pub const FOOTER_BYTES: usize = 64;
pub const DEFAULT_ROW_GROUP_TARGET_BYTES: usize = 64 * 1024;
pub const DEFAULT_ROW_GROUP_MAX_ROWS: usize = 2 * 1024;
pub const MAX_SEGMENT_BYTES: u64 = 1024 * 1024 * 1024;
pub const MAX_INDEX_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_KEY_BYTES: usize = 1024 * 1024;
pub const MAX_VALUE_BYTES: usize = 8 * 1024 * 1024;
pub const COMPRESSION_BASIS_POINTS: u16 = 10_000;
pub const DEFAULT_COMPRESSION_MINIMUM_SAVINGS_BASIS_POINTS: u16 = 1_250;
pub const DEFAULT_COMPRESSION_MAXIMUM_PAGE_LOGICAL_BYTES: u64 = 16 * 1024 * 1024;
pub const FILTER_FORMAT_VERSION: u8 = 1;
pub const FILTER_BITS_PER_KEY: usize = 10;
pub const FILTER_HASH_FUNCTIONS: usize = 7;

/// The immutable v6 segment schema is intentionally narrower than the RRFlow
/// semantic model. Families such as graph adjacency, lexical postings, and
/// vectors remain transactionally encoded keys and values above this physical
/// layer; this digest identifies only their common MVCC storage columns.
pub const SEGMENT_SCHEMA_DIGEST: &str =
    "7f9ffd70086074178df404606f44560f7d04546ff21ae49e802935563e5c8861";
pub const SEGMENT_KEY_CODEC_DIGEST: &str =
    "542c8fdcc2419e2b8de3b54d85e4d1fc168d4c8705acceaa1b196883a20938da";
pub const SEGMENT_PAGE_FORMAT_DIGEST: &str =
    "250248477e88f988e5b8a5eecbb5e71a31fd57dabab9dd42d24e1bd58982a20d";

/// Authenticated policy used for future immutable segment output. Each v6
/// segment persists its validated policy so readers never depend on process
/// configuration to interpret durable bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum SegmentCompressionPolicy {
    None,
    AdaptiveLz4 {
        minimum_savings_basis_points: u16,
        maximum_page_logical_bytes: u64,
    },
}

pub const DEFAULT_SEGMENT_COMPRESSION_POLICY: SegmentCompressionPolicy =
    SegmentCompressionPolicy::AdaptiveLz4 {
        minimum_savings_basis_points: DEFAULT_COMPRESSION_MINIMUM_SAVINGS_BASIS_POINTS,
        maximum_page_logical_bytes: DEFAULT_COMPRESSION_MAXIMUM_PAGE_LOGICAL_BYTES,
    };

impl Default for SegmentCompressionPolicy {
    fn default() -> Self {
        DEFAULT_SEGMENT_COMPRESSION_POLICY
    }
}

impl SegmentCompressionPolicy {
    pub(crate) fn validate(self) -> Result<Self> {
        match self {
            Self::None => Ok(self),
            Self::AdaptiveLz4 {
                minimum_savings_basis_points,
                maximum_page_logical_bytes,
            } if (1..COMPRESSION_BASIS_POINTS).contains(&minimum_savings_basis_points)
                && (1..=MAX_SEGMENT_BYTES).contains(&maximum_page_logical_bytes) =>
            {
                Ok(self)
            }
            Self::AdaptiveLz4 { .. } => Err(Error::InvalidConfiguration(
                "adaptive LZ4 savings must be 1..=9999 basis points and the logical page limit must be 1 byte..=1 GiB".into(),
            )),
        }
    }

    pub const fn kind(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::AdaptiveLz4 { .. } => "adaptive_lz4",
        }
    }

    pub const fn minimum_savings_basis_points(self) -> u16 {
        match self {
            Self::None => 0,
            Self::AdaptiveLz4 {
                minimum_savings_basis_points,
                ..
            } => minimum_savings_basis_points,
        }
    }

    pub const fn maximum_page_logical_bytes(self) -> u64 {
        match self {
            Self::None => 0,
            Self::AdaptiveLz4 {
                maximum_page_logical_bytes,
                ..
            } => maximum_page_logical_bytes,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub(super) enum PageCompression {
    None = 0,
    Lz4Block = 1,
}

impl PageCompression {
    fn from_byte(value: u8) -> Result<Self> {
        match value {
            0 => Ok(Self::None),
            1 => Ok(Self::Lz4Block),
            other => invalid(format!("unknown v6 page compression codec {other}")),
        }
    }
}

/// Soft limits used when a mutable key/version table is transformed into
/// immutable Arrow-layout row groups. A complete version chain for one key is
/// indivisible and may therefore exceed either limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SegmentRowGroupBudget {
    pub max_rows: usize,
    pub target_bytes: usize,
}

impl Default for SegmentRowGroupBudget {
    fn default() -> Self {
        Self {
            max_rows: DEFAULT_ROW_GROUP_MAX_ROWS,
            target_bytes: DEFAULT_ROW_GROUP_TARGET_BYTES,
        }
    }
}

impl SegmentRowGroupBudget {
    pub(crate) fn validate(self) -> Result<Self> {
        let target_bytes = u64::try_from(self.target_bytes).ok();
        if self.max_rows == 0
            || self.max_rows > u32::MAX as usize
            || self.target_bytes == 0
            || target_bytes.is_none_or(|bytes| bytes > MAX_SEGMENT_BYTES)
        {
            return Err(Error::InvalidConfiguration(
                "segment row-group target bytes must fit within the segment limit and max rows must fit u32; both must be non-zero".into(),
            ));
        }
        Ok(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub(super) enum PageKind {
    KeyOffsets = 1,
    KeyData = 2,
    SequenceValues = 3,
    ValueValidity = 4,
    ValueOffsets = 5,
    ValueData = 6,
}

impl PageKind {
    pub(super) const ORDERED: [Self; PAGES_PER_ROW_GROUP] = [
        Self::KeyOffsets,
        Self::KeyData,
        Self::SequenceValues,
        Self::ValueValidity,
        Self::ValueOffsets,
        Self::ValueData,
    ];

    fn from_byte(value: u8) -> Result<Self> {
        match value {
            1 => Ok(Self::KeyOffsets),
            2 => Ok(Self::KeyData),
            3 => Ok(Self::SequenceValues),
            4 => Ok(Self::ValueValidity),
            5 => Ok(Self::ValueOffsets),
            6 => Ok(Self::ValueData),
            other => invalid(format!("unknown v6 page kind {other}")),
        }
    }

    fn column(self) -> u8 {
        match self {
            Self::KeyOffsets | Self::KeyData => 1,
            Self::SequenceValues => 2,
            Self::ValueValidity | Self::ValueOffsets | Self::ValueData => 3,
        }
    }

    fn buffer_kind(self) -> u8 {
        match self {
            Self::KeyOffsets | Self::ValueOffsets => 2,
            Self::KeyData | Self::SequenceValues | Self::ValueData => 3,
            Self::ValueValidity => 1,
        }
    }

    fn logical_type(self) -> u8 {
        match self {
            Self::SequenceValues => 2,
            _ => 1,
        }
    }

    fn physical_type(self) -> u8 {
        match self {
            Self::KeyOffsets | Self::ValueOffsets => 2,
            Self::SequenceValues => 3,
            Self::KeyData | Self::ValueData => 4,
            Self::ValueValidity => 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PageDescriptor {
    pub kind: PageKind,
    pub compression: PageCompression,
    pub row_start: u64,
    pub row_count: u32,
    pub null_count: u32,
    pub offset: u64,
    pub physical_bytes: usize,
    pub logical_bytes: usize,
    pub statistic_min: u64,
    pub statistic_max: u64,
    pub digest: [u8; 32],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PageStatistics {
    null_count: u32,
    minimum: u64,
    maximum: u64,
}

impl PageStatistics {
    const fn new(null_count: u32, minimum: u64, maximum: u64) -> Self {
        Self {
            null_count,
            minimum,
            maximum,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PageWriteMetadata {
    kind: PageKind,
    row_start: u64,
    row_count: u32,
    statistics: PageStatistics,
}

impl PageWriteMetadata {
    const fn new(
        kind: PageKind,
        row_start: u64,
        row_count: u32,
        statistics: PageStatistics,
    ) -> Self {
        Self {
            kind,
            row_start,
            row_count,
            statistics,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RowGroupDescriptor {
    pub row_start: u64,
    pub row_count: u32,
    pub unique_key_count: u32,
    pub first_key: Vec<u8>,
    pub last_key: Vec<u8>,
    pub pages: [PageDescriptor; PAGES_PER_ROW_GROUP],
    pub filter: RowGroupFilter,
}

impl RowGroupDescriptor {
    pub(super) fn page(&self, kind: PageKind) -> &PageDescriptor {
        &self.pages[kind as usize - 1]
    }
}

/// Canonical RRFlow Bloom filter stored in each authenticated row-group index
/// entry. A negative answer can skip a point read; a positive answer always
/// falls through to the exact sorted key/version spine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RowGroupFilter {
    words: Vec<u64>,
}

impl RowGroupFilter {
    pub(super) fn new(unique_key_count: u32) -> Result<Self> {
        let word_count = Self::word_count(unique_key_count)?;
        Ok(Self {
            words: vec![0; word_count],
        })
    }

    pub(super) fn from_words(unique_key_count: u32, words: Vec<u64>) -> Result<Self> {
        let expected = Self::word_count(unique_key_count)?;
        if words.len() != expected {
            return invalid("v6 row-group filter word count is noncanonical");
        }
        Ok(Self { words })
    }

    pub(super) fn word_count(unique_key_count: u32) -> Result<usize> {
        if unique_key_count == 0 {
            return invalid("v6 row-group filter has no unique keys");
        }
        let bits = usize::try_from(unique_key_count)
            .map_err(|_| Error::InvalidSegment("v6 unique-key count exceeds usize".into()))?
            .checked_mul(FILTER_BITS_PER_KEY)
            .ok_or_else(|| Error::InvalidSegment("v6 row-group filter size overflow".into()))?
            .max(u64::BITS as usize);
        Ok(bits.div_ceil(u64::BITS as usize))
    }

    pub(super) fn insert(&mut self, key: &[u8]) {
        for bit in self.positions(key) {
            self.words[bit / u64::BITS as usize] |= 1u64 << (bit % u64::BITS as usize);
        }
    }

    pub(super) fn may_contain(&self, key: &[u8]) -> bool {
        self.positions(key).into_iter().all(|bit| {
            self.words[bit / u64::BITS as usize] & (1u64 << (bit % u64::BITS as usize)) != 0
        })
    }

    pub(super) fn byte_len(&self) -> usize {
        self.words.len() * std::mem::size_of::<u64>()
    }

    pub(super) fn words(&self) -> &[u64] {
        &self.words
    }

    fn positions(&self, key: &[u8]) -> [usize; FILTER_HASH_FUNCTIONS] {
        let bit_count = u64::try_from(self.words.len())
            .expect("bounded filter word count fits u64")
            * u64::BITS as u64;
        let first = filter_hash(key, 0xcbf2_9ce4_8422_2325);
        let second = filter_hash(key, 0x9e37_79b9_7f4a_7c15) | 1;
        std::array::from_fn(|index| {
            usize::try_from(first.wrapping_add((index as u64).wrapping_mul(second)) % bit_count)
                .expect("bounded filter bit position fits usize")
        })
    }
}

fn filter_hash(key: &[u8], seed: u64) -> u64 {
    let mut hash = seed;
    for byte in key {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash ^= hash >> 33;
    hash = hash.wrapping_mul(0xff51_afd7_ed55_8ccd);
    hash ^= hash >> 33;
    hash = hash.wrapping_mul(0xc4ce_b9fe_1a85_ec53);
    hash ^ (hash >> 33)
}

pub(super) struct EncodedSegment {
    pub bytes: Vec<u8>,
    pub checksum: String,
    pub evidence: SegmentWriteEvidence,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct SegmentWriteEvidence {
    pub raw_page_count: u64,
    pub compressed_page_count: u64,
    pub stored_page_bytes: u64,
    pub logical_page_bytes: u64,
    pub compression_scratch_high_water_bytes: u64,
}

pub(super) struct ParsedMetadata {
    pub descriptor: SegmentDescriptor,
    pub row_group_budget: SegmentRowGroupBudget,
    pub compression_policy: SegmentCompressionPolicy,
    pub row_groups: Vec<RowGroupDescriptor>,
    pub header_bytes_read: u64,
    pub index_bytes_read: u64,
    pub padding_bytes_read: u64,
}

#[derive(Default)]
struct RowGroupBuilder {
    row_start: u64,
    keys: Vec<u8>,
    key_offsets: Vec<i64>,
    sequences: Vec<u64>,
    value_validity: Vec<u8>,
    values: Vec<u8>,
    value_offsets: Vec<i64>,
    first_key: Vec<u8>,
    last_key: Vec<u8>,
    unique_key_count: u32,
    null_count: u32,
    minimum_key_bytes: u64,
    maximum_key_bytes: u64,
    minimum_value_bytes: u64,
    maximum_value_bytes: u64,
}

impl RowGroupBuilder {
    fn new(row_start: u64) -> Self {
        Self {
            row_start,
            key_offsets: vec![0],
            value_offsets: vec![0],
            minimum_key_bytes: u64::MAX,
            minimum_value_bytes: u64::MAX,
            ..Self::default()
        }
    }

    fn reset(&mut self, row_start: u64) {
        self.row_start = row_start;
        self.keys.clear();
        self.key_offsets.clear();
        self.key_offsets.push(0);
        self.sequences.clear();
        self.value_validity.clear();
        self.values.clear();
        self.value_offsets.clear();
        self.value_offsets.push(0);
        self.first_key.clear();
        self.last_key.clear();
        self.unique_key_count = 0;
        self.null_count = 0;
        self.minimum_key_bytes = u64::MAX;
        self.maximum_key_bytes = 0;
        self.minimum_value_bytes = u64::MAX;
        self.maximum_value_bytes = 0;
    }

    fn rows(&self) -> usize {
        self.sequences.len()
    }

    fn estimated_bytes(&self) -> usize {
        self.keys
            .len()
            .saturating_add(self.values.len())
            .saturating_add(self.key_offsets.len().saturating_mul(8))
            .saturating_add(self.value_offsets.len().saturating_mul(8))
            .saturating_add(self.sequences.len().saturating_mul(8))
            .saturating_add(self.sequences.len().div_ceil(8))
    }

    fn append(&mut self, key: &[u8], version: &VersionedValue) -> Result<()> {
        validate_key_value(key, version.value.as_deref().unwrap_or_default())?;
        if self.first_key.is_empty() {
            self.first_key.extend_from_slice(key);
        }
        if self.last_key.as_slice() != key {
            self.unique_key_count = self.unique_key_count.checked_add(1).ok_or_else(|| {
                Error::InvalidSegment("v6 row-group unique-key count overflow".into())
            })?;
        }
        self.last_key.clear();
        self.last_key.extend_from_slice(key);
        self.keys.extend_from_slice(key);
        self.key_offsets
            .push(i64::try_from(self.keys.len()).map_err(|_| {
                Error::InvalidSegment("v6 key page length exceeds signed Arrow offsets".into())
            })?);
        self.sequences.push(version.sequence);
        let row = self.rows() - 1;
        let byte = row / 8;
        if self.value_validity.len() <= byte {
            self.value_validity.resize(byte + 1, 0);
        }
        if version.value.is_some() {
            self.value_validity[byte] |= 1 << (row % 8);
        } else {
            self.null_count = self.null_count.saturating_add(1);
        }
        let value = version.value.as_deref().unwrap_or_default();
        self.values.extend_from_slice(value);
        self.value_offsets
            .push(i64::try_from(self.values.len()).map_err(|_| {
                Error::InvalidSegment("v6 value page length exceeds signed Arrow offsets".into())
            })?);
        let key_bytes = key.len() as u64;
        let value_bytes = value.len() as u64;
        self.minimum_key_bytes = self.minimum_key_bytes.min(key_bytes);
        self.maximum_key_bytes = self.maximum_key_bytes.max(key_bytes);
        self.minimum_value_bytes = self.minimum_value_bytes.min(value_bytes);
        self.maximum_value_bytes = self.maximum_value_bytes.max(value_bytes);
        Ok(())
    }

    fn filter(&self) -> Result<RowGroupFilter> {
        let mut filter = RowGroupFilter::new(self.unique_key_count)?;
        let mut previous: Option<&[u8]> = None;
        for offsets in self.key_offsets.windows(2) {
            let start = usize::try_from(offsets[0])
                .map_err(|_| Error::InvalidSegment("v6 key offset is negative".into()))?;
            let end = usize::try_from(offsets[1])
                .map_err(|_| Error::InvalidSegment("v6 key offset is negative".into()))?;
            let key = self
                .keys
                .get(start..end)
                .ok_or_else(|| Error::InvalidSegment("v6 key offsets escape key data".into()))?;
            if previous != Some(key) {
                filter.insert(key);
                previous = Some(key);
            }
        }
        Ok(filter)
    }
}

pub(super) fn encode(
    table: &Memtable,
    row_group_budget: SegmentRowGroupBudget,
    compression_policy: SegmentCompressionPolicy,
) -> Result<EncodedSegment> {
    encode_versions(table.all_versions(), row_group_budget, compression_policy)
}

pub(super) fn encode_versions<'a>(
    versions: impl Iterator<Item = (&'a [u8], &'a [VersionedValue])> + Clone,
    row_group_budget: SegmentRowGroupBudget,
    compression_policy: SegmentCompressionPolicy,
) -> Result<EncodedSegment> {
    let row_group_budget = row_group_budget.validate()?;
    let compression_policy = compression_policy.validate()?;
    let (entries, minimum_sequence, maximum_sequence) = versions
        .clone()
        .flat_map(|(_, versions)| versions.iter())
        .try_fold(
            (0u64, u64::MAX, 0u64),
            |(entries, minimum, maximum), version| {
                let entries = entries
                    .checked_add(1)
                    .ok_or_else(|| Error::InvalidSegment("v6 entry count exceeds u64".into()))?;
                Ok::<_, Error>((
                    entries,
                    minimum.min(version.sequence),
                    maximum.max(version.sequence),
                ))
            },
        )?;
    if entries == 0 {
        return invalid("cannot write an empty segment");
    }

    let mut output = vec![0; SEGMENT_HEADER_BYTES];
    let mut write_evidence = SegmentWriteEvidence::default();
    let mut row_groups = Vec::new();
    // The encoder owns one live row group and one bounded compression buffer.
    // Earlier payload columns are released/reused before reading the next group.
    let mut scratch = Vec::new();
    let mut current = RowGroupBuilder::new(0);
    for (key, versions) in versions {
        let key_bytes = versions.iter().try_fold(0usize, |total, version| {
            let value = version.value.as_deref().unwrap_or_default();
            validate_key_value(key, value)?;
            total
                .checked_add(key.len())
                .and_then(|total| total.checked_add(value.len()))
                .and_then(|total| total.checked_add(24))
                .ok_or_else(|| Error::InvalidSegment("v6 row-group estimate overflow".into()))
        })?;
        if current.rows() > 0
            && (current.rows().saturating_add(versions.len()) > row_group_budget.max_rows
                || current.estimated_bytes().saturating_add(key_bytes)
                    > row_group_budget.target_bytes)
        {
            let next = current
                .row_start
                .checked_add(current.rows() as u64)
                .ok_or_else(|| Error::InvalidSegment("v6 row offset overflow".into()))?;
            row_groups.push(append_row_group(
                &current,
                &mut output,
                compression_policy,
                &mut scratch,
                &mut write_evidence,
            )?);
            current.reset(next);
        }
        for version in versions {
            current.append(key, version)?;
        }
    }
    if current.rows() > 0 {
        row_groups.push(append_row_group(
            &current,
            &mut output,
            compression_policy,
            &mut scratch,
            &mut write_evidence,
        )?);
    }

    align(&mut output);
    let index_offset = output.len() as u64;
    output.extend_from_slice(INDEX_MAGIC);
    output.extend_from_slice(&(row_groups.len() as u32).to_le_bytes());
    output.extend_from_slice(&(PAGES_PER_ROW_GROUP as u16).to_le_bytes());
    output.extend_from_slice(&(ROW_GROUP_HEADER_BYTES as u16).to_le_bytes());
    output.extend_from_slice(&(PAGE_DESCRIPTOR_BYTES as u16).to_le_bytes());
    output.push(FILTER_FORMAT_VERSION);
    output.push(FILTER_BITS_PER_KEY as u8);
    output.push(FILTER_HASH_FUNCTIONS as u8);
    output.extend_from_slice(&[0; 11]);
    for group in &row_groups {
        let filter_word_count = u32::try_from(group.filter.words().len())
            .map_err(|_| Error::InvalidSegment("v6 filter word count exceeds u32".into()))?;
        output.extend_from_slice(&group.row_start.to_le_bytes());
        output.extend_from_slice(&group.row_count.to_le_bytes());
        output.extend_from_slice(&(group.first_key.len() as u32).to_le_bytes());
        output.extend_from_slice(&(group.last_key.len() as u32).to_le_bytes());
        output.extend_from_slice(&(PAGES_PER_ROW_GROUP as u16).to_le_bytes());
        output.extend_from_slice(&[0; 2]);
        output.extend_from_slice(&group.unique_key_count.to_le_bytes());
        output.extend_from_slice(&filter_word_count.to_le_bytes());
        output.extend_from_slice(&group.first_key);
        output.extend_from_slice(&group.last_key);
        for page in &group.pages {
            encode_page_descriptor(&mut output, page);
        }
        for word in group.filter.words() {
            output.extend_from_slice(&word.to_le_bytes());
        }
    }
    let index_bytes = output.len() as u64 - index_offset;
    if index_bytes as usize > MAX_INDEX_BYTES {
        return invalid("v6 index exceeds its bounded contract");
    }
    if output.len() as u64 > MAX_SEGMENT_BYTES - FOOTER_BYTES as u64 {
        return invalid("encoded v6 segment exceeds the 1 GiB safety limit");
    }

    output[..8].copy_from_slice(SEGMENT_MAGIC);
    output[8..10].copy_from_slice(&SEGMENT_FORMAT_VERSION.to_le_bytes());
    output[10..12].copy_from_slice(&(SEGMENT_HEADER_BYTES as u16).to_le_bytes());
    output[12..16].copy_from_slice(&0u32.to_le_bytes());
    output[16..24].copy_from_slice(&entries.to_le_bytes());
    output[24..32].copy_from_slice(&minimum_sequence.to_le_bytes());
    output[32..40].copy_from_slice(&maximum_sequence.to_le_bytes());
    output[40..44].copy_from_slice(&(row_groups.len() as u32).to_le_bytes());
    output[44..46].copy_from_slice(&(PAGES_PER_ROW_GROUP as u16).to_le_bytes());
    output[46..48].copy_from_slice(&(PAGE_ALIGNMENT as u16).to_le_bytes());
    output[48..56].copy_from_slice(&index_offset.to_le_bytes());
    output[56..64].copy_from_slice(&index_bytes.to_le_bytes());
    output[64..96].copy_from_slice(&decode_fixed_digest(SEGMENT_SCHEMA_DIGEST));
    output[96..128].copy_from_slice(&decode_fixed_digest(SEGMENT_KEY_CODEC_DIGEST));
    output[128..160].copy_from_slice(&decode_fixed_digest(SEGMENT_PAGE_FORMAT_DIGEST));
    output[160..168].copy_from_slice(&(row_group_budget.target_bytes as u64).to_le_bytes());
    output[168..176].copy_from_slice(&(row_group_budget.max_rows as u64).to_le_bytes());
    encode_compression_policy(&mut output, compression_policy);

    validate_logical_envelope(&row_groups, index_bytes)?;

    let checksum = sha256_hex(&output);
    output.extend_from_slice(checksum.as_bytes());
    Ok(EncodedSegment {
        bytes: output,
        checksum,
        evidence: write_evidence,
    })
}

fn append_row_group(
    builder: &RowGroupBuilder,
    output: &mut Vec<u8>,
    compression_policy: SegmentCompressionPolicy,
    scratch: &mut Vec<u8>,
    write_evidence: &mut SegmentWriteEvidence,
) -> Result<RowGroupDescriptor> {
    let row_count = u32::try_from(builder.rows())
        .map_err(|_| Error::InvalidSegment("v6 row-group count exceeds u32".into()))?;
    let unique_key_count = builder.unique_key_count;
    let filter = builder.filter()?;
    let sequence_min = *builder
        .sequences
        .iter()
        .min()
        .expect("non-empty row group has sequences");
    let sequence_max = *builder
        .sequences
        .iter()
        .max()
        .expect("non-empty row group has sequences");
    let key_offsets = encode_i64_values(&builder.key_offsets);
    let sequences = encode_u64_values(&builder.sequences);
    let value_offsets = encode_i64_values(&builder.value_offsets);
    let pages = [
        append_page(
            output,
            PageWriteMetadata::new(
                PageKind::KeyOffsets,
                builder.row_start,
                row_count,
                PageStatistics::new(0, builder.minimum_key_bytes, builder.maximum_key_bytes),
            ),
            &key_offsets,
            compression_policy,
            scratch,
            write_evidence,
        )?,
        append_page(
            output,
            PageWriteMetadata::new(
                PageKind::KeyData,
                builder.row_start,
                row_count,
                PageStatistics::new(0, builder.minimum_key_bytes, builder.maximum_key_bytes),
            ),
            &builder.keys,
            compression_policy,
            scratch,
            write_evidence,
        )?,
        append_page(
            output,
            PageWriteMetadata::new(
                PageKind::SequenceValues,
                builder.row_start,
                row_count,
                PageStatistics::new(0, sequence_min, sequence_max),
            ),
            &sequences,
            compression_policy,
            scratch,
            write_evidence,
        )?,
        append_page(
            output,
            PageWriteMetadata::new(
                PageKind::ValueValidity,
                builder.row_start,
                row_count,
                PageStatistics::new(builder.null_count, 0, 1),
            ),
            &builder.value_validity,
            compression_policy,
            scratch,
            write_evidence,
        )?,
        append_page(
            output,
            PageWriteMetadata::new(
                PageKind::ValueOffsets,
                builder.row_start,
                row_count,
                PageStatistics::new(
                    builder.null_count,
                    builder.minimum_value_bytes,
                    builder.maximum_value_bytes,
                ),
            ),
            &value_offsets,
            compression_policy,
            scratch,
            write_evidence,
        )?,
        append_page(
            output,
            PageWriteMetadata::new(
                PageKind::ValueData,
                builder.row_start,
                row_count,
                PageStatistics::new(
                    builder.null_count,
                    builder.minimum_value_bytes,
                    builder.maximum_value_bytes,
                ),
            ),
            &builder.values,
            compression_policy,
            scratch,
            write_evidence,
        )?,
    ];
    Ok(RowGroupDescriptor {
        row_start: builder.row_start,
        row_count,
        unique_key_count,
        first_key: builder.first_key.clone(),
        last_key: builder.last_key.clone(),
        pages,
        filter,
    })
}

pub(super) fn parse_metadata(
    reader: &mut (impl Read + Seek),
    physical_bytes: u64,
    checksum: String,
) -> Result<ParsedMetadata> {
    if physical_bytes < (SEGMENT_HEADER_BYTES + INDEX_HEADER_BYTES + FOOTER_BYTES) as u64 {
        return invalid("v6 segment is shorter than its framing");
    }
    let mut header = [0; SEGMENT_HEADER_BYTES];
    reader.seek(SeekFrom::Start(0))?;
    reader.read_exact(&mut header)?;
    require_current_format(&header)?;
    if read_u16(&header, 10)? as usize != SEGMENT_HEADER_BYTES
        || read_u32(&header, 12)? != 0
        || read_u16(&header, 44)? as usize != PAGES_PER_ROW_GROUP
        || read_u16(&header, 46)? as usize != PAGE_ALIGNMENT
        || header[188..].iter().any(|byte| *byte != 0)
    {
        return invalid("v6 segment header contains unsupported framing or flags");
    }
    let entries = read_u64(&header, 16)?;
    let minimum_sequence = read_u64(&header, 24)?;
    let maximum_sequence = read_u64(&header, 32)?;
    let row_group_count = read_u32(&header, 40)? as usize;
    let index_offset = read_u64(&header, 48)?;
    let index_bytes = usize::try_from(read_u64(&header, 56)?)
        .map_err(|_| Error::InvalidSegment("v6 index length exceeds usize".into()))?;
    let row_group_budget = decode_row_group_budget(&header)?;
    let compression_policy = decode_compression_policy(&header)?;
    let content_end = physical_bytes - FOOTER_BYTES as u64;
    if entries == 0
        || minimum_sequence == 0
        || minimum_sequence > maximum_sequence
        || row_group_count == 0
        || index_offset < SEGMENT_HEADER_BYTES as u64
        || index_offset % PAGE_ALIGNMENT as u64 != 0
        || !(INDEX_HEADER_BYTES..=MAX_INDEX_BYTES).contains(&index_bytes)
        || index_offset.checked_add(index_bytes as u64) != Some(content_end)
    {
        return invalid("v6 segment header contract is invalid");
    }
    if encode_digest(&header[64..96]) != SEGMENT_SCHEMA_DIGEST
        || encode_digest(&header[96..128]) != SEGMENT_KEY_CODEC_DIGEST
        || encode_digest(&header[128..160]) != SEGMENT_PAGE_FORMAT_DIGEST
    {
        return invalid("v6 segment schema, key codec, or page format digest is unknown");
    }

    reader.seek(SeekFrom::Start(index_offset))?;
    let mut index = vec![0; index_bytes];
    reader.read_exact(&mut index)?;
    if index.get(..8) != Some(INDEX_MAGIC.as_slice())
        || read_u32(&index, 8)? as usize != row_group_count
        || read_u16(&index, 12)? as usize != PAGES_PER_ROW_GROUP
        || read_u16(&index, 14)? as usize != ROW_GROUP_HEADER_BYTES
        || read_u16(&index, 16)? as usize != PAGE_DESCRIPTOR_BYTES
        || index[18] != FILTER_FORMAT_VERSION
        || index[19] as usize != FILTER_BITS_PER_KEY
        || index[20] as usize != FILTER_HASH_FUNCTIONS
        || index[21..32].iter().any(|byte| *byte != 0)
    {
        return invalid("v6 segment index header or filter policy is invalid");
    }

    let minimum_group_bytes = ROW_GROUP_HEADER_BYTES
        .checked_add(PAGES_PER_ROW_GROUP * PAGE_DESCRIPTOR_BYTES)
        .and_then(|bytes| bytes.checked_add(std::mem::size_of::<u64>()))
        .expect("fixed v6 row-group metadata size fits usize");
    if row_group_count
        > index_bytes
            .saturating_sub(INDEX_HEADER_BYTES)
            .checked_div(minimum_group_bytes)
            .unwrap_or(0)
    {
        return invalid("v6 row-group count cannot fit in the bounded index");
    }

    let mut cursor = INDEX_HEADER_BYTES;
    let mut row_groups = Vec::with_capacity(row_group_count);
    let mut expected_row_start = 0u64;
    let mut prior_last_key: Option<Vec<u8>> = None;
    let mut prior_page_end = SEGMENT_HEADER_BYTES as u64;
    for _ in 0..row_group_count {
        let fixed = slice(
            &index,
            cursor,
            ROW_GROUP_HEADER_BYTES,
            "v6 row-group header",
        )?;
        let row_start = read_u64(fixed, 0)?;
        let row_count = read_u32(fixed, 8)?;
        let first_key_bytes = read_u32(fixed, 12)? as usize;
        let last_key_bytes = read_u32(fixed, 16)? as usize;
        let unique_key_count = read_u32(fixed, 24)?;
        let filter_word_count = read_u32(fixed, 28)? as usize;
        if row_start != expected_row_start
            || row_count == 0
            || read_u16(fixed, 20)? as usize != PAGES_PER_ROW_GROUP
            || fixed[22..24].iter().any(|byte| *byte != 0)
            || unique_key_count == 0
            || unique_key_count > row_count
            || filter_word_count != RowGroupFilter::word_count(unique_key_count)?
            || first_key_bytes == 0
            || first_key_bytes > MAX_KEY_BYTES
            || last_key_bytes == 0
            || last_key_bytes > MAX_KEY_BYTES
        {
            return invalid("v6 row-group framing is invalid");
        }
        cursor += ROW_GROUP_HEADER_BYTES;
        let first_key = slice(&index, cursor, first_key_bytes, "v6 first key")?.to_vec();
        cursor += first_key_bytes;
        let last_key = slice(&index, cursor, last_key_bytes, "v6 last key")?.to_vec();
        cursor += last_key_bytes;
        if first_key > last_key
            || prior_last_key
                .as_ref()
                .is_some_and(|prior| prior >= &first_key)
        {
            return invalid("v6 row-group key ranges are not strictly ordered");
        }
        let mut pages = Vec::with_capacity(PAGES_PER_ROW_GROUP);
        for expected_kind in PageKind::ORDERED {
            let encoded = slice(&index, cursor, PAGE_DESCRIPTOR_BYTES, "v6 page descriptor")?;
            let page = decode_page_descriptor(encoded, compression_policy)?;
            if page.kind != expected_kind
                || page.row_start != row_start
                || page.row_count != row_count
                || page.offset % PAGE_ALIGNMENT as u64 != 0
                || page.offset < prior_page_end
                || page.offset.saturating_sub(prior_page_end) >= PAGE_ALIGNMENT as u64
                || page
                    .offset
                    .checked_add(page.physical_bytes as u64)
                    .is_none_or(|end| end > index_offset)
            {
                return invalid("v6 page descriptor violates ordering or bounds");
            }
            prior_page_end = page.offset + page.physical_bytes as u64;
            pages.push(page);
            cursor += PAGE_DESCRIPTOR_BYTES;
        }
        let filter_bytes = filter_word_count
            .checked_mul(std::mem::size_of::<u64>())
            .ok_or_else(|| Error::InvalidSegment("v6 filter byte count overflow".into()))?;
        let encoded_filter = slice(&index, cursor, filter_bytes, "v6 row-group filter")?;
        let filter = RowGroupFilter::from_words(
            unique_key_count,
            encoded_filter
                .as_chunks::<8>()
                .0
                .iter()
                .map(|word| u64::from_le_bytes(*word))
                .collect(),
        )?;
        cursor = cursor
            .checked_add(filter_bytes)
            .ok_or_else(|| Error::InvalidSegment("v6 filter cursor overflow".into()))?;
        validate_page_shapes(&pages, row_count)?;
        expected_row_start = expected_row_start
            .checked_add(u64::from(row_count))
            .ok_or_else(|| Error::InvalidSegment("v6 row count overflow".into()))?;
        prior_last_key = Some(last_key.clone());
        row_groups.push(RowGroupDescriptor {
            row_start,
            row_count,
            unique_key_count,
            first_key,
            last_key,
            pages: pages.try_into().expect("v6 page count was fixed"),
            filter,
        });
    }
    if cursor != index.len() || expected_row_start != entries {
        return invalid("v6 index does not exactly describe all rows");
    }
    validate_logical_envelope(&row_groups, index_bytes as u64)?;
    let padding_bytes_read = validate_zero_padding(reader, &row_groups, index_offset)?;
    let first_key = row_groups
        .first()
        .expect("validated v6 segment has a row group")
        .first_key
        .clone();
    let last_key = row_groups
        .last()
        .expect("validated v6 segment has a row group")
        .last_key
        .clone();
    Ok(ParsedMetadata {
        descriptor: SegmentDescriptor {
            id: checksum.clone(),
            level: 0,
            format_version: SEGMENT_FORMAT_VERSION,
            schema_digest: SEGMENT_SCHEMA_DIGEST.into(),
            key_codec_digest: SEGMENT_KEY_CODEC_DIGEST.into(),
            page_format_digest: SEGMENT_PAGE_FORMAT_DIGEST.into(),
            first_key,
            last_key,
            minimum_sequence,
            maximum_sequence,
            entries,
            bytes: physical_bytes,
            checksum,
        },
        row_group_budget,
        compression_policy,
        row_groups,
        header_bytes_read: SEGMENT_HEADER_BYTES as u64,
        index_bytes_read: index_bytes as u64,
        padding_bytes_read,
    })
}

fn encode_compression_policy(output: &mut [u8], policy: SegmentCompressionPolicy) {
    match policy {
        SegmentCompressionPolicy::None => {
            output[176] = 0;
            output[177] = 0;
            output[178..180].copy_from_slice(&0u16.to_le_bytes());
            output[180..188].copy_from_slice(&0u64.to_le_bytes());
        }
        SegmentCompressionPolicy::AdaptiveLz4 {
            minimum_savings_basis_points,
            maximum_page_logical_bytes,
        } => {
            output[176] = 1;
            output[177] = PageCompression::Lz4Block as u8;
            output[178..180].copy_from_slice(&minimum_savings_basis_points.to_le_bytes());
            output[180..188].copy_from_slice(&maximum_page_logical_bytes.to_le_bytes());
        }
    }
}

fn decode_compression_policy(header: &[u8]) -> Result<SegmentCompressionPolicy> {
    let minimum_savings_basis_points = read_u16(header, 178)?;
    let maximum_page_logical_bytes = read_u64(header, 180)?;
    match (
        header[176],
        header[177],
        minimum_savings_basis_points,
        maximum_page_logical_bytes,
    ) {
        (0, 0, 0, 0) => Ok(SegmentCompressionPolicy::None),
        (1, 1, minimum_savings_basis_points, maximum_page_logical_bytes) => {
            SegmentCompressionPolicy::AdaptiveLz4 {
                minimum_savings_basis_points,
                maximum_page_logical_bytes,
            }
            .validate()
            .map_err(|_| Error::InvalidSegment("v6 compression policy is invalid".into()))
        }
        _ => invalid("v6 compression policy metadata is unknown or noncanonical"),
    }
}

fn validate_logical_envelope(row_groups: &[RowGroupDescriptor], index_bytes: u64) -> Result<()> {
    let mut cursor = SEGMENT_HEADER_BYTES as u64;
    for page in row_groups.iter().flat_map(|group| group.pages.iter()) {
        cursor = align_u64(cursor)?;
        cursor = cursor
            .checked_add(
                u64::try_from(page.logical_bytes).map_err(|_| {
                    Error::InvalidSegment("v6 logical page length exceeds u64".into())
                })?,
            )
            .ok_or_else(|| Error::InvalidSegment("v6 logical segment length overflow".into()))?;
    }
    cursor = align_u64(cursor)?;
    cursor = cursor
        .checked_add(index_bytes)
        .and_then(|bytes| bytes.checked_add(FOOTER_BYTES as u64))
        .ok_or_else(|| Error::InvalidSegment("v6 logical segment envelope overflow".into()))?;
    if cursor > MAX_SEGMENT_BYTES {
        return invalid("v6 reconstructed logical segment exceeds the 1 GiB safety limit");
    }
    Ok(())
}

fn align_u64(value: u64) -> Result<u64> {
    value
        .checked_add(PAGE_ALIGNMENT as u64 - 1)
        .map(|value| value / PAGE_ALIGNMENT as u64 * PAGE_ALIGNMENT as u64)
        .ok_or_else(|| Error::InvalidSegment("v6 logical alignment overflow".into()))
}

fn decode_row_group_budget(header: &[u8]) -> Result<SegmentRowGroupBudget> {
    let target_bytes = usize::try_from(read_u64(header, 160)?)
        .map_err(|_| Error::InvalidSegment("v6 row-group byte target exceeds usize".into()))?;
    let max_rows = usize::try_from(read_u64(header, 168)?)
        .map_err(|_| Error::InvalidSegment("v6 row-group row limit exceeds usize".into()))?;
    if target_bytes == 0
        || target_bytes as u64 > MAX_SEGMENT_BYTES
        || max_rows == 0
        || max_rows > u32::MAX as usize
    {
        return invalid("v6 segment row-group budget is invalid");
    }
    Ok(SegmentRowGroupBudget {
        max_rows,
        target_bytes,
    })
}

pub(super) fn require_current_format(bytes: &[u8]) -> Result<()> {
    if let Some(version) = match bytes.get(..8) {
        Some(b"RRDSEG01") => Some(1),
        Some(b"RRDSEG02") => Some(2),
        Some(b"RRDSEG03") => Some(3),
        Some(b"RRDSEG04") => Some(4),
        Some(b"RRDSEG05") => Some(5),
        _ => None,
    } {
        return Err(Error::UnsupportedVersion {
            object: "segment",
            version,
        });
    }
    let version = bytes
        .get(8..10)
        .map(|field| u16::from_le_bytes(field.try_into().expect("fixed version field")))
        .ok_or_else(|| Error::InvalidSegment("segment has no complete format header".into()))?;
    if version != SEGMENT_FORMAT_VERSION {
        return Err(Error::UnsupportedVersion {
            object: "segment",
            version,
        });
    }
    if bytes.get(..8) != Some(SEGMENT_MAGIC.as_slice()) {
        return invalid("segment magic does not match its format version");
    }
    Ok(())
}

fn append_page(
    output: &mut Vec<u8>,
    metadata: PageWriteMetadata,
    bytes: &[u8],
    compression_policy: SegmentCompressionPolicy,
    scratch: &mut Vec<u8>,
    evidence: &mut SegmentWriteEvidence,
) -> Result<PageDescriptor> {
    align(output);
    let offset = output.len() as u64;
    let logical_bytes = u64::try_from(bytes.len())
        .map_err(|_| Error::InvalidSegment("v6 logical page length exceeds u64".into()))?;
    evidence.logical_page_bytes = evidence
        .logical_page_bytes
        .checked_add(logical_bytes)
        .ok_or_else(|| Error::InvalidSegment("v6 logical page evidence overflow".into()))?;

    let (compression, stored) = match compression_policy {
        SegmentCompressionPolicy::None => (PageCompression::None, bytes),
        SegmentCompressionPolicy::AdaptiveLz4 {
            minimum_savings_basis_points,
            maximum_page_logical_bytes,
        } if !bytes.is_empty() && logical_bytes <= maximum_page_logical_bytes => {
            let maximum_output = lz4_flex::block::get_maximum_output_size(bytes.len());
            if scratch.len() < maximum_output {
                scratch
                    .try_reserve_exact(maximum_output - scratch.len())
                    .map_err(|_| {
                        Error::InvalidSegment("v6 LZ4 compression scratch allocation failed".into())
                    })?;
                scratch.resize(maximum_output, 0);
            }
            evidence.compression_scratch_high_water_bytes = evidence
                .compression_scratch_high_water_bytes
                .max(u64::try_from(scratch.capacity()).map_err(|_| {
                    Error::InvalidSegment("v6 LZ4 compression scratch exceeds u64".into())
                })?);
            let compressed_bytes =
                lz4_flex::block::compress_into(bytes, &mut scratch[..maximum_output]).map_err(
                    |error| Error::InvalidSegment(format!("v6 LZ4 encode failed: {error}")),
                )?;
            let compressed = scratch.get(..compressed_bytes).ok_or_else(|| {
                Error::InvalidSegment("v6 LZ4 encoder returned an invalid length".into())
            })?;
            if compression_saves(
                logical_bytes,
                u64::try_from(compressed_bytes).map_err(|_| {
                    Error::InvalidSegment("v6 compressed page length exceeds u64".into())
                })?,
                minimum_savings_basis_points,
            )? {
                (PageCompression::Lz4Block, compressed)
            } else {
                (PageCompression::None, bytes)
            }
        }
        SegmentCompressionPolicy::AdaptiveLz4 { .. } => (PageCompression::None, bytes),
    };
    output.extend_from_slice(stored);
    let stored_bytes = u64::try_from(stored.len())
        .map_err(|_| Error::InvalidSegment("v6 stored page length exceeds u64".into()))?;
    evidence.stored_page_bytes = evidence
        .stored_page_bytes
        .checked_add(stored_bytes)
        .ok_or_else(|| Error::InvalidSegment("v6 stored page evidence overflow".into()))?;
    match compression {
        PageCompression::None => {
            evidence.raw_page_count = evidence
                .raw_page_count
                .checked_add(1)
                .ok_or_else(|| Error::InvalidSegment("v6 raw page count overflow".into()))?;
        }
        PageCompression::Lz4Block => {
            evidence.compressed_page_count = evidence
                .compressed_page_count
                .checked_add(1)
                .ok_or_else(|| Error::InvalidSegment("v6 compressed page count overflow".into()))?;
        }
    }
    Ok(PageDescriptor {
        kind: metadata.kind,
        compression,
        row_start: metadata.row_start,
        row_count: metadata.row_count,
        null_count: metadata.statistics.null_count,
        offset,
        physical_bytes: stored.len(),
        logical_bytes: bytes.len(),
        statistic_min: metadata.statistics.minimum,
        statistic_max: metadata.statistics.maximum,
        digest: sha256(stored),
    })
}

fn encode_page_descriptor(output: &mut Vec<u8>, page: &PageDescriptor) {
    output.push(page.kind as u8);
    output.push(page.kind.column());
    output.push(page.kind.buffer_kind());
    output.push(page.kind.logical_type());
    output.push(page.kind.physical_type());
    output.push(0); // plain Arrow-layout encoding
    output.push(page.compression as u8);
    output.push(0);
    output.extend_from_slice(&page.row_start.to_le_bytes());
    output.extend_from_slice(&page.row_count.to_le_bytes());
    output.extend_from_slice(&page.null_count.to_le_bytes());
    output.extend_from_slice(&page.offset.to_le_bytes());
    output.extend_from_slice(&(page.physical_bytes as u64).to_le_bytes());
    output.extend_from_slice(&(page.logical_bytes as u64).to_le_bytes());
    output.extend_from_slice(&page.statistic_min.to_le_bytes());
    output.extend_from_slice(&page.statistic_max.to_le_bytes());
    output.extend_from_slice(&page.digest);
}

fn decode_page_descriptor(
    bytes: &[u8],
    compression_policy: SegmentCompressionPolicy,
) -> Result<PageDescriptor> {
    let kind = PageKind::from_byte(bytes[0])?;
    if bytes[1] != kind.column()
        || bytes[2] != kind.buffer_kind()
        || bytes[3] != kind.logical_type()
        || bytes[4] != kind.physical_type()
        || bytes[5] != 0
        || bytes[7] != 0
    {
        return invalid("v6 page type, encoding, or reserved metadata is unknown");
    }
    let compression = PageCompression::from_byte(bytes[6])?;
    let physical_bytes = usize::try_from(read_u64(bytes, 32)?)
        .map_err(|_| Error::InvalidSegment("v6 physical page length exceeds usize".into()))?;
    let logical_bytes = usize::try_from(read_u64(bytes, 40)?)
        .map_err(|_| Error::InvalidSegment("v6 logical page length exceeds usize".into()))?;
    validate_page_compression(
        compression_policy,
        compression,
        u64::try_from(physical_bytes)
            .map_err(|_| Error::InvalidSegment("v6 physical page length exceeds u64".into()))?,
        u64::try_from(logical_bytes)
            .map_err(|_| Error::InvalidSegment("v6 logical page length exceeds u64".into()))?,
    )?;
    Ok(PageDescriptor {
        kind,
        compression,
        row_start: read_u64(bytes, 8)?,
        row_count: read_u32(bytes, 16)?,
        null_count: read_u32(bytes, 20)?,
        offset: read_u64(bytes, 24)?,
        physical_bytes,
        logical_bytes,
        statistic_min: read_u64(bytes, 48)?,
        statistic_max: read_u64(bytes, 56)?,
        digest: bytes[64..96].try_into().expect("fixed page digest"),
    })
}

fn compression_saves(logical: u64, stored: u64, minimum_savings: u16) -> Result<bool> {
    if stored >= logical {
        return Ok(false);
    }
    let stored_scaled = u128::from(stored)
        .checked_mul(u128::from(COMPRESSION_BASIS_POINTS))
        .ok_or_else(|| Error::InvalidSegment("v6 compression threshold overflow".into()))?;
    let retained_basis_points = COMPRESSION_BASIS_POINTS
        .checked_sub(minimum_savings)
        .ok_or_else(|| Error::InvalidSegment("v6 compression threshold is invalid".into()))?;
    let logical_scaled = u128::from(logical)
        .checked_mul(u128::from(retained_basis_points))
        .ok_or_else(|| Error::InvalidSegment("v6 compression threshold overflow".into()))?;
    Ok(stored_scaled <= logical_scaled)
}

fn validate_page_compression(
    policy: SegmentCompressionPolicy,
    compression: PageCompression,
    physical_bytes: u64,
    logical_bytes: u64,
) -> Result<()> {
    match (policy, compression) {
        (_, PageCompression::None) if physical_bytes == logical_bytes => Ok(()),
        (_, PageCompression::None) => {
            invalid("v6 raw page has different physical and logical lengths")
        }
        (SegmentCompressionPolicy::None, PageCompression::Lz4Block) => {
            invalid("v6 none policy cannot contain an LZ4 page")
        }
        (
            SegmentCompressionPolicy::AdaptiveLz4 {
                minimum_savings_basis_points,
                maximum_page_logical_bytes,
            },
            PageCompression::Lz4Block,
        ) if physical_bytes > 0
            && logical_bytes <= maximum_page_logical_bytes
            && compression_saves(logical_bytes, physical_bytes, minimum_savings_basis_points)? =>
        {
            Ok(())
        }
        (SegmentCompressionPolicy::AdaptiveLz4 { .. }, PageCompression::Lz4Block) => {
            invalid("v6 LZ4 page violates its authenticated lengths or saving policy")
        }
    }
}

fn validate_page_shapes(pages: &[PageDescriptor], rows: u32) -> Result<()> {
    let rows = rows as usize;
    let expected_offsets = rows
        .checked_add(1)
        .and_then(|count| count.checked_mul(8))
        .ok_or_else(|| Error::InvalidSegment("v6 offset page length overflow".into()))?;
    if pages[0].logical_bytes != expected_offsets
        || pages[2].logical_bytes != rows.saturating_mul(8)
        || pages[3].logical_bytes != rows.div_ceil(8)
        || pages[4].logical_bytes != expected_offsets
        || pages.iter().any(|page| page.null_count > page.row_count)
        || pages[0].null_count != 0
        || pages[1].null_count != 0
        || pages[2].null_count != 0
        || pages[3].null_count != pages[4].null_count
        || pages[3].null_count != pages[5].null_count
    {
        return invalid("v6 Arrow-compatible page lengths or null counts are invalid");
    }
    Ok(())
}

fn validate_zero_padding(
    reader: &mut (impl Read + Seek),
    row_groups: &[RowGroupDescriptor],
    index_offset: u64,
) -> Result<u64> {
    let mut expected = SEGMENT_HEADER_BYTES as u64;
    let mut bytes_read = 0u64;
    for page in row_groups.iter().flat_map(|group| group.pages.iter()) {
        bytes_read = bytes_read
            .checked_add(validate_zero_range(reader, expected, page.offset)?)
            .ok_or_else(|| Error::InvalidSegment("v6 padding read count overflow".into()))?;
        expected = page
            .offset
            .checked_add(page.physical_bytes as u64)
            .ok_or_else(|| Error::InvalidSegment("v6 page end overflow".into()))?;
    }
    bytes_read
        .checked_add(validate_zero_range(reader, expected, index_offset)?)
        .ok_or_else(|| Error::InvalidSegment("v6 padding read count overflow".into()))
}

fn validate_zero_range(reader: &mut (impl Read + Seek), start: u64, end: u64) -> Result<u64> {
    let length = usize::try_from(
        end.checked_sub(start)
            .ok_or_else(|| Error::InvalidSegment("v6 padding range is inverted".into()))?,
    )
    .map_err(|_| Error::InvalidSegment("v6 padding range exceeds usize".into()))?;
    if length >= PAGE_ALIGNMENT {
        return invalid("v6 page padding exceeds the alignment contract");
    }
    if length == 0 {
        return Ok(0);
    }
    let mut padding = [0; PAGE_ALIGNMENT - 1];
    reader.seek(SeekFrom::Start(start))?;
    reader.read_exact(&mut padding[..length])?;
    if padding[..length].iter().any(|byte| *byte != 0) {
        return invalid("v6 page alignment padding is non-zero");
    }
    Ok(length as u64)
}

fn validate_key_value(key: &[u8], value: &[u8]) -> Result<()> {
    if key.is_empty() || key.len() > MAX_KEY_BYTES || value.len() > MAX_VALUE_BYTES {
        return invalid("key or value exceeds the v6 segment contract");
    }
    Ok(())
}

fn encode_i64_values(values: &[i64]) -> Vec<u8> {
    let mut output = Vec::with_capacity(values.len() * 8);
    for value in values {
        output.extend_from_slice(&value.to_le_bytes());
    }
    output
}

fn encode_u64_values(values: &[u64]) -> Vec<u8> {
    let mut output = Vec::with_capacity(values.len() * 8);
    for value in values {
        output.extend_from_slice(&value.to_le_bytes());
    }
    output
}

fn align(output: &mut Vec<u8>) {
    let aligned = output.len().next_multiple_of(PAGE_ALIGNMENT);
    output.resize(aligned, 0);
}

fn slice<'a>(bytes: &'a [u8], offset: usize, length: usize, name: &str) -> Result<&'a [u8]> {
    let end = offset
        .checked_add(length)
        .ok_or_else(|| Error::InvalidSegment(format!("{name} range overflow")))?;
    bytes
        .get(offset..end)
        .ok_or_else(|| Error::InvalidSegment(format!("truncated {name}")))
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16> {
    Ok(u16::from_le_bytes(
        slice(bytes, offset, 2, "u16 field")?.try_into().unwrap(),
    ))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(
        slice(bytes, offset, 4, "u32 field")?.try_into().unwrap(),
    ))
}

fn read_u64(bytes: &[u8], offset: usize) -> Result<u64> {
    Ok(u64::from_le_bytes(
        slice(bytes, offset, 8, "u64 field")?.try_into().unwrap(),
    ))
}

fn sha256(bytes: &[u8]) -> [u8; 32] {
    ring::digest::digest(&ring::digest::SHA256, bytes)
        .as_ref()
        .try_into()
        .expect("SHA-256 output is 32 bytes")
}

pub(super) fn sha256_hex(bytes: &[u8]) -> String {
    encode_digest(&sha256(bytes))
}

fn encode_digest(digest: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = Vec::with_capacity(digest.len() * 2);
    for byte in digest {
        encoded.push(HEX[(byte >> 4) as usize]);
        encoded.push(HEX[(byte & 0x0f) as usize]);
    }
    String::from_utf8(encoded).expect("hex digest is ASCII")
}

fn decode_fixed_digest(encoded: &str) -> [u8; 32] {
    let mut digest = [0; 32];
    let (pairs, remainder) = encoded.as_bytes().as_chunks::<2>();
    debug_assert!(remainder.is_empty());
    for (index, pair) in pairs.iter().enumerate() {
        digest[index] = (hex_nibble(pair[0]) << 4) | hex_nibble(pair[1]);
    }
    digest
}

fn hex_nibble(byte: u8) -> u8 {
    match byte {
        b'0'..=b'9' => byte - b'0',
        b'a'..=b'f' => byte - b'a' + 10,
        _ => unreachable!("fixed RRFlow digest is lowercase hexadecimal"),
    }
}

fn invalid<T>(reason: impl Into<String>) -> Result<T> {
    Err(Error::InvalidSegment(reason.into()))
}
