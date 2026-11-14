//! Ordered physical-key grammar for rrflowKV.
//!
//! Every key begins with one typed address:
//!
//! ```text
//! application format / optional tenant / optional scope / family / tuple...
//! ```
//!
//! Variable-width tuple fields use a memcomparable, self-delimiting encoding:
//! eight data bytes followed by a padding marker. This preserves bytewise
//! lexical order, admits every byte value without delimiter escaping, and
//! rejects truncated or non-canonical encodings. Fixed-width numeric fields use
//! big-endian order; descending `u64` fields invert every bit so newer versions
//! sort first.
//!
//! This module owns physical bytes. Kernel types remain semantic and neither
//! `rrd-core` nor a client may construct rrflowKV keys.

use std::fmt;

/// Manifest identity for the first typed rrflowKV key format (`RRKV0001`).
pub(crate) const APPLICATION_FORMAT: u64 = u64::from_be_bytes(*b"RRKV0001");

const TAG_ABSENT: u8 = 0x00;

const TAG_BYTES: u8 = 0x20;

const TAG_TEXT: u8 = 0x21;

const TAG_BOOL: u8 = 0x30;

const TAG_U8: u8 = 0x31;

const TAG_U64: u8 = 0x32;

const TAG_I64: u8 = 0x33;

const TAG_DESC_U64: u8 = 0x34;

const TAG_FORMAT: u8 = 0xf0;

const TAG_FAMILY: u8 = 0xf1;

const MEMCOMPARABLE_GROUP_BYTES: usize = 8;

const MEMCOMPARABLE_FULL_GROUP: u8 = 0xff;

/// Stable top-level physical families. Numeric values are wire bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub(crate) enum KeyFamily {
    Current = 0x10,
    Temporal = 0x11,
    OutgoingEdge = 0x12,
    IncomingEdge = 0x13,
    Scalar = 0x14,
    Unique = 0x15,
    TermDictionary = 0x16,
    TermStatistic = 0x17,
    TermPosting = 0x18,
    Vector = 0x19,
    ProjectionDelta = 0x1a,
    Catalogue = 0x1b,
    RuntimeCommit = 0x1c,
    Outbox = 0x1d,
    Audit = 0x1e,
    EngineEvent = 0x1f,
    System = 0x20,
}

impl TryFrom<u8> for KeyFamily {
    type Error = KeyCodecError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x10 => Ok(Self::Current),
            0x11 => Ok(Self::Temporal),
            0x12 => Ok(Self::OutgoingEdge),
            0x13 => Ok(Self::IncomingEdge),
            0x14 => Ok(Self::Scalar),
            0x15 => Ok(Self::Unique),
            0x16 => Ok(Self::TermDictionary),
            0x17 => Ok(Self::TermStatistic),
            0x18 => Ok(Self::TermPosting),
            0x19 => Ok(Self::Vector),
            0x1a => Ok(Self::ProjectionDelta),
            0x1b => Ok(Self::Catalogue),
            0x1c => Ok(Self::RuntimeCommit),
            0x1d => Ok(Self::Outbox),
            0x1e => Ok(Self::Audit),
            0x1f => Ok(Self::EngineEvent),
            0x20 => Ok(Self::System),
            _ => Err(KeyCodecError::new("unknown rrflowKV key family")),
        }
    }
}

/// Stable catalogue subdivisions. Executable bytes are values addressed by a
/// content digest; a membership head never repeats those bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum CatalogueSubfamily {
    FunctionArtifact = 0x01,
    FunctionDefinition = 0x02,
    TransactionFunctionBinding = 0x03,
    Membership = 0x04,
    Head = 0x05,
    InvocationReceipt = 0x06,
}

impl TryFrom<u8> for CatalogueSubfamily {
    type Error = KeyCodecError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x01 => Ok(Self::FunctionArtifact),
            0x02 => Ok(Self::FunctionDefinition),
            0x03 => Ok(Self::TransactionFunctionBinding),
            0x04 => Ok(Self::Membership),
            0x05 => Ok(Self::Head),
            0x06 => Ok(Self::InvocationReceipt),
            _ => Err(KeyCodecError::new("unknown rrflowKV catalogue subfamily")),
        }
    }
}

/// Tenant and scope coordinates preceding every family.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct KeyAddress<'a> {
    pub(crate) tenant: Option<&'a str>,
    pub(crate) scope: Option<&'a str>,
}

impl KeyAddress<'static> {
    pub(crate) const GLOBAL: Self = Self {
        tenant: None,
        scope: None,
    };
}

/// One typed tuple field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KeyPart<'a> {
    #[allow(
        dead_code,
        reason = "C-01 freezes opaque index and artifact bytes before their E-gate consumers"
    )]
    Bytes(&'a [u8]),
    Text(&'a str),
    #[allow(
        dead_code,
        reason = "C-01 freezes boolean index fields before their E-gate consumers"
    )]
    Bool(bool),
    U8(u8),
    U64(u64),
    #[allow(
        dead_code,
        reason = "C-01 freezes ordered signed fields before their E-gate consumers"
    )]
    I64(i64),
    DescU64(u64),
}

