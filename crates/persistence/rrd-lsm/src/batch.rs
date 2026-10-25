use crate::{Error, Result, WAL_MAX_PAYLOAD_BYTES};
use serde::{Deserialize, Deserializer, Serialize};

pub const BATCH_FORMAT_VERSION: u16 = 2;
const BATCH_MAGIC: &[u8; 8] = b"RRDBAT02";
const BATCH_HEADER_BYTES: usize = 16;
const OP_HEADER_BYTES: usize = 8;
const DELETE_MASK: u32 = 1 << 31;
const VALUE_LEN_MASK: u32 = !DELETE_MASK;
const MAX_OPERATIONS: usize = 1_000_000;
const MAX_KEY_BYTES: usize = 1024 * 1024;
const MAX_VALUE_BYTES: usize = 8 * 1024 * 1024;

/// Existing physical batch-format ceilings, not process-memory budgets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteBatchResource {
    EncodedBytes,
    Operations,
}

impl WriteBatchResource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::EncodedBytes => "encoded_bytes",
            Self::Operations => "operations",
        }
    }
}

/// Checked current-format accounting for one staged physical batch.
///
/// The caller owns the map and supplies its actual prior value. A successful
/// replacement returns a new charge; a rejection leaves both owners unchanged.
/// This measures encoded bytes, including tombstones, not heap or allocator use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WriteBatchSize {
    operations: usize,
    encoded_bytes: usize,
}

impl Default for WriteBatchSize {
    fn default() -> Self {
        Self {
            operations: 0,
            encoded_bytes: BATCH_HEADER_BYTES,
        }
    }
}

impl WriteBatchSize {
    pub fn encoded_len(self) -> usize {
        self.encoded_bytes
    }

