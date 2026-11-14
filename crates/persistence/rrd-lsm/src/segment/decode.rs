//! Row-group column decoding and whole-file verification helpers.

use super::*;

pub(super) fn lower_bound(spine: &KeySpine, rows: usize, key: &[u8]) -> Result<usize> {
    let mut left = 0;
    let mut right = rows;
    while left < right {
        let middle = left + (right - left) / 2;
        if key_at(spine, middle)? < key {
            left = middle + 1;
        } else {
            right = middle;
        }
    }
    Ok(left)
}

pub(super) fn key_at(spine: &KeySpine, row: usize) -> Result<&[u8]> {
    binary_at(
        spine.offsets.buffer.as_slice(),
        spine.data.buffer.as_slice(),
        row,
    )
}

pub(super) fn sequence_at(spine: &KeySpine, row: usize) -> Result<u64> {
    let offset = row
        .checked_mul(8)
        .ok_or_else(|| Error::InvalidSegment("v6 sequence offset overflow".into()))?;
    let bytes = spine
        .sequences
        .buffer
        .as_slice()
        .get(offset..offset + 8)
        .ok_or_else(|| Error::InvalidSegment("v6 sequence row is absent".into()))?;
    Ok(u64::from_le_bytes(bytes.try_into().unwrap()))
}

pub(super) fn value_from_loaded(group: &LoadedRowGroup, row: usize) -> Result<Option<&[u8]>> {
    if !valid_at(group.validity.buffer.as_slice(), row)? {
        return Ok(None);
    }
    Ok(Some(binary_at(
        group.value_offsets.buffer.as_slice(),
        group.values.buffer.as_slice(),
        row,
    )?))
}

pub(super) fn value_from_columns(
    columns: &LoadedValueColumns,
    row: usize,
) -> Result<Option<&[u8]>> {
    if !valid_at(columns.validity.buffer.as_slice(), row)? {
        return Ok(None);
    }
    Ok(Some(binary_at(
        columns.offsets.buffer.as_slice(),
        columns.values.buffer.as_slice(),
        row,
    )?))
}

pub(super) fn valid_at(validity: &[u8], row: usize) -> Result<bool> {
    let byte = validity
        .get(row / 8)
        .ok_or_else(|| Error::InvalidSegment("v6 validity row is absent".into()))?;
    Ok(byte & (1 << (row % 8)) != 0)
}

pub(super) fn binary_at<'a>(offsets: &[u8], data: &'a [u8], row: usize) -> Result<&'a [u8]> {
    let start = offset_at(offsets, row)?;
    let end = offset_at(offsets, row + 1)?;
    if start > end || end > data.len() {
        return invalid("v6 Arrow binary offsets are not monotonic or in bounds");
    }
    Ok(&data[start..end])
}

pub(super) fn offset_at(offsets: &[u8], index: usize) -> Result<usize> {
    let offset = index
        .checked_mul(8)
        .ok_or_else(|| Error::InvalidSegment("v6 Arrow offset index overflow".into()))?;
    let bytes = offsets
        .get(offset..offset + 8)
        .ok_or_else(|| Error::InvalidSegment("v6 Arrow offset is absent".into()))?;
    let value = i64::from_le_bytes(bytes.try_into().unwrap());
    usize::try_from(value)
        .map_err(|_| Error::InvalidSegment("v6 Arrow offset is negative or too large".into()))
}

pub(super) fn validate_offsets(
    offsets: &[u8],
    data: &[u8],
    rows: usize,
    maximum_item_bytes: usize,
    allow_empty: bool,
) -> Result<()> {
    if offsets.len() != (rows + 1).saturating_mul(8) || offset_at(offsets, 0)? != 0 {
        return invalid("v6 Arrow binary offset page has the wrong shape");
    }
    for row in 0..rows {
        let start = offset_at(offsets, row)?;
        let end = offset_at(offsets, row + 1)?;
        if start > end
            || end > data.len()
            || end - start > maximum_item_bytes
            || (!allow_empty && start == end)
        {
            return invalid("v6 Arrow binary offsets violate item bounds");
        }
    }
    if offset_at(offsets, rows)? != data.len() {
        return invalid("v6 Arrow binary offsets do not terminate at the data length");
    }
    Ok(())
}

pub(super) fn validate_validity(validity: &[u8], rows: usize) -> Result<()> {
    if validity.len() != rows.div_ceil(8) {
        return invalid("v6 Arrow validity bitmap has the wrong length");
    }
    let used = rows % 8;
    if used != 0 && validity.last().is_some_and(|byte| byte >> used != 0) {
        return invalid("v6 Arrow validity bitmap has non-zero padding bits");
    }
    Ok(())
}

pub(super) fn verify_file_digest(mut file: &File, physical_bytes: u64) -> Result<String> {
    let content_bytes = physical_bytes
        .checked_sub(FOOTER_BYTES as u64)
        .ok_or_else(|| Error::InvalidSegment("segment footer underflow".into()))?;
    file.seek(std::io::SeekFrom::Start(0))?;
    let mut hasher = ring::digest::Context::new(&ring::digest::SHA256);
    let mut buffer = vec![0; COPY_BUFFER_BYTES];
    let mut remaining = content_bytes;
    while remaining > 0 {
        let take = usize::try_from(remaining.min(buffer.len() as u64)).unwrap();
        file.read_exact(&mut buffer[..take])?;
        hasher.update(&buffer[..take]);
        remaining -= take as u64;
    }
    let mut footer = [0; FOOTER_BYTES];
    file.read_exact(&mut footer)?;
    let expected = std::str::from_utf8(&footer)
        .map_err(|_| Error::InvalidSegment("segment footer is not ASCII".into()))?;
    let actual = digest_hex(hasher.finish().as_ref());
    if expected != actual {
        return invalid("segment content checksum does not match");
    }
    Ok(actual)
}

pub(super) fn digest_hex(digest: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = Vec::with_capacity(digest.len() * 2);
    for byte in digest {
        encoded.push(HEX[(byte >> 4) as usize]);
        encoded.push(HEX[(byte & 0x0f) as usize]);
    }
    String::from_utf8(encoded).expect("hex digest is ASCII")
}

pub(super) fn file_equals_bytes(path: &AnchoredPath, expected: &[u8]) -> Result<bool> {
    let mut file = path.open_read()?;
    if file.metadata()?.len() != expected.len() as u64 {
        return Ok(false);
    }
    let mut buffer = vec![0; COPY_BUFFER_BYTES];
    let mut cursor = 0;
    while cursor < expected.len() {
        let end = cursor.saturating_add(buffer.len()).min(expected.len());
        file.read_exact(&mut buffer[..end - cursor])?;
        if buffer[..end - cursor] != expected[cursor..end] {
            return Ok(false);
        }
        cursor = end;
    }
    Ok(true)
}

pub(super) fn invalid<T>(reason: impl Into<String>) -> Result<T> {
    Err(Error::InvalidSegment(reason.into()))
}