/// Owned form returned by strict decoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DecodedKeyPart {
    Bytes(Vec<u8>),
    Text(String),
    Bool(bool),
    U8(u8),
    U64(u64),
    I64(i64),
    DescU64(u64),
}

/// Strictly decoded physical key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DecodedKey {
    pub(crate) tenant: Option<String>,
    pub(crate) scope: Option<String>,
    pub(crate) family: KeyFamily,
    pub(crate) parts: Vec<DecodedKeyPart>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct KeyCodec;

impl KeyCodec {
    pub(crate) fn from_application_format(format: Option<u64>) -> Option<Self> {
        (format == Some(APPLICATION_FORMAT)).then_some(Self)
    }

    /// Encodes a complete key or a component-aligned range prefix.
    pub(crate) fn encode(
        self,
        address: KeyAddress<'_>,
        family: KeyFamily,
        parts: &[KeyPart<'_>],
    ) -> Vec<u8> {
        let mut encoded = Vec::new();
        encoded.push(TAG_FORMAT);
        encoded.extend_from_slice(&APPLICATION_FORMAT.to_be_bytes());
        encode_optional_text(&mut encoded, address.tenant);
        encode_optional_text(&mut encoded, address.scope);
        encoded.push(TAG_FAMILY);
        encoded.push(family as u8);
        for part in parts {
            encode_part(&mut encoded, *part);
        }
        encoded
    }

    /// Decodes an entire key and rejects unknown tags, malformed lengths,
    /// non-canonical padding, invalid UTF-8, and trailing partial fields.
    pub(crate) fn decode(self, encoded: &[u8]) -> Result<DecodedKey, KeyCodecError> {
        let mut reader = TupleReader::new(encoded);
        reader.expect_tag(TAG_FORMAT, "rrflowKV key lacks its format field")?;
        let format = reader.read_u64_payload("rrflowKV key has a truncated format field")?;
        if format != APPLICATION_FORMAT {
            return Err(KeyCodecError::new("unsupported rrflowKV key format"));
        }
        let tenant = reader.read_optional_text("tenant")?;
        let scope = reader.read_optional_text("scope")?;
        reader.expect_tag(TAG_FAMILY, "rrflowKV key lacks its family field")?;
        let family = KeyFamily::try_from(reader.read_byte("rrflowKV key lacks its family byte")?)?;
        let mut parts = Vec::new();
        while !reader.is_empty() {
            parts.push(reader.read_part()?);
        }
        Ok(DecodedKey {
            tenant,
            scope,
            family,
            parts,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KeyCodecError {
    reason: &'static str,
}

impl KeyCodecError {
    const fn new(reason: &'static str) -> Self {
        Self { reason }
    }
}

impl fmt::Display for KeyCodecError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.reason)
    }
}

impl std::error::Error for KeyCodecError {}

/// Exclusive upper bound for a byte prefix.
pub(crate) fn prefix_end(prefix: &[u8]) -> Option<Vec<u8>> {
    let mut end = prefix.to_vec();
    while let Some(last) = end.last_mut() {
        if *last != u8::MAX {
            *last += 1;
            return Some(end);
        }
        end.pop();
    }
    None
}

fn encode_optional_text(out: &mut Vec<u8>, value: Option<&str>) {
    match value {
        None => out.push(TAG_ABSENT),
        Some(value) => {
            out.push(TAG_TEXT);
            encode_memcomparable(out, value.as_bytes());
        }
    }
}

fn encode_part(out: &mut Vec<u8>, part: KeyPart<'_>) {
    match part {
        KeyPart::Bytes(value) => {
            out.push(TAG_BYTES);
            encode_memcomparable(out, value);
        }
        KeyPart::Text(value) => {
            out.push(TAG_TEXT);
            encode_memcomparable(out, value.as_bytes());
        }
        KeyPart::Bool(value) => {
            out.push(TAG_BOOL);
            out.push(u8::from(value));
        }
        KeyPart::U8(value) => {
            out.push(TAG_U8);
            out.push(value);
        }
        KeyPart::U64(value) => {
            out.push(TAG_U64);
            out.extend_from_slice(&value.to_be_bytes());
        }
        KeyPart::I64(value) => {
            out.push(TAG_I64);
            out.extend_from_slice(&((value as u64) ^ (1_u64 << 63)).to_be_bytes());
        }
        KeyPart::DescU64(value) => {
            out.push(TAG_DESC_U64);
            out.extend_from_slice(&(!value).to_be_bytes());
        }
    }
}

fn encode_memcomparable(out: &mut Vec<u8>, value: &[u8]) {
    let mut offset = 0;
    loop {
        let remaining = value.len().saturating_sub(offset);
        let take = remaining.min(MEMCOMPARABLE_GROUP_BYTES);
        out.extend_from_slice(&value[offset..offset + take]);
        let padding = MEMCOMPARABLE_GROUP_BYTES - take;
        out.resize(out.len() + padding, 0);
        out.push(MEMCOMPARABLE_FULL_GROUP - padding as u8);
        offset += take;
        if padding != 0 {
            break;
        }
    }
}

struct TupleReader<'a> {
    remaining: &'a [u8],
}

impl<'a> TupleReader<'a> {
    fn new(encoded: &'a [u8]) -> Self {
        Self { remaining: encoded }
    }

