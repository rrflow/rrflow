//! Segment format constants, compression policy, page and row-group descriptors, filters and the row-group builder.

use super::*;

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
pub(in super::super) enum PageCompression {
    None = 0,
    Lz4Block = 1,
}

impl PageCompression {
    pub(super) fn from_byte(value: u8) -> Result<Self> {
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
pub(in super::super) enum PageKind {
    KeyOffsets = 1,
    KeyData = 2,
    SequenceValues = 3,
    ValueValidity = 4,
    ValueOffsets = 5,
    ValueData = 6,
}

impl PageKind {
    pub(in super::super) const ORDERED: [Self; PAGES_PER_ROW_GROUP] = [
        Self::KeyOffsets,
        Self::KeyData,
        Self::SequenceValues,
        Self::ValueValidity,
        Self::ValueOffsets,
        Self::ValueData,
    ];

    pub(super) fn from_byte(value: u8) -> Result<Self> {
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

    pub(super) fn column(self) -> u8 {
        match self {
            Self::KeyOffsets | Self::KeyData => 1,
            Self::SequenceValues => 2,
            Self::ValueValidity | Self::ValueOffsets | Self::ValueData => 3,
        }
    }

    pub(super) fn buffer_kind(self) -> u8 {
        match self {
            Self::KeyOffsets | Self::ValueOffsets => 2,
            Self::KeyData | Self::SequenceValues | Self::ValueData => 3,
            Self::ValueValidity => 1,
        }
    }

    pub(super) fn logical_type(self) -> u8 {
        match self {
            Self::SequenceValues => 2,
            _ => 1,
        }
    }

    pub(super) fn physical_type(self) -> u8 {
        match self {
            Self::KeyOffsets | Self::ValueOffsets => 2,
            Self::SequenceValues => 3,
            Self::KeyData | Self::ValueData => 4,
            Self::ValueValidity => 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in super::super) struct PageDescriptor {
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
pub(super) struct PageStatistics {
    pub(super) null_count: u32,
    pub(super) minimum: u64,
    pub(super) maximum: u64,
}

impl PageStatistics {
    pub(super) const fn new(null_count: u32, minimum: u64, maximum: u64) -> Self {
        Self {
            null_count,
            minimum,
            maximum,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct PageWriteMetadata {
    pub(super) kind: PageKind,
    pub(super) row_start: u64,
    pub(super) row_count: u32,
    pub(super) statistics: PageStatistics,
}

impl PageWriteMetadata {
    pub(super) const fn new(
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
pub(in super::super) struct RowGroupDescriptor {
    pub row_start: u64,
    pub row_count: u32,
    pub unique_key_count: u32,
    pub first_key: Vec<u8>,
    pub last_key: Vec<u8>,
    pub pages: [PageDescriptor; PAGES_PER_ROW_GROUP],
    pub filter: RowGroupFilter,
}

impl RowGroupDescriptor {
    pub(in super::super) fn page(&self, kind: PageKind) -> &PageDescriptor {
        &self.pages[kind as usize - 1]
    }
}

/// Canonical RRFlow Bloom filter stored in each authenticated row-group index
/// entry. A negative answer can skip a point read; a positive answer always
/// falls through to the exact sorted key/version spine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in super::super) struct RowGroupFilter {
    words: Vec<u64>,
}

impl RowGroupFilter {
    pub(in super::super) fn new(unique_key_count: u32) -> Result<Self> {
        let word_count = Self::word_count(unique_key_count)?;
        Ok(Self {
            words: vec![0; word_count],
        })
    }

    pub(in super::super) fn from_words(unique_key_count: u32, words: Vec<u64>) -> Result<Self> {
        let expected = Self::word_count(unique_key_count)?;
        if words.len() != expected {
            return invalid("v6 row-group filter word count is noncanonical");
        }
        Ok(Self { words })
    }

    pub(in super::super) fn word_count(unique_key_count: u32) -> Result<usize> {
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

    pub(in super::super) fn insert(&mut self, key: &[u8]) {
        for bit in self.positions(key) {
            self.words[bit / u64::BITS as usize] |= 1u64 << (bit % u64::BITS as usize);
        }
    }

    pub(in super::super) fn may_contain(&self, key: &[u8]) -> bool {
        self.positions(key).into_iter().all(|bit| {
            self.words[bit / u64::BITS as usize] & (1u64 << (bit % u64::BITS as usize)) != 0
        })
    }

    pub(in super::super) fn byte_len(&self) -> usize {
        self.words.len() * std::mem::size_of::<u64>()
    }

    pub(in super::super) fn words(&self) -> &[u64] {
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

pub(super) fn filter_hash(key: &[u8], seed: u64) -> u64 {
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

pub(in super::super) struct EncodedSegment {
    pub bytes: Vec<u8>,
    pub checksum: String,
    pub evidence: SegmentWriteEvidence,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(in super::super) struct SegmentWriteEvidence {
    pub raw_page_count: u64,
    pub compressed_page_count: u64,
    pub stored_page_bytes: u64,
    pub logical_page_bytes: u64,
    pub compression_scratch_high_water_bytes: u64,
}

pub(in super::super) struct ParsedMetadata {
    pub descriptor: SegmentDescriptor,
    pub row_group_budget: SegmentRowGroupBudget,
    pub compression_policy: SegmentCompressionPolicy,
    pub row_groups: Vec<RowGroupDescriptor>,
    pub header_bytes_read: u64,
    pub index_bytes_read: u64,
    pub padding_bytes_read: u64,
}

#[derive(Default)]
pub(super) struct RowGroupBuilder {
    pub(super) row_start: u64,
    pub(super) keys: Vec<u8>,
    pub(super) key_offsets: Vec<i64>,
    pub(super) sequences: Vec<u64>,
    pub(super) value_validity: Vec<u8>,
    pub(super) values: Vec<u8>,
    pub(super) value_offsets: Vec<i64>,
    pub(super) first_key: Vec<u8>,
    pub(super) last_key: Vec<u8>,
    pub(super) unique_key_count: u32,
    pub(super) null_count: u32,
    pub(super) minimum_key_bytes: u64,
    pub(super) maximum_key_bytes: u64,
    pub(super) minimum_value_bytes: u64,
    pub(super) maximum_value_bytes: u64,
}

impl RowGroupBuilder {
    pub(super) fn new(row_start: u64) -> Self {
        Self {
            row_start,
            key_offsets: vec![0],
            value_offsets: vec![0],
            minimum_key_bytes: u64::MAX,
            minimum_value_bytes: u64::MAX,
            ..Self::default()
        }
    }

    pub(super) fn reset(&mut self, row_start: u64) {
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

    pub(super) fn rows(&self) -> usize {
        self.sequences.len()
    }

    pub(super) fn estimated_bytes(&self) -> usize {
        self.keys
            .len()
            .saturating_add(self.values.len())
            .saturating_add(self.key_offsets.len().saturating_mul(8))
            .saturating_add(self.value_offsets.len().saturating_mul(8))
            .saturating_add(self.sequences.len().saturating_mul(8))
            .saturating_add(self.sequences.len().div_ceil(8))
    }

    pub(super) fn append(&mut self, key: &[u8], version: &VersionedValue) -> Result<()> {
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

    pub(super) fn filter(&self) -> Result<RowGroupFilter> {
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
