use crate::filesystem::{file_io, AnchoredPath, OpenOptions};
use crate::write_diagnostics::{PhaseClock, WalAppendPhases};
use crate::{Error, Result, WriteBatch};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{IoSlice, Seek, SeekFrom};
use std::path::{Path, PathBuf};

pub const WAL_FORMAT_VERSION: u16 = 1;
pub const WAL_MAX_PAYLOAD_BYTES: usize = 16 * 1024 * 1024;
const FILE_MAGIC: &[u8; 8] = b"RRDWAL01";
const RECORD_MAGIC: &[u8; 4] = b"RRD1";
const FILE_HEADER_BYTES: usize = 16;
const RECORD_HEADER_BYTES: usize = 32;
const RECORD_KIND_BATCH: u8 = 1;
#[cfg(target_os = "linux")]
const INITIAL_WAL_RESERVATION_BYTES: u64 = 1024 * 1024;
const INITIAL_WAL_RESERVATION_MIN_BATCH_BYTES: usize = 16 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Durability {
    Buffered,
    Authoritative,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WalBatch<'a> {
    pub first_sequence: u64,
    pub last_sequence: u64,
    pub payload: &'a [u8],
}

impl WalBatch<'_> {
    fn validate(&self) -> Result<()> {
        if self.first_sequence == 0
            || self.last_sequence < self.first_sequence
            || self.last_sequence == u64::MAX
        {
            return Err(Error::InvalidBatch(
                "sequence range must be non-zero, ordered, and leave room for its successor".into(),
            ));
        }
        if self.payload.is_empty() {
            return Err(Error::InvalidBatch("payload must not be empty".into()));
        }
        if self.payload.len() > WAL_MAX_PAYLOAD_BYTES {
            return Err(Error::InvalidBatch(format!(
                "payload is {} bytes; maximum is {WAL_MAX_PAYLOAD_BYTES}",
                self.payload.len()
            )));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppendReceipt {
    pub offset: u64,
    pub end_offset: u64,
    pub first_sequence: u64,
    pub last_sequence: u64,
    pub checksum: u32,
    pub durable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoveredBatch {
    pub offset: u64,
    pub first_sequence: u64,
    pub last_sequence: u64,
    pub checksum: u32,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Recovery {
    pub batches: Vec<RecoveredBatch>,
    pub recovered_through: u64,
    pub valid_bytes: u64,
    pub torn_tail: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RecoverySummary {
    pub recovered_through: u64,
    pub valid_bytes: u64,
    pub torn_tail: Option<u64>,
    pub payload_bytes: usize,
}

pub struct WalWriter {
    path: PathBuf,
    file: File,
    offset: u64,
    next_sequence: u64,
    poisoned: bool,
}

impl WalWriter {
    pub fn create(path: &Path) -> Result<Self> {
        Self::create_at(path, 1)
    }

    pub fn create_at(path: &Path, next_sequence: u64) -> Result<Self> {
        Self::create_at_anchored(&AnchoredPath::from_path(path)?, next_sequence)
    }

    pub(crate) fn create_at_anchored(path: &AnchoredPath, next_sequence: u64) -> Result<Self> {
        if next_sequence == 0 {
            return Err(Error::InvalidBatch(
                "WAL starting sequence must be non-zero".into(),
            ));
        }
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(path)?;
        file_io::write_all(&mut file, path.diagnostic_path(), &file_header())?;
        file_io::sync_all(&file, path.diagnostic_path())?;
        path.parent()
            .ok_or_else(|| Error::InvalidBatch("WAL requires an anchored parent".into()))?
            .sync_directory()?;
        Ok(Self {
            path: path.diagnostic_path().to_owned(),
            file,
            offset: FILE_HEADER_BYTES as u64,
            next_sequence,
            poisoned: false,
        })
    }

    pub fn open(path: &Path) -> Result<Self> {
        Self::open_at(path, 1)
    }

    pub fn open_at(path: &Path, starting_sequence: u64) -> Result<Self> {
        Self::open_at_anchored(&AnchoredPath::from_path(path)?, starting_sequence)
    }

    pub(crate) fn open_at_anchored(path: &AnchoredPath, starting_sequence: u64) -> Result<Self> {
        let mut file = OpenOptions::new().read(true).write(true).open(path)?;
        let recovery = replay_file(&mut file, starting_sequence, |_| Ok(()))?;
        Self::from_recovered_file(path, file, recovery)
    }

    pub(crate) fn open_empty_at_anchored(
        path: &AnchoredPath,
        starting_sequence: u64,
    ) -> Result<Self> {
        let mut file = OpenOptions::new().read(true).write(true).open(path)?;
        let summary = replay_file(&mut file, starting_sequence, |_| {
            Err(Error::Corruption {
                offset: 0,
                reason: "unpublished successor WAL contains data".into(),
            })
        })?;
        // An earlier create may have failed its sync/name barrier. This WAL is
        // about to become reachable, so a validated header alone is insufficient.
        file_io::sync_all(&file, path.diagnostic_path())?;
        path.parent()
            .ok_or_else(|| Error::InvalidBatch("WAL requires an anchored parent".into()))?
            .sync_directory()?;
        Self::from_recovered_file(path, file, summary)
    }

    pub(crate) fn from_recovered_file(
        path: &AnchoredPath,
        mut file: File,
        recovery: RecoverySummary,
    ) -> Result<Self> {
        if let Some(offset) = recovery.torn_tail {
            return Err(Error::TornTail { offset });
        }
        file.seek(SeekFrom::Start(recovery.valid_bytes))?;
        Ok(Self {
            path: path.diagnostic_path().to_owned(),
            file,
            offset: recovery.valid_bytes,
            next_sequence: recovery
                .recovered_through
                .checked_add(1)
                .ok_or_else(|| Error::InvalidBatch("sequence overflow".into()))?,
            poisoned: false,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn next_sequence(&self) -> u64 {
        self.next_sequence
    }

    pub fn append(
        &mut self,
        batch: &WalBatch<'_>,
        durability: Durability,
    ) -> Result<AppendReceipt> {
        self.append_inner(batch, durability, None)
    }

    fn append_inner(
        &mut self,
        batch: &WalBatch<'_>,
        durability: Durability,
        mut phases: Option<&mut WalAppendPhases>,
    ) -> Result<AppendReceipt> {
        let record_started = phases.is_some().then(PhaseClock::start);
        if self.poisoned {
            return Err(Error::PoisonedWriter);
        }
        batch.validate()?;
        if batch.first_sequence != self.next_sequence {
            return Err(Error::InvalidBatch(format!(
                "expected first sequence {}, received {}",
                self.next_sequence, batch.first_sequence
            )));
        }
        let header = record_header(batch)?;
        let offset = self.offset;
        let end_offset = offset
            .checked_add(RECORD_HEADER_BYTES as u64)
            .and_then(|value| value.checked_add(batch.payload.len() as u64))
            .ok_or_else(|| Error::InvalidBatch("WAL offset overflow".into()))?;
        if let Some(started) = record_started {
            phases
                .as_mut()
                .expect("phase recorder checked")
                .record_prepare = started.finish();
        }
        if offset == FILE_HEADER_BYTES as u64
            && batch.first_sequence == 1
            && batch.payload.len() >= INITIAL_WAL_RESERVATION_MIN_BATCH_BYTES
        {
            let reservation_started = phases.is_some().then(PhaseClock::start);
            reserve_initial_wal_extent(&self.file);
            if let Some(started) = reservation_started {
                phases
                    .as_mut()
                    .expect("phase recorder checked")
                    .initial_reservation = started.finish();
            }
        }
        let write_started = phases.is_some().then(PhaseClock::start);
        let write = (|| -> std::io::Result<()> {
            let written = loop {
                match file_io::write_vectored(
                    &mut self.file,
                    &self.path,
                    &[IoSlice::new(&header), IoSlice::new(batch.payload)],
                ) {
                    Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                    result => break result?,
                }
            };
            if written == 0 {
                return Err(std::io::ErrorKind::WriteZero.into());
            }
            if written < header.len() {
                file_io::write_all(&mut self.file, &self.path, &header[written..])?;
                file_io::write_all(&mut self.file, &self.path, batch.payload)?;
            } else {
                file_io::write_all(
                    &mut self.file,
                    &self.path,
                    &batch.payload[written - header.len()..],
                )?;
            }
            Ok(())
        })();
        if let Err(error) = write {
            self.poisoned = true;
            return Err(Error::Io(error));
        }
        if let Some(started) = write_started {
            phases.as_mut().expect("phase recorder checked").write = started.finish();
        }
        if durability == Durability::Authoritative {
            let sync_started = phases.is_some().then(PhaseClock::start);
            if let Err(error) = file_io::sync_data(&self.file, &self.path) {
                self.poisoned = true;
                return Err(Error::Io(error));
            }
            if let Some(started) = sync_started {
                phases.as_mut().expect("phase recorder checked").sync = started.finish();
            }
        }
        self.offset = end_offset;
        self.next_sequence = batch
            .last_sequence
            .checked_add(1)
            .ok_or_else(|| Error::InvalidBatch("sequence overflow".into()))?;
        Ok(AppendReceipt {
            offset,
            end_offset,
            first_sequence: batch.first_sequence,
            last_sequence: batch.last_sequence,
            checksum: u32::from_be_bytes(header[28..32].try_into().expect("fixed checksum field")),
            durable: durability == Durability::Authoritative,
        })
    }

    /// Allocates one MVCC sequence per operation and writes the full batch as
    /// one indivisible recovery frame.
    pub fn append_write_batch(
        &mut self,
        batch: &WriteBatch,
        durability: Durability,
    ) -> Result<AppendReceipt> {
        let payload = batch.encode()?;
        self.append_encoded_write_batch(batch, &payload, durability)
    }

    pub(crate) fn append_encoded_write_batch(
        &mut self,
        batch: &WriteBatch,
        payload: &[u8],
        durability: Durability,
    ) -> Result<AppendReceipt> {
        self.append_encoded_write_batch_inner(batch, payload, durability, None)
    }

    pub(crate) fn append_encoded_write_batch_profiled(
        &mut self,
        batch: &WriteBatch,
        payload: &[u8],
        durability: Durability,
    ) -> Result<(AppendReceipt, WalAppendPhases)> {
        let mut phases = WalAppendPhases::default();
        let receipt =
            self.append_encoded_write_batch_inner(batch, payload, durability, Some(&mut phases))?;
        Ok((receipt, phases))
    }

    fn append_encoded_write_batch_inner(
        &mut self,
        batch: &WriteBatch,
        payload: &[u8],
        durability: Durability,
        phases: Option<&mut WalAppendPhases>,
    ) -> Result<AppendReceipt> {
        let count = u64::try_from(batch.len())
            .map_err(|_| Error::InvalidBatch("operation count exceeds u64".into()))?;
        let last_sequence = self
            .next_sequence
            .checked_add(count - 1)
            .ok_or_else(|| Error::InvalidBatch("sequence range overflow".into()))?;
        self.append_inner(
            &WalBatch {
                first_sequence: self.next_sequence,
                last_sequence,
                payload,
            },
            durability,
            phases,
        )
    }

    pub fn sync(&mut self) -> Result<u64> {
        if self.poisoned {
            return Err(Error::PoisonedWriter);
        }
        if let Err(error) = file_io::sync_data(&self.file, &self.path) {
            self.poisoned = true;
            return Err(error.into());
        }
        Ok(self.next_sequence.saturating_sub(1))
    }
}

pub fn recover(path: &Path) -> Result<Recovery> {
    recover_from(path, 1)
}

pub fn recover_from(path: &Path, starting_sequence: u64) -> Result<Recovery> {
    recover_anchored(&AnchoredPath::from_path(path)?, starting_sequence)
}

pub(crate) fn recover_anchored(path: &AnchoredPath, starting_sequence: u64) -> Result<Recovery> {
    let mut batches = Vec::new();
    let summary = replay_anchored(path, starting_sequence, |batch| {
        batches.push(batch);
        Ok(())
    })?;
    Ok(Recovery {
        recovered_through: summary.recovered_through,
        batches,
        valid_bytes: summary.valid_bytes,
        torn_tail: summary.torn_tail,
    })
}

pub(crate) fn replay_anchored<F>(
    path: &AnchoredPath,
    starting_sequence: u64,
    replay: F,
) -> Result<RecoverySummary>
where
    F: FnMut(RecoveredBatch) -> Result<()>,
{
    let mut file = path.open_read()?;
    replay_file(&mut file, starting_sequence, replay)
}

pub(crate) fn replay_file<F>(
    file: &mut File,
    starting_sequence: u64,
    mut replay: F,
) -> Result<RecoverySummary>
where
    F: FnMut(RecoveredBatch) -> Result<()>,
{
    if starting_sequence == 0 {
        return Err(Error::InvalidBatch(
            "WAL recovery sequence must be non-zero".into(),
        ));
    }
    file.seek(SeekFrom::Start(0))?;
    let length = file.metadata()?.len();
    if length < FILE_HEADER_BYTES as u64 {
        return Err(Error::Corruption {
            offset: 0,
            reason: "incomplete WAL file header".into(),
        });
    }
    let mut file_header_bytes = [0u8; FILE_HEADER_BYTES];
    file_io::read_exact(file, &mut file_header_bytes)?;
    validate_file_header(&file_header_bytes)?;

    let mut offset = FILE_HEADER_BYTES as u64;
    let mut expected_sequence = starting_sequence;
    let mut torn_tail = None;
    let mut payload_bytes = 0usize;
    while offset < length {
        let remaining = length - offset;
        if remaining < RECORD_HEADER_BYTES as u64 {
            torn_tail = Some(offset);
            break;
        }
        let mut header = [0u8; RECORD_HEADER_BYTES];
        file_io::read_exact(file, &mut header)?;
        let decoded = decode_record_header(&header, offset)?;
        if decoded.first_sequence != expected_sequence {
            return Err(Error::Corruption {
                offset,
                reason: format!(
                    "non-contiguous sequence: expected {expected_sequence}, found {}",
                    decoded.first_sequence
                ),
            });
        }
        let end = offset
            .checked_add(RECORD_HEADER_BYTES as u64)
            .and_then(|value| value.checked_add(decoded.payload_len as u64))
            .ok_or_else(|| Error::Corruption {
                offset,
                reason: "record length overflow".into(),
            })?;
        if end > length {
            torn_tail = Some(offset);
            break;
        }
        let mut payload = vec![0u8; decoded.payload_len];
        file_io::read_exact(file, &mut payload)?;
        let actual = crc32c(&[&header[4..28], &payload]);
        if actual != decoded.checksum {
            return Err(Error::Corruption {
                offset,
                reason: format!(
                    "checksum mismatch: expected {:08x}, computed {actual:08x}",
                    decoded.checksum
                ),
            });
        }
        payload_bytes =
            payload_bytes
                .checked_add(payload.len())
                .ok_or_else(|| Error::Corruption {
                    offset,
                    reason: "recovered payload byte count overflow".into(),
                })?;
        replay(RecoveredBatch {
            offset,
            first_sequence: decoded.first_sequence,
            last_sequence: decoded.last_sequence,
            checksum: decoded.checksum,
            payload,
        })?;
        expected_sequence =
            decoded
                .last_sequence
                .checked_add(1)
                .ok_or_else(|| Error::Corruption {
                    offset,
                    reason: "sequence overflow".into(),
                })?;
        offset = end;
    }
    Ok(RecoverySummary {
        recovered_through: expected_sequence.saturating_sub(1),
        valid_bytes: offset,
        torn_tail,
        payload_bytes,
    })
}

/// Truncates only an incomplete final frame previously classified by recovery.
/// Complete corrupt frames are never converted into apparent data loss.
pub fn repair_torn_tail(path: &Path) -> Result<Recovery> {
    repair_torn_tail_from(path, 1)
}

pub(crate) fn repair_torn_tail_from(path: &Path, starting_sequence: u64) -> Result<Recovery> {
    let path = AnchoredPath::from_path(path)?;
    let mut file = OpenOptions::new().read(true).write(true).open(&path)?;
    let mut batches = Vec::new();
    let summary = replay_file(&mut file, starting_sequence, |batch| {
        batches.push(batch);
        Ok(())
    })?;
    if summary.torn_tail.is_some() {
        file.set_len(summary.valid_bytes)?;
        file.sync_all()?;
    }
    Ok(Recovery {
        batches,
        recovered_through: summary.recovered_through,
        valid_bytes: summary.valid_bytes,
        torn_tail: None,
    })
}

struct DecodedHeader {
    payload_len: usize,
    first_sequence: u64,
    last_sequence: u64,
    checksum: u32,
}

fn file_header() -> [u8; FILE_HEADER_BYTES] {
    let mut header = [0u8; FILE_HEADER_BYTES];
    header[0..8].copy_from_slice(FILE_MAGIC);
    header[8..10].copy_from_slice(&WAL_FORMAT_VERSION.to_be_bytes());
    header[10..12].copy_from_slice(&(FILE_HEADER_BYTES as u16).to_be_bytes());
    let checksum = crc32c(&[&header[0..12]]);
    header[12..16].copy_from_slice(&checksum.to_be_bytes());
    header
}

fn validate_file_header(header: &[u8; FILE_HEADER_BYTES]) -> Result<()> {
    if &header[0..8] != FILE_MAGIC {
        return Err(Error::Corruption {
            offset: 0,
            reason: "WAL file magic does not match".into(),
        });
    }
    let version = u16::from_be_bytes(header[8..10].try_into().expect("fixed version field"));
    if version != WAL_FORMAT_VERSION {
        return Err(Error::UnsupportedVersion {
            object: "WAL",
            version,
        });
    }
    let header_len = u16::from_be_bytes(header[10..12].try_into().expect("fixed length field"));
    if header_len as usize != FILE_HEADER_BYTES {
        return Err(Error::Corruption {
            offset: 10,
            reason: format!("unexpected WAL header length {header_len}"),
        });
    }
    let expected = u32::from_be_bytes(header[12..16].try_into().expect("fixed checksum field"));
    let actual = crc32c(&[&header[0..12]]);
    if actual != expected {
        return Err(Error::Corruption {
            offset: 0,
            reason: "WAL header checksum mismatch".into(),
        });
    }
    Ok(())
}

fn record_header(batch: &WalBatch<'_>) -> Result<[u8; RECORD_HEADER_BYTES]> {
    let payload_len = u32::try_from(batch.payload.len())
        .map_err(|_| Error::InvalidBatch("payload length exceeds u32".into()))?;
    let mut header = [0u8; RECORD_HEADER_BYTES];
    header[0..4].copy_from_slice(RECORD_MAGIC);
    header[4..6].copy_from_slice(&WAL_FORMAT_VERSION.to_be_bytes());
    header[6] = RECORD_KIND_BATCH;
    header[7] = 0;
    header[8..12].copy_from_slice(&payload_len.to_be_bytes());
    header[12..20].copy_from_slice(&batch.first_sequence.to_be_bytes());
    header[20..28].copy_from_slice(&batch.last_sequence.to_be_bytes());
    let checksum = crc32c(&[&header[4..28], batch.payload]);
    header[28..32].copy_from_slice(&checksum.to_be_bytes());
    Ok(header)
}

fn decode_record_header(header: &[u8; RECORD_HEADER_BYTES], offset: u64) -> Result<DecodedHeader> {
    if &header[0..4] != RECORD_MAGIC {
        return Err(Error::Corruption {
            offset,
            reason: "record magic does not match".into(),
        });
    }
    let version = u16::from_be_bytes(header[4..6].try_into().expect("fixed version field"));
    if version != WAL_FORMAT_VERSION {
        return Err(Error::UnsupportedVersion {
            object: "WAL record",
            version,
        });
    }
    if header[6] != RECORD_KIND_BATCH || header[7] != 0 {
        return Err(Error::Corruption {
            offset,
            reason: "unknown WAL record kind or flags".into(),
        });
    }
    let payload_len =
        u32::from_be_bytes(header[8..12].try_into().expect("fixed payload length")) as usize;
    if payload_len == 0 || payload_len > WAL_MAX_PAYLOAD_BYTES {
        return Err(Error::Corruption {
            offset,
            reason: format!("invalid payload length {payload_len}"),
        });
    }
    let first_sequence =
        u64::from_be_bytes(header[12..20].try_into().expect("fixed sequence field"));
    let last_sequence =
        u64::from_be_bytes(header[20..28].try_into().expect("fixed sequence field"));
    if first_sequence == 0 || last_sequence < first_sequence {
        return Err(Error::Corruption {
            offset,
            reason: "invalid record sequence range".into(),
        });
    }
    Ok(DecodedHeader {
        payload_len,
        first_sequence,
        last_sequence,
        checksum: u32::from_be_bytes(header[28..32].try_into().expect("fixed checksum field")),
    })
}

fn crc32c(chunks: &[&[u8]]) -> u32 {
    chunks
        .iter()
        .fold(0, |checksum, chunk| crc32c::crc32c_append(checksum, chunk))
}

#[cfg(target_os = "linux")]
fn reserve_initial_wal_extent(file: &File) {
    use std::os::fd::AsRawFd;

    // Best effort only: reservation must never turn a write that fits into an
    // artificial ENOSPC. KEEP_SIZE leaves the recovery-visible length at the
    // last acknowledged byte, and this bounded first extent is never grown.
    unsafe {
        libc::fallocate(
            file.as_raw_fd(),
            libc::FALLOC_FL_KEEP_SIZE,
            0,
            INITIAL_WAL_RESERVATION_BYTES as libc::off_t,
        );
    }
}

#[cfg(not(target_os = "linux"))]
fn reserve_initial_wal_extent(_file: &File) {}

#[cfg(test)]
mod tests {
    use super::crc32c;

    #[test]
    fn crc32c_matches_the_published_check_value() {
        assert_eq!(crc32c(&[b"123456789"]), 0xe306_9283);
    }
}