    fn is_empty(&self) -> bool {
        self.remaining.is_empty()
    }

    fn read_byte(&mut self, reason: &'static str) -> Result<u8, KeyCodecError> {
        let (byte, remaining) = self
            .remaining
            .split_first()
            .ok_or_else(|| KeyCodecError::new(reason))?;
        self.remaining = remaining;
        Ok(*byte)
    }

    fn expect_tag(&mut self, tag: u8, reason: &'static str) -> Result<(), KeyCodecError> {
        if self.read_byte(reason)? != tag {
            return Err(KeyCodecError::new(reason));
        }
        Ok(())
    }

    fn read_u64_payload(&mut self, reason: &'static str) -> Result<u64, KeyCodecError> {
        if self.remaining.len() < 8 {
            return Err(KeyCodecError::new(reason));
        }
        let (value, remaining) = self.remaining.split_at(8);
        self.remaining = remaining;
        Ok(u64::from_be_bytes(
            value.try_into().expect("eight-byte slice"),
        ))
    }

    fn read_optional_text(&mut self, field: &'static str) -> Result<Option<String>, KeyCodecError> {
        match self.read_byte("rrflowKV key lacks an address component")? {
            TAG_ABSENT => Ok(None),
            TAG_TEXT => {
                let value = self.read_memcomparable()?;
                String::from_utf8(value).map(Some).map_err(|_| match field {
                    "tenant" => KeyCodecError::new("rrflowKV tenant is not UTF-8"),
                    _ => KeyCodecError::new("rrflowKV scope is not UTF-8"),
                })
            }
            _ => Err(KeyCodecError::new(
                "rrflowKV address component has the wrong type",
            )),
        }
    }

    fn read_part(&mut self) -> Result<DecodedKeyPart, KeyCodecError> {
        match self.read_byte("rrflowKV tuple has a partial field")? {
            TAG_BYTES => Ok(DecodedKeyPart::Bytes(self.read_memcomparable()?)),
            TAG_TEXT => String::from_utf8(self.read_memcomparable()?)
                .map(DecodedKeyPart::Text)
                .map_err(|_| KeyCodecError::new("rrflowKV text field is not UTF-8")),
            TAG_BOOL => match self.read_byte("rrflowKV bool field is truncated")? {
                0 => Ok(DecodedKeyPart::Bool(false)),
                1 => Ok(DecodedKeyPart::Bool(true)),
                _ => Err(KeyCodecError::new("rrflowKV bool field is non-canonical")),
            },
            TAG_U8 => Ok(DecodedKeyPart::U8(
                self.read_byte("rrflowKV u8 field is truncated")?,
            )),
            TAG_U64 => Ok(DecodedKeyPart::U64(
                self.read_u64_payload("rrflowKV u64 field is truncated")?,
            )),
            TAG_I64 => Ok(DecodedKeyPart::I64(
                (self.read_u64_payload("rrflowKV i64 field is truncated")? ^ (1_u64 << 63)) as i64,
            )),
            TAG_DESC_U64 => Ok(DecodedKeyPart::DescU64(
                !self.read_u64_payload("rrflowKV descending u64 field is truncated")?,
            )),
            _ => Err(KeyCodecError::new("unknown rrflowKV tuple field type")),
        }
    }

    fn read_memcomparable(&mut self) -> Result<Vec<u8>, KeyCodecError> {
        let mut decoded = Vec::new();
        loop {
            if self.remaining.len() < MEMCOMPARABLE_GROUP_BYTES + 1 {
                return Err(KeyCodecError::new(
                    "rrflowKV variable-width field is truncated",
                ));
            }
            let (group, rest) = self.remaining.split_at(MEMCOMPARABLE_GROUP_BYTES);
            let (&marker, remaining) = rest
                .split_first()
                .expect("length checked before splitting marker");
            if marker < MEMCOMPARABLE_FULL_GROUP - MEMCOMPARABLE_GROUP_BYTES as u8 {
                return Err(KeyCodecError::new(
                    "rrflowKV variable-width field has an invalid length marker",
                ));
            }
            let padding = (MEMCOMPARABLE_FULL_GROUP - marker) as usize;
            if padding > MEMCOMPARABLE_GROUP_BYTES
                || (padding != 0
                    && group[MEMCOMPARABLE_GROUP_BYTES - padding..]
                        .iter()
                        .any(|byte| *byte != 0))
            {
                return Err(KeyCodecError::new(
                    "rrflowKV variable-width field has non-canonical padding",
                ));
            }
            decoded.extend_from_slice(&group[..MEMCOMPARABLE_GROUP_BYTES - padding]);
            self.remaining = remaining;
            if padding != 0 {
                return Ok(decoded);
            }
        }
    }
}

#[cfg(test)]
mod tests;