    pub fn with_replacement(
        self,
        key: &[u8],
        previous: Option<Option<&[u8]>>,
        value: Option<&[u8]>,
    ) -> Result<Self> {
        validate_key(key)?;
        if let Some(value) = value {
            validate_value(value)?;
        }
        let operations = if previous.is_none() {
            self.operations
                .checked_add(1)
                .ok_or_else(|| Error::InvalidBatch("operation count overflow".into()))?
        } else {
            if self.operations == 0 {
                return invalid("replacement has no charged operation");
            }
            self.operations
        };
        if operations > MAX_OPERATIONS {
            return Err(Error::WriteBatchLimit {
                resource: WriteBatchResource::Operations,
                limit: MAX_OPERATIONS as u64,
                observed: operations as u64,
            });
        }
        let length = |value: Option<&[u8]>| {
            OP_HEADER_BYTES
                .checked_add(key.len())
                .and_then(|n| n.checked_add(value.map_or(0, <[u8]>::len)))
                .ok_or_else(|| Error::InvalidBatch("encoded operation length overflow".into()))
        };
        let old = previous.map(length).transpose()?.unwrap_or(0);
        let new = length(value)?;
        let encoded_bytes = self
            .encoded_bytes
            .checked_sub(old)
            .filter(|n| *n >= BATCH_HEADER_BYTES)
            .and_then(|n| n.checked_add(new))
            .ok_or_else(|| Error::InvalidBatch("encoded batch accounting overflow".into()))?;
        if encoded_bytes > WAL_MAX_PAYLOAD_BYTES {
            return Err(Error::WriteBatchLimit {
                resource: WriteBatchResource::EncodedBytes,
                limit: WAL_MAX_PAYLOAD_BYTES as u64,
                observed: encoded_bytes as u64,
            });
        }
        Ok(Self {
            operations,
            encoded_bytes,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case")]
pub enum Mutation {
    Put { key: Vec<u8>, value: Vec<u8> },
    Delete { key: Vec<u8> },
}

impl Mutation {
    pub fn key(&self) -> &[u8] {
        match self {
            Self::Put { key, .. } | Self::Delete { key } => key,
        }
    }

    /// Validates one physical mutation before it enters a transaction write
    /// set or atomic batch.
    pub fn validate(&self) -> Result<()> {
        validate_key(self.key())?;
        if let Self::Put { value, .. } = self {
            validate_value(value)?;
        }
        Ok(())
    }
}

pub(crate) fn validate_key(key: &[u8]) -> Result<()> {
    if key.is_empty() || key.len() > MAX_KEY_BYTES {
        return Err(Error::InvalidBatch(format!(
            "key length must be in 1..={MAX_KEY_BYTES} bytes"
        )));
    }
    Ok(())
}

pub(crate) fn validate_value(value: &[u8]) -> Result<()> {
    if value.len() > MAX_VALUE_BYTES {
        return Err(Error::InvalidBatch(format!(
            "value length exceeds {MAX_VALUE_BYTES} bytes"
        )));
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WriteBatch {
    pub(crate) operations: Vec<Mutation>,
    #[serde(skip)]
    encoded_len: usize,
}

impl WriteBatch {
    pub fn new(operations: Vec<Mutation>) -> Result<Self> {
        let encoded_len = validate_operations(&operations)?;
        Ok(Self {
            operations,
            encoded_len,
        })
    }

    pub fn len(&self) -> usize {
        self.operations.len()
    }

    pub fn is_empty(&self) -> bool {
        self.operations.is_empty()
    }

    pub fn operations(&self) -> &[Mutation] {
        &self.operations
    }

    /// Exact byte length of this batch in the current authenticated WAL
    /// payload encoding.
    ///
    /// Higher semantic layers use this only for pre-admission evidence. The
    /// WAL remains the physical authority and validates the encoded payload
    /// again when it is appended.
    pub fn encoded_len(&self) -> usize {
        self.encoded_len
    }

    /// Consumes a validated batch and returns its ordered mutations.
    pub fn into_operations(self) -> Vec<Mutation> {
        self.operations
    }

    pub fn encode(&self) -> Result<Vec<u8>> {
        let count = u32::try_from(self.operations.len())
            .map_err(|_| Error::InvalidBatch("operation count exceeds u32".into()))?;
        let mut output = Vec::with_capacity(self.encoded_len);
        output.extend_from_slice(BATCH_MAGIC);
        output.extend_from_slice(&BATCH_FORMAT_VERSION.to_be_bytes());
        output.extend_from_slice(&0u16.to_be_bytes());
        output.extend_from_slice(&count.to_be_bytes());
        for operation in &self.operations {
            let (key, value, tagged_value_len): (&[u8], &[u8], u32) = match operation {
                Mutation::Put { key, value } => (
                    key,
                    value,
                    u32::try_from(value.len())
                        .map_err(|_| Error::InvalidBatch("value length exceeds u32".into()))?,
                ),
                Mutation::Delete { key } => (key, &[], DELETE_MASK),
            };
            output.extend_from_slice(
                &u32::try_from(key.len())
                    .map_err(|_| Error::InvalidBatch("key length exceeds u32".into()))?
                    .to_be_bytes(),
            );
            output.extend_from_slice(&tagged_value_len.to_be_bytes());
            output.extend_from_slice(key);
            output.extend_from_slice(value);
        }
        debug_assert_eq!(output.len(), self.encoded_len);
        Ok(output)
    }

    pub fn decode(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < BATCH_HEADER_BYTES {
            return invalid("incomplete batch header");
        }
        if bytes.len() > WAL_MAX_PAYLOAD_BYTES {
            return invalid(format!(
                "encoded batch exceeds {WAL_MAX_PAYLOAD_BYTES} bytes"
            ));
        }
        let version = u16::from_be_bytes(bytes[8..10].try_into().expect("fixed version field"));
        if version != BATCH_FORMAT_VERSION {
            return Err(Error::UnsupportedVersion {
                object: "write batch",
                version,
            });
        }
        if &bytes[..8] != BATCH_MAGIC {
            return invalid("batch magic does not match its format version");
        }
        if bytes[10..12] != [0, 0] {
            return invalid("unknown batch flags");
        }
        let count =
            u32::from_be_bytes(bytes[12..16].try_into().expect("fixed count field")) as usize;
        if count == 0 || count > MAX_OPERATIONS {
            return invalid(format!("invalid operation count {count}"));
        }
        let mut cursor = BATCH_HEADER_BYTES;
        let mut operations = Vec::with_capacity(count);
        for _ in 0..count {
            let header_end = cursor
                .checked_add(OP_HEADER_BYTES)
                .ok_or_else(|| Error::InvalidBatch("operation header length overflow".into()))?;
            let header = bytes
                .get(cursor..header_end)
                .ok_or_else(|| Error::InvalidBatch("incomplete operation header".into()))?;
            let tagged_value_len =
                u32::from_be_bytes(header[4..8].try_into().expect("fixed tagged value length"));
            let delete = tagged_value_len & DELETE_MASK != 0;
            let value_len = (tagged_value_len & VALUE_LEN_MASK) as usize;
            if delete && value_len != 0 {
                return invalid("delete operation carries a value length");
            }
            let kind = if delete { 2 } else { 1 };
            let key_len =
                u32::from_be_bytes(header[0..4].try_into().expect("fixed key length")) as usize;
            if key_len == 0 || key_len > MAX_KEY_BYTES || value_len > MAX_VALUE_BYTES {
                return invalid("operation key/value length exceeds its contract");
            }
            cursor = header_end;
            let end = cursor
                .checked_add(key_len)
                .and_then(|value| value.checked_add(value_len))
                .ok_or_else(|| Error::InvalidBatch("operation length overflow".into()))?;
            let body = bytes
                .get(cursor..end)
                .ok_or_else(|| Error::InvalidBatch("incomplete operation body".into()))?;
            let key = body[..key_len].to_vec();
            let value = body[key_len..].to_vec();
            let operation = match kind {
                1 => Mutation::Put { key, value },
                2 if value.is_empty() => Mutation::Delete { key },
                2 => return invalid("delete operation carries a value"),
                _ => return invalid(format!("unknown operation kind {kind}")),
            };
            operations.push(operation);
            cursor = end;
        }
        if cursor != bytes.len() {
            return invalid(format!(
                "{} trailing byte(s) after the declared batch",
                bytes.len() - cursor
            ));
        }
        Self::new(operations)
    }
}

impl<'de> Deserialize<'de> for WriteBatch {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct SerializedWriteBatch {
            operations: Vec<Mutation>,
        }

        let serialized = SerializedWriteBatch::deserialize(deserializer)?;
        Self::new(serialized.operations).map_err(serde::de::Error::custom)
    }
}

fn validate_operations(operations: &[Mutation]) -> Result<usize> {
    if operations.is_empty() {
        return invalid("operation count must be greater than zero");
    }
    if operations.len() > MAX_OPERATIONS {
        return Err(Error::WriteBatchLimit {
            resource: WriteBatchResource::Operations,
            limit: MAX_OPERATIONS as u64,
            observed: operations.len() as u64,
        });
    }
    let mut size = WriteBatchSize::default();
    for operation in operations {
        let value = match operation {
            Mutation::Put { value, .. } => Some(value.as_slice()),
            Mutation::Delete { .. } => None,
        };
        // An encoded operation is appended even if an earlier key repeats.
        // Only staging-map owners coalesce replacements before this boundary.
        size = size.with_replacement(operation.key(), None, value)?;
    }
    Ok(size.encoded_len())
}

fn invalid<T>(reason: impl Into<String>) -> Result<T> {
    Err(Error::InvalidBatch(reason.into()))
}

#[cfg(test)]
mod admission_tests {
    use super::*;

    #[test]
    fn encoded_boundary_matches_real_bytes_and_preserves_charge_on_rejection() {
        let first = vec![1; MAX_VALUE_BYTES];
        let remaining = WAL_MAX_PAYLOAD_BYTES - first.len() - 34;
        let initial = WriteBatchSize::default()
            .with_replacement(b"a", None, Some(&first))
            .unwrap();
        for length in [remaining - 1, remaining] {
            let second = vec![2; length];
            let admitted = initial.with_replacement(b"b", None, Some(&second)).unwrap();
            let batch = WriteBatch::new(vec![
                Mutation::Put {
                    key: b"a".to_vec(),
                    value: first.clone(),
                },
                Mutation::Put {
                    key: b"b".to_vec(),
                    value: second,
                },
            ])
            .unwrap();
            assert_eq!(admitted.encoded_len(), 34 + first.len() + length);
            assert_eq!(admitted.encoded_len(), batch.encode().unwrap().len());
        }
        let oversized = vec![2; remaining + 1];
        assert!(
            matches!(initial.with_replacement(b"b", None, Some(&oversized)),
            Err(Error::WriteBatchLimit { resource: WriteBatchResource::EncodedBytes, limit, observed })
                if limit == WAL_MAX_PAYLOAD_BYTES as u64 && observed == limit + 1)
        );
        assert_eq!(initial.encoded_len(), 25 + first.len());
        assert!(matches!(
            WriteBatch::new(vec![
                Mutation::Put {
                    key: b"a".to_vec(),
                    value: first
                },
                Mutation::Put {
                    key: b"b".to_vec(),
                    value: oversized
                },
            ]),
            Err(Error::WriteBatchLimit {
                resource: WriteBatchResource::EncodedBytes,
                ..
            })
        ));
    }

    #[test]
    fn replacements_tombstones_and_duplicate_encoded_keys_have_exact_charges() {
        let mut size = WriteBatchSize::default();
        let mut previous: Option<Vec<u8>> = None;
        let mut exists = false;
        for value in [
            Some(vec![1; 100]),
            None,
            None,
            Some(vec![]),
            Some(vec![2; 200]),
            None,
        ] {
            size = size
                .with_replacement(
                    b"key",
                    exists.then_some(previous.as_deref()),
                    value.as_deref(),
                )
                .unwrap();
            let mutation = match &value {
                Some(value) => Mutation::Put {
                    key: b"key".to_vec(),
                    value: value.clone(),
                },
                None => Mutation::Delete {
                    key: b"key".to_vec(),
                },
            };
            let encoded = WriteBatch::new(vec![mutation]).unwrap().encode().unwrap();
            assert_eq!(size.encoded_len(), encoded.len());
            assert_eq!(size.operations, 1);
            previous = value;
            exists = true;
        }
        // The batch codec must not coalesce repeated keys: physical callers
        // can supply separate operations even though staging maps coalesce.
        let repeated = WriteBatch::new(vec![
            Mutation::Delete {
                key: b"key".to_vec()
            };
            2
        ])
        .unwrap();
        assert_eq!(repeated.encode().unwrap().len(), 16 + 2 * 11);
    }

    #[test]
    fn operation_cap_allows_replacement_but_denies_new_operation() {
        let mut size = WriteBatchSize::default();
        for _ in 0..MAX_OPERATIONS {
            size = size.with_replacement(b"a", None, None).unwrap();
        }
        assert_eq!(size.operations, MAX_OPERATIONS);
        assert_eq!(size.encoded_len(), 16 + MAX_OPERATIONS * 9);
        assert_eq!(size.with_replacement(b"a", Some(None), None).unwrap(), size);
        assert!(matches!(size.with_replacement(b"b", None, None),
            Err(Error::WriteBatchLimit { resource: WriteBatchResource::Operations, limit, observed })
                if limit == MAX_OPERATIONS as u64 && observed == limit + 1));
    }

    #[test]
    fn malformed_replacements_and_checked_overflow_fail_without_new_charge() {
        let empty = WriteBatchSize::default();
        assert!(matches!(
            empty.with_replacement(b"a", Some(None), None),
            Err(Error::InvalidBatch(_))
        ));
        assert!(matches!(
            empty.with_replacement(b"", None, None),
            Err(Error::InvalidBatch(_))
        ));
        let size = empty.with_replacement(b"a", None, Some(b"old")).unwrap();
        assert!(matches!(
            size.with_replacement(
                b"a",
                Some(Some(b"old")),
                Some(&vec![0; MAX_VALUE_BYTES + 1])
            ),
            Err(Error::InvalidBatch(_))
        ));
        assert_eq!(size.encoded_len(), 28);
        for broken in [
            WriteBatchSize {
                operations: usize::MAX,
                encoded_bytes: 16,
            },
            WriteBatchSize {
                operations: 0,
                encoded_bytes: usize::MAX,
            },
        ] {
            assert!(matches!(
                broken.with_replacement(b"a", None, None),
                Err(Error::InvalidBatch(_))
            ));
        }
        assert!(matches!(
            size.with_replacement(b"a", Some(Some(&[0; 100])), None),
            Err(Error::InvalidBatch(_))
        ));
    }
}
