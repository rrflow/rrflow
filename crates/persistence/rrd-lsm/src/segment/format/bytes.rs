//! Integer, digest and slice helpers.

use super::*;

pub(super) fn encode_i64_values(values: &[i64]) -> Vec<u8> {
    let mut output = Vec::with_capacity(values.len() * 8);
    for value in values {
        output.extend_from_slice(&value.to_le_bytes());
    }
    output
}

pub(super) fn encode_u64_values(values: &[u64]) -> Vec<u8> {
    let mut output = Vec::with_capacity(values.len() * 8);
    for value in values {
        output.extend_from_slice(&value.to_le_bytes());
    }
    output
}

pub(super) fn align(output: &mut Vec<u8>) {
    let aligned = output.len().next_multiple_of(PAGE_ALIGNMENT);
    output.resize(aligned, 0);
}

pub(super) fn slice<'a>(
    bytes: &'a [u8],
    offset: usize,
    length: usize,
    name: &str,
) -> Result<&'a [u8]> {
    let end = offset
        .checked_add(length)
        .ok_or_else(|| Error::InvalidSegment(format!("{name} range overflow")))?;
    bytes
        .get(offset..end)
        .ok_or_else(|| Error::InvalidSegment(format!("truncated {name}")))
}

pub(super) fn read_u16(bytes: &[u8], offset: usize) -> Result<u16> {
    Ok(u16::from_le_bytes(
        slice(bytes, offset, 2, "u16 field")?.try_into().unwrap(),
    ))
}

pub(super) fn read_u32(bytes: &[u8], offset: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(
        slice(bytes, offset, 4, "u32 field")?.try_into().unwrap(),
    ))
}

pub(super) fn read_u64(bytes: &[u8], offset: usize) -> Result<u64> {
    Ok(u64::from_le_bytes(
        slice(bytes, offset, 8, "u64 field")?.try_into().unwrap(),
    ))
}

pub(super) fn sha256(bytes: &[u8]) -> [u8; 32] {
    ring::digest::digest(&ring::digest::SHA256, bytes)
        .as_ref()
        .try_into()
        .expect("SHA-256 output is 32 bytes")
}

pub(in super::super) fn sha256_hex(bytes: &[u8]) -> String {
    encode_digest(&sha256(bytes))
}

pub(super) fn encode_digest(digest: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = Vec::with_capacity(digest.len() * 2);
    for byte in digest {
        encoded.push(HEX[(byte >> 4) as usize]);
        encoded.push(HEX[(byte & 0x0f) as usize]);
    }
    String::from_utf8(encoded).expect("hex digest is ASCII")
}

pub(super) fn decode_fixed_digest(encoded: &str) -> [u8; 32] {
    let mut digest = [0; 32];
    let (pairs, remainder) = encoded.as_bytes().as_chunks::<2>();
    debug_assert!(remainder.is_empty());
    for (index, pair) in pairs.iter().enumerate() {
        digest[index] = (hex_nibble(pair[0]) << 4) | hex_nibble(pair[1]);
    }
    digest
}

pub(super) fn hex_nibble(byte: u8) -> u8 {
    match byte {
        b'0'..=b'9' => byte - b'0',
        b'a'..=b'f' => byte - b'a' + 10,
        _ => unreachable!("fixed RRFlow digest is lowercase hexadecimal"),
    }
}

pub(super) fn invalid<T>(reason: impl Into<String>) -> Result<T> {
    Err(Error::InvalidSegment(reason.into()))
}
